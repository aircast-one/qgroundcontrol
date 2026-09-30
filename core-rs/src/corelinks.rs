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

type Opener = fn(LinkConfig) -> Result<crate::transport::LinkId, (String, &'static str)>;
type Closer = fn(crate::transport::LinkId);

static ENTRIES: LazyLock<Mutex<Option<Vec<Entry>>>> = LazyLock::new(|| Mutex::new(None));
static HOST_OWNS_LINKS: AtomicBool = AtomicBool::new(false);
static HOOKS: Mutex<Option<(Opener, Closer)>> = Mutex::new(None);
static LAST_ERRORS: LazyLock<Mutex<std::collections::BTreeMap<String, (String, &'static str)>>> = LazyLock::new(|| Mutex::new(std::collections::BTreeMap::new()));

pub fn set_hooks(open: Opener, close: Closer) {
    *HOOKS.lock().unwrap_or_else(PoisonError::into_inner) = Some((open, close));
}

fn live() -> Vec<(crate::transport::LinkId, LinkConfig)> {
    let transports = crate::linkhost::TRANSPORTS.lock().unwrap_or_else(PoisonError::into_inner);
    transports.open_ids().into_iter().filter_map(|id| transports.config(id).map(|config| (id, config))).collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub link: Option<crate::transport::LinkId>,
    pub heard: bool,
    pub error: Option<(String, &'static str)>,
}

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

fn saved() -> Vec<Entry> {
    let mut held = entries();
    held.get_or_insert_with(|| {
        let saved = crate::settingsstore::entries_under(crate::linkconfig::ROOT);
        crate::linkconfig::load(&saved, &table(), &defaults()).into_iter().map(|config| Entry { config, dynamic: false }).collect()
    })
    .clone()
}

pub fn listed_with(saved: Vec<Entry>, live: &[(crate::transport::LinkId, LinkConfig)]) -> Vec<Entry> {
    let transient: Vec<Entry> = live.iter().filter(|(_, config)| !saved.iter().any(|e| e.config.name == config.name)).map(|(_, config)| Entry { config: config.clone(), dynamic: true }).collect();
    saved.into_iter().chain(transient).collect()
}

pub fn listed() -> Vec<Entry> {
    listed_with(saved(), &live())
}

fn state_of(name: &str, live: &[(crate::transport::LinkId, LinkConfig)]) -> State {
    let link = live.iter().find(|(_, config)| config.name == name).map(|(id, _)| *id);
    State { link, heard: link.is_some_and(|id| crate::hub::lock().heard_on(id)), error: LAST_ERRORS.lock().unwrap_or_else(PoisonError::into_inner).get(name).cloned() }
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

pub fn element(entry: &Entry, table: &TypeTable, state: &State) -> Value {
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
        "lastError": state.error.as_ref().map_or("", |(reason, _)| reason.as_str()),
        "lastErrorRemedy": i64::from(state.error.as_ref().is_some_and(|(_, remedy)| *remedy == "editAddress")),
        "heardVehicle": state.heard,
        "linkActive": state.link.is_some(),
    });
    let mut merged = common;
    fields.as_object().into_iter().flatten().for_each(|(k, v)| merged[k.as_str()] = v.clone());
    match state.link {
        Some(_) => merged["children"] = json!(["link"]),
        None => merged["link"] = Value::Null,
    }
    merged
}

pub fn model() -> Value {
    let table = table();
    let live = live();
    let elements: Vec<Value> = listed_with(saved(), &live).iter().map(|entry| element(entry, &table, &state_of(&entry.config.name, &live))).collect();
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
    let live = live();
    let entry = listed_with(saved(), &live).into_iter().nth(index.parse().ok()?)?;
    let element = element(&entry, &table(), &state_of(&entry.config.name, &live));
    Some(match field {
        None => element,
        Some(field) => match element.get(field).filter(|_| field != "kind") {
            Some(value) => json!({ "kind": "value", "value": value }),
            None => json!({ "kind": "value", "value": null, "found": false }),
        },
    })
}

fn indexed(reference: &str) -> Option<usize> {
    reference.strip_prefix('@')?.strip_prefix(MODEL)?.strip_prefix('.')?.parse().ok()
}

fn connect(index: usize) -> bool {
    let Some(entry) = listed().into_iter().nth(index) else { return false };
    let Some((open, _)) = *HOOKS.lock().unwrap_or_else(PoisonError::into_inner) else { return false };
    if state_of(&entry.config.name, &live()).link.is_some() {
        return true;
    }
    let opened = open(entry.config.clone());
    let mut errors = LAST_ERRORS.lock().unwrap_or_else(PoisonError::into_inner);
    match opened {
        Ok(_) => {
            errors.remove(&entry.config.name);
            true
        }
        Err(failure) => {
            errors.insert(entry.config.name.clone(), failure);
            false
        }
    }
}

fn disconnect(index: usize) -> bool {
    let Some(entry) = listed().into_iter().nth(index) else { return false };
    let Some((_, close)) = *HOOKS.lock().unwrap_or_else(PoisonError::into_inner) else { return false };
    match state_of(&entry.config.name, &live()).link {
        Some(id) => {
            close(id);
            true
        }
        None => false,
    }
}

pub fn invoke(path: &str, args: &str) -> Option<Value> {
    owned().then_some(())?;
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let result = match path {
        "links.createConnectedLink" => connect(given.get(0).and_then(Value::as_str).and_then(indexed)?),
        _ => disconnect(path.strip_prefix(MODEL)?.strip_prefix('.')?.strip_suffix(".link.disconnect")?.parse().ok()?),
    };
    Some(json!({ "ok": true, "result": result }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_opened_outside_the_saved_list_is_listed_as_dynamic_after_it() {
        let tcp = |name: &str| LinkConfig { name: name.into(), auto_connect: false, high_latency: false, kind: Kind::Tcp { host: "127.0.0.1".into(), port: 5760 } };
        let saved = vec![Entry { config: tcp("SITL"), dynamic: false }];
        let listed = listed_with(saved, &[(1, tcp("SITL")), (2, tcp("debug-api 127.0.0.1:5760"))]);
        assert_eq!(listed.iter().map(|e| (e.config.name.as_str(), e.dynamic)).collect::<Vec<_>>(), vec![("SITL", false), ("debug-api 127.0.0.1:5760", true)]);
    }

    #[test]
    fn an_element_reads_as_the_qt_configuration_does() {
        let table = TypeTable::new(true, true);
        let tcp = Entry { config: LinkConfig { name: "iter7".into(), auto_connect: false, high_latency: false, kind: Kind::Tcp { host: "145.223.98.65".into(), port: 5760 } }, dynamic: false };
        let idle = State { link: None, heard: false, error: None };
        let shown = element(&tcp, &table, &idle);
        assert_eq!((shown["class"].as_str(), shown["linkType"].as_i64(), shown["summary"].as_str(), shown["settingsURL"].as_str()), (Some("TCPConfiguration"), Some(2), Some("145.223.98.65:5760"), Some("TcpSettings.qml")));
        let udp = Entry { config: LinkConfig { name: "u".into(), auto_connect: true, high_latency: false, kind: Kind::Udp { local_port: 14550, hosts: vec![("10.0.0.2".into(), 14551)] } }, dynamic: true };
        let shown = element(&udp, &table, &State { link: Some(3), heard: true, error: None });
        assert_eq!((shown["summary"].as_str(), shown["hostList"][0].as_str(), shown["localPort"].as_u64()), (Some("UDP port 14550"), Some("10.0.0.2:14551"), Some(14550)));
        assert_eq!((shown["linkActive"].as_bool(), shown["children"][0].as_str(), shown.get("link"), shown["heardVehicle"].as_bool()), (Some(true), Some("link"), None, Some(true)), "a connected configuration carries its link as a child object, as the bridge walks it");
        let serial = Entry { config: LinkConfig { name: "s".into(), auto_connect: false, high_latency: false, kind: Kind::Serial { baud: 57600, data_bits: 8, flow_control: 0, stop_bits: 1, parity: 0, port_name: "/dev/cu.usbmodem1".into(), port_display_name: String::new() } }, dynamic: false };
        assert_eq!(element(&serial, &table, &idle)["summary"], "/dev/cu.usbmodem1 at 57600 baud", "the port name stands in when there is no display name");
        let refused = element(&tcp, &table, &State { link: None, heard: false, error: Some(("Reached host but nothing is listening on port 5760.".into(), "editAddress")) });
        assert_eq!((refused["lastErrorRemedy"].as_i64(), refused["link"].is_null(), refused["children"].as_array().map(Vec::len)), (Some(1), true, Some(0)));
    }
}
