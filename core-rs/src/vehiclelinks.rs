use serde_json::{Value, json};

use crate::read::{flag, object, text};
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

fn strings(read: &Value, key: &str) -> Vec<String> {
    read.get(key)
        .and_then(Value::as_array)
        .map(|listed| listed.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
        .unwrap_or_default()
}

pub fn vehicle_links_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let manager = object(&backend.get_fields("vehicle.vehicleLinkManager", FIELDS));
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
    json!({
        "kind": "object",
        "class": "VehicleLinks",
        "available": true,
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
