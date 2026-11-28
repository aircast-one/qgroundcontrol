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

type Plan = Core<Owner<Facade<NoHost>>>;

fn invoked(core: &Plan, path: &str, args: Value) -> Value {
    serde_json::from_str(&core.invoke(path, &args.to_string())).unwrap_or(Value::Null)
}

fn actions(core: &Plan, waypoint: usize) -> Vec<(String, usize)> {
    serde_json::from_str::<Value>(&core.get(&format!("view.itemFacts({waypoint})"))).unwrap()["actions"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|step| {
            (
                step["kind"].as_str().unwrap_or_default().to_string(),
                step["places"].as_array().map_or(0, Vec::len),
            )
        })
        .collect()
}

fn sent() -> Vec<i64> {
    groundstation::planitems::flatten(&plandoc::mission(&plandoc::as_flown(
        &groundstation::coreplan::current_document(),
    )))
    .unwrap()
    .iter()
    .map(|item| item.command)
    .collect()
}

#[test]
fn the_core_lists_a_waypoints_actions_in_flight_order_and_moves_one_when_asked() {
    let scratch = std::env::temp_dir().join(format!("plan-actions-{}", std::process::id()));
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
    let aimed = [
        ("specifyGimbal", json!(true)),
        ("gimbalYaw", json!(60.0)),
        ("cameraAction", json!(6)),
    ]
    .iter()
    .try_fold(plandoc::load(PLAN, 2).unwrap(), |doc, (property, value)| {
        plandoc::set_camera(&doc, 2, property, value)
    })
    .and_then(|doc| plandoc::set_param(&doc, 2, 1, 10.0))
    .unwrap();
    assert_eq!(groundstation::coreplan::apply(|_| Ok(aimed))["ok"], true);
    assert_eq!(
        invoked(&core, "core.plan.placeDrop", json!([2]))["ok"],
        true
    );
    let listed = |kinds: [&str; 4]| -> Vec<(String, usize)> {
        kinds.iter().map(|kind| (kind.to_string(), 4)).collect()
    };
    let revision = || groundstation::coreplan::revision();
    assert_eq!(
        actions(&core, 2),
        listed(["Hover", "Gimbal", "Camera", "Drop"])
    );
    assert_eq!(
        actions(&core, 1),
        Vec::new(),
        "the takeoff has no actions to order"
    );
    assert_eq!(sent(), [16, 22, 16, 205, 2000, 211, 16]);
    groundstation::coreplan::note_undo_tracking(true);
    let before = revision();
    assert_eq!(
        invoked(&core, "core.plan.moveAction", json!([2, 0, 2]))["ok"],
        true,
        "the hold dragged two places down in one move"
    );
    assert!(
        revision() > before,
        "a move raises the revision, so a head knows its rows are older than the plan"
    );
    assert_eq!(
        actions(&core, 2),
        listed(["Gimbal", "Camera", "Wait", "Drop"])
    );
    assert_eq!(invoked(&core, "plan.undo", json!([]))["ok"], true);
    assert_eq!(
        actions(&core, 2),
        listed(["Hover", "Gimbal", "Camera", "Drop"]),
        "one undo puts back the order from before the drag, however quickly the edits before it came"
    );
    assert_eq!(invoked(&core, "plan.redo", json!([]))["ok"], true);
    assert_eq!(
        actions(&core, 2),
        listed(["Gimbal", "Camera", "Wait", "Drop"])
    );
    assert_eq!(
        sent(),
        [16, 22, 16, 205, 2000, 93, 211, 16],
        "aim, photo, a ten second delay, then the drop: the order the pilot set is the order uploaded"
    );
    assert_eq!(
        invoked(&core, "core.plan.setWait", json!([2, 2, 20]))["ok"],
        true
    );
    assert_eq!(
        invoked(&core, "core.plan.setWait", json!([2, 2, 0.5]))["reason"],
        plandoc::WAIT_SECONDS_RANGE
    );
    assert_eq!(
        invoked(&core, "core.plan.setWait", json!([2, 0, 5]))["reason"],
        plandoc::NOT_A_WAIT
    );
    let waited = || match groundstation::coreplan::current_document().items.get(2) {
        Some(plandoc::Item::Simple(delay)) => delay.params[0],
        _ => None,
    };
    assert_eq!(waited(), Some(20.0), "the wait is retimed where it stands");
    assert_eq!(
        invoked(&core, "core.plan.moveAction", json!([2, 3, 0]))["ok"],
        true
    );
    assert_eq!(
        actions(&core, 2),
        listed(["Drop", "Gimbal", "Camera", "Wait"])
    );
    assert_eq!(sent(), [16, 22, 16, 211, 205, 2000, 93, 16]);
    assert_eq!(
        invoked(&core, "core.plan.moveAction", json!([2, 0, 0]))["reason"],
        plandoc::ACTION_AT_THE_END
    );
    assert_eq!(
        invoked(&core, "core.plan.moveAction", json!([2, 0, 9]))["reason"],
        plandoc::NO_SUCH_ACTION
    );
    assert_eq!(
        invoked(&core, "core.plan.moveAction", json!([1, 0, 1]))["reason"],
        plandoc::NOT_A_WAYPOINT
    );
    assert_eq!(
        invoked(&core, "core.plan.moveAction", json!([2]))["reason"],
        "Moving an action needs the waypoint, the action's place and its new place."
    );
    assert_eq!(
        invoked(&core, "core.plan.removeDrop", json!([2]))["ok"],
        true
    );
    assert_eq!(
        sent(),
        [16, 22, 16, 205, 2000, 93, 16],
        "the drop is found and removed wherever it stands in the order"
    );
    let _ = std::fs::remove_dir_all(scratch);
}
