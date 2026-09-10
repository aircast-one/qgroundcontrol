use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["links.linkConfigurations"];

fn kind(settings_url: &str) -> &'static str {
    match settings_url {
        "TcpSettings.qml" => "tcp",
        "UdpSettings.qml" => "udp",
        "SerialSettings.qml" => "serial",
        "BluetoothSettings.qml" => "bluetooth",
        "LogReplaySettings.qml" => "logReplay",
        _ => "other",
    }
}

fn type_label(kind: &str, title: &str) -> String {
    match kind {
        "tcp" => "TCP".to_string(),
        "udp" => "UDP".to_string(),
        "serial" => "Serial".to_string(),
        "bluetooth" => "Bluetooth".to_string(),
        "logReplay" => "Log Replay".to_string(),
        _ => strip_title(title),
    }
}

fn strip_title(title: &str) -> String {
    title.strip_suffix(" Link Settings").or_else(|| title.strip_suffix(" Settings")).unwrap_or(title).to_string()
}

fn editing(kind: &str) -> &'static str {
    match kind {
        "tcp" => "hostAndPort",
        "udp" => "portOnly",
        "serial" => "serial",
        "logReplay" => "logFile",
        _ => "none",
    }
}

pub fn link_json(index: usize, element: &Value) -> Value {
    let text = |key: &str| element.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let flag = |key: &str| element.get(key).and_then(Value::as_bool).unwrap_or(false);
    let number = |key: &str| element.get(key).and_then(Value::as_i64);
    let kind = kind(&text("settingsURL"));
    let connected = element.get("children").and_then(Value::as_array).map(|c| c.iter().any(|v| v.as_str() == Some("link"))).unwrap_or(false);
    let (name, summary, host, filename, last_error) = (text("name"), text("summary"), text("host"), text("filename"), text("lastError"));
    let display_summary = match (kind, host.is_empty(), filename.is_empty()) {
        ("tcp", true, _) => "No host set".to_string(),
        ("logReplay", _, true) => "No log chosen".to_string(),
        _ => summary.clone(),
    };
    // Binding a UDP socket cannot fail, so an open link says nothing about whether anything is
    // on the other end. Only a decoded MAVLink packet does, which is what heardVehicle carries.
    let heard = flag("heardVehicle");
    let state = match (connected, heard) {
        (false, _) => "Not connected",
        (true, false) => "Waiting for the vehicle",
        (true, true) => "Connected",
    };
    let detail = summary.trim();
    let status_line = if detail.is_empty() || name.contains(detail) { state.to_string() } else { format!("{state} \u{b7} {detail}") };
    json!({
        "index": index,
        "path": format!("links.linkConfigurations.{index}"),
        "name": name,
        "type": kind,
        "typeLabel": type_label(kind, &text("settingsTitle")),
        "editing": editing(kind),
        "summary": summary,
        "displaySummary": display_summary,
        "statusLine": status_line,
        "connected": connected,
        "heardVehicle": heard,
        "autoConnect": flag("autoConnect"),
        "dynamic": flag("dynamic"),
        "host": host,
        "port": number("port").or_else(|| number("localPort")).unwrap_or(0),
        "portName": text("portName"),
        "baud": number("baud").unwrap_or(0),
        "filename": filename,
        "logFileName": filename.rsplit('/').next().unwrap_or("").to_string(),
        "lastError": last_error,
    })
}

pub fn links_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let model = object(&backend.get("links.linkConfigurations"));
    let root = object(&backend.get_fields("links", "linkTypeStrings,serialBaudRates"));
    let links: Vec<Value> = model.get("elements").and_then(Value::as_array).map(|e| e.iter().enumerate().map(|(i, el)| link_json(i, el)).collect()).unwrap_or_default();
    let configured: Vec<Value> = links.iter().filter(|l| l["dynamic"] == false).cloned().collect();
    json!({
        "kind": "object",
        "class": "Links",
        "available": model.get("kind").and_then(Value::as_str) == Some("object"),
        "links": links,
        "configured": configured,
        "linkTypes": root.get("linkTypeStrings").cloned().unwrap_or(json!([])),
        "baudRates": root.get("serialBaudRates").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().and_then(|s| s.parse::<i64>().ok())).collect::<Vec<_>>()).unwrap_or_default(),
    })
}

pub fn link_form_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let arg = |i: usize| args.get(i).cloned().unwrap_or_default();
    let (kind, host, port) = (arg(0).to_lowercase(), arg(1), arg(2));
    let parsed = port.parse::<i64>().ok().filter(|p| (1..=65535).contains(p));
    let error = match (parsed, kind.as_str(), host.trim().is_empty()) {
        (None, _, _) => Some("Port must be a number between 1 and 65535."),
        (Some(_), "tcp", true) => Some("A TCP link needs the address of the device to call."),
        _ => None,
    };
    let name = match host.trim().is_empty() {
        true => format!("{} {port}", kind.to_uppercase()),
        false => format!("{} {}:{port}", kind.to_uppercase(), host.trim()),
    };
    json!({ "kind": "object", "class": "LinkForm", "type": kind, "name": name, "valid": error.is_none(), "error": error.unwrap_or("") })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_configuration_is_classified_by_its_settings_page() {
        let tcp = link_json(0, &json!({ "name": "Ground", "settingsURL": "TcpSettings.qml", "settingsTitle": "TCP Link Settings", "summary": "", "host": "", "port": 5760, "children": ["link"] }));
        assert_eq!(tcp["type"], "tcp");
        assert_eq!(tcp["typeLabel"], "TCP");
        assert_eq!(tcp["editing"], "hostAndPort");
        assert_eq!(tcp["displaySummary"], "No host set");
        assert_eq!(tcp["connected"], true);
        assert_eq!(tcp["heardVehicle"], false);
        assert_eq!(tcp["statusLine"], "Waiting for the vehicle");
        let heard = link_json(0, &json!({ "name": "Ground", "settingsURL": "TcpSettings.qml", "summary": "", "children": ["link"], "heardVehicle": true }));
        assert_eq!(heard["statusLine"], "Connected");
        let silent = link_json(0, &json!({ "name": "Ground", "settingsURL": "TcpSettings.qml", "summary": "", "children": [], "heardVehicle": true }));
        assert_eq!(silent["statusLine"], "Not connected");
        let udp = link_json(1, &json!({ "name": "UDP 14550", "settingsURL": "UdpSettings.qml", "summary": "14550", "localPort": 14550, "children": [] }));
        assert_eq!(udp["port"], 14550);
        assert_eq!(udp["statusLine"], "Not connected");
        let other = link_json(2, &json!({ "name": "wfb", "settingsURL": "WfbSettings.qml", "settingsTitle": "Packet Radio Settings", "summary": "2.4 GHz", "children": [] }));
        assert_eq!(other["typeLabel"], "Packet Radio");
        assert_eq!(other["statusLine"], "Not connected \u{b7} 2.4 GHz");
        let replay = link_json(3, &json!({ "name": "Replay", "settingsURL": "LogReplaySettings.qml", "filename": "/tmp/flight.tlog", "children": [] }));
        assert_eq!(replay["logFileName"], "flight.tlog");
    }

    #[test]
    fn the_form_validates_the_way_android_does_and_names_the_link() {
        let bad_port = link_form_view(&Nothing, &["udp".into(), "".into(), "70000".into()]);
        assert_eq!(bad_port["valid"], false);
        let no_host = link_form_view(&Nothing, &["tcp".into(), "".into(), "5760".into()]);
        assert!(no_host["error"].as_str().unwrap().contains("TCP"));
        let good = link_form_view(&Nothing, &["tcp".into(), "10.0.0.2".into(), "5760".into()]);
        assert_eq!(good["valid"], true);
        assert_eq!(good["name"], "TCP 10.0.0.2:5760");
        assert_eq!(link_form_view(&Nothing, &["udp".into(), "".into(), "14550".into()])["name"], "UDP 14550");
    }

    struct Nothing;
    impl Backend for Nothing {
        fn get(&self, _p: &str) -> String { String::new() }
        fn get_fields(&self, _p: &str, _f: &str) -> String { String::new() }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }
}
