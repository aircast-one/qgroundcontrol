use serde_json::{Value, json};

use crate::read::{flag, integer, object, text};
use crate::router::Backend;

pub const VIDEO_DEPS: &[&str] = &["video.hasVideo", "video.decoding", "video.streaming", "video.recording", "video.activeVideoSource", "video.videoSize", "video.cameraStatuses", "video.cameraConnecting", "video.cameraRecording"];
pub const CAMERA_FIELDS: &str = "modelName,vendor,cameraMode,photoCaptureStatus,videoCaptureStatus,recordTimeStr,storageStatus,storageFreeStr,capturesPhotos,capturesVideo,hasModes,batteryRemaining,hasZoom,zoomLevel";
pub const CAMERA_DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.cameraManager.currentCameraInstance.modelName",
    "vehicle.cameraManager.currentCameraInstance.vendor",
    "vehicle.cameraManager.currentCameraInstance.cameraMode",
    "vehicle.cameraManager.currentCameraInstance.photoCaptureStatus",
    "vehicle.cameraManager.currentCameraInstance.videoCaptureStatus",
    "vehicle.cameraManager.currentCameraInstance.recordTimeStr",
    "vehicle.cameraManager.currentCameraInstance.storageStatus",
    "vehicle.cameraManager.currentCameraInstance.storageFreeStr",
    "vehicle.cameraManager.currentCameraInstance.capturesPhotos",
    "vehicle.cameraManager.currentCameraInstance.capturesVideo",
    "vehicle.cameraManager.currentCameraInstance.hasModes",
    "vehicle.cameraManager.currentCameraInstance.batteryRemaining",
    "vehicle.cameraManager.currentCameraInstance.hasZoom",
    "vehicle.cameraManager.currentCameraInstance.zoomLevel",
    "vehicle.cameraManager.currentCamera",
    "vehicle.cameraTriggerPoints.count",
];

const NO_URL_STATUS: &str = "No stream URL";
const UNDEFINED_MODE: i64 = -1;
const PHOTO_MODE: i64 = 0;
const VIDEO_MODE: i64 = 1;
const SURVEY_MODE: i64 = 2;
const VIDEO_RUNNING: i64 = 1;
const PHOTO_IN_PROGRESS: i64 = 1;
const PHOTO_INTERVAL_IN_PROGRESS: i64 = 3;
const IDLE_CLOCK: &str = "00:00:00";

pub fn video_summary(available: bool, decoding: bool, recording: bool, connecting: bool, configured: usize) -> &'static str {
    match (available, decoding, recording, connecting, configured) {
        (false, ..) => "This build cannot show video.",
        (true, true, true, ..) => "Streaming and recording.",
        (true, true, false, ..) => "Streaming.",
        (true, false, _, true, _) => "Waiting for a stream.",
        (true, false, _, false, 0) => "No stream URL is set.",
        _ => "Not streaming.",
    }
}

pub fn video_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let video = object(&backend.get_fields("video", "hasVideo,gstreamerEnabled,isStreamSource,decoding,streaming,recording,activeVideoSource,videoSize,hasMultipleVideoSources,cameraStatuses,cameraConnecting,cameraRecording"));
    let strings = |key: &str| -> Vec<String> { video.get(key).and_then(Value::as_array).map(|a| a.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect()).unwrap_or_default() };
    let flags = |key: &str| -> Vec<bool> { video.get(key).and_then(Value::as_array).map(|a| a.iter().map(|v| v.as_bool().unwrap_or(false)).collect()).unwrap_or_default() };
    let (statuses, connecting, recording_flags) = (strings("cameraStatuses"), flags("cameraConnecting"), flags("cameraRecording"));
    let cameras: Vec<Value> = statuses
        .iter()
        .enumerate()
        .map(|(slot, status)| {
            json!({
                "slot": slot,
                "title": crate::read::ok_result(&backend.invoke("video.cameraName", &json!([slot]).to_string())).and_then(|v| v.as_str().map(str::to_string)).filter(|n| !n.is_empty()).unwrap_or_else(|| format!("Camera {}", slot + 1)),
                "status": status,
                "connecting": connecting.get(slot).copied().unwrap_or(false),
                "recording": recording_flags.get(slot).copied().unwrap_or(false),
                "configured": !status.is_empty() && status != NO_URL_STATUS,
            })
        })
        .collect();
    let available = flag(&video, "hasVideo");
    let decoding = flag(&video, "decoding");
    let source_size = video
        .get("videoSize")
        .filter(|_| decoding)
        .and_then(|size| Some((size.get("width")?.as_i64()?, size.get("height")?.as_i64()?)))
        .filter(|(w, h)| *w > 0 && *h > 0)
        .map(|(width, height)| json!({ "width": width, "height": height }));
    let recording = flag(&video, "recording");
    let any_connecting = cameras.iter().any(|c| c["connecting"] == true);
    let configured = cameras.iter().filter(|c| c["configured"] == true).count();
    json!({
        "kind": "object",
        "class": "Video",
        "available": available,
        "gstreamer": flag(&video, "gstreamerEnabled"),
        "streamSource": flag(&video, "isStreamSource"),
        "decoding": decoding,
        "streaming": flag(&video, "streaming"),
        "recording": recording,
        "sourceSize": source_size,
        "activeSource": integer(&video, "activeVideoSource").unwrap_or(0),
        "multipleSources": flag(&video, "hasMultipleVideoSources"),
        "anyConnecting": any_connecting,
        "configuredCount": configured,
        "summary": video_summary(available, decoding, recording, any_connecting, configured),
        "cameras": cameras,
    })
}

pub fn camera_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let camera = object(&backend.get_fields("vehicle.cameraManager.currentCameraInstance", CAMERA_FIELDS));
    let model = text(&camera, "modelName");
    let present = camera.get("kind").and_then(Value::as_str) == Some("object") && !model.is_empty();
    let shots = integer(&object(&backend.get_fields("vehicle.cameraTriggerPoints", "count")), "count").unwrap_or(0);
    let vendor = text(&camera, "vendor");
    let mode = integer(&camera, "cameraMode").unwrap_or(UNDEFINED_MODE);
    let photo_status = integer(&camera, "photoCaptureStatus").unwrap_or(0);
    let video_status = integer(&camera, "videoCaptureStatus").unwrap_or(0);
    let record_time = text(&camera, "recordTimeStr");
    let storage_status = integer(&camera, "storageStatus").unwrap_or(3);
    let storage_free = text(&camera, "storageFreeStr");
    let (captures_photos, captures_video, has_modes) = (flag(&camera, "capturesPhotos"), flag(&camera, "capturesVideo"), flag(&camera, "hasModes"));
    let battery = integer(&camera, "batteryRemaining").unwrap_or(-1);
    let is_recording = video_status == VIDEO_RUNNING;
    let taking_photo = photo_status == PHOTO_IN_PROGRESS || photo_status == PHOTO_INTERVAL_IN_PROGRESS;
    json!({
        "kind": "object",
        "class": "CameraControl",
        "present": present,
        "title": if !model.is_empty() { model.clone() } else if vendor.is_empty() { "Camera".to_string() } else { vendor.clone() },
        "model": model,
        "vendor": vendor,
        "mode": mode,
        "modeKnown": mode != UNDEFINED_MODE,
        "modeText": match mode { PHOTO_MODE => "Photo", VIDEO_MODE => "Video", SURVEY_MODE => "Survey", _ => "Not set" },
        "isRecording": is_recording,
        "isTakingPhoto": taking_photo,
        "stateText": match (is_recording, taking_photo, record_time.is_empty()) {
            (true, _, true) => "Recording".to_string(),
            (true, _, false) => format!("Recording {record_time}"),
            (false, true, _) => "Taking a photo".to_string(),
            _ => "Idle".to_string(),
        },
        "clockText": if is_recording && !record_time.is_empty() { record_time.clone() } else { IDLE_CLOCK.to_string() },
        "storageStatus": storage_status,
        "storageText": match storage_status { 0 => "No card".to_string(), 1 => "Not formatted".to_string(), 2 => if storage_free.is_empty() { "Ready".to_string() } else { storage_free.clone() }, _ => "Not reported".to_string() },
        "shots": shots,
        "shotsText": format!("{shots:05}"),
        "batteryRemaining": battery,
        "batteryText": if battery >= 0 { format!("{battery}%") } else { String::new() },
        "hasZoom": flag(&camera, "hasZoom"),
        "zoomLevel": camera.get("zoomLevel").and_then(Value::as_f64).unwrap_or(1.0),
        "canRecord": present && captures_video && (!has_modes || mode != PHOTO_MODE),
        "canPhoto": present && captures_photos && (!has_modes || mode != VIDEO_MODE),
        "hasModes": has_modes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake { video: Value, camera: Value }
    impl Backend for Fake {
        fn get(&self, _p: &str) -> String { json!({ "kind": "null" }).to_string() }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "video" => self.video.to_string(),
                "vehicle.cameraManager.currentCameraInstance" => self.camera.to_string(),
                _ => json!({ "kind": "object", "count": 42 }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_video_summary_follows_the_stream_state() {
        assert_eq!(video_summary(false, false, false, false, 0), "This build cannot show video.");
        assert_eq!(video_summary(true, true, true, false, 1), "Streaming and recording.");
        assert_eq!(video_summary(true, false, false, true, 1), "Waiting for a stream.");
        assert_eq!(video_summary(true, false, false, false, 0), "No stream URL is set.");
        assert_eq!(video_summary(true, false, false, false, 2), "Not streaming.");
        let view = video_view(&Fake { video: json!({ "kind": "object", "hasVideo": true, "decoding": false, "videoSize": { "width": 640, "height": 480 }, "cameraStatuses": ["Connecting", "No stream URL"], "cameraConnecting": [true, false], "cameraRecording": [] }), camera: json!({ "kind": "null" }) }, &[]);
        assert_eq!(view["sourceSize"], Value::Null, "a size only counts while a frame is decoding");
        let decoding = video_view(&Fake { video: json!({ "kind": "object", "hasVideo": true, "decoding": true, "videoSize": { "width": 640, "height": 480 }, "cameraStatuses": [], "cameraConnecting": [], "cameraRecording": [] }), camera: json!({ "kind": "null" }) }, &[]);
        assert_eq!(decoding["sourceSize"], json!({ "width": 640, "height": 480 }));
        assert_eq!(view["cameras"][0]["configured"], true);
        assert_eq!(view["cameras"][1]["configured"], false);
        assert_eq!(view["configuredCount"], 1);
        assert_eq!(view["summary"], "Waiting for a stream.");
    }

    #[test]
    fn the_camera_control_reads_like_the_swift_model() {
        let view = camera_view(&Fake { video: json!({ "kind": "null" }), camera: json!({ "kind": "object", "modelName": "ZR30", "vendor": "SIYI", "cameraMode": 1, "videoCaptureStatus": 1, "recordTimeStr": "00:01:15", "storageStatus": 2, "storageFreeStr": "12 GB", "capturesPhotos": true, "capturesVideo": true, "hasModes": true, "batteryRemaining": 80 }) }, &[]);
        assert_eq!(view["present"], true);
        assert_eq!(view["title"], "ZR30");
        assert_eq!(view["modeText"], "Video");
        assert_eq!(view["stateText"], "Recording 00:01:15");
        assert_eq!(view["clockText"], "00:01:15");
        assert_eq!(view["storageText"], "12 GB");
        assert_eq!(view["shotsText"], "00042");
        assert_eq!(view["batteryText"], "80%");
        assert_eq!(view["canPhoto"], false);
        assert_eq!(view["canRecord"], true);
        let none = camera_view(&Fake { video: json!({ "kind": "null" }), camera: json!({ "kind": "null" }) }, &[]);
        assert_eq!(none["present"], false);
        assert_eq!(none["title"], "Camera");
        assert_eq!(none["canRecord"], false);
    }
}
