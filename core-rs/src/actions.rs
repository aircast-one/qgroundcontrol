use serde_json::{Value, json};

use crate::missionkinds::{default_area, default_line, insertable, lookup, refusal};
use crate::read::object;
use crate::router::Backend;

const INSERT: &str = "mission.insert";
const REMOVE: &str = "mission.remove";
const ORBIT: &str = "guided.orbit";

pub fn owns(path: &str) -> bool {
    matches!(path, INSERT | REMOVE | ORBIT)
}

pub fn run(backend: &dyn Backend, path: &str, args: &str) -> Value {
    match path {
        INSERT => insert(backend, args),
        REMOVE => remove(backend, args),
        ORBIT => orbit(backend, args),
        _ => json!({ "ok": false, "reason": format!("{path} is not an action the core performs") }),
    }
}

// What may be inserted next is recomputed only when the plan view selects an item, so a head that
// never selects one reads whatever the controller was constructed with. The insert point is chosen
// here before the question is asked, which is the same thing the plan view does when a user clicks.
// QML asks about the selected item and inserts after it, so the question belongs to the slot
// before the insertion point. Asking at the insertion point asks about the item that will be
// displaced, which is a different item with a different answer.
fn point_at(backend: &dyn Backend, index: i64) -> Option<i64> {
    let count = item_count(backend)?;
    if count <= 0 {
        return None;
    }
    let wanted = match index {
        index if index < 0 => count - 1,
        index => (index - 1).clamp(0, count - 1),
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
    let Some(held) = item_count(backend).filter(|count| *count > 0) else {
        return json!({ "ok": false, "reason": "the plan did not say how many items it holds" });
    };
    // Index 0 would put an item before the plan's own settings entry, which every later read of
    // visualItems[0] assumes is there. QmlObjectListModel::insert warns on an index past the end
    // and inserts anyway.
    if index >= 0 && (index < 1 || index > held) {
        return json!({ "ok": false, "reason": format!("this plan has no place {index} to put an item") });
    }
    let Some(at_sequence) = point_at(backend, index) else {
        return json!({ "ok": false, "reason": "the plan view has no item selected, so there is no point to insert against" });
    };
    let at_sequence = Some(at_sequence);
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
    // These invokables return void, so the bridge answers ok whether or not anything was added.
    // Without counting, a failed insert leaves the selection on an item the operator already had,
    // and the rollback below would delete it.
    if item_count(backend) != Some(held + 1) {
        return json!({ "ok": false, "reason": "the plan did not grow, so nothing was added" });
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

// Index 0 is the mission settings item, which holds the planned home position and is not something
// an operator deletes; the plan view offers no way to. Removing it leaves a plan the controller
// cannot describe rather than a shorter one.
const SETTINGS_ITEM: i64 = 0;

fn remove(backend: &dyn Backend, args: &str) -> Value {
    let args: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let Some(index) = args.as_array().and_then(|args| args.first()).and_then(Value::as_i64) else {
        return json!({ "ok": false, "reason": "mission.remove takes the index of the item to remove" });
    };
    let Some(count) = item_count(backend) else {
        return json!({ "ok": false, "reason": "the plan did not say how many items it holds" });
    };
    if index == SETTINGS_ITEM {
        return json!({ "ok": false, "reason": "The first entry holds the plan's own settings and cannot be removed." });
    }
    if index < 0 || index >= count {
        return json!({ "ok": false, "reason": format!("this plan has no item {index}") });
    }
    backend.invoke("plan.missionController.removeVisualItem", &json!([index]).to_string());
    match item_count(backend) {
        Some(now) if now == count - 1 => json!({ "ok": true, "removed": index, "remaining": now }),
        Some(now) if now < count - 1 => json!({ "ok": false, "reason": format!("the plan lost {} items rather than the one asked for", count - now) }),
        _ => json!({ "ok": false, "reason": "the plan still holds the item, so it was not removed" }),
    }
}

// guidedModeOrbit takes a radius whose sign is the turn direction and an altitude above sea level.
// A head asked to compose that is holding two pieces of vehicle knowledge it has no way to check,
// and the macOS head sent zero for both to a flying aircraft. It says where, how wide, which way
// round and how far above the launch point; the sign and the sea level conversion happen here.
fn orbit(backend: &dyn Backend, args: &str) -> Value {
    let args: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let Some(args) = args.as_array() else {
        return json!({ "ok": false, "reason": "guided.orbit takes a latitude, a longitude, a radius, a direction and a height above the launch point" });
    };
    let number = |index: usize| args.get(index).and_then(Value::as_f64).filter(|value| value.is_finite());
    let (Some(latitude), Some(longitude), Some(radius), Some(above_home)) = (number(0), number(1), number(2), number(4)) else {
        return json!({ "ok": false, "reason": "guided.orbit takes a latitude, a longitude, a radius, a direction and a height above the launch point" });
    };
    if radius <= 0.0 {
        return json!({ "ok": false, "reason": "An orbit needs a radius to fly around." });
    }
    let Some(clockwise) = args.get(3).and_then(Value::as_bool) else {
        return json!({ "ok": false, "reason": "An orbit has to turn one way or the other." });
    };
    if !crate::read::flag(&object(&backend.get_fields("vehicle", "orbitModeSupported")), "orbitModeSupported") {
        return json!({ "ok": false, "reason": "This vehicle does not support orbiting." });
    }
    let limit = |name: &str| crate::read::value_number(&backend.get(&format!("settings.flyViewSettings.{name}.rawValue")));
    match (limit("guidedMinimumAltitude"), limit("guidedMaximumAltitude")) {
        (Some(lowest), Some(highest)) if above_home < lowest || above_home > highest => {
            return json!({ "ok": false, "reason": format!("An orbit has to be between {lowest} and {highest} metres above the launch point.") });
        }
        _ => {}
    }
    let home = object(&backend.get("vehicle.homePosition"));
    if home.get("valid").and_then(Value::as_bool) != Some(true) {
        return json!({ "ok": false, "reason": "The vehicle has not reported where it launched from, so there is nothing to measure the orbit height against." });
    }
    let Some(home_altitude) = home.get("altitude").and_then(Value::as_f64).filter(|value| value.is_finite()) else {
        return json!({ "ok": false, "reason": "The launch position carries no altitude, so an orbit height above sea level cannot be worked out." });
    };
    let signed = if clockwise { radius } else { -radius };
    let amsl = home_altitude + above_home;
    let called: Value = serde_json::from_str(&backend.invoke(
        "vehicle.guidedModeOrbit",
        &json!([{ "latitude": latitude, "longitude": longitude, "altitude": 0.0 }, signed, amsl]).to_string(),
    ))
    .unwrap_or(Value::Null);
    match called.get("ok").and_then(Value::as_bool) {
        Some(true) => json!({ "ok": true, "radius": signed, "altitudeAmsl": amsl }),
        _ => json!({ "ok": false, "reason": called.get("reason").and_then(Value::as_str).unwrap_or("the vehicle refused the orbit").to_string() }),
    }
}

fn item_count(backend: &dyn Backend) -> Option<i64> {
    serde_json::from_str::<Value>(&backend.get("plan.missionController.visualItems.count")).ok().and_then(|v| v.get("value").and_then(Value::as_i64))
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
        count: Mutex<i64>,
    }

    impl Plan {
        fn new(mission: Value) -> Plan {
            Plan { mission, calls: Mutex::new(Vec::new()), answer: json!({ "ok": true }), count: Mutex::new(3) }
        }
    }

    impl Backend for Plan {
        fn get(&self, path: &str) -> String {
            match path {
                "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": *self.count.lock().unwrap() }).to_string(),
                "plan.missionController.currentPlanViewVIIndex" => json!({ "kind": "value", "value": *self.count.lock().unwrap() - 1 }).to_string(),
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
            // These invokables return void and the bridge answers ok either way, so a stub that
            // never grows the plan is a stub of a plan that never accepts anything.
            if path.contains("insert") && self.answer.get("ok").and_then(Value::as_bool) == Some(true) {
                *self.count.lock().unwrap() += 1;
            }
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
    fn an_insert_that_did_not_grow_the_plan_never_reaches_the_rollback() {
        struct Deaf(Plan);
        impl Backend for Deaf {
            fn get(&self, path: &str) -> String { self.0.get(path) }
            fn get_fields(&self, path: &str, fields: &str) -> String { self.0.get_fields(path, fields) }
            fn set(&self, path: &str, value: &str) -> String { self.0.set(path, value) }
            fn invoke(&self, path: &str, args: &str) -> String {
                self.0.calls.lock().unwrap().push((path.to_string(), args.to_string()));
                json!({ "ok": true }).to_string()
            }
            fn watch(&self, paths: &[String]) { self.0.watch(paths) }
        }
        let deaf = Deaf(Plan::new(flying_mission()));
        let refused = run(&deaf, "mission.insert", "[\"survey\", 47.0, 8.0, -1]");
        assert_eq!(refused["ok"], false);
        assert!(refused["reason"].as_str().unwrap().contains("did not grow"));
        let calls = deaf.0.calls.lock().unwrap();
        assert!(calls.iter().all(|(called, _)| !called.ends_with("removeVisualItem")), "rolling back here would delete an item the operator already had");
        assert!(calls.iter().all(|(called, _)| !called.ends_with("appendVertex")), "and shaping here would draw over one");
    }

    #[test]
    fn a_place_the_plan_does_not_have_is_refused_before_anything_is_touched() {
        let plan = Plan::new(flying_mission());
        ["[\"waypoint\", 47.0, 8.0, 0]", "[\"waypoint\", 47.0, 8.0, 99]", "[\"waypoint\", 47.0, 8.0, 4]"]
            .iter()
            .for_each(|args| assert_eq!(run(&plan, "mission.insert", args)["ok"], false, "{args}"));
        let calls = plan.calls.lock().unwrap();
        assert!(calls.iter().all(|(called, _)| !called.contains("insert")), "index zero would put an item before the plan's own settings entry, and an index past the end the model inserts anyway with only a warning");
    }

    #[test]
    fn the_question_is_asked_about_the_slot_the_item_will_follow() {
        let plan = Plan::new(flying_mission());
        assert_eq!(run(&plan, "mission.insert", "[\"waypoint\", 47.0, 8.0, 2]")["ok"], true);
        let selected: Vec<i64> = plan
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(path, _)| path.ends_with("setCurrentPlanViewSeqNum"))
            .map(|(_, args)| serde_json::from_str::<Value>(args).unwrap()[0].as_i64().unwrap())
            .collect();
        assert_eq!(selected.len(), 1, "the plan view selects an item and inserts after it, so an insert at place two is a question about place one");
    }

    #[test]
    fn an_orbit_height_outside_what_the_operator_can_choose_is_refused() {
        let flying = orbiting::Flying::launched(480.0);
        ["[47.5, 8.5, 150.0, true, -500.0]", "[47.5, 8.5, 150.0, true, 0.0]", "[47.5, 8.5, 150.0, true, 5000.0]"]
            .iter()
            .for_each(|args| {
                let refused = run(&flying, "guided.orbit", args);
                assert_eq!(refused["ok"], false, "{args} is outside the range the slider offers");
                assert!(refused["reason"].as_str().unwrap().contains("above the launch point"));
            });
        assert!(flying.calls.lock().unwrap().is_empty(), "a height of minus five hundred metres is five hundred metres into the ground");
        assert_eq!(run(&flying, "guided.orbit", "[47.5, 8.5, 150.0, true, 60.0]")["ok"], true);
    }

    #[test]
    fn a_vehicle_that_cannot_orbit_is_not_asked_to() {
        let mut unable = orbiting::Flying::launched(480.0);
        unable.supported = false;
        let refused = run(&unable, "guided.orbit", "[47.5, 8.5, 150.0, true, 60.0]");
        assert_eq!(refused["ok"], false);
        assert!(refused["reason"].as_str().unwrap().contains("does not support"));
        assert!(unable.calls.lock().unwrap().is_empty(), "guidedModeOrbit returns void and shows a dialog, so asking anyway would report success while nothing was sent");
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
        let path = "plan.missionController.visualItems.3.surveyAreaPolygon";
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
    fn an_insert_the_plan_turned_down_leaves_nothing_to_roll_back() {
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

#[cfg(test)]
mod removal {
    use super::*;
    use std::sync::Mutex;

    struct Shrinking {
        count: Mutex<i64>,
        calls: Mutex<Vec<String>>,
    }

    impl Shrinking {
        fn holding(count: i64) -> Shrinking {
            Shrinking { count: Mutex::new(count), calls: Mutex::new(Vec::new()) }
        }
    }

    impl Backend for Shrinking {
        fn get(&self, path: &str) -> String {
            match path {
                "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": *self.count.lock().unwrap() }).to_string(),
                _ => String::new(),
            }
        }
        fn get_fields(&self, _p: &str, _f: &str) -> String { String::new() }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, path: &str, args: &str) -> String {
            self.calls.lock().unwrap().push(format!("{path} {args}"));
            if path.ends_with("removeVisualItem") {
                *self.count.lock().unwrap() -= 1;
            }
            json!({ "ok": true }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn an_item_is_removed_and_the_plan_says_what_is_left() {
        let plan = Shrinking::holding(4);
        let gone = run(&plan, "mission.remove", "[2]");
        assert_eq!(gone["ok"], true);
        assert_eq!(gone["removed"], 2);
        assert_eq!(gone["remaining"], 3);
        assert_eq!(plan.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn the_plans_own_settings_entry_is_not_an_item_an_operator_can_delete() {
        let plan = Shrinking::holding(4);
        let refused = run(&plan, "mission.remove", "[0]");
        assert_eq!(refused["ok"], false);
        assert!(refused["reason"].as_str().unwrap().contains("settings"));
        assert!(plan.calls.lock().unwrap().is_empty(), "the plan view offers no way to remove it, and removing it leaves a plan the controller cannot describe");
        assert_eq!(*plan.count.lock().unwrap(), 4);
    }

    #[test]
    fn an_index_the_plan_does_not_hold_is_refused_rather_than_passed_on() {
        let plan = Shrinking::holding(3);
        ["[3]", "[99]", "[-1]", "[]", "not json"].iter().for_each(|args| {
            assert_eq!(run(&plan, "mission.remove", args)["ok"], false, "{args} names no item");
        });
        assert!(plan.calls.lock().unwrap().is_empty());
        assert_eq!(*plan.count.lock().unwrap(), 3);
    }

    #[test]
    fn a_removal_that_did_not_shrink_the_plan_is_reported_as_a_failure() {
        struct Stubborn;
        impl Backend for Stubborn {
            fn get(&self, path: &str) -> String {
                match path {
                    "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": 3 }).to_string(),
                    _ => String::new(),
                }
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String { String::new() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true }).to_string() }
            fn watch(&self, _p: &[String]) {}
        }
        let refused = run(&Stubborn, "mission.remove", "[1]");
        assert_eq!(refused["ok"], false, "the call answered ok and the item is still there, and a head told ok would redraw a list that has not changed");
        assert!(refused["reason"].as_str().unwrap().contains("still holds"));
    }
}

#[cfg(test)]
pub(super) mod orbiting {
    use super::*;
    use std::sync::Mutex;

    pub(in crate::actions) struct Flying {
        home: Value,
        pub(in crate::actions) calls: Mutex<Vec<String>>,
        answer: Value,
        pub(in crate::actions) supported: bool,
    }

    impl Flying {
        pub(in crate::actions) fn launched(altitude: f64) -> Flying {
            Flying {
                home: json!({ "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0, "altitude": altitude }),
                calls: Mutex::new(Vec::new()),
                answer: json!({ "ok": true }),
                supported: true,
            }
        }
    }

    impl Backend for Flying {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.homePosition" => self.home.to_string(),
                "settings.flyViewSettings.guidedMinimumAltitude.rawValue" => json!({ "kind": "value", "value": 2.0 }).to_string(),
                "settings.flyViewSettings.guidedMaximumAltitude.rawValue" => json!({ "kind": "value", "value": 121.92 }).to_string(),
                _ => String::new(),
            }
        }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "vehicle" => json!({ "kind": "object", "orbitModeSupported": self.supported }).to_string(),
                _ => String::new(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, path: &str, args: &str) -> String {
            self.calls.lock().unwrap().push(format!("{path} {args}"));
            self.answer.to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    fn sent(plan: &Flying) -> Value {
        let calls = plan.calls.lock().unwrap();
        let (_, args) = calls[0].split_once(' ').unwrap();
        serde_json::from_str(args).unwrap()
    }

    #[test]
    fn the_direction_of_the_turn_is_the_sign_of_the_radius() {
        let clockwise = Flying::launched(480.0);
        assert_eq!(run(&clockwise, "guided.orbit", "[47.5, 8.5, 150.0, true, 60.0]")["ok"], true);
        assert_eq!(sent(&clockwise)[1], 150.0);

        let widdershins = Flying::launched(480.0);
        assert_eq!(run(&widdershins, "guided.orbit", "[47.5, 8.5, 150.0, false, 60.0]")["ok"], true);
        assert_eq!(sent(&widdershins)[1], -150.0, "a negative radius is how this command says anticlockwise, which is not something a head should have to know");
    }

    #[test]
    fn the_height_is_measured_from_the_launch_point_and_sent_above_sea_level() {
        let flying = Flying::launched(480.0);
        let started = run(&flying, "guided.orbit", "[47.5, 8.5, 150.0, true, 60.0]");
        assert_eq!(started["altitudeAmsl"], 540.0);
        assert_eq!(sent(&flying)[2], 540.0, "the operator chooses a height above where it took off, and the command wants sea level");
        assert_eq!(sent(&flying)[0]["latitude"], 47.5);
    }

    #[test]
    fn an_orbit_is_refused_when_there_is_nothing_to_measure_it_against() {
        let unlaunched = Flying {
            home: json!({ "kind": "coordinate", "valid": false, "latitude": 0.0, "longitude": 0.0, "altitude": 0.0 }),
            calls: Mutex::new(Vec::new()),
            answer: json!({ "ok": true }),
            supported: true,
        };
        let refused = run(&unlaunched, "guided.orbit", "[47.5, 8.5, 150.0, true, 60.0]");
        assert_eq!(refused["ok"], false);
        assert!(refused["reason"].as_str().unwrap().contains("launched from"));
        assert!(unlaunched.calls.lock().unwrap().is_empty(), "sending an orbit measured against an unknown launch height is sending an altitude nobody chose");
    }

    #[test]
    fn an_orbit_with_no_radius_or_no_direction_is_refused_rather_than_guessed() {
        let flying = Flying::launched(480.0);
        ["[47.5, 8.5, 0.0, true, 60.0]", "[47.5, 8.5, -20.0, true, 60.0]", "[47.5, 8.5, 150.0, null, 60.0]", "[47.5, 8.5, 150.0, true]", "[]"]
            .iter()
            .for_each(|args| assert_eq!(run(&flying, "guided.orbit", args)["ok"], false, "{args}"));
        assert!(flying.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn a_vehicle_that_refuses_the_orbit_is_reported_rather_than_reported_as_started() {
        let mut flying = Flying::launched(480.0);
        flying.answer = json!({ "ok": false, "reason": "not in guided mode" });
        let refused = run(&flying, "guided.orbit", "[47.5, 8.5, 150.0, true, 60.0]");
        assert_eq!(refused["ok"], false);
        assert_eq!(refused["reason"], "not in guided mode");
    }
}
