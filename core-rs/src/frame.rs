use serde_json::{Value, json};

use crate::read::{flag, integer, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.id",
    "vehicle.armed",
    "vehicle.apmFirmware",
    "vehicle.multiRotor",
    "vehicle.vtol",
    "vehicle.rover",
    "vehicle.sub",
    "vehicle.fixedWing",
    "vehicle.airship",
    "vehicle.parameterManager.parametersReady",
    "vehicle.vehicleLinkManager.communicationLost",
    "vehicle.vehicleLinkManager.communicationLostEnabled",
    "vehicle.parameterManager.getParameter(-1,FRAME_CONFIG).rawValue",
];

const FIELDS: &str = "vehicleTypeString,motorCount,apmFirmware,armed,multiRotor,vtol,rover,sub,fixedWing,airship";
const NO_MOTOR_COUNT: i64 = -1;

fn token(vehicle: &Value) -> &'static str {
    match () {
        _ if flag(vehicle, "rover") => "RoverBoat",
        _ if flag(vehicle, "sub") => "Sub",
        _ if flag(vehicle, "multiRotor") => "MultiRotor",
        _ if flag(vehicle, "vtol") => "VTOL",
        _ if flag(vehicle, "fixedWing") => "FixedWing",
        _ if flag(vehicle, "airship") => "Airship",
        _ => "Generic",
    }
}

// The motor test compares connected, armed, apmFirmware and the count as a whole, so the fifth
// field it needs has to arrive in the same read - view.flyState and view.vehicles both serve this
// fact, and taking it from either would be a second snapshot at a different instant. Null means
// the link is not being monitored, which is neither lost nor fine: a head must not read it as fine.
fn contact_lost(backend: &dyn Backend) -> Option<bool> {
    let link = object(&backend.get_fields("vehicle.vehicleLinkManager", "communicationLost,communicationLostEnabled"));
    flag(&link, "communicationLostEnabled").then(|| flag(&link, "communicationLost"))
}

fn motors(vehicle: &Value, parameters_ready: bool) -> Option<i64> {
    let counted = integer(vehicle, "motorCount").filter(|count| *count != NO_MOTOR_COUNT)?;
    (!flag(vehicle, "sub") || parameters_ready).then_some(counted)
}

pub fn frame_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let ready = flag(&object(&backend.get("vehicle.parameterManager.parametersReady")), "value");
    let vehicle = object(&backend.get_fields("vehicle", FIELDS));
    let connected = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    json!({
        "kind": "object",
        "class": "Frame",
        "connected": connected,
        "vehicleType": connected.then(|| token(&vehicle)),
        "vehicleTypeText": text(&vehicle, "vehicleTypeString"),
        "motorCount": connected.then(|| motors(&vehicle, ready)).flatten(),
        "apmFirmware": flag(&vehicle, "apmFirmware"),
        "armed": flag(&vehicle, "armed"),
        "contactLost": connected.then(|| contact_lost(backend)).flatten(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Aircraft(Value, bool);

    impl Backend for Aircraft {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.parameterManager.parametersReady" => json!({ "kind": "value", "value": self.1 }).to_string(),
                _ => String::new(),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "vehicle" => self.0.to_string(),
                _ => String::new(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn vehicle(kind: &str, motors: i64) -> Value {
        json!({
            "kind": "object", "vehicleTypeString": "Quadrotor", "motorCount": motors,
            "apmFirmware": false, "armed": false,
            "multiRotor": kind == "multiRotor", "vtol": kind == "vtol", "rover": kind == "rover",
            "sub": kind == "sub", "fixedWing": kind == "fixedWing", "airship": kind == "airship",
        })
    }

    #[test]
    fn an_airframe_with_no_motor_count_says_nothing_rather_than_minus_one() {
        let view = frame_view(&Aircraft(vehicle("fixedWing", -1), true), &[]);
        assert_eq!(view["motorCount"], Value::Null, "QGCMAVLink::motorCount answers -1 for every airframe it does not enumerate, so a head testing count > 0 is right by accident and a head spelling it draws -1 motors");
        assert_eq!(view["vehicleType"], "FixedWing");

        let view = frame_view(&Aircraft(vehicle("multiRotor", 4), true), &[]);
        assert_eq!(view["motorCount"], 4);
    }

    #[test]
    fn a_submarine_counts_no_motors_until_its_parameters_arrive() {
        let view = frame_view(&Aircraft(vehicle("sub", 6), false), &[]);
        assert_eq!(view["motorCount"], Value::Null, "ParameterManager::getParameter hands back a default fact for a parameter it does not hold, so FRAME_CONFIG reads 0, which is SUB_FRAME_BLUEROV1, which is six - a confident wrong number on the only airframe whose count depends on a parameter");

        let view = frame_view(&Aircraft(vehicle("sub", 6), true), &[]);
        assert_eq!(view["motorCount"], 6);
        assert_eq!(view["vehicleType"], "Sub");
    }

    #[test]
    fn the_token_is_the_one_qgc_uses_internally_rather_than_the_spelled_name() {
        let view = frame_view(&Aircraft(vehicle("rover", -1), true), &[]);
        assert_eq!(view["vehicleType"], "RoverBoat", "vehicleClassToInternalString spells a boat and a rover the same, and vehicleTypeString is wrapped in tr() so it changes with the operator's language");
        assert_eq!(view["vehicleTypeText"], "Quadrotor");
    }

    #[test]
    fn no_vehicle_answers_nothing_rather_than_generic() {
        let view = frame_view(&Aircraft(json!({ "kind": "null" }), true), &[]);
        assert_eq!(view["connected"], false);
        assert_eq!(view["vehicleType"], Value::Null, "Generic is a vehicle QGC could not classify, and no vehicle at all is not that");
        assert_eq!(view["motorCount"], Value::Null);
        assert_eq!(view["armed"], false);
    }

    #[test]
    fn a_submarine_that_is_re_framed_recounts_its_motors() {
        assert!(DEPS.contains(&"vehicle.parameterManager.getParameter(-1,FRAME_CONFIG).rawValue"), "FRAME_CONFIG is the only input to a motor count that an operator can change while connected, and parametersReady does not fire again when they do - without it a re-framed sub keeps the count it had");
    }

    #[test]
    fn a_link_nobody_is_watching_is_neither_lost_nor_fine() {
        struct Link(bool, bool, bool);
        impl Backend for Link {
            fn get(&self, _p: &str) -> String { json!({ "kind": "value", "value": true }).to_string() }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "vehicle" => vehicle("multiRotor", 4).to_string(),
                    "vehicle.vehicleLinkManager" => json!({ "kind": "object", "communicationLostEnabled": self.1, "communicationLost": self.2 }).to_string(),
                    _ => json!({ "kind": "null" }).to_string(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        assert_eq!(frame_view(&Link(true, true, true), &[])["contactLost"], true);
        assert_eq!(frame_view(&Link(true, true, false), &[])["contactLost"], false);
        assert_eq!(frame_view(&Link(true, false, false), &[])["contactLost"], Value::Null, "with the watch off the flag stays false however long the vehicle has been silent, so serving it raw would call an unmonitored link healthy on the page that decides whether a motor may spin");
    }
}
