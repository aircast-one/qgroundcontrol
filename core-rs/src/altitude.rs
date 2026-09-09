use serde_json::{Value, json};

use crate::read::{Unit, value_number};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "settings.flyViewSettings.guidedMinimumAltitude",
    "settings.flyViewSettings.guidedMaximumAltitude",
    "vehicle.altitudeRelative",
    "settings.unitsSettings.verticalDistanceUnits",
];

const SMALLEST_CHANGE_METERS: f64 = 0.01;

struct Range {
    current: f64,
    minimum: f64,
    maximum: f64,
}

pub fn altitude_view(backend: &dyn Backend, args: &[String]) -> Value {
    let unit = Unit::vertical(backend);
    let range = range_meters(backend);
    let target = args.first().and_then(|a| a.parse::<f64>().ok()).filter(|t| t.is_finite());
    let pause = args.get(1).is_some_and(|a| a.trim().eq_ignore_ascii_case("pause"));
    let base = json!({
        "kind": "object",
        "class": "GuidedAltitude",
        "label": "Height above launch",
        "available": range.is_some(),
        "unit": unit.name,
        "current": range.as_ref().map(|r| unit.show(r.current)),
        "minimum": range.as_ref().map(|r| unit.show(r.minimum)),
        "maximum": range.as_ref().map(|r| unit.show(r.maximum)),
        "currentMeters": range.as_ref().map(|r| r.current),
    });
    match (range, target) {
        (Some(range), Some(target)) => merge(base, with_target(&range, target, &unit, pause)),
        _ => base,
    }
}

fn with_target(range: &Range, target: f64, unit: &Unit, pause: bool) -> Value {
    let target_meters = unit.meters(target);
    let delta_meters = target_meters - range.current;
    let changes = delta_meters.abs() >= SMALLEST_CHANGE_METERS;
    let sends = pause || changes;
    let sentence = match (pause, changes, delta_meters > 0.0) {
        (true, false, _) => format!("The aircraft will stop and hold at {}.", unit.label(range.current)),
        (true, true, true) => format!("The aircraft will stop, then climb {} to {}.", unit.label(delta_meters), unit.label(target_meters)),
        (true, true, false) => format!("The aircraft will stop, then descend {} to {}.", unit.label(-delta_meters), unit.label(target_meters)),
        (false, false, _) => format!("The aircraft is already at {} and will not move.", unit.label(range.current)),
        (false, true, true) => format!("The aircraft will climb {} to {}.", unit.label(delta_meters), unit.label(target_meters)),
        (false, true, false) => format!("The aircraft will descend {} to {}.", unit.label(-delta_meters), unit.label(target_meters)),
    };
    json!({
        "target": target,
        "targetMeters": target_meters,
        "delta": unit.show(delta_meters),
        "deltaMeters": delta_meters,
        "sends": sends,
        "pause": pause,
        "sentence": sentence,
    })
}

pub fn merge(base: Value, extra: Value) -> Value {
    match (base, extra) {
        (Value::Object(map), Value::Object(more)) => Value::Object(map.into_iter().chain(more).collect()),
        (base, _) => base,
    }
}

fn range_meters(backend: &dyn Backend) -> Option<Range> {
    let current = value_number(&backend.get("vehicle.altitudeRelative.rawValue"))?;
    let minimum = value_number(&backend.get("settings.flyViewSettings.guidedMinimumAltitude.rawValue"))?;
    let maximum = value_number(&backend.get("settings.flyViewSettings.guidedMaximumAltitude.rawValue"))?;
    (maximum > minimum).then(|| Range { current, minimum: minimum.min(current), maximum: maximum.max(current) })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        current: Option<f64>,
        feet: bool,
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            let value = match path {
                "vehicle.altitudeRelative.rawValue" => self.current,
                "settings.flyViewSettings.guidedMinimumAltitude.rawValue" => Some(2.0),
                "settings.flyViewSettings.guidedMaximumAltitude.rawValue" => Some(121.0),
                _ => None,
            };
            match value {
                Some(v) => json!({ "kind": "value", "value": v }).to_string(),
                None => json!({ "kind": "null" }).to_string(),
            }
        }
        fn get_fields(&self, _path: &str, _fields: &str) -> String {
            json!({ "kind": "object", "appSettingsVerticalDistanceUnitsString": if self.feet { "ft" } else { "m" } }).to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            json!({ "ok": true, "result": if self.feet { 3.28084 } else { 1.0 } }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_range_widens_to_contain_the_aircraft() {
        let view = altitude_view(&Fake { current: Some(150.0), feet: false }, &[]);
        assert_eq!(view["available"], true);
        assert_eq!(view["minimum"], 2.0);
        assert_eq!(view["maximum"], 150.0);
        assert_eq!(view["current"], 150.0);
        assert_eq!(view["unit"], "m");
        assert_eq!(view["label"], "Height above launch");
        assert!(view.get("sentence").is_none());
    }

    #[test]
    fn a_target_gives_the_delta_to_send_and_the_sentence() {
        let climb = altitude_view(&Fake { current: Some(25.0), feet: false }, &["68.5".to_string()]);
        assert_eq!(climb["sends"], true);
        assert_eq!(climb["deltaMeters"], 43.5);
        assert_eq!(climb["sentence"], "The aircraft will climb 43.5 m to 68.5 m.");
        let descend = altitude_view(&Fake { current: Some(25.0), feet: false }, &["10".to_string()]);
        assert_eq!(descend["sentence"], "The aircraft will descend 15.0 m to 10.0 m.");
    }

    #[test]
    fn a_change_under_the_firmware_threshold_does_not_send_and_says_so() {
        let same = altitude_view(&Fake { current: Some(25.0), feet: false }, &["25.005".to_string()]);
        assert_eq!(same["sends"], false);
        assert_eq!(same["sentence"], "The aircraft is already at 25.0 m and will not move.");
        let just = altitude_view(&Fake { current: Some(25.0), feet: false }, &["25.03".to_string()]);
        assert_eq!(just["sends"], true);
        let hold = altitude_view(&Fake { current: Some(25.0), feet: false }, &["25.0".to_string(), "pause".to_string()]);
        assert_eq!((hold["sends"].clone(), hold["pause"].clone()), (json!(true), json!(true)));
        assert_eq!(hold["sentence"], "The aircraft will stop and hold at 25.0 m.");
        let stop_then_climb = altitude_view(&Fake { current: Some(25.0), feet: false }, &["30".to_string(), "pause".to_string()]);
        assert_eq!(stop_then_climb["sentence"], "The aircraft will stop, then climb 5.0 m to 30.0 m.");
        assert_eq!(same["pause"], false);
    }

    #[test]
    fn numbers_are_in_the_operators_unit_and_metres_ride_alongside() {
        let feet = altitude_view(&Fake { current: Some(10.0), feet: true }, &["65.6168".to_string()]);
        assert_eq!(feet["unit"], "ft");
        assert!((feet["current"].as_f64().unwrap() - 32.8084).abs() < 1e-3);
        assert!((feet["targetMeters"].as_f64().unwrap() - 20.0).abs() < 1e-3);
        assert!((feet["deltaMeters"].as_f64().unwrap() - 10.0).abs() < 1e-3);
        assert_eq!(feet["sentence"], "The aircraft will climb 32.8 ft to 65.6 ft.");
    }

    #[test]
    fn without_a_vehicle_nothing_is_available() {
        let view = altitude_view(&Fake { current: None, feet: false }, &["30".to_string()]);
        assert_eq!(view["available"], false);
        assert_eq!(view["current"], Value::Null);
        assert!(view.get("sentence").is_none());
    }
}
