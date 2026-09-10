use serde_json::{Value, json};

use crate::missionkinds::{insertable, lookup, refusal};
use crate::router::Backend;

const INSERT: &str = "mission.insert";

pub fn owns(path: &str) -> bool {
    path == INSERT
}

pub fn run(backend: &dyn Backend, path: &str, args: &str) -> Value {
    match path {
        INSERT => insert(backend, args),
        _ => json!({ "ok": false, "reason": format!("{path} is not an action the core performs") }),
    }
}

// What may be inserted next is recomputed only when the plan view selects an item, so a head that
// never selects one reads whatever the controller was constructed with. The insert point is chosen
// here before the question is asked, which is the same thing the plan view does when a user clicks.
fn point_at(backend: &dyn Backend, index: i64) -> Option<i64> {
    let count = serde_json::from_str::<Value>(&backend.get("plan.missionController.visualItems.count")).ok().and_then(|v| v.get("value").and_then(Value::as_i64))?;
    if count <= 0 {
        return None;
    }
    let wanted = match index {
        index if index < 0 || index >= count => count - 1,
        index => index,
    };
    let sequence = serde_json::from_str::<Value>(&backend.get(&format!("plan.missionController.visualItems.{wanted}.sequenceNumber")))
        .ok()
        .and_then(|v| v.get("value").and_then(Value::as_i64))?;
    backend.invoke("plan.missionController.setCurrentPlanViewSeqNum", &json!([sequence, true]).to_string());
    Some(sequence)
}

fn insert(backend: &dyn Backend, args: &str) -> Value {
    let args: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let Some(args) = args.as_array() else {
        return json!({ "ok": false, "reason": "mission.insert takes a kind, a latitude, a longitude and an index" });
    };
    let number = |index: usize| args.get(index).and_then(Value::as_f64).filter(|value| value.is_finite());
    let (Some(named), Some(latitude), Some(longitude)) = (args.first().and_then(Value::as_str), number(1), number(2)) else {
        return json!({ "ok": false, "reason": "mission.insert takes a kind, a latitude, a longitude and an index" });
    };
    let Some(kind) = lookup(named) else {
        return json!({ "ok": false, "reason": format!("{named} is not a mission item this plan can hold") });
    };
    let index = args.get(3).and_then(Value::as_i64).unwrap_or(-1);
    let at_sequence = point_at(backend, index);
    if let Some(reason) = refusal(kind, &insertable(backend)) {
        return json!({ "ok": false, "reason": reason, "refused": kind.id, "atSequence": at_sequence });
    }
    let at = json!({ "latitude": latitude, "longitude": longitude, "altitude": 0.0 });
    // The controller recomputes what may be inserted next from whichever item the plan view has
    // selected, so an insert that does not select what it added leaves the next answer stale.
    let call: Vec<Value> = match kind.complex_name {
        Some(name) => vec![json!(name), at, json!(index), json!(true)],
        None => vec![at, json!(index), json!(true)],
    };
    let answered: Value = serde_json::from_str(&backend.invoke(&format!("plan.missionController.{}", kind.invokable), &Value::Array(call).to_string())).unwrap_or(Value::Null);
    match answered.get("ok").and_then(Value::as_bool) {
        Some(true) => json!({ "ok": true, "inserted": kind.id, "atSequence": at_sequence }),
        _ => json!({ "ok": false, "reason": answered.get("reason").and_then(Value::as_str).unwrap_or("the plan refused the item").to_string() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Plan {
        mission: Value,
        calls: Mutex<Vec<(String, String)>>,
        answer: Value,
    }

    impl Plan {
        fn new(mission: Value) -> Plan {
            Plan { mission, calls: Mutex::new(Vec::new()), answer: json!({ "ok": true }) }
        }
    }

    impl Backend for Plan {
        fn get(&self, path: &str) -> String {
            match path {
                "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": 3 }).to_string(),
                path if path.ends_with(".sequenceNumber") => json!({ "kind": "value", "value": 2 }).to_string(),
                _ => String::new(),
            }
        }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "plan.missionController" => self.mission.to_string(),
                _ => String::new(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, path: &str, args: &str) -> String {
            self.calls.lock().unwrap().push((path.to_string(), args.to_string()));
            self.answer.to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    fn empty_ground_mission() -> Value {
        json!({ "kind": "object", "onlyInsertTakeoffValid": true, "isInsertTakeoffValid": true, "isInsertLandValid": false, "flyThroughCommandsAllowed": true })
    }

    fn flying_mission() -> Value {
        json!({ "kind": "object", "onlyInsertTakeoffValid": false, "isInsertTakeoffValid": false, "isInsertLandValid": true, "flyThroughCommandsAllowed": true })
    }

    #[test]
    fn an_item_the_plan_has_decided_against_is_refused_before_it_is_inserted() {
        let plan = Plan::new(empty_ground_mission());
        let refused = run(&plan, "mission.insert", "[\"waypoint\", 47.0, 8.0, -1]");
        assert_eq!(refused["ok"], false);
        assert_eq!(refused["refused"], "waypoint");
        assert!(refused["reason"].as_str().unwrap().contains("takeoff"));
        let calls = plan.calls.lock().unwrap();
        assert!(calls.iter().all(|(path, _)| path.ends_with("setCurrentPlanViewSeqNum")), "the refusal has to happen before the plan is touched, or a head racing a stale view still gets its item in");
    }

    #[test]
    fn an_item_the_plan_allows_is_inserted_by_the_name_the_core_holds() {
        let plan = Plan::new(flying_mission());
        let inserted = run(&plan, "mission.insert", "[\"survey\", 47.5, 8.5, 3]");
        assert_eq!(inserted["ok"], true);
        assert_eq!(inserted["inserted"], "survey");
        let calls = plan.calls.lock().unwrap();
        assert_eq!(calls[0].0, "plan.missionController.setCurrentPlanViewSeqNum", "the insert point is selected before the question is asked, because the answer is about that point");
        assert_eq!(calls[1].0, "plan.missionController.insertComplexMissionItem");
        let sent: Value = serde_json::from_str(&calls[1].1).unwrap();
        assert_eq!(sent[0], "Survey", "the head names the kind and the core supplies the complex name, so a head never spells it");
        assert_eq!(sent[1]["latitude"], 47.5);
        assert_eq!(sent[2], 3);
        assert_eq!(sent[3], true, "the inserted item becomes the selected one, which is what makes the controller recompute what can be inserted next");
    }

    #[test]
    fn a_simple_item_is_called_without_a_complex_name() {
        let plan = Plan::new(flying_mission());
        assert_eq!(run(&plan, "mission.insert", "[\"waypoint\", 47.0, 8.0, -1]")["ok"], true);
        let calls = plan.calls.lock().unwrap();
        assert_eq!(calls[1].0, "plan.missionController.insertSimpleMissionItem");
        let sent: Value = serde_json::from_str(&calls[1].1).unwrap();
        assert_eq!(sent.as_array().unwrap().len(), 3, "a simple item takes a coordinate, an index, and the flag that selects it");
        assert_eq!(sent[0]["longitude"], 8.0);
        assert_eq!(sent[2], true);
    }

    #[test]
    fn a_takeoff_is_the_one_thing_an_empty_ground_mission_accepts() {
        let plan = Plan::new(empty_ground_mission());
        assert_eq!(run(&plan, "mission.insert", "[\"takeoff\", 47.0, 8.0, -1]")["ok"], true);
        assert_eq!(run(&plan, "mission.insert", "[\"land\", 47.0, 8.0, -1]")["ok"], false);
        assert_eq!(run(&plan, "mission.insert", "[\"survey\", 47.0, 8.0, -1]")["ok"], false);
        assert_eq!(plan.calls.lock().unwrap().iter().filter(|(path, _)| path.contains("insert")).count(), 1);
    }

    #[test]
    fn a_kind_that_does_not_exist_is_named_in_the_refusal() {
        let plan = Plan::new(flying_mission());
        let refused = run(&plan, "mission.insert", "[\"orbit\", 47.0, 8.0, -1]");
        assert_eq!(refused["ok"], false);
        assert!(refused["reason"].as_str().unwrap().contains("orbit"));
        assert!(plan.calls.lock().unwrap().is_empty(), "a kind the core does not know is refused before the plan view is even moved");
    }

    #[test]
    fn arguments_that_are_not_a_place_are_refused_rather_than_placed_at_zero() {
        let plan = Plan::new(flying_mission());
        ["[]", "[\"waypoint\"]", "[\"waypoint\", null, 8.0, -1]", "not json", "{}"].iter().for_each(|args| {
            assert_eq!(run(&plan, "mission.insert", args)["ok"], false, "{args} is not a place to put a mission item");
        });
        assert!(plan.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn a_plan_that_refuses_the_call_is_reported_rather_than_reported_as_inserted() {
        let mut plan = Plan::new(flying_mission());
        plan.answer = json!({ "ok": false, "reason": "the plan is syncing with the vehicle" });
        let refused = run(&plan, "mission.insert", "[\"waypoint\", 47.0, 8.0, -1]");
        assert_eq!(refused["ok"], false);
        assert_eq!(refused["reason"], "the plan is syncing with the vehicle");
    }

    #[test]
    fn the_core_only_claims_the_action_it_performs() {
        assert!(owns("mission.insert"));
        assert!(!owns("plan.missionController.insertSimpleMissionItem"));
        assert!(!owns("mission.insertion"));
        assert_eq!(run(&Plan::new(flying_mission()), "mission.remove", "[]")["ok"], false);
    }
}
