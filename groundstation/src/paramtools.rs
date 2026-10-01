use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.parameterManager.parametersReady",
    "vehicle.apmFirmware",
    "vehicle.px4Firmware",
];

pub const REFRESH: &str = "parameterTools.refresh";
pub const RESET_DEFAULTS: &str = "parameterTools.resetDefaults";
pub const RESET_VEHICLE_CONFIG: &str = "parameterTools.resetVehicleConfig";
const REBOOT: &str = "vehicle.rebootVehicle";
const AUTOCONFIG: &str = "vehicle.parameterManager.getParameter(-1,SYS_AUTOCONFIG)";
const AUTOCONFIG_RESET: f64 = 2.0;

fn tool(path: &str, label: &str, title: &str, message: &str) -> Value {
    json!({ "path": path, "label": label, "confirmTitle": title, "confirmMessage": message, "confirm": !title.is_empty() })
}

fn autoconfig_exists(backend: &dyn Backend) -> bool {
    object(&backend.get(AUTOCONFIG)).get("name").and_then(Value::as_str).is_some_and(|n| !n.is_empty())
}

pub fn parameter_tools_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", "apmFirmware,px4Firmware"));
    let connected = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let ready = connected && flag(&object(&backend.get("vehicle.parameterManager.parametersReady")), "value");
    let apm = flag(&vehicle, "apmFirmware");
    let tools: Vec<Value> = [
        Some(tool(REFRESH, "Refresh", "", "")),
        Some(tool(crate::paramfile::FILE_REVIEW, "Load from file for review...", "", "")),
        Some(tool(crate::paramfile::FILE_SAVE, "Save to file...", "", "")),
        Some(tool(RESET_DEFAULTS, "Reset all to firmware's defaults", "Reset All", "Select Reset to reset all parameters to their defaults.\n\nNote that this will also completely reset everything, including UAVCAN nodes, all vehicle settings, setup and calibrations.")),
        (!apm && autoconfig_exists(backend)).then(|| tool(RESET_VEHICLE_CONFIG, "Reset to vehicle's configuration defaults", "Reset All", "Select Reset to reset all parameters to the vehicle's configuration defaults.")),
        flag(&vehicle, "px4Firmware").then(|| tool(crate::rctoparam::CLEAR_RC_TO_PARAM, "Clear all RC to Param", "", "")),
        Some(tool(REBOOT, "Reboot Vehicle", "Reboot Vehicle", "Select Ok to reboot vehicle.")),
    ]
    .into_iter()
    .flatten()
    .collect();
    json!({ "kind": "object", "class": "ParameterTools", "available": ready, "tools": if ready { tools } else { vec![] } })
}

pub fn owns(path: &str) -> bool {
    [REFRESH, RESET_DEFAULTS, RESET_VEHICLE_CONFIG].contains(&path)
}

pub fn run(backend: &dyn Backend, path: &str) -> Value {
    let vehicle = crate::guided::active_id(backend);
    if vehicle.is_none() {
        return json!({ "ok": false, "refusal": "noVehicle", "reason": "No vehicle is connected." });
    }
    match path {
        REFRESH => crate::guided::dispatch(backend, Some(json!({ "action": "refreshParameters" })), vehicle, "vehicle.parameterManager.refreshAllParameters", "[]"),
        RESET_DEFAULTS => crate::guided::dispatch(backend, Some(json!({ "action": "resetParameters" })), vehicle, "vehicle.parameterManager.resetAllParametersToDefaults", "[]"),
        _ => match autoconfig_exists(backend) {
            true => crate::factwrite::write(backend, AUTOCONFIG, &json!({ "value": AUTOCONFIG_RESET }).to_string()),
            false => json!({ "ok": false, "refusal": "unsupported", "reason": "This vehicle has no SYS_AUTOCONFIG to reset from." }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        apm: bool,
        autoconfig: bool,
        ready: bool,
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.parameterManager.parametersReady" => json!({ "value": self.ready }),
                AUTOCONFIG if self.autoconfig => json!({ "kind": "fact", "name": "SYS_AUTOCONFIG", "value": 0 }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "object", "apmFirmware": self.apm, "px4Firmware": !self.apm, "id": 1 }).to_string() }
        fn set(&self, _p: &str, _v: &str) -> String { json!({ "ok": true }).to_string() }
        fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true }).to_string() }
        fn watch(&self, _p: &[String]) {}
    }

    fn labels(fake: &Fake) -> Vec<String> {
        parameter_tools_view(fake, &[])["tools"].as_array().unwrap().iter().map(|t| t["label"].as_str().unwrap().to_string()).collect()
    }

    #[test]
    fn the_menu_matches_the_parameter_editor() {
        assert_eq!(labels(&Fake { apm: true, autoconfig: false, ready: true }), ["Refresh", "Load from file for review...", "Save to file...", "Reset all to firmware's defaults", "Reboot Vehicle"], "ArduPilot has no vehicle configuration reset");
        assert_eq!(labels(&Fake { apm: false, autoconfig: true, ready: true })[4], "Reset to vehicle's configuration defaults");
        assert_eq!(labels(&Fake { apm: false, autoconfig: true, ready: true })[5], "Clear all RC to Param", "ParameterEditor.qml shows it for PX4 only");
        assert!(labels(&Fake { apm: false, autoconfig: true, ready: false }).is_empty(), "nothing to act on until the parameters are in");
        let tools = parameter_tools_view(&Fake { apm: true, autoconfig: false, ready: true }, &[]);
        assert_eq!(tools["tools"][0]["confirm"], false, "Refresh is immediate");
        assert_eq!(tools["tools"][3]["confirm"], true, "the resets ask first");
    }

    #[test]
    fn a_vehicle_configuration_reset_needs_the_parameter() {
        assert_eq!(run(&Fake { apm: false, autoconfig: false, ready: true }, RESET_VEHICLE_CONFIG)["refusal"], "unsupported");
        assert_eq!(run(&Fake { apm: false, autoconfig: true, ready: true }, REFRESH)["ok"], true);
    }
}
