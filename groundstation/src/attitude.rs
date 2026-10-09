use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.roll",
    "vehicle.pitch",
    "vehicle.heading",
    "vehicle.headingToHome",
    "vehicle.headingToNextWP",
    "vehicle.groundSpeed",
    "vehicle.gps.courseOverGround",
    "settings.flyViewSettings.showAdditionalIndicatorsCompass",
    "settings.flyViewSettings.lockNoseUpCompass",
    "vehicle.coordinate",
    "positionManager.gcsPosition",
];

const SAME_SPOT_M: f64 = 2.0;

const COG_MINIMUM_SPEED: f64 = 0.5;

pub(crate) fn raw(backend: &dyn Backend, path: &str) -> Option<f64> {
    let fact = backend.value(path);
    fact.get("rawValue").or(fact.get("value")).and_then(Value::as_f64).filter(|v| v.is_finite())
}

fn setting(backend: &dyn Backend, name: &str) -> bool {
    let fact = backend.value(&format!("settings.flyViewSettings.{name}"));
    fact.get("value").is_some_and(|v| v.as_bool().unwrap_or_else(|| v.as_f64().is_some_and(|n| n != 0.0)))
}

pub(crate) fn point(backend: &dyn Backend, path: &str) -> Option<(f64, f64)> {
    let at = backend.value(path);
    let valid = at.get("valid").and_then(Value::as_bool).unwrap_or(true);
    Some((at.get("latitude")?.as_f64()?, at.get("longitude")?.as_f64()?)).filter(|(lat, lon)| valid && lat.is_finite() && lon.is_finite() && (*lat, *lon) != (0.0, 0.0))
}

pub fn pilot_bearing(aircraft: Option<(f64, f64)>, pilot: Option<(f64, f64)>) -> Option<f64> {
    let (from, to) = (aircraft?, pilot?);
    (crate::track::distance_m(from, to) >= SAME_SPOT_M).then(|| crate::track::azimuth_deg(from, to))
}

pub fn heading_text(heading: f64) -> String {
    format!("{}\u{b0}", heading.round() as i64)
}

pub fn attitude_view(backend: &dyn Backend, _args: &[String]) -> Value {
    if backend.value_fields("vehicle", "heading").get("kind").and_then(Value::as_str) != Some("object") {
        return json!({ "kind": "object", "class": "Attitude", "available": false });
    }
    let heading = raw(backend, "vehicle.heading").unwrap_or(0.0);
    let additional = setting(backend, "showAdditionalIndicatorsCompass");
    let moving = raw(backend, "vehicle.groundSpeed").is_some_and(|s| s >= COG_MINIMUM_SPEED);
    let shown = |value: Option<f64>, when: bool| value.filter(|_| when).map_or(Value::Null, |v| json!(v));
    json!({
        "kind": "object",
        "class": "Attitude",
        "available": true,
        "roll": raw(backend, "vehicle.roll").unwrap_or(0.0),
        "pitch": raw(backend, "vehicle.pitch").unwrap_or(0.0),
        "heading": heading,
        "headingText": heading_text(heading),
        "courseOverGround": shown(raw(backend, "vehicle.gps.courseOverGround"), additional && moving),
        "headingToHome": shown(raw(backend, "vehicle.headingToHome"), additional),
        "headingToNextWaypoint": shown(raw(backend, "vehicle.headingToNextWP"), additional),
        "noseUp": setting(backend, "lockNoseUpCompass"),
        "homeBearing": raw(backend, "vehicle.headingToHome").map_or(Value::Null, |v| json!(v)),
        "pilotBearing": pilot_bearing(point(backend, "vehicle.coordinate"), point(backend, "positionManager.gcsPosition")).map_or(Value::Null, |v| json!(v)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    struct Fake(BTreeMap<&'static str, Value>);

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            self.0.get(path).map_or_else(|| json!({ "kind": "null" }), |v| json!({ "kind": "fact", "rawValue": v, "value": v })).to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match (p, self.0.contains_key("vehicles.activeVehicleAvailable")) {
                ("vehicle", true) => json!({ "kind": "object" }).to_string(),
                _ => self.get(p),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn vehicle(extra: &[(&'static str, Value)]) -> Fake {
        let base = [("vehicles.activeVehicleAvailable", json!(true)), ("vehicle.roll", json!(-12.5)), ("vehicle.pitch", json!(4.0)), ("vehicle.heading", json!(7.4)), ("vehicle.groundSpeed", json!(3.0)), ("vehicle.gps.courseOverGround", json!(15.0)), ("vehicle.headingToHome", json!(190.0)), ("vehicle.headingToNextWP", Value::Null)];
        Fake(base.into_iter().chain(extra.iter().cloned()).collect())
    }

    #[test]
    fn serves_the_angles_qgc_draws() {
        let view = attitude_view(&vehicle(&[]), &[]);
        assert_eq!((view["roll"].as_f64(), view["pitch"].as_f64(), view["heading"].as_f64()), (Some(-12.5), Some(4.0), Some(7.4)));
        assert_eq!(view["headingText"], "7\u{b0}", "QGCCompassWidget writes heading.toFixed(0) + \"\u{b0}\"; the attitude widget's padded label is never shown");
        assert_eq!(view["courseOverGround"], Value::Null, "the extra pointers are off until showAdditionalIndicatorsCompass is set");
    }

    #[test]
    fn extra_indicators_follow_the_compass_rules() {
        let on = vehicle(&[("settings.flyViewSettings.showAdditionalIndicatorsCompass", json!(true))]);
        let view = attitude_view(&on, &[]);
        assert_eq!(view["courseOverGround"], 15.0);
        assert_eq!(view["headingToHome"], 190.0);
        assert_eq!(view["headingToNextWaypoint"], Value::Null, "a NaN bearing (no next waypoint) hides its pointer");
        let slow = vehicle(&[("settings.flyViewSettings.showAdditionalIndicatorsCompass", json!(true)), ("vehicle.groundSpeed", json!(0.4))]);
        assert_eq!(attitude_view(&slow, &[])["courseOverGround"], Value::Null, "course over ground means nothing below 0.5 m/s, so QGC hides it");
    }

    #[test]
    fn the_dial_always_knows_home_and_points_at_the_pilot() {
        assert_eq!(attitude_view(&vehicle(&[]), &[])["homeBearing"], 190.0, "home shows on the flight dial whatever the QGC compass setting, as DJI's navigation display does");
        let east = pilot_bearing(Some((41.7, 44.8)), Some((41.7, 44.801))).unwrap();
        assert!((east - 90.0).abs() < 0.5, "a pilot due east is at 90 degrees, got {east}");
        assert_eq!(pilot_bearing(Some((41.7, 44.8)), Some((41.7, 44.8))), None, "standing on the aircraft there is no direction to point");
        assert_eq!(pilot_bearing(None, Some((41.7, 44.8))), None);
    }

    #[test]
    fn no_vehicle_is_unavailable() {
        assert_eq!(attitude_view(&Fake(BTreeMap::new()), &[])["available"], false);
        assert_eq!(heading_text(359.6), "360\u{b0}", "QGC rounds with toFixed(0) and never wraps");
    }
}
