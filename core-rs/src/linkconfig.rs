use std::collections::BTreeMap;

use crate::settingsini::Setting;

pub const ROOT: &str = "LinkConfigurations";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Serial,
    Udp,
    Tcp,
    Bluetooth,
    Mock,
    AirLink,
    LogReplay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeTable {
    order: Vec<LinkKind>,
}

impl TypeTable {
    pub fn new(serial: bool, bluetooth: bool, mock: bool, airlink: bool) -> Self {
        let order = [
            serial.then_some(LinkKind::Serial),
            Some(LinkKind::Udp),
            Some(LinkKind::Tcp),
            bluetooth.then_some(LinkKind::Bluetooth),
            mock.then_some(LinkKind::Mock),
            airlink.then_some(LinkKind::AirLink),
            Some(LinkKind::LogReplay),
        ]
        .into_iter()
        .flatten()
        .collect();
        TypeTable { order }
    }

    pub fn kind(&self, code: i64) -> Option<LinkKind> {
        usize::try_from(code).ok().and_then(|i| self.order.get(i)).copied()
    }

    pub fn code(&self, kind: LinkKind) -> Option<i64> {
        self.order.iter().position(|k| *k == kind).map(|i| i as i64)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Serial { baud: i64, data_bits: i64, flow_control: i64, stop_bits: i64, parity: i64, port_name: String, port_display_name: String },
    Udp { local_port: u16, hosts: Vec<(String, u16)> },
    Tcp { host: String, port: u16 },
    Bluetooth { device_name: String, address: String },
    Mock { firmware_type: i64, vehicle_type: i64, send_status_text: bool, increment_vehicle_id: bool, failure_mode: i64 },
    AirLink,
    LogReplay { file: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkConfig {
    pub name: String,
    pub auto_connect: bool,
    pub high_latency: bool,
    pub kind: Kind,
}

pub struct Defaults {
    pub udp_port: u16,
    pub tcp_host: String,
    pub tcp_port: u16,
    pub serial_baud: i64,
}

fn text(settings: &BTreeMap<String, Setting>, key: &str) -> Option<String> {
    match settings.get(key)? {
        Setting::Text(t) => Some(t.clone()),
        Setting::List(items) => Some(items.join(", ")),
        _ => None,
    }
}

fn number<T: std::str::FromStr>(settings: &BTreeMap<String, Setting>, key: &str) -> Option<T> {
    text(settings, key)?.trim().parse().ok()
}

fn flag(settings: &BTreeMap<String, Setting>, key: &str) -> bool {
    text(settings, key).is_some_and(|t| t == "true" || t == "1")
}

fn kind(settings: &BTreeMap<String, Setting>, root: &str, link: LinkKind, defaults: &Defaults) -> Kind {
    let key = |k: &str| format!("{root}/{k}");
    match link {
        LinkKind::Serial => Kind::Serial {
            baud: number(settings, &key("baud")).unwrap_or(defaults.serial_baud),
            data_bits: number(settings, &key("dataBits")).unwrap_or(8),
            flow_control: number(settings, &key("flowControl")).unwrap_or(0),
            stop_bits: number(settings, &key("stopBits")).unwrap_or(1),
            parity: number(settings, &key("parity")).unwrap_or(0),
            port_name: text(settings, &key("portName")).unwrap_or_default(),
            port_display_name: text(settings, &key("portDisplayName")).unwrap_or_default(),
        },
        LinkKind::Udp => {
            let count: usize = number(settings, &key("hostCount")).unwrap_or(0);
            let hosts = (0..count).filter_map(|i| Some((text(settings, &key(&format!("host{i}")))?, number(settings, &key(&format!("port{i}")))?))).collect();
            Kind::Udp { local_port: number(settings, &key("port")).unwrap_or(defaults.udp_port), hosts }
        }
        LinkKind::Tcp => Kind::Tcp { host: text(settings, &key("host")).unwrap_or_else(|| defaults.tcp_host.clone()), port: number(settings, &key("port")).unwrap_or(defaults.tcp_port) },
        LinkKind::Bluetooth => Kind::Bluetooth {
            device_name: text(settings, &key("deviceName")).unwrap_or_default(),
            address: text(settings, &key("address")).or_else(|| text(settings, &key("uuid"))).unwrap_or_default(),
        },
        LinkKind::Mock => Kind::Mock {
            firmware_type: number(settings, &key("FirmwareType")).unwrap_or(12),
            vehicle_type: number(settings, &key("VehicleType")).unwrap_or(2),
            send_status_text: flag(settings, &key("SendStatusText")),
            increment_vehicle_id: text(settings, &key("IncrementVehicleId")).map(|v| v == "true" || v == "1").unwrap_or(true),
            failure_mode: number(settings, &key("FailureMode")).unwrap_or(0),
        },
        LinkKind::AirLink => Kind::AirLink,
        LinkKind::LogReplay => Kind::LogReplay { file: text(settings, &key("logFilename")).unwrap_or_default() },
    }
}

pub fn load(settings: &BTreeMap<String, Setting>, table: &TypeTable, defaults: &Defaults) -> Vec<LinkConfig> {
    let count: i64 = number(settings, &format!("{ROOT}/count")).unwrap_or(0);
    (0..count)
        .filter_map(|i| {
            let root = format!("{ROOT}/Link{i}");
            let link = table.kind(number(settings, &format!("{root}/type"))?)?;
            let name = text(settings, &format!("{root}/name")).filter(|n| !n.is_empty())?;
            Some(LinkConfig { name, auto_connect: flag(settings, &format!("{root}/auto")), high_latency: flag(settings, &format!("{root}/high_latency")), kind: kind(settings, &root, link, defaults) })
        })
        .collect()
}

fn link_kind(kind: &Kind) -> LinkKind {
    match kind {
        Kind::Serial { .. } => LinkKind::Serial,
        Kind::Udp { .. } => LinkKind::Udp,
        Kind::Tcp { .. } => LinkKind::Tcp,
        Kind::Bluetooth { .. } => LinkKind::Bluetooth,
        Kind::Mock { .. } => LinkKind::Mock,
        Kind::AirLink => LinkKind::AirLink,
        Kind::LogReplay { .. } => LinkKind::LogReplay,
    }
}

fn entries(root: &str, kind: &Kind) -> Vec<(String, String)> {
    let key = |k: &str, v: String| (format!("{root}/{k}"), v);
    match kind {
        Kind::Serial { baud, data_bits, flow_control, stop_bits, parity, port_name, port_display_name } => vec![
            key("baud", baud.to_string()),
            key("dataBits", data_bits.to_string()),
            key("flowControl", flow_control.to_string()),
            key("stopBits", stop_bits.to_string()),
            key("parity", parity.to_string()),
            key("portName", port_name.clone()),
            key("portDisplayName", port_display_name.clone()),
        ],
        Kind::Udp { local_port, hosts } => [key("hostCount", hosts.len().to_string()), key("port", local_port.to_string())]
            .into_iter()
            .chain(hosts.iter().enumerate().flat_map(|(i, (host, port))| [key(&format!("host{i}"), host.clone()), key(&format!("port{i}"), port.to_string())]))
            .collect(),
        Kind::Tcp { host, port } => vec![key("host", host.clone()), key("port", port.to_string())],
        Kind::Bluetooth { device_name, address } => vec![key("deviceName", device_name.clone()), key("address", address.clone())],
        Kind::Mock { firmware_type, vehicle_type, send_status_text, increment_vehicle_id, failure_mode } => vec![
            key("FirmwareType", firmware_type.to_string()),
            key("VehicleType", vehicle_type.to_string()),
            key("SendStatusText", send_status_text.to_string()),
            key("IncrementVehicleId", increment_vehicle_id.to_string()),
            key("FailureMode", failure_mode.to_string()),
        ],
        Kind::AirLink => Vec::new(),
        Kind::LogReplay { file } => vec![key("logFilename", file.clone())],
    }
}

pub fn save(configs: &[LinkConfig], table: &TypeTable) -> BTreeMap<String, Setting> {
    let saved: Vec<(usize, &LinkConfig, i64)> = configs.iter().filter_map(|c| table.code(link_kind(&c.kind)).map(|code| (c, code))).enumerate().map(|(i, (c, code))| (i, c, code)).collect();
    saved
        .iter()
        .flat_map(|(i, config, code)| {
            let root = format!("{ROOT}/Link{i}");
            [
                (format!("{root}/name"), config.name.clone()),
                (format!("{root}/type"), code.to_string()),
                (format!("{root}/auto"), config.auto_connect.to_string()),
                (format!("{root}/high_latency"), config.high_latency.to_string()),
            ]
            .into_iter()
            .chain(entries(&root, &config.kind))
        })
        .chain([(format!("{ROOT}/count"), saved.len().to_string())])
        .map(|(k, v)| (k, Setting::Text(v)))
        .collect()
}

pub fn to_json(config: &LinkConfig) -> serde_json::Value {
    use serde_json::json;
    let kind = match &config.kind {
        Kind::Serial { baud, data_bits, flow_control, stop_bits, parity, port_name, port_display_name } => json!({ "kind": "serial", "baud": baud, "dataBits": data_bits, "flowControl": flow_control, "stopBits": stop_bits, "parity": parity, "portName": port_name, "portDisplayName": port_display_name }),
        Kind::Udp { local_port, hosts } => json!({ "kind": "udp", "port": local_port, "hosts": hosts.iter().map(|(h, p)| json!({ "host": h, "port": p })).collect::<Vec<_>>() }),
        Kind::Tcp { host, port } => json!({ "kind": "tcp", "host": host, "port": port }),
        Kind::Bluetooth { device_name, address } => json!({ "kind": "bluetooth", "deviceName": device_name, "address": address }),
        Kind::Mock { firmware_type, vehicle_type, send_status_text, increment_vehicle_id, failure_mode } => json!({ "kind": "mock", "firmwareType": firmware_type, "vehicleType": vehicle_type, "sendStatusText": send_status_text, "incrementVehicleId": increment_vehicle_id, "failureMode": failure_mode }),
        Kind::AirLink => json!({ "kind": "airlink" }),
        Kind::LogReplay { file } => json!({ "kind": "logReplay", "logFilename": file }),
    };
    let mut object = kind;
    object["name"] = json!(config.name);
    object["auto"] = json!(config.auto_connect);
    object["highLatency"] = json!(config.high_latency);
    object
}

pub fn from_json(value: &serde_json::Value) -> Result<LinkConfig, String> {
    use serde_json::Value;
    let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_string);
    let int = |key: &str, fallback: i64| value.get(key).and_then(Value::as_i64).unwrap_or(fallback);
    let port = |key: &str| value.get(key).and_then(Value::as_u64).and_then(|p| u16::try_from(p).ok());
    let name = text("name").filter(|n| !n.is_empty()).ok_or("link configuration without a name")?;
    let kind = match text("kind").as_deref() {
        Some("serial") => Kind::Serial { baud: int("baud", 57600), data_bits: int("dataBits", 8), flow_control: int("flowControl", 0), stop_bits: int("stopBits", 1), parity: int("parity", 0), port_name: text("portName").unwrap_or_default(), port_display_name: text("portDisplayName").unwrap_or_default() },
        Some("udp") => Kind::Udp {
            local_port: port("port").ok_or("udp link without a port")?,
            hosts: value.get("hosts").and_then(Value::as_array).map(|h| h.iter().filter_map(|e| Some((e.get("host")?.as_str()?.to_string(), u16::try_from(e.get("port")?.as_u64()?).ok()?))).collect()).unwrap_or_default(),
        },
        Some("tcp") => Kind::Tcp { host: text("host").ok_or("tcp link without a host")?, port: port("port").ok_or("tcp link without a port")? },
        Some("bluetooth") => Kind::Bluetooth { device_name: text("deviceName").unwrap_or_default(), address: text("address").unwrap_or_default() },
        Some("mock") => Kind::Mock { firmware_type: int("firmwareType", 12), vehicle_type: int("vehicleType", 2), send_status_text: value.get("sendStatusText").and_then(Value::as_bool).unwrap_or(false), increment_vehicle_id: value.get("incrementVehicleId").and_then(Value::as_bool).unwrap_or(true), failure_mode: int("failureMode", 0) },
        Some("airlink") => Kind::AirLink,
        Some("logReplay") => Kind::LogReplay { file: text("logFilename").unwrap_or_default() },
        other => return Err(format!("unknown link kind {other:?}")),
    };
    Ok(LinkConfig { name, auto_connect: value.get("auto").and_then(Value::as_bool).unwrap_or(false), high_latency: value.get("highLatency").and_then(Value::as_bool).unwrap_or(false), kind })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> Defaults {
        Defaults { udp_port: 14550, tcp_host: "0.0.0.0".into(), tcp_port: 5760, serial_baud: 57600 }
    }

    #[test]
    fn the_type_code_follows_the_build_flags() {
        let desktop = TypeTable::new(true, false, true, true);
        assert_eq!((desktop.code(LinkKind::Serial), desktop.code(LinkKind::Udp), desktop.code(LinkKind::LogReplay)), (Some(0), Some(1), Some(5)));
        let android = TypeTable::new(true, true, false, true);
        assert_eq!((android.kind(3), android.kind(4), android.kind(5)), (Some(LinkKind::Bluetooth), Some(LinkKind::AirLink), Some(LinkKind::LogReplay)));
        let ios = TypeTable::new(false, false, false, false);
        assert_eq!((ios.kind(0), ios.kind(2), ios.kind(3), ios.code(LinkKind::Serial)), (Some(LinkKind::Udp), Some(LinkKind::LogReplay), None, None));
    }

    #[test]
    fn a_saved_list_reads_back_and_links_the_build_lacks_are_dropped() {
        let table = TypeTable::new(true, true, false, false);
        let configs = vec![
            LinkConfig { name: "Field UDP".into(), auto_connect: true, high_latency: false, kind: Kind::Udp { local_port: 14550, hosts: vec![("192.168.4.1".into(), 14550), ("10.0.0.2".into(), 14551)] } },
            LinkConfig { name: "SITL".into(), auto_connect: false, high_latency: false, kind: Kind::Tcp { host: "127.0.0.1".into(), port: 5760 } },
            LinkConfig { name: "Radio".into(), auto_connect: true, high_latency: true, kind: Kind::Serial { baud: 57600, data_bits: 8, flow_control: 0, stop_bits: 1, parity: 0, port_name: "cu.usbserial".into(), port_display_name: "USB Serial".into() } },
            LinkConfig { name: "Mock".into(), auto_connect: false, high_latency: false, kind: Kind::Mock { firmware_type: 3, vehicle_type: 1, send_status_text: true, increment_vehicle_id: false, failure_mode: 0 } },
            LinkConfig { name: "Replay".into(), auto_connect: false, high_latency: false, kind: Kind::LogReplay { file: "/tmp/mav.tlog".into() } },
        ];
        let saved = save(&configs, &table);
        assert_eq!(saved[&format!("{ROOT}/count")], Setting::Text("4".into()));
        assert_eq!(saved[&format!("{ROOT}/Link0/type")], Setting::Text("1".into()));
        assert_eq!(saved[&format!("{ROOT}/Link2/high_latency")], Setting::Text("true".into()));
        let loaded = load(&saved, &table, &defaults());
        assert_eq!(loaded, configs.iter().filter(|c| !matches!(c.kind, Kind::Mock { .. })).cloned().collect::<Vec<_>>());
        let debug = TypeTable::new(true, false, true, false);
        let mock_only: Vec<LinkConfig> = configs.iter().filter(|c| matches!(c.kind, Kind::Mock { .. })).cloned().collect();
        assert_eq!(load(&save(&mock_only, &debug), &debug, &defaults()), mock_only);
    }

    #[test]
    fn the_json_shape_round_trips_every_kind() {
        let table = TypeTable::new(true, true, true, true);
        let configs = vec![
            LinkConfig { name: "u".into(), auto_connect: true, high_latency: false, kind: Kind::Udp { local_port: 14550, hosts: vec![("10.0.0.1".into(), 14550)] } },
            LinkConfig { name: "t".into(), auto_connect: false, high_latency: true, kind: Kind::Tcp { host: "127.0.0.1".into(), port: 5760 } },
            LinkConfig { name: "s".into(), auto_connect: false, high_latency: false, kind: Kind::Serial { baud: 115200, data_bits: 8, flow_control: 0, stop_bits: 1, parity: 0, port_name: "/dev/x".into(), port_display_name: "x".into() } },
            LinkConfig { name: "b".into(), auto_connect: false, high_latency: false, kind: Kind::Bluetooth { device_name: "r".into(), address: "aa:bb".into() } },
            LinkConfig { name: "m".into(), auto_connect: false, high_latency: false, kind: Kind::Mock { firmware_type: 3, vehicle_type: 2, send_status_text: true, increment_vehicle_id: false, failure_mode: 1 } },
            LinkConfig { name: "r".into(), auto_connect: false, high_latency: false, kind: Kind::LogReplay { file: "/tmp/x.tlog".into() } },
        ];
        for config in &configs {
            assert_eq!(from_json(&to_json(config)).unwrap(), *config);
            assert!(table.code(link_kind(&config.kind)).is_some());
        }
        assert!(from_json(&serde_json::json!({ "kind": "udp", "name": "x" })).is_err());
        assert!(from_json(&serde_json::json!({ "kind": "warp", "name": "x" })).is_err());
    }

    #[test]
    fn a_qt_written_file_loads_with_defaults_for_what_is_missing() {
        let ini = "[LinkConfigurations]\ncount=3\nLink0\\name=\"Old UDP\"\nLink0\\type=1\nLink0\\auto=true\nLink0\\high_latency=false\nLink0\\hostCount=1\nLink0\\host0=192.168.1.10\nLink0\\port0=14550\nLink1\\name=\nLink1\\type=2\nLink2\\name=\"Bad\"\nLink2\\type=99\n";
        let settings = crate::settingsini::read(ini);
        let loaded = load(&settings, &TypeTable::new(true, false, false, false), &defaults());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].kind, Kind::Udp { local_port: 14550, hosts: vec![("192.168.1.10".into(), 14550)] });
        assert!(loaded[0].auto_connect && !loaded[0].high_latency);
    }
}
