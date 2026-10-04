use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "settings.gimbalControllerSettings.toolbarIndicatorShowAzimuth",
    "settings.gimbalControllerSettings.toolbarIndicatorShowAcquireReleaseControl",
    "settings.gimbalControllerSettings.enableOnScreenControl",
    "settings.gimbalControllerSettings.clickAndDrag",
    "vehicle.homePosition",
    crate::gimbal::GIMBAL_CHANGED,
];

pub const GIMBAL_CENTER: &str = "gimbal.center";
pub const GIMBAL_TILT_90: &str = "gimbal.tilt90";
pub const GIMBAL_POINT_HOME: &str = "gimbal.pointHome";
pub const GIMBAL_RETRACT: &str = "gimbal.retract";
pub const GIMBAL_YAW_LOCK: &str = "gimbal.yawLock";
pub const GIMBAL_CONTROL: &str = "gimbal.control";
pub const GIMBAL_SELECT: &str = "gimbal.select";
pub const GIMBAL_ON_SCREEN: &str = "gimbal.onScreen";
pub const GIMBAL_PITCH: &str = "gimbal.pitch";

fn setting(backend: &dyn Backend, name: &str) -> bool {
    let fact = object(&backend.get(&format!("settings.gimbalControllerSettings.{name}")));
    fact.get("value").is_some_and(|v| v.as_bool().unwrap_or_else(|| v.as_f64().is_some_and(|n| n != 0.0)))
}

fn number_setting(backend: &dyn Backend, name: &str) -> Option<f64> {
    crate::read::value_number(&backend.get(&format!("settings.gimbalControllerSettings.{name}.rawValue")))
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

pub fn indicator(snapshot: &Value, show_azimuth: bool, show_control: bool, on_screen: (bool, bool)) -> Value {
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
        "gimbals": gimbals.iter().map(|g| json!({ "name": format!("Gimbal {}-{}", g["managerCompid"].as_i64().unwrap_or(0), g["deviceId"].as_i64().unwrap_or(0)), "managerCompid": g["managerCompid"], "deviceId": g["deviceId"], "active": flag(g, "active") })).collect::<Vec<_>>(),
        "statusText": status_text(&active),
        "pitchText": angle(&active["pitch"]).map(|v| format!("P: {v}")),
        "pitchDegrees": active["pitch"].as_f64(),
        "yawText": yaw,
        "yawLockLabel": if flag(&active, "yawLock") { "Yaw Follow" } else { "Yaw Lock" },
        "yawLocked": flag(&active, "yawLock"),
        "yawLockOffered": active["supportsYawLock"] == true,
        "retractOffered": active["supportsRetract"] == true,
        "controlOffered": show_control,
        "controlLabel": if active["haveControl"] == true { "Release Control" } else { "Acquire Control" },
        "haveControl": active["haveControl"] == true,
        "onScreen": { "enabled": on_screen.0, "clickAndDrag": on_screen.1 },
    })
}

pub fn indicator_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let snapshot = crate::gimbal::gimbal_view(backend, &[]);
    let mut shown = indicator(
        &snapshot,
        setting(backend, "toolbarIndicatorShowAzimuth"),
        setting(backend, "toolbarIndicatorShowAcquireReleaseControl"),
        (setting(backend, "enableOnScreenControl"), setting(backend, "clickAndDrag")),
    );
    shown["askSerial"] = json!(crate::gimbal::asked());
    shown
}

pub const AZIMUTH_DEPS: &[&str] = &["settings.gimbalControllerSettings.showAzimuthIndicatorOnMap"];

pub fn azimuths(snapshot: &Value, shown: bool) -> Vec<Value> {
    snapshot["gimbals"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|_| shown)
        .filter_map(|gimbal| gimbal["absoluteYaw"].as_f64().filter(|yaw| yaw.is_finite()).map(|yaw| json!({ "yaw": yaw.rem_euclid(360.0), "active": flag(gimbal, "active") })))
        .collect()
}

pub fn azimuth_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let snapshot = crate::gimbal::gimbal_view(backend, &[]);
    json!({ "kind": "object", "class": "GimbalAzimuth", "gimbals": azimuths(&snapshot, setting(backend, "showAzimuthIndicatorOnMap")) })
}

pub fn owns(path: &str) -> bool {
    [GIMBAL_CENTER, GIMBAL_TILT_90, GIMBAL_POINT_HOME, GIMBAL_RETRACT, GIMBAL_YAW_LOCK, GIMBAL_CONTROL, GIMBAL_SELECT, GIMBAL_ON_SCREEN, GIMBAL_PITCH].contains(&path)
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
        GIMBAL_PITCH => match given.get(0).and_then(Value::as_f64).filter(|v| v.is_finite()) {
            Some(pitch) => {
                let qt_body_yaw = crate::read::value_number(&backend.get(&qt_path("activeGimbal.bodyYaw"))).filter(|v| v.is_finite());
                let qt_args = qt_body_yaw.map_or_else(|| "[]".to_string(), |yaw| json!([pitch, yaw, false]).to_string());
                crate::guided::dispatch(backend, Some(op("pitch", json!({ "pitch": pitch }))), vehicle, &qt_path("sendPitchBodyYaw"), &qt_args)
            }
            None => json!({ "ok": false, "refusal": "badPitch", "reason": "A tilt is a number of degrees." }),
        },
        GIMBAL_RETRACT => crate::guided::dispatch(backend, Some(op("retract", Value::Null)), vehicle, &qt_path("setGimbalRetract"), "[true]"),
        GIMBAL_YAW_LOCK => crate::guided::dispatch(backend, Some(op("yawLock", json!({ "lock": on }))), vehicle, &qt_path("setGimbalYawLock"), &json!([on]).to_string()),
        GIMBAL_CONTROL => match on {
            true => crate::guided::dispatch(backend, Some(op("acquire", Value::Null)), vehicle, &qt_path("acquireGimbalControl"), "[]"),
            false => crate::guided::dispatch(backend, Some(op("release", Value::Null)), vehicle, &qt_path("releaseGimbalControl"), "[]"),
        },
        GIMBAL_ON_SCREEN => {
            let number = |i: usize| given.get(i).and_then(Value::as_f64).filter(|v| v.is_finite() && (-1.0..=1.0).contains(v));
            let (Some(pan), Some(tilt)) = (number(0), number(1)) else {
                return json!({ "ok": false, "refusal": "badPoint", "reason": "Pan and tilt are fractions of the screen from -1 to 1." });
            };
            let point = given.get(2).and_then(Value::as_bool).unwrap_or(true);
            let screen = json!({
                "pan": pan,
                "tilt": tilt,
                "point": point,
                "hFov": number_setting(backend, "cameraHFov"),
                "vFov": number_setting(backend, "cameraVFov"),
                "slide": number_setting(backend, "cameraSlideSpeed"),
            });
            crate::guided::dispatch(backend, Some(op("onScreen", screen)), vehicle, &qt_path("gimbalOnScreenControl"), &json!([pan, tilt, point, !point, !point]).to_string())
        }
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
                    crate::mapclick::Click::HomeRoi,
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

    #[test]
    fn the_map_wedge_follows_each_gimbal_absolute_yaw_only_when_the_setting_asks() {
        let snapshot = json!({ "gimbals": [{ "active": true, "absoluteYaw": -90.0 }, { "active": false, "absoluteYaw": null }, { "active": false, "absoluteYaw": 10.0 }] });
        assert_eq!(azimuths(&snapshot, true), vec![json!({ "yaw": 270.0, "active": true }), json!({ "yaw": 10.0, "active": false })]);
        assert!(azimuths(&snapshot, false).is_empty());
    }

    fn snapshot(gimbal: Value) -> Value {
        json!({ "gimbals": [gimbal] })
    }

    #[test]
    fn the_indicator_reads_as_the_toolbar_does() {
        let shown = indicator(&snapshot(json!({ "active": true, "pitch": -12.34, "bodyYaw": 5.0, "absoluteYaw": 185.0, "yawLock": false, "retracted": false, "supportsRetract": true, "haveControl": false })), false, true, (true, false));
        assert_eq!(shown["statusText"], "Yaw follow");
        assert_eq!(shown["pitchText"], "P: -12.3");
        assert_eq!(shown["yawText"], "Y: 5.0");
        assert_eq!(shown["yawLockLabel"], "Yaw Lock");
        assert_eq!(shown["controlLabel"], "Acquire Control");
        assert_eq!(shown["onScreen"]["enabled"], true);
        let azimuth = indicator(&snapshot(json!({ "active": true, "absoluteYaw": 185.0, "yawLock": true })), true, false, (false, false));
        assert_eq!(azimuth["yawText"], "Az: 185.0", "the toolbar shows azimuth when the setting asks");
        assert_eq!(azimuth["statusText"], "Yaw locked");
        assert_eq!(azimuth["retractOffered"], false);
        assert_eq!((shown["yawLockOffered"].clone(), indicator(&snapshot(json!({ "active": true, "supportsYawLock": true })), false, false, (false, false))["yawLockOffered"].clone()), (json!(false), json!(true)), "GimbalIndicator.qml shows Yaw Lock only when the gimbal reports the capability");
        assert_eq!(indicator(&snapshot(json!({ "retracted": true, "yawLock": true })), false, false, (false, false))["statusText"], "Retracted");
        assert_eq!(indicator(&json!({ "gimbals": [] }), false, false, (false, false))["shown"], false, "no gimbal, no indicator");
    }
}
