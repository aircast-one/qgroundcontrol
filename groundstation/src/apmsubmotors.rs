use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.parameterManager.parametersReady", "vehicle.sub", "vehicle.apmFirmware", "vehicle.armed", "vehicle.flightMode", "vehicle.firmwareMajorVersion"];
pub const SUB_MOTORS_SCREEN: &str = "apmSubMotors";
pub const REVERSE: &str = "apmSubMotors.reverse";
pub const TEST: &str = "apmSubMotors.test";
pub const ARM: &str = "apmSubMotors.arm";
pub const AUTO_DETECT: &str = "apmSubMotors.autoDetect";
pub const MOTOR_DETECTION_MODE: &str = "Motor Detection";
const UNKNOWN_MOTOR_COUNT_SLIDERS: i64 = 8;
const AUTO_DETECT_MAJOR: i64 = 4;
const REVERSED: f64 = -1.0;
const NORMAL: f64 = 1.0;
const FULL_SLIDER: f64 = 100.0;
const WARNING: &str = "Moving the sliders will cause the motors to spin. Make sure the motors and propellers are clear from obstructions! The direction of the motor rotation is dependent on how the three phases of the motor are physically connected to the ESCs (if any two wires are swapped, the direction of rotation will flip). Because we cannot guarantee what order the phases are connected, the motor directions must be configured in software. When a slider is moved DOWN, the thruster should push air/water TOWARD the cable entering the housing. Click the checkbox to reverse the direction of the corresponding thruster.\n\nBlue Robotics thrusters are lubricated by water and are not designed to be run in air. Testing the thrusters in air is ok at low speeds for short periods of time. Extended operation of Blue Robotics in air may lead to overheating and permanent damage. Without water lubrication, Blue Robotics thrusters may also make some unpleasant noises when operated in air; this is normal.";
const AUTO_DETECT_HELP: &str = "This will attempt to automatically detect the direction (normal/reversed) of your thrusters.\nPlease place your vehicle in water, click the button, and wait. Note that the thrusters still need to be connected to the correct outputs (thrusters 2 and 3 can't be swapped, for example).";

static DETECTION_LOG: Mutex<String> = Mutex::new(String::new());

pub fn is_detection_text(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("thruster") || lower.contains("motor")
}

pub fn on_text(detecting: bool, text: &str) {
    if detecting && is_detection_text(text) {
        let mut log = DETECTION_LOG.lock().unwrap_or_else(PoisonError::into_inner);
        log.push_str(text);
        log.push('\n');
    }
}

fn in_detection(vehicle: &Value) -> bool {
    let detection = vehicle.get("motorDetectionFlightMode").and_then(Value::as_str).unwrap_or(MOTOR_DETECTION_MODE);
    vehicle.get("flightMode").and_then(Value::as_str) == Some(detection)
}

fn direction_path(motor: i64) -> String {
    format!("vehicle.parameterManager.getParameter(-1,MOT_{motor}_DIRECTION)")
}

fn reversed(backend: &dyn Backend, motor: i64) -> bool {
    object(&backend.get(&direction_path(motor))).get("rawValue").and_then(Value::as_f64) == Some(REVERSED)
}

pub fn slider_count(motor_count: Option<i64>) -> i64 {
    motor_count.filter(|c| *c != -1).unwrap_or(UNKNOWN_MOTOR_COUNT_SLIDERS)
}

pub fn test_percent(slider: f64, reversed: bool) -> f64 {
    if reversed { FULL_SLIDER - slider } else { slider }
}

pub fn apm_sub_motors_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", "sub,apmFirmware,armed,flightMode,motorDetectionFlightMode,motorCount,firmwareMajorVersion"));
    if !(flag(&vehicle, "sub") && flag(&vehicle, "apmFirmware")) {
        return json!({ "kind": "object", "class": "ApmSubMotors", "available": false });
    }
    let detecting = in_detection(&vehicle);
    let armed = flag(&vehicle, "armed");
    let count = slider_count(vehicle.get("motorCount").and_then(Value::as_i64));
    json!({
        "kind": "object",
        "class": "ApmSubMotors",
        "available": true,
        "armed": armed,
        "detecting": detecting,
        "canRunManualTest": armed && !detecting,
        "motors": (1..=count).map(|motor| json!({ "motor": motor, "reversed": reversed(backend, motor) })).collect::<Vec<_>>(),
        "warning": WARNING,
        "offersAutoDetect": vehicle.get("firmwareMajorVersion").and_then(Value::as_i64).is_some_and(|major| major >= AUTO_DETECT_MAJOR),
        "autoDetectHelp": AUTO_DETECT_HELP,
        "detectionMessages": DETECTION_LOG.lock().unwrap_or_else(PoisonError::into_inner).clone(),
    })
}

fn dispatch(backend: &dyn Backend, action: Value) -> Value {
    crate::guided::dispatch(backend, Some(action), crate::guided::active_id(backend), "", "[]")
}

pub fn run(backend: &dyn Backend, action: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Vec<Value>>(args).unwrap_or_default();
    let number = |at: usize| given.get(at).and_then(Value::as_f64);
    match action {
        REVERSE => match (number(0).filter(|m| *m >= 1.0), given.get(1).and_then(Value::as_bool)) {
            (Some(motor), Some(reverse)) => object(&backend.set(&format!("{}.rawValue", direction_path(motor as i64)), &json!({ "value": if reverse { REVERSED } else { NORMAL } }).to_string())),
            _ => json!({ "ok": false, "reason": "apmSubMotors.reverse takes a motor number and whether it is reversed" }),
        },
        TEST => {
            let vehicle = object(&backend.get_fields("vehicle", "armed,flightMode,motorDetectionFlightMode"));
            if !flag(&vehicle, "armed") || in_detection(&vehicle) {
                return json!({ "ok": false, "reason": "Arm the vehicle with the switch to test the motors." });
            }
            match (number(0).filter(|i| *i >= 0.0), number(1).filter(|v| (0.0..=FULL_SLIDER).contains(v))) {
                (Some(index), Some(slider)) => {
                    let percent = test_percent(slider, reversed(backend, index as i64 + 1));
                    object(&backend.invoke("vehicle.motorTest", &json!([index, percent, 0, false]).to_string()))
                }
                _ => json!({ "ok": false, "reason": "apmSubMotors.test takes a motor index and a slider value from 0 to 100" }),
            }
        }
        ARM => match given.first().and_then(Value::as_bool) {
            Some(arm) => dispatch(backend, json!({ "action": "arm", "arm": arm })),
            None => json!({ "ok": false, "reason": "apmSubMotors.arm takes true or false" }),
        },
        AUTO_DETECT => {
            DETECTION_LOG.lock().unwrap_or_else(PoisonError::into_inner).clear();
            dispatch(backend, json!({ "action": "setModeAndArm", "mode": MOTOR_DETECTION_MODE }))
        }
        _ => json!({ "ok": false, "reason": format!("{action} is not a Sub motor action") }),
    }
}

pub fn owns(path: &str) -> bool {
    [REVERSE, TEST, ARM, AUTO_DETECT].contains(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sliders_and_reversed_thrusters_follow_the_sub_motor_page() {
        assert_eq!(slider_count(Some(-1)), 8, "an unknown frame still shows eight sliders");
        assert_eq!(slider_count(None), 8);
        assert_eq!(slider_count(Some(6)), 6);
        assert_eq!(test_percent(70.0, false), 70.0);
        assert_eq!(test_percent(70.0, true), 30.0, "a reversed thruster is driven from the other end of the slider");
    }

    #[test]
    fn only_thruster_texts_in_detection_mode_are_kept() {
        assert!(is_detection_text("Thruster 1 is reversed"));
        assert!(is_detection_text("MOTOR 3 ok"));
        assert!(!is_detection_text("EKF3 IMU0 is using GPS"));
    }
}
