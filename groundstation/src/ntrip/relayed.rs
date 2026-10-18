use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use base64::Engine;
use web_time::Instant;

use super::{Config, DATA_WATCHDOG, Framer, GgaSchedule, Response, SOURCE_TABLE_MAX_BYTES, SOURCE_TABLE_TIMEOUT, datagram_frames, find, forward, inject, lock, response, send_gga};
use crate::hostsocket::{Event, Follower, HostSocket};

pub const STEP: Duration = Duration::from_millis(200);
const INPUT_RETRY: Duration = Duration::from_secs(5);
const PEER_CLOSED_BEFORE_HEADER: &str = "Server disconnected (peer closed before HTTP response; check mountpoint and credentials)";

#[derive(Default)]
struct Inbox {
    bytes: Vec<u8>,
    closed: Option<String>,
}

struct Connection {
    socket: HostSocket,
    inbox: Arc<Mutex<Inbox>>,
}

impl Connection {
    fn open(url: Option<String>) -> Result<Connection, String> {
        let url = url.ok_or_else(|| crate::device::NEEDS_DEVICE.to_string())?;
        let inbox = Arc::new(Mutex::new(Inbox::default()));
        let filled = inbox.clone();
        let socket = HostSocket::open(&url, move |event| {
            let mut inbox = filled.lock().unwrap_or_else(PoisonError::into_inner);
            match event {
                Event::Opened => {}
                Event::Bytes(bytes) => inbox.bytes.extend(bytes),
                Event::Closed(reason) => inbox.closed = Some(reason),
            }
        })?;
        Ok(Connection { socket, inbox })
    }

    fn take(&self) -> (Vec<u8>, Option<String>) {
        let mut inbox = self.inbox.lock().unwrap_or_else(PoisonError::into_inner);
        (std::mem::take(&mut inbox.bytes), inbox.closed.take())
    }
}

fn caster(config: &Config) -> Option<String> {
    match config.use_tls {
        true => crate::device::tls(&config.host, config.port, config.allow_self_signed),
        false => crate::device::tcp(&config.host, config.port),
    }
}

fn udp_target() -> Option<HostSocket> {
    let target = lock().udp_target.clone()?;
    let (host, port) = target.rsplit_once(':')?;
    let url = crate::device::udp(0, &[(host.to_string(), port.parse().ok()?)])?;
    HostSocket::open(&url, |event| {
        if let Event::Closed(reason) = event {
            log::warn!("NTRIP UDP forward through the device stopped: {reason}");
        }
    })
    .ok()
}

enum Phase {
    Idle,
    Waiting(Instant),
    Header(Vec<u8>),
    Streaming { framer: Framer, schedule: GgaSchedule },
}

pub struct Session {
    config: Config,
    generation: u64,
    connection: Option<Connection>,
    forward_udp: Option<HostSocket>,
    phase: Phase,
    heard: Instant,
}

impl Session {
    pub fn new(config: Config, generation: u64) -> Session {
        Session { config, generation, connection: None, forward_udp: None, phase: Phase::Idle, heard: Instant::now() }
    }

    pub fn step(&mut self, now: Instant) -> bool {
        if lock().generation != self.generation {
            return true;
        }
        match self.phase {
            Phase::Idle => self.open(now),
            Phase::Waiting(at) if now < at => false,
            Phase::Waiting(_) if lock().reconnecting(self.generation) => self.open(now),
            Phase::Waiting(_) => true,
            Phase::Header(_) | Phase::Streaming { .. } => match self.exchange(now) {
                Ok(done) => done,
                Err((fatal, detail)) => self.fail(now, fatal, &detail),
            },
        }
    }

    fn open(&mut self, now: Instant) -> bool {
        match Connection::open(caster(&self.config)) {
            Ok(connection) => {
                connection.socket.send(&self.config.request());
                self.connection = Some(connection);
                self.forward_udp = udp_target();
                self.phase = Phase::Header(Vec::new());
                self.heard = now;
                false
            }
            Err(detail) => self.fail(now, false, &detail),
        }
    }

    fn fail(&mut self, now: Instant, fatal: bool, detail: &str) -> bool {
        self.connection = None;
        self.forward_udp = None;
        match lock().failed(self.generation, fatal, detail) {
            Some(backoff_ms) => {
                self.phase = Phase::Waiting(now + Duration::from_millis(backoff_ms));
                false
            }
            None => true,
        }
    }

    fn exchange(&mut self, now: Instant) -> Result<bool, (bool, String)> {
        let generation = self.generation;
        let Some(connection) = self.connection.as_ref() else { return Err((false, "Server disconnected".to_string())) };
        let (bytes, closed) = connection.take();
        if !bytes.is_empty() {
            self.heard = now;
        }
        let forward_udp = self.forward_udp.as_ref();
        let to_udp = |frame: &[u8]| {
            if let Some(socket) = forward_udp {
                socket.send(frame);
            }
        };
        let reply = match &mut self.phase {
            Phase::Header(header) => {
                header.extend_from_slice(&bytes);
                Some(response(header))
            }
            Phase::Streaming { framer, .. } => {
                if !bytes.is_empty() && !forward(generation, framer, &bytes, &to_udp) {
                    return Ok(true);
                }
                None
            }
            _ => None,
        };
        match reply {
            Some(Response::Failed { fatal, message }) => return Err((fatal, message)),
            Some(Response::Connected(rest)) => {
                lock().connected(generation);
                let mut framer = Framer::default();
                if !forward(generation, &mut framer, &rest, &to_udp) {
                    return Ok(true);
                }
                self.phase = Phase::Streaming { framer, schedule: GgaSchedule::default() };
            }
            Some(Response::Pending) | None => {}
        }
        if let Phase::Streaming { schedule, .. } = &mut self.phase {
            let mut write = |gga: &[u8]| if connection.socket.send(gga) { Ok(()) } else { Err("Could not send the position to the caster".to_string()) };
            send_gga(&mut write, schedule, generation).map_err(|detail| (false, detail))?;
        }
        let streaming = matches!(self.phase, Phase::Streaming { .. });
        match closed {
            Some(reason) if !reason.is_empty() => Err((false, reason)),
            Some(_) if streaming => Err((false, "Server disconnected".to_string())),
            Some(_) => Err((false, PEER_CLOSED_BEFORE_HEADER.to_string())),
            None if now.duration_since(self.heard) >= DATA_WATCHDOG => Err((false, format!("No data received for {} seconds", DATA_WATCHDOG.as_secs()))),
            None => Ok(false),
        }
    }
}

pub fn udp_input_follower(port: u16, validate: bool, generation: u64) -> Follower {
    let framer = Mutex::new(Framer::default());
    Follower::new(INPUT_RETRY, move || crate::device::udp(port, &[]), move || super::udp_input().generation == generation, move |event| match event {
        Event::Bytes(datagram) => datagram_frames(&mut framer.lock().unwrap_or_else(PoisonError::into_inner), &datagram, validate).iter().filter(|rtcm| !rtcm.is_empty()).for_each(|rtcm| inject(rtcm)),
        Event::Closed(reason) if reason == crate::device::NEEDS_DEVICE => log::debug!("UDP RTCM input on port {port}: {reason}"),
        Event::Closed(reason) => log::warn!("UDP RTCM input on port {port} through the device: {reason}"),
        Event::Opened => {}
    })
}

fn source_table_request(config: &Config) -> Vec<u8> {
    let auth = (!config.username.is_empty() || !config.password.is_empty())
        .then(|| format!("Authorization: Basic {}\r\n", base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", config.username, config.password))))
        .unwrap_or_default();
    format!("GET / HTTP/1.1\r\nHost: {}\r\nNtrip-Version: Ntrip/2.0\r\nUser-Agent: QGC-NTRIP\r\n{auth}Connection: close\r\n\r\n", config.host).into_bytes()
}

fn dechunked(rest: &[u8]) -> Vec<u8> {
    let Some(end) = find(rest, b"\r\n") else { return Vec::new() };
    let size = usize::from_str_radix(String::from_utf8_lossy(&rest[..end]).split(';').next().unwrap_or_default().trim(), 16).unwrap_or(0);
    let start = end + 2;
    match size {
        0 => Vec::new(),
        _ if rest.len() < start + size => rest[start..].to_vec(),
        _ => [&rest[start..start + size], dechunked(rest.get(start + size + 2..).unwrap_or_default()).as_slice()].concat(),
    }
}

pub fn source_table_body(raw: &[u8]) -> Result<String, String> {
    let end = find(raw, b"\r\n\r\n").ok_or("Response does not contain a valid source table")?;
    let header = String::from_utf8_lossy(&raw[..end]).to_ascii_lowercase();
    let status = header.lines().next().unwrap_or_default().to_string();
    if !status.split_whitespace().nth(1).is_some_and(|code| code.starts_with('2')) {
        return Err(format!("Source table request failed: {}", status.trim()));
    }
    let body = match header.contains("transfer-encoding: chunked") {
        true => dechunked(&raw[end + 4..]),
        false => raw[end + 4..].to_vec(),
    };
    let body = String::from_utf8_lossy(&body).to_string();
    match body.contains("ENDSOURCETABLE") {
        true => Ok(body),
        false => Err("Response does not contain a valid source table".to_string()),
    }
}

type Finish = Box<dyn FnOnce(Result<String, String>) + Send>;

pub struct SourceTable {
    connection: Result<Connection, String>,
    raw: Vec<u8>,
    deadline: Instant,
    finish: Option<Finish>,
}

impl SourceTable {
    pub fn new(config: &Config, finish: impl FnOnce(Result<String, String>) + Send + 'static) -> SourceTable {
        let connection = Connection::open(caster(config)).inspect(|connection| {
            connection.socket.send(&source_table_request(config));
        });
        SourceTable { connection, raw: Vec::new(), deadline: Instant::now() + SOURCE_TABLE_TIMEOUT, finish: Some(Box::new(finish)) }
    }

    pub fn step(&mut self, now: Instant) -> bool {
        let outcome = match &self.connection {
            Err(reason) => Some(Err(reason.clone())),
            Ok(connection) => {
                let (bytes, closed) = connection.take();
                self.raw.extend_from_slice(&bytes);
                match () {
                    _ if self.raw.len() as u64 > SOURCE_TABLE_MAX_BYTES => Some(Err(format!("Source table too large (exceeds {} MB)", SOURCE_TABLE_MAX_BYTES / (1024 * 1024)))),
                    _ if find(&self.raw, b"ENDSOURCETABLE").is_some() => Some(source_table_body(&self.raw)),
                    _ if closed.as_ref().is_some_and(|reason| !reason.is_empty()) => closed.map(Err),
                    _ if closed.is_some() => Some(source_table_body(&self.raw)),
                    _ if now >= self.deadline => Some(Err("Timed out waiting for the source table".to_string())),
                    _ => None,
                }
            }
        };
        match outcome {
            Some(outcome) => {
                if let Some(finish) = self.finish.take() {
                    finish(outcome);
                }
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caster_config() -> Config {
        Config { host: "caster.example".into(), port: 2101, username: "u".into(), password: "p".into(), mountpoint: "MOUNT".into(), whitelist: Vec::new(), use_tls: false, allow_self_signed: false }
    }

    #[test]
    fn a_chunked_v2_source_table_reads_as_the_plain_text() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n6\r\nSTR;A;\r\n12\r\nB;\r\nENDSOURCETABLE\r\n0\r\n\r\n";
        assert_eq!(source_table_body(raw).unwrap(), "STR;A;B;\r\nENDSOURCETABLE");
        let v1 = b"SOURCETABLE 200 OK\r\nContent-Type: text/plain\r\n\r\nSTR;MOUNT;\r\nENDSOURCETABLE\r\n";
        assert_eq!(source_table_body(v1).unwrap(), "STR;MOUNT;\r\nENDSOURCETABLE\r\n");
        assert_eq!(source_table_body(b"HTTP/1.1 401 Unauthorized\r\n\r\n").unwrap_err(), "Source table request failed: http/1.1 401 unauthorized");
    }

    #[test]
    fn a_browser_on_the_device_fetches_the_source_table_through_its_relay() {
        let _host = crate::hostsocket::fake::install();
        let _pairing = crate::device::TEST_PAIRING.lock().unwrap_or_else(PoisonError::into_inner);
        crate::device::pair("http://falcon.local/fly/").unwrap();
        let fetched = Arc::new(Mutex::new(None));
        let into = fetched.clone();
        let mut table = SourceTable::new(&caster_config(), move |outcome| *into.lock().unwrap() = Some(outcome));
        let (id, url) = crate::hostsocket::fake::last_opened().unwrap();
        assert_eq!(url, "ws://falcon.local/fly/socket?tcp=caster.example%3A2101");
        assert!(String::from_utf8(crate::hostsocket::fake::sent_to(id)).unwrap().starts_with("GET / HTTP/1.1\r\nHost: caster.example\r\nNtrip-Version: Ntrip/2.0\r\n"));
        assert!(!table.step(Instant::now()));
        crate::hostsocket::received(id, b"SOURCETABLE 200 OK\r\n\r\nSTR;MOUNT;\r\n".to_vec());
        assert!(!table.step(Instant::now()), "half a table is not a table");
        crate::hostsocket::received(id, b"ENDSOURCETABLE\r\n".to_vec());
        assert!(table.step(Instant::now()));
        assert_eq!(fetched.lock().unwrap().clone(), Some(Ok("STR;MOUNT;\r\nENDSOURCETABLE\r\n".to_string())));
        crate::device::unpair();
    }

    #[test]
    fn a_session_through_the_relay_connects_streams_and_backs_off_when_the_caster_hangs_up() {
        let _host = crate::hostsocket::fake::install();
        let _pairing = crate::device::TEST_PAIRING.lock().unwrap_or_else(PoisonError::into_inner);
        crate::device::pair("http://falcon.local/fly/").unwrap();
        let generation = lock().retarget(Some(Config { use_tls: true, allow_self_signed: true, ..caster_config() })).map(|(_, generation)| generation).unwrap();
        let mut session = Session::new(Config { use_tls: true, allow_self_signed: true, ..caster_config() }, generation);
        let start = Instant::now();
        assert!(!session.step(start));
        let (id, url) = crate::hostsocket::fake::last_opened().unwrap();
        assert_eq!(url, "ws://falcon.local/fly/socket?tls=caster.example%3A2101&selfsigned=1");
        assert!(crate::hostsocket::fake::sent_to(id).starts_with(b"GET /MOUNT HTTP/1.1\r\n"));

        crate::hostsocket::received(id, b"ICY 200 OK\r\n".to_vec());
        assert!(!session.step(start));
        assert_eq!(lock().snapshot(0)["status"], "connected");

        crate::hostsocket::closed(id, "");
        assert!(!session.step(start));
        let status = lock().snapshot(0);
        assert_eq!((status["status"].as_str(), status["statusMessage"].as_str()), (Some("reconnecting"), Some("Reconnecting in 1s: Server disconnected")));
        assert!(!session.step(start + Duration::from_millis(999)));
        assert_eq!(crate::hostsocket::fake::last_opened().unwrap().0, id, "it waits out the backoff");
        assert!(!session.step(start + Duration::from_secs(1)));
        assert_ne!(crate::hostsocket::fake::last_opened().unwrap().0, id, "then dials the caster again");
        lock().retarget(None);
        assert!(session.step(start + Duration::from_secs(2)), "a session for an old setting is done");
        crate::device::unpair();
    }
}
