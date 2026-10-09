use std::sync::{Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};

use crate::videostate::{MAIN_RECEIVER, Out, Outcome, PIP_RECEIVER, Settings, SourceSlot, Status, VideoState};

const RECORD_TEE: &str = "tee name=nativerec ! queue";
#[cfg(not(target_os = "android"))]
const NATIVE_SINK: &str = "videoconvert ! appsink name=nativesink";
#[cfg(target_os = "android")]
const NATIVE_SINK: &str = "glupload ! glcolorconvert ! glimagesink name=nativesink qos=false max-lateness=-1";

pub const VIDEO_CHANNELS: usize = 2;
pub const MAIN_CHANNEL: usize = 0;
pub const PIP_CHANNEL: usize = 1;
pub const VIDEO_ABI_VERSION: i32 = 3;
const CHANNEL_RECEIVERS: [&str; VIDEO_CHANNELS] = [MAIN_RECEIVER, PIP_RECEIVER];

#[derive(Default)]
struct Channel {
    wanted: bool,
    restart_at_ms: Option<u64>,
    reported: (bool, bool, u32, u32),
    timeout_s: u32,
    progress: Option<Watch>,
    flowing: bool,
    problem: Option<(String, String)>,
}

#[derive(Default)]
struct Host {
    state: VideoState,
    registered: usize,
    channels: [Channel; VIDEO_CHANNELS],
    recording_file: Option<String>,
    vehicle: Option<u8>,
    camera_rotation: u32,
}

fn channel_of(receiver: &str) -> Option<usize> {
    CHANNEL_RECEIVERS.iter().position(|played| *played == receiver)
}

const FILE_EXTENSIONS: [&str; 3] = ["mkv", "mov", "mp4"];
const DEFAULT_RECORDING_FORMAT: i64 = 2;
const DEFAULT_MAX_VIDEO_MB: u64 = 10240;
const BAD_FORMAT_MESSAGE: &str = "Invalid video format defined.";
const NO_SAVE_PATH_MESSAGE: &str = "Unabled to record video. Video save path must be specified in Settings.";
pub const SET_DEVICE_CAMERA_ROTATION: &str = "video.setDeviceCameraRotation";
pub const SET_PIP_SHOWN: &str = "video.setPipShown";
const DEVICE_CAMERA_RECORDING_MESSAGE: &str = "Recording is not available for this device's own camera.";

static HOST: Mutex<Option<Host>> = Mutex::new(None);

thread_local! {
    static RENDERED: std::cell::RefCell<Option<(u64, std::rc::Rc<Value>)>> = const { std::cell::RefCell::new(None) };
}

fn setting(name: &str) -> Value {
    crate::settingsstore::raw_setting(&format!("settings.videoSettings.{name}")).unwrap_or(Value::Null)
}

pub fn settings_from(text: &dyn Fn(&str) -> String, flag: &dyn Fn(&str, bool) -> bool, number: &dyn Fn(&str, i64) -> i64) -> Settings {
    Settings {
        cameras: crate::cameras::parse(&text(crate::cameras::CAMERAS_FACT)).unwrap_or_default().into_iter().map(|camera| SourceSlot { source: camera.source, url: camera.url, name: camera.name, drone: false }).collect(),
        active_source: number(crate::cameras::ACTIVE_FACT, 0),
        multi_view: flag("multiViewEnabled", false),
        pip_shown: false,
        stream_enabled: flag("streamEnabled", true),
        low_latency: flag("lowLatencyMode", false),
        save_path_set: false,
        recording_format_valid: false,
        rtsp_timeout_s: u32::try_from(number("rtspTimeout", 8)).unwrap_or(8),
        reconnect_disabled: !flag("rtspAutoReconnect", true),
    }
}

pub fn with_drone(settings: Settings, drone: &[(String, u8, u8, String)]) -> Settings {
    let announced = drone.iter().filter_map(|(name, kind, encoding, uri)| {
        let (source, uri) = crate::videostate::auto_stream_source(*kind, *encoding, uri);
        let url = match source {
            crate::videostate::SOURCE_RTSP | crate::videostate::SOURCE_WEBRTC => uri.clone(),
            _ => uri.split_once("://").map_or(uri.as_str(), |(_, rest)| rest).to_string(),
        };
        (!uri.is_empty()).then(|| SourceSlot { source: source.to_string(), url, name: name.clone(), drone: true })
    });
    Settings { cameras: settings.cameras.into_iter().chain(announced).collect(), ..settings }
}

fn stored_settings() -> Settings {
    let [cameras, active] = crate::settingsstore::raw_settings([crate::cameras::CAMERAS_PATH, crate::cameras::ACTIVE_PATH]);
    let read = |name: &str| match name {
        crate::cameras::CAMERAS_FACT => cameras.clone(),
        crate::cameras::ACTIVE_FACT => active.clone(),
        _ => setting(name),
    };
    let settings = settings_from(&|name| read(name).as_str().unwrap_or_default().trim().to_string(), &|name, default| read(name).as_bool().unwrap_or(default), &|name, default| read(name).as_i64().unwrap_or(default));
    Settings { save_path_set: crate::settingsstore::video_save_path().is_some(), recording_format_valid: extension(recording_format()).is_some(), ..settings }
}

fn recording_format() -> i64 {
    setting("recordingFormat").as_i64().unwrap_or(DEFAULT_RECORDING_FORMAT)
}

fn extension(format: i64) -> Option<&'static str> {
    usize::try_from(format).ok().and_then(|index| FILE_EXTENSIONS.get(index)).copied()
}

pub fn recording_file_name(folder: &str, stamp: &str, format: i64) -> Option<String> {
    Some(format!("{folder}/{stamp}.{}", extension(format)?))
}

pub fn videos_to_delete(mut videos: Vec<(String, u64, std::time::SystemTime)>, max_bytes: u64) -> Vec<String> {
    videos.sort_by(|a, b| b.2.cmp(&a.2));
    let total: u64 = videos.iter().map(|v| v.1).sum();
    videos
        .iter()
        .rev()
        .scan(total, |left, (name, size, _)| {
            let over = *left >= max_bytes;
            *left = left.saturating_sub(*size);
            over.then(|| name.clone())
        })
        .collect()
}

fn cleanup_old_videos(folder: &str) {
    if !setting("enableStorageLimit").as_bool().unwrap_or(false) {
        return;
    }
    let max_bytes = setting("maxVideoSize").as_u64().unwrap_or(DEFAULT_MAX_VIDEO_MB) * 1024 * 1024;
    let videos: Vec<(String, u64, std::time::SystemTime)> = std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().extension().and_then(|e| e.to_str()).is_some_and(|e| FILE_EXTENSIONS.contains(&e)))
                .filter_map(|entry| {
                    let meta = entry.metadata().ok().filter(std::fs::Metadata::is_file)?;
                    Some((entry.path().to_string_lossy().into_owned(), meta.len(), meta.modified().ok()?))
                })
                .collect()
        })
        .unwrap_or_default();
    videos_to_delete(videos, max_bytes).iter().for_each(|path| {
        let _ = std::fs::remove_file(path);
    });
}

pub fn start_recording() -> Result<(), Option<&'static str>> {
    let started = begin_recording();
    if let Err(Some(message)) = started {
        crate::noticeboard::post(crate::noticeboard::MESSAGE, "", message);
    }
    started
}

fn begin_recording() -> Result<(), Option<&'static str>> {
    let mut guard = synced();
    let host = guard.as_mut().ok_or(None)?;
    if crate::videostate::device_camera(&host.state.desired_uri(MAIN_RECEIVER)).is_some() {
        return Err(Some(DEVICE_CAMERA_RECORDING_MESSAGE));
    }
    match host.state.record_refusal() {
        Some(crate::videostate::REFUSED_BAD_FORMAT) => return Err(Some(BAD_FORMAT_MESSAGE)),
        Some(crate::videostate::REFUSED_NO_SAVE_PATH) => return Err(Some(NO_SAVE_PATH_MESSAGE)),
        Some(_) => return Err(None),
        None => {}
    }
    let records_main = host.state.start_recording().iter().any(|out| matches!(out, Out::StartRecording { receivers } if receivers.iter().any(|receiver| receiver == MAIN_RECEIVER)));
    records_main.then_some(()).ok_or(None)?;
    let folder = crate::settingsstore::video_save_path().ok_or(Some(NO_SAVE_PATH_MESSAGE))?;
    cleanup_old_videos(&folder);
    let _ = std::fs::create_dir_all(&folder);
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H.%M.%S").to_string();
    let file = recording_file_name(&folder, &stamp, recording_format()).ok_or(Some(BAD_FORMAT_MESSAGE))?;
    let (_, _, width, height) = host.channels[MAIN_CHANNEL].reported;
    crate::subtitles::start(&file, Some((width, height)), crate::hub::now_ms());
    host.recording_file = Some(file);
    Ok(())
}

pub fn stop_recording() {
    let mut guard = synced();
    if let Some(host) = guard.as_mut() {
        host.recording_file = None;
        crate::subtitles::stop();
        let outs = host.state.stop_recording();
        apply(host, outs, crate::hub::now_ms());
    }
}

#[cfg(all(feature = "jni-host", not(test)))]
const FRAME_CAPACITY_FACTORS: [usize; 3] = [1, 2, 4];

pub const NO_FRAME_MESSAGE: &str = "There is no video frame to capture.";
const FRAME_TOO_LARGE: &str = "The video frame is too large to save as a photo.";

pub fn tight_rgba(buffer: &[u8], width: usize, height: usize, stride: usize) -> Option<Vec<u8>> {
    let row = width * 4;
    (stride >= row && buffer.len() >= stride * height.saturating_sub(1) + row).then(|| (0..height).flat_map(|y| buffer[y * stride..y * stride + row].iter().copied()).collect())
}

pub fn photo_file_name(folder: &str, stamp: &str) -> String {
    format!("{}/{stamp}.jpg", folder.trim_end_matches('/'))
}

#[cfg(all(feature = "jni-host", not(test)))]
fn latest_frame() -> Option<(Vec<u8>, usize, usize)> {
    let video = crate::androidvideo::video()?;
    let main = MAIN_CHANNEL as std::ffi::c_int;
    let (width, height) = unsafe { ((video.width)(main), (video.height)(main)) };
    let tight = usize::try_from(width).ok().filter(|w| *w > 0)? * usize::try_from(height).ok().filter(|h| *h > 0)? * 4;
    FRAME_CAPACITY_FACTORS.iter().find_map(|factor| {
        let mut buffer = vec![0u8; tight * factor];
        let (mut w, mut h, mut stride) = (0, 0, 0);
        let copied = unsafe { (video.copy_frame)(main, buffer.as_mut_ptr().cast(), i32::try_from(buffer.len()).ok()?, &mut w, &mut h, &mut stride) };
        let (w, h, stride) = (usize::try_from(w).ok()?, usize::try_from(h).ok()?, usize::try_from(stride).ok()?);
        copied.then(|| tight_rgba(&buffer, w, h, stride).map(|rgba| (rgba, w, h))).flatten()
    })
}

#[cfg(not(all(feature = "jni-host", not(test))))]
fn latest_frame() -> Option<(Vec<u8>, usize, usize)> {
    None
}

pub fn grab_image() -> Result<String, &'static str> {
    let (rgba, width, height) = latest_frame().ok_or(NO_FRAME_MESSAGE)?;
    let folder = crate::settingsstore::photo_save_path().ok_or("Unable to save the photo. The save path must be specified in Settings.")?;
    let _ = std::fs::create_dir_all(&folder);
    let file = photo_file_name(&folder, &chrono::Local::now().format("%Y-%m-%d_%H.%M.%S%.3f").to_string());
    let (width, height) = (u16::try_from(width).map_err(|_| FRAME_TOO_LARGE)?, u16::try_from(height).map_err(|_| FRAME_TOO_LARGE)?);
    jpeg_encoder::Encoder::new_file(&file, 90)
        .and_then(|encoder| encoder.encode(&rgba, width, height, jpeg_encoder::ColorType::Rgba))
        .map(|_| file)
        .map_err(|_| "The photo could not be saved.")
}

pub fn recording() -> bool {
    get("video.recording").and_then(|v| v.get("value")?.as_bool()).unwrap_or(false) || HOST.lock().unwrap_or_else(PoisonError::into_inner).as_ref().is_some_and(|host| host.recording_file.is_some())
}

fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

pub fn host_port(rest: &str) -> Option<(String, u16)> {
    let authority = rest.split(['/', '?']).next()?;
    let (host, port) = authority.rsplit_once(':')?;
    Some((if host.is_empty() { "0.0.0.0".to_string() } else { host.to_string() }, port.parse().ok()?))
}

const RETRANSMISSION_MIN_LATENCY_MS: i64 = 40;
const UDP_BUFFER_BYTES: u32 = 8 * 1024 * 1024;
const TS_VIDEO: &str = "capsfilter caps=\"video/x-h264;video/x-h265\"";
const WHEP_TIMEOUT_S: u32 = 8;
const WHEP_LOW_LATENCY_JITTER_MS: i64 = 40;
const WHEP_LATENCY_PREFIX: &str = "whep_latency_";
const WHEP_VIDEO_CAPS: &str = "application/x-rtp,media=(string)video,encoding-name=(string)H264,payload=(int)96,clock-rate=(int)90000;application/x-rtp,media=(string)video,encoding-name=(string)H265,payload=(int)97,clock-rate=(int)90000";

fn flip_method(rotation: &str) -> &'static str {
    match rotation {
        "90" => "clockwise",
        "180" => "rotate-180",
        "270" => "counterclockwise",
        _ => "none",
    }
}

fn oriented(uri: &str, rotation: u32) -> String {
    match crate::videostate::device_camera(uri) {
        Some(_) => format!("{uri}?rotation={rotation}"),
        None => uri.to_string(),
    }
}

pub fn pipeline(uri: &str, latency_ms: i64, low_latency: bool) -> Option<String> {
    let retransmit = latency_ms >= RETRANSMISSION_MIN_LATENCY_MS && !low_latency;
    let jitter = match low_latency {
        true => String::new(),
        false => format!(" ! rtpjitterbuffer latency={latency_ms} do-lost=true do-retransmission={retransmit} drop-on-latency=true{}", if retransmit { " rtx-delay=25 rtx-max-retries=1" } else { "" }),
    };
    let rtp = |encoding: &str, rest: &str| {
        let (host, port) = host_port(rest)?;
        Some(format!(
            "udpsrc address={host} port={port} buffer-size={UDP_BUFFER_BYTES} caps=\"application/x-rtp, media=(string)video, clock-rate=(int)90000, encoding-name=(string){encoding}\"{jitter}"
        ))
    };
    let source = match uri.split_once("://")? {
        (scheme, _) if scheme.to_ascii_lowercase().starts_with("rtsp") => format!("rtspsrc location={} latency={latency_ms} do-rtcp=true do-retransmission={retransmit} drop-on-latency=true", quoted(uri)),
        ("udp", rest) => rtp("H264", rest)?,
        ("udp265", rest) => rtp("H265", rest)?,
        ("mpegts", rest) => {
            let (host, port) = host_port(rest)?;
            format!("udpsrc address={host} port={port} buffer-size={UDP_BUFFER_BYTES} ! tsdemux ! {TS_VIDEO}")
        }
        ("tcp", rest) => {
            let (host, port) = host_port(rest)?;
            format!("tcpclientsrc host={host} port={port} ! tsdemux ! {TS_VIDEO}")
        }
        ("http" | "https" | "whep" | "wheps", _) => {
            let endpoint = uri.replacen("wheps://", "https://", 1).replacen("whep://", "http://", 1);
            let webrtc_latency_ms = if low_latency { WHEP_LOW_LATENCY_JITTER_MS } else { latency_ms };
            format!("whepsrc name={WHEP_LATENCY_PREFIX}{webrtc_latency_ms} whep-endpoint={} video-caps={} audio-caps=EMPTY timeout={WHEP_TIMEOUT_S}", quoted(&endpoint), quoted(WHEP_VIDEO_CAPS))
        }
        ("ahc", rest) => {
            let (device, rotation) = rest.split_once("?rotation=").unwrap_or((rest, "0"));
            let device: u32 = device.parse().ok()?;
            return Some(format!("ahcsrc device={device} ! {RECORD_TEE} ! glupload ! glcolorconvert ! glvideoflip method={} ! {NATIVE_SINK} sync=false", flip_method(rotation)));
        }
        _ => return None,
    };
    Some(format!("{source} ! {RECORD_TEE} ! decodebin3 ! {NATIVE_SINK} sync={}", !low_latency))
}

fn apply(host: &mut Host, outs: Vec<Out>, now_ms: u64) {
    let follow_ups: Vec<Out> = outs
        .into_iter()
        .flat_map(|out| match out {
            Out::StartReceiver { receiver, timeout_s, low_latency } => match channel_of(&receiver) {
                Some(channel) => {
                    let uri = host.state.desired_uri(&receiver);
                    let buildable = pipeline(&uri, setting("rtpJitterLatencyMs").as_i64().unwrap_or(80), low_latency).is_some();
                    let played = &mut host.channels[channel];
                    played.restart_at_ms = None;
                    played.timeout_s = timeout_s;
                    played.wanted = buildable;
                    played.progress = buildable.then(|| Watch::fresh(now_ms));
                    match buildable {
                        true => Vec::new(),
                        false => host.state.on_start_complete(&receiver, unbuildable_outcome(&uri), now_ms / 1000),
                    }
                }
                None => Vec::new(),
            },
            Out::StopReceiver { receiver } => match channel_of(&receiver) {
                Some(channel) => {
                    if channel == MAIN_CHANNEL {
                        crate::subtitles::stop();
                    }
                    let played = &mut host.channels[channel];
                    played.wanted = false;
                    played.progress = None;
                    played.reported = (false, false, 0, 0);
                    played.flowing = false;
                    host.state.on_stop_complete(&receiver, Outcome::Ok)
                }
                None => Vec::new(),
            },
            Out::StopTelemetryCapture => {
                crate::subtitles::stop();
                Vec::new()
            }
            Out::SetSetting { name, value } => {
                crate::settingsstore::set_raw(&format!("settings.videoSettings.{name}"), &json!(value));
                Vec::new()
            }
            Out::RestartAfter { receiver, delay_ms } => {
                if let Some(channel) = channel_of(&receiver) {
                    host.channels[channel].restart_at_ms = Some(now_ms + delay_ms);
                }
                Vec::new()
            }
            _ => Vec::new(),
        })
        .collect();
    if !follow_ups.is_empty() {
        apply(host, follow_ups, now_ms);
    }
}

pub fn stream_switches(was: Option<u8>, now: Option<u8>, outs: &[Out]) -> Vec<(u8, bool)> {
    outs.iter()
        .filter_map(|out| match out {
            Out::StopCameraStream => was.map(|id| (id, false)),
            Out::StartCameraStream => now.map(|id| (id, true)),
            _ => None,
        })
        .collect()
}

static STREAM_SWITCHES: std::sync::LazyLock<Mutex<std::sync::mpsc::Sender<(u8, bool)>>> = std::sync::LazyLock::new(|| {
    let (sender, receiver) = std::sync::mpsc::channel::<(u8, bool)>();
    std::thread::spawn(move || {
        receiver.iter().for_each(|(vehicle, start)| {
            let op = if start { "resumeStream" } else { "stopStream" };
            let frames = crate::hub::lock().guided(Some(vehicle), &json!({ "action": "camera", "op": op }), crate::hub::now_ms()).unwrap_or_default();
            frames.iter().for_each(|(link, bytes)| { crate::linkhost::write(&crate::linkhost::TRANSPORTS, *link, bytes); });
        });
    });
    Mutex::new(sender)
});

fn send_stream_switches(switches: Vec<(u8, bool)>) {
    if switches.is_empty() {
        return;
    }
    let sender = STREAM_SWITCHES.lock().unwrap_or_else(PoisonError::into_inner);
    switches.into_iter().for_each(|switch| {
        let _ = sender.send(switch);
    });
}

fn synced() -> MutexGuard<'static, Option<Host>> {
    let (drone, active) = {
        let hub = crate::hub::lock();
        (hub.active().map(crate::hub::Vehicle::drone_streams).unwrap_or_default(), hub.active_id())
    };
    let mut guard = HOST.lock().unwrap_or_else(PoisonError::into_inner);
    let host = guard.get_or_insert_with(Host::default);
    let now_ms = crate::hub::now_ms();
    if active != host.vehicle {
        let was = std::mem::replace(&mut host.vehicle, active);
        let outs = host.state.on_vehicle(active.is_some());
        send_stream_switches(stream_switches(was, active, &outs));
        apply(host, outs, now_ms);
    }
    let settings = Settings { pip_shown: host.state.settings.pip_shown, ..with_drone(stored_settings(), &drone) };
    if settings != host.state.settings {
        let outs = host.state.on_settings(settings);
        apply(host, outs, now_ms);
    }
    let receivers = host.state.settings.count().min(VIDEO_CHANNELS);
    if host.registered < receivers {
        let names = &CHANNEL_RECEIVERS[host.registered..receivers];
        host.registered = receivers;
        names.iter().for_each(|name| {
            let outs = host.state.register_receiver(name);
            apply(host, outs, now_ms);
        });
    }
    restart_due(host, now_ms);
    guard
}

fn show_pip(host: &mut Host, shown: bool, now_ms: u64) {
    if host.state.settings.pip_shown != shown {
        let outs = host.state.on_settings(Settings { pip_shown: shown, ..host.state.settings.clone() });
        apply(host, outs, now_ms);
    }
}

fn restart_due(host: &mut Host, now_ms: u64) {
    let due: Vec<usize> = (0..VIDEO_CHANNELS).filter(|channel| host.channels[*channel].restart_at_ms.is_some_and(|at| now_ms >= at)).collect();
    due.into_iter().for_each(|channel| {
        host.channels[channel].restart_at_ms = None;
        let outs = host.state.start_receiver(CHANNEL_RECEIVERS[channel]);
        apply(host, outs, now_ms);
    });
}

fn status_text(status: Option<Status>) -> &'static str {
    match status.unwrap_or(Status::NoReceiver) {
        Status::Decoding => "",
        Status::WaitingForFrames => "Connected, waiting for frames",
        Status::NotStarted => "Not started",
        Status::NoStreamUrl => "No stream URL",
        Status::InvalidStreamUrl => "Invalid stream URL",
        Status::Connecting => "Connecting\u{2026}",
        Status::Reconnecting => "Reconnecting\u{2026}",
        Status::ConnectionFailed => "Connection failed, retrying",
        _ => "No video source",
    }
}

fn object(host: &Host) -> Value {
    let state = &host.state;
    let settings = &state.settings;
    let current = settings.current_index();
    let uri = state.desired_uri(MAIN_RECEIVER);
    let (width, height) = state.video_size.unwrap_or((0, 0));
    json!({
        "kind": "object",
        "class": "VideoManager",
        "hasVideo": state.has_video(),
        "gstreamerEnabled": true,
        "isStreamSource": state.is_stream_source(),
        "decoding": state.decoding,
        "streaming": state.streaming,
        "recording": state.recording,
        "activeVideoSource": current,
        "videoSize": { "width": width, "height": height },
        "hasMultipleVideoSources": state.has_multiple_sources(),
        "cameraStatuses": (0..settings.count()).map(|i| status_text(state.receiver_status(i))).collect::<Vec<_>>(),
        "cameraConnecting": (0..settings.count()).map(|i| state.camera_connecting(i)).collect::<Vec<_>>(),
        "cameraSignals": (0..settings.count()).map(|i| state.camera_signal(i)).collect::<Vec<_>>(),
        "cameraRecording": (0..settings.count()).map(|i| state.camera_recording(i)).collect::<Vec<_>>(),
        "cameraConfigured": (0..settings.count()).map(|i| settings.configured(i)).collect::<Vec<_>>(),
        "cameraUsable": (0..settings.count()).map(|i| settings.usable(i)).collect::<Vec<_>>(),
        "cameraFromDrone": (0..settings.count()).map(|i| settings.from_drone(i)).collect::<Vec<_>>(),
        "cameraNames": (0..settings.count()).map(|i| settings.name_at(i).unwrap_or("")).collect::<Vec<_>>(),
        "cameraSources": (0..settings.count()).map(|i| settings.source_at(i)).collect::<Vec<_>>(),
        "cameraUrls": (0..settings.count()).map(|i| settings.url_at(i)).collect::<Vec<_>>(),
        "pipSlot": settings.pip_choice(),
        "nativePipeline": pipeline_for(host, MAIN_CHANNEL),
        "pipPipeline": pipeline_for(host, PIP_CHANNEL),
        "deviceCamera": crate::videostate::device_camera(&uri),
        "nativeRecording": host.recording_file,
        "nativeRecordingFormat": recording_format(),
        "streamProblem": host.channels[MAIN_CHANNEL].problem.as_ref().filter(|(at, _)| *at == uri).map_or("", |(_, text)| text.as_str()),
    })
}

fn pipeline_for(host: &Host, channel: usize) -> Option<String> {
    let uri = host.state.desired_uri(CHANNEL_RECEIVERS.get(channel)?);
    (host.channels[channel].wanted && host.state.has_video()).then(|| pipeline(&oriented(&uri, host.camera_rotation), setting("rtpJitterLatencyMs").as_i64().unwrap_or(80), host.state.settings.low_latency)).flatten()
}

fn served() -> bool {
    !crate::qthost::present()
}

pub fn has_video() -> bool {
    get("video.hasVideo").and_then(|v| v.get("value")?.as_bool()).unwrap_or(false)
}

pub fn native_recording() -> Option<Value> {
    let file = get("video.nativeRecording")?.get("value")?.as_str()?.to_string();
    Some(json!({ "file": file, "format": recording_format() }))
}

pub static RESTART: [std::sync::atomic::AtomicBool; VIDEO_CHANNELS] = [const { std::sync::atomic::AtomicBool::new(false) }; VIDEO_CHANNELS];

pub fn unbuildable_outcome(uri: &str) -> Outcome {
    if uri.trim().is_empty() { Outcome::InvalidUrl } else { Outcome::Failed }
}

pub fn stream_problem_text() -> String {
    get("video.streamProblem").and_then(|value| value.get("value")?.as_str().map(str::to_string)).unwrap_or_default()
}

pub fn device_camera() -> Option<u64> {
    get("video.deviceCamera")?.get("value")?.as_u64()
}

pub fn native_pipeline() -> Option<String> {
    get("video.nativePipeline")?.get("value")?.as_str().map(str::to_string)
}

pub fn pip_pipeline() -> Option<String> {
    get("video.pipPipeline")?.get("value")?.as_str().map(str::to_string)
}

pub fn channel_pipeline(channel: usize) -> Option<String> {
    served().then_some(())?;
    let guard = synced();
    pipeline_for(guard.as_ref()?, channel)
}

pub fn get(path: &str) -> Option<Value> {
    served().then_some(())?;
    match path {
        "video" => Some(rendered()?.as_ref().clone()),
        _ => {
            let field = path.strip_prefix("video.")?;
            Some(json!({ "kind": "value", "value": rendered()?.get(field)?.clone() }))
        }
    }
}

fn rendered() -> Option<std::rc::Rc<Value>> {
    let pass = crate::vehiclefacade::pass_number();
    let kept = pass.and_then(|number| RENDERED.with(|kept| kept.borrow().as_ref().filter(|(at, _)| *at == number).map(|(_, whole)| whole.clone())));
    kept.or_else(|| {
        let fresh = std::rc::Rc::new(object(synced().as_ref()?));
        pass.map(|number| RENDERED.with(|kept| kept.replace(Some((number, fresh.clone())))));
        Some(fresh)
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Watch {
    source: (u64, i64),
    decoded: (u64, i64),
}

impl Watch {
    pub fn fresh(now_ms: u64) -> Watch {
        Watch { source: (now_ms, 0), decoded: (now_ms, 0) }
    }
}

pub fn stalled(watch: Option<Watch>, source: i64, decoded: i64, timeout_s: u32, now_ms: u64) -> (Option<Watch>, bool) {
    let Some(watch) = watch else { return (None, false) };
    let moved = |(at, seen): (u64, i64), count: i64| if count != seen { (now_ms, count) } else { (at, seen) };
    let next = Watch { source: moved(watch.source, source), decoded: moved(watch.decoded, decoded) };
    let budget_ms = u64::from(timeout_s) * 1000;
    let quiet = |(at, _): (u64, i64)| now_ms.saturating_sub(at);
    let stall = timeout_s > 0 && (quiet(next.source) > budget_ms || (decoded > 0 && quiet(next.decoded) > 2 * budget_ms));
    (Some(next), stall)
}

fn report(host: &mut Host, channel: usize, running: bool, frames: i64, width: u32, height: u32, source: Option<i64>, restarted: bool, error: &str, now_ms: u64) {
    let receiver = CHANNEL_RECEIVERS[channel];
    let played = &mut host.channels[channel];
    if !played.wanted {
        return;
    }
    if restarted {
        played.progress = Some(Watch::fresh(now_ms));
    }
    let buffers = source.unwrap_or(frames);
    let (progress, stall) = stalled(played.progress, buffers, frames, played.timeout_s, now_ms);
    played.progress = progress;
    if stall {
        let uri = host.state.desired_uri(receiver);
        let problem = stream_problem(&uri, buffers, frames, error);
        log::warn!("Video channel {channel} stalled after {buffers} source buffers and {frames} frames: {problem}");
        played.problem = Some((uri.clone(), problem));
        played.wanted = false;
        played.progress = None;
        played.reported = (false, false, 0, 0);
        played.flowing = false;
        RESTART[channel].store(true, std::sync::atomic::Ordering::Relaxed);
        let outs: Vec<Out> = [host.state.on_decoding(receiver, false), host.state.on_streaming(receiver, false), host.state.on_stop_complete(receiver, Outcome::Failed)].into_iter().flatten().collect();
        apply(host, outs, now_ms);
        return;
    }
    let decoding = running && frames > 0;
    let flowing = running && source.is_none_or(|count| count > 0);
    if decoding {
        played.problem = None;
    } else if !error.is_empty() {
        let uri = host.state.desired_uri(receiver);
        played.problem = Some((uri.clone(), stream_problem(&uri, buffers, frames, error)));
    }
    let (before, was_flowing) = (played.reported, played.flowing);
    played.reported = (running, decoding, width, height);
    played.flowing = flowing;
    let outs: Vec<Out> = [
        (running && !before.0).then(|| host.state.on_start_complete(receiver, Outcome::Ok, now_ms / 1000)),
        (flowing != was_flowing).then(|| host.state.on_streaming(receiver, flowing)),
        (decoding != before.1).then(|| host.state.on_decoding(receiver, decoding)),
        ((width, height) != (before.2, before.3)).then(|| host.state.on_video_size(receiver, width, height)),
    ]
    .into_iter()
    .flatten()
    .flatten()
    .collect();
    apply(host, outs, now_ms);
}

fn place(uri: &str) -> String {
    let rest = uri.split_once("://").map_or(uri, |(_, rest)| rest);
    let authority = rest.split(['/', '?']).next().unwrap_or(rest);
    authority.rsplit_once('@').map_or(authority, |(_, host)| host).to_string()
}

fn path_of(uri: &str) -> String {
    let rest = uri.split_once("://").map_or(uri, |(_, rest)| rest);
    rest.find('/').map_or("/".to_string(), |at| rest[at..].to_string())
}

pub fn stream_problem(uri: &str, source: i64, decoded: i64, error: &str) -> String {
    let scheme = uri.split_once("://").map_or("", |(scheme, _)| scheme).to_ascii_lowercase();
    let at = place(uri);
    let host = at.rsplit_once(':').map_or(at.as_str(), |(host, _)| host).to_string();
    let said = error.split_whitespace().filter(|word| !word.contains("://")).collect::<Vec<_>>().join(" ").to_ascii_lowercase();
    let says = |words: &[&str]| words.iter().any(|word| said.contains(word));
    match () {
        _ if crate::videostate::device_camera(uri).is_some() && decoded > 0 => "This device's camera stopped.".to_string(),
        _ if crate::videostate::device_camera(uri).is_some() => "This device's camera would not open. Allow camera access, and close any other app using it.".to_string(),
        _ if decoded > 0 => format!("The stream from {at} stopped."),
        _ if source > 0 => format!("Data arrives from {at}, but it can't be shown as video."),
        _ if matches!(scheme.as_str(), "udp" | "udp265" | "mpegts") => format!("Nothing is arriving on UDP port {}. Check the drone is sending video to this device.", at.rsplit(':').next().unwrap_or(&at)),
        _ if says(&["404", "not found"]) => format!("{at} has no stream at {}. Check the path.", path_of(uri)),
        _ if says(&["401", "403", "unauthorized", "forbidden"]) => format!("{at} refused access to the stream."),
        _ if says(&["refused"]) => format!("Nothing is listening at {at}. Check the port."),
        _ if says(&["dns", "lookup", "resolve", "unknown host", "name or service"]) => format!("Can't find {host}. Check the address."),
        _ => format!("No answer from {at}. Check the address and that this device is on the drone's network."),
    }
}

pub fn invoke(path: &str, args: &str) -> Option<Value> {
    served().then_some(())?;
    match path {
        "video.startRecording" => {
            return Some(match start_recording() {
                Ok(()) => json!({ "ok": true }),
                Err(reason) => json!({ "ok": false, "reason": reason }),
            });
        }
        "video.stopRecording" => {
            stop_recording();
            return Some(json!({ "ok": true }));
        }
        _ => {}
    }
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let mut guard = synced();
    let host = guard.as_mut()?;
    let write_active = |index: usize| crate::settingsstore::written("Video/activeVideoSource", &index.to_string());
    match path {
        "video.cameraName" => {
            let slot = given.get(0).and_then(Value::as_i64).and_then(|i| usize::try_from(i).ok()).unwrap_or(0);
            Some(json!({ "ok": true, "result": host.state.settings.name_at(slot).unwrap_or("") }))
        }
        "video.setActiveVideoSource" => {
            if let Some(index) = host.state.set_active_source(given.get(0).and_then(Value::as_i64).unwrap_or(0)) {
                write_active(index);
            }
            Some(json!({ "ok": true }))
        }
        "video.switchActiveVideoSource" => {
            if let Some(index) = host.state.settings.next_switchable() {
                write_active(index);
            }
            Some(json!({ "ok": true }))
        }
        "video.setNativeRendering" | "video.initNative" => Some(json!({ "ok": true })),
        SET_PIP_SHOWN => {
            show_pip(host, given.get(0).and_then(Value::as_bool).unwrap_or(false), crate::hub::now_ms());
            Some(json!({ "ok": true }))
        }
        SET_DEVICE_CAMERA_ROTATION => {
            host.camera_rotation = given.get(0).and_then(Value::as_u64).map_or(0, |degrees| (degrees % 360) as u32);
            Some(json!({ "ok": true }))
        }
        "video.restart" => {
            RESTART[MAIN_CHANNEL].store(true, std::sync::atomic::Ordering::Relaxed);
            if !host.channels[MAIN_CHANNEL].wanted {
                host.channels[MAIN_CHANNEL].restart_at_ms = None;
                let outs = host.state.start_receiver(MAIN_RECEIVER);
                apply(host, outs, crate::hub::now_ms());
            }
            Some(json!({ "ok": true }))
        }
        "video.reportRecording" => {
            let active = given.get(0).and_then(Value::as_bool).unwrap_or(false);
            let outs = host.state.on_recording(MAIN_RECEIVER, active, crate::hub::now_ms() / 1000);
            if !active {
                host.recording_file = None;
            }
            apply(host, outs, crate::hub::now_ms());
            Some(json!({ "ok": true }))
        }
        "video.reportNative" => {
            let number = |i: usize| given.get(i).and_then(Value::as_i64).unwrap_or(0);
            let size = |i: usize| u32::try_from(number(i)).unwrap_or(0);
            match usize::try_from(number(7)).ok().filter(|channel| *channel < VIDEO_CHANNELS) {
                Some(channel) => {
                    let running = given.get(0).and_then(Value::as_bool).unwrap_or(false);
                    if channel == MAIN_CHANNEL {
                        crate::videostats::sample(running, number(1), number(3), crate::hub::now_ms());
                    }
                    report(host, channel, running, number(1), size(2), size(3), given.get(5).and_then(Value::as_i64), given.get(6).and_then(Value::as_bool).unwrap_or(false), given.get(4).and_then(Value::as_str).unwrap_or_default(), crate::hub::now_ms());
                    Some(json!({ "ok": true }))
                }
                None => Some(json!({ "ok": false, "reason": "no such video channel" })),
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_vehicles_stops_the_old_cameras_stream_and_resumes_the_new_one_like_video_manager() {
        assert_eq!(stream_switches(Some(1), Some(2), &[Out::StopCameraStream, Out::StartCameraStream]), vec![(1, false), (2, true)]);
        assert_eq!(stream_switches(None, Some(2), &[Out::StartCameraStream]), vec![(2, true)]);
        assert_eq!(stream_switches(Some(1), None, &[Out::StopCameraStream]), vec![(1, false)]);
    }

    #[test]
    fn an_address_no_pipeline_can_be_built_for_fails_like_gst_source_factory() {
        assert_eq!(unbuildable_outcome(""), Outcome::InvalidUrl, "an empty address is GstVideoReceiver's Invalid stream URL");
        assert_eq!(unbuildable_outcome("udp://host-without-port"), Outcome::Failed, "anything else is a failed start, retried");
        assert!(pipeline("rtspt://cam/main", 80, false).is_some(), "rtspt forces RTSP over TCP and GstSourceFactory accepts any rtsp scheme");
    }

    #[test]
    fn a_grabbed_frame_drops_row_padding_and_lands_as_a_dated_jpeg() {
        let padded = [1, 2, 3, 4, 9, 9, 5, 6, 7, 8, 9, 9];
        assert_eq!(tight_rgba(&padded, 1, 2, 6), Some(vec![1, 2, 3, 4, 5, 6, 7, 8]));
        assert_eq!(tight_rgba(&padded, 2, 2, 6), None, "a stride shorter than the row is not a frame");
        assert_eq!(photo_file_name("/data/Photo/", "2026-10-03_12.00.00.000"), "/data/Photo/2026-10-03_12.00.00.000.jpg");
    }

    #[test]
    fn a_recording_is_named_and_trimmed_as_video_manager_does() {
        assert_eq!(recording_file_name("/v", "2026-10-01_14.30.00", 2).as_deref(), Some("/v/2026-10-01_14.30.00.mp4"));
        assert_eq!(recording_file_name("/v", "s", 0).as_deref(), Some("/v/s.mkv"));
        assert_eq!(recording_file_name("/v", "s", 3), None, "VideoReceiver::isValidFileFormat");
        let at = |s: u64| std::time::UNIX_EPOCH + std::time::Duration::from_secs(s);
        let videos = vec![("old".to_string(), 6, at(1)), ("new".to_string(), 6, at(3)), ("mid".to_string(), 6, at(2))];
        assert_eq!(videos_to_delete(videos.clone(), 10), vec!["old".to_string(), "mid".to_string()], "oldest first until the total is under the limit");
        assert!(videos_to_delete(videos, 100).is_empty());
    }

    #[test]
    fn a_stream_that_stops_delivering_is_a_failure_as_the_receiver_watchdog_says() {
        let start = Some(Watch::fresh(0));
        assert_eq!(stalled(None, 0, 0, 3, 99_000), (None, false), "nothing started, nothing to watch");
        assert!(!stalled(start, 0, 0, 3, 3_000).1);
        assert!(stalled(start, 0, 0, 3, 3_001).1, "no source data inside the start budget");
        let (flowing, stall) = stalled(start, 40, 0, 3, 3_001);
        assert!(!stall, "data arriving keeps a stream waiting for its first keyframe alive");
        assert!(!stalled(flowing, 80, 0, 3, 9_000).1, "the decoder is only held to a budget once it has produced a frame");
        let (decoding, _) = stalled(flowing, 90, 5, 3, 4_000);
        assert!(!stalled(decoding, 200, 5, 3, 10_000).1 && stalled(decoding, 300, 5, 3, 10_001).1, "once decoding, the decoder gets twice the budget");
    }

    #[test]
    fn a_stalled_stream_says_why_in_words_an_operator_can_act_on() {
        let whep = "http://192.168.1.50:8889/cam/whep";
        assert_eq!(stream_problem(whep, 0, 0, ""), "No answer from 192.168.1.50:8889. Check the address and that this device is on the drone's network.");
        assert_eq!(stream_problem(whep, 0, 0, "GStreamer encountered a general resource error. whepsrc: Unexpected response: 404 - no such stream"), "192.168.1.50:8889 has no stream at /cam/whep. Check the path.");
        assert_eq!(stream_problem("http://10.0.0.5:4040/whep", 0, 0, "error sending request for url (http://10.0.0.5:4040/whep)"), "No answer from 10.0.0.5:4040. Check the address and that this device is on the drone's network.", "the address inside the error is not the answer");
        assert_eq!(stream_problem(whep, 0, 0, "error sending request: Connection refused (os error 111)"), "Nothing is listening at 192.168.1.50:8889. Check the port.");
        assert_eq!(stream_problem("rtsp://user:pw@cam.local:554/live", 0, 0, "dns error: failed to lookup address"), "Can't find cam.local. Check the address.");
        assert_eq!(stream_problem("rtsp://cam:554/live", 0, 0, "Unauthorized (401)"), "cam:554 refused access to the stream.");
        assert_eq!(stream_problem("udp://0.0.0.0:5600", 0, 0, ""), "Nothing is arriving on UDP port 5600. Check the drone is sending video to this device.");
        assert_eq!(stream_problem(whep, 40, 0, ""), "Data arrives from 192.168.1.50:8889, but it can't be shown as video.");
        assert_eq!(stream_problem(whep, 900, 30, ""), "The stream from 192.168.1.50:8889 stopped.");
    }

    #[test]
    fn the_drones_streams_join_the_operators_cameras_on_addresses_a_pipeline_can_open() {
        let mine = Settings { cameras: vec![SourceSlot { source: crate::videostate::SOURCE_RTSP.to_string(), url: "rtsp://10.0.0.5:8554/front".to_string(), name: "Front".to_string(), drone: false }], ..Settings::default() };
        let announced = vec![
            ("SIYI A8".to_string(), crate::videostate::STREAM_TYPE_RTP_UDP, crate::videostate::ENCODING_H265, "5600".to_string()),
            ("Gimbal".to_string(), crate::videostate::STREAM_TYPE_RTSP, 0, "rtsp://192.168.144.25:8554/main".to_string()),
            ("Belly".to_string(), crate::videostate::STREAM_TYPE_TCP_MPEG, 0, "1.2.3.4:5600".to_string()),
            ("Odd".to_string(), 200, 0, "garbage-uri".to_string()),
        ];
        let joined = with_drone(mine, &announced);
        assert_eq!(joined.cameras.len(), 4, "a stream of a type nobody knows is never started on the text the air vehicle sent");
        assert_eq!(joined.cameras[0].name, "Front", "the operator's own cameras keep their places ahead of the drone's");
        assert!(joined.cameras[1..].iter().all(|camera| camera.drone));
        assert_eq!(crate::videostate::source_uri(&joined.cameras[1].source, &joined.cameras[1].url), "udp265://0.0.0.0:5600", "a bare port becomes a listen address, and the scheme is not doubled");
        assert_eq!(crate::videostate::source_uri(&joined.cameras[2].source, &joined.cameras[2].url), "rtsp://192.168.144.25:8554/main");
        assert_eq!(crate::videostate::source_uri(&joined.cameras[3].source, &joined.cameras[3].url), "tcp://1.2.3.4:5600");
    }

    #[test]
    fn a_uri_becomes_a_pipeline_ending_in_the_native_sink() {
        assert_eq!(pipeline("udp://0.0.0.0:5600", 80, false).unwrap(), "udpsrc address=0.0.0.0 port=5600 buffer-size=8388608 caps=\"application/x-rtp, media=(string)video, clock-rate=(int)90000, encoding-name=(string)H264\" ! rtpjitterbuffer latency=80 do-lost=true do-retransmission=true drop-on-latency=true rtx-delay=25 rtx-max-retries=1 ! tee name=nativerec ! queue ! decodebin3 ! videoconvert ! appsink name=nativesink sync=true");
        assert_eq!(pipeline("udp://0.0.0.0:5600", 80, true).unwrap(), "udpsrc address=0.0.0.0 port=5600 buffer-size=8388608 caps=\"application/x-rtp, media=(string)video, clock-rate=(int)90000, encoding-name=(string)H264\" ! tee name=nativerec ! queue ! decodebin3 ! videoconvert ! appsink name=nativesink sync=false", "low latency drops the jitter buffer and the clock sync, as GstVideoReceiver does with _buffer -1");
        assert_eq!(pipeline("rtsp://cam/main", 20, false).unwrap(), "rtspsrc location=\"rtsp://cam/main\" latency=20 do-rtcp=true do-retransmission=false drop-on-latency=true ! tee name=nativerec ! queue ! decodebin3 ! videoconvert ! appsink name=nativesink sync=true", "retransmission needs 40 ms of headroom");
        assert!(pipeline("whep://sfu/whep/x", 80, false).unwrap().starts_with("whepsrc name=whep_latency_80 whep-endpoint=\"http://sfu/whep/x\""), "the native side reads the webrtcbin latency from the source's name");
        assert!(pipeline("whep://sfu/whep/x", 80, true).unwrap().starts_with("whepsrc name=whep_latency_40 "), "buildWhepSource uses 40 ms when the jitter buffer is off");
        assert!(pipeline("whep://sfu/whep/x", 80, false).unwrap().contains("encoding-name=(string)H265,payload=(int)97") && pipeline("whep://sfu/whep/x", 80, false).unwrap().contains("audio-caps=EMPTY timeout=8"), "buildWhepSource offers H.264 and H.265 video only");
        assert!(pipeline("mpegts://0.0.0.0:5600", 80, false).unwrap().contains("tsdemux ! capsfilter caps=\"video/x-h264;video/x-h265\" ! tee"), "only a video pad of the transport stream reaches the tee");
        assert_eq!(crate::videostate::source_uri(crate::videostate::SOURCE_WEBRTC, "sfu.host/whep/x"), "http://sfu.host/whep/x", "QUrl::fromUserInput supplies the scheme");
        assert_eq!(pipeline("bogus", 80, false), None);
    }

    #[test]
    fn a_device_camera_is_read_raw_turned_upright_and_never_decoded() {
        let back = crate::videostate::source_uri(crate::videostate::SOURCE_BACK_CAMERA, "");
        assert_eq!(pipeline(&oriented(&back, 0), 80, false).unwrap(), "ahcsrc device=0 ! tee name=nativerec ! queue ! glupload ! glcolorconvert ! glvideoflip method=none ! videoconvert ! appsink name=nativesink sync=false");
        let front = crate::videostate::source_uri(crate::videostate::SOURCE_FRONT_CAMERA, "");
        assert!(pipeline(&oriented(&front, 270), 80, false).unwrap().starts_with("ahcsrc device=1 ! tee name=nativerec ! queue ! glupload ! glcolorconvert ! glvideoflip method=counterclockwise ! "));
        assert!(pipeline(&oriented(&back, 90), 80, false).unwrap().contains("method=clockwise"));
        assert!(pipeline(&oriented(&back, 180), 80, false).unwrap().contains("method=rotate-180"));
        assert_eq!(oriented("rtsp://cam/main", 90), "rtsp://cam/main", "only a device camera is turned");
        assert_eq!(pipeline("ahc://front", 80, false), None, "the camera is picked by index");
        assert_eq!(stream_problem(&back, 0, 0, "the pipeline refused to play"), "This device's camera would not open. Allow camera access, and close any other app using it.");
    }

    #[test]
    fn the_main_receiver_is_started_and_reports_drive_the_camera_status() {
        let settings = settings_from(
            &|name| match name {
                "cameras" => r#"[{"name":"","source":"UDP h.264 Video Stream","url":"0.0.0.0:5600"}]"#.to_string(),
                _ => String::new(),
            },
            &|_, default| default,
            &|_, default| default,
        );
        let mut host = Host::default();
        let outs = host.state.on_settings(settings);
        apply(&mut host, outs, 0);
        let outs = host.state.register_receiver(MAIN_RECEIVER);
        apply(&mut host, outs, 0);
        assert!(host.channels[MAIN_CHANNEL].wanted, "a configured stream starts the main receiver");
        assert_eq!(status_text(host.state.receiver_status(0)), "Connecting\u{2026}");
        report(&mut host, MAIN_CHANNEL, true, 0, 0, 0, Some(0), false, "", 0);
        assert_eq!(status_text(host.state.receiver_status(0)), "Connecting\u{2026}", "a pipeline that has received nothing is still connecting, not connected");
        report(&mut host, MAIN_CHANNEL, true, 0, 0, 0, Some(4), false, "", 0);
        assert_eq!(status_text(host.state.receiver_status(0)), "Connected, waiting for frames");
        assert_eq!(object(&host)["cameraSignals"], json!(["connecting"]));
        report(&mut host, MAIN_CHANNEL, true, 12, 640, 360, Some(12), false, "", 0);
        assert_eq!((host.state.decoding, host.state.video_size), (true, Some((640, 360))));
        assert_eq!(status_text(host.state.receiver_status(0)), "");
        assert_eq!(object(&host)["cameraSignals"], json!(["live"]), "the head reads one token per camera beside the sentence");
        assert_eq!(status_text(host.state.receiver_status(1)), "No video source", "a camera with no receiver is no video source, as VideoManager::_cameraStatus says");
    }

    fn hosted(cameras: &str, active: i64, multi_view: bool) -> Host {
        let mut host = Host::default();
        let outs = host.state.on_settings(Settings { multi_view, pip_shown: true, ..settings_from(&|name| if name == "cameras" { cameras.to_string() } else { String::new() }, &|_, default| default, &|name, default| if name == "activeVideoSource" { active } else { default }) });
        apply(&mut host, outs, 0);
        CHANNEL_RECEIVERS.iter().for_each(|name| {
            let outs = host.state.register_receiver(name);
            apply(&mut host, outs, 0);
        });
        host
    }

    fn set_multi_view(host: &mut Host, multi_view: bool) {
        let outs = host.state.on_settings(Settings { multi_view, ..host.state.settings.clone() });
        apply(host, outs, 0);
    }

    fn set_stream(host: &mut Host, stream_enabled: bool, now_ms: u64) {
        let outs = host.state.on_settings(Settings { stream_enabled, ..host.state.settings.clone() });
        apply(host, outs, now_ms);
    }

    const THREE: &str = r#"[{"name":"Front","source":"UDP h.264 Video Stream","url":"0.0.0.0:5600"},{"name":"Belly","source":"UDP h.265 Video Stream","url":"0.0.0.0:5601"},{"name":"Tail","source":"UDP h.264 Video Stream","url":"0.0.0.0:5602"}]"#;

    #[test]
    fn with_multi_view_on_the_picture_in_picture_camera_plays_on_the_second_channel_and_nothing_else_starts() {
        let mut host = hosted(THREE, 0, false);
        assert_eq!(pipeline_for(&host, 1), None, "a single view plays nothing on the second channel");
        set_multi_view(&mut host, true);
        assert!(pipeline_for(&host, MAIN_CHANNEL).is_some_and(|played| played.contains("port=5600")));
        assert!(pipeline_for(&host, 1).is_some_and(|played| played.contains("port=5601")), "the camera after the one on screen plays on channel 1");
        assert_eq!(object(&host)["cameraSignals"], json!(["connecting", "connecting", "idle"]), "the third camera is never started");
        [(0, "port=5601"), (1, "port=5602"), (2, "port=5600")].iter().for_each(|(active, port)| {
            let on = hosted(THREE, *active, true);
            assert!(pipeline_for(&on, 1).is_some_and(|played| played.contains(port)), "active {active}: the next camera round plays in the picture in picture");
            let pip = on.state.settings.camera_index_for_receiver(PIP_RECEIVER);
            let view = crate::cameras::view_of(&object(&on), crate::cameras::parse(THREE), *active, true);
            assert_eq!(view["pip"]["slot"].as_u64().map(|slot| slot as usize), pip, "active {active}: view.cameras names the camera channel 1 plays");
        });
        set_multi_view(&mut host, false);
        assert_eq!(pipeline_for(&host, 1), None, "multi view off stops the second channel");
        assert!(pipeline_for(&host, MAIN_CHANNEL).is_some(), "and leaves the main one playing");
    }

    #[test]
    fn every_camera_signal_is_read_from_what_its_receiver_reports() {
        let mut host = hosted(THREE, 0, true);
        let signals = |host: &Host| object(host)["cameraSignals"].clone();
        assert_eq!(signals(&host), json!(["connecting", "connecting", "idle"]), "a started receiver is connecting and a camera nobody plays is idle");
        report(&mut host, MAIN_CHANNEL, true, 12, 640, 360, Some(12), false, "", 1_000);
        assert_eq!(signals(&host), json!(["live", "connecting", "idle"]), "frames arriving are live");
        report(&mut host, 1, true, 0, 0, 0, Some(0), false, "", 10_000);
        assert_eq!(signals(&host), json!(["live", "noSignal", "idle"]), "a stream that delivers nothing inside its budget has stalled");
        assert!(host.channels[1].restart_at_ms.is_some(), "and is retried");
        let outs = host.state.start_receiver(PIP_RECEIVER);
        apply(&mut host, outs, 11_000);
        assert_eq!(signals(&host), json!(["live", "noSignal", "idle"]), "it stays no signal through the retry");
        report(&mut host, 1, true, 4, 320, 240, Some(4), true, "", 11_500);
        assert_eq!(signals(&host), json!(["live", "live", "idle"]), "until frames arrive again");
        set_multi_view(&mut host, false);
        assert_eq!(signals(&host), json!(["live", "idle", "idle"]), "a camera taken out of the picture in picture is idle");
    }

    #[test]
    fn the_picture_in_picture_pipeline_is_served_beside_the_main_one_and_null_when_nothing_plays_there() {
        let mut host = hosted(THREE, 0, true);
        assert_eq!(object(&host)["pipPipeline"], json!(pipeline_for(&host, PIP_CHANNEL)));
        assert!(object(&host)["pipPipeline"].as_str().is_some_and(|played| played.contains("port=5601")), "the headless macOS head starts channel 1 from this field");
        report(&mut host, PIP_CHANNEL, true, 0, 0, 0, None, false, "", 3_001);
        assert_eq!(object(&host)["pipPipeline"], Value::Null, "a stalled picture in picture is taken down so the head stops it");
        restart_due(&mut host, 5_000);
        assert!(object(&host)["pipPipeline"].is_string(), "and served again when it is retried");
        set_multi_view(&mut host, false);
        assert_eq!((object(&host)["pipPipeline"].clone(), object(&host)["nativePipeline"].is_string()), (Value::Null, true));
    }

    #[test]
    fn a_head_that_counts_no_source_buffers_brings_the_picture_in_picture_live() {
        let signal = |host: &Host| object(host)["cameraSignals"][1].clone();
        let mut host = hosted(THREE, 0, true);
        report(&mut host, PIP_CHANNEL, true, 0, 0, 0, None, false, "", 500);
        assert_eq!(signal(&host), json!("connecting"));
        report(&mut host, PIP_CHANNEL, true, 1, 320, 240, None, false, "", 4_500);
        assert_eq!(signal(&host), json!("live"), "a head that counts no source buffers can see its first frame land after the start budget, and that frame is the data it saw");
        let mut silent = hosted(THREE, 0, true);
        report(&mut silent, PIP_CHANNEL, true, 0, 0, 0, None, false, "", 3_001);
        assert_eq!((signal(&silent), silent.channels[PIP_CHANNEL].wanted), (json!("noSignal"), false), "nothing decoded inside the budget is still a stream that delivers nothing");
    }

    #[test]
    fn a_decoding_main_channel_is_never_restarted_while_the_picture_in_picture_reports() {
        let mut host = hosted(THREE, 0, true);
        let main = pipeline_for(&host, MAIN_CHANNEL);
        let run: Vec<(u64, bool, bool)> = (1..=200u64)
            .map(|tick| {
                let now = tick * 500;
                restart_due(&mut host, now);
                if tick == 140 || tick == 160 {
                    let outs = host.state.on_settings(Settings { multi_view: tick == 160, ..host.state.settings.clone() });
                    apply(&mut host, outs, now);
                }
                report(&mut host, MAIN_CHANNEL, true, (tick * 15) as i64, 640, 360, Some((tick * 20) as i64), tick == 1, "", now);
                let (frames, source) = match tick as i64 {
                    early @ 1..=30 => (0, early / 4),
                    playing @ 31..=80 => (playing, playing * 2),
                    _ => (80, 160),
                };
                let pip_running = host.channels[PIP_CHANNEL].wanted;
                report(&mut host, PIP_CHANNEL, pip_running, frames, 320, 240, Some(source), false, "", now);
                let steady = host.channels[MAIN_CHANNEL].wanted && host.channels[MAIN_CHANNEL].restart_at_ms.is_none() && pipeline_for(&host, MAIN_CHANNEL) == main && host.state.decoding;
                (tick, steady, host.channels[PIP_CHANNEL].wanted)
            })
            .collect();
        let disturbed: Vec<u64> = run.iter().filter(|(_, steady, _)| !steady).map(|(tick, ..)| *tick).collect();
        assert!(disturbed.is_empty(), "the main channel was stopped, rescheduled or handed a new pipeline at ticks {disturbed:?}");
        assert!(run.iter().any(|(.., pip)| !pip) && run.iter().any(|(.., pip)| *pip), "and the picture in picture really stalled, was retried and played in the same run");
        assert_eq!(object(&host)["cameraSignals"][0], json!("live"));
    }

    #[test]
    fn switching_the_stream_off_for_seconds_and_back_on_plays_both_cameras_again() {
        let mut host = hosted(THREE, 0, true);
        report(&mut host, MAIN_CHANNEL, true, 12, 640, 360, Some(12), false, "", 1_000);
        report(&mut host, PIP_CHANNEL, true, 12, 320, 240, Some(12), false, "", 1_000);
        set_stream(&mut host, false, 2_000);
        assert_eq!((pipeline_for(&host, MAIN_CHANNEL), pipeline_for(&host, PIP_CHANNEL)), (None, None), "off stops both channels");
        restart_due(&mut host, 7_000);
        set_stream(&mut host, true, 7_000);
        assert!(pipeline_for(&host, MAIN_CHANNEL).is_some_and(|played| played.contains("port=5600")), "the retry the stop scheduled was spent while video was off, so turning it on has to start the main camera itself");
        assert!(pipeline_for(&host, PIP_CHANNEL).is_some_and(|played| played.contains("port=5601")), "and the picture in picture");
    }

    #[test]
    fn a_late_report_from_a_pipeline_already_taken_down_is_ignored() {
        let mut host = hosted(THREE, 0, true);
        report(&mut host, PIP_CHANNEL, true, 0, 0, 0, Some(0), false, "", 3_001);
        assert!(!host.channels[PIP_CHANNEL].wanted, "a stalled channel is taken down");
        report(&mut host, PIP_CHANNEL, true, 30, 320, 240, Some(30), false, "", 3_200);
        assert!(!host.state.started_receivers().contains(&PIP_RECEIVER.to_string()), "the old pipeline's last word cannot mark a stopped channel started");
        assert_eq!(object(&host)["cameraSignals"][1], json!("noSignal"));
        restart_due(&mut host, 4_001);
        report(&mut host, PIP_CHANNEL, true, 4, 320, 240, Some(4), true, "", 4_500);
        assert_eq!(object(&host)["cameraSignals"][1], json!("live"), "the restarted pipeline's own reports count again");
    }

    #[test]
    fn the_picture_in_picture_plays_only_while_a_head_shows_it_and_the_stream_is_on() {
        let mut host = hosted(THREE, 0, true);
        assert!(pipeline_for(&host, PIP_CHANNEL).is_some());
        show_pip(&mut host, false, 100);
        assert_eq!(pipeline_for(&host, PIP_CHANNEL), None, "a thumbnail no head shows decodes nothing");
        assert!(pipeline_for(&host, MAIN_CHANNEL).is_some());
        assert_eq!(object(&host)["cameraSignals"], json!(["connecting", "idle", "idle"]));
        assert_eq!(object(&host)["pipSlot"], json!(1), "the camera the thumbnail would show is served shown or not, so a head knows what it would offer");
        show_pip(&mut host, true, 200);
        assert!(pipeline_for(&host, PIP_CHANNEL).is_some_and(|played| played.contains("port=5601")));
        set_stream(&mut host, false, 300);
        assert_eq!(pipeline_for(&host, PIP_CHANNEL), None, "the stream switch takes the picture in picture with it");
        set_stream(&mut host, true, 400);
        assert!(pipeline_for(&host, PIP_CHANNEL).is_some());
        set_multi_view(&mut host, false);
        assert_eq!((pipeline_for(&host, PIP_CHANNEL), object(&host)["pipSlot"].clone()), (None, json!(1)));
        let single = hosted(r#"[{"name":"Front","source":"UDP h.264 Video Stream","url":"0.0.0.0:5600"}]"#, 0, true);
        assert_eq!((pipeline_for(&single, PIP_CHANNEL), object(&single)["pipSlot"].clone()), (None, Value::Null), "one camera has nothing to put beside itself");
    }

    #[test]
    fn the_native_channels_and_abi_are_the_numbers_the_c_header_defines() {
        assert_eq!((MAIN_CHANNEL, PIP_CHANNEL, VIDEO_CHANNELS, VIDEO_ABI_VERSION), (0, 1, 2, 3));
        let header = include_str!("../../src/Bridge/QGCVideoC.h");
        ["#define QGC_VIDEO_MAIN 0", "#define QGC_VIDEO_PIP 1", "#define QGC_VIDEO_CHANNELS 2", "int qgc_video_abi_version(void);"].iter().for_each(|line| assert!(header.contains(line), "QGCVideoC.h no longer says {line}"));
        assert_eq!(CHANNEL_RECEIVERS, [MAIN_RECEIVER, PIP_RECEIVER]);
    }
}
