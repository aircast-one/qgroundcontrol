use std::sync::{Mutex, PoisonError};

use serde_json::{Map, Value, json};

const CAM_MODE_UNDEFINED: i64 = -1;
const CAM_MODE_PHOTO: i64 = 0;
const CAM_MODE_VIDEO: i64 = 1;
const CAM_MODE_SURVEY: i64 = 2;
const CAPTURE_DISABLED: i64 = 0;
const CAPTURE_IDLE: i64 = 1;
const CAPTURE_SINGLE_PHOTO: i64 = 2;
const STORAGE_NOT_SUPPORTED: i64 = 3;
const PHOTO_CAPTURE_SINGLE: i64 = 0;
const PHOTO_IN_PROGRESS_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Inputs {
    pub captures_video: bool,
    pub captures_photos: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Sim {
    vehicle: u8,
    mode: i64,
    photo_until_ms: Option<u64>,
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
        _ => Sim { vehicle, mode: initial_mode(inputs), photo_until_ms: None },
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
    let video_state = match (inputs.captures_video, taking) {
        (false, _) | (true, true) => CAPTURE_DISABLED,
        (true, false) => CAPTURE_IDLE,
    };
    let object = json!({
        "class": "SimulatedCameraControl",
        "modelName": "Simulated Camera",
        "vendor": "QGroundControl",
        "cameraMode": sim.mode,
        "capturePhotosState": photos_state,
        "captureVideoState": video_state,
        "recordTimeStr": "00:00:00",
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

pub fn invoke(vehicle: u8, inputs: Inputs, name: &str, now_ms: u64, trigger: &dyn Fn() -> bool) -> Option<bool> {
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
        "stopTakePhoto" | "toggleVideoRecording" | "startVideoRecording" | "stopVideoRecording" => false,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_simulated_camera_follows_simulatedcameracontrol() {
        let both = Inputs { captures_video: true, captures_photos: true };
        let start = fields(1, both, 0);
        assert_eq!((start["cameraMode"].clone(), start["captureVideoState"].clone(), start["capturePhotosState"].clone()), (json!(CAM_MODE_VIDEO), json!(CAPTURE_IDLE), json!(CAPTURE_IDLE)), "video first when the stream is configured");
        assert_eq!(invoke(1, both, "takePhoto", 0, &|| true), Some(false), "takePhoto refuses outside photo mode");
        assert_eq!(invoke(1, both, "setCameraModePhoto", 0, &|| true), Some(true));
        assert_eq!(invoke(1, both, "takePhoto", 10, &|| true), Some(true));
        assert_eq!(fields(1, both, 100)["capturePhotosState"], json!(CAPTURE_SINGLE_PHOTO));
        assert_eq!(fields(1, both, 100)["captureVideoState"], json!(CAPTURE_DISABLED), "a photo in progress disables video, as captureVideoState does");
        assert_eq!(fields(1, both, 600)["capturePhotosState"], json!(CAPTURE_IDLE), "the in-progress state clears after 500 ms");
        let video_only = Inputs { captures_video: true, captures_photos: false };
        assert_eq!(fields(2, video_only, 0)["capturePhotosState"], json!(CAPTURE_DISABLED), "a new vehicle gets a new camera");
        assert_eq!(invoke(2, video_only, "setCameraModePhoto", 0, &|| true), Some(false), "no modes without photos");
    }
}
