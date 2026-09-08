use serde_json::{Value, json};

use crate::altitude::merge;
use crate::read::{Unit, flag, object, result_number};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.vtolInFwdFlight",
    "vehicle.fixedWing",
    "settings.unitsSettings.speedUnits",
];

const SLOWEST_GROUND_SPEED_METERS_SECOND: f64 = 0.1;

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
    let target = args.first().and_then(|a| a.parse::<f64>().ok()).filter(|t| t.is_finite());
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
            merge(base, json!({
                "target": unit.show(target_meters_second),
                "targetMetersSecond": target_meters_second,
                "sentence": format!("The aircraft will fly at {}.", unit.label(target_meters_second)),
            }))
        }
        _ => base,
    }
}

fn range_meters_second(backend: &dyn Backend) -> Option<Range> {
    let vehicle = object(&backend.get_fields("vehicle", "vtolInFwdFlight,fixedWing"));
    if vehicle.get("kind") != Some(&Value::String("object".into())) {
        return None;
    }
    let forward_flight = flag(&vehicle, "vtolInFwdFlight") || flag(&vehicle, "fixedWing");
    let number = |path: &str| result_number(&backend.invoke(path, "[]"));
    let range = match forward_flight {
        true => {
            let minimum = number("vehicle.minimumEquivalentAirspeed")?;
            let maximum = number("vehicle.maximumEquivalentAirspeed")?;
            Range { label: "Airspeed", command: "guidedModeChangeEquivalentAirspeedMetersSecond", minimum, maximum, initial: (minimum + maximum) / 2.0 }
        }
        false => {
            let maximum = number("vehicle.maximumHorizontalSpeedMultirotor")?;
            Range { label: "Ground speed", command: "guidedModeChangeGroundSpeedMetersSecond", minimum: SLOWEST_GROUND_SPEED_METERS_SECOND, maximum, initial: maximum / 2.0 }
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
                ("vehicle", true) => json!({ "kind": "object", "vtolInFwdFlight": self.forward, "fixedWing": false }),
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
                "vehicle.maximumHorizontalSpeedMultirotor" => self.maximum,
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
        let view = speed_view(&Fake { connected: true, forward: false, maximum: 15.0 }, &["20".to_string()]);
        assert_eq!(view["label"], "Ground speed");
        assert_eq!(view["command"], "guidedModeChangeGroundSpeedMetersSecond");
        assert_eq!(view["minimum"], 0.1);
        assert_eq!(view["initial"], 7.5);
        assert_eq!(view["targetMetersSecond"], 15.0);
        assert_eq!(view["sentence"], "The aircraft will fly at 15.0 m/s.");
    }

    #[test]
    fn forward_flight_gets_airspeed_between_the_equivalent_limits() {
        let view = speed_view(&Fake { connected: true, forward: true, maximum: 24.0 }, &[]);
        assert_eq!(view["label"], "Airspeed");
        assert_eq!(view["command"], "guidedModeChangeEquivalentAirspeedMetersSecond");
        assert_eq!(view["minimum"], 8.0);
        assert_eq!(view["initial"], 16.0);
        assert!(view.get("sentence").is_none());
    }

    #[test]
    fn an_empty_or_absent_range_is_unavailable() {
        assert_eq!(speed_view(&Fake { connected: true, forward: false, maximum: 0.0 }, &[])["available"], false);
        assert_eq!(speed_view(&Fake { connected: false, forward: false, maximum: 15.0 }, &[])["available"], false);
    }
}
