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
    "vehicle.gps.telemetryAvailable",
    "vehicle.gps.lock",
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
    "settings.flightModeSettings.px4PinnedFlightModesMultiRotor",
    "settings.flightModeSettings.px4PinnedFlightModesFixedWing",
    "settings.flightModeSettings.px4PinnedFlightModesVTOL",
    "settings.flightModeSettings.px4PinnedFlightModesRoverBoat",
    "settings.flightModeSettings.px4PinnedFlightModesSub",
    "settings.flightModeSettings.px4PinnedFlightModesAirship",
    "settings.flightModeSettings.apmPinnedFlightModesMultiRotor",
    "settings.flightModeSettings.apmPinnedFlightModesFixedWing",
    "settings.flightModeSettings.apmPinnedFlightModesVTOL",
    "settings.flightModeSettings.apmPinnedFlightModesRoverBoat",
    "settings.flightModeSettings.apmPinnedFlightModesSub",
    "settings.flightModeSettings.apmPinnedFlightModesAirship",
];

const VEHICLE_CLASSES: &[(&str, &str)] = &[("vtol", "VTOL"), ("fixedWing", "FixedWing"), ("multiRotor", "MultiRotor"), ("rover", "RoverBoat"), ("sub", "Sub"), ("airship", "Airship")];

const QUICK_SLOTS: &[(&str, &[&[&str]])] = &[
    ("MultiRotor", &[&["Position", "Position Hold", "Loiter"], &["Altitude", "Altitude Hold"], &["Stabilized", "Stabilize", "Manual"], &["Mission", "Auto"]]),
    ("FixedWing", &[&["Position", "Cruise"], &["Altitude", "FBW A"], &["Hold", "Loiter"], &["Mission", "Auto"]]),
    ("VTOL", &[&["Position", "QuadPlane Loiter"], &["Altitude", "QuadPlane Hover"], &["Stabilized", "Cruise"], &["Mission", "Auto"]]),
    ("RoverBoat", &[&["Position", "Steering"], &["Hold"], &["Manual"], &["Mission", "Auto"]]),
    ("Sub", &[&["Position Hold"], &["Depth Hold"], &["Stabilize", "Stabilized"], &["Manual"]]),
];
const QUICK_COUNT: usize = 4;
const QUICK_MINIMUM: usize = 2;

const GPS_MODES: &[&str] = &[
    "Position", "Position Hold", "Loiter", "QuadPlane Loiter", "Hold", "Brake", "Mission", "Auto", "Guided", "RTL", "Return", "Return to Groundstation",
    "Smart RTL", "AutoRTL", "QuadPlane RTL", "Circle", "Orbit", "Follow", "Follow Me", "Drift", "ZigZag", "Takeoff",
];
const GPS_3D_FIX: i64 = 3;
const NEEDS_GPS: &str = "Needs GPS";
const CANNOT_SET: &str = "This vehicle does not accept a flight mode change from here.";

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

fn vehicle_class(vehicle: &Value) -> Option<&'static str> {
    VEHICLE_CLASSES.iter().find(|(field, _)| flag(vehicle, field)).map(|(_, class)| *class)
}

fn mode_list_setting(vehicle: &Value, list: &str) -> Option<String> {
    let firmware = match (flag(vehicle, "px4Firmware"), flag(vehicle, "apmFirmware")) {
        (true, _) => "px4",
        (false, true) => "apm",
        _ => return None,
    };
    Some(format!("settings.flightModeSettings.{firmware}{list}FlightModes{}", vehicle_class(vehicle)?))
}

pub fn hidden_modes_setting(vehicle: &Value) -> Option<String> {
    mode_list_setting(vehicle, "Hidden")
}

pub fn pinned_modes_setting(vehicle: &Value) -> Option<String> {
    mode_list_setting(vehicle, "Pinned")
}

pub fn needs_gps(mode: &str, class: Option<&str>) -> bool {
    match class {
        Some("Sub") => false,
        Some("RoverBoat") => mode != "Hold" && GPS_MODES.contains(&mode),
        _ => GPS_MODES.contains(&mode),
    }
}

pub fn quick_modes(all: &[String], advanced: &[String], pinned: &[String], class: Option<&str>) -> Vec<String> {
    let pins: Vec<String> = pinned.iter().filter(|name| all.contains(name)).cloned().collect();
    let slots = QUICK_SLOTS.iter().find(|(name, _)| Some(*name) == class).map(|(_, slots)| *slots).unwrap_or(QUICK_SLOTS[0].1);
    let picked: Vec<String> = slots.iter().filter_map(|names| names.iter().find(|name| all.iter().any(|mode| mode.as_str() == **name)).map(|name| name.to_string())).collect();
    let everyday = || all.iter().filter(|name| !advanced.contains(name) && section(name) == "normal").take(QUICK_COUNT).cloned().collect();
    match () {
        _ if !pins.is_empty() => pins,
        _ if picked.len() >= QUICK_MINIMUM => picked,
        _ => everyday(),
    }
}

fn no_gps_fix(backend: &dyn Backend) -> bool {
    backend.value("vehicle.gps.telemetryAvailable")["value"] == true && backend.value("vehicle.gps.lock")["value"].as_i64().is_some_and(|lock| lock < GPS_3D_FIX)
}

fn listed_modes(backend: &dyn Backend, setting: Option<&str>) -> Vec<String> {
    let listed = setting.map(|path| text(&backend.value(path), "value")).unwrap_or_default();
    listed.split(',').filter(|mode| !mode.is_empty()).map(str::to_string).collect()
}

pub fn flight_modes_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = backend.value_fields("vehicle", "flightMode,flightModes,advancedFlightModes,armed,flying,flightModeSetAvailable,px4Firmware,apmFirmware,vtol,fixedWing,multiRotor,rover,sub,airship");
    let hidden_setting = hidden_modes_setting(&vehicle);
    let hidden = listed_modes(backend, hidden_setting.as_deref());
    let pinned_setting = pinned_modes_setting(&vehicle);
    let pinned = listed_modes(backend, pinned_setting.as_deref());
    let class = vehicle_class(&vehicle);
    let connected = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let strings = |key: &str| -> Vec<String> { vehicle.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default() };
    let (all, advanced) = (strings("flightModes"), strings("advancedFlightModes"));
    let current = text(&vehicle, "flightMode");
    let (armed, flying) = (flag(&vehicle, "armed"), flag(&vehicle, "flying"));
    let quick = quick_modes(&all, &advanced, &pinned, class);
    let gps_missing = connected && no_gps_fix(backend);
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
                "quick": quick.contains(name),
                "caution": if gps_missing && needs_gps(name, class) { NEEDS_GPS } else { "" },
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
    let can_set = connected && flag(&vehicle, "flightModeSetAvailable");
    let quick_list: Vec<Value> = quick.iter().filter_map(|name| modes.iter().find(|m| m["name"] == name.as_str()).cloned()).collect();
    json!({
        "kind": "object",
        "class": "FlightModes",
        "available": connected && !all.is_empty(),
        "canSet": can_set,
        "cannotSetNotice": if connected && !all.is_empty() && !can_set { CANNOT_SET } else { "" },
        "current": current,
        "currentSummary": description(&current),
        "unknownModeNotice": unknown_mode_notice(connected, &current, &all),
        "everyday": modes.iter().filter(|m| !folded(m)).cloned().collect::<Vec<_>>(),
        "folded": modes.iter().filter(|m| folded(m)).cloned().collect::<Vec<_>>(),
        "hiddenSetting": hidden_setting,
        "hidden": hidden,
        "pinnedSetting": pinned_setting,
        "quick": quick_list,
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
        _ if view["canSet"] != true => Some(("cannotSet", CANNOT_SET.to_string())),
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
                match p {
                    "settings.flightModeSettings.px4HiddenFlightModesMultiRotor" => json!({ "value": "Manual,Offboard,Hold" }).to_string(),
                    "settings.flightModeSettings.px4PinnedFlightModesMultiRotor" => json!({ "value": "" }).to_string(),
                    _ => String::new(),
                }
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
        assert_eq!(view["pinnedSetting"], "settings.flightModeSettings.px4PinnedFlightModesMultiRotor");
        assert_eq!(names("quick"), ["Position", "Manual"], "a hidden mode still fills its slot: hiding shortens QGC's list, the quick list is its own");

        assert_eq!(hidden_modes_setting(&json!({ "apmFirmware": true, "vtol": true, "fixedWing": true })).as_deref(), Some("settings.flightModeSettings.apmHiddenFlightModesVTOL"));
        assert_eq!(hidden_modes_setting(&json!({ "px4Firmware": true })), None, "a generic vehicle has no list to edit, so QGC turns editing off");
        assert_eq!(hidden_modes_setting(&json!({ "multiRotor": true })), None);
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn the_quick_list_holds_the_four_modes_each_kind_of_vehicle_flies_in() {
        let quick = |all: &[&str], advanced: &[&str], class: &str| quick_modes(&names(all), &names(advanced), &[], Some(class));
        let apm_copter = ["Stabilize", "Acro", "Altitude Hold", "Auto", "Guided", "Loiter", "RTL", "Circle", "Land", "Position Hold", "Brake", "Smart RTL"];
        assert_eq!(quick(&apm_copter, &[], "MultiRotor"), ["Position Hold", "Altitude Hold", "Stabilize", "Auto"]);
        assert_eq!(quick(&["Manual", "Stabilized", "Acro", "Altitude", "Position", "Mission", "Hold", "Return", "Land"], &[], "MultiRotor"), ["Position", "Altitude", "Stabilized", "Mission"]);
        let apm_plane = ["Manual", "Circle", "Stabilize", "Training", "Acro", "FBW A", "FBW B", "Cruise", "Autotune", "Auto", "RTL", "Loiter", "Takeoff", "Guided"];
        assert_eq!(quick(&apm_plane, &[], "FixedWing"), ["Cruise", "FBW A", "Loiter", "Auto"], "a plane pilot flies FBW A and Cruise, which a copter vocabulary never offered");
        let quadplane = ["Manual", "Stabilize", "FBW A", "Cruise", "Auto", "RTL", "Loiter", "QuadPlane Stabilize", "QuadPlane Hover", "QuadPlane Loiter", "QuadPlane Land", "QuadPlane RTL"];
        assert_eq!(quick(&quadplane, &[], "VTOL"), ["QuadPlane Loiter", "QuadPlane Hover", "Cruise", "Auto"]);
        assert_eq!(quick(&["Manual", "Acro", "Steering", "Hold", "Loiter", "Auto", "RTL", "Smart RTL", "Guided"], &[], "RoverBoat"), ["Steering", "Hold", "Manual", "Auto"]);
        assert_eq!(quick(&["Manual", "Stabilize", "Depth Hold", "Position Hold", "Auto", "Surface"], &[], "Sub"), ["Position Hold", "Depth Hold", "Stabilize", "Manual"]);
        assert_eq!(quick(&["Ready", "Takeoff", "Hold", "Track", "Return"], &["Track"], "Airship"), ["Ready", "Takeoff", "Hold"], "with no known vocabulary the vehicle's own everyday modes fill it, never a return mode");
    }

    #[test]
    fn pinned_modes_replace_the_defaults_in_the_order_they_were_pinned() {
        let all = names(&["Stabilize", "Acro", "Altitude Hold", "Auto", "Loiter", "Position Hold"]);
        assert_eq!(quick_modes(&all, &[], &names(&["Acro", "Gone", "Loiter"]), Some("MultiRotor")), ["Acro", "Loiter"], "a pin the vehicle does not offer is skipped");
        assert_eq!(quick_modes(&all, &[], &names(&["Gone"]), Some("MultiRotor")), ["Position Hold", "Altitude Hold", "Stabilize", "Auto"]);
        assert_eq!(pinned_modes_setting(&json!({ "apmFirmware": true, "vtol": true, "fixedWing": true })).as_deref(), Some("settings.flightModeSettings.apmPinnedFlightModesVTOL"));
    }

    #[test]
    fn a_mode_that_needs_a_position_fix_says_so_while_there_is_none() {
        assert!(needs_gps("Position", Some("MultiRotor")));
        assert!(needs_gps("Loiter", Some("FixedWing")));
        assert!(!needs_gps("Altitude Hold", Some("MultiRotor")));
        assert!(!needs_gps("Cruise", Some("FixedWing")), "Cruise flies on without GPS, like FBW B");
        assert!(!needs_gps("Hold", Some("RoverBoat")), "a rover stops in Hold with or without a fix, and Hold is how a pilot stops it");
        assert!(!needs_gps("Position Hold", Some("Sub")), "a sub holds position on its DVL");

        struct NoFix(i64);
        impl Backend for NoFix {
            fn get(&self, p: &str) -> String {
                match p {
                    "vehicle.gps.telemetryAvailable" => json!({ "kind": "fact", "value": true }).to_string(),
                    "vehicle.gps.lock" => json!({ "kind": "fact", "value": self.0 }).to_string(),
                    _ => String::new(),
                }
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String {
                json!({ "kind": "object", "flightMode": "Altitude Hold", "flightModes": ["Stabilize", "Altitude Hold", "Loiter", "Auto"], "advancedFlightModes": [], "flying": true, "flightModeSetAvailable": true, "apmFirmware": true, "multiRotor": true }).to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let cautions = |lock: i64| flight_modes_view(&NoFix(lock), &[])["modes"].as_array().unwrap().iter().map(|m| (m["name"].as_str().unwrap().to_string(), m["caution"].as_str().unwrap().to_string())).collect::<Vec<_>>();
        assert_eq!(cautions(1), [("Stabilize", ""), ("Altitude Hold", ""), ("Loiter", "Needs GPS"), ("Auto", "Needs GPS")].map(|(n, c)| (n.to_string(), c.to_string())));
        assert!(cautions(3).iter().all(|(_, caution)| caution.is_empty()), "a 3D fix clears it");
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
        assert_eq!(listed["cannotSetNotice"], CANNOT_SET, "the menu says why every row is greyed");
        assert_eq!(view["cannotSetNotice"], "");
        assert_eq!(view["currentSummary"], "Sticks set rotation rate, no self-levelling");
    }
}
