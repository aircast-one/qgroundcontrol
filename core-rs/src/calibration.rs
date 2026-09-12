use serde_json::{Value, json};

use crate::read::{flag, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["sensorsCal", "vehicles.activeVehicleAvailable"];

const SIDES: &[(&str, &str)] = &[("Down", "Level"), ("UpsideDown", "Upside down"), ("Left", "Left side"), ("Right", "Right side"), ("NoseDown", "Nose down"), ("TailDown", "Tail down")];
const ACCEL_FIRST: &str = "Calibrate the accelerometer first.";

struct Routine {
    id: &'static str,
    title: &'static str,
    method: &'static str,
    arguments: &'static [bool],
    needs_accel_first: bool,
    explanation: &'static str,
    warning: &'static str,
    spins_propeller: bool,
}

const ROUTINES: &[Routine] = &[
    Routine { id: "accelerometer", title: "Accelerometer", method: "calibrateAccel", arguments: &[false], needs_accel_first: false, explanation: "Hold the vehicle in each orientation it asks for.", warning: "", spins_propeller: false },
    Routine { id: "compass", title: "Compass", method: "calibrateCompass", arguments: &[], needs_accel_first: true, explanation: "Rotate the vehicle about every axis until each side is done.", warning: "", spins_propeller: false },
    Routine { id: "levelHorizon", title: "Level Horizon", method: "levelHorizon", arguments: &[], needs_accel_first: true, explanation: "Place the vehicle in its level flight position", warning: "", spins_propeller: false },
    Routine { id: "gyro", title: "Gyro", method: "calibrateGyro", arguments: &[], needs_accel_first: false, explanation: "Leave the vehicle still while the gyros settle.", warning: "", spins_propeller: false },
    Routine { id: "pressure", title: "Pressure", method: "calibratePressure", arguments: &[], needs_accel_first: false, explanation: "Zero the barometer at the current altitude.", warning: "", spins_propeller: false },
    Routine { id: "compassMot", title: "CompassMot", method: "calibrateMotorInterference", arguments: &[], needs_accel_first: false, explanation: "Disconnect your props, flip them over and rotate them one position around the frame. In this configuration they should push the copter down into the ground when the throttle is raised. Secure the copter so that it does not move, turn on your transmitter and keep throttle at zero.", warning: "This spins the motors. CompassMot only works well if you have a battery current monitor, because the magnetic interference is linear with current drawn.", spins_propeller: true },
];

pub fn needs_attention(accel: bool, compass: bool) -> &'static str {
    match (accel, compass) {
        (true, true) => "The accelerometer and compass both need calibrating.",
        (true, false) => "The accelerometer needs calibrating.",
        (false, true) => "The compass needs calibrating.",
        (false, false) => "",
    }
}

fn sides(cal: &Value) -> Vec<Value> {
    SIDES
        .iter()
        .map(|(key, title)| {
            let side = |suffix: &str| flag(cal, &format!("orientationCal{key}Side{suffix}"));
            let stage = match (side("Done"), side("InProgress")) {
                (true, _) => "done",
                (false, true) => "inProgress",
                _ => "waiting",
            };
            json!({ "key": key, "title": title, "visible": side("Visible"), "stage": stage, "rotate": side("Rotate") })
        })
        .collect()
}

fn routines(connected: bool, busy: bool, accel_needed: bool) -> Vec<Value> {
    ROUTINES
        .iter()
        .map(|r| {
            let blocked = r.needs_accel_first && accel_needed;
            json!({
                "id": r.id,
                "title": r.title,
                "invocation": format!("sensorsCal.{}", r.method),
                "arguments": r.arguments,
                "blocked": blocked,
                "enabled": connected && !busy && !blocked,
                "description": if blocked { ACCEL_FIRST } else { r.explanation },
                "warning": r.warning,
                "spinsPropeller": r.spins_propeller,
            })
        })
        .collect()
}

pub fn calibration_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let cal = object(&backend.get("sensorsCal"));
    let connected = cal.get("kind").and_then(Value::as_str) == Some("object");
    let in_progress = flag(&cal, "calibrationInProgress");
    let waiting_for_cancel = flag(&cal, "waitingForCancel");
    let busy = in_progress || waiting_for_cancel;
    let accel_needed = flag(&cal, "accelSetupNeeded");
    let compass_needed = flag(&cal, "compassSetupNeeded");
    let progress = cal.get("calProgress").and_then(Value::as_f64).unwrap_or(0.0);
    let listed = sides(&cal);
    json!({
        "kind": "object",
        "class": "Calibration",
        "connected": connected,
        "inProgress": in_progress,
        "busy": busy,
        "waitingForCancel": waiting_for_cancel,
        "showsSides": flag(&cal, "showOrientationCalArea"),
        "nextEnabled": flag(&cal, "nextEnabled"),
        "cancelEnabled": flag(&cal, "cancelEnabled"),
        "progress": progress,
        "progressText": format!("{progress:.0}%"),
        "helpText": text(&cal, "orientationHelpText"),
        "statusText": text(&cal, "statusText"),
        "accelNeeded": accel_needed,
        "compassNeeded": compass_needed,
        "needsAttention": needs_attention(accel_needed, compass_needed),
        "visibleSides": listed.iter().filter(|s| s["visible"] == true).cloned().collect::<Vec<_>>(),
        "sides": listed,
        "routines": routines(connected, busy, accel_needed),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(Value);
    impl Backend for Fake {
        fn get(&self, _p: &str) -> String { self.0.to_string() }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn without_an_apm_vehicle_everything_is_disabled_but_listed() {
        let view = calibration_view(&Fake(json!({ "kind": "null" })), &[]);
        assert_eq!(view["connected"], false);
        assert_eq!(view["routines"].as_array().unwrap().len(), ROUTINES.len());
        assert!(view["routines"].as_array().unwrap().iter().all(|r| r["enabled"] == false));
        assert_eq!(view["sides"].as_array().unwrap().len(), 6);
        assert_eq!(view["needsAttention"], "");
    }

    #[test]
    fn a_routine_that_spins_a_propeller_says_so_and_says_why_it_matters() {
        let spinning: Vec<&Routine> = ROUTINES.iter().filter(|r| r.spins_propeller).collect();
        assert_eq!(spinning.len(), 1, "one routine turns the motors today and the flag is what a head gates on - hiding it instead left the core describing a vehicle that does not exist, and a head built the row by hand against an invokable the contract knew nothing about");
        for routine in &spinning {
            assert!(!routine.warning.is_empty(), "{} turns the motors and carries no warning", routine.method);
            assert!(routine.warning.contains("spins the motors"), "the warning has to name what happens rather than counsel care: {}", routine.warning);
        }
        for routine in ROUTINES.iter().filter(|r| !r.spins_propeller) {
            assert_ne!(routine.method, "calibrateMotorInterference", "this one turns the motors whatever the flag says");
        }
        assert!(spinning[0].explanation.contains("push the copter down into the ground"), "QGC's own instruction, because inverting the props is what makes the test safe and a paraphrase could lose it");
    }

    #[test]
    fn compass_and_level_wait_for_the_accelerometer() {
        let view = calibration_view(&Fake(json!({ "kind": "object", "accelSetupNeeded": true, "compassSetupNeeded": true })), &[]);
        let routines = view["routines"].as_array().unwrap();
        assert_eq!(routines[0]["enabled"], true);
        assert_eq!(routines[1]["blocked"], true);
        assert_eq!(routines[1]["description"], ACCEL_FIRST);
        assert_eq!(routines[2]["blocked"], true);
        assert_eq!(routines[3]["enabled"], true);
        assert_eq!(view["needsAttention"], "The accelerometer and compass both need calibrating.");
        assert_eq!(routines[0]["invocation"], "sensorsCal.calibrateAccel");
        assert_eq!(routines[0]["arguments"][0], false);
    }

    #[test]
    fn a_running_calibration_reports_its_sides_and_blocks_new_starts() {
        let view = calibration_view(&Fake(json!({
            "kind": "object", "calibrationInProgress": true, "showOrientationCalArea": true, "calProgress": 33.4,
            "orientationCalDownSideVisible": true, "orientationCalDownSideDone": true,
            "orientationCalLeftSideVisible": true, "orientationCalLeftSideInProgress": true, "orientationCalLeftSideRotate": true,
        })), &[]);
        assert_eq!(view["busy"], true);
        assert_eq!(view["progressText"], "33%");
        assert_eq!(view["visibleSides"].as_array().unwrap().len(), 2);
        assert_eq!(view["visibleSides"][0]["stage"], "done");
        assert_eq!(view["visibleSides"][1]["stage"], "inProgress");
        assert_eq!(view["visibleSides"][1]["rotate"], true);
        assert!(view["routines"].as_array().unwrap().iter().all(|r| r["enabled"] == false));
    }
}
