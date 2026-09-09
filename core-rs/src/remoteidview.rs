use serde_json::Value;

use crate::read::{object, value_number, value_string};
use crate::remoteid::{GcsFix, Settings};
use crate::router::Backend;

pub const GROUP: &str = "settings.remoteIDSettings";
pub const GCS_POSITION: &str = "positionManager.gcsPosition";
pub const GCS_TIMESTAMP: &str = "positionManager.gcsPositionTimestamp";
const FACTS: [&str; 21] = ["operatorID", "operatorIDValid", "operatorIDType", "sendOperatorID", "selfIDFree", "selfIDEmergency", "selfIDExtended", "selfIDType", "sendSelfID", "basicID", "basicIDType", "basicIDUaType", "sendBasicID", "region", "locationType", "latitudeFixed", "longitudeFixed", "altitudeFixed", "classificationType", "categoryEU", "classEU"];

pub fn deps() -> Vec<String> {
    FACTS.iter().map(|f| format!("{GROUP}.{f}.rawValue")).chain([GCS_POSITION.to_string(), GCS_TIMESTAMP.to_string()]).collect()
}

fn fact_path(name: &str) -> String {
    format!("{GROUP}.{name}.rawValue")
}

pub fn settings(backend: &dyn Backend) -> Settings {
    let number = |name: &str| value_number(&backend.get(&fact_path(name))).unwrap_or(0.0);
    let text = |name: &str| value_string(&backend.get(&fact_path(name)));
    let flag = |name: &str| object(&backend.get(&fact_path(name))).get("value").and_then(Value::as_bool).unwrap_or(false);
    Settings {
        region: number("region") as i64,
        operator_id: text("operatorID"),
        operator_id_type: number("operatorIDType") as i64,
        operator_id_valid: flag("operatorIDValid"),
        send_operator_id: flag("sendOperatorID"),
        basic_id: text("basicID"),
        basic_id_type: number("basicIDType") as i64,
        basic_id_ua_type: number("basicIDUaType") as i64,
        send_basic_id: flag("sendBasicID"),
        send_self_id: flag("sendSelfID"),
        self_id_type: number("selfIDType") as i64,
        self_id_free: text("selfIDFree"),
        self_id_emergency: text("selfIDEmergency"),
        self_id_extended: text("selfIDExtended"),
        location_type: number("locationType") as u32,
        classification_type: number("classificationType") as u32,
        latitude_fixed: number("latitudeFixed"),
        longitude_fixed: number("longitudeFixed"),
        altitude_fixed: number("altitudeFixed"),
        category_eu: number("categoryEU") as u32,
        class_eu: number("classEU") as u32,
    }
}

pub fn fix(backend: &dyn Backend, wall_ms: u64) -> GcsFix {
    let position = object(&backend.get(GCS_POSITION));
    let coordinate = position.get("value").filter(|v| v.is_object()).unwrap_or(&position);
    let valid = coordinate.get("valid").and_then(Value::as_bool).unwrap_or(false);
    let read = |key: &str| coordinate.get(key).and_then(Value::as_f64).unwrap_or(f64::NAN);
    let stamp = value_number(&backend.get(GCS_TIMESTAMP)).filter(|t| *t > 0.0).map(|t| t as u64);
    GcsFix { valid: valid && stamp.is_some(), latitude: read("latitude"), longitude: read("longitude"), altitude: read("altitude"), age_ms: stamp.map(|t| wall_ms.saturating_sub(t)).unwrap_or(u64::MAX) }
}

pub fn inputs(backend: &dyn Backend, wall_ms: u64) -> (Settings, GcsFix) {
    (settings(backend), fix(backend, wall_ms))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Fake;
    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            match path {
                "settings.remoteIDSettings.region.rawValue" => json!({ "kind": "value", "value": 1 }),
                "settings.remoteIDSettings.operatorID.rawValue" => json!({ "kind": "value", "value": "FIN87astrdge12k8" }),
                "settings.remoteIDSettings.sendBasicID.rawValue" => json!({ "kind": "value", "value": true }),
                "settings.remoteIDSettings.latitudeFixed.rawValue" => json!({ "kind": "value", "value": 47.25 }),
                "positionManager.gcsPosition" => json!({ "kind": "value", "value": { "valid": true, "latitude": 47.5, "longitude": 8.5, "altitude": null } }),
                "positionManager.gcsPositionTimestamp" => json!({ "kind": "value", "value": 1_700_000_000_000u64 }),
                _ => json!({ "kind": "value", "value": null }),
            }
            .to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn settings_and_the_gcs_fix_come_from_the_facts_and_the_fix_ages_until_it_moves() {
        let settings = settings(&Fake);
        assert_eq!((settings.region, settings.operator_id.as_str(), settings.send_basic_id, settings.latitude_fixed, settings.send_self_id), (1, "FIN87astrdge12k8", true, 47.25, false));
        let fresh = fix(&Fake, 1_700_000_000_100);
        assert!(fresh.valid && fresh.latitude == 47.5 && fresh.age_ms == 100);
        assert!(fresh.altitude.is_nan(), "a coordinate without an altitude keeps saying so, for the FAA rule");
        assert_eq!(fix(&Fake, 1_700_000_006_000).age_ms, 6_000, "the fix ages from the position manager's own timestamp");
        assert_eq!(deps().len(), 23);
        assert!(deps().contains(&"settings.remoteIDSettings.classEU.rawValue".to_string()));
    }
}
