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
        (!apm).then(|| tool(RESET_VEHICLE_CONFIG, "Reset to vehicle's configuration defaults", "Reset All", "Select Reset to reset all parameters to the vehicle's configuration defaults.")),
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
    let refresh = || crate::guided::dispatch(backend, Some(json!({ "action": "refreshParameters" })), vehicle, "vehicle.parameterManager.refreshAllParameters", "[]");
    let then_refresh = |answer: Value| {
        if answer.get("ok").and_then(Value::as_bool) == Some(true) {
            refresh();
        }
        answer
    };
    match path {
        REFRESH => refresh(),
        RESET_DEFAULTS => then_refresh(crate::guided::dispatch(backend, Some(json!({ "action": "resetParameters" })), vehicle, "vehicle.parameterManager.resetAllParametersToDefaults", "[]")),
        _ => {
            let written = autoconfig_exists(backend).then(|| crate::factwrite::write(backend, AUTOCONFIG, &json!({ "value": AUTOCONFIG_RESET }).to_string()));
            let refreshed = refresh();
            written.unwrap_or(refreshed)
        }
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
        assert_eq!(labels(&Fake { apm: false, autoconfig: false, ready: true })[4], "Reset to vehicle's configuration defaults", "ParameterEditor.qml shows it for every non-ArduPilot vehicle");
        assert_eq!(labels(&Fake { apm: false, autoconfig: true, ready: true })[5], "Clear all RC to Param", "ParameterEditor.qml shows it for PX4 only");
        assert!(labels(&Fake { apm: false, autoconfig: true, ready: false }).is_empty(), "nothing to act on until the parameters are in");
        let tools = parameter_tools_view(&Fake { apm: true, autoconfig: false, ready: true }, &[]);
        assert_eq!(tools["tools"][0]["confirm"], false, "Refresh is immediate");
        assert_eq!(tools["tools"][3]["confirm"], true, "the resets ask first");
    }

    #[test]
    fn a_reset_is_followed_by_a_refresh_as_the_editor_controller_does() {
        struct Recorder(std::cell::RefCell<Vec<String>>);
        impl Backend for Recorder {
            fn get(&self, path: &str) -> String {
                match path {
                    "vehicle.parameterManager.parametersReady" => json!({ "value": true }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "object", "apmFirmware": true, "px4Firmware": false, "id": 1 }).to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { json!({ "ok": true }).to_string() }
            fn invoke(&self, path: &str, _a: &str) -> String {
                self.0.borrow_mut().push(path.to_string());
                json!({ "ok": true }).to_string()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let recorder = Recorder(std::cell::RefCell::new(Vec::new()));
        assert_eq!(run(&recorder, RESET_DEFAULTS)["ok"], true);
        assert_eq!(*recorder.0.borrow(), ["vehicle.parameterManager.resetAllParametersToDefaults", "vehicle.parameterManager.refreshAllParameters"]);
        recorder.0.borrow_mut().clear();
        assert_eq!(run(&recorder, RESET_VEHICLE_CONFIG)["ok"], true);
        assert_eq!(*recorder.0.borrow(), ["vehicle.parameterManager.refreshAllParameters"], "no SYS_AUTOCONFIG: ParameterEditorController still refreshes");
    }

    #[test]
    fn a_vehicle_configuration_reset_without_the_parameter_only_refreshes() {
        assert_eq!(run(&Fake { apm: false, autoconfig: false, ready: true }, RESET_VEHICLE_CONFIG)["ok"], true);
        assert_eq!(run(&Fake { apm: false, autoconfig: true, ready: true }, RESET_VEHICLE_CONFIG)["ok"], true);
    }
}
