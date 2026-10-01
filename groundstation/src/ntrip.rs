use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use base64::Engine;
use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &[];
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DATA_WATCHDOG: Duration = Duration::from_secs(30);
const MAX_HTTP_HEADER_BYTES: usize = 32768;
const MIN_RECONNECT_MS: u64 = 1000;
const MAX_RECONNECT_MS: u64 = 30000;
const MAX_RECONNECT_ATTEMPTS: u32 = 100;
const STALE_MS: u64 = 5000;
const DATA_WARNING_BYTES: u64 = 50 * 1024 * 1024;
const PREAMBLE: u8 = 0xD3;
const RTCM_HEADER_BYTES: usize = 3;
const RTCM_CRC_BYTES: usize = 3;
const RTCM_MAX_PAYLOAD: usize = 1023;
const CRC24Q_POLY: u32 = 0x186_4CFB;
const RATE_WINDOW_MS: u64 = 1000;
const READ_SLICE: Duration = Duration::from_secs(1);
const GGA_FAST_RETRY_MS: u64 = 1000;
const GGA_FAST_RETRIES: u32 = 5;
const GGA_DEFAULT_INTERVAL_SEC: u64 = 5;
const GGA_FIX_QUALITY: u8 = 1;
const GGA_SATELLITES: u8 = 12;

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub mountpoint: String,
    pub whitelist: Vec<u16>,
    pub use_tls: bool,
    pub allow_self_signed: bool,
}

impl Config {
    pub fn validation_error(&self) -> Option<&'static str> {
        let control = |text: &str| text.chars().any(char::is_control);
        match () {
            _ if self.host.is_empty() => Some("No host address"),
            _ if self.port == 0 => Some("Invalid port"),
            _ if control(&self.host) => Some("Invalid host (contains control characters)"),
            _ if control(&self.mountpoint) => Some("Invalid mountpoint name (contains control characters)"),
            _ if self.username.contains(':') => Some("Invalid username (must not contain ':')"),
            _ => None,
        }
    }

    pub fn request(&self) -> Vec<u8> {
        let auth = (!self.username.is_empty() || !self.password.is_empty())
            .then(|| format!("Authorization: Basic {}\r\n", base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", self.username, self.password))))
            .unwrap_or_default();
        format!("GET /{} HTTP/1.1\r\nHost: {}\r\nNtrip-Version: Ntrip/2.0\r\nUser-Agent: NTRIP QGroundControl/1.0\r\n{auth}\r\n", self.mountpoint, self.host).into_bytes()
    }

    pub fn credentials_in_clear(&self) -> bool {
        !self.use_tls && (!self.username.is_empty() || !self.password.is_empty())
    }
}

pub fn parse_whitelist(csv: &str) -> Vec<u16> {
    csv.split(',').filter_map(|token| token.trim().parse::<u16>().ok()).filter(|id| *id > 0).collect()
}

#[derive(Debug, PartialEq)]
pub enum Response {
    Pending,
    Connected(Vec<u8>),
    Failed { fatal: bool, message: String },
}

fn status_line(line: &str) -> Option<(u16, String)> {
    let mut words = line.trim().splitn(3, ' ');
    let protocol = words.next()?;
    (protocol.starts_with("HTTP/") || protocol.eq_ignore_ascii_case("ICY")).then_some(())?;
    let code = words.next()?.parse().ok()?;
    Some((code, words.next().unwrap_or("").trim().to_string()))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn without_tags(body: &str) -> String {
    let stripped: String = body.chars().take(500).fold((String::new(), false), |(out, tag), c| match (c, tag) {
        ('<', _) => (out, true),
        ('>', true) => (out, false),
        (_, true) => (out, true),
        (c, false) => (out + &c.to_string(), false),
    }).0;
    stripped.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(200).collect()
}

pub fn response(buffer: &[u8]) -> Response {
    let Some(end) = find(buffer, b"\r\n\r\n") else {
        let icy = find(buffer, b"\r\n").and_then(|at| {
            let line = String::from_utf8_lossy(&buffer[..at]).to_string();
            line.to_ascii_uppercase().starts_with("ICY ").then(|| status_line(&line)).flatten().filter(|(code, _)| (200..300).contains(code)).map(|_| at)
        });
        return match (icy, buffer.len() >= MAX_HTTP_HEADER_BYTES) {
            (Some(at), _) => Response::Connected(buffer[at + 2..].to_vec()),
            (None, true) => Response::Failed { fatal: false, message: "HTTP response header too large".to_string() },
            (None, false) => Response::Pending,
        };
    };
    let header = String::from_utf8_lossy(&buffer[..end]).to_string();
    let Some((code, reason)) = header.split('\n').find_map(status_line) else { return Response::Failed { fatal: false, message: "Invalid HTTP response from caster".to_string() } };
    if (200..300).contains(&code) {
        return Response::Connected(buffer[end + 4..].to_vec());
    }
    if code == 401 {
        return Response::Failed { fatal: true, message: "Authentication failed (401): check username and password".to_string() };
    }
    let body = without_tags(String::from_utf8_lossy(&buffer[end + 4..]).trim());
    let base = if reason.is_empty() { format!("HTTP {code}") } else { format!("HTTP {code}: {reason}") };
    Response::Failed { fatal: false, message: if body.is_empty() { base } else { format!("{base} — {body}") } }
}

pub fn crc24q(data: &[u8]) -> u32 {
    data.iter().fold(0u32, |crc, byte| (0..8).fold(crc ^ (u32::from(*byte) << 16), |crc, _| if (crc << 1) & 0x100_0000 != 0 { (crc << 1) ^ CRC24Q_POLY } else { crc << 1 })) & 0xFF_FFFF
}

#[derive(Debug, PartialEq)]
pub struct Frame {
    pub id: u16,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct Framer {
    pending: Vec<u8>,
}

impl Framer {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Frame> {
        self.pending.extend_from_slice(bytes);
        std::iter::from_fn(|| self.next_frame()).flatten().collect()
    }

    fn next_frame(&mut self) -> Option<Option<Frame>> {
        let start = self.pending.iter().position(|b| *b == PREAMBLE);
        let Some(start) = start else {
            self.pending.clear();
            return None;
        };
        self.pending.drain(..start);
        if self.pending.len() < RTCM_HEADER_BYTES {
            return None;
        }
        let length = (usize::from(self.pending[1] & 0x03) << 8) | usize::from(self.pending[2]);
        if length == 0 || length > RTCM_MAX_PAYLOAD {
            self.pending.drain(..1);
            return Some(None);
        }
        let total = RTCM_HEADER_BYTES + length + RTCM_CRC_BYTES;
        if self.pending.len() < total {
            return None;
        }
        let frame: Vec<u8> = self.pending.drain(..total).collect();
        let received = (u32::from(frame[total - 3]) << 16) | (u32::from(frame[total - 2]) << 8) | u32::from(frame[total - 1]);
        let id = if length >= 2 { ((u16::from(frame[3]) << 4) | (u16::from(frame[4]) >> 4)) & 0xFFF } else { 0 };
        Some((crc24q(&frame[..total - RTCM_CRC_BYTES]) == received).then_some(Frame { id, bytes: frame }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Status {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

impl Status {
    fn token(self) -> &'static str {
        match self {
            Status::Disconnected => "disconnected",
            Status::Connecting => "connecting",
            Status::Connected => "connected",
            Status::Reconnecting => "reconnecting",
            Status::Error => "error",
        }
    }

    fn default_message(self) -> &'static str {
        match self {
            Status::Disconnected => "Disconnected",
            Status::Connecting => "Connecting...",
            Status::Connected => "Connected",
            Status::Reconnecting => "Reconnecting...",
            Status::Error => "",
        }
    }

    fn button(self) -> &'static str {
        match self {
            Status::Connecting => "Connecting…",
            Status::Reconnecting => "Reconnecting…",
            Status::Connected => "Disconnect",
            _ => "Connect",
        }
    }
}

#[derive(Debug, Default)]
struct Rate {
    window_start_ms: u64,
    window_bytes: u64,
    per_second: f64,
}

impl Rate {
    fn record(&mut self, bytes: u64, now_ms: u64) {
        self.roll(now_ms);
        self.window_bytes += bytes;
    }

    fn roll(&mut self, now_ms: u64) {
        let elapsed = now_ms.saturating_sub(self.window_start_ms);
        if elapsed >= RATE_WINDOW_MS {
            self.per_second = match elapsed >= 2 * RATE_WINDOW_MS {
                true => 0.0,
                false => self.window_bytes as f64 * 1000.0 / elapsed as f64,
            };
            self.window_start_ms = now_ms;
            self.window_bytes = 0;
        }
    }
}

#[derive(Debug, Default)]
pub struct Ntrip {
    pub generation: u64,
    config: Option<Config>,
    status: Status,
    message: String,
    attempts: u32,
    messages: u64,
    bytes_received: u64,
    received: Rate,
    sent_bytes: u64,
    sent: Rate,
    counts: BTreeMap<u16, u64>,
    last_message_ms: Option<u64>,
    udp_target: Option<String>,
    gga_source: &'static str,
}

impl Ntrip {
    fn enter(&mut self, status: Status, detail: &str) {
        if status != Status::Connected {
            self.gga_source = "";
        }
        self.status = status;
        self.message = if detail.is_empty() { status.default_message().to_string() } else { detail.to_string() };
    }

    pub fn retarget(&mut self, wanted: Option<Config>) -> Option<(Config, u64)> {
        if wanted == self.config {
            return None;
        }
        self.generation += 1;
        self.config = wanted.clone();
        self.attempts = 0;
        match wanted {
            None => {
                self.enter(Status::Disconnected, "");
                None
            }
            Some(config) => match config.validation_error() {
                Some(reason) => {
                    self.enter(Status::Error, reason);
                    None
                }
                None => {
                    self.enter(Status::Connecting, "");
                    self.reset_stats();
                    Some((config, self.generation))
                }
            },
        }
    }

    fn reset_stats(&mut self) {
        self.messages = 0;
        self.bytes_received = 0;
        self.received = Rate::default();
        self.counts.clear();
        self.last_message_ms = None;
    }

    pub fn gga_source(&mut self, generation: u64, source: &'static str) {
        if generation == self.generation {
            self.gga_source = source;
        }
    }

    pub fn connected(&mut self, generation: u64) {
        if generation == self.generation {
            self.attempts = 0;
            self.enter(Status::Connected, "");
        }
    }

    pub fn failed(&mut self, generation: u64, fatal: bool, detail: &str) -> Option<u64> {
        if generation != self.generation {
            return None;
        }
        if fatal {
            self.enter(Status::Error, detail);
            return None;
        }
        let backoff = (MIN_RECONNECT_MS << self.attempts.min(5)).min(MAX_RECONNECT_MS);
        self.attempts += 1;
        if self.attempts > MAX_RECONNECT_ATTEMPTS {
            self.enter(Status::Error, &format!("Gave up after {MAX_RECONNECT_ATTEMPTS} reconnect attempts"));
            return None;
        }
        self.enter(Status::Reconnecting, &format!("Reconnecting in {}s: {detail}", backoff / 1000));
        Some(backoff)
    }

    pub fn reconnecting(&mut self, generation: u64) -> bool {
        let current = generation == self.generation && self.status == Status::Reconnecting;
        if current {
            self.enter(Status::Connecting, "");
        }
        current
    }

    pub fn received(&mut self, generation: u64, bytes: usize, now_ms: u64) -> bool {
        let current = generation == self.generation;
        if current {
            self.bytes_received += bytes as u64;
            self.received.record(bytes as u64, now_ms);
        }
        current
    }

    pub fn accept(&mut self, generation: u64, frame: &Frame, now_ms: u64) -> bool {
        let wanted = generation == self.generation && self.config.as_ref().is_some_and(|c| c.whitelist.is_empty() || c.whitelist.contains(&frame.id));
        if wanted {
            self.messages += 1;
            *self.counts.entry(frame.id).or_default() += 1;
            self.last_message_ms = Some(now_ms);
            self.sent_bytes += frame.bytes.len() as u64;
            self.sent.record(frame.bytes.len() as u64, now_ms);
        }
        wanted
    }

    pub fn snapshot(&mut self, now_ms: u64) -> Value {
        self.received.roll(now_ms);
        self.sent.roll(now_ms);
        let connected = self.status == Status::Connected;
        let config = self.config.as_ref();
        json!({
            "kind": "object",
            "class": "NtripStatus",
            "status": self.status.token(),
            "statusMessage": self.message,
            "button": self.status.button(),
            "buttonEnabled": self.status != Status::Connecting,
            "connected": connected,
            "dataStale": self.last_message_ms.is_some_and(|at| now_ms.saturating_sub(at) >= STALE_MS) && self.messages > 0,
            "mountpoint": config.map(|c| c.mountpoint.clone()).unwrap_or_default(),
            "messages": self.messages,
            "messageTypes": self.counts.iter().map(|(id, count)| json!([id, count])).collect::<Vec<_>>(),
            "bytesReceived": self.bytes_received,
            "dataRateBytesPerSec": self.received.per_second,
            "dataWarning": self.bytes_received > DATA_WARNING_BYTES,
            "bytesSent": self.sent_bytes,
            "sentKBps": self.sent.per_second / 1024.0,
            "ggaSource": self.gga_source,
            "securityWarning": if connected && config.is_some_and(Config::credentials_in_clear) { "Credentials are being sent without TLS encryption." } else { "" },
        })
    }
}

pub static NTRIP: LazyLock<Mutex<Ntrip>> = LazyLock::new(|| Mutex::new(Ntrip::default()));

pub fn lock() -> MutexGuard<'static, Ntrip> {
    NTRIP.lock().unwrap_or_else(PoisonError::into_inner)
}

fn setting(name: &str) -> Value {
    crate::settingsstore::raw_setting(&format!("settings.ntripSettings.{name}")).unwrap_or(Value::Null)
}

fn text(name: &str) -> String {
    setting(name).as_str().unwrap_or_default().trim().to_string()
}

fn truthy(name: &str) -> bool {
    match setting(name) {
        Value::Bool(b) => b,
        Value::Number(n) => n.as_f64().is_some_and(|v| v != 0.0),
        Value::String(s) => s == "true" || s == "1",
        _ => false,
    }
}

pub fn configured() -> Option<Config> {
    truthy("ntripServerConnectEnabled").then(from_settings)
}

fn from_settings() -> Config {
    Config {
        host: text("ntripServerHostAddress"),
        port: setting("ntripServerPort").as_f64().filter(|p| (1.0..=65535.0).contains(p)).map_or(0, |p| p as u16),
        username: setting("ntripUsername").as_str().unwrap_or_default().to_string(),
        password: setting("ntripPassword").as_str().unwrap_or_default().to_string(),
        mountpoint: text("ntripMountpoint"),
        whitelist: parse_whitelist(&text("ntripWhitelist")),
        use_tls: truthy("ntripUseTls"),
        allow_self_signed: truthy("ntripAllowSelfSignedCerts"),
    }
}

fn udp_target() -> Option<String> {
    let port = setting("ntripUdpTargetPort").as_f64().filter(|p| (1.0..=65535.0).contains(p))?;
    let host = text("ntripUdpTargetAddress");
    (truthy("ntripUdpForwardEnabled") && !host.is_empty()).then(|| format!("{host}:{}", port as u16))
}

pub const FETCH_MOUNTPOINTS: &str = "ntrip.fetchMountpoints";
pub const SELECT_MOUNTPOINT: &str = "ntrip.selectMountpoint";
const SOURCE_TABLE_TTL_MS: u64 = 60_000;
const SOURCE_TABLE_TIMEOUT: Duration = Duration::from_secs(10);
const SOURCE_TABLE_MAX_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct Mountpoint {
    pub mountpoint: String,
    pub format: String,
    pub nav_system: String,
    pub country: String,
    pub latitude: f64,
    pub longitude: f64,
    pub bitrate: i64,
    pub distance_km: f64,
}

fn coordinate_field(text: &str, limit: f64) -> f64 {
    text.trim().parse::<f64>().ok().filter(|v| v.is_finite() && v.abs() <= limit).unwrap_or(0.0)
}

pub fn mountpoint_line(line: &str) -> Option<Mountpoint> {
    let fields: Vec<&str> = line.split(';').collect();
    (fields.len() >= 18 && fields[0].trim().eq_ignore_ascii_case("STR")).then(|| Mountpoint {
        mountpoint: fields[1].trim().to_string(),
        format: fields[3].trim().to_string(),
        nav_system: fields[6].trim().to_string(),
        country: fields[8].trim().to_string(),
        latitude: coordinate_field(fields[9], 90.0),
        longitude: coordinate_field(fields[10], 180.0),
        bitrate: fields[17].trim().parse().unwrap_or(0),
        distance_km: -1.0,
    })
}

pub fn source_table(raw: &str, from: Option<(f64, f64)>) -> Vec<Mountpoint> {
    let located = |m: Mountpoint| match from {
        Some((lat, lon)) if !(m.latitude == 0.0 && m.longitude == 0.0) => Mountpoint { distance_km: crate::surveygrid::distance_between((lat, lon), (m.latitude, m.longitude)) / 1000.0, ..m },
        _ => m,
    };
    let mut parsed: Vec<Mountpoint> = raw.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("ENDSOURCETABLE")).filter_map(mountpoint_line).map(located).collect();
    parsed.sort_by(|a, b| (a.distance_km < 0.0).cmp(&(b.distance_km < 0.0)).then(a.distance_km.total_cmp(&b.distance_km)));
    parsed
}

pub fn mountpoint_detail(m: &Mountpoint) -> String {
    [
        (!m.format.is_empty()).then(|| m.format.clone()),
        (!m.nav_system.is_empty()).then(|| m.nav_system.clone()),
        (!m.country.is_empty()).then(|| m.country.clone()),
        (m.bitrate > 0).then(|| format!("{} bps", m.bitrate)),
        (m.distance_km >= 0.0).then(|| format!("{:.1} km", m.distance_km)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ")
}

#[derive(Debug, Default)]
struct Browser {
    fetching: bool,
    error: String,
    mountpoints: Vec<Mountpoint>,
    fetched_at_ms: Option<u64>,
    key: Option<(String, u16, String, String, bool)>,
}

static BROWSER: LazyLock<Mutex<Browser>> = LazyLock::new(|| Mutex::new(Browser::default()));

fn browser() -> MutexGuard<'static, Browser> {
    BROWSER.lock().unwrap_or_else(PoisonError::into_inner)
}

fn caster_key(config: &Config) -> (String, u16, String, String, bool) {
    (config.host.clone(), config.port, config.username.clone(), config.password.clone(), config.use_tls)
}

fn download_source_table(config: &Config) -> Result<String, String> {
    let tls = ureq::tls::TlsConfig::builder().disable_verification(config.allow_self_signed).build();
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(SOURCE_TABLE_TIMEOUT)).tls_config(tls).build().into();
    let url = format!("{}://{}:{}/", if config.use_tls { "https" } else { "http" }, config.host, config.port);
    let request = agent.get(&url).header("Ntrip-Version", "Ntrip/2.0").header("User-Agent", "QGC-NTRIP");
    let request = match config.username.is_empty() && config.password.is_empty() {
        true => request,
        false => request.header("Authorization", format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", config.username, config.password)))),
    };
    let mut response = request.call().map_err(|e| e.to_string())?;
    let body = response.body_mut().with_config().limit(SOURCE_TABLE_MAX_BYTES).read_to_string().map_err(|e| match e {
        ureq::Error::BodyExceedsLimit(_) => format!("Source table too large (exceeds {} MB)", SOURCE_TABLE_MAX_BYTES / (1024 * 1024)),
        other => other.to_string(),
    })?;
    match body.contains("ENDSOURCETABLE") {
        true => Ok(body),
        false => Err("Response does not contain a valid source table".to_string()),
    }
}

fn vehicle_coordinate() -> Option<(f64, f64)> {
    crate::hub::lock().active().and_then(|v| v.facts.coordinate.map(|(lat, lon, _)| (lat, lon)))
}

pub fn fetch_mountpoints() -> Value {
    let config = from_settings();
    let key = caster_key(&config);
    let now_ms = crate::hub::now_ms();
    {
        let mut browser = browser();
        let same = browser.key.as_ref() == Some(&key);
        if same && (browser.fetching || (!browser.mountpoints.is_empty() && browser.fetched_at_ms.is_some_and(|at| now_ms.saturating_sub(at) < SOURCE_TABLE_TTL_MS))) {
            return json!({ "ok": true });
        }
        if let Some(reason) = config.validation_error() {
            *browser = Browser { error: reason.to_string(), ..Browser::default() };
            return json!({ "ok": true });
        }
        *browser = Browser { fetching: true, key: Some(key.clone()), ..Browser::default() };
    }
    let from = vehicle_coordinate();
    std::thread::Builder::new()
        .name("groundstation-ntrip-sourcetable".to_string())
        .spawn(move || {
            let fetched = download_source_table(&config);
            let mut browser = browser();
            if browser.key.as_ref() != Some(&key) {
                return;
            }
            browser.fetching = false;
            match fetched {
                Ok(body) => {
                    browser.mountpoints = source_table(&body, from);
                    browser.fetched_at_ms = Some(crate::hub::now_ms());
                }
                Err(error) => {
                    browser.error = error;
                    browser.mountpoints.clear();
                    browser.fetched_at_ms = None;
                }
            }
        })
        .expect("ntrip source table thread");
    json!({ "ok": true })
}

pub fn select_mountpoint(backend: &dyn Backend, args: &str) -> Value {
    match serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0)?.as_str().map(str::to_string)) {
        Some(mountpoint) => object(&backend.set("settings.ntripSettings.ntripMountpoint", &json!({ "value": mountpoint }).to_string())),
        None => json!({ "ok": false, "reason": "ntrip.selectMountpoint takes the mountpoint name" }),
    }
}

pub fn owns(path: &str) -> bool {
    [FETCH_MOUNTPOINTS, SELECT_MOUNTPOINT].contains(&path)
}

fn browser_json(active: bool, has_host: bool) -> Value {
    let browser = browser();
    let selected = text("ntripMountpoint");
    json!({
        "status": if browser.fetching { "inProgress" } else if !browser.error.is_empty() { "error" } else if browser.fetched_at_ms.is_some() { "success" } else { "idle" },
        "error": browser.error,
        "canBrowse": !active && has_host && !browser.fetching,
        "mountpoints": browser.mountpoints.iter().map(|m| json!({ "mountpoint": m.mountpoint, "detail": mountpoint_detail(m), "selected": m.mountpoint == selected })).collect::<Vec<_>>(),
    })
}

#[derive(Debug, Default)]
struct UdpInput {
    wanted: Option<(u16, bool)>,
    generation: u64,
}

static UDP_INPUT: LazyLock<Mutex<UdpInput>> = LazyLock::new(|| Mutex::new(UdpInput::default()));

fn udp_input() -> MutexGuard<'static, UdpInput> {
    UDP_INPUT.lock().unwrap_or_else(PoisonError::into_inner)
}

fn udp_input_wanted() -> Option<(u16, bool)> {
    let port = setting("rtcmUdpInputPort").as_f64().filter(|p| (1.0..=65535.0).contains(p))? as u16;
    truthy("rtcmUdpInputEnabled").then_some((port, truthy("rtcmUdpValidate")))
}

fn inject(rtcm: &[u8]) {
    let outbound = crate::hub::lock().inject_rtcm(rtcm);
    outbound.iter().for_each(|(link, bytes)| {
        crate::linkhost::write(&crate::linkhost::TRANSPORTS, *link, bytes);
    });
}

pub fn datagram_frames(framer: &mut Framer, datagram: &[u8], validate: bool) -> Vec<Vec<u8>> {
    match validate {
        true => framer.push(datagram).into_iter().map(|frame| frame.bytes).collect(),
        false => vec![datagram.to_vec()],
    }
}

fn listen(port: u16, validate: bool, generation: u64) {
    let socket = match std::net::UdpSocket::bind(("0.0.0.0", port)) {
        Ok(socket) => socket,
        Err(e) => {
            log::warn!("UDP RTCM input could not bind port {port}: {e}");
            return;
        }
    };
    if let Err(e) = socket.set_read_timeout(Some(READ_SLICE)) {
        log::warn!("UDP RTCM input on port {port}: {e}");
        return;
    }
    let mut framer = Framer::default();
    let mut buffer = [0u8; 65536];
    while udp_input().generation == generation {
        if let Ok((n, _)) = socket.recv_from(&mut buffer)
            && n > 0
        {
            datagram_frames(&mut framer, &buffer[..n], validate).iter().for_each(|rtcm| inject(rtcm));
        }
    }
}

fn sync_udp_input() {
    let wanted = udp_input_wanted();
    let start = {
        let mut input = udp_input();
        (input.wanted != wanted).then(|| {
            input.generation += 1;
            input.wanted = wanted;
            wanted.map(|(port, validate)| (port, validate, input.generation))
        })
    };
    if let Some(Some((port, validate, generation))) = start {
        std::thread::Builder::new().name("groundstation-rtcm-udp".to_string()).spawn(move || listen(port, validate, generation)).expect("rtcm udp thread");
    }
}

pub fn sync() {
    sync_udp_input();
    let wanted = configured();
    let target = udp_target();
    let start = {
        let mut ntrip = lock();
        ntrip.udp_target = target;
        ntrip.retarget(wanted)
    };
    if let Some((config, generation)) = start {
        std::thread::Builder::new().name("groundstation-ntrip".to_string()).spawn(move || follow(config, generation)).expect("ntrip thread");
    }
}

trait Stream: Read + Write + Send {}
impl<T: Read + Write + Send> Stream for T {}

#[derive(Debug)]
struct AnyCertificate(Arc<rustls::crypto::CryptoProvider>);

impl rustls::client::danger::ServerCertVerifier for AnyCertificate {
    fn verify_server_cert(&self, _: &rustls::pki_types::CertificateDer<'_>, _: &[rustls::pki_types::CertificateDer<'_>], _: &rustls::pki_types::ServerName<'_>, _: &[u8], _: rustls::pki_types::UnixTime) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(&self, message: &[u8], cert: &rustls::pki_types::CertificateDer<'_>, dss: &rustls::DigitallySignedStruct) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn verify_tls13_signature(&self, message: &[u8], cert: &rustls::pki_types::CertificateDer<'_>, dss: &rustls::DigitallySignedStruct) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn tls(config: &Config, tcp: TcpStream) -> Result<Box<dyn Stream>, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = rustls::ClientConfig::builder_with_provider(provider.clone()).with_safe_default_protocol_versions().map_err(|e| e.to_string())?;
    let client = match config.allow_self_signed {
        true => builder.dangerous().with_custom_certificate_verifier(Arc::new(AnyCertificate(provider))).with_no_client_auth(),
        false => builder.with_root_certificates(rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() }).with_no_client_auth(),
    };
    let name = rustls::pki_types::ServerName::try_from(config.host.clone()).map_err(|e| e.to_string())?;
    let connection = rustls::ClientConnection::new(Arc::new(client), name).map_err(|e| e.to_string())?;
    Ok(Box::new(rustls::StreamOwned::new(connection, tcp)))
}

fn open(config: &Config) -> Result<Box<dyn Stream>, String> {
    let address = (config.host.as_str(), config.port).to_socket_addrs().map_err(|e| e.to_string())?.next().ok_or_else(|| format!("{} does not resolve", config.host))?;
    let tcp = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).map_err(|e| if e.kind() == std::io::ErrorKind::TimedOut { "Connection timeout".to_string() } else { e.to_string() })?;
    tcp.set_read_timeout(Some(READ_SLICE)).map_err(|e| e.to_string())?;
    match config.use_tls {
        true => tls(config, tcp),
        false => Ok(Box::new(tcp)),
    }
}

fn forward(generation: u64, framer: &mut Framer, bytes: &[u8]) -> bool {
    let now_ms = crate::hub::now_ms();
    if !lock().received(generation, bytes.len(), now_ms) {
        return false;
    }
    let target = lock().udp_target.clone();
    let udp = target.as_ref().and_then(|_| std::net::UdpSocket::bind("0.0.0.0:0").ok());
    framer.push(bytes).iter().filter(|frame| lock().accept(generation, frame, now_ms)).for_each(|frame| {
        if let (Some(socket), Some(target)) = (udp.as_ref(), target.as_ref())
            && let Err(e) = socket.send_to(&frame.bytes, target)
        {
            log::warn!("NTRIP UDP forward to {target} failed: {e}");
        }
        inject(&frame.bytes);
    });
    true
}

fn nmea_checksum(body: &str) -> u8 {
    body.bytes().fold(0, |sum, b| sum ^ b)
}

fn degrees_minutes(degrees: f64, width: usize) -> String {
    let whole = degrees.abs().trunc();
    let minutes = ((degrees.abs() - whole) * 60.0 * 10000.0 + 0.5).trunc() / 10000.0;
    let (whole, minutes) = if minutes >= 60.0 { (whole + 1.0, minutes - 60.0) } else { (whole, minutes) };
    format!("{:0width$}{:07.4}", whole as u32, minutes)
}

pub fn gga(latitude: f64, longitude: f64, altitude_msl: f64, utc: (u32, u32, u32)) -> String {
    let altitude = if altitude_msl.is_finite() { altitude_msl } else { 0.0 };
    let body = format!(
        "GPGGA,{:02}{:02}{:02},{},{},{},{},{GGA_FIX_QUALITY},{GGA_SATELLITES},1.0,{altitude:.1},M,0.0,M,,",
        utc.0,
        utc.1,
        utc.2,
        degrees_minutes(latitude, 2),
        if latitude >= 0.0 { "N" } else { "S" },
        degrees_minutes(longitude, 3),
        if longitude >= 0.0 { "E" } else { "W" },
    );
    format!("${body}*{:02X}\r\n", nmea_checksum(&body))
}

fn sane(latitude: f64, longitude: f64) -> bool {
    latitude.is_finite() && longitude.is_finite() && !(latitude == 0.0 && longitude == 0.0) && latitude.abs() <= 90.0 && longitude.abs() <= 180.0
}

const GGA_SOURCES: [(u8, &str); 3] = [(1, "Vehicle GPS"), (2, "Vehicle EKF"), (4, "GCS Position")];

fn position_from(source: u8) -> Option<(f64, f64, f64)> {
    let found = match source {
        1 => crate::hub::lock().active().and_then(|v| Some((v.gps.latitude?, v.gps.longitude?, v.facts.altitude_amsl))),
        2 => crate::hub::lock().active().and_then(|v| v.facts.coordinate.map(|(lat, lon, _)| (lat, lon, v.facts.altitude_amsl))),
        4 => match crate::gcsposition::lock().coordinate() {
            (Some(lat), Some(lon), alt) => Some((lat, lon, alt.unwrap_or(0.0))),
            _ => None,
        },
        _ => None,
    };
    found.filter(|(lat, lon, _)| sane(*lat, *lon))
}

pub fn gga_position(source: u8) -> Option<((f64, f64, f64), &'static str)> {
    match source {
        0 => GGA_SOURCES.iter().find_map(|(id, name)| position_from(*id).map(|p| (p, *name))),
        id => GGA_SOURCES.iter().find(|(s, _)| *s == id).and_then(|(id, name)| position_from(*id).map(|p| (p, *name))),
    }
}

#[derive(Debug, Default)]
pub struct GgaSchedule {
    next_ms: u64,
    misses: u32,
    settled: bool,
}

impl GgaSchedule {
    pub fn due(&self, now_ms: u64) -> bool {
        now_ms >= self.next_ms
    }

    pub fn sent(&mut self, now_ms: u64, interval_ms: u64) {
        self.settled = true;
        self.misses = 0;
        self.next_ms = now_ms + interval_ms;
    }

    pub fn missed(&mut self, now_ms: u64, interval_ms: u64) {
        self.misses += 1;
        self.settled = self.settled || self.misses >= GGA_FAST_RETRIES;
        self.next_ms = now_ms + if self.settled { interval_ms } else { GGA_FAST_RETRY_MS };
    }
}

fn send_gga(stream: &mut Box<dyn Stream>, schedule: &mut GgaSchedule, generation: u64) -> Result<(), String> {
    let now_ms = crate::hub::now_ms();
    if !schedule.due(now_ms) {
        return Ok(());
    }
    let interval_ms = setting("ntripGgaIntervalSec").as_u64().filter(|s| *s > 0).unwrap_or(GGA_DEFAULT_INTERVAL_SEC) * 1000;
    let source = setting("ntripGgaPositionSource").as_u64().and_then(|s| u8::try_from(s).ok()).unwrap_or(0);
    match gga_position(source) {
        None => {
            schedule.missed(now_ms, interval_ms);
            Ok(())
        }
        Some(((lat, lon, alt), name)) => {
            let now = chrono::Utc::now();
            use chrono::Timelike;
            stream.write_all(gga(lat, lon, alt, (now.hour(), now.minute(), now.second())).as_bytes()).map_err(|e| e.to_string())?;
            schedule.sent(now_ms, interval_ms);
            lock().gga_source(generation, name);
            Ok(())
        }
    }
}

fn session(config: &Config, generation: u64) -> (bool, String) {
    let mut stream = match open(config) {
        Ok(stream) => stream,
        Err(detail) => return (false, detail),
    };
    if let Err(e) = stream.write_all(&config.request()) {
        return (false, e.to_string());
    }
    let mut header = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut quiet = Duration::ZERO;
    let timed_out = |e: &std::io::Error| matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut);
    let watchdog = format!("No data received for {} seconds", DATA_WATCHDOG.as_secs());
    let rest = loop {
        match stream.read(&mut chunk) {
            Ok(0) => return (false, "Server disconnected (peer closed before HTTP response; check mountpoint and credentials)".to_string()),
            Ok(n) => header.extend_from_slice(&chunk[..n]),
            Err(e) if timed_out(&e) && quiet + READ_SLICE < DATA_WATCHDOG => {
                quiet += READ_SLICE;
                continue;
            }
            Err(e) if timed_out(&e) => return (false, watchdog),
            Err(e) => return (false, e.to_string()),
        }
        match response(&header) {
            Response::Pending => {}
            Response::Connected(rest) => break rest,
            Response::Failed { fatal, message } => return (fatal, message),
        }
    };
    lock().connected(generation);
    let mut framer = Framer::default();
    if !forward(generation, &mut framer, &rest) {
        return (false, String::new());
    }
    let mut schedule = GgaSchedule::default();
    let mut quiet = Duration::ZERO;
    loop {
        if let Err(detail) = send_gga(&mut stream, &mut schedule, generation) {
            return (false, detail);
        }
        match stream.read(&mut chunk) {
            Ok(0) => return (false, "Server disconnected".to_string()),
            Ok(n) if !forward(generation, &mut framer, &chunk[..n]) => return (false, String::new()),
            Ok(_) => quiet = Duration::ZERO,
            Err(e) if timed_out(&e) && lock().generation != generation => return (false, String::new()),
            Err(e) if timed_out(&e) && quiet + READ_SLICE < DATA_WATCHDOG => quiet += READ_SLICE,
            Err(e) if timed_out(&e) => return (false, watchdog),
            Err(e) => return (false, e.to_string()),
        }
    }
}

fn follow(config: Config, generation: u64) {
    loop {
        let (fatal, detail) = session(&config, generation);
        let Some(backoff) = lock().failed(generation, fatal, &detail) else { return };
        std::thread::sleep(Duration::from_millis(backoff));
        if !lock().reconnecting(generation) {
            return;
        }
    }
}

pub fn ntrip_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    if !crate::vehiclefacade::switched_on() {
        return json!({ "kind": "object", "class": "NtripStatus", "status": "unavailable", "statusMessage": "Unavailable" });
    }
    sync();
    let active = truthy("ntripServerConnectEnabled");
    let has_host = !text("ntripServerHostAddress").is_empty();
    let can_connect = active || has_host;
    let mut snapshot = lock().snapshot(crate::hub::now_ms());
    snapshot["browser"] = browser_json(active, has_host);
    snapshot["active"] = json!(active);
    snapshot["buttonEnabled"] = json!(snapshot["buttonEnabled"].as_bool().unwrap_or(false) && can_connect);
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(id: u16, payload_len: usize) -> Vec<u8> {
        let mut body = vec![(id >> 4) as u8, ((id & 0x0F) << 4) as u8];
        body.resize(payload_len, 0x55);
        let mut bytes = vec![PREAMBLE, (payload_len >> 8) as u8 & 0x03, payload_len as u8];
        bytes.extend(body);
        let crc = crc24q(&bytes);
        bytes.extend([(crc >> 16) as u8, (crc >> 8) as u8, crc as u8]);
        bytes
    }

    fn config() -> Config {
        Config { host: "caster.example".into(), port: 2101, username: "user".into(), password: "pass".into(), mountpoint: "MOUNT".into(), whitelist: vec![], use_tls: false, allow_self_signed: false }
    }

    #[test]
    fn the_request_is_ntrip_two_with_basic_auth() {
        let request = String::from_utf8(config().request()).unwrap();
        assert_eq!(request, "GET /MOUNT HTTP/1.1\r\nHost: caster.example\r\nNtrip-Version: Ntrip/2.0\r\nUser-Agent: NTRIP QGroundControl/1.0\r\nAuthorization: Basic dXNlcjpwYXNz\r\n\r\n");
        assert!(config().credentials_in_clear());
        let anonymous = Config { username: String::new(), password: String::new(), ..config() };
        assert!(!String::from_utf8(anonymous.request()).unwrap().contains("Authorization"));
    }

    #[test]
    fn the_config_is_refused_before_any_socket_opens() {
        assert_eq!(Config { host: String::new(), ..config() }.validation_error(), Some("No host address"));
        assert_eq!(Config { port: 0, ..config() }.validation_error(), Some("Invalid port"));
        assert_eq!(Config { username: "a:b".into(), ..config() }.validation_error(), Some("Invalid username (must not contain ':')"));
        assert_eq!(config().validation_error(), None);
        assert_eq!(parse_whitelist("1005, 1077,x,0,,1087"), [1005, 1077, 1087]);
    }

    #[test]
    fn the_caster_reply_decides_connected_retry_or_give_up() {
        assert_eq!(response(b"HTTP/1.1 200 OK\r\nServer: x\r\n\r\n\xD3\x00"), Response::Connected(vec![0xD3, 0x00]));
        assert_eq!(response(b"ICY 200 OK\r\n\xD3"), Response::Connected(vec![0xD3]), "a v1 caster streams straight after the status line");
        assert_eq!(response(b"HTTP/1.1 200 OK\r\n"), Response::Pending);
        assert_eq!(response(b"HTTP/1.1 401 Unauthorized\r\n\r\n"), Response::Failed { fatal: true, message: "Authentication failed (401): check username and password".into() });
        assert_eq!(response(b"HTTP/1.1 404 Not Found\r\n\r\n<html><b>No such</b> mount</html>"), Response::Failed { fatal: false, message: "HTTP 404: Not Found — No such mount".into() });
        assert_eq!(response(b"garbage\r\n\r\n"), Response::Failed { fatal: false, message: "Invalid HTTP response from caster".into() });
    }

    #[test]
    fn frames_are_cut_from_a_split_stream_and_a_bad_crc_is_dropped() {
        let mut framer = Framer::default();
        let first = frame(1005, 19);
        let second = frame(1077, 40);
        let mut corrupt = frame(1087, 10);
        corrupt[6] ^= 0xFF;
        let stream: Vec<u8> = [vec![0x00, 0x11], first.clone(), corrupt, second.clone()].concat();
        let (head, tail) = stream.split_at(10);
        assert!(framer.push(head).is_empty());
        let frames = framer.push(tail);
        assert_eq!(frames, [Frame { id: 1005, bytes: first }, Frame { id: 1077, bytes: second }]);
    }

    #[test]
    fn the_whitelist_filters_and_failures_back_off_until_auth_stops_it() {
        let mut ntrip = Ntrip::default();
        let (_, generation) = ntrip.retarget(Some(Config { whitelist: vec![1005], ..config() })).unwrap();
        assert_eq!(ntrip.status, Status::Connecting);
        ntrip.connected(generation);
        assert!(ntrip.accept(generation, &Frame { id: 1005, bytes: vec![0; 25] }, 0));
        assert!(!ntrip.accept(generation, &Frame { id: 1077, bytes: vec![0; 25] }, 0));
        let snapshot = ntrip.snapshot(6000);
        assert_eq!((snapshot["messages"].clone(), snapshot["messageTypes"].clone(), snapshot["dataStale"].clone()), (json!(1), json!([[1005, 1]]), json!(true)));
        assert_eq!(snapshot["securityWarning"], "Credentials are being sent without TLS encryption.");
        assert_eq!(ntrip.failed(generation, false, "boom"), Some(1000));
        assert_eq!(ntrip.message, "Reconnecting in 1s: boom");
        assert!(ntrip.reconnecting(generation));
        assert_eq!(ntrip.failed(generation, false, "boom"), Some(2000));
        assert_eq!(ntrip.failed(generation, true, "Authentication failed"), None);
        assert_eq!(ntrip.status, Status::Error);
        assert_eq!(ntrip.failed(generation - 1, false, "stale thread"), None, "a superseded connection reports nothing");
        assert_eq!(ntrip.retarget(None), None);
        assert_eq!(ntrip.status, Status::Disconnected);
        assert_eq!(ntrip.retarget(Some(Config { host: String::new(), ..config() })), None);
        assert_eq!((ntrip.status, ntrip.message.as_str()), (Status::Error, "No host address"));
    }

    #[test]
    fn the_gga_sentence_matches_qgc_make_gga() {
        assert_eq!(gga(47.3977419, 8.5455938, 488.0, (12, 34, 56)), "$GPGGA,123456,4723.8645,N,00832.7356,E,1,12,1.0,488.0,M,0.0,M,,*70\r\n");
        assert!(gga(-33.5, -70.25, f64::NAN, (0, 0, 0)).starts_with("$GPGGA,000000,3330.0000,S,07015.0000,W,1,12,1.0,0.0,M"));
        assert_eq!(degrees_minutes(9.99999999, 3), "01000.0000", "minutes rounding to 60 carry into the degrees");
    }

    #[test]
    fn gga_retries_fast_five_times_then_settles_on_the_interval() {
        let mut schedule = GgaSchedule::default();
        assert!(schedule.due(0));
        (0..4).for_each(|i| schedule.missed(i * 1000, 5000));
        assert!(schedule.due(4000) && !schedule.due(3999));
        schedule.missed(4000, 5000);
        assert!(!schedule.due(8999) && schedule.due(9000), "after five misses it waits the full interval");
        schedule.sent(9000, 5000);
        schedule.missed(14000, 5000);
        assert!(!schedule.due(15000), "once settled a miss does not go back to the fast retry");
    }

    #[test]
    fn a_udp_datagram_is_validated_frame_by_frame_or_passed_whole() {
        let good = frame(1005, 19);
        let mut bad = frame(1077, 10);
        bad[5] ^= 0xFF;
        let datagram = [good.clone(), bad].concat();
        assert_eq!(datagram_frames(&mut Framer::default(), &datagram, true), [good]);
        assert_eq!(datagram_frames(&mut Framer::default(), &datagram, false), [datagram.clone()]);
    }

    #[test]
    fn the_source_table_is_parsed_and_nearest_first() {
        let table = "SOURCETABLE 200 OK\r\nCAS;caster;2101;x\r\nSTR;FAR;Far;RTCM 3.2;1005;2;GPS+GLO;NET;DEU;52.5;13.4;0;0;gen;none;B;N;9600;\r\nSTR;NEAR;Near;RTCM 3.3;1077;2;GPS;NET;CHE;47.4;8.5;1;0;gen;none;B;N;4800;\r\nSTR;NOWHERE;x;RTCM 3;;2;GPS;NET;;0;0;0;0;gen;none;N;N;0;\r\nSTR;short;line\r\nENDSOURCETABLE\r\n";
        let parsed = source_table(table, Some((47.39, 8.54)));
        assert_eq!(parsed.iter().map(|m| m.mountpoint.as_str()).collect::<Vec<_>>(), ["NEAR", "FAR", "NOWHERE"], "known distances ascend, unknown last");
        assert!(parsed[0].distance_km < 5.0);
        assert_eq!(mountpoint_detail(&parsed[2]), "RTCM 3 · GPS");
        assert!(mountpoint_detail(&parsed[1]).starts_with("RTCM 3.2 · GPS+GLO · DEU · 9600 bps · "));
        assert_eq!(source_table(table, None)[0].distance_km, -1.0);
    }

    #[test]
    fn the_crc_matches_the_rtcm_reference() {
        assert_eq!(crc24q(b"123456789"), 0xCDE703);
    }
}
