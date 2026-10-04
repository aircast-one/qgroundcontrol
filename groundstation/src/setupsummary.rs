use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Value, json};

use crate::read::{flag, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.parameterManager.parametersReady", "vehicle.autopilotPlugin.vehicleComponents", "vehicle.multiRotor", "vehicle.fixedWing", "vehicle.vtol", "vehicle.airship", "vehicle.vehicleTypeString", "vehicle.rover", "vehicle.sub", "vehicle.apmFirmware", "vehicle.firmwareMajorVersion", "vehicle.firmwareMinorVersion", "vehicle.firmwarePatchVersion", "vehicle.firmwareVersionTypeString", "vehicle.gitHash", "vehicle.firmwareCustomMajorVersion", "vehicle.firmwareCustomMinorVersion", "vehicle.firmwareCustomPatchVersion"];
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
    pub forward_flight: bool,
    pub helicopter: bool,
    pub rover: bool,
    pub sub: bool,
    pub safety_supported: bool,
    pub version: (i64, i64, i64),
    pub firmware: String,
    pub firmware_type: String,
    pub git_hash: String,
    pub custom: Option<String>,
}

pub type Facts<'a> = &'a dyn Fn(&str) -> Option<Value>;
type Rows = Vec<SummaryRow>;

#[derive(Debug, Clone, PartialEq)]
pub struct SummaryRow {
    pub label: String,
    pub value: String,
    pub warn: bool,
}

pub fn row(label: &str, value: impl Into<String>) -> SummaryRow {
    SummaryRow { label: label.to_string(), value: value.into(), warn: false }
}

fn number(fact: &Value) -> f64 {
    fact.get("rawValue").or(fact.get("value")).and_then(Value::as_f64).unwrap_or(0.0)
}

fn enum_text(fact: &Value) -> String {
    crate::read::enum_label(fact)
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

const MAV_TYPE_HELICOPTER: u8 = 4;

fn apm_airframe(facts: Facts, vehicle: &Vehicle) -> Rows {
    if vehicle.helicopter || facts("FRAME_CLASS").is_none() {
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

fn esp_text(facts: Facts, prefix: &str) -> String {
    let words: Option<Vec<u32>> = (1..=4).map(|i| facts(&format!("{prefix}{i}")).map(|f| number(&f) as i64 as u32)).collect();
    words.map(|w| crate::espbridge::unpack([w[0], w[1], w[2], w[3]])).unwrap_or_default()
}

fn esp_version(raw: u32) -> String {
    format!("{}.{}.{}", raw >> 24, (raw >> 16) & 0xFF, raw & 0xFFFF)
}

fn esp8266(facts: Facts) -> Rows {
    let value_string = |name: &str| facts(name).map(|f| text(&f, "valueString")).unwrap_or_default();
    let station = facts("WIFI_MODE").is_some_and(|f| number(&f) != 0.0);
    [
        Some(row("Firmware Version", facts("SW_VER").map(|f| esp_version(number(&f) as i64 as u32)).unwrap_or_default())),
        Some(row("WiFi Mode", if station { "Station Mode" } else { "AP Mode" })),
        (!station).then(|| row("WiFi Channel", value_string("WIFI_CHANNEL"))),
        Some(row("WiFi AP SSID", esp_text(facts, "WIFI_SSID"))),
        Some(row("WiFi AP Password", esp_text(facts, "WIFI_PASSWORD"))),
        Some(row("UART Baud Rate", value_string("UART_BAUDRATE"))),
    ]
    .into_iter()
    .flatten()
    .collect()
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
    match vehicle.forward_flight {
        true => [Some(row("Compass", ready_unless_zero(facts, "CAL_MAG0_ID"))), Some(row("Gyro", ready_unless_zero(facts, "CAL_GYRO0_ID"))), Some(row("Accelerometer", ready_unless_zero(facts, "CAL_ACC0_ID"))), px4_airspeed_row(facts, vehicle)].into_iter().flatten().collect(),
        false => [Some(row("Compass 0", ready_unless_zero(facts, "CAL_MAG0_ID"))), extra_compass(1), extra_compass(2), Some(row("Gyro", ready_unless_zero(facts, "CAL_GYRO0_ID"))), Some(row("Accelerometer", ready_unless_zero(facts, "CAL_ACC0_ID")))].into_iter().flatten().collect(),
    }
}

const AIRSPEED_CHECK_CIRCUIT_BREAKER: i64 = 162_128;

fn px4_airspeed_checks(facts: Facts, vehicle: &Vehicle) -> (Vec<&'static str>, bool) {
    let value = |name: &str| facts(name).map(|f| number(&f)).unwrap_or(0.0);
    let (major, minor, _) = vehicle.version;
    let (read, supported) = match major > 1 || (major == 1 && minor > 14) {
        true => (vec!["SYS_HAS_NUM_ASPD"], value("SYS_HAS_NUM_ASPD") != 0.0),
        false => {
            let mode_off = value("FW_ARSP_MODE") == 0.0;
            (std::iter::once("FW_ARSP_MODE").chain(mode_off.then_some("CBRK_AIRSPD_CHK")).collect(), mode_off && value("CBRK_AIRSPD_CHK") as i64 != AIRSPEED_CHECK_CIRCUIT_BREAKER)
        }
    };
    (read.into_iter().chain(supported.then_some("SENS_DPRES_OFF")).collect(), supported)
}

fn px4_airspeed_row(facts: Facts, vehicle: &Vehicle) -> Option<SummaryRow> {
    let unset = facts("SENS_DPRES_OFF").is_none_or(|f| number(&f) == 0.0);
    px4_airspeed_checks(facts, vehicle).1.then(|| row("Airspeed", if unset { SETUP_REQUIRED } else { READY }))
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
        "APMFailsafesComponent" if !vehicle.safety_supported => return None,
        "APMFlightSafetyComponent" if !vehicle.safety_supported => vec![row("", "Not supported")],
        "APMFailsafesComponent" if vehicle.sub => sub_failsafes(facts, vehicle),
        "APMFailsafesComponent" => apm_failsafes(facts, vehicle),
        "APMFlightSafetyComponent" if vehicle.sub => vec![row("Arming Checks:", arming_checks(facts))],
        "APMFlightSafetyComponent" => apm_flight_safety(facts, vehicle),
        "APMLightsComponent" => apm_lights(facts),
        "SafetyComponent" => px4_safety(facts),
        "SensorsComponent" => px4_sensors(facts, vehicle),
        "AirframeComponent" => px4_airframe(facts, vehicle),
        "JoystickComponent" => crate::joystickhost::summary(),
        _ => return None,
    })
}

pub fn firmware_text(major: i64, minor: i64, patch: i64, kind: &str) -> String {
    match major {
        -1 => "Unknown".to_string(),
        _ => format!("{major}.{minor}.{patch}{kind}"),
    }
}

fn vehicle(backend: &dyn Backend) -> Vehicle {
    let read = object(&backend.get_fields("vehicle", "multiRotor,fixedWing,vtol,airship,vehicleTypeString,rover,sub,apmFirmware,firmwareMajorVersion,firmwareMinorVersion,firmwarePatchVersion,firmwareVersionTypeString,gitHash,firmwareCustomMajorVersion,firmwareCustomMinorVersion,firmwareCustomPatchVersion"));
    let part = |key: &str| read.get(key).and_then(Value::as_i64).unwrap_or(-1);
    Vehicle {
        multi_rotor: flag(&read, "multiRotor"),
        fixed_wing: flag(&read, "fixedWing"),
        forward_flight: ["fixedWing", "vtol", "airship"].iter().any(|key| flag(&read, key)),
        helicopter: text(&read, "vehicleTypeString") == crate::vehiclefacade::mav_type_text(MAV_TYPE_HELICOPTER),
        rover: flag(&read, "rover") && flag(&read, "apmFirmware"),
        sub: flag(&read, "sub"),
        safety_supported: crate::setup::apm_safety_supported(&text(&read, "vehicleTypeString")),
        version: (part("firmwareMajorVersion"), part("firmwareMinorVersion"), part("firmwarePatchVersion")),
        firmware: firmware_text(part("firmwareMajorVersion"), part("firmwareMinorVersion"), part("firmwarePatchVersion"), &text(&read, "firmwareVersionTypeString")),
        firmware_type: text(&read, "firmwareVersionTypeString"),
        git_hash: read.get("gitHash").map(|h| h.as_str().map_or_else(|| h.to_string(), str::to_string)).unwrap_or_default(),
        custom: (part("firmwareCustomMajorVersion") != -1).then(|| format!("{}.{}.{}", part("firmwareCustomMajorVersion"), part("firmwareCustomMinorVersion"), part("firmwareCustomPatchVersion"))),
    }
}

fn parameter(backend: &dyn Backend, name: &str) -> Option<Value> {
    Some(object(&backend.get(&format!("vehicle.parameterManager.getParameter(-1,{name})")))).filter(|f| f.get("kind").and_then(Value::as_str) == Some("fact") && !text(f, "name").is_empty())
}

fn component_classes(backend: &dyn Backend) -> Vec<Value> {
    object(&backend.get(COMPONENTS)).get("value").and_then(Value::as_array).cloned().unwrap_or_default()
}

pub const PAGE: &str = "Summary";

const DEFAULT_COMPONENT: i64 = -1;
const APM_RADIO_SUMMARY_LOOKUPS: &[&str] = &["RCMAP_ROLL", "RCMAP_PITCH", "RCMAP_YAW", "RCMAP_THROTTLE"];
const PX4_RADIO_SUMMARY_LOOKUPS: &[&str] = &["RC_MAP_ROLL", "RC_MAP_PITCH", "RC_MAP_YAW", "RC_MAP_THROTTLE", "RC_MAP_FLAPS", "RC_MAP_AUX1", "RC_MAP_AUX2"];
const PX4_SAFETY_SUMMARY_LOOKUPS: &[&str] = &["RTL_RETURN_ALT", "RTL_DESCEND_ALT", "COM_RC_LOSS_T", "COM_LOW_BAT_ACT", "NAV_RCL_ACT", "NAV_DLL_ACT", "RTL_LAND_DELAY"];
const PX4_AIRFRAME_SUMMARY_LOOKUPS: &[&str] = &["SYS_AUTOSTART", "SYS_AUTOCONFIG", "MAV_SYS_ID"];
const ESP_SUMMARY_LOOKUPS: &[&str] = &["DEBUG_ENABLED", "WIFI_CHANNEL", "WIFI_UDP_HPORT", "WIFI_UDP_CPORT", "UART_BAUDRATE"];

fn summary_lookups(class: &str, facts: Facts, vehicle: &Vehicle) -> Vec<(i64, String)> {
    let at = |names: Vec<String>| names.into_iter().map(|name| (DEFAULT_COMPONENT, name)).collect::<Vec<_>>();
    let listed = |names: &[&str]| at(names.iter().map(|n| n.to_string()).collect());
    match class {
        "APMSubFrameComponent" => listed(&["FRAME_CONFIG"]),
        "APMRadioComponent" => listed(APM_RADIO_SUMMARY_LOOKUPS),
        "PX4RadioComponent" => listed(PX4_RADIO_SUMMARY_LOOKUPS),
        "APMFlightModesComponent" => at((1..=6).map(|slot| format!("{}{slot}", if facts("MODE1").is_some() { "MODE" } else { "FLTMODE" })).collect()),
        "FlightModesComponent" => at((1..=6).map(|slot| format!("COM_FLTMODE{slot}")).collect()),
        "APMPowerComponent" => listed(&["BATT_MONITOR"]),
        "APMSensorsComponent" => listed(&crate::vehicleconfig::apm_sensor_params_lookups(&|name| facts(name).is_some())),
        "APMFailsafesComponent" if !vehicle.safety_supported => vec![],
        "APMFailsafesComponent" if vehicle.sub => listed(&["FS_EKF_ACTION", "FS_GCS_ENABLE", "FS_LEAK_ENABLE"].into_iter().chain((vehicle.version >= (3, 5, 0)).then_some("FS_PILOT_INPUT")).chain(["FS_TEMP_ENABLE", "FS_PRESS_ENABLE"]).collect::<Vec<_>>()),
        "APMFailsafesComponent" => listed(&["BATT_MONITOR"]),
        "APMLightsComponent" => at(LIGHTS_CHANNELS.map(|channel| format!("SERVO{channel}_FUNCTION")).collect()),
        "SafetyComponent" => listed(PX4_SAFETY_SUMMARY_LOOKUPS),
        "SensorsComponent" if vehicle.forward_flight => listed(&["CAL_MAG0_ID", "CAL_GYRO0_ID", "CAL_ACC0_ID"].into_iter().chain(px4_airspeed_checks(facts, vehicle).0).collect::<Vec<_>>()),
        "SensorsComponent" => listed(&["CAL_MAG0_ID", "CAL_MAG1_ID", "CAL_MAG2_ID", "CAL_GYRO0_ID", "CAL_ACC0_ID"]),
        "AirframeComponent" => listed(PX4_AIRFRAME_SUMMARY_LOOKUPS),
        "ESP8266Component" => crate::vehicleconfig::ESP_CONTROLLER_LOOKUPS.iter().chain(ESP_SUMMARY_LOOKUPS).map(|name| (i64::from(crate::espbridge::COMPONENT), name.to_string())).collect(),
        _ => vec![],
    }
}

pub fn reported_lookups(backend: &dyn Backend) -> Vec<(i64, String)> {
    let vehicle = vehicle(backend);
    let facts = |name: &str| parameter(backend, name);
    component_classes(backend).iter().flat_map(|component| summary_lookups(&text(component, "class"), &facts, &vehicle)).collect()
}

pub fn setup_summary_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = vehicle(backend);
    let facts = |name: &str| parameter(backend, name);
    let esp_facts = |name: &str| crate::espbridge::fact(backend, name);
    let components: Vec<Value> = component_classes(backend)
        .iter()
        .filter_map(|component| {
            let found = match text(component, "class").as_str() {
                "ESP8266Component" => esp8266(&esp_facts),
                class => rows(class, &facts, &vehicle)?,
            };
            Some(json!({ "name": text(component, "name"), "rows": found.iter().map(|r| json!({ "label": r.label, "value": r.value, "warn": r.warn })).collect::<Vec<_>>() }))
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

    #[test]
    fn a_px4_vtol_sensors_summary_uses_the_fixed_wing_rows_with_airspeed() {
        let map: HashMap<&str, Value> = [("CAL_MAG0_ID", json!({ "kind": "fact", "name": "CAL_MAG0_ID", "value": 1 })), ("SYS_HAS_NUM_ASPD", json!({ "kind": "fact", "name": "SYS_HAS_NUM_ASPD", "value": 1 })), ("SENS_DPRES_OFF", json!({ "kind": "fact", "name": "SENS_DPRES_OFF", "value": 0 }))].into_iter().collect();
        let vtol = Vehicle { multi_rotor: false, forward_flight: true, version: (1, 15, 0), ..copter() };
        let rows = px4_sensors(&lookup(&map), &vtol);
        assert_eq!(rows[0].label, "Compass", "SensorsComponent picks the fixed-wing summary for fixed wing, VTOL and airship");
        assert!(rows.contains(&row("Airspeed", SETUP_REQUIRED)), "with an airspeed sensor and no offset, the Airspeed row asks for setup: {rows:?}");
    }

    fn copter() -> Vehicle {
        Vehicle { multi_rotor: true, fixed_wing: false, forward_flight: false, helicopter: false, rover: false, sub: false, safety_supported: true, version: (4, 5, 7), firmware: "4.5.7".into(), firmware_type: String::new(), git_hash: String::new(), custom: None }
    }

    fn names(found: Vec<(i64, String)>) -> Vec<String> {
        found.into_iter().map(|(_, name)| name).collect()
    }

    #[test]
    fn summary_lookups_follow_each_summary_qml_and_its_controller() {
        let none: HashMap<&str, Value> = HashMap::new();
        let bare = lookup(&none);
        assert_eq!(names(summary_lookups("APMRadioComponent", &bare, &copter())), APM_RADIO_SUMMARY_LOOKUPS);
        let rover = HashMap::from([("MODE1", fact(0.0, "0", "", ""))]);
        assert_eq!(names(summary_lookups("APMFlightModesComponent", &lookup(&rover), &copter()))[5], "MODE6", "APMFlightModesComponentSummary picks MODE when MODE1 exists");
        assert_eq!(names(summary_lookups("APMFlightModesComponent", &bare, &copter()))[0], "FLTMODE1");
        let old_sub = Vehicle { sub: true, version: (3, 4, 0), ..copter() };
        assert!(!names(summary_lookups("APMFailsafesComponent", &bare, &old_sub)).contains(&"FS_PILOT_INPUT".to_string()), "_firmware34 skips FS_PILOT_INPUT");
        assert!(names(summary_lookups("APMFailsafesComponent", &bare, &Vehicle { sub: true, ..copter() })).contains(&"FS_PILOT_INPUT".to_string()));
        assert_eq!(names(summary_lookups("APMFailsafesComponent", &bare, &copter())), ["BATT_MONITOR"]);
        assert!(summary_lookups("APMFailsafesComponent", &bare, &Vehicle { safety_supported: false, ..copter() }).is_empty(), "no summary for unsupported vehicle types");
        assert!(summary_lookups("APMAirspeedComponent", &bare, &copter()).is_empty() && summary_lookups("APMFollowComponent", &bare, &copter()).is_empty(), "guarded or reportMissing false only");
        assert!(!names(summary_lookups("APMSensorsComponent", &bare, &copter())).contains(&"AHRS_ORIENTATION".to_string()), "APMSensorParams has no board orientation; the page adds it");
        let has_airspeed = HashMap::from([("SYS_HAS_NUM_ASPD", fact(1.0, "1", "", ""))]);
        let vtol = Vehicle { multi_rotor: false, forward_flight: true, version: (1, 15, 0), ..copter() };
        assert_eq!(names(summary_lookups("SensorsComponent", &lookup(&has_airspeed), &vtol))[3..], ["SYS_HAS_NUM_ASPD", "SENS_DPRES_OFF"], "airspeedCalSupported/Required read through ParameterManager::getParameter");
        let mode_on = HashMap::from([("FW_ARSP_MODE", fact(1.0, "1", "", ""))]);
        assert_eq!(names(summary_lookups("SensorsComponent", &lookup(&mode_on), &Vehicle { version: (1, 14, 0), ..vtol }))[3..], ["FW_ARSP_MODE"], "&& stops before CBRK_AIRSPD_CHK");
        let esp = summary_lookups("ESP8266Component", &bare, &copter());
        assert_eq!((esp[0].0, esp.iter().filter(|(_, n)| n == "UART_BAUDRATE").count()), (i64::from(crate::espbridge::COMPONENT), 2), "controller first, then the summary's own lookups on component 240");
    }

    struct Summary(&'static [&'static str]);

    impl Backend for Summary {
        fn get(&self, path: &str) -> String {
            let name = path.strip_prefix("vehicle.parameterManager.getParameter(").and_then(|p| p.strip_suffix(')')).and_then(|p| p.split_once(',')).map(|(_, n)| n);
            match (path, name) {
                (COMPONENTS, _) => json!({ "value": [{ "class": "APMRadioComponent" }, { "class": "APMLightsComponent" }, { "class": "APMPowerComponent" }] }).to_string(),
                (_, Some(name)) if self.0.contains(&name) => json!({ "kind": "fact", "name": name }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn get_fields(&self, _path: &str, _fields: &str) -> String {
            json!({ "multiRotor": true, "vehicleTypeString": "Quadrotor", "firmwareMajorVersion": 4, "firmwareMinorVersion": 5, "firmwarePatchVersion": 7 }).to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_summary_page_reports_every_listed_component_s_lookups() {
        let found = names(reported_lookups(&Summary(&[])));
        assert_eq!((found.first().map(String::as_str), found.last().map(String::as_str), found.len()), (Some("RCMAP_ROLL"), Some("BATT_MONITOR"), 15), "in component order: {found:?}");
    }

    #[test]
    fn radio_and_airframe_rows_read_like_the_summary_qml() {
        let map = HashMap::from([("RCMAP_ROLL", fact(1.0, "1", "", "")), ("RCMAP_PITCH", fact(0.0, "0", "", "")), ("FRAME_CLASS", fact(1.0, "1", "Quad", ""))]);
        let facts = lookup(&map);
        assert_eq!(rows("APMRadioComponent", &facts, &copter()).unwrap()[..2], [row("Roll", "Channel 1"), row("Pitch", SETUP_REQUIRED)]);
        assert_eq!(rows("APMAirframeComponent", &facts, &copter()).unwrap(), [row("Frame Class", "Quad"), row("Firmware Version", "4.5.7")], "FRAME_TYPE is absent so its row is hidden");
        let custom = Vehicle { custom: Some("1.2.3".into()), ..copter() };
        assert_eq!(rows("AirframeComponent", &facts, &custom).unwrap().last(), Some(&row("Custom Fw. Ver.", "1.2.3")), "AirframeComponentSummary shows it once the custom major version is set");
        assert!(rows("AirframeComponent", &facts, &copter()).unwrap().iter().all(|r| r.label != "Custom Fw. Ver."));
        assert!(rows("APMTuningComponent", &facts, &copter()).is_none());
        assert_eq!(firmware_text(-1, 0, 0, ""), "Unknown");
        assert_eq!(firmware_text(1, 15, 2, "beta"), "1.15.2beta");
    }

    #[test]
    fn the_wifi_bridge_summary_reads_like_esp8266_component_summary() {
        let ssid = u32::from_le_bytes(*b"Air\0");
        let ap = HashMap::from([("SW_VER", fact(f64::from(0x0102_0003u32), "", "", "")), ("WIFI_CHANNEL", fact(6.0, "6", "", "")), ("WIFI_SSID1", fact(f64::from(ssid), "", "", "")), ("WIFI_SSID2", fact(0.0, "", "", "")), ("WIFI_SSID3", fact(0.0, "", "", "")), ("WIFI_SSID4", fact(0.0, "", "", "")), ("UART_BAUDRATE", fact(921600.0, "921600", "", ""))]);
        assert_eq!(
            esp8266(&lookup(&ap)),
            [row("Firmware Version", "1.2.3"), row("WiFi Mode", "AP Mode"), row("WiFi Channel", "6"), row("WiFi AP SSID", "Air"), row("WiFi AP Password", ""), row("UART Baud Rate", "921600")]
        );
        let station = HashMap::from([("WIFI_MODE", fact(1.0, "1", "", ""))]);
        let shown = esp8266(&lookup(&station));
        assert_eq!((shown[1].clone(), shown.iter().any(|r| r.label == "WiFi Channel")), (row("WiFi Mode", "Station Mode"), false), "the channel row hides in Station Mode");
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
        assert!(!shown.iter().any(|r| r.label == "Land Delay"), "a negative delay never lands");
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

    #[test]
    fn the_summary_reads_each_component_from_the_listed_array_as_a_backend_without_indexed_paths_answers() {
        struct ListOnly;
        impl Backend for ListOnly {
            fn get(&self, path: &str) -> String {
                match path {
                    COMPONENTS => json!({ "kind": "value", "value": [{ "kind": "object", "class": "APMFailsafesComponent", "name": "Failsafes" }, { "kind": "object", "class": "APMAirframeComponent", "name": "Frame" }] }).to_string(),
                    _ => json!({ "kind": "null" }).to_string(),
                }
            }
            fn get_fields(&self, path: &str, _fields: &str) -> String {
                if path == "vehicle" { json!({ "kind": "object", "multiRotor": true, "apmFirmware": true, "vehicleTypeString": "Quadrotor" }).to_string() } else { json!({ "kind": "null" }).to_string() }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let names: Vec<String> = setup_summary_view(&ListOnly, &[])["components"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap().to_string()).collect();
        assert_eq!(names, ["Failsafes", "Frame"], "the Android core answers vehicleComponents whole and null for vehicleComponents.N");
    }

    #[test]
    fn a_vehicle_type_without_safety_pages_summarises_like_apm_not_supported() {
        let boat = Vehicle { rover: true, safety_supported: false, ..copter() };
        let map: HashMap<&str, Value> = HashMap::new();
        assert_eq!(rows("APMFlightSafetyComponent", &lookup(&map), &boat), Some(vec![row("", "Not supported")]));
        assert_eq!(rows("APMFailsafesComponent", &lookup(&map), &boat), None, "its summaryQmlSource is an empty QUrl");
        assert!(crate::setup::apm_safety_supported("Submarine") && !crate::setup::apm_safety_supported("Surface vessel, boat, ship"));
    }
}
