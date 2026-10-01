use serde_json::{Value, json};

use crate::read::{flag, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["sensorsCal", "vehicles.activeVehicleAvailable", "vehicle.px4Firmware", "vehicle.coordinate", "positionManager.gcsPosition", "vehicle.multiRotor", "vehicle.rover", "vehicle.sub", "vehicle.fixedWing"];

const SIDES: &[(&str, &str)] = &[("Down", "Level"), ("UpsideDown", "Upside down"), ("Left", "Left side"), ("Right", "Right side"), ("NoseDown", "Nose down"), ("TailDown", "Tail down")];
const COMPASS_NORTH: &str = "calibrateCompassNorth";
const PRIORITY_PARAMS: [&str; 3] = ["COMPASS_PRIO1_ID", "COMPASS_PRIO2_ID", "COMPASS_PRIO3_ID"];
const FAST_COMPASS_HELP: &str = "Fast compass calibration given vehicle position and yaw. This results in zero diagonal and off-diagonal elements, so is only suitable for vehicles where the field is close to spherical. It is useful for large vehicles where moving the vehicle to calibrate it is difficult. Point the vehicle North before using it.";
const ACCEL_FIRST: &str = "Calibrate the accelerometer first.";

pub struct Classes {
    pub multi_rotor: bool,
    pub rover: bool,
    pub sub: bool,
    pub fixed_wing: bool,
}

struct Routine {
    id: &'static str,
    title: &'static str,
    method: &'static str,
    arguments: &'static [bool],
    blocked_by: &'static [&'static str],
    blocked_text: &'static str,
    needs: Option<&'static str>,
    shown_if: Option<&'static str>,
    explanation: &'static str,
    dialog_help: &'static str,
    warning: &'static str,
    spins_propeller: bool,
    visible: fn(&Classes) -> bool,
    on_fixed_wing: Option<(&'static str, &'static str)>,
}

const ANY_VEHICLE: fn(&Classes) -> bool = |_| true;
const ROTOR_OR_GROUND: fn(&Classes) -> bool = |c| c.multi_rotor || c.rover || c.sub;
const ACCEL_NEEDED: &str = "accelSetupNeeded";
const GYRO_NEEDED: &str = "gyroSetupNeeded";
const COMPASS_NEEDED: &str = "compassSetupNeeded";

const APM_ROUTINES: &[Routine] = &[
    Routine { id: "accelerometer", title: "Accelerometer", method: "calibrateAccel", arguments: &[false], blocked_by: &[], blocked_text: "", needs: Some(ACCEL_NEEDED), shown_if: None, explanation: "Hold the vehicle in each orientation it asks for.", dialog_help: "", warning: "", spins_propeller: false, visible: ANY_VEHICLE, on_fixed_wing: None },
    Routine { id: "compass", title: "Compass", method: "calibrateCompass", arguments: &[], blocked_by: &[ACCEL_NEEDED], blocked_text: ACCEL_FIRST, needs: Some(COMPASS_NEEDED), shown_if: None, explanation: "Rotate the vehicle about every axis until each side is done.", dialog_help: "", warning: "", spins_propeller: false, visible: ANY_VEHICLE, on_fixed_wing: None },
    Routine { id: "levelHorizon", title: "Level Horizon", method: "levelHorizon", arguments: &[], blocked_by: &[ACCEL_NEEDED], blocked_text: ACCEL_FIRST, needs: None, shown_if: None, explanation: "Place the vehicle in its level flight position", dialog_help: "", warning: "", spins_propeller: false, visible: ANY_VEHICLE, on_fixed_wing: None },
    Routine { id: "gyro", title: "Gyro", method: "calibrateGyro", arguments: &[], blocked_by: &[], blocked_text: "", needs: None, shown_if: None, explanation: "Leave the vehicle still while the gyros settle.", dialog_help: "", warning: "", spins_propeller: false, visible: ROTOR_OR_GROUND, on_fixed_wing: None },
    Routine { id: "pressure", title: "Pressure", method: "calibratePressure", arguments: &[], blocked_by: &[], blocked_text: "", needs: None, shown_if: None, explanation: "Zero the barometer at the current altitude.", dialog_help: "", warning: "", spins_propeller: false, visible: ANY_VEHICLE, on_fixed_wing: Some(("Baro/Airspeed", "Shield the airspeed sensor from the wind and leave the holes clear.")) },
    Routine { id: "compassMot", title: "CompassMot", method: "calibrateMotorInterference", arguments: &[], blocked_by: &[], blocked_text: "", needs: None, shown_if: None, explanation: "Disconnect your props, flip them over and rotate them one position around the frame. In this configuration they should push the copter down into the ground when the throttle is raised. Secure the copter so that it does not move, turn on your transmitter and keep throttle at zero.", dialog_help: "", warning: "This spins the motors. CompassMot only works well if you have a battery current monitor, because the magnetic interference is linear with current drawn.", spins_propeller: true, visible: ANY_VEHICLE, on_fixed_wing: None },
];

const PX4_ROUTINES: &[Routine] = &[
    Routine { id: "compass", title: "Compass", method: "calibrateCompass", arguments: &[], blocked_by: &[], blocked_text: "", needs: Some(COMPASS_NEEDED), shown_if: Some("magEnabled"), explanation: "Rotate the vehicle through several positions", dialog_help: "For Compass calibration you will need to rotate your vehicle through a number of positions.", warning: "", spins_propeller: false, visible: ANY_VEHICLE, on_fixed_wing: None },
    Routine { id: "gyro", title: "Gyroscope", method: "calibrateGyro", arguments: &[], blocked_by: &[], blocked_text: "", needs: Some(GYRO_NEEDED), shown_if: None, explanation: "Place the vehicle on a surface and leave it still", dialog_help: "For Gyroscope calibration you will need to place your vehicle on a surface and leave it still.", warning: "", spins_propeller: false, visible: ANY_VEHICLE, on_fixed_wing: None },
    Routine { id: "accelerometer", title: "Accelerometer", method: "calibrateAccel", arguments: &[], blocked_by: &[], blocked_text: "", needs: Some(ACCEL_NEEDED), shown_if: None, explanation: "Hold the vehicle still on all six sides", dialog_help: "For Accelerometer calibration you will need to place your vehicle on all six sides on a perfectly level surface and hold it still in each orientation for a few seconds.", warning: "", spins_propeller: false, visible: ANY_VEHICLE, on_fixed_wing: None },
    Routine { id: "levelHorizon", title: "Level Horizon", method: "levelHorizon", arguments: &[], blocked_by: &[ACCEL_NEEDED, GYRO_NEEDED], blocked_text: "Calibrate Accelerometer and Gyroscope first", needs: None, shown_if: None, explanation: "Place the vehicle in its level flight position", dialog_help: "To level the horizon you need to place the vehicle in its level flight position and leave still.", warning: "", spins_propeller: false, visible: ANY_VEHICLE, on_fixed_wing: None },
    Routine { id: "airspeed", title: "Airspeed", method: "calibrateAirspeed", arguments: &[], blocked_by: &[], blocked_text: "", needs: Some("airspeedSetupNeeded"), shown_if: Some("airspeedSupported"), explanation: "Shield the airspeed sensor from the wind", dialog_help: "For Airspeed calibration you will need to keep your airspeed sensor out of any wind and then blow across the sensor. Do not touch the sensor or obstruct any holes during the calibration.", warning: "", spins_propeller: false, visible: ANY_VEHICLE, on_fixed_wing: None },
];

fn table(cal: &Value) -> &'static [Routine] {
    if flag(cal, "px4") { PX4_ROUTINES } else { APM_ROUTINES }
}

fn gated_as(method: &str) -> &str {
    if method == COMPASS_NORTH { "calibrateCompass" } else { method }
}

fn coordinate(backend: &dyn Backend, path: &str) -> Value {
    let point = object(&backend.get(path));
    let valid = flag(&point, "valid") && point.get("latitude").and_then(Value::as_f64).is_some();
    json!({ "valid": valid, "latitude": if valid { point["latitude"].clone() } else { Value::Null }, "longitude": if valid { point["longitude"].clone() } else { Value::Null } })
}

fn fast_compass(backend: &dyn Backend, cal: &Value) -> Value {
    if flag(cal, "px4") {
        return Value::Null;
    }
    json!({
        "invocation": format!("sensorsCal.{COMPASS_NORTH}"),
        "help": FAST_COMPASS_HELP,
        "vehicleHasPosition": coordinate(backend, "vehicle.coordinate")["valid"],
        "gcsPosition": coordinate(backend, "positionManager.gcsPosition"),
    })
}

pub fn compass_priority_mask(priorities: &[Option<f64>]) -> u8 {
    priorities.iter().enumerate().filter(|(_, id)| id.is_some_and(|id| id != 0.0)).map(|(i, _)| 1u8 << i).sum()
}

fn priority_mask(backend: &dyn Backend) -> u8 {
    let priorities: Vec<Option<f64>> = PRIORITY_PARAMS
        .iter()
        .map(|name| object(&backend.get(&format!("vehicle.parameterManager.getParameter(-1,{name})"))).get("rawValue").and_then(Value::as_f64))
        .collect();
    compass_priority_mask(&priorities)
}

pub fn north_arguments(given: &str, mask: u8) -> Option<Value> {
    let given = serde_json::from_str::<Vec<Value>>(given).ok()?;
    let degrees = |at: usize, limit: f64| given.get(at).and_then(Value::as_f64).filter(|v| v.is_finite() && v.abs() <= limit);
    Some(json!([degrees(0, 90.0)?, degrees(1, 180.0)?, mask]))
}

fn drives_this_firmware(cal: &Value, vehicle: &Value) -> bool {
    cal.get("kind").and_then(Value::as_str) == Some("object") && flag(cal, "px4") == flag(vehicle, "px4Firmware")
}

fn offered(routine: &Routine, cal: &Value, classes: Option<&Classes>) -> bool {
    classes.is_none_or(|c| (routine.visible)(c)) && routine.shown_if.is_none_or(|key| flag(cal, key))
}

fn blocked(routine: &Routine, cal: &Value) -> bool {
    routine.blocked_by.iter().any(|key| flag(cal, key))
}

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

fn routines(cal: &Value, connected: bool, busy: bool, classes: Option<&Classes>) -> Vec<Value> {
    table(cal)
        .iter()
        .filter(|r| offered(r, cal, classes))
        .map(|r| {
            let blocked = blocked(r, cal);
            let (title, explanation) = match (classes.is_some_and(|c| c.fixed_wing), r.on_fixed_wing) {
                (true, Some(named)) => named,
                _ => (r.title, r.explanation),
            };
            let status = match (blocked, r.needs.map(|key| flag(cal, key))) {
                (true, _) => r.blocked_text,
                (false, Some(true)) => "Not calibrated",
                (false, Some(false)) => "Calibrated",
                (false, None) => "",
            };
            json!({
                "id": r.id,
                "title": title,
                "invocation": format!("sensorsCal.{}", r.method),
                "arguments": r.arguments,
                "blocked": blocked,
                "enabled": connected && !busy && !blocked,
                "description": if blocked { r.blocked_text } else { explanation },
                "dialogHelp": r.dialog_help,
                "status": status,
                "warning": r.warning,
                "spinsPropeller": r.spins_propeller,
            })
        })
        .collect()
}

pub fn calibration_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let cal = object(&backend.get("sensorsCal"));
    let vehicle = object(&backend.get_fields("vehicle", "px4Firmware,multiRotor,rover,sub,fixedWing"));
    let classes = vehicle.get("multiRotor").map(|_| Classes {
        multi_rotor: flag(&vehicle, "multiRotor"),
        rover: flag(&vehicle, "rover"),
        sub: flag(&vehicle, "sub"),
        fixed_wing: flag(&vehicle, "fixedWing"),
    });
    let connected = drives_this_firmware(&cal, &vehicle);
    let in_progress = flag(&cal, "calibrationInProgress");
    let waiting_for_cancel = flag(&cal, "waitingForCancel");
    let busy = in_progress || waiting_for_cancel;
    let accel_needed = flag(&cal, ACCEL_NEEDED);
    let compass_needed = flag(&cal, COMPASS_NEEDED);
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
        "progressText": format!("{:.0}%", progress * 100.0),
        "helpText": text(&cal, "orientationHelpText"),
        "statusText": text(&cal, "statusText"),
        "accelNeeded": accel_needed,
        "compassNeeded": compass_needed,
        "needsAttention": needs_attention(accel_needed, compass_needed),
        "visibleSides": listed.iter().filter(|s| s["visible"] == true).cloned().collect::<Vec<_>>(),
        "sides": listed,
        "px4": flag(&cal, "px4"),
        "settingsTitle": if flag(&cal, "px4") { "Orientations" } else { "Sensor Settings" },
        "routines": routines(&cal, connected, busy, classes.as_ref()),
        "fastCompass": fast_compass(backend, &cal),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Start(&'static str),
    Next,
    Cancel,
}

pub const METHODS: &[&str] = &["calibrateAccel", "calibrateCompass", "levelHorizon", "calibrateGyro", "calibratePressure", "calibrateMotorInterference", "calibrateAirspeed", COMPASS_NORTH];

#[derive(Clone, Copy, Debug, Default)]
struct Session {
    connected: bool,
    busy: bool,
    next_enabled: bool,
    cancel_enabled: bool,
}

fn refusal(action: Action, state: Session, cal: &Value, classes: Option<&Classes>) -> Option<(&'static str, &'static str)> {
    if !state.connected {
        return Some(("noVehicle", "No vehicle is connected."));
    }
    match action {
        Action::Next if !state.next_enabled => Some(("waiting", "The calibration is not waiting for Next.")),
        Action::Cancel if !state.cancel_enabled => Some(("notCancellable", "This calibration cannot be cancelled now.")),
        Action::Next | Action::Cancel => None,
        Action::Start(method) => {
            let Some(routine) = table(cal).iter().find(|r| r.method == gated_as(method)) else { return Some(("notForThisVehicle", "This calibration does not apply to this vehicle.")) };
            match () {
                _ if state.busy => Some(("busy", "Another calibration is still running.")),
                _ if !offered(routine, cal, classes) => Some(("notForThisVehicle", "This calibration does not apply to this vehicle.")),
                _ if blocked(routine, cal) => Some(("blocked", routine.blocked_text)),
                _ => None,
            }
        }
    }
}

pub fn act(backend: &dyn Backend, action: Action, path: &str, args: &str) -> Value {
    let cal = object(&backend.get("sensorsCal"));
    let vehicle = object(&backend.get_fields("vehicle", "px4Firmware,multiRotor,rover,sub,fixedWing"));
    let classes = vehicle.get("multiRotor").map(|_| Classes {
        multi_rotor: flag(&vehicle, "multiRotor"),
        rover: flag(&vehicle, "rover"),
        sub: flag(&vehicle, "sub"),
        fixed_wing: flag(&vehicle, "fixedWing"),
    });
    let state = Session {
        connected: drives_this_firmware(&cal, &vehicle),
        busy: flag(&cal, "calibrationInProgress") || flag(&cal, "waitingForCancel"),
        next_enabled: flag(&cal, "nextEnabled"),
        cancel_enabled: flag(&cal, "cancelEnabled"),
    };
    if let Some((token, reason)) = refusal(action, state, &cal, classes.as_ref()) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let args = match action {
        Action::Start(COMPASS_NORTH) if flag(&cal, "px4") => return json!({ "ok": false, "refusal": "notForThisVehicle", "reason": "This calibration does not apply to this vehicle." }),
        Action::Start(COMPASS_NORTH) => match north_arguments(args, priority_mask(backend)) {
            Some(north) => north.to_string(),
            None => return json!({ "ok": false, "refusal": "badPosition", "reason": "Enter a valid latitude and longitude." }),
        },
        Action::Start(method) => {
            let given = serde_json::from_str::<Value>(args).ok().filter(|a| a.as_array().is_some_and(|a| !a.is_empty()));
            let defaults = table(&cal).iter().find(|r| r.method == method).map_or(json!([]), |r| json!(r.arguments));
            given.unwrap_or(defaults).to_string()
        }
        _ => "[]".to_string(),
    };
    let dispatched = flag(&object(&backend.invoke(path, &args)), "ok");
    json!({
        "ok": dispatched,
        "refusal": Value::Null,
        "reason": match dispatched { true => Value::Null, false => json!("The sensor calibration did not take the request.") },
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

    struct Airframe(Value, Value);
    impl Backend for Airframe {
        fn get(&self, _p: &str) -> String { self.0.to_string() }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "vehicle" => self.1.to_string(),
                _ => self.0.to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn listed(vehicle: Value) -> Vec<(String, String)> {
        let cal = json!({ "kind": "object", "calibrationInProgress": false, "accelSetupNeeded": false });
        calibration_view(&Airframe(cal, vehicle), &[])["routines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r["id"].as_str().unwrap().to_string(), r["title"].as_str().unwrap().to_string()))
            .collect()
    }

    #[test]
    fn a_fixed_wing_is_offered_the_airspeed_calibration_and_not_the_gyro() {
        let plane = listed(json!({ "kind": "object", "multiRotor": false, "rover": false, "sub": false, "fixedWing": true }));
        assert!(!plane.iter().any(|(id, _)| id == "gyro"), "QGC shows the gyro row only to a multirotor, a rover or a sub");
        let pressure = plane.iter().find(|(id, _)| id == "pressure").expect("a fixed wing still calibrates its barometer");
        assert_eq!(pressure.1, "Baro/Airspeed", "on a plane the same routine zeroes the airspeed sensor too, and the name is what tells the operator to shield it");
    }

    #[test]
    fn a_multirotor_keeps_the_gyro_and_the_plain_pressure_name() {
        let copter = listed(json!({ "kind": "object", "multiRotor": true, "rover": false, "sub": false, "fixedWing": false }));
        assert!(copter.iter().any(|(id, _)| id == "gyro"));
        assert_eq!(copter.iter().find(|(id, _)| id == "pressure").unwrap().1, "Pressure");
    }

    #[test]
    fn a_vehicle_that_never_said_what_it_is_keeps_every_routine() {
        let unknown = listed(json!({ "kind": "null" }));
        assert!(unknown.iter().any(|(id, _)| id == "gyro"), "a read that did not answer is not a vehicle without a gyro");
    }

    #[test]
    fn without_an_apm_vehicle_everything_is_disabled_but_listed() {
        let view = calibration_view(&Fake(json!({ "kind": "null" })), &[]);
        assert_eq!(view["connected"], false);
        assert_eq!(view["routines"].as_array().unwrap().len(), APM_ROUTINES.len());
        assert!(view["routines"].as_array().unwrap().iter().all(|r| r["enabled"] == false));
        assert_eq!(view["sides"].as_array().unwrap().len(), 6);
        assert_eq!(view["needsAttention"], "");
    }

    #[test]
    fn a_routine_that_spins_a_propeller_says_so_and_says_why_it_matters() {
        let spinning: Vec<&Routine> = APM_ROUTINES.iter().filter(|r| r.spins_propeller).collect();
        assert_eq!(spinning.len(), 1, "one routine turns the motors today and the flag is what a head gates on - hiding it instead left the core describing a vehicle that does not exist, and a head built the row by hand against an invokable the contract knew nothing about");
        for routine in &spinning {
            assert!(!routine.warning.is_empty(), "{} turns the motors and carries no warning", routine.method);
            assert!(routine.warning.contains("spins the motors"), "the warning has to name what happens rather than counsel care: {}", routine.warning);
        }
        for routine in APM_ROUTINES.iter().filter(|r| !r.spins_propeller) {
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
            "kind": "object", "calibrationInProgress": true, "showOrientationCalArea": true, "calProgress": 0.334,
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

    #[test]
    fn a_calibration_that_would_start_on_top_of_another_is_refused() {
        let idle = Session { connected: true, busy: false, next_enabled: false, cancel_enabled: false };
        let apm = json!({ "kind": "object" });
        let copter = Classes { multi_rotor: true, rover: false, sub: false, fixed_wing: false };
        let plane = Classes { multi_rotor: false, rover: false, sub: false, fixed_wing: true };
        let token = |action, state, classes| refusal(action, state, &apm, classes).map(|(t, _)| t);
        assert_eq!(METHODS, APM_ROUTINES.iter().chain(PX4_ROUTINES).map(|r| r.method).chain([COMPASS_NORTH]).fold(Vec::new(), |seen, m| if seen.contains(&m) { seen } else { [seen, vec![m]].concat() }).as_slice(), "a routine the view offers and the core does not claim reaches Qt ungated, and a claimed one the view does not list starts with no table entry to gate it");
        APM_ROUTINES.iter().for_each(|r| assert_eq!(token(Action::Start(r.method), idle, Some(&copter)), None, "{}", r.method));
        assert_eq!(token(Action::Start("calibrateAirspeed"), idle, Some(&plane)), Some("notForThisVehicle"), "airspeed is a PX4 routine");
        assert_eq!(
            token(Action::Start("calibrateMotorInterference"), Session { busy: true, ..idle }, Some(&copter)),
            Some("busy"),
            "calibrateMotorInterference overwrites _calTypeInProgress without asking, so a second start mid-run spins the props under a calibration the firmware is still running"
        );
        assert_eq!(refusal(Action::Start("calibrateCompass"), idle, &json!({ "kind": "object", "accelSetupNeeded": true }), Some(&copter)).map(|(t, _)| t), Some("blocked"));
        assert_eq!(token(Action::Start("calibrateGyro"), idle, Some(&plane)), Some("notForThisVehicle"));
        assert_eq!(token(Action::Start("calibrateGyro"), idle, None), None, "a vehicle whose class has not been read yet is not refused on a guess");
        assert_eq!(token(Action::Next, idle, None), Some("waiting"), "nextClicked sends a COMMAND_ACK whether or not a step is waiting for one");
        assert_eq!(token(Action::Next, Session { next_enabled: true, ..idle }, None), None);
        assert_eq!(token(Action::Cancel, idle, None), Some("notCancellable"));
        assert_eq!(token(Action::Cancel, Session { busy: true, cancel_enabled: true, ..idle }, None), None, "cancel is the one action a running calibration must not refuse as busy");
        assert_eq!(token(Action::Start("calibrateAccel"), Session { connected: false, ..idle }, None), Some("noVehicle"));
    }

    #[test]
    fn fast_compass_sends_the_priority_mask_and_a_checked_position() {
        assert_eq!(compass_priority_mask(&[Some(97_539.0), Some(0.0), Some(131_874.0)]), 0b101, "QGC's compassMask: one bit per priority slot that holds a device");
        assert_eq!(compass_priority_mask(&[None, None, None]), 0);
        assert_eq!(north_arguments("[41.7, 44.8]", 0b11), Some(json!([41.7, 44.8, 3])));
        assert_eq!(north_arguments("[91.0, 44.8]", 1), None);
        assert_eq!(north_arguments("[\"x\", 44.8]", 1), None, "QGC drops the request when parseFloat gives NaN");
        let view = calibration_view(&Fake(json!({ "kind": "object", "valid": true, "latitude": 41.7, "longitude": 44.8 })), &[]);
        assert_eq!(view["fastCompass"]["invocation"], "sensorsCal.calibrateCompassNorth");
        assert_eq!(view["fastCompass"]["gcsPosition"]["latitude"], 41.7);
        let px4 = calibration_view(&Fake(json!({ "kind": "object", "px4": true, "px4Firmware": true })), &[]);
        assert!(px4["fastCompass"].is_null(), "PX4 has no fixed-yaw compass calibration");
        let token = |cal: Value| refusal(Action::Start(COMPASS_NORTH), Session { connected: true, ..Session::default() }, &cal, None).map(|(t, _)| t);
        assert_eq!(token(json!({ "kind": "object", "accelSetupNeeded": true })), Some("blocked"), "the fast path shares the compass button and its accelerometer gate");
        assert_eq!(token(json!({ "kind": "object" })), None);
    }

    #[test]
    fn an_ardupilot_controller_never_drives_a_px4_vehicle() {
        let qt_mode = calibration_view(&Airframe(json!({ "kind": "object", "accelSetupNeeded": true }), json!({ "kind": "object", "px4Firmware": true, "multiRotor": true })), &[]);
        assert_eq!(qt_mode["connected"], false, "with the core off sensorsCal is APMSensorsComponentController, whose commands mean something else to PX4");
        assert!(qt_mode["routines"].as_array().unwrap().iter().all(|r| r["enabled"] == false));
    }

    #[test]
    fn a_px4_vehicle_gets_the_px4_routines_in_qgc_order_with_their_status() {
        let view = calibration_view(&Fake(json!({ "kind": "object", "px4": true, "px4Firmware": true, "magEnabled": true, "accelSetupNeeded": false, "gyroSetupNeeded": true, "compassSetupNeeded": false, "airspeedSupported": false })), &[]);
        let routines = view["routines"].as_array().unwrap();
        let ids: Vec<&str> = routines.iter().map(|r| r["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["compass", "gyro", "accelerometer", "levelHorizon"], "airspeed only shows when the airframe has a sensor");
        assert_eq!(routines[0]["status"], "Calibrated");
        assert_eq!(routines[1]["status"], "Not calibrated");
        assert_eq!(routines[1]["title"], "Gyroscope");
        assert_eq!(routines[3]["blocked"], true);
        assert_eq!(routines[3]["description"], "Calibrate Accelerometer and Gyroscope first");
        assert_eq!(routines[2]["arguments"].as_array().unwrap().len(), 0, "PX4 has no simple accelerometer calibration");
        assert_eq!((view["connected"].clone(), routines[2]["enabled"].clone()), (json!(true), json!(true)), "the core's PX4 calibrator drives a PX4 vehicle");
        let plane = calibration_view(&Fake(json!({ "kind": "object", "px4": true, "magEnabled": false, "airspeedSupported": true, "airspeedSetupNeeded": true })), &[]);
        let ids: Vec<&str> = plane["routines"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["gyro", "accelerometer", "levelHorizon", "airspeed"], "SYS_HAS_MAG=0 hides the compass");
        assert_eq!(plane["routines"][3]["status"], "Not calibrated");
        assert_eq!(refusal(Action::Start("calibrateCompass"), Session { connected: true, ..Session::default() }, &json!({ "kind": "object", "px4": true, "magEnabled": false }), None).map(|(t, _)| t), Some("notForThisVehicle"));
        assert_eq!(refusal(Action::Start("calibratePressure"), Session { connected: true, ..Session::default() }, &json!({ "kind": "object", "px4": true }), None).map(|(t, _)| t), Some("notForThisVehicle"));
    }

    #[test]
    fn a_started_routine_carries_the_arguments_the_view_offered() {
        use std::cell::RefCell;
        struct Recording(RefCell<Vec<(String, String)>>);
        impl Backend for Recording {
            fn get(&self, _p: &str) -> String { json!({ "kind": "object" }).to_string() }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "object", "multiRotor": true }).to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, p: &str, a: &str) -> String {
                self.0.borrow_mut().push((p.to_string(), a.to_string()));
                json!({ "ok": true }).to_string()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let backend = Recording(RefCell::new(Vec::new()));
        assert_eq!(act(&backend, Action::Start("calibrateAccel"), "sensorsCal.calibrateAccel", "[]")["ok"], true);
        assert_eq!(act(&backend, Action::Start("calibrateAccel"), "sensorsCal.calibrateAccel", "[true]")["ok"], true);
        assert_eq!(act(&backend, Action::Next, "sensorsCal.nextClicked", "[]")["refusal"], "waiting");
        assert_eq!(
            backend.0.borrow().as_slice(),
            &[("sensorsCal.calibrateAccel".to_string(), "[false]".to_string()), ("sensorsCal.calibrateAccel".to_string(), "[true]".to_string())],
            "calibrateAccel(bool) has no default, so a head that sends nothing gets the full six-sided calibration the view describes rather than a failed invoke"
        );
    }
}
