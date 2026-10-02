use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Value, json};

use crate::read::{flag, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.parameterManager.parametersReady", "vehicle.autopilotPlugin.vehicleComponents", "vehicle.multiRotor", "vehicle.fixedWing", "vehicle.rover", "vehicle.sub", "vehicle.apmFirmware", "vehicle.firmwareMajorVersion", "vehicle.firmwareMinorVersion", "vehicle.firmwarePatchVersion", "vehicle.firmwareVersionTypeString", "vehicle.gitHash", "vehicle.firmwareCustomMajorVersion", "vehicle.firmwareCustomMinorVersion", "vehicle.firmwareCustomPatchVersion"];
const COMPONENTS: &str = "vehicle.autopilotPlugin.vehicleComponents";
const SETUP_REQUIRED: &str = "Setup required";
const READY: &str = "Ready";
const DISABLED: &str = "Disabled";
const NOT_AVAILABLE: &str = "N/A";
const MAX_BATTERIES: usize = 16;
const LIGHTS_CHANNELS: std::ops::RangeInclusive<i64> = 5..=14;
const LIGHTS_1_FUNCTION: f64 = 59.0;
const LIGHTS_2_FUNCTION: f64 = 60.0;
const DSHOT_FIRST_TYPE: f64 = 4.0;

static LEVEL_SUFFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)at\s+(critical|emergency)\s+level").expect("level suffix pattern"));

pub struct Vehicle {
    pub multi_rotor: bool,
    pub fixed_wing: bool,
    pub rover: bool,
    pub sub: bool,
    pub version: (i64, i64, i64),
    pub firmware: String,
    pub firmware_type: String,
    pub git_hash: String,
    pub custom: Option<String>,
}

pub type Facts<'a> = &'a dyn Fn(&str) -> Option<Value>;
type Rows = Vec<(String, String)>;

fn row(label: &str, value: impl Into<String>) -> (String, String) {
    (label.to_string(), value.into())
}

fn number(fact: &Value) -> f64 {
    fact.get("rawValue").or(fact.get("value")).and_then(Value::as_f64).unwrap_or(0.0)
}

fn enum_text(fact: &Value) -> String {
    text(fact, "enumStringValue")
}

fn with_units(fact: &Value) -> String {
    format!("{} {}", text(fact, "valueString"), text(fact, "units"))
}

fn enum_of(facts: Facts, name: &str) -> String {
    facts(name).map(|f| enum_text(&f)).unwrap_or_default()
}

fn ready_unless_zero(facts: Facts, name: &str) -> String {
    facts(name).map(|f| if number(&f) == 0.0 { SETUP_REQUIRED } else { READY }.to_string()).unwrap_or_default()
}

pub fn battery_prefix(index: usize) -> String {
    match index {
        0 => "BATT_".to_string(),
        1..=8 => format!("BATT{}_", index + 1),
        _ => format!("BATT{}_", char::from(b'A' + (index - 9) as u8)),
    }
}

fn battery_label(index: usize) -> String {
    match index {
        0..=8 => (index + 1).to_string(),
        _ => char::from(b'A' + (index - 9) as u8).to_string(),
    }
}

const APM_HELI_FRAME_CLASS: f64 = 6.0;

fn apm_airframe(facts: Facts, vehicle: &Vehicle) -> Rows {
    if facts("FRAME_CLASS").is_none_or(|class| number(&class) == APM_HELI_FRAME_CLASS) {
        return Vec::new();
    }
    [Some(row("Frame Class", enum_of(facts, "FRAME_CLASS"))), facts("FRAME_TYPE").map(|f| row("Frame Type", enum_text(&f))), Some(row("Firmware Version", vehicle.firmware.clone()))].into_iter().flatten().collect()
}

fn apm_sub_frame(facts: Facts, vehicle: &Vehicle) -> Rows {
    let name = match facts("FRAME_CONFIG").map(|f| number(&f) as i64) {
        Some(0) => "BlueROV1",
        Some(1) => "Vectored/BlueROV2",
        Some(2) => "Vectored 6DOF",
        Some(3) => "Vectored 6DOF 90Degree",
        Some(4) => "SimpleROV-3",
        Some(5) => "SimpleROV-4",
        Some(6) => "SimpleROV-5",
        Some(7) => "Custom",
        _ => "Unknown",
    };
    let (major, minor, patch) = vehicle.version;
    let firmware = match major {
        -1 => "Unknown".to_string(),
        _ => format!("{major}.{minor}.{patch} {}", vehicle.firmware_type),
    };
    let git = Some(vehicle.git_hash.clone()).filter(|h| !h.is_empty() && h != "-1").unwrap_or_else(|| "Unknown".to_string());
    vec![row("Frame Type", name), row("Firmware Version", firmware), row("Git Revision", git)]
}

fn apm_radio(facts: Facts) -> Rows {
    [("Roll", "RCMAP_ROLL"), ("Pitch", "RCMAP_PITCH"), ("Yaw", "RCMAP_YAW"), ("Throttle", "RCMAP_THROTTLE")]
        .iter()
        .map(|(label, name)| row(label, facts(name).map(|f| if number(&f) == 0.0 { SETUP_REQUIRED.to_string() } else { format!("Channel {}", text(&f, "valueString")) }).unwrap_or_default()))
        .collect()
}

fn px4_radio(facts: Facts, vehicle: &Vehicle) -> Rows {
    let mapped = |name: &str, unset: &str| facts(name).map(|f| if number(&f) == 0.0 { unset.to_string() } else { text(&f, "valueString") }).unwrap_or_default();
    [("Roll", "RC_MAP_ROLL", SETUP_REQUIRED), ("Pitch", "RC_MAP_PITCH", SETUP_REQUIRED), ("Yaw", "RC_MAP_YAW", SETUP_REQUIRED), ("Throttle", "RC_MAP_THROTTLE", SETUP_REQUIRED), ("Flaps", "RC_MAP_FLAPS", DISABLED), ("Aux1", "RC_MAP_AUX1", DISABLED), ("Aux2", "RC_MAP_AUX2", DISABLED)]
        .iter()
        .filter(|(label, _, _)| !(*label == "Flaps" && vehicle.multi_rotor))
        .map(|(label, name, unset)| row(label, mapped(name, unset)))
        .collect()
}

fn apm_flight_modes(facts: Facts, vehicle: &Vehicle) -> Rows {
    (1..=6).map(|slot| row(&format!("Flight Mode {slot}"), enum_of(facts, &format!("{}{slot}", if vehicle.rover { "MODE" } else { "FLTMODE" })))).collect()
}

fn px4_flight_modes(facts: Facts) -> Rows {
    let switch = facts("RC_MAP_FLTMODE").map(|f| if number(&f) == 0.0 { SETUP_REQUIRED.to_string() } else { enum_text(&f) }).unwrap_or_default();
    std::iter::once(row("Mode switch", switch)).chain((1..=6).map(|slot| row(&format!("Flight Mode {slot} "), enum_of(facts, &format!("COM_FLTMODE{slot}"))))).collect()
}

fn apm_power(facts: Facts) -> Rows {
    (0..MAX_BATTERIES)
        .map_while(|index| facts(&format!("{}MONITOR", battery_prefix(index))).map(|monitor| (index, monitor)))
        .filter(|(_, monitor)| number(monitor) != 0.0)
        .flat_map(|(index, monitor)| {
            let label = battery_label(index);
            std::iter::once(row(&format!("Batt{label} monitor"), enum_text(&monitor))).chain(facts(&format!("{}CAPACITY", battery_prefix(index))).map(|c| row(&format!("Batt{label} capacity"), with_units(&c))))
        })
        .collect()
}

const COMPASS_IDS: [&str; 3] = ["COMPASS_DEV_ID", "COMPASS_DEV_ID2", "COMPASS_DEV_ID3"];
const COMPASS_PRIOS: [&str; 3] = ["COMPASS_PRIO1_ID", "COMPASS_PRIO2_ID", "COMPASS_PRIO3_ID"];
const COMPASS_EXTERNALS: [&str; 3] = ["COMPASS_EXTERNAL", "COMPASS_EXTERN2", "COMPASS_EXTERN3"];
const COMPASS_OFFSETS: [&str; 3] = ["COMPASS_OFS", "COMPASS_OFS2", "COMPASS_OFS3"];
const PRIORITY_NAMES: [&str; 3] = ["Primary", "Secondary", "Tertiary"];
const INS_IDS: [&str; 3] = ["INS_ACC_ID", "INS_ACC2_ID", "INS_ACC3_ID"];
const BARO_IDS: [&str; 3] = ["BARO1_DEVID", "BARO2_DEVID", "BARO3_DEVID"];

fn device(facts: Facts, name: &str) -> String {
    facts(name).map(|f| crate::sensorsettings::decode_device_id(name, number(&f) as u32)).unwrap_or_default()
}

fn compass_text(facts: Facts, index: usize) -> String {
    let id = facts(COMPASS_IDS[index]).map_or(0.0, |f| number(&f));
    let calibrated = ["X", "Y", "Z"].iter().all(|axis| facts(&format!("{}_{axis}", COMPASS_OFFSETS[index])).is_some_and(|f| number(&f) != 0.0));
    let priority = COMPASS_PRIOS.iter().position(|name| facts(name).is_some_and(|f| number(&f) == id)).map_or("Unused", |at| PRIORITY_NAMES[at]);
    let placement = facts(COMPASS_EXTERNALS[index]).map(|f| if number(&f) != 0.0 { ", External" } else { ", Internal" }).unwrap_or("");
    match (id > 0.0, calibrated) {
        (false, _) => "Not installed".to_string(),
        (true, false) => SETUP_REQUIRED.to_string(),
        (true, true) => format!("{priority}{placement}"),
    }
}

fn apm_sensors(facts: Facts) -> Rows {
    let accel_needed = ["INS_ACCOFFS_X", "INS_ACCOFFS_Y", "INS_ACCOFFS_Z"].iter().all(|name| facts(name).is_none_or(|f| number(&f) == 0.0));
    let decoded = |names: &[&str; 3]| names.iter().map(|name| device(facts, name)).filter(|text| !text.is_empty()).map(|text| row("", text)).collect::<Vec<_>>();
    std::iter::once(row("Compasses:", ""))
        .chain((0..COMPASS_IDS.len()).map(|index| row(&compass_text(facts, index), device(facts, COMPASS_PRIOS[index]))))
        .chain(std::iter::once(row("Accelerometer(s):", if accel_needed { SETUP_REQUIRED } else { READY })))
        .chain(decoded(&INS_IDS))
        .chain(std::iter::once(row("Barometer(s):", if facts(BARO_IDS[0]).is_some() { "" } else { "Not Supported(Over APM 4.1)" })))
        .chain(decoded(&BARO_IDS))
        .collect()
}

fn px4_power(facts: Facts) -> Rows {
    let count = (1..).take_while(|index| facts(&format!("BAT{index}_SOURCE")).is_some()).count();
    let label = |index: usize, numbered: &str, single: &str| if count > 1 { format!("Battery {index} {numbered}") } else { single.to_string() };
    let or_na = |name: String, show: fn(&Value) -> String| facts(&name).map_or_else(|| NOT_AVAILABLE.to_string(), |f| show(&f));
    (1..=count)
        .flat_map(|index| {
            [
                row(&label(index, "Source", "Battery Source"), enum_of(facts, &format!("BAT{index}_SOURCE"))),
                row(&label(index, "Full", "Battery Full"), or_na(format!("BAT{index}_V_CHARGED"), with_units)),
                row(&label(index, "Empty", "Battery Empty"), or_na(format!("BAT{index}_V_EMPTY"), with_units)),
                row(&label(index, "Number of Cells", "Number of Cells"), or_na(format!("BAT{index}_N_CELLS"), |f| text(f, "valueString"))),
            ]
        })
        .collect()
}

fn apm_esc(facts: Facts) -> Rows {
    let prefix = if facts("MOT_PWM_TYPE").is_none() && facts("Q_M_PWM_TYPE").is_some() { "Q_M_" } else { "MOT_" };
    let pwm = facts(&format!("{prefix}PWM_TYPE"));
    let dshot = pwm.as_ref().is_some_and(|f| number(f) >= DSHOT_FIRST_TYPE);
    [
        pwm.map(|f| row("Output type", enum_text(&f))),
        facts("SERVO_DSHOT_ESC").filter(|_| dshot).map(|f| row("DShot ESC type", enum_text(&f))),
        facts("SERVO_DSHOT_RATE").filter(|_| dshot).map(|f| row("DShot output rate", enum_text(&f))),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn apm_airspeed(facts: Facts) -> Rows {
    let kind = facts("ARSPD_TYPE");
    let enabled = kind.as_ref().is_some_and(|f| number(f) != 0.0);
    let or_na = |fact: Option<Value>, show: fn(&Value) -> String| fact.map_or_else(|| NOT_AVAILABLE.to_string(), |f| show(&f));
    [
        Some(row("Sensor type", or_na(kind, enum_text))),
        enabled.then(|| row("Use airspeed", or_na(facts("ARSPD_USE"), enum_text))),
        facts("ARSPD2_TYPE").map(|f| row("Sensor 2 type", enum_text(&f))),
        facts("AIRSPEED_CRUISE").filter(|_| enabled).map(|f| row("Cruise airspeed", with_units(&f))),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn apm_follow(facts: Facts) -> Rows {
    let detailed = facts("FOLL_SYSID").is_some();
    let shown = |name: &str| facts(name).map(|f| Some(enum_text(&f)).filter(|t| !t.is_empty()).unwrap_or_else(|| with_units(&f))).unwrap_or_default();
    [("Follow Enabled", "FOLL_ENABLE"), ("Follow System ID", "FOLL_SYSID"), ("Max Distance", "FOLL_DIST_MAX"), ("Offset X", "FOLL_OFS_X"), ("Offset Y", "FOLL_OFS_Y"), ("Offset Z", "FOLL_OFS_Z"), ("Offset Type", "FOLL_OFS_TYPE"), ("Altitude Type", "FOLL_ALT_TYPE"), ("Yaw Behavior", "FOLL_YAW_BEHAVE")]
        .iter()
        .enumerate()
        .filter(|(at, _)| *at == 0 || detailed)
        .map(|(_, (label, name))| row(label, shown(name)))
        .collect()
}

fn apm_failsafes(facts: Facts, vehicle: &Vehicle) -> Rows {
    let monitored = |name: &str| facts(name).is_some_and(|f| number(&f) != 0.0);
    let (batt1, batt2) = (monitored("BATT_MONITOR"), monitored("BATT2_MONITOR"));
    let throttle = match (vehicle.multi_rotor, vehicle.fixed_wing, vehicle.rover) {
        (true, _, _) | (_, _, true) => Some("FS_THR_ENABLE"),
        (_, true, _) => Some("THR_FAILSAFE"),
        _ => None,
    };
    [
        throttle.map(|name| row("Throttle failsafe:", enum_of(facts, name))),
        vehicle.rover.then(|| row("Failsafe Action:", enum_of(facts, "FS_ACTION"))),
        vehicle.rover.then(|| row("Failsafe Crash Check:", enum_of(facts, "FS_CRASH_CHECK"))),
        batt1.then(|| row("Batt1 low failsafe:", enum_of(facts, "BATT_FS_LOW_ACT"))),
        facts("BATT_FS_CRT_ACT").map(|f| row("Batt1 critical failsafe:", enum_text(&f))),
        batt2.then(|| row("Batt2 low failsafe:", enum_of(facts, "BATT2_FS_LOW_ACT"))),
        batt2.then(|| row("Batt2 critical failsafe:", enum_of(facts, "BATT2_FS_CRT_ACT"))),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn arming_checks(facts: Facts) -> String {
    match (facts("ARMING_CHECK"), facts("ARMING_SKIPCHK")) {
        (Some(check), _) => if (number(&check) as i64) & 1 != 0 { "Enabled" } else { "Some disabled" }.to_string(),
        (None, Some(skip)) => if number(&skip) == 0.0 { "Enabled" } else { "Some disabled" }.to_string(),
        (None, None) => String::new(),
    }
}

fn fence_kind(enable: &Value, kind: &Value) -> &'static str {
    match (number(enable) as i64, number(kind) as i64) {
        (0, _) | (_, 0) => "Disabled",
        (_, 1) => "Altitude",
        (_, 2) => "Circle",
        _ => "Altitude,Circle",
    }
}

fn either_text(fact: &Value) -> String {
    Some(text(fact, "enumOrValueString")).filter(|t| !t.is_empty()).unwrap_or_else(|| text(fact, "valueString"))
}

fn sub_failsafes(facts: Facts, vehicle: &Vehicle) -> Rows {
    let modern = vehicle.version >= (3, 5, 0);
    let shown = |name: &str| facts(name).map(|f| either_text(&f)).unwrap_or_default();
    [
        Some(row("GCS failsafe:", shown("FS_GCS_ENABLE"))),
        Some(row("Leak failsafe:", shown("FS_LEAK_ENABLE"))),
        modern.then(|| row("Battery failsafe:", facts("BATT_FS_LOW_ACT").map_or_else(|| DISABLED.to_string(), |f| either_text(&f)))),
        modern.then(|| row("EKF failsafe:", shown("FS_EKF_ACTION"))),
        modern.then(|| row("Pilot Input failsafe:", shown("FS_PILOT_INPUT"))),
        Some(row("Int. Temperature failsafe:", shown("FS_TEMP_ENABLE"))),
        Some(row("Int. Pressure failsafe:", shown("FS_PRESS_ENABLE"))),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn apm_flight_safety(facts: Facts, vehicle: &Vehicle) -> Rows {
    let enable = facts("FENCE_ENABLE");
    let action = facts("FENCE_ACTION").map(|a| match number(&a) as i64 {
        0 => "Report only",
        1 => "RTL or Land",
        _ => "Unknown",
    });
    let rtl = |name: &str, current: fn(f64) -> bool| facts(name).map(|f| if current(number(&f)) { "current".to_string() } else { with_units(&f) }).unwrap_or_default();
    [
        Some(row("Arming Checks:", arming_checks(facts))),
        vehicle.multi_rotor.then(|| row("GeoFence:", enable.as_ref().zip(facts("FENCE_TYPE").as_ref()).map(|(e, k)| fence_kind(e, k)).unwrap_or(""))),
        (vehicle.multi_rotor && enable.as_ref().is_some_and(|e| number(e) != 0.0)).then(|| row("GeoFence:", action.unwrap_or(""))),
        vehicle.multi_rotor.then(|| row("RTL min alt:", rtl("RTL_ALT_M", |v| v == 0.0))),
        vehicle.fixed_wing.then(|| row("RTL min alt:", rtl("RTL_ALTITUDE", |v| v < 0.0))),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn apm_lights(facts: Facts) -> Rows {
    let channel_for = |function: f64| {
        LIGHTS_CHANNELS
            .filter(|channel| facts(&format!("SERVO{channel}_FUNCTION")).is_some_and(|f| number(&f) == function))
            .last()
            .map_or_else(|| DISABLED.to_string(), |channel| format!("Channel {channel}"))
    };
    vec![row("Lights Output 1", channel_for(LIGHTS_1_FUNCTION)), row("Lights Output 2", channel_for(LIGHTS_2_FUNCTION))]
}

pub fn clean_behavior(part: &str) -> String {
    let cleaned = LEVEL_SUFFIX.replace_all(part, "").trim().to_string();
    match cleaned.is_empty() {
        true => part.trim().to_string(),
        false => cleaned.chars().take(1).flat_map(char::to_uppercase).chain(cleaned.chars().skip(1)).collect(),
    }
}

fn px4_safety(facts: Facts) -> Rows {
    let low = enum_of(facts, "COM_LOW_BAT_ACT");
    let parts: Vec<&str> = if facts("COM_LOW_BAT_ACT").is_some() { low.split(',').collect() } else { Vec::new() };
    let split = parts.len() > 1;
    let delay = facts("RTL_LAND_DELAY").map_or(0, |f| number(&f) as i64);
    let delay_units = facts("RTL_LAND_DELAY").map(|f| text(&f, "units")).unwrap_or_default();
    let then = match delay {
        0 => "Land immediately",
        d if d < 0 => "Loiter and do not land",
        _ => "Loiter and land after specified time",
    };
    let units_of = |name: &str| facts(name).map(|f| with_units(&f)).unwrap_or_default();
    [
        Some(row("Low Battery Failsafe", if split { String::new() } else { low.clone() })),
        split.then(|| row("  Critical Level", clean_behavior(parts[0]))),
        split.then(|| row("  Emergency Level", clean_behavior(parts[1]))),
        Some(row("RC/Joystick Loss Failsafe", enum_of(facts, "NAV_RCL_ACT"))),
        Some(row("RC/Joystick Loss Timeout", units_of("COM_RC_LOSS_T"))),
        Some(row("Data Link Loss Failsafe", enum_of(facts, "NAV_DLL_ACT"))),
        Some(row("RTL Climb To", units_of("RTL_RETURN_ALT"))),
        Some(row("RTL, Then", then)),
        facts("RTL_DESCEND_ALT").filter(|_| delay != 0).map(|f| row("Loiter Alt", with_units(&f))),
        (delay > 0).then(|| row("Land Delay", format!("{delay} {delay_units}"))),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn px4_sensors(facts: Facts, vehicle: &Vehicle) -> Rows {
    let extra_compass = |n: u8| facts(&format!("CAL_MAG{n}_ID")).filter(|f| number(f) != 0.0).map(|_| row(&format!("Compass {n}"), READY));
    match vehicle.fixed_wing {
        true => vec![row("Compass", ready_unless_zero(facts, "CAL_MAG0_ID")), row("Gyro", ready_unless_zero(facts, "CAL_GYRO0_ID")), row("Accelerometer", ready_unless_zero(facts, "CAL_ACC0_ID"))],
        false => [Some(row("Compass 0", ready_unless_zero(facts, "CAL_MAG0_ID"))), extra_compass(1), extra_compass(2), Some(row("Gyro", ready_unless_zero(facts, "CAL_GYRO0_ID"))), Some(row("Accelerometer", ready_unless_zero(facts, "CAL_ACC0_ID")))].into_iter().flatten().collect(),
    }
}

fn px4_airframe(facts: Facts, vehicle: &Vehicle) -> Rows {
    let autostart = facts("SYS_AUTOSTART").map(|f| number(&f) as i64).filter(|id| *id != 0);
    let names = autostart.and_then(crate::px4airframe::current_names);
    let named = |pick: fn(&(String, String)) -> String| match autostart {
        Some(_) => names.as_ref().map(pick).unwrap_or_default(),
        None => SETUP_REQUIRED.to_string(),
    };
    vec![
        row("System ID", facts("MAV_SYS_ID").map(|f| text(&f, "valueString")).unwrap_or_default()),
        row("Airframe type", named(|n| n.0.clone())),
        row("Vehicle", named(|n| n.1.clone())),
        row("Firmware Version", vehicle.firmware.clone()),
    ]
    .into_iter()
    .chain(vehicle.custom.clone().map(|custom| row("Custom Fw. Ver.", custom)))
    .collect()
}

pub fn rows(class: &str, facts: Facts, vehicle: &Vehicle) -> Option<Rows> {
    Some(match class {
        "APMAirframeComponent" => apm_airframe(facts, vehicle),
        "APMSubFrameComponent" => apm_sub_frame(facts, vehicle),
        "APMRadioComponent" => apm_radio(facts),
        "PX4RadioComponent" => px4_radio(facts, vehicle),
        "APMFlightModesComponent" => apm_flight_modes(facts, vehicle),
        "FlightModesComponent" => px4_flight_modes(facts),
        "APMPowerComponent" => apm_power(facts),
        "APMESCComponent" => apm_esc(facts),
        "PowerComponent" => px4_power(facts),
        "APMSensorsComponent" => apm_sensors(facts),
        "APMAirspeedComponent" => apm_airspeed(facts),
        "APMFollowComponent" => apm_follow(facts),
        "APMFailsafesComponent" if vehicle.sub => sub_failsafes(facts, vehicle),
        "APMFailsafesComponent" => apm_failsafes(facts, vehicle),
        "APMFlightSafetyComponent" if vehicle.sub => vec![row("Arming Checks:", arming_checks(facts))],
        "APMFlightSafetyComponent" => apm_flight_safety(facts, vehicle),
        "APMLightsComponent" => apm_lights(facts),
        "SafetyComponent" => px4_safety(facts),
        "SensorsComponent" => px4_sensors(facts, vehicle),
        "AirframeComponent" => px4_airframe(facts, vehicle),
        _ => return None,
    })
}

pub fn firmware_text(major: i64, minor: i64, patch: i64, kind: &str) -> String {
    match major {
        -1 => "Unknown".to_string(),
        _ => format!("{major}.{minor}.{patch}{kind}"),
    }
}

pub fn setup_summary_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let read = object(&backend.get_fields("vehicle", "multiRotor,fixedWing,rover,sub,apmFirmware,firmwareMajorVersion,firmwareMinorVersion,firmwarePatchVersion,firmwareVersionTypeString,gitHash,firmwareCustomMajorVersion,firmwareCustomMinorVersion,firmwareCustomPatchVersion"));
    let part = |key: &str| read.get(key).and_then(Value::as_i64).unwrap_or(-1);
    let vehicle = Vehicle {
        multi_rotor: flag(&read, "multiRotor"),
        fixed_wing: flag(&read, "fixedWing"),
        rover: flag(&read, "rover") && flag(&read, "apmFirmware"),
        sub: flag(&read, "sub"),
        version: (part("firmwareMajorVersion"), part("firmwareMinorVersion"), part("firmwarePatchVersion")),
        firmware: firmware_text(part("firmwareMajorVersion"), part("firmwareMinorVersion"), part("firmwarePatchVersion"), &text(&read, "firmwareVersionTypeString")),
        firmware_type: text(&read, "firmwareVersionTypeString"),
        git_hash: read.get("gitHash").map(|h| h.as_str().map_or_else(|| h.to_string(), str::to_string)).unwrap_or_default(),
        custom: (part("firmwareCustomMajorVersion") != -1).then(|| format!("{}.{}.{}", part("firmwareCustomMajorVersion"), part("firmwareCustomMinorVersion"), part("firmwareCustomPatchVersion"))),
    };
    let facts = |name: &str| Some(object(&backend.get(&format!("vehicle.parameterManager.getParameter(-1,{name})")))).filter(|f| f.get("kind").and_then(Value::as_str) == Some("fact") && !text(f, "name").is_empty());
    let count = object(&backend.get(COMPONENTS)).get("value").and_then(Value::as_array).map_or(0, Vec::len);
    let components: Vec<Value> = (0..count)
        .filter_map(|index| {
            let component = object(&backend.get_fields(&format!("{COMPONENTS}.{index}"), "name"));
            let found = rows(&text(&component, "class"), &facts, &vehicle)?;
            Some(json!({ "name": text(&component, "name"), "rows": found.iter().map(|(label, value)| json!({ "label": label, "value": value })).collect::<Vec<_>>() }))
        })
        .collect();
    json!({ "kind": "object", "class": "SetupSummary", "components": components })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn fact(raw: f64, value: &str, label: &str, units: &str) -> Value {
        json!({ "kind": "fact", "rawValue": raw, "valueString": value, "enumStringValue": label, "units": units })
    }

    fn lookup<'a>(map: &'a HashMap<&'a str, Value>) -> impl Fn(&str) -> Option<Value> + 'a {
        move |name| map.get(name).cloned()
    }

    fn copter() -> Vehicle {
        Vehicle { multi_rotor: true, fixed_wing: false, rover: false, sub: false, version: (4, 5, 7), firmware: "4.5.7".into(), firmware_type: String::new(), git_hash: String::new(), custom: None }
    }

    #[test]
    fn radio_and_airframe_rows_read_like_the_summary_qml() {
        let map = HashMap::from([("RCMAP_ROLL", fact(1.0, "1", "", "")), ("RCMAP_PITCH", fact(0.0, "0", "", "")), ("FRAME_CLASS", fact(1.0, "1", "Quad", ""))]);
        let facts = lookup(&map);
        assert_eq!(rows("APMRadioComponent", &facts, &copter()).unwrap()[..2], [row("Roll", "Channel 1"), row("Pitch", SETUP_REQUIRED)]);
        assert_eq!(rows("APMAirframeComponent", &facts, &copter()).unwrap(), [row("Frame Class", "Quad"), row("Firmware Version", "4.5.7")], "FRAME_TYPE is absent so its row is hidden");
        let custom = Vehicle { custom: Some("1.2.3".into()), ..copter() };
        assert_eq!(rows("AirframeComponent", &facts, &custom).unwrap().last(), Some(&row("Custom Fw. Ver.", "1.2.3")), "AirframeComponentSummary shows it once the custom major version is set");
        assert!(rows("AirframeComponent", &facts, &copter()).unwrap().iter().all(|(label, _)| label != "Custom Fw. Ver."));
        assert!(rows("APMTuningComponent", &facts, &copter()).is_none());
        assert_eq!(firmware_text(-1, 0, 0, ""), "Unknown");
        assert_eq!(firmware_text(1, 15, 2, "beta"), "1.15.2beta");
    }

    #[test]
    fn batteries_stop_at_the_first_missing_monitor_and_skip_disabled_ones() {
        let map = HashMap::from([("BATT_MONITOR", fact(4.0, "4", "Analog Voltage and Current", "")), ("BATT_CAPACITY", fact(5000.0, "5000", "", "mAh")), ("BATT2_MONITOR", fact(0.0, "0", "Disabled", "")), ("BATT4_MONITOR", fact(4.0, "4", "x", ""))]);
        assert_eq!(rows("APMPowerComponent", &lookup(&map), &copter()).unwrap(), [row("Batt1 monitor", "Analog Voltage and Current"), row("Batt1 capacity", "5000 mAh")]);
        assert_eq!((battery_prefix(0), battery_prefix(8), battery_prefix(9), battery_label(15)), ("BATT_".into(), "BATT9_".into(), "BATTA_".into(), "G".into()));
    }

    #[test]
    fn px4_power_numbers_its_batteries_only_when_there_are_several() {
        let one = HashMap::from([("BAT1_SOURCE", fact(0.0, "0", "Power Module", "")), ("BAT1_V_CHARGED", fact(4.2, "4.20", "", "V"))]);
        assert_eq!(
            rows("PowerComponent", &lookup(&one), &copter()).unwrap(),
            [row("Battery Source", "Power Module"), row("Battery Full", "4.20 V"), row("Battery Empty", NOT_AVAILABLE), row("Number of Cells", NOT_AVAILABLE)]
        );
        let two = HashMap::from([("BAT1_SOURCE", fact(0.0, "0", "Power Module", "")), ("BAT2_SOURCE", fact(1.0, "1", "External", ""))]);
        let shown = rows("PowerComponent", &lookup(&two), &copter()).unwrap();
        assert_eq!((shown.len(), shown[4].clone()), (8, row("Battery 2 Source", "External")));
    }

    #[test]
    fn px4_safety_splits_the_low_battery_action_and_follows_land_delay() {
        let map = HashMap::from([("COM_LOW_BAT_ACT", fact(3.0, "3", "Return at critical level, land at emergency level", "")), ("RTL_LAND_DELAY", fact(-1.0, "-1", "", "s")), ("RTL_DESCEND_ALT", fact(30.0, "30", "", "m"))]);
        let shown = rows("SafetyComponent", &lookup(&map), &copter()).unwrap();
        assert_eq!(shown[..3], [row("Low Battery Failsafe", ""), row("  Critical Level", "Return"), row("  Emergency Level", "Land")]);
        assert!(shown.contains(&row("RTL, Then", "Loiter and do not land")));
        assert!(shown.contains(&row("Loiter Alt", "30 m")));
        assert!(!shown.iter().any(|(label, _)| label == "Land Delay"), "a negative delay never lands");
        assert_eq!(clean_behavior("Warning"), "Warning");
    }

    #[test]
    fn apm_sensors_name_each_compass_by_its_priority_and_decode_the_devices() {
        let ist = f64::from((0x0A << 16) | (1 << 3) | 1);
        let map = HashMap::from([
            ("COMPASS_DEV_ID", fact(ist, "", "", "")),
            ("COMPASS_OFS_X", fact(1.0, "", "", "")),
            ("COMPASS_OFS_Y", fact(1.0, "", "", "")),
            ("COMPASS_OFS_Z", fact(1.0, "", "", "")),
            ("COMPASS_EXTERNAL", fact(1.0, "", "", "")),
            ("COMPASS_PRIO1_ID", fact(ist, "", "", "")),
            ("COMPASS_DEV_ID2", fact(5.0, "", "", "")),
            ("INS_ACCOFFS_X", fact(0.1, "", "", "")),
        ]);
        let shown = rows("APMSensorsComponent", &lookup(&map), &copter()).unwrap();
        assert_eq!(shown[..4], [row("Compasses:", ""), row("Primary, External", "IST8310 (I2C1)"), row(SETUP_REQUIRED, ""), row("Not installed", "")]);
        assert_eq!(shown[4], row("Accelerometer(s):", READY));
        assert_eq!(shown.last(), Some(&row("Barometer(s):", "Not Supported(Over APM 4.1)")));
    }

    #[test]
    fn sub_failsafes_drop_the_rows_older_firmware_lacks() {
        let map = HashMap::from([("FS_GCS_ENABLE", json!({ "kind": "fact", "enumOrValueString": "Warn only", "valueString": "1" })), ("ARMING_SKIPCHK", fact(0.0, "0", "", ""))]);
        let sub = Vehicle { sub: true, ..copter() };
        let modern = rows("APMFailsafesComponent", &lookup(&map), &sub).unwrap();
        assert_eq!((modern[0].clone(), modern[2].clone()), (row("GCS failsafe:", "Warn only"), row("Battery failsafe:", DISABLED)));
        assert_eq!(rows("APMFailsafesComponent", &lookup(&map), &Vehicle { version: (3, 4, 0), ..sub }).unwrap().len(), 4);
        assert_eq!(rows("APMFlightSafetyComponent", &lookup(&map), &Vehicle { sub: true, ..copter() }).unwrap(), [row("Arming Checks:", "Enabled")]);
    }

    #[test]
    fn flight_safety_fence_and_lights_follow_their_parameters() {
        let map = HashMap::from([("ARMING_CHECK", fact(0.0, "0", "", "")), ("FENCE_ENABLE", fact(1.0, "1", "", "")), ("FENCE_TYPE", fact(3.0, "3", "", "")), ("FENCE_ACTION", fact(1.0, "1", "", "")), ("RTL_ALT_M", fact(0.0, "0", "", "m")), ("SERVO9_FUNCTION", fact(59.0, "59", "", ""))]);
        let facts = lookup(&map);
        assert_eq!(
            rows("APMFlightSafetyComponent", &facts, &copter()).unwrap(),
            [row("Arming Checks:", "Some disabled"), row("GeoFence:", "Altitude,Circle"), row("GeoFence:", "RTL or Land"), row("RTL min alt:", "current")]
        );
        assert_eq!(rows("APMLightsComponent", &facts, &copter()).unwrap(), [row("Lights Output 1", "Channel 9"), row("Lights Output 2", DISABLED)]);
    }
}
