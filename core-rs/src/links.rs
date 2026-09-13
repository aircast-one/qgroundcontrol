use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["links.linkConfigurations", "vehicle.vehicleLinkManager.communicationLostEnabled", "vehicle.vehicleLinkManager.linkNames", "vehicle.vehicleLinkManager.linkStatuses"];

fn kind(settings_url: &str) -> &'static str {
    match settings_url {
        "TcpSettings.qml" => "tcp",
        "UdpSettings.qml" => "udp",
        "SerialSettings.qml" => "serial",
        "BluetoothSettings.qml" => "bluetooth",
        "LogReplaySettings.qml" => "logReplay",
        "MockLinkSettings.qml" => "mock",
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

pub fn quiet_links(backend: &dyn Backend) -> Vec<String> {
    let manager = object(&backend.get_fields("vehicle.vehicleLinkManager", "communicationLostEnabled,linkNames,linkStatuses"));
    if manager.get("communicationLostEnabled").and_then(Value::as_bool) != Some(true) {
        return Vec::new();
    }
    let strings = |key: &str| {
        manager.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect::<Vec<_>>()).unwrap_or_default()
    };
    let (names, statuses) = (strings("linkNames"), strings("linkStatuses"));
    names
        .iter()
        .enumerate()
        .filter(|(index, _)| statuses.get(*index).is_some_and(|status| !status.is_empty()))
        .map(|(_, name)| name.clone())
        .collect()
}

pub fn link_json(index: usize, element: &Value) -> Value {
    link_json_with(index, element, &[])
}

pub fn link_json_with(index: usize, element: &Value, quiet: &[String]) -> Value {
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
    let heard = flag("heardVehicle");
    let gone_quiet = quiet.iter().any(|quiet_name| *quiet_name == name);
    let state = match (connected, heard, gone_quiet) {
        (false, _, _) => "Not connected",
        (true, false, _) => "Waiting for the vehicle",
        (true, true, true) => "Not hearing the vehicle",
        (true, true, false) => "Connected",
    };
    let detail = display_summary.trim();
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
        "goneQuiet": gone_quiet,
        "autoConnect": flag("autoConnect"),
        "dynamic": flag("dynamic"),
        "host": host,
        "port": number("port").or_else(|| number("localPort")),
        "portName": text("portName"),
        "baud": number("baud"),
        "filename": filename,
        "logFileName": filename.rsplit('/').next().unwrap_or("").to_string(),
        "lastError": last_error.clone(),
        // MainStatusIndicatorOfflinePage.qml:47 branches on this to send the operator to edit the
        // address instead of offering Retry, and TCPLink raises it for three cases - no address,
        // host not found, nothing listening. Serving the sentence without it tells someone their
        // link failed and lets them retry a configuration that cannot succeed until it is changed.
        "errorRemedy": match (last_error.is_empty(), number("lastErrorRemedy")) {
            (true, _) => Value::Null,
            (false, Some(1)) => json!("editAddress"),
            (false, _) => json!("retry"),
        },
    })
}

pub fn links_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let model = object(&backend.get("links.linkConfigurations"));
    let root = object(&backend.get_fields("links", "linkTypeStrings,linkTypeIds,serialBaudRates"));
    let quiet = quiet_links(backend);
    let links: Vec<Value> = model.get("elements").and_then(Value::as_array).map(|e| e.iter().enumerate().map(|(i, el)| link_json_with(i, el, &quiet)).collect()).unwrap_or_default();
    let configured: Vec<Value> = links.iter().filter(|l| l["dynamic"] == false).cloned().collect();
    json!({
        "kind": "object",
        "class": "Links",
        "available": model.get("kind").and_then(Value::as_str) == Some("object"),
        "links": links,
        "configured": configured,
        "linkTypes": root.get("linkTypeStrings").cloned().unwrap_or(json!([])),
        "linkTypeIds": root.get("linkTypeIds").cloned().unwrap_or(json!([])),
        "baudRates": root.get("serialBaudRates").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().and_then(|s| s.parse::<i64>().ok())).collect::<Vec<_>>()).unwrap_or_default(),
    })
}

fn serial_baud_rates(backend: &dyn Backend) -> Vec<i64> {
    object(&backend.get_fields("links", "serialBaudRates")).get("serialBaudRates").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().and_then(|s| s.parse::<i64>().ok()).or_else(|| v.as_i64())).collect()).unwrap_or_default()
}

pub fn link_form_view(backend: &dyn Backend, args: &[String]) -> Value {
    let arg = |i: usize| args.get(i).cloned().unwrap_or_default();
    let (kind, host, port) = (arg(0).to_lowercase(), arg(1), arg(2));
    let serial = kind == "serial";
    let rates = serial_baud_rates(backend);
    let number = port.parse::<i64>().ok();
    let ok = match serial {
        true => number.filter(|b| *b > 0 && (rates.is_empty() || rates.contains(b))),
        false => number.filter(|p| (1..=65535).contains(p)),
    };
    let error = match (ok, serial, kind.as_str(), host.trim().is_empty()) {
        (None, true, _, _) => Some("Choose one of the rates the radio offers."),
        (None, false, _, _) => Some("Port must be a number between 1 and 65535."),
        (Some(_), true, _, true) => Some("A serial link needs the device to open."),
        (Some(_), _, "tcp", true) => Some("A TCP link needs the address of the device to call."),
        _ => None,
    };
    let name = match (serial, host.trim().is_empty()) {
        (true, _) => format!("{} {}", host.trim(), port).trim().to_string(),
        (false, true) => format!("{} {port}", kind.to_uppercase()),
        (false, false) => format!("{} {}:{port}", kind.to_uppercase(), host.trim()),
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
        assert_eq!(tcp["statusLine"], "Waiting for the vehicle \u{b7} No host set", "the line took the RAW summary while displaySummary held the words that replace it, so a TCP link with no host read as a bare colon and a port with nothing in front of it");
        assert_eq!(tcp["connected"], true);
        assert_eq!(tcp["heardVehicle"], false);
        let heard = link_json(0, &json!({ "name": "Ground", "settingsURL": "TcpSettings.qml", "summary": "", "host": "10.0.0.2", "children": ["link"], "heardVehicle": true }));
        let quiet = link_json_with(
            0,
            &json!({ "name": "Ground", "settingsURL": "TcpSettings.qml", "summary": "", "host": "10.0.0.2", "children": ["link"], "heardVehicle": true }),
            &["Ground".to_string()],
        );
        assert_eq!(quiet["statusLine"], "Not hearing the vehicle", "heardVehicle latches on the first packet ever decoded, so a radio that has gone silent still reads Connected unless the roster is consulted");
        assert_eq!(quiet["goneQuiet"], true);
        assert_eq!(heard["goneQuiet"], false, "a link nothing reported quiet is not quiet");
        assert_eq!(heard["statusLine"], "Connected", "a link with a host and nothing to add says only where it stands");
        let silent = link_json(0, &json!({ "name": "Ground", "settingsURL": "TcpSettings.qml", "summary": "", "host": "10.0.0.2", "children": [], "heardVehicle": true }));
        assert_eq!(silent["statusLine"], "Not connected");
        let udp = link_json(1, &json!({ "name": "UDP 14550", "settingsURL": "UdpSettings.qml", "summary": "14550", "localPort": 14550, "children": [] }));
        assert_eq!(udp["port"], 14550);
        assert_eq!(udp["statusLine"], "Not connected");
        let other = link_json(2, &json!({ "name": "wfb", "settingsURL": "WfbSettings.qml", "settingsTitle": "Packet Radio Settings", "summary": "2.4 GHz", "children": [] }));
        assert_eq!(other["typeLabel"], "Packet Radio");
        assert_eq!(other["statusLine"], "Not connected \u{b7} 2.4 GHz");
        let replay = link_json(3, &json!({ "name": "Replay", "settingsURL": "LogReplaySettings.qml", "filename": "/tmp/flight.tlog", "children": [] }));
        assert_eq!(replay["logFileName"], "flight.tlog");

        assert_eq!(udp["baud"], Value::Null, "baud is a Q_PROPERTY on SerialLink alone, so a UDP link has none - and a head drawing zero shows a rate rather than an absence");
        assert_eq!(replay["port"], Value::Null, "port is on TCPLink and localPort on UDPLink, so a log replay has neither");
        assert_eq!(replay["baud"], Value::Null);
        assert_eq!(tcp["port"], 5760, "where the property exists the number still travels");
        let serial = link_json(4, &json!({ "name": "Pixhawk", "settingsURL": "SerialSettings.qml", "summary": "", "portName": "/dev/cu.usbmodem1", "baud": 57600, "children": [] }));
        assert_eq!(serial["baud"], 57600);
        assert_eq!(serial["port"], Value::Null, "and a serial link has no network port at all");
    }

    #[test]
    fn a_failed_link_says_whether_retrying_could_ever_work() {
        // variantJson serialises a Q_ENUM as its integer (QGCBridgeCore.cc), and ErrorRemedy is
        // declared RemedyRetry then RemedyEditAddress, so the wire value is 0 or 1.
        let link = |error: &str, remedy: Value| {
            let mut element = json!({ "name": "Ground", "settingsURL": "TcpSettings.qml", "host": "192.168.1.5", "port": 5760 });
            element["lastError"] = json!(error);
            if !remedy.is_null() { element["lastErrorRemedy"] = remedy; }
            link_json(0, &element)
        };

        let unreachable = link("Can't find 192.168.1.5 on this network.", json!(1));
        assert_eq!(unreachable["errorRemedy"], "editAddress", "TCPLink raises RemedyEditAddress for no address, host not found and nothing listening; serving only the sentence leaves an operator retrying a configuration that cannot succeed until it is changed");
        assert_eq!(unreachable["lastError"], "Can't find 192.168.1.5 on this network.", "and the sentence still travels beside it");

        let busy = link("Serial port is already open.", json!(0));
        assert_eq!(busy["errorRemedy"], "retry", "a remedy the core does not recognise is a retry, because that is the enum's own default and the wrong guess costs one tap");

        let unreported = link("Something went wrong.", Value::Null);
        assert_eq!(unreported["errorRemedy"], "retry");

        let healthy = link("", json!(1));
        assert_eq!(healthy["errorRemedy"], Value::Null, "a link with no error has no remedy to offer, and a stale remedy beside an empty error would draw a fix-this prompt on a working link");
        assert_eq!(healthy["lastError"], "");
    }

    #[test]
    fn the_type_list_carries_an_id_beside_the_word_an_operator_reads() {
        struct Types;
        impl Backend for Types {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "links" => json!({ "kind": "object", "linkTypeStrings": ["Seriell", "UDP", "TCP"], "linkTypeIds": ["serial", "udp", "tcp"], "serialBaudRates": ["57600", "115200"] }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = links_view(&Types, &[]);
        assert_eq!(view["linkTypeIds"][0], "serial", "LinkManager::linkTypeStrings is a list of tr() calls filled on first call, after the translator is installed - so a head deciding whether to offer Port and Baud by comparing the picked type against \"Serial\" could not configure a serial link at all in any other locale");
        assert_eq!(view["linkTypes"][0], "Seriell", "the word an operator reads still travels, and the index still means what createConfiguration expects");
        assert_eq!(view["linkTypeIds"].as_array().unwrap().len(), view["linkTypes"].as_array().unwrap().len(), "both come from one table in LinkManager, so they cannot fall out of step or out of order");
    }

    #[test]
    fn the_form_validates_the_way_android_does_and_names_the_link() {
        struct Rates;
        impl Backend for Rates {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "links" => json!({ "kind": "object", "serialBaudRates": ["57600", "115200"] }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let bad_port = link_form_view(&Nothing, &["udp".into(), "".into(), "70000".into()]);
        assert_eq!(bad_port["valid"], false);
        let no_host = link_form_view(&Nothing, &["tcp".into(), "".into(), "5760".into()]);
        assert!(no_host["error"].as_str().unwrap().contains("TCP"));
        let good = link_form_view(&Nothing, &["tcp".into(), "10.0.0.2".into(), "5760".into()]);
        assert_eq!(good["valid"], true);
        assert_eq!(good["name"], "TCP 10.0.0.2:5760");
        assert_eq!(link_form_view(&Nothing, &["udp".into(), "".into(), "14550".into()])["name"], "UDP 14550");
        let serial = link_form_view(&Rates, &["serial".into(), "/dev/cu.usbmodem1".into(), "57600".into()]);
        assert_eq!(serial["valid"], true, "qgc_links_create builds a serial link with the device in host and the baud in port, so the form has to accept one");
        assert_eq!(serial["name"], "/dev/cu.usbmodem1 57600", "a serial link is a device at a rate, never an address at a port");
        let fast = link_form_view(&Rates, &["serial".into(), "/dev/cu.usbmodem1".into(), "115200".into()]);
        assert_eq!(fast["valid"], true, "115200 is a rate the radio offers and not a port at all - a port range would refuse it for being over 65535");
        let odd = link_form_view(&Rates, &["serial".into(), "/dev/cu.usbmodem1".into(), "57601".into()]);
        assert_eq!(odd["valid"], false, "57601 sits inside the port range, so only the offered rates can tell it apart from 57600");
        let no_device = link_form_view(&Rates, &["serial".into(), "".into(), "57600".into()]);
        assert_eq!(no_device["valid"], false, "a serial link with no device names nothing to open");
        assert_eq!(link_form_view(&Nothing, &["serial".into(), "/dev/cu.usbmodem1".into(), "460800".into()])["valid"], true, "an unavailable rate list is not evidence the rate is wrong");
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
