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
    {"type":"SimpleItem","autoContinue":true,"command":183,"doJumpId":3,"frame":2,"params":[4,1500,0,0,0,0,0]},
    {"type":"SimpleItem","autoContinue":true,"command":16,"doJumpId":4,"frame":3,"params":[0,0,0,null,47.002,8.0,50]},
    {"type":"SimpleItem","autoContinue":true,"command":16,"doJumpId":5,"frame":3,"params":[0,0,0,null,47.003,8.0,50]},
    {"type":"SimpleItem","autoContinue":true,"command":177,"doJumpId":6,"frame":2,"params":[4,10,0,0,0,0,0]}]},"geoFence":{"version":2,"polygons":[],"circles":[]},"rallyPoints":{"version":2,"points":[]}}"#;

const JUMP_ROW: usize = 6;

type Plan = Core<Owner<Facade<NoHost>>>;

fn target_field(core: &Plan) -> Value {
    serde_json::from_str::<Value>(&core.get(&format!("view.itemFacts({JUMP_ROW})"))).unwrap()["fields"]
        .as_array()
        .and_then(|fields| {
            fields
                .iter()
                .find(|field| field["pathSuffix"] == "textFieldFacts.0")
                .cloned()
        })
        .unwrap()
}

fn jumps_to() -> Option<f64> {
    match groundstation::coreplan::current_document()
        .items
        .get(JUMP_ROW - 1)
    {
        Some(plandoc::Item::Simple(jump)) => jump.params[0],
        _ => None,
    }
}

fn typed(core: &Plan, waypoint: i64) -> Value {
    serde_json::from_str(&core.set(
        &format!("plan.missionController.visualItems.{JUMP_ROW}.textFieldFacts.0"),
        &json!({ "value": waypoint }).to_string(),
    ))
    .unwrap_or(Value::Null)
}

#[test]
fn a_jump_is_typed_as_the_waypoint_number_on_the_rows_and_sent_as_that_waypoints_sequence() {
    let scratch = std::env::temp_dir().join(format!("plan-jump-{}", std::process::id()));
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
    let shown = target_field(&core);
    assert_eq!(
        (shown["label"].clone(), shown["value"].as_f64(), jumps_to()),
        (json!("Waypoint"), Some(2.0), Some(4.0)),
        "the jump goes to mission sequence 4, which the rows call Waypoint 2: a servo command sits between the waypoints"
    );
    assert_eq!(typed(&core, 3)["ok"], true);
    assert_eq!(
        (target_field(&core)["value"].as_f64(), jumps_to()),
        (Some(3.0), Some(5.0)),
        "Waypoint 3 typed, sequence 5 sent"
    );
    assert_eq!(typed(&core, 1)["ok"], true);
    assert_eq!(jumps_to(), Some(2.0));
    assert_eq!(
        typed(&core, 9)["reason"],
        "This plan has no waypoint with that number. A jump goes to a numbered waypoint."
    );
    assert_eq!(jumps_to(), Some(2.0), "a refused number changes nothing");
    let _ = std::fs::remove_dir_all(scratch);
}
