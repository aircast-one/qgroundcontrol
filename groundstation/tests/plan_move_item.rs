use std::sync::{Arc, Mutex};

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

fn invoked(core: &Core<Owner<Facade<NoHost>>>, path: &str, args: Value) -> Value {
    serde_json::from_str(&core.invoke(path, &args.to_string())).unwrap_or(Value::Null)
}

fn selected() -> Value {
    groundstation::coreplan::controller_fields("plan.missionController")
        .map_or(Value::Null, |fields| {
            fields["currentPlanViewVIIndex"].clone()
        })
}

#[test]
fn an_item_moves_one_place_and_is_selected_when_the_change_is_announced() {
    let scratch = std::env::temp_dir().join(format!("plan-move-item-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    groundstation::settingsstore::open(&scratch.join("settings.ini"));
    let core = Core::new(Owner(Facade(NoHost)));
    [47.397, 47.398, 47.399].iter().for_each(|latitude| {
        assert_eq!(
            invoked(
                &core,
                "mission.insert",
                json!(["waypoint", latitude, 8.546, -1])
            )["ok"],
            true
        );
    });
    while groundstation::coreplan::home_ground_pending() {
        std::thread::yield_now();
    }
    let last = groundstation::coreplan::controller_fields("plan.missionController.visualItems")
        .and_then(|items| items["count"].as_i64())
        .unwrap()
        - 1;
    [json!([last, 2]), json!([last, 0]), json!([last, -3])]
        .into_iter()
        .for_each(|args| {
            assert_eq!(
                invoked(&core, "core.plan.moveItem", args.clone())["reason"],
                "Moving an item needs its index and a step of 1 or -1.",
                "{args} is not one place up or down"
            );
        });
    let announced = Arc::new(Mutex::new(Vec::new()));
    let heard = Arc::clone(&announced);
    *groundstation::coreplan::ON_CHANGE.lock().unwrap() = Some(Arc::new(move || {
        heard.lock().unwrap().push(selected());
    }));
    assert_eq!(
        invoked(&core, "core.plan.moveItem", json!([last, -1])),
        json!({ "ok": true, "index": last - 1 })
    );
    *groundstation::coreplan::ON_CHANGE.lock().unwrap() = None;
    assert_eq!(
        *announced.lock().unwrap(),
        vec![json!(last - 1)],
        "one announcement, made with the moved item already selected"
    );
    let _ = std::fs::remove_dir_all(scratch);
}
