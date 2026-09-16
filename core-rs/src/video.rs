use serde_json::{Value, json};

use crate::read::{flag, integer, object, text};
use crate::router::Backend;

pub const VIDEO_DEPS: &[&str] = &[
    "video.isStreamSource",
    "video.hasMultipleVideoSources","settings.videoSettings.extraVideoSources", "video.hasVideo", "video.decoding", "video.streaming", "video.recording", "video.activeVideoSource", "video.videoSize", "video.cameraStatuses", "video.cameraConnecting", "video.cameraRecording"];
pub const CAMERA_FIELDS: &str = "modelName,vendor,cameraMode,photoCaptureStatus,videoCaptureStatus,recordTimeStr,storageStatus,storageFreeStr,capturesPhotos,capturesVideo,hasModes,photosInVideoMode,videoInPhotoMode,photoCaptureMode,photoLapse,photoLapseCount,batteryRemaining,hasZoom,zoomLevel,hasTracking,thermalMode,thermalOpacity,thermalStreamInstance,trackingEnabled,trackingImageStatus,trackingImageRect,trackingStatus";
pub const CAMERA_DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.cameraManager.cameraLabels",
    "vehicle.cameraManager.currentCameraInstance.modelName",
    "vehicle.cameraManager.currentCameraInstance.vendor",
    "vehicle.cameraManager.currentCameraInstance.cameraMode",
    "vehicle.cameraManager.currentCameraInstance.photoCaptureStatus",
    "vehicle.cameraManager.currentCameraInstance.videoCaptureStatus",
    "vehicle.cameraManager.currentCameraInstance.recordTimeStr",
    "vehicle.cameraManager.currentCameraInstance.storageStatus",
    "vehicle.cameraManager.currentCameraInstance.storageFreeStr",
    "vehicle.cameraManager.currentCameraInstance.capturesPhotos",
    "vehicle.cameraManager.currentCameraInstance.photosInVideoMode",
    "vehicle.cameraManager.currentCameraInstance.photoCaptureMode",
    "vehicle.cameraManager.currentCameraInstance.photoLapse",
    "vehicle.cameraManager.currentCameraInstance.photoLapseCount",
    "vehicle.cameraManager.currentCameraInstance.videoInPhotoMode",
    "vehicle.cameraManager.currentCameraInstance.capturesVideo",
    "vehicle.cameraManager.currentCameraInstance.hasModes",
    "vehicle.cameraManager.currentCameraInstance.batteryRemaining",
    "vehicle.cameraManager.currentCameraInstance.hasZoom",
    "vehicle.cameraManager.currentCameraInstance.zoomLevel",
    "vehicle.cameraManager.currentCamera",
    "vehicle.cameraTriggerPoints.count",
];

const UNDEFINED_MODE: i64 = -1;
const PHOTO_CAPTURE_IDLE: i64 = 0;
const PHOTO_CAPTURE_IN_PROGRESS: i64 = 1;
const PHOTO_CAPTURE_INTERVAL_IDLE: i64 = 2;
const PHOTO_CAPTURE_INTERVAL_IN_PROGRESS: i64 = 3;
const STORAGE_NOT_SUPPORTED: i64 = 3;
const VIDEO_CAPTURE_STOPPED: i64 = 0;
const VIDEO_CAPTURE_RUNNING: i64 = 1;
const TIMELAPSE: i64 = 1;
const PHOTO_MODE: i64 = 0;
const VIDEO_MODE: i64 = 1;
const SURVEY_MODE: i64 = 2;
const IDLE_CLOCK: &str = "00:00:00";

fn slot_flag(backend: &dyn Backend, path: &str, slot: usize) -> bool {
    crate::read::result_flag(&backend.invoke(path, &json!([slot]).to_string()))
}

pub fn can_change_mode(mode: i64, photo_status: i64, video_status: i64) -> bool {
    match mode {
        PHOTO_MODE => matches!(photo_status, PHOTO_CAPTURE_IDLE | PHOTO_CAPTURE_INTERVAL_IDLE),
        _ => video_status == VIDEO_CAPTURE_STOPPED,
    }
}

pub fn video_summary(build_shows_video: bool, available: bool, decoding: bool, recording: bool, connecting: bool, configured: usize) -> &'static str {
    match (build_shows_video, available, decoding, recording, connecting, configured) {
        (false, ..) => "This build cannot show video.",
        (true, true, true, true, ..) => "Streaming and recording.",
        (true, true, true, false, ..) => "Streaming.",
        (true, true, false, _, true, _) => "Waiting for a stream.",
        (true, false, .., 0) => "No stream URL is set.",
        _ => "Not streaming.",
    }
}

fn shot_points(backend: &dyn Backend) -> Vec<Value> {
    object(&backend.get("vehicle.cameraTriggerPoints"))
        .get("elements")
        .and_then(Value::as_array)
        .map(|listed| {
            listed
                .iter()
                .filter_map(|point| {
                    let at = point.get("coordinate")?;
                    Some(json!({ "latitude": at.get("latitude")?.as_f64()?, "longitude": at.get("longitude")?.as_f64()? }))
                })
                .collect()
        })
        .unwrap_or_default()
}

// extraVideoSources is a JSON array kept in a string setting, and VideoSourceModel.swift:30
// returns [] for every parse failure - which is the same value as "no extra sources configured".
// A corrupted or hand-edited string therefore makes an operator's cameras vanish looking exactly
// like a fresh install, and the next edit calls encode() on the empty list and writes it back, so
// the original is destroyed rather than merely hidden. Empty and unreadable have to be different
// answers, and the second one has to travel far enough for a head to refuse to overwrite.
fn extra_sources(backend: &dyn Backend) -> Value {
    let stored = text(&object(&backend.get("settings.videoSettings.extraVideoSources")), "valueString");
    let trimmed = stored.trim();
    if trimmed.is_empty() {
        return json!({ "readable": true, "sources": [], "stored": stored });
    }
    let listed = serde_json::from_str::<Value>(trimmed).ok().and_then(|parsed| parsed.as_array().cloned());
    match listed {
        Some(entries) => json!({
            "readable": true,
            "sources": entries.iter().enumerate().map(|(slot, entry)| json!({
                "slot": slot,
                "name": text(entry, "name"),
                "source": text(entry, "source"),
                "url": text(entry, "url"),
            })).collect::<Vec<_>>(),
            "stored": stored,
        }),
        // The stored text travels back so a head can show what is there and let someone repair it
        // rather than silently replacing it with an empty list.
        None => json!({
            "readable": false,
            "sources": [],
            "stored": stored,
            "reason": "The extra video sources setting is not a readable list, so the cameras it holds cannot be shown. Editing them now would replace it.",
        }),
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
                "enabled": slot_flag(backend, "settings.videoSettings.sourceEnabled", slot),
                "configured": slot_flag(backend, "settings.videoSettings.sourceEnabled", slot) && slot_flag(backend, "settings.videoSettings.sourceConfigured", slot),
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
        "summary": video_summary(flag(&video, "gstreamerEnabled"), available, decoding, recording, any_connecting, configured),
        "cameras": cameras,
        "extraSources": extra_sources(backend),
    })
}

pub fn camera_present(camera: &Value) -> bool {
    camera.get("kind").and_then(Value::as_str) == Some("object") && !text(camera, "modelName").is_empty()
}

const THERMAL_BLEND: i64 = 1;
const TRACKING_RECTANGLE: i64 = 4;
const TRACKING_POINT: i64 = 8;

fn thermal_token(mode: Option<i64>) -> Option<&'static str> {
    match mode? {
        0 => Some("off"),
        1 => Some("blend"),
        2 => Some("full"),
        3 => Some("picInPic"),
        _ => None,
    }
}

fn tracking_shapes(status: i64) -> Vec<&'static str> {
    [(TRACKING_RECTANGLE, "rectangle"), (TRACKING_POINT, "point")]
        .iter()
        .filter(|(bit, _)| status & bit != 0)
        .map(|(_, name)| *name)
        .collect()
}

const DESTRUCTIVE_OFFERS: [(&str, &str, &str, bool); 2] = [
    ("formatStorage", "Format", "Erase every file on the camera's storage. This cannot be undone.", true),
    ("resetSettings", "Reset", "Put every camera setting back to its factory value. This cannot be undone.", false),
];

fn destructive_offers(present: bool, reports_storage: bool, recording: bool) -> Value {
    DESTRUCTIVE_OFFERS
        .iter()
        .map(|(id, title, prompt, needs_storage)| {
            let shown = present && (!needs_storage || reports_storage);
            let busy = shown && *needs_storage && recording;
            json!({
                "id": id,
                "title": title,
                "prompt": prompt,
                "offer": match (shown, busy) { (false, _) => "hidden", (true, true) => "blocked", (true, false) => "ready" },
                "reason": match busy { true => "The camera is recording.", false => "" },
                "destructive": true,
            })
        })
        .collect::<Vec<Value>>()
        .into()
}

pub fn camera_view(backend: &dyn Backend, _args: &[String]) -> Value {
    // The only field any head took from vehicle.cameraManager: the switcher needs every camera's
    // name, and this view carried only the current one's. One field short kept a whole Qt path
    // alive on both heads.
    let manager = object(&backend.get_fields("vehicle.cameraManager", "cameraLabels"));
    let labels: Vec<&str> = manager
        .get("cameraLabels")
        .and_then(Value::as_array)
        .map(|names| names.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let camera = object(&backend.get_fields("vehicle.cameraManager.currentCameraInstance", CAMERA_FIELDS));
    let model = text(&camera, "modelName");
    let present = camera_present(&camera);
    let thermal_available = present && camera.get("thermalStreamInstance").is_some_and(|stream| stream.is_object());
    let shots = integer(&object(&backend.get_fields("vehicle.cameraTriggerPoints", "count")), "count").unwrap_or(0);
    let vendor = text(&camera, "vendor");
    let mode = integer(&camera, "cameraMode").unwrap_or(UNDEFINED_MODE);
    let photo_status = integer(&camera, "photoCaptureStatus").unwrap_or(PHOTO_CAPTURE_IDLE);
    let video_status = integer(&camera, "videoCaptureStatus").unwrap_or(VIDEO_CAPTURE_STOPPED);
    let record_time = text(&camera, "recordTimeStr");
    // STORAGE_STATUS_NOT_SUPPORTED is 3 and means "Camera does not supply storage status
    // information" - a claim about the hardware, not a neutral placeholder. Defaulting absence to
    // it turned a camera that had not answered yet into one that had declared it does not track
    // storage, and PhotoVideoControl.qml:423 hides the whole storage row on that value.
    let storage_status = integer(&camera, "storageStatus");
    let storage_free = text(&camera, "storageFreeStr");
    let (captures_photos, captures_video, has_modes) = (flag(&camera, "capturesPhotos"), flag(&camera, "capturesVideo"), flag(&camera, "hasModes"));
    let battery = integer(&camera, "batteryRemaining").unwrap_or(-1);
    let is_recording = video_status == VIDEO_CAPTURE_RUNNING;
    let timelapse = integer(&camera, "photoCaptureMode") == Some(TIMELAPSE);
    let taking_photo = matches!(photo_status, PHOTO_CAPTURE_IN_PROGRESS | PHOTO_CAPTURE_INTERVAL_IN_PROGRESS);
    json!({
        "kind": "object",
        "class": "CameraControl",
        "present": present,
        "title": if !model.is_empty() { model.clone() } else if vendor.is_empty() { "Camera".to_string() } else { vendor.clone() },
        "model": model,
        "labels": labels,
        "choices": labels.len(),
        "selected": crate::read::value_number(&backend.get("vehicle.cameraManager.currentCamera")).map(|n| n as i64),
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
        // A camera answering NOT_SUPPORTED did report - it reported that this is not a thing it
        // tracks. "Not reported" is the sentence for silence, and both wore it.
        "reportsStorage": storage_status != Some(STORAGE_NOT_SUPPORTED),
        "storageText": match storage_status {
            Some(0) => "No card".to_string(),
            Some(1) => "Not formatted".to_string(),
            Some(2) => if storage_free.is_empty() { "Ready".to_string() } else { storage_free.clone() },
            Some(STORAGE_NOT_SUPPORTED) => "This camera does not report storage.".to_string(),
            _ => "Not reported".to_string(),
        },
        "shots": shots,
        "shotsText": if shots > 0 { shots.to_string() } else { "\u{2014}".to_string() },
        "shotPoints": shot_points(backend),
        "batteryRemaining": battery,
        "batteryText": if battery >= 0 { format!("{battery}%") } else { String::new() },
        "thermalAvailable": thermal_available,
        "thermalMode": thermal_available.then(|| thermal_token(integer(&camera, "thermalMode"))).flatten(),
        "thermalOpacity": (thermal_available && integer(&camera, "thermalMode") == Some(THERMAL_BLEND))
            .then(|| camera.get("thermalOpacity").and_then(Value::as_f64))
            .flatten(),
        "tracking": present.then(|| json!({
            "supported": flag(&camera, "hasTracking"),
            "enabled": flag(&camera, "trackingEnabled"),
            "active": flag(&camera, "trackingImageStatus"),
            "shapes": tracking_shapes(integer(&camera, "trackingStatus").unwrap_or(0)),
            "rect": flag(&camera, "trackingImageStatus").then(|| camera.get("trackingImageRect").cloned().filter(|r| r.is_object())).flatten(),
        })),
        "hasZoom": flag(&camera, "hasZoom"),
        "zoomLevel": camera.get("zoomLevel").and_then(Value::as_f64).unwrap_or(0.0),
        // VehicleCameraControl.cc:351 and :~300 refuse on terms this gate did not carry, so it was
        // wrong in both directions: a camera that shoots stills in video mode had a working shutter
        // greyed out, and a camera mid-capture had a live button whose tap returns false in silence.
        "canRecord": present && captures_video && (!has_modes || mode != PHOTO_MODE || flag(&camera, "videoInPhotoMode")),
        "canPhoto": present && captures_photos && (!has_modes || mode != VIDEO_MODE || flag(&camera, "photosInVideoMode")) && photo_status == PHOTO_CAPTURE_IDLE,
        // takePhoto sends `_photoMode == PHOTO_CAPTURE_SINGLE ? 0 : _photoLapse` and
        // `? 1 : _photoLapseCount`, so the same shutter press either takes one photo or starts an
        // interval capture of lapseCount shots. Serving only canPhoto makes those one button with
        // one meaning, and a count of zero is unlimited - the press that never stops on its own.
        "photoMode": match timelapse { true => "timelapse", false => "single" },
        // Gated the same way the action gates its copy. photoLapseCount keeps whatever it was last
        // configured to in single mode, so serving it ungated hands a head a count of shots for a
        // press that takes one - and a count of zero there would read as none rather than unlimited.
        "lapseSeconds": timelapse.then(|| camera.get("photoLapse").and_then(Value::as_f64)).flatten(),
        "lapseCount": timelapse.then(|| integer(&camera, "photoLapseCount")).flatten(),
        "lapseUnlimited": timelapse && integer(&camera, "photoLapseCount") == Some(0),
        // stopTakePhoto refuses unless the status is one of the two interval states, and nothing in
        // QGC's QML calls it - so a head that starts a timelapse today cannot end it.
        "canStopPhoto": present && matches!(photo_status, PHOTO_CAPTURE_INTERVAL_IDLE | PHOTO_CAPTURE_INTERVAL_IN_PROGRESS),
        "hasModes": has_modes,
        "canChangeMode": present && has_modes && can_change_mode(mode, photo_status, video_status),
        "destructiveActions": destructive_offers(present, storage_status != Some(STORAGE_NOT_SUPPORTED), is_recording),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake { video: Value, camera: Value, enabled: Vec<bool>, configured: Vec<bool> }

    impl Fake {
        fn new(video: Value, camera: Value) -> Fake {
            Fake { video, camera, enabled: Vec::new(), configured: Vec::new() }
        }
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.cameraTriggerPoints" => json!({ "kind": "object", "elements": [
                    { "coordinate": { "latitude": 47.397, "longitude": 8.546, "valid": true } },
                    { "coordinate": { "latitude": 47.398, "longitude": 8.547, "valid": true } },
                ] }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "video" => self.video.to_string(),
                "vehicle.cameraManager.currentCameraInstance" => self.camera.to_string(),
                "vehicle.cameraManager" => json!({ "kind": "object", "cameraLabels": ["Sony ILCE-7", "Thermal"] }).to_string(),
                _ => json!({ "kind": "object", "count": 42 }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, path: &str, args: &str) -> String {
            let slot = serde_json::from_str::<Vec<usize>>(args).ok().and_then(|a| a.first().copied()).unwrap_or(0);
            let answer = |slots: &[bool]| json!({ "ok": true, "result": slots.get(slot).copied().unwrap_or(false) }).to_string();
            match path {
                "settings.videoSettings.sourceEnabled" => answer(&self.enabled),
                "settings.videoSettings.sourceConfigured" => answer(&self.configured),
                _ => String::new(),
            }
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_thermal_slider_is_withheld_unless_the_camera_is_actually_blending() {
        let cam = |extra: Value| {
            let mut c = json!({ "kind": "object", "modelName": "ZR30" });
            extra.as_object().unwrap().iter().for_each(|(k, v)| { c[k] = v.clone(); });
            camera_view(&Fake::new(json!({ "kind": "object" }), c), &[])
        };

        let none = cam(json!({}));
        assert_eq!((none["thermalAvailable"].clone(), none["thermalMode"].clone(), none["thermalOpacity"].clone()), (json!(false), Value::Null, Value::Null), "PhotoVideoControl gates the whole control on thermalStreamInstance being non-null, so a camera with no thermal stream has no mode rather than mode off");

        let blending = cam(json!({ "thermalStreamInstance": { "kind": "object" }, "thermalMode": 1, "thermalOpacity": 60.0 }));
        assert_eq!((blending["thermalMode"].clone(), blending["thermalOpacity"].clone()), (json!("blend"), json!(60.0)));

        let full = cam(json!({ "thermalStreamInstance": { "kind": "object" }, "thermalMode": 2, "thermalOpacity": 60.0 }));
        assert_eq!(full["thermalMode"], "full");
        assert_eq!(
            full["thermalOpacity"],
            Value::Null,
            "Qt shows the blend slider only when the mode IS blend, so serving the number in every mode invites a head to draw a live slider that changes nothing - the enforce-checklist shape"
        );

        assert_eq!(cam(json!({ "thermalStreamInstance": { "kind": "object" }, "thermalMode": 9 }))["thermalMode"], Value::Null, "an enumerator the core does not know is not silently the first one");
    }

    #[test]
    fn an_unreported_zoom_is_the_value_qgc_itself_would_hold() {
        let cam = |extra: Value| {
            let mut c = json!({ "kind": "object", "modelName": "ZR30" });
            extra.as_object().unwrap().iter().for_each(|(k, v)| { c[k] = v.clone(); });
            camera_view(&Fake::new(json!({ "kind": "object" }), c), &[])
        };

        assert_eq!(cam(json!({ "hasZoom": true, "zoomLevel": 4.5 }))["zoomLevel"], json!(4.5));
        assert_eq!(
            cam(json!({ "hasZoom": true }))["zoomLevel"],
            json!(0.0),
            "VehicleCameraControl.h:268 declares _zoomLevel = 0.0, so a camera that has never reported one reads zero in QGC; serving 1.0 invents a 1x the vehicle never claimed, and the Android head uses its own 0.0 as the zoom-out clamp floor"
        );
        assert_eq!(cam(json!({}))["zoomLevel"], json!(0.0), "a camera with no zoom at all is not sitting at 1x either");
    }

    #[test]
    fn the_two_camera_actions_that_cannot_be_undone_are_offered_the_way_qgc_offers_them() {
        let offers = |camera: Value| {
            camera_view(&Fake::new(json!({ "kind": "object" }), camera), &[])["destructiveActions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| (a["id"].as_str().unwrap().to_string(), a["offer"].as_str().unwrap().to_string(), a["destructive"].as_bool().unwrap()))
                .collect::<Vec<(String, String, bool)>>()
        };

        let reporting = offers(json!({ "kind": "object", "modelName": "ZR30", "storageStatus": 2 }));
        assert_eq!(
            reporting,
            vec![("formatStorage".to_string(), "ready".to_string(), true), ("resetSettings".to_string(), "ready".to_string(), true)],
            "PhotoVideoControl.qml draws both buttons for a camera that reports storage, and both open a confirmation because both are irreversible"
        );

        let no_storage = offers(json!({ "kind": "object", "modelName": "ZR30", "storageStatus": 3 }));
        assert_eq!(
            no_storage.iter().map(|o| o.1.as_str()).collect::<Vec<_>>(),
            vec!["hidden", "ready"],
            "PhotoVideoControl.qml:582 gates only Format on _cameraStorageSupported; Reset has no such gate, so hiding both would withhold a control QGC offers"
        );

        assert_eq!(
            offers(json!({ "kind": "null" })).iter().map(|o| o.1.as_str()).collect::<Vec<_>>(),
            vec!["hidden", "hidden"],
            "there is nothing to format or reset without a camera"
        );

        let recording = camera_view(&Fake::new(json!({ "kind": "object" }), json!({ "kind": "object", "modelName": "ZR30", "storageStatus": 2, "videoCaptureStatus": 1 })), &[])["destructiveActions"].clone();
        assert_eq!(
            (recording[0]["offer"].clone(), recording[0]["reason"].clone()),
            (json!("blocked"), json!("The camera is recording.")),
            "cameraproto.rs:713 refuses FormatStorage with Refusal::Busy while recording, so offering it as ready is a success that never happens - the view has to tell the truth about a gate the core already enforces"
        );
        assert_eq!(recording[1]["offer"], "ready", "ResetSettings is gated on nothing in either the QML or the protocol, so a recording camera can still be reset");
    }

    #[test]
    fn a_tracking_rectangle_is_offered_only_while_the_camera_is_tracking() {
        let cam = |extra: Value| {
            let mut c = json!({ "kind": "object", "modelName": "ZR30" });
            extra.as_object().unwrap().iter().for_each(|(k, v)| { c[k] = v.clone(); });
            camera_view(&Fake::new(json!({ "kind": "object" }), c), &[])
        };
        let rect = json!({ "x": 0.1, "y": 0.2, "width": 0.3, "height": 0.4 });

        let idle = cam(json!({ "hasTracking": true, "trackingStatus": 5, "trackingEnabled": false, "trackingImageStatus": false, "trackingImageRect": rect }));
        assert_eq!((idle["tracking"]["supported"].clone(), idle["tracking"]["enabled"].clone(), idle["tracking"]["active"].clone()), (json!(true), json!(false), json!(false)));
        assert_eq!(idle["tracking"]["shapes"], json!(["rectangle"]), "TrackingStatus is a bitmask and the two shape bits say which gestures the camera accepts, so a head offering a drag on a point-only camera is offering a command it will refuse");
        assert_eq!(
            idle["tracking"]["rect"],
            Value::Null,
            "the rectangle is whatever was last tracked, and trackingImageStatus is the only thing saying it is current - drawing a stale box over live video is a claim about where the target is now"
        );

        let live = cam(json!({ "hasTracking": true, "trackingStatus": 14, "trackingEnabled": true, "trackingImageStatus": true, "trackingImageRect": rect }));
        assert_eq!(live["tracking"]["rect"], rect, "and a QRectF only reaches a head at all because variantJson gained a case for it");
        assert_eq!(live["tracking"]["shapes"], json!(["rectangle", "point"]));

        assert_eq!(cam(json!({}))["tracking"]["supported"], false, "a camera that does not report tracking supports none of it");
    }

    #[test]
    fn the_video_summary_follows_the_stream_state() {
        assert_eq!(video_summary(false, false, false, false, false, 0), "This build cannot show video.");
        assert_eq!(video_summary(true, true, true, true, false, 1), "Streaming and recording.");
        assert_eq!(video_summary(true, true, true, false, false, 1), "Streaming.");
        assert_eq!(video_summary(true, true, false, false, true, 1), "Waiting for a stream.");
        assert_eq!(
            "No stream URL is set.",
            video_summary(true, false, false, false, false, 0),
            "hasVideo is streamEnabled && streamConfigured - both settings - so an operator who has \
             picked no source has to be told to pick one, not that their build cannot do video"
        );
        assert_eq!(video_summary(true, false, false, false, false, 2), "Not streaming.");
        assert_eq!(
            "This build cannot show video.",
            video_summary(false, true, true, false, false, 1),
            "only the build flag may claim a build limitation, whatever the stream is doing"
        );
        let two_slots = Fake { enabled: vec![true, true, false], configured: vec![true, false, true], ..Fake::new(json!({ "kind": "object", "gstreamerEnabled": true, "hasVideo": true, "decoding": false, "videoSize": { "width": 640, "height": 480 }, "cameraStatuses": ["Connecting", "Waiting", "Waiting"], "cameraConnecting": [true, false, false], "cameraRecording": [] }), json!({ "kind": "null" })) };
        let view = video_view(&two_slots, &[]);
        assert!(view["extraSources"]["readable"].is_boolean(), "video_view has to CARRY the block; a head reads view.video and never calls extra_sources, so testing that function alone leaves the wiring unpinned - fifth time tonight");
        assert_eq!(view["sourceSize"], Value::Null, "a size only counts while a frame is decoding");
        let decoding = video_view(&Fake::new(json!({ "kind": "object", "gstreamerEnabled": true, "hasVideo": true, "decoding": true, "videoSize": { "width": 640, "height": 480 }, "cameraStatuses": [], "cameraConnecting": [], "cameraRecording": [] }), json!({ "kind": "null" })), &[]);
        assert_eq!(decoding["sourceSize"], json!({ "width": 640, "height": 480 }));
        assert_eq!(view["cameras"][0]["configured"], true);
        assert_eq!(view["cameras"][1]["configured"], false, "an enabled slot with no address is not configured");
        assert_eq!(view["cameras"][2]["configured"], false, "a disabled slot needs no address, so asking only whether it is configured would call it ready");
        assert_eq!(view["cameras"][2]["enabled"], false);
        assert_eq!(view["configuredCount"], 1);
        assert_eq!(view["summary"], "Waiting for a stream.");
    }

    #[test]
    fn an_unreadable_source_list_is_not_an_empty_one() {
        struct Stored(&'static str);
        impl Backend for Stored {
            fn get(&self, path: &str) -> String {
                match path {
                    "settings.videoSettings.extraVideoSources" => json!({ "kind": "fact", "name": "extraVideoSources", "valueString": self.0 }).to_string(),
                    _ => json!({ "kind": "null" }).to_string(),
                }
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        let two = extra_sources(&Stored(r#"[{"name":"Nose","source":"RTSP","url":"rtsp://a"},{"name":"Belly","source":"UDP","url":"udp://b"}]"#));
        assert_eq!(two["readable"], true);
        assert_eq!(two["sources"].as_array().unwrap().len(), 2);
        assert_eq!(two["sources"][1]["name"], "Belly");
        assert_eq!(two["sources"][1]["slot"], 1);

        let none = extra_sources(&Stored("[]"));
        assert_eq!(none["readable"], true, "an empty list is a readable answer: the operator has configured no extras");
        assert_eq!(none["sources"].as_array().unwrap().len(), 0);

        let unset = extra_sources(&Stored(""));
        assert_eq!(unset["readable"], true, "an unset setting is not a corrupt one");

        let broken = extra_sources(&Stored(r#"[{"name":"Nose","url":"rtsp://a"#));
        assert_eq!(broken["readable"], false, "VideoSourceModel.swift returns [] here, which is the same value as no-extras - so an operator's cameras vanish looking exactly like a fresh install");
        assert_eq!(broken["sources"].as_array().unwrap().len(), 0);
        assert!(broken["stored"].as_str().unwrap().contains("Nose"), "the stored text travels so a head can show what is there and offer a repair; encode() over an empty list destroys it");
        assert!(broken["reason"].as_str().unwrap().contains("replace"));

        let wrong_shape = extra_sources(&Stored(r#"{"name":"Nose"}"#));
        assert_eq!(wrong_shape["readable"], false, "valid JSON that is not an array is not a list of sources either");
        assert_ne!(broken["readable"], none["readable"], "the whole point is that these two answer differently");
    }

    #[test]
    fn a_shutter_is_offered_on_the_terms_the_camera_control_actually_refuses_on() {
        let cam = |extra: Value| {
            let mut base = json!({ "kind": "object", "modelName": "ZR30", "capturesPhotos": true, "capturesVideo": true, "hasModes": true });
            extra.as_object().unwrap().iter().for_each(|(k, v)| { base[k] = v.clone(); });
            camera_view(&Fake::new(json!({ "kind": "null" }), base), &[])
        };

        assert_eq!(cam(json!({ "cameraMode": 1 }))["canPhoto"], false, "in video mode a camera that cannot shoot stills there refuses, and the gate says so");
        assert_eq!(cam(json!({ "cameraMode": 1, "photosInVideoMode": true }))["canPhoto"], true,
            "VehicleCameraControl.cc only refuses on the mode when photosInVideoMode is false; without that term the core greys out a shutter that works");
        assert_eq!(cam(json!({ "cameraMode": 0, "photoCaptureStatus": 1 }))["canPhoto"], false,
            "takePhoto returns false when the status is not idle, so serving canPhoto here gives a head a live button whose tap does nothing and says nothing");
        assert_eq!(cam(json!({ "cameraMode": 0, "photoCaptureStatus": 2 }))["canPhoto"], false,
            "the wait between interval shots is idle enough to change mode but not to fire: takePhoto tests against IDLE alone, and the two questions have different answers");
        assert_eq!(cam(json!({ "cameraMode": 0 }))["canPhoto"], true);

        let silent = cam(json!({ "cameraMode": 0 }));
        assert_eq!(silent["storageStatus"], Value::Null, "STORAGE_STATUS_NOT_SUPPORTED is 3 and asserts the camera does not track storage, so defaulting silence to it makes a claim about hardware the core has heard nothing from");
        assert_eq!(silent["reportsStorage"], true, "a camera that has not answered has not said it cannot answer, and PhotoVideoControl.qml hides the whole storage row on the value this used to default to");
        assert_eq!(silent["storageText"], "Not reported");

        let unsupported = cam(json!({ "cameraMode": 0, "storageStatus": 3 }));
        assert_eq!(unsupported["reportsStorage"], false);
        assert_eq!(unsupported["storageText"], "This camera does not report storage.", "this camera DID report - it reported that storage is not a thing it tracks, which is not silence");
        assert_ne!(unsupported["storageText"], silent["storageText"]);

        let ready = cam(json!({ "cameraMode": 0, "storageStatus": 2, "storageFreeStr": "12 GB" }));
        assert_eq!(ready["storageText"], "12 GB");
        assert_eq!(ready["reportsStorage"], true);
        assert_eq!(cam(json!({ "cameraMode": 0, "storageStatus": 0 }))["storageText"], "No card");

        let configured = cam(json!({ "cameraMode": 0, "photoCaptureMode": 0, "photoLapse": 5.0, "photoLapseCount": 10 }));
        assert_eq!(configured["photoMode"], "single");
        assert_eq!(configured["lapseCount"], Value::Null, "photoLapseCount keeps its last configured value in single mode, so serving it ungated tells a head this press takes ten shots when it takes one");
        assert_eq!(configured["lapseSeconds"], Value::Null);
        assert_eq!(configured["lapseUnlimited"], false);

        let lapsing = cam(json!({ "cameraMode": 0, "photoCaptureMode": 1, "photoLapse": 5.0, "photoLapseCount": 0 }));
        assert_eq!(lapsing["photoMode"], "timelapse");
        assert_eq!(lapsing["lapseCount"], 0);
        assert_eq!(lapsing["lapseUnlimited"], true, "zero is MAV_CMD_IMAGE_START_CAPTURE's unlimited, so a head rendering the number alone says none when it means forever");
        assert_eq!(lapsing["canStopPhoto"], false, "configured for a timelapse is not the same as running one; the stop control appears when the status says an interval is under way");
        assert_eq!(cam(json!({ "cameraMode": 0, "photoCaptureStatus": 3 }))["canStopPhoto"], true);
        assert_eq!(cam(json!({ "cameraMode": 0, "photoCaptureStatus": 2 }))["canStopPhoto"], true, "the wait between interval shots is still an interval to stop");
        assert_eq!(cam(json!({ "cameraMode": 0, "photoCaptureStatus": 1 }))["canStopPhoto"], false, "a single shot in progress is not an interval and stopTakePhoto refuses it");

        assert_eq!(cam(json!({ "cameraMode": 0 }))["canRecord"], false);
        assert_eq!(cam(json!({ "cameraMode": 0, "videoInPhotoMode": true }))["canRecord"], true,
            "startVideoRecording refuses on photo mode only when videoInPhotoMode is false, the same missing term the other way round");
        assert_eq!(cam(json!({ "cameraMode": 1, "videoCaptureStatus": 1 }))["canRecord"], true,
            "a running recording is not a refusal, because the toggle is what stops it - which is why record takes no busy term and photo does");
    }

    #[test]
    fn a_camera_mid_capture_cannot_be_switched_out_of_its_mode() {
        assert!(can_change_mode(PHOTO_MODE, 0, 0));
        assert!(can_change_mode(PHOTO_MODE, 2, 0), "status 2 is the wait between interval shots, which is idle enough to leave photo mode");
        assert!(!can_change_mode(PHOTO_MODE, 1, 0), "a shot in progress holds the camera in photo mode");
        assert!(!can_change_mode(PHOTO_MODE, 3, 0), "status 3 is an interval shot in progress, not the wait before one");
        assert!(can_change_mode(VIDEO_MODE, 1, 0));
        assert!(!can_change_mode(VIDEO_MODE, 0, 1), "a running recording holds the camera in video mode");
        assert!(!can_change_mode(SURVEY_MODE, 0, 1), "the Qt control treats every mode that is not photo as video mode, so a recording holds survey too");
        let recording = camera_view(&Fake::new(json!({ "kind": "null" }), json!({ "kind": "object", "modelName": "ZR30", "cameraMode": 1, "videoCaptureStatus": 1, "capturesPhotos": true, "capturesVideo": true, "hasModes": true })), &[]);
        assert_eq!(recording["canChangeMode"], false);
        assert_eq!(recording["canRecord"], true, "the record control stays live while recording, because it is what stops it");
        let idle = camera_view(&Fake::new(json!({ "kind": "null" }), json!({ "kind": "object", "modelName": "ZR30", "cameraMode": 1, "videoCaptureStatus": 0, "capturesPhotos": true, "capturesVideo": true, "hasModes": true })), &[]);
        assert_eq!(idle["canChangeMode"], true);
        let fixed = camera_view(&Fake::new(json!({ "kind": "null" }), json!({ "kind": "object", "modelName": "Fixed", "cameraMode": 1, "videoCaptureStatus": 0, "capturesVideo": true, "hasModes": false })), &[]);
        assert_eq!(fixed["canChangeMode"], false, "a camera with no modes is never offered a mode change");
    }

    #[test]
    fn the_switcher_is_told_every_camera_and_not_just_the_open_one() {
        let view = camera_view(&Fake::new(json!({ "kind": "object" }), json!({ "kind": "object", "modelName": "Sony ILCE-7" })), &[]);
        assert_eq!(view["labels"], json!(["Sony ILCE-7", "Thermal"]), "the only field any head read off vehicle.cameraManager, and this view carried the current camera alone");
        assert_eq!(view["choices"], 2);
        assert_eq!(view["model"], "Sony ILCE-7", "the open camera is still named separately from the list it sits in");
    }

    #[test]
    fn the_camera_control_reads_like_the_swift_model() {
        let view = camera_view(&Fake::new(json!({ "kind": "null" }), json!({ "kind": "object", "modelName": "ZR30", "vendor": "SIYI", "cameraMode": 1, "videoCaptureStatus": 1, "recordTimeStr": "00:01:15", "storageStatus": 2, "storageFreeStr": "12 GB", "capturesPhotos": true, "capturesVideo": true, "hasModes": true, "batteryRemaining": 80 })), &[]);
        assert_eq!(view["present"], true);
        assert_eq!(view["title"], "ZR30");
        assert_eq!(view["modeText"], "Video");
        assert_eq!(view["stateText"], "Recording 00:01:15");
        assert_eq!(view["clockText"], "00:01:15");
        assert_eq!(view["storageText"], "12 GB");
        assert_eq!(view["shotsText"], "42", "shots is trigger points received, not a frame counter, so the fixed-width reading that would justify padding is not what the number is");
        assert_eq!(view["shotPoints"].as_array().unwrap().len(), 2, "the core counted the photos and never said where they were taken - a head could report 42 shots and draw none of them, while QGC marks every one on the map");
        assert_eq!(view["shotPoints"][0]["latitude"], 47.397);
        assert_eq!(view["batteryText"], "80%");
        assert_eq!(view["canPhoto"], false);
        assert_eq!(view["canRecord"], true);
        let none = camera_view(&Fake::new(json!({ "kind": "null" }), json!({ "kind": "null" })), &[]);
        assert_eq!(none["present"], false);
        assert_eq!(none["title"], "Camera");
        assert_eq!(none["canRecord"], false);
    }
    #[test]
    fn the_camera_status_numbers_are_the_ones_the_cpp_and_the_dialect_declare() {
        let header = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/Camera/MavlinkCameraControl.h")).unwrap_or_default();
        assert!(header.contains("enum PhotoCaptureStatus"), "this guard reads MavlinkCameraControl.h, which declares the states these gates branch on; without it the two copies drift in silence");

        let ordinals = |name: &str| -> Vec<(String, i64)> {
            header
                .split_once(&format!("enum {name}"))
                .and_then(|(_, rest)| rest.split_once('{'))
                .and_then(|(_, body)| body.split_once('}'))
                .map(|(body, _)| body.lines().map(|line| line.split("//").next().unwrap_or("")).collect::<Vec<_>>().join(" "))
                .unwrap_or_default()
                .split(',')
                .map(|entry| entry.trim().to_string())
                .filter(|entry| !entry.is_empty())
                .scan(0i64, |next, entry| {
                    let (name, value) = match entry.split_once('=') {
                        Some((name, given)) => (name.trim().to_string(), given.trim().parse().unwrap_or(*next)),
                        None => (entry.clone(), *next),
                    };
                    *next = value + 1;
                    Some((name, value))
                })
                .collect()
        };

        let photo = ordinals("PhotoCaptureStatus");
        let at = |list: &[(String, i64)], name: &str| list.iter().find(|(n, _)| n == name).map(|(_, v)| *v);
        assert_eq!(at(&photo, "PHOTO_CAPTURE_IDLE"), Some(PHOTO_CAPTURE_IDLE), "canPhoto is refused unless the camera is idle, so this number decides whether the shutter is offered at all");
        assert_eq!(at(&photo, "PHOTO_CAPTURE_IN_PROGRESS"), Some(PHOTO_CAPTURE_IN_PROGRESS));
        assert_eq!(at(&photo, "PHOTO_CAPTURE_INTERVAL_IDLE"), Some(PHOTO_CAPTURE_INTERVAL_IDLE), "and the two interval states are the only ones canStopPhoto accepts, so a wrong one hides the stop button mid-timelapse");
        assert_eq!(at(&photo, "PHOTO_CAPTURE_INTERVAL_IN_PROGRESS"), Some(PHOTO_CAPTURE_INTERVAL_IN_PROGRESS));
        assert_eq!(at(&ordinals("VideoCaptureStatus"), "VIDEO_CAPTURE_STATUS_STOPPED"), Some(VIDEO_CAPTURE_STOPPED));

        assert_eq!(
            STORAGE_NOT_SUPPORTED,
            mavlink::dialects::ardupilotmega::StorageStatus::STORAGE_STATUS_NOT_SUPPORTED as i64,
            "the C++ aliases its StorageStatus straight to the MAVLink one, so the dialect is the source here rather than the header"
        );
    }

    #[test]
    fn a_session_that_has_taken_no_photographs_says_so_rather_than_counting_to_five_digits() {
        struct Fresh;
        impl Backend for Fresh {
            fn get(&self, _p: &str) -> String { json!({ "kind": "null" }).to_string() }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "vehicle.cameraTriggerPoints" => json!({ "kind": "object", "count": 0 }).to_string(),
                    "vehicle.cameraManager.currentCameraInstance" => json!({ "kind": "object", "modelName": "ZR30" }).to_string(),
                    _ => json!({ "kind": "null" }).to_string(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        assert_eq!(camera_view(&Fresh, &[])["shotsText"], "\u{2014}", "zero padded to five digits read 'Photos taken 00000' on a fresh session - the padding suits a camera's own frame counter and shots is vehicle.cameraTriggerPoints.count, which is trigger points received");
    }
}
