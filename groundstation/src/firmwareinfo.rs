use serde_json::{Value, json};

use crate::read::{integer, object, text};
use crate::router::Backend;

// SetupScreen.kt read six raw Vehicle properties to spell one line - "PX4 1.15.0 beta" - and kept
// the rule for an unset version (Vehicle's versionNotSetValue, -1) to itself. All six notify on
// firmwareVersionChanged, firmwareTypeChanged or vehicleTypeChanged.
pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.firmwareTypeString",
    "vehicle.vehicleTypeString",
    "vehicle.firmwareMajorVersion",
    "vehicle.firmwareMinorVersion",
    "vehicle.firmwarePatchVersion",
    "vehicle.firmwareVersionTypeString",
];

const FIELDS: &str = "firmwareTypeString,vehicleTypeString,firmwareMajorVersion,firmwareMinorVersion,firmwarePatchVersion,firmwareVersionTypeString";

pub fn version(major: Option<i64>, minor: Option<i64>, patch: Option<i64>) -> Option<String> {
    let major = major.filter(|m| *m >= 0)?;
    Some(format!("{major}.{}.{}", minor.filter(|v| *v >= 0).unwrap_or(0), patch.filter(|v| *v >= 0).unwrap_or(0)))
}

pub fn summary(firmware_type: &str, version: Option<&str>, version_type: &str) -> String {
    [firmware_type, version.unwrap_or(""), version_type].iter().map(|part| part.trim()).filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" ")
}

pub fn firmware_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", FIELDS));
    let available = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let firmware_type = text(&vehicle, "firmwareTypeString");
    let version_type = text(&vehicle, "firmwareVersionTypeString");
    let version = version(integer(&vehicle, "firmwareMajorVersion"), integer(&vehicle, "firmwareMinorVersion"), integer(&vehicle, "firmwarePatchVersion"));
    json!({
        "kind": "object",
        "class": "Firmware",
        "available": available,
        "firmwareType": firmware_type,
        "vehicleType": text(&vehicle, "vehicleTypeString"),
        "version": version,
        "versionType": version_type,
        "summary": summary(&firmware_type, version.as_deref(), &version_type),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(firmware: &str, major: i64, minor: i64, patch: i64, kind: &str) -> String {
        summary(firmware, version(Some(major), Some(minor), Some(patch)).as_deref(), kind)
    }

    #[test]
    fn the_firmware_line_is_the_one_setup_screen_spelled() {
        assert_eq!(line("ArduPilot", 4, 5, 7, ""), "ArduPilot 4.5.7", "an official release has an empty version type");
        assert_eq!(line("PX4", 1, 15, 0, "beta"), "PX4 1.15.0 beta");
        assert_eq!(line("ArduPilot", -1, 0, 0, ""), "ArduPilot", "Vehicle's versionNotSetValue is -1 until AUTOPILOT_VERSION arrives");
        assert_eq!(line("", -1, 0, 0, ""), "", "nothing known reads as empty rather than stray separators");
    }

    #[test]
    fn the_view_reads_the_vehicle_once_and_says_when_there_is_none() {
        struct Vehicle(Value);
        impl Backend for Vehicle {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, _p: &str, _f: &str) -> String { self.0.to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let px4 = firmware_view(&Vehicle(json!({ "kind": "object", "firmwareTypeString": "PX4 Pro", "vehicleTypeString": "Quadrotor", "firmwareMajorVersion": 1, "firmwareMinorVersion": 14, "firmwarePatchVersion": 3, "firmwareVersionTypeString": "" })), &[]);
        assert_eq!((&px4["available"], &px4["version"], &px4["summary"], &px4["vehicleType"]), (&json!(true), &json!("1.14.3"), &json!("PX4 Pro 1.14.3"), &json!("Quadrotor")));
        let none = firmware_view(&Vehicle(json!({ "kind": "null" })), &[]);
        assert_eq!((&none["available"], &none["version"], &none["summary"]), (&json!(false), &Value::Null, &json!("")));
    }
}
