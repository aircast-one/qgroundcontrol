use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "links.linkConfigurations",
    "settings.autoConnectSettings.autoConnectPixhawk",
    "settings.autoConnectSettings.autoConnectSiKRadio",
    "settings.autoConnectSettings.autoConnectUDP",
];

const OFFLINE_STALL_MS: u64 = 10_000;

static CONNECTING_SINCE: Mutex<Option<(String, u64)>> = Mutex::new(None);

fn connecting_stalled(name: &str, now: u64) -> bool {
    let mut since = CONNECTING_SINCE
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    match (name.is_empty(), since.as_ref()) {
        (true, _) => {
            *since = None;
            false
        }
        (false, Some((current, started))) if current == name => {
            now.saturating_sub(*started) >= OFFLINE_STALL_MS
        }
        (false, _) => {
            *since = Some((name.to_string(), now));
            false
        }
    }
}

fn auto_connect(backend: &dyn Backend, name: &str) -> bool {
    crate::read::truthy(
        &backend.value(&format!("settings.autoConnectSettings.{name}.rawValue")),
        "value",
    )
}

pub struct Offline {
    pub title: String,
    pub main_status: String,
    pub footnote: String,
    pub busy: bool,
    pub no_links: bool,
    pub edit_address: bool,
    pub on_device: bool,
    pub no_autopilot: bool,
}

pub fn offline(
    links: &[Value],
    watched: &[&str],
    stalled_for: impl FnOnce(&str) -> bool,
) -> Offline {
    let configured: Vec<&Value> = links.iter().filter(|l| l["dynamic"] != true).collect();
    let no_links = configured.is_empty();
    let connected = configured.iter().find(|l| l["connected"] == true);
    let connecting = connected
        .and_then(|l| l["name"].as_str())
        .unwrap_or("")
        .to_string();
    let failed = configured
        .iter()
        .find(|l| l["connected"] != true && l["lastError"].as_str().is_some_and(|e| !e.is_empty()));
    let failed_name = failed
        .and_then(|l| l["name"].as_str())
        .unwrap_or("")
        .to_string();
    let edit_address = failed.is_some_and(|l| l["errorRemedy"] == "editAddress");
    let stalled = stalled_for(&connecting);
    let on_device = connected.is_some_and(|l| l["device"] == true);
    let no_autopilot = stalled && on_device;
    let watching = !watched.is_empty();
    let watched_links = watched.join(", ");
    let title = match (no_autopilot, connecting.is_empty(), failed_name.is_empty()) {
        (true, _, _) => "No Flight Controller",
        (false, false, _) => "Connecting…",
        (false, true, false) => "Connection Failed",
        (false, true, true) => "Not Connected",
    };
    let main_status = match (no_autopilot, connecting.is_empty(), failed_name.is_empty()) {
        (true, _, _) => "No Flight Controller",
        (false, false, _) => "Connecting…",
        (false, true, false) => "Can't Connect",
        (false, true, true) => "Not Connected",
    };
    let footnote = match () {
        _ if no_autopilot => format!(
            "{connecting} hears no flight controller. Check it is plugged in and powered on."
        ),
        _ if !connecting.is_empty() && stalled => format!(
            "No telemetry from {connecting} yet. Check the port is the drone's MAVLink port."
        ),
        _ if !connecting.is_empty() => format!("Connecting to {connecting}…"),
        _ if edit_address => {
            format!("Couldn't connect to {failed_name}. Check its address in Connection Settings.")
        }
        _ if !failed_name.is_empty() => format!(
            "Couldn't connect to {failed_name}. Check the drone is powered on and on the same network, then tap Retry."
        ),
        _ if !watching => "Automatic connection is off.".to_string(),
        _ => format!("Looking for a vehicle on {watched_links}…"),
    };
    let looking = watching && connecting.is_empty() && failed_name.is_empty();
    Offline {
        title: title.to_string(),
        main_status: main_status.to_string(),
        footnote,
        busy: (!connecting.is_empty() && !stalled) || looking,
        no_links,
        edit_address,
        on_device,
        no_autopilot,
    }
}

fn on_device(link: Value, device: &str) -> Value {
    let reaches = link["type"] == "websocket"
        && !device.is_empty()
        && link["summary"]
            .as_str()
            .is_some_and(|url| crate::deviceviews::host_of(url) == device);
    match link {
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .chain([("device".to_string(), json!(reaches))])
                .collect(),
        ),
        other => other,
    }
}

pub fn offline_status_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let model = backend.value("links.linkConfigurations");
    let device = crate::deviceviews::paired_host();
    let links: Vec<Value> = model
        .get("elements")
        .and_then(Value::as_array)
        .map(|e| {
            e.iter()
                .enumerate()
                .map(|(i, el)| on_device(crate::links::link_json(i, el), &device))
                .collect()
        })
        .unwrap_or_default();
    let watched: Vec<&str> = [
        ("autoConnectPixhawk", "USB"),
        ("autoConnectSiKRadio", "SiK radio"),
        ("autoConnectUDP", "Wi\u{2011}Fi"),
    ]
    .into_iter()
    .filter(|(setting, _)| auto_connect(backend, setting))
    .map(|(_, name)| name)
    .collect();
    let shown = offline(&links, &watched, |name| {
        connecting_stalled(name, crate::hub::now_ms())
    });
    json!({
        "kind": "object",
        "class": "OfflineStatus",
        "title": shown.title,
        "mainStatus": shown.main_status,
        "footnote": shown.footnote,
        "busy": shown.busy,
        "noLinks": shown.no_links,
        "editAddress": shown.edit_address,
        "onDevice": shown.on_device,
        "links": links.iter().filter(|l| l["dynamic"] != true).map(offline_link).collect::<Vec<_>>(),
    })
}

fn offline_link(link: &Value) -> Value {
    let error = link["lastError"].as_str().unwrap_or("");
    let summary = link["summary"].as_str().unwrap_or("");
    let connected = link["connected"] == true;
    let name = link["name"].as_str().unwrap_or("");
    let address = if name.contains(summary) { "" } else { summary };
    let description = [address, error]
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    json!({
        "index": link["index"],
        "text": name,
        "description": description,
        "retry": !error.is_empty() && !connected,
        "failed": !error.is_empty(),
        "connected": connected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(name: &str, connected: bool, error: &str, remedy: &str) -> Value {
        json!({ "name": name, "dynamic": false, "connected": connected, "lastError": error, "errorRemedy": if remedy.is_empty() { Value::Null } else { json!(remedy) } })
    }

    #[test]
    fn the_footnote_says_what_auto_connect_is_doing() {
        let all = ["USB", "SiK radio", "Wi\u{2011}Fi"];
        let looking = offline(&[], &all, |_| false);
        assert_eq!(
            (looking.title.as_str(), looking.busy, looking.no_links),
            ("Not Connected", true, true)
        );
        assert_eq!(
            looking.footnote,
            "Looking for a vehicle on USB, SiK radio, Wi\u{2011}Fi…"
        );
        assert_eq!(
            offline(&[], &[], |_| false).footnote,
            "Automatic connection is off."
        );
        let idle = offline(&[link("Drone", false, "", "")], &["USB"], |_| false);
        assert_eq!(
            (idle.footnote.as_str(), idle.busy),
            ("Looking for a vehicle on USB…", true),
            "saved links do not stop auto-connect from looking"
        );
        let off = offline(&[link("Drone", false, "", "")], &[], |_| false);
        assert_eq!(
            (off.footnote.as_str(), off.busy),
            ("Automatic connection is off.", false)
        );
        let connecting = offline(&[link("Drone", true, "", "")], &all, |name| name == "Drone");
        assert_eq!(
            (connecting.title.as_str(), connecting.busy),
            ("Connecting…", false),
            "a stalled connect stops the spinner"
        );
        assert_eq!(
            connecting.footnote,
            "No telemetry from Drone yet. Check the port is the drone's MAVLink port."
        );
        let refused = offline(
            &[link(
                "Drone",
                false,
                "Reached host but nothing is listening.",
                "editAddress",
            )],
            &all,
            |_| false,
        );
        assert_eq!(
            (refused.title.as_str(), refused.edit_address),
            ("Connection Failed", true)
        );
        assert_eq!(
            refused.footnote,
            "Couldn't connect to Drone. Check its address in Connection Settings."
        );
        let retry = offline(&[link("Drone", false, "timeout", "retry")], &all, |_| false);
        assert!(retry.footnote.ends_with("then tap Retry."));
        assert!(!retry.busy, "a failed link shows its error, not a spinner");
        let dynamic = offline(
            &[json!({ "name": "UDP", "dynamic": true, "connected": true })],
            &all,
            |_| false,
        );
        assert!(
            dynamic.no_links,
            "auto-connect links are not the operator's links"
        );
    }

    #[test]
    fn a_device_that_hears_no_flight_controller_says_so_instead_of_blaming_the_port() {
        let device = json!({ "name": "Aircast zero3w", "dynamic": false, "connected": true, "device": true });
        let waiting = offline(std::slice::from_ref(&device), &["USB"], |_| false);
        assert_eq!(
            (
                waiting.title.as_str(),
                waiting.on_device,
                waiting.no_autopilot,
                waiting.busy
            ),
            ("Connecting…", true, false, true)
        );
        let silent = offline(&[device], &["USB"], |_| true);
        assert_eq!(
            (
                silent.title.as_str(),
                silent.main_status.as_str(),
                silent.busy
            ),
            ("No Flight Controller", "No Flight Controller", false)
        );
        assert!(silent.no_autopilot);
        assert_eq!(
            silent.footnote,
            "Aircast zero3w hears no flight controller. Check it is plugged in and powered on."
        );
        let radio = offline(&[link("Drone", true, "", "")], &["USB"], |_| true);
        assert!(
            !radio.on_device && !radio.no_autopilot,
            "only the device's own link speaks for the device"
        );
    }

    #[test]
    fn only_a_websocket_to_the_paired_device_is_the_devices_link() {
        let socket = |url: &str| json!({ "type": "websocket", "summary": url });
        assert_eq!(
            on_device(socket("ws://zero3w/fly/mavlink"), "zero3w")["device"],
            true
        );
        assert_eq!(
            on_device(socket("wss://localhost:8813/mavlink"), "localhost:8813")["device"],
            true
        );
        assert_eq!(
            on_device(socket("ws://falcon/fly/mavlink"), "zero3w")["device"],
            false
        );
        assert_eq!(
            on_device(socket("ws://zero3w/fly/mavlink"), "")["device"],
            false
        );
        assert_eq!(
            on_device(
                json!({ "type": "udp", "summary": "zero3w:14550" }),
                "zero3w"
            )["device"],
            false
        );
    }

    #[test]
    fn the_toolbar_status_follows_main_status_indicator() {
        let status = |links: &[Value]| offline(links, &["USB"], |_| false).main_status;
        assert_eq!(status(&[]), "Not Connected", "a state, not an instruction");
        assert_eq!(status(&[link("Drone", false, "", "")]), "Not Connected");
        assert_eq!(status(&[link("Drone", true, "", "")]), "Connecting…");
        assert_eq!(
            status(&[link("Drone", false, "timeout", "retry")]),
            "Can't Connect"
        );
        assert_eq!(
            status(&[
                link("Drone", true, "", ""),
                link("Other", false, "timeout", "retry")
            ]),
            "Connecting…",
            "a connecting link outranks a failed one"
        );
    }

    #[test]
    fn a_link_row_names_the_link_once_and_shows_its_error() {
        let row = |name: &str, summary: &str, error: &str| {
            offline_link(
                &json!({ "index": 0, "name": name, "summary": summary, "lastError": error, "connected": false }),
            )
        };
        let named_by_address = row("TCP sitl.aircast.one:5760", "sitl.aircast.one:5760", "");
        assert_eq!(
            (&named_by_address["text"], &named_by_address["description"]),
            (&json!("TCP sitl.aircast.one:5760"), &json!(""))
        );
        assert_eq!(row("Drone", "UDP 14550", "")["description"], "UDP 14550");
        let refused = row("Drone", "TCP 10.0.0.2:5760", "refused");
        assert_eq!(refused["description"], "TCP 10.0.0.2:5760\nrefused");
        assert_eq!(refused["retry"], true);
    }

    #[test]
    fn auto_connect_turned_off_is_not_listed_as_watched() {
        struct Settings;
        impl Backend for Settings {
            fn get(&self, path: &str) -> String {
                let on = !path.contains("autoConnectUDP");
                json!({ "kind": "value", "value": on }).to_string()
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String {
                String::new()
            }
            fn set(&self, _p: &str, _v: &str) -> String {
                String::new()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String {
                String::new()
            }
            fn watch(&self, _p: &[String]) {}
        }
        assert_eq!(
            offline_status_view(&Settings, &[])["footnote"],
            "Looking for a vehicle on USB, SiK radio…",
            "settings are JSON booleans, so false must read as off"
        );
    }

    #[test]
    fn a_connect_stalls_after_ten_seconds_without_a_vehicle() {
        assert!(!connecting_stalled("Drone", 1_000));
        assert!(!connecting_stalled("Drone", 10_999));
        assert!(connecting_stalled("Drone", 11_000));
        assert!(
            !connecting_stalled("Other", 12_000),
            "another link starts its own clock"
        );
        assert!(!connecting_stalled("", 30_000));
    }
}
