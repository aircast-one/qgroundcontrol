use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

const CAMERA: &str = "vehicle.cameraManager.currentCameraInstance";

#[derive(Clone, Debug, PartialEq)]
enum Request {
    Rect { x: f64, y: f64, width: f64, height: f64 },
    Point { x: f64, y: f64, radius: f64 },
}

fn unit(value: Option<&Value>) -> Option<f64> {
    value?.as_f64().filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
}

fn request(args: &Value) -> Option<Request> {
    let first = args.get(0)?;
    let (x, y) = (unit(first.get("x"))?, unit(first.get("y"))?);
    match (first.get("width"), first.get("height")) {
        (Some(_), Some(_)) => {
            let (width, height) = (unit(first.get("width"))?, unit(first.get("height"))?);
            (width > 0.0 && height > 0.0 && x + width <= 1.0 && y + height <= 1.0).then_some(Request::Rect { x, y, width, height })
        }
        _ => {
            let radius = unit(args.get(1))?;
            (radius > 0.0).then_some(Request::Point { x, y, radius })
        }
    }
}

fn start_refusal(camera: &Value, asked: Option<&Request>) -> Option<(&'static str, &'static str)> {
    match asked {
        _ if camera.get("kind").and_then(Value::as_str) != Some("object") => Some(("noCamera", "No camera is connected.")),
        None => Some(("malformed", "Tracking takes a rectangle {x, y, width, height} or a point {x, y} and a radius, each a fraction of the image from 0 to 1.")),
        Some(Request::Rect { .. }) if !flag(camera, "supportsTrackingRect") => Some(("unsupported", "This camera cannot track a rectangle.")),
        Some(Request::Point { .. }) if !flag(camera, "supportsTrackingPoint") => Some(("unsupported", "This camera cannot track a point.")),
        Some(_) => None,
    }
}

pub fn start(backend: &dyn Backend, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let asked = request(&given);
    let camera = object(&backend.get_fields(CAMERA, "supportsTrackingRect,supportsTrackingPoint,trackingEnabled"));
    if let Some((token, reason)) = start_refusal(&camera, asked.as_ref()) {
        return json!({ "ok": false, "result": Value::Null, "refusal": token, "reason": reason });
    }
    let (invokable, forwarded) = match asked {
        Some(Request::Rect { x, y, width, height }) => ("startTrackingRect", json!([{ "x": x, "y": y, "width": width, "height": height }])),
        Some(Request::Point { x, y, radius }) => ("startTrackingPoint", json!([{ "x": x, "y": y }, radius])),
        None => unreachable!("refused above"),
    };
    let dispatched = flag(&object(&backend.invoke(&format!("{CAMERA}.{invokable}"), &forwarded.to_string())), "ok");
    json!({
        "ok": dispatched,
        "result": Value::Null,
        "refusal": Value::Null,
        "tracking": invokable,
        "trackingEnabled": flag(&camera, "trackingEnabled"),
        "reason": match dispatched { true => Value::Null, false => json!("The camera was not asked to track.") },
    })
}

const THERMAL_MODES: std::ops::RangeInclusive<i64> = 0..=3;

fn camera(backend: &dyn Backend) -> Value {
    object(&backend.get_fields(CAMERA, "hasTracking,trackingEnabled,thermalStreamInstance"))
}

fn present(camera: &Value) -> bool {
    camera.get("kind").and_then(Value::as_str) == Some("object")
}

fn thermal(camera: &Value) -> bool {
    camera.get("thermalStreamInstance").and_then(|s| s.get("kind")).and_then(Value::as_str) == Some("object")
}

pub fn stop(backend: &dyn Backend, path: &str) -> Value {
    let camera = camera(backend);
    let refusal = match () {
        _ if !present(&camera) => Some(("noCamera", "No camera is connected.")),
        _ if !flag(&camera, "hasTracking") => Some(("unsupported", "This camera does not track.")),
        _ => None,
    };
    if let Some((token, reason)) = refusal {
        return json!({ "ok": false, "result": Value::Null, "refusal": token, "reason": reason });
    }
    let dispatched = flag(&object(&backend.invoke(path, "[]")), "ok");
    json!({ "ok": dispatched, "result": Value::Null, "refusal": Value::Null, "reason": match dispatched { true => Value::Null, false => json!("The camera was not asked to stop tracking.") } })
}

fn written(backend: &dyn Backend, path: &str, value: Value) -> bool {
    flag(&object(&backend.set(path, &json!({ "value": value }).to_string())), "ok")
}

fn refused(token: &str, reason: &str) -> Value {
    json!({ "ok": false, "result": false, "refusal": token, "reason": reason })
}

pub fn write(backend: &dyn Backend, path: &str, value: &str) -> Value {
    let asked = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").cloned()).unwrap_or(Value::Null);
    let camera = camera(backend);
    if !present(&camera) {
        return refused("noCamera", "No camera is connected.");
    }
    let property = path.rsplit('.').next().unwrap_or("");
    let (sent, clamped) = match property {
        "thermalMode" => match asked.as_i64().filter(|m| THERMAL_MODES.contains(m)) {
            None => return refused("unknownMode", "A thermal view is 0 off, 1 blended, 2 full or 3 picture in picture."),
            Some(_) if !thermal(&camera) => return refused("noThermal", "This camera has no thermal stream."),
            Some(mode) => (json!(mode), false),
        },
        "thermalOpacity" => match asked.as_f64().filter(|v| v.is_finite()) {
            None => return refused("malformed", "Thermal opacity is a percentage."),
            Some(_) if !thermal(&camera) => return refused("noThermal", "This camera has no thermal stream."),
            Some(opacity) => (json!(opacity.clamp(0.0, 100.0)), !(0.0..=100.0).contains(&opacity)),
        },
        "trackingEnabled" => match asked.as_bool() {
            None => return refused("malformed", "Tracking is turned on with true and off with false."),
            Some(true) if !flag(&camera, "hasTracking") => return refused("unsupported", "This camera does not track."),
            Some(on) => (json!(on), false),
        },
        _ => return refused("malformed", "That is not a camera setting the core writes."),
    };
    let answered = written(backend, path, sent.clone());
    json!({
        "ok": answered,
        "result": answered,
        "refusal": Value::Null,
        "value": sent,
        "clamped": clamped,
        "reason": match answered { true => Value::Null, false => json!("The camera did not take the setting.") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn a_tracking_request_goes_to_the_invokable_that_exists_for_its_shape() {
        assert_eq!(request(&json!([{ "x": 0.4, "y": 0.4, "width": 0.2, "height": 0.2 }])), Some(Request::Rect { x: 0.4, y: 0.4, width: 0.2, height: 0.2 }));
        assert_eq!(request(&json!([{ "x": 0.5, "y": 0.5 }, 0.05])), Some(Request::Point { x: 0.5, y: 0.5, radius: 0.05 }));
        assert_eq!(request(&json!([{ "x": 0.9, "y": 0.4, "width": 0.2, "height": 0.2 }])), None, "a rectangle running off the right edge of the image");
        assert_eq!(request(&json!([{ "x": 120.0, "y": 80.0 }, 50.0])), None, "pixels rather than fractions of the image");
        assert_eq!(request(&json!([{ "x": 0.5, "y": 0.5 }])), None, "a point without its radius");

        let camera = json!({ "kind": "object", "supportsTrackingRect": true, "supportsTrackingPoint": false });
        assert_eq!(start_refusal(&camera, Some(&Request::Point { x: 0.5, y: 0.5, radius: 0.1 })).map(|r| r.0), Some("unsupported"), "startTrackingPoint returns with a qCCritical on a camera without point tracking, which no head hears");
        assert_eq!(start_refusal(&camera, Some(&Request::Rect { x: 0.4, y: 0.4, width: 0.2, height: 0.2 })), None);
        assert_eq!(start_refusal(&json!({ "kind": "null" }), None).map(|r| r.0), Some("noCamera"));

        struct Camera(RefCell<Vec<(String, String)>>);
        impl Backend for Camera {
            fn get(&self, _p: &str) -> String { String::new() }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "object", "supportsTrackingRect": true, "supportsTrackingPoint": true, "trackingEnabled": true }).to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, p: &str, a: &str) -> String {
                self.0.borrow_mut().push((p.to_string(), a.to_string()));
                json!({ "ok": true }).to_string()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let backend = Camera(RefCell::new(Vec::new()));
        assert_eq!(start(&backend, r#"[{"x":0.4,"y":0.4,"width":0.2,"height":0.2}]"#)["tracking"], "startTrackingRect");
        assert_eq!(start(&backend, r#"[{"x":0.5,"y":0.5},0.05]"#)["tracking"], "startTrackingPoint");
        assert_eq!(
            backend.0.borrow().iter().map(|(p, _)| p.rsplit('.').next().unwrap().to_string()).collect::<Vec<_>>(),
            vec!["startTrackingRect", "startTrackingPoint"],
            "the camera interface has had no startTracking since the upstream merge, so every tap on the Android tracking overlay reached a method that does not exist"
        );
    }

    #[test]
    fn a_camera_setting_outside_what_the_camera_can_hold_is_refused_or_reported_clamped() {
        struct Cam(Value, RefCell<Vec<(String, String)>>);
        impl Backend for Cam {
            fn get(&self, _p: &str) -> String { String::new() }
            fn get_fields(&self, _p: &str, _f: &str) -> String { self.0.to_string() }
            fn set(&self, p: &str, v: &str) -> String {
                self.1.borrow_mut().push((p.to_string(), v.to_string()));
                json!({ "ok": true }).to_string()
            }
            fn invoke(&self, p: &str, _a: &str) -> String {
                self.1.borrow_mut().push((p.to_string(), String::new()));
                json!({ "ok": true }).to_string()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let full = Cam(json!({ "kind": "object", "hasTracking": true, "thermalStreamInstance": { "kind": "object" } }), RefCell::new(Vec::new()));
        let mode = |b: &Cam, v: Value| write(b, "vehicle.cameraManager.currentCameraInstance.thermalMode", &json!({ "value": v }).to_string());
        assert_eq!(mode(&full, json!(2))["ok"], true);
        assert_eq!(mode(&full, json!(7))["refusal"], "unknownMode", "setThermalMode stores any number into QSettings, and view.camera then serves a mode no head can draw");
        let opacity = write(&full, "vehicle.cameraManager.currentCameraInstance.thermalOpacity", r#"{"value":140}"#);
        assert_eq!((&opacity["value"], &opacity["clamped"]), (&json!(100.0), &json!(true)), "setThermalOpacity clamps to 100 without a word, and the answer now says so");
        assert_eq!(write(&full, "vehicle.cameraManager.currentCameraInstance.trackingEnabled", r#"{"value":true}"#)["ok"], true);
        assert_eq!(stop(&full, "vehicle.cameraManager.currentCameraInstance.stopTracking")["ok"], true);

        let plain = Cam(json!({ "kind": "object", "hasTracking": false }), RefCell::new(Vec::new()));
        assert_eq!(mode(&plain, json!(1))["refusal"], "noThermal");
        assert_eq!(write(&plain, "vehicle.cameraManager.currentCameraInstance.trackingEnabled", r#"{"value":true}"#)["refusal"], "unsupported");
        assert_eq!(write(&plain, "vehicle.cameraManager.currentCameraInstance.trackingEnabled", r#"{"value":false}"#)["ok"], true, "turning tracking off is always allowed");
        assert_eq!(stop(&plain, "vehicle.cameraManager.currentCameraInstance.stopTracking")["refusal"], "unsupported", "stopTracking sends MAV_CMD_CAMERA_STOP_TRACKING to a camera that never tracked");
        assert_eq!(plain.1.borrow().len(), 1);
        assert_eq!(stop(&Cam(json!({ "kind": "null" }), RefCell::new(Vec::new())), "x")["refusal"], "noCamera");
    }
}
