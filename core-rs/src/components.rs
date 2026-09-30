use serde_json::{Value, json};

use crate::cmdinfo::VehicleClass;

pub const KNOWN_RADIO: i64 = 0;
pub const KNOWN_FLIGHT_MODES: i64 = 1;
pub const KNOWN_SENSORS: i64 = 2;
pub const KNOWN_SAFETY: i64 = 3;
pub const KNOWN_POWER: i64 = 4;
pub const KNOWN_JOYSTICK: i64 = 5;
pub const KNOWN_ESC: i64 = 6;
pub const UNKNOWN: i64 = 7;

const HELICOPTER: u8 = 4;
const SUBMARINE: u8 = 12;
const UDP_BRIDGE_COMPONENT: u8 = 240;
const RC_MAP: [&str; 4] = ["RCMAP_ROLL", "RCMAP_PITCH", "RCMAP_YAW", "RCMAP_THROTTLE"];

pub struct Vehicle<'a> {
    pub vehicle_type: u8,
    pub version: Option<(u8, u8, u8)>,
    pub parameter: &'a dyn Fn(u8, &str) -> Option<f64>,
    pub default_component: u8,
}

struct Entry {
    name: &'static str,
    class: &'static str,
    known: i64,
    requires_setup: bool,
    setup_complete: bool,
    armed: bool,
    flying: bool,
}

fn entry(name: &'static str, class: &'static str, known: i64, armed: bool, flying: bool) -> Entry {
    Entry { name, class, known, requires_setup: false, setup_complete: true, armed, flying }
}

fn at_least(version: Option<(u8, u8, u8)>, wanted: (u8, u8, u8)) -> bool {
    version.unwrap_or((0, 0, 0)) >= wanted
}

fn radio_complete(param: &dyn Fn(&str) -> Option<f64>) -> bool {
    let channels: Option<Vec<i64>> = RC_MAP.iter().map(|name| param(name).map(|v| v as i64).filter(|channel| *channel > 0)).collect();
    channels.is_some_and(|channels| {
        channels.iter().any(|channel| {
            [("MIN", 1100), ("MAX", 1900), ("TRIM", 1500)].iter().any(|(suffix, default)| param(&format!("RC{channel}_{suffix}")).map_or(0, |v| v as i64) != *default)
        })
    })
}

pub fn ardupilot(vehicle: &Vehicle) -> Vec<Value> {
    let param = |name: &str| (vehicle.parameter)(vehicle.default_component, name);
    let exists = |name: &str| param(name).is_some();
    let zeroed = |name: &str| param(name).unwrap_or(0.0);
    let class = crate::plandoc::vehicle_class(i64::from(vehicle.vehicle_type));
    let sub = vehicle.vehicle_type == SUBMARINE;
    let requires_frame = exists("FRAME_CLASS") && vehicle.vehicle_type != HELICOPTER;
    let sensors_complete = !crate::sensorcal::compass_setup_needed(&zeroed) && !crate::sensorcal::accel_setup_needed(&zeroed);
    let entries = [
        Some(Entry { requires_setup: requires_frame, setup_complete: !requires_frame || param("FRAME_CLASS").is_some_and(|v| v as i64 != 0), ..entry("Frame", "APMAirframeComponent", UNKNOWN, false, false) }),
        (!sub).then(|| Entry { requires_setup: true, setup_complete: radio_complete(&param), ..entry("Radio", "APMRadioComponent", KNOWN_RADIO, false, false) }),
        (!sub || !at_least(vehicle.version, (3, 5, 0))).then(|| Entry { requires_setup: true, ..entry("Flight Modes", "APMFlightModesComponent", KNOWN_FLIGHT_MODES, false, false) }),
        Some(Entry { requires_setup: true, setup_complete: sensors_complete, ..entry("Sensors", "APMSensorsComponent", KNOWN_SENSORS, false, false) }),
        exists("ARSPD_TYPE").then(|| entry("Airspeed", "APMAirspeedComponent", UNKNOWN, true, false)),
        Some(entry("Power", "APMPowerComponent", KNOWN_POWER, true, false)),
        (exists("MOT_PWM_TYPE") || exists("Q_M_PWM_TYPE")).then(|| entry("ESC", "APMESCComponent", KNOWN_ESC, true, false)),
        (!sub || at_least(vehicle.version, (3, 5, 3))).then(|| entry("Motors", "APMMotorComponent", UNKNOWN, true, false)),
        exists("SERVO1_MIN").then(|| entry("Servo Outputs", "APMServoComponent", UNKNOWN, false, false)),
        Some(entry("Flight Safety", "APMFlightSafetyComponent", KNOWN_SAFETY, true, true)),
        Some(entry("Failsafes", "APMFailsafesComponent", UNKNOWN, true, true)),
        (cfg!(debug_assertions) && matches!(class, VehicleClass::MultiRotor | VehicleClass::Rover) && exists("FOLL_ENABLE")).then(|| entry("Follow Me", "APMFollowComponent", UNKNOWN, true, true)),
        (vehicle.vehicle_type == HELICOPTER && at_least(vehicle.version, (4, 0, 0))).then(|| entry("Heli", "APMHeliComponent", UNKNOWN, true, false)),
        (!sub).then(|| entry("Tuning", "APMTuningComponent", UNKNOWN, true, false)),
        (class == VehicleClass::MultiRotor).then(|| entry("Tuning - Advanced", "APMAdvancedTuningCopterComponent", UNKNOWN, true, false)),
        Some(entry("Gimbal", "APMGimbalComponent", UNKNOWN, false, false)),
        sub.then(|| entry("Lights", "APMLightsComponent", UNKNOWN, false, false)),
        (sub && at_least(vehicle.version, (3, 5, 0))).then(|| entry("Frame", "APMSubFrameComponent", UNKNOWN, false, false)),
        (vehicle.parameter)(UDP_BRIDGE_COMPONENT, "SW_VER").is_some().then(|| entry("WiFi Bridge", "ESP8266Component", UNKNOWN, false, false)),
        Some(entry("Logging", "APMLoggingComponent", UNKNOWN, true, true)),
        Some(entry("Remote Support", "APMRemoteSupportComponent", UNKNOWN, false, false)),
        Some(entry("Joystick", "JoystickComponent", KNOWN_JOYSTICK, false, false)),
        Some(entry("Scripting", "ScriptingComponent", UNKNOWN, false, false)),
    ];
    let mut listed: Vec<Entry> = entries.into_iter().flatten().collect();
    listed.sort_by_key(|e| e.name.to_lowercase());
    listed
        .into_iter()
        .map(|e| json!({ "kind": "object", "name": e.name, "class": e.class, "KnownVehicleComponent": e.known, "requiresSetup": e.requires_setup, "setupComplete": e.setup_complete, "allowSetupWhileArmed": e.armed, "allowSetupWhileFlying": e.flying }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listed(vehicle_type: u8, version: Option<(u8, u8, u8)>, params: &'static [(u8, &'static str, f64)]) -> Vec<Value> {
        let parameter = move |component: u8, name: &str| params.iter().find(|(c, n, _)| *c == component && *n == name).map(|(_, _, v)| *v);
        ardupilot(&Vehicle { vehicle_type, version, parameter: &parameter, default_component: 1 })
    }

    fn names(list: &[Value]) -> Vec<&str> {
        list.iter().map(|c| c["name"].as_str().unwrap()).collect()
    }

    fn named<'a>(list: &'a [Value], name: &str) -> &'a Value {
        list.iter().find(|c| c["name"] == name).unwrap()
    }

    #[test]
    fn a_copter_lists_what_apm_auto_pilot_plugin_builds_sorted_by_name() {
        let copter = listed(2, Some((4, 5, 7)), &[(1, "FRAME_CLASS", 1.0), (1, "MOT_PWM_TYPE", 0.0), (1, "SERVO1_MIN", 1100.0), (1, "FOLL_ENABLE", 0.0)]);
        let expected: Vec<&str> = [
            "ESC", "Failsafes", "Flight Modes", "Flight Safety", "Frame", "Gimbal", "Joystick", "Logging", "Motors", "Power", "Radio", "Remote Support", "Scripting", "Sensors", "Servo Outputs", "Tuning", "Tuning - Advanced",
        ]
        .into_iter()
        .chain(cfg!(debug_assertions).then_some("Follow Me"))
        .collect::<Vec<_>>();
        assert_eq!(names(&copter).len(), expected.len());
        assert!(expected.iter().all(|name| names(&copter).contains(name)));
        assert_eq!(names(&copter).windows(2).all(|pair| pair[0].to_lowercase() <= pair[1].to_lowercase()), true);
        assert_eq!(named(&copter, "Frame")["setupComplete"], true);
        assert_eq!(named(&copter, "Radio")["setupComplete"], false, "unmapped attitude channels need a radio calibration");
        assert_eq!(named(&copter, "Sensors")["setupComplete"], false, "accel offsets all zero mean the accelerometer was never calibrated");
        let calibrated = listed(2, None, &[(1, "INS_ACCOFFS_X", 0.01)]);
        assert_eq!(named(&calibrated, "Sensors")["setupComplete"], true);
    }

    #[test]
    fn radio_is_complete_once_any_mapped_channel_left_its_defaults() {
        let at_defaults: &[(u8, &str, f64)] = &[(1, "RCMAP_ROLL", 1.0), (1, "RCMAP_PITCH", 2.0), (1, "RCMAP_YAW", 4.0), (1, "RCMAP_THROTTLE", 3.0), (1, "RC1_MIN", 1100.0), (1, "RC1_MAX", 1900.0), (1, "RC1_TRIM", 1500.0), (1, "RC2_MIN", 1100.0), (1, "RC2_MAX", 1900.0), (1, "RC2_TRIM", 1500.0), (1, "RC3_MIN", 1100.0), (1, "RC3_MAX", 1900.0), (1, "RC3_TRIM", 1500.0), (1, "RC4_MIN", 1100.0), (1, "RC4_MAX", 1900.0), (1, "RC4_TRIM", 1500.0)];
        assert_eq!(named(&listed(2, None, at_defaults), "Radio")["setupComplete"], false);
        let calibrated: &[(u8, &str, f64)] = &[(1, "RCMAP_ROLL", 1.0), (1, "RCMAP_PITCH", 2.0), (1, "RCMAP_YAW", 4.0), (1, "RCMAP_THROTTLE", 3.0), (1, "RC1_MIN", 982.0)];
        assert_eq!(named(&listed(2, None, calibrated), "Radio")["setupComplete"], true);
    }

    #[test]
    fn a_frame_class_of_zero_needs_setup_except_on_a_helicopter() {
        assert_eq!(named(&listed(2, None, &[(1, "FRAME_CLASS", 0.0)]), "Frame")["setupComplete"], false);
        assert_eq!(named(&listed(4, None, &[(1, "FRAME_CLASS", 0.0)]), "Frame")["requiresSetup"], false);
        assert!(names(&listed(4, Some((4, 0, 0)), &[])).contains(&"Heli"));
        assert!(!names(&listed(4, Some((3, 6, 0)), &[])).contains(&"Heli"));
    }

    #[test]
    fn a_sub_drops_radio_and_tuning_and_gains_lights_and_its_own_frame() {
        let sub_list = listed(12, Some((4, 1, 0)), &[]);
        let sub = names(&sub_list);
        assert!(!sub.contains(&"Radio") && !sub.contains(&"Tuning") && !sub.contains(&"Flight Modes"));
        assert!(sub.contains(&"Lights") && sub.iter().filter(|n| **n == "Frame").count() == 2);
        assert!(names(&listed(1, None, &[(240, "SW_VER", 1.0)])).contains(&"WiFi Bridge"));
    }
}
