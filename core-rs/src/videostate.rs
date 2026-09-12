use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[];

pub const MAIN_RECEIVER: &str = "videoContent";
pub const THERMAL_RECEIVER: &str = "thermalVideo";
pub const TILE_RECEIVER_PREFIX: &str = "extraVideo";
pub const MAX_VIDEO_TILES: usize = 8;
pub const START_TIMEOUT_S: u32 = 3;
pub const RESTART_DELAY_MS: u64 = 1000;

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

pub const SETTING_VIDEO_SOURCE: &str = "videoSource";
pub const SETTING_RTSP_URL: &str = "rtspUrl";

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
        SOURCE_DISABLED => "disabled",
        SOURCE_NO_VIDEO => "noVideo",
        _ => "unknown",
    }
}

pub fn requires_restart(source: &str) -> bool {
    source == SOURCE_3DR_SOLO
}

pub fn source_usable(source: &str, url: &str) -> bool {
    match source {
        SOURCE_NO_VIDEO | SOURCE_DISABLED => false,
        _ if needs_url(source) => !url.is_empty(),
        SOURCE_HERELINK_AIR_UNIT | SOURCE_HERELINK_HOTSPOT => true,
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
        SOURCE_WEBRTC => url.trim().to_string(),
        SOURCE_3DR_SOLO => "udp://0.0.0.0:5600".to_string(),
        SOURCE_PARROT_DISCOVERY => "udp://0.0.0.0:8888".to_string(),
        SOURCE_YUNEEC_MANTIS_G => "rtsp://192.168.42.1:554/live".to_string(),
        SOURCE_HERELINK_AIR_UNIT => "rtsp://192.168.0.10:8554/H264Video".to_string(),
        SOURCE_HERELINK_HOTSPOT => "rtsp://192.168.43.1:8554/fpv_stream".to_string(),
        _ => String::new(),
    }
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
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PrimaryUrls {
    pub udp: String,
    pub rtsp: String,
    pub tcp: String,
    pub whep: String,
}

pub fn primary_url<'a>(source: &str, urls: &'a PrimaryUrls) -> &'a str {
    match source {
        SOURCE_UDP_H264 | SOURCE_UDP_H265 | SOURCE_MPEGTS => &urls.udp,
        SOURCE_RTSP => &urls.rtsp,
        SOURCE_TCP => &urls.tcp,
        SOURCE_WEBRTC => &urls.whep,
        _ => "",
    }
}

pub fn receiver_slot(receiver: &str) -> Option<usize> {
    match receiver {
        MAIN_RECEIVER => Some(0),
        _ => receiver.strip_prefix(TILE_RECEIVER_PREFIX).and_then(|slot| slot.parse::<usize>().ok()).map(|slot| slot + 1),
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    pub primary_source: String,
    pub primary_name: String,
    pub primary_urls: PrimaryUrls,
    pub extras: Vec<SourceSlot>,
    pub active_source: i64,
    pub multi_view: bool,
    pub stream_enabled: bool,
    pub low_latency: bool,
    pub save_path_set: bool,
    pub recording_format_valid: bool,
    pub rtsp_timeout_s: u32,
}

impl Settings {
    pub fn count(&self) -> usize {
        1 + self.extras.len()
    }

    fn extra(&self, index: usize) -> Option<&SourceSlot> {
        index.checked_sub(1).and_then(|slot| self.extras.get(slot))
    }

    pub fn source_at(&self, index: usize) -> &str {
        self.extra(index).map(|slot| slot.source.as_str()).unwrap_or(&self.primary_source)
    }

    pub fn url_at(&self, index: usize) -> &str {
        match self.extra(index) {
            Some(slot) => &slot.url,
            None if index == 0 => primary_url(&self.primary_source, &self.primary_urls),
            None => "",
        }
    }

    pub fn name_at(&self, index: usize) -> Option<&str> {
        let name = match self.extra(index) {
            Some(slot) => slot.name.as_str(),
            None if index == 0 => self.primary_name.as_str(),
            None => "",
        };
        Some(name).filter(|name| !name.is_empty())
    }

    pub fn enabled(&self, index: usize) -> bool {
        self.source_at(index) != SOURCE_DISABLED
    }

    pub fn configured(&self, index: usize) -> bool {
        !needs_url(self.source_at(index)) || !self.url_at(index).is_empty()
    }

    pub fn current_index(&self) -> usize {
        let asked = self.active_source;
        match asked > 0 && asked < self.count() as i64 {
            true => {
                let index = asked as usize;
                match self.configured(index) {
                    true => index,
                    false => 0,
                }
            }
            false => 0,
        }
    }

    pub fn active_refusal(&self) -> Option<&'static str> {
        let asked = self.active_source;
        match asked {
            0 => None,
            asked if asked < 0 || asked >= self.count() as i64 => Some(REFUSED_SOURCE_OUT_OF_RANGE),
            asked if !self.configured(asked as usize) => Some(REFUSED_SOURCE_UNCONFIGURED),
            _ => None,
        }
    }

    pub fn shown(&self, index: usize) -> bool {
        index < self.count() && (index == self.current_index() || self.multi_view)
    }

    pub fn clamp_active(&self, asked: i64) -> usize {
        asked.clamp(0, self.count() as i64 - 1) as usize
    }

    pub fn switchable(&self) -> Vec<usize> {
        std::iter::once(0)
            .chain((1..self.count()).filter(|index| is_stream_source(self.source_at(*index)) && self.configured(*index)))
            .collect()
    }

    pub fn tiles(&self) -> Vec<usize> {
        let current = self.current_index();
        self.switchable().into_iter().filter(|index| *index != current).collect()
    }

    pub fn tile_camera_number(&self, slot: usize) -> Option<usize> {
        self.multi_view.then(|| self.tiles().get(slot).copied().filter(|_| slot < MAX_VIDEO_TILES)).flatten().map(|index| index + 1)
    }

    pub fn next_switchable(&self) -> Option<usize> {
        let indices = self.switchable();
        let at = indices.iter().position(|index| *index == self.current_index()).unwrap_or(0);
        (indices.len() > 1).then(|| indices[(at + 1) % indices.len()])
    }

    pub fn camera_index_for_receiver(&self, receiver: &str) -> Option<usize> {
        receiver_slot(receiver).filter(|index| self.shown(*index))
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
    pub auto_stream_uri: Option<String>,
    pub streaming: bool,
    pub decoding: bool,
    pub recording: bool,
    pub video_size: Option<(u32, u32)>,
    pub full_screen: bool,
    pub vehicle_present: bool,
    receivers: BTreeMap<String, Receiver>,
}

impl VideoState {
    pub fn auto_stream_configured(&self) -> bool {
        self.auto_stream_uri.as_ref().is_some_and(|uri| !uri.is_empty())
    }

    pub fn url_at(&self, index: usize) -> &str {
        match (index, self.auto_stream_uri.as_deref()) {
            (0, Some(uri)) if !uri.is_empty() => uri,
            _ => self.settings.url_at(index),
        }
    }

    pub fn configured(&self, index: usize) -> bool {
        !needs_url(self.settings.source_at(index)) || !self.url_at(index).is_empty()
    }

    pub fn usable(&self, index: usize) -> bool {
        source_usable(self.settings.source_at(index), self.url_at(index))
    }

    pub fn stream_configured(&self) -> bool {
        self.auto_stream_configured() || self.usable(self.settings.current_index())
    }

    pub fn has_video(&self) -> bool {
        self.settings.stream_enabled && self.stream_configured()
    }

    pub fn is_stream_source(&self) -> bool {
        is_stream_source(self.settings.source_at(self.settings.current_index())) || self.auto_stream_configured()
    }

    pub fn has_multiple_sources(&self) -> bool {
        self.settings.switchable().len() > 1
    }

    fn desired_uri(&self, receiver: &str) -> String {
        let Some(index) = self.settings.camera_index_for_receiver(receiver) else { return String::new() };
        match (index, self.auto_stream_uri.as_ref()) {
            (0, Some(uri)) if receiver == MAIN_RECEIVER => uri.clone(),
            _ => source_uri(self.settings.source_at(index), self.settings.url_at(index)),
        }
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
        self.settings = settings;
        let names: Vec<String> = self.receivers.keys().cloned().collect();
        let moved: Vec<String> = names
            .iter()
            .filter(|name| Self::camera_receiver(name))
            .filter(|name| {
                let uri = self.desired_uri(name);
                self.receivers.get(*name).is_some_and(|receiver| receiver.uri != uri)
            })
            .cloned()
            .collect();
        moved.iter().for_each(|name| {
            let uri = self.desired_uri(name);
            self.receivers.entry(name.clone()).and_modify(|receiver| receiver.uri = uri);
        });
        let changed = match latency_changed {
            true => names.clone(),
            false => moved,
        };
        let restarts: Vec<Out> = match (changed.is_empty(), self.has_video()) {
            (true, _) => Vec::new(),
            (false, true) => changed.iter().flat_map(|name| self.restart_receiver(name)).collect(),
            (false, false) => names.iter().map(|receiver| Out::StopReceiver { receiver: receiver.clone() }).collect(),
        };
        let cleared = match self.has_video() {
            true => Vec::new(),
            false => self.set_full_screen(false),
        };
        std::iter::once(Out::CamerasChanged).chain(self.refresh()).chain(restarts).chain(cleared).collect()
    }

    pub fn on_auto_stream(&mut self, stream_type: u8, encoding: u8, uri: &str) -> Vec<Out> {
        let (source, url) = match uri.is_empty() {
            true => (SOURCE_NO_VIDEO, String::new()),
            false => auto_stream_source(stream_type, encoding, uri),
        };
        let (settings, persisted): (Settings, Vec<Out>) = match url.is_empty() {
            true => {
                self.auto_stream_uri = None;
                (self.settings.clone(), Vec::new())
            }
            false => {
                self.auto_stream_uri = Some(url.clone());
                let rtsp = (source == SOURCE_RTSP).then(|| Out::SetSetting { name: SETTING_RTSP_URL, value: url.clone() });
                let writes = std::iter::once(Out::SetSetting { name: SETTING_VIDEO_SOURCE, value: source.to_string() }).chain(rtsp).collect();
                (Settings { primary_source: source.to_string(), ..self.settings.clone() }, writes)
            }
        };
        self.on_settings(settings).into_iter().chain(persisted).collect()
    }

    pub fn on_vehicle(&mut self, present: bool) -> Vec<Out> {
        let was = std::mem::replace(&mut self.vehicle_present, present);
        let stream: Vec<Out> = was.then_some(Out::StopCameraStream).into_iter().chain(present.then_some(Out::StartCameraStream)).collect();
        match present {
            true => stream,
            false => {
                self.auto_stream_uri = None;
                let settings = self.settings.clone();
                stream.into_iter().chain(self.on_settings(settings)).chain(self.set_full_screen(false)).collect()
            }
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
        let index = receiver_slot(receiver).filter(|index| *index < self.settings.count()).unwrap_or_else(|| self.settings.current_index());
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
                self.set_status(receiver, Status::ConnectionFailed, true).into_iter().chain(self.restart_receiver(receiver)).collect()
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
            _ => self
                .set_status(receiver, Status::Reconnecting, true)
                .into_iter()
                .chain(std::iter::once(Out::RestartAfter { receiver: receiver.to_string(), delay_ms: RESTART_DELAY_MS }))
                .collect(),
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
        self.receivers.entry(receiver.to_string()).and_modify(|state| state.last_frame_s = Some(at_s));
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
            "autoStreamConfigured": self.auto_stream_configured(),
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
            "tiles": self.settings.tiles(),
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

    fn settings() -> Settings {
        Settings {
            primary_source: SOURCE_RTSP.to_string(),
            primary_name: String::new(),
            primary_urls: PrimaryUrls { rtsp: "rtsp://10.0.0.1:8554/live".to_string(), ..PrimaryUrls::default() },
            extras: vec![
                SourceSlot { source: SOURCE_UDP_H265.to_string(), url: "0.0.0.0:5601".to_string(), name: "Thermal side".to_string() },
                SourceSlot { source: SOURCE_RTSP.to_string(), url: String::new(), name: String::new() },
            ],
            active_source: 0,
            multi_view: false,
            stream_enabled: true,
            low_latency: false,
            save_path_set: true,
            recording_format_valid: true,
            rtsp_timeout_s: 12,
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
        wire(settings(), &[MAIN_RECEIVER, THERMAL_RECEIVER, "extraVideo0", "extraVideo1"])
    }

    #[test]
    fn a_source_is_usable_only_once_it_has_the_address_its_kind_needs() {
        assert!(needs_url(SOURCE_WEBRTC) && needs_url(SOURCE_RTSP) && needs_url(SOURCE_MPEGTS), "the six url kinds are the ones that stay unconfigured until an address is typed");
        assert!(!needs_url(SOURCE_HERELINK_AIR_UNIT), "a fixed-address source is configured the moment it is picked");
        assert!(is_stream_source(SOURCE_HERELINK_HOTSPOT) && !is_stream_source(SOURCE_DISABLED));
        let mut state = VideoState { settings: Settings { primary_source: SOURCE_RTSP.to_string(), stream_enabled: true, ..Settings::default() }, ..VideoState::default() };
        assert!(!state.stream_configured(), "rtsp with no url is not configured");
        assert!(!state.has_video());
        state.settings.primary_urls.rtsp = "rtsp://1.2.3.4/live".to_string();
        assert!(state.has_video(), "stream enabled plus a configured source is the whole of hasVideo");
        state.settings.stream_enabled = false;
        assert!(!state.has_video(), "the enable switch alone can take video away");
        state.settings.primary_source = SOURCE_HERELINK_AIR_UNIT.to_string();
        state.settings.stream_enabled = true;
        assert!(state.stream_configured(), "herelink needs no url");
        state.settings.primary_source = SOURCE_NO_VIDEO.to_string();
        assert!(!state.stream_configured());
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
        let blank = wire(Settings { primary_source: SOURCE_NO_VIDEO.to_string(), ..settings() }, &[MAIN_RECEIVER]);
        assert_eq!(blank.camera_status(0), Status::NoVideoSource, "noVideoSource is reserved for a slot whose source really is none");
        let multi = wire(Settings { multi_view: true, ..settings() }, &[MAIN_RECEIVER]);
        assert_eq!(multi.camera_status(1), Status::NoReceiver, "a shown camera the host never registered a receiver for is a fourth answer again");
    }

    #[test]
    fn a_source_that_can_never_start_says_so_instead_of_looking_ready() {
        let solo = wire(Settings { primary_source: SOURCE_3DR_SOLO.to_string(), ..settings() }, &[MAIN_RECEIVER]);
        assert!(!solo.has_video(), "the three fixed-address sources outside herelink are not accepted by streamConfigured, so they never produce a frame");
        assert!(solo.configured(0), "they need no url, so the configured question answers yes");
        assert!(!solo.usable(0), "usable is the per-slot mirror of streamConfigured, so one row can no longer contradict hasVideo");
        assert_eq!(solo.camera_status(0), Status::UnsupportedSource, "a source that is offered but can never start has to be named");
        let view = solo.snapshot(0);
        assert_eq!(view["cameras"][0]["usable"], false);
        assert_eq!(view["cameras"][0]["status"], "unsupportedSource");
        assert_eq!(view["hasVideo"], false);
        let blank = wire(Settings { primary_source: SOURCE_NO_VIDEO.to_string(), ..settings() }, &[MAIN_RECEIVER]);
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
    fn the_primary_camera_reads_the_url_belonging_to_its_own_source_kind() {
        let urls = PrimaryUrls { udp: "0.0.0.0:5600".to_string(), rtsp: "rtsp://a/live".to_string(), tcp: "1.2.3.4:5600".to_string(), whep: "http://a/whep".to_string() };
        assert_eq!(primary_url(SOURCE_UDP_H265, &urls), "0.0.0.0:5600", "both udp kinds and mpeg-ts share the one udp url field");
        assert_eq!(primary_url(SOURCE_MPEGTS, &urls), "0.0.0.0:5600");
        assert_eq!(primary_url(SOURCE_RTSP, &urls), "rtsp://a/live");
        assert_eq!(primary_url(SOURCE_WEBRTC, &urls), "http://a/whep");
        assert_eq!(primary_url(SOURCE_HERELINK_HOTSPOT, &urls), "", "a fixed-address source reads no url field");
    }

    #[test]
    fn the_timeouts_are_the_cpp_numbers_and_only_negotiating_sources_get_the_long_one() {
        assert_eq!(start_timeout_s(SOURCE_UDP_H264, 12), 3, "a plain udp stream gets the three second start budget");
        assert_eq!(start_timeout_s(SOURCE_RTSP, 12), 12, "rtsp may fall back to tcp after five seconds, so it gets the configured budget");
        assert_eq!(start_timeout_s(SOURCE_WEBRTC, 12), 12, "whep signalling plus ice needs the same headroom as rtsp");
        assert_eq!(RESTART_DELAY_MS, 1000, "a stopped receiver is retried after exactly one second");
    }

    #[test]
    fn only_configured_stream_sources_can_be_switched_to_and_the_switch_wraps() {
        let state = wired();
        assert_eq!(state.settings.switchable(), vec![0, 1], "slot two has no url so it is not offered");
        assert_eq!(state.settings.current_index(), 0);
        assert_eq!(state.settings.next_switchable(), Some(1));
        let on_second = VideoState { settings: Settings { active_source: 1, ..settings() }, ..VideoState::default() };
        assert_eq!(on_second.settings.current_index(), 1);
        assert_eq!(on_second.settings.next_switchable(), Some(0), "the switch wraps back round to the primary camera");
        let unconfigured = VideoState { settings: Settings { active_source: 2, ..settings() }, ..VideoState::default() };
        assert_eq!(unconfigured.settings.current_index(), 0, "an active camera that is not configured falls back to the primary");
        let out_of_range = VideoState { settings: Settings { active_source: 9, ..settings() }, ..VideoState::default() };
        assert_eq!(out_of_range.settings.current_index(), 0);
        assert_eq!(out_of_range.settings.clamp_active(9), 2, "setting the active camera clamps into the slot range instead of refusing");
        assert_eq!(out_of_range.settings.clamp_active(-4), 0);
        let single = VideoState { settings: Settings { extras: Vec::new(), ..settings() }, ..VideoState::default() };
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
    fn tiles_are_the_switchable_cameras_that_are_not_on_screen_and_only_in_multi_view() {
        let state = wired();
        assert_eq!(state.settings.tiles(), vec![1]);
        assert_eq!(state.settings.tile_camera_number(0), None, "with multi view off no tile carries a camera");
        let multi = VideoState { settings: Settings { multi_view: true, ..settings() }, ..VideoState::default() };
        assert_eq!(multi.settings.tile_camera_number(0), Some(2), "a tile reports a one-based camera number");
        assert_eq!(multi.settings.tile_camera_number(1), None, "an empty tile has no camera, which is not camera zero");
        let crowded = Settings {
            multi_view: true,
            extras: (0..10).map(|slot| SourceSlot { source: SOURCE_UDP_H264.to_string(), url: format!("0.0.0.0:56{slot:02}"), name: String::new() }).collect(),
            ..settings()
        };
        assert_eq!(crowded.tiles().len(), 10, "ten configured extras are all switchable, so the cap is the only thing that can stop the eleventh tile");
        assert_eq!(crowded.tile_camera_number(7), Some(9));
        assert_eq!(crowded.tile_camera_number(8), None, "the eight tile widgets are the whole of the multi view, so slot eight carries nothing however many cameras are configured");
    }

    #[test]
    fn a_receiver_belongs_to_one_pinned_camera_and_is_invisible_when_its_camera_is_not_shown() {
        let state = wired();
        assert_eq!(state.settings.camera_index_for_receiver(MAIN_RECEIVER), Some(0));
        assert_eq!(state.settings.camera_index_for_receiver("extraVideo0"), None, "in single view only the active camera's receiver is bound");
        assert_eq!(state.settings.camera_index_for_receiver(THERMAL_RECEIVER), None, "the thermal receiver is no camera slot");
        assert_eq!(state.settings.camera_index_for_receiver("extraVideo7"), None, "a slot past the configured cameras is bound to nothing");
        let multi = Settings { multi_view: true, ..settings() };
        assert_eq!(multi.camera_index_for_receiver("extraVideo0"), Some(1), "multi view binds every camera's receiver at once");
        assert_eq!(receiver_slot("extraVideo3"), Some(4), "the pinning itself is independent of what is on screen");
        assert_eq!(receiver_slot(THERMAL_RECEIVER), None);
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
        let mut state = wire(Settings { primary_source: SOURCE_RTSP.to_string(), stream_enabled: true, ..Settings::default() }, &[MAIN_RECEIVER]);
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
        let gone = state.on_settings(Settings { stream_enabled: false, primary_urls: PrimaryUrls { rtsp: "rtsp://10.0.0.9:8554/live".to_string(), ..PrimaryUrls::default() }, ..settings() });
        assert!(gone.contains(&Out::StopReceiver { receiver: MAIN_RECEIVER.to_string() }), "turning the stream off while a url also moved stops every receiver");
        let stopped = state.on_stop_complete(MAIN_RECEIVER, Outcome::Failed);
        assert!(stopped.contains(&Out::RestartAfter { receiver: MAIN_RECEIVER.to_string(), delay_ms: RESTART_DELAY_MS }));
        assert!(state.start_receiver(MAIN_RECEIVER).is_empty(), "and the retry that stop scheduled must find no video to show, or the stop never sticks");
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
        state.on_start_complete(MAIN_RECEIVER, Outcome::Failed, 10);
        state.on_start_complete(MAIN_RECEIVER, Outcome::Failed, 20);
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
        let mut state = wire(Settings { multi_view: true, ..settings() }, &[MAIN_RECEIVER, "extraVideo0"]);
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
        let zero = state.on_video_size("extraVideo0", 0, 0);
        assert!(zero.is_empty(), "a zero size is no size, not a new one");
        assert_eq!(state.video_size, Some((1920, 1080)));
    }

    #[test]
    fn the_recording_indicator_follows_the_camera_on_screen_and_carries_a_clock() {
        let mut state = wire(Settings { multi_view: true, ..settings() }, &[MAIN_RECEIVER, "extraVideo0"]);
        let tile = state.on_recording("extraVideo0", true, 10);
        assert!(tile.contains(&Out::RecordingChanged { receiver: "extraVideo0".to_string(), active: true }), "the receiver that reported has to be named, the telemetry capture is per receiver");
        assert!(!state.recording, "a tile recording must not light the indicator for the camera the operator is watching");
        assert!(!tile.contains(&Out::ActiveRecordingChanged(true)));
        assert!(state.camera_recording(1) && !state.camera_recording(0));
        let main = state.on_recording(MAIN_RECEIVER, true, 20);
        assert!(main.contains(&Out::ActiveRecordingChanged(true)), "the camera on screen is what the top level flag follows, same as streaming and decoding");
        assert!(state.recording);
        let view = state.snapshot(50);
        assert_eq!(view["cameras"][0]["recordingSeconds"], 30, "a bool with no clock cannot tell a growing recording from a latched flag");
        assert_eq!(view["cameras"][1]["recordingSeconds"], 40);
        let tile_stopped = state.on_recording("extraVideo0", false, 60);
        assert!(!tile_stopped.contains(&Out::ActiveRecordingChanged(false)), "and a tile stopping must not put the indicator out while the camera on screen is still recording");
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
        [MAIN_RECEIVER, THERMAL_RECEIVER, "extraVideo0"].iter().for_each(|name| {
            state.on_start_complete(name, Outcome::Ok, 0);
        });
        let toggled = state.on_settings(Settings { low_latency: true, ..settings() });
        [MAIN_RECEIVER, THERMAL_RECEIVER, "extraVideo0"].iter().for_each(|name| {
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
    fn a_vehicle_stream_configures_the_source_and_losing_the_vehicle_clears_it() {
        let mut state = wire(Settings { primary_source: SOURCE_DISABLED.to_string(), ..settings() }, &[MAIN_RECEIVER]);
        assert!(!state.has_video());
        state.on_auto_stream(STREAM_TYPE_RTP_UDP, ENCODING_H265, "5600");
        assert_eq!(state.settings.primary_source, SOURCE_UDP_H265);
        assert_eq!(state.auto_stream_uri.as_deref(), Some("udp265://0.0.0.0:5600"), "a bare port from the vehicle becomes a full listen uri");
        assert!(state.has_video() && state.is_stream_source(), "an auto configured stream is configured whatever the source setting said");
        state.on_vehicle(false);
        assert!(!state.auto_stream_configured(), "the auto stream flag has to clear when the vehicle that supplied it goes");
        assert!(!state.has_video());
    }

    #[test]
    fn an_auto_configured_stream_is_persisted_and_never_reads_as_unconfigured() {
        let mut state = wire(Settings { primary_source: SOURCE_DISABLED.to_string(), ..settings() }, &[MAIN_RECEIVER]);
        let out = state.on_auto_stream(STREAM_TYPE_RTSP, 0, "rtsp://1.2.3.4/live");
        assert!(
            out.contains(&Out::SetSetting { name: SETTING_VIDEO_SOURCE, value: SOURCE_RTSP.to_string() }),
            "the cpp stuck because it wrote the setting, so the port has to ask the host to persist it or the next settings push reverts it"
        );
        assert!(out.contains(&Out::SetSetting { name: SETTING_RTSP_URL, value: "rtsp://1.2.3.4/live".to_string() }), "and the rtsp url write is the second thing the cpp did");
        assert!(state.configured(0), "while the vehicle stream plays, the camera showing it must not report itself unconfigured");
        assert_eq!(state.url_at(0), "rtsp://1.2.3.4/live");
        let view = state.snapshot(0);
        assert_eq!(view["cameras"][0]["configured"], true);
        assert_eq!(view["cameras"][0]["usable"], true);
        assert_eq!(view["cameras"][0]["status"], "connecting", "and it reads as a camera coming up, not as one nobody typed an address for");
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
    fn a_tcp_vehicle_stream_starts_on_a_uri_a_pipeline_can_parse() {
        let mut state = wire(Settings { primary_source: SOURCE_DISABLED.to_string(), ..settings() }, &[MAIN_RECEIVER]);
        let out = state.on_auto_stream(STREAM_TYPE_TCP_MPEG, 0, "1.2.3.4:5600");
        assert!(out.contains(&Out::StartReceiver { receiver: MAIN_RECEIVER.to_string(), timeout_s: 3, low_latency: false }));
        assert_eq!(state.auto_stream_uri.as_deref(), Some("tcp://1.2.3.4:5600"), "the whole of the vehicle's video is lost if the receiver is started on an address with no scheme");
    }

    #[test]
    fn an_unknown_stream_type_is_never_started_on_the_string_the_vehicle_sent() {
        let mut state = wire(Settings { primary_source: SOURCE_DISABLED.to_string(), ..settings() }, &[MAIN_RECEIVER]);
        let out = state.on_auto_stream(200, 0, "garbage-uri");
        assert!(!state.auto_stream_configured(), "an unrecognised stream type must not escalate a refusal to a start on unvalidated text from the air vehicle");
        assert!(!out.iter().any(|o| matches!(o, Out::StartReceiver { .. })));
        assert!(!state.has_video());
        assert_eq!(state.settings.primary_source, SOURCE_DISABLED, "and it leaves the source the operator chose alone");
        let view = state.snapshot(0);
        assert_eq!(view["streamSource"], false, "the snapshot cannot say noVideo and streamSource and hasVideo all at once");
        assert_eq!(view["hasVideo"], false);
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
        assert_eq!(solo["usable"], false, "solo needs no url and still cannot start, and only usable says so");
        assert!(video_source_view(&NoBackend, &[])["reason"].is_string(), "a view that refuses its arguments says what it wanted");
    }
}
