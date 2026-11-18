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
    let sign_in = failed.is_some_and(|l| l["errorRemedy"] == "signIn");
    let stalled = stalled_for(&connecting);
    let on_device = connected.is_some_and(|l| l["device"] == true);
    let answered = connected.is_some_and(|l| l["type"] != "udp");
    let no_autopilot = stalled && on_device && answered;
    let watching = !watched.is_empty();
    let watched_links = watched.join(", ");
    let state = |failed: &'static str| match (
        no_autopilot,
        connecting.is_empty(),
        stalled,
        failed_name.is_empty(),
    ) {
        (true, _, _, _) => "No Flight Controller",
        (false, false, true, _) => "No Telemetry",
        (false, false, false, _) => "Connecting…",
        (false, true, _, false) => failed,
        (false, true, _, true) => "Not Connected",
    };
    let (title, main_status) = (state("Connection Failed"), state("Can't Connect"));
    let footnote = match () {
        _ if no_autopilot => format!(
            "{connecting} hears no flight controller. Check it is plugged in and powered on."
        ),
        _ if !connecting.is_empty() && stalled && on_device => format!(
            "No telemetry from {connecting} yet. Check the drone is powered on and the device hears its flight controller."
        ),
        _ if !connecting.is_empty() && stalled => format!(
            "No telemetry from {connecting} yet. Check the port is the drone's MAVLink port."
        ),
        _ if !connecting.is_empty() => format!("Connecting to {connecting}…"),
        _ if sign_in => format!("Sign in to your Aircast account to connect {failed_name}."),
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
    let bare = crate::devicesetup::bare_host(device);
    let reaches = !device.is_empty()
        && match link["type"].as_str() {
            Some("websocket") => link["summary"]
                .as_str()
                .is_some_and(|url| crate::deviceviews::host_of(url) == device),
            Some("udp") => link["hostList"].as_array().is_some_and(|hosts| {
                hosts
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|host| crate::devicesetup::bare_host(host) == bare)
            }),
            Some("tcp") => link["host"] == bare,
            _ => false,
        };
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

fn watched_links(on: impl Fn(&str) -> bool, browser: bool) -> Vec<&'static str> {
    [
        ("autoConnectPixhawk", "USB"),
        ("autoConnectSiKRadio", "SiK radio"),
        ("autoConnectUDP", "Wi\u{2011}Fi"),
    ]
    .into_iter()
    .filter(|(setting, _)| !(browser && *setting == "autoConnectUDP"))
    .filter(|(setting, _)| on(setting))
    .map(|(_, name)| name)
    .collect()
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
    let watched = watched_links(
        |setting| auto_connect(backend, setting),
        cfg!(target_family = "wasm"),
    );
    let shown = offline(&links, &watched, |name| {
        connecting_stalled(name, crate::hub::now_ms())
    });
    let (account_status, sign_in_url) = crate::account::sign_in_progress();
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
        "links": links.iter().filter(|l| l["dynamic"] != true).map(|l| offline_link(l, &account_status)).collect::<Vec<_>>(),
        "signInUrl": sign_in_url,
    })
}

fn row_action(connected: bool, error: &str, remedy: &str) -> (&'static str, &'static str) {
    match (connected, error.is_empty(), remedy) {
        (true, _, _) => ("", ""),
        (false, true, _) => ("connect", "Connect"),
        (false, false, "signIn") => ("signIn", "Sign in"),
        (false, false, "editAddress") => ("edit", "Edit"),
        (false, false, _) => ("retry", "Retry"),
    }
}

fn offline_link(link: &Value, account_status: &str) -> Value {
    let error = link["lastError"].as_str().unwrap_or("");
    let summary = link["summary"].as_str().unwrap_or("");
    let connected = link["connected"] == true;
    let name = link["name"].as_str().unwrap_or("");
    let address = if name.contains(summary) { "" } else { summary };
    let (action, action_label) =
        row_action(connected, error, link["errorRemedy"].as_str().unwrap_or(""));
    let progress = if action == "signIn" {
        account_status
    } else {
        ""
    };
    let description = [address, error, progress]
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    json!({
        "index": link["index"],
        "text": name,
        "description": description,
        "action": action,
        "actionLabel": action_label,
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
    fn a_browser_never_says_it_looks_on_wi_fi_since_it_cannot_listen_for_udp() {
        assert_eq!(
            watched_links(|_| true, false),
            ["USB", "SiK radio", "Wi\u{2011}Fi"]
        );
        assert_eq!(watched_links(|_| true, true), ["USB", "SiK radio"]);
        assert_eq!(
            watched_links(|setting| setting != "autoConnectSiKRadio", true),
            ["USB"]
        );
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
            ("No Telemetry", false),
            "a stalled connect stops the spinner and stops saying it is connecting"
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
        let signed_out = offline(
            &[link(
                "Aircast cloud",
                false,
                "Sign in to your Aircast account to use the cloud backup link.",
                "signIn",
            )],
            &all,
            |_| false,
        );
        assert_eq!(
            signed_out.footnote, "Sign in to your Aircast account to connect Aircast cloud.",
            "the footnote does not tell the pilot to tap Retry when the row offers Sign in"
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
    fn a_silent_udp_link_to_the_device_points_at_the_device_not_the_port() {
        let device = json!({ "name": "Aircast 100.64.0.18", "type": "udp", "dynamic": false, "connected": true, "device": true });
        let silent = offline(&[device], &["USB"], |_| true);
        assert_eq!(
            (silent.title.as_str(), silent.on_device, silent.no_autopilot),
            ("No Telemetry", true, false),
            "UDP never proves the device answered, so it cannot claim the device hears no flight controller"
        );
        assert_eq!(
            silent.footnote,
            "No telemetry from Aircast 100.64.0.18 yet. Check the drone is powered on and the device hears its flight controller."
        );
    }

    #[test]
    fn only_a_link_that_calls_the_paired_device_is_the_devices_link() {
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
        let udp = |hosts: Value| json!({ "type": "udp", "hostList": hosts });
        assert_eq!(
            on_device(udp(json!(["100.64.0.18:14550"])), "100.64.0.18")["device"],
            true,
            "the telemetry link device setup made calls the device"
        );
        assert_eq!(
            on_device(udp(json!(["10.0.0.9:14550"])), "100.64.0.18")["device"],
            false
        );
        assert_eq!(
            on_device(udp(Value::Null), "zero3w")["device"],
            false,
            "a listening UDP link reaches no one in particular"
        );
        assert_eq!(
            on_device(json!({ "type": "tcp", "host": "zero3w" }), "zero3w:8080")["device"],
            true
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
                "",
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
        assert_eq!(
            (&refused["action"], &refused["actionLabel"]),
            (&json!("retry"), &json!("Retry"))
        );
    }

    #[test]
    fn a_link_row_offers_the_action_that_fixes_its_failure() {
        let row = |error: &str, remedy: &str, connected: bool| {
            let shown = offline_link(
                &json!({ "index": 0, "name": "Aircast pi4 (cloud)", "summary": "", "lastError": error, "errorRemedy": remedy, "connected": connected }),
                "Approve code ABCD in the browser to sign in.",
            );
            (
                shown["action"].as_str().unwrap_or("").to_string(),
                shown["actionLabel"].as_str().unwrap_or("").to_string(),
                shown["description"].as_str().unwrap_or("").to_string(),
            )
        };
        assert_eq!(
            row("Sign in to your Aircast account to use the cloud backup link.", "signIn", false),
            (
                "signIn".into(),
                "Sign in".into(),
                "Sign in to your Aircast account to use the cloud backup link.\nApprove code ABCD in the browser to sign in.".into()
            ),
            "Retry can never fix a missing sign-in, and the sign-in's progress shows on the row"
        );
        assert_eq!(
            row(
                "Reached host but nothing is listening.",
                "editAddress",
                false
            )
            .0,
            "edit"
        );
        assert_eq!(row("", "", false).0, "connect");
        assert_eq!(row("", "", true).0, "", "a connected row has no button");
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
