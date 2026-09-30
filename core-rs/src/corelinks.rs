use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};

use crate::linkconfig::{Defaults, Kind, LinkConfig, TypeTable};

pub const MODEL: &str = "links.linkConfigurations";
const DEFAULT_UDP_PORT: u16 = 14550;
const DEFAULT_TCP_PORT: u16 = 5760;
const DEFAULT_SERIAL_BAUD: i64 = 57600;

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub config: LinkConfig,
    pub dynamic: bool,
}

static ENTRIES: LazyLock<Mutex<Option<Vec<Entry>>>> = LazyLock::new(|| Mutex::new(None));
static HOST_OWNS_LINKS: AtomicBool = AtomicBool::new(false);

pub fn host_owns_links() {
    HOST_OWNS_LINKS.store(true, Ordering::SeqCst);
}

pub fn owned() -> bool {
    !HOST_OWNS_LINKS.load(Ordering::SeqCst)
}

pub fn table() -> TypeTable {
    TypeTable::new(cfg!(not(any(target_os = "ios", target_os = "android"))), cfg!(debug_assertions))
}

fn defaults() -> Defaults {
    let udp_port = crate::settingsstore::raw_setting("settings.autoConnectSettings.udpListenPort").and_then(|v| v.as_u64()).and_then(|p| u16::try_from(p).ok()).unwrap_or(DEFAULT_UDP_PORT);
    Defaults { udp_port, tcp_host: String::new(), tcp_port: DEFAULT_TCP_PORT, serial_baud: DEFAULT_SERIAL_BAUD }
}

fn entries() -> MutexGuard<'static, Option<Vec<Entry>>> {
    ENTRIES.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn listed() -> Vec<Entry> {
    let mut held = entries();
    held.get_or_insert_with(|| {
        let saved = crate::settingsstore::entries_under(crate::linkconfig::ROOT);
        crate::linkconfig::load(&saved, &table(), &defaults()).into_iter().map(|config| Entry { config, dynamic: false }).collect()
    })
    .clone()
}

fn kind_fields(kind: &Kind) -> (&'static str, &'static str, &'static str, String, Value) {
    match kind {
        Kind::Tcp { host, port } => ("TCPConfiguration", "TcpSettings.qml", "TCP Link Settings", format!("{host}:{port}"), json!({ "host": host, "port": port })),
        Kind::Udp { local_port, hosts } => (
            "UDPConfiguration",
            "UdpSettings.qml",
            "UDP Link Settings",
            format!("UDP port {local_port}"),
            json!({ "localPort": local_port, "hostList": hosts.iter().map(|(host, port)| format!("{host}:{port}")).collect::<Vec<_>>() }),
        ),
        Kind::Serial { baud, data_bits, flow_control, stop_bits, parity, port_name, port_display_name } => (
            "SerialConfiguration",
            "SerialSettings.qml",
            "Serial Link Settings",
            format!("{} at {baud} baud", if port_display_name.is_empty() { port_name } else { port_display_name }),
            json!({ "baud": baud, "dataBits": data_bits, "flowControl": flow_control, "stopBits": stop_bits, "parity": parity, "portName": port_name, "portDisplayName": port_display_name }),
        ),
        Kind::LogReplay { file } => ("LogReplayConfiguration", "LogReplaySettings.qml", "Log Replay Link Settings", String::new(), json!({ "filename": file })),
        Kind::AircastCloud { api_base, device_id } => ("AircastCloudConfiguration", "AircastCloudSettings.qml", "Aircast Cloud Link Settings", format!("Aircast cloud \u{b7} {device_id}"), json!({ "apiBase": api_base, "deviceId": device_id })),
        Kind::Bluetooth { device_name, address } => ("BluetoothConfiguration", "BluetoothSettings.qml", "Bluetooth Link Settings", String::new(), json!({ "deviceName": device_name, "address": address })),
        Kind::Mock { firmware_type, vehicle_type, send_status_text, increment_vehicle_id, .. } => (
            "MockConfiguration",
            "MockLinkSettings.qml",
            "Mock Link Settings",
            String::new(),
            json!({ "firmware": firmware_type, "vehicle": vehicle_type, "sendStatus": send_status_text, "incrementVehicleId": increment_vehicle_id }),
        ),
    }
}

pub fn element(entry: &Entry, table: &TypeTable) -> Value {
    let (class, url, title, summary, fields) = kind_fields(&entry.config.kind);
    let code = table.code(crate::linkconfig::kind_of(&entry.config.kind));
    let common = json!({
        "kind": "object",
        "class": class,
        "objectName": "",
        "children": [],
        "facts": [],
        "name": entry.config.name,
        "linkType": code,
        "dynamic": entry.dynamic,
        "autoConnect": entry.config.auto_connect,
        "highLatency": entry.config.high_latency,
        "settingsURL": url,
        "settingsTitle": title,
        "summary": summary,
        "lastError": "",
        "lastErrorRemedy": 0,
        "heardVehicle": false,
        "linkActive": false,
        "link": null,
    });
    let mut merged = common;
    fields.as_object().into_iter().flatten().for_each(|(k, v)| merged[k.as_str()] = v.clone());
    merged
}

pub fn model() -> Value {
    let table = table();
    let elements: Vec<Value> = listed().iter().map(|entry| element(entry, &table)).collect();
    json!({ "kind": "object", "class": "QmlObjectListModel", "objectName": "", "children": [], "facts": [], "dirty": false, "count": elements.len(), "elements": elements })
}

pub fn get(path: &str) -> Option<Value> {
    owned().then_some(())?;
    let rest = path.strip_prefix(MODEL)?;
    if rest.is_empty() {
        return Some(model());
    }
    let rest = rest.strip_prefix('.')?;
    let (index, field) = rest.split_once('.').map_or((rest, None), |(i, f)| (i, Some(f)));
    if index == "count" {
        return Some(json!({ "kind": "value", "value": listed().len() }));
    }
    let entry = listed().into_iter().nth(index.parse().ok()?)?;
    let element = element(&entry, &table());
    Some(match field {
        None => element,
        Some(field) => match element.get(field).filter(|_| field != "kind") {
            Some(value) => json!({ "kind": "value", "value": value }),
            None => json!({ "kind": "value", "value": null, "found": false }),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_element_reads_as_the_qt_configuration_does() {
        let table = TypeTable::new(true, true);
        let tcp = Entry { config: LinkConfig { name: "iter7".into(), auto_connect: false, high_latency: false, kind: Kind::Tcp { host: "145.223.98.65".into(), port: 5760 } }, dynamic: false };
        let shown = element(&tcp, &table);
        assert_eq!((shown["class"].as_str(), shown["linkType"].as_i64(), shown["summary"].as_str(), shown["settingsURL"].as_str()), (Some("TCPConfiguration"), Some(2), Some("145.223.98.65:5760"), Some("TcpSettings.qml")));
        let udp = Entry { config: LinkConfig { name: "u".into(), auto_connect: true, high_latency: false, kind: Kind::Udp { local_port: 14550, hosts: vec![("10.0.0.2".into(), 14551)] } }, dynamic: true };
        let shown = element(&udp, &table);
        assert_eq!((shown["summary"].as_str(), shown["hostList"][0].as_str(), shown["localPort"].as_u64()), (Some("UDP port 14550"), Some("10.0.0.2:14551"), Some(14550)));
        let serial = Entry { config: LinkConfig { name: "s".into(), auto_connect: false, high_latency: false, kind: Kind::Serial { baud: 57600, data_bits: 8, flow_control: 0, stop_bits: 1, parity: 0, port_name: "/dev/cu.usbmodem1".into(), port_display_name: String::new() } }, dynamic: false };
        assert_eq!(element(&serial, &table)["summary"], "/dev/cu.usbmodem1 at 57600 baud", "the port name stands in when there is no display name");
    }
}
