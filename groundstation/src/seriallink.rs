use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};

const PORT_TIMEOUT: Duration = Duration::from_millis(200);

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

fn not_found(config: &SerialConfig, error: impl std::fmt::Display) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("{}: {}", config.port_name, error),
    )
}

#[cfg(windows)]
const RTS_CONTROL: u32 = 0b11 << 12;
#[cfg(windows)]
const RTS_CONTROL_ENABLE: u32 = 0b01 << 12;
#[cfg(windows)]
const ABORT_ON_ERROR: u32 = 1 << 14;

#[cfg(windows)]
fn full_duplex(config: &SerialConfig) -> io::Result<serial2::SerialPort> {
    let flow = flow_control(config.flow_control);
    let mut port =
        serial2::SerialPort::open(&config.port_name, |mut settings: serial2::Settings| {
            settings.set_raw();
            settings.set_baud_rate(config.baud)?;
            settings.set_char_size(match data_bits(config.data_bits) {
                DataBits::Five => serial2::CharSize::Bits5,
                DataBits::Six => serial2::CharSize::Bits6,
                DataBits::Seven => serial2::CharSize::Bits7,
                DataBits::Eight => serial2::CharSize::Bits8,
            });
            settings.set_parity(match parity(config.parity) {
                Parity::None => serial2::Parity::None,
                Parity::Odd => serial2::Parity::Odd,
                Parity::Even => serial2::Parity::Even,
            });
            settings.set_stop_bits(match stop_bits(config.stop_bits) {
                StopBits::One => serial2::StopBits::One,
                StopBits::Two => serial2::StopBits::Two,
            });
            settings.set_flow_control(match flow {
                FlowControl::None => serial2::FlowControl::None,
                FlowControl::Software => serial2::FlowControl::XonXoff,
                FlowControl::Hardware => serial2::FlowControl::RtsCts,
            });
            let dcb = settings.as_raw_dbc_mut();
            dcb.XonChar = 17;
            dcb.XoffChar = 19;
            dcb._bitfield = dcb._bitfield & !(ABORT_ON_ERROR | RTS_CONTROL)
                | if flow == FlowControl::Hardware {
                    RTS_CONTROL_ENABLE
                } else {
                    0
                };
            Ok(settings)
        })?;
    port.set_read_timeout(PORT_TIMEOUT)?;
    port.set_write_timeout(PORT_TIMEOUT)?;
    let _ = port.set_dtr(true);
    Ok(port)
}

pub struct SerialLink {
    port: Mutex<Box<dyn Write + Send>>,
    stop: Arc<AtomicBool>,
    reader: Mutex<Option<JoinHandle<()>>>,
}

impl SerialLink {
    #[cfg(not(windows))]
    pub fn open(
        config: &SerialConfig,
        sink: impl FnMut(Event) + Send + 'static,
    ) -> io::Result<SerialLink> {
        let port = serialport::new(&config.port_name, config.baud)
            .data_bits(data_bits(config.data_bits))
            .parity(parity(config.parity))
            .stop_bits(stop_bits(config.stop_bits))
            .flow_control(flow_control(config.flow_control))
            .timeout(PORT_TIMEOUT)
            .open()
            .map_err(|e| not_found(config, e))?;
        Self::from_port(port, sink)
    }

    #[cfg(windows)]
    pub fn open(
        config: &SerialConfig,
        sink: impl FnMut(Event) + Send + 'static,
    ) -> io::Result<SerialLink> {
        let port = full_duplex(config).map_err(|e| not_found(config, e))?;
        Self::from_halves(port.try_clone()?, port, sink)
    }

    pub fn from_port(
        mut port: Box<dyn SerialPort>,
        sink: impl FnMut(Event) + Send + 'static,
    ) -> io::Result<SerialLink> {
        port.set_timeout(PORT_TIMEOUT)
            .map_err(|e| io::Error::other(e.to_string()))?;
        let _ = port.write_data_terminal_ready(true);
        let reader_port = port
            .try_clone()
            .map_err(|e| io::Error::other(e.to_string()))?;
        Self::from_halves(reader_port, port, sink)
    }

    pub fn from_halves(
        mut reader_port: impl Read + Send + 'static,
        port: impl Write + Send + 'static,
        mut sink: impl FnMut(Event) + Send + 'static,
    ) -> io::Result<SerialLink> {
        let stop = Arc::new(AtomicBool::new(false));
        let reader = {
            let stop = Arc::clone(&stop);
            std::thread::Builder::new()
                .name("qgc-serial".into())
                .spawn(move || {
                    let mut buffer = vec![0u8; 4096];
                    while !stop.load(Ordering::Relaxed) {
                        match reader_port.read(&mut buffer) {
                            Ok(0) => {
                                sink(Event::Disconnected("device returned no data".into()));
                                return;
                            }
                            Ok(len) => sink(Event::Bytes(buffer[..len].to_vec())),
                            Err(e)
                                if matches!(
                                    e.kind(),
                                    io::ErrorKind::WouldBlock
                                        | io::ErrorKind::TimedOut
                                        | io::ErrorKind::Interrupted
                                ) => {}
                            Err(e) => {
                                sink(Event::Disconnected(e.to_string()));
                                return;
                            }
                        }
                    }
                })?
        };
        Ok(SerialLink {
            port: Mutex::new(Box::new(port)),
            stop,
            reader: Mutex::new(Some(reader)),
        })
    }

    pub fn write(&self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Data to Send is Empty",
            ));
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

const MACOS_SYSTEM_PORTS: [&str; 5] = [
    "tty.MALS",
    "tty.SOC",
    "tty.Bluetooth-Incoming-Port",
    "tty.usbserial",
    "tty.usbmodem",
];
const MACOS_BAUD_RATES: [u32; 25] = [
    50, 75, 110, 134, 150, 200, 300, 600, 1200, 1800, 2400, 4800, 7200, 9600, 14400, 19200, 28800,
    38400, 57600, 76800, 115200, 230400, 460800, 500000, 921600,
];

const WINDOWS_BAUD_RATES: [u32; 19] = [
    110, 300, 600, 1200, 2400, 4800, 9600, 14400, 19200, 38400, 56000, 57600, 115200, 128000,
    230400, 256000, 460800, 500000, 921600,
];

fn baud_rates(os: &str) -> Option<&'static [u32]> {
    match os {
        "macos" => Some(&MACOS_BAUD_RATES),
        "windows" => Some(&WINDOWS_BAUD_RATES),
        _ => None,
    }
}

fn lists_ports(os: &str) -> bool {
    baud_rates(os).is_some()
}

pub fn visible_ports(system_locations: &[String]) -> Vec<String> {
    system_locations
        .iter()
        .filter(|port| {
            !MACOS_SYSTEM_PORTS
                .iter()
                .any(|system| port.contains(system))
        })
        .cloned()
        .collect()
}

pub fn links_field(field: &str) -> Option<serde_json::Value> {
    let ports = || -> Option<Vec<String>> {
        lists_ports(std::env::consts::OS).then_some(())?;
        Some(visible_ports(
            &serialport::available_ports()
                .ok()?
                .into_iter()
                .map(|p| p.port_name)
                .collect::<Vec<_>>(),
        ))
    };
    Some(match field {
        "linkTypeStrings" | "linkTypeIds" => crate::linkconfig::link_type_field(field)?,
        "serialBaudRates" => serde_json::json!(
            baud_rates(std::env::consts::OS)?
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
        ),
        "serialPorts" => serde_json::json!(ports()?),
        "serialPortStrings" => serde_json::json!(
            ports()?
                .iter()
                .map(|p| crate::linkconfig::port_display_name(p))
                .collect::<Vec<_>>()
        ),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn macos_system_ports_are_hidden_and_a_port_shows_by_its_device_name() {
        let found = [
            "/dev/cu.debug-console",
            "/dev/tty.Bluetooth-Incoming-Port",
            "/dev/cu.Bluetooth-Incoming-Port",
            "/dev/tty.usbmodem14101",
            "/dev/cu.usbmodem14101",
        ]
        .map(String::from);
        assert_eq!(
            visible_ports(&found),
            vec![
                "/dev/cu.debug-console",
                "/dev/cu.Bluetooth-Incoming-Port",
                "/dev/cu.usbmodem14101"
            ],
            "only the tty side of the listed system names is hidden, as QGCSerialPortInfo::isSystemPort matches"
        );
        assert_eq!(
            crate::linkconfig::port_display_name("/dev/cu.SpeedyBeeF405Wing-SPP"),
            "cu.SpeedyBeeF405Wing-SPP"
        );
    }

    #[test]
    fn windows_lists_its_com_ports_with_the_rates_qgc_offers_there() {
        assert!(lists_ports("windows") && lists_ports("macos") && !lists_ports("linux"));
        assert_eq!(
            visible_ports(&["COM3".to_string()]),
            vec!["COM3"],
            "QGCSerialPortInfo::isSystemPort hides ports on macOS only"
        );
        assert_eq!(crate::linkconfig::port_display_name("COM3"), "COM3");
        let rates = baud_rates("windows").unwrap();
        assert!(
            [14400, 56000, 57600, 128000, 256000, 921600]
                .iter()
                .all(|rate| rates.contains(rate)),
            "SerialConfiguration::supportedBaudRates adds the Q_OS_WIN rates to QSerialPortInfo::standardBaudRates"
        );
        assert!(
            !rates.contains(&50) && rates.is_sorted(),
            "the Q_OS_UNIX rates are not offered on Windows"
        );
    }

    use super::*;

    #[test]
    fn qt_framing_numbers_map_to_the_crate_settings() {
        assert_eq!(
            (data_bits(7), data_bits(8), data_bits(99)),
            (DataBits::Seven, DataBits::Eight, DataBits::Eight)
        );
        assert_eq!(
            (parity(0), parity(2), parity(3)),
            (Parity::None, Parity::Even, Parity::Odd)
        );
        assert_eq!((stop_bits(1), stop_bits(2)), (StopBits::One, StopBits::Two));
        assert_eq!(
            (flow_control(0), flow_control(1), flow_control(2)),
            (
                FlowControl::None,
                FlowControl::Hardware,
                FlowControl::Software
            )
        );
        let missing = SerialConfig {
            port_name: "/dev/does-not-exist".into(),
            baud: 57600,
            data_bits: 8,
            parity: 0,
            stop_bits: 1,
            flow_control: 0,
            usb_direct: false,
        };
        assert!(SerialLink::open(&missing, |_| {}).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn a_write_reaches_the_radio_while_the_reader_waits_for_it_like_qserialport_overlapped_io() {
        use std::os::windows::fs::OpenOptionsExt;
        use std::os::windows::io::{FromRawHandle, OwnedHandle};
        use std::sync::mpsc;
        use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
        use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX};
        use windows_sys::Win32::System::Pipes::{CreateNamedPipeW, PIPE_TYPE_BYTE};

        let name = format!(r"\\.\pipe\qgc-serial-{}", std::process::id());
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let server = unsafe {
            CreateNamedPipeW(
                wide.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_BYTE,
                1,
                4096,
                4096,
                0,
                std::ptr::null(),
            )
        };
        assert_ne!(server, INVALID_HANDLE_VALUE);
        let radio = serial2::SerialPort::from(unsafe { OwnedHandle::from_raw_handle(server) });
        let app = serial2::SerialPort::from(OwnedHandle::from(
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(FILE_FLAG_OVERLAPPED)
                .open(&name)
                .unwrap(),
        ));
        let (heard_tx, heard) = mpsc::channel();
        let link = Arc::new(
            SerialLink::from_halves(app.try_clone().unwrap(), app, move |event| {
                let _ = heard_tx.send(event);
            })
            .unwrap(),
        );
        (&radio).write_all(b"$").unwrap();
        assert_eq!(
            heard.recv_timeout(Duration::from_secs(2)),
            Ok(Event::Bytes(b"$".to_vec()))
        );
        let (sent_tx, sent) = mpsc::channel();
        let writer = Arc::clone(&link);
        std::thread::spawn(move || sent_tx.send(writer.write(b"rc").is_ok()));
        let (got_tx, got) = mpsc::channel();
        let listener = radio.try_clone().unwrap();
        std::thread::spawn(move || {
            let mut bytes = [0u8; 2];
            let _ = got_tx.send(
                (&listener)
                    .read_exact(&mut bytes)
                    .ok()
                    .map(|_| bytes.to_vec()),
            );
        });
        assert_eq!(
            got.recv_timeout(Duration::from_secs(2)),
            Ok(Some(b"rc".to_vec())),
            "a synchronous COM handle queues WriteFile behind the reader's pending ReadFile, so RC and gimbal writes waited for the next downlink byte"
        );
        assert_eq!(sent.recv_timeout(Duration::from_secs(2)), Ok(true));
        drop(radio);
        assert!(matches!(
            heard.recv_timeout(Duration::from_secs(2)),
            Ok(Event::Disconnected(_))
        ));
        link.close();
    }
}
