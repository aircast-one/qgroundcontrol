use serde_json::{Value, json};

use crate::read::{flag, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.flightMode",
    "vehicle.flightModes",
    "vehicle.advancedFlightModes",
    "vehicle.flying",
    "vehicle.armed",
    "vehicle.px4Firmware",
    "vehicle.apmFirmware",
    "vehicle.vtol",
    "vehicle.fixedWing",
    "vehicle.multiRotor",
    "vehicle.rover",
    "vehicle.sub",
    "vehicle.airship",
    "settings.flightModeSettings.px4HiddenFlightModesMultiRotor",
    "settings.flightModeSettings.px4HiddenFlightModesFixedWing",
    "settings.flightModeSettings.px4HiddenFlightModesVTOL",
    "settings.flightModeSettings.px4HiddenFlightModesRoverBoat",
    "settings.flightModeSettings.px4HiddenFlightModesSub",
    "settings.flightModeSettings.px4HiddenFlightModesAirship",
    "settings.flightModeSettings.apmHiddenFlightModesMultiRotor",
    "settings.flightModeSettings.apmHiddenFlightModesFixedWing",
    "settings.flightModeSettings.apmHiddenFlightModesVTOL",
    "settings.flightModeSettings.apmHiddenFlightModesRoverBoat",
    "settings.flightModeSettings.apmHiddenFlightModesSub",
    "settings.flightModeSettings.apmHiddenFlightModesAirship",
];

const VEHICLE_CLASSES: &[(&str, &str)] = &[("vtol", "VTOL"), ("fixedWing", "FixedWing"), ("multiRotor", "MultiRotor"), ("rover", "RoverBoat"), ("sub", "Sub"), ("airship", "Airship")];

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
    ("Return to Groundstation", "Returns to the ground station"),
    ("Return", "Climbs, returns home and lands"),
    ("Smart RTL", "Retraces its own path back home"),
    ("AutoRTL", "Follows the mission's landing sequence home"),
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

const RETURN_KEYWORDS: [&str; 3] = ["return", "rtl", "land"];

pub fn section(mode: &str) -> &'static str {
    let name = mode.to_lowercase();
    match () {
        _ if RETURN_KEYWORDS.iter().any(|keyword| name.contains(keyword)) => "return",
        _ if name.contains("mocklink") => "dev",
        _ => "normal",
    }
}

fn section_rank(mode: &Value) -> usize {
    ["normal", "return", "dev"].iter().position(|s| mode["section"] == *s).unwrap_or(0)
}

pub fn needs_confirming(mode: &str, armed: bool, flying: bool) -> bool {
    section(mode) == "return" && armed && flying
}

pub fn hidden_modes_setting(vehicle: &Value) -> Option<String> {
    let firmware = match (flag(vehicle, "px4Firmware"), flag(vehicle, "apmFirmware")) {
        (true, _) => "px4",
        (false, true) => "apm",
        _ => return None,
    };
    let class = VEHICLE_CLASSES.iter().find(|(field, _)| flag(vehicle, field)).map(|(_, class)| *class)?;
    Some(format!("settings.flightModeSettings.{firmware}HiddenFlightModes{class}"))
}

fn hidden_modes(backend: &dyn Backend, setting: Option<&str>) -> Vec<String> {
    let listed = setting.map(|path| text(&object(&backend.get(path)), "value")).unwrap_or_default();
    listed.split(',').filter(|mode| !mode.is_empty()).map(str::to_string).collect()
}

pub fn flight_modes_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", "flightMode,flightModes,advancedFlightModes,armed,flying,flightModeSetAvailable,px4Firmware,apmFirmware,vtol,fixedWing,multiRotor,rover,sub,airship"));
    let hidden_setting = hidden_modes_setting(&vehicle);
    let hidden = hidden_modes(backend, hidden_setting.as_deref());
    let connected = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let strings = |key: &str| -> Vec<String> { vehicle.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default() };
    let (all, advanced) = (strings("flightModes"), strings("advancedFlightModes"));
    let current = text(&vehicle, "flightMode");
    let (armed, flying) = (flag(&vehicle, "armed"), flag(&vehicle, "flying"));
    let modes: Vec<Value> = all
        .iter()
        .map(|name| {
            json!({
                "name": name,
                "advanced": advanced.contains(name),
                "hidden": hidden.contains(name),
                "current": *name == current,
                "summary": description(name),
                "needsConfirm": needs_confirming(name, armed, flying),
                "section": section(name),
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .enumerate()
        .collect::<Vec<_>>()
        .into_iter()
        .map(|(order, mode)| ((section_rank(&mode), order), mode))
        .collect::<std::collections::BTreeMap<_, _>>()
        .into_values()
        .collect();
    json!({
        "kind": "object",
        "class": "FlightModes",
        "available": connected && !all.is_empty(),
        "canSet": connected && flag(&vehicle, "flightModeSetAvailable"),
        "current": current,
        "currentSummary": description(&current),
        "unknownModeNotice": unknown_mode_notice(connected, &current, &all),
        "everyday": modes.iter().filter(|m| !folded(m)).cloned().collect::<Vec<_>>(),
        "folded": modes.iter().filter(|m| folded(m)).cloned().collect::<Vec<_>>(),
        "hiddenSetting": hidden_setting,
        "hidden": hidden,
        "modes": modes,
        "modeAck": crate::hub::lock().active().and_then(|v| v.mode_ack).map(|(result, serial)| json!({ "serial": serial, "accepted": result == RESULT_ACCEPTED, "wording": rejection_wording(result) })),
    })
}

pub fn unknown_mode_notice(connected: bool, current: &str, known: &[String]) -> Option<String> {
    (connected && !current.is_empty() && !known.iter().any(|m| m == current))
        .then(|| format!("The vehicle is in {current}, which this version of the app doesn't know. Choose a mode below to change it."))
}

const RESULT_ACCEPTED: u8 = 0;

pub fn rejection_wording(result: u8) -> &'static str {
    match result {
        RESULT_ACCEPTED => "",
        1 => "refused for now",
        2 => "denied",
        3 => "not supported",
        _ => "failed",
    }
}

fn folded(mode: &Value) -> bool {
    mode["current"] == false && (mode["advanced"] == true || mode["hidden"] == true)
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
    fn a_mode_the_vehicle_reports_but_the_list_lacks_is_named_as_unknown() {
        let known = ["Hold".to_string(), "Position".to_string()];
        assert_eq!(unknown_mode_notice(true, "Custom 7", &known).as_deref(), Some("The vehicle is in Custom 7, which this version of the app doesn't know. Choose a mode below to change it."), "FlightModeIndicator's orange line");
        assert_eq!(unknown_mode_notice(true, "Hold", &known), None);
        assert_eq!(unknown_mode_notice(true, "", &known), None, "still connecting");
        assert_eq!(unknown_mode_notice(false, "Custom 7", &known), None);
    }

    #[test]
    fn a_refused_mode_change_reads_like_flight_mode_indicator() {
        assert_eq!([0, 1, 2, 3, 4, 9].map(rejection_wording), ["", "refused for now", "denied", "not supported", "failed", "failed"]);
    }

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
    fn modes_hidden_for_this_firmware_and_vehicle_class_fold_unless_current() {
        struct Px4Copter;
        impl Backend for Px4Copter {
            fn get(&self, p: &str) -> String {
                assert_eq!(p, "settings.flightModeSettings.px4HiddenFlightModesMultiRotor");
                json!({ "value": "Manual,Offboard,Hold" }).to_string()
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String {
                json!({ "kind": "object", "flightMode": "Hold", "flightModes": ["Hold", "Position", "Manual", "Offboard"], "advancedFlightModes": [], "flying": false, "flightModeSetAvailable": true, "px4Firmware": true, "multiRotor": true }).to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = flight_modes_view(&Px4Copter, &[]);
        let names = |key: &str| view[key].as_array().unwrap().iter().map(|m| m["name"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        assert_eq!(names("everyday"), ["Hold", "Position"], "the current mode stays listed even when hidden");
        assert_eq!(names("folded"), ["Manual", "Offboard"]);
        assert_eq!(view["hiddenSetting"], "settings.flightModeSettings.px4HiddenFlightModesMultiRotor");

        assert_eq!(hidden_modes_setting(&json!({ "apmFirmware": true, "vtol": true, "fixedWing": true })).as_deref(), Some("settings.flightModeSettings.apmHiddenFlightModesVTOL"));
        assert_eq!(hidden_modes_setting(&json!({ "px4Firmware": true })), None, "a generic vehicle has no list to edit, so QGC turns editing off");
        assert_eq!(hidden_modes_setting(&json!({ "multiRotor": true })), None);
    }

    #[test]
    fn descriptions_and_confirmation_follow_the_vehicles_own_names() {
        assert_eq!(description("Return"), "Climbs, returns home and lands");
        assert_eq!(description("Nope"), "");
        assert!(needs_confirming("Return", true, true));
        assert!(needs_confirming("Smart RTL", true, true), "FlightModeIndicator confirms any mode whose name holds return, rtl or land");
        assert!(needs_confirming("QLand", true, true));
        assert!(!needs_confirming("Return", true, false));
        assert!(!needs_confirming("Return", false, true), "and only while armed");
        assert!(!needs_confirming("Hold", true, true));
        assert_eq!((section("Mission"), section("Precision Land"), section("MockLink Dev")), ("normal", "return", "dev"));
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
