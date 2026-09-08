use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.sysStatusSensorInfo"];

pub fn state(enabled: bool, healthy: bool) -> &'static str {
    match (enabled, healthy) {
        (false, _) => "disabled",
        (true, true) => "healthy",
        (true, false) => "unhealthy",
    }
}

fn state_label(state: &str) -> &'static str {
    match state {
        "healthy" => "Healthy",
        "unhealthy" => "Fault",
        _ => "Not enabled",
    }
}

fn rank(state: &str) -> u8 {
    match state {
        "unhealthy" => 0,
        "healthy" => 1,
        _ => 2,
    }
}

pub fn sensors(info: &Value) -> Vec<(String, &'static str)> {
    let names: Vec<&str> = info.get("sensorNames").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    let flags = |key: &str| -> Vec<bool> { info.get(key).and_then(Value::as_array).map(|a| a.iter().map(|v| v.as_bool().unwrap_or(v.as_i64().unwrap_or(0) != 0)).collect()).unwrap_or_default() };
    let (enabled, healthy) = (flags("sensorEnabled"), flags("sensorHealthy"));
    if names.len() != enabled.len() || names.len() != healthy.len() {
        return Vec::new();
    }
    let mut listed: Vec<(usize, String, &'static str)> = names.iter().enumerate().map(|(i, n)| (i, n.to_string(), state(enabled[i], healthy[i]))).collect();
    listed.sort_by_key(|(i, _, s)| (rank(s), *i));
    listed.into_iter().map(|(_, n, s)| (n, s)).collect()
}

pub fn sensors_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let listed = sensors(&object(&backend.get("vehicle.sysStatusSensorInfo")));
    let failing: Vec<&str> = listed.iter().filter(|(_, s)| *s == "unhealthy").map(|(n, _)| n.as_str()).collect();
    json!({
        "kind": "object",
        "class": "SensorHealth",
        "available": !listed.is_empty(),
        "status": if listed.is_empty() { "No vehicle is reporting sensor status." } else { "" },
        "failing": failing,
        "sensors": listed.iter().map(|(name, s)| json!({ "name": name, "state": s, "label": state_label(s) })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failing_sensors_come_first_and_disabled_ones_last() {
        let info = json!({
            "sensorNames": ["Geofence", "GPS", "Gyro", "Logging"],
            "sensorEnabled": [false, true, true, false],
            "sensorHealthy": [false, false, true, true],
        });
        let listed = sensors(&info);
        assert_eq!(listed[0], ("GPS".to_string(), "unhealthy"));
        assert_eq!(listed[1], ("Gyro".to_string(), "healthy"));
        assert_eq!(listed[2].1, "disabled");
        assert_eq!(listed[3].1, "disabled");
    }

    #[test]
    fn mismatched_lists_are_treated_as_no_report() {
        assert!(sensors(&json!({ "sensorNames": ["GPS"], "sensorEnabled": [true], "sensorHealthy": [] })).is_empty());
    }

    #[test]
    fn the_view_names_the_failing_sensors() {
        struct Fake;
        impl Backend for Fake {
            fn get(&self, _p: &str) -> String {
                json!({ "kind": "object", "sensorNames": ["GPS", "Gyro"], "sensorEnabled": [true, true], "sensorHealthy": [false, true] }).to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = sensors_view(&Fake, &[]);
        assert_eq!(view["available"], true);
        assert_eq!(view["failing"][0], "GPS");
        assert_eq!(view["sensors"][0]["label"], "Fault");
        assert_eq!(view["status"], "");
    }
}
