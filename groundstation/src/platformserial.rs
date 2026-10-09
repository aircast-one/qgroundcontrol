use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use crate::boards::PortInfo;

pub struct Hooks {
    pub open: fn(u32, &str, u32, i64, i64, i64) -> bool,
    pub write: fn(u32, &[u8]) -> bool,
    pub close: fn(u32),
    pub ports: fn() -> Vec<PortInfo>,
}

pub enum Event {
    Bytes(Vec<u8>),
    Disconnected(String),
}

type Receiver = Arc<dyn Fn(Event) + Send + Sync>;

static HOOKS: OnceLock<Hooks> = OnceLock::new();
static RECEIVERS: Mutex<BTreeMap<u32, Receiver>> = Mutex::new(BTreeMap::new());

pub fn install(hooks: Hooks) {
    let _ = HOOKS.set(hooks);
}

pub fn ports() -> Vec<PortInfo> {
    HOOKS.get().map(|hooks| (hooks.ports)()).unwrap_or_default().into_iter().chain(uart_ports()).collect()
}

const UART_PREFIX: &str = "/dev/ttyS";
const UART_COUNT: u32 = 10;

pub fn uarts(usable: impl Fn(&str) -> bool) -> Vec<PortInfo> {
    (0..UART_COUNT)
        .map(|n| format!("{UART_PREFIX}{n}"))
        .filter(|location| usable(location))
        .map(|location| PortInfo { port_name: location.trim_start_matches("/dev/").to_string(), system_location: location, ..PortInfo::default() })
        .collect()
}

#[cfg(target_os = "android")]
fn uart_ports() -> Vec<PortInfo> {
    static UARTS: OnceLock<Vec<PortInfo>> = OnceLock::new();
    UARTS
        .get_or_init(|| uarts(|location| std::ffi::CString::new(location).is_ok_and(|path| unsafe { libc::access(path.as_ptr(), libc::R_OK | libc::W_OK) } == 0)))
        .clone()
}

#[cfg(not(target_os = "android"))]
fn uart_ports() -> Vec<PortInfo> {
    Vec::new()
}

pub fn port_from_info(info: &str) -> Option<PortInfo> {
    let fields: Vec<&str> = info.split('\t').collect();
    let text = |i: usize| fields.get(i).map(|f| f.trim()).filter(|f| !f.is_empty() && *f != "null").unwrap_or_default().to_string();
    let location = text(0);
    (fields.len() >= 6 && !location.is_empty()).then(|| PortInfo {
        port_name: location.strip_prefix("/dev/").unwrap_or(&location).to_string(),
        system_location: location.clone(),
        description: text(1),
        manufacturer: text(2),
        serial_number: text(3),
        product_id: text(4).parse().ok(),
        vendor_id: text(5).parse().ok(),
    })
}

const ANDROID_BAUD_RATES: [u32; 30] = [
    50, 75, 110, 134, 150, 200, 300, 600, 1200, 1800, 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 500000, 576000, 921600, 1000000, 1152000, 1500000, 2000000, 2500000, 3000000,
    3500000, 4000000,
];

pub fn android_parity(qt_parity: i64) -> i64 {
    match qt_parity {
        2 => 2,
        3 => 1,
        4 => 4,
        5 => 3,
        _ => 0,
    }
}

pub fn display_name(port: &PortInfo) -> String {
    match [port.description.as_str(), port.manufacturer.as_str()].into_iter().find(|name| !name.is_empty()) {
        Some(name) => format!("{name} ({})", port.port_name),
        None => port.port_name.clone(),
    }
}

pub fn links_field(field: &str) -> Option<serde_json::Value> {
    match field {
        "linkTypeStrings" | "linkTypeIds" => crate::linkconfig::link_type_field(field),
        "serialBaudRates" => Some(serde_json::json!(ANDROID_BAUD_RATES.iter().map(u32::to_string).collect::<Vec<_>>())),
        "serialPorts" => Some(serde_json::json!(ports().iter().map(|p| p.system_location.clone()).collect::<Vec<_>>())),
        "serialPortStrings" => Some(serde_json::json!(ports().iter().map(display_name).collect::<Vec<_>>())),
        _ => None,
    }
}

fn receiver(id: u32) -> Option<Receiver> {
    RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).get(&id).cloned()
}

pub fn received(id: u32, bytes: Vec<u8>) {
    if let Some(deliver) = receiver(id) {
        deliver(Event::Bytes(bytes));
    }
}

pub fn closed(id: u32, reason: &str) {
    let taken = RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).remove(&id);
    if let Some(deliver) = taken {
        deliver(Event::Disconnected(reason.to_string()));
    }
}

pub enum PlatformSerial {
    Hosted(u32),
    #[cfg(target_os = "android")]
    Uart(crate::seriallink::SerialLink),
}

#[cfg(target_os = "android")]
fn open_uart(port_name: &str, baud: u32, data_bits: i64, stop_bits: i64, parity: i64, on_event: impl Fn(Event) + Send + Sync + 'static) -> Result<PlatformSerial, String> {
    let config = crate::seriallink::SerialConfig { port_name: port_name.to_string(), baud, data_bits, parity, stop_bits, flow_control: 0, usb_direct: false };
    crate::seriallink::SerialLink::open(&config, move |event| {
        on_event(match event {
            crate::seriallink::Event::Bytes(bytes) => Event::Bytes(bytes),
            crate::seriallink::Event::Disconnected(reason) => Event::Disconnected(reason),
        })
    })
    .map(PlatformSerial::Uart)
    .map_err(|e| e.to_string())
}

impl PlatformSerial {
    pub fn open(id: u32, port_name: &str, baud: u32, data_bits: i64, stop_bits: i64, parity: i64, on_event: impl Fn(Event) + Send + Sync + 'static) -> Result<PlatformSerial, String> {
        #[cfg(target_os = "android")]
        if port_name.starts_with(UART_PREFIX) {
            return open_uart(port_name, baud, data_bits, stop_bits, parity, on_event);
        }
        let hooks = HOOKS.get().ok_or_else(|| "this build has no serial host".to_string())?;
        RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).insert(id, Arc::new(on_event));
        match (hooks.open)(id, port_name, baud, data_bits, stop_bits, parity) {
            true => Ok(PlatformSerial::Hosted(id)),
            false => {
                RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).remove(&id);
                Err("Unknown error".to_string())
            }
        }
    }

    pub fn write(&self, bytes: &[u8]) -> bool {
        match self {
            PlatformSerial::Hosted(id) => HOOKS.get().is_some_and(|hooks| (hooks.write)(*id, bytes)),
            #[cfg(target_os = "android")]
            PlatformSerial::Uart(link) => link.write(bytes).is_ok(),
        }
    }

    pub fn close(&self) {
        match self {
            PlatformSerial::Hosted(id) => {
                RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).remove(id);
                if let Some(hooks) = HOOKS.get() {
                    (hooks.close)(*id);
                }
            }
            #[cfg(target_os = "android")]
            PlatformSerial::Uart(link) => link.close(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_info_line_reads_as_androidserial_splits_it() {
        let port = port_from_info("/dev/bus/usb/001/002\tPixhawk6X\tHolybro\tnull\t54\t12346").unwrap();
        assert_eq!((port.system_location.as_str(), port.port_name.as_str(), port.description.as_str(), port.manufacturer.as_str()), ("/dev/bus/usb/001/002", "bus/usb/001/002", "Pixhawk6X", "Holybro"));
        assert_eq!((port.serial_number.as_str(), port.product_id, port.vendor_id), ("", Some(54), Some(12346)), "a null serial number is no serial number");
        assert!(port_from_info("short\tline").is_none(), "AndroidSerial skips a line with fewer than six fields");
        assert_eq!(display_name(&port), "Pixhawk6X (bus/usb/001/002)", "cleanPortDisplayName on Android: description, then the port name to keep entries unique");
        assert_eq!([0, 2, 3, 4, 5].map(android_parity), [0, 2, 1, 4, 3], "QSerialPort parity to AndroidSerial parity, as _parityToAndroidParity maps it");
    }

    #[test]
    fn built_in_uarts_the_app_can_open_are_listed_in_port_order() {
        let ports = uarts(|location| ["/dev/ttyS3", "/dev/ttyS1", "/dev/ttyUSB0"].contains(&location));
        assert_eq!(ports.iter().map(|p| (p.system_location.as_str(), display_name(p))).collect::<Vec<_>>(), [("/dev/ttyS1", "ttyS1".to_string()), ("/dev/ttyS3", "ttyS3".to_string())]);
        assert!(ports.iter().all(|p| p.vendor_id.is_none()), "a UART is not a USB board, so autoconnect never claims it");
        assert!(ports.iter().all(|p| p.system_location.starts_with(UART_PREFIX)), "every listed UART opens through the UART path");
    }
}
