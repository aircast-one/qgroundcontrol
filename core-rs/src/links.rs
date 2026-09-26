use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["links.mavlinkSupportForwardingEnabled", "links.linkConfigurations", "vehicle.vehicleLinkManager.communicationLostEnabled", "vehicle.vehicleLinkManager.linkNames", "vehicle.vehicleLinkManager.linkStatuses", "links.serialPorts", "links.serialPortStrings"];

pub(crate) fn kind(settings_url: &str) -> &'static str {
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

// LinkManager serves the ports and their display strings as two parallel lists. LinksScreen.kt
// dropped blank ports BEFORE pairing them with labels by position, so one blank entry shifted every
// label after it onto the wrong port. They are paired first here, and a blank label falls back to
// the port itself.
pub(crate) fn serial_ports(ports: Option<&Value>, labels: Option<&Value>) -> Vec<Value> {
    let texts = |list: Option<&Value>| list.and_then(Value::as_array).map(|a| a.iter().map(|v| v.as_str().unwrap_or_default().to_string()).collect::<Vec<_>>()).unwrap_or_default();
    let labels = texts(labels);
    texts(ports)
        .into_iter()
        .enumerate()
        .filter(|(_, port)| !port.trim().is_empty())
        .map(|(i, port)| {
            let label = labels.get(i).filter(|l| !l.trim().is_empty()).cloned().unwrap_or_else(|| port.clone());
            json!({ "port": port, "label": label })
        })
        .collect()
}

pub fn links_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let model = object(&backend.get("links.linkConfigurations"));
    let root = object(&backend.get_fields("links", "linkTypeStrings,linkTypeIds,serialBaudRates,mavlinkSupportForwardingEnabled,serialPorts,serialPortStrings"));
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
        "supportForwarding": crate::read::flag(&root, "mavlinkSupportForwardingEnabled"),
        "linkTypeIds": root.get("linkTypeIds").cloned().unwrap_or(json!([])),
        "serialPorts": serial_ports(root.get("serialPorts"), root.get("serialPortStrings")),
        "baudRates": root.get("serialBaudRates").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().and_then(|s| s.parse::<i64>().ok())).collect::<Vec<_>>()).unwrap_or_default(),
    })
}

pub(crate) fn serial_baud_rates(backend: &dyn Backend) -> Vec<i64> {
    object(&backend.get_fields("links", "serialBaudRates")).get("serialBaudRates").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().and_then(|s| s.parse::<i64>().ok()).or_else(|| v.as_i64())).collect()).unwrap_or_default()
}

pub fn support_host_view(_backend: &dyn Backend, args: &[String]) -> Value {
    // NOT trimmed. LinkManager.cc:942 hands the stored value to addHost exactly as typed, and
    // _getIpAddress cannot resolve a name with a leading space - so trimming here judged a string
    // QGC never uses and answered valid for an address that forwards nowhere. A normalisation on
    // one side of a boundary and not the other is a disagreement no test inside either side finds.
    let typed = args.first().map(String::as_str).unwrap_or_default();
    let parts: Vec<&str> = typed.split(':').collect();
    let error = match (typed.trim().is_empty(), parts.len()) {
        (true, _) => Some("Enter the address of the support engineer's ground station."),
        // A view's arguments are split on commas, so a typed comma arrives here as a second
        // argument and the first half alone could answer valid. The caller cannot tell this
        // happened; the view can, because it is the only thing that sees the extra argument.
        _ if args.len() > 1 => Some("An address cannot contain a comma."),
        _ if parts[0].chars().any(char::is_whitespace) => Some("An address cannot contain a space."),
        (false, 1) => None,
        (false, 2) if parts[0].is_empty() => Some("An address is needed before the colon, or the link's own port is used without one."),
        (false, 2) => match parts[1].parse::<u32>().ok().filter(|port| (1..=65535).contains(port)) {
            Some(_) => None,
            None => Some("The part after the colon has to be a port number between 1 and 65535."),
        },
        _ => Some("An address with more than one colon is refused, so an IPv6 literal has to be given without a port."),
    };
    json!({ "kind": "object", "class": "SupportHost", "valid": error.is_none(), "error": error.unwrap_or("") })
}

pub(crate) fn link_type_ids(backend: &dyn Backend) -> Vec<String> {
    object(&backend.get_fields("links", "linkTypeIds"))
        .get("linkTypeIds")
        .and_then(Value::as_array)
        .map(|ids| ids.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}

pub(crate) fn form_error(kind: &str, host: &str, ok: Option<i64>, known: &[String]) -> Option<(&'static str, &'static str)> {
    let unknown_type = !known.is_empty() && !known.iter().any(|k| k == kind);
    match (ok, kind == "serial", kind, host.trim().is_empty()) {
        _ if unknown_type => Some(("type", "Choose one of the link types this build offers.")),
        (None, true, _, _) => Some(("port", "Choose one of the rates the radio offers.")),
        (None, false, _, _) => Some(("port", "Port must be a number between 1 and 65535.")),
        (Some(_), true, _, true) => Some(("host", "A serial link needs the device to open.")),
        (Some(_), _, "tcp", true) => Some(("host", "A TCP link needs the address of the device to call.")),
        _ => None,
    }
}

pub(crate) fn port_ok(kind: &str, port: Option<i64>, rates: &[i64]) -> Option<i64> {
    match kind == "serial" {
        true => port.filter(|b| *b > 0 && (rates.is_empty() || rates.contains(b))),
        false => port.filter(|p| (1..=65535).contains(p)),
    }
}

pub fn link_form_view(backend: &dyn Backend, args: &[String]) -> Value {
    let arg = |i: usize| args.get(i).cloned().unwrap_or_default();
    let (kind, host, port) = (arg(0).to_lowercase(), arg(1), arg(2));
    let serial = kind == "serial";
    let rates = serial_baud_rates(backend);
    let number = port.parse::<i64>().ok();
    let ok = port_ok(&kind, number, &rates);
    // One flag for two fields left a head no way to tell which one it was about, so a field gated
    // on valid refuses whichever field the operator happens to be editing: an empty TCP host made
    // every port entry fail with a sentence about the host, and an absent port made the host
    // unrepairable with a sentence about the port. errorField names the field so a head gates that
    // one and leaves the other alone. It is never null while valid is false - a check about the
    // pair rather than either field would need its own token, because a head reading null as
    // "nothing is wrong" would gate nothing at all.
    // An unrecognised token was never examined: the host check keys on "tcp" and the rate check on
    // serial, so anything else fell through to None and the form called a link it cannot build
    // fine. linkTypeIds is LinkManager::linkTypeTable(), the same list the type picker is filled
    // from, so the form cannot accept a type the manager could not construct. An empty list
    // refuses nothing - a bridge that cannot answer must not become a validator rejecting every
    // type.
    let error = form_error(&kind, &host, ok, &link_type_ids(backend));
    let name = match (serial, host.trim().is_empty()) {
        (true, _) => format!("{} {}", host.trim(), port).trim().to_string(),
        (false, true) => format!("{} {port}", kind.to_uppercase()),
        (false, false) => format!("{} {}:{port}", kind.to_uppercase(), host.trim()),
    };
    json!({
        "kind": "object",
        "class": "LinkForm",
        "type": kind,
        "name": name,
        "valid": error.is_none(),
        "error": error.map(|(_, text)| text).unwrap_or(""),
        "errorField": error.map(|(field, _)| field),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Types(Vec<&'static str>);
    impl Backend for Types {
        fn get(&self, p: &str) -> String { self.get_fields(p, "") }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "links" => json!({ "kind": "object", "linkTypeIds": self.0, "serialBaudRates": ["57600"] }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_support_host_shipped_as_a_placeholder_is_refused_rather_than_dialled() {
        let judged = |typed: &str| support_host_view(&Nothing, &[typed.to_string()]);

        let shipped = judged("support.ardupilot.org:xxxx");
        assert_eq!(
            (shipped["valid"].clone(), shipped["error"].as_str().is_some_and(|e| !e.is_empty())),
            (json!(false), true),
            "Mavlink.SettingsGroup.json ships this string as the VALUE of the setting, and UDPConfiguration::addHost splits it into two parts so the format check passes, then QString::toUInt returns 0 for xxxx with no ok pointer, then the name resolves so the DNS check passes. Three error paths each decline to fire and the page reports MAVLink is being forwarded to port 0"
        );

        assert_eq!(judged("support.ardupilot.org:14550")["valid"], true);
        assert_eq!(
            judged("support.ardupilot.org")["valid"],
            true,
            "UDPLink.cc:157 falls back to the link's own local port when there is no colon, so refusing a bare host would refuse a configuration QGC accepts"
        );
        assert_eq!(judged("support.ardupilot.org:")["valid"], false, "a trailing colon still splits into two parts, and the empty half is toUInt 0 - the same silent port 0 as the placeholder");
        assert_eq!(judged("support.ardupilot.org:0")["valid"], false, "port 0 typed outright is the destination the placeholder reaches by accident");
        assert_eq!(judged("support.ardupilot.org:70000")["valid"], false);
        assert_eq!(judged("")["valid"], false, "an empty field is not a bare host with a fallback port, it is nothing to dial");

        assert_eq!(
            judged("::1")["valid"],
            false,
            "addHost refuses anything that does not split into exactly two parts, so QGC drops an IPv6 literal with a warning and adds no host at all. A head reading only the LAST colon takes the final group as a port, calls it valid, and the operator is told forwarding is on when no client was ever appended"
        );
        assert_eq!(judged("fe80::1:14550")["valid"], false);

        assert_eq!(
            judged("two words:14550")["valid"],
            false,
            "_getIpAddress cannot resolve an address with a space, so addHost returns at UDPLink.cc:167 without appending a client - the page reports forwarding started and nothing is forwarded. Measured on the Android device against a fresh library, and it is the same defect as the placeholder reaching port 0, arriving through the served verdict instead of a head's own rule"
        );
        assert_eq!(judged("has space.org")["valid"], false, "and with no colon either, because the bare-host path resolves the same name");
        assert_eq!(
            judged("host:14 550")["error"],
            "The part after the colon has to be a port number between 1 and 65535.",
            "the whitespace rule reads only the address half, so a space in the PORT half falls through to the port parse - which refuses it. This is the seam between two rules rather than the inside of either, and it is pinned so neither can be tightened into covering the other's case or loosened into covering neither"
        );
        assert_eq!(
            judged("  padded:14550")["valid"],
            false,
            "this asserted TRUE until the Android session probed it against a fresh library: LinkManager.cc:942 hands the stored value to addHost untrimmed, QHostInfo::fromName cannot resolve a name with a leading space, and UDPLink.cc:166 returns without appending a client. Trimming here judged a string QGC never uses - the verdict has to be about the value that will be dialled, not a cleaned copy of it"
        );
        assert_eq!(
            support_host_view(&Nothing, &["a".to_string(), "b:14550".to_string()])["valid"],
            false,
            "a view's arguments are split on commas, so a typed comma arrives as a second argument and the first half alone would answer valid. The caller cannot see that its string was truncated; this view can, because the extra argument is the evidence"
        );
        assert_eq!(
            judged(":14550")["valid"],
            false,
            "two parts and a port in range, so the split checks pass - but _getIpAddress on an empty string resolves to nothing and addHost returns at UDPLink.cc:167 without appending a client. QGC refuses it one step later than it refuses a bad port, and the question here is whether forwarding will work rather than whether addHost returns"
        );
    }

    #[test]
    fn a_refused_form_says_which_field_is_wrong_rather_than_only_that_something_is() {
        let form = |kind: &str, host: &str, port: &str| link_form_view(&Nothing, &[kind.to_string(), host.to_string(), port.to_string()]);

        let no_host = form("tcp", "", "5760");
        assert_eq!((no_host["valid"].clone(), no_host["errorField"].clone()), (json!(false), json!("host")), "a head gating its PORT field on the form-level flag refuses every port entry while the sentence talks about the host, which is how an operator ends up unable to fix the field the message names");

        let no_port = form("tcp", "127.0.0.1", "");
        assert_eq!((no_port["valid"].clone(), no_port["errorField"].clone()), (json!(false), json!("port")), "and the mirror: a link whose port reads as absent left the HOST uneditable, blocked by a complaint about a field the operator is not touching");

        assert_eq!(form("serial", "", "57600")["errorField"], "host", "a serial link keeps its device path in host, so the field to blame is host even though the sentence says device");
        assert_eq!(form("serial", "/dev/tty.usb", "0")["errorField"], "port", "a rate of zero is the port field's problem - with no radio attached there is no rate list to check against, so this is the only bad rate a fake can present");

        let good = form("tcp", "127.0.0.1", "5760");
        assert_eq!((good["valid"].clone(), good["errorField"].clone()), (json!(true), Value::Null));

        let nonsense = link_form_view(&Types(vec!["serial", "udp", "tcp"]), &["websocket".to_string(), String::new(), "5760".to_string()]);
        assert_eq!(
            (nonsense["valid"].clone(), nonsense["errorField"].clone()),
            (json!(false), json!("type")),
            "an unrecognised token was never examined - the host check keys on tcp and the rate check on serial, so anything else fell through and the form called a link it cannot build valid"
        );
        assert_eq!(
            link_form_view(&Types(vec![]), &["websocket".to_string(), String::new(), "5760".to_string()])["valid"],
            true,
            "with no linkTypeIds to check against nothing is refused for its type: a bridge that cannot answer must not become a validator that rejects every type, which is a default answering for the unasked in the harsher direction"
        );
        assert_eq!(link_form_view(&Types(vec!["serial", "udp", "tcp"]), &["udp".to_string(), String::new(), "14550".to_string()])["valid"], true, "and a type the build does offer still passes");

        [("tcp", "", "5760"), ("tcp", "127.0.0.1", ""), ("tcp", "", ""), ("serial", "", "0"), ("udp", "", "0"), ("udp", "", "70000")]
            .iter()
            .for_each(|(kind, host, port)| {
                let refused = form(kind, host, port);
                assert_ne!(
                    refused["errorField"],
                    Value::Null,
                    "errorField is null only when there is no error. A check about the pair rather than either field would need its own token: a head reading null as nothing-is-wrong would gate nothing at all, which is the failure this field exists to prevent. Refused form: {kind} {host} {port}"
                );
            });
    }

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
        assert_eq!(bad_port["errorField"], "port");
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
    #[test]
    fn the_support_forwarding_flag_travels_with_the_links_view() {
        struct Forwarding(bool);
        impl Backend for Forwarding {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "links" => json!({ "kind": "object", "linkTypeStrings": [], "linkTypeIds": [], "serialBaudRates": [], "mavlinkSupportForwardingEnabled": self.0 }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        assert_eq!(links_view(&Forwarding(true), &[])["supportForwarding"], json!(true), "RemoteSupport.swift read this one field off the raw links object and no view served it, so that page was entirely on Qt for the first thing it needs");
        assert_eq!(links_view(&Forwarding(false), &[])["supportForwarding"], json!(false));
    }


    #[test]
    fn a_serial_port_keeps_its_own_label_when_a_blank_port_is_dropped() {
        let ports = json!(["", "/dev/ttyUSB0", "/dev/ttyACM0"]);
        let labels = json!(["ghost", "FTDI UART", ""]);
        assert_eq!(
            serial_ports(Some(&ports), Some(&labels)),
            vec![json!({ "port": "/dev/ttyUSB0", "label": "FTDI UART" }), json!({ "port": "/dev/ttyACM0", "label": "/dev/ttyACM0" })],
            "filtering before pairing gave /dev/ttyUSB0 the blank port's label and /dev/ttyACM0 the FTDI one"
        );
        assert!(serial_ports(None, None).is_empty());
    }
}
