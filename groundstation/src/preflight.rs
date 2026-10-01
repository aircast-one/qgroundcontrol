use serde::Serialize;
use serde_json::{Value, json};

use crate::read::{flag, integer, object, value_number};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.vehicleLinkManager.communicationLost",
    "vehicle.multiRotor",
    "vehicle.vtol",
    "vehicle.rover",
    "vehicle.sub",
    "vehicle.fixedWing",
    "vehicle.sensorsUnhealthyBits",
    "vehicle.gps.lock",
    "vehicle.gps.count",
    "vehicle.batteries.0.percentRemaining",
    "settings.appSettings.audioMuted",
    "settings.appSettings.useChecklist",
];

const FAILURE_SATELLITES: i64 = 9;
const FAILURE_PERCENT: f64 = 40.0;
const SENSOR_MASK: i64 = 1 | 2 | 4 | 8 | 16 | 32 | 2097152;
const SENSOR_FAILURES: &[(i64, &str)] = &[
    (4, "Failure. Magnetometer issues. Check console."),
    (2, "Failure. Accelerometer issues. Check console."),
    (1, "Failure. Gyroscope issues. Check console."),
    (8, "Failure. Barometer issues. Check console."),
    (16, "Failure. Airspeed sensor issues. Check console."),
    (2097152, "Failure. AHRS issues. Check console."),
    (32, "Failure. GPS issues. Check console."),
];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Airframe {
    MultiRotor,
    Vtol,
    Rover,
    Sub,
    FixedWing,
    Generic,
}

impl Airframe {
    pub fn of(multi_rotor: bool, vtol: bool, rover: bool, sub: bool, fixed_wing: bool) -> Airframe {
        match (multi_rotor, vtol, rover, sub, fixed_wing) {
            (true, ..) => Airframe::MultiRotor,
            (_, true, ..) => Airframe::Vtol,
            (_, _, true, ..) => Airframe::Rover,
            (_, _, _, true, _) => Airframe::Sub,
            (_, _, _, _, true) => Airframe::FixedWing,
            _ => Airframe::Generic,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Airframe::MultiRotor => "Multirotor",
            Airframe::Vtol => "VTOL",
            Airframe::Rover => "Rover",
            Airframe::Sub => "Submarine",
            Airframe::FixedWing => "Fixed wing",
            Airframe::Generic => "Generic",
        }
    }

    fn hardware_prompt(self) -> &'static str {
        match self {
            Airframe::MultiRotor => "Props mounted and secured?",
            Airframe::Rover => "Battery mounted and secured?",
            Airframe::Sub => "All seals in place?",
            Airframe::Vtol | Airframe::FixedWing | Airframe::Generic => "Props mounted? Wings secured? Tail secured?",
        }
    }

    fn initial_checks(self) -> &'static str {
        match self {
            Airframe::MultiRotor => "Multirotor Initial Checks",
            Airframe::Vtol => "VTOL Initial Checks",
            Airframe::Rover => "Rover Initial Checks",
            Airframe::Sub => "Submarine Initial checks",
            Airframe::FixedWing => "Fixed Wing Initial Checks",
            Airframe::Generic => "Generic Initial checks",
        }
    }

    fn checks_actuators(self) -> bool {
        !matches!(self, Airframe::MultiRotor | Airframe::Rover)
    }

    fn checks_motors(self) -> bool {
        self != Airframe::Rover
    }

    fn wind_prompt(self) -> Option<&'static str> {
        match self {
            Airframe::Sub => None,
            Airframe::MultiRotor | Airframe::Rover => Some("OK for your platform?"),
            Airframe::Vtol | Airframe::FixedWing | Airframe::Generic => Some("OK for your platform? Launching into the wind?"),
        }
    }

    fn area(self) -> Option<(&'static str, &'static str)> {
        match self {
            Airframe::Sub => None,
            Airframe::Rover => Some(("Mission area", "Mission area and path free of obstacles/people?")),
            _ => Some(("Flight area", "Launch area and path free of obstacles/people?")),
        }
    }
}

#[derive(Serialize, PartialEq, Debug)]
pub struct Check {
    pub name: &'static str,
    pub prompt: &'static str,
    pub verdict: &'static str,
    pub reason: String,
    pub blocked: bool,
}

impl Check {
    fn manual(name: &'static str, prompt: &'static str) -> Check {
        Check { name, prompt, verdict: "manual", reason: prompt.to_string(), blocked: false }
    }
    fn passing(name: &'static str, prompt: &'static str) -> Check {
        Check { name, prompt, verdict: "passing", reason: prompt.to_string(), blocked: false }
    }
    fn failing(name: &'static str, prompt: &'static str, reason: String) -> Check {
        Check { name, prompt, verdict: "failing", reason, blocked: true }
    }
    fn overridable(name: &'static str, prompt: &'static str, reason: String) -> Check {
        Check { name, prompt, verdict: "overridable", reason, blocked: false }
    }
}

#[derive(Serialize, PartialEq, Debug)]
pub struct Group {
    pub name: &'static str,
    pub checks: Vec<Check>,
}

pub struct Inputs {
    pub airframe: Airframe,
    pub contact_lost: bool,
    pub lock: Option<i64>,
    pub satellites: Option<i64>,
    pub battery_percent: Option<f64>,
    pub unhealthy_bits: Option<i64>,
    pub audio_muted: bool,
}

pub fn gps(lock: Option<i64>, satellites: Option<i64>) -> Check {
    const PROMPT: &str = "3D lock and enough satellites.";
    match (lock, satellites.unwrap_or(0)) {
        (None, _) => Check::failing("GPS", PROMPT, "No vehicle is reporting a GPS.".to_string()),
        (Some(l), _) if l < 3 => Check::failing("GPS", PROMPT, "Waiting for 3D lock.".to_string()),
        (Some(_), n) if n <= FAILURE_SATELLITES => Check::overridable("GPS", PROMPT, format!("Warning - Sat count below {}.", FAILURE_SATELLITES + 1)),
        _ => Check::passing("GPS", PROMPT),
    }
}

pub fn battery(percent: Option<f64>) -> Check {
    const PROMPT: &str = "Battery connector firmly plugged?";
    match percent {
        None => Check::failing("Battery", PROMPT, "No vehicle is reporting a battery.".to_string()),
        Some(p) if p < FAILURE_PERCENT => Check::failing("Battery", PROMPT, format!("Battery charge below {FAILURE_PERCENT:.0}%. Please recharge.")),
        Some(_) => Check::passing("Battery", PROMPT),
    }
}

pub fn sensors(unhealthy_bits: Option<i64>) -> Check {
    const PROMPT: &str = "Every sensor the autopilot needs is healthy.";
    match unhealthy_bits.map(|b| b & SENSOR_MASK) {
        None => Check::failing("Sensors", PROMPT, "No vehicle is reporting sensor health.".to_string()),
        Some(0) => Check::passing("Sensors", PROMPT),
        Some(bits) => Check::failing("Sensors", PROMPT, SENSOR_FAILURES.iter().find(|(bit, _)| bits & bit != 0).map_or("Failure. Check console.", |(_, text)| text).to_string()),
    }
}

pub fn contact(lost: bool) -> Option<Check> {
    const PROMPT: &str = "The vehicle is answering.";
    lost.then(|| {
        Check::failing(
            "Contact",
            PROMPT,
            "No heartbeat from the vehicle. Every reading below is the last one it sent and does not describe the aircraft now.".to_string(),
        )
    })
}

fn sound(muted: bool) -> Check {
    const PROMPT: &str = "QGC audio output enabled. System audio output enabled, too?";
    match muted {
        true => Check::failing("Sound output", PROMPT, "QGC audio output is disabled. Please enable it under application settings->general to hear audio warnings!".to_string()),
        false => Check::passing("Sound output", PROMPT),
    }
}

pub fn groups(inputs: &Inputs) -> Vec<Group> {
    let airframe = inputs.airframe;
    vec![
        Group {
            name: airframe.initial_checks(),
            checks: [
                contact(inputs.contact_lost),
                Some(Check::manual("Hardware", airframe.hardware_prompt())),
                Some(battery(inputs.battery_percent)),
                Some(sensors(inputs.unhealthy_bits)),
                Some(gps(inputs.lock, inputs.satellites)),
                Some(Check::manual("Radio Control", "Receiving signal. Perform range test & confirm.")),
            ]
            .into_iter()
            .flatten()
            .collect(),
        },
        Group {
            name: "Please arm the vehicle here",
            checks: [
                airframe.checks_actuators().then(|| Check::manual("Actuators", "Move all control surfaces. Did they work properly?")),
                airframe.checks_motors().then(|| Check::manual("Motors", "Propellers free? Then throttle up gently. Working properly?")),
                Some(Check::manual("Mission", "Please confirm mission is valid (waypoints valid, no terrain collision).")),
                Some(sound(inputs.audio_muted)),
            ]
            .into_iter()
            .flatten()
            .collect(),
        },
        Group {
            name: "Last preparations before launch",
            checks: [
                Some(Check::manual("Payload", "Configured and started? Payload lid closed?")),
                airframe.wind_prompt().map(|p| Check::manual("Wind & weather", p)),
                airframe.area().map(|(name, prompt)| Check::manual(name, prompt)),
            ]
            .into_iter()
            .flatten()
            .collect(),
        },
    ]
}

pub fn preflight_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let inputs = read_inputs(backend);
    let groups = groups(&inputs);
    let total: usize = groups.iter().map(|g| g.checks.len()).sum();
    let blocked: Vec<&str> = groups.iter().flat_map(|g| g.checks.iter()).filter(|c| c.blocked).map(|c| c.name).collect();
    json!({
        "kind": "object",
        "class": "Preflight",
        "airframe": inputs.airframe.name(),
        "offered": offered(backend),
        "total": total,
        "blocked": blocked,
        "groups": groups,
    })
}

fn offered(backend: &dyn Backend) -> bool {
    crate::read::fact_flag(&object(&backend.get_fields("settings.appSettings", "useChecklist")), "useChecklist")
}

fn read_inputs(backend: &dyn Backend) -> Inputs {
    let vehicle = object(&backend.get_fields("vehicle", "multiRotor,vtol,rover,sub,fixedWing,sensorsUnhealthyBits"));
    let gps_fact = |name: &str| value_number(&backend.get(&format!("vehicle.gps.{name}")));
    Inputs {
        contact_lost: flag(&object(&backend.get("vehicle.vehicleLinkManager.communicationLost")), "value"),
        airframe: Airframe::of(flag(&vehicle, "multiRotor"), flag(&vehicle, "vtol"), flag(&vehicle, "rover"), flag(&vehicle, "sub"), flag(&vehicle, "fixedWing")),
        lock: gps_fact("lock").map(|v| v as i64),
        satellites: gps_fact("count").map(|v| v as i64),
        battery_percent: value_number(&backend.get("vehicle.batteries.0.percentRemaining")),
        unhealthy_bits: integer(&vehicle, "sensorsUnhealthyBits"),
        audio_muted: value_number(&backend.get("settings.appSettings.audioMuted.rawValue")).map(|v| v != 0.0).unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gps_wants_a_3d_lock_and_more_than_nine_satellites() {
        assert_eq!(gps(None, None).verdict, "failing");
        assert_eq!(gps(Some(2), Some(12)).reason, "Waiting for 3D lock.");
        let few = gps(Some(3), Some(1));
        assert_eq!(few.verdict, "overridable");
        assert_eq!(few.reason, "Warning - Sat count below 10.");
        assert!(!few.blocked);
        assert_eq!(gps(Some(3), Some(9)).verdict, "overridable", "PreFlightGPSCheck fails a count at or below failureSatCount");
        assert_eq!(gps(Some(3), Some(10)).verdict, "passing");
    }

    #[test]
    fn battery_and_sensors_name_what_is_wrong() {
        assert_eq!(battery(Some(35.0)).reason, "Battery charge below 40%. Please recharge.");
        assert_eq!(battery(Some(40.0)).verdict, "passing");
        assert_eq!(sensors(Some(4 | 32)).reason, "Failure. Magnetometer issues. Check console.", "the first failure in PreFlightSensorsHealthCheck's order");
        assert_eq!(sensors(Some(1 << 10)).verdict, "passing");
        assert_eq!(sensors(None).verdict, "failing");
    }

    #[test]
    fn the_groups_follow_the_airframe() {
        let base = Inputs { airframe: Airframe::MultiRotor, contact_lost: false, lock: Some(3), satellites: Some(12), battery_percent: Some(90.0), unhealthy_bits: Some(0), audio_muted: false };
        let copter = groups(&base);
        assert_eq!(copter.len(), 3);
        assert_eq!(copter[0].checks[0].prompt, "Props mounted and secured?");
        assert!(copter[1].checks.iter().all(|c| c.name != "Actuators"));
        let rover = groups(&Inputs { airframe: Airframe::Rover, ..base });
        assert!(rover[1].checks.iter().all(|c| c.name != "Motors"));
        assert_eq!(rover[2].checks.last().unwrap().name, "Mission area");
        let sub = groups(&Inputs { airframe: Airframe::Sub, ..base });
        assert_eq!(sub[2].checks.len(), 1);
    }

    #[test]
    fn a_silent_vehicle_says_so_before_the_readings_it_makes_stale() {
        let talking = Inputs { airframe: Airframe::MultiRotor, contact_lost: false, lock: Some(3), satellites: Some(12), battery_percent: Some(90.0), unhealthy_bits: Some(0), audio_muted: false };
        assert!(groups(&talking)[0].checks.iter().all(|c| c.name != "Contact"), "a link that is up is the precondition for the list, not a line in it - a passing Contact row would be one more thing to read on every flight that is fine");

        let silent = groups(&Inputs { contact_lost: true, ..talking });
        assert_eq!(silent[0].checks[0].name, "Contact", "it has to come first: every check under it is reading a figure the vehicle sent before it went quiet");
        assert!(silent[0].checks[0].blocked);
        assert_eq!(silent[0].checks.iter().filter(|c| c.name == "Contact").count(), 1);
        assert_eq!(
            silent[0].checks[1].name, "Hardware",
            "the rest of the group keeps its order and its content; the vehicle going quiet adds a check rather than replacing the ones whose inputs are now stale"
        );
        assert_eq!(silent[0].checks[3].verdict, "passing", "battery still reads 90% because Vehicle latches its last value, which is exactly why Contact has to be the thing that says the number is old");
    }

    #[test]
    fn a_muted_app_blocks_the_sound_check() {
        let muted = Inputs { airframe: Airframe::Generic, contact_lost: false, lock: Some(3), satellites: Some(12), battery_percent: Some(90.0), unhealthy_bits: Some(0), audio_muted: true };
        let sound = groups(&muted)[1].checks.iter().find(|c| c.name == "Sound output").unwrap().blocked;
        assert!(sound);
    }

    #[test]
    fn the_view_reads_its_facts_one_by_one() {
        struct Fake;
        impl Backend for Fake {
            fn get(&self, path: &str) -> String {
                match path {
                    "vehicle.gps.lock" => json!({ "kind": "fact", "name": "lock", "value": 3 }),
                    "vehicle.gps.count" => json!({ "kind": "fact", "name": "count", "value": 12 }),
                    "vehicle.batteries.0.percentRemaining" => json!({ "kind": "fact", "name": "percentRemaining", "value": 35.0 }),
                    _ => json!({ "kind": "value", "value": null }),
                }
                .to_string()
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "object", "multiRotor": true, "sensorsUnhealthyBits": 0 }).to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let inputs = read_inputs(&Fake);
        assert_eq!((inputs.lock, inputs.satellites, inputs.battery_percent, inputs.airframe), (Some(3), Some(12), Some(35.0), Airframe::MultiRotor));
        struct Bare;
        impl Backend for Bare {
            fn get(&self, _p: &str) -> String { json!({ "kind": "value", "value": null }).to_string() }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "null" }).to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let bare = read_inputs(&Bare);
        assert_eq!((bare.lock, bare.satellites, bare.battery_percent), (None, None, None));
    }

    #[test]
    fn the_checks_are_offered_when_the_operator_asked_for_them_and_not_when_a_head_cannot_draw_them() {
        struct Gate(bool);
        impl Backend for Gate {
            fn get(&self, _p: &str) -> String { json!({ "kind": "value", "value": null }).to_string() }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "settings.appSettings" => json!({ "kind": "object", "facts": [{ "name": "useChecklist", "value": self.0 }] }).to_string(),
                    _ => json!({ "kind": "object", "multiRotor": true }).to_string(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        assert_eq!(preflight_view(&Gate(true), &[])["offered"], true);
        assert_eq!(preflight_view(&Gate(false), &[])["offered"], false, "an operator who turned the checklist off in Settings is still being offered it on every head that reads only the vehicle state");

        let served = include_str!("preflight.rs").split("#[cfg(test)]").next().unwrap_or("");
        assert!(!served.contains("preFlightChecklistUrl"), "QGC's own gate also requires corePlugin.options.preFlightChecklistUrl to be non-empty, and that is a statement about whether the QT head ships a checklist QML - a native head draws its own and can never load that qrc. Serving it here would let a build where one head cannot draw the checks disable a preference the operator never got to withdraw, on the same build where FlyViewSettings.qml hides the control they would withdraw it with");
    }
}
