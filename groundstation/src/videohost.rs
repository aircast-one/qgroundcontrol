use std::sync::{Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};

use crate::videostate::{MAIN_RECEIVER, Out, Outcome, PrimaryUrls, Settings, SourceSlot, Status, TILE_RECEIVER_PREFIX, VideoState, source_uri};

const RECORD_TEE: &str = "tee name=nativerec ! queue";
#[cfg(not(target_os = "android"))]
const NATIVE_SINK: &str = "videoconvert ! appsink name=nativesink sync=false";
#[cfg(target_os = "android")]
const NATIVE_SINK: &str = "glupload ! glcolorconvert ! video/x-raw(memory:GLMemory),format=RGBA,texture-target=2D ! gldownload ! videoconvert ! appsink name=nativesink sync=false";

#[derive(Default)]
struct Host {
    state: VideoState,
    registered: usize,
    wanted: bool,
    restart_at_ms: Option<u64>,
    reported: (bool, bool, u32, u32),
    recording_file: Option<String>,
}

const FILE_EXTENSIONS: [&str; 3] = ["mkv", "mov", "mp4"];
const DEFAULT_RECORDING_FORMAT: i64 = 2;
const DEFAULT_MAX_VIDEO_MB: u64 = 10240;
const BAD_FORMAT_MESSAGE: &str = "Invalid video format defined.";
const NO_SAVE_PATH_MESSAGE: &str = "Unabled to record video. Video save path must be specified in Settings.";

static HOST: Mutex<Option<Host>> = Mutex::new(None);

fn setting(name: &str) -> Value {
    crate::settingsstore::raw_setting(&format!("settings.videoSettings.{name}")).unwrap_or(Value::Null)
}

fn text(name: &str) -> String {
    setting(name).as_str().unwrap_or_default().trim().to_string()
}

pub fn settings_from(text: &dyn Fn(&str) -> String, flag: &dyn Fn(&str, bool) -> bool, number: &dyn Fn(&str, i64) -> i64) -> Settings {
    let extras = serde_json::from_str::<Value>(&text("extraVideoSources")).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
    let field = |entry: &Value, key: &str| entry.get(key).and_then(Value::as_str).unwrap_or_default().trim().to_string();
    Settings {
        primary_source: text("videoSource"),
        primary_name: text("primaryCameraName"),
        primary_urls: PrimaryUrls { udp: text("udpUrl"), rtsp: text("rtspUrl"), tcp: text("tcpUrl"), whep: text("whepUrl") },
        extras: extras.iter().map(|entry| SourceSlot { source: field(entry, "source"), url: field(entry, "url"), name: field(entry, "name") }).collect(),
        active_source: number("activeVideoSource", 0),
        multi_view: flag("multiViewEnabled", false),
        stream_enabled: flag("streamEnabled", true),
        low_latency: flag("lowLatencyMode", false),
        save_path_set: false,
        recording_format_valid: false,
        rtsp_timeout_s: u32::try_from(number("rtspTimeout", 8)).unwrap_or(8),
    }
}

fn stored_settings() -> Settings {
    let settings = settings_from(&text, &|name, default| setting(name).as_bool().unwrap_or(default), &|name, default| setting(name).as_i64().unwrap_or(default));
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
    let mut guard = synced();
    let host = guard.as_mut().ok_or(None)?;
    match host.state.record_refusal() {
        Some(crate::videostate::REFUSED_BAD_FORMAT) => return Err(Some(BAD_FORMAT_MESSAGE)),
        Some(crate::videostate::REFUSED_NO_SAVE_PATH) => return Err(Some(NO_SAVE_PATH_MESSAGE)),
        Some(_) => return Err(None),
        None => {}
    }
    let folder = crate::settingsstore::video_save_path().ok_or(Some(NO_SAVE_PATH_MESSAGE))?;
    cleanup_old_videos(&folder);
    let _ = std::fs::create_dir_all(&folder);
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H.%M.%S").to_string();
    let file = recording_file_name(&folder, &stamp, recording_format()).ok_or(Some(BAD_FORMAT_MESSAGE))?;
    let outs = host.state.start_recording();
    if outs.iter().any(|out| matches!(out, Out::StartRecording { receivers } if receivers.iter().any(|r| r == MAIN_RECEIVER))) {
        host.recording_file = Some(file);
    }
    Ok(())
}

pub fn stop_recording() {
    let mut guard = synced();
    if let Some(host) = guard.as_mut() {
        host.recording_file = None;
        let outs = host.state.stop_recording();
        apply(host, outs, crate::hub::now_ms());
    }
}

pub fn recording() -> bool {
    get("video.recording").and_then(|v| v.get("value")?.as_bool()).unwrap_or(false) || HOST.lock().unwrap_or_else(PoisonError::into_inner).as_ref().is_some_and(|host| host.recording_file.is_some())
}

fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn host_port(rest: &str) -> Option<(String, u16)> {
    let authority = rest.split(['/', '?']).next()?;
    let (host, port) = authority.rsplit_once(':')?;
    Some((if host.is_empty() { "0.0.0.0".to_string() } else { host.to_string() }, port.parse().ok()?))
}

pub fn pipeline(uri: &str, latency_ms: i64) -> Option<String> {
    let rtp = |encoding: &str, rest: &str| {
        let (host, port) = host_port(rest)?;
        Some(format!(
            "udpsrc address={host} port={port} caps=\"application/x-rtp, media=(string)video, clock-rate=(int)90000, encoding-name=(string){encoding}\" ! rtpjitterbuffer latency={latency_ms}"
        ))
    };
    let source = match uri.split_once("://")? {
        ("rtsp" | "rtsps", _) => format!("rtspsrc location={} latency={latency_ms}", quoted(uri)),
        ("udp", rest) => rtp("H264", rest)?,
        ("udp265", rest) => rtp("H265", rest)?,
        ("mpegts", rest) => {
            let (host, port) = host_port(rest)?;
            format!("udpsrc address={host} port={port} ! tsdemux")
        }
        ("tcp", rest) => {
            let (host, port) = host_port(rest)?;
            format!("tcpclientsrc host={host} port={port} ! tsdemux")
        }
        ("http" | "https" | "whep" | "wheps", _) => {
            let endpoint = uri.replacen("wheps://", "https://", 1).replacen("whep://", "http://", 1);
            format!("whepsrc whep-endpoint={}", quoted(&endpoint))
        }
        _ => return None,
    };
    Some(format!("{source} ! {RECORD_TEE} ! decodebin3 ! {NATIVE_SINK}"))
}

fn apply(host: &mut Host, outs: Vec<Out>, now_ms: u64) {
    let follow_ups: Vec<Out> = outs
        .into_iter()
        .flat_map(|out| match out {
            Out::StartReceiver { receiver, .. } if receiver == MAIN_RECEIVER => {
                host.wanted = true;
                host.restart_at_ms = None;
                Vec::new()
            }
            Out::StopReceiver { receiver } if receiver == MAIN_RECEIVER => {
                host.wanted = false;
                host.reported = (false, false, 0, 0);
                host.state.on_stop_complete(MAIN_RECEIVER, Outcome::Ok)
            }
            Out::RestartAfter { receiver, delay_ms } if receiver == MAIN_RECEIVER => {
                host.restart_at_ms = Some(now_ms + delay_ms);
                Vec::new()
            }
            _ => Vec::new(),
        })
        .collect();
    if !follow_ups.is_empty() {
        apply(host, follow_ups, now_ms);
    }
}

fn synced() -> MutexGuard<'static, Option<Host>> {
    let mut guard = HOST.lock().unwrap_or_else(PoisonError::into_inner);
    let host = guard.get_or_insert_with(Host::default);
    let now_ms = crate::hub::now_ms();
    let settings = stored_settings();
    if settings != host.state.settings {
        let outs = host.state.on_settings(settings);
        apply(host, outs, now_ms);
    }
    let receivers = host.state.settings.count();
    if host.registered < receivers {
        let names: Vec<String> = (host.registered..receivers).map(|index| if index == 0 { MAIN_RECEIVER.to_string() } else { format!("{TILE_RECEIVER_PREFIX}{}", index - 1) }).collect();
        host.registered = receivers;
        names.iter().for_each(|name| {
            let outs = host.state.register_receiver(name);
            apply(host, outs, now_ms);
        });
    }
    if host.restart_at_ms.is_some_and(|at| now_ms >= at) {
        host.restart_at_ms = None;
        let outs = host.state.start_receiver(MAIN_RECEIVER);
        apply(host, outs, now_ms);
    }
    guard
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
    let latency = setting("rtpJitterLatencyMs").as_i64().unwrap_or(80);
    let uri = state.main_camera().map(|main| source_uri(settings.source_at(main), state.url_at(main))).unwrap_or_default();
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
        "cameraRecording": (0..settings.count()).map(|i| state.camera_recording(i)).collect::<Vec<_>>(),
        "nativePipeline": (host.wanted && state.has_video()).then(|| pipeline(&uri, latency)).flatten(),
        "nativeRecording": host.recording_file,
        "nativeRecordingFormat": recording_format(),
    })
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

pub fn native_pipeline() -> Option<String> {
    get("video.nativePipeline")?.get("value")?.as_str().map(str::to_string)
}

pub fn get(path: &str) -> Option<Value> {
    served().then_some(())?;
    let guard = synced();
    let whole = object(guard.as_ref()?);
    match path {
        "video" => Some(whole),
        _ => Some(json!({ "kind": "value", "value": whole.get(path.strip_prefix("video.")?)?.clone() })),
    }
}

fn report(host: &mut Host, running: bool, frames: i64, width: u32, height: u32) {
    let decoding = running && frames > 0;
    let before = host.reported;
    host.reported = (running, decoding, width, height);
    let outs: Vec<Out> = [
        (running && !before.0).then(|| host.state.on_start_complete(MAIN_RECEIVER, Outcome::Ok, crate::hub::now_ms() / 1000)),
        (running != before.0).then(|| host.state.on_streaming(MAIN_RECEIVER, running)),
        (decoding != before.1).then(|| host.state.on_decoding(MAIN_RECEIVER, decoding)),
        ((width, height) != (before.2, before.3)).then(|| host.state.on_video_size(MAIN_RECEIVER, width, height)),
    ]
    .into_iter()
    .flatten()
    .flatten()
    .collect();
    apply(host, outs, crate::hub::now_ms());
}

pub fn invoke(path: &str, args: &str) -> Option<Value> {
    served().then_some(())?;
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
            report(host, given.get(0).and_then(Value::as_bool).unwrap_or(false), number(1), size(2), size(3));
            Some(json!({ "ok": true }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn a_uri_becomes_a_pipeline_ending_in_the_native_sink() {
        assert_eq!(pipeline("udp://0.0.0.0:5600", 80).unwrap(), "udpsrc address=0.0.0.0 port=5600 caps=\"application/x-rtp, media=(string)video, clock-rate=(int)90000, encoding-name=(string)H264\" ! rtpjitterbuffer latency=80 ! tee name=nativerec ! queue ! decodebin3 ! videoconvert ! appsink name=nativesink sync=false");
        assert_eq!(pipeline("rtsp://cam/main", 40).unwrap(), "rtspsrc location=\"rtsp://cam/main\" latency=40 ! tee name=nativerec ! queue ! decodebin3 ! videoconvert ! appsink name=nativesink sync=false");
        assert!(pipeline("whep://sfu/whep/x", 80).unwrap().starts_with("whepsrc whep-endpoint=\"http://sfu/whep/x\""));
        assert_eq!(pipeline("bogus", 80), None);
    }

    #[test]
    fn the_main_receiver_is_started_and_reports_drive_the_camera_status() {
        let settings = settings_from(
            &|name| match name {
                "videoSource" => "UDP h.264 Video Stream".to_string(),
                "udpUrl" => "0.0.0.0:5600".to_string(),
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
        assert!(host.wanted, "a configured stream starts the main receiver");
        assert_eq!(status_text(host.state.receiver_status(0)), "Connecting\u{2026}");
        report(&mut host, true, 0, 0, 0);
        assert_eq!(status_text(host.state.receiver_status(0)), "Connected, waiting for frames");
        report(&mut host, true, 12, 640, 360);
        assert_eq!((host.state.decoding, host.state.video_size), (true, Some((640, 360))));
        assert_eq!(status_text(host.state.receiver_status(0)), "");
        assert_eq!(status_text(host.state.receiver_status(1)), "No video source", "a camera with no receiver is no video source, as VideoManager::_cameraStatus says");
    }
}
