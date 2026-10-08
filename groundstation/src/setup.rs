use serde_json::{Value, json};

use crate::control::decode;
use crate::read::flag;
use crate::router::Backend;
use crate::sensors;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.parameterManager.parametersReady", "vehicle.parameterManager.requestUnanswered", "vehicle.parameterManager.parameterDownloadSkipped", "vehicle.parameterManager.missingParameters", "vehicle.parameterManager.getParameter(-1,COM_RC_IN_MODE).rawValue", "vehicle.autopilotPlugin.vehicleComponents", "vehicle.sysStatusSensorInfo.sensorNames", "vehicle.sysStatusSensorInfo.sensorStatus", "vehicle.armed", "vehicle.flying", "vehicle.rover", "vehicle.vtol", "vehicle.fixedWing", "vehicle.px4Firmware", "vehicle.apmFirmware", "vehicle.id", "vehicle.vehicleTypeString"];

const SAFETY_PAGES: [&str; 2] = ["Flight Safety", "Failsafes"];
const SAFETY_MAV_TYPES: [u8; 9] = [1, 2, 3, 4, 10, 12, 13, 14, 15];
pub const NOT_SUPPORTED_SCREEN: &str = "notSupported";

pub fn apm_safety_supported(vehicle_type: &str) -> bool {
    SAFETY_MAV_TYPES.iter().any(|t| crate::vehiclefacade::mav_type_text(*t) == vehicle_type)
}

const PX4_ONLY: &[&str] = &["Flight Behavior", "Safety"];
const APM_ONLY: &[&str] = &["Flight Safety", "Failsafes", "Logging", "Gimbal", "Airspeed", "ESC", "Servo Outputs", "Heli", "Follow Me", "Tuning - Advanced", "Scripting", "Lights", "Remote Support"];

/// Which component backs each page. The join is on the KnownVehicleComponent enum where the
/// firmware declares one and on the C++ class name otherwise - both untranslated, where the
/// component's own name is tr() wrapped and would gate in English only. Pages with no
/// component are named so that adding one without deciding fails the test below rather than
/// silently reading as never blocked.
const PAGE_COMPONENTS: &[(&str, &[&str])] = &[
    ("Summary", &[]),
    ("Parameters", &[]),
    ("Sensors", &["known:sensors"]),
    ("Radio", &["known:radio"]),
    ("Flight Modes", &["known:flightModes"]),
    ("Safety", &["known:safety"]),
    ("Flight Safety", &["known:safety"]),
    ("Failsafes", &["APMFailsafesComponent"]),
    ("Logging", &["APMLoggingComponent"]),
    ("Power", &["known:power"]),
    ("Frame", &["AirframeComponent", "APMAirframeComponent", "APMSubFrameComponent"]),
    ("Motors", &["MotorComponent", "APMMotorComponent"]),
    ("Tuning", &["APMTuningComponent", "PX4TuningComponent"]),
    ("Tuning - Advanced", &["APMAdvancedTuningCopterComponent"]),
    ("Scripting", &["ScriptingComponent"]),
    ("Joystick", &["JoystickComponent"]),
    ("Gimbal", &["APMGimbalComponent"]),
    ("Airspeed", &["APMAirspeedComponent"]),
    ("ESC", &["APMESCComponent"]),
    ("Servo Outputs", &["APMServoComponent"]),
    ("Lights", &["APMLightsComponent"]),
    ("Flight Behavior", &["PX4FlightBehavior"]),
    ("Remote Support", &["APMRemoteSupportComponent"]),
    ("Actuators", &["ActuatorComponent"]),
    ("Heli", &["APMHeliComponent"]),
    ("Follow Me", &["APMFollowComponent"]),
    ("WiFi Bridge", &["ESP8266Component"]),
    ("Syslink", &["SyslinkComponent"]),
];

fn page_block(page: &str, components: &[Component]) -> Option<&'static str> {
    let keys = PAGE_COMPONENTS.iter().find(|(name, _)| *name == page).map(|(_, keys)| *keys)?;
    components
        .iter()
        .find(|c| {
            keys.iter().any(|key| match key.strip_prefix("known:") {
                Some(known) => c.known.as_deref() == Some(known),
                None => c.class_name == *key,
            })
        })
        .and_then(|c| c.blocked_reason)
}

pub fn page_exists(page: &str, px4: bool) -> bool {
    match (PX4_ONLY.contains(&page), APM_ONLY.contains(&page)) {
        (true, _) => px4,
        (_, true) => !px4,
        _ => true,
    }
}

pub fn page_absence(px4: bool) -> &'static str {
    match px4 {
        true => "This is an ArduPilot setup screen. PX4 firmware has no equivalent.",
        false => "This is a PX4 setup screen. ArduPilot firmware has no equivalent.",
    }
}

pub const PAGES: &[(&str, &[&str])] = &[
    ("Vehicle", &["Summary"]),
    ("Setup", &["Sensors", "Radio", "Frame", "Flight Modes", "Safety", "Flight Safety", "Failsafes", "Power", "Airspeed", "ESC", "Servo Outputs", "Motors", "Actuators", "Heli", "Tuning", "Tuning - Advanced", "Gimbal", "Lights", "Flight Behavior", "Follow Me", "Joystick"]),
    ("Advanced", &["Logging", "Scripting", "Remote Support", "WiFi Bridge", "Syslink", "Parameters"]),
];

pub struct Section {
    pub title: &'static str,
    pub note: &'static str,
    pub parameters: &'static [&'static str],
}

const FLIGHT_MODES_APM: &[Section] = &[
    Section { title: "Flight Mode Settings", note: "", parameters: &["FLTMODE_CH", "MODE_CH", "FLTMODE1", "FLTMODE2", "FLTMODE3", "FLTMODE4", "FLTMODE5", "FLTMODE6", "MODE1", "MODE2", "MODE3", "MODE4", "MODE5", "MODE6"] },
    Section { title: "Options", note: "", parameters: &["INITIAL_MODE"] },
    Section { title: "Switch Options", note: "", parameters: &["RC6_OPTION", "RC7_OPTION", "RC8_OPTION", "RC9_OPTION", "RC10_OPTION", "RC11_OPTION", "RC12_OPTION", "RC13_OPTION", "RC14_OPTION", "RC15_OPTION", "RC16_OPTION"] },
];
const FLIGHT_MODES_PX4: &[Section] = &[
    Section { title: "Flight Mode Settings", note: "", parameters: &["RC_MAP_FLTMODE", "COM_FLTMODE1", "COM_FLTMODE2", "COM_FLTMODE3", "COM_FLTMODE4", "COM_FLTMODE5", "COM_FLTMODE6"] },
    Section { title: "Switch Settings", note: "", parameters: &["RC_MAP_ARM_SW", "RC_MAP_GEAR_SW", "RC_MAP_KILL_SW", "RC_MAP_LOITER_SW", "RC_MAP_OFFB_SW", "RC_MAP_RETURN_SW", "RC_MAP_TRANS_SW", "RC_MAP_FLAPS"] },
];

pub fn switch_applies(parameter: &str, vtol: bool, fixed_wing: bool) -> bool {
    match parameter {
        "RC_MAP_TRANS_SW" => vtol,
        "RC_MAP_FLAPS" => fixed_wing,
        _ => true,
    }
}
const HELI_APM: &[Section] = &[
    Section { title: "Servo Setup", note: "", parameters: &["label:Servo 1", "SERVO1_FUNCTION", "SERVO1_MIN", "SERVO1_MAX", "SERVO1_TRIM", "SERVO1_REVERSED", "label:Servo 2", "SERVO2_FUNCTION", "SERVO2_MIN", "SERVO2_MAX", "SERVO2_TRIM", "SERVO2_REVERSED", "label:Servo 3", "SERVO3_FUNCTION", "SERVO3_MIN", "SERVO3_MAX", "SERVO3_TRIM", "SERVO3_REVERSED", "label:Servo 4", "SERVO4_FUNCTION", "SERVO4_MIN", "SERVO4_MAX", "SERVO4_TRIM", "SERVO4_REVERSED", "label:Servo 5", "SERVO5_FUNCTION", "SERVO5_MIN", "SERVO5_MAX", "SERVO5_TRIM", "SERVO5_REVERSED", "label:Servo 6", "SERVO6_FUNCTION", "SERVO6_MIN", "SERVO6_MAX", "SERVO6_TRIM", "SERVO6_REVERSED", "label:Servo 7", "SERVO7_FUNCTION", "SERVO7_MIN", "SERVO7_MAX", "SERVO7_TRIM", "SERVO7_REVERSED", "label:Servo 8", "SERVO8_FUNCTION", "SERVO8_MIN", "SERVO8_MAX", "SERVO8_TRIM", "SERVO8_REVERSED"] },
    Section { title: "Swashplate Setup", note: "", parameters: &["H_SV_MAN", "H_SW_TYPE", "H_SW_COL_DIR", "H_SW_LIN_SVO", "H_FLYBAR_MODE", "H_CYC_MAX", "H_COL_MAX", "H_COL_ANG_MAX", "H_COL_MIN", "H_COL_ANG_MIN", "H_COL_ZERO_THRST", "H_COL_LAND_MIN"] },
    Section { title: "Throttle Settings", note: "", parameters: &["H_RSC_MODE", "H_RSC_CRITICAL", "H_RSC_RAMP_TIME", "H_RSC_RUNUP_TIME", "H_RSC_CLDWN_TIME", "H_RSC_SETPOINT", "H_RSC_IDLE", "H_RSC_THRCRV_0", "H_RSC_THRCRV_25", "H_RSC_THRCRV_50", "H_RSC_THRCRV_75", "H_RSC_THRCRV_100"] },
    Section { title: "Governor Settings", note: "", parameters: &["H_RSC_GOV_COMP", "H_RSC_GOV_DROOP", "H_RSC_GOV_FF", "H_RSC_GOV_RANGE", "H_RSC_GOV_RPM", "H_RSC_GOV_TORQUE"] },
    Section { title: "Miscellaneous Settings", note: "", parameters: &["label:* Stabilize Collective Curve *", "IM_STB_COL_1", "IM_STB_COL_2", "IM_STB_COL_3", "IM_STB_COL_4", "label:* Tail & Gyros *", "H_TAIL_TYPE", "H_TAIL_SPEED", "H_GYR_GAIN", "H_GYR_GAIN_ACRO", "H_COLYAW"] },
];

pub fn screen_for(page: &str, px4: bool) -> Option<&'static str> {
    match (page, px4) {
        ("Tuning", true) => Some("px4Tuning"),
        ("Frame", true) => Some("px4Airframe"),
        ("Frame", false) => Some(crate::apmairframe::AIRFRAME_SCREEN),
        ("Tuning - Advanced", false) => Some("px4Tuning"),
        ("Scripting", false) => Some("scripting"),
        ("Joystick", _) => Some("joystick"),
        ("WiFi Bridge", _) => Some("espBridge"),
        ("Syslink", _) => Some("syslink"),
        ("Servo Outputs", false) => Some("apmServos"),
        ("Follow Me", false) => Some("apmFollow"),
        ("Actuators", true) if crate::vehiclefacade::switched_on() => Some("actuators"),
        _ => None,
    }
}

pub fn sections_for(page: &str, px4: bool) -> Option<&'static [Section]> {
    match (page, px4) {
        ("Flight Modes", true) => Some(FLIGHT_MODES_PX4),
        ("Flight Modes", false) => Some(FLIGHT_MODES_APM),
        ("Heli", false) => Some(HELI_APM),
        _ => None,
    }
}

pub fn setup_complete_of(components: &[(String, bool)]) -> bool {
    components.iter().all(|(_, needs)| !needs)
}

pub fn readiness(connected: bool, parameters_ready: bool, components: &[(String, bool)], sensor_faults: &[String]) -> (Option<bool>, String, String) {
    let outstanding: Vec<&str> = components.iter().filter(|(_, needs)| *needs).map(|(n, _)| n.as_str()).collect();
    if !connected {
        return (None, "No vehicle connected".into(), "Connect a vehicle to check what it needs.".into());
    }
    if !parameters_ready {
        return (None, "Waiting for this vehicle's parameters".into(), "Its setup cannot be checked until it has answered.".into());
    }
    let ready = Some(outstanding.is_empty() && sensor_faults.is_empty() && !components.is_empty());
    let headline = match (outstanding.len(), sensor_faults.len(), components.is_empty()) {
        (1, _, _) => "1 component needs setup".to_string(),
        (n, _, _) if n > 1 => format!("{n} components need setup"),
        (0, f, _) if f > 0 => format!("{f} sensor{} reporting a fault", if f == 1 { "" } else { "s" }),
        (0, 0, true) => "This vehicle reports no setup components".to_string(),
        _ => "Ready to fly".to_string(),
    };
    let detail = match (sensor_faults.is_empty(), outstanding.is_empty(), components.is_empty()) {
        (false, _, _) => sensor_faults.join(", "),
        (true, false, _) => outstanding.join(", "),
        (true, true, true) => "Nothing to check.".to_string(),
        _ => "Setup complete and all enabled sensors are healthy.".to_string(),
    };
    (ready, headline, detail)
}

pub fn setup_view(backend: &dyn Backend, args: &[String]) -> Value {
    let vehicle = backend.value_fields("vehicle", "px4Firmware");
    let connected = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let px4 = flag(&vehicle, "px4Firmware");
    match args.first() {
        Some(page) if crate::vehicleconfig::has(page, px4) => crate::vehicleconfig::page(backend, page, px4),
        Some(page) => page_json(backend, page, px4),
        None => overview(backend, connected, px4),
    }
}

pub const COMPONENTS: &str = "vehicle.autopilotPlugin.vehicleComponents";

pub struct Component {
    pub name: String,
    pub class_name: String,
    pub known: Option<String>,
    pub needs_attention: bool,
    pub blocked_reason: Option<&'static str>,
    pub setup_complete: bool,
}

pub(crate) const AIRFRAME_CLASSES: [&str; 2] = ["APMAirframeComponent", "AirframeComponent"];
pub(crate) const RADIO_CLASSES: [&str; 2] = ["APMRadioComponent", "PX4RadioComponent"];
pub(crate) const NEEDS_AIRFRAME: [&str; 12] = [
    "APMFlightModesComponent", "APMRadioComponent", "APMPowerComponent", "APMESCComponent", "APMFlightSafetyComponent", "APMTuningComponent", "APMSensorsComponent", "APMAirspeedComponent",
    "PX4TuningComponent", "PowerComponent", "SafetyComponent", "SensorsComponent",
];
pub(crate) const RC_IN_MODE_NO_RC: i64 = 1;

pub fn prerequisite(component: &Component, all: &[Component], rc_in_mode: Option<i64>) -> Option<String> {
    let unfinished = |classes: &[&str]| all.iter().find(|c| classes.contains(&c.class_name.as_str()) && !c.setup_complete).map(|c| c.name.clone());
    match component.class_name.as_str() {
        "APMFlightModesComponent" => unfinished(&AIRFRAME_CLASSES).or_else(|| unfinished(&RADIO_CLASSES)),
        "FlightModesComponent" if rc_in_mode == Some(RC_IN_MODE_NO_RC) => None,
        "FlightModesComponent" => unfinished(&AIRFRAME_CLASSES).or_else(|| unfinished(&RADIO_CLASSES)),
        class if NEEDS_AIRFRAME.contains(&class) => unfinished(&AIRFRAME_CLASSES),
        _ => None,
    }
}

fn rc_in_mode(backend: &dyn Backend, px4: bool) -> Option<i64> {
    px4.then(|| crate::read::value_number(&backend.value("vehicle.parameterManager.getParameter(-1,COM_RC_IN_MODE).rawValue")).map(|m| m as i64)).flatten()
}

pub fn opened_component(backend: &dyn Backend, name: &str, px4: bool) -> Option<(String, Option<String>)> {
    let components = vehicle_components(backend);
    let rc_in_mode = rc_in_mode(backend, px4);
    components.iter().find(|c| c.name == name).map(|c| (c.class_name.clone(), prerequisite(c, &components, rc_in_mode)))
}

fn known_component(component: &Value) -> Option<String> {
    let raw = component.get("KnownVehicleComponent")?;
    let named = raw
        .as_str()
        .map(str::to_string)
        .or_else(|| raw.as_i64().map(|index| KNOWN_COMPONENTS.get(index as usize).copied().unwrap_or("unknown").to_string()))?;
    let trimmed = named.trim_start_matches("Known").trim_end_matches("VehicleComponent");
    match trimmed.is_empty() || trimmed == "Unknown" {
        true => None,
        false if trimmed.chars().all(|c| c.is_ascii_uppercase()) => Some(trimmed.to_lowercase()),
        false => Some(trimmed[..1].to_lowercase() + &trimmed[1..]),
    }
}

const KNOWN_COMPONENTS: [&str; 8] = [
    "KnownRadioVehicleComponent",
    "KnownFlightModesVehicleComponent",
    "KnownSensorsVehicleComponent",
    "KnownSafetyVehicleComponent",
    "KnownPowerVehicleComponent",
    "KnownJoystickVehicleComponent",
    "KnownESCVehicleComponent",
    "UnknownVehicleComponent",
];

fn blocked_by(component: &Value, armed: bool, flying: bool, rover: bool) -> Option<&'static str> {
    let by_armed = !flag(component, "allowSetupWhileArmed") && armed;
    let by_flying = !rover && !flag(component, "allowSetupWhileFlying") && flying;
    match (by_armed, by_flying) {
        (true, _) => Some("armed"),
        (false, true) => Some("flying"),
        (false, false) => None,
    }
}

pub fn setup_complete(backend: &dyn Backend) -> bool {
    vehicle_components(backend).iter().all(|component| component.setup_complete)
}

fn vehicle_components(backend: &dyn Backend) -> Vec<Component> {
    let state = backend.value_fields("vehicle", "armed,flying,rover");
    let (armed, flying, rover) = (flag(&state, "armed"), flag(&state, "flying"), flag(&state, "rover"));
    let listed = backend.value(COMPONENTS);
    let count = listed.get("value").and_then(Value::as_array).map(|elements| elements.len()).unwrap_or(0);
    (0..count)
        .filter_map(|index| {
            let component = backend.value_fields(&format!("{COMPONENTS}.{index}"), "name,class,requiresSetup,setupComplete,allowSetupWhileArmed,allowSetupWhileFlying,KnownVehicleComponent");
            let name = component.get("name").and_then(Value::as_str).filter(|name| !name.is_empty())?;
            Some(Component {
                name: name.to_string(),
                class_name: component.get("class").and_then(Value::as_str).unwrap_or_default().to_string(),
                known: known_component(&component),
                needs_attention: flag(&component, "requiresSetup") && !flag(&component, "setupComplete"),
                blocked_reason: blocked_by(&component, armed, flying, rover),
                setup_complete: flag(&component, "setupComplete"),
            })
        })
        .collect()
}

const INCOMPLETE: &str = "incomplete";
const INCOMPLETE_PAGES: &[&str] = &["Summary", "Firmware", "Optical Flow", "Parameters"];

fn parameter_state(backend: &dyn Backend, connected: bool) -> (bool, &'static str, &'static str) {
    if !connected {
        return (false, "noVehicle", "");
    }
    let manager = backend.value_fields("vehicle.parameterManager", "parametersReady,requestUnanswered,parameterDownloadSkipped,missingParameters");
    match (flag(&manager, "parametersReady"), flag(&manager, "requestUnanswered")) {
        (true, _) if flag(&manager, "missingParameters") => (true, INCOMPLETE, "The vehicle didn't return its full parameter list, so some setup options are unavailable."),
        (true, _) => (true, "", ""),
        _ if flag(&manager, "parameterDownloadSkipped") => (false, "skipped", "Parameter download was skipped because the vehicle is flying. Configuration pages will be available after parameters are downloaded."),
        (false, true) => (false, "unanswered", "This vehicle has not answered the request for its parameters, and the retries are finished."),
        (false, false) => (false, "loading", ""),
    }
}

fn overview(backend: &dyn Backend, connected: bool, px4: bool) -> Value {
    let components = vehicle_components(backend);
    let rc_in_mode = rc_in_mode(backend, px4);
    let faults: Vec<String> = sensors::sensors(&backend.value("vehicle.sysStatusSensorInfo")).into_iter().filter(|(_, s)| *s == "unhealthy").map(|(n, _)| n).collect();
    let named: Vec<(String, bool)> = components.iter().map(|c| (c.name.clone(), !c.setup_complete)).collect();
    let (parameters_ready, parameters_reason, parameters_text) = parameter_state(backend, connected);
    let incomplete = parameters_reason == INCOMPLETE;
    let (ready, headline, detail) = match incomplete {
        true => (None, "Parameters incomplete".to_string(), parameters_text.to_string()),
        false => readiness(connected, parameters_ready, &named, &faults),
    };
    let sub_frame = components.iter().any(|c| c.class_name == "APMSubFrameComponent");
    let safety_unsupported = connected && !px4 && !apm_safety_supported(&crate::read::text(&backend.value_fields("vehicle", "vehicleTypeString"), "vehicleTypeString"));
    let flow_images = connected && crate::hub::lock().active_id().is_some_and(|id| crate::flowimage::image_index(id) > 0);
    let components: Vec<Component> = if incomplete { Vec::new() } else { components };
    json!({
        "kind": "object",
        "class": "VehicleSetup",
        "connected": connected,
        "parametersReady": parameters_ready,
        "parametersReason": parameters_reason,
        "parametersText": parameters_text,
        "firmware": if !connected { "none" } else if px4 { "px4" } else { "apm" },
        "vehicleId": connected.then(|| backend.value("vehicle.id").get("value").cloned()).flatten(),
        "ready": ready,
        "setupComplete": (connected && parameters_ready).then(|| incomplete || setup_complete_of(&named)),
        "headline": headline,
        "detail": detail,
        "components": components.iter().map(|c| json!({
            "name": c.name,
            "className": c.class_name,
            "known": c.known,
            "needsAttention": c.needs_attention,
            "openable": c.blocked_reason.is_none(),
            "blockedReason": c.blocked_reason,
            "prerequisite": prerequisite(c, &components, rc_in_mode),
        })).collect::<Vec<_>>(),
        "groups": PAGES.iter().map(|(title, pages)| json!({
            "title": title,
            "pages": pages.iter().filter(|p| page_exists(p, px4) && (!incomplete || INCOMPLETE_PAGES.contains(p))).chain((*title == "Setup" && flow_images).then_some(&"Optical Flow")).map(|p| {
                let blocked = page_block(p, &components);
                let screen = match (sub_frame, *p) {
                    (_, page) if safety_unsupported && SAFETY_PAGES.contains(&page) => Some(NOT_SUPPORTED_SCREEN),
                    (true, "Frame") => Some(crate::apmsubframe::SUB_FRAME_SCREEN),
                    (true, "Motors") => Some(crate::apmsubmotors::SUB_MOTORS_SCREEN),
                    (_, "Optical Flow") => Some(crate::flowimage::OPTICAL_FLOW_SCREEN),
                    _ => screen_for(p, px4),
                };
                json!({ "name": p, "parameterSections": screen.is_none() && (sections_for(p, px4).is_some() || crate::vehicleconfig::has(p, px4)), "screen": screen, "openable": blocked.is_none(), "blockedReason": blocked })
            }).collect::<Vec<_>>(),
            "omitted": pages.iter().filter(|p| !page_exists(p, px4)).map(|p| json!({ "name": p, "reason": page_absence(px4) })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

const APM_SLOT_PWM: [&str; 6] = ["PWM 0 - 1230", "PWM 1231 - 1360", "PWM 1361 - 1490", "PWM 1491 - 1620", "PWM 1621 - 1749", "PWM 1750 +"];

fn apm_mode_row(name: &str, mut row: Value, px4: bool) -> Value {
    if px4 {
        return row;
    }
    let numbered = |prefix: &str| name.strip_prefix(prefix).and_then(|rest| rest.parse::<usize>().ok());
    let option = name.strip_prefix("RC").and_then(|rest| rest.strip_suffix("_OPTION")).and_then(|n| n.parse::<usize>().ok());
    let slot = numbered("FLTMODE").or_else(|| numbered("MODE")).filter(|n| (1..=6).contains(n));
    match (option, slot) {
        (Some(channel), _) => {
            row["label"] = json!(format!("Channel option {channel}"));
            row["shortLabel"] = json!(format!("Channel option {channel}"));
        }
        (None, Some(slot)) => {
            row["shortLabel"] = json!(format!("Flight Mode {slot}"));
            row["label"] = json!(APM_SLOT_PWM[slot - 1]);
        }
        _ if name == "FLTMODE_CH" || name == "MODE_CH" => {
            row["label"] = json!("Flight mode channel");
            row["shortLabel"] = json!("Flight mode channel");
            let options: Vec<Value> = std::iter::once("Not assigned".to_string()).chain((1..=8).map(|n| format!("Channel {n}"))).enumerate().map(|(raw, label)| json!({ "label": label, "raw": raw.to_string() })).collect();
            let chosen = row["value"].as_f64().filter(|v| v.fract() == 0.0 && (0.0..=8.0).contains(v)).and_then(|v| options.get(v as usize)).map(|o| o["label"].clone());
            row["display"] = chosen.unwrap_or_else(|| row["valueString"].clone());
            row["options"] = json!(options);
            row["control"] = json!("choice");
            row["rawChoice"] = json!(true);
        }
        _ => {}
    }
    row
}

const LABEL_ROW: &str = "label:";

fn servo_column_row(name: &str, mut row: Value) -> Value {
    let column = name.strip_prefix("SERVO").and_then(|rest| rest.split_once('_')).filter(|(n, _)| n.parse::<u8>().is_ok()).and_then(|(_, field)| match field {
        "FUNCTION" => Some("Function"),
        "MIN" => Some("Min"),
        "MAX" => Some("Max"),
        "TRIM" => Some("Trim"),
        "REVERSED" => Some("Reversed"),
        _ => None,
    });
    if let Some(column) = column {
        row["label"] = json!(column);
        row["shortLabel"] = json!(column);
    }
    row
}

fn page_json(backend: &dyn Backend, page: &str, px4: bool) -> Value {
    let Some(sections) = sections_for(page, px4) else { return crate::read::refused(&format!("no setup page is called {page} for this firmware; the pages a vehicle offers depend on which plugin built them")) };
    let read = |name: &str| {
        let path = format!("vehicle.parameterManager.getParameter(-1,{name})");
        let fact = backend.value(&path);
        let present = fact.get("kind").and_then(Value::as_str) == Some("fact") && fact.get("name").and_then(Value::as_str).is_some_and(|n| !n.is_empty());
        present.then(|| decode(&fact, &path))
    };
    let simple_modes = match (page, px4) {
        ("Flight Modes", false) => crate::vehicleconfig::page(backend, crate::vehicleconfig::SIMPLE_MODES, false)["sections"].as_array().cloned().unwrap_or_default(),
        _ => Vec::new(),
    };
    let shape = backend.value_fields("vehicle", "vtol,fixedWing");
    let (vtol, fixed_wing) = (crate::read::flag(&shape, "vtol"), crate::read::flag(&shape, "fixedWing"));
    let listed: Vec<Value> = sections
        .iter()
        .map(|s| json!({ "title": s.title, "note": s.note, "controls": s.parameters.iter().filter(|p| switch_applies(p, vtol, fixed_wing)).filter_map(|p| match p.strip_prefix(LABEL_ROW) {
            Some(text) => Some(json!({ "control": "label", "name": text, "label": text, "path": format!("{page}.{text}"), "enabled": true })),
            None => read(p).map(|row| servo_column_row(p, apm_mode_row(p, row, px4))),
        }).collect::<Vec<_>>() }))
        .chain(simple_modes)
        .filter(|s| s["controls"].as_array().is_some_and(|rows| rows.iter().any(|row| row["control"] != "label")))
        .collect();
    let fixed_channel = page == "Flight Modes" && !px4 && read("FLTMODE_CH").is_none() && read("MODE_CH").is_none();
    let listed: Vec<Value> = listed.into_iter().map(|mut section| {
        if fixed_channel && section["title"] == "Flight Mode Settings" {
            section["title"] = json!("Flight Mode Settings (Channel 5)");
        }
        section
    }).collect();
    json!({ "kind": "object", "class": "SetupPage", "page": page, "firmware": if px4 { "px4" } else { "apm" }, "available": !listed.is_empty(), "sections": listed })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apm_mode_rows_carry_qgc_channel_labels_and_pwm_ranges() {
        let option = apm_mode_row("RC9_OPTION", json!({ "label": "RC input option", "shortLabel": "" }), false);
        assert_eq!((option["label"].as_str(), option["shortLabel"].as_str()), (Some("Channel option 9"), Some("Channel option 9")), "APMFlightModesComponent titles each row by its channel");
        let slot = apm_mode_row("MODE6", json!({ "label": "Mode6", "shortLabel": "" }), false);
        assert_eq!((slot["shortLabel"].as_str(), slot["label"].as_str()), (Some("Flight Mode 6"), Some("PWM 1750 +")), "every firmware's slot reads Flight Mode N");
        let channel = apm_mode_row("MODE_CH", json!({ "label": "Mode channel", "shortLabel": "" }), false);
        assert_eq!(channel["label"], "Flight mode channel", "APMFlightModesComponent labels the channel combo");
        let assigned = apm_mode_row("FLTMODE_CH", json!({ "label": "", "value": 5, "valueString": "5", "control": "number", "options": [] }), false);
        let labels: Vec<&str> = assigned["options"].as_array().unwrap().iter().filter_map(|o| o["label"].as_str()).collect();
        assert_eq!((labels.first().copied(), labels.last().copied(), labels.len()), (Some("Not assigned"), Some("Channel 8"), 9), "modeChannelCombo model is Not assigned then Channel 1-8");
        assert_eq!((assigned["display"].as_str(), assigned["control"].as_str(), assigned["options"][5]["raw"].as_str()), (Some("Channel 5"), Some("choice"), Some("5")), "the combo index is the raw FLTMODE_CH value");
        assert_eq!(assigned["rawChoice"], true, "FLTMODE_CH has no enum metadata, so the head must write the raw value, not enumIndex");
        let off_list = apm_mode_row("FLTMODE_CH", json!({ "value": 12, "valueString": "12" }), false);
        assert_eq!(off_list["display"], "12");
        assert_eq!(apm_mode_row("FLTMODE1", json!({ "label": "x" }), true)["label"], "x", "PX4's page shows no PWM ranges");
    }

    #[test]
    fn setup_complete_only_asks_the_components_like_vehicle_summary() {
        assert!(setup_complete_of(&[("Radio".into(), false)]));
        assert!(!setup_complete_of(&[("Radio".into(), false), ("Sensors".into(), true)]));
    }

    #[test]
    fn px4_switches_follow_px4_flight_modes_qml() {
        assert!(!switch_applies("RC_MAP_TRANS_SW", false, true), "the transition switch is for a VTOL");
        assert!(switch_applies("RC_MAP_TRANS_SW", true, false));
        assert!(switch_applies("RC_MAP_FLAPS", false, true) && !switch_applies("RC_MAP_FLAPS", false, false), "flaps only for a plane");
        assert!(switch_applies("RC_MAP_KILL_SW", false, false));
        assert_eq!(FLIGHT_MODES_PX4[1].parameters[..6], ["RC_MAP_ARM_SW", "RC_MAP_GEAR_SW", "RC_MAP_KILL_SW", "RC_MAP_LOITER_SW", "RC_MAP_OFFB_SW", "RC_MAP_RETURN_SW"]);
    }

    #[test]
    fn an_acronym_component_is_named_in_lower_case_for_the_head() {
        assert_eq!(known_component(&json!({ "KnownVehicleComponent": 6 })).as_deref(), Some("esc"), "the head picks the ESC icon by esc, and eSC matched nothing");
        assert_eq!(known_component(&json!({ "KnownVehicleComponent": 1 })).as_deref(), Some("flightModes"));
    }

    #[test]
    fn apm_flight_modes_offers_every_switch_option_and_no_raw_simple_masks() {
        let parameters: Vec<&str> = FLIGHT_MODES_APM.iter().flat_map(|s| s.parameters.iter().copied()).collect();
        assert!((6..=16).all(|channel| parameters.contains(&format!("RC{channel}_OPTION").as_str())), "APMFlightModesComponent lists RC6 to RC16 options");
        assert!(!parameters.contains(&"SIMPLE"), "the Simple Mode choice writes SIMPLE and SUPER_SIMPLE; raw rows beside it disagreed with it");
        assert!(["MODE_CH", "MODE1", "MODE6"].iter().all(|name| parameters.contains(name)), "APMFlightModesComponentController switches to MODE_CH and MODE1-6 on ArduRover");
    }

    #[test]
    fn a_page_only_one_firmware_has_is_offered_only_to_that_firmware() {
        assert!(page_exists("Flight Behavior", true), "PX4AutoPilotPlugin constructs PX4FlightBehavior and nothing under APM does");
        assert!(!page_exists("Flight Behavior", false));
        ["Gimbal", "Lights", "Remote Support"].iter().for_each(|page| {
            assert!(page_exists(page, false), "{page} is registered by APMAutoPilotPlugin");
            assert!(!page_exists(page, true), "{page} has no component in PX4AutoPilotPlugin, so offering it made every head drop it silently");
        });
        ["Flight Safety", "Failsafes", "Logging"].iter().for_each(|page| {
            assert!(page_exists(page, false) && !page_exists(page, true), "{page} is an APMAutoPilotPlugin component with its own VehicleConfig definition");
        });
        assert!(page_exists("Safety", true) && !page_exists("Safety", false), "ArduPilot's safety component is Flight Safety; APMSafetyComponent is never constructed");
        ["Sensors", "Radio", "Flight Modes", "Power", "Motors", "Tuning", "Frame", "Parameters"].iter().for_each(|page| {
            assert!(page_exists(page, true), "{page} is registered by both plugins");
            assert!(page_exists(page, false));
        });
        [("Tuning", "PX4TuningComponent"), ("Frame", "AirframeComponent")].iter().for_each(|(page, component)| {
            assert!(sections_for(page, true).is_none(), "the core describes no PX4 parameters for {page}");
            assert!(page_exists(page, true), "but PX4AutoPilotPlugin constructs {component}, so the page is real on the desktop - which is why whether a page exists cannot be read off sections_for, and why searching filenames for \"Frame\" misses it");
        });
        assert_ne!(page_absence(true), page_absence(false), "the reason names the firmware that does have it, so a head can say which");
    }

    #[test]
    fn the_firmware_table_answers_only_what_a_firmware_can_never_have() {
        assert!(page_exists("Gimbal", false), "APMAutoPilotPlugin builds the gimbal component only when MNT1_TYPE exists, and Lights only for sub(); PX4 builds Flight Behavior only when SYS_VEHICLE_RESP exists");
        assert!(page_exists("Lights", false), "so an ArduPilot copter with no gimbal has neither, and this table cannot say so - a firmware flag cannot express a parameter or a vehicle type");
        assert!(!page_exists("Gimbal", true), "what it does say is sound in the other direction: PX4 has no gimbal component under any condition");
        assert!(!page_exists("Flight Behavior", false));
        assert!(page_exists("Remote Support", false), "and Remote Support is the one entry that is purely firmware - APM builds it unconditionally and PX4 has none");

        assert!(DEPS.contains(&"vehicle.autopilotPlugin.vehicleComponents"), "which is why view.components stays the authoritative list once a vehicle is connected: QGC has already evaluated every condition, per vehicle rather than per firmware");
    }

    #[test]
    fn readiness_reads_like_the_summary_page() {
        assert_eq!(readiness(false, true, &[], &[]).0, None, "no vehicle is no verdict; the same false that means \"checked and not ready\" drew an amber Check pill beside an instruction to connect one, an imperative verb with nothing behind it");
        assert_eq!(readiness(false, true, &[], &[]).1, "No vehicle connected");
        assert_eq!(readiness(true, true, &[], &[]).0, Some(false), "a connected vehicle reporting no components has been checked and is not ready, which is a different answer from having nothing to check");
        let ok = readiness(true, true, &[("Sensors".into(), false)], &[]);
        assert_eq!(ok, (Some(true), "Ready to fly".into(), "Setup complete and all enabled sensors are healthy.".into()));
        let two = readiness(true, true, &[("Sensors".into(), true), ("Radio".into(), true)], &[]);
        assert_eq!(two.1, "2 components need setup");
        assert_eq!(two.2, "Sensors, Radio");
        let faults = readiness(true, true, &[("Sensors".into(), false)], &["GPS".into()]);
        assert_eq!(faults.1, "1 sensor reporting a fault");
        assert_eq!(readiness(true, true, &[], &[]).1, "This vehicle reports no setup components");
    }

    #[test]
    fn pages_follow_the_firmware() {
        assert!(crate::vehicleconfig::has("Tuning", false), "APMTuningComponent reads APMTuningCopter.VehicleConfig.json");
        assert!(!crate::vehicleconfig::has("Tuning", true) && sections_for("Tuning", true).is_none());
        assert!(crate::vehicleconfig::has("Safety", true) && crate::vehicleconfig::has("Flight Safety", false));
        assert!(crate::vehicleconfig::has("Flight Behavior", true), "PX4FlightBehaviorCopter is a VehicleConfig page with enable switches");
    }

    #[test]
    fn a_page_says_whether_the_core_can_describe_it_and_never_whether_a_head_has_built_it() {
        let described = |page: &str, px4: bool| sections_for(page, px4).is_some();
        assert!(described("Heli", false), "the core can lay out APM heli servos as parameter sections");
        assert!(!described("Radio", false), "and it cannot lay out radio calibration, which is a screen rather than a list of parameters");
        assert!(!described("Motors", false), "nor motors, which is the page a head opened because this view once claimed it had one");
    }

    #[test]
    fn a_page_lists_only_parameters_the_vehicle_has() {
        struct Fake;
        impl Backend for Fake {
            fn get(&self, path: &str) -> String {
                match (path.contains("SERVO1_FUNCTION)") || path.contains("SERVO2_FUNCTION)"), path.contains("SERVO1_MIN")) {
                    (true, _) => json!({ "kind": "fact", "name": path.rsplit(',').next().unwrap().trim_end_matches(')'), "value": 30, "min": 0, "max": 100, "minIsDefaultForType": false, "maxIsDefaultForType": false }),
                    (false, true) => json!({ "kind": "fact", "name": "", "value": 0, "valueString": "0", "decimalPlaces": 3 }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "object", "px4Firmware": false, "apmFirmware": true }).to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let page = setup_view(&Fake, &["Heli".to_string()]);
        let sections = page["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 1, "a group of only subheadings is dropped");
        assert_eq!(sections[0]["title"], "Servo Setup");
        let rows: Vec<(&str, &str)> = sections[0]["controls"].as_array().unwrap().iter().map(|r| (r["name"].as_str().unwrap(), r["label"].as_str().unwrap_or(""))).collect();
        assert_eq!(rows[..4], [("Servo 1", "Servo 1"), ("SERVO1_FUNCTION", "Function"), ("Servo 2", "Servo 2"), ("SERVO2_FUNCTION", "Function")], "the Servo Setup grid: a row per servo, a column per field; a nameless answer is a parameter the vehicle does not have");
        assert_eq!(sections[0]["controls"][1]["control"], "number");
        assert_eq!(setup_view(&Fake, &["Nope".to_string()])["kind"], "null");
    }

    #[test]
    fn a_component_that_will_not_say_it_is_finished_is_not_finished() {
        let component = |json: Value| {
            let listed = json.get("elements").unwrap().as_array().unwrap();
            listed
                .iter()
                .filter_map(|c| {
                    let name = c.get("name").and_then(Value::as_str).filter(|n| !n.is_empty())?;
                    let needs = c.get("requiresSetup").and_then(Value::as_bool).unwrap_or(false) && !c.get("setupComplete").and_then(Value::as_bool).unwrap_or(false);
                    Some((name.to_string(), needs))
                })
                .collect::<Vec<_>>()
        };
        let silent = component(json!({ "elements": [ { "name": "Radio", "requiresSetup": true } ] }));
        assert_eq!(silent, vec![("Radio".to_string(), true)], "a component that declares it needs setup and will not say it is done counts as not done");
        let done = component(json!({ "elements": [ { "name": "Radio", "requiresSetup": true, "setupComplete": true } ] }));
        assert_eq!(done, vec![("Radio".to_string(), false)]);
        let irrelevant = component(json!({ "elements": [ { "name": "Summary" } ] }));
        assert_eq!(irrelevant, vec![("Summary".to_string(), false)], "a component that never asked for setup is not chased for it");
        assert_eq!(readiness(true, true, &silent, &[]).1, "1 component needs setup");
    }
}

#[cfg(test)]
mod components {
    use super::*;

    #[test]
    fn an_unfinished_airframe_or_radio_comes_first() {
        let part = |class: &str, name: &str, done: bool| Component { name: name.into(), class_name: class.into(), known: None, needs_attention: !done, blocked_reason: None, setup_complete: done };
        let all = vec![part("APMAirframeComponent", "Frame", false), part("APMRadioComponent", "Radio", false), part("APMFlightModesComponent", "Flight Modes", false), part("APMSensorsComponent", "Sensors", false)];
        assert_eq!(prerequisite(&all[2], &all, None).as_deref(), Some("Frame"));
        assert_eq!(prerequisite(&all[3], &all, None).as_deref(), Some("Frame"));
        let framed = vec![part("APMAirframeComponent", "Frame", true), part("APMRadioComponent", "Radio", false), part("APMFlightModesComponent", "Flight Modes", false)];
        assert_eq!(prerequisite(&framed[2], &framed, None).as_deref(), Some("Radio"));
        let px4 = vec![part("AirframeComponent", "Airframe", true), part("PX4RadioComponent", "Radio", false), part("FlightModesComponent", "Flight Modes", false)];
        assert_eq!(prerequisite(&px4[2], &px4, Some(1)), None, "no RC input, no radio needed");
        assert_eq!(prerequisite(&px4[2], &px4, Some(0)).as_deref(), Some("Radio"));
    }
    use crate::router::Backend;

    struct Vehicle {
        components: Vec<Value>,
    }

    impl Backend for Vehicle {
        fn get(&self, path: &str) -> String {
            match path {
                COMPONENTS => json!({ "kind": "value", "value": self.components.iter().map(|_| json!("QVariant(VehicleComponent*)")).collect::<Vec<_>>() }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path.strip_prefix(&format!("{COMPONENTS}.")).and_then(|index| index.parse::<usize>().ok()).and_then(|index| self.components.get(index)) {
                Some(component) => component.to_string(),
                None => match path {
                    "vehicle" => json!({ "kind": "object", "px4Firmware": true, "apmFirmware": false }).to_string(),
                    "vehicle.parameterManager" => json!({ "kind": "object", "parametersReady": true, "requestUnanswered": false }).to_string(),
                    _ => json!({ "kind": "null" }).to_string(),
                },
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    struct Params {
        ready: bool,
        unanswered: bool,
        connected: bool,
    }

    impl Backend for Params {
        fn get(&self, p: &str) -> String { self.get_fields(p, "") }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "vehicle" if self.connected => json!({ "kind": "object", "px4Firmware": false }).to_string(),
                "vehicle.parameterManager" => json!({
                    "kind": "object",
                    "parametersReady": self.ready,
                    "requestUnanswered": self.unanswered,
                }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_parameter_load_that_has_stopped_does_not_read_as_one_still_running() {
        let state = |ready, unanswered, connected| {
            let view = setup_view(&Params { ready, unanswered, connected }, &[]);
            (view["parametersReady"].as_bool().unwrap(), view["parametersReason"].as_str().unwrap().to_string())
        };

        assert_eq!(state(false, false, true), (false, "loading".to_string()));
        assert_eq!(
            state(false, true, true),
            (false, "unanswered".to_string()),
            "QGC gives up after five unanswered requests and says so only in a translated dialog, so parametersReady alone cannot tell a head whether to keep promising a load",
        );
        assert_eq!(state(true, false, true), (true, String::new()));
        assert_eq!(state(true, true, true), (true, String::new()), "a load that finished after a failed first attempt is ready, and the stale flag must not outrank it");
        assert_eq!(state(false, false, false), (false, "noVehicle".to_string()), "with no vehicle there is nothing to wait for");

        let waiting = setup_view(&Params { ready: false, unanswered: true, connected: true }, &[]);
        assert!(waiting["parametersText"].as_str().unwrap().contains("has not answered"), "the sentence states what the VEHICLE did; what to do about it is the head's line to write");
        assert!(setup_view(&Params { ready: false, unanswered: false, connected: true }, &[])["parametersText"].as_str().unwrap().is_empty());

        assert!(DEPS.contains(&"vehicle.parameterManager.requestUnanswered"), "without the dep the view never re-emits when the retries run out and the head waits forever anyway");
        assert!(DEPS.contains(&"vehicle.parameterManager.parametersReady"));
    }

    struct Flying {
        components: Vec<Value>,
        armed: bool,
        flying: bool,
        rover: bool,
    }

    impl Backend for Flying {
        fn get(&self, path: &str) -> String {
            match path {
                COMPONENTS => json!({ "kind": "value", "value": self.components.iter().map(|_| json!("QVariant(VehicleComponent*)")).collect::<Vec<_>>() }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path.strip_prefix(&format!("{COMPONENTS}.")).and_then(|index| index.parse::<usize>().ok()).and_then(|index| self.components.get(index)) {
                Some(component) => component.to_string(),
                None => match path {
                    "vehicle" => json!({ "kind": "object", "px4Firmware": true, "armed": self.armed, "flying": self.flying, "rover": self.rover }).to_string(),
                    _ => json!({ "kind": "null" }).to_string(),
                },
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn every_page_decides_whether_a_component_backs_it() {
        let named: Vec<&str> = PAGE_COMPONENTS.iter().map(|(page, _)| *page).collect();
        let missing: Vec<&&str> = PAGES.iter().flat_map(|(_, pages)| pages.iter()).filter(|page| !named.contains(page)).collect();
        assert!(missing.is_empty(), "a page nothing names reads as never blocked, which is the quiet direction to be wrong in: {missing:?}");
        assert_eq!(named.len(), PAGES.iter().map(|(_, pages)| pages.len()).sum::<usize>(), "and nothing is named that is not a page");
    }

    #[test]
    fn the_openable_field_carries_the_gate_and_not_only_the_rule_behind_it() {
        let gated = json!({ "kind": "object", "name": "Sensoren", "requiresSetup": true, "allowSetupWhileArmed": false, "allowSetupWhileFlying": false, "KnownVehicleComponent": "KnownSensorsVehicleComponent" });
        let permitted = json!({ "kind": "object", "name": "Safety", "requiresSetup": false, "allowSetupWhileArmed": true, "allowSetupWhileFlying": true, "KnownVehicleComponent": "UnknownVehicleComponent" });

        let parked = setup_view(&Flying { components: vec![gated.clone(), permitted.clone()], armed: false, flying: false, rover: false }, &[]);
        let resting = parked["components"].as_array().unwrap().clone();
        assert_eq!(resting[0]["openable"], true);
        assert_eq!(resting[0]["blockedReason"], Value::Null);

        let armed = setup_view(&Flying { components: vec![gated, permitted], armed: true, flying: false, rover: false }, &[]);
        let held = armed["components"].as_array().unwrap().clone();
        assert_eq!(held[0]["openable"], false, "a component that forbids setup while armed is not openable on an armed vehicle");
        assert_eq!(held[0]["blockedReason"], "armed");
        assert_eq!(held[0]["known"], "sensors", "the component's name is tr() wrapped, so a head keying its sensors badge on the word cannot find it in another locale - KnownVehicleComponent is an enum and is the same on both firmwares, where className is APMSensorsComponent on one and PX4 on the other");
        let sensors_page = armed["groups"].as_array().unwrap().iter()
            .flat_map(|g| g["pages"].as_array().unwrap().clone())
            .find(|p| p["name"] == "Sensors").unwrap();
        assert_eq!(sensors_page["openable"], false, "the page a head disables is the thing it needs answered, and it had been joining page to component on a tr() wrapped name - so six pages gated in English only");
        assert_eq!(sensors_page["blockedReason"], "armed");
        let summary_page = armed["groups"].as_array().unwrap().iter()
            .flat_map(|g| g["pages"].as_array().unwrap().clone())
            .find(|p| p["name"] == "Summary").unwrap();
        assert_eq!(summary_page["openable"], true, "Summary is backed by no component at all, and QGC shows it on an armed vehicle - blocking every page whose component cannot be identified would have disabled three that were never gated");

        assert_eq!(held[1]["known"], Value::Null, "a firmware-specific component ANSWERS UnknownVehicleComponent, and that is no identity rather than an identity spelled unknown that a head could key on");
        assert_eq!(held[1]["openable"], true, "the one beside it permits it, so the gate is per component rather than per vehicle");
        assert_eq!(held[1]["blockedReason"], Value::Null, "a reason and an openable that disagree would be worse than either alone");
        assert_eq!(held[0]["needsAttention"], true, "being blocked does not stop a component still needing setup, and a head shows both");
    }

    fn component(name: &str, requires: bool, complete: bool) -> Value {
        json!({ "kind": "object", "name": name, "requiresSetup": requires, "setupComplete": complete })
    }

    #[test]
    fn a_list_of_components_is_read_from_the_value_the_bridge_answers_with() {
        let vehicle = Vehicle {
            components: vec![
                component("Airframe", true, true),
                component("Sensors", true, false),
                component("Radio", true, true),
                component("Camera", false, false),
            ],
        };
        let read = vehicle_components(&vehicle);
        assert_eq!(read.len(), 4, "vehicleComponents is a plain list property, so the bridge answers a value rather than an object with elements; reading elements found nothing on every vehicle there has ever been");
        assert_eq!((read[1].name.as_str(), read[1].needs_attention), ("Sensors", true), "a component that requires setup and has not had it is the one that holds the vehicle back");
        assert_eq!((read[0].name.as_str(), read[0].needs_attention), ("Airframe", false));
        assert_eq!((read[3].name.as_str(), read[3].needs_attention), ("Camera", false), "a component that does not require setup is never outstanding");
    }

    #[test]
    fn a_vehicle_with_components_can_reach_ready_to_fly() {
        let vehicle = Vehicle { components: vec![component("Airframe", true, true), component("Radio", true, true)] };
        let view = setup_view(&vehicle, &[]);
        assert_eq!(view["ready"], true, "every branch but the empty one was unreachable while the read found nothing, so a configured vehicle could never say it was ready");
        assert!(!view["headline"].as_str().unwrap().contains("no setup components"));
        assert_eq!(view["components"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn a_vehicle_that_really_reports_nothing_still_says_so() {
        let view = setup_view(&Vehicle { components: Vec::new() }, &[]);
        assert_eq!(view["ready"], false);
        assert!(view["headline"].as_str().unwrap().contains("no setup components"), "the empty case is a real answer for a vehicle that gives one, which is why the defect was invisible");
    }

    #[test]
    fn a_component_with_no_name_is_not_counted() {
        let vehicle = Vehicle { components: vec![component("", true, false), component("Sensors", true, false)] };
        let read = vehicle_components(&vehicle);
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].name, "Sensors");
    }

    #[test]
    fn a_page_qgc_builds_is_a_page_this_catalogue_offers() {
        for page in ["Actuators", "Heli", "Follow Me", "WiFi Bridge", "Syslink"] {
            assert!(PAGES.iter().any(|(_, pages)| pages.contains(&page)), "{page} is built by a plugin and belongs to no group");
            assert!(PAGE_COMPONENTS.iter().any(|(name, keys)| *name == page && !keys.is_empty()), "{page} names no component, so it can never be backed");
        }
        let motors = PAGE_COMPONENTS.iter().find(|(name, _)| *name == "Motors").unwrap().1;
        assert!(motors.contains(&"APMMotorComponent"), "an ArduPilot vehicle builds APMMotorComponent, and Motors listed only the PX4 class");
    }

    #[test]
    fn whether_a_page_can_be_opened_is_the_vehicles_answer_and_not_the_screens() {
        let permits = |armed: bool, flying: bool| json!({ "allowSetupWhileArmed": armed, "allowSetupWhileFlying": flying });
        let strict = permits(false, false);

        assert_eq!(blocked_by(&strict, false, false, false), None, "a vehicle sitting on the ground blocks nothing");
        assert_eq!(blocked_by(&strict, true, false, false), Some("armed"));
        assert_eq!(blocked_by(&strict, false, true, false), Some("flying"));
        assert_eq!(blocked_by(&strict, true, true, false), Some("armed"), "SetupPage names armed first when both hold, and a head choosing the other one would tell the operator to land when disarming is what is wanted");

        assert_eq!(blocked_by(&permits(true, false), true, false, false), None, "a component that says it can be set up while armed is the whole point of the flag");
        assert_eq!(blocked_by(&strict, false, true, true), None, "a rover's flying flag means nothing, and a head reading the raw permissions would have to know that too");
        assert_eq!(blocked_by(&strict, true, true, true), Some("armed"), "the rover exemption is only about flying");
    }

    #[test]
    fn a_verdict_is_withheld_until_the_vehicle_has_answered_for_its_parameters() {
        struct Silent;
        impl Backend for Silent {
            fn get(&self, _p: &str) -> String { json!({ "kind": "null" }).to_string() }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "vehicle" => json!({ "kind": "object", "px4Firmware": true }).to_string(),
                    "vehicle.parameterManager" => json!({ "kind": "object", "parametersReady": false, "requestUnanswered": false }).to_string(),
                    _ => json!({ "kind": "null" }).to_string(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        let view = setup_view(&Silent, &[]);
        assert_eq!(view["ready"], Value::Null, "needs_attention is parameter-derived and getParameter hands back a default Fact reading zero for one it does not hold, so a component reports needing setup on evidence the vehicle never sent - and the head drew Not ready to fly from it, on every connect, while the parameters were still arriving");
        assert_eq!(view["parametersReady"], false, "and the same payload says why, so a head can put the reason where the verdict would have been");
        assert!(view["headline"].as_str().unwrap().contains("parameters"));

        assert_eq!(readiness(true, true, &[("Radio".into(), false)], &[]).0, Some(true), "once the parameters are in, a verdict is owed");
        let waiting = readiness(true, false, &[("Radio".into(), false)], &[]);
        assert_eq!(waiting.0, None);
        assert_eq!(waiting.1, "Waiting for this vehicle's parameters", "connected and parameters_ready are adjacent bools, so transposing them at the call site still compiles and still returns None - only the headline tells the two apart");
    }

    #[test]
    fn heli_is_an_ardupilot_page_listing_the_first_eight_servos_then_the_rotor() {
        assert!(!page_exists("Heli", true));
        let titles: Vec<&str> = sections_for("Heli", false).unwrap().iter().map(|s| s.title).collect();
        assert_eq!(titles, ["Servo Setup", "Swashplate Setup", "Throttle Settings", "Governor Settings", "Miscellaneous Settings"], "APMHeliComponent QGCGroupBox titles");
    }

    struct Typed(&'static str);

    impl Backend for Typed {
        fn get(&self, p: &str) -> String { self.get_fields(p, "") }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "vehicle" => json!({ "kind": "object", "px4Firmware": false, "apmFirmware": true, "vehicleTypeString": self.0 }).to_string(),
                "vehicle.parameterManager" => json!({ "kind": "object", "parametersReady": true, "requestUnanswered": false }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn screen_of(view: &Value, name: &str) -> Value {
        view["groups"].as_array().unwrap().iter().flat_map(|g| g["pages"].as_array().unwrap().clone()).find(|p| p["name"] == name).map(|p| p["screen"].clone()).unwrap_or(Value::Null)
    }

    #[test]
    fn apm_safety_pages_are_not_supported_on_a_vehicle_type_qgc_leaves_out() {
        let boat = setup_view(&Typed("Surface vessel, boat, ship"), &[]);
        assert_eq!(screen_of(&boat, "Flight Safety"), NOT_SUPPORTED_SCREEN, "APMFlightSafetyComponent::setupSource falls back to APMNotSupported.qml");
        assert_eq!(screen_of(&boat, "Failsafes"), NOT_SUPPORTED_SCREEN);
        assert_eq!(screen_of(&boat, "Power"), Value::Null, "only the two pages with a vehicle type switch");
        let quadplane = setup_view(&Typed("VTOL Fixedrotor"), &[]);
        assert_eq!(screen_of(&quadplane, "Failsafes"), NOT_SUPPORTED_SCREEN);
        ["Quadrotor", "Fixed wing aircraft", "Ground rover", "Submarine", "trirotor"].iter().for_each(|kind| {
            assert_eq!(screen_of(&setup_view(&Typed(kind), &[]), "Flight Safety"), Value::Null, "{kind} has the generated page");
        });
    }
}
