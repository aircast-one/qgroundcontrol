use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::read::object;
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
    let mut since = CONNECTING_SINCE.lock().unwrap_or_else(PoisonError::into_inner);
    match (name.is_empty(), since.as_ref()) {
        (true, _) => {
            *since = None;
            false
        }
        (false, Some((current, started))) if current == name => now.saturating_sub(*started) >= OFFLINE_STALL_MS,
        (false, _) => {
            *since = Some((name.to_string(), now));
            false
        }
    }
}

fn auto_connect(backend: &dyn Backend, name: &str) -> bool {
    crate::read::value_number(&backend.get(&format!("settings.autoConnectSettings.{name}.rawValue"))).is_none_or(|v| v != 0.0)
}

pub struct Offline {
    pub title: String,
    pub footnote: String,
    pub busy: bool,
    pub no_links: bool,
    pub edit_address: bool,
}

pub fn offline(links: &[Value], watched: &[&str], stalled_for: impl FnOnce(&str) -> bool) -> Offline {
    let configured: Vec<&Value> = links.iter().filter(|l| l["dynamic"] != true).collect();
    let no_links = configured.is_empty();
    let connecting = configured.iter().find(|l| l["connected"] == true).and_then(|l| l["name"].as_str()).unwrap_or("").to_string();
    let failed = configured.iter().find(|l| l["connected"] != true && l["lastError"].as_str().is_some_and(|e| !e.is_empty()));
    let failed_name = failed.and_then(|l| l["name"].as_str()).unwrap_or("").to_string();
    let edit_address = failed.is_some_and(|l| l["errorRemedy"] == "editAddress");
    let stalled = stalled_for(&connecting);
    let watching = !watched.is_empty();
    let watched_links = watched.join(", ");
    let title = match (connecting.is_empty(), failed_name.is_empty()) {
        (false, _) => "Connecting…",
        (true, false) => "Connection Failed",
        (true, true) => "Not Connected",
    };
    let footnote = match () {
        _ if !connecting.is_empty() && stalled => format!("No telemetry from {connecting} yet. Check the port is the drone's MAVLink port."),
        _ if !connecting.is_empty() => format!("Connecting to {connecting}…"),
        _ if edit_address => format!("Couldn't connect to {failed_name}. Check its address in Connection Settings."),
        _ if !failed_name.is_empty() => format!("Couldn't connect to {failed_name}. Check the drone is powered on and on the same network, then tap it to retry."),
        _ if !watching => "Automatic connection is off.".to_string(),
        _ if no_links => format!("Looking for a vehicle on {watched_links}…"),
        _ => format!("Auto-connect watches {watched_links}. Tap a link to connect."),
    };
    Offline {
        title: title.to_string(),
        footnote,
        busy: (!connecting.is_empty() && !stalled) || (no_links && watching),
        no_links,
        edit_address,
    }
}

pub fn offline_status_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let model = object(&backend.get("links.linkConfigurations"));
    let links: Vec<Value> = model.get("elements").and_then(Value::as_array).map(|e| e.iter().enumerate().map(|(i, el)| crate::links::link_json(i, el)).collect()).unwrap_or_default();
    let watched: Vec<&str> = [("autoConnectPixhawk", "USB"), ("autoConnectSiKRadio", "SiK radio"), ("autoConnectUDP", "Wi\u{2011}Fi")]
        .into_iter()
        .filter(|(setting, _)| auto_connect(backend, setting))
        .map(|(_, name)| name)
        .collect();
    let shown = offline(&links, &watched, |name| connecting_stalled(name, crate::hub::now_ms()));
    json!({
        "kind": "object",
        "class": "OfflineStatus",
        "title": shown.title,
        "footnote": shown.footnote,
        "busy": shown.busy,
        "noLinks": shown.no_links,
        "editAddress": shown.edit_address,
        "links": links.iter().filter(|l| l["dynamic"] != true).map(|l| {
            let error = l["lastError"].as_str().unwrap_or("");
            let summary = l["summary"].as_str().unwrap_or("");
            let connected = l["connected"] == true;
            let name = l["name"].as_str().unwrap_or("");
            json!({
                "index": l["index"],
                "text": if connected { name.to_string() } else { format!("Connect to {name}") },
                "description": if error.is_empty() { summary.to_string() } else { format!("{summary}\n{error}") },
                "retry": !error.is_empty() && !connected,
                "failed": !error.is_empty(),
                "connected": connected,
            })
        }).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(name: &str, connected: bool, error: &str, remedy: &str) -> Value {
        json!({ "name": name, "dynamic": false, "connected": connected, "lastError": error, "errorRemedy": if remedy.is_empty() { Value::Null } else { json!(remedy) } })
    }

    #[test]
    fn the_footnote_follows_the_offline_page() {
        let all = ["USB", "SiK radio", "Wi\u{2011}Fi"];
        let looking = offline(&[], &all, |_| false);
        assert_eq!((looking.title.as_str(), looking.busy, looking.no_links), ("Not Connected", true, true));
        assert_eq!(looking.footnote, "Looking for a vehicle on USB, SiK radio, Wi\u{2011}Fi…");
        assert_eq!(offline(&[], &[], |_| false).footnote, "Automatic connection is off.");
        let idle = offline(&[link("Drone", false, "", "")], &["USB"], |_| false);
        assert_eq!((idle.footnote.as_str(), idle.busy), ("Auto-connect watches USB. Tap a link to connect.", false));
        let connecting = offline(&[link("Drone", true, "", "")], &all, |name| name == "Drone");
        assert_eq!((connecting.title.as_str(), connecting.busy), ("Connecting…", false), "a stalled connect stops the spinner");
        assert_eq!(connecting.footnote, "No telemetry from Drone yet. Check the port is the drone's MAVLink port.");
        let refused = offline(&[link("Drone", false, "Reached host but nothing is listening.", "editAddress")], &all, |_| false);
        assert_eq!((refused.title.as_str(), refused.edit_address), ("Connection Failed", true));
        assert_eq!(refused.footnote, "Couldn't connect to Drone. Check its address in Connection Settings.");
        let retry = offline(&[link("Drone", false, "timeout", "retry")], &all, |_| false);
        assert!(retry.footnote.ends_with("then tap it to retry."));
        let dynamic = offline(&[json!({ "name": "UDP", "dynamic": true, "connected": true })], &all, |_| false);
        assert!(dynamic.no_links, "auto-connect links are not the operator's links");
    }

    #[test]
    fn a_connect_stalls_after_ten_seconds_without_a_vehicle() {
        assert!(!connecting_stalled("Drone", 1_000));
        assert!(!connecting_stalled("Drone", 10_999));
        assert!(connecting_stalled("Drone", 11_000));
        assert!(!connecting_stalled("Other", 12_000), "another link starts its own clock");
        assert!(!connecting_stalled("", 30_000));
    }
}
