use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

// MissionItemEditor.qml hands MissionCommandDialog masterController.controllerVehicle, which the plan
// always has - offline it is the vehicle being planned for. The macOS head passed @vehicle instead,
// the ACTIVE vehicle, which differs from the plan's while planning offline or for another airframe,
// and with nothing connected does not resolve at all: the bridge then answers {"ok":false} with no
// reason and the picker drew empty. MissionCommandTree::_firmwareAndVehicleClassInfo also
// dereferences the vehicle it is given without a null check, so the plan's vehicle is not only the
// faithful argument but the only one that is always safe to pass.
const PLAN_VEHICLE: &str = "plan.controllerVehicle";

fn refused(token: &str, reason: &str) -> Value {
    json!({ "ok": false, "result": Value::Null, "refusal": token, "reason": reason })
}

fn plan_vehicle_ready(backend: &dyn Backend) -> bool {
    object(&backend.get_fields(PLAN_VEHICLE, "firmwareTypeString")).get("kind").and_then(Value::as_str) == Some("object")
}

fn listed(backend: &dyn Backend, path: &str, args: Value, what: &str) -> Value {
    let answer = object(&backend.invoke(path, &args.to_string()));
    let result = answer.get("result").filter(|r| r.is_array()).cloned();
    let took = flag(&answer, "ok") && result.is_some();
    json!({
        "ok": took,
        "result": result.unwrap_or(Value::Null),
        "refusal": Value::Null,
        "reason": match took { true => Value::Null, false => json!(format!("The command tree did not list the {what}.")) },
    })
}

pub fn categories(backend: &dyn Backend, path: &str) -> Value {
    if !plan_vehicle_ready(backend) {
        return refused("noPlan", "There is no plan to list mission commands for.");
    }
    listed(backend, path, json!([format!("@{PLAN_VEHICLE}")]), "categories")
}

pub fn commands(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let Some(category) = given.get(1).and_then(Value::as_str).filter(|c| !c.trim().is_empty()) else {
        return refused("noCategory", "Name the category to list, as categoriesForVehicle spelled it.");
    };
    let fly_through = match given.get(2) {
        None | Some(Value::Null) => true,
        Some(Value::Bool(b)) => *b,
        Some(_) => return refused("malformed", "Whether to list fly-through commands is true or false."),
    };
    if !plan_vehicle_ready(backend) {
        return refused("noPlan", "There is no plan to list mission commands for.");
    }
    listed(backend, path, json!([format!("@{PLAN_VEHICLE}"), category, fly_through]), "commands")
}

// SimpleMissionItem::setCommand takes any integer, so a command written to an item was stored whether
// or not the vehicle it is planned for flies it - and one it does not fly is dropped by the autopilot
// on upload, or rejected there, far from the edit that caused it. The core accepts only a command the
// plan's own command tree lists, the list the picker was drawn from, and only on an item that has a
// command to change (the planned home and the complex items have none).
const ITEM_COMMANDS: &str = "plan.missionController.visualItems.";

pub fn command_target(path: &str) -> Option<usize> {
    path.strip_prefix(ITEM_COMMANDS)?.strip_suffix(".command")?.parse().ok()
}

fn offered_commands(backend: &dyn Backend) -> Option<Vec<i64>> {
    let at = json!([format!("@{PLAN_VEHICLE}")]).to_string();
    let categories = object(&backend.invoke("missionCommandTree.categoriesForVehicle", &at)).get("result")?.as_array()?.iter().filter_map(|c| c.as_str().map(str::to_string)).collect::<Vec<_>>();
    let mut commands: Vec<i64> = categories
        .iter()
        .filter_map(|category| {
            let asked = json!([format!("@{PLAN_VEHICLE}"), category, true]).to_string();
            object(&backend.invoke("missionCommandTree.getCommandsForCategory", &asked)).get("result").and_then(Value::as_array).cloned()
        })
        .flatten()
        .filter_map(|c| c.get("command").and_then(Value::as_i64))
        .collect();
    commands.sort_unstable();
    commands.dedup();
    (!commands.is_empty()).then_some(commands)
}

pub fn write_command(backend: &dyn Backend, path: &str, value: &str) -> Value {
    let refused = |token: &str, reason: String| json!({ "ok": false, "result": false, "refusal": token, "reason": reason });
    let Some(index) = command_target(path) else {
        return refused("malformed", "Name the item as plan.missionController.visualItems.<index>.command.".to_string());
    };
    if flag(&object(&backend.get_fields("plan", "syncInProgress")), "syncInProgress") {
        return refused("busy", "Wait for the sync to finish before changing the plan.".to_string());
    }
    let item = object(&backend.get_fields(&format!("{ITEM_COMMANDS}{index}"), "command"));
    if item.get("command").and_then(Value::as_i64).is_none() {
        return refused("noCommand", format!("Item {index} has no command to change."));
    }
    let Some(asked) = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value")?.as_i64()) else {
        return refused("malformed", "A command is its MAV_CMD number.".to_string());
    };
    if !plan_vehicle_ready(backend) {
        return refused("noPlan", "There is no plan to change a command in.".to_string());
    }
    match offered_commands(backend) {
        None => return refused("unanswered", "The command tree did not list this vehicle's commands.".to_string()),
        Some(offered) if !offered.contains(&asked) => return refused("notOffered", format!("Command {asked} is not one this vehicle flies in a mission.")),
        Some(_) => {}
    }
    let answered = flag(&object(&backend.set(path, &json!({ "value": asked }).to_string())), "ok");
    json!({ "ok": answered, "result": answered, "refusal": Value::Null, "reason": match answered { true => Value::Null, false => json!("The item did not take the command.") } })
}

// SimpleMissionItem copies this hint into param5 and param6 when a command without a coordinate is
// changed to one with, so a point off the globe became the item's position on the next command
// change, unchecked. It is refused here, and only an item with a command takes a hint.
pub fn hint_target(path: &str) -> Option<usize> {
    path.strip_prefix(ITEM_COMMANDS)?.strip_suffix(".setMapCenterHintForCommandChange")?.parse().ok()
}

pub fn hint(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let refused = |token: &str, reason: String| json!({ "ok": false, "refusal": token, "reason": reason });
    let Some(index) = hint_target(path) else {
        return refused("malformed", "Name the item as plan.missionController.visualItems.<index>.".to_string());
    };
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let Some((latitude, longitude)) = crate::fenceedit::point(given.get(0)) else {
        return refused("badCoordinate", "The map centre needs a latitude from -90 to 90 and a longitude from -180 to 180.".to_string());
    };
    let item = object(&backend.get_fields(&format!("{ITEM_COMMANDS}{index}"), "command"));
    if item.get("command").and_then(Value::as_i64).is_none() {
        return refused("noCommand", format!("Item {index} has no command to change."));
    }
    let dispatched = flag(&object(&backend.invoke(path, &json!([{ "latitude": latitude, "longitude": longitude }]).to_string())), "ok");
    json!({ "ok": dispatched, "refusal": Value::Null, "reason": match dispatched { true => Value::Null, false => json!("The item did not take the hint.") } })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Tree {
        plan: bool,
        calls: RefCell<Vec<(String, String)>>,
    }

    impl Backend for Tree {
        fn get(&self, p: &str) -> String { self.get_fields(p, "") }
        fn get_fields(&self, _p: &str, _f: &str) -> String {
            match self.plan {
                true => json!({ "kind": "object", "firmwareTypeString": "PX4 Pro" }),
                false => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, p: &str, a: &str) -> String {
            self.calls.borrow_mut().push((p.to_string(), a.to_string()));
            json!({ "ok": true, "result": ["Basic", "Advanced"] }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_command_tree_is_asked_about_the_plans_vehicle_as_qgc_asks_it() {
        let tree = Tree { plan: true, calls: RefCell::new(Vec::new()) };
        let listed = categories(&tree, "missionCommandTree.categoriesForVehicle");
        assert_eq!((&listed["ok"], &listed["result"]), (&json!(true), &json!(["Basic", "Advanced"])), "the head reads result as a list, so the claim keeps Qt's key");
        let _ = commands(&tree, "missionCommandTree.getCommandsForCategory", r#"["@vehicle","Basic",true]"#);
        assert_eq!(
            tree.calls.borrow().as_slice(),
            &[
                ("missionCommandTree.categoriesForVehicle".to_string(), r#"["@plan.controllerVehicle"]"#.to_string()),
                ("missionCommandTree.getCommandsForCategory".to_string(), r#"["@plan.controllerVehicle","Basic",true]"#.to_string()),
            ],
            "@vehicle is the active vehicle, not the one being planned for, and does not resolve at all offline"
        );
        assert_eq!(commands(&tree, "missionCommandTree.getCommandsForCategory", r#"["@vehicle",""]"#)["refusal"], "noCategory");
        assert_eq!(commands(&tree, "missionCommandTree.getCommandsForCategory", r#"["@vehicle","Basic","yes"]"#)["refusal"], "malformed");

        let none = Tree { plan: false, calls: RefCell::new(Vec::new()) };
        assert_eq!(categories(&none, "missionCommandTree.categoriesForVehicle")["refusal"], "noPlan");
        assert!(none.calls.borrow().is_empty(), "nothing reaches a command tree that would dereference a vehicle it was not given");
    }

    struct Plan(RefCell<Vec<String>>);
    impl Backend for Plan {
        fn get(&self, p: &str) -> String { self.get_fields(p, "") }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "plan" => json!({ "kind": "object", "syncInProgress": false }),
                "plan.controllerVehicle" => json!({ "kind": "object", "firmwareTypeString": "PX4 Pro" }),
                "plan.missionController.visualItems.0" => json!({ "kind": "object" }),
                "plan.missionController.visualItems.2" => json!({ "kind": "object", "command": 16 }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn set(&self, p: &str, _v: &str) -> String {
            self.0.borrow_mut().push(p.to_string());
            json!({ "ok": true }).to_string()
        }
        fn invoke(&self, p: &str, a: &str) -> String {
            match p {
                "missionCommandTree.categoriesForVehicle" => json!({ "ok": true, "result": ["Basic", "Loiter"] }),
                _ if a.contains("Basic") => json!({ "ok": true, "result": [{ "command": 16 }, { "command": 21 }] }),
                _ => json!({ "ok": true, "result": [{ "command": 19 }] }),
            }
            .to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn an_item_takes_only_a_command_its_vehicle_flies() {
        let plan = Plan(RefCell::new(Vec::new()));
        let path = "plan.missionController.visualItems.2.command";
        assert!(crate::actions::owns_write(path));
        assert_eq!(command_target(path), Some(2));
        assert_eq!(write_command(&plan, path, r#"{"value":19}"#)["ok"], true, "loiter time is listed under a second category");
        assert_eq!(write_command(&plan, path, r#"{"value":31000}"#)["refusal"], "notOffered", "SimpleMissionItem::setCommand stores any integer");
        assert_eq!(write_command(&plan, "plan.missionController.visualItems.0.command", r#"{"value":16}"#)["refusal"], "noCommand", "the planned home has no command");
        assert_eq!(write_command(&plan, path, r#"{"value":"16"}"#)["refusal"], "malformed");
        assert_eq!(plan.0.borrow().as_slice(), &[path.to_string()]);
    }

    #[test]
    fn a_map_centre_hint_is_a_real_place_given_to_an_item_with_a_command() {
        let plan = Plan(RefCell::new(Vec::new()));
        let path = "plan.missionController.visualItems.2.setMapCenterHintForCommandChange";
        assert_eq!(hint_target(path), Some(2));
        assert!(crate::actions::owns(path));
        assert_eq!(hint(&plan, path, r#"[{"latitude":47.39,"longitude":8.54}]"#)["ok"], true);
        assert_eq!(hint(&plan, path, r#"[{"latitude":147.39,"longitude":8.54}]"#)["refusal"], "badCoordinate", "the hint becomes param5 and param6 on the next command change");
        assert_eq!(hint(&plan, "plan.missionController.visualItems.0.setMapCenterHintForCommandChange", r#"[{"latitude":47.39,"longitude":8.54}]"#)["refusal"], "noCommand");
    }
}
