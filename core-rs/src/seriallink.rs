use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};

const READ_TIMEOUT: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialConfig {
    pub port_name: String,
    pub baud: u32,
    pub data_bits: i64,
    pub parity: i64,
    pub stop_bits: i64,
    pub flow_control: i64,
    pub usb_direct: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Bytes(Vec<u8>),
    Disconnected(String),
}

pub fn data_bits(qt: i64) -> DataBits {
    match qt {
        5 => DataBits::Five,
        6 => DataBits::Six,
        7 => DataBits::Seven,
        _ => DataBits::Eight,
    }
}

pub fn parity(qt: i64) -> Parity {
    match qt {
        2 => Parity::Even,
        3 => Parity::Odd,
        _ => Parity::None,
    }
}

pub fn stop_bits(qt: i64) -> StopBits {
    match qt {
        2 => StopBits::Two,
        _ => StopBits::One,
    }
}

pub fn flow_control(qt: i64) -> FlowControl {
    match qt {
        1 => FlowControl::Hardware,
        2 => FlowControl::Software,
        _ => FlowControl::None,
    }
}

pub struct SerialLink {
    port: Arc<Mutex<Box<dyn SerialPort>>>,
    stop: Arc<AtomicBool>,
    reader: Mutex<Option<JoinHandle<()>>>,
}

impl SerialLink {
    pub fn open(config: &SerialConfig, sink: impl FnMut(Event) + Send + 'static) -> io::Result<SerialLink> {
        let port = serialport::new(&config.port_name, config.baud)
            .data_bits(data_bits(config.data_bits))
            .parity(parity(config.parity))
            .stop_bits(stop_bits(config.stop_bits))
            .flow_control(flow_control(config.flow_control))
            .timeout(READ_TIMEOUT)
            .open()
            .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("{}: {}", config.port_name, e)))?;
        Self::from_port(port, sink)
    }

    pub fn from_port(mut port: Box<dyn SerialPort>, mut sink: impl FnMut(Event) + Send + 'static) -> io::Result<SerialLink> {
        port.set_timeout(READ_TIMEOUT).map_err(|e| io::Error::other(e.to_string()))?;
        let _ = port.write_data_terminal_ready(true);
        let mut reader_port = port.try_clone().map_err(|e| io::Error::other(e.to_string()))?;
        let stop = Arc::new(AtomicBool::new(false));
        let reader = {
            let stop = Arc::clone(&stop);
            std::thread::Builder::new().name("qgc-serial".into()).spawn(move || {
                let mut buffer = vec![0u8; 4096];
                while !stop.load(Ordering::Relaxed) {
                    match reader_port.read(&mut buffer) {
                        Ok(0) => {
                            sink(Event::Disconnected("device returned no data".into()));
                            return;
                        }
                        Ok(len) => sink(Event::Bytes(buffer[..len].to_vec())),
                        Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted) => {}
                        Err(e) => {
                            sink(Event::Disconnected(e.to_string()));
                            return;
                        }
                    }
                }
            })?
        };
        Ok(SerialLink { port: Arc::new(Mutex::new(port)), stop, reader: Mutex::new(Some(reader)) })
    }

    pub fn write(&self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Data to Send is Empty"));
        }
        let mut port = self.port.lock().unwrap();
        port.write_all(bytes)?;
        port.flush()?;
        Ok(bytes.len())
    }

    pub fn close(&self) {
        self.stop.store(true, Ordering::Relaxed);
        let handle = self.reader.lock().unwrap().take();
        if let Some(reader) = handle {
            let _ = reader.join();
        }
    }
}

impl Drop for SerialLink {
    fn drop(&mut self) {
        self.close();
    }
}

const MACOS_SYSTEM_PORTS: [&str; 5] = ["tty.MALS", "tty.SOC", "tty.Bluetooth-Incoming-Port", "tty.usbserial", "tty.usbmodem"];
const MACOS_BAUD_RATES: [u32; 25] = [50, 75, 110, 134, 150, 200, 300, 600, 1200, 1800, 2400, 4800, 7200, 9600, 14400, 19200, 28800, 38400, 57600, 76800, 115200, 230400, 460800, 500000, 921600];

pub fn visible_ports(system_locations: &[String]) -> Vec<String> {
    system_locations.iter().filter(|port| !MACOS_SYSTEM_PORTS.iter().any(|system| port.contains(system))).cloned().collect()
}

pub fn links_field(field: &str) -> Option<serde_json::Value> {
    let ports = || -> Option<Vec<String>> {
        cfg!(target_os = "macos").then_some(())?;
        Some(visible_ports(&serialport::available_ports().ok()?.into_iter().map(|p| p.port_name).collect::<Vec<_>>()))
    };
    Some(match field {
        "linkTypeStrings" | "linkTypeIds" => crate::linkconfig::link_type_field(field)?,
        "serialBaudRates" if cfg!(target_os = "macos") => serde_json::json!(MACOS_BAUD_RATES.iter().map(u32::to_string).collect::<Vec<_>>()),
        "serialPorts" => serde_json::json!(ports()?),
        "serialPortStrings" => serde_json::json!(ports()?.iter().map(|p| crate::linkconfig::port_display_name(p)).collect::<Vec<_>>()),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn macos_system_ports_are_hidden_and_a_port_shows_by_its_device_name() {
        let found = ["/dev/cu.debug-console", "/dev/tty.Bluetooth-Incoming-Port", "/dev/cu.Bluetooth-Incoming-Port", "/dev/tty.usbmodem14101", "/dev/cu.usbmodem14101"].map(String::from);
        assert_eq!(visible_ports(&found), vec!["/dev/cu.debug-console", "/dev/cu.Bluetooth-Incoming-Port", "/dev/cu.usbmodem14101"], "only the tty side of the listed system names is hidden, as QGCSerialPortInfo::isSystemPort matches");
        assert_eq!(crate::linkconfig::port_display_name("/dev/cu.SpeedyBeeF405Wing-SPP"), "cu.SpeedyBeeF405Wing-SPP");
    }

    use super::*;

    #[test]
    fn qt_framing_numbers_map_to_the_crate_settings() {
        assert_eq!((data_bits(7), data_bits(8), data_bits(99)), (DataBits::Seven, DataBits::Eight, DataBits::Eight));
        assert_eq!((parity(0), parity(2), parity(3)), (Parity::None, Parity::Even, Parity::Odd));
        assert_eq!((stop_bits(1), stop_bits(2)), (StopBits::One, StopBits::Two));
        assert_eq!((flow_control(0), flow_control(1), flow_control(2)), (FlowControl::None, FlowControl::Hardware, FlowControl::Software));
        let missing = SerialConfig { port_name: "/dev/does-not-exist".into(), baud: 57600, data_bits: 8, parity: 0, stop_bits: 1, flow_control: 0, usb_direct: false };
        assert!(SerialLink::open(&missing, |_| {}).is_err());
    }
}
