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
}
