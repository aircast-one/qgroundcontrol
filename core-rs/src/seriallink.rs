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
    reader: Option<JoinHandle<()>>,
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
                        Ok(0) => {}
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
        Ok(SerialLink { port: Arc::new(Mutex::new(port)), stop, reader: Some(reader) })
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

    pub fn close(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

impl Drop for SerialLink {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
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
