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
    {"type":"SimpleItem","autoContinue":true,"command":16,"doJumpId":3,"frame":3,"params":[0,0,0,null,47.002,8.0,50]}]},"geoFence":{"version":2,"polygons":[],"circles":[]},"rallyPoints":{"version":2,"points":[]}}"#;

const GRIPPER: i64 = 211;
const SET_SERVO: i64 = 183;
const DELAY: i64 = 93;
const WAYPOINT: i64 = 16;
const TAKEOFF: i64 = 22;

type Plan = Core<Owner<Facade<NoHost>>>;

fn invoked(core: &Plan, path: &str, args: Value) -> Value {
    serde_json::from_str(&core.invoke(path, &args.to_string())).unwrap_or(Value::Null)
}

fn refusal(core: &Plan, path: &str, args: Value) -> Value {
    invoked(core, path, args)["reason"].clone()
}

fn drop_of(core: &Plan, waypoint: usize) -> Value {
    serde_json::from_str::<Value>(&core.get(&format!("view.itemFacts({waypoint})"))).unwrap()["drop"]
        .clone()
}

fn flown() -> Vec<(i64, Option<f64>, Option<f64>)> {
    groundstation::coreplan::current_document()
        .items
        .iter()
        .filter_map(|item| match item {
            plandoc::Item::Simple(s) => Some((s.command, s.params[0], s.params[1])),
            plandoc::Item::Complex { .. } => None,
        })
        .collect()
}

fn commands() -> Vec<i64> {
    flown().iter().map(|(command, ..)| *command).collect()
}

#[test]
fn a_waypoints_drop_is_placed_edited_and_removed_by_naming_the_waypoint_and_keeps_its_own_servo() {
    let scratch = std::env::temp_dir().join(format!("plan-drop-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    groundstation::settingsstore::open(&scratch.join("settings.ini"));
    let core = Core::new(Owner(Facade(NoHost)));
    assert_eq!(
        invoked(&core, "mission.insert", json!(["waypoint", 47.0, 8.0, -1]))["ok"],
        true
    );
    while groundstation::coreplan::home_ground_pending() {
        std::thread::yield_now();
    }
    let loaded = plandoc::load(PLAN, 2).unwrap();
    assert_eq!(groundstation::coreplan::apply(|_| Ok(loaded))["ok"], true);
    assert_eq!(
        drop_of(&core, 2),
        json!({ "index": null, "wait": 0.0, "waitOffered": true, "servo": false, "output": null, "pwm": null, "bay": null }),
        "a waypoint with no drop offers one"
    );
    assert_eq!(drop_of(&core, 1), Value::Null, "the takeoff takes no drop");
    assert_eq!(
        refusal(&core, "core.plan.placeDrop", json!([])),
        plandoc::NOT_A_WAYPOINT
    );
    assert_eq!(
        refusal(&core, "core.plan.placeDrop", json!([1])),
        plandoc::NOT_A_WAYPOINT
    );
    assert_eq!(
        refusal(&core, "core.plan.removeDrop", json!([2])),
        plandoc::NO_DROP_HERE
    );
    assert_eq!(
        invoked(&core, "core.plan.placeDrop", json!([2]))["ok"],
        true
    );
    assert_eq!(commands(), [TAKEOFF, WAYPOINT, GRIPPER, WAYPOINT]);
    assert_eq!(
        invoked(
            &core,
            "core.plan.setDropRelease",
            json!([2, { "servo": true }])
        )["ok"],
        true
    );
    assert_eq!(
        invoked(
            &core,
            "core.plan.setDropRelease",
            json!([2, { "output": 4, "pwm": 1500 }])
        )["ok"],
        true
    );
    assert_eq!(
        flown()[2],
        (SET_SERVO, Some(4.0), Some(1500.0)),
        "the servo and PWM are written on the mission item"
    );
    assert_eq!(
        refusal(&core, "core.plan.placeDrop", json!([2, 700])),
        plandoc::WAIT_OUT_OF_RANGE
    );
    assert_eq!(
        refusal(&core, "core.plan.placeDrop", json!([2, "long"])),
        plandoc::WAIT_OUT_OF_RANGE
    );
    assert_eq!(
        invoked(&core, "core.plan.placeDrop", json!([2, 10]))["ok"],
        true
    );
    assert_eq!(commands(), [TAKEOFF, WAYPOINT, DELAY, SET_SERVO, WAYPOINT]);
    assert_eq!(
        drop_of(&core, 2),
        json!({ "index": 4, "wait": 10.0, "waitOffered": true, "servo": true, "output": 4.0, "pwm": 1500.0, "bay": null })
    );
    assert_eq!(
        refusal(&core, "core.plan.placeDrop", json!([3])),
        plandoc::NOT_A_WAYPOINT,
        "the wait is not a waypoint"
    );
    assert!(
        refusal(&core, "core.plan.placeDrop", json!([5]))
            .as_str()
            .is_some_and(|reason| reason.starts_with("Every bay already drops")),
        "the one bay is used, and the second waypoint is told so"
    );
    assert_eq!(commands().len(), 5, "a refused drop adds nothing");
    assert_eq!(
        refusal(
            &core,
            "core.plan.setDropRelease",
            json!([2, { "output": 40 }])
        ),
        "A servo output is a whole number from 1 to 32."
    );
    let on_servo_four = groundstation::coreplan::current_document();
    assert_eq!(
        invoked(
            &core,
            "core.plan.setDropRelease",
            json!([2, { "output": 9 }])
        )["ok"],
        true
    );
    assert_eq!(
        drop_of(&core, 2)["output"],
        json!(9.0),
        "the drop moved to servo 9, and the bay with it"
    );
    let on_servo_nine = groundstation::coreplan::current_document();
    assert_eq!(
        groundstation::coreplan::apply(|_| Ok(on_servo_four))["ok"],
        true
    );
    assert_eq!(
        (flown()[3], drop_of(&core, 2)["index"].clone()),
        ((SET_SERVO, Some(4.0), Some(1500.0)), Value::Null),
        "undone, the item is servo 4 again, but the bay stays on servo 9, so servo 4 opens no bay and is a plain servo"
    );
    assert_eq!(
        groundstation::coreplan::apply(|_| Ok(on_servo_nine))["ok"],
        true
    );
    assert_eq!(
        invoked(
            &core,
            "core.plan.setDropRelease",
            json!([2, { "servo": false }])
        )["ok"],
        true
    );
    assert_eq!(
        commands(),
        [TAKEOFF, WAYPOINT, DELAY, GRIPPER, WAYPOINT],
        "switched to the gripper, the drop keeps its wait"
    );
    assert_eq!(
        invoked(&core, "core.plan.removeDrop", json!([2]))["ok"],
        true
    );
    assert_eq!(commands(), [TAKEOFF, WAYPOINT, WAYPOINT]);
    assert_eq!(
        invoked(
            &core,
            "core.plan.setDropRelease",
            json!([2, { "servo": true }])
        )["reason"],
        plandoc::NO_DROP_HERE
    );
    let _ = std::fs::remove_dir_all(scratch);
}
