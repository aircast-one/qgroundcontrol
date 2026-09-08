use serde_json::{Value, json};

use crate::altitude::merge;
use crate::read::{Unit, flag, object, result_number, value_number};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "settings.flyViewSettings.guidedMaximumAltitude",
    "vehicles.activeVehicleAvailable",
    "settings.unitsSettings.verticalDistanceUnits",
];

const FALLBACK_MINIMUM_METERS: f64 = 3.0;

pub fn takeoff_view(backend: &dyn Backend, args: &[String]) -> Value {
    let unit = Unit::vertical(backend);
    let range = range_meters(backend);
    let target = args.first().and_then(|a| a.parse::<f64>().ok()).filter(|t| t.is_finite());
    let base = json!({
        "kind": "object",
        "class": "GuidedTakeoff",
        "available": range.is_some(),
        "unit": unit.name,
        "minimum": range.map(|(lo, _)| unit.show(lo)),
        "maximum": range.map(|(_, hi)| unit.show(hi)),
        "initial": range.map(|(lo, _)| unit.show(lo)),
        "minimumMeters": range.map(|(lo, _)| lo),
    });
    match (range, target) {
        (Some((lo, hi)), Some(target)) => {
            let target_meters = unit.meters(target).clamp(lo, hi);
            merge(base, json!({
                "target": unit.show(target_meters),
                "targetMeters": target_meters,
                "sentence": format!("The aircraft will take off and climb to {}.", unit.label(target_meters)),
            }))
        }
        _ => base,
    }
}

fn range_meters(backend: &dyn Backend) -> Option<(f64, f64)> {
    if !flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable") {
        return None;
    }
    let minimum = result_number(&backend.invoke("vehicle.minimumTakeoffAltitudeMeters", "[]")).filter(|m| *m > 0.0).unwrap_or(FALLBACK_MINIMUM_METERS);
    let maximum = value_number(&backend.get("settings.flyViewSettings.guidedMaximumAltitude.rawValue"))?;
    (maximum > minimum).then_some((minimum, maximum))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        minimum: Option<f64>,
        maximum: Option<f64>,
        connected: bool,
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            match (path, self.maximum) {
                ("settings.flyViewSettings.guidedMaximumAltitude.rawValue", Some(hi)) => json!({ "kind": "value", "value": hi }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": self.connected }),
                _ => json!({ "kind": "object", "appSettingsVerticalDistanceUnitsString": "m" }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, path: &str, _a: &str) -> String {
            match (path, self.minimum) {
                ("vehicle.minimumTakeoffAltitudeMeters", Some(lo)) => json!({ "ok": true, "result": lo }),
                ("vehicle.minimumTakeoffAltitudeMeters", None) => json!({ "ok": false }),
                _ => json!({ "ok": true, "result": 1.0 }),
            }
            .to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_firmware_minimum_is_the_floor_and_the_initial_value() {
        let view = takeoff_view(&Fake { minimum: Some(10.0), maximum: Some(121.0), connected: true }, &[]);
        assert_eq!(view["available"], true);
        assert_eq!(view["minimum"], 10.0);
        assert_eq!(view["initial"], 10.0);
        assert_eq!(view["maximum"], 121.0);
    }

    #[test]
    fn a_firmware_that_reports_no_minimum_gets_the_fallback() {
        let zero = takeoff_view(&Fake { minimum: Some(0.0), maximum: Some(121.0), connected: true }, &[]);
        assert_eq!(zero["minimumMeters"], 3.0);
        let missing = takeoff_view(&Fake { minimum: None, maximum: Some(121.0), connected: true }, &[]);
        assert_eq!(missing["minimumMeters"], 3.0);
    }

    #[test]
    fn a_target_is_clamped_into_the_range_and_sentenced() {
        let high = takeoff_view(&Fake { minimum: Some(3.0), maximum: Some(50.0), connected: true }, &["80".to_string()]);
        assert_eq!(high["targetMeters"], 50.0);
        assert_eq!(high["sentence"], "The aircraft will take off and climb to 50.0 m.");
        let low = takeoff_view(&Fake { minimum: Some(3.0), maximum: Some(50.0), connected: true }, &["1".to_string()]);
        assert_eq!(low["targetMeters"], 3.0);
    }

    #[test]
    fn without_a_vehicle_there_is_nothing_to_take_off() {
        let view = takeoff_view(&Fake { minimum: Some(3.0), maximum: Some(50.0), connected: false }, &["10".to_string()]);
        assert_eq!(view["available"], false);
        assert!(view.get("sentence").is_none());
    }

    #[test]
    fn no_ceiling_means_no_range() {
        let view = takeoff_view(&Fake { minimum: Some(3.0), maximum: None, connected: true }, &["10".to_string()]);
        assert_eq!(view["available"], false);
        assert!(view.get("sentence").is_none());
    }
}
