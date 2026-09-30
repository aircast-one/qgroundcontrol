use serde_json::{Value, json};

use crate::read::{flag, integer, object};
use crate::router::Backend;

// MultiVehicleManager::deselectAllVehicles clears the list and returns void, so the bridge answered
// ok for an empty selection it did nothing to and for a clear it could not confirm alike. The
// selection count is read before and after; ok means the selection is empty now.
const SELECTED_COUNT: &str = "vehicles.selectedVehicles.count";

fn selected(backend: &dyn Backend) -> i64 {
    integer(&object(&backend.get(SELECTED_COUNT)), "value").unwrap_or(0).max(0)
}

pub fn deselect_all(backend: &dyn Backend, path: &str) -> Value {
    let before = selected(backend);
    if before == 0 {
        return json!({ "ok": false, "refusal": "nothingSelected", "reason": "No vehicle is selected.", "cleared": 0 });
    }
    let dispatched = flag(&object(&backend.invoke(path, "[]")), "ok");
    let cleared = dispatched && selected(backend) == 0;
    json!({
        "ok": cleared,
        "refusal": Value::Null,
        "cleared": if cleared { before } else { 0 },
        "reason": match cleared { true => Value::Null, false => json!("The selection is still there.") },
    })
}

// The fleet picker sends pauseVehicle and startMission to each selected vehicle by its position in
// selectedVehicles. Vehicle::pauseVehicle on a vehicle that cannot pause, and startMission on one
// that is disarmed or already flying its mission, went out as asked. These are the rules view.vehicles
// applies to the whole selection, applied to the one vehicle the path names; the answer carries its
// id so the head can confirm the command landed on the vehicle it meant.
const FLEET_SELECTION: &str = "vehicles.selectedVehicles.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FleetCommand {
    Pause,
    StartMission,
}

pub fn fleet_target(path: &str) -> Option<(usize, FleetCommand)> {
    let (index, member) = path.strip_prefix(FLEET_SELECTION)?.split_once('.')?;
    let command = match member {
        "pauseVehicle" => FleetCommand::Pause,
        "startMission" => FleetCommand::StartMission,
        _ => return None,
    };
    Some((index.parse().ok()?, command))
}

fn fleet_refusal(command: FleetCommand, vehicle: &Value, pause_supported: bool) -> Option<(&'static str, &'static str)> {
    match command {
        _ if vehicle.get("kind").and_then(Value::as_str) != Some("object") => Some(("noSuchVehicle", "No vehicle is selected at that position.")),
        _ if !flag(vehicle, "armed") => Some(("disarmed", "That vehicle is not armed.")),
        FleetCommand::Pause if !pause_supported => Some(("unsupported", "That vehicle does not support being paused.")),
        FleetCommand::StartMission if vehicle.get("flightMode").is_some() && vehicle.get("flightMode") == vehicle.get("missionFlightMode") => {
            Some(("alreadyOnMission", "That vehicle is already flying its mission."))
        }
        _ => None,
    }
}

pub fn fleet_command(backend: &dyn Backend, path: &str) -> Value {
    let Some((index, command)) = fleet_target(path) else {
        return json!({ "ok": false, "refusal": "malformed", "reason": "That is not a fleet command the core sends." });
    };
    let vehicle = object(&backend.get_fields(&format!("{FLEET_SELECTION}{index}"), "id,armed,flightMode,missionFlightMode"));
    let pause_supported = flag(&object(&backend.get_fields(&format!("{FLEET_SELECTION}{index}.supports"), "pauseVehicle")), "pauseVehicle");
    let id = integer(&vehicle, "id");
    if let Some((token, reason)) = fleet_refusal(command, &vehicle, pause_supported) {
        return json!({ "ok": false, "refusal": token, "reason": reason, "vehicle": id });
    }
    let dispatched = flag(&object(&backend.invoke(path, "[]")), "ok");
    json!({ "ok": dispatched, "refusal": Value::Null, "vehicle": id, "reason": match dispatched { true => Value::Null, false => json!("The vehicle was not sent the command.") } })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Fleet {
        chosen: RefCell<i64>,
        obeys: bool,
        calls: RefCell<usize>,
    }

    impl Backend for Fleet {
        fn get(&self, _p: &str) -> String { json!({ "kind": "value", "value": *self.chosen.borrow() }).to_string() }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            *self.calls.borrow_mut() += 1;
            if self.obeys {
                *self.chosen.borrow_mut() = 0;
            }
            json!({ "ok": true }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    const PATH: &str = "vehicles.deselectAllVehicles";

    #[test]
    fn clearing_the_selection_says_what_it_cleared_and_refuses_an_empty_one() {
        let fleet = Fleet { chosen: RefCell::new(3), obeys: true, calls: RefCell::new(0) };
        let cleared = deselect_all(&fleet, PATH);
        assert_eq!((&cleared["ok"], &cleared["cleared"]), (&json!(true), &json!(3)));
        assert_eq!(deselect_all(&fleet, PATH)["refusal"], "nothingSelected", "the bridge answered ok for a clear of nothing");
        assert_eq!(*fleet.calls.borrow(), 1, "an empty selection is not dispatched");

        let deaf = Fleet { chosen: RefCell::new(2), obeys: false, calls: RefCell::new(0) };
        assert_eq!(deselect_all(&deaf, PATH)["ok"], false, "the count is read back, since a dispatched void call is not a cleared selection");
    }

    #[test]
    fn a_fleet_command_reaches_only_a_vehicle_that_can_take_it() {
        assert_eq!(fleet_target("vehicles.selectedVehicles.2.pauseVehicle"), Some((2, FleetCommand::Pause)));
        assert_eq!(fleet_target("vehicles.selectedVehicles.2.armed"), None, "arming stays with the vehicle's own path");
        let flying = json!({ "kind": "object", "id": 3, "armed": true, "flightMode": "Hold", "missionFlightMode": "Mission" });
        assert_eq!(fleet_refusal(FleetCommand::Pause, &flying, true), None);
        assert_eq!(fleet_refusal(FleetCommand::Pause, &flying, false).map(|r| r.0), Some("unsupported"));
        assert_eq!(fleet_refusal(FleetCommand::StartMission, &flying, true), None);
        let on_mission = json!({ "kind": "object", "armed": true, "flightMode": "Mission", "missionFlightMode": "Mission" });
        assert_eq!(fleet_refusal(FleetCommand::StartMission, &on_mission, true).map(|r| r.0), Some("alreadyOnMission"));
        assert_eq!(fleet_refusal(FleetCommand::StartMission, &json!({ "kind": "object", "armed": false }), true).map(|r| r.0), Some("disarmed"));
        assert_eq!(fleet_refusal(FleetCommand::Pause, &json!({ "kind": "null" }), true).map(|r| r.0), Some("noSuchVehicle"), "a vehicle dropping off renumbers the selection");

        struct One(RefCell<usize>);
        impl Backend for One {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, p: &str, _f: &str) -> String {
                match p {
                    "vehicles.selectedVehicles.0" => json!({ "kind": "object", "id": 7, "armed": true, "flightMode": "Hold", "missionFlightMode": "Mission" }),
                    "vehicles.selectedVehicles.0.supports" => json!({ "kind": "object", "pauseVehicle": true }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String {
                *self.0.borrow_mut() += 1;
                json!({ "ok": true }).to_string()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let one = One(RefCell::new(0));
        let paused = fleet_command(&one, "vehicles.selectedVehicles.0.pauseVehicle");
        assert_eq!((&paused["ok"], &paused["vehicle"]), (&json!(true), &json!(7)), "the id names the vehicle the command landed on");
        assert_eq!(fleet_command(&one, "vehicles.selectedVehicles.1.pauseVehicle")["refusal"], "noSuchVehicle");
        assert_eq!(*one.0.borrow(), 1);
    }
}
