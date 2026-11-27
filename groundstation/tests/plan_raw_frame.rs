use groundstation::plandoc;
use groundstation::router::{Backend, Core};
use groundstation::settingsstore::Owner;
use groundstation::vehiclefacade::Facade;
use serde_json::{Value, json};

struct NoHost;

impl Backend for NoHost {
    fn get(&self, _path: &str) -> String {
        json!({ "kind": "null" }).to_string()
    }
    fn get_fields(&self, _path: &str, _fields: &str) -> String {
        json!({ "kind": "null" }).to_string()
    }
    fn set(&self, _path: &str, _value: &str) -> String {
        json!({ "ok": false, "reason": "no host" }).to_string()
    }
    fn invoke(&self, _path: &str, _args: &str) -> String {
        json!({ "ok": false, "reason": "no host" }).to_string()
    }
    fn watch(&self, _paths: &[String]) {}
}

const PLAN: &str = r#"{"fileType":"Plan","version":1,"groundStation":"QGroundControl","mission":{"version":2,"firmwareType":3,"vehicleType":2,"cruiseSpeed":15,"hoverSpeed":5,"plannedHomePosition":[47.0,8.0,485],"items":[
    {"type":"SimpleItem","autoContinue":true,"command":22,"doJumpId":1,"frame":3,"params":[0,0,0,null,0,0,5]},
    {"type":"SimpleItem","autoContinue":true,"command":16,"doJumpId":2,"frame":3,"params":[0,0,0,null,47.001,8.0,50]},
    {"type":"SimpleItem","autoContinue":true,"command":183,"doJumpId":3,"frame":2,"params":[4,1500,0,0,0,0,0]}]},"geoFence":{"version":2,"polygons":[],"circles":[]},"rallyPoints":{"version":2,"points":[]}}"#;

const ABOVE_SEA_LEVEL: i64 = 0;
const LOCAL_NED: i64 = 1;
const RELATIVE_WHOLE_NUMBERS: i64 = 6;
const ABOVE_TERRAIN: i64 = 10;

fn written(core: &Core<Owner<Facade<NoHost>>>, item: usize, property: &str, value: Value) -> Value {
    serde_json::from_str(&core.set(
        &format!("plan.missionController.visualItems.{item}.{property}"),
        &json!({ "value": value }).to_string(),
    ))
    .unwrap_or(Value::Null)
}

fn waypoint() -> (i64, i64, Option<f64>, f64) {
    let document = groundstation::coreplan::current_document();
    let sent = plandoc::mission(&document)["mission"]["items"][1].clone();
    let read = groundstation::missionitems::document_reads(&document, -1).unwrap()[2].clone();
    (
        sent["frame"].as_i64().unwrap(),
        read["altitudeFrame"].as_i64().unwrap(),
        sent["params"][6].as_f64(),
        read["amslEntryAlt"].as_f64().unwrap() - document.home.map_or(0.0, |home| home[2]),
    )
}

#[test]
fn a_frame_picked_in_the_raw_view_is_the_frame_sent_and_the_altitude_mode_every_reading_uses() {
    let scratch = std::env::temp_dir().join(format!("plan-raw-frame-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    groundstation::settingsstore::open(&scratch.join("settings.ini"));
    let core = Core::new(Owner(Facade(NoHost)));
    assert_eq!(
        serde_json::from_str::<Value>(&core.invoke(
            "mission.insert",
            &json!(["waypoint", 47.0, 8.0, -1]).to_string()
        ))
        .unwrap()["ok"],
        true
    );
    while groundstation::coreplan::home_ground_pending() {
        std::thread::yield_now();
    }
    let loaded = plandoc::load(PLAN, 2).unwrap();
    assert_eq!(groundstation::coreplan::apply(|_| Ok(loaded))["ok"], true);
    assert_eq!(
        waypoint(),
        (3, groundstation::altitudemodes::RELATIVE, Some(50.0), 50.0),
        "50 m over the launch"
    );
    assert_eq!(written(&core, 2, "rawEdit", json!(true))["ok"], true);
    assert_eq!(
        written(
            &core,
            2,
            "comboboxFacts.1.enumIndex",
            json!(ABOVE_SEA_LEVEL)
        )["ok"],
        true
    );
    let launch = groundstation::coreplan::current_document().home.unwrap()[2];
    assert!(launch > 400.0, "the launch stands well above the sea");
    assert_eq!(
        waypoint(),
        (
            0,
            groundstation::altitudemodes::ABSOLUTE,
            Some(50.0),
            50.0 - launch
        ),
        "the raw value stays 50 and is now read as 50 m above sea level, far under the launch, by the list, the chart and the terrain check, as the aircraft will fly it"
    );
    assert_eq!(
        written(
            &core,
            2,
            "comboboxFacts.1.enumIndex",
            json!(RELATIVE_WHOLE_NUMBERS)
        )["ok"],
        true
    );
    assert_eq!(
        waypoint(),
        (6, groundstation::altitudemodes::RELATIVE, Some(50.0), 50.0),
        "a whole-number frame is the same datum"
    );
    assert_eq!(
        written(&core, 2, "comboboxFacts.1.enumIndex", json!(ABOVE_TERRAIN))["ok"],
        true
    );
    assert_eq!(
        (waypoint().0, waypoint().1),
        (10, groundstation::altitudemodes::TERRAIN_FRAME)
    );
    assert_eq!(
        written(&core, 2, "comboboxFacts.1.enumIndex", json!(LOCAL_NED)),
        json!({ "ok": false, "reason": plandoc::FRAME_NEEDS_A_DATUM }),
        "an item with a height cannot take a frame that names no datum"
    );
    assert_eq!(written(&core, 3, "rawEdit", json!(true))["ok"], true);
    assert_eq!(
        written(&core, 3, "comboboxFacts.1.enumIndex", json!(LOCAL_NED))["ok"],
        true,
        "a command with no height takes any frame"
    );
    assert_eq!(written(&core, 2, "textFieldFacts.3", json!(90))["ok"], true);
    let yaw = || match groundstation::coreplan::current_document().items.get(1) {
        Some(plandoc::Item::Simple(waypoint)) => waypoint.params[3],
        _ => None,
    };
    assert_eq!(yaw(), Some(90.0));
    assert_eq!(
        written(&core, 2, "textFieldFacts.3", json!(""))["ok"],
        true,
        "a raw value emptied is unset again, as it was before it was typed"
    );
    assert_eq!(yaw(), None);
    assert_eq!(
        written(&core, 2, "textFieldFacts.6", json!(""))["reason"],
        "That field takes a number.",
        "a height cannot be left unset"
    );
    let reloaded = plandoc::load(
        &plandoc::save(&groundstation::coreplan::current_document()).to_string(),
        2,
    )
    .unwrap();
    assert_eq!(
        reloaded,
        groundstation::coreplan::current_document(),
        "the saved plan carries one frame, not a frame and a mode that disagree"
    );
    let _ = std::fs::remove_dir_all(scratch);
}
