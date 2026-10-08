use serde_json::{Value, json};

use crate::read::{flag, text, value_number};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.vehicleLinkManager.primaryLinkName",
    "vehicle.vehicleLinkManager.linkNames",
    "vehicle.vehicleLinkManager.linkStatuses",
    "vehicle.vehicleLinkManager.communicationLost",
    "vehicle.vehicleLinkManager.communicationLostEnabled",
    "vehicle.vehicleLinkManager.autoDisconnect",
];

const FIELDS: &str = "primaryLinkName,linkNames,linkStatuses,communicationLost,communicationLostEnabled,autoDisconnect";

const NO_FAILSAFE: &str = "No failsafe";
const APM_DEFAULT_GCS_TIMEOUT_S: f64 = 5.0;

fn px4_loss_action(action: i64) -> Option<&'static str> {
    match action {
        0 => Some(NO_FAILSAFE),
        1 => Some("Hover"),
        2 => Some("Return home"),
        3 => Some("Land"),
        5 => Some("Stop motors"),
        6 => Some("Lockdown"),
        _ => None,
    }
}

fn apm_loss_action(action: i64) -> Option<&'static str> {
    match action {
        0 => Some(NO_FAILSAFE),
        1 | 3 | 4 => Some("Return home"),
        2 => Some("Continue mission"),
        5 => Some("Land"),
        _ => None,
    }
}

pub fn loss_failsafe(px4: (Option<f64>, Option<f64>), apm: (Option<f64>, Option<f64>)) -> (Option<&'static str>, Option<f64>) {
    let (action, after) = match (px4, apm) {
        ((Some(action), after), _) => (px4_loss_action(action as i64), after),
        (_, (Some(action), after)) => (apm_loss_action(action as i64), after.or(Some(APM_DEFAULT_GCS_TIMEOUT_S))),
        _ => (None, None),
    };
    (action, after.filter(|_| action.is_some_and(|a| a != NO_FAILSAFE)))
}

fn parameter(backend: &dyn Backend, name: &str) -> Option<f64> {
    value_number(&backend.value(&format!("vehicle.parameterManager.getParameter(-1,{name}).rawValue")))
}

fn strings(read: &Value, key: &str) -> Vec<String> {
    read.get(key)
        .and_then(Value::as_array)
        .map(|listed| listed.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
        .unwrap_or_default()
}

pub fn vehicle_links_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let manager = backend.value_fields("vehicle.vehicleLinkManager", FIELDS);
    if manager.get("kind").and_then(Value::as_str) != Some("object") {
        return json!({
            "kind": "object",
            "class": "VehicleLinks",
            "available": false,
            "links": [],
            "contactLost": Value::Null,
            "reason": "No vehicle is connected.",
        });
    }
    let watching = flag(&manager, "communicationLostEnabled");
    let primary = text(&manager, "primaryLinkName");
    let names = strings(&manager, "linkNames");
    let statuses = strings(&manager, "linkStatuses");
    let links: Vec<Value> = names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            json!({
                "name": name,
                "primary": !primary.is_empty() && *name == primary,
                "commLost": watching.then(|| statuses.get(index).map(|status| !status.is_empty())).flatten(),
            })
        })
        .collect();
    let (loss_action, loss_after) = loss_failsafe(
        (parameter(backend, "NAV_DLL_ACT"), parameter(backend, "COM_DL_LOSS_T")),
        (parameter(backend, "FS_GCS_ENABLE"), parameter(backend, "FS_GCS_TIMEOUT")),
    );
    json!({
        "kind": "object",
        "class": "VehicleLinks",
        "available": true,
        "lossAction": loss_action,
        "lossAfter": loss_after,
        "watching": watching,
        "primary": (!primary.is_empty()).then_some(primary.clone()),
        "links": links,
        "contactLost": watching.then(|| flag(&manager, "communicationLost")),
        "autoDisconnect": flag(&manager, "autoDisconnect"),
        "reason": match (watching, flag(&manager, "communicationLost")) {
            (false, _) => "This vehicle is not being watched for lost contact.",
            (true, true) => "No link to this vehicle is being heard.",
            (true, false) => "",
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Radios {
        watching: bool,
        lost_second: bool,
    }

    impl Backend for Radios {
        fn get(&self, path: &str) -> String { self.get_fields(path, "") }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "vehicle.vehicleLinkManager" => json!({
                    "kind": "object",
                    "primaryLinkName": "Telemetry",
                    "linkNames": ["Telemetry", "WiFi"],
                    "linkStatuses": ["", if self.lost_second { "Comm Lost" } else { "" }],
                    "communicationLost": false,
                    "communicationLostEnabled": self.watching,
                    "autoDisconnect": false,
                }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_lost_link_names_the_failsafe_the_aircraft_will_fly() {
        assert_eq!(loss_failsafe((Some(2.0), Some(10.0)), (None, None)), (Some("Return home"), Some(10.0)));
        assert_eq!(loss_failsafe((Some(0.0), Some(10.0)), (None, None)), (Some("No failsafe"), None), "a disabled failsafe never counts down");
        assert_eq!(loss_failsafe((None, None), (Some(5.0), None)), (Some("Land"), Some(5.0)), "older ArduPilot has no FS_GCS_TIMEOUT and waits its fixed 5 s");
        assert_eq!(loss_failsafe((None, None), (Some(2.0), Some(8.0))), (Some("Continue mission"), Some(8.0)));
        assert_eq!(loss_failsafe((None, None), (None, None)), (None, None), "before parameters load nothing is promised");
        assert_eq!(loss_failsafe((Some(42.0), Some(10.0)), (None, None)), (None, None));
    }

    #[test]
    fn the_roster_says_which_radio_carries_this_vehicle_and_which_has_gone_quiet() {
        let view = vehicle_links_view(&Radios { watching: true, lost_second: true }, &[]);
        let links = view["links"].as_array().unwrap();
        assert_eq!(links[0]["name"], "Telemetry");
        assert_eq!(links[0]["primary"], true, "this is the radio actually carrying the operator's commands, which view.links cannot answer - that reads the application-wide configuration list, whose connected flag says a link is up SOMEWHERE rather than that this vehicle is still heard on it");
        assert_eq!(links[0]["commLost"], false);
        assert_eq!(links[1]["commLost"], true, "linkStatuses is a parallel QStringList of tr(\"Comm Lost\") or an empty string, so it has to be zipped to linkNames by index and turned into a boolean here - keying on the words breaks in any other locale, and it is the seventh time today");

        let unwatched = vehicle_links_view(&Radios { watching: false, lost_second: true }, &[]);
        assert_eq!(unwatched["contactLost"], Value::Null, "_commLostCheck returns early when the watch is disabled, so the flags never update and false means 'nobody is looking' rather than 'every link is fine'");
        assert_eq!(unwatched["links"][1]["commLost"], Value::Null, "same for the per-link flags, which are the same stale array");
        assert!(unwatched["reason"].as_str().unwrap().contains("not being watched"));
    }
}
