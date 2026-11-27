use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::router::Backend;

pub const AUX_SWITCHES_VIEW: &str = "view.auxSwitches";
pub const AUX_SWITCH_SET: &str = "vehicle.auxSwitch";
pub const NOT_ARDUPILOT: &str = "Only ArduPilot switches sensors through its aux functions.";
const SENSOR_SLOTS: [&str; 11] = ["", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A"];
const REBOOT_SLACK_MS: u32 = 5_000;

pub struct Switch {
    pub id: &'static str,
    pub label: &'static str,
    pub function: u16,
    sensor: &'static str,
    off_means: &'static str,
}

pub const SWITCHES: [Switch; 2] = [
    Switch {
        id: "rangefinder",
        label: "range finder",
        function: 10,
        sensor: "RNGFND",
        off_means: "The aircraft stops reading its height above the ground from the range finder.",
    },
    Switch {
        id: "avoidance",
        label: "obstacle avoidance",
        function: 40,
        sensor: "PRX",
        off_means: "The aircraft no longer stops or steers away from what its proximity sensor sees.",
    },
];

#[derive(Debug, Default)]
pub struct Switched {
    asked: Option<(u16, bool)>,
    off: BTreeSet<u16>,
    uptime_ms: u32,
}

impl Switched {
    pub fn asked(&mut self, function: u16, high: bool) {
        self.asked.get_or_insert((function, high));
    }

    pub fn answered(&mut self, accepted: bool) {
        match self.asked.take().filter(|_| accepted) {
            Some((function, true)) => {
                self.off.remove(&function);
            }
            Some((function, false)) => {
                self.off.insert(function);
            }
            None => {}
        }
    }

    pub fn heard_uptime(&mut self, uptime_ms: u32) {
        if uptime_ms.saturating_add(REBOOT_SLACK_MS) < self.uptime_ms {
            *self = Self::default();
        }
        self.uptime_ms = uptime_ms;
    }

    pub fn on(&self, function: u16) -> bool {
        !self.off.contains(&function)
    }
}

pub fn uptime_ms(message: &mavlink::dialects::ardupilotmega::MavMessage) -> Option<u32> {
    use mavlink::dialects::ardupilotmega::MavMessage;
    match message {
        MavMessage::SYSTEM_TIME(time) => Some(time.time_boot_ms),
        MavMessage::ATTITUDE(attitude) => Some(attitude.time_boot_ms),
        MavMessage::GLOBAL_POSITION_INT(position) => Some(position.time_boot_ms),
        _ => None,
    }
}

pub fn fitted(ardupilot: bool, sensor_type: impl Fn(&str) -> Option<f64>, switch: &Switch) -> bool {
    ardupilot
        && SENSOR_SLOTS.iter().any(|slot| {
            sensor_type(&format!("{}{slot}_TYPE", switch.sensor)).is_some_and(|kind| kind != 0.0)
        })
}

pub fn offered(
    ardupilot: bool,
    armed: bool,
    sensor_type: impl Fn(&str) -> Option<f64> + Copy,
    switched: &Switched,
) -> Vec<Value> {
    SWITCHES
        .iter()
        .filter(|switch| fitted(ardupilot, sensor_type, switch))
        .map(|switch| {
            let on = switched.on(switch.function);
            let action = format!("Turn {} {}", switch.label, if on { "off" } else { "on" });
            let confirm =
                (on && armed).then(|| json!({ "title": action, "text": switch.off_means }));
            json!({ "id": switch.id, "on": on, "action": action, "confirm": confirm })
        })
        .collect()
}

pub fn view(_backend: &dyn Backend, _args: &[String]) -> Value {
    let hub = crate::hub::lock();
    let switches = hub
        .active()
        .map(|vehicle| {
            offered(
                vehicle.autopilot == crate::modes::AUTOPILOT_ARDUPILOT,
                vehicle.armed(),
                |name: &str| {
                    vehicle
                        .parameter(vehicle.component, name)
                        .map(|p| p.as_f64())
                },
                &vehicle.aux_switches,
            )
        })
        .unwrap_or_default();
    json!({ "switches": switches })
}

pub fn set(backend: &dyn Backend, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let switch = given
        .get(0)
        .and_then(Value::as_str)
        .and_then(|id| SWITCHES.iter().find(|switch| switch.id == id));
    let (Some(switch), Some(on)) = (switch, given.get(1).and_then(Value::as_bool)) else {
        return json!({ "ok": false, "refusal": "badSwitch", "reason": "A switch is the range finder or obstacle avoidance, turned on or off." });
    };
    crate::guided::dispatch(
        backend,
        Some(json!({ "action": "auxSwitch", "function": switch.function, "high": on })),
        crate::guided::active_id(backend),
        AUX_SWITCH_SET,
        args,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_switch_shows_on_ardupilot_only_when_one_of_its_sensors_is_set_up() {
        let [rangefinder, avoidance] = &SWITCHES;
        let owel = |name: &str| match name {
            "RNGFND1_TYPE" => Some(14.0),
            "PRX1_TYPE" | "PRX2_TYPE" | "RNGFND2_TYPE" => Some(0.0),
            _ => None,
        };
        assert!(
            fitted(true, owel, rangefinder),
            "owel-red-01's TeraRanger is RNGFND1_TYPE 14"
        );
        assert!(
            !fitted(true, owel, avoidance),
            "its Lidar360 port has no PRX type set, so there is nothing to switch"
        );
        assert!(
            !fitted(false, owel, rangefinder),
            "PX4 has no ArduPilot aux functions"
        );
        assert!(fitted(
            true,
            |name| (name == "PRX3_TYPE").then_some(5.0),
            avoidance
        ));
        assert!(
            fitted(
                true,
                |name| (name == "RNGFNDA_TYPE").then_some(8.0),
                rangefinder
            ),
            "the tenth range finder is slot A"
        );
        assert!(
            fitted(true, |name| (name == "PRX_TYPE").then_some(4.0), avoidance),
            "firmware before 4.3 has one unnumbered proximity sensor"
        );
    }

    #[test]
    fn the_switches_are_the_aux_functions_ardupilot_documents() {
        let functions: Vec<(&str, u16)> = SWITCHES
            .iter()
            .map(|switch| (switch.id, switch.function))
            .collect();
        assert_eq!(
            functions,
            [("rangefinder", 10), ("avoidance", 40)],
            "RC_Channel.h: RANGEFINDER = 10, AVOID_PROXIMITY = 40 (enable object avoidance using proximity sensors)"
        );
    }

    #[test]
    fn a_switch_reads_off_only_once_the_vehicle_accepted_it() {
        let mut switched = Switched::default();
        switched.asked(10, false);
        assert!(switched.on(10), "queued is not switched");
        switched.answered(false);
        assert!(
            switched.on(10),
            "a refused command leaves the sensor as it was"
        );
        switched.answered(true);
        assert!(
            switched.on(10),
            "an answer to nothing asked changes nothing"
        );
        switched.asked(10, false);
        switched.answered(true);
        assert!(!switched.on(10));
        assert!(switched.on(40));
        switched.asked(10, true);
        switched.answered(true);
        assert!(switched.on(10));
    }

    #[test]
    fn a_second_ask_does_not_replace_the_one_the_vehicle_is_answering() {
        let mut switched = Switched::default();
        switched.asked(10, false);
        switched.asked(40, false);
        switched.answered(true);
        assert!(!switched.on(10));
        assert!(switched.on(40));
    }

    #[test]
    fn a_flight_controller_that_rebooted_has_its_sensors_on_again() {
        let mut switched = Switched::default();
        switched.heard_uptime(600_000);
        switched.asked(10, false);
        switched.answered(true);
        switched.heard_uptime(599_000);
        assert!(
            !switched.on(10),
            "a late frame from another link is not a reboot"
        );
        switched.heard_uptime(601_000);
        switched.heard_uptime(4_000);
        assert!(switched.on(10));
    }

    #[test]
    fn the_view_names_the_action_and_asks_before_turning_a_sensor_off_in_flight() {
        let both = |name: &str| matches!(name, "RNGFND1_TYPE" | "PRX1_TYPE").then_some(1.0);
        let fresh = Switched::default();
        let ground = offered(true, false, both, &fresh);
        assert_eq!(
            ground,
            [
                json!({ "id": "rangefinder", "on": true, "action": "Turn range finder off", "confirm": null }),
                json!({ "id": "avoidance", "on": true, "action": "Turn obstacle avoidance off", "confirm": null }),
            ]
        );
        let flying = offered(true, true, both, &fresh);
        assert_eq!(flying[0]["confirm"]["title"], "Turn range finder off");
        assert_eq!(
            flying[1]["confirm"]["text"],
            "The aircraft no longer stops or steers away from what its proximity sensor sees."
        );
        let mut off = Switched::default();
        off.asked(40, false);
        off.answered(true);
        let back = offered(true, true, both, &off);
        assert_eq!(back[1]["on"], false);
        assert_eq!(back[1]["action"], "Turn obstacle avoidance on");
        assert_eq!(
            back[1]["confirm"],
            Value::Null,
            "turning a sensor back on is never asked about"
        );
        assert!(offered(false, true, both, &fresh).is_empty());
    }

    struct Sent(std::cell::RefCell<Vec<Value>>, Result<(), String>);

    impl Backend for Sent {
        fn get(&self, _path: &str) -> String {
            String::new()
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "vehicle" => json!({ "kind": "object", "id": 7 }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn set(&self, _path: &str, _value: &str) -> String {
            String::new()
        }
        fn invoke(&self, _path: &str, _args: &str) -> String {
            String::new()
        }
        fn watch(&self, _paths: &[String]) {}
        fn core_guided(&self, action: &Value) -> Option<Result<(), String>> {
            self.0.borrow_mut().push(action.clone());
            Some(self.1.clone())
        }
    }

    #[test]
    fn setting_a_switch_sends_its_aux_function_high_or_low_to_the_active_vehicle() {
        let backend = Sent(Default::default(), Ok(()));
        assert_eq!(set(&backend, r#"["rangefinder", false]"#)["ok"], true);
        assert_eq!(set(&backend, r#"["avoidance", true]"#)["ok"], true);
        assert_eq!(
            *backend.0.borrow(),
            [
                json!({ "action": "auxSwitch", "function": 10, "high": false, "vehicle": 7 }),
                json!({ "action": "auxSwitch", "function": 40, "high": true, "vehicle": 7 }),
            ]
        );
    }

    #[test]
    fn a_switch_that_is_not_one_or_has_no_state_is_refused_and_nothing_is_sent() {
        let backend = Sent(Default::default(), Ok(()));
        let refused = [
            r#"["lidar", false]"#,
            r#"["rangefinder"]"#,
            r#"["rangefinder", "off"]"#,
            "",
            "{}",
        ]
        .map(|args| set(&backend, args));
        assert!(
            refused
                .iter()
                .all(|answer| answer["ok"] == false && answer["refusal"] == "badSwitch")
        );
        assert!(backend.0.borrow().is_empty());
    }

    #[test]
    fn a_vehicle_that_refuses_the_switch_says_why() {
        let backend = Sent(Default::default(), Err(NOT_ARDUPILOT.to_string()));
        let answer = set(&backend, r#"["rangefinder", false]"#);
        assert_eq!(answer["ok"], false);
        assert_eq!(answer["reason"], NOT_ARDUPILOT);
    }
}
