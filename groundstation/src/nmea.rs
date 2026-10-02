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
            }),
        }
    }
}

struct Listener {
    port: u16,
    running: Arc<AtomicBool>,
}

static LISTENER: Mutex<Option<Listener>> = Mutex::new(None);

pub fn wanted_port() -> Option<u16> {
    let source = crate::settingsstore::raw_setting("settings.autoConnectSettings.nmeaSource").and_then(|v| v.as_u64())?;
    (source == NMEA_SOURCE_UDP).then(|| {
        crate::settingsstore::raw_setting("settings.autoConnectSettings.nmeaUdpPort").and_then(|v| v.as_u64()).and_then(|p| u16::try_from(p).ok()).unwrap_or(DEFAULT_NMEA_UDP_PORT)
    })
}

fn listen(socket: UdpSocket, running: Arc<AtomicBool>) {
    let mut reader = Reader::default();
    let mut buffer = [0u8; 2048];
    while running.load(Ordering::Relaxed) {
        if let Ok(size) = socket.recv(&mut buffer) {
            String::from_utf8_lossy(&buffer[..size]).lines().filter_map(parse).for_each(|sentence| {
                if let Some(update) = reader.read(sentence) {
                    crate::gcsposition::report_nmea(update);
                }
            });
        }
    }
}

pub fn maintain() {
    let wanted = wanted_port();
    let mut current = LISTENER.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if current.as_ref().map(|l| l.port) == wanted {
        return;
    }
    if let Some(old) = current.take() {
        old.running.store(false, Ordering::Relaxed);
    }
    let bound = wanted.and_then(|port| {
        let socket = UdpSocket::bind(("0.0.0.0", port)).ok()?;
        socket.set_read_timeout(Some(std::time::Duration::from_millis(NMEA_POLL_MS))).ok()?;
        let running = Arc::new(AtomicBool::new(true));
        let thread_running = running.clone();
        std::thread::spawn(move || listen(socket, thread_running));
        Some(Listener { port, running })
    });
    crate::gcsposition::lock().use_nmea(bound.is_some());
    *current = bound;
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
    fn no_fix_or_a_bad_checksum_is_nothing() {
        assert_eq!(Reader::default().read(parse("$GPGGA,123519,4807.038,N,01131.000,E,0,08,0.9,545.4,M,46.9,M,,*46").unwrap()), None);
        assert_eq!(parse("$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*00"), None);
        assert_eq!(parse("$GPGGA,123519,4807.038,S,01131.000,W,1,08,0.9,545.4,M,46.9,M,,*48").map(|s| match s { Sentence::Gga { latitude, longitude, .. } => (latitude < 0.0, longitude < 0.0), _ => (false, false) }), Some((true, true)));
    }
}
