use std::sync::{Mutex, PoisonError};

use serde_json::{Map, Value, json};

const CAM_MODE_UNDEFINED: i64 = -1;
const CAM_MODE_PHOTO: i64 = 0;
const CAM_MODE_VIDEO: i64 = 1;
const CAM_MODE_SURVEY: i64 = 2;
const CAPTURE_DISABLED: i64 = 0;
const CAPTURE_IDLE: i64 = 1;
const CAPTURE_SINGLE_PHOTO: i64 = 2;
const CAPTURE_VIDEO_CAPTURING: i64 = 2;
const STORAGE_NOT_SUPPORTED: i64 = 3;
const PHOTO_CAPTURE_SINGLE: i64 = 0;
const PHOTO_IN_PROGRESS_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Inputs {
    pub captures_video: bool,
    pub captures_photos: bool,
    pub recording: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Sim {
    vehicle: u8,
    mode: i64,
    photo_until_ms: Option<u64>,
    recording_since_ms: Option<u64>,
}

static SIM: Mutex<Option<Sim>> = Mutex::new(None);

fn initial_mode(inputs: Inputs) -> i64 {
    match (inputs.captures_video, inputs.captures_photos) {
        (true, _) => CAM_MODE_VIDEO,
        (false, true) => CAM_MODE_PHOTO,
        _ => CAM_MODE_UNDEFINED,
    }
}

fn current(vehicle: u8, inputs: Inputs) -> Sim {
    let mut held = SIM.lock().unwrap_or_else(PoisonError::into_inner);
    let sim = match *held {
        Some(sim) if sim.vehicle == vehicle => sim,
        _ => Sim { vehicle, mode: initial_mode(inputs), photo_until_ms: None, recording_since_ms: None },
    };
    *held = Some(sim);
    sim
}

fn store(sim: Sim) {
    *SIM.lock().unwrap_or_else(PoisonError::into_inner) = Some(sim);
}

pub fn fields(vehicle: u8, inputs: Inputs, now_ms: u64) -> Map<String, Value> {
    let sim = current(vehicle, inputs);
    let taking = sim.photo_until_ms.is_some_and(|until| now_ms < until);
    let photos_state = match (taking, inputs.captures_photos) {
        (true, _) => CAPTURE_SINGLE_PHOTO,
        (false, true) => CAPTURE_IDLE,
        (false, false) => CAPTURE_DISABLED,
    };
    let video_state = match (inputs.captures_video, inputs.recording, taking) {
        (false, ..) => CAPTURE_DISABLED,
        (true, true, _) => CAPTURE_VIDEO_CAPTURING,
        (true, false, true) => CAPTURE_DISABLED,
        (true, false, false) => CAPTURE_IDLE,
    };
    let elapsed_s = sim.recording_since_ms.map_or(0, |since| now_ms.saturating_sub(since) / 1000);
    let object = json!({
        "class": "SimulatedCameraControl",
        "modelName": "Simulated Camera",
        "vendor": "QGroundControl",
        "cameraMode": sim.mode,
        "capturePhotosState": photos_state,
        "captureVideoState": video_state,
        "recordTimeStr": format!("{:02}:{:02}:{:02}", elapsed_s / 3600, elapsed_s / 60 % 60, elapsed_s % 60),
        "storageStatus": STORAGE_NOT_SUPPORTED,
        "storageFreeStr": "",
        "capturesPhotos": inputs.captures_photos,
        "capturesVideo": inputs.captures_video,
        "hasModes": inputs.captures_photos && inputs.captures_video,
        "photosInVideoMode": true,
        "videoInPhotoMode": false,
        "photoCaptureMode": PHOTO_CAPTURE_SINGLE,
        "photoLapse": 1,
        "photoLapseCount": 0,
        "batteryRemaining": -1,
        "hasZoom": false,
        "zoomLevel": 1,
        "hasTracking": false,
        "thermalMode": 0,
        "thermalOpacity": 0,
        "thermalStreamInstance": null,
        "trackingEnabled": false,
        "trackingImageIsActive": false,
        "trackingImageRect": null,
        "supportsTrackingRect": false,
        "supportsTrackingPoint": false,
    });
    object.as_object().cloned().unwrap_or_default()
}

pub struct Recorder<'a> {
    pub start: &'a dyn Fn(),
    pub stop: &'a dyn Fn(),
}

pub fn invoke(vehicle: u8, inputs: Inputs, name: &str, now_ms: u64, trigger: &dyn Fn() -> bool, recorder: &Recorder) -> Option<bool> {
    let sim = current(vehicle, inputs);
    let has_modes = inputs.captures_photos && inputs.captures_video;
    let with_mode = |mode: i64| {
        if has_modes {
            store(Sim { mode, ..sim });
        }
        has_modes
    };
    Some(match name {
        "setCameraModePhoto" => with_mode(CAM_MODE_PHOTO),
        "setCameraModeVideo" => with_mode(CAM_MODE_VIDEO),
        "toggleCameraMode" => with_mode(if sim.mode == CAM_MODE_VIDEO { CAM_MODE_PHOTO } else { CAM_MODE_VIDEO }),
        "takePhoto" => {
            let idle = sim.photo_until_ms.is_none_or(|until| now_ms >= until);
            let ready = inputs.captures_photos && idle && matches!(sim.mode, CAM_MODE_PHOTO | CAM_MODE_SURVEY) && trigger();
            if ready {
                store(Sim { photo_until_ms: Some(now_ms + PHOTO_IN_PROGRESS_MS), ..sim });
            }
            ready
        }
        "startVideoRecording" => start_recording(sim, inputs, now_ms, recorder),
        "stopVideoRecording" => stop_recording(sim, inputs, recorder),
        "toggleVideoRecording" if inputs.recording => stop_recording(sim, inputs, recorder),
        "toggleVideoRecording" => start_recording(sim, inputs, now_ms, recorder),
        "stopTakePhoto" => false,
        _ => return None,
    })
}

fn start_recording(sim: Sim, inputs: Inputs, now_ms: u64, recorder: &Recorder) -> bool {
    let ready = inputs.captures_video && sim.mode != CAM_MODE_PHOTO && !inputs.recording;
    if ready {
        (recorder.start)();
        store(Sim { recording_since_ms: Some(now_ms), ..sim });
    }
    ready
}

fn stop_recording(sim: Sim, inputs: Inputs, recorder: &Recorder) -> bool {
    if inputs.recording {
        (recorder.stop)();
        store(Sim { recording_since_ms: None, ..sim });
    }
    inputs.recording
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_RECORDER: Recorder = Recorder { start: &|| {}, stop: &|| {} };
    static ONE_CAMERA: Mutex<()> = Mutex::new(());

    #[test]
    fn recording_follows_simulatedcameracontrol_start_and_stop_video_recording() {
        let _camera = ONE_CAMERA.lock().unwrap_or_else(PoisonError::into_inner);
        let started = std::cell::Cell::new(0);
        let stopped = std::cell::Cell::new(0);
        let recorder = Recorder { start: &|| started.set(started.get() + 1), stop: &|| stopped.set(stopped.get() + 1) };
        let idle = Inputs { captures_video: true, captures_photos: true, recording: false };
        assert_eq!(invoke(7, idle, "setCameraModePhoto", 0, &|| true, &recorder), Some(true));
        assert_eq!(invoke(7, idle, "toggleVideoRecording", 0, &|| true, &recorder), Some(false), "no video in photo mode");
        assert_eq!(invoke(7, idle, "setCameraModeVideo", 0, &|| true, &recorder), Some(true));
        assert_eq!(invoke(7, idle, "toggleVideoRecording", 1_000, &|| true, &recorder), Some(true));
        let recording = Inputs { recording: true, ..idle };
        assert_eq!(fields(7, recording, 66_000)["captureVideoState"], json!(CAPTURE_VIDEO_CAPTURING));
        assert_eq!(fields(7, recording, 3_662_000)["recordTimeStr"], json!("01:01:01"), "hh:mm:ss since the start, as recordTimeStr formats recordTime");
        assert_eq!(invoke(7, recording, "startVideoRecording", 0, &|| true, &recorder), Some(false), "already recording");
        assert_eq!(invoke(7, recording, "toggleVideoRecording", 2_000, &|| true, &recorder), Some(true));
        assert_eq!((started.get(), stopped.get()), (1, 1));
        assert_eq!(fields(7, idle, 9_000)["recordTimeStr"], json!("00:00:00"), "the timer stops with the recording");
        assert_eq!(invoke(7, idle, "stopVideoRecording", 0, &|| true, &recorder), Some(false), "not recording");
    }

    #[test]
    fn the_simulated_camera_follows_simulatedcameracontrol() {
        let _camera = ONE_CAMERA.lock().unwrap_or_else(PoisonError::into_inner);
        let both = Inputs { captures_video: true, captures_photos: true, recording: false };
        let start = fields(1, both, 0);
        assert_eq!((start["cameraMode"].clone(), start["captureVideoState"].clone(), start["capturePhotosState"].clone()), (json!(CAM_MODE_VIDEO), json!(CAPTURE_IDLE), json!(CAPTURE_IDLE)), "video first when the stream is configured");
        assert_eq!(invoke(1, both, "takePhoto", 0, &|| true, &NO_RECORDER), Some(false), "takePhoto refuses outside photo mode");
        assert_eq!(invoke(1, both, "setCameraModePhoto", 0, &|| true, &NO_RECORDER), Some(true));
        assert_eq!(invoke(1, both, "takePhoto", 10, &|| true, &NO_RECORDER), Some(true));
        assert_eq!(fields(1, both, 100)["capturePhotosState"], json!(CAPTURE_SINGLE_PHOTO));
        assert_eq!(fields(1, both, 100)["captureVideoState"], json!(CAPTURE_DISABLED), "a photo in progress disables video, as captureVideoState does");
        assert_eq!(fields(1, both, 600)["capturePhotosState"], json!(CAPTURE_IDLE), "the in-progress state clears after 500 ms");
        let video_only = Inputs { captures_video: true, captures_photos: false, recording: false };
        assert_eq!(fields(2, video_only, 0)["capturePhotosState"], json!(CAPTURE_DISABLED), "a new vehicle gets a new camera");
        assert_eq!(invoke(2, video_only, "setCameraModePhoto", 0, &|| true, &NO_RECORDER), Some(false), "no modes without photos");
    }
}
