use serde_json::{Value, json};

use crate::altitude::merge;
use crate::read::{Unit, flag, result_number};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.vtolInFwdFlight",
    "vehicle.fixedWing",
    "vehicle.airSpeed",
    "vehicle.groundSpeed",
    "settings.unitsSettings.speedUnits",
];

const SLOWEST_GROUND_SPEED_METERS_SECOND: f64 = 0.1;
const MOVING_METERS_SECOND: f64 = 0.5;
const WAYPOINT_SPEED_PARAMETERS: &[&str] = &["MPC_XY_CRUISE", "WP_SPD", "WPNAV_SPEED"];

struct Range {
    label: &'static str,
    command: &'static str,
    minimum: f64,
    maximum: f64,
    initial: f64,
}

pub fn speed_view(backend: &dyn Backend, args: &[String]) -> Value {
    let unit = Unit::speed(backend);
    let range = range_meters_second(backend);
    let target = args
        .first()
        .and_then(|a| a.parse::<f64>().ok())
        .filter(|t| t.is_finite());
    let base = json!({
        "kind": "object",
        "class": "GuidedSpeed",
        "available": range.is_some(),
        "unit": unit.name,
        "label": range.as_ref().map(|r| r.label),
        "command": range.as_ref().map(|r| r.command),
        "minimum": range.as_ref().map(|r| unit.show(r.minimum)),
        "maximum": range.as_ref().map(|r| unit.show(r.maximum)),
        "initial": range.as_ref().map(|r| unit.show(r.initial)),
    });
    match (range, target) {
        (Some(range), Some(target)) => {
            let target_meters_second = unit.meters(target).clamp(range.minimum, range.maximum);
            merge(
                base,
                json!({
                    "target": unit.show(target_meters_second),
                    "targetMetersSecond": target_meters_second,
                    "sentence": format!("The aircraft will fly at {}.", unit.label(target_meters_second)),
                }),
            )
        }
        _ => base,
    }
}

fn speed_refusal(
    range: Option<&Range>,
    command: &str,
    asked: Option<f64>,
) -> Option<(&'static str, String)> {
    let Some(range) = range else {
        return Some((
            "unavailable",
            "This vehicle does not report a speed range.".to_string(),
        ));
    };
    if command != range.command {
        return Some((
            "wrongCommand",
            format!(
                "This vehicle is flying on {}; change it with {}.",
                range.label.to_lowercase(),
                range.command
            ),
        ));
    }
    match asked {
        Some(v) if v.is_finite() && (range.minimum..=range.maximum).contains(&v) => None,
        _ => Some((
            "outOfRange",
            format!(
                "{} runs from {:.1} to {:.1} m/s.",
                range.label, range.minimum, range.maximum
            ),
        )),
    }
}

pub fn change(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let command = path.strip_prefix("vehicle.").unwrap_or(path);
    let asked = serde_json::from_str::<Value>(args)
        .ok()
        .and_then(|a| a.get(0)?.as_f64());
    let refusal = crate::guided::offer_refusal(backend, &[crate::guided::Action::ChangeSpeed])
        .or_else(|| speed_refusal(range_meters_second(backend).as_ref(), command, asked));
    if let Some((token, reason)) = refusal {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let core = json!({ "action": "speed", "ground": command == "guidedModeChangeGroundSpeedMetersSecond", "metresPerSecond": asked });
    let answer = crate::guided::dispatch(
        backend,
        Some(core),
        crate::guided::active_id(backend),
        path,
        &json!([asked]).to_string(),
    );
    let dispatched = flag(&answer, "ok");
    json!({ "ok": dispatched, "refusal": Value::Null, "reason": match dispatched { true => Value::Null, false => answer.get("reason").cloned().unwrap_or_else(|| json!("The vehicle was not sent the command.")) } })
}

fn range_meters_second(backend: &dyn Backend) -> Option<Range> {
    let vehicle = backend.value_fields("vehicle", "vtolInFwdFlight,fixedWing");
    if vehicle.get("kind") != Some(&Value::String("object".into())) {
        return None;
    }
    let forward_flight = flag(&vehicle, "vtolInFwdFlight") || flag(&vehicle, "fixedWing");
    let number = |path: &str| result_number(&backend.invoke(path, "[]"));
    let range = match forward_flight {
        true => {
            let minimum = number("vehicle.minimumEquivalentAirspeed")?;
            let maximum = number("vehicle.maximumEquivalentAirspeed")?;
            let flying_at = crate::read::value_number(&backend.value("vehicle.airSpeed.rawValue"))
                .filter(|v| v.is_finite());
            Range {
                label: "Airspeed",
                command: "guidedModeChangeEquivalentAirspeedMetersSecond",
                minimum,
                maximum,
                initial: flying_at.unwrap_or((minimum + maximum) / 2.0),
            }
        }
        false => {
            let maximum = number("vehicle.maximumHorizontalSpeedMultirotorMetersSecond")?;
            let moving = backend
                .raw("vehicle.groundSpeed")
                .filter(|v| *v >= MOVING_METERS_SECOND);
            let planned = crate::battery::parameter_si(backend, WAYPOINT_SPEED_PARAMETERS)
                .filter(|v| v.is_finite() && *v > 0.0);
            Range {
                label: "Speed",
                command: "guidedModeChangeGroundSpeedMetersSecond",
                minimum: SLOWEST_GROUND_SPEED_METERS_SECOND,
                maximum,
                initial: moving.or(planned).map_or(maximum / 2.0, |v| {
                    v.clamp(SLOWEST_GROUND_SPEED_METERS_SECOND, maximum)
                }),
            }
        }
    };
    (range.maximum > range.minimum).then_some(range)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        connected: bool,
        forward: bool,
        maximum: f64,
    }

    impl Backend for Fake {
        fn get(&self, _p: &str) -> String {
            String::new()
        }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match (path, self.connected) {
                ("vehicle", true) => {
                    json!({ "kind": "object", "vtolInFwdFlight": self.forward, "fixedWing": false })
                }
                ("vehicle", false) => json!({ "kind": "null" }),
                _ => json!({ "kind": "object", "appSettingsSpeedUnitsString": "m/s" }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, path: &str, _a: &str) -> String {
            let result = match path {
                "vehicle.maximumHorizontalSpeedMultirotorMetersSecond" => self.maximum,
                "vehicle.minimumEquivalentAirspeed" => 8.0,
                "vehicle.maximumEquivalentAirspeed" => self.maximum,
                _ => 1.0,
            };
            json!({ "ok": true, "result": result }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_multirotor_gets_ground_speed_from_a_tenth_to_its_maximum() {
        let view = speed_view(
            &Fake {
                connected: true,
                forward: false,
                maximum: 15.0,
            },
            &["20".to_string()],
        );
        assert_eq!(view["label"], "Speed");
        assert_eq!(view["command"], "guidedModeChangeGroundSpeedMetersSecond");
        assert_eq!(view["minimum"], 0.1);
        assert_eq!(view["initial"], 7.5);
        assert_eq!(view["targetMetersSecond"], 15.0);
        assert_eq!(view["sentence"], "The aircraft will fly at 15.0 m/s.");
    }

    struct Flying {
        ground_speed: Option<f64>,
        waypoint_speed: Option<(&'static str, f64, &'static str)>,
    }

    impl Backend for Flying {
        fn get(&self, path: &str) -> String {
            let parameter = self.waypoint_speed.filter(|(name, ..)| {
                path == format!("vehicle.parameterManager.getParameter(-1,{name})")
            });
            match (path, self.ground_speed, parameter) {
                ("vehicle.groundSpeed", Some(v), _) => {
                    json!({ "kind": "fact", "rawValue": v })
                }
                (_, _, Some((name, value, units))) => {
                    json!({ "kind": "fact", "name": name, "rawValue": value, "rawUnits": units })
                }
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, path: &str, fields: &str) -> String {
            Fake {
                connected: true,
                forward: false,
                maximum: 15.0,
            }
            .get_fields(path, fields)
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, path: &str, args: &str) -> String {
            Fake {
                connected: true,
                forward: false,
                maximum: 15.0,
            }
            .invoke(path, args)
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_multirotor_speed_change_opens_at_the_speed_it_flies() {
        let initial = |ground_speed, waypoint_speed| {
            speed_view(
                &Flying {
                    ground_speed,
                    waypoint_speed,
                },
                &[],
            )["initial"]
                .clone()
        };
        assert_eq!(
            initial(Some(6.2), Some(("WPNAV_SPEED", 1000.0, "cm/s"))),
            6.2,
            "a vehicle already moving changes from the speed it is flying"
        );
        assert_eq!(
            initial(Some(0.0), Some(("WPNAV_SPEED", 1000.0, "cm/s"))),
            10.0,
            "a hovering copter opens at the waypoint speed its missions fly, not at half the maximum"
        );
        assert_eq!(
            initial(None, Some(("MPC_XY_CRUISE", 30.0, "m/s"))),
            15.0,
            "a cruise speed past the guided maximum opens at the maximum"
        );
        assert_eq!(initial(None, None), 7.5);
    }

    #[test]
    fn forward_flight_gets_airspeed_between_the_equivalent_limits() {
        let view = speed_view(
            &Fake {
                connected: true,
                forward: true,
                maximum: 24.0,
            },
            &[],
        );
        assert_eq!(view["label"], "Airspeed");
        assert_eq!(
            view["command"],
            "guidedModeChangeEquivalentAirspeedMetersSecond"
        );
        assert_eq!(view["minimum"], 8.0);
        assert_eq!(view["initial"], 16.0);
        assert!(view.get("sentence").is_none());
    }

    #[test]
    fn an_empty_or_absent_range_is_unavailable() {
        assert_eq!(
            speed_view(
                &Fake {
                    connected: true,
                    forward: false,
                    maximum: 0.0
                },
                &[]
            )["available"],
            false
        );
        assert_eq!(
            speed_view(
                &Fake {
                    connected: false,
                    forward: false,
                    maximum: 15.0
                },
                &[]
            )["available"],
            false
        );
    }

    #[test]
    fn a_speed_change_is_sent_only_on_the_command_the_frame_flies_and_inside_its_range() {
        let ground = Range {
            label: "Speed",
            command: "guidedModeChangeGroundSpeedMetersSecond",
            minimum: 0.1,
            maximum: 12.0,
            initial: 6.0,
        };
        let air = Range {
            label: "Airspeed",
            command: "guidedModeChangeEquivalentAirspeedMetersSecond",
            minimum: 14.0,
            maximum: 30.0,
            initial: 22.0,
        };
        assert_eq!(
            speed_refusal(
                Some(&ground),
                "guidedModeChangeGroundSpeedMetersSecond",
                Some(8.0)
            ),
            None
        );
        assert_eq!(
            speed_refusal(
                Some(&ground),
                "guidedModeChangeGroundSpeedMetersSecond",
                Some(40.0)
            )
            .map(|r| r.1),
            Some("Speed runs from 0.1 to 12.0 m/s.".to_string()),
            "QGC forwards a speed past the vehicle's own maximum"
        );
        assert_eq!(
            speed_refusal(
                Some(&air),
                "guidedModeChangeGroundSpeedMetersSecond",
                Some(20.0)
            )
            .map(|r| r.0),
            Some("wrongCommand"),
            "a VTOL that transitioned after the view was read flies on airspeed now"
        );
        assert_eq!(
            speed_refusal(
                Some(&air),
                "guidedModeChangeEquivalentAirspeedMetersSecond",
                Some(10.0)
            )
            .map(|r| r.0),
            Some("outOfRange"),
            "below stall is refused, not clamped"
        );
        assert_eq!(
            speed_refusal(
                Some(&air),
                "guidedModeChangeEquivalentAirspeedMetersSecond",
                None
            )
            .map(|r| r.0),
            Some("outOfRange")
        );
        assert_eq!(
            speed_refusal(None, "guidedModeChangeGroundSpeedMetersSecond", Some(5.0)).map(|r| r.0),
            Some("unavailable")
        );
    }
}
