use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "settings.gimbalControllerSettings.toolbarIndicatorShowAzimuth",
    "settings.gimbalControllerSettings.toolbarIndicatorShowAcquireReleaseControl",
    "vehicle.homePosition",
];

pub const GIMBAL_CENTER: &str = "gimbal.center";
pub const GIMBAL_TILT_90: &str = "gimbal.tilt90";
pub const GIMBAL_POINT_HOME: &str = "gimbal.pointHome";
pub const GIMBAL_RETRACT: &str = "gimbal.retract";
pub const GIMBAL_YAW_LOCK: &str = "gimbal.yawLock";
pub const GIMBAL_CONTROL: &str = "gimbal.control";
pub const GIMBAL_SELECT: &str = "gimbal.select";

fn setting(backend: &dyn Backend, name: &str) -> bool {
    let fact = object(&backend.get(&format!("settings.gimbalControllerSettings.{name}")));
    fact.get("value").is_some_and(|v| v.as_bool().unwrap_or_else(|| v.as_f64().is_some_and(|n| n != 0.0)))
}

fn angle(value: &Value) -> Option<String> {
    value.as_f64().map(|v| format!("{v:.1}"))
}

pub fn status_text(gimbal: &Value) -> &'static str {
    match (flag(gimbal, "retracted"), flag(gimbal, "yawLock")) {
        (true, _) => "Retracted",
        (false, true) => "Yaw locked",
        (false, false) => "Yaw follow",
    }
}

pub fn indicator(snapshot: &Value, show_azimuth: bool, show_control: bool) -> Value {
    let gimbals = snapshot["gimbals"].as_array().cloned().unwrap_or_default();
    let Some(active) = gimbals.iter().find(|g| flag(g, "active")).or(gimbals.first()).cloned() else {
        return json!({ "kind": "object", "class": "GimbalIndicator", "shown": false });
    };
    let yaw = match show_azimuth {
        true => angle(&active["absoluteYaw"]).map(|v| format!("Az: {v}")),
        false => angle(&active["bodyYaw"]).map(|v| format!("Y: {v}")),
    };
    json!({
        "kind": "object",
        "class": "GimbalIndicator",
        "shown": true,
        "multi": gimbals.len() > 1,
        "gimbals": gimbals.iter().enumerate().map(|(i, g)| json!({ "name": format!("Gimbal {}", i + 1), "managerCompid": g["managerCompid"], "deviceId": g["deviceId"], "active": flag(g, "active") })).collect::<Vec<_>>(),
        "statusText": status_text(&active),
        "pitchText": angle(&active["pitch"]).map(|v| format!("P: {v}")),
        "yawText": yaw,
        "yawLockLabel": if flag(&active, "yawLock") { "Yaw Follow" } else { "Yaw Lock" },
        "yawLocked": flag(&active, "yawLock"),
        "retractOffered": active["supportsRetract"] == true,
        "controlOffered": show_control,
        "controlLabel": if active["haveControl"] == true { "Release Control" } else { "Acquire Control" },
        "haveControl": active["haveControl"] == true,
    })
}

pub fn indicator_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let snapshot = crate::gimbal::lock().snapshot(crate::hub::now_ms());
    indicator(&snapshot, setting(backend, "toolbarIndicatorShowAzimuth"), setting(backend, "toolbarIndicatorShowAcquireReleaseControl"))
}

pub fn owns(path: &str) -> bool {
    [GIMBAL_CENTER, GIMBAL_TILT_90, GIMBAL_POINT_HOME, GIMBAL_RETRACT, GIMBAL_YAW_LOCK, GIMBAL_CONTROL, GIMBAL_SELECT].contains(&path)
}

fn qt_path(method: &str) -> String {
    format!("vehicle.gimbalController.{method}")
}

pub fn run(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let vehicle = crate::guided::active_id(backend);
    if vehicle.is_none() {
        return json!({ "ok": false, "refusal": "noVehicle", "reason": "No vehicle is connected." });
    }
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let on = given.get(0).and_then(Value::as_bool).unwrap_or(false);
    let op = |name: &str, extra: Value| {
        let mut action = json!({ "action": "gimbal", "op": name });
        if let Value::Object(fields) = extra {
            fields.into_iter().for_each(|(k, v)| action[k] = v);
        }
        action
    };
    match path {
        GIMBAL_CENTER => crate::guided::dispatch(backend, Some(op("center", Value::Null)), vehicle, &qt_path("centerGimbal"), "[]"),
        GIMBAL_TILT_90 => crate::guided::dispatch(backend, Some(op("tilt90", Value::Null)), vehicle, &qt_path("sendPitchBodyYaw"), "[-90, 0]"),
        GIMBAL_RETRACT => crate::guided::dispatch(backend, Some(op("retract", Value::Null)), vehicle, &qt_path("setGimbalRetract"), "[true]"),
        GIMBAL_YAW_LOCK => crate::guided::dispatch(backend, Some(op("yawLock", json!({ "lock": on }))), vehicle, &qt_path("setGimbalYawLock"), &json!([on]).to_string()),
        GIMBAL_CONTROL => match on {
            true => crate::guided::dispatch(backend, Some(op("acquire", Value::Null)), vehicle, &qt_path("acquireGimbalControl"), "[]"),
            false => crate::guided::dispatch(backend, Some(op("release", Value::Null)), vehicle, &qt_path("releaseGimbalControl"), "[]"),
        },
        GIMBAL_SELECT => crate::guided::dispatch(
            backend,
            Some(op("select", json!({ "managerCompid": given.get(0), "deviceId": given.get(1) }))),
            vehicle,
            &qt_path("setActiveGimbal"),
            args,
        ),
        _ => {
            let home = object(&backend.get("vehicle.homePosition"));
            match (flag(&home, "valid"), home.get("latitude").and_then(Value::as_f64), home.get("longitude").and_then(Value::as_f64)) {
                (true, Some(latitude), Some(longitude)) => crate::mapclick::send(
                    backend,
                    crate::mapclick::Click::Roi,
                    "vehicle.guidedModeROI",
                    &json!([{ "latitude": latitude, "longitude": longitude, "altitude": home.get("altitude").cloned().unwrap_or(json!(0.0)) }]).to_string(),
                ),
                _ => json!({ "ok": false, "refusal": "noHome", "reason": "The vehicle has not reported a home position to point at." }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(gimbal: Value) -> Value {
        json!({ "gimbals": [gimbal] })
    }

    #[test]
    fn the_indicator_reads_as_the_toolbar_does() {
        let shown = indicator(&snapshot(json!({ "active": true, "pitch": -12.34, "bodyYaw": 5.0, "absoluteYaw": 185.0, "yawLock": false, "retracted": false, "supportsRetract": true, "haveControl": false })), false, true);
        assert_eq!(shown["statusText"], "Yaw follow");
        assert_eq!(shown["pitchText"], "P: -12.3");
        assert_eq!(shown["yawText"], "Y: 5.0");
        assert_eq!(shown["yawLockLabel"], "Yaw Lock");
        assert_eq!(shown["controlLabel"], "Acquire Control");
        let azimuth = indicator(&snapshot(json!({ "active": true, "absoluteYaw": 185.0, "yawLock": true })), true, false);
        assert_eq!(azimuth["yawText"], "Az: 185.0", "the toolbar shows azimuth when the setting asks");
        assert_eq!(azimuth["statusText"], "Yaw locked");
        assert_eq!(azimuth["retractOffered"], false);
        assert_eq!(indicator(&snapshot(json!({ "retracted": true, "yawLock": true })), false, false)["statusText"], "Retracted");
        assert_eq!(indicator(&json!({ "gimbals": [] }), false, false)["shown"], false, "no gimbal, no indicator");
    }
}
