use serde_json::{Value, json};

use crate::control::decode;
use crate::read::{flag, object};
use crate::router::Backend;
use crate::sensors;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.parameterManager.parametersReady", "vehicle.parameterManager.requestUnanswered", "vehicle.autopilotPlugin.vehicleComponents", "vehicle.sysStatusSensorInfo.sensorNames", "vehicle.sysStatusSensorInfo.sensorStatus", "vehicle.armed", "vehicle.flying", "vehicle.rover", "vehicle.px4Firmware", "vehicle.apmFirmware"];

const PX4_ONLY: &[&str] = &["Flight Behavior"];
const APM_ONLY: &[&str] = &["Camera", "Lights", "Remote Support"];

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
    ("Power", &["known:power"]),
    ("Frame", &["AirframeComponent", "APMAirframeComponent", "APMSubFrameComponent"]),
    ("Motors", &["MotorComponent", "APMMotorComponent"]),
    ("Tuning", &["APMTuningComponent", "PX4TuningComponent"]),
    ("Camera", &["APMCameraComponent"]),
    ("Lights", &["APMLightsComponent"]),
    ("Flight Behavior", &["PX4FlightBehavior"]),
    ("Remote Support", &["APMRemoteSupportComponent"]),
    // Five pages QGC constructs and this catalogue never offered, so no head could draw them
    // however willing: a page absent here is not a page a head declined. Keyed on the class the
    // plugin builds rather than on the name it shows, because the name is tr()-wrapped.
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
    ("Setup", &["Sensors", "Radio", "Frame", "Flight Modes", "Safety", "Power", "Motors", "Actuators", "Heli", "Tuning", "Camera", "Lights", "Flight Behavior", "Follow Me"]),
    ("Advanced", &["Remote Support", "WiFi Bridge", "Syslink", "Parameters"]),
];

pub struct Section {
    pub title: &'static str,
    pub note: &'static str,
    pub parameters: &'static [&'static str],
}

const SAFETY_APM: &[Section] = &[
    Section { title: "Throttle and link failsafe", note: "What the vehicle does when it stops hearing from the transmitter or the ground station.", parameters: &["FS_THR_ENABLE", "FS_THR_VALUE", "FS_GCS_ENABLE", "FS_OPTIONS"] },
    Section { title: "Battery failsafe", note: "Thresholds and the action taken when the pack runs low.", parameters: &["BATT_MONITOR", "BATT_FS_LOW_ACT", "BATT_FS_CRT_ACT", "BATT_LOW_VOLT", "BATT_LOW_MAH", "BATT_CRT_VOLT", "BATT_CRT_MAH"] },
    Section { title: "Second battery", note: "", parameters: &["BATT2_MONITOR", "BATT2_FS_LOW_ACT", "BATT2_FS_CRT_ACT", "BATT2_LOW_VOLT", "BATT2_LOW_MAH", "BATT2_CRT_VOLT", "BATT2_CRT_MAH"] },
    Section { title: "Geofence", note: "The boundary the vehicle will not cross, and what it does at the edge.", parameters: &["FENCE_ENABLE", "FENCE_TYPE", "FENCE_ACTION", "FENCE_ALT_MAX", "FENCE_RADIUS", "FENCE_MARGIN"] },
    Section { title: "Return and land", note: "The path home when a failsafe or the operator triggers a return.", parameters: &["RTL_ALT", "RTL_ALT_FINAL", "RTL_LOIT_TIME", "LAND_SPEED"] },
    Section { title: "Arming", note: "Which pre-arm checks must pass before the vehicle will arm.", parameters: &["ARMING_CHECK"] },
];
const SAFETY_PX4: &[Section] = &[
    Section { title: "Link failsafe", note: "What the vehicle does when it stops hearing from the transmitter or the ground station.", parameters: &["NAV_RCL_ACT", "COM_RC_LOSS_T", "NAV_DLL_ACT", "COM_DL_LOSS_T"] },
    Section { title: "Battery failsafe", note: "Thresholds and the action taken when the pack runs low.", parameters: &["COM_LOW_BAT_ACT", "BAT_LOW_THR", "BAT_CRIT_THR", "BAT_EMERGEN_THR"] },
    Section { title: "Geofence", note: "The boundary the vehicle will not cross, and what it does at the edge.", parameters: &["GF_ACTION", "GF_MAX_HOR_DIST", "GF_MAX_VER_DIST"] },
    Section { title: "Return and land", note: "The path home when a failsafe or the operator triggers a return.", parameters: &["RTL_RETURN_ALT", "RTL_DESCEND_ALT", "RTL_LAND_DELAY", "MPC_LAND_SPEED", "COM_DISARM_LAND"] },
];
const POWER_APM: &[Section] = &[
    Section { title: "Battery 1", note: "How the pack is measured. Compare the readings against a meter and correct the multipliers until they agree.", parameters: &["BATT_MONITOR", "BATT_CAPACITY", "BATT_VOLT_PIN", "BATT_CURR_PIN", "BATT_VOLT_MULT", "BATT_AMP_PERVLT", "BATT_AMP_OFFSET", "BATT_ARM_VOLT"] },
    Section { title: "Battery 2", note: "A second pack. The rest of its settings appear once a monitor is chosen.", parameters: &["BATT2_MONITOR", "BATT2_CAPACITY", "BATT2_VOLT_PIN", "BATT2_CURR_PIN", "BATT2_VOLT_MULT", "BATT2_AMP_PERVLT", "BATT2_AMP_OFFSET", "BATT2_ARM_VOLT"] },
];
const POWER_PX4: &[Section] = &[
    Section { title: "Battery", note: "", parameters: &["BAT_N_CELLS", "BAT_V_CHARGED", "BAT_V_EMPTY", "BAT_CAPACITY", "BAT1_N_CELLS", "BAT1_V_CHARGED", "BAT1_V_EMPTY", "BAT1_CAPACITY"] },
    Section { title: "Sensor calibration", note: "Measured during calibration. The calibration wizard is not here yet, so these are the raw values it would write.", parameters: &["BAT_V_DIV", "BAT_A_PER_V", "BAT1_V_DIV", "BAT1_A_PER_V"] },
];
const TUNING_APM: &[Section] = &[
    Section { title: "Stick feel", note: "How long the vehicle takes to follow the stick. Shorter is crisper, longer is softer.", parameters: &["ATC_INPUT_TC"] },
    Section { title: "Angle gains", note: "How hard the controller leans to reach the angle the stick asks for.", parameters: &["ATC_ANG_RLL_P", "ATC_ANG_PIT_P", "ATC_ANG_YAW_P"] },
    Section { title: "Rate gains", note: "How hard it works to hold that rate once it is turning. Raise until the vehicle is crisp, then back off before it oscillates.", parameters: &["ATC_RAT_RLL_P", "ATC_RAT_RLL_I", "ATC_RAT_RLL_D", "ATC_RAT_PIT_P", "ATC_RAT_PIT_I", "ATC_RAT_PIT_D", "ATC_RAT_YAW_P", "ATC_RAT_YAW_I"] },
    Section { title: "Climb", note: "How aggressively the vehicle chases a change in height.", parameters: &["PSC_ACCZ_P", "PSC_ACCZ_I"] },
    Section { title: "Motor thrust", note: "Minimum thrust should sit above spin-while-armed, or the vehicle cannot move once it is armed.", parameters: &["MOT_SPIN_ARM", "MOT_SPIN_MIN", "MOT_THST_HOVER"] },
];
const FRAME_APM: &[Section] = &[
    Section { title: "Airframe", note: "The class picks the layout, the type picks how its arms are oriented. Changing either changes motor numbering and direction; re-check motor order before flying.", parameters: &["FRAME_CLASS", "FRAME_TYPE"] },
];
const FLIGHT_MODES_APM: &[Section] = &[
    Section { title: "Mode switch channel", note: "", parameters: &["FLTMODE_CH"] },
    Section { title: "Mode slots", note: "", parameters: &["FLTMODE1", "FLTMODE2", "FLTMODE3", "FLTMODE4", "FLTMODE5", "FLTMODE6"] },
    Section { title: "Options", note: "", parameters: &["SIMPLE", "SUPER_SIMPLE", "INITIAL_MODE"] },
];
const FLIGHT_MODES_PX4: &[Section] = &[
    Section { title: "Mode switch channel", note: "", parameters: &["RC_MAP_FLTMODE"] },
    Section { title: "Mode slots", note: "", parameters: &["COM_FLTMODE1", "COM_FLTMODE2", "COM_FLTMODE3", "COM_FLTMODE4", "COM_FLTMODE5", "COM_FLTMODE6"] },
    Section { title: "Single function switches", note: "", parameters: &["RC_MAP_RETURN_SW", "RC_MAP_KILL_SW", "RC_MAP_ARM_SW", "RC_MAP_LOITER_SW", "RC_MAP_OFFB_SW", "RC_MAP_GEAR_SW", "RC_MAP_TRANS_SW"] },
];
const CAMERA_APM: &[Section] = &[
    Section { title: "Gimbal", note: "Choose a mount type and its own settings appear under MNT1 in Parameters.", parameters: &["MNT_TYPE", "MNT1_TYPE", "MNT2_TYPE", "MNT_DEFLT_MODE"] },
    Section { title: "Camera", note: "Choose a camera type and its trigger settings appear under CAM1 in Parameters.", parameters: &["CAM1_TYPE", "CAM2_TYPE"] },
    Section { title: "Triggering", note: "How photos are taken, whichever camera is wired.", parameters: &["CAM_AUTO_ONLY", "CAM_MAX_ROLL", "CAM_RC_TYPE"] },
    Section { title: "Angle limits", note: "", parameters: &["MNT_ANGMIN_PAN", "MNT_ANGMAX_PAN", "MNT_ANGMIN_ROL", "MNT_ANGMAX_ROL", "MNT_ANGMIN_TIL", "MNT_ANGMAX_TIL"] },
    Section { title: "Neutral angles", note: "", parameters: &["MNT_NEUTRAL_X", "MNT_NEUTRAL_Y", "MNT_NEUTRAL_Z"] },
    Section { title: "Retract angles", note: "", parameters: &["MNT_RETRACT_X", "MNT_RETRACT_Y", "MNT_RETRACT_Z"] },
    Section { title: "Stabilisation", note: "", parameters: &["MNT_STAB_PAN", "MNT_STAB_ROLL", "MNT_STAB_TILT"] },
    Section { title: "RC input", note: "", parameters: &["MNT_RC_IN_PAN", "MNT_RC_IN_ROLL", "MNT_RC_IN_TILT"] },
];
const LIGHTS_APM: &[Section] = &[
    Section { title: "Light channels", note: "", parameters: &["SERVO5_FUNCTION", "SERVO6_FUNCTION", "SERVO7_FUNCTION", "SERVO8_FUNCTION", "SERVO9_FUNCTION", "SERVO10_FUNCTION", "SERVO11_FUNCTION", "SERVO12_FUNCTION", "SERVO13_FUNCTION", "SERVO14_FUNCTION", "SERVO15_FUNCTION", "SERVO16_FUNCTION"] },
    Section { title: "Brightness steps", note: "", parameters: &["JS_LIGHTS_STEPS", "JS_LIGHTS_STEP", "BRD_PWM_COUNT"] },
];
const FLIGHT_BEHAVIOR_PX4: &[Section] = &[
    Section { title: "Responsiveness", note: "", parameters: &["SYS_VEHICLE_RESP", "MPC_XY_VEL_ALL", "MPC_Z_VEL_ALL"] },
];

pub fn sections_for(page: &str, px4: bool) -> Option<&'static [Section]> {
    match (page, px4) {
        ("Safety", true) => Some(SAFETY_PX4),
        ("Safety", false) => Some(SAFETY_APM),
        ("Power", true) => Some(POWER_PX4),
        ("Power", false) => Some(POWER_APM),
        ("Tuning", false) => Some(TUNING_APM),
        ("Frame", false) => Some(FRAME_APM),
        ("Flight Modes", true) => Some(FLIGHT_MODES_PX4),
        ("Flight Modes", false) => Some(FLIGHT_MODES_APM),
        ("Lights", false) => Some(LIGHTS_APM),
        ("Camera", false) => Some(CAMERA_APM),
        ("Flight Behavior", true) => Some(FLIGHT_BEHAVIOR_PX4),
        _ => None,
    }
}

pub fn readiness(connected: bool, parameters_ready: bool, components: &[(String, bool)], sensor_faults: &[String]) -> (Option<bool>, String, String) {
    let outstanding: Vec<&str> = components.iter().filter(|(_, needs)| *needs).map(|(n, _)| n.as_str()).collect();
    if !connected {
        return (None, "No vehicle connected".into(), "Connect a vehicle to check what it needs.".into());
    }
    // needs_attention is parameter-derived, and ParameterManager hands back a default Fact reading
    // zero for a parameter it does not hold - so with parameters outstanding a component reports
    // needing setup on evidence the vehicle never sent. The Option exists because no vehicle is no
    // verdict; parameters not yet answered is the same thing and reached the same false.
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
    let vehicle = object(&backend.get_fields("vehicle", "px4Firmware"));
    let connected = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let px4 = flag(&vehicle, "px4Firmware");
    match args.first() {
        Some(page) => page_json(backend, page, px4),
        None => overview(backend, connected, px4),
    }
}

const COMPONENTS: &str = "vehicle.autopilotPlugin.vehicleComponents";

pub struct Component {
    pub name: String,
    pub class_name: String,
    pub known: Option<String>,
    pub needs_attention: bool,
    pub blocked_reason: Option<&'static str>,
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
        false => Some(trimmed[..1].to_lowercase() + &trimmed[1..]),
    }
}

const KNOWN_COMPONENTS: [&str; 6] = [
    "KnownRadioVehicleComponent",
    "KnownFlightModesVehicleComponent",
    "KnownSensorsVehicleComponent",
    "KnownSafetyVehicleComponent",
    "KnownPowerVehicleComponent",
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

fn vehicle_components(backend: &dyn Backend) -> Vec<Component> {
    let state = object(&backend.get_fields("vehicle", "armed,flying,rover"));
    let (armed, flying, rover) = (flag(&state, "armed"), flag(&state, "flying"), flag(&state, "rover"));
    let listed = object(&backend.get(COMPONENTS));
    let count = listed.get("value").and_then(Value::as_array).map(|elements| elements.len()).unwrap_or(0);
    (0..count)
        .filter_map(|index| {
            let component = object(&backend.get_fields(&format!("{COMPONENTS}.{index}"), "name,requiresSetup,setupComplete,allowSetupWhileArmed,allowSetupWhileFlying,KnownVehicleComponent"));
            let name = component.get("name").and_then(Value::as_str).filter(|name| !name.is_empty())?;
            Some(Component {
                name: name.to_string(),
                class_name: component.get("class").and_then(Value::as_str).unwrap_or_default().to_string(),
                known: known_component(&component),
                needs_attention: flag(&component, "requiresSetup") && !flag(&component, "setupComplete"),
                blocked_reason: blocked_by(&component, armed, flying, rover),
            })
        })
        .collect()
}

fn parameter_state(backend: &dyn Backend, connected: bool) -> (bool, &'static str, &'static str) {
    if !connected {
        return (false, "noVehicle", "");
    }
    let manager = object(&backend.get_fields("vehicle.parameterManager", "parametersReady,requestUnanswered"));
    match (flag(&manager, "parametersReady"), flag(&manager, "requestUnanswered")) {
        (true, _) => (true, "", ""),
        (false, true) => (false, "unanswered", "This vehicle has not answered the request for its parameters, and the retries are finished."),
        (false, false) => (false, "loading", ""),
    }
}

fn overview(backend: &dyn Backend, connected: bool, px4: bool) -> Value {
    let components = vehicle_components(backend);
    let faults: Vec<String> = sensors::sensors(&object(&backend.get("vehicle.sysStatusSensorInfo"))).into_iter().filter(|(_, s)| *s == "unhealthy").map(|(n, _)| n).collect();
    let named: Vec<(String, bool)> = components.iter().map(|c| (c.name.clone(), c.needs_attention)).collect();
    let (parameters_ready, parameters_reason, parameters_text) = parameter_state(backend, connected);
    let (ready, headline, detail) = readiness(connected, parameters_ready, &named, &faults);
    json!({
        "kind": "object",
        "class": "VehicleSetup",
        "connected": connected,
        "parametersReady": parameters_ready,
        "parametersReason": parameters_reason,
        "parametersText": parameters_text,
        "firmware": if !connected { "none" } else if px4 { "px4" } else { "apm" },
        "ready": ready,
        "headline": headline,
        "detail": detail,
        "components": components.iter().map(|c| json!({
            "name": c.name,
            "className": c.class_name,
            "known": c.known,
            "needsAttention": c.needs_attention,
            "openable": c.blocked_reason.is_none(),
            "blockedReason": c.blocked_reason,
        })).collect::<Vec<_>>(),
        "groups": PAGES.iter().map(|(title, pages)| json!({
            "title": title,
            "pages": pages.iter().filter(|p| page_exists(p, px4)).map(|p| {
                let blocked = page_block(p, &components);
                json!({ "name": p, "parameterSections": sections_for(p, px4).is_some(), "openable": blocked.is_none(), "blockedReason": blocked })
            }).collect::<Vec<_>>(),
            "omitted": pages.iter().filter(|p| !page_exists(p, px4)).map(|p| json!({ "name": p, "reason": page_absence(px4) })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

fn page_json(backend: &dyn Backend, page: &str, px4: bool) -> Value {
    let Some(sections) = sections_for(page, px4) else { return crate::read::refused(&format!("no setup page is called {page} for this firmware; the pages a vehicle offers depend on which plugin built them")) };
    let read = |name: &str| {
        let path = format!("vehicle.parameterManager.getParameter(-1,{name})");
        let fact = object(&backend.get(&path));
        let present = fact.get("kind").and_then(Value::as_str) == Some("fact") && fact.get("name").and_then(Value::as_str).is_some_and(|n| !n.is_empty());
        present.then(|| decode(&fact, &path))
    };
    let shown = |s: &Section, controls: Vec<Value>| -> Vec<Value> {
        let Some(monitor) = s.parameters.first().filter(|p| p.ends_with("_MONITOR")) else { return controls };
        let chosen = |c: &Value| c["name"] != **monitor || c["value"].as_f64() != Some(0.0);
        match controls.iter().all(chosen) {
            true => controls,
            false => controls.into_iter().filter(|c| c["name"] == **monitor).collect(),
        }
    };
    let listed: Vec<Value> = sections
        .iter()
        .map(|s| json!({ "title": s.title, "note": s.note, "controls": shown(s, s.parameters.iter().filter_map(|p| read(p)).collect::<Vec<_>>()) }))
        .filter(|s| !s["controls"].as_array().unwrap().is_empty())
        .collect();
    json!({ "kind": "object", "class": "SetupPage", "page": page, "firmware": if px4 { "px4" } else { "apm" }, "available": !listed.is_empty(), "sections": listed })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_only_one_firmware_has_is_offered_only_to_that_firmware() {
        assert!(page_exists("Flight Behavior", true), "PX4AutoPilotPlugin constructs PX4FlightBehavior and nothing under APM does");
        assert!(!page_exists("Flight Behavior", false));
        ["Camera", "Lights", "Remote Support"].iter().for_each(|page| {
            assert!(page_exists(page, false), "{page} is registered by APMAutoPilotPlugin");
            assert!(!page_exists(page, true), "{page} has no component in PX4AutoPilotPlugin, so offering it made every head drop it silently");
        });
        ["Sensors", "Radio", "Flight Modes", "Safety", "Power", "Motors", "Tuning", "Frame", "Parameters"].iter().for_each(|page| {
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
        assert!(page_exists("Camera", false), "APMAutoPilotPlugin builds the camera component only when MNT1_TYPE exists, and Lights only for sub(); PX4 builds Flight Behavior only when SYS_VEHICLE_RESP exists");
        assert!(page_exists("Lights", false), "so an ArduPilot copter with no gimbal has neither, and this table cannot say so - a firmware flag cannot express a parameter or a vehicle type");
        assert!(!page_exists("Camera", true), "what it does say is sound in the other direction: PX4 has no camera component under any condition");
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
        assert!(sections_for("Tuning", false).is_some());
        assert!(sections_for("Tuning", true).is_none());
        assert!(sections_for("Flight Behavior", true).is_some());
    }

    #[test]
    fn a_page_says_whether_the_core_can_describe_it_and_never_whether_a_head_has_built_it() {
        let described = |page: &str, px4: bool| sections_for(page, px4).is_some();
        assert!(described("Safety", false), "the core can lay out APM safety as parameter sections");
        assert!(!described("Radio", false), "and it cannot lay out radio calibration, which is a screen rather than a list of parameters");
        assert!(!described("Motors", false), "nor motors, which is the page a head opened because this view once claimed it had one");
    }

    #[test]
    fn a_page_lists_only_parameters_the_vehicle_has() {
        struct Fake;
        impl Backend for Fake {
            fn get(&self, path: &str) -> String {
                match (path.contains("RTL_ALT)") || path.contains("ARMING_CHECK"), path.contains("LAND_SPEED")) {
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
        let page = setup_view(&Fake, &["Safety".to_string()]);
        let sections = page["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0]["title"], "Return and land");
        assert_eq!(sections[0]["controls"][0]["name"], "RTL_ALT");
        assert_eq!(sections[0]["controls"].as_array().unwrap().len(), 1);
        assert_eq!(sections[0]["controls"][0]["control"], "number");
        assert_eq!(sections[1]["title"], "Arming");
        assert_eq!(setup_view(&Fake, &["Nope".to_string()])["kind"], "null");
    }

    #[test]
    fn a_battery_section_offers_only_its_monitor_until_one_is_chosen() {
        struct Packs(i64);
        impl Backend for Packs {
            fn get(&self, path: &str) -> String {
                let name = path.rsplit(',').next().unwrap().trim_end_matches(')');
                match name.starts_with("BATT") {
                    true => json!({ "kind": "fact", "name": name, "value": if name.ends_with("_MONITOR") { self.0 } else { 7 }, "valueString": "7" }),
                    false => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "object", "px4Firmware": false, "apmFirmware": true }).to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let controls = |monitor: i64| {
            let page = setup_view(&Packs(monitor), &["Power".to_string()]);
            page["sections"].as_array().unwrap().iter()
                .map(|s| s["controls"].as_array().unwrap().len())
                .collect::<Vec<_>>()
        };

        assert_eq!(controls(0), vec![1, 1], "a pack with no monitor has nothing to calibrate, and QGC hides the rest too");
        assert_eq!(controls(4), vec![8, 8], "once one is chosen every setting for that pack is offered");
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
        // Five pages QGC constructs were absent here, so both heads read as declining to draw
        // them when nothing had ever offered them. A page missing from the catalogue is invisible
        // in exactly the direction that looks like head debt.
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
}
