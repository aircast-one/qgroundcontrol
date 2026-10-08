use serde_json::{Value, json};

use crate::read::{Unit, flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicle.parameterManager.parametersReady",
    "vehicle.rover",
    "settings.unitsSettings.horizontalDistanceUnits",
    "settings.unitsSettings.verticalDistanceUnits",
];
pub const FOLLOW_ENABLE: &str = "apmFollow.enable";
pub const FOLLOW_RESET: &str = "apmFollow.reset";
pub const FOLLOW_POSITION: &str = "apmFollow.position";
pub const FOLLOW_POINT: &str = "apmFollow.point";
pub const FOLLOW_OFFSETS: &str = "apmFollow.offsets";
pub const FOLLOW_HEIGHT: &str = "apmFollow.height";

const OFFSET_TYPE_RELATIVE: f64 = 1.0;
const ALTITUDE_TYPE_RELATIVE: f64 = 1.0;
const YAW_NONE: f64 = 0.0;
const YAW_FACE: f64 = 1.0;
const YAW_FLIGHT: f64 = 3.0;
const DEFAULT_ANGLE: f64 = 45.0;
const DEFAULT_DISTANCE: f64 = 5.0;
const POINT_VALUES: [f64; 3] = [YAW_NONE, YAW_FACE, YAW_FLIGHT];
const POINT_OPTIONS: [&str; 3] = ["Maintain current vehicle orientation", "Point at ground station location", "Same direction as ground station movement"];
const POSITION_OPTIONS: [&str; 2] = ["Maintain Current Offsets", "Specify Offsets"];
const ROVER_MISSING: [&str; 6] = ["FOLL_DIST_MAX", "FOLL_SYSID", "FOLL_OFS_X", "FOLL_OFS_Y", "FOLL_OFS_Z", "FOLL_OFS_TYPE"];
const COPTER_MISSING: [&str; 2] = ["FOLL_ALT_TYPE", "FOLL_YAW_BEHAVE"];

fn heading_to_radians(heading: f64) -> f64 {
    (-(heading - 90.0)).to_radians()
}

fn radians_to_heading(radians: f64) -> f64 {
    let heading = 90.0 - radians.to_degrees();
    match heading {
        h if h < 0.0 => h + 360.0,
        h if h > 360.0 => h - 360.0,
        h => h,
    }
}

pub fn offsets(heading: f64, distance: f64) -> (f64, f64) {
    let radians = heading_to_radians(heading);
    let snap = |v: f64| if v.abs() < 0.0001 { 0.0 } else { v };
    match (distance == 0.0, radians == 0.0) {
        (true, _) => (0.0, 0.0),
        (false, true) => (0.0, distance),
        (false, false) => (snap(radians.sin() * distance), snap(radians.cos() * distance)),
    }
}

pub fn angle_and_distance(x: f64, y: f64) -> (f64, f64) {
    let radians = x.atan2(y);
    let distance = if radians == 0.0 { y } else { x / radians.sin() };
    (radians_to_heading(radians), distance)
}

pub fn supported(sysid: f64, gcs: f64, offset_type: f64, altitude_type: f64, yaw: f64, rover: bool) -> bool {
    let copter_ok = rover || (altitude_type == ALTITUDE_TYPE_RELATIVE && [YAW_NONE, YAW_FACE, YAW_FLIGHT].contains(&yaw));
    sysid == gcs && offset_type == OFFSET_TYPE_RELATIVE && copter_ok
}

fn path(name: &str) -> String {
    format!("vehicle.parameterManager.getParameter(-1,{name})")
}

fn raw(backend: &dyn Backend, name: &str) -> Option<f64> {
    let fact = backend.value(&path(name));
    let present = fact.get("kind").and_then(Value::as_str) == Some("fact") && fact.get("name").and_then(Value::as_str).is_some_and(|n| !n.is_empty());
    present.then(|| fact.get("rawValue").or(fact.get("value")).and_then(Value::as_f64)).flatten()
}

fn write(backend: &dyn Backend, name: &str, value: f64) -> bool {
    flag(&object(&backend.set(&format!("{}.rawValue", path(name)), &json!({ "value": value }).to_string())), "ok")
}

fn rover(backend: &dyn Backend) -> bool {
    flag(&backend.value_fields("vehicle", "rover"), "rover")
}

pub fn follow_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let Some(enable) = raw(backend, "FOLL_ENABLE") else { return json!({ "kind": "object", "class": "ApmFollow", "available": false }) };
    let rover = rover(backend);
    let params_available = raw(backend, "FOLL_SYSID").is_some();
    let value = |name: &str| raw(backend, name).unwrap_or(0.0);
    let ok = params_available && supported(value("FOLL_SYSID"), f64::from(crate::mavout::gcs_system()), value("FOLL_OFS_TYPE"), value("FOLL_ALT_TYPE"), value("FOLL_YAW_BEHAVE"), rover);
    let (x, y, z) = (value("FOLL_OFS_X"), value("FOLL_OFS_Y"), value("FOLL_OFS_Z"));
    let maintain = x == 0.0 && y == 0.0 && z == 0.0;
    let (angle, distance) = if maintain { (0.0, 0.0) } else { angle_and_distance(x, y) };
    let enabled = enable == 1.0;
    let (horizontal, vertical) = (Unit::horizontal(backend), Unit::vertical(backend));
    json!({
        "kind": "object",
        "class": "ApmFollow",
        "available": true,
        "enabled": enabled,
        "waiting": enabled && !params_available,
        "supported": !params_available || ok,
        "unsupportedText": "The vehicle parameters required for follow me are currently set in a way which is not supported. Using follow with this setup may lead to unpredictable/hazardous results.",
        "showSettings": enabled && params_available && ok,
        "rover": rover,
        "positionOptions": POSITION_OPTIONS,
        "positionIndex": if maintain { 0 } else { 1 },
        "pointOptions": if rover { Vec::new() } else { POINT_OPTIONS.to_vec() },
        "pointIndex": POINT_VALUES.iter().position(|v| *v == value("FOLL_YAW_BEHAVE")).map_or(-1, |i| i as i64),
        "angle": angle,
        "distance": distance,
        "height": -z,
        "horizontalUnit": horizontal.name,
        "horizontalMetresPerUnit": horizontal.meters(1.0),
        "verticalUnit": vertical.name,
        "verticalMetresPerUnit": vertical.meters(1.0),
    })
}

fn args(text: &str) -> Vec<Value> {
    serde_json::from_str::<Vec<Value>>(text).unwrap_or_default()
}

fn answer(ok: bool) -> Value {
    match ok {
        true => json!({ "ok": true }),
        false => json!({ "ok": false, "reason": "The vehicle did not take the follow me parameters." }),
    }
}

fn write_offsets(backend: &dyn Backend, heading: f64, distance: f64) -> bool {
    let (x, y) = offsets(heading, distance);
    write(backend, "FOLL_OFS_X", x) && write(backend, "FOLL_OFS_Y", y)
}

fn reset(backend: &dyn Backend) -> bool {
    let base = write(backend, "FOLL_SYSID", f64::from(crate::mavout::gcs_system())) && write(backend, "FOLL_OFS_TYPE", OFFSET_TYPE_RELATIVE);
    let copter = rover(backend) || (write(backend, "FOLL_ALT_TYPE", ALTITUDE_TYPE_RELATIVE) && write(backend, "FOLL_YAW_BEHAVE", YAW_FACE));
    base && copter && write_offsets(backend, DEFAULT_ANGLE, DEFAULT_DISTANCE)
}

fn enable(backend: &dyn Backend, on: bool) -> Value {
    if !write(backend, "FOLL_ENABLE", if on { 1.0 } else { 0.0 }) {
        return answer(false);
    }
    if !on {
        return answer(true);
    }
    let names: Vec<&str> = ROVER_MISSING.iter().chain(if rover(backend) { [].iter() } else { COPTER_MISSING.iter() }).copied().collect();
    crate::guided::dispatch(backend, Some(json!({ "action": "refreshParameters", "names": names })), crate::guided::active_id(backend), "vehicle.parameterManager.refreshAllParameters", "[]")
}

pub fn run(backend: &dyn Backend, action: &str, text: &str) -> Value {
    let given = args(text);
    let number = |i: usize| given.get(i).and_then(Value::as_f64);
    match action {
        FOLLOW_ENABLE => enable(backend, given.first().and_then(Value::as_bool).unwrap_or(false)),
        FOLLOW_RESET => answer(reset(backend)),
        FOLLOW_POSITION => match number(0) {
            Some(0.0) => answer(["FOLL_OFS_X", "FOLL_OFS_Y", "FOLL_OFS_Z"].iter().all(|n| write(backend, n, 0.0))),
            Some(_) => answer(reset(backend)),
            None => json!({ "ok": false, "reason": "apmFollow.position takes the option index." }),
        },
        FOLLOW_POINT => match number(0).and_then(|i| POINT_VALUES.get(i as usize)) {
            Some(yaw) => answer(write(backend, "FOLL_YAW_BEHAVE", *yaw)),
            None => json!({ "ok": false, "reason": "apmFollow.point takes one of the three options." }),
        },
        FOLLOW_OFFSETS => match (number(0), number(1)) {
            (Some(angle), Some(distance)) if (0.0..=360.0).contains(&angle) && distance >= 0.0 => answer(write_offsets(backend, angle, distance)),
            _ => json!({ "ok": false, "reason": "apmFollow.offsets takes an angle of 0 to 360 degrees and a distance of at least 0 m." }),
        },
        FOLLOW_HEIGHT => match number(0).filter(|h| *h >= 0.0) {
            Some(height) => answer(write(backend, "FOLL_OFS_Z", -height)),
            None => json!({ "ok": false, "reason": "apmFollow.height takes a height of at least 0 m." }),
        },
        _ => json!({ "ok": false, "reason": format!("{action} is not a follow me action") }),
    }
}

pub fn owns(path: &str) -> bool {
    [FOLLOW_ENABLE, FOLLOW_RESET, FOLLOW_POSITION, FOLLOW_POINT, FOLLOW_OFFSETS, FOLLOW_HEIGHT].contains(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angle_and_distance_round_trip_through_the_xy_offsets() {
        assert_eq!(offsets(0.0, 0.0), (0.0, 0.0));
        assert_eq!(offsets(90.0, 5.0), (0.0, 5.0), "a 90 degree heading is geometric zero, straight along y");
        let (x, y) = offsets(45.0, 5.0);
        assert!((x - 3.5355).abs() < 0.001 && (y - 3.5355).abs() < 0.001);
        let (angle, distance) = angle_and_distance(x, y);
        assert!((angle - 45.0).abs() < 1e-9 && (distance - 5.0).abs() < 1e-9);
        assert_eq!(offsets(0.0, 5.0).1, 0.0, "a near-zero component snaps to zero");
        assert_eq!(angle_and_distance(0.0, 7.0), (90.0, 7.0));
    }

    #[test]
    fn only_a_relative_follow_of_this_ground_station_is_supported() {
        assert!(supported(255.0, 255.0, 1.0, 1.0, 1.0, false));
        assert!(!supported(254.0, 255.0, 1.0, 1.0, 1.0, false), "following another system");
        assert!(!supported(255.0, 255.0, 0.0, 1.0, 1.0, false), "NED offsets");
        assert!(!supported(255.0, 255.0, 1.0, 0.0, 1.0, false), "absolute altitude");
        assert!(!supported(255.0, 255.0, 1.0, 1.0, 2.0, false), "same heading as the target");
        assert!(supported(255.0, 255.0, 1.0, 0.0, 2.0, true), "a rover has no altitude or yaw requirement");
    }
}
