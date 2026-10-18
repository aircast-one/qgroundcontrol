use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use crate::cameratrack::{Request, request};
use crate::detections::Source;
use crate::read::flag;
use crate::router::Backend;

pub const TRACK_CHANGED: &str = "core.deviceTrack@changed";
const CAMERA: &str = "vehicle.cameraManager.currentCameraInstance";
const TRACK_TIMEOUT: Duration = Duration::from_secs(3);
const JSON: [(&str, &str); 2] = [
    ("Content-Type", "application/json"),
    ("Accept", "application/json"),
];

static ARMED: AtomicBool = AtomicBool::new(false);
static ANNOUNCED: Mutex<Value> = Mutex::new(Value::Null);

fn camera_tracks(camera: &Value) -> bool {
    flag(camera, "hasTracking")
}

fn tracker() -> Option<(Source, Option<Value>)> {
    crate::detections::lock().tracker(crate::hub::now_ms())
}

fn reading(found: Option<(Source, Option<Value>)>, armed: bool) -> Option<Value> {
    found.map(|(_, target)| {
        json!({
            "supported": true,
            "requested": armed,
            "reported": target.is_some(),
            "shapes": ["rectangle", "point"],
            "rect": target,
            "via": "device",
        })
    })
}

pub fn tracking(camera: &Value) -> Option<Value> {
    (!camera_tracks(camera))
        .then(|| reading(tracker(), ARMED.load(Ordering::Relaxed)))
        .flatten()
}

pub fn take_changed() -> bool {
    let now = reading(tracker(), ARMED.load(Ordering::Relaxed)).unwrap_or(Value::Null);
    let mut announced = ANNOUNCED.lock().unwrap_or_else(PoisonError::into_inner);
    let changed = *announced != now;
    *announced = now;
    changed
}

pub fn applies(backend: &dyn Backend) -> Option<Source> {
    (!camera_tracks(&backend.value_fields(CAMERA, "hasTracking")))
        .then(tracker)
        .flatten()
        .map(|(source, _)| source)
}

fn refusal(status: u16, bytes: &[u8]) -> String {
    serde_json::from_slice::<Value>(bytes)
        .ok()
        .and_then(|body| {
            body.get("error")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| format!("The device refused to track (HTTP {status})."))
}

fn answer(
    sent: Result<(u16, Vec<u8>), String>,
    accepted: impl Fn(&Value) -> Option<&'static str>,
) -> Value {
    let (ok, reason) = match sent {
        Err(_) => (false, Some("The device did not answer.".to_string())),
        Ok((status, bytes)) if !(200..300).contains(&status) => {
            (false, Some(refusal(status, &bytes)))
        }
        Ok((_, bytes)) => match accepted(&serde_json::from_slice(&bytes).unwrap_or(Value::Null)) {
            Some(reason) => (false, Some(reason.to_string())),
            None => (true, None),
        },
    };
    json!({ "ok": ok, "result": Value::Null, "refusal": Value::Null, "reason": reason })
}

pub fn start(source: &Source, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let (url, body) = match request(&given) {
        None => {
            return json!({ "ok": false, "result": Value::Null, "refusal": "malformed", "reason": "Tracking takes a rectangle {x, y, width, height} or a point {x, y} and a radius, each a fraction of the image from 0 to 1." });
        }
        Some(Request::Point { x, y, .. }) => (source.track_url(), json!({ "x": x, "y": y })),
        Some(Request::Rect {
            x,
            y,
            width,
            height,
        }) => (
            format!("{}/visual", source.track_url()),
            json!({ "x": x, "y": y, "w": width, "h": height }),
        ),
    };
    let sent = crate::http::send(
        "PUT",
        &url,
        &JSON,
        Some(body.to_string().as_bytes()),
        TRACK_TIMEOUT,
    );
    answer(sent, |reply| {
        (reply.get("status").and_then(Value::as_str) == Some("no-target"))
            .then_some("Nothing to track there.")
    })
}

pub fn stop(source: &Source) -> Value {
    answer(
        crate::http::send("DELETE", &source.track_url(), &JSON, None, TRACK_TIMEOUT),
        |_| None,
    )
}

pub fn write(value: &str) -> Value {
    match serde_json::from_str::<Value>(value)
        .ok()
        .and_then(|v| v.get("value")?.as_bool())
    {
        None => {
            json!({ "ok": false, "result": false, "refusal": "malformed", "reason": "Tracking is turned on with true and off with false." })
        }
        Some(on) => {
            ARMED.store(on, Ordering::Relaxed);
            json!({ "ok": true, "result": true, "refusal": Value::Null, "value": on, "clamped": false, "reason": Value::Null })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Source {
        Source {
            host: "falcon.local".into(),
            camera: "gimbal".into(),
            port: None,
        }
    }

    #[test]
    fn the_device_tracker_is_offered_only_while_its_detector_is_reporting() {
        assert_eq!(
            reading(None, true),
            None,
            "no detections from an aircastd camera means no tracker to drive"
        );
        let idle = reading(Some((source(), None)), false).unwrap();
        assert_eq!(
            (
                &idle["supported"],
                &idle["requested"],
                &idle["reported"],
                &idle["rect"]
            ),
            (&json!(true), &json!(false), &json!(false), &Value::Null)
        );
        let rect = json!({ "x": 0.1, "y": 0.2, "width": 0.3, "height": 0.4 });
        let locked = reading(Some((source(), Some(rect.clone()))), true).unwrap();
        assert_eq!(
            (&locked["reported"], &locked["rect"]),
            (&json!(true), &rect),
            "the box aircastd marks as its target is the tracked box"
        );
    }

    #[test]
    fn a_camera_that_tracks_over_mavlink_keeps_its_own_tracking() {
        assert_eq!(
            tracking(&json!({ "kind": "object", "hasTracking": true })),
            None
        );
    }

    #[test]
    fn a_point_locks_on_a_detection_and_a_rectangle_starts_the_visual_tracker() {
        assert_eq!(
            source().track_url(),
            "http://falcon.local/api/streams/gimbal/track"
        );
        let refused = answer(
            Ok((
                400,
                br#"{"error":"front is not the camera on the gimbal"}"#.to_vec(),
            )),
            |_| None,
        );
        assert_eq!(
            (&refused["ok"], &refused["reason"]),
            (
                &json!(false),
                &json!("front is not the camera on the gimbal")
            ),
            "aircastd's own words reach the pilot"
        );
        let missed = answer(Ok((200, br#"{"status":"no-target"}"#.to_vec())), |r| {
            (r["status"] == "no-target").then_some("Nothing to track there.")
        });
        assert_eq!(
            missed["reason"], "Nothing to track there.",
            "a tap on empty sky is not a lock"
        );
        assert_eq!(
            answer(Ok((200, br#"{"status":"tracking"}"#.to_vec())), |_| None)["ok"],
            true
        );
        assert_eq!(
            answer(Err("refused".into()), |_| None)["reason"],
            "The device did not answer."
        );
        assert_eq!(
            start(&source(), r#"[{"x":2,"y":0.5},0.05]"#)["refusal"],
            "malformed",
            "pixels are refused before anything is sent"
        );
    }

    #[test]
    fn arming_is_held_by_the_core() {
        assert_eq!(write(r#"{"value":"yes"}"#)["refusal"], "malformed");
        assert_eq!(write(r#"{"value":true}"#)["ok"], true);
        assert!(ARMED.load(Ordering::Relaxed));
        write(r#"{"value":false}"#);
    }
}
