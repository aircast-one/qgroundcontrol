use serde_json::{Value, json};

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
    let factor = result_number(&backend.invoke("units.metersToAppSettingsVerticalDistanceUnits", "[1.0]"))
        .filter(|f| f.is_finite() && *f > 0.0)
        .unwrap_or(1.0);
    let unit = object(&backend.get_fields("units", "appSettingsVerticalDistanceUnitsString"))
        .get("appSettingsVerticalDistanceUnitsString")
        .and_then(Value::as_str)
        .filter(|u| !u.is_empty())
        .unwrap_or("m")
        .to_string();
    let range = range_meters(backend);
    let target = args.first().and_then(|a| a.parse::<f64>().ok()).filter(|t| t.is_finite());
    let base = json!({
        "kind": "object",
        "class": "GuidedAltitude",
        "available": range.is_some(),
        "unit": unit,
        "current": range.as_ref().map(|r| r.current * factor),
        "minimum": range.as_ref().map(|r| r.minimum * factor),
        "maximum": range.as_ref().map(|r| r.maximum * factor),
        "currentMeters": range.as_ref().map(|r| r.current),
    });
    match (range, target) {
        (Some(range), Some(target)) => with_target(base, &range, target, factor, &unit),
        _ => base,
    }
}

fn with_target(base: Value, range: &Range, target: f64, factor: f64, unit: &str) -> Value {
    let target_meters = target / factor;
    let delta_meters = target_meters - range.current;
    let sends = delta_meters.abs() >= SMALLEST_CHANGE_METERS;
    let sentence = match (sends, delta_meters > 0.0) {
        (false, _) => format!("The aircraft is already at {:.1} {unit} and will not move.", range.current * factor),
        (true, true) => format!("The aircraft will climb {:.1} {unit} to {:.1} {unit}.", delta_meters * factor, target),
        (true, false) => format!("The aircraft will descend {:.1} {unit} to {:.1} {unit}.", -delta_meters * factor, target),
    };
    let extra = json!({
        "target": target,
        "targetMeters": target_meters,
        "delta": delta_meters * factor,
        "deltaMeters": delta_meters,
        "sends": sends,
        "sentence": sentence,
    });
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

fn object(json: &str) -> Value {
    serde_json::from_str(json).unwrap_or(Value::Null)
}

fn value_number(json: &str) -> Option<f64> {
    object(json).get("value")?.as_f64().filter(|v| v.is_finite())
}

fn result_number(json: &str) -> Option<f64> {
    let reply = object(json);
    (reply.get("ok") == Some(&Value::Bool(true))).then(|| reply.get("result")?.as_f64()).flatten()
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
