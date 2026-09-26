use serde_json::{Value, json};

use crate::read::{flag, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.flightMode", "vehicle.flightModes", "vehicle.advancedFlightModes", "vehicle.flying"];

const DESCRIPTIONS: &[(&str, &str)] = &[
    ("Stabilize", "You fly it by hand, it only levels itself"),
    ("Stabilized", "You fly it by hand, it only levels itself"),
    ("Manual", "Sticks go straight to the motors, no help"),
    ("Acro", "Sticks set rotation rate, no self-levelling"),
    ("Altitude Hold", "Holds height, you steer"),
    ("Altitude", "Holds height, you steer"),
    ("Depth Hold", "Holds depth, you steer"),
    ("Position Hold", "Holds position and height, sticks nudge it"),
    ("Position", "Holds position and height, sticks nudge it"),
    ("Loiter", "Holds position and height, or circles it on a plane"),
    ("Hold", "Stops and holds where it is"),
    ("Brake", "Stops as fast as it can and holds"),
    ("Guided", "Flies to points you tap on the map"),
    ("Guided No GPS", "Accepts attitude commands without a position fix"),
    ("Auto", "Flies the uploaded mission"),
    ("Mission", "Flies the uploaded mission"),
    ("RTL", "Climbs, returns home and lands"),
    ("Return", "Climbs, returns home and lands"),
    ("Smart RTL", "Retraces its own path back home"),
    ("AutoRTL", "Follows the mission's landing sequence home"),
    ("Return to Groundstation", "Returns to the ground station"),
    ("Land", "Lands straight down where it is"),
    ("Precision Land", "Lands on the landing target"),
    ("Precision Landing", "Lands on the landing target"),
    ("Takeoff", "Climbs to takeoff height and holds"),
    ("Circle", "Circles the point below it"),
    ("Orbit", "Circles a point you choose"),
    ("Follow", "Follows the ground station or a beacon"),
    ("Follow Me", "Follows the ground station"),
    ("Drift", "Coordinated turns, like a plane"),
    ("Sport", "Rate control with height hold"),
    ("Flip", "Does one flip, then returns to the previous mode"),
    ("Throw", "Starts flying when thrown"),
    ("Autotune", "Tunes the controllers automatically, needs room"),
    ("Flow Hold", "Holds position with optical flow, no GPS"),
    ("ZigZag", "Sweeps between two points you record"),
    ("SystemID", "Injects test signals for system identification"),
    ("AutoRotate", "Helicopter autorotation after engine loss"),
    ("Avoid ADSB", "Dodges ADS-B traffic automatically"),
    ("Turtle", "Flips itself upright after a crash"),
    ("Cruise", "Holds heading and height, sticks trim"),
    ("FBW A", "Sticks set bank and pitch, wings stay level"),
    ("FBW B", "Sticks set height and heading"),
    ("Training", "Manual with bank and pitch limits"),
    ("Thermal", "Circles rising air automatically"),
    ("Autoland", "Lands on the runway automatically"),
    ("Loiter to QLand", "Circles, then lands as a quadcopter"),
    ("QuadPlane Stabilize", "Hovers by hand, it only levels itself"),
    ("QuadPlane Hover", "Hovers holding height, you steer"),
    ("QuadPlane Loiter", "Hovers holding position and height"),
    ("QuadPlane Land", "Lands as a quadcopter where it is"),
    ("QuadPlane RTL", "Returns home and lands as a quadcopter"),
    ("QuadPlane AutoTune", "Tunes the hover controllers automatically"),
    ("QuadPlane Acro", "Rate control while hovering"),
    ("Steering", "Sticks set speed and turn rate"),
    ("Learning", "Records waypoints as you drive"),
    ("Simple", "Sticks steer relative to where you stand"),
    ("Dock", "Drives onto the docking target"),
    ("Surface", "Rises to the surface"),
    ("Surftrak", "Holds a set distance above the seabed"),
    ("Motor Detection", "Works out motor order and direction"),
    ("Rattitude", "Levels near centre, rate control at full stick"),
    ("Offboard", "Controlled by a companion computer"),
    ("Ready", "Armed and waiting on the ground"),
    ("Initializing", "Booting, cannot fly yet"),
];

pub fn description(mode: &str) -> &'static str {
    DESCRIPTIONS.iter().find(|(name, _)| *name == mode).map(|(_, d)| *d).unwrap_or("")
}

pub fn needs_confirming(mode: &str, flying: bool, rtl: &str, land: &str) -> bool {
    flying && !mode.is_empty() && (mode == rtl || mode == land)
}

pub fn flight_modes_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", "flightMode,flightModes,advancedFlightModes,flying,rtlFlightMode,landFlightMode,flightModeSetAvailable"));
    let connected = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let strings = |key: &str| -> Vec<String> { vehicle.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default() };
    let (all, advanced) = (strings("flightModes"), strings("advancedFlightModes"));
    let current = text(&vehicle, "flightMode");
    let flying = flag(&vehicle, "flying");
    let (rtl, land) = (text(&vehicle, "rtlFlightMode"), text(&vehicle, "landFlightMode"));
    let modes: Vec<Value> = all
        .iter()
        .map(|name| {
            json!({
                "name": name,
                "advanced": advanced.contains(name),
                "current": *name == current,
                "summary": description(name),
                "needsConfirm": needs_confirming(name, flying, &rtl, &land),
            })
        })
        .collect();
    json!({
        "kind": "object",
        "class": "FlightModes",
        "available": connected && !all.is_empty(),
        "canSet": connected && flag(&vehicle, "flightModeSetAvailable"),
        "current": current,
        "currentSummary": description(&current),
        "everyday": modes.iter().filter(|m| m["advanced"] == false || m["current"] == true).cloned().collect::<Vec<_>>(),
        "folded": modes.iter().filter(|m| m["advanced"] == true && m["current"] == false).cloned().collect::<Vec<_>>(),
        "modes": modes,
    })
}

fn mode_refusal(view: &Value, asked: Option<&str>) -> Option<(&'static str, String)> {
    let Some(asked) = asked.filter(|m| !m.is_empty()) else {
        return Some(("malformed", "A flight mode is set by name.".to_string()));
    };
    let listed = view["modes"].as_array().is_some_and(|modes| modes.iter().any(|m| m["name"] == asked));
    match () {
        _ if view["available"] != true => Some(("noVehicle", "No vehicle with flight modes is connected.".to_string())),
        _ if view["canSet"] != true => Some(("cannotSet", "This vehicle does not accept a flight mode change from here.".to_string())),
        _ if !listed => Some(("unknownMode", format!("{asked} is not one of this vehicle's flight modes."))),
        _ => None,
    }
}

pub fn write_mode(backend: &dyn Backend, path: &str, value: &str) -> Value {
    let asked = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value")?.as_str().map(str::to_string));
    let view = flight_modes_view(backend, &[]);
    if let Some((token, reason)) = mode_refusal(&view, asked.as_deref()) {
        return json!({ "ok": false, "result": false, "refusal": token, "reason": reason });
    }
    let asked = asked.unwrap_or_default();
    if view["current"] == asked.as_str() {
        return json!({ "ok": true, "result": true, "refusal": Value::Null, "unchanged": true, "reason": Value::Null });
    }
    let answered = flag(&object(&backend.set(path, &json!({ "value": asked }).to_string())), "ok");
    json!({
        "ok": answered,
        "result": answered,
        "refusal": Value::Null,
        "unchanged": false,
        "reason": match answered { true => Value::Null, false => json!("The vehicle was not asked to change mode.") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flight_mode_is_set_only_by_a_name_the_vehicle_lists() {
        let view = json!({ "available": true, "canSet": true, "current": "Hold", "modes": [{ "name": "Hold" }, { "name": "Position" }, { "name": "Return" }] });
        assert_eq!(mode_refusal(&view, Some("Position")), None);
        assert_eq!(mode_refusal(&view, Some("Loiter")).map(|r| r.0), Some("unknownMode"), "setFlightModeCustom fails on a name the firmware plugin does not know and setFlightMode returns with nothing sent and nothing said");
        assert_eq!(mode_refusal(&view, Some("")).map(|r| r.0), Some("malformed"));
        assert_eq!(mode_refusal(&view, None).map(|r| r.0), Some("malformed"));
        assert_eq!(mode_refusal(&json!({ "available": true, "canSet": false, "modes": [{ "name": "Hold" }] }), Some("Hold")).map(|r| r.0), Some("cannotSet"));
        assert_eq!(mode_refusal(&json!({ "available": false }), Some("Hold")).map(|r| r.0), Some("noVehicle"));

        use std::cell::RefCell;
        struct Vehicle(RefCell<Vec<String>>);
        impl Backend for Vehicle {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, _p: &str, _f: &str) -> String {
                json!({ "kind": "object", "flightMode": "Hold", "flightModes": ["Hold", "Position"], "advancedFlightModes": [], "flying": false, "rtlFlightMode": "Return", "landFlightMode": "Land", "flightModeSetAvailable": true }).to_string()
            }
            fn set(&self, _p: &str, v: &str) -> String {
                self.0.borrow_mut().push(v.to_string());
                json!({ "ok": true }).to_string()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let vehicle = Vehicle(RefCell::new(Vec::new()));
        assert_eq!(write_mode(&vehicle, "vehicle.flightMode", r#"{"value":"Hold"}"#)["unchanged"], true, "asking for the mode it is already in sends nothing");
        assert!(vehicle.0.borrow().is_empty());
        let changed = write_mode(&vehicle, "vehicle.flightMode", r#"{"value":"Position"}"#);
        assert_eq!((&changed["ok"], &changed["result"]), (&json!(true), &json!(true)));
        assert_eq!(vehicle.0.borrow().as_slice(), &[r#"{"value":"Position"}"#.to_string()]);
    }

    #[test]
    fn descriptions_and_confirmation_follow_the_vehicles_own_names() {
        assert_eq!(description("Return"), "Climbs, returns home and lands");
        assert_eq!(description("Nope"), "");
        assert!(needs_confirming("Return", true, "Return", "Land"));
        assert!(!needs_confirming("Return", false, "Return", "Land"));
        assert!(!needs_confirming("Hold", true, "Return", "Land"));
        assert!(!needs_confirming("", true, "", ""));
    }

    #[test]
    fn advanced_modes_fold_unless_current() {
        struct Fake;
        impl Backend for Fake {
            fn get(&self, _p: &str) -> String { String::new() }
            fn get_fields(&self, _p: &str, _f: &str) -> String {
                json!({ "kind": "object", "flightMode": "Acro", "flightModes": ["Hold", "Position", "Acro", "Offboard"], "advancedFlightModes": ["Acro", "Offboard"], "flying": true, "rtlFlightMode": "Return", "landFlightMode": "Land", "flightModeSetAvailable": true }).to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = flight_modes_view(&Fake, &[]);
        assert_eq!(view["available"], true);
        assert_eq!(view["canSet"], true);
        assert_eq!(view["everyday"].as_array().unwrap().len(), 3);
        assert_eq!(view["folded"].as_array().unwrap().len(), 1);
        assert_eq!(view["folded"][0]["name"], "Offboard");

        struct NoVehicle;
        impl Backend for NoVehicle {
            fn get(&self, _p: &str) -> String { String::new() }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "null" }).to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let none = flight_modes_view(&NoVehicle, &[]);
        assert_eq!(none["available"], false, "there is nothing to pick a mode on");
        assert_eq!(none["canSet"], false);

        struct Watching;
        impl Backend for Watching {
            fn get(&self, _p: &str) -> String { String::new() }
            fn get_fields(&self, _p: &str, _f: &str) -> String {
                json!({ "kind": "object", "flightMode": "Hold", "flightModes": ["Hold", "Position"], "advancedFlightModes": [], "flying": true, "rtlFlightMode": "Return", "landFlightMode": "Land", "flightModeSetAvailable": false }).to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let listed = flight_modes_view(&Watching, &[]);
        assert_eq!(listed["available"], true, "the modes are worth showing even when this link may not set one");
        assert_eq!(listed["canSet"], false, "the vehicle says the set is unavailable, and offering it anyway is a command that silently does nothing");
        assert_eq!(view["currentSummary"], "Sticks set rotation rate, no self-levelling");
    }
}
