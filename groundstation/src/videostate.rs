use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[];

pub const MAIN_RECEIVER: &str = "videoContent";
pub const THERMAL_RECEIVER: &str = "thermalVideo";
pub const PIP_RECEIVER: &str = "extraVideo1";
pub const START_TIMEOUT_S: u32 = 3;
pub const RESTART_DELAY_MS: u64 = 1000;
const MAX_VIDEO_RECONNECT_ATTEMPTS: u32 = 30;
const MAX_RECONNECT_DELAY_S: u64 = 30;

pub fn reconnect_delay_ms(attempt: u32) -> u64 {
    (1u64 << attempt.saturating_sub(1).min(5)).min(MAX_RECONNECT_DELAY_S) * RESTART_DELAY_MS
}

pub const SOURCE_NO_VIDEO: &str = "No Video Available";
pub const SOURCE_DISABLED: &str = "Video Stream Disabled";
pub const SOURCE_RTSP: &str = "RTSP Video Stream";
pub const SOURCE_UDP_H264: &str = "UDP h.264 Video Stream";
pub const SOURCE_UDP_H265: &str = "UDP h.265 Video Stream";
pub const SOURCE_TCP: &str = "TCP-MPEG2 Video Stream";
pub const SOURCE_MPEGTS: &str = "MPEG-TS Video Stream";
pub const SOURCE_WEBRTC: &str = "WebRTC (WHEP) Video Stream";
pub const SOURCE_3DR_SOLO: &str = "3DR Solo (requires restart)";
pub const SOURCE_PARROT_DISCOVERY: &str = "Parrot Discovery";
pub const SOURCE_YUNEEC_MANTIS_G: &str = "Yuneec Mantis G";
pub const SOURCE_HERELINK_AIR_UNIT: &str = "Herelink AirUnit";
pub const SOURCE_HERELINK_HOTSPOT: &str = "Herelink Hotspot";
pub const SOURCE_BACK_CAMERA: &str = "Back Camera";
pub const SOURCE_FRONT_CAMERA: &str = "Front Camera";
pub const DEVICE_CAMERAS: &[&str] = &[SOURCE_BACK_CAMERA, SOURCE_FRONT_CAMERA];
pub const SOURCE_SYNTHETIC: &str = "Synthetic View";
pub const DEVICE_CAMERA_SCHEME: &str = "ahc://";

pub const STREAM_SOURCES: &[&str] = &[
    SOURCE_UDP_H264,
    SOURCE_UDP_H265,
    SOURCE_RTSP,
    SOURCE_TCP,
    SOURCE_MPEGTS,
    SOURCE_WEBRTC,
    SOURCE_3DR_SOLO,
    SOURCE_PARROT_DISCOVERY,
    SOURCE_YUNEEC_MANTIS_G,
    SOURCE_HERELINK_AIR_UNIT,
    SOURCE_HERELINK_HOTSPOT,
];

pub const URL_SOURCES: &[&str] = &[SOURCE_UDP_H264, SOURCE_UDP_H265, SOURCE_MPEGTS, SOURCE_RTSP, SOURCE_TCP, SOURCE_WEBRTC];

pub const NEGOTIATING_SOURCES: &[&str] = &[SOURCE_RTSP, SOURCE_WEBRTC];

pub const SIGNAL_LIVE: &str = "live";
pub const SIGNAL_CONNECTING: &str = "connecting";
pub const SIGNAL_NONE: &str = "noSignal";
pub const SIGNAL_IDLE: &str = "idle";

pub const REFUSED_SOURCE_OUT_OF_RANGE: &str = "sourceOutOfRange";
pub const REFUSED_SOURCE_UNCONFIGURED: &str = "sourceUnconfigured";
pub const REFUSED_NO_STARTED_RECEIVER: &str = "noStartedReceiver";
pub const REFUSED_NO_SAVE_PATH: &str = "noSavePath";
pub const REFUSED_BAD_FORMAT: &str = "badFormat";

pub const STREAM_TYPE_RTSP: u8 = 0;
pub const STREAM_TYPE_RTP_UDP: u8 = 1;
pub const STREAM_TYPE_TCP_MPEG: u8 = 2;
pub const STREAM_TYPE_MPEG_TS: u8 = 3;
pub const ENCODING_H265: u8 = 2;

pub fn is_stream_source(source: &str) -> bool {
    STREAM_SOURCES.contains(&source)
}

pub fn needs_url(source: &str) -> bool {
    URL_SOURCES.contains(&source)
}

pub fn negotiates(source: &str) -> bool {
    NEGOTIATING_SOURCES.contains(&source)
}

pub fn source_token(source: &str) -> &'static str {
    match source {
        SOURCE_UDP_H264 => "udpH264",
        SOURCE_UDP_H265 => "udpH265",
        SOURCE_RTSP => "rtsp",
        SOURCE_TCP => "tcp",
        SOURCE_MPEGTS => "mpegts",
        SOURCE_WEBRTC => "webrtc",
        SOURCE_3DR_SOLO => "solo3dr",
        SOURCE_PARROT_DISCOVERY => "parrot",
        SOURCE_YUNEEC_MANTIS_G => "yuneecMantisG",
        SOURCE_HERELINK_AIR_UNIT => "herelinkAirUnit",
        SOURCE_HERELINK_HOTSPOT => "herelinkHotspot",
        SOURCE_BACK_CAMERA => "backCamera",
        SOURCE_FRONT_CAMERA => "frontCamera",
        SOURCE_SYNTHETIC => "synthetic",
        SOURCE_DISABLED => "disabled",
        SOURCE_NO_VIDEO => "noVideo",
        _ => "unknown",
    }
}

pub fn device_camera(uri: &str) -> Option<u32> {
    uri.strip_prefix(DEVICE_CAMERA_SCHEME)?.parse().ok()
}

pub fn requires_restart(source: &str) -> bool {
    source == SOURCE_3DR_SOLO
}

pub fn source_usable(source: &str, url: &str) -> bool {
    match source {
        SOURCE_NO_VIDEO | SOURCE_DISABLED => false,
        _ if needs_url(source) => !url.is_empty(),
        SOURCE_3DR_SOLO | SOURCE_PARROT_DISCOVERY | SOURCE_YUNEEC_MANTIS_G | SOURCE_HERELINK_AIR_UNIT | SOURCE_HERELINK_HOTSPOT | SOURCE_BACK_CAMERA | SOURCE_FRONT_CAMERA | SOURCE_SYNTHETIC => true,
        _ => false,
    }
}

pub fn start_timeout_s(source: &str, rtsp_timeout_s: u32) -> u32 {
    match negotiates(source) {
        true => rtsp_timeout_s,
        false => START_TIMEOUT_S,
    }
}

pub fn source_uri(source: &str, url: &str) -> String {
    match source {
        SOURCE_UDP_H264 => format!("udp://{url}"),
        SOURCE_UDP_H265 => format!("udp265://{url}"),
        SOURCE_MPEGTS => format!("mpegts://{url}"),
        SOURCE_RTSP => url.to_string(),
        SOURCE_TCP => format!("tcp://{url}"),
        SOURCE_WEBRTC => match url.trim() {
            "" => String::new(),
            bare if !bare.contains("://") => format!("http://{bare}"),
            given => given.to_string(),
        },
        SOURCE_3DR_SOLO => "udp://0.0.0.0:5600".to_string(),
        SOURCE_PARROT_DISCOVERY => "udp://0.0.0.0:8888".to_string(),
        SOURCE_YUNEEC_MANTIS_G => "rtsp://192.168.42.1:554/live".to_string(),
        SOURCE_HERELINK_AIR_UNIT => "rtsp://192.168.0.10:8554/H264Video".to_string(),
        SOURCE_HERELINK_HOTSPOT => "rtsp://192.168.43.1:8554/fpv_stream".to_string(),
        SOURCE_BACK_CAMERA => format!("{DEVICE_CAMERA_SCHEME}0"),
        SOURCE_FRONT_CAMERA => format!("{DEVICE_CAMERA_SCHEME}1"),
        _ => String::new(),
    }
}

fn listen_port(uri: &str) -> Option<&str> {
    let (scheme, rest) = uri.split_once("://")?;
    ["udp", "udp265", "mpegts"].contains(&scheme).then(|| rest.split(['/', '?']).next()?.rsplit_once(':').map(|(_, port)| port)).flatten()
}

fn stream_key(uri: &str) -> String {
    let trimmed = uri.trim();
    let Some((scheme, rest)) = trimmed.split_once("://") else { return trimmed.to_string() };
    let (authority, path) = rest.split_at(rest.find(['/', '?']).unwrap_or(rest.len()));
    let host = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
    format!("{}://{}{}", scheme.to_ascii_lowercase(), host.to_ascii_lowercase(), path.trim_end_matches('/'))
}

pub fn same_stream(a: &str, b: &str) -> bool {
    stream_key(a) == stream_key(b) || listen_port(a).is_some_and(|port| listen_port(b) == Some(port))
}

fn pairs_with(main: &str, other: &str) -> bool {
    !same_stream(main, other) && !(device_camera(main).is_some() && device_camera(other).is_some())
}

pub fn auto_stream_source(stream_type: u8, encoding: u8, uri: &str) -> (&'static str, String) {
    match stream_type {
        STREAM_TYPE_RTSP => (SOURCE_RTSP, uri.to_string()),
        STREAM_TYPE_TCP_MPEG => (SOURCE_TCP, schemed("tcp://", uri)),
        STREAM_TYPE_RTP_UDP if encoding == ENCODING_H265 => (SOURCE_UDP_H265, port_uri("udp265://", uri)),
        STREAM_TYPE_RTP_UDP => (SOURCE_UDP_H264, port_uri("udp://", uri)),
        STREAM_TYPE_MPEG_TS => (SOURCE_MPEGTS, port_uri("mpegts://", uri)),
        _ => (SOURCE_NO_VIDEO, String::new()),
    }
}

fn schemed(scheme: &str, uri: &str) -> String {
    match uri.contains("://") {
        true => uri.to_string(),
        false => format!("{scheme}{uri}"),
    }
}

fn port_uri(scheme: &str, uri: &str) -> String {
    match uri.contains(scheme) {
        true => uri.to_string(),
        false => format!("{scheme}0.0.0.0:{uri}"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    NoVideoSource,
    UnsupportedSource,
    NotShown,
    NoReceiver,
    NotStarted,
    NoStreamUrl,
    InvalidStreamUrl,
    Connecting,
    Reconnecting,
    ConnectionFailed,
    WaitingForFrames,
    Decoding,
}

impl Status {
    pub fn token(self) -> &'static str {
        match self {
            Status::NoVideoSource => "noVideoSource",
            Status::UnsupportedSource => "unsupportedSource",
            Status::NotShown => "notShown",
            Status::NoReceiver => "noReceiver",
            Status::NotStarted => "notStarted",
            Status::NoStreamUrl => "noStreamUrl",
            Status::InvalidStreamUrl => "invalidStreamUrl",
            Status::Connecting => "connecting",
            Status::Reconnecting => "reconnecting",
            Status::ConnectionFailed => "connectionFailed",
            Status::WaitingForFrames => "waitingForFrames",
            Status::Decoding => "decoding",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    InvalidUrl,
    InvalidState,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Age {
    NotBound,
    NoCamera,
    NoFrameYet,
    Seconds(u64),
}

impl Age {
    pub fn token(self) -> &'static str {
        match self {
            Age::NotBound => "notBound",
            Age::NoCamera => "noCamera",
            Age::NoFrameYet => "noFrameYet",
            Age::Seconds(_) => "seconds",
        }
    }

    pub fn seconds(self) -> Option<u64> {
        match self {
            Age::Seconds(age) => Some(age),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SourceSlot {
    pub source: String,
    pub url: String,
    pub name: String,
    pub drone: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    pub cameras: Vec<SourceSlot>,
    pub active_source: i64,
    pub multi_view: bool,
    pub pip_shown: bool,
    pub stream_enabled: bool,
    pub low_latency: bool,
    pub save_path_set: bool,
    pub recording_format_valid: bool,
    pub rtsp_timeout_s: u32,
    pub reconnect_disabled: bool,
}

impl Settings {
    pub fn count(&self) -> usize {
        self.cameras.len()
    }

    pub fn source_at(&self, index: usize) -> &str {
        self.cameras.get(index).map_or(SOURCE_DISABLED, |camera| camera.source.as_str())
    }

    pub fn url_at(&self, index: usize) -> &str {
        self.cameras.get(index).map_or("", |camera| camera.url.as_str())
    }

    pub fn name_at(&self, index: usize) -> Option<&str> {
        self.cameras.get(index).map(|camera| camera.name.as_str()).filter(|name| !name.is_empty())
    }

    pub fn from_drone(&self, index: usize) -> bool {
        self.cameras.get(index).is_some_and(|camera| camera.drone)
    }

    pub fn enabled(&self, index: usize) -> bool {
        self.source_at(index) != SOURCE_DISABLED
    }

    pub fn configured(&self, index: usize) -> bool {
        index < self.count() && (!needs_url(self.source_at(index)) || !self.url_at(index).is_empty())
    }

    pub fn usable(&self, index: usize) -> bool {
        index < self.count() && source_usable(self.source_at(index), self.url_at(index))
    }

    pub fn current_index(&self) -> usize {
        usize::try_from(self.active_source)
            .ok()
            .filter(|index| self.usable(*index))
            .or_else(|| (0..self.count()).find(|index| self.usable(*index)))
            .unwrap_or(0)
    }

    pub fn active_refusal(&self) -> Option<&'static str> {
        let asked = self.active_source;
        match asked {
            asked if asked < 0 || asked >= self.count() as i64 => (asked != 0).then_some(REFUSED_SOURCE_OUT_OF_RANGE),
            asked if !self.configured(asked as usize) => Some(REFUSED_SOURCE_UNCONFIGURED),
            _ => None,
        }
    }

    pub fn shown(&self, index: usize) -> bool {
        self.camera_index_for_receiver(MAIN_RECEIVER) == Some(index) || self.pip_camera() == Some(index)
    }

    pub fn clamp_active(&self, asked: i64) -> usize {
        asked.clamp(0, (self.count() as i64 - 1).max(0)) as usize
    }

    pub fn switchable(&self) -> Vec<usize> {
        (0..self.count()).filter(|index| self.usable(*index)).collect()
    }

    pub fn next_switchable(&self) -> Option<usize> {
        let current = self.current_index();
        (1..self.count()).map(|step| (current + step) % self.count()).find(|index| self.usable(*index))
    }

    pub fn uri_at(&self, index: usize) -> String {
        source_uri(self.source_at(index), self.url_at(index))
    }

    pub fn pip_choice(&self) -> Option<usize> {
        let current = self.current_index();
        let main = self.uri_at(current);
        (1..self.count()).map(|step| (current + step) % self.count()).find(|index| self.usable(*index) && pairs_with(&main, &self.uri_at(*index)))
    }

    pub fn pip_camera(&self) -> Option<usize> {
        (self.multi_view && self.pip_shown).then(|| self.pip_choice()).flatten()
    }

    pub fn camera_index_for_receiver(&self, receiver: &str) -> Option<usize> {
        match receiver {
            MAIN_RECEIVER => (self.count() > 0).then(|| self.current_index()),
            PIP_RECEIVER => self.pip_camera(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Receiver {
    uri: String,
    started: bool,
    streaming: bool,
    decoding: bool,
    connecting: bool,
    recording: bool,
    recording_since_s: Option<u64>,
    attempts: u32,
    failing_since_s: Option<u64>,
    timeout_s: Option<u32>,
    status: Option<Status>,
    size: Option<(u32, u32)>,
    last_frame_s: Option<u64>,
    reconnect_attempts: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    StartReceiver { receiver: String, timeout_s: u32, low_latency: bool },
    StopReceiver { receiver: String },
    RestartAfter { receiver: String, delay_ms: u64 },
    StartRecording { receivers: Vec<String> },
    StopRecording { receivers: Vec<String> },
    StartCameraStream,
    StopCameraStream,
    SetSetting { name: &'static str, value: String },
    CamerasChanged,
    StreamingChanged(bool),
    DecodingChanged(bool),
    RecordingChanged { receiver: String, active: bool },
    ActiveRecordingChanged(bool),
    VideoSizeChanged { width_pixels: u32, height_pixels: u32 },
    StopTelemetryCapture,
    FullScreenChanged(bool),
}

#[derive(Debug, Default)]
pub struct VideoState {
    pub settings: Settings,
    pub streaming: bool,
    pub decoding: bool,
    pub recording: bool,
    pub video_size: Option<(u32, u32)>,
    pub full_screen: bool,
    pub vehicle_present: bool,
    receivers: BTreeMap<String, Receiver>,
}

impl VideoState {
    pub fn url_at(&self, index: usize) -> &str {
        self.settings.url_at(index)
    }

    pub fn configured(&self, index: usize) -> bool {
        self.settings.configured(index)
    }

    pub fn usable(&self, index: usize) -> bool {
        self.settings.usable(index)
    }

    pub fn stream_configured(&self) -> bool {
        self.usable(self.settings.current_index())
    }

    pub fn has_video(&self) -> bool {
        self.settings.stream_enabled && self.stream_configured()
    }

    pub fn is_stream_source(&self) -> bool {
        is_stream_source(self.settings.source_at(self.settings.current_index()))
    }

    pub fn from_drone(&self) -> bool {
        self.settings.from_drone(self.settings.current_index())
    }

    pub fn has_multiple_sources(&self) -> bool {
        self.settings.switchable().len() > 1
    }

    pub fn desired_uri(&self, receiver: &str) -> String {
        self.settings.camera_index_for_receiver(receiver).map(|index| self.settings.uri_at(index)).unwrap_or_default()
    }

    fn camera_receiver(receiver: &str) -> bool {
        receiver != THERMAL_RECEIVER
    }

    fn receiver_for(&self, index: usize) -> Option<&Receiver> {
        self.receivers
            .iter()
            .find(|(name, _)| Self::camera_receiver(name) && self.settings.camera_index_for_receiver(name) == Some(index))
            .map(|(_, receiver)| receiver)
    }

    pub fn register_receiver(&mut self, receiver: &str) -> Vec<Out> {
        let uri = self.desired_uri(receiver);
        self.receivers.insert(receiver.to_string(), Receiver { uri, ..Receiver::default() });
        match Self::camera_receiver(receiver) {
            true => self.start_receiver(receiver),
            false => Vec::new(),
        }
    }

    pub fn on_settings(&mut self, settings: Settings) -> Vec<Out> {
        let latency_changed = settings.low_latency != self.settings.low_latency;
        let had_video = self.has_video();
        self.settings = settings;
        let names: Vec<String> = self.receivers.keys().cloned().collect();
        let cameras: Vec<String> = names.iter().filter(|name| Self::camera_receiver(name)).cloned().collect();
        let moved: Vec<String> = cameras
            .iter()
            .filter(|name| {
                let uri = self.desired_uri(name);
                self.receivers.get(*name).is_some_and(|receiver| receiver.uri != uri)
            })
            .cloned()
            .collect();
        let reset: Vec<String> = match self.has_video() {
            true => moved.clone(),
            false => cameras.clone(),
        };
        reset.iter().for_each(|name| {
            let uri = self.desired_uri(name);
            self.receivers.entry(name.clone()).and_modify(|receiver| {
                receiver.uri = uri;
                receiver.streaming = false;
                receiver.decoding = false;
                receiver.size = None;
                receiver.last_frame_s = None;
                receiver.attempts = 0;
                receiver.reconnect_attempts = 0;
                receiver.failing_since_s = None;
            });
        });
        let changed = match (latency_changed, had_video) {
            (true, _) => names.clone(),
            (false, false) => cameras,
            (false, true) => moved,
        };
        let restarts: Vec<Out> = match (self.has_video(), changed.is_empty()) {
            (true, true) => Vec::new(),
            (true, false) => changed.iter().flat_map(|name| self.restart_receiver(name)).collect(),
            (false, _) => names.iter().map(|receiver| Out::StopReceiver { receiver: receiver.clone() }).collect(),
        };
        let cleared = match self.has_video() {
            true => Vec::new(),
            false => self.set_full_screen(false),
        };
        std::iter::once(Out::CamerasChanged).chain(self.refresh()).chain(restarts).chain(cleared).collect()
    }

    pub fn on_vehicle(&mut self, present: bool) -> Vec<Out> {
        let was = std::mem::replace(&mut self.vehicle_present, present);
        let stream: Vec<Out> = was.then_some(Out::StopCameraStream).into_iter().chain(present.then_some(Out::StartCameraStream)).collect();
        match present {
            true => stream,
            false => stream.into_iter().chain(self.set_full_screen(false)).collect(),
        }
    }

    pub fn on_communication_lost(&mut self, lost: bool) -> Vec<Out> {
        match lost {
            true => self.set_full_screen(false),
            false => Vec::new(),
        }
    }

    pub fn set_full_screen(&mut self, asked: bool) -> Vec<Out> {
        let wanted = asked && self.has_video();
        match wanted == self.full_screen {
            true => Vec::new(),
            false => {
                self.full_screen = wanted;
                vec![Out::FullScreenChanged(wanted)]
            }
        }
    }

    pub fn set_active_source(&mut self, asked: i64) -> Option<usize> {
        let clamped = self.settings.clamp_active(asked);
        (clamped as i64 != self.settings.active_source).then_some(clamped)
    }

    pub fn started_receivers(&self) -> Vec<String> {
        self.receivers.iter().filter(|(_, state)| state.started).map(|(name, _)| name.clone()).collect()
    }

    pub fn record_refusal(&self) -> Option<&'static str> {
        match (self.settings.recording_format_valid, self.settings.save_path_set, self.started_receivers().is_empty()) {
            (false, ..) => Some(REFUSED_BAD_FORMAT),
            (_, false, _) => Some(REFUSED_NO_SAVE_PATH),
            (_, _, true) => Some(REFUSED_NO_STARTED_RECEIVER),
            _ => None,
        }
    }

    pub fn can_record(&self) -> bool {
        self.record_refusal().is_none()
    }

    pub fn start_recording(&self) -> Vec<Out> {
        match self.can_record() {
            false => Vec::new(),
            true => vec![Out::StartRecording { receivers: self.started_receivers() }],
        }
    }

    pub fn stop_recording(&self) -> Vec<Out> {
        let receivers: Vec<String> = self.receivers.iter().filter(|(_, state)| state.recording).map(|(name, _)| name.clone()).collect();
        match receivers.is_empty() {
            true => Vec::new(),
            false => vec![Out::StopRecording { receivers }],
        }
    }

    pub fn start_receiver(&mut self, receiver: &str) -> Vec<Out> {
        let Some(state) = self.receivers.get(receiver) else { return Vec::new() };
        if state.started {
            return Vec::new();
        }
        if state.uri.is_empty() {
            return self.set_status(receiver, Status::NoStreamUrl, false);
        }
        if !self.has_video() {
            return Vec::new();
        }
        let index = self.settings.camera_index_for_receiver(receiver).unwrap_or_else(|| self.settings.current_index());
        let source = self.settings.source_at(index).to_string();
        let timeout_s = start_timeout_s(&source, self.settings.rtsp_timeout_s);
        let low_latency = self.settings.low_latency;
        self.receivers.entry(receiver.to_string()).and_modify(|state| {
            state.attempts += 1;
            state.timeout_s = Some(timeout_s);
        });
        self.set_status(receiver, Status::Connecting, true)
            .into_iter()
            .chain(std::iter::once(Out::StartReceiver { receiver: receiver.to_string(), timeout_s, low_latency }))
            .collect()
    }

    pub fn restart_receiver(&mut self, receiver: &str) -> Vec<Out> {
        match self.receivers.get(receiver).is_some_and(|state| state.started) {
            true => vec![Out::StopReceiver { receiver: receiver.to_string() }],
            false => self.start_receiver(receiver),
        }
    }

    pub fn on_start_complete(&mut self, receiver: &str, outcome: Outcome, at_s: u64) -> Vec<Out> {
        if !self.receivers.contains_key(receiver) {
            return Vec::new();
        }
        match outcome {
            Outcome::Ok => {
                self.receivers.entry(receiver.to_string()).and_modify(|state| state.started = true);
                self.set_status(receiver, Status::Connecting, true)
            }
            Outcome::InvalidUrl => self.set_status(receiver, Status::InvalidStreamUrl, false),
            Outcome::InvalidState => Vec::new(),
            Outcome::Failed => {
                self.receivers.entry(receiver.to_string()).and_modify(|state| state.failing_since_s = state.failing_since_s.or(Some(at_s)));
                let retry = match self.receivers.get(receiver).is_some_and(|state| state.started) {
                    true => self.restart_receiver(receiver),
                    false => vec![Out::RestartAfter { receiver: receiver.to_string(), delay_ms: RESTART_DELAY_MS }],
                };
                self.set_status(receiver, Status::ConnectionFailed, true).into_iter().chain(retry).collect()
            }
        }
    }

    pub fn on_stop_complete(&mut self, receiver: &str, outcome: Outcome) -> Vec<Out> {
        if !self.receivers.contains_key(receiver) {
            return Vec::new();
        }
        self.receivers.entry(receiver.to_string()).and_modify(|state| state.started = false);
        match outcome {
            Outcome::InvalidUrl => self.set_status(receiver, Status::InvalidStreamUrl, false),
            _ if outcome != Outcome::Failed => self
                .set_status(receiver, Status::Reconnecting, true)
                .into_iter()
                .chain(std::iter::once(Out::RestartAfter { receiver: receiver.to_string(), delay_ms: RESTART_DELAY_MS }))
                .collect(),
            _ if self.settings.reconnect_disabled => self.set_status(receiver, Status::ConnectionFailed, false),
            _ => {
                let attempt = self.receivers.get(receiver).map_or(0, |state| state.reconnect_attempts).saturating_add(1).min(MAX_VIDEO_RECONNECT_ATTEMPTS);
                self.receivers.entry(receiver.to_string()).and_modify(|state| state.reconnect_attempts = attempt);
                self.set_status(receiver, Status::Reconnecting, true)
                    .into_iter()
                    .chain(std::iter::once(Out::RestartAfter { receiver: receiver.to_string(), delay_ms: reconnect_delay_ms(attempt) }))
                    .collect()
            }
        }
    }

    pub fn on_streaming(&mut self, receiver: &str, active: bool) -> Vec<Out> {
        if !Self::camera_receiver(receiver) || !self.receivers.contains_key(receiver) {
            return Vec::new();
        }
        self.receivers.entry(receiver.to_string()).and_modify(|state| state.streaming = active);
        std::iter::once(Out::CamerasChanged).chain(self.refresh()).collect()
    }

    pub fn on_decoding(&mut self, receiver: &str, active: bool) -> Vec<Out> {
        if !Self::camera_receiver(receiver) || !self.receivers.contains_key(receiver) {
            return Vec::new();
        }
        self.receivers.entry(receiver.to_string()).and_modify(|state| {
            state.decoding = active;
            state.attempts = match active {
                true => 0,
                false => state.attempts,
            };
            state.reconnect_attempts = match active {
                true => 0,
                false => state.reconnect_attempts,
            };
            state.failing_since_s = match active {
                true => None,
                false => state.failing_since_s,
            };
        });
        std::iter::once(Out::CamerasChanged).chain(self.refresh()).collect()
    }

    pub fn on_recording(&mut self, receiver: &str, active: bool, at_s: u64) -> Vec<Out> {
        if !Self::camera_receiver(receiver) || !self.receivers.contains_key(receiver) {
            return Vec::new();
        }
        self.receivers.entry(receiver.to_string()).and_modify(|state| {
            state.recording = active;
            state.recording_since_s = match active {
                true => state.recording_since_s.or(Some(at_s)),
                false => None,
            };
        });
        std::iter::once(Out::RecordingChanged { receiver: receiver.to_string(), active })
            .chain((!active).then_some(Out::StopTelemetryCapture))
            .chain(std::iter::once(Out::CamerasChanged))
            .chain(self.refresh())
            .collect()
    }

    pub fn on_video_size(&mut self, receiver: &str, width_pixels: u32, height_pixels: u32) -> Vec<Out> {
        if !Self::camera_receiver(receiver) || !self.receivers.contains_key(receiver) {
            return Vec::new();
        }
        let size = (width_pixels > 0 && height_pixels > 0).then_some((width_pixels, height_pixels));
        self.receivers.entry(receiver.to_string()).and_modify(|state| state.size = size.or(state.size));
        self.refresh()
    }

    pub fn on_frame(&mut self, receiver: &str, at_s: u64) {
        self.receivers.entry(receiver.to_string()).and_modify(|state| {
            state.last_frame_s = Some(at_s);
            state.reconnect_attempts = 0;
        });
    }

    fn set_status(&mut self, receiver: &str, status: Status, connecting: bool) -> Vec<Out> {
        if !Self::camera_receiver(receiver) {
            return Vec::new();
        }
        let changed = self.receivers.get(receiver).is_some_and(|state| state.status != Some(status) || state.connecting != connecting);
        self.receivers.entry(receiver.to_string()).and_modify(|state| {
            state.status = Some(status);
            state.connecting = connecting;
        });
        match changed {
            true => vec![Out::CamerasChanged],
            false => Vec::new(),
        }
    }

    fn refresh(&mut self) -> Vec<Out> {
        let active = self.settings.current_index();
        let (streaming, decoding, recording, size) = self
            .receiver_for(active)
            .map(|state| (state.streaming, state.decoding, state.recording, state.size))
            .unwrap_or((false, false, false, None));
        let streaming_out = (self.streaming != streaming).then_some(Out::StreamingChanged(streaming));
        let decoding_out = (self.decoding != decoding).then_some(Out::DecodingChanged(decoding));
        let recording_out = (self.recording != recording).then_some(Out::ActiveRecordingChanged(recording));
        let size_out = size
            .filter(|new| Some(*new) != self.video_size)
            .map(|(width_pixels, height_pixels)| Out::VideoSizeChanged { width_pixels, height_pixels });
        self.streaming = streaming;
        self.decoding = decoding;
        self.recording = recording;
        self.video_size = size.or(self.video_size);
        streaming_out.into_iter().chain(decoding_out).chain(recording_out).chain(size_out).collect()
    }

    pub fn camera_status(&self, index: usize) -> Status {
        let source = self.settings.source_at(index);
        match index {
            _ if index >= self.settings.count() => Status::NoVideoSource,
            _ if matches!(source, SOURCE_NO_VIDEO | SOURCE_DISABLED) => Status::NoVideoSource,
            _ if !self.configured(index) => Status::NoStreamUrl,
            _ if !self.usable(index) => Status::UnsupportedSource,
            _ if !self.settings.shown(index) => Status::NotShown,
            _ => match self.receiver_for(index) {
                None => Status::NoReceiver,
                Some(state) => match (state.decoding, state.streaming) {
                    (true, _) => Status::Decoding,
                    (false, true) => Status::WaitingForFrames,
                    (false, false) => state.status.unwrap_or(Status::NotStarted),
                },
            },
        }
    }

    pub fn receiver_status(&self, index: usize) -> Option<Status> {
        self.receiver_for(index).map(|state| match (state.decoding, state.streaming) {
            (true, _) => Status::Decoding,
            (false, true) => Status::WaitingForFrames,
            (false, false) => state.status.unwrap_or(Status::NotStarted),
        })
    }

    pub fn camera_signal(&self, index: usize) -> &'static str {
        match (self.has_video() && self.settings.shown(index), self.receiver_for(index)) {
            (false, _) => SIGNAL_IDLE,
            (true, Some(state)) if state.decoding => SIGNAL_LIVE,
            (true, Some(state)) if state.reconnect_attempts > 0 || state.failing_since_s.is_some() => SIGNAL_NONE,
            (true, Some(state)) if state.streaming || state.connecting => SIGNAL_CONNECTING,
            (true, _) => SIGNAL_NONE,
        }
    }

    pub fn camera_connecting(&self, index: usize) -> bool {
        self.receiver_for(index).is_some_and(|state| !state.decoding && (state.streaming || state.connecting))
    }

    pub fn camera_recording(&self, index: usize) -> bool {
        self.receiver_for(index).is_some_and(|state| state.recording)
    }

    pub fn frame_age(&self, index: usize, now_s: u64) -> Age {
        match self.settings.shown(index) {
            false => Age::NotBound,
            true => match self.receiver_for(index).map(|state| state.last_frame_s) {
                None => Age::NoCamera,
                Some(None) => Age::NoFrameYet,
                Some(Some(at_s)) => Age::Seconds(now_s.saturating_sub(at_s)),
            },
        }
    }

    fn size_value(size: Option<(u32, u32)>) -> Value {
        size.map(|(width_pixels, height_pixels)| json!({ "widthPixels": width_pixels, "heightPixels": height_pixels })).unwrap_or(Value::Null)
    }

    pub fn snapshot(&self, now_s: u64) -> Value {
        let current = self.settings.current_index();
        let cameras: Vec<Value> = (0..self.settings.count())
            .map(|index| {
                let age = self.frame_age(index, now_s);
                let receiver = self.receiver_for(index);
                let elapsed = |since: Option<u64>| since.map(|at_s| now_s.saturating_sub(at_s));
                json!({
                    "slot": index,
                    "name": self.settings.name_at(index),
                    "fromDrone": self.settings.from_drone(index),
                    "source": self.settings.source_at(index),
                    "sourceToken": source_token(self.settings.source_at(index)),
                    "requiresRestart": requires_restart(self.settings.source_at(index)),
                    "enabled": self.settings.enabled(index),
                    "configured": self.configured(index),
                    "usable": self.usable(index),
                    "shown": self.settings.shown(index),
                    "status": self.camera_status(index).token(),
                    "connecting": self.camera_connecting(index),
                    "recording": self.camera_recording(index),
                    "recordingSeconds": elapsed(receiver.and_then(|state| state.recording_since_s)),
                    "attempts": receiver.map(|state| state.attempts),
                    "failingSeconds": elapsed(receiver.and_then(|state| state.failing_since_s)),
                    "startTimeoutSeconds": receiver.and_then(|state| state.timeout_s),
                    "videoSize": Self::size_value(receiver.and_then(|state| state.size)),
                    "frameAge": age.token(),
                    "frameAgeSeconds": age.seconds(),
                })
            })
            .collect();
        json!({
            "kind": "object",
            "class": "VideoState",
            "hasVideo": self.has_video(),
            "streamEnabled": self.settings.stream_enabled,
            "streamConfigured": self.stream_configured(),
            "fromDrone": self.from_drone(),
            "streamSource": self.is_stream_source(),
            "streaming": self.streaming,
            "decoding": self.decoding,
            "recording": self.recording,
            "canRecord": self.can_record(),
            "recordRefusal": self.record_refusal(),
            "fullScreen": self.full_screen,
            "lowLatency": self.settings.low_latency,
            "activeSource": current,
            "requestedSource": self.settings.active_source,
            "activeSourceRefused": self.settings.active_refusal(),
            "multipleSources": self.has_multiple_sources(),
            "multiView": self.settings.multi_view,
            "videoSize": Self::size_value(self.receiver_for(current).and_then(|state| state.size)),
            "lastKnownVideoSize": Self::size_value(self.video_size),
            "switchable": self.settings.switchable(),
            "pipSource": self.settings.pip_camera(),
            "nextSource": self.settings.next_switchable(),
            "cameras": cameras,
        })
    }
}

pub fn video_source_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(source) = args.first().map(String::as_str).filter(|source| !source.is_empty()) else {
        return crate::read::refused("this needs a video source token, then optionally its url and the rtsp timeout in seconds");
    };
    let url = args.get(1).map(String::as_str).unwrap_or("");
    let rtsp_timeout_s = args.get(2).and_then(|arg| arg.trim().parse::<u32>().ok());
    let uri = source_uri(source, url);
    json!({
        "kind": "object",
        "class": "VideoSource",
        "source": source,
        "sourceToken": source_token(source),
        "requiresRestart": requires_restart(source),
        "enabled": source != SOURCE_DISABLED,
        "streamSource": is_stream_source(source),
        "needsUrl": needs_url(source),
        "configured": !needs_url(source) || !url.is_empty(),
        "usable": source_usable(source, url),
        "negotiates": negotiates(source),
        "uri": (!uri.is_empty()).then_some(uri),
        "startTimeoutSeconds": rtsp_timeout_s.map(|timeout| start_timeout_s(source, timeout)).or_else(|| (!negotiates(source)).then_some(START_TIMEOUT_S)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cam(source: &str, url: &str, name: &str) -> SourceSlot {
        SourceSlot { source: source.to_string(), url: url.to_string(), name: name.to_string(), drone: false }
    }

    fn only(camera: SourceSlot) -> Settings {
        Settings { cameras: vec![camera], ..settings() }
    }

    fn settings() -> Settings {
        Settings {
            cameras: vec![cam(SOURCE_RTSP, "rtsp://10.0.0.1:8554/live", ""), cam(SOURCE_UDP_H265, "0.0.0.0:5601", "Thermal side"), cam(SOURCE_RTSP, "", "")],
            active_source: 0,
            multi_view: false,
            pip_shown: true,
            stream_enabled: true,
            low_latency: false,
            save_path_set: true,
            recording_format_valid: true,
            rtsp_timeout_s: 12,
            reconnect_disabled: false,
        }
    }

    fn wire(settings: Settings, receivers: &[&str]) -> VideoState {
        let mut state = VideoState { settings, ..VideoState::default() };
        receivers.iter().for_each(|name| {
            state.register_receiver(name);
        });
        state
    }

    fn wired() -> VideoState {
        wire(settings(), &[MAIN_RECEIVER, THERMAL_RECEIVER, PIP_RECEIVER])
    }

    #[test]
    fn switching_the_stream_off_stops_every_camera_and_reads_no_picture() {
        let mut state = wired();
        state.on_streaming(MAIN_RECEIVER, true);
        state.on_decoding(MAIN_RECEIVER, true);
        assert!(state.decoding);
        let outs = state.on_settings(Settings { stream_enabled: false, ..settings() });
        assert!(!state.decoding && !state.streaming, "a stream that is off has no picture, so the Video off panel can show and offer to turn it back on");
        assert!(outs.iter().any(|out| *out == Out::StopReceiver { receiver: MAIN_RECEIVER.to_string() }), "and the main pipeline is told to stop");
        state.on_settings(settings());
        assert!(!state.decoding, "turning it back on starts from nothing rather than a stale picture");
    }

    #[test]
    fn a_source_is_usable_only_once_it_has_the_address_its_kind_needs() {
        assert!(needs_url(SOURCE_WEBRTC) && needs_url(SOURCE_RTSP) && needs_url(SOURCE_MPEGTS), "the six url kinds are the ones that stay unconfigured until an address is typed");
        assert!(!needs_url(SOURCE_HERELINK_AIR_UNIT), "a fixed-address source is configured the moment it is picked");
        assert!(is_stream_source(SOURCE_HERELINK_HOTSPOT) && !is_stream_source(SOURCE_DISABLED));
        let mut state = VideoState { settings: Settings { cameras: vec![cam(SOURCE_RTSP, "", "")], stream_enabled: true, ..Settings::default() }, ..VideoState::default() };
        assert!(!state.stream_configured(), "rtsp with no url is not configured");
        assert!(!state.has_video());
        state.settings.cameras[0].url = "rtsp://1.2.3.4/live".to_string();
        assert!(state.has_video(), "stream enabled plus a configured source is the whole of hasVideo");
        state.settings.stream_enabled = false;
        assert!(!state.has_video(), "the enable switch alone can take video away");
        state.settings.cameras[0].source = SOURCE_HERELINK_AIR_UNIT.to_string();
        state.settings.stream_enabled = true;
        assert!(state.stream_configured(), "herelink needs no url");
        state.settings.cameras[0].source = SOURCE_NO_VIDEO.to_string();
        assert!(!state.stream_configured());
        assert!(!VideoState { settings: Settings { stream_enabled: true, ..Settings::default() }, ..VideoState::default() }.has_video(), "no cameras at all is no video, not a crash on camera zero");
    }

    #[test]
    fn a_configured_camera_that_is_off_screen_is_not_a_camera_with_no_source() {
        let state = wired();
        assert_eq!(state.camera_status(1), Status::NotShown, "camera one is wired and configured, it is only not on screen, and saying noVideoSource sends the operator looking for a cable");
        assert_eq!(state.frame_age(1, 100), Age::NotBound, "a camera that is not bound has no frame clock, which is not the same as having no receiver");
        let view = state.snapshot(100);
        assert_eq!(view["cameras"][1]["shown"], false, "the binding is its own fact so a head never has to infer it from the status token");
        assert_eq!(view["cameras"][1]["status"], "notShown");
        assert_eq!(view["cameras"][1]["frameAge"], "notBound");
        assert_eq!(view["cameras"][1]["configured"], true, "and the row stays self-consistent: configured, enabled, switchable and simply not on screen");
        assert_eq!(view["cameras"][0]["shown"], true);
        let blank = wire(only(cam(SOURCE_NO_VIDEO, "", "")), &[MAIN_RECEIVER]);
        assert_eq!(blank.camera_status(0), Status::NoVideoSource, "noVideoSource is reserved for a slot whose source really is none");
        let multi = wire(Settings { multi_view: true, ..settings() }, &[MAIN_RECEIVER]);
        assert_eq!(multi.camera_status(1), Status::NoReceiver, "a shown camera the host never registered a receiver for is a fourth answer again");
    }

    #[test]
    fn a_vehicle_preset_plays_its_fixed_address_and_no_source_never_looks_ready() {
        let solo = wire(only(cam(SOURCE_3DR_SOLO, "", "")), &[MAIN_RECEIVER]);
        assert!(solo.has_video() && solo.usable(0), "3DR Solo, Parrot Discovery and Yuneec Mantis G play the fixed address QGC always gave them");
        assert_eq!(source_uri(SOURCE_PARROT_DISCOVERY, ""), "udp://0.0.0.0:8888");
        assert!(source_usable(SOURCE_YUNEEC_MANTIS_G, ""));
        let blank = wire(only(cam(SOURCE_NO_VIDEO, "", "")), &[MAIN_RECEIVER]);
        assert!(!blank.usable(0) && blank.snapshot(0)["cameras"][0]["usable"] == false, "no-video and disabled are unusable too, where configured alone called them ready");
        assert!(wired().usable(1), "a configured udp camera stays usable");
    }

    #[test]
    fn every_source_carries_a_token_the_head_can_localise() {
        assert_eq!(source_token(SOURCE_UDP_H264), "udpH264");
        assert_eq!(source_token(SOURCE_UDP_H265), "udpH265");
        assert_eq!(source_token(SOURCE_RTSP), "rtsp");
        assert_eq!(source_token(SOURCE_TCP), "tcp");
        assert_eq!(source_token(SOURCE_MPEGTS), "mpegts");
        assert_eq!(source_token(SOURCE_WEBRTC), "webrtc");
        assert_eq!(source_token(SOURCE_3DR_SOLO), "solo3dr");
        assert_eq!(source_token(SOURCE_PARROT_DISCOVERY), "parrot");
        assert_eq!(source_token(SOURCE_YUNEEC_MANTIS_G), "yuneecMantisG");
        assert_eq!(source_token(SOURCE_HERELINK_AIR_UNIT), "herelinkAirUnit");
        assert_eq!(source_token(SOURCE_HERELINK_HOTSPOT), "herelinkHotspot");
        assert_eq!(source_token(SOURCE_DISABLED), "disabled");
        assert_eq!(source_token(SOURCE_NO_VIDEO), "noVideo");
        assert_eq!(source_token("something a plugin added"), "unknown");
        assert!(requires_restart(SOURCE_3DR_SOLO) && !requires_restart(SOURCE_RTSP), "the parenthetical instruction in the solo label is a flag, not prose to be string-matched");
        let view = wired().snapshot(0);
        assert_eq!(view["cameras"][0]["sourceToken"], "rtsp", "the settings key stays english, the token is what a ukrainian head localises against");
        assert_eq!(view["cameras"][0]["requiresRestart"], false);
        assert_eq!(video_source_view(&NoBackend, &[SOURCE_3DR_SOLO.to_string()])["sourceToken"], "solo3dr");
        assert_eq!(video_source_view(&NoBackend, &[SOURCE_3DR_SOLO.to_string()])["requiresRestart"], true);
    }

    #[test]
    fn the_uri_of_every_source_kind_keeps_the_exact_address_the_cpp_used() {
        assert_eq!(source_uri(SOURCE_UDP_H264, "0.0.0.0:5600"), "udp://0.0.0.0:5600");
        assert_eq!(source_uri(SOURCE_UDP_H265, "0.0.0.0:5600"), "udp265://0.0.0.0:5600");
        assert_eq!(source_uri(SOURCE_MPEGTS, "0.0.0.0:5600"), "mpegts://0.0.0.0:5600");
        assert_eq!(source_uri(SOURCE_TCP, "1.2.3.4:5600"), "tcp://1.2.3.4:5600");
        assert_eq!(source_uri(SOURCE_RTSP, "rtsp://1.2.3.4/live"), "rtsp://1.2.3.4/live", "an rtsp url is already a uri");
        assert_eq!(source_uri(SOURCE_WEBRTC, "  http://1.2.3.4/whep  "), "http://1.2.3.4/whep");
        assert_eq!(source_uri(SOURCE_3DR_SOLO, "ignored"), "udp://0.0.0.0:5600");
        assert_eq!(source_uri(SOURCE_PARROT_DISCOVERY, ""), "udp://0.0.0.0:8888");
        assert_eq!(source_uri(SOURCE_YUNEEC_MANTIS_G, ""), "rtsp://192.168.42.1:554/live");
        assert_eq!(source_uri(SOURCE_HERELINK_AIR_UNIT, ""), "rtsp://192.168.0.10:8554/H264Video");
        assert_eq!(source_uri(SOURCE_HERELINK_HOTSPOT, ""), "rtsp://192.168.43.1:8554/fpv_stream");
        assert_eq!(source_uri(SOURCE_DISABLED, "x"), "", "a disabled source has no uri, which is what stops a receiver");
    }

    #[test]
    fn the_timeouts_are_the_cpp_numbers_and_only_negotiating_sources_get_the_long_one() {
        assert_eq!(start_timeout_s(SOURCE_UDP_H264, 12), 3, "a plain udp stream gets the three second start budget");
        assert_eq!(start_timeout_s(SOURCE_RTSP, 12), 12, "rtsp may fall back to tcp after five seconds, so it gets the configured budget");
        assert_eq!(start_timeout_s(SOURCE_WEBRTC, 12), 12, "whep signalling plus ice needs the same headroom as rtsp");
        assert_eq!(RESTART_DELAY_MS, 1000, "a stopped receiver is first retried after exactly one second");
        assert_eq!((1..=8).map(reconnect_delay_ms).collect::<Vec<_>>(), [1000, 2000, 4000, 8000, 16000, 30000, 30000, 30000], "GstVideoReceiver::_scheduleReconnect backs off 1, 2, 4, 8, 16 then 30 s");
    }

    #[test]
    fn only_configured_stream_sources_can_be_switched_to_and_the_switch_wraps() {
        let state = wired();
        assert_eq!(state.settings.switchable(), vec![0, 1], "slot two has no url so it is not offered");
        assert_eq!(state.settings.current_index(), 0);
        assert_eq!(state.settings.next_switchable(), Some(1));
        let on_second = VideoState { settings: Settings { active_source: 1, ..settings() }, ..VideoState::default() };
        assert_eq!(on_second.settings.current_index(), 1);
        assert_eq!(on_second.settings.next_switchable(), Some(0), "the switch wraps back round to the first camera");
        let unconfigured = VideoState { settings: Settings { active_source: 2, ..settings() }, ..VideoState::default() };
        assert_eq!(unconfigured.settings.current_index(), 0, "an active camera that cannot show falls back to the first one that can");
        let first_broken = VideoState { settings: Settings { cameras: vec![cam(SOURCE_RTSP, "", ""), cam(SOURCE_UDP_H264, "0.0.0.0:5600", "")], ..settings() }, ..VideoState::default() };
        assert_eq!(first_broken.settings.current_index(), 1, "no camera is the primary: when the first cannot show, the next that can is on screen");
        assert!(first_broken.has_video());
        let out_of_range = VideoState { settings: Settings { active_source: 9, ..settings() }, ..VideoState::default() };
        assert_eq!(out_of_range.settings.current_index(), 0);
        assert_eq!(out_of_range.settings.clamp_active(9), 2, "setting the active camera clamps into the slot range instead of refusing");
        assert_eq!(out_of_range.settings.clamp_active(-4), 0);
        let single = VideoState { settings: only(cam(SOURCE_RTSP, "rtsp://10.0.0.1:8554/live", "")), ..VideoState::default() };
        assert_eq!(single.settings.next_switchable(), None, "one camera cannot be switched away from");
        assert!(!single.has_multiple_sources());
    }

    #[test]
    fn the_camera_the_operator_asked_for_is_reported_beside_the_one_they_got() {
        let unconfigured = VideoState { settings: Settings { active_source: 2, ..settings() }, ..VideoState::default() }.snapshot(0);
        assert_eq!(unconfigured["activeSource"], 0);
        assert_eq!(unconfigured["requestedSource"], 2, "the tap the operator made has to survive into the snapshot, or a refused switch looks like a missed touch");
        assert_eq!(unconfigured["activeSourceRefused"], REFUSED_SOURCE_UNCONFIGURED);
        let out_of_range = VideoState { settings: Settings { active_source: 9, ..settings() }, ..VideoState::default() }.snapshot(0);
        assert_eq!(out_of_range["requestedSource"], 9);
        assert_eq!(out_of_range["activeSourceRefused"], REFUSED_SOURCE_OUT_OF_RANGE);
        let accepted = VideoState { settings: Settings { active_source: 1, ..settings() }, ..VideoState::default() }.snapshot(0);
        assert_eq!(accepted["activeSourceRefused"], Value::Null, "a switch that was honoured names no refusal");
        assert_eq!(accepted["requestedSource"], 1);
    }

    #[test]
    fn the_picture_in_picture_is_the_one_usable_camera_after_the_one_on_screen_wrapping_round() {
        let four = |active_source: i64| Settings { cameras: vec![cam(SOURCE_RTSP, "rtsp://a/0", ""), cam(SOURCE_RTSP, "", ""), cam(SOURCE_RTSP, "rtsp://a/2", ""), cam(SOURCE_RTSP, "rtsp://a/3", "")], multi_view: true, active_source, ..settings() };
        assert_eq!([0, 2, 3].map(|active| four(active).pip_camera()), [Some(2), Some(3), Some(0)], "first, middle and last: the next usable camera, skipping one with no address and wrapping round");
        assert_eq!([0, 2, 3].map(|active| four(active).camera_index_for_receiver(PIP_RECEIVER)), [Some(2), Some(3), Some(0)], "the picture in picture receiver plays exactly that camera");
        assert_eq!([0, 2, 3].map(|active| four(active).next_switchable()), [Some(2), Some(3), Some(0)], "which is the camera the switch goes to next");
        assert_eq!((0..4).filter(|index| four(0).shown(*index)).collect::<Vec<_>>(), vec![0, 2], "the camera on screen and the picture in picture are the only cameras played");
        assert_eq!(["extraVideo0", "extraVideo2", "extraVideo7"].map(|receiver| four(0).camera_index_for_receiver(receiver)), [None; 3], "no other tile receiver carries a camera");
        let drone = SourceSlot { drone: true, ..cam(SOURCE_UDP_H264, "0.0.0.0:5600", "SIYI A8") };
        let mine_and_drone = |active_source: i64| Settings { cameras: vec![cam(SOURCE_RTSP, "rtsp://a/0", ""), drone.clone()], multi_view: true, active_source, ..settings() };
        assert_eq!((mine_and_drone(0).pip_camera(), mine_and_drone(1).pip_camera()), (Some(1), Some(0)), "a drone camera is in the same ring as the operator's");
        assert_eq!(Settings { multi_view: true, ..only(cam(SOURCE_RTSP, "rtsp://a/0", "")) }.camera_index_for_receiver(PIP_RECEIVER), None, "one camera has nothing to put in the picture in picture");
        let off = Settings { multi_view: false, ..four(0) };
        assert_eq!((off.pip_camera(), off.camera_index_for_receiver(PIP_RECEIVER), off.next_switchable()), (None, None, Some(2)), "with the switch off nothing plays in the picture in picture, though the switch still knows the next camera");
        assert!(off.shown(0) && !off.shown(2));
    }

    #[test]
    fn the_picture_in_picture_never_plays_the_stream_the_main_camera_is_already_listening_for() {
        let cameras = vec![
            cam(SOURCE_UDP_H264, "0.0.0.0:5600", "Belly"),
            SourceSlot { drone: true, ..cam(SOURCE_UDP_H264, "127.0.0.1:5600", "MockCam") },
            cam(SOURCE_MPEGTS, "0.0.0.0:5601", "Tail"),
            cam(SOURCE_RTSP, "rtsp://a/0", "Gimbal"),
            cam(SOURCE_RTSP, "rtsp://a/0", "Gimbal again"),
        ];
        let at = |active_source: i64| Settings { cameras: cameras.clone(), multi_view: true, active_source, ..settings() }.pip_camera();
        assert_eq!(at(0), Some(2), "one UDP port feeds one socket: two receivers on 5600 leave one starved and restarting forever, so the drone's 127.0.0.1:5600 is skipped");
        assert_eq!(at(1), Some(2));
        assert_eq!(at(3), Some(0), "the same RTSP address is the same picture, so it is skipped too");
        assert_eq!(at(4), Some(0));
        let pair = Settings { cameras: cameras[..2].to_vec(), multi_view: true, ..settings() };
        assert_eq!((pair.pip_camera(), pair.next_switchable()), (None, Some(1)), "with nothing else to show there is no picture in picture, though the switch still offers the other camera");
        assert!(same_stream("udp265://0.0.0.0:5600", "mpegts://10.0.0.1:5600/x") && !same_stream("udp://0.0.0.0:5600", "udp://0.0.0.0:5601") && !same_stream("tcp://a:5600", "tcp://b:5600"), "only a listen port is shared; a TCP client dials out");
    }

    #[test]
    fn one_camera_typed_two_ways_is_still_one_stream() {
        assert!(same_stream("RTSP://admin:pw@Cam.Local:554/live/", "rtsp://cam.local:554/live"), "scheme and host are case-blind, and credentials or a trailing slash do not make another camera");
        assert!(same_stream("HTTP://SFU.example/whep/front", "http://sfu.example/whep/front/"));
        assert!(!same_stream("rtsp://cam/Live", "rtsp://cam/live"), "a path is the server's to compare, so its case counts");
        assert!(!same_stream("rtsp://cam/live", "rtsps://cam/live") && !same_stream("rtsp://cam:554/live", "rtsp://cam:8554/live"));
        let typed_twice = Settings { cameras: vec![cam(SOURCE_RTSP, "rtsp://cam.local/live", "Gimbal"), cam(SOURCE_RTSP, "RTSP://user@CAM.local/live/", "Gimbal again"), cam(SOURCE_UDP_H264, "0.0.0.0:5600", "Belly")], multi_view: true, ..settings() };
        assert_eq!(typed_twice.pip_camera(), Some(2), "the second spelling of the main camera is skipped for the picture in picture");
    }

    #[test]
    fn the_phone_never_plays_two_of_its_own_cameras_at_once() {
        let phone = |active_source: i64| Settings { cameras: vec![cam(SOURCE_BACK_CAMERA, "", "Back"), cam(SOURCE_FRONT_CAMERA, "", "Front"), cam(SOURCE_RTSP, "rtsp://a/0", "Gimbal")], multi_view: true, active_source, ..settings() };
        assert_eq!(phone(0).pip_camera(), Some(2), "a phone opens one camera at a time, so the front camera is skipped beside the back one");
        assert_eq!(phone(1).pip_camera(), Some(2));
        assert_eq!(phone(2).pip_camera(), Some(0), "a drone camera on screen can have a phone camera beside it");
        let only_phone = Settings { cameras: phone(0).cameras[..2].to_vec(), ..phone(0) };
        assert_eq!((only_phone.pip_camera(), only_phone.next_switchable()), (None, Some(1)), "with only the phone's two cameras there is no picture in picture, though the switch still flips between them");
    }

    #[test]
    fn the_picture_in_picture_plays_only_while_a_head_shows_it() {
        let shown = Settings { multi_view: true, ..settings() };
        assert_eq!(shown.camera_index_for_receiver(PIP_RECEIVER), Some(1));
        let hidden = Settings { pip_shown: false, ..shown.clone() };
        assert_eq!((hidden.camera_index_for_receiver(PIP_RECEIVER), hidden.pip_choice()), (None, Some(1)), "a thumbnail nobody can see decodes nothing, though the camera it would show is still known");
        assert!(!hidden.shown(1));
        let mut state = wire(shown, &[MAIN_RECEIVER, PIP_RECEIVER]);
        assert!(state.started_receivers().is_empty());
        state.on_start_complete(PIP_RECEIVER, Outcome::Ok, 0);
        let outs = state.on_settings(hidden);
        assert!(outs.contains(&Out::StopReceiver { receiver: PIP_RECEIVER.to_string() }), "hiding the thumbnail stops its receiver");
        assert!(!outs.contains(&Out::StopReceiver { receiver: MAIN_RECEIVER.to_string() }), "and leaves the main one alone");
        assert_eq!(state.camera_signal(1), SIGNAL_IDLE);
    }

    #[test]
    fn turning_the_stream_back_on_starts_every_camera_again_however_long_it_was_off() {
        let mut state = wire(Settings { multi_view: true, ..settings() }, &[MAIN_RECEIVER, PIP_RECEIVER]);
        [MAIN_RECEIVER, PIP_RECEIVER].iter().for_each(|name| {
            state.on_start_complete(name, Outcome::Ok, 0);
        });
        state.on_settings(Settings { stream_enabled: false, multi_view: true, ..settings() });
        [MAIN_RECEIVER, PIP_RECEIVER].iter().for_each(|name| {
            state.on_stop_complete(name, Outcome::Ok);
            assert!(state.start_receiver(name).is_empty(), "{name}: the retry the stop scheduled finds no video and is spent");
        });
        let on = state.on_settings(Settings { multi_view: true, ..settings() });
        [MAIN_RECEIVER, PIP_RECEIVER].iter().for_each(|name| {
            assert!(on.iter().any(|out| matches!(out, Out::StartReceiver { receiver, .. } if receiver == name)), "{name}: switching the stream on starts it, with no retry left to do it");
        });
    }

    #[test]
    fn every_camera_reads_live_connecting_no_signal_or_idle() {
        let mut state = wired();
        assert_eq!((state.camera_signal(0), state.camera_signal(1), state.camera_signal(2)), (SIGNAL_CONNECTING, SIGNAL_IDLE, SIGNAL_IDLE), "a camera nobody plays is idle, configured or not");
        state.on_decoding(MAIN_RECEIVER, true);
        assert_eq!(state.camera_signal(0), SIGNAL_LIVE);
        state.on_decoding(MAIN_RECEIVER, false);
        state.on_stop_complete(MAIN_RECEIVER, Outcome::Failed);
        assert_eq!(state.camera_signal(0), SIGNAL_NONE, "a stream that failed is no signal while it waits to retry");
        state.start_receiver(MAIN_RECEIVER);
        assert_eq!(state.camera_signal(0), SIGNAL_NONE, "and stays so through the retries instead of flickering back to connecting");
        state.on_frame(MAIN_RECEIVER, 5);
        assert_eq!(state.camera_signal(0), SIGNAL_CONNECTING, "a frame arriving ends the failure");
        state.on_stop_complete(MAIN_RECEIVER, Outcome::Ok);
        assert_eq!(state.camera_signal(0), SIGNAL_CONNECTING, "a deliberate restart is not a lost signal");
        state.on_stop_complete(MAIN_RECEIVER, Outcome::Failed);
        state.on_settings(Settings { active_source: 1, ..settings() });
        assert_eq!(state.camera_signal(1), SIGNAL_CONNECTING, "a camera switched to does not inherit the last camera's failure");
        assert_eq!(state.camera_signal(0), SIGNAL_IDLE);
        let off = wire(Settings { stream_enabled: false, ..settings() }, &[MAIN_RECEIVER]);
        assert_eq!(off.camera_signal(0), SIGNAL_IDLE, "with the stream switched off no camera is played");
        let unregistered = wire(Settings { multi_view: true, ..settings() }, &[MAIN_RECEIVER]);
        assert_eq!(unregistered.camera_signal(1), SIGNAL_NONE, "a picture in picture camera with no receiver to play it is no signal, not idle");
        let mut pip = wire(Settings { multi_view: true, ..settings() }, &[MAIN_RECEIVER, PIP_RECEIVER]);
        assert_eq!((pip.camera_signal(0), pip.camera_signal(1), pip.camera_signal(2)), (SIGNAL_CONNECTING, SIGNAL_CONNECTING, SIGNAL_IDLE), "the picture in picture camera is played, the rest are idle");
        pip.on_decoding(PIP_RECEIVER, true);
        assert_eq!(pip.camera_signal(1), SIGNAL_LIVE);
        pip.on_decoding(PIP_RECEIVER, false);
        pip.on_stop_complete(PIP_RECEIVER, Outcome::Failed);
        assert_eq!((pip.camera_signal(0), pip.camera_signal(1)), (SIGNAL_CONNECTING, SIGNAL_NONE), "a stalled picture in picture is no signal without touching the main camera");
    }

    #[test]
    fn the_main_receiver_shows_the_camera_on_screen_and_the_picture_in_picture_the_next() {
        let state = wired();
        assert_eq!(state.settings.camera_index_for_receiver(MAIN_RECEIVER), Some(0));
        assert_eq!(state.settings.camera_index_for_receiver(PIP_RECEIVER), None, "with the picture in picture off only the main receiver is bound");
        assert_eq!(state.settings.camera_index_for_receiver(THERMAL_RECEIVER), None, "the thermal receiver is no camera slot");
        let second = Settings { active_source: 1, ..settings() };
        assert_eq!(second.camera_index_for_receiver(MAIN_RECEIVER), Some(1), "the main receiver follows the camera the operator picked, so a host that plays only one stream plays the right one");
        let multi = Settings { multi_view: true, active_source: 1, ..settings() };
        assert_eq!(multi.camera_index_for_receiver(PIP_RECEIVER), Some(0), "the picture in picture plays the next camera round");
        assert_eq!(Settings::default().camera_index_for_receiver(MAIN_RECEIVER), None, "with no camera the main receiver shows nothing");
        let mut state = wired();
        let switched = state.on_settings(Settings { active_source: 1, ..settings() });
        assert!(switched.contains(&Out::StopReceiver { receiver: MAIN_RECEIVER.to_string() }) || switched.iter().any(|out| matches!(out, Out::StartReceiver { receiver, .. } if receiver == MAIN_RECEIVER)), "switching camera restarts the main receiver on the new camera's address");
        assert_eq!(state.desired_uri(MAIN_RECEIVER), "udp265://0.0.0.0:5601");
    }

    #[test]
    fn a_camera_with_no_receiver_reads_differently_from_one_that_has_not_started() {
        let mut idle = wire(Settings { stream_enabled: false, ..settings() }, &[MAIN_RECEIVER]);
        assert_eq!(idle.camera_status(0), Status::NotStarted, "a registered receiver that was never started says so");
        assert_eq!(idle.camera_status(1), Status::NotShown, "camera one is configured and simply off screen");
        idle.settings.multi_view = true;
        assert_eq!(idle.camera_status(1), Status::NoReceiver, "once it is on screen with no receiver registered, that is what it says");
        let mut state = wired();
        assert_eq!(state.camera_status(0), Status::Connecting, "registering a receiver while there is video to show starts it");
        state.on_streaming(MAIN_RECEIVER, true);
        assert_eq!(state.camera_status(0), Status::WaitingForFrames, "streaming without frames is progress, not a picture");
        assert!(state.camera_connecting(0));
        state.on_decoding(MAIN_RECEIVER, true);
        assert_eq!(state.camera_status(0), Status::Decoding);
        assert!(!state.camera_connecting(0), "a decoding camera has arrived and is no longer connecting");
        state.on_decoding(MAIN_RECEIVER, false);
        state.on_streaming(MAIN_RECEIVER, false);
        assert_eq!(state.camera_status(0), Status::Connecting, "the stored status is what shows once the stream drops");
    }

    #[test]
    fn a_receiver_with_no_address_is_told_so_instead_of_being_started() {
        let mut state = wire(Settings { cameras: vec![cam(SOURCE_RTSP, "", "")], stream_enabled: true, ..Settings::default() }, &[MAIN_RECEIVER]);
        let out = state.start_receiver(MAIN_RECEIVER);
        assert!(!out.iter().any(|o| matches!(o, Out::StartReceiver { .. })), "an empty uri must not be started");
        assert_eq!(state.camera_status(0), Status::NoStreamUrl);
        let mut ready = wired();
        ready.on_stop_complete(MAIN_RECEIVER, Outcome::Failed);
        let out = ready.start_receiver(MAIN_RECEIVER);
        assert!(
            out.contains(&Out::StartReceiver { receiver: MAIN_RECEIVER.to_string(), timeout_s: 12, low_latency: false }),
            "rtsp starts with the configured negotiation budget"
        );
        assert_eq!(ready.camera_status(0), Status::Connecting);
    }

    #[test]
    fn a_stop_after_the_stream_is_switched_off_cannot_resurrect_the_receiver() {
        let mut state = wired();
        state.on_start_complete(MAIN_RECEIVER, Outcome::Ok, 0);
        let moved = Settings { stream_enabled: false, ..settings() };
        let gone = state.on_settings(Settings { cameras: std::iter::once(cam(SOURCE_RTSP, "rtsp://10.0.0.9:8554/live", "")).chain(moved.cameras.iter().skip(1).cloned()).collect(), ..moved });
        assert!(gone.contains(&Out::StopReceiver { receiver: MAIN_RECEIVER.to_string() }), "turning the stream off while a url also moved stops every receiver");
        let stopped = state.on_stop_complete(MAIN_RECEIVER, Outcome::Failed);
        assert!(stopped.contains(&Out::RestartAfter { receiver: MAIN_RECEIVER.to_string(), delay_ms: RESTART_DELAY_MS }));
        assert!(state.start_receiver(MAIN_RECEIVER).is_empty(), "and the retry that stop scheduled must find no video to show, or the stop never sticks");
    }

    #[test]
    fn a_deliberate_restart_is_not_a_failure_to_back_off_from() {
        let mut state = wired();
        state.on_start_complete(MAIN_RECEIVER, Outcome::Ok, 0);
        state.settings.reconnect_disabled = true;
        let restarted = (0..8).map(|_| state.on_stop_complete(MAIN_RECEIVER, Outcome::Ok)).last().unwrap();
        assert!(restarted.contains(&Out::RestartAfter { receiver: MAIN_RECEIVER.to_string(), delay_ms: RESTART_DELAY_MS }), "a settings restart starts again at once, auto-reconnect or not, however many times");
        state.settings.reconnect_disabled = false;
        state.on_stop_complete(MAIN_RECEIVER, Outcome::Failed);
        state.on_stop_complete(MAIN_RECEIVER, Outcome::Failed);
        state.on_decoding(MAIN_RECEIVER, true);
        assert!(state.on_stop_complete(MAIN_RECEIVER, Outcome::Failed).contains(&Out::RestartAfter { receiver: MAIN_RECEIVER.to_string(), delay_ms: RESTART_DELAY_MS }), "decoding frames resets the backoff, as the first tee frame does in GstVideoReceiver");
    }

    #[test]
    fn a_deliberate_stop_after_a_failure_restarts_at_once_even_without_auto_reconnect() {
        let mut state = wired();
        state.on_start_complete(MAIN_RECEIVER, Outcome::Failed, 10);
        state.settings.reconnect_disabled = true;
        let stopped = state.on_stop_complete(MAIN_RECEIVER, Outcome::Ok);
        assert!(stopped.contains(&Out::RestartAfter { receiver: MAIN_RECEIVER.to_string(), delay_ms: RESTART_DELAY_MS }), "VideoManager restarts after any stop that is not a bad URL; only a failed one backs off");
    }

    #[test]
    fn every_failed_start_and_every_stop_schedules_something_that_clears_the_connecting_flag() {
        let mut state = wired();
        state.on_start_complete(MAIN_RECEIVER, Outcome::Ok, 0);
        assert_eq!(state.camera_status(0), Status::Connecting);
        let failed = state.on_start_complete(MAIN_RECEIVER, Outcome::Failed, 10);
        assert_eq!(state.camera_status(0), Status::ConnectionFailed);
        assert!(failed.contains(&Out::StopReceiver { receiver: MAIN_RECEIVER.to_string() }), "a started receiver is stopped first and restarts on its stop");
        let stopped = state.on_stop_complete(MAIN_RECEIVER, Outcome::Failed);
        assert_eq!(state.camera_status(0), Status::Reconnecting);
        assert!(stopped.contains(&Out::RestartAfter { receiver: MAIN_RECEIVER.to_string(), delay_ms: RESTART_DELAY_MS }), "a stop must schedule the retry, or the receiver is latched off");
        let again = state.on_stop_complete(MAIN_RECEIVER, Outcome::Failed);
        assert!(again.contains(&Out::RestartAfter { receiver: MAIN_RECEIVER.to_string(), delay_ms: 2000 }), "the second retry waits longer");
        state.on_frame(MAIN_RECEIVER, 20);
        assert!(state.on_stop_complete(MAIN_RECEIVER, Outcome::Failed).contains(&Out::RestartAfter { receiver: MAIN_RECEIVER.to_string(), delay_ms: RESTART_DELAY_MS }), "a frame resets the backoff");
        state.settings.reconnect_disabled = true;
        assert!(!state.on_stop_complete(MAIN_RECEIVER, Outcome::Failed).iter().any(|o| matches!(o, Out::RestartAfter { .. })), "rtspAutoReconnect off gives up after one failure");
        assert_eq!(state.camera_status(0), Status::ConnectionFailed);
        state.settings.reconnect_disabled = false;
        let bad_url = state.on_stop_complete(MAIN_RECEIVER, Outcome::InvalidUrl);
        assert_eq!(state.camera_status(0), Status::InvalidStreamUrl);
        assert!(!bad_url.iter().any(|o| matches!(o, Out::RestartAfter { .. })), "an invalid url is not retried forever");
        assert!(!state.camera_connecting(0), "an invalid url is not progress being made");
    }

    #[test]
    fn a_stream_that_will_never_come_up_counts_its_attempts_and_dates_the_failure() {
        let mut state = wired();
        let first = state.snapshot(0);
        assert_eq!(first["cameras"][0]["attempts"], 1, "the start the registration made is an attempt, and the head has nothing else to count with");
        assert_eq!(first["cameras"][0]["startTimeoutSeconds"], 12, "the budget the start was given is what turns connecting into wait-or-go-check-the-cable");
        assert_eq!(first["cameras"][0]["failingSeconds"], Value::Null, "a first connect is not yet a failure");
        assert!(state.on_start_complete(MAIN_RECEIVER, Outcome::Failed, 10).contains(&Out::RestartAfter { receiver: MAIN_RECEIVER.to_string(), delay_ms: RESTART_DELAY_MS }), "a start that never ran retries a second later, as VideoManager's singleShot(1000) does, rather than at once");
        state.start_receiver(MAIN_RECEIVER);
        state.on_start_complete(MAIN_RECEIVER, Outcome::Failed, 20);
        state.start_receiver(MAIN_RECEIVER);
        let failing = state.snapshot(50);
        assert_eq!(failing["cameras"][0]["attempts"], 3, "a retry loop that counts nothing is indistinguishable from a first negotiation");
        assert_eq!(failing["cameras"][0]["failingSeconds"], 40, "the failure is dated from when it started, not from the latest retry");
        state.on_decoding(MAIN_RECEIVER, true);
        let arrived = state.snapshot(50);
        assert_eq!(arrived["cameras"][0]["attempts"], 0, "a decoded frame is what ends the retry story");
        assert_eq!(arrived["cameras"][0]["failingSeconds"], Value::Null);
    }

    #[test]
    fn switching_camera_resets_the_live_flags_but_keeps_the_last_known_frame_size() {
        let mut state = wire(Settings { multi_view: true, ..settings() }, &[MAIN_RECEIVER, PIP_RECEIVER]);
        state.on_streaming(MAIN_RECEIVER, true);
        state.on_decoding(MAIN_RECEIVER, true);
        let sized = state.on_video_size(MAIN_RECEIVER, 1920, 1080);
        assert!(sized.contains(&Out::VideoSizeChanged { width_pixels: 1920, height_pixels: 1080 }));
        assert_eq!(state.video_size, Some((1920, 1080)));
        assert!(state.streaming && state.decoding);
        let before = state.snapshot(0);
        assert_eq!(before["cameras"][0]["videoSize"], json!({ "widthPixels": 1920, "heightPixels": 1080 }), "a frame size belongs to the camera that reported it");
        assert_eq!(before["cameras"][1]["videoSize"], Value::Null, "and a camera that has never decoded has none, where the old global made it look like it had");
        let switched = state.on_settings(Settings { active_source: 1, multi_view: true, ..settings() });
        assert!(
            switched.contains(&Out::StreamingChanged(false)) && switched.contains(&Out::DecodingChanged(false)),
            "the new camera has not started, so the flags must drop instead of keeping the old camera's"
        );
        let after = state.snapshot(0);
        assert_eq!(after["videoSize"], Value::Null, "the camera now on screen has reported no size, and the aspect must not be drawn from the previous camera's");
        assert_eq!(after["lastKnownVideoSize"], json!({ "widthPixels": 1920, "heightPixels": 1080 }), "the sticky value survives, named as stale rather than passed off as current");
        let zero = state.on_video_size(PIP_RECEIVER, 0, 0);
        assert!(zero.is_empty(), "a zero size is no size, not a new one");
        assert_eq!(state.video_size, Some((1920, 1080)));
    }

    #[test]
    fn the_recording_indicator_follows_the_camera_on_screen_and_carries_a_clock() {
        let mut state = wire(Settings { multi_view: true, ..settings() }, &[MAIN_RECEIVER, PIP_RECEIVER]);
        let pip = state.on_recording(PIP_RECEIVER, true, 10);
        assert!(pip.contains(&Out::RecordingChanged { receiver: PIP_RECEIVER.to_string(), active: true }), "the receiver that reported has to be named, the telemetry capture is per receiver");
        assert!(!state.recording, "a picture in picture recording must not light the indicator for the camera the operator is watching");
        assert!(!pip.contains(&Out::ActiveRecordingChanged(true)));
        assert!(state.camera_recording(1) && !state.camera_recording(0));
        let main = state.on_recording(MAIN_RECEIVER, true, 20);
        assert!(main.contains(&Out::ActiveRecordingChanged(true)), "the camera on screen is what the top level flag follows, same as streaming and decoding");
        assert!(state.recording);
        let view = state.snapshot(50);
        assert_eq!(view["cameras"][0]["recordingSeconds"], 30, "a bool with no clock cannot tell a growing recording from a latched flag");
        assert_eq!(view["cameras"][1]["recordingSeconds"], 40);
        let pip_stopped = state.on_recording(PIP_RECEIVER, false, 60);
        assert!(!pip_stopped.contains(&Out::ActiveRecordingChanged(false)), "and the picture in picture stopping must not put the indicator out while the camera on screen is still recording");
        assert!(state.recording);
        assert_eq!(state.snapshot(60)["cameras"][1]["recordingSeconds"], Value::Null);
    }

    #[test]
    fn recording_stops_the_telemetry_capture_that_started_with_it() {
        let mut state = wired();
        let started = state.on_recording(MAIN_RECEIVER, true, 0);
        assert!(state.recording && state.camera_recording(0));
        assert!(!started.contains(&Out::StopTelemetryCapture));
        let stopped = state.on_recording(MAIN_RECEIVER, false, 5);
        assert!(!state.recording && !state.camera_recording(0));
        assert!(stopped.contains(&Out::StopTelemetryCapture), "nothing else ends the subtitle capture, so the recording flag has to");
        assert!(state.on_recording(THERMAL_RECEIVER, true, 6).is_empty(), "the thermal receiver owns no camera state");
    }

    #[test]
    fn recording_is_refused_with_a_reason_until_a_receiver_is_actually_started() {
        let mut state = wired();
        assert_eq!(state.record_refusal(), Some(REFUSED_NO_STARTED_RECEIVER), "a head with only hasVideo to go on enables record mid-reconnect and the operator finds out after the flight");
        assert!(!state.can_record() && state.start_recording().is_empty());
        assert_eq!(state.snapshot(0)["recordRefusal"], REFUSED_NO_STARTED_RECEIVER);
        state.on_start_complete(MAIN_RECEIVER, Outcome::Ok, 0);
        assert!(state.can_record());
        assert_eq!(state.start_recording(), vec![Out::StartRecording { receivers: vec![MAIN_RECEIVER.to_string()] }], "only the started receivers can record, the cpp skips the rest silently");
        assert_eq!(state.snapshot(0)["canRecord"], true);
        let no_path = VideoState { settings: Settings { save_path_set: false, ..state.settings.clone() }, ..VideoState::default() };
        assert_eq!(no_path.record_refusal(), Some(REFUSED_NO_SAVE_PATH));
        let bad_format = VideoState { settings: Settings { recording_format_valid: false, ..state.settings.clone() }, ..VideoState::default() };
        assert_eq!(bad_format.record_refusal(), Some(REFUSED_BAD_FORMAT), "the two host side refusals are tokens the head can render, not silent returns");
        state.on_recording(MAIN_RECEIVER, true, 0);
        assert_eq!(state.stop_recording(), vec![Out::StopRecording { receivers: vec![MAIN_RECEIVER.to_string()] }]);
    }

    #[test]
    fn changing_the_low_latency_setting_restarts_every_receiver() {
        let mut state = wired();
        [MAIN_RECEIVER, THERMAL_RECEIVER, PIP_RECEIVER].iter().for_each(|name| {
            state.on_start_complete(name, Outcome::Ok, 0);
        });
        let toggled = state.on_settings(Settings { low_latency: true, ..settings() });
        [MAIN_RECEIVER, THERMAL_RECEIVER, PIP_RECEIVER].iter().for_each(|name| {
            assert!(
                toggled.contains(&Out::StopReceiver { receiver: name.to_string() }),
                "a low latency change is its own restart trigger in the cpp and it is the one thing that restarts the thermal receiver too"
            );
        });
        assert_eq!(state.snapshot(0)["lowLatency"], true);
        let again = state.on_settings(Settings { low_latency: true, ..settings() });
        assert!(!again.iter().any(|out| matches!(out, Out::StopReceiver { .. })), "and a settings push that changes nothing restarts nothing");
        let mut fresh = wire(Settings { low_latency: true, ..settings() }, &[MAIN_RECEIVER]);
        fresh.on_stop_complete(MAIN_RECEIVER, Outcome::Failed);
        assert!(
            fresh.start_receiver(MAIN_RECEIVER).contains(&Out::StartReceiver { receiver: MAIN_RECEIVER.to_string(), timeout_s: 12, low_latency: true }),
            "the setting has to reach the host on the start, or the pipeline keeps the old buffering for the rest of the flight"
        );
    }

    #[test]
    fn full_screen_is_refused_without_video_and_cleared_by_everything_that_takes_video_away() {
        let mut state = wired();
        assert_eq!(state.set_full_screen(true), vec![Out::FullScreenChanged(true)]);
        assert!(state.full_screen, "fullscreen needs video to show, not a connected vehicle");
        assert!(state.on_communication_lost(true).contains(&Out::FullScreenChanged(false)));
        assert!(!state.full_screen);
        state.set_full_screen(true);
        assert!(state.on_vehicle(false).contains(&Out::FullScreenChanged(false)), "losing the vehicle clears fullscreen");
        state.set_full_screen(true);
        let gone = state.on_settings(Settings { stream_enabled: false, ..settings() });
        assert!(gone.contains(&Out::FullScreenChanged(false)), "turning the stream off must clear fullscreen too, or the flag latches with nothing to show");
        assert!(state.set_full_screen(true).is_empty(), "fullscreen cannot be entered with no video");
    }

    #[test]
    fn the_vehicle_camera_is_told_to_start_streaming_when_it_arrives_and_to_stop_when_it_goes() {
        let mut state = wired();
        let arrived = state.on_vehicle(true);
        assert!(arrived.contains(&Out::StartCameraStream), "a mavlink camera that honours VIDEO_START_STREAMING is never told to begin otherwise, and the receiver sits on a stream that never started");
        assert!(!arrived.contains(&Out::StopCameraStream), "there was no previous vehicle to stop");
        let swapped = state.on_vehicle(true);
        assert_eq!(
            swapped.iter().filter(|out| matches!(out, Out::StopCameraStream | Out::StartCameraStream)).collect::<Vec<_>>(),
            vec![&Out::StopCameraStream, &Out::StartCameraStream],
            "swapping vehicles stops the departing camera before starting the arriving one, in that order"
        );
        let lost = state.on_vehicle(false);
        assert!(lost.contains(&Out::StopCameraStream));
        assert!(!lost.contains(&Out::StartCameraStream));
        assert!(!state.on_vehicle(false).contains(&Out::StopCameraStream), "and with no vehicle there is nothing left to stop");
    }

    #[test]
    fn a_camera_the_drone_announces_is_a_camera_like_any_other_and_says_where_it_came_from() {
        let drone = SourceSlot { drone: true, ..cam(SOURCE_UDP_H265, "0.0.0.0:5600", "SIYI A8") };
        let state = wire(Settings { cameras: vec![drone.clone()], ..settings() }, &[MAIN_RECEIVER]);
        assert!(state.has_video() && state.is_stream_source() && state.from_drone(), "with nothing configured the drone's own camera is on screen");
        assert_eq!(state.desired_uri(MAIN_RECEIVER), "udp265://0.0.0.0:5600");
        let view = state.snapshot(0);
        assert_eq!((view["fromDrone"].clone(), view["cameras"][0]["fromDrone"].clone()), (json!(true), json!(true)));
        let mine_first = Settings { cameras: settings().cameras.into_iter().chain(std::iter::once(drone)).collect(), ..settings() };
        assert_eq!(mine_first.current_index(), 0, "a drone camera joins the list and never takes the screen from the camera the operator picked");
        assert_eq!(mine_first.switchable(), vec![0, 1, 3], "and it can be switched to like any other");
    }

    #[test]
    fn the_vehicle_stream_types_map_to_the_source_tokens_and_keep_a_full_uri_whole() {
        assert_eq!(auto_stream_source(STREAM_TYPE_RTSP, 0, "rtsp://1.2.3.4/live"), (SOURCE_RTSP, "rtsp://1.2.3.4/live".to_string()));
        assert_eq!(
            auto_stream_source(STREAM_TYPE_TCP_MPEG, 0, "1.2.3.4:5600"),
            (SOURCE_TCP, "tcp://1.2.3.4:5600".to_string()),
            "every branch has to hand back a complete uri, a bare host and port is unparseable and latches the camera at invalidStreamUrl with no retry"
        );
        assert_eq!(auto_stream_source(STREAM_TYPE_TCP_MPEG, 0, "tcp://1.2.3.4:5600"), (SOURCE_TCP, "tcp://1.2.3.4:5600".to_string()), "and a uri that already carries tcp:// is not given it twice");
        assert_eq!(auto_stream_source(STREAM_TYPE_RTP_UDP, 1, "5600"), (SOURCE_UDP_H264, "udp://0.0.0.0:5600".to_string()));
        assert_eq!(
            auto_stream_source(STREAM_TYPE_RTP_UDP, ENCODING_H265, "udp265://1.2.3.4:5600"),
            (SOURCE_UDP_H265, "udp265://1.2.3.4:5600".to_string()),
            "a uri that already carries its scheme is left alone"
        );
        assert_eq!(auto_stream_source(STREAM_TYPE_MPEG_TS, 0, "5600"), (SOURCE_MPEGTS, "mpegts://0.0.0.0:5600".to_string()));
        assert_eq!(auto_stream_source(9, 0, "whatever"), (SOURCE_NO_VIDEO, String::new()), "an unknown stream type is not guessed at, and it carries no url either");
    }

    #[test]
    fn a_camera_that_has_never_shown_a_frame_reads_differently_from_one_with_no_receiver() {
        let mut state = wired();
        assert_eq!(state.frame_age(1, 100), Age::NotBound, "camera one is configured and off screen, so it has no frame clock rather than no camera");
        assert_eq!(state.frame_age(0, 100), Age::NoFrameYet, "a bound receiver with no frame yet is not a frame zero seconds old");
        let multi = wire(Settings { multi_view: true, ..settings() }, &[MAIN_RECEIVER]);
        assert_eq!(multi.frame_age(1, 100), Age::NoCamera, "on screen with no receiver registered is the answer noCamera is for");
        state.on_frame(MAIN_RECEIVER, 40);
        assert_eq!(state.frame_age(0, 100), Age::Seconds(60));
        assert_eq!(state.frame_age(0, 20).seconds(), Some(0), "a clock that has gone backwards reads as fresh rather than wrapping");
    }

    #[test]
    fn the_snapshot_carries_tokens_and_numbers_for_the_head_to_format() {
        let mut state = wired();
        state.on_streaming(MAIN_RECEIVER, true);
        state.on_video_size(MAIN_RECEIVER, 1280, 720);
        let view = state.snapshot(100);
        assert_eq!(view["cameras"][0]["status"], "waitingForFrames", "the head formats the sentence, the core names the state");
        assert_eq!(view["cameras"][0]["name"], Value::Null, "an unnamed camera is absent, not the string the head would show");
        assert_eq!(view["cameras"][1]["name"], "Thermal side");
        assert_eq!(view["cameras"][2]["configured"], false);
        assert_eq!(view["videoSize"], json!({ "widthPixels": 1280, "heightPixels": 720 }));
        assert_eq!(view["cameras"][1]["frameAge"], "notBound");
        assert_eq!(view["cameras"][1]["frameAgeSeconds"], Value::Null);
        assert_eq!(view["switchable"], json!([0, 1]));
        assert_eq!(view["nextSource"], 1);
    }

    struct NoBackend;

    impl Backend for NoBackend {
        fn get(&self, _p: &str) -> String {
            String::new()
        }
        fn get_fields(&self, _p: &str, _f: &str) -> String {
            String::new()
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            String::new()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_source_view_answers_from_its_arguments_alone() {
        let view = video_source_view(&NoBackend, &[SOURCE_RTSP.to_string(), "rtsp://1.2.3.4/live".to_string(), "12".to_string()]);
        assert_eq!(view["configured"], true);
        assert_eq!(view["usable"], true);
        assert_eq!(view["uri"], "rtsp://1.2.3.4/live");
        assert_eq!(view["startTimeoutSeconds"], 12);
        let bare = video_source_view(&NoBackend, &[SOURCE_RTSP.to_string()]);
        assert_eq!(bare["configured"], false);
        assert_eq!(bare["uri"], Value::Null, "an rtsp source with no url has no uri to start");
        assert_eq!(bare["startTimeoutSeconds"], Value::Null, "the negotiation budget is a setting, so without it the view says nothing rather than guessing three seconds");
        let udp = video_source_view(&NoBackend, &[SOURCE_UDP_H264.to_string(), "0.0.0.0:5600".to_string()]);
        assert_eq!(udp["startTimeoutSeconds"], 3, "the plain start budget is the cpp's three seconds, pinned as a number rather than through the constant the code reads");
        let solo = video_source_view(&NoBackend, &[SOURCE_3DR_SOLO.to_string()]);
        assert_eq!(solo["configured"], true);
        assert_eq!((&solo["usable"], &solo["uri"]), (&json!(true), &json!("udp://0.0.0.0:5600")), "solo needs no url and starts on the port it always used");
        assert!(video_source_view(&NoBackend, &[])["reason"].is_string(), "a view that refuses its arguments says what it wanted");
    }
}
