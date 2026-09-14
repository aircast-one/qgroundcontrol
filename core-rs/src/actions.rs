use serde_json::{Value, json};

use crate::missionkinds::{default_area, default_line, insertable, lookup, refusal};
use crate::read::object;
use crate::router::Backend;

const INSERT: &str = "mission.insert";
const REMOVE: &str = "mission.remove";
const ORBIT: &str = "guided.orbit";
const ACTIVATE: &str = "vehicles.setActive";
const PHOTO: &str = "camera.takePhoto";
const RECORD: &str = "camera.toggleRecording";
const MODE: &str = "camera.setMode";
const STOP_PHOTO: &str = "camera.stopPhoto";
const UNDO: &str = "plan.undo";
const REDO: &str = "plan.redo";
const ZOOM: &str = "vehicle.cameraManager.currentCameraInstance.zoomLevel";

pub const OWNED: &[&str] = &[INSERT, REMOVE, ORBIT, ACTIVATE, PHOTO, RECORD, MODE, STOP_PHOTO, UNDO, REDO];

pub fn owns(path: &str) -> bool {
    OWNED.contains(&path)
}

// A write had no route to the core at all: router.set refused view paths and passed everything
// else straight to the backend, and owns() was consulted only by invoke. A write is not a read
// going the other way, so it needs its own door rather than either of the two that existed.
pub const OWNED_WRITES: &[&str] = &[ZOOM];

pub fn owns_write(path: &str) -> bool {
    OWNED_WRITES.contains(&path)
}

pub fn write(backend: &dyn Backend, path: &str, value: &str) -> Value {
    match path {
        ZOOM => zoom(backend, value),
        _ => json!({ "ok": false, "reason": format!("{path} is not a write the core performs") }),
    }
}

// setZoomLevel drops the write on a camera with no zoom and on a null vehicle, and silently CLAMPS
// to 0..100 in between - so a head asking for 150 is told the write succeeded and the camera goes
// to 100. Three ways to be wrong about what happened, none of them reported. The clamp travels
// back here rather than being refused, because clamping is what the camera does and the head
// asking too high is not an error; being unable to see it is.
const ZOOM_LOWEST: f64 = 0.0;
const ZOOM_HIGHEST: f64 = 100.0;

fn zoom(backend: &dyn Backend, value: &str) -> Value {
    let Some(asked) = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value")?.as_f64()) else {
        return json!({ "ok": false, "result": false, "reason": "A zoom level has to be a number." });
    };
    // camera_view makes seven backend round trips - trigger points, labels, shot points - and this
    // needs two flags off one object. A slider sends a write per drag tick, so the whole view per
    // tick is six reads of things nobody asked for.
    let camera = object(&backend.get_fields(CAMERA, "modelName,hasZoom"));
    if !crate::video::camera_present(&camera) {
        return json!({ "ok": false, "result": false, "reason": "No camera is connected." });
    }
    if !crate::read::flag(&camera, "hasZoom") {
        return json!({ "ok": false, "result": false, "reason": "This camera has no zoom." });
    }
    let level = asked.clamp(ZOOM_LOWEST, ZOOM_HIGHEST);
    // A bridge set answers {"ok": true} and carries no "result", so result_flag - which requires
    // both - reads every successful write as a failure.
    let answered = crate::read::flag(&object(&backend.set(ZOOM, &json!({ "value": level }).to_string())), "ok");
    json!({
        "ok": answered,
        "result": answered,
        "asked": asked,
        "level": level,
        "clamped": level != asked,
        "reason": match answered {
            true => Value::Null,
            false => json!("The camera did not take the zoom level."),
        },
    })
}

pub fn run(backend: &dyn Backend, path: &str, args: &str) -> Value {
    match path {
        INSERT => insert(backend, args),
        REMOVE => remove(backend, args),
        ORBIT => orbit(backend, args),
        ACTIVATE => activate(backend, args),
        PHOTO | RECORD | MODE | STOP_PHOTO => camera(backend, path, args),
        UNDO | REDO => step(backend, path),
        _ => json!({ "ok": false, "reason": format!("{path} is not an action the core performs") }),
    }
}

// PlanMasterController snapshots the plan on a timer that only runs while undoTracking is set. Every
// head turns it on when its plan screen appears and OFF again when that screen goes away -
// PlanView.qml binds it to planActive, Mission.swift and PlanTab.kt set and clear it on dispose - so
// a plan screen that is not open is the ordinary state, not a broken one. canUndo is false there for
// a reason that has nothing to do with the stack, and a gate reading canUndo alone answers "nothing
// to undo" to a head that has made twenty edits and merely navigated away. The two are told apart.
//
// undo() returns void, and QGCBridgeCore answers a void invoke with {"ok": bool} and NO result key -
// ok being "the call dispatched", not "the plan moved". Reading result here would report failure on
// every successful undo, so it is not read and not served.
fn step(backend: &dyn Backend, path: &str) -> Value {
    let plan = object(&backend.get_fields("plan", "canUndo,canRedo,undoTracking"));
    let undoing = path == UNDO;
    let word = if undoing { "undo" } else { "redo" };
    if !crate::read::flag(&plan, "undoTracking") {
        return json!({
            "ok": false,
            "reason": format!("This plan is not recording edits, so there is nothing to {word}."),
            "refusal": "notTracking",
        });
    }
    if !crate::read::flag(&plan, if undoing { "canUndo" } else { "canRedo" }) {
        return json!({ "ok": false, "reason": format!("Nothing to {word}."), "refusal": "nothingTo" });
    }
    let dispatched = crate::read::flag(&object(&backend.invoke(path, "[]")), "ok");
    json!({
        "ok": dispatched,
        "reason": match dispatched { true => Value::Null, false => json!(format!("The plan did not take the {word}.")) },
        "refusal": Value::Null,
    })
}

const CAMERA: &str = "vehicle.cameraManager.currentCameraInstance";

// VehicleCameraControl refuses on terms it never reports back: takePhoto returns false with only a
// qCWarning, so a head that fires blind cannot tell a photo taken from a photo refused. view.camera
// already computes every one of those terms for its gates. Doing the check and the call together is
// what turns silence into an answer, and it is the same read-then-write the mission actions do.
// These reach the camera through the bridge rather than through Hub::guided, so unlike the guided
// actions they work in a default build with no QGC_CORE_LINKS.
fn camera(backend: &dyn Backend, path: &str, args: &str) -> Value {
    // What is being ASKED is resolved before whether it can be done. A mode the core has no name
    // for is a malformed call with or without hardware attached, and answering it "No camera is
    // connected" hides a caller's bug behind a fact about the rig - two different failures wearing
    // one sentence, which is the shape this whole file exists to avoid.
    let mode = match path {
        MODE => match serde_json::from_str::<Value>(args).ok().and_then(|a| a.as_array()?.first()?.as_str().map(str::to_string)).as_deref() {
            Some("photo") => Some("Photo"),
            Some("video") => Some("Video"),
            Some(other) => return json!({ "ok": false, "result": false, "unknown": other, "reason": format!("a camera mode is photo or video, not {other}") }),
            None => return json!({ "ok": false, "result": false, "reason": "camera.setMode takes photo or video" }),
        },
        _ => None,
    };
    let view = crate::video::camera_view(backend, &[]);
    let gate = |name: &str| view.get(name).and_then(Value::as_bool) == Some(true);
    if !gate("present") {
        return json!({ "ok": false, "result": false, "reason": "No camera is connected." });
    }
    let (allowed, refusal, invokable) = match path {
        PHOTO => (gate("canPhoto"), photo_refusal(&view), "takePhoto".to_string()),
        RECORD => (gate("canRecord"), "This camera cannot record video in the mode it is in.".to_string(), "toggleVideoRecording".to_string()),
        STOP_PHOTO => (gate("canStopPhoto"), "The camera is not taking an interval capture.".to_string(), "stopTakePhoto".to_string()),
        MODE => match mode {
            Some(wanted) => (gate("canChangeMode"), mode_refusal(&view), format!("setCameraMode{wanted}")),
            None => return json!({ "ok": false, "result": false, "reason": "camera.setMode takes photo or video" }),
        },
        _ => return json!({ "ok": false, "reason": format!("{path} is not a camera action") }),
    };
    if !allowed {
        return json!({ "ok": false, "result": false, "reason": refusal });
    }
    // The Qt method's own bool is the answer to "did it happen", and these gates cannot see every
    // reason it says no - _resetting is not a property the core can read. Ignoring it would put the
    // silence back one layer down, having just removed it. It also travels as `result`, because
    // that is where QGCBridgeCore puts a return value and a head must not have to learn which
    // paths the core has claimed in order to read one.
    let took = crate::read::flag(&object(&backend.invoke(&format!("{CAMERA}.{invokable}"), "[]")), "result");
    // No post-state travels back. setCameraModePhoto and takePhoto set their own status before
    // returning, but startVideoRecording only sends MAV_CMD_VIDEO_START_CAPTURE and waits for
    // CAMERA_CAPTURE_STATUS - so isRecording read here is the value from BEFORE the toggle, while
    // mode and isTakingPhoto beside it are current. Two fresh fields and one stale one with nothing
    // to tell them apart is worse than none: a head toggling record would read false and conclude
    // it failed. The action answers whether the command was taken; view.camera is watched and is
    // where the state comes from when the vehicle confirms it.
    // A shutter press in timelapse mode starts lapseCount shots at lapseSeconds apart, and with a
    // count of zero it does not stop on its own. Answering ok for that and for one photo is the
    // same button meaning two things, so the answer says which capture it began.
    let timelapse = path == PHOTO && view.get("photoMode").and_then(Value::as_str) == Some("timelapse");
    json!({
        "ok": took,
        "result": took,
        "reason": match took { true => Value::Null, false => json!("The camera did not carry out the command.") },
        "started": match (took, timelapse) { (false, _) => Value::Null, (true, true) => json!("timelapse"), (true, false) => json!("single") },
        "lapseCount": timelapse.then(|| view.get("lapseCount").cloned().unwrap_or(Value::Null)),
        "lapseSeconds": timelapse.then(|| view.get("lapseSeconds").cloned().unwrap_or(Value::Null)),
        "lapseUnlimited": timelapse.then(|| view.get("lapseUnlimited").cloned().unwrap_or(Value::Null)),
    })
}

fn photo_refusal(view: &Value) -> String {
    match view.get("isTakingPhoto").and_then(Value::as_bool) {
        Some(true) => "The camera is still taking the last photo.".to_string(),
        _ => "This camera cannot take a photo in the mode it is in.".to_string(),
    }
}

fn mode_refusal(view: &Value) -> String {
    match (view.get("hasModes").and_then(Value::as_bool), view.get("isRecording").and_then(Value::as_bool)) {
        (Some(false), _) => "This camera has only one mode.".to_string(),
        (_, Some(true)) => "The camera is recording, so it cannot leave video mode.".to_string(),
        _ => "The camera is busy capturing, so it cannot change mode.".to_string(),
    }
}

fn point_at(backend: &dyn Backend, index: i64) -> Option<i64> {
    let count = item_count(backend)?;
    if count <= 0 {
        return None;
    }
    let wanted = match index {
        index if index < 0 => count - 1,
        index => (index - 1).clamp(0, count - 1),
    };
    let sequence = serde_json::from_str::<Value>(&backend.get(&format!("plan.missionController.visualItems.{wanted}.sequenceNumber")))
        .ok()
        .and_then(|v| v.get("value").and_then(Value::as_i64))?;
    backend.invoke("plan.missionController.setCurrentPlanViewSeqNum", &json!([sequence, true]).to_string());
    Some(sequence)
}

fn insert(backend: &dyn Backend, args: &str) -> Value {
    let args: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let Some(args) = args.as_array() else {
        return json!({ "ok": false, "reason": "mission.insert takes a kind, a latitude, a longitude and an index" });
    };
    let number = |index: usize| args.get(index).and_then(Value::as_f64).filter(|value| value.is_finite());
    let (Some(named), Some(latitude), Some(longitude)) = (args.first().and_then(Value::as_str), number(1), number(2)) else {
        return json!({ "ok": false, "reason": "mission.insert takes a kind, a latitude, a longitude and an index" });
    };
    let Some(kind) = lookup(named) else {
        return json!({ "ok": false, "unknown": named, "reason": format!("the core has no {named} in its catalogue, so this one has to be inserted directly") });
    };
    let index = args.get(3).and_then(Value::as_i64).unwrap_or(-1);
    let Some(held) = item_count(backend).filter(|count| *count > 0) else {
        return json!({ "ok": false, "reason": "the plan did not say how many items it holds" });
    };
    if index >= 0 && (index < 1 || index > held) {
        return json!({ "ok": false, "reason": format!("this plan has no place {index} to put an item") });
    }
    let Some(at_sequence) = point_at(backend, index) else {
        return json!({ "ok": false, "reason": "the plan view has no item selected, so there is no point to insert against" });
    };
    let at_sequence = Some(at_sequence);
    if let Some(reason) = refusal(kind, &insertable(backend)) {
        return json!({ "ok": false, "reason": reason, "refused": kind.id, "atSequence": at_sequence });
    }
    let at = json!({ "latitude": latitude, "longitude": longitude, "altitude": 0.0 });
    let call: Vec<Value> = match kind.complex_name {
        Some(name) => vec![json!(name), at, json!(index), json!(true)],
        None => vec![at, json!(index), json!(true)],
    };
    let answered: Value = serde_json::from_str(&backend.invoke(&format!("plan.missionController.{}", kind.invokable), &Value::Array(call).to_string())).unwrap_or(Value::Null);
    if answered.get("ok").and_then(Value::as_bool) != Some(true) {
        return json!({ "ok": false, "reason": answered.get("reason").and_then(Value::as_str).unwrap_or("the plan refused the item").to_string() });
    }
    if item_count(backend) != Some(held + 1) {
        return json!({ "ok": false, "reason": "the plan did not grow, so nothing was added" });
    }
    let Some(placed) = inserted_index(backend) else {
        return json!({ "ok": false, "reason": "the item was added and then could not be found, so the plan is not in a state to build on" });
    };
    match shape(backend, kind, placed, latitude, longitude) {
        Ok(()) => json!({ "ok": true, "inserted": kind.id, "index": placed, "atSequence": at_sequence }),
        Err(reason) => {
            backend.invoke("plan.missionController.removeVisualItem", &json!([placed]).to_string());
            json!({ "ok": false, "reason": reason, "removed": kind.id })
        }
    }
}

const SETTINGS_ITEM: i64 = 0;

fn remove(backend: &dyn Backend, args: &str) -> Value {
    let args: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let Some(index) = args.as_array().and_then(|args| args.first()).and_then(Value::as_i64) else {
        return json!({ "ok": false, "reason": "mission.remove takes the index of the item to remove" });
    };
    let Some(count) = item_count(backend) else {
        return json!({ "ok": false, "reason": "the plan did not say how many items it holds" });
    };
    if index == SETTINGS_ITEM {
        return json!({ "ok": false, "reason": "The first entry holds the plan's own settings and cannot be removed." });
    }
    if index < 0 || index >= count {
        return json!({ "ok": false, "reason": format!("this plan has no item {index}") });
    }
    backend.invoke("plan.missionController.removeVisualItem", &json!([index]).to_string());
    match item_count(backend) {
        Some(now) if now == count - 1 => json!({ "ok": true, "removed": index, "remaining": now }),
        Some(now) if now < count - 1 => json!({ "ok": false, "reason": format!("the plan lost {} items rather than the one asked for", count - now) }),
        _ => json!({ "ok": false, "reason": "the plan still holds the item, so it was not removed" }),
    }
}

fn orbit(backend: &dyn Backend, args: &str) -> Value {
    let args: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let Some(args) = args.as_array() else {
        return json!({ "ok": false, "reason": "guided.orbit takes a latitude, a longitude, a radius, a direction and a height above the launch point" });
    };
    let number = |index: usize| args.get(index).and_then(Value::as_f64).filter(|value| value.is_finite());
    let (Some(latitude), Some(longitude), Some(radius), Some(above_home)) = (number(0), number(1), number(2), number(4)) else {
        return json!({ "ok": false, "reason": "guided.orbit takes a latitude, a longitude, a radius, a direction and a height above the launch point" });
    };
    if radius <= 0.0 {
        return json!({ "ok": false, "reason": "An orbit needs a radius to fly around." });
    }
    let Some(clockwise) = args.get(3).and_then(Value::as_bool) else {
        return json!({ "ok": false, "reason": "An orbit has to turn one way or the other." });
    };
    if !crate::read::flag(&object(&backend.get_fields("vehicle", "orbitModeSupported")), "orbitModeSupported") {
        return json!({ "ok": false, "reason": "This vehicle does not support orbiting." });
    }
    let limit = |name: &str| crate::read::value_number(&backend.get(&format!("settings.flyViewSettings.{name}.rawValue")));
    match (limit("guidedMinimumAltitude"), limit("guidedMaximumAltitude")) {
        (Some(lowest), Some(highest)) if above_home < lowest || above_home > highest => {
            return json!({ "ok": false, "reason": format!("An orbit has to be between {lowest} and {highest} metres above the launch point.") });
        }
        _ => {}
    }
    let home = object(&backend.get("vehicle.homePosition"));
    if home.get("valid").and_then(Value::as_bool) != Some(true) {
        return json!({ "ok": false, "reason": "The vehicle has not reported where it launched from, so there is nothing to measure the orbit height against." });
    }
    let Some(home_altitude) = home.get("altitude").and_then(Value::as_f64).filter(|value| value.is_finite()) else {
        return json!({ "ok": false, "reason": "The launch position carries no altitude, so an orbit height above sea level cannot be worked out." });
    };
    let signed = if clockwise { radius } else { -radius };
    let amsl = home_altitude + above_home;
    let called: Value = serde_json::from_str(&backend.invoke(
        "vehicle.guidedModeOrbit",
        &json!([{ "latitude": latitude, "longitude": longitude, "altitude": 0.0 }, signed, amsl]).to_string(),
    ))
    .unwrap_or(Value::Null);
    match called.get("ok").and_then(Value::as_bool) {
        Some(true) => json!({ "ok": true, "radius": signed, "altitudeAmsl": amsl }),
        _ => json!({ "ok": false, "reason": called.get("reason").and_then(Value::as_str).unwrap_or("the vehicle refused the orbit").to_string() }),
    }
}

fn activate(backend: &dyn Backend, args: &str) -> Value {
    let args: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let Some(wanted) = args.as_array().and_then(|args| args.first()).and_then(Value::as_i64) else {
        return json!({ "ok": false, "reason": "vehicles.setActive takes the id of the vehicle to command" });
    };
    let count = serde_json::from_str::<Value>(&backend.get("vehicles.vehicles.count")).ok().and_then(|v| v.get("value").and_then(Value::as_i64)).unwrap_or(0);
    let found = (0..count).find(|index| {
        crate::read::integer(&object(&backend.get_fields(&format!("vehicles.vehicles.{index}"), "id")), "id") == Some(wanted)
    });
    let Some(index) = found else {
        return json!({ "ok": false, "reason": format!("no vehicle {wanted} is connected") });
    };
    let written: Value = serde_json::from_str(&backend.set("vehicles.activeVehicle", &json!({ "value": format!("@vehicles.vehicles.{index}") }).to_string())).unwrap_or(Value::Null);
    match written.get("ok").and_then(Value::as_bool) {
        Some(true) => json!({ "ok": true, "activating": wanted }),
        _ => json!({ "ok": false, "reason": written.get("reason").and_then(Value::as_str).unwrap_or("the vehicle could not be made active").to_string() }),
    }
}

fn item_count(backend: &dyn Backend) -> Option<i64> {
    serde_json::from_str::<Value>(&backend.get("plan.missionController.visualItems.count")).ok().and_then(|v| v.get("value").and_then(Value::as_i64))
}

fn inserted_index(backend: &dyn Backend) -> Option<i64> {
    serde_json::from_str::<Value>(&backend.get("plan.missionController.currentPlanViewVIIndex")).ok().and_then(|v| v.get("value").and_then(Value::as_i64)).filter(|index| *index > 0)
}

fn shape(backend: &dyn Backend, kind: &crate::missionkinds::Kind, index: i64, latitude: f64, longitude: f64) -> Result<(), String> {
    if kind.id == "takeoff" {
        let at = json!({ "latitude": latitude, "longitude": longitude, "altitude": 0.0 });
        let written: Value = serde_json::from_str(&backend.set(&format!("plan.missionController.visualItems.{index}.launchCoordinate"), &json!({ "value": at }).to_string())).unwrap_or(Value::Null);
        if written.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err("the takeoff would not take a launch position, and a takeoff without one cannot be flown".to_string());
        }
        let home = object(&backend.get_fields("plan.missionController.visualItems.0", "coordinate"));
        return match home.get("coordinate").and_then(|at| at.get("valid")).and_then(Value::as_bool) {
            Some(true) => Ok(()),
            _ => Err("the launch position was accepted and the plan still has no launch point, so nothing the takeoff is measured from exists".to_string()),
        };
    }
    let Some((geometry, property)) = kind.geometry else { return Ok(()) };
    let points = match geometry {
        "line" => default_line(latitude, longitude),
        _ => default_area(latitude, longitude),
    };
    let path = format!("plan.missionController.visualItems.{index}.{property}");
    backend.invoke(&format!("{path}.clear"), "[]");
    let refused = points.iter().find_map(|(lat, lon)| {
        let at = json!([{ "latitude": lat, "longitude": lon, "altitude": 0.0 }]);
        let answered: Value = serde_json::from_str(&backend.invoke(&format!("{path}.appendVertex"), &at.to_string())).unwrap_or(Value::Null);
        match answered.get("ok").and_then(Value::as_bool) {
            Some(true) => None,
            _ => Some(format!("the {} would not take a {}", kind.title.to_lowercase(), kind.shape_noun())),
        }
    });
    match refused {
        Some(reason) => Err(reason),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Plan {
        mission: Value,
        calls: Mutex<Vec<(String, String)>>,
        answer: Value,
        count: Mutex<i64>,
    }

    impl Plan {
        fn new(mission: Value) -> Plan {
            Plan { mission, calls: Mutex::new(Vec::new()), answer: json!({ "ok": true }), count: Mutex::new(3) }
        }
    }

    impl Backend for Plan {
        fn get(&self, path: &str) -> String {
            match path {
                "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": *self.count.lock().unwrap() }).to_string(),
                "plan.missionController.currentPlanViewVIIndex" => json!({ "kind": "value", "value": *self.count.lock().unwrap() - 1 }).to_string(),
                path if path.ends_with(".sequenceNumber") => json!({ "kind": "value", "value": 2 }).to_string(),
                _ => String::new(),
            }
        }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "plan.missionController" => self.mission.to_string(),
                path if path.starts_with("plan.missionController.visualItems.") => {
                    let placed = self.calls.lock().unwrap().iter().any(|(called, _)| called.ends_with(".launchCoordinate"));
                    json!({ "kind": "object", "coordinate": { "kind": "coordinate", "valid": placed, "latitude": 47.0, "longitude": 8.0 } }).to_string()
                }
                _ => String::new(),
            }
        }
        fn set(&self, path: &str, value: &str) -> String {
            self.calls.lock().unwrap().push((path.to_string(), value.to_string()));
            self.answer.to_string()
        }
        fn invoke(&self, path: &str, args: &str) -> String {
            self.calls.lock().unwrap().push((path.to_string(), args.to_string()));
            if path.contains("insert") && self.answer.get("ok").and_then(Value::as_bool) == Some(true) {
                *self.count.lock().unwrap() += 1;
            }
            self.answer.to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    fn empty_ground_mission() -> Value {
        json!({ "kind": "object", "onlyInsertTakeoffValid": true, "isInsertTakeoffValid": true, "isInsertLandValid": false, "flyThroughCommandsAllowed": true })
    }

    fn flying_mission() -> Value {
        json!({ "kind": "object", "onlyInsertTakeoffValid": false, "isInsertTakeoffValid": false, "isInsertLandValid": true, "flyThroughCommandsAllowed": true })
    }


    #[test]
    fn a_claimed_path_keeps_the_shape_its_qt_answer_had() {
        // Two different obligations, and conflating them made my first version of this fail on four
        // correct actions. A path the core INVENTS - mission.insert, guided.orbit, camera.takePhoto,
        // vehicles.setActive - has no Qt predecessor, so there is no answer it can take away; it owes
        // ok, and a reason whenever ok is false. A path that SHADOWS a real Qt one owes everything Qt
        // gave as well, because a head cannot know the core has claimed it. QGCBridgeCore puts a
        // return value under "result" and seven macOS readers take it, three as `as? [String] ?? []`
        // - and a list reader losing it draws an empty picker that reads as a quiet vehicle rather
        // than a broken read.
        struct Nothing;
        impl Backend for Nothing {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "null" }).to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { json!({ "ok": false }).to_string() }
            fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": false }).to_string() }
            fn watch(&self, _p: &[String]) {}
        }
        let answer = |path: &str| match owns_write(path) {
            true => write(&Nothing, path, &json!({ "value": 1.0 }).to_string()),
            false => run(&Nothing, path, "[]"),
        };

        let silent: Vec<&&str> = OWNED.iter().chain(OWNED_WRITES.iter())
            .filter(|path| {
                let given = answer(path);
                given.get("ok").and_then(Value::as_bool) != Some(false) || given.get("reason").and_then(Value::as_str).is_none_or(str::is_empty)
            })
            .collect();
        assert!(silent.is_empty(), "these refuse without saying why, which is the silence the whole table exists to remove: {silent:?}");

        let shapeless: Vec<&&str> = OWNED_WRITES.iter().filter(|path| answer(path).get("result").is_none()).collect();
        assert!(shapeless.is_empty(), "these shadow a Qt path and dropped its result key, so a head reading one the Qt way sees a failure or an empty list: {shapeless:?}");

        assert!(!OWNED.iter().any(|path| path.contains("cameraManager")), "an invented name is exempt from the result rule only because no Qt path answers to it; a full Qt path in this list would be claiming one and owes the shape");
    }

    #[test]
    fn a_camera_action_answers_when_the_camera_would_have_refused_in_silence() {
        use std::cell::RefCell;
        struct Cam { camera: Value, fired: RefCell<Vec<String>> }
        impl Backend for Cam {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            // objectJson at QGCBridgeCore.cc:443 serves a property only when the fields set is
            // empty or contains its name, so a field the core forgets to ask for is ABSENT rather
            // than merely unread. A fake that ignores the list cannot fail on the omission, and
            // dropping hasZoom from the request went green here while refusing every real write.
            fn get_fields(&self, path: &str, fields: &str) -> String {
                if !path.ends_with("currentCameraInstance") {
                    return json!({ "kind": "null" }).to_string();
                }
                let asked: Vec<&str> = fields.split(',').filter(|f| !f.is_empty()).collect();
                match asked.is_empty() {
                    true => self.camera.to_string(),
                    false => Value::Object(self.camera.as_object().unwrap().iter()
                        .filter(|(key, _)| *key == "kind" || asked.contains(&key.as_str()))
                        .map(|(key, value)| (key.clone(), value.clone())).collect()).to_string(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, path: &str, _a: &str) -> String {
                self.fired.borrow_mut().push(path.to_string());
                json!({ "result": true }).to_string()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let cam = |extra: Value| {
            let mut base = json!({ "kind": "object", "modelName": "ZR30", "capturesPhotos": true, "capturesVideo": true, "hasModes": true });
            extra.as_object().unwrap().iter().for_each(|(k, v)| { base[k] = v.clone(); });
            Cam { camera: base, fired: RefCell::new(vec![]) }
        };

        let busy = cam(json!({ "cameraMode": 0, "photoCaptureStatus": 1 }));
        let answer = run(&busy, PHOTO, "[]");
        assert_eq!(answer["ok"], false);
        assert!(answer["reason"].as_str().unwrap().contains("still taking"), "takePhoto returns false here with only a qCWarning, so the whole point of owning the action is that the head gets a sentence instead of nothing: {answer}");
        assert!(busy.fired.borrow().is_empty(), "and the call the camera would have dropped is not made at all");

        let ready = cam(json!({ "cameraMode": 0 }));
        let answer = run(&ready, PHOTO, "[]");
        assert_eq!(answer["ok"], true);
        assert_eq!(*ready.fired.borrow(), vec![format!("{CAMERA}.takePhoto")]);

        let wrong_mode = cam(json!({ "cameraMode": 1 }));
        let answer = run(&wrong_mode, PHOTO, "[]");
        assert_eq!(answer["ok"], false);
        assert!(answer["reason"].as_str().unwrap().contains("mode"));
        let in_video = cam(json!({ "cameraMode": 1, "photosInVideoMode": true }));
        assert_eq!(run(&in_video, PHOTO, "[]")["ok"], true, "a camera that shoots stills in video mode is not refused, which is the term view.camera was missing");

        let recording = cam(json!({ "cameraMode": 1, "videoCaptureStatus": 1 }));
        assert_eq!(run(&recording, RECORD, "[]")["ok"], true, "the toggle is what stops a running recording");
        assert_eq!(*recording.fired.borrow(), vec![format!("{CAMERA}.toggleVideoRecording")]);
        let held = cam(json!({ "cameraMode": 1, "videoCaptureStatus": 1 }));
        let answer = run(&held, MODE, "[\"photo\"]");
        assert_eq!(answer["ok"], false);
        assert!(answer["reason"].as_str().unwrap().contains("recording"), "{answer}");
        assert!(held.fired.borrow().is_empty());

        let switchable = cam(json!({ "cameraMode": 1 }));
        assert_eq!(run(&switchable, MODE, "[\"photo\"]")["ok"], true);
        assert_eq!(*switchable.fired.borrow(), vec![format!("{CAMERA}.setCameraModePhoto")]);
        assert_eq!(*cam(json!({ "cameraMode": 0 })).fired.borrow(), Vec::<String>::new());

        let fixed = cam(json!({ "cameraMode": 1, "hasModes": false }));
        let answer = run(&fixed, MODE, "[\"video\"]");
        assert!(answer["reason"].as_str().unwrap().contains("one mode"), "{answer}");

        let nonsense = cam(json!({ "cameraMode": 0 }));
        assert_eq!(run(&nonsense, MODE, "[\"panorama\"]")["unknown"], "panorama");
        assert!(run(&nonsense, MODE, "[]")["reason"].as_str().unwrap().contains("photo or video"));
        assert_eq!(run(&None_, MODE, "[\"panorama\"]")["unknown"], "panorama", "a mode the core cannot name is a malformed call whether or not a camera is attached; answering it with the rig's state hides the caller's bug");

        let lapsing = cam(json!({ "cameraMode": 0, "photoCaptureMode": 1, "photoLapse": 5.0, "photoLapseCount": 0 }));
        let answer = run(&lapsing, PHOTO, "[]");
        assert_eq!(answer["ok"], true);
        assert_eq!(answer["started"], "timelapse", "takePhoto sends photoLapse and photoLapseCount when the mode is timelapse, so this press began a run of shots rather than taking one");
        assert_eq!(answer["lapseUnlimited"], true, "count zero is MAV_CMD_IMAGE_START_CAPTURE's unlimited, so nothing stops this until someone stops it");
        assert_eq!(answer["lapseSeconds"], 5.0);
        assert_eq!(*lapsing.fired.borrow(), vec![format!("{CAMERA}.takePhoto")], "and it is the same invokable, which is exactly why the answer has to say which capture it started");

        let single = cam(json!({ "cameraMode": 0 }));
        assert_eq!(run(&single, PHOTO, "[]")["started"], "single");
        assert_eq!(run(&cam(json!({ "cameraMode": 0 })), PHOTO, "[]")["lapseCount"], Value::Null, "a single shot has no interval to report, and a count beside it would read as one");

        let mid_interval = cam(json!({ "cameraMode": 0, "photoCaptureStatus": 3 }));
        let answer = run(&mid_interval, STOP_PHOTO, "[]");
        assert_eq!(answer["ok"], true, "stopTakePhoto is the only way to end an unlimited timelapse and nothing in QGC's QML calls it");
        assert_eq!(*mid_interval.fired.borrow(), vec![format!("{CAMERA}.stopTakePhoto")]);

        let idle = cam(json!({ "cameraMode": 0 }));
        let answer = run(&idle, STOP_PHOTO, "[]");
        assert_eq!(answer["ok"], false, "stopTakePhoto refuses unless the status is one of the two interval states");
        assert!(answer["reason"].as_str().unwrap().contains("interval"));
        assert!(idle.fired.borrow().is_empty());

        struct Refusing;
        impl Backend for Refusing {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path.ends_with("currentCameraInstance") {
                    true => json!({ "kind": "object", "modelName": "ZR30", "capturesPhotos": true, "hasModes": true, "cameraMode": 0 }).to_string(),
                    false => json!({ "kind": "null" }).to_string(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true, "result": false }).to_string() }
            fn watch(&self, _p: &[String]) {}
        }
        let answer = run(&Refusing, PHOTO, "[]");
        assert_eq!(answer["ok"], false, "takePhoto has refusals the core cannot see - _resetting is not a readable property - so reporting ok because the gate passed puts the silence back one layer down");
        assert_eq!(answer["result"], false);
        assert!(answer["reason"].as_str().unwrap().contains("did not carry out"));

        struct None_;
        impl Backend for None_ {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "null" }).to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        assert!(run(&None_, PHOTO, "[]")["reason"].as_str().unwrap().contains("No camera"), "no camera is not the same refusal as a camera that will not");
        assert!(run(&None_, MODE, "[\"photo\"]")["reason"].as_str().unwrap().contains("No camera"), "a well-formed call with no camera still gets the rig's answer");

        assert!(owns(PHOTO) && owns(RECORD) && owns(MODE) && owns(STOP_PHOTO), "the router only reaches these if owns says so, which is how qgc_core_guided ended up defined with no caller");
    }
    #[test]
    fn a_zoom_write_says_what_the_camera_actually_took() {
        use std::cell::RefCell;
        struct Cam { camera: Value, wrote: RefCell<Vec<String>> }
        impl Backend for Cam {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            // objectJson at QGCBridgeCore.cc:443 serves a property only when the fields set is
            // empty or contains its name, so a field the core forgets to ask for is ABSENT rather
            // than merely unread. A fake that ignores the list cannot fail on the omission, and
            // dropping hasZoom from the request went green here while refusing every real write.
            fn get_fields(&self, path: &str, fields: &str) -> String {
                if !path.ends_with("currentCameraInstance") {
                    return json!({ "kind": "null" }).to_string();
                }
                let asked: Vec<&str> = fields.split(',').filter(|f| !f.is_empty()).collect();
                match asked.is_empty() {
                    true => self.camera.to_string(),
                    false => Value::Object(self.camera.as_object().unwrap().iter()
                        .filter(|(key, _)| *key == "kind" || asked.contains(&key.as_str()))
                        .map(|(key, value)| (key.clone(), value.clone())).collect()).to_string(),
                }
            }
            fn set(&self, _p: &str, value: &str) -> String {
                self.wrote.borrow_mut().push(value.to_string());
                json!({ "ok": true }).to_string()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let cam = |extra: Value| {
            let mut base = json!({ "kind": "object", "modelName": "ZR30", "hasZoom": true });
            extra.as_object().unwrap().iter().for_each(|(k, v)| { base[k] = v.clone(); });
            Cam { camera: base, wrote: RefCell::new(vec![]) }
        };

        let plain = cam(json!({}));
        let answer = write(&plain, ZOOM, &json!({ "value": 40.0 }).to_string());
        assert_eq!(answer["ok"], true);
        assert_eq!(answer["result"], true, "zoomLevel shadows a real Qt path, so its success answer owes the result key a Qt write would have carried");
        assert_eq!(answer["clamped"], false);
        assert_eq!(*plain.wrote.borrow(), vec![json!({ "value": 40.0 }).to_string()]);

        let high = cam(json!({}));
        let answer = write(&high, ZOOM, &json!({ "value": 150.0 }).to_string());
        assert_eq!(answer["ok"], true);
        assert_eq!(answer["result"], true);
        assert_eq!(answer["level"], 100.0);
        assert_eq!(answer["asked"], 150.0);
        assert_eq!(answer["clamped"], true, "setZoomLevel clamps in silence, so a head asking for 150 is told it succeeded and never learns the camera went to 100");
        assert_eq!(*high.wrote.borrow(), vec![json!({ "value": 100.0 }).to_string()], "and the clamped value is what travels, not the one that was asked for");

        let fixed = cam(json!({ "hasZoom": false }));
        let answer = write(&fixed, ZOOM, &json!({ "value": 40.0 }).to_string());
        assert_eq!(answer["ok"], false);
        assert!(answer["reason"].as_str().unwrap().contains("no zoom"));
        assert!(fixed.wrote.borrow().is_empty(), "setZoomLevel drops this write entirely, so passing it through only spends a MAVLink message to achieve nothing");

        let absent = cam(json!({ "modelName": "" }));
        assert!(write(&absent, ZOOM, &json!({ "value": 40.0 }).to_string())["reason"].as_str().unwrap().contains("No camera"), "no camera and no zoom are different refusals");

        let nonsense = cam(json!({}));
        assert_eq!(write(&nonsense, ZOOM, "{\"value\": \"lots\"}")["ok"], false);
        assert!(nonsense.wrote.borrow().is_empty());
        assert_eq!(write(&nonsense, ZOOM, "{}")["ok"], false, "a payload with no value at all is not a zoom of zero");

        assert!(owns_write(ZOOM), "router.set consults this and nothing else; unclaimed, the write goes straight past every check above");
        assert!(!owns_write("vehicle.armed"), "arming is deliberately not claimed: a gate that can wrongly refuse an arm needs an airframe to prove it wrong");
    }

    #[test]
    fn an_item_the_plan_has_decided_against_is_refused_before_it_is_inserted() {
        let plan = Plan::new(empty_ground_mission());
        let refused = run(&plan, "mission.insert", "[\"waypoint\", 47.0, 8.0, -1]");
        assert_eq!(refused["ok"], false);
        assert_eq!(refused["refused"], "waypoint");
        assert!(refused["reason"].as_str().unwrap().contains("takeoff"));
        let calls = plan.calls.lock().unwrap();
        assert!(calls.iter().all(|(path, _)| path.ends_with("setCurrentPlanViewSeqNum")), "the refusal has to happen before the plan is touched, or a head racing a stale view still gets its item in");
    }

    #[test]
    fn an_item_the_plan_allows_is_inserted_by_the_name_the_core_holds() {
        let plan = Plan::new(flying_mission());
        let inserted = run(&plan, "mission.insert", "[\"survey\", 47.5, 8.5, 3]");
        assert_eq!(inserted["ok"], true);
        assert_eq!(inserted["inserted"], "survey");
        let calls = plan.calls.lock().unwrap();
        assert_eq!(calls[0].0, "plan.missionController.setCurrentPlanViewSeqNum", "the insert point is selected before the question is asked, because the answer is about that point");
        assert_eq!(calls[1].0, "plan.missionController.insertComplexMissionItem");
        let sent: Value = serde_json::from_str(&calls[1].1).unwrap();
        assert_eq!(sent[0], "Survey", "the head names the kind and the core supplies the complex name, so a head never spells it");
        assert_eq!(sent[1]["latitude"], 47.5);
        assert_eq!(sent[2], 3);
        assert_eq!(sent[3], true, "the inserted item becomes the selected one, which is what makes the controller recompute what can be inserted next");
    }

    #[test]
    fn a_simple_item_is_called_without_a_complex_name() {
        let plan = Plan::new(flying_mission());
        assert_eq!(run(&plan, "mission.insert", "[\"waypoint\", 47.0, 8.0, -1]")["ok"], true);
        let calls = plan.calls.lock().unwrap();
        assert_eq!(calls[1].0, "plan.missionController.insertSimpleMissionItem");
        let sent: Value = serde_json::from_str(&calls[1].1).unwrap();
        assert_eq!(sent.as_array().unwrap().len(), 3, "a simple item takes a coordinate, an index, and the flag that selects it");
        assert_eq!(sent[0]["longitude"], 8.0);
        assert_eq!(sent[2], true);
    }

    #[test]
    fn in_a_state_that_wants_a_takeoff_first_a_takeoff_is_the_one_thing_accepted() {
        let plan = Plan::new(empty_ground_mission());
        assert_eq!(run(&plan, "mission.insert", "[\"takeoff\", 47.0, 8.0, -1]")["ok"], true);
        let refused = Plan::new(empty_ground_mission());
        assert_eq!(run(&refused, "mission.insert", "[\"land\", 47.0, 8.0, -1]")["ok"], false);
        assert_eq!(run(&refused, "mission.insert", "[\"survey\", 47.0, 8.0, -1]")["ok"], false);
        assert!(refused.calls.lock().unwrap().iter().all(|(path, _)| !path.contains("insert")));
    }

    #[test]
    fn a_kind_the_core_never_heard_of_says_so_rather_than_saying_no() {
        let plan = Plan::new(flying_mission());
        let unknown = run(&plan, "mission.insert", "[\"Fixed Wing Landing Pattern\", 47.0, 8.0, -1]");
        assert_eq!(unknown["ok"], false);
        assert_eq!(unknown["unknown"], "Fixed Wing Landing Pattern", "a head holding an item type this catalogue never listed has to be able to tell that apart from the plan turning it down, because the first means insert it yourself and the second means do not");
        assert!(unknown["reason"].as_str().unwrap().contains("catalogue"));
        assert!(plan.calls.lock().unwrap().is_empty(), "a kind the core does not know is refused before the plan view is even moved");

        let refused = run(&Plan::new(empty_ground_mission()), "mission.insert", "[\"survey\", 47.0, 8.0, -1]");
        assert_eq!(refused["ok"], false);
        assert_eq!(refused["unknown"], Value::Null, "a kind the core does know and is refusing carries no unknown marker");
        assert_eq!(refused["refused"], "survey");
    }

    #[test]
    fn arguments_that_are_not_a_place_are_refused_rather_than_placed_at_zero() {
        let plan = Plan::new(flying_mission());
        ["[]", "[\"waypoint\"]", "[\"waypoint\", null, 8.0, -1]", "not json", "{}"].iter().for_each(|args| {
            assert_eq!(run(&plan, "mission.insert", args)["ok"], false, "{args} is not a place to put a mission item");
        });
        assert!(plan.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn a_plan_that_refuses_the_call_is_reported_rather_than_reported_as_inserted() {
        let mut plan = Plan::new(flying_mission());
        plan.answer = json!({ "ok": false, "reason": "the plan is syncing with the vehicle" });
        let refused = run(&plan, "mission.insert", "[\"waypoint\", 47.0, 8.0, -1]");
        assert_eq!(refused["ok"], false);
        assert_eq!(refused["reason"], "the plan is syncing with the vehicle");
    }

    #[test]
    fn an_insert_that_did_not_grow_the_plan_never_reaches_the_rollback() {
        struct Deaf(Plan);
        impl Backend for Deaf {
            fn get(&self, path: &str) -> String { self.0.get(path) }
            fn get_fields(&self, path: &str, fields: &str) -> String { self.0.get_fields(path, fields) }
            fn set(&self, path: &str, value: &str) -> String { self.0.set(path, value) }
            fn invoke(&self, path: &str, args: &str) -> String {
                self.0.calls.lock().unwrap().push((path.to_string(), args.to_string()));
                json!({ "ok": true }).to_string()
            }
            fn watch(&self, paths: &[String]) { self.0.watch(paths) }
        }
        let deaf = Deaf(Plan::new(flying_mission()));
        let refused = run(&deaf, "mission.insert", "[\"survey\", 47.0, 8.0, -1]");
        assert_eq!(refused["ok"], false);
        assert!(refused["reason"].as_str().unwrap().contains("did not grow"));
        let calls = deaf.0.calls.lock().unwrap();
        assert!(calls.iter().all(|(called, _)| !called.ends_with("removeVisualItem")), "rolling back here would delete an item the operator already had");
        assert!(calls.iter().all(|(called, _)| !called.ends_with("appendVertex")), "and shaping here would draw over one");
    }

    #[test]
    fn a_place_the_plan_does_not_have_is_refused_before_anything_is_touched() {
        let plan = Plan::new(flying_mission());
        ["[\"waypoint\", 47.0, 8.0, 0]", "[\"waypoint\", 47.0, 8.0, 99]", "[\"waypoint\", 47.0, 8.0, 4]"]
            .iter()
            .for_each(|args| assert_eq!(run(&plan, "mission.insert", args)["ok"], false, "{args}"));
        let calls = plan.calls.lock().unwrap();
        assert!(calls.iter().all(|(called, _)| !called.contains("insert")), "index zero would put an item before the plan's own settings entry, and an index past the end the model inserts anyway with only a warning");
    }

    #[test]
    fn the_question_is_asked_about_the_slot_the_item_will_follow() {
        let plan = Plan::new(flying_mission());
        assert_eq!(run(&plan, "mission.insert", "[\"waypoint\", 47.0, 8.0, 2]")["ok"], true);
        let selected: Vec<i64> = plan
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(path, _)| path.ends_with("setCurrentPlanViewSeqNum"))
            .map(|(_, args)| serde_json::from_str::<Value>(args).unwrap()[0].as_i64().unwrap())
            .collect();
        assert_eq!(selected.len(), 1, "the plan view selects an item and inserts after it, so an insert at place two is a question about place one");
    }

    #[test]
    fn an_orbit_height_outside_what_the_operator_can_choose_is_refused() {
        let flying = orbiting::Flying::launched(480.0);
        ["[47.5, 8.5, 150.0, true, -500.0]", "[47.5, 8.5, 150.0, true, 0.0]", "[47.5, 8.5, 150.0, true, 5000.0]"]
            .iter()
            .for_each(|args| {
                let refused = run(&flying, "guided.orbit", args);
                assert_eq!(refused["ok"], false, "{args} is outside the range the slider offers");
                assert!(refused["reason"].as_str().unwrap().contains("above the launch point"));
            });
        assert!(flying.calls.lock().unwrap().is_empty(), "a height of minus five hundred metres is five hundred metres into the ground");
        assert_eq!(run(&flying, "guided.orbit", "[47.5, 8.5, 150.0, true, 60.0]")["ok"], true);
    }

    #[test]
    fn a_vehicle_that_cannot_orbit_is_not_asked_to() {
        let mut unable = orbiting::Flying::launched(480.0);
        unable.supported = false;
        let refused = run(&unable, "guided.orbit", "[47.5, 8.5, 150.0, true, 60.0]");
        assert_eq!(refused["ok"], false);
        assert!(refused["reason"].as_str().unwrap().contains("does not support"));
        assert!(unable.calls.lock().unwrap().is_empty(), "guidedModeOrbit returns void and shows a dialog, so asking anyway would report success while nothing was sent");
    }

    #[test]
    fn the_core_only_claims_the_action_it_performs() {
        assert!(owns("mission.insert"));
        assert!(!owns("plan.missionController.insertSimpleMissionItem"));
        assert!(!owns("mission.insertion"));
        assert_eq!(run(&Plan::new(flying_mission()), "mission.remove", "[]")["ok"], false);
    }

    #[test]
    fn a_survey_arrives_with_an_area_around_where_it_was_placed() {
        let plan = Plan::new(flying_mission());
        assert_eq!(run(&plan, "mission.insert", "[\"survey\", 47.0, 8.0, -1]")["ok"], true);
        let calls = plan.calls.lock().unwrap();
        let path = "plan.missionController.visualItems.3.surveyAreaPolygon";
        assert_eq!(calls.iter().filter(|(called, _)| called == &format!("{path}.clear")).count(), 1);
        let vertices: Vec<&(String, String)> = calls.iter().filter(|(called, _)| called == &format!("{path}.appendVertex")).collect();
        assert_eq!(vertices.len(), 4, "a survey with no area draws nothing and uploads nothing, so the action gives it one");
        let first: Value = serde_json::from_str(&vertices[0].1).unwrap();
        assert!((first[0]["latitude"].as_f64().unwrap() - 47.0).abs() < 0.01, "the area is placed around where the operator tapped");
    }

    #[test]
    fn a_corridor_arrives_with_a_path_rather_than_an_area() {
        let plan = Plan::new(flying_mission());
        assert_eq!(run(&plan, "mission.insert", "[\"corridor\", 47.0, 8.0, -1]")["ok"], true);
        let calls = plan.calls.lock().unwrap();
        let vertices = calls.iter().filter(|(called, _)| called.ends_with("corridorPolyline.appendVertex")).count();
        assert_eq!(vertices, 2, "a corridor is a line to scan along, so two points rather than four");
    }

    #[test]
    fn a_takeoff_arrives_knowing_where_the_vehicle_launches_from() {
        let plan = Plan::new(empty_ground_mission());
        assert_eq!(run(&plan, "mission.insert", "[\"takeoff\", 47.25, 8.75, -1]")["ok"], true);
        let calls = plan.calls.lock().unwrap();
        let written = calls.iter().find(|(called, _)| called.ends_with(".launchCoordinate")).expect("the launch position was never written");
        let value: Value = serde_json::from_str(&written.1).unwrap();
        assert_eq!(value["value"]["latitude"], 47.25);
        assert_eq!(value["value"]["longitude"], 8.75);
    }

    #[test]
    fn an_insert_the_plan_turned_down_leaves_nothing_to_roll_back() {
        let mut plan = Plan::new(flying_mission());
        plan.answer = json!({ "ok": false, "reason": "no" });
        let refused = run(&plan, "mission.insert", "[\"survey\", 47.0, 8.0, -1]");
        assert_eq!(refused["ok"], false);
        let calls = plan.calls.lock().unwrap();
        assert!(calls.iter().all(|(called, _)| !called.ends_with("removeVisualItem")), "the insert itself failed, so there is nothing to remove");
    }

    #[test]
    fn a_survey_that_will_not_take_an_area_is_removed_rather_than_left_empty() {
        struct Fussy(Plan);
        impl Backend for Fussy {
            fn get(&self, path: &str) -> String { self.0.get(path) }
            fn get_fields(&self, path: &str, fields: &str) -> String { self.0.get_fields(path, fields) }
            fn set(&self, path: &str, value: &str) -> String { self.0.set(path, value) }
            fn invoke(&self, path: &str, args: &str) -> String {
                match path.ends_with("appendVertex") {
                    true => {
                        self.0.calls.lock().unwrap().push((path.to_string(), args.to_string()));
                        json!({ "ok": false, "reason": "no" }).to_string()
                    }
                    false => self.0.invoke(path, args),
                }
            }
            fn watch(&self, paths: &[String]) { self.0.watch(paths) }
        }
        let fussy = Fussy(Plan::new(flying_mission()));
        let refused = run(&fussy, "mission.insert", "[\"survey\", 47.0, 8.0, -1]");
        assert_eq!(refused["ok"], false);
        assert_eq!(refused["removed"], "survey");
        let calls = fussy.0.calls.lock().unwrap();
        assert!(calls.iter().any(|(called, _)| called.ends_with("removeVisualItem")), "a survey with no area is worse than no survey, because an operator has to find it to delete it");
    }
}

#[cfg(test)]
mod removal {
    use super::*;
    use std::sync::Mutex;

    struct Shrinking {
        count: Mutex<i64>,
        calls: Mutex<Vec<String>>,
    }

    impl Shrinking {
        fn holding(count: i64) -> Shrinking {
            Shrinking { count: Mutex::new(count), calls: Mutex::new(Vec::new()) }
        }
    }

    impl Backend for Shrinking {
        fn get(&self, path: &str) -> String {
            match path {
                "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": *self.count.lock().unwrap() }).to_string(),
                _ => String::new(),
            }
        }
        fn get_fields(&self, _p: &str, _f: &str) -> String { String::new() }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, path: &str, args: &str) -> String {
            self.calls.lock().unwrap().push(format!("{path} {args}"));
            if path.ends_with("removeVisualItem") {
                *self.count.lock().unwrap() -= 1;
            }
            json!({ "ok": true }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn an_item_is_removed_and_the_plan_says_what_is_left() {
        let plan = Shrinking::holding(4);
        let gone = run(&plan, "mission.remove", "[2]");
        assert_eq!(gone["ok"], true);
        assert_eq!(gone["removed"], 2);
        assert_eq!(gone["remaining"], 3);
        assert_eq!(plan.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn the_plans_own_settings_entry_is_not_an_item_an_operator_can_delete() {
        let plan = Shrinking::holding(4);
        let refused = run(&plan, "mission.remove", "[0]");
        assert_eq!(refused["ok"], false);
        assert!(refused["reason"].as_str().unwrap().contains("settings"));
        assert!(plan.calls.lock().unwrap().is_empty(), "the plan view offers no way to remove it, and removing it leaves a plan the controller cannot describe");
        assert_eq!(*plan.count.lock().unwrap(), 4);
    }

    #[test]
    fn an_index_the_plan_does_not_hold_is_refused_rather_than_passed_on() {
        let plan = Shrinking::holding(3);
        ["[3]", "[99]", "[-1]", "[]", "not json"].iter().for_each(|args| {
            assert_eq!(run(&plan, "mission.remove", args)["ok"], false, "{args} names no item");
        });
        assert!(plan.calls.lock().unwrap().is_empty());
        assert_eq!(*plan.count.lock().unwrap(), 3);
    }

    #[test]
    fn a_removal_that_did_not_shrink_the_plan_is_reported_as_a_failure() {
        struct Stubborn;
        impl Backend for Stubborn {
            fn get(&self, path: &str) -> String {
                match path {
                    "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": 3 }).to_string(),
                    _ => String::new(),
                }
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String { String::new() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true }).to_string() }
            fn watch(&self, _p: &[String]) {}
        }
        let refused = run(&Stubborn, "mission.remove", "[1]");
        assert_eq!(refused["ok"], false, "the call answered ok and the item is still there, and a head told ok would redraw a list that has not changed");
        assert!(refused["reason"].as_str().unwrap().contains("still holds"));
    }
}

#[cfg(test)]
pub(super) mod orbiting {
    use super::*;
    use std::sync::Mutex;

    pub(in crate::actions) struct Flying {
        home: Value,
        pub(in crate::actions) calls: Mutex<Vec<String>>,
        answer: Value,
        pub(in crate::actions) supported: bool,
    }

    impl Flying {
        pub(in crate::actions) fn launched(altitude: f64) -> Flying {
            Flying {
                home: json!({ "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0, "altitude": altitude }),
                calls: Mutex::new(Vec::new()),
                answer: json!({ "ok": true }),
                supported: true,
            }
        }
    }

    impl Backend for Flying {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.homePosition" => self.home.to_string(),
                "settings.flyViewSettings.guidedMinimumAltitude.rawValue" => json!({ "kind": "value", "value": 2.0 }).to_string(),
                "settings.flyViewSettings.guidedMaximumAltitude.rawValue" => json!({ "kind": "value", "value": 121.92 }).to_string(),
                _ => String::new(),
            }
        }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "vehicle" => json!({ "kind": "object", "orbitModeSupported": self.supported }).to_string(),
                _ => String::new(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, path: &str, args: &str) -> String {
            self.calls.lock().unwrap().push(format!("{path} {args}"));
            self.answer.to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    fn sent(plan: &Flying) -> Value {
        let calls = plan.calls.lock().unwrap();
        let (_, args) = calls[0].split_once(' ').unwrap();
        serde_json::from_str(args).unwrap()
    }

    #[test]
    fn the_direction_of_the_turn_is_the_sign_of_the_radius() {
        let clockwise = Flying::launched(480.0);
        assert_eq!(run(&clockwise, "guided.orbit", "[47.5, 8.5, 150.0, true, 60.0]")["ok"], true);
        assert_eq!(sent(&clockwise)[1], 150.0);

        let widdershins = Flying::launched(480.0);
        assert_eq!(run(&widdershins, "guided.orbit", "[47.5, 8.5, 150.0, false, 60.0]")["ok"], true);
        assert_eq!(sent(&widdershins)[1], -150.0, "a negative radius is how this command says anticlockwise, which is not something a head should have to know");
    }

    #[test]
    fn the_height_is_measured_from_the_launch_point_and_sent_above_sea_level() {
        let flying = Flying::launched(480.0);
        let started = run(&flying, "guided.orbit", "[47.5, 8.5, 150.0, true, 60.0]");
        assert_eq!(started["altitudeAmsl"], 540.0);
        assert_eq!(sent(&flying)[2], 540.0, "the operator chooses a height above where it took off, and the command wants sea level");
        assert_eq!(sent(&flying)[0]["latitude"], 47.5);
    }

    #[test]
    fn an_orbit_is_refused_when_there_is_nothing_to_measure_it_against() {
        let unlaunched = Flying {
            home: json!({ "kind": "coordinate", "valid": false, "latitude": 0.0, "longitude": 0.0, "altitude": 0.0 }),
            calls: Mutex::new(Vec::new()),
            answer: json!({ "ok": true }),
            supported: true,
        };
        let refused = run(&unlaunched, "guided.orbit", "[47.5, 8.5, 150.0, true, 60.0]");
        assert_eq!(refused["ok"], false);
        assert!(refused["reason"].as_str().unwrap().contains("launched from"));
        assert!(unlaunched.calls.lock().unwrap().is_empty(), "sending an orbit measured against an unknown launch height is sending an altitude nobody chose");
    }

    #[test]
    fn an_orbit_with_no_radius_or_no_direction_is_refused_rather_than_guessed() {
        let flying = Flying::launched(480.0);
        ["[47.5, 8.5, 0.0, true, 60.0]", "[47.5, 8.5, -20.0, true, 60.0]", "[47.5, 8.5, 150.0, null, 60.0]", "[47.5, 8.5, 150.0, true]", "[]"]
            .iter()
            .for_each(|args| assert_eq!(run(&flying, "guided.orbit", args)["ok"], false, "{args}"));
        assert!(flying.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn a_vehicle_that_refuses_the_orbit_is_reported_rather_than_reported_as_started() {
        let mut flying = Flying::launched(480.0);
        flying.answer = json!({ "ok": false, "reason": "not in guided mode" });
        let refused = run(&flying, "guided.orbit", "[47.5, 8.5, 150.0, true, 60.0]");
        assert_eq!(refused["ok"], false);
        assert_eq!(refused["reason"], "not in guided mode");
    }



    #[test]
    fn an_untracked_plan_is_not_a_plan_with_nothing_to_undo() {
        use std::cell::RefCell;
        struct Plan {
            fields: Value,
            called: RefCell<Vec<String>>,
        }
        impl Backend for Plan {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, _p: &str, _f: &str) -> String { self.fields.to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, p: &str, _a: &str) -> String {
                self.called.borrow_mut().push(p.to_string());
                json!({ "ok": true }).to_string()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let plan = |fields: Value| Plan { fields, called: RefCell::new(vec![]) };

        let untracked = plan(json!({ "kind": "object", "undoTracking": false, "canUndo": false, "canRedo": false }));
        let refused = run(&untracked, UNDO, "[]");
        assert_eq!(refused["ok"], false);
        assert_eq!(refused["refusal"], "notTracking", "every head clears undoTracking when its plan screen goes away, so canUndo is false after twenty edits and a navigation as surely as after none - answering nothingTo there is true about the stack and wrong about the plan");
        assert!(untracked.called.borrow().is_empty(), "a refusal that still fired the invoke would be a refusal in the answer only");

        let empty = plan(json!({ "kind": "object", "undoTracking": true, "canUndo": false, "canRedo": true }));
        assert_eq!(run(&empty, UNDO, "[]")["refusal"], "nothingTo");
        assert_eq!(run(&empty, REDO, "[]")["ok"], true, "the two stacks are separate and redo is gated on its own");

        let ready = plan(json!({ "kind": "object", "undoTracking": true, "canUndo": true, "canRedo": false }));
        let done = run(&ready, UNDO, "[]");
        assert_eq!(done["ok"], true);
        assert_eq!(done["refusal"], Value::Null);
        assert_eq!(done.get("result"), None, "undo() returns void and the bridge answers a void invoke with ok and no result key, so serving result would mean inventing one");
        assert_eq!(ready.called.borrow().as_slice(), ["plan.undo"]);

        assert!(owns(UNDO) && owns(REDO));
    }
}
