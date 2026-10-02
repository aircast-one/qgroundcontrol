use std::sync::Mutex;
use std::sync::PoisonError;

use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const CREATE_FROM_TEMPLATE: &str = "plan.createFromTemplate";
pub const NO_TEMPLATE: &str = "No Template";
const TAKEOFF_SEQUENCE: i64 = 1;

#[derive(Default)]
struct Manual {
    chosen: bool,
    held_items: bool,
}

static MANUAL: Mutex<Manual> = Mutex::new(Manual { chosen: false, held_items: false });

pub fn show(contains_items: bool) -> bool {
    let mut manual = MANUAL.lock().unwrap_or_else(PoisonError::into_inner);
    if contains_items {
        manual.held_items = true;
    } else if manual.held_items {
        manual.held_items = false;
        manual.chosen = false;
    }
    !contains_items && !manual.chosen
}

fn choose_manual() {
    MANUAL.lock().unwrap_or_else(PoisonError::into_inner).chosen = true;
}

pub fn templates_json(backend: &dyn Backend, patterns: &[String], contains_items: bool) -> Value {
    let home_set = flag(&object(&backend.get_fields("plan.missionController", "homePositionSet")), "homePositionSet");
    json!({
        "show": show(contains_items),
        "enabled": home_set || crate::coreplan::enabled(),
        "prompt": if home_set { "Drag to move home position. Click to set new position." } else { "Click in map to set position" },
        "names": patterns.iter().map(String::as_str).chain(std::iter::once(NO_TEMPLATE)).collect::<Vec<_>>(),
    })
}

pub fn steps(name: &str, latitude: f64, longitude: f64) -> Vec<(&'static str, Value)> {
    let at = json!({ "latitude": latitude, "longitude": longitude });
    vec![
        ("plan.removeAll", json!([])),
        ("plan.missionController.insertTakeoffItem", json!([at, -1, false])),
        ("plan.missionController.insertComplexMissionItem", json!([name, at, -1, false])),
        ("plan.missionController.insertLandItem", json!([at, -1, false])),
        ("plan.missionController.setCurrentPlanViewSeqNum", json!([TAKEOFF_SEQUENCE, true])),
    ]
}

fn core_kind(name: &str) -> Option<&'static str> {
    match name {
        "Survey" => Some("survey"),
        "Corridor Scan" => Some("corridor"),
        "Structure Scan" => Some("structure"),
        _ => None,
    }
}

pub fn core_steps(name: &str, latitude: f64, longitude: f64) -> Vec<(&'static str, Value)> {
    let Some(kind) = core_kind(name) else { return Vec::new() };
    vec![
        ("plan.removeAll", json!([])),
        ("mission.insert", json!(["takeoff", latitude, longitude, -1])),
        ("mission.insert", json!([kind, latitude, longitude, -1])),
        ("mission.insert", json!(["land", latitude, longitude, -1])),
        ("plan.missionController.setCurrentPlanViewSeqNum", json!([TAKEOFF_SEQUENCE, true])),
    ]
}

pub fn create(backend: &dyn Backend, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let name = given.get(0).and_then(Value::as_str).unwrap_or_default();
    if name == NO_TEMPLATE {
        choose_manual();
        return json!({ "ok": true });
    }
    let coordinate = |i: usize, limit: f64| given.get(i).and_then(Value::as_f64).filter(|v| v.is_finite() && v.abs() <= limit);
    let (Some(latitude), Some(longitude)) = (coordinate(1, 90.0), coordinate(2, 180.0)) else {
        return json!({ "ok": false, "reason": "A template is placed at the map centre's latitude and longitude." });
    };
    let core = crate::coreplan::enabled();
    let planned = match core {
        true => core_steps(name, latitude, longitude),
        false => steps(name, latitude, longitude),
    };
    if core && planned.is_empty() {
        return json!({ "ok": false, "reason": format!("There is no {name} template.") });
    }
    let homeless = core && crate::coreplan::current_document().home.is_none();
    let failed = planned.into_iter().find_map(|(path, args)| {
        let answer = match core {
            true => crate::coreplan::route_invoke(backend, path, &args.to_string()).unwrap_or_else(|| json!({ "ok": false, "reason": "The core plan did not carry out the template." })),
            false => crate::actions::run(backend, path, &args.to_string()),
        };
        if homeless && path == "plan.removeAll" {
            crate::coreplan::route_set(backend, "plan.missionController.visualItems.0.coordinate", &json!({ "value": { "latitude": latitude, "longitude": longitude } }).to_string());
        }
        (!flag(&answer, "ok")).then(|| answer.get("reason").and_then(Value::as_str).unwrap_or("The plan refused the template.").to_string())
    });
    match failed {
        None => json!({ "ok": true }),
        Some(reason) => json!({ "ok": false, "reason": reason }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_clears_then_adds_takeoff_pattern_and_land_at_the_centre() {
        let paths: Vec<&str> = steps("Survey", 47.4, 8.5).into_iter().map(|(path, _)| path).collect();
        assert_eq!(paths, ["plan.removeAll", "plan.missionController.insertTakeoffItem", "plan.missionController.insertComplexMissionItem", "plan.missionController.insertLandItem", "plan.missionController.setCurrentPlanViewSeqNum"]);
        assert_eq!(steps("Survey", 47.4, 8.5)[2].1, json!(["Survey", { "latitude": 47.4, "longitude": 8.5 }, -1, false]));
        assert_eq!(steps("Survey", 47.4, 8.5)[4].1, json!([1, true]), "the takeoff is selected, as setCurrentPlanViewSeqNum(takeoff, true)");
        let core: Vec<Value> = core_steps("Corridor Scan", 47.4, 8.5).into_iter().map(|(_, args)| args).collect();
        assert_eq!(core[2], json!(["corridor", 47.4, 8.5, -1]), "the core plan inserts through its own mission.insert, so a template never reaches the absent Qt controller");
        assert!(core_steps("Bogus", 47.4, 8.5).is_empty(), "an unknown template name is refused rather than quietly becoming a survey");
    }

    #[test]
    fn no_template_hides_the_picker_until_the_plan_has_held_items_and_emptied_again() {
        *MANUAL.lock().unwrap() = Manual::default();
        assert!(show(false));
        choose_manual();
        assert!(!show(false), "the user chose to build by hand");
        assert!(!show(true));
        assert!(show(false), "clearing the plan brings the templates back, as _updateShowCreateFromTemplate does");
    }
}
