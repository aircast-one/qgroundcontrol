use std::collections::BTreeMap;

use serde_json::{Value, json};

pub use crate::mavcmd::{RESULT_ACCEPTED, RESULT_FAILED, RESULT_IN_PROGRESS};
pub const RESULT_TEMPORARILY_REJECTED: u8 = 1;
pub const RESULT_DENIED: u8 = 2;
pub const RESULT_UNSUPPORTED: u8 = 3;

pub const RESULTS: &[(u8, &str)] = &[
    (RESULT_ACCEPTED, "accepted"),
    (RESULT_TEMPORARILY_REJECTED, "temporarilyRejected"),
    (RESULT_DENIED, "denied"),
    (RESULT_UNSUPPORTED, "unsupported"),
    (RESULT_FAILED, "failed"),
    (RESULT_IN_PROGRESS, "inProgress"),
];

pub const COMP_ID_CAMERA: u8 = 100;
pub const COMP_ID_CAMERA6: u8 = 105;

pub const MSG_CAMERA_INFORMATION: u32 = 259;
pub const MSG_CAMERA_SETTINGS: u32 = 260;
pub const MSG_STORAGE_INFORMATION: u32 = 261;
pub const MSG_CAMERA_CAPTURE_STATUS: u32 = 262;
pub const MSG_VIDEO_STREAM_INFORMATION: u32 = 269;
pub const MSG_VIDEO_STREAM_STATUS: u32 = 270;

pub const CMD_REQUEST_MESSAGE: u16 = 512;
pub const CMD_REQUEST_CAMERA_INFORMATION: u16 = 521;
pub const CMD_REQUEST_CAMERA_SETTINGS: u16 = 522;
pub const CMD_REQUEST_STORAGE_INFORMATION: u16 = 525;
pub const CMD_STORAGE_FORMAT: u16 = 526;
pub const CMD_REQUEST_CAMERA_CAPTURE_STATUS: u16 = 527;
pub const CMD_RESET_CAMERA_SETTINGS: u16 = 529;
pub const CMD_SET_CAMERA_MODE: u16 = 530;
pub const CMD_SET_CAMERA_ZOOM: u16 = 531;
pub const CMD_SET_CAMERA_FOCUS: u16 = 532;
pub const CMD_IMAGE_START_CAPTURE: u16 = 2000;
pub const CMD_IMAGE_STOP_CAPTURE: u16 = 2001;
pub const CMD_VIDEO_START_CAPTURE: u16 = 2500;
pub const CMD_VIDEO_STOP_CAPTURE: u16 = 2501;
pub const CMD_VIDEO_START_STREAMING: u16 = 2502;
pub const CMD_VIDEO_STOP_STREAMING: u16 = 2503;
pub const CMD_REQUEST_VIDEO_STREAM_INFORMATION: u16 = 2504;
pub const CMD_REQUEST_VIDEO_STREAM_STATUS: u16 = 2505;

pub const LEVEL_TYPE_STEP: f64 = 0.0;
pub const LEVEL_TYPE_CONTINUOUS: f64 = 1.0;
pub const LEVEL_TYPE_RANGE: f64 = 2.0;

pub const CAP_CAPTURE_VIDEO: u32 = 1;
pub const CAP_CAPTURE_IMAGE: u32 = 2;
pub const CAP_HAS_MODES: u32 = 4;
pub const CAP_IMAGE_IN_VIDEO_MODE: u32 = 8;
pub const CAP_VIDEO_IN_IMAGE_MODE: u32 = 16;
pub const CAP_HAS_IMAGE_SURVEY_MODE: u32 = 32;
pub const CAP_HAS_BASIC_ZOOM: u32 = 64;
pub const CAP_HAS_BASIC_FOCUS: u32 = 128;
pub const CAP_HAS_VIDEO_STREAM: u32 = 256;
pub const CAP_HAS_TRACKING_POINT: u32 = 512;
pub const CAP_HAS_TRACKING_RECTANGLE: u32 = 1024;
pub const CAP_HAS_TRACKING_GEO_STATUS: u32 = 2048;

pub const CAPABILITIES: &[(u32, &str)] = &[
    (CAP_CAPTURE_VIDEO, "captureVideo"),
    (CAP_CAPTURE_IMAGE, "captureImage"),
    (CAP_HAS_MODES, "modes"),
    (CAP_IMAGE_IN_VIDEO_MODE, "imageInVideoMode"),
    (CAP_VIDEO_IN_IMAGE_MODE, "videoInImageMode"),
    (CAP_HAS_IMAGE_SURVEY_MODE, "imageSurveyMode"),
    (CAP_HAS_BASIC_ZOOM, "basicZoom"),
    (CAP_HAS_BASIC_FOCUS, "basicFocus"),
    (CAP_HAS_VIDEO_STREAM, "videoStream"),
    (CAP_HAS_TRACKING_POINT, "trackingPoint"),
    (CAP_HAS_TRACKING_RECTANGLE, "trackingRectangle"),
    (CAP_HAS_TRACKING_GEO_STATUS, "trackingGeoStatus"),
];

pub const MODE_PHOTO: u8 = 0;
pub const MODE_VIDEO: u8 = 1;
pub const MODE_SURVEY: u8 = 2;
pub const MODES: &[(u8, &str)] = &[(MODE_PHOTO, "photo"), (MODE_VIDEO, "video"), (MODE_SURVEY, "survey")];

pub const PHOTO_IDLE: u8 = 0;
pub const PHOTO_IN_PROGRESS: u8 = 1;
pub const PHOTO_INTERVAL_IDLE: u8 = 2;
pub const PHOTO_INTERVAL_IN_PROGRESS: u8 = 3;
pub const PHOTO_STATUS_LAST: u8 = 4;
pub const STATUS_UNDEFINED: u8 = 255;
pub const PHOTO_STATUS: &[(u8, &str)] = &[
    (PHOTO_IDLE, "idle"),
    (PHOTO_IN_PROGRESS, "inProgress"),
    (PHOTO_INTERVAL_IDLE, "intervalIdle"),
    (PHOTO_INTERVAL_IN_PROGRESS, "intervalInProgress"),
    (STATUS_UNDEFINED, "undefined"),
];

pub const VIDEO_STOPPED: u8 = 0;
pub const VIDEO_RUNNING: u8 = 1;
pub const VIDEO_STATUS_LAST: u8 = 2;
pub const VIDEO_STATUS: &[(u8, &str)] = &[(VIDEO_STOPPED, "stopped"), (VIDEO_RUNNING, "running"), (STATUS_UNDEFINED, "undefined")];

pub const STORAGE_EMPTY: u8 = 0;
pub const STORAGE_UNFORMATTED: u8 = 1;
pub const STORAGE_READY: u8 = 2;
pub const STORAGE_NOT_SUPPORTED: u8 = 3;
pub const STORAGE_STATUS: &[(u8, &str)] =
    &[(STORAGE_EMPTY, "empty"), (STORAGE_UNFORMATTED, "unformatted"), (STORAGE_READY, "ready"), (STORAGE_NOT_SUPPORTED, "notSupported")];

pub const STREAM_TYPES: &[(u8, &str)] = &[(0, "rtsp"), (1, "rtpUdp"), (2, "tcpMpeg"), (3, "mpegTs")];
pub const STREAM_FLAG_RUNNING: u16 = 1;
pub const STREAM_FLAG_THERMAL: u16 = 2;

pub const HEARTBEAT_TICK_MS: u64 = 500;
pub const SILENT_TIMEOUT_MS: u64 = 5000;
pub const MAX_INFORMATION_ATTEMPTS: u32 = 10;
pub const SETTINGS_DELAY_MS: u64 = 500;
pub const STREAM_CHECK_DELAY_MS: u64 = 1000;
pub const CAPTURE_STATUS_DELAY_MS: u64 = 1500;
pub const STORAGE_DELAY_MS: u64 = 2000;
pub const REQUEST_TIMEOUT_MS: u64 = 1000;
pub const STREAM_DISCOVERY_MS: u64 = 2000;
pub const SETTINGS_REFRESH_DELAY_MS: u64 = 1000;
pub const RESET_SETTINGS_DELAY_MS: u64 = 2500;
pub const RESET_ACK_TIMEOUT_MS: u64 = 5000;
pub const MAX_REQUEST_ATTEMPTS: u32 = 5;
pub const STREAM_ATTEMPTS_PER_STREAM: u32 = 6;
pub const RECORDING_POLL_MS: u64 = 5000;
pub const BUSY_POLL_MS: u64 = 1000;
pub const MODE_CHANGE_POLL_MS: u64 = 1000;
pub const STALE_MS: u64 = 10_000;

pub const MIN_LEVEL_PERCENT: f64 = 0.0;
pub const MAX_LEVEL_PERCENT: f64 = 100.0;
pub const FALLBACK_ASPECT: f64 = 9.0 / 16.0;
pub const MAX_FIELD_OF_VIEW_DEGREES: f64 = 180.0;
pub const FULL_TURN_DEGREES: f64 = 360.0;

pub const STORAGE_UNITS: &str = "mebibyte";
pub const DURATION_UNITS: &str = "millisecond";
pub const ANGLE_UNITS: &str = "degree";
pub const FRAME_RATE_UNITS: &str = "hertz";
pub const BIT_RATE_UNITS: &str = "bitPerSecond";
pub const LENGTH_UNITS: &str = "millimetre";
pub const RESOLUTION_UNITS: &str = "pixel";
pub const LEVEL_UNITS: &str = "percent";
pub const INTERVAL_UNITS: &str = "second";

fn token(table: &[(u8, &'static str)], value: u8) -> Option<&'static str> {
    table.iter().find(|(code, _)| *code == value).map(|(_, name)| *name)
}

fn wrap_degrees(degrees: f64) -> f64 {
    degrees.rem_euclid(FULL_TURN_DEGREES)
}

fn finite_positive(value: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0).then_some(value)
}

fn finite_capacity(value: f64) -> Option<f64> {
    (value.is_finite() && value >= 0.0).then_some(value)
}

fn age_ms(at_ms: Option<u64>, now_ms: u64) -> Option<u64> {
    at_ms.map(|at| now_ms.saturating_sub(at))
}

fn stale(at_ms: Option<u64>, now_ms: u64) -> bool {
    age_ms(at_ms, now_ms).is_none_or(|age| age > STALE_MS)
}

pub fn is_camera_component(compid: u8) -> bool {
    (COMP_ID_CAMERA..=COMP_ID_CAMERA6).contains(&compid)
}

pub fn vertical_field_of_view_degrees(horizontal_degrees: f64, aspect_vertical_over_horizontal: Option<f64>) -> Option<f64> {
    let horizontal = finite_positive(horizontal_degrees).filter(|hfov| *hfov < MAX_FIELD_OF_VIEW_DEGREES)?;
    let aspect = aspect_vertical_over_horizontal.and_then(finite_positive).unwrap_or(FALLBACK_ASPECT);
    let vertical = 2.0 * ((horizontal.to_radians() * 0.5).tan() * aspect).atan();
    finite_positive(vertical.to_degrees()).filter(|vfov| *vfov < MAX_FIELD_OF_VIEW_DEGREES)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Command {
    pub compid: u8,
    pub command: u16,
    pub params: [f64; 7],
}

fn command(compid: u8, command: u16, params: [f64; 7]) -> Command {
    Command { compid, command, params }
}

fn request_message(compid: u8, message: u32, param2: f64) -> Command {
    command(compid, CMD_REQUEST_MESSAGE, [message as f64, param2, 0.0, 0.0, 0.0, 0.0, 0.0])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    NoCamera,
    Resetting,
    Unsupported,
    Busy,
    NotCapturing,
    WrongMode,
    UnknownStream,
    UnknownStorage,
    ModeIsParameter,
}

pub const REFUSALS: &[Refusal] = &[
    Refusal::NoCamera,
    Refusal::Resetting,
    Refusal::Unsupported,
    Refusal::Busy,
    Refusal::NotCapturing,
    Refusal::WrongMode,
    Refusal::UnknownStream,
    Refusal::UnknownStorage,
    Refusal::ModeIsParameter,
];

impl Refusal {
    pub fn token(self) -> &'static str {
        match self {
            Refusal::NoCamera => "noCamera",
            Refusal::Resetting => "resetting",
            Refusal::Unsupported => "unsupported",
            Refusal::Busy => "busy",
            Refusal::NotCapturing => "notCapturing",
            Refusal::WrongMode => "wrongMode",
            Refusal::UnknownStream => "unknownStream",
            Refusal::UnknownStorage => "unknownStorage",
            Refusal::ModeIsParameter => "modeIsParameter",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    SetMode,
    TakePhoto,
    StopTakePhoto,
    StartRecording,
    StopRecording,
    Zoom,
    Focus,
    FormatStorage,
    SelectStream,
    ResetSettings,
}

pub const ACTIONS: &[Action] = &[
    Action::SetMode,
    Action::TakePhoto,
    Action::StopTakePhoto,
    Action::StartRecording,
    Action::StopRecording,
    Action::Zoom,
    Action::Focus,
    Action::FormatStorage,
    Action::SelectStream,
    Action::ResetSettings,
];

impl Action {
    pub fn token(self) -> &'static str {
        match self {
            Action::SetMode => "setMode",
            Action::TakePhoto => "takePhoto",
            Action::StopTakePhoto => "stopTakePhoto",
            Action::StartRecording => "startRecording",
            Action::StopRecording => "stopRecording",
            Action::Zoom => "zoom",
            Action::Focus => "focus",
            Action::FormatStorage => "formatStorage",
            Action::SelectStream => "selectStream",
            Action::ResetSettings => "resetSettings",
        }
    }
}

fn offer(refusal: Option<Refusal>) -> Value {
    json!({
        "offer": match refusal { Some(_) => "blocked", None => "ready" },
        "refusal": refusal.map(Refusal::token).unwrap_or(""),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Zoom,
    Focus,
}

impl Axis {
    fn command(self) -> u16 {
        match self {
            Axis::Zoom => CMD_SET_CAMERA_ZOOM,
            Axis::Focus => CMD_SET_CAMERA_FOCUS,
        }
    }

    fn action(self) -> Action {
        match self {
            Axis::Zoom => Action::Zoom,
            Axis::Focus => Action::Focus,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Firmware {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub dev: u8,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Info {
    pub vendor: String,
    pub model: String,
    pub firmware_version: u32,
    pub focal_length_mm: f64,
    pub sensor_size_h_mm: f64,
    pub sensor_size_v_mm: f64,
    pub resolution_h: u16,
    pub resolution_v: u16,
    pub flags: u32,
    pub definition_version: u16,
    pub definition_uri: String,
    pub gimbal_device_id: u8,
}

impl Info {
    pub fn has(&self, capability: u32) -> bool {
        self.flags & capability != 0
    }

    pub fn firmware(&self) -> Option<Firmware> {
        (self.firmware_version != 0).then_some(Firmware {
            major: (self.firmware_version & 0xff) as u8,
            minor: (self.firmware_version >> 8 & 0xff) as u8,
            patch: (self.firmware_version >> 16 & 0xff) as u8,
            dev: (self.firmware_version >> 24 & 0xff) as u8,
        })
    }

    pub fn aspect_vertical_over_horizontal(&self) -> Option<f64> {
        match (self.resolution_h, self.resolution_v) {
            (0, _) | (_, 0) => finite_positive(self.sensor_size_h_mm)
                .zip(finite_positive(self.sensor_size_v_mm))
                .map(|(horizontal, vertical)| vertical / horizontal),
            (horizontal, vertical) => Some(f64::from(vertical) / f64::from(horizontal)),
        }
    }

    pub fn capability_tokens(&self) -> Vec<&'static str> {
        CAPABILITIES.iter().filter(|(flag, _)| self.has(*flag)).map(|(_, name)| *name).collect()
    }

    pub fn definition_uri(&self) -> Option<&str> {
        (!self.definition_uri.is_empty()).then_some(self.definition_uri.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SettingsReport {
    pub mode_id: u8,
    pub zoom_percent: f64,
    pub focus_percent: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CaptureStatusReport {
    pub image_status: u8,
    pub video_status: u8,
    pub image_interval_s: f64,
    pub recording_time_ms: u32,
    pub available_capacity_mib: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StorageReport {
    pub storage_id: u8,
    pub storage_count: u8,
    pub status: u8,
    pub total_capacity_mib: f64,
    pub available_capacity_mib: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StreamReport {
    pub stream_id: u8,
    pub count: u8,
    pub kind: u8,
    pub flags: u16,
    pub framerate_hz: f64,
    pub resolution_h: u16,
    pub resolution_v: u16,
    pub bitrate_bps: u32,
    pub rotation_deg: f64,
    pub hfov_deg: f64,
    pub name: String,
    pub uri: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StreamStatusReport {
    pub stream_id: u8,
    pub flags: u16,
    pub framerate_hz: f64,
    pub resolution_h: u16,
    pub resolution_v: u16,
    pub bitrate_bps: u32,
    pub rotation_deg: f64,
    pub hfov_deg: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    pub stream_id: u8,
    pub kind: u8,
    pub flags: u16,
    pub framerate_hz: f64,
    pub resolution_h: u16,
    pub resolution_v: u16,
    pub bitrate_bps: u32,
    pub rotation_deg: f64,
    pub hfov_deg: f64,
    pub name: String,
    pub uri: String,
    pub heard_ms: u64,
}

impl Stream {
    pub fn running(&self) -> bool {
        self.flags & STREAM_FLAG_RUNNING != 0
    }

    pub fn thermal(&self) -> bool {
        self.flags & STREAM_FLAG_THERMAL != 0
    }

    fn snapshot(&self, now_ms: u64) -> Value {
        json!({
            "streamId": self.stream_id,
            "type": token(STREAM_TYPES, self.kind),
            "name": self.name,
            "uri": self.uri,
            "running": self.running(),
            "thermal": self.thermal(),
            "frameRate": finite_positive(self.framerate_hz),
            "resolution": resolution(self.resolution_h, self.resolution_v),
            "bitRate": (self.bitrate_bps != 0).then_some(self.bitrate_bps),
            "rotation": self.rotation_deg,
            "horizontalFieldOfView": finite_positive(self.hfov_deg),
            "ageMs": age_ms(Some(self.heard_ms), now_ms),
            "stale": stale(Some(self.heard_ms), now_ms),
        })
    }
}

fn resolution(horizontal: u16, vertical: u16) -> Option<Value> {
    (horizontal != 0 && vertical != 0).then(|| json!({ "horizontal": horizontal, "vertical": vertical }))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Storage {
    pub storage_id: u8,
    pub status: u8,
    pub total_mib: Option<f64>,
    pub free_mib: Option<f64>,
    pub at_ms: u64,
}

impl Storage {
    fn snapshot(&self, now_ms: u64) -> Value {
        json!({
            "storageId": self.storage_id,
            "status": token(STORAGE_STATUS, self.status),
            "total": self.total_mib,
            "free": self.free_mib,
            "ageMs": age_ms(Some(self.at_ms), now_ms),
            "stale": stale(Some(self.at_ms), now_ms),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Outcome {
    command: u16,
    result: Option<u8>,
    at_ms: u64,
}

impl Outcome {
    fn snapshot(&self, now_ms: u64) -> Value {
        json!({
            "command": self.command,
            "result": self.result.and_then(|result| token(RESULTS, result)),
            "pending": self.result.is_none(),
            "ageMs": now_ms.saturating_sub(self.at_ms),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Commanded {
    mode: Option<u8>,
    photo_status: Option<u8>,
    video_status: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Pending {
    attempts: u32,
    due: Option<u64>,
    asked: bool,
    answered: bool,
}

impl Pending {
    fn arm(&mut self, at: u64) {
        self.due = Some(at);
    }

    fn settle(&mut self) {
        *self = Pending { answered: true, ..Pending::default() };
    }

    fn give_up(&mut self) {
        self.due = None;
    }

    fn ripe(&self, now_ms: u64) -> bool {
        self.due.is_some_and(|due| now_ms >= due)
    }

    fn spend(&mut self, budget: u32) -> bool {
        if !self.asked {
            return true;
        }
        self.attempts += 1;
        match self.attempts > budget {
            true => {
                self.give_up();
                false
            }
            false => true,
        }
    }

    fn state(&self) -> &'static str {
        match (self.due.is_some(), self.asked, self.answered) {
            (true, _, _) => "waiting",
            (false, true, _) => "gaveUp",
            (false, false, true) => "answered",
            (false, false, false) => "idle",
        }
    }

    fn snapshot(&self) -> Value {
        json!({ "attempts": self.attempts, "state": self.state() })
    }
}

#[derive(Debug)]
pub struct Camera {
    pub compid: u8,
    pub info: Info,
    pub mode: Option<u8>,
    pub zoom_percent: Option<f64>,
    pub focus_percent: Option<f64>,
    pub photo_status: Option<u8>,
    pub video_status: Option<u8>,
    pub storage_count: u8,
    pub free_mib: Option<f64>,
    pub battery_percent: Option<i8>,
    pub streams: Vec<Stream>,
    pub expected_streams: u8,
    pub selected_stream: Option<u8>,
    pub image_interval_s: Option<f64>,
    storage: BTreeMap<u8, Storage>,
    commanded: Commanded,
    outcome: Option<Outcome>,
    last_heartbeat_ms: u64,
    record_since_ms: Option<u64>,
    settings_at_ms: Option<u64>,
    capture_at_ms: Option<u64>,
    battery_at_ms: Option<u64>,
    free_at_ms: Option<u64>,
    parameters_expected: Option<bool>,
    mode_is_parameter: bool,
    resetting_until: Option<u64>,
    settings: Pending,
    settings_refresh: Option<u64>,
    storage_request: Pending,
    capture: Pending,
    capture_command_attempts: u32,
    stream_info: Pending,
    stream_status: Pending,
}

impl Camera {
    fn new(compid: u8, info: Info, now_ms: u64) -> Camera {
        let streams = info.has(CAP_HAS_VIDEO_STREAM);
        Camera {
            compid,
            info,
            mode: None,
            zoom_percent: None,
            focus_percent: None,
            photo_status: None,
            video_status: None,
            storage_count: 0,
            free_mib: None,
            battery_percent: None,
            streams: Vec::new(),
            expected_streams: 1,
            selected_stream: None,
            image_interval_s: None,
            storage: BTreeMap::new(),
            commanded: Commanded::default(),
            outcome: None,
            last_heartbeat_ms: now_ms,
            record_since_ms: None,
            settings_at_ms: None,
            capture_at_ms: None,
            battery_at_ms: None,
            free_at_ms: None,
            parameters_expected: None,
            mode_is_parameter: false,
            resetting_until: None,
            settings: Pending { due: Some(now_ms + SETTINGS_DELAY_MS), ..Pending::default() },
            settings_refresh: None,
            storage_request: Pending { due: Some(now_ms + STORAGE_DELAY_MS), ..Pending::default() },
            capture: Pending { due: Some(now_ms + CAPTURE_STATUS_DELAY_MS), ..Pending::default() },
            capture_command_attempts: 0,
            stream_info: Pending { due: streams.then_some(now_ms + STREAM_CHECK_DELAY_MS), ..Pending::default() },
            stream_status: Pending::default(),
        }
    }

    pub fn record_time_ms(&self, now_ms: u64) -> Option<u64> {
        self.record_since_ms.map(|since| now_ms.saturating_sub(since))
    }

    pub fn mode_now(&self) -> Option<u8> {
        self.commanded.mode.or(self.mode)
    }

    pub fn photo_status_now(&self) -> Option<u8> {
        self.commanded.photo_status.or(self.photo_status)
    }

    pub fn video_status_now(&self) -> Option<u8> {
        self.commanded.video_status.or(self.video_status)
    }

    pub fn recording(&self) -> bool {
        self.video_status_now() == Some(VIDEO_RUNNING)
    }

    pub fn resetting(&self) -> bool {
        self.resetting_until.is_some()
    }

    pub fn expects_parameters(&self) -> bool {
        self.parameters_expected.unwrap_or(self.info.definition_uri().is_some())
    }

    pub fn listed_streams(&self) -> Vec<&Stream> {
        self.streams.iter().filter(|stream| !stream.thermal()).collect()
    }

    pub fn thermal_stream(&self) -> Option<&Stream> {
        self.streams.iter().find(|stream| stream.thermal())
    }

    pub fn current_stream(&self) -> Option<&Stream> {
        let listed = self.listed_streams();
        self.selected_stream
            .and_then(|id| listed.iter().find(|stream| stream.stream_id == id).copied())
            .or_else(|| listed.first().copied())
    }

    pub fn gate(&self, action: Action) -> Option<Refusal> {
        let resetting = self.resetting().then_some(Refusal::Resetting);
        let missing = |capability: u32| (!self.info.has(capability)).then_some(Refusal::Unsupported);
        match action {
            Action::SetMode => resetting
                .or_else(|| missing(CAP_HAS_MODES))
                .or_else(|| self.mode_is_parameter.then_some(Refusal::ModeIsParameter)),
            Action::TakePhoto => resetting
                .or_else(|| missing(CAP_CAPTURE_IMAGE))
                .or_else(|| (self.mode_now() == Some(MODE_VIDEO) && !self.info.has(CAP_IMAGE_IN_VIDEO_MODE)).then_some(Refusal::WrongMode))
                .or_else(|| (self.photo_status_now().unwrap_or(PHOTO_IDLE) != PHOTO_IDLE).then_some(Refusal::Busy)),
            Action::StopTakePhoto => resetting
                .or_else(|| (!matches!(self.photo_status_now(), Some(PHOTO_INTERVAL_IDLE | PHOTO_INTERVAL_IN_PROGRESS))).then_some(Refusal::NotCapturing)),
            Action::StartRecording => resetting
                .or_else(|| missing(CAP_CAPTURE_VIDEO))
                .or_else(|| self.recording().then_some(Refusal::Busy))
                .or_else(|| (self.mode_now() == Some(MODE_PHOTO) && !self.info.has(CAP_VIDEO_IN_IMAGE_MODE)).then_some(Refusal::WrongMode)),
            Action::StopRecording => resetting
                .or_else(|| missing(CAP_CAPTURE_VIDEO))
                .or_else(|| (self.video_status_now() == Some(VIDEO_STOPPED)).then_some(Refusal::NotCapturing)),
            Action::Zoom => missing(CAP_HAS_BASIC_ZOOM),
            Action::Focus => missing(CAP_HAS_BASIC_FOCUS),
            Action::FormatStorage => resetting.or_else(|| self.recording().then_some(Refusal::Busy)),
            Action::SelectStream => resetting.or_else(|| self.listed_streams().is_empty().then_some(Refusal::UnknownStream)),
            Action::ResetSettings => None,
        }
    }

    fn note_sent(&mut self, sent: u16, now_ms: u64) {
        self.outcome = Some(Outcome { command: sent, result: None, at_ms: now_ms });
    }

    fn request_settings(&mut self, now_ms: u64) -> Command {
        let sent = match self.settings.attempts % 2 {
            0 => request_message(self.compid, MSG_CAMERA_SETTINGS, 0.0),
            _ => command(self.compid, CMD_REQUEST_CAMERA_SETTINGS, [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        };
        self.settings.asked = true;
        self.settings.arm(now_ms + REQUEST_TIMEOUT_MS);
        sent
    }

    fn request_storage(&mut self, now_ms: u64) -> Command {
        let sent = match self.storage_request.attempts % 2 {
            0 => request_message(self.compid, MSG_STORAGE_INFORMATION, 0.0),
            _ => command(self.compid, CMD_REQUEST_STORAGE_INFORMATION, [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        };
        self.storage_request.asked = true;
        self.storage_request.arm(now_ms + REQUEST_TIMEOUT_MS);
        sent
    }

    fn request_capture_status(&mut self, now_ms: u64) -> Command {
        let sent = match self.capture.attempts % 2 {
            0 => request_message(self.compid, MSG_CAMERA_CAPTURE_STATUS, 0.0),
            _ => command(self.compid, CMD_REQUEST_CAMERA_CAPTURE_STATUS, [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        };
        self.capture.asked = true;
        self.capture.arm(now_ms + REQUEST_TIMEOUT_MS);
        sent
    }

    fn request_stream_info(&mut self, stream_id: u8, now_ms: u64) -> Command {
        let sent = match self.stream_info.attempts % 2 {
            0 => request_message(self.compid, MSG_VIDEO_STREAM_INFORMATION, f64::from(stream_id)),
            _ => command(self.compid, CMD_REQUEST_VIDEO_STREAM_INFORMATION, [f64::from(stream_id), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        };
        self.stream_info.asked = true;
        self.stream_info.arm(now_ms + REQUEST_TIMEOUT_MS);
        sent
    }

    fn request_stream_status(&mut self, stream_id: u8, now_ms: u64) -> Command {
        let sent = match self.stream_status.attempts % 2 {
            0 => request_message(self.compid, MSG_VIDEO_STREAM_STATUS, f64::from(stream_id)),
            _ => command(self.compid, CMD_REQUEST_VIDEO_STREAM_STATUS, [f64::from(stream_id), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        };
        self.stream_status.asked = true;
        self.stream_status.arm(now_ms + REQUEST_TIMEOUT_MS);
        sent
    }

    fn settings_due(&mut self, now_ms: u64) -> Option<Command> {
        self.settings.spend(MAX_REQUEST_ATTEMPTS).then(|| self.request_settings(now_ms))
    }

    fn storage_due(&mut self, now_ms: u64) -> Option<Command> {
        self.storage_request.spend(MAX_REQUEST_ATTEMPTS).then(|| self.request_storage(now_ms))
    }

    fn capture_due(&mut self, now_ms: u64) -> Option<Command> {
        self.capture.spend(MAX_REQUEST_ATTEMPTS).then(|| self.request_capture_status(now_ms))
    }

    fn stream_info_due(&mut self, now_ms: u64) -> Option<Command> {
        if !self.stream_info.asked {
            let sent = self.request_stream_info(0, now_ms);
            self.stream_info.arm(now_ms + STREAM_DISCOVERY_MS);
            return Some(sent);
        }
        if !self.stream_info.spend(u32::from(self.expected_streams) * STREAM_ATTEMPTS_PER_STREAM) {
            return None;
        }
        let missing = (1..=self.expected_streams).find(|id| !self.streams.iter().any(|stream| stream.stream_id == *id));
        match missing {
            Some(id) => Some(self.request_stream_info(id, now_ms)),
            None => {
                self.stream_info.give_up();
                None
            }
        }
    }

    fn stream_status_due(&mut self, now_ms: u64) -> Option<Command> {
        self.stream_status.attempts += 1;
        let spent = self.stream_status.attempts > MAX_REQUEST_ATTEMPTS;
        let stream_id = self.current_stream().map(|stream| stream.stream_id);
        match (spent, stream_id) {
            (false, Some(id)) => Some(self.request_stream_status(id, now_ms)),
            _ => {
                self.stream_status.give_up();
                None
            }
        }
    }

    fn tick(&mut self, now_ms: u64) -> Vec<Command> {
        self.resetting_until = self.resetting_until.filter(|until| now_ms < *until);
        let settings = self.settings.ripe(now_ms).then(|| self.settings_due(now_ms)).flatten();
        let refresh = self.settings_refresh.filter(|due| now_ms >= *due).map(|_| {
            self.settings_refresh = None;
            self.settings.settle();
            self.request_settings(now_ms)
        });
        let storage = self.storage_request.ripe(now_ms).then(|| self.storage_due(now_ms)).flatten();
        let capture = self.capture.ripe(now_ms).then(|| self.capture_due(now_ms)).flatten();
        let stream_info = self.stream_info.ripe(now_ms).then(|| self.stream_info_due(now_ms)).flatten();
        let stream_status = self.stream_status.ripe(now_ms).then(|| self.stream_status_due(now_ms)).flatten();
        settings.into_iter().chain(refresh).chain(storage).chain(capture).chain(stream_info).chain(stream_status).collect()
    }

    fn command_mode(&mut self, mode: u8, now_ms: u64) {
        if self.mode_now() != Some(mode) {
            self.commanded.mode = Some(mode);
            self.stream_status.arm(now_ms + MODE_CHANGE_POLL_MS);
            self.settings_refresh = Some(now_ms + SETTINGS_REFRESH_DELAY_MS);
        }
    }

    fn adopt_mode(&mut self, mode: u8, now_ms: u64) {
        self.commanded.mode = None;
        if self.mode != Some(mode) {
            self.mode = Some(mode);
            self.stream_status.arm(now_ms + MODE_CHANGE_POLL_MS);
        }
    }

    fn on_settings(&mut self, report: SettingsReport, now_ms: u64) {
        self.settings.settle();
        self.settings_at_ms = Some(now_ms);
        self.adopt_mode(report.mode_id, now_ms);
        self.zoom_percent = report.zoom_percent.is_finite().then_some(report.zoom_percent).or(self.zoom_percent);
        self.focus_percent = report.focus_percent.is_finite().then_some(report.focus_percent).or(self.focus_percent);
    }

    fn on_storage(&mut self, report: StorageReport, now_ms: u64) {
        self.storage_request.settle();
        self.storage_count = report.storage_count;
        let ready = report.status == STORAGE_READY;
        let kept = self.storage.get(&report.storage_id).copied();
        let total = ready.then(|| finite_capacity(report.total_capacity_mib)).flatten();
        let free = ready.then(|| finite_capacity(report.available_capacity_mib)).flatten();
        self.storage.insert(
            report.storage_id,
            Storage {
                storage_id: report.storage_id,
                status: report.status,
                total_mib: total.or(kept.and_then(|slot| slot.total_mib)),
                free_mib: free.or(kept.and_then(|slot| slot.free_mib)),
                at_ms: now_ms,
            },
        );
        self.free_at_ms = free.map(|_| now_ms).or(self.free_at_ms);
        self.free_mib = free.or(self.free_mib);
    }

    fn on_capture_status(&mut self, report: CaptureStatusReport, now_ms: u64) {
        self.capture.settle();
        self.capture_at_ms = Some(now_ms);
        self.commanded.photo_status = None;
        self.commanded.video_status = None;
        let free = finite_capacity(report.available_capacity_mib);
        self.free_at_ms = free.map(|_| now_ms).or(self.free_at_ms);
        self.free_mib = free.or(self.free_mib);
        let video = match report.video_status < VIDEO_STATUS_LAST {
            true => report.video_status,
            false => STATUS_UNDEFINED,
        };
        let photo = match report.image_status < PHOTO_STATUS_LAST {
            true => report.image_status,
            false => STATUS_UNDEFINED,
        };
        self.adopt_video_status(video, now_ms);
        self.photo_status = Some(photo);
        self.image_interval_s = finite_positive(report.image_interval_s);
        if report.recording_time_ms != 0 {
            self.record_since_ms = Some(now_ms.saturating_sub(u64::from(report.recording_time_ms)));
        }
        let lapse = photo == PHOTO_INTERVAL_IDLE || photo == PHOTO_INTERVAL_IN_PROGRESS;
        if self.recording() || lapse {
            self.capture.arm(now_ms + RECORDING_POLL_MS);
        } else if photo != PHOTO_IDLE && photo != STATUS_UNDEFINED {
            self.capture.arm(now_ms + BUSY_POLL_MS);
        }
    }

    fn adopt_video_status(&mut self, status: u8, now_ms: u64) {
        self.commanded.video_status = None;
        if self.video_status == Some(status) {
            return;
        }
        self.video_status = Some(status);
        self.record_since_ms = (status == VIDEO_RUNNING).then_some(now_ms);
    }

    fn on_stream_info(&mut self, report: StreamReport, now_ms: u64) {
        self.expected_streams = report.count;
        let known = self.streams.iter().any(|stream| stream.stream_id == report.stream_id);
        let first_listed = !known && report.flags & STREAM_FLAG_THERMAL == 0 && self.listed_streams().is_empty();
        if !known {
            let arrived = Stream {
                stream_id: report.stream_id,
                kind: report.kind,
                flags: report.flags,
                framerate_hz: report.framerate_hz,
                resolution_h: report.resolution_h,
                resolution_v: report.resolution_v,
                bitrate_bps: report.bitrate_bps,
                rotation_deg: wrap_degrees(report.rotation_deg),
                hfov_deg: report.hfov_deg,
                name: report.name,
                uri: report.uri,
                heard_ms: now_ms,
            };
            self.streams = self.streams.iter().cloned().chain([arrived]).collect();
        }
        match self.streams.len() < usize::from(self.expected_streams) {
            true => self.stream_info.arm(now_ms + REQUEST_TIMEOUT_MS),
            false => self.stream_info.settle(),
        }
        if first_listed {
            self.stream_status.settle();
            self.stream_status.arm(now_ms + STREAM_CHECK_DELAY_MS);
        }
    }

    fn on_stream_status(&mut self, report: StreamStatusReport, now_ms: u64) {
        self.stream_status.settle();
        if let Some(stream) = self.streams.iter_mut().find(|stream| stream.stream_id == report.stream_id) {
            stream.flags = report.flags;
            stream.framerate_hz = report.framerate_hz;
            stream.resolution_h = report.resolution_h;
            stream.resolution_v = report.resolution_v;
            stream.bitrate_bps = report.bitrate_bps;
            stream.rotation_deg = wrap_degrees(report.rotation_deg);
            stream.hfov_deg = report.hfov_deg;
            stream.heard_ms = now_ms;
        }
    }

    fn on_command_result(&mut self, sent: u16, result: u8, now_ms: u64) {
        self.outcome = Some(Outcome { command: sent, result: Some(result), at_ms: now_ms });
        if result == RESULT_IN_PROGRESS {
            return;
        }
        if sent == CMD_RESET_CAMERA_SETTINGS {
            self.resetting_until = None;
        }
        match (result, sent) {
            (RESULT_ACCEPTED, CMD_RESET_CAMERA_SETTINGS) => {
                self.settings.settle();
                self.settings.arm(match self.expects_parameters() {
                    true => now_ms + RESET_SETTINGS_DELAY_MS,
                    false => now_ms,
                });
            }
            (RESULT_ACCEPTED, CMD_VIDEO_START_CAPTURE) => {
                self.adopt_video_status(VIDEO_RUNNING, now_ms);
                self.capture.arm(now_ms + BUSY_POLL_MS);
            }
            (RESULT_ACCEPTED, CMD_VIDEO_STOP_CAPTURE) => {
                self.adopt_video_status(VIDEO_STOPPED, now_ms);
                self.capture.arm(now_ms + BUSY_POLL_MS);
            }
            (RESULT_ACCEPTED, CMD_REQUEST_CAMERA_CAPTURE_STATUS) => self.capture.attempts = 0,
            (RESULT_ACCEPTED, CMD_REQUEST_STORAGE_INFORMATION) => self.storage_request.attempts = 0,
            (RESULT_ACCEPTED, CMD_IMAGE_START_CAPTURE) => self.capture.arm(now_ms + BUSY_POLL_MS),
            (RESULT_ACCEPTED, CMD_SET_CAMERA_ZOOM | CMD_SET_CAMERA_FOCUS) => self.settings_refresh = Some(now_ms + SETTINGS_REFRESH_DELAY_MS),
            (RESULT_ACCEPTED, CMD_STORAGE_FORMAT) => {
                self.storage_request.settle();
                self.storage_request.arm(now_ms + STORAGE_DELAY_MS);
            }
            (RESULT_ACCEPTED, CMD_SET_CAMERA_MODE) => self.settings_refresh = Some(now_ms + SETTINGS_REFRESH_DELAY_MS),
            (_, CMD_SET_CAMERA_MODE) => self.commanded.mode = None,
            (_, CMD_VIDEO_START_CAPTURE | CMD_VIDEO_STOP_CAPTURE) => self.commanded.video_status = None,
            (_, CMD_STORAGE_FORMAT) => {
                self.storage_request.settle();
                self.storage_request.arm(now_ms + STORAGE_DELAY_MS);
            }
            (RESULT_TEMPORARILY_REJECTED | RESULT_FAILED, CMD_IMAGE_START_CAPTURE | CMD_IMAGE_STOP_CAPTURE) => {
                self.capture_command_attempts += 1;
                match self.capture_command_attempts <= MAX_REQUEST_ATTEMPTS {
                    true => self.capture.arm(now_ms + BUSY_POLL_MS),
                    false => self.commanded.photo_status = None,
                }
            }
            (_, CMD_IMAGE_START_CAPTURE | CMD_IMAGE_STOP_CAPTURE) => self.commanded.photo_status = None,
            (RESULT_TEMPORARILY_REJECTED | RESULT_FAILED, CMD_REQUEST_CAMERA_CAPTURE_STATUS) => {
                self.capture.attempts += 1;
                if self.capture.attempts <= MAX_REQUEST_ATTEMPTS {
                    self.capture.arm(now_ms + BUSY_POLL_MS);
                }
            }
            (RESULT_TEMPORARILY_REJECTED | RESULT_FAILED, CMD_REQUEST_STORAGE_INFORMATION) => {
                self.storage_request.attempts += 1;
                if self.storage_request.attempts <= MAX_REQUEST_ATTEMPTS {
                    self.storage_request.arm(now_ms + REQUEST_TIMEOUT_MS);
                }
            }
            _ => (),
        }
    }

    fn storage_snapshot(&self, now_ms: u64) -> Option<Value> {
        (!self.storage.is_empty() || self.free_mib.is_some()).then(|| {
            json!({
                "free": self.free_mib,
                "ageMs": age_ms(self.free_at_ms, now_ms),
                "stale": stale(self.free_at_ms, now_ms),
                "storageCount": (self.storage_count != 0).then_some(self.storage_count),
                "slots": self.storage.values().map(|slot| slot.snapshot(now_ms)).collect::<Vec<_>>(),
            })
        })
    }

    fn actions_snapshot(&self) -> Value {
        Value::Object(ACTIONS.iter().map(|action| (action.token().to_string(), offer(self.gate(*action)))).collect())
    }

    fn snapshot(&self, now_ms: u64) -> Value {
        json!({
            "componentId": self.compid,
            "vendor": self.info.vendor,
            "model": self.info.model,
            "firmware": self.info.firmware().map(|firmware| json!({ "major": firmware.major, "minor": firmware.minor, "patch": firmware.patch, "dev": firmware.dev })),
            "capabilities": self.info.capability_tokens(),
            "definitionVersion": self.info.definition_version,
            "definitionUri": self.info.definition_uri(),
            "expectsParameters": self.expects_parameters(),
            "modeIsParameter": self.mode_is_parameter,
            "gimbalDeviceId": (self.info.gimbal_device_id != 0).then_some(self.info.gimbal_device_id),
            "resolution": resolution(self.info.resolution_h, self.info.resolution_v),
            "sensorSize": finite_positive(self.info.sensor_size_h_mm).zip(finite_positive(self.info.sensor_size_v_mm)).map(|(horizontal, vertical)| json!({ "horizontal": horizontal, "vertical": vertical })),
            "focalLength": finite_positive(self.info.focal_length_mm),
            "aspect": self.info.aspect_vertical_over_horizontal(),
            "mode": self.mode.and_then(|mode| token(MODES, mode)),
            "commandedMode": self.commanded.mode.and_then(|mode| token(MODES, mode)),
            "zoom": self.zoom_percent,
            "focus": self.focus_percent,
            "settingsAgeMs": age_ms(self.settings_at_ms, now_ms),
            "settingsStale": stale(self.settings_at_ms, now_ms),
            "photoStatus": self.photo_status.and_then(|status| token(PHOTO_STATUS, status)),
            "commandedPhotoStatus": self.commanded.photo_status.and_then(|status| token(PHOTO_STATUS, status)),
            "videoStatus": self.video_status.and_then(|status| token(VIDEO_STATUS, status)),
            "commandedVideoStatus": self.commanded.video_status.and_then(|status| token(VIDEO_STATUS, status)),
            "captureAgeMs": age_ms(self.capture_at_ms, now_ms),
            "captureStale": stale(self.capture_at_ms, now_ms),
            "imageInterval": self.image_interval_s,
            "recordTime": self.record_time_ms(now_ms),
            "storage": self.storage_snapshot(now_ms),
            "battery": self.battery_percent,
            "batteryAgeMs": age_ms(self.battery_at_ms, now_ms),
            "batteryStale": stale(self.battery_at_ms, now_ms),
            "resetting": self.resetting(),
            "expectedStreams": self.expected_streams,
            "streams": self.listed_streams().iter().map(|stream| stream.snapshot(now_ms)).collect::<Vec<_>>(),
            "selectedStream": self.current_stream().map(|stream| stream.stream_id),
            "thermalStream": self.thermal_stream().map(|stream| stream.stream_id),
            "lastCommand": self.outcome.map(|outcome| outcome.snapshot(now_ms)),
            "actions": self.actions_snapshot(),
            "requests": json!({
                "cameraSettings": self.settings.snapshot(),
                "storageInformation": self.storage_request.snapshot(),
                "captureStatus": self.capture.snapshot(),
                "videoStreamInformation": self.stream_info.snapshot(),
                "videoStreamStatus": self.stream_status.snapshot(),
            }),
        })
    }
}

#[derive(Debug)]
struct Tracked {
    compid: u8,
    last_heartbeat_ms: u64,
    attempts: u32,
    due: Option<u64>,
    identified: bool,
}

impl Tracked {
    fn request(&mut self) -> Option<Command> {
        self.due = None;
        if self.attempts >= MAX_INFORMATION_ATTEMPTS {
            return None;
        }
        match self.attempts % 2 {
            0 => Some(request_message(self.compid, MSG_CAMERA_INFORMATION, 0.0)),
            _ => Some(command(self.compid, CMD_REQUEST_CAMERA_INFORMATION, [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])),
        }
    }

    fn retry(&mut self, now_ms: u64) -> Option<Command> {
        if self.attempts >= MAX_INFORMATION_ATTEMPTS {
            self.due = None;
            return None;
        }
        self.attempts += 1;
        match self.attempts >= 2 && self.attempts.is_multiple_of(2) {
            true => {
                self.due = Some(now_ms + (1u64 << (self.attempts / 2)) * 1000);
                None
            }
            false => self.request(),
        }
    }

    fn snapshot(&self) -> Value {
        json!({
            "componentId": self.compid,
            "attempts": self.attempts,
            "waiting": self.due.is_some(),
            "gaveUp": self.due.is_none() && self.attempts >= MAX_INFORMATION_ATTEMPTS,
        })
    }
}

#[derive(Debug, Default)]
pub struct Cameras {
    tracked: Vec<Tracked>,
    cameras: Vec<Camera>,
    selected: Option<u8>,
}

impl Cameras {
    pub fn new() -> Cameras {
        Cameras::default()
    }

    pub fn count(&self) -> usize {
        self.cameras.len()
    }

    pub fn selected_index(&self) -> Option<usize> {
        let chosen = self.chosen_compid()?;
        self.cameras.iter().position(|camera| camera.compid == chosen)
    }

    pub fn selected(&self) -> Option<&Camera> {
        let chosen = self.chosen_compid()?;
        self.camera(chosen)
    }

    pub fn camera(&self, compid: u8) -> Option<&Camera> {
        self.cameras.iter().find(|camera| camera.compid == compid)
    }

    fn camera_mut(&mut self, compid: u8) -> Option<&mut Camera> {
        self.cameras.iter_mut().find(|camera| camera.compid == compid)
    }

    fn chosen_compid(&self) -> Option<u8> {
        self.selected
            .filter(|compid| self.cameras.iter().any(|camera| camera.compid == *compid))
            .or_else(|| self.cameras.first().map(|camera| camera.compid))
    }

    fn chosen_mut(&mut self) -> Result<&mut Camera, Refusal> {
        let chosen = self.chosen_compid().ok_or(Refusal::NoCamera)?;
        self.camera_mut(chosen).ok_or(Refusal::NoCamera)
    }

    fn act(&mut self, action: Action) -> Result<&mut Camera, Refusal> {
        let camera = self.chosen_mut()?;
        match camera.gate(action) {
            Some(refusal) => Err(refusal),
            None => Ok(camera),
        }
    }

    pub fn select(&mut self, compid: u8) -> Result<(), Refusal> {
        match self.cameras.iter().any(|camera| camera.compid == compid) {
            true => {
                self.selected = Some(compid);
                Ok(())
            }
            false => Err(Refusal::NoCamera),
        }
    }

    pub fn on_heartbeat(&mut self, compid: u8, now_ms: u64) -> Vec<Command> {
        if !is_camera_component(compid) {
            return Vec::new();
        }
        if let Some(camera) = self.camera_mut(compid) {
            camera.last_heartbeat_ms = now_ms;
        }
        let Some(tracked) = self.tracked.iter_mut().find(|tracked| tracked.compid == compid) else {
            self.tracked.push(Tracked { compid, last_heartbeat_ms: now_ms, attempts: 0, due: None, identified: false });
            return self.tracked.last_mut().and_then(Tracked::request).into_iter().collect();
        };
        let silent = now_ms.saturating_sub(tracked.last_heartbeat_ms) > SILENT_TIMEOUT_MS;
        tracked.last_heartbeat_ms = now_ms;
        if tracked.identified || !silent {
            return Vec::new();
        }
        tracked.attempts = 0;
        tracked.due = None;
        tracked.request().into_iter().collect()
    }

    pub fn on_camera_information(&mut self, compid: u8, info: Info, now_ms: u64) {
        let Some(tracked) = self.tracked.iter_mut().find(|tracked| tracked.compid == compid) else {
            return;
        };
        if tracked.identified {
            return;
        }
        tracked.identified = true;
        tracked.attempts = 0;
        tracked.due = None;
        tracked.last_heartbeat_ms = now_ms;
        self.cameras.push(Camera::new(compid, info, now_ms));
    }

    pub fn on_definition_known(&mut self, compid: u8, has_parameters: bool, has_mode_parameter: bool) {
        if let Some(camera) = self.camera_mut(compid) {
            camera.parameters_expected = Some(has_parameters);
            camera.mode_is_parameter = has_parameters && has_mode_parameter;
        }
    }

    pub fn on_camera_settings(&mut self, compid: u8, report: SettingsReport, now_ms: u64) {
        if let Some(camera) = self.camera_mut(compid) {
            camera.on_settings(report, now_ms);
        }
    }

    pub fn on_storage_information(&mut self, compid: u8, report: StorageReport, now_ms: u64) {
        if let Some(camera) = self.camera_mut(compid) {
            camera.on_storage(report, now_ms);
        }
    }

    pub fn on_capture_status(&mut self, compid: u8, report: CaptureStatusReport, now_ms: u64) {
        if let Some(camera) = self.camera_mut(compid) {
            camera.on_capture_status(report, now_ms);
        }
    }

    pub fn on_battery_status(&mut self, compid: u8, remaining_percent: i8, now_ms: u64) {
        if let Some(camera) = self.camera_mut(compid) {
            let known = (remaining_percent >= 0).then_some(remaining_percent);
            camera.battery_at_ms = known.map(|_| now_ms).or(camera.battery_at_ms);
            camera.battery_percent = known.or(camera.battery_percent);
        }
    }

    pub fn on_video_stream_information(&mut self, compid: u8, report: StreamReport, now_ms: u64) {
        if let Some(camera) = self.camera_mut(compid) {
            camera.on_stream_info(report, now_ms);
        }
    }

    pub fn on_video_stream_status(&mut self, compid: u8, report: StreamStatusReport, now_ms: u64) {
        if let Some(camera) = self.camera_mut(compid) {
            camera.on_stream_status(report, now_ms);
        }
    }

    pub fn on_command_result(&mut self, compid: u8, sent: u16, param1: f64, result: u8, now_ms: u64) -> Vec<Command> {
        let asked_for_information = sent == CMD_REQUEST_CAMERA_INFORMATION || (sent == CMD_REQUEST_MESSAGE && param1 as u32 == MSG_CAMERA_INFORMATION);
        let retried = match (asked_for_information, result) {
            (true, RESULT_ACCEPTED | RESULT_IN_PROGRESS) => None,
            (true, _) => self
                .tracked
                .iter_mut()
                .find(|tracked| tracked.compid == compid && !tracked.identified)
                .and_then(|tracked| tracked.retry(now_ms)),
            (false, _) => None,
        };
        if let Some(camera) = self.camera_mut(compid) {
            camera.on_command_result(sent, result, now_ms);
        }
        retried.into_iter().collect()
    }

    pub fn tick(&mut self, now_ms: u64) -> Vec<Command> {
        let waking: Vec<Command> = self
            .tracked
            .iter_mut()
            .filter(|tracked| !tracked.identified && tracked.due.is_some_and(|due| now_ms >= due))
            .filter_map(Tracked::request)
            .collect();
        let lost: Vec<u8> = self
            .cameras
            .iter()
            .filter(|camera| now_ms.saturating_sub(camera.last_heartbeat_ms) > SILENT_TIMEOUT_MS)
            .map(|camera| camera.compid)
            .collect();
        if !lost.is_empty() {
            self.cameras.retain(|camera| !lost.contains(&camera.compid));
            self.tracked.retain(|tracked| !lost.contains(&tracked.compid));
            self.selected = self.selected.filter(|compid| !lost.contains(compid));
        }
        let requests: Vec<Command> = self.cameras.iter_mut().flat_map(|camera| camera.tick(now_ms)).collect();
        waking.into_iter().chain(requests).collect()
    }

    pub fn set_mode(&mut self, mode: u8, now_ms: u64) -> Result<Vec<Command>, Refusal> {
        let camera = self.act(Action::SetMode)?;
        if mode != MODE_PHOTO && mode != MODE_VIDEO {
            return Err(Refusal::WrongMode);
        }
        if camera.mode_now() == Some(mode) {
            return Ok(Vec::new());
        }
        let sent = command(camera.compid, CMD_SET_CAMERA_MODE, [0.0, f64::from(mode), 0.0, 0.0, 0.0, 0.0, 0.0]);
        camera.command_mode(mode, now_ms);
        camera.note_sent(CMD_SET_CAMERA_MODE, now_ms);
        Ok(vec![sent])
    }

    pub fn take_photo(&mut self, interval_s: Option<f64>, count: u32, now_ms: u64) -> Result<Vec<Command>, Refusal> {
        let camera = self.act(Action::TakePhoto)?;
        let (interval, shots) = match interval_s.and_then(finite_positive) {
            Some(interval) => (interval, f64::from(count)),
            None => (0.0, 1.0),
        };
        let sent = command(camera.compid, CMD_IMAGE_START_CAPTURE, [0.0, interval, shots, 0.0, 0.0, 0.0, 0.0]);
        camera.commanded.photo_status = Some(PHOTO_IN_PROGRESS);
        camera.capture_command_attempts = 0;
        camera.note_sent(CMD_IMAGE_START_CAPTURE, now_ms);
        Ok(vec![sent])
    }

    pub fn stop_take_photo(&mut self, now_ms: u64) -> Result<Vec<Command>, Refusal> {
        let camera = self.act(Action::StopTakePhoto)?;
        let sent = command(camera.compid, CMD_IMAGE_STOP_CAPTURE, [0.0; 7]);
        camera.commanded.photo_status = Some(PHOTO_IDLE);
        camera.capture_command_attempts = 0;
        camera.note_sent(CMD_IMAGE_STOP_CAPTURE, now_ms);
        Ok(vec![sent])
    }

    pub fn start_recording(&mut self, now_ms: u64) -> Result<Vec<Command>, Refusal> {
        let camera = self.act(Action::StartRecording)?;
        let sent = command(camera.compid, CMD_VIDEO_START_CAPTURE, [0.0; 7]);
        camera.commanded.video_status = Some(VIDEO_RUNNING);
        camera.note_sent(CMD_VIDEO_START_CAPTURE, now_ms);
        Ok(vec![sent])
    }

    pub fn stop_recording(&mut self, now_ms: u64) -> Result<Vec<Command>, Refusal> {
        let camera = self.act(Action::StopRecording)?;
        let sent = command(camera.compid, CMD_VIDEO_STOP_CAPTURE, [0.0; 7]);
        camera.commanded.video_status = Some(VIDEO_STOPPED);
        camera.note_sent(CMD_VIDEO_STOP_CAPTURE, now_ms);
        Ok(vec![sent])
    }

    fn axis_command(&mut self, axis: Axis, kind: f64, value: f64, now_ms: u64) -> Result<Vec<Command>, Refusal> {
        let camera = self.act(axis.action())?;
        let sent = command(camera.compid, axis.command(), [kind, value, 0.0, 0.0, 0.0, 0.0, 0.0]);
        camera.note_sent(axis.command(), now_ms);
        Ok(vec![sent])
    }

    pub fn set_level(&mut self, axis: Axis, percent: f64, now_ms: u64) -> Result<Vec<Command>, Refusal> {
        self.axis_command(axis, LEVEL_TYPE_RANGE, percent.clamp(MIN_LEVEL_PERCENT, MAX_LEVEL_PERCENT), now_ms)
    }

    pub fn step(&mut self, axis: Axis, direction: i32, now_ms: u64) -> Result<Vec<Command>, Refusal> {
        self.axis_command(axis, LEVEL_TYPE_STEP, f64::from(direction), now_ms)
    }

    pub fn slew(&mut self, axis: Axis, direction: i32, now_ms: u64) -> Result<Vec<Command>, Refusal> {
        self.axis_command(axis, LEVEL_TYPE_CONTINUOUS, f64::from(direction), now_ms)
    }

    pub fn reset_settings(&mut self, now_ms: u64) -> Result<Vec<Command>, Refusal> {
        let camera = self.act(Action::ResetSettings)?;
        if camera.resetting() {
            return Ok(Vec::new());
        }
        camera.resetting_until = Some(now_ms + RESET_ACK_TIMEOUT_MS);
        camera.note_sent(CMD_RESET_CAMERA_SETTINGS, now_ms);
        Ok(vec![command(camera.compid, CMD_RESET_CAMERA_SETTINGS, [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])])
    }

    pub fn format_storage(&mut self, storage_id: u8, now_ms: u64) -> Result<Vec<Command>, Refusal> {
        let camera = self.act(Action::FormatStorage)?;
        let known = camera.storage_count == 0 || (1..=camera.storage_count).contains(&storage_id);
        if !known {
            return Err(Refusal::UnknownStorage);
        }
        camera.note_sent(CMD_STORAGE_FORMAT, now_ms);
        Ok(vec![command(camera.compid, CMD_STORAGE_FORMAT, [f64::from(storage_id), 1.0, 0.0, 0.0, 0.0, 0.0, 0.0])])
    }

    pub fn select_stream(&mut self, stream_id: u8, now_ms: u64) -> Result<Vec<Command>, Refusal> {
        let camera = self.act(Action::SelectStream)?;
        if !camera.listed_streams().iter().any(|stream| stream.stream_id == stream_id) {
            return Err(Refusal::UnknownStream);
        }
        let showing = camera.current_stream().map(|stream| stream.stream_id);
        if showing == Some(stream_id) {
            camera.selected_stream = Some(stream_id);
            return Ok(Vec::new());
        }
        let stopping = showing.map(|id| command(camera.compid, CMD_VIDEO_STOP_STREAMING, [f64::from(id), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
        camera.selected_stream = Some(stream_id);
        camera.stream_status.settle();
        let starting = command(camera.compid, CMD_VIDEO_START_STREAMING, [f64::from(stream_id), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let asking = camera.request_stream_status(stream_id, now_ms);
        camera.note_sent(CMD_VIDEO_START_STREAMING, now_ms);
        Ok(stopping.into_iter().chain([starting, asking]).collect())
    }

    pub fn snapshot(&self, now_ms: u64) -> Value {
        json!({
            "kind": "object",
            "class": "Cameras",
            "count": self.cameras.len(),
            "selected": self.selected_index(),
            "selectedComponentId": self.chosen_compid(),
            "cameras": self.cameras.iter().map(|camera| camera.snapshot(now_ms)).collect::<Vec<_>>(),
            "discovering": self.tracked.iter().filter(|tracked| !tracked.identified).map(Tracked::snapshot).collect::<Vec<_>>(),
            "staleMs": STALE_MS,
            "storageUnits": STORAGE_UNITS,
            "durationUnits": DURATION_UNITS,
            "angleUnits": ANGLE_UNITS,
            "frameRateUnits": FRAME_RATE_UNITS,
            "bitRateUnits": BIT_RATE_UNITS,
            "lengthUnits": LENGTH_UNITS,
            "resolutionUnits": RESOLUTION_UNITS,
            "levelUnits": LEVEL_UNITS,
            "intervalUnits": INTERVAL_UNITS,
        })
    }
}

pub fn protocol_view(_backend: &dyn crate::router::Backend, _args: &[String]) -> Value {
    json!({
        "kind": "object",
        "class": "CameraProtocol",
        "cameraComponentIds": { "first": COMP_ID_CAMERA, "last": COMP_ID_CAMERA6 },
        "messages": {
            "cameraInformation": MSG_CAMERA_INFORMATION,
            "cameraSettings": MSG_CAMERA_SETTINGS,
            "storageInformation": MSG_STORAGE_INFORMATION,
            "cameraCaptureStatus": MSG_CAMERA_CAPTURE_STATUS,
            "videoStreamInformation": MSG_VIDEO_STREAM_INFORMATION,
            "videoStreamStatus": MSG_VIDEO_STREAM_STATUS,
        },
        "attempts": {
            "cameraInformation": MAX_INFORMATION_ATTEMPTS,
            "request": MAX_REQUEST_ATTEMPTS,
            "videoStreamInformationPerStream": STREAM_ATTEMPTS_PER_STREAM,
        },
        "delays": {
            "heartbeatTick": HEARTBEAT_TICK_MS,
            "silentTimeout": SILENT_TIMEOUT_MS,
            "cameraSettings": SETTINGS_DELAY_MS,
            "videoStreamCheck": STREAM_CHECK_DELAY_MS,
            "captureStatus": CAPTURE_STATUS_DELAY_MS,
            "storageInformation": STORAGE_DELAY_MS,
            "requestTimeout": REQUEST_TIMEOUT_MS,
            "videoStreamDiscovery": STREAM_DISCOVERY_MS,
            "cameraSettingsRefresh": SETTINGS_REFRESH_DELAY_MS,
            "cameraSettingsAfterReset": RESET_SETTINGS_DELAY_MS,
            "resetAckTimeout": RESET_ACK_TIMEOUT_MS,
            "recordingPoll": RECORDING_POLL_MS,
            "busyPoll": BUSY_POLL_MS,
            "modeChangePoll": MODE_CHANGE_POLL_MS,
        },
        "staleMs": STALE_MS,
        "durationUnits": DURATION_UNITS,
        "levelTypes": { "step": LEVEL_TYPE_STEP, "continuous": LEVEL_TYPE_CONTINUOUS, "range": LEVEL_TYPE_RANGE },
        "levelRange": { "min": MIN_LEVEL_PERCENT, "max": MAX_LEVEL_PERCENT },
        "fallbackAspect": FALLBACK_ASPECT,
        "capabilities": CAPABILITIES.iter().map(|(flag, name)| json!({ "flag": flag, "token": name })).collect::<Vec<_>>(),
        "modes": MODES.iter().map(|(code, name)| json!({ "code": code, "token": name })).collect::<Vec<_>>(),
        "photoStatus": PHOTO_STATUS.iter().map(|(code, name)| json!({ "code": code, "token": name })).collect::<Vec<_>>(),
        "videoStatus": VIDEO_STATUS.iter().map(|(code, name)| json!({ "code": code, "token": name })).collect::<Vec<_>>(),
        "storageStatus": STORAGE_STATUS.iter().map(|(code, name)| json!({ "code": code, "token": name })).collect::<Vec<_>>(),
        "streamTypes": STREAM_TYPES.iter().map(|(code, name)| json!({ "code": code, "token": name })).collect::<Vec<_>>(),
        "results": RESULTS.iter().map(|(code, name)| json!({ "code": code, "token": name })).collect::<Vec<_>>(),
        "actions": ACTIONS.iter().map(|action| action.token()).collect::<Vec<_>>(),
        "refusals": REFUSALS.iter().map(|refusal| refusal.token()).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::Backend;

    const CAMERA: u8 = COMP_ID_CAMERA;
    const INFORMATION: f64 = MSG_CAMERA_INFORMATION as f64;

    struct Nothing;

    impl Backend for Nothing {
        fn get(&self, _path: &str) -> String {
            json!({ "kind": "null" }).to_string()
        }
        fn get_fields(&self, _path: &str, _fields: &str) -> String {
            json!({ "kind": "null" }).to_string()
        }
        fn set(&self, _path: &str, _value: &str) -> String {
            json!({ "ok": false }).to_string()
        }
        fn invoke(&self, _path: &str, _args: &str) -> String {
            json!({ "ok": false }).to_string()
        }
        fn watch(&self, _paths: &[String]) {}
    }

    fn info(flags: u32) -> Info {
        Info {
            vendor: "Auterion".into(),
            model: "Skynode".into(),
            firmware_version: 0x0004_0302,
            focal_length_mm: 4.5,
            sensor_size_h_mm: 6.17,
            sensor_size_v_mm: 4.55,
            resolution_h: 1920,
            resolution_v: 1080,
            flags,
            definition_version: 3,
            definition_uri: "mftp://camera.xml".into(),
            gimbal_device_id: 0,
        }
    }

    fn discovered(flags: u32) -> Cameras {
        let mut cameras = Cameras::new();
        cameras.on_heartbeat(CAMERA, 0);
        cameras.on_camera_information(CAMERA, info(flags), 0);
        cameras
    }

    fn commands(sent: &[Command]) -> Vec<(u16, f64)> {
        sent.iter().map(|command| (command.command, command.params[0])).collect()
    }

    fn quiet(flags: u32) -> Cameras {
        let mut cameras = discovered(flags);
        if let Some(camera) = cameras.camera_mut(CAMERA) {
            camera.settings.settle();
            camera.storage_request.settle();
            camera.capture.settle();
        }
        cameras
    }

    fn idle(flags: u32) -> Cameras {
        let mut cameras = quiet(flags);
        cameras.on_capture_status(CAMERA, CaptureStatusReport::default(), 0);
        cameras
    }

    fn at(cameras: &mut Cameras, now_ms: u64) -> Vec<Command> {
        cameras.on_heartbeat(CAMERA, now_ms);
        cameras.tick(now_ms)
    }

    fn ticks(cameras: &mut Cameras, from_ms: u64, rounds: u64, step_ms: u64) -> Vec<Command> {
        (0..rounds).fold(Vec::new(), |seen: Vec<Command>, round| seen.into_iter().chain(at(cameras, from_ms + round * step_ms)).collect())
    }

    fn about(sent: &[Command], message: u32, legacy: u16) -> Vec<Command> {
        sent.iter()
            .filter(|command| (command.command == CMD_REQUEST_MESSAGE && command.params[0] == message as f64) || command.command == legacy)
            .cloned()
            .collect()
    }

    #[test]
    fn camera_information_is_asked_for_alternately_backs_off_by_powers_of_two_and_stops_at_ten() {
        let mut cameras = Cameras::new();
        assert!(cameras.on_heartbeat(1, 0).is_empty(), "an autopilot heartbeat is not a camera and must never start a discovery");
        assert_eq!(commands(&cameras.on_heartbeat(CAMERA, 0)), vec![(CMD_REQUEST_MESSAGE, INFORMATION)]);
        let second = cameras.on_command_result(CAMERA, CMD_REQUEST_MESSAGE, INFORMATION, RESULT_FAILED, 10);
        assert_eq!(commands(&second), vec![(CMD_REQUEST_CAMERA_INFORMATION, 1.0)], "the odd attempt is the legacy command, the same alternation the C++ does");
        let third = cameras.on_command_result(CAMERA, CMD_REQUEST_CAMERA_INFORMATION, 1.0, RESULT_FAILED, 20);
        assert!(third.is_empty(), "attempt two is even, so it waits out a backoff instead of asking again straight away");
        assert!(cameras.tick(20 + 1999).is_empty(), "the backoff for attempt two is one shifted by one seconds, not a millisecond less");
        assert_eq!(commands(&cameras.tick(20 + 2000)), vec![(CMD_REQUEST_MESSAGE, INFORMATION)]);
        let rest = (0..30).fold(Vec::new(), |seen: Vec<Command>, round| {
            let now = 10_000 + round * 1_000_000;
            let answered = cameras.on_command_result(CAMERA, CMD_REQUEST_MESSAGE, INFORMATION, RESULT_FAILED, now);
            let answered = match answered.is_empty() {
                true => cameras.tick(now + 999_999),
                false => answered,
            };
            seen.into_iter().chain(answered).collect()
        });
        assert_eq!(3 + rest.len(), MAX_INFORMATION_ATTEMPTS as usize, "ten attempts is the whole budget, counting the two alternating shapes together");
        assert_eq!(cameras.snapshot(0)["discovering"][0]["gaveUp"], true);
        let woken = cameras.on_heartbeat(CAMERA, 40_000_000);
        assert_eq!(commands(&woken), vec![(CMD_REQUEST_MESSAGE, INFORMATION)], "a camera heard from again after five silent seconds clears the spent budget, which is what keeps it from being a latch");
    }

    #[test]
    fn identifying_a_camera_schedules_the_first_requests_at_the_c_plus_plus_offsets() {
        let mut cameras = discovered(CAP_HAS_VIDEO_STREAM);
        assert_eq!(cameras.count(), 1);
        assert!(cameras.tick(SETTINGS_DELAY_MS - 1).is_empty(), "nothing is due before the first offset");
        assert_eq!(commands(&cameras.tick(SETTINGS_DELAY_MS)), vec![(CMD_REQUEST_MESSAGE, MSG_CAMERA_SETTINGS as f64)], "the first ask is the modern request, not the legacy one");
        assert_eq!(commands(&cameras.tick(STREAM_CHECK_DELAY_MS)), vec![(CMD_REQUEST_MESSAGE, MSG_VIDEO_STREAM_INFORMATION as f64)]);
        let later = commands(&cameras.tick(STORAGE_DELAY_MS));
        assert!(later.contains(&(CMD_REQUEST_MESSAGE, MSG_CAMERA_CAPTURE_STATUS as f64)) && later.contains(&(CMD_REQUEST_MESSAGE, MSG_STORAGE_INFORMATION as f64)));
        cameras.on_camera_information(CAMERA, info(0), STORAGE_DELAY_MS);
        assert_eq!(cameras.count(), 1, "a second CAMERA_INFORMATION from a camera already known is ignored, never a duplicate camera");
        let mut plain = discovered(0);
        assert!(!commands(&ticks(&mut plain, 0, 10, 1000)).iter().any(|(_, param)| *param == MSG_VIDEO_STREAM_INFORMATION as f64), "a camera that claims no video stream is never asked about streams");
    }

    #[test]
    fn a_camera_that_stops_beating_is_dropped_and_the_selection_follows_the_camera_not_its_position() {
        let mut cameras = Cameras::new();
        cameras.on_heartbeat(CAMERA, 0);
        cameras.on_camera_information(CAMERA, info(0), 0);
        cameras.on_heartbeat(CAMERA + 1, 0);
        cameras.on_camera_information(CAMERA + 1, info(0), 0);
        cameras.on_heartbeat(CAMERA + 2, 0);
        cameras.on_camera_information(CAMERA + 2, info(0), 0);
        assert_eq!(cameras.select(CAMERA + 2), Ok(()));
        cameras.on_heartbeat(CAMERA + 1, SILENT_TIMEOUT_MS);
        cameras.on_heartbeat(CAMERA + 2, SILENT_TIMEOUT_MS);
        cameras.tick(SILENT_TIMEOUT_MS + 1);
        assert_eq!(cameras.count(), 2, "the first camera went five seconds without a heartbeat and is gone");
        assert_eq!(
            cameras.selected().map(|camera| camera.compid),
            Some(CAMERA + 2),
            "a camera dropping out of the middle of the list must not move the operator onto a different camera, which is what an index selection does"
        );
        assert_eq!(cameras.selected_index(), Some(1));
        assert_eq!(cameras.select(4), Err(Refusal::NoCamera));
        cameras.on_heartbeat(CAMERA + 1, SILENT_TIMEOUT_MS + 2);
        cameras.tick(2 * SILENT_TIMEOUT_MS + 2);
        assert_eq!(cameras.selected().map(|camera| camera.compid), Some(CAMERA + 1), "when the selected camera is the one that left, the survivor takes over");
        let rediscovered = cameras.on_heartbeat(CAMERA, 2 * SILENT_TIMEOUT_MS + 3);
        assert_eq!(commands(&rediscovered), vec![(CMD_REQUEST_MESSAGE, INFORMATION)], "a dropped camera is discovered again from scratch when it comes back");
    }

    #[test]
    fn no_data_zero_data_and_nonsense_data_stay_three_different_answers() {
        let mut cameras = discovered(CAP_CAPTURE_VIDEO | CAP_CAPTURE_IMAGE);
        let camera = cameras.snapshot(0)["cameras"][0].clone();
        assert!(camera["videoStatus"].is_null() && camera["photoStatus"].is_null(), "a camera that has reported no capture status yet must not read as idle");
        assert!(camera["storage"].is_null() && camera["battery"].is_null() && camera["recordTime"].is_null());
        cameras.on_capture_status(CAMERA, CaptureStatusReport::default(), 100);
        let reported = cameras.snapshot(100)["cameras"][0].clone();
        assert_eq!(reported["videoStatus"], "stopped");
        assert_eq!(reported["storage"]["free"], 0.0, "a camera reporting a full card says zero, and zero is not the answer silence gives");
        assert_eq!(reported["storage"]["slots"], json!([]), "a capture status carries free space but says nothing about the card, so no card is described");
        assert!(reported["imageInterval"].is_null(), "an interval of zero seconds is no interval at all");
        cameras.on_capture_status(CAMERA, CaptureStatusReport { image_status: 9, video_status: 7, ..Default::default() }, 200);
        let nonsense = cameras.snapshot(200)["cameras"][0].clone();
        assert_eq!((nonsense["videoStatus"].as_str(), nonsense["photoStatus"].as_str()), (Some("undefined"), Some("undefined")), "codes out of range are clamped to undefined, never to idle");
        cameras.on_battery_status(CAMERA, -1, 200);
        assert!(cameras.snapshot(200)["cameras"][0]["battery"].is_null(), "a negative battery reading means unknown and must not land as a number");
        cameras.on_battery_status(CAMERA, 40, 200);
        assert_eq!(cameras.snapshot(200)["cameras"][0]["battery"], 40);
    }

    #[test]
    fn a_capacity_the_camera_does_not_know_is_never_written_over_one_it_measured() {
        let mut cameras = quiet(CAP_CAPTURE_VIDEO);
        cameras.on_storage_information(CAMERA, StorageReport { storage_id: 1, storage_count: 1, status: STORAGE_READY, total_capacity_mib: 61_000.0, available_capacity_mib: 42_000.0 }, 0);
        cameras.on_capture_status(CAMERA, CaptureStatusReport { available_capacity_mib: f64::NAN, ..Default::default() }, 1_000);
        let after = cameras.snapshot(1_000)["cameras"][0]["storage"].clone();
        assert_eq!(
            after["free"].as_f64(),
            Some(42_000.0),
            "a capture status whose available capacity is not a number means the camera does not know, and it must not evict the measurement the storage message gave"
        );
        assert_eq!(
            after["ageMs"], 1_000,
            "the age published is the age of the measurement the head is drawing, not the age of the last poll that carried no number"
        );
        let mut blind = quiet(CAP_CAPTURE_VIDEO);
        blind.on_capture_status(CAMERA, CaptureStatusReport { available_capacity_mib: f64::NAN, ..Default::default() }, 0);
        assert!(
            blind.snapshot(0)["cameras"][0]["storage"].is_null(),
            "a camera that has only ever said it does not know how much room is left has described no card, so no card block is drawn"
        );
    }

    #[test]
    fn every_reading_the_operator_acts_on_carries_its_age_and_says_when_it_went_stale() {
        let mut cameras = quiet(CAP_CAPTURE_VIDEO | CAP_HAS_MODES | CAP_HAS_VIDEO_STREAM);
        let blind = cameras.snapshot(0)["cameras"][0].clone();
        assert_eq!(
            (blind["captureStale"].as_bool(), blind["settingsStale"].as_bool(), blind["captureAgeMs"].as_u64()),
            (Some(true), Some(true), None),
            "a camera that has never reported reads as stale with no age, never as a fresh reading"
        );
        cameras.on_capture_status(CAMERA, CaptureStatusReport { available_capacity_mib: 42_000.0, ..Default::default() }, 1_000);
        cameras.on_camera_settings(CAMERA, SettingsReport { mode_id: MODE_VIDEO, zoom_percent: 5.0, focus_percent: 5.0 }, 1_000);
        cameras.on_video_stream_information(CAMERA, StreamReport { stream_id: 1, count: 1, framerate_hz: 30.0, ..Default::default() }, 1_000);
        let fresh = cameras.snapshot(1_000)["cameras"][0].clone();
        assert_eq!((fresh["captureAgeMs"].as_u64(), fresh["settingsAgeMs"].as_u64(), fresh["streams"][0]["ageMs"].as_u64()), (Some(0), Some(0), Some(0)));
        assert!(!fresh["captureStale"].as_bool().unwrap() && !fresh["settingsStale"].as_bool().unwrap() && !fresh["streams"][0]["stale"].as_bool().unwrap());
        let old = cameras.snapshot(1_000 + STALE_MS + 1)["cameras"][0].clone();
        assert_eq!(
            (old["captureStale"].as_bool(), old["settingsStale"].as_bool(), old["storage"]["stale"].as_bool(), old["streams"][0]["stale"].as_bool()),
            (Some(true), Some(true), Some(true), Some(true)),
            "capture status stops being polled once the camera is idle, so a minutes-old free space figure must not look like a live one"
        );
        assert_eq!(old["captureAgeMs"], json!(STALE_MS + 1));
        assert_eq!(cameras.snapshot(0)["staleMs"], json!(STALE_MS), "the threshold is named in the payload so two heads cannot pick two of them");
    }

    #[test]
    fn the_recording_clock_resyncs_to_the_camera_and_clears_when_recording_stops() {
        let mut cameras = quiet(CAP_CAPTURE_VIDEO);
        cameras.on_command_result(CAMERA, CMD_VIDEO_START_CAPTURE, 0.0, RESULT_ACCEPTED, 100_000);
        assert_eq!(cameras.camera(CAMERA).unwrap().record_time_ms(103_000), Some(3_000));
        cameras.on_capture_status(CAMERA, CaptureStatusReport { video_status: VIDEO_RUNNING, recording_time_ms: 30_000, ..Default::default() }, 104_000);
        assert_eq!(cameras.camera(CAMERA).unwrap().record_time_ms(104_000), Some(30_000), "the camera's own elapsed time wins over the stamp we kept");
        cameras.on_capture_status(CAMERA, CaptureStatusReport { video_status: VIDEO_STOPPED, ..Default::default() }, 105_000);
        assert_eq!(cameras.camera(CAMERA).unwrap().record_time_ms(105_000), None, "the clock is cleared, not frozen at its last reading");
        assert!(cameras.snapshot(105_000)["cameras"][0]["recordTime"].is_null());
    }

    #[test]
    fn capture_status_polling_follows_what_the_camera_is_doing_and_stops_when_it_is_idle() {
        let mut cameras = quiet(CAP_CAPTURE_VIDEO | CAP_CAPTURE_IMAGE);
        cameras.on_capture_status(CAMERA, CaptureStatusReport { video_status: VIDEO_RUNNING, ..Default::default() }, 0);
        assert!(at(&mut cameras, RECORDING_POLL_MS - 1).is_empty());
        assert_eq!(commands(&at(&mut cameras, RECORDING_POLL_MS)).len(), 1, "a recording camera is asked again after five seconds");
        cameras.on_capture_status(CAMERA, CaptureStatusReport { image_status: PHOTO_IN_PROGRESS, ..Default::default() }, RECORDING_POLL_MS);
        assert_eq!(commands(&at(&mut cameras, RECORDING_POLL_MS + BUSY_POLL_MS)).len(), 1, "a busy shutter is asked again after a second");
        cameras.on_capture_status(CAMERA, CaptureStatusReport { image_status: PHOTO_INTERVAL_IN_PROGRESS, ..Default::default() }, 10_000);
        assert!(
            ticks(&mut cameras, 11_000, 4, BUSY_POLL_MS).is_empty(),
            "a time lapse can run for an hour, so it is polled at the recording cadence rather than once a second for its whole length"
        );
        assert_eq!(commands(&at(&mut cameras, 10_000 + RECORDING_POLL_MS)).len(), 1);
        cameras.on_capture_status(CAMERA, CaptureStatusReport::default(), 20_000);
        assert!(ticks(&mut cameras, 21_000, 60, 1_000).is_empty(), "an idle camera is left alone, so none of this is a poll that never stops");
    }

    #[test]
    fn a_capture_status_request_that_is_never_answered_is_asked_again_and_says_it_is_waiting() {
        let mut cameras = discovered(CAP_CAPTURE_IMAGE);
        let first = about(&at(&mut cameras, CAPTURE_STATUS_DELAY_MS), MSG_CAMERA_CAPTURE_STATUS, CMD_REQUEST_CAMERA_CAPTURE_STATUS);
        assert_eq!(commands(&first), vec![(CMD_REQUEST_MESSAGE, MSG_CAMERA_CAPTURE_STATUS as f64)]);
        assert_eq!(
            cameras.snapshot(CAPTURE_STATUS_DELAY_MS)["cameras"][0]["requests"]["captureStatus"],
            json!({ "attempts": 0, "state": "waiting" }),
            "a request in flight has to read as waiting, or a head cannot tell it from one that was abandoned"
        );
        let asked = about(&ticks(&mut cameras, CAPTURE_STATUS_DELAY_MS + REQUEST_TIMEOUT_MS, 12, REQUEST_TIMEOUT_MS), MSG_CAMERA_CAPTURE_STATUS, CMD_REQUEST_CAMERA_CAPTURE_STATUS);
        assert_eq!(
            asked.len(),
            MAX_REQUEST_ATTEMPTS as usize,
            "a camera that silently drops the first capture status request is asked again up to the budget, not left with a null shutter state for the whole flight"
        );
        assert_eq!(cameras.snapshot(60_000)["cameras"][0]["requests"]["captureStatus"]["state"], "gaveUp", "and once the budget is spent it says so instead of looking settled");
        cameras.on_capture_status(CAMERA, CaptureStatusReport::default(), 60_000);
        assert_eq!(cameras.snapshot(60_000)["cameras"][0]["requests"]["captureStatus"], json!({ "attempts": 0, "state": "answered" }));
    }

    #[test]
    fn the_capture_guards_answer_from_what_the_camera_reports_not_from_what_a_head_could_manage() {
        let mut streaming = idle(CAP_HAS_VIDEO_STREAM | CAP_HAS_MODES);
        assert_eq!(streaming.take_photo(None, 1, 0), Err(Refusal::Unsupported), "a video stream means a head could grab a frame, and that is the head's business, never a camera capability");
        assert_eq!(streaming.start_recording(0), Err(Refusal::Unsupported), "and the same for saving a stream to disk");
        let mut camera = idle(CAP_CAPTURE_IMAGE | CAP_CAPTURE_VIDEO | CAP_HAS_MODES);
        assert_eq!(commands(&camera.set_mode(MODE_VIDEO, 0).unwrap()), vec![(CMD_SET_CAMERA_MODE, 0.0)]);
        assert!(camera.set_mode(MODE_VIDEO, 0).unwrap().is_empty(), "the mode it is already in is not sent twice");
        assert_eq!(camera.set_mode(MODE_SURVEY, 0), Err(Refusal::WrongMode));
        assert_eq!(camera.take_photo(None, 1, 0), Err(Refusal::WrongMode), "a camera in video mode that does not claim stills in video mode cannot take one");
        assert_eq!(idle(CAP_CAPTURE_IMAGE).set_mode(MODE_VIDEO, 0), Err(Refusal::Unsupported));
        let mut both = idle(CAP_CAPTURE_IMAGE | CAP_IMAGE_IN_VIDEO_MODE | CAP_HAS_MODES);
        both.set_mode(MODE_VIDEO, 0).unwrap();
        let single = both.take_photo(None, 1, 0).unwrap();
        assert_eq!(single[0].params, [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0], "a single shot is one image with no interval");
        assert_eq!(both.take_photo(None, 1, 0), Err(Refusal::Busy));
        assert_eq!(both.stop_take_photo(0), Err(Refusal::NotCapturing), "stopping applies to an interval, and a single shot is not one");
        both.on_capture_status(CAMERA, CaptureStatusReport { image_status: PHOTO_INTERVAL_IN_PROGRESS, ..Default::default() }, 10);
        assert_eq!(commands(&both.stop_take_photo(10).unwrap()), vec![(CMD_IMAGE_STOP_CAPTURE, 0.0)]);
        let lapse = both.take_photo(Some(2.5), 6, 20).unwrap();
        assert_eq!(lapse[0].params[1..3], [2.5, 6.0]);
        assert_eq!(Cameras::new().take_photo(None, 1, 0), Err(Refusal::NoCamera));
    }

    #[test]
    fn a_camera_that_refuses_a_command_takes_the_commanded_state_back_and_says_what_it_refused() {
        let mut cameras = idle(CAP_HAS_MODES | CAP_CAPTURE_IMAGE | CAP_CAPTURE_VIDEO | CAP_IMAGE_IN_VIDEO_MODE | CAP_VIDEO_IN_IMAGE_MODE);
        cameras.on_camera_settings(CAMERA, SettingsReport { mode_id: MODE_PHOTO, zoom_percent: f64::NAN, focus_percent: f64::NAN }, 0);
        cameras.set_mode(MODE_VIDEO, 100).unwrap();
        let pressed = cameras.snapshot(100)["cameras"][0].clone();
        assert_eq!(
            (pressed["mode"].as_str(), pressed["commandedMode"].as_str()),
            (Some("photo"), Some("video")),
            "a mode that was asked for is not a mode the camera reported, and the snapshot has to keep the two apart"
        );
        assert_eq!(pressed["lastCommand"], json!({ "command": CMD_SET_CAMERA_MODE, "result": Value::Null, "pending": true, "ageMs": 0 }));
        cameras.on_command_result(CAMERA, CMD_SET_CAMERA_MODE, 0.0, RESULT_DENIED, 200);
        let denied = cameras.snapshot(200)["cameras"][0].clone();
        assert_eq!(
            (denied["mode"].as_str(), denied["commandedMode"].as_str()),
            (Some("photo"), None),
            "the camera said no, so the mode goes back to the one the camera last reported instead of reading as video for the rest of the flight"
        );
        assert_eq!(denied["lastCommand"]["result"], "denied", "and the refusal itself is published, which is the only way an operator learns the camera said no");
        cameras.start_recording(300).unwrap();
        assert_eq!(
            cameras.snapshot(300)["cameras"][0]["commandedVideoStatus"],
            "running",
            "the seconds between the press and the answer are a state of their own, not silence"
        );
        cameras.on_command_result(CAMERA, CMD_VIDEO_START_CAPTURE, 0.0, RESULT_UNSUPPORTED, 400);
        let refused = cameras.snapshot(400)["cameras"][0].clone();
        assert!(refused["commandedVideoStatus"].is_null() && refused["videoStatus"] == "stopped", "an unsupported record command leaves the camera where the camera says it is");
        assert!(!cameras.camera(CAMERA).unwrap().recording());
        cameras.take_photo(None, 1, 500).unwrap();
        cameras.on_command_result(CAMERA, CMD_IMAGE_START_CAPTURE, 0.0, RESULT_DENIED, 600);
        assert!(cameras.snapshot(600)["cameras"][0]["commandedPhotoStatus"].is_null(), "a denied shutter is not a shutter left reading as busy forever");
        assert_eq!(cameras.snapshot(600)["cameras"][0]["actions"]["takePhoto"], json!({ "offer": "ready", "refusal": "" }));
    }

    #[test]
    fn every_control_a_head_can_draw_publishes_whether_it_is_offered_and_why_not() {
        let mut cameras = idle(CAP_CAPTURE_IMAGE | CAP_HAS_MODES);
        let actions = cameras.snapshot(0)["cameras"][0]["actions"].clone();
        assert_eq!(actions["takePhoto"], json!({ "offer": "ready", "refusal": "" }));
        assert_eq!(actions["startRecording"], json!({ "offer": "blocked", "refusal": "unsupported" }), "a camera that cannot record says so before the operator presses record");
        assert_eq!(actions["zoom"], json!({ "offer": "blocked", "refusal": "unsupported" }));
        assert_eq!(actions["stopTakePhoto"], json!({ "offer": "blocked", "refusal": "notCapturing" }));
        assert_eq!(actions["selectStream"], json!({ "offer": "blocked", "refusal": "unknownStream" }));
        cameras.set_mode(MODE_VIDEO, 0).unwrap();
        assert_eq!(
            cameras.snapshot(0)["cameras"][0]["actions"]["takePhoto"],
            json!({ "offer": "blocked", "refusal": "wrongMode" }),
            "the mode and capability interaction is worked out here, so two heads cannot each invent their own version of it"
        );
        assert_eq!(cameras.take_photo(None, 1, 0), Err(Refusal::WrongMode), "and the offer is the same predicate the command itself uses, so the two cannot drift");
        let refusals: Vec<String> = ACTIONS
            .iter()
            .map(|action| cameras.snapshot(0)["cameras"][0]["actions"][action.token()]["offer"].as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(refusals.len(), ACTIONS.len(), "every action the module can refuse is named in the snapshot");
    }

    #[test]
    fn recording_starts_and_stops_from_the_state_the_camera_reports() {
        let mut cameras = idle(CAP_CAPTURE_VIDEO | CAP_HAS_MODES);
        assert_eq!(cameras.stop_recording(0), Err(Refusal::NotCapturing), "a camera that reports itself stopped is already where the operator wants it");
        cameras.set_mode(MODE_PHOTO, 0).unwrap();
        assert_eq!(cameras.start_recording(0), Err(Refusal::WrongMode), "a camera in photo mode that does not claim video in photo mode cannot record");
        cameras.set_mode(MODE_VIDEO, 0).unwrap();
        assert_eq!(commands(&cameras.start_recording(0).unwrap()), vec![(CMD_VIDEO_START_CAPTURE, 0.0)]);
        cameras.on_command_result(CAMERA, CMD_VIDEO_START_CAPTURE, 0.0, RESULT_ACCEPTED, 100);
        assert_eq!(cameras.start_recording(100), Err(Refusal::Busy));
        assert_eq!(commands(&cameras.stop_recording(100).unwrap()), vec![(CMD_VIDEO_STOP_CAPTURE, 0.0)]);
        cameras.on_command_result(CAMERA, CMD_VIDEO_STOP_CAPTURE, 0.0, RESULT_ACCEPTED, 200);
        assert!(!cameras.camera(CAMERA).unwrap().recording() && cameras.camera(CAMERA).unwrap().record_time_ms(300).is_none());
    }

    #[test]
    fn a_camera_whose_recording_state_is_unknown_is_still_stoppable() {
        let mut cameras = quiet(CAP_CAPTURE_VIDEO);
        assert_eq!(cameras.snapshot(0)["cameras"][0]["actions"]["stopRecording"], json!({ "offer": "ready", "refusal": "" }));
        assert_eq!(
            commands(&cameras.stop_recording(0).unwrap()),
            vec![(CMD_VIDEO_STOP_CAPTURE, 0.0)],
            "a camera already recording when the link came up, or one whose capture status never arrived, has to be stoppable - the camera is the authority on whether stopping is a no-op"
        );
        assert_eq!(
            cameras.snapshot(0)["cameras"][0]["actions"]["stopRecording"],
            json!({ "offer": "blocked", "refusal": "notCapturing" }),
            "and once the stop has been asked for, pressing it again is not offered until the camera says otherwise"
        );
    }

    #[test]
    fn a_failed_capture_command_retries_five_times_and_then_admits_the_shutter_is_idle() {
        let mut cameras = idle(CAP_CAPTURE_IMAGE);
        cameras.take_photo(None, 1, 0).unwrap();
        let asked = (1..=MAX_REQUEST_ATTEMPTS).fold(Vec::new(), |seen: Vec<Command>, attempt| {
            let now = u64::from(attempt) * 10_000;
            cameras.on_command_result(CAMERA, CMD_IMAGE_START_CAPTURE, 0.0, RESULT_TEMPORARILY_REJECTED, now);
            seen.into_iter().chain(at(&mut cameras, now + BUSY_POLL_MS)).collect()
        });
        assert_eq!(asked.len(), MAX_REQUEST_ATTEMPTS as usize, "five retries, the C++ number, no rounding");
        assert_eq!(cameras.camera(CAMERA).unwrap().photo_status_now(), Some(PHOTO_IN_PROGRESS));
        cameras.on_command_result(CAMERA, CMD_IMAGE_START_CAPTURE, 0.0, RESULT_FAILED, 100_000);
        assert_eq!(cameras.camera(CAMERA).unwrap().photo_status_now(), Some(PHOTO_IDLE), "the failure past the budget clears the in progress state, which nothing else here would clear");
    }

    #[test]
    fn a_settings_request_gives_up_after_five_and_an_accepted_zoom_starts_a_fresh_cycle() {
        let mut cameras = discovered(CAP_HAS_BASIC_ZOOM);
        let asked = commands(&ticks(&mut cameras, SETTINGS_DELAY_MS, 10, REQUEST_TIMEOUT_MS));
        let settings: Vec<&(u16, f64)> = asked.iter().filter(|(sent, param)| *sent == CMD_REQUEST_CAMERA_SETTINGS || *param == MSG_CAMERA_SETTINGS as f64).collect();
        assert_eq!(settings.len(), (MAX_REQUEST_ATTEMPTS + 1) as usize, "one first ask plus five retries, and then it stops");
        assert!(settings.iter().any(|(sent, _)| *sent == CMD_REQUEST_CAMERA_SETTINGS), "the retries alternate onto the legacy command");
        let zoom = cameras.set_level(Axis::Zoom, 250.0, 15_000).unwrap();
        assert_eq!(zoom[0].params[0..2], [LEVEL_TYPE_RANGE, MAX_LEVEL_PERCENT], "a level is clamped to the range the protocol defines");
        cameras.on_command_result(CAMERA, CMD_SET_CAMERA_ZOOM, LEVEL_TYPE_RANGE, RESULT_ACCEPTED, 20_000);
        assert!(at(&mut cameras, 20_000 + SETTINGS_REFRESH_DELAY_MS - 1).is_empty());
        assert_eq!(
            commands(&at(&mut cameras, 20_000 + SETTINGS_REFRESH_DELAY_MS)),
            vec![(CMD_REQUEST_MESSAGE, MSG_CAMERA_SETTINGS as f64)],
            "a camera need not announce a new zoom, so an accepted zoom asks once more and the spent budget is fresh again"
        );
        assert_eq!(cameras.set_level(Axis::Focus, 10.0, 21_000), Err(Refusal::Unsupported));
        assert_eq!(commands(&cameras.slew(Axis::Zoom, 0, 21_000).unwrap()), vec![(CMD_SET_CAMERA_ZOOM, LEVEL_TYPE_CONTINUOUS)], "a continuous zoom with no direction is how the protocol says stop");
        assert_eq!(commands(&cameras.step(Axis::Zoom, -1, 21_000).unwrap()), vec![(CMD_SET_CAMERA_ZOOM, LEVEL_TYPE_STEP)]);
    }

    #[test]
    fn a_camera_settings_message_updates_the_mode_and_keeps_a_level_it_cannot_read() {
        let mut cameras = quiet(CAP_HAS_MODES | CAP_HAS_BASIC_ZOOM);
        cameras.on_camera_settings(CAMERA, SettingsReport { mode_id: MODE_VIDEO, zoom_percent: 40.0, focus_percent: 10.0 }, 0);
        cameras.on_camera_settings(CAMERA, SettingsReport { mode_id: MODE_VIDEO, zoom_percent: f64::NAN, focus_percent: f64::NAN }, 100);
        let camera = cameras.camera(CAMERA).unwrap();
        assert_eq!((camera.mode, camera.zoom_percent, camera.focus_percent), (Some(MODE_VIDEO), Some(40.0), Some(10.0)), "a level reported as not a number leaves the last real reading alone");
        assert!(ticks(&mut cameras, 200, 5, REQUEST_TIMEOUT_MS).is_empty(), "the settings that arrived stopped the asking");
    }

    #[test]
    fn a_camera_whose_mode_lives_in_its_definition_is_not_commanded_over_mavlink() {
        let mut cameras = idle(CAP_HAS_MODES);
        cameras.on_definition_known(CAMERA, true, true);
        assert_eq!(
            cameras.set_mode(MODE_VIDEO, 0),
            Err(Refusal::ModeIsParameter),
            "the C++ sets the CAM_MODE parameter for these cameras and only falls back to the command when there is none, so sending the command here would fight the parameter tree"
        );
        assert_eq!(cameras.snapshot(0)["cameras"][0]["actions"]["setMode"], json!({ "offer": "blocked", "refusal": "modeIsParameter" }));
        assert!(cameras.camera(CAMERA).unwrap().mode_now().is_none(), "and nothing was adopted locally, so the core never disagrees with the parameter");
        let mut plain = idle(CAP_HAS_MODES);
        plain.on_definition_known(CAMERA, true, false);
        assert_eq!(commands(&plain.set_mode(MODE_VIDEO, 0).unwrap()), vec![(CMD_SET_CAMERA_MODE, 0.0)], "a definition with no mode parameter still takes the command path");
    }

    #[test]
    fn the_resetting_guard_is_cleared_by_every_final_answer_a_camera_can_give() {
        [RESULT_ACCEPTED, RESULT_TEMPORARILY_REJECTED, RESULT_FAILED, RESULT_DENIED, RESULT_UNSUPPORTED].iter().for_each(|result| {
            let mut cameras = quiet(CAP_HAS_MODES);
            assert_eq!(commands(&cameras.reset_settings(0).unwrap()), vec![(CMD_RESET_CAMERA_SETTINGS, 1.0)]);
            assert!(cameras.reset_settings(0).unwrap().is_empty(), "a reset already in flight is not sent twice");
            assert_eq!(cameras.set_mode(MODE_VIDEO, 0), Err(Refusal::Resetting));
            cameras.on_command_result(CAMERA, CMD_RESET_CAMERA_SETTINGS, 1.0, RESULT_IN_PROGRESS, 10);
            assert_eq!(cameras.set_mode(MODE_VIDEO, 10), Err(Refusal::Resetting), "in progress is not an answer yet");
            cameras.on_command_result(CAMERA, CMD_RESET_CAMERA_SETTINGS, 1.0, *result, 20);
            assert!(cameras.set_mode(MODE_VIDEO, 20).is_ok(), "a denied or unsupported reset has to clear the guard too, or the camera stays locked for the rest of the flight");
        });
        let mut waiting = quiet(CAP_HAS_MODES);
        waiting.on_definition_known(CAMERA, true, false);
        waiting.reset_settings(0).unwrap();
        waiting.on_command_result(CAMERA, CMD_RESET_CAMERA_SETTINGS, 1.0, RESULT_ACCEPTED, 0);
        assert!(at(&mut waiting, RESET_SETTINGS_DELAY_MS - 1).is_empty(), "a camera with a definition to reload gets the longer wait before settings are asked for again");
        assert_eq!(commands(&at(&mut waiting, RESET_SETTINGS_DELAY_MS)).len(), 1);
        let mut basic = quiet(CAP_HAS_MODES);
        basic.on_definition_known(CAMERA, false, false);
        basic.reset_settings(0).unwrap();
        basic.on_command_result(CAMERA, CMD_RESET_CAMERA_SETTINGS, 1.0, RESULT_ACCEPTED, 0);
        assert_eq!(commands(&at(&mut basic, 0)).len(), 1, "a camera with no parameters to reload is asked for its settings straight away");
        let mut undeclared = quiet(CAP_HAS_MODES);
        undeclared.reset_settings(0).unwrap();
        undeclared.on_command_result(CAMERA, CMD_RESET_CAMERA_SETTINGS, 1.0, RESULT_ACCEPTED, 0);
        assert!(
            at(&mut undeclared, RESET_SETTINGS_DELAY_MS - 1).is_empty(),
            "a camera naming a definition file expects parameters whether or not a host has said so, so the longer wait is right before the hook is ever called"
        );
    }

    #[test]
    fn a_reset_whose_answer_never_comes_back_unlocks_itself() {
        let mut cameras = quiet(CAP_HAS_MODES | CAP_CAPTURE_IMAGE);
        cameras.reset_settings(0).unwrap();
        assert_eq!(cameras.set_mode(MODE_VIDEO, 0), Err(Refusal::Resetting));
        at(&mut cameras, RESET_ACK_TIMEOUT_MS - 1);
        assert_eq!(cameras.take_photo(None, 1, RESET_ACK_TIMEOUT_MS - 1), Err(Refusal::Resetting), "the guard holds while an answer could still arrive");
        at(&mut cameras, RESET_ACK_TIMEOUT_MS);
        assert!(
            !cameras.camera(CAMERA).unwrap().resetting(),
            "one ack lost on a packet radio link must not lock every camera command for the rest of the flight - the guard expires because this module owns a clock"
        );
        assert_eq!(cameras.snapshot(RESET_ACK_TIMEOUT_MS)["cameras"][0]["actions"]["setMode"]["offer"], "ready");
    }

    #[test]
    fn streams_are_listed_without_the_thermal_one_and_choosing_one_swaps_the_stream() {
        let mut cameras = quiet(CAP_HAS_VIDEO_STREAM);
        let wide = StreamReport {
            stream_id: 1,
            count: 3,
            kind: 0,
            flags: STREAM_FLAG_RUNNING,
            framerate_hz: 30.0,
            resolution_h: 1920,
            resolution_v: 1080,
            bitrate_bps: 4_000_000,
            rotation_deg: 450.0,
            hfov_deg: 63.0,
            name: "Wide".into(),
            uri: "rtsp://one".into(),
        };
        cameras.on_video_stream_information(CAMERA, wide.clone(), 0);
        cameras.on_video_stream_information(CAMERA, StreamReport { stream_id: 2, flags: STREAM_FLAG_THERMAL, name: "Thermal".into(), ..wide.clone() }, 0);
        cameras.on_video_stream_information(CAMERA, StreamReport { stream_id: 3, flags: 0, name: "Narrow".into(), ..wide.clone() }, 0);
        let camera = cameras.camera(CAMERA).unwrap();
        assert_eq!(camera.listed_streams().len(), 2, "the thermal stream is kept but never listed among the pickable ones");
        assert_eq!(camera.thermal_stream().map(|stream| stream.stream_id), Some(2));
        assert_eq!(camera.streams[0].rotation_deg, 90.0, "four hundred and fifty degrees of rotation is ninety, because an angle wraps");
        let swap = cameras.select_stream(3, 1_000).unwrap();
        assert_eq!(
            commands(&swap),
            vec![(CMD_VIDEO_STOP_STREAMING, 1.0), (CMD_VIDEO_START_STREAMING, 3.0), (CMD_REQUEST_MESSAGE, MSG_VIDEO_STREAM_STATUS as f64)]
        );
        assert!(cameras.select_stream(3, 1_000).unwrap().is_empty(), "choosing the stream already chosen sends nothing");
        assert_eq!(cameras.select_stream(9, 1_000), Err(Refusal::UnknownStream));
        cameras.on_video_stream_status(
            CAMERA,
            StreamStatusReport { stream_id: 3, flags: STREAM_FLAG_RUNNING, framerate_hz: 25.0, resolution_h: 1280, resolution_v: 720, bitrate_bps: 2_000_000, rotation_deg: -90.0, hfov_deg: 70.0 },
            1_500,
        );
        let updated = cameras.camera(CAMERA).unwrap().current_stream().unwrap().clone();
        assert!(updated.running() && updated.framerate_hz == 25.0 && updated.rotation_deg == 270.0, "a status message rewrites the stream and its angle wraps the same way");
        assert!(ticks(&mut cameras, 2_000, 30, REQUEST_TIMEOUT_MS).is_empty(), "every stream arrived and the status came back, so both sweeps are finished and neither is still running");
    }

    #[test]
    fn the_stream_a_head_draws_is_asked_for_its_status_even_when_the_thermal_one_arrived_first() {
        let mut cameras = quiet(CAP_HAS_VIDEO_STREAM);
        let thermal = StreamReport { stream_id: 1, count: 2, flags: STREAM_FLAG_THERMAL, name: "Thermal".into(), ..Default::default() };
        cameras.on_video_stream_information(CAMERA, thermal, 0);
        assert!(ticks(&mut cameras, REQUEST_TIMEOUT_MS, 8, REQUEST_TIMEOUT_MS).iter().all(|sent| sent.command != CMD_REQUEST_VIDEO_STREAM_STATUS), "with nothing listed yet there is no stream to ask about");
        cameras.on_video_stream_information(CAMERA, StreamReport { stream_id: 2, count: 2, flags: 0, framerate_hz: 30.0, name: "Wide".into(), ..Default::default() }, 10_000);
        let asked = about(&ticks(&mut cameras, 10_000 + STREAM_CHECK_DELAY_MS, 3, REQUEST_TIMEOUT_MS), MSG_VIDEO_STREAM_STATUS, CMD_REQUEST_VIDEO_STREAM_STATUS);
        assert!(
            !asked.is_empty() && asked[0].params[0] == 2.0,
            "the first stream a head can actually draw has to start the status sweep, or it reports its discovery time running flag for the rest of the flight"
        );
    }

    #[test]
    fn a_stream_reports_the_field_of_view_it_was_given() {
        let mut cameras = quiet(CAP_HAS_VIDEO_STREAM);
        cameras.on_video_stream_information(CAMERA, StreamReport { stream_id: 1, count: 1, hfov_deg: 200.0, name: "Fisheye".into(), ..Default::default() }, 0);
        assert_eq!(
            cameras.snapshot(0)["cameras"][0]["streams"][0]["horizontalFieldOfView"],
            200.0,
            "a panoramic stream really does see past half a turn, and the C++ passes the stream hfov straight through - the half turn ceiling belongs to the vertical field of view it computes, not to a span a camera reported"
        );
    }

    #[test]
    fn video_stream_discovery_gives_up_after_six_attempts_for_every_stream_it_expects() {
        let mut silent = quiet(CAP_HAS_VIDEO_STREAM);
        let asked = about(&ticks(&mut silent, STREAM_CHECK_DELAY_MS, 40, REQUEST_TIMEOUT_MS), MSG_VIDEO_STREAM_INFORMATION, CMD_REQUEST_VIDEO_STREAM_INFORMATION);
        assert_eq!(asked.len(), 1 + STREAM_ATTEMPTS_PER_STREAM as usize, "one stream is expected until a camera says otherwise, so the sweep asks six more times and stops");
        let mut partial = quiet(CAP_HAS_VIDEO_STREAM);
        at(&mut partial, STREAM_CHECK_DELAY_MS);
        partial.on_video_stream_information(CAMERA, StreamReport { stream_id: 1, count: 2, ..Default::default() }, STREAM_CHECK_DELAY_MS);
        let chasing = about(&ticks(&mut partial, 10_000, 40, REQUEST_TIMEOUT_MS), MSG_VIDEO_STREAM_INFORMATION, CMD_REQUEST_VIDEO_STREAM_INFORMATION);
        assert!(chasing.iter().all(|command| command.params[0] == 2.0 || command.params[1] == 2.0), "the sweep asks for the stream that is missing, never for the one already held");
        assert_eq!(chasing.len(), (2 * STREAM_ATTEMPTS_PER_STREAM) as usize, "six attempts for each of the two streams the camera promised, and then it stops asking");
        assert!(partial.camera(CAMERA).unwrap().current_stream().is_some(), "the one stream that did arrive is still usable");
    }

    #[test]
    fn storage_capacities_are_only_believed_when_the_card_says_it_is_ready() {
        let mut cameras = quiet(0);
        cameras.on_storage_information(CAMERA, StorageReport { storage_id: 1, storage_count: 1, status: STORAGE_READY, total_capacity_mib: 61_000.0, available_capacity_mib: 42_000.0 }, 0);
        let ready = cameras.snapshot(0)["cameras"][0]["storage"].clone();
        assert_eq!((ready["slots"][0]["status"].as_str(), ready["slots"][0]["total"].as_f64(), ready["free"].as_f64()), (Some("ready"), Some(61_000.0), Some(42_000.0)));
        cameras.on_storage_information(CAMERA, StorageReport { storage_id: 1, storage_count: 1, status: STORAGE_UNFORMATTED, total_capacity_mib: 0.0, available_capacity_mib: 0.0 }, 0);
        let unformatted = cameras.snapshot(0)["cameras"][0]["storage"].clone();
        assert_eq!(
            (unformatted["slots"][0]["status"].as_str(), unformatted["slots"][0]["total"].as_f64()),
            (Some("unformatted"), Some(61_000.0)),
            "the capacities in a message that is not ready are meaningless and must not overwrite what was measured"
        );
        assert!(ticks(&mut cameras, 3_000, 10, REQUEST_TIMEOUT_MS).is_empty(), "the storage information arrived, so the asking stopped");
    }

    #[test]
    fn a_camera_with_two_cards_describes_both_of_them() {
        let mut cameras = quiet(0);
        cameras.on_storage_information(CAMERA, StorageReport { storage_id: 1, storage_count: 2, status: STORAGE_READY, total_capacity_mib: 61_000.0, available_capacity_mib: 40_000.0 }, 0);
        cameras.on_storage_information(CAMERA, StorageReport { storage_id: 2, storage_count: 2, status: STORAGE_UNFORMATTED, total_capacity_mib: 0.0, available_capacity_mib: 0.0 }, 0);
        let storage = cameras.snapshot(0)["cameras"][0]["storage"].clone();
        assert_eq!(storage["storageCount"], 2);
        assert_eq!(
            (storage["slots"][0]["storageId"].as_u64(), storage["slots"][1]["storageId"].as_u64(), storage["slots"][1]["status"].as_str()),
            (Some(1), Some(2), Some("unformatted")),
            "the second card is the one the camera may be writing to, so its state cannot be overwritten by whichever message arrived last"
        );
    }

    #[test]
    fn formatting_a_card_is_guarded_and_the_card_is_read_again_afterwards() {
        let mut cameras = quiet(CAP_CAPTURE_VIDEO);
        cameras.on_storage_information(CAMERA, StorageReport { storage_id: 1, storage_count: 1, status: STORAGE_UNFORMATTED, ..Default::default() }, 0);
        assert_eq!(cameras.format_storage(2, 0), Err(Refusal::UnknownStorage), "a camera with one card has no second card to format");
        cameras.on_capture_status(CAMERA, CaptureStatusReport { video_status: VIDEO_RUNNING, ..Default::default() }, 0);
        assert_eq!(cameras.format_storage(1, 0), Err(Refusal::Busy), "formatting the card a recording is being written to is not something to accept silently");
        cameras.on_capture_status(CAMERA, CaptureStatusReport::default(), 1_000);
        assert_eq!(commands(&cameras.format_storage(1, 1_000).unwrap()), vec![(CMD_STORAGE_FORMAT, 1.0)]);
        cameras.on_command_result(CAMERA, CMD_STORAGE_FORMAT, 1.0, RESULT_ACCEPTED, 1_100);
        assert!(at(&mut cameras, 1_100 + STORAGE_DELAY_MS - 1).is_empty());
        assert_eq!(
            commands(&at(&mut cameras, 1_100 + STORAGE_DELAY_MS)).len(),
            1,
            "after a format the card is read again, or the screen keeps saying unformatted and full and the operator cannot tell whether the format worked"
        );
    }

    #[test]
    fn the_firmware_version_the_aspect_and_the_field_of_view_come_back_as_numbers() {
        let camera = info(0);
        assert_eq!(camera.firmware(), Some(Firmware { major: 2, minor: 3, patch: 4, dev: 0 }), "0x00040302 is 2.3.4, the low byte first encoding the C++ and the MAVLink specification both use");
        assert_eq!(Info { firmware_version: 0, ..camera.clone() }.firmware(), None, "a firmware version of zero means not known, not version zero");
        assert_eq!(camera.aspect_vertical_over_horizontal(), Some(1080.0 / 1920.0));
        let sensor_only = Info { resolution_h: 0, resolution_v: 0, ..camera.clone() };
        assert_eq!(sensor_only.aspect_vertical_over_horizontal(), Some(4.55 / 6.17), "with no pixel count the sensor size answers instead");
        let blind = Info { resolution_h: 0, resolution_v: 0, sensor_size_h_mm: f64::NAN, sensor_size_v_mm: f64::NAN, ..camera.clone() };
        assert_eq!(blind.aspect_vertical_over_horizontal(), None);
        assert!((vertical_field_of_view_degrees(90.0, Some(1.0)).unwrap() - 90.0).abs() < 1e-9);
        assert!(
            (vertical_field_of_view_degrees(90.0, None).unwrap() - 58.715_507_085_582_54).abs() < 1e-9,
            "an unknown aspect falls back to sixteen by nine in the one convention this module uses, tall over wide - the reciprocal would double every field of view a gimbal tilt is derived from"
        );
        assert!(
            (vertical_field_of_view_degrees(90.0, None).unwrap() - vertical_field_of_view_degrees(90.0, Info { ..camera.clone() }.aspect_vertical_over_horizontal()).unwrap()).abs() < 1e-9,
            "and a sixteen by nine camera and the fallback have to agree, which is what says the fallback is in the right convention"
        );
        assert_eq!(vertical_field_of_view_degrees(0.0, Some(1.0)), None);
        assert_eq!(vertical_field_of_view_degrees(MAX_FIELD_OF_VIEW_DEGREES, Some(1.0)), None);
        assert_eq!(vertical_field_of_view_degrees(f64::NAN, Some(1.0)), None);
        assert!(
            vertical_field_of_view_degrees(170.0, Some(9.0)).is_some_and(|vertical| vertical < MAX_FIELD_OF_VIEW_DEGREES),
            "a very wide lens on a very tall sensor still lands inside half a turn, and the answer says so rather than clipping"
        );
        assert_eq!(wrap_degrees(-1.0), 359.0);
    }

    #[test]
    fn the_snapshot_speaks_tokens_names_its_units_and_never_renders_a_number() {
        let mut cameras = discovered(CAP_CAPTURE_IMAGE | CAP_HAS_MODES | CAP_HAS_BASIC_ZOOM);
        cameras.on_camera_settings(CAMERA, SettingsReport { mode_id: MODE_PHOTO, zoom_percent: 12.5, focus_percent: f64::NAN }, 0);
        let view = cameras.snapshot(0);
        let camera = &view["cameras"][0];
        assert_eq!(camera["mode"], "photo");
        assert_eq!(camera["zoom"], 12.5);
        assert!(camera["focus"].is_null(), "a focus level the camera reports as not a number stays absent");
        assert_eq!(camera["capabilities"], json!(["captureImage", "modes", "basicZoom"]));
        assert_eq!(camera["definitionUri"], "mftp://camera.xml", "the definition file is named for whoever parses it, and this module stops there");
        assert_eq!(camera["requests"]["cameraSettings"], json!({ "attempts": 0, "state": "answered" }));
        assert_eq!(
            (view["storageUnits"].as_str(), view["durationUnits"].as_str(), view["angleUnits"].as_str(), view["levelUnits"].as_str()),
            (Some("mebibyte"), Some("millisecond"), Some("degree"), Some("percent"))
        );
        let rendered = readable(&view);
        assert!(
            !rendered.iter().any(|text| text.chars().any(|c| c.is_ascii_digit()) && text.contains(['.', ',', ':', '%'])),
            "a rendered number carries a separator and a unit an operator reading Ukrainian would not recognise, so the core emits neither: {rendered:?}"
        );
    }

    fn readable(value: &Value) -> Vec<String> {
        match value {
            Value::String(text) => vec![text.clone()],
            Value::Array(items) => items.iter().flat_map(readable).collect(),
            Value::Object(fields) => fields.iter().filter(|(key, _)| *key != "definitionUri" && *key != "uri").flat_map(|(_, item)| readable(item)).collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn the_contract_view_keeps_the_numbers_the_c_plus_plus_uses() {
        let view = protocol_view(&Nothing, &[]);
        assert_eq!(view["attempts"], json!({ "cameraInformation": 10, "request": 5, "videoStreamInformationPerStream": 6 }));
        assert_eq!(view["delays"]["silentTimeout"], 5000);
        assert_eq!(view["delays"]["heartbeatTick"], 500);
        assert_eq!(view["delays"]["cameraSettings"], 500);
        assert_eq!(view["delays"]["videoStreamCheck"], 1000);
        assert_eq!(view["delays"]["captureStatus"], 1500);
        assert_eq!(view["delays"]["storageInformation"], 2000);
        assert_eq!(view["delays"]["requestTimeout"], 1000);
        assert_eq!(view["delays"]["videoStreamDiscovery"], 2000);
        assert_eq!(view["delays"]["cameraSettingsRefresh"], 1000);
        assert_eq!(view["delays"]["cameraSettingsAfterReset"], 2500);
        assert_eq!(view["delays"]["resetAckTimeout"], 5000);
        assert_eq!(view["delays"]["recordingPoll"], 5000);
        assert_eq!(view["delays"]["busyPoll"], 1000);
        assert_eq!(view["delays"]["modeChangePoll"], 1000);
        assert_eq!(view["staleMs"], 10_000);
        assert_eq!(view["messages"]["cameraInformation"], 259);
        assert_eq!(view["cameraComponentIds"], json!({ "first": 100, "last": 105 }));
        assert_eq!(view["refusals"][0], "noCamera");
        assert!(is_camera_component(COMP_ID_CAMERA) && is_camera_component(COMP_ID_CAMERA6) && !is_camera_component(1) && !is_camera_component(106));
    }

    #[test]
    fn the_contract_view_pins_every_wire_value_to_a_literal_not_to_the_constant_itself() {
        let view = protocol_view(&Nothing, &[]);
        assert_eq!(view["levelTypes"], json!({ "step": 0.0, "continuous": 1.0, "range": 2.0 }), "the zoom and focus type codes are wire values, so a literal has to hold them");
        assert_eq!(view["levelRange"], json!({ "min": 0.0, "max": 100.0 }));
        assert_eq!(view["fallbackAspect"], json!(0.5625), "sixteen by nine as tall over wide, the convention every aspect in this module is in");
        assert_eq!(
            view["capabilities"],
            json!([
                { "flag": 1, "token": "captureVideo" },
                { "flag": 2, "token": "captureImage" },
                { "flag": 4, "token": "modes" },
                { "flag": 8, "token": "imageInVideoMode" },
                { "flag": 16, "token": "videoInImageMode" },
                { "flag": 32, "token": "imageSurveyMode" },
                { "flag": 64, "token": "basicZoom" },
                { "flag": 128, "token": "basicFocus" },
                { "flag": 256, "token": "videoStream" },
                { "flag": 512, "token": "trackingPoint" },
                { "flag": 1024, "token": "trackingRectangle" },
                { "flag": 2048, "token": "trackingGeoStatus" },
            ]),
            "a capability bit that shifts by one silently points every guard in this module at the wrong flag"
        );
        assert_eq!(view["modes"], json!([{ "code": 0, "token": "photo" }, { "code": 1, "token": "video" }, { "code": 2, "token": "survey" }]));
        assert_eq!(
            view["photoStatus"],
            json!([
                { "code": 0, "token": "idle" },
                { "code": 1, "token": "inProgress" },
                { "code": 2, "token": "intervalIdle" },
                { "code": 3, "token": "intervalInProgress" },
                { "code": 255, "token": "undefined" },
            ])
        );
        assert_eq!(view["videoStatus"], json!([{ "code": 0, "token": "stopped" }, { "code": 1, "token": "running" }, { "code": 255, "token": "undefined" }]));
        assert_eq!(
            view["storageStatus"],
            json!([{ "code": 0, "token": "empty" }, { "code": 1, "token": "unformatted" }, { "code": 2, "token": "ready" }, { "code": 3, "token": "notSupported" }])
        );
        assert_eq!(view["streamTypes"], json!([{ "code": 0, "token": "rtsp" }, { "code": 1, "token": "rtpUdp" }, { "code": 2, "token": "tcpMpeg" }, { "code": 3, "token": "mpegTs" }]));
        assert_eq!(
            view["results"],
            json!([
                { "code": 0, "token": "accepted" },
                { "code": 1, "token": "temporarilyRejected" },
                { "code": 2, "token": "denied" },
                { "code": 3, "token": "unsupported" },
                { "code": 4, "token": "failed" },
                { "code": 5, "token": "inProgress" },
            ]),
            "the result codes are what a head turns into what the camera said, and they are MAV_RESULT, not a local numbering"
        );
        assert_eq!(
            view["actions"],
            json!(["setMode", "takePhoto", "stopTakePhoto", "startRecording", "stopRecording", "zoom", "focus", "formatStorage", "selectStream", "resetSettings"])
        );
    }
}
