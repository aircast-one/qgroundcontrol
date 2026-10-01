use serde_json::{Value, json};

use crate::cmdinfo::{Command, Firmware, VehicleClass};
use crate::read::{flag, object};
use crate::router::Backend;

const PLAN_VEHICLE: &str = "plan.controllerVehicle";
const ALL_COMMANDS: &str = "All commands";
const CONDITION_GATE: i64 = 4501;
const ITEM_COMMANDS: &str = "plan.missionController.visualItems.";

fn refused(token: &str, reason: &str) -> Value {
    json!({ "ok": false, "result": Value::Null, "refusal": token, "reason": reason })
}

fn answered(result: Value) -> Value {
    json!({ "ok": true, "result": result, "refusal": Value::Null, "reason": Value::Null })
}

fn planned_for(backend: &dyn Backend) -> Option<(Firmware, VehicleClass)> {
    let vehicle = object(&backend.get_fields(PLAN_VEHICLE, "apmFirmware,px4Firmware,fixedWing,multiRotor,vtol,rover,sub"));
    (vehicle.get("kind").and_then(Value::as_str) == Some("object")).then_some(())?;
    let firmware = match (flag(&vehicle, "apmFirmware"), flag(&vehicle, "px4Firmware")) {
        (true, _) => Firmware::ArduPilot,
        (false, true) => Firmware::Px4,
        _ => Firmware::Generic,
    };
    let class = [("fixedWing", VehicleClass::FixedWing), ("multiRotor", VehicleClass::MultiRotor), ("vtol", VehicleClass::Vtol), ("rover", VehicleClass::Rover), ("sub", VehicleClass::Sub)]
        .into_iter()
        .find(|(key, _)| flag(&vehicle, key))
        .map_or(VehicleClass::Generic, |(_, class)| class);
    Some((firmware, class))
}

pub fn supported_commands(firmware: Firmware, class: VehicleClass, condition_gate: bool) -> Vec<i64> {
    let base: &[i64] = match firmware {
        Firmware::ArduPilot => &[16, 17, 18, 19, 20, 30, 31, 82, 92, 93, 112, 114, 115, 176, 177, 178, 179, 181, 182, 183, 184, 189, 201, 202, 203, 205, 1000, 206, 2000, 2001, 2500, 2501, 207, 208, 210, 211, 222, 212],
        Firmware::Px4 => &[16, 17, 19, 20, 177, 203, 206, 183, 187, 178, 179, 189, 195, 196, 197, 204, 205, 530, 2000, 2001, 2500, 2501, 93, 115, 31, 211],
        Firmware::Generic => return Vec::new(),
    };
    let vtol: &[i64] = match class {
        VehicleClass::Generic | VehicleClass::Vtol => &[84, 85, 3000],
        _ => &[],
    };
    let flight: &[i64] = match class {
        VehicleClass::Generic | VehicleClass::Vtol | VehicleClass::FixedWing | VehicleClass::MultiRotor => &[21, 22],
        _ => &[],
    };
    let gate: &[i64] = if condition_gate { &[CONDITION_GATE] } else { &[] };
    [base, vtol, flight, gate].concat()
}

fn condition_gate() -> bool {
    crate::settingsstore::raw_setting("settings.planViewSettings.useConditionGate").and_then(|v| v.as_bool()).unwrap_or(false)
}

fn offered(firmware: Firmware, class: VehicleClass) -> Vec<Command> {
    let plugin_for_class = !(firmware == Firmware::ArduPilot && class == VehicleClass::Generic);
    let supported = if plugin_for_class { supported_commands(firmware, class, condition_gate()) } else { Vec::new() };
    crate::cmdinfo::tree(firmware, class).into_values().filter(|c| supported.is_empty() || supported.contains(&c.id)).collect()
}

pub fn category_names(firmware: Firmware, class: VehicleClass) -> Vec<String> {
    let supported = supported_commands(firmware, class, condition_gate());
    let named = crate::cmdinfo::tree(firmware, class)
        .into_values()
        .filter(|c| supported.contains(&c.id))
        .fold(Vec::<String>::new(), |seen, c| if seen.contains(&c.category) { seen } else { [seen, vec![c.category]].concat() });
    [named, vec![ALL_COMMANDS.to_string()]].concat()
}

fn ui_info(c: &Command) -> Value {
    json!({
        "kind": "object",
        "class": "MissionCommandUIInfo",
        "objectName": "",
        "children": [],
        "facts": [],
        "command": c.id,
        "rawName": c.raw_name,
        "friendlyName": c.friendly_name,
        "description": c.description,
        "category": c.category,
        "friendlyEdit": c.friendly_edit,
        "specifiesCoordinate": c.specifies_coordinate,
        "specifiesAltitudeOnly": c.specifies_altitude_only,
        "isStandaloneCoordinate": c.standalone_coordinate,
        "isTakeoffCommand": c.is_takeoff,
        "isLandCommand": c.is_land,
        "isLoiterCommand": c.is_loiter,
    })
}

pub fn categories(backend: &dyn Backend, _path: &str) -> Value {
    match planned_for(backend) {
        Some((firmware, class)) => answered(json!(category_names(firmware, class))),
        None => refused("noPlan", "There is no plan to list mission commands for."),
    }
}

pub fn commands(backend: &dyn Backend, _path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let Some(category) = given.get(1).and_then(Value::as_str).filter(|c| !c.trim().is_empty()) else {
        return refused("noCategory", "Name the category to list, as categoriesForVehicle spelled it.");
    };
    let fly_through = match given.get(2) {
        None | Some(Value::Null) => true,
        Some(Value::Bool(b)) => *b,
        Some(_) => return refused("malformed", "Whether to list fly-through commands is true or false."),
    };
    let Some((firmware, class)) = planned_for(backend) else {
        return refused("noPlan", "There is no plan to list mission commands for.");
    };
    let listed: Vec<Value> = offered(firmware, class)
        .iter()
        .filter(|c| (c.category == category || category == ALL_COMMANDS) && (fly_through || !c.specifies_coordinate || c.standalone_coordinate))
        .map(ui_info)
        .collect();
    answered(Value::Array(listed))
}

pub fn command_target(path: &str) -> Option<usize> {
    path.strip_prefix(ITEM_COMMANDS)?.strip_suffix(".command")?.parse().ok()
}

fn offered_commands(backend: &dyn Backend) -> Option<Vec<i64>> {
    planned_for(backend).map(|(firmware, class)| offered(firmware, class).iter().map(|c| c.id).collect())
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
    match offered_commands(backend) {
        None => return refused("noPlan", "There is no plan to change a command in.".to_string()),
        Some(offered) if !offered.contains(&asked) => return refused("notOffered", format!("Command {asked} is not one this vehicle flies in a mission.")),
        Some(_) => {}
    }
    let answered = flag(&object(&backend.set(path, &json!({ "value": asked }).to_string())), "ok");
    json!({ "ok": answered, "result": answered, "refusal": Value::Null, "reason": match answered { true => Value::Null, false => json!("The item did not take the command.") } })
}

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
    if crate::coreplan::enabled() {
        let simple = crate::coreplan::current_document().items.get(index.wrapping_sub(1)).is_some_and(|item| matches!(item, crate::plandoc::Item::Simple(_)));
        if !simple {
            return refused("noCommand", format!("Item {index} has no command to change."));
        }
        crate::coreplan::set_map_center_hint(latitude, longitude);
        return json!({ "ok": true, "refusal": Value::Null, "reason": Value::Null });
    }
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
        vehicle: Value,
        calls: RefCell<Vec<String>>,
    }

    impl Backend for Tree {
        fn get(&self, p: &str) -> String { self.get_fields(p, "") }
        fn get_fields(&self, _p: &str, _f: &str) -> String { self.vehicle.to_string() }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, p: &str, _a: &str) -> String {
            self.calls.borrow_mut().push(p.to_string());
            String::new()
        }
        fn watch(&self, _p: &[String]) {}
    }

    fn names(listed: &Value) -> Vec<i64> {
        listed["result"].as_array().unwrap().iter().map(|c| c["command"].as_i64().unwrap()).collect()
    }

    #[test]
    fn the_command_tree_lists_what_qgc_lists_for_the_plans_vehicle() {
        let quad = Tree { vehicle: json!({ "kind": "object", "apmFirmware": true, "multiRotor": true }), calls: RefCell::new(Vec::new()) };
        assert_eq!(categories(&quad, "")["result"], json!(["Basic", "Loiter", "Flight control", "Advanced", "Conditionals", "Camera", "Safety", "All commands"]));
        assert_eq!(names(&commands(&quad, "", r#"["@vehicle","Basic",true]"#)), vec![16, 20, 21, 22, 82, 112]);
        assert_eq!(names(&commands(&quad, "", r#"["@vehicle","Basic",false]"#)), vec![20, 22, 112], "fly-through commands hide the ones that fly to a coordinate");
        assert_eq!(names(&commands(&quad, "", r#"["@vehicle","All commands",true]"#)).len(), 40);
        assert!(quad.calls.borrow().is_empty(), "the list is the core's, no Qt command tree is asked");

        let vtol = Tree { vehicle: json!({ "kind": "object", "px4Firmware": true, "vtol": true }), calls: RefCell::new(Vec::new()) };
        assert_eq!(categories(&vtol, "")["result"], json!(["Basic", "Loiter", "Advanced", "Conditionals", "Flight control", "Camera", "VTOL", "All commands"]));
        let generic = Tree { vehicle: json!({ "kind": "object", "multiRotor": true }), calls: RefCell::new(Vec::new()) };
        assert_eq!(categories(&generic, "")["result"], json!(["All commands"]), "a firmware with no supported list offers only the whole tree");
        let apm_generic = Tree { vehicle: json!({ "kind": "object", "apmFirmware": true }), calls: RefCell::new(Vec::new()) };
        assert_eq!(names(&commands(&apm_generic, "", r#"["@vehicle","Advanced",true]"#)).len(), 53, "ArduPilot has no plugin for a generic airframe, so its list is not filtered");

        assert_eq!(commands(&quad, "", r#"["@vehicle",""]"#)["refusal"], "noCategory");
        assert_eq!(commands(&quad, "", r#"["@vehicle","Basic","yes"]"#)["refusal"], "malformed");
        let none = Tree { vehicle: json!({ "kind": "null" }), calls: RefCell::new(Vec::new()) };
        assert_eq!(categories(&none, "")["refusal"], "noPlan");
    }

    struct Plan(RefCell<Vec<String>>);
    impl Backend for Plan {
        fn get(&self, p: &str) -> String { self.get_fields(p, "") }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "plan" => json!({ "kind": "object", "syncInProgress": false }),
                "plan.controllerVehicle" => json!({ "kind": "object", "px4Firmware": true, "multiRotor": true }),
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
        fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true }).to_string() }
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
