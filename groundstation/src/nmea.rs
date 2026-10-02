use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::gcsposition::Update;

const USER_EQUIVALENT_RANGE_ERROR_M: f64 = 5.1;
const NMEA_SOURCE_UDP: u64 = 1;
const DEFAULT_NMEA_UDP_PORT: u16 = 14401;
const NMEA_POLL_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sentence {
    Gga { latitude: f64, longitude: f64, altitude: Option<f64>, quality: u8, hdop: Option<f64> },
    Gsa { vdop: Option<f64> },
}

fn checksum_ok(line: &str) -> bool {
    let Some((body, sum)) = line.strip_prefix('$').and_then(|rest| rest.split_once('*')) else { return false };
    u8::from_str_radix(sum.trim(), 16).is_ok_and(|expected| body.bytes().fold(0u8, |acc, b| acc ^ b) == expected)
}

fn degrees(value: &str, hemisphere: &str, whole_digits: usize) -> Option<f64> {
    let whole: f64 = value.get(..whole_digits)?.parse().ok()?;
    let minutes: f64 = value.get(whole_digits..)?.parse().ok()?;
    let unsigned = whole + minutes / 60.0;
    match hemisphere {
        "N" | "E" => Some(unsigned),
        "S" | "W" => Some(-unsigned),
        _ => None,
    }
}

pub fn parse(line: &str) -> Option<Sentence> {
    let line = line.trim();
    checksum_ok(line).then_some(())?;
    let fields: Vec<&str> = line[1..line.find('*')?].split(',').collect();
    let number = |i: usize| fields.get(i).and_then(|f| f.parse::<f64>().ok());
    match fields.first()?.get(2..)? {
        "GGA" => Some(Sentence::Gga {
            latitude: degrees(fields.get(2)?, fields.get(3)?, 2)?,
            longitude: degrees(fields.get(4)?, fields.get(5)?, 3)?,
            altitude: number(9),
            quality: fields.get(6)?.parse().ok()?,
            hdop: number(8),
        }),
        "GSA" => Some(Sentence::Gsa { vdop: number(17) }),
        _ => None,
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Reader {
    vdop: Option<f64>,
}

impl Reader {
    pub fn read(&mut self, sentence: Sentence) -> Option<Update> {
        match sentence {
            Sentence::Gsa { vdop } => {
                self.vdop = vdop;
                None
            }
            Sentence::Gga { quality: 0, .. } => None,
            Sentence::Gga { latitude, longitude, altitude, hdop, .. } => Some(Update {
                latitude: Some(latitude),
                longitude: Some(longitude),
                altitude,
                horizontal_accuracy_m: hdop.map(|h| h * USER_EQUIVALENT_RANGE_ERROR_M),
                vertical_accuracy_m: self.vdop.map(|v| v * USER_EQUIVALENT_RANGE_ERROR_M),
                direction_deg: None,
                direction_accuracy_deg: None,
                ground_speed_m_s: None,
            }),
        }
    }
}

#[derive(Debug, Default)]
pub struct Lines {
    pending: String,
    reader: Reader,
}

impl Lines {
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Update> {
        self.pending.push_str(&String::from_utf8_lossy(bytes));
        let complete = self.pending.rfind('\n').map(|end| self.pending.drain(..=end).collect::<String>()).unwrap_or_default();
        if self.pending.len() > MAX_NMEA_LINE_BYTES {
            self.pending.clear();
        }
        complete.lines().filter_map(parse).filter_map(|sentence| self.reader.read(sentence)).collect()
    }
}

const MAX_NMEA_LINE_BYTES: usize = 4096;
const NMEA_SOURCE_SERIAL: u64 = 2;
const NMEA_SERIAL_ID: u32 = 0xfff0_0003;
const DEFAULT_NMEA_BAUD: u32 = 4800;

#[derive(Debug, Clone, PartialEq)]
pub enum Wanted {
    Udp(u16),
    Serial(String, u32),
}

enum Running {
    Udp(Arc<AtomicBool>),
    Serial { close: Box<dyn Fn() + Send>, dead: Arc<AtomicBool> },
}

impl Running {
    fn alive(&self) -> bool {
        match self {
            Running::Udp(_) => true,
            Running::Serial { dead, .. } => !dead.load(Ordering::Relaxed),
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        match self {
            Running::Udp(flag) => flag.store(false, Ordering::Relaxed),
            Running::Serial { close, .. } => close(),
        }
    }
}

struct Attempt {
    wanted: Wanted,
    running: Option<Running>,
    at: std::time::Instant,
}

const RETRY_AFTER: std::time::Duration = std::time::Duration::from_secs(1);

static LISTENER: Mutex<Option<Attempt>> = Mutex::new(None);

fn setting(name: &str) -> Option<serde_json::Value> {
    crate::settingsstore::raw_setting(&format!("settings.autoConnectSettings.{name}"))
}

pub fn wanted() -> Option<Wanted> {
    match setting("nmeaSource").and_then(|v| v.as_u64())? {
        NMEA_SOURCE_UDP => Some(Wanted::Udp(setting("nmeaUdpPort").and_then(|v| v.as_u64()).and_then(|p| u16::try_from(p).ok()).unwrap_or(DEFAULT_NMEA_UDP_PORT))),
        NMEA_SOURCE_SERIAL => {
            let port = setting("autoConnectNmeaPort").and_then(|v| v.as_str().map(str::to_string)).filter(|p| !p.trim().is_empty())?;
            Some(Wanted::Serial(port, setting("autoConnectNmeaBaud").and_then(|v| v.as_u64()).and_then(|b| u32::try_from(b).ok()).unwrap_or(DEFAULT_NMEA_BAUD)))
        }
        _ => None,
    }
}

fn listen_udp(socket: UdpSocket, running: Arc<AtomicBool>) {
    let mut lines = Lines::default();
    let mut buffer = [0u8; 2048];
    while running.load(Ordering::Relaxed) {
        if let Ok(size) = socket.recv(&mut buffer) {
            lines.feed(&buffer[..size]).into_iter().for_each(crate::gcsposition::report_nmea);
        }
    }
}

fn open_udp(port: u16) -> Option<Running> {
    let socket = UdpSocket::bind(("0.0.0.0", port)).ok()?;
    socket.set_read_timeout(Some(std::time::Duration::from_millis(NMEA_POLL_MS))).ok()?;
    let running = Arc::new(AtomicBool::new(true));
    let thread_running = running.clone();
    std::thread::spawn(move || listen_udp(socket, thread_running));
    Some(Running::Udp(running))
}

#[cfg(target_os = "android")]
fn open_serial(port: &str, baud: u32) -> Option<Running> {
    let lines = Mutex::new(Lines::default());
    let dead = Arc::new(AtomicBool::new(false));
    let lost = dead.clone();
    let opened = crate::platformserial::PlatformSerial::open(NMEA_SERIAL_ID, port, baud, 8, 1, 0, move |event| match event {
        crate::platformserial::Event::Bytes(bytes) => lines.lock().unwrap_or_else(std::sync::PoisonError::into_inner).feed(&bytes).into_iter().for_each(crate::gcsposition::report_nmea),
        crate::platformserial::Event::Disconnected(_) => lost.store(true, Ordering::Relaxed),
    })
    .ok()?;
    Some(Running::Serial { close: Box::new(move || opened.close()), dead })
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn open_serial(port: &str, baud: u32) -> Option<Running> {
    let config = crate::seriallink::SerialConfig { port_name: port.to_string(), baud, data_bits: 8, parity: 0, stop_bits: 1, flow_control: 0, usb_direct: false };
    let mut lines = Lines::default();
    let dead = Arc::new(AtomicBool::new(false));
    let lost = dead.clone();
    let link = crate::seriallink::SerialLink::open(&config, move |event| match event {
        crate::seriallink::Event::Bytes(bytes) => lines.feed(&bytes).into_iter().for_each(crate::gcsposition::report_nmea),
        crate::seriallink::Event::Disconnected(_) => lost.store(true, Ordering::Relaxed),
    })
    .ok()?;
    Some(Running::Serial { close: Box::new(move || link.close()), dead })
}

#[cfg(target_os = "ios")]
fn open_serial(_port: &str, _baud: u32) -> Option<Running> {
    None
}

pub fn maintain() {
    let wanted = wanted();
    let mut current = LISTENER.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let alive = |attempt: &Attempt| attempt.running.as_ref().is_some_and(Running::alive);
    let settled = current.as_ref().is_some_and(|attempt| Some(&attempt.wanted) == wanted.as_ref() && (alive(attempt) || attempt.at.elapsed() < RETRY_AFTER));
    if settled || (current.is_none() && wanted.is_none()) {
        crate::gcsposition::lock().use_nmea(current.as_ref().is_some_and(alive));
        return;
    }
    *current = None;
    let attempt = wanted.map(|wanted| {
        let running = match &wanted {
            Wanted::Udp(port) => open_udp(*port),
            Wanted::Serial(port, baud) => open_serial(port, *baud),
        };
        Attempt { wanted, running, at: std::time::Instant::now() }
    });
    crate::gcsposition::lock().use_nmea(attempt.as_ref().is_some_and(alive));
    *current = attempt;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gga_fix_becomes_a_position_with_qts_accuracy() {
        let gga = parse("$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47").unwrap();
        let mut reader = Reader::default();
        reader.read(parse("$GPGSA,A,3,04,05,,09,12,,,24,,,,,2.5,1.3,2.1*39").unwrap());
        let update = reader.read(gga).unwrap();
        assert!((update.latitude.unwrap() - 48.1173).abs() < 1e-4);
        assert!((update.longitude.unwrap() - 11.516_666).abs() < 1e-4);
        assert_eq!(update.altitude, Some(545.4));
        assert!((update.horizontal_accuracy_m.unwrap() - 0.9 * 5.1).abs() < 1e-9, "QGCPositionManager sets a 5.1 m user equivalent range error on its NMEA source");
        assert!((update.vertical_accuracy_m.unwrap() - 2.1 * 5.1).abs() < 1e-9);
    }

    #[test]
    fn sentences_split_across_serial_reads_are_joined() {
        let mut lines = Lines::default();
        assert!(lines.feed(b"$GPGGA,123519,4807.038,N,01131.0").is_empty());
        let fixes = lines.feed(b"00,E,1,08,0.9,545.4,M,46.9,M,,*47\r\n$GPGGA,1235");
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].altitude, Some(545.4));
    }

    #[test]
    fn no_fix_or_a_bad_checksum_is_nothing() {
        assert_eq!(Reader::default().read(parse("$GPGGA,123519,4807.038,N,01131.000,E,0,08,0.9,545.4,M,46.9,M,,*46").unwrap()), None);
        assert_eq!(parse("$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*00"), None);
        assert_eq!(parse("$GPGGA,123519,4807.038,S,01131.000,W,1,08,0.9,545.4,M,46.9,M,,*48").map(|s| match s { Sentence::Gga { latitude, longitude, .. } => (latitude < 0.0, longitude < 0.0), _ => (false, false) }), Some((true, true)));
    }
}
