use serde_json::{Value, json};

use crate::missionkinds::{default_area, default_line, insertable, lookup, refusal};
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
        // Refusing a kind and never having heard of it are different answers. QGC has item types
        // this catalogue does not list, and a head holding one needs to know it can insert it
        // directly rather than that the plan turned it down.
        return json!({ "ok": false, "unknown": named, "reason": format!("the core has no {named} in its catalogue, so this one has to be inserted directly") });
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
    if answered.get("ok").and_then(Value::as_bool) != Some(true) {
        return json!({ "ok": false, "reason": answered.get("reason").and_then(Value::as_str).unwrap_or("the plan refused the item").to_string() });
    }
    let Some(placed) = inserted_index(backend) else {
        return json!({ "ok": false, "reason": "the item was added and then could not be found, so the plan is not in a state to build on" });
    };
    match shape(backend, kind, placed, latitude, longitude) {
        Ok(()) => json!({ "ok": true, "inserted": kind.id, "index": placed, "atSequence": at_sequence }),
        Err(reason) => {
            backend.invoke("plan.missionController.removeVisualItem", &json!([placed]).to_string());
            json!({ "ok": false, "reason": reason, "removed": kind.id })
        }
    }
}

fn inserted_index(backend: &dyn Backend) -> Option<i64> {
    serde_json::from_str::<Value>(&backend.get("plan.missionController.currentPlanViewVIIndex")).ok().and_then(|v| v.get("value").and_then(Value::as_i64)).filter(|index| *index > 0)
}

// An item the plan draws with a shape is useless without one, and a takeoff that does not know where
// the vehicle launches from is worse than useless, so a shape that cannot be written takes the item
// with it rather than leaving a survey with no area for an operator to find later.
fn shape(backend: &dyn Backend, kind: &crate::missionkinds::Kind, index: i64, latitude: f64, longitude: f64) -> Result<(), String> {
    if kind.id == "takeoff" {
        let at = json!({ "latitude": latitude, "longitude": longitude, "altitude": 0.0 });
        let written: Value = serde_json::from_str(&backend.set(&format!("plan.missionController.visualItems.{index}.launchCoordinate"), &json!({ "value": at }).to_string())).unwrap_or(Value::Null);
        return match written.get("ok").and_then(Value::as_bool) {
            Some(true) => Ok(()),
            _ => Err("the takeoff would not take a launch position, and a takeoff without one cannot be flown".to_string()),
        };
    }
    let Some((geometry, property)) = kind.geometry else { return Ok(()) };
    let points = match geometry {
        "line" => default_line(latitude, longitude),
        _ => default_area(latitude, longitude),
    };
    let path = format!("plan.missionController.visualItems.{index}.{property}");
    backend.invoke(&format!("{path}.clear"), "[]");
    let refused = points.iter().find_map(|(lat, lon)| {
        let at = json!([{ "latitude": lat, "longitude": lon, "altitude": 0.0 }]);
        let answered: Value = serde_json::from_str(&backend.invoke(&format!("{path}.appendVertex"), &at.to_string())).unwrap_or(Value::Null);
        match answered.get("ok").and_then(Value::as_bool) {
            Some(true) => None,
            _ => Some(format!("the {} would not take a {}", kind.title.to_lowercase(), kind.shape_noun())),
        }
    });
    match refused {
        Some(reason) => Err(reason),
        None => Ok(()),
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
                "plan.missionController.currentPlanViewVIIndex" => json!({ "kind": "value", "value": 2 }).to_string(),
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
        fn set(&self, path: &str, value: &str) -> String {
            self.calls.lock().unwrap().push((path.to_string(), value.to_string()));
            self.answer.to_string()
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
    fn in_a_state_that_wants_a_takeoff_first_a_takeoff_is_the_one_thing_accepted() {
        // Each call is asked in the same state, because this stub's flags do not move. What a real
        // controller answers after a takeoff has gone in is a different question and belongs to a
        // test with a real controller behind it; asserting it here would pin the fixture.
        let plan = Plan::new(empty_ground_mission());
        assert_eq!(run(&plan, "mission.insert", "[\"takeoff\", 47.0, 8.0, -1]")["ok"], true);
        let refused = Plan::new(empty_ground_mission());
        assert_eq!(run(&refused, "mission.insert", "[\"land\", 47.0, 8.0, -1]")["ok"], false);
        assert_eq!(run(&refused, "mission.insert", "[\"survey\", 47.0, 8.0, -1]")["ok"], false);
        assert!(refused.calls.lock().unwrap().iter().all(|(path, _)| !path.contains("insert")));
    }

    #[test]
    fn a_kind_the_core_never_heard_of_says_so_rather_than_saying_no() {
        let plan = Plan::new(flying_mission());
        let unknown = run(&plan, "mission.insert", "[\"Fixed Wing Landing Pattern\", 47.0, 8.0, -1]");
        assert_eq!(unknown["ok"], false);
        assert_eq!(unknown["unknown"], "Fixed Wing Landing Pattern", "a head holding an item type this catalogue never listed has to be able to tell that apart from the plan turning it down, because the first means insert it yourself and the second means do not");
        assert!(unknown["reason"].as_str().unwrap().contains("catalogue"));
        assert!(plan.calls.lock().unwrap().is_empty(), "a kind the core does not know is refused before the plan view is even moved");

        let refused = run(&Plan::new(empty_ground_mission()), "mission.insert", "[\"survey\", 47.0, 8.0, -1]");
        assert_eq!(refused["ok"], false);
        assert_eq!(refused["unknown"], Value::Null, "a kind the core does know and is refusing carries no unknown marker");
        assert_eq!(refused["refused"], "survey");
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

    #[test]
    fn a_survey_arrives_with_an_area_around_where_it_was_placed() {
        let plan = Plan::new(flying_mission());
        assert_eq!(run(&plan, "mission.insert", "[\"survey\", 47.0, 8.0, -1]")["ok"], true);
        let calls = plan.calls.lock().unwrap();
        let path = "plan.missionController.visualItems.2.surveyAreaPolygon";
        assert_eq!(calls.iter().filter(|(called, _)| called == &format!("{path}.clear")).count(), 1);
        let vertices: Vec<&(String, String)> = calls.iter().filter(|(called, _)| called == &format!("{path}.appendVertex")).collect();
        assert_eq!(vertices.len(), 4, "a survey with no area draws nothing and uploads nothing, so the action gives it one");
        let first: Value = serde_json::from_str(&vertices[0].1).unwrap();
        assert!((first[0]["latitude"].as_f64().unwrap() - 47.0).abs() < 0.01, "the area is placed around where the operator tapped");
    }

    #[test]
    fn a_corridor_arrives_with_a_path_rather_than_an_area() {
        let plan = Plan::new(flying_mission());
        assert_eq!(run(&plan, "mission.insert", "[\"corridor\", 47.0, 8.0, -1]")["ok"], true);
        let calls = plan.calls.lock().unwrap();
        let vertices = calls.iter().filter(|(called, _)| called.ends_with("corridorPolyline.appendVertex")).count();
        assert_eq!(vertices, 2, "a corridor is a line to scan along, so two points rather than four");
    }

    #[test]
    fn a_takeoff_arrives_knowing_where_the_vehicle_launches_from() {
        let plan = Plan::new(empty_ground_mission());
        assert_eq!(run(&plan, "mission.insert", "[\"takeoff\", 47.25, 8.75, -1]")["ok"], true);
        let calls = plan.calls.lock().unwrap();
        let written = calls.iter().find(|(called, _)| called.ends_with(".launchCoordinate")).expect("the launch position was never written");
        let value: Value = serde_json::from_str(&written.1).unwrap();
        assert_eq!(value["value"]["latitude"], 47.25);
        assert_eq!(value["value"]["longitude"], 8.75);
    }

    #[test]
    fn an_item_that_will_not_take_its_shape_is_taken_out_again() {
        let mut plan = Plan::new(flying_mission());
        plan.answer = json!({ "ok": false, "reason": "no" });
        let refused = run(&plan, "mission.insert", "[\"survey\", 47.0, 8.0, -1]");
        assert_eq!(refused["ok"], false);
        let calls = plan.calls.lock().unwrap();
        assert!(calls.iter().all(|(called, _)| !called.ends_with("removeVisualItem")), "the insert itself failed, so there is nothing to remove");
    }

    #[test]
    fn a_survey_that_will_not_take_an_area_is_removed_rather_than_left_empty() {
        struct Fussy(Plan);
        impl Backend for Fussy {
            fn get(&self, path: &str) -> String { self.0.get(path) }
            fn get_fields(&self, path: &str, fields: &str) -> String { self.0.get_fields(path, fields) }
            fn set(&self, path: &str, value: &str) -> String { self.0.set(path, value) }
            fn invoke(&self, path: &str, args: &str) -> String {
                match path.ends_with("appendVertex") {
                    true => {
                        self.0.calls.lock().unwrap().push((path.to_string(), args.to_string()));
                        json!({ "ok": false, "reason": "no" }).to_string()
                    }
                    false => self.0.invoke(path, args),
                }
            }
            fn watch(&self, paths: &[String]) { self.0.watch(paths) }
        }
        let fussy = Fussy(Plan::new(flying_mission()));
        let refused = run(&fussy, "mission.insert", "[\"survey\", 47.0, 8.0, -1]");
        assert_eq!(refused["ok"], false);
        assert_eq!(refused["removed"], "survey");
        let calls = fussy.0.calls.lock().unwrap();
        assert!(calls.iter().any(|(called, _)| called.ends_with("removeVisualItem")), "a survey with no area is worse than no survey, because an operator has to find it to delete it");
    }
}
