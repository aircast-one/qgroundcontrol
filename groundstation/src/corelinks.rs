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
static HOOKS: Mutex<Option<(Opener, Closer)>> = Mutex::new(None);
static LAST_ERRORS: LazyLock<Mutex<std::collections::BTreeMap<String, (String, &'static str)>>> = LazyLock::new(|| Mutex::new(std::collections::BTreeMap::new()));
static RUNTIME: LazyLock<Mutex<std::collections::BTreeMap<String, Runtime>>> = LazyLock::new(|| Mutex::new(std::collections::BTreeMap::new()));
static DYNAMIC: Mutex<Vec<Entry>> = Mutex::new(Vec::new());
static AUTOCONNECTING: AtomicBool = AtomicBool::new(false);
static LAST_TICK_MS: Mutex<u64> = Mutex::new(0);
const RECONNECT_BASE_MS: u64 = 1000;
const RECONNECT_MAX_MS: u64 = 5000;
const RECONNECT_STABLE_MS: u64 = 2000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Runtime {
    pub started: bool,
    pub suppressed: bool,
    attempts: u32,
    next_ms: u64,
    connected_at: Option<u64>,
}

impl Runtime {
    pub fn connect_requested(&mut self) {
        self.started = true;
        self.suppressed = false;
        self.attempts = 0;
        self.next_ms = 0;
    }

    pub fn reconnect_due(&self, now_ms: u64) -> bool {
        self.started && !self.suppressed && now_ms >= self.next_ms
    }

    pub fn note_attempt(&mut self, now_ms: u64) {
        let exponent = self.attempts.min(16);
        self.attempts = (self.attempts + 1).min(17);
        self.next_ms = now_ms + (RECONNECT_BASE_MS << exponent).min(RECONNECT_MAX_MS);
    }

    pub fn note_link(&mut self, up: bool, now_ms: u64) {
        match (up, self.connected_at) {
            (true, None) => self.connected_at = Some(now_ms),
            (false, Some(since)) => {
                if now_ms.saturating_sub(since) >= RECONNECT_STABLE_MS {
                    self.attempts = 0;
                    self.next_ms = 0;
                }
                self.connected_at = None;
            }
            _ => {}
        }
    }
}

fn runtime(name: &str) -> Runtime {
    RUNTIME.lock().unwrap_or_else(PoisonError::into_inner).get(name).copied().unwrap_or_default()
}

fn update_runtime(name: &str, change: impl FnOnce(&mut Runtime)) {
    change(RUNTIME.lock().unwrap_or_else(PoisonError::into_inner).entry(name.to_string()).or_default());
}

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
    pub reconnecting: bool,
}

pub fn owned() -> bool {
    !crate::qthost::present()
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

fn with_saved<T>(change: impl FnOnce(&mut Vec<Entry>) -> T) -> T {
    let mut held = entries();
    change(held.get_or_insert_with(|| {
        let saved = crate::settingsstore::entries_under(crate::linkconfig::ROOT);
        crate::linkconfig::load(&saved, &table(), &defaults()).into_iter().map(|config| Entry { config, dynamic: false }).collect()
    }))
}

fn saved() -> Vec<Entry> {
    with_saved(|entries| entries.clone())
}

fn save() {
    let configs: Vec<LinkConfig> = saved().into_iter().map(|e| e.config).collect();
    crate::settingsstore::replace_group(crate::linkconfig::ROOT, crate::linkconfig::save(&configs, &table()));
}

fn name_taken(name: &str) -> bool {
    listed().iter().any(|e| e.config.name == name)
}

pub fn add(config: LinkConfig) -> bool {
    if config.name.is_empty() || name_taken(&config.name) {
        return false;
    }
    with_saved(|entries| entries.push(Entry { config, dynamic: false }));
    save();
    true
}

pub fn replace_and_connect(config: LinkConfig) -> bool {
    let name = config.name.clone();
    if let Some(index) = listed().iter().position(|e| e.config.name == name) {
        remove(index);
    }
    add(config) && listed().iter().position(|e| e.config.name == name).is_some_and(connect)
}

pub fn created(kind: &str, name: &str, host: &str, port: i64) -> Option<LinkConfig> {
    let port = u16::try_from(port).ok().filter(|p| *p > 0)?;
    let config = |kind| LinkConfig { name: name.to_string(), auto_connect: false, high_latency: false, kind };
    match kind.to_lowercase().as_str() {
        "udp" => Some(config(Kind::Udp { local_port: port, hosts: if host.is_empty() { Vec::new() } else { vec![(host.to_string(), port)] } })),
        "tcp" if !host.is_empty() => Some(config(Kind::Tcp { host: host.to_string(), port })),
        _ => None,
    }
}

fn create_and_connect(kind: &str, name: &str, host: &str, port: i64) -> bool {
    let Some(config) = created(kind, name, host, port) else { return false };
    if !add(config) {
        return false;
    }
    let index = listed().iter().position(|e| e.config.name == name).unwrap_or(0);
    connect(index)
}

fn create_serial(name: &str, port_name: &str, baud: i64) -> bool {
    if table().code(crate::linkconfig::LinkKind::Serial).is_none() || port_name.is_empty() || baud <= 0 {
        return false;
    }
    add(LinkConfig { name: name.to_string(), auto_connect: false, high_latency: false, kind: Kind::Serial { baud, data_bits: 8, flow_control: 0, stop_bits: 1, parity: 0, port_name: port_name.to_string(), port_display_name: String::new() } })
}

fn create_bluetooth(name: &str, device_name: &str, address: &str) -> bool {
    if name.is_empty() || address.is_empty() || !add(LinkConfig { name: name.to_string(), auto_connect: false, high_latency: false, kind: Kind::Bluetooth { device_name: device_name.to_string(), address: address.to_string() } }) {
        return false;
    }
    let index = listed().iter().position(|e| e.config.name == name).unwrap_or(0);
    connect(index)
}

fn remove(index: usize) -> bool {
    let Some(entry) = listed().into_iter().nth(index) else { return false };
    let hooks = *HOOKS.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((id, (_, close))) = state_of(&entry.config.name, &live()).link.zip(hooks) {
        close(id);
    }
    with_saved(|entries| entries.retain(|e| e.config.name != entry.config.name));
    DYNAMIC.lock().unwrap_or_else(PoisonError::into_inner).retain(|e| e.config.name != entry.config.name);
    RUNTIME.lock().unwrap_or_else(PoisonError::into_inner).remove(&entry.config.name);
    save();
    true
}

pub fn edited(config: &LinkConfig, field: &str, value: &Value) -> Option<LinkConfig> {
    let text = value.as_str().map(|t| t.trim().to_string());
    let port = value.as_u64().and_then(|p| u16::try_from(p).ok());
    let kind = match (&config.kind, field) {
        (_, "name") => return text.filter(|t| !t.is_empty()).map(|name| LinkConfig { name, ..config.clone() }),
        (_, "autoConnect") => return value.as_bool().map(|auto_connect| LinkConfig { auto_connect, ..config.clone() }),
        (_, "highLatency") => return value.as_bool().map(|high_latency| LinkConfig { high_latency, ..config.clone() }),
        (Kind::Tcp { port, .. }, "host") => Kind::Tcp { host: text?, port: *port },
        (Kind::Tcp { host, .. }, "port") => Kind::Tcp { host: host.clone(), port: port? },
        (Kind::Udp { hosts, .. }, "localPort") => Kind::Udp { local_port: port?, hosts: hosts.clone() },
        (Kind::Serial { baud, data_bits, flow_control, stop_bits, parity, port_display_name, .. }, "portName") => {
            Kind::Serial { baud: *baud, data_bits: *data_bits, flow_control: *flow_control, stop_bits: *stop_bits, parity: *parity, port_name: text?, port_display_name: port_display_name.clone() }
        }
        (Kind::Serial { data_bits, flow_control, stop_bits, parity, port_name, port_display_name, .. }, "baud") => {
            Kind::Serial { baud: value.as_i64()?, data_bits: *data_bits, flow_control: *flow_control, stop_bits: *stop_bits, parity: *parity, port_name: port_name.clone(), port_display_name: port_display_name.clone() }
        }
        _ => return None,
    };
    Some(LinkConfig { kind, ..config.clone() })
}

pub fn set(path: &str, value: &str) -> Option<Value> {
    owned().then_some(())?;
    let (index, field) = path.strip_prefix(MODEL)?.strip_prefix('.')?.split_once('.')?;
    let index: usize = index.parse().ok()?;
    let given = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").cloned()).unwrap_or(Value::Null);
    let name = saved().into_iter().nth(index)?.config.name;
    let changed = with_saved(|entries| {
        let entry = entries.get_mut(index)?;
        let updated = edited(&entry.config, field, &given)?;
        entry.config = updated;
        Some(())
    });
    let renamed = saved().into_iter().nth(index).map(|e| e.config.name).unwrap_or_else(|| name.clone());
    if changed.is_some() && renamed != name {
        let mut runtime = RUNTIME.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(run) = runtime.remove(&name) {
            runtime.insert(renamed, run);
        }
    }
    Some(json!({ "ok": changed.is_some() }))
}

pub fn listed_with(known: Vec<Entry>, live: &[(crate::transport::LinkId, LinkConfig)]) -> Vec<Entry> {
    let transient: Vec<Entry> = live.iter().filter(|(_, config)| !known.iter().any(|e| e.config.name == config.name)).map(|(_, config)| Entry { config: config.clone(), dynamic: true }).collect();
    known.into_iter().chain(transient).collect()
}

fn known() -> Vec<Entry> {
    saved().into_iter().chain(DYNAMIC.lock().unwrap_or_else(PoisonError::into_inner).iter().cloned()).collect()
}

pub fn listed() -> Vec<Entry> {
    listed_with(known(), &live())
}

fn state_of(name: &str, live: &[(crate::transport::LinkId, LinkConfig)]) -> State {
    let link = live.iter().find(|(_, config)| config.name == name).map(|(id, _)| *id);
    let run = runtime(name);
    State { link, heard: link.is_some_and(|id| crate::hub::lock().heard_on(id)), error: LAST_ERRORS.lock().unwrap_or_else(PoisonError::into_inner).get(name).cloned(), reconnecting: run.started && !run.suppressed }
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
        "linkActive": state.link.is_some() || (entry.config.auto_connect && state.reconnecting),
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
    let elements: Vec<Value> = listed_with(known(), &live).iter().map(|entry| element(entry, &table, &state_of(&entry.config.name, &live))).collect();
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
    let entry = listed_with(known(), &live).into_iter().nth(index.parse().ok()?)?;
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
    update_runtime(&entry.config.name, Runtime::connect_requested);
    open_entry(&entry)
}

fn open_entry(entry: &Entry) -> bool {
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
    update_runtime(&entry.config.name, |run| run.suppressed = true);
    match state_of(&entry.config.name, &live()).link {
        Some(id) => {
            close(id);
            true
        }
        None => false,
    }
}

fn udp_autoconnect_entry() -> Option<Entry> {
    let wanted = crate::settingsstore::raw_setting("settings.autoConnectSettings.autoConnectUDP").and_then(|v| v.as_bool()).unwrap_or(true);
    wanted.then(|| Entry {
        config: LinkConfig { name: crate::autoconnect::DEFAULT_UDP_LINK_NAME.to_string(), auto_connect: true, high_latency: false, kind: Kind::Udp { local_port: defaults().udp_port, hosts: Vec::new() } },
        dynamic: true,
    })
}

pub fn start() {
    AUTOCONNECTING.store(true, Ordering::SeqCst);
    saved().iter().filter(|e| e.config.auto_connect).for_each(|entry| {
        update_runtime(&entry.config.name, |run| run.started = true);
        open_entry(entry);
    });
}

static SERIAL_AUTO: LazyLock<Mutex<crate::autoconnect::AutoConnect>> = LazyLock::new(|| Mutex::new(crate::autoconnect::AutoConnect::default()));
static BOARDS: LazyLock<Option<crate::boards::BoardTable>> = LazyLock::new(|| crate::boards::BoardTable::bundled().ok());

fn autoconnect_setting(name: &str, unset: bool) -> bool {
    crate::settingsstore::raw_setting(&format!("settings.autoConnectSettings.{name}")).and_then(|v| v.as_bool()).unwrap_or(unset)
}

fn autoconnect_settings() -> crate::autoconnect::Settings {
    crate::autoconnect::Settings {
        pixhawk: autoconnect_setting("autoConnectPixhawk", true),
        sik_radio: autoconnect_setting("autoConnectSiKRadio", true),
        libre_pilot: autoconnect_setting("autoConnectLibrePilot", true),
        rtk_gps: autoconnect_setting("autoConnectRTKGPS", true),
        udp: autoconnect_setting("autoConnectUDP", true),
        forward_mavlink: false,
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn serial_ports() -> Vec<crate::boards::PortInfo> {
    let found = serialport::available_ports().unwrap_or_default();
    let visible = crate::seriallink::visible_ports(&found.iter().map(|p| p.port_name.clone()).collect::<Vec<_>>());
    found
        .into_iter()
        .filter(|p| visible.contains(&p.port_name))
        .map(|p| {
            let usb = match p.port_type {
                serialport::SerialPortType::UsbPort(usb) => Some(usb),
                _ => None,
            };
            crate::boards::PortInfo {
                port_name: crate::linkconfig::port_display_name(&p.port_name),
                system_location: p.port_name,
                description: usb.as_ref().and_then(|u| u.product.clone()).unwrap_or_default(),
                manufacturer: usb.as_ref().and_then(|u| u.manufacturer.clone()).unwrap_or_default(),
                serial_number: usb.as_ref().and_then(|u| u.serial_number.clone()).unwrap_or_default(),
                vendor_id: usb.as_ref().map(|u| u.vid),
                product_id: usb.as_ref().map(|u| u.pid),
            }
        })
        .collect()
}

#[cfg(target_os = "android")]
fn serial_ports() -> Vec<crate::boards::PortInfo> {
    crate::platformserial::ports()
}

#[cfg(target_os = "ios")]
fn serial_ports() -> Vec<crate::boards::PortInfo> {
    Vec::new()
}

pub fn serial_entry(name: &str, port: &str, baud: u32) -> Entry {
    Entry {
        config: LinkConfig { name: name.to_string(), auto_connect: true, high_latency: false, kind: Kind::Serial { baud: i64::from(baud), data_bits: 8, flow_control: 0, stop_bits: 1, parity: 0, port_name: port.to_string(), port_display_name: crate::linkconfig::port_display_name(port) } },
        dynamic: true,
    }
}

fn add_dynamic(entry: &Entry) {
    let mut dynamic = DYNAMIC.lock().unwrap_or_else(PoisonError::into_inner);
    if !dynamic.iter().any(|e| e.config.name == entry.config.name) {
        dynamic.push(entry.clone());
    }
}

fn autoconnect_serial(live: &[(crate::transport::LinkId, LinkConfig)]) {
    let Some(boards) = BOARDS.as_ref() else { return };
    let connected: Vec<String> = live.iter().filter_map(|(_, c)| match &c.kind {
        Kind::Serial { port_name, .. } => Some(port_name.clone()),
        _ => None,
    }).collect();
    let nmea = crate::settingsstore::raw_setting("settings.autoConnectSettings.autoConnectNmeaPort").and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
    let host = crate::autoconnect::Host { android: cfg!(target_os = "android"), windows: cfg!(target_os = "windows") };
    let actions = SERIAL_AUTO.lock().unwrap_or_else(PoisonError::into_inner).serial(boards, &autoconnect_settings(), &host, serial_ports(), &connected, &nmea);
    actions.into_iter().for_each(|action| {
        if let crate::autoconnect::Action::OpenSerial { name, port, baud, .. } = action {
            let entry = serial_entry(&name, &port, baud);
            add_dynamic(&entry);
            open_entry(&entry);
        }
    });
}

pub fn tick(now_ms: u64) {
    if !owned() || !AUTOCONNECTING.load(Ordering::SeqCst) {
        return;
    }
    {
        let mut last = LAST_TICK_MS.lock().unwrap_or_else(PoisonError::into_inner);
        if now_ms.saturating_sub(*last) < crate::autoconnect::UPDATE_INTERVAL_MS as u64 {
            return;
        }
        *last = now_ms;
    }
    let live = live();
    DYNAMIC.lock().unwrap_or_else(PoisonError::into_inner).retain(|e| live.iter().any(|(_, c)| c.name == e.config.name));
    if let Some(udp) = udp_autoconnect_entry().filter(|udp| !live.iter().any(|(_, c)| c.name == udp.config.name && matches!(c.kind, Kind::Udp { .. }))) {
        add_dynamic(&udp);
        open_entry(&udp);
    }
    autoconnect_serial(&live);
    saved().iter().filter(|e| e.config.auto_connect).for_each(|entry| {
        let up = live.iter().any(|(_, c)| c.name == entry.config.name);
        update_runtime(&entry.config.name, |run| run.note_link(up, now_ms));
        if !up && runtime(&entry.config.name).reconnect_due(now_ms) {
            update_runtime(&entry.config.name, |run| run.note_attempt(now_ms));
            open_entry(entry);
        }
    });
}

pub fn invoke(path: &str, args: &str) -> Option<Value> {
    owned().then_some(())?;
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let text = |i: usize| given.get(i).and_then(Value::as_str).unwrap_or_default().to_string();
    let whole = |i: usize| given.get(i).and_then(Value::as_i64).unwrap_or(0);
    let result = match path {
        "links.createConnectedLink" => json!(connect(given.get(0).and_then(Value::as_str).and_then(indexed)?)),
        "links.createAndConnectLink" => json!(create_and_connect(&text(0), &text(1), &text(2), whole(3))),
        "links.createSerialConfiguration" => json!(create_serial(&text(0), &text(1), whole(2))),
        "links.createBluetoothLink" => json!(create_bluetooth(&text(0), &text(1), &text(2))),
        crate::platformbluetooth::SCAN => return crate::platformbluetooth::scan(args),
        "links.removeConfiguration" => json!(remove(given.get(0).and_then(Value::as_str).and_then(indexed)?)),
        "links.commitLinkConfigurations" => {
            save();
            Value::Null
        }
        _ => json!(disconnect(path.strip_prefix(MODEL)?.strip_prefix('.')?.strip_suffix(".link.disconnect")?.parse().ok()?)),
    };
    Some(json!({ "ok": true, "result": result }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_created_link_follows_create_and_connect_link_and_an_edit_changes_only_its_field() {
        assert_eq!(created("TCP", "Bench", "", 5760), None, "a TCP link needs the address to call");
        assert_eq!(created("tcp", "Bench", "10.0.0.5", 0), None);
        assert_eq!(created("bluetooth", "B", "x", 1), None);
        let udp = created("UDP", "Field", "192.168.4.1", 14550).unwrap();
        assert_eq!(udp.kind, Kind::Udp { local_port: 14550, hosts: vec![("192.168.4.1".into(), 14550)] }, "createAndConnectLink listens on the port it sends to");
        let tcp = created("tcp", "Bench", "10.0.0.5", 5760).unwrap();
        assert_eq!(edited(&tcp, "host", &json!(" 10.0.0.6 ")).unwrap().kind, Kind::Tcp { host: "10.0.0.6".into(), port: 5760 });
        assert_eq!(edited(&tcp, "port", &json!(5761)).unwrap().kind, Kind::Tcp { host: "10.0.0.5".into(), port: 5761 });
        assert_eq!(edited(&tcp, "name", &json!("Bench 2")).unwrap().name, "Bench 2");
        assert_eq!(edited(&tcp, "localPort", &json!(1)), None, "a field the kind does not carry is not written");
        assert!(edited(&tcp, "autoConnect", &json!(true)).is_some_and(|c| c.auto_connect), "Automatically Connect on Start");
        assert!(edited(&tcp, "highLatency", &json!(true)).is_some_and(|c| c.high_latency));
        assert_eq!(edited(&tcp, "highLatency", &json!("yes")), None);
        assert_eq!(edited(&tcp, "port", &json!(70000)), None);
    }

    #[test]
    fn an_autoconnected_board_is_a_dynamic_serial_link_named_for_its_port() {
        let entry = serial_entry("PX4 FMU V2 on cu.usbmodem1 (AutoConnect)", "/dev/cu.usbmodem1", 115200);
        let shown = element(&entry, &TypeTable::new(true, true), &State { link: None, heard: false, error: None, reconnecting: false });
        assert_eq!((shown["dynamic"].as_bool(), shown["autoConnect"].as_bool(), shown["baud"].as_i64(), shown["portDisplayName"].as_str()), (Some(true), Some(true), Some(115200), Some("cu.usbmodem1")));
        assert_eq!(shown["summary"], "cu.usbmodem1 at 115200 baud");
    }

    #[test]
    fn a_failing_auto_link_backs_off_to_five_seconds_and_a_stable_connection_resets_it() {
        let mut run = Runtime::default();
        assert!(!run.reconnect_due(0), "a link nobody started is not reconnected");
        run.connect_requested();
        let gaps: Vec<u64> = (0..5).map(|_| {
            let at = run.next_ms;
            run.note_attempt(at);
            run.next_ms - at
        }).collect();
        assert_eq!(gaps, vec![1000, 2000, 4000, 5000, 5000]);
        run.note_link(true, 100_000);
        run.note_link(false, 101_000);
        assert_eq!(run.attempts, 5, "one second up is not a working link");
        run.note_link(true, 110_000);
        run.note_link(false, 112_000);
        assert_eq!((run.attempts, run.reconnect_due(112_000)), (0, true));
        run.suppressed = true;
        assert!(!run.reconnect_due(200_000), "an operator disconnect stops reconnecting until a manual connect");
    }

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
        let idle = State { link: None, heard: false, error: None, reconnecting: false };
        let shown = element(&tcp, &table, &idle);
        assert_eq!((shown["class"].as_str(), shown["linkType"].as_i64(), shown["summary"].as_str(), shown["settingsURL"].as_str()), (Some("TCPConfiguration"), Some(2), Some("145.223.98.65:5760"), Some("TcpSettings.qml")));
        let udp = Entry { config: LinkConfig { name: "u".into(), auto_connect: true, high_latency: false, kind: Kind::Udp { local_port: 14550, hosts: vec![("10.0.0.2".into(), 14551)] } }, dynamic: true };
        let shown = element(&udp, &table, &State { link: Some(3), heard: true, error: None, reconnecting: false });
        assert_eq!((shown["summary"].as_str(), shown["hostList"][0].as_str(), shown["localPort"].as_u64()), (Some("UDP port 14550"), Some("10.0.0.2:14551"), Some(14550)));
        assert_eq!((shown["linkActive"].as_bool(), shown["children"][0].as_str(), shown.get("link"), shown["heardVehicle"].as_bool()), (Some(true), Some("link"), None, Some(true)), "a connected configuration carries its link as a child object, as the bridge walks it");
        let serial = Entry { config: LinkConfig { name: "s".into(), auto_connect: false, high_latency: false, kind: Kind::Serial { baud: 57600, data_bits: 8, flow_control: 0, stop_bits: 1, parity: 0, port_name: "/dev/cu.usbmodem1".into(), port_display_name: String::new() } }, dynamic: false };
        assert_eq!(element(&serial, &table, &idle)["summary"], "/dev/cu.usbmodem1 at 57600 baud", "the port name stands in when there is no display name");
        let refused = element(&tcp, &table, &State { link: None, heard: false, error: Some(("Reached host but nothing is listening on port 5760.".into(), "editAddress")), reconnecting: false });
        assert_eq!((refused["lastErrorRemedy"].as_i64(), refused["link"].is_null(), refused["children"].as_array().map(Vec::len)), (Some(1), true, Some(0)));
    }
}
