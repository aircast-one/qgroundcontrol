use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::attitude::{point, raw};
use crate::read::flag;
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.coordinate",
    "vehicle.homePosition",
    "vehicle.altitudeRelative",
    "vehicle.heading",
    "vehicle.pitch",
    "vehicle.roll",
    "vehicle.customMode",
    "settings.flightMapSettings.mapProvider",
    crate::gimbal::GIMBAL_CHANGED,
    SYNTHETIC_CHANGED,
];

pub const OVERLAY_DEPS: &[&str] = &["vehicle.homePosition", FENCE_TOP_ARDUPILOT, FENCE_TOP_PX4];
const FENCE_TOP_ARDUPILOT: &str = "vehicle.parameterManager.getParameter(-1,FENCE_ALT_MAX)";
const FENCE_TOP_PX4: &str = "vehicle.parameterManager.getParameter(-1,GF_MAX_VER_DIST)";
const DEFAULT_FENCE_TOP_M: f64 = 50.0;

pub const SYNTHETIC_CHANGED: &str = "core.synthetic@changed";
pub const SYNTHETIC_AIM: &str = "syntheticView.aim";

pub const FIXED_CAMERA_TILT_DEG: f64 = -15.0;
pub const CAMERA_FOV_DEG: f64 = 70.0;
const FALLBACK_IMAGERY: &str = "Bing Satellite";
const TILT_RANGE_DEG: (f64, f64) = (-90.0, 0.0);
const BEAM_REACH_M: f64 = 1500.0;
const FRAME_ASPECT: f64 = 9.0 / 16.0;
const BELOW_HORIZON_DEG: f64 = -0.5;
const LOWEST_HEIGHT_M: f64 = 1.0;
const NEEDS_DEGREES: &str = "Give the tilt, from 0 (level) to -90 (straight down), and the pan from the nose, in degrees.";

pub const DEFAULT_AIM: (f64, f64) = (FIXED_CAMERA_TILT_DEG, 0.0);
const FASTEST_TURN_DEG_S: f64 = 720.0;

static AIM: Mutex<(f64, f64)> = Mutex::new(DEFAULT_AIM);
static CHANGED: AtomicBool = AtomicBool::new(false);
static DRAW: Mutex<&'static str> = Mutex::new(DRAW_LOADING);
const DRAW_LOADING: &str = "loading";

pub fn take_changed() -> bool {
    CHANGED.swap(false, Ordering::SeqCst)
}

pub fn set_draw(state: &'static str) {
    let mut draw = DRAW.lock().unwrap_or_else(PoisonError::into_inner);
    if *draw != state {
        *draw = state;
        CHANGED.store(true, Ordering::SeqCst);
    }
}

fn drawn() -> &'static str {
    *DRAW.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn owns(path: &str) -> bool {
    path == SYNTHETIC_AIM
}

fn aimed() -> (f64, f64) {
    *AIM.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn wrapped(degrees: f64) -> f64 {
    let turned = (degrees + 180.0).rem_euclid(360.0) - 180.0;
    if turned == -180.0 { 180.0 } else { turned }
}

pub fn run(_path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let number = |index: usize| given.get(index).and_then(Value::as_f64).filter(|degrees| degrees.is_finite());
    match number(0).zip(number(1)) {
        None => json!({ "ok": false, "reason": NEEDS_DEGREES }),
        Some((tilt, pan)) => {
            let aim = (tilt.clamp(TILT_RANGE_DEG.0, TILT_RANGE_DEG.1), wrapped(pan));
            *AIM.lock().unwrap_or_else(PoisonError::into_inner) = aim;
            CHANGED.store(true, Ordering::SeqCst);
            json!({ "ok": true, "tilt": aim.0, "pan": aim.1 })
        }
    }
}

pub fn imagery(provider: &str) -> &'static str {
    crate::maptypes::QGC_ORDER
        .iter()
        .copied()
        .find(|name| name.strip_prefix(provider).is_some_and(|kind| kind.starts_with(' ') && kind.contains("Sat")))
        .unwrap_or(FALLBACK_IMAGERY)
}

fn active_gimbal(gimbals: &Value) -> Option<Value> {
    let all = gimbals["gimbals"].as_array().cloned().unwrap_or_default();
    all.iter().find(|gimbal| flag(gimbal, "active")).or(all.first()).cloned()
}

pub fn gimbal_aim(gimbals: &Value) -> Option<(f64, f64)> {
    active_gimbal(gimbals)
        .and_then(|gimbal| Some((gimbal["absoluteYaw"].as_f64()?, gimbal["pitch"].as_f64()?)))
        .filter(|(yaw, pitch)| yaw.is_finite() && pitch.is_finite())
}

fn yaw_from_nose(gimbals: &Value) -> Option<f64> {
    active_gimbal(gimbals)
        .filter(|gimbal| gimbal["yawLock"] == false)
        .and_then(|gimbal| gimbal["bodyYaw"].as_f64())
        .filter(|yaw| yaw.is_finite())
}

pub fn euler_rates(roll: f64, pitch: f64, [p, q, r]: [f64; 3]) -> (f64, f64, f64) {
    let (sin_roll, cos_roll) = roll.to_radians().sin_cos();
    let pitch = pitch.to_radians();
    let turning = q * sin_roll + r * cos_roll;
    let bounded = |rate: f64| rate.clamp(-FASTEST_TURN_DEG_S, FASTEST_TURN_DEG_S);
    (bounded(p + turning * pitch.tan()), bounded(q * cos_roll - r * sin_roll), bounded(turning / pitch.cos()))
}

pub fn body_rates(roll: f64, pitch: f64, (roll_rate, pitch_rate, heading_rate): (f64, f64, f64)) -> [f64; 3] {
    let (sin_roll, cos_roll) = roll.to_radians().sin_cos();
    let (sin_pitch, cos_pitch) = pitch.to_radians().sin_cos();
    [roll_rate - heading_rate * sin_pitch, pitch_rate * cos_roll + heading_rate * sin_roll * cos_pitch, heading_rate * cos_roll * cos_pitch - pitch_rate * sin_roll]
}

fn ground_reach(height: f64, elevation_deg: f64) -> f64 {
    match elevation_deg < BELOW_HORIZON_DEG {
        true => (height / (-elevation_deg).to_radians().tan()).min(BEAM_REACH_M),
        false => BEAM_REACH_M,
    }
}

pub fn beam(at: (f64, f64), height: f64, heading: f64, pitch: f64, fov: f64) -> Vec<(f64, f64)> {
    let half_width = fov / 2.0;
    let half_height = (half_width.to_radians().tan() * FRAME_ASPECT).atan().to_degrees();
    let height = height.max(LOWEST_HEIGHT_M);
    let (near, far) = (ground_reach(height, pitch - half_height), ground_reach(height, pitch + half_height));
    let ground = |bearing: f64, metres: f64| crate::surveygrid::at_distance_and_azimuth(at, metres, bearing.rem_euclid(360.0));
    vec![at, ground(heading - half_width, near), ground(heading - half_width, far), ground(heading + half_width, far), ground(heading + half_width, near), at]
}

pub fn synthetic(backend: &dyn Backend, gimbals: &Value, aim: (f64, f64)) -> Value {
    let placed = point(backend, "vehicle.coordinate").zip(point(backend, "vehicle.homePosition")).zip(raw(backend, "vehicle.altitudeRelative"));
    let Some((((latitude, longitude), (home_latitude, home_longitude)), above_home)) = placed else {
        return json!({ "kind": "object", "class": "SyntheticView", "available": false, "native": native(), "draw": drawn() });
    };
    let gimbal = gimbal_aim(gimbals);
    let (tilt, pan) = aim;
    let reading = |path: &str| raw(backend, path).unwrap_or(0.0);
    let nose = |offset: f64| (reading("vehicle.heading") + offset).rem_euclid(360.0);
    let follows = gimbal.and(yaw_from_nose(gimbals));
    let (heading, aimed_pitch) = match (gimbal, follows) {
        (Some((_, pitch)), Some(body_yaw)) => (nose(body_yaw), pitch),
        (Some(aim), None) => aim,
        (None, _) => (nose(pan), tilt),
    };
    let frame_locked = crate::virtualjoystick::rate_mode_now(&backend.value_fields("vehicle", crate::virtualjoystick::VEHICLE_FIELDS));
    let (pitch, roll) = match frame_locked {
        true => (reading("vehicle.pitch") + aimed_pitch, reading("vehicle.roll")),
        false => (aimed_pitch, 0.0),
    };
    let (roll_rate, pitch_rate, heading_rate) = euler_rates(reading("vehicle.roll"), reading("vehicle.pitch"), [reading("vehicle.rollRate"), reading("vehicle.pitchRate"), reading("vehicle.yawRate")]);
    let (pitch_rate, roll_rate) = match frame_locked {
        true => (Some(pitch_rate), roll_rate),
        false => (gimbal.is_none().then_some(0.0), 0.0),
    };
    let provider = crate::read::value_string(&backend.value("settings.flightMapSettings.mapProvider.rawValue"));
    json!({
        "kind": "object",
        "class": "SyntheticView",
        "available": true,
        "latitude": latitude,
        "longitude": longitude,
        "aboveHome": above_home,
        "homeLatitude": home_latitude,
        "homeLongitude": home_longitude,
        "heading": heading,
        "pitch": pitch,
        "roll": roll,
        "aimable": gimbal.is_none(),
        "pan": if gimbal.is_none() { pan } else { 0.0 },
        "tilt": if gimbal.is_none() { tilt } else { 0.0 },
        "fov": CAMERA_FOV_DEG,
        "imagery": imagery(&provider),
        "northSpeed": reading("vehicle.localPosition.vx"),
        "eastSpeed": reading("vehicle.localPosition.vy"),
        "climbSpeed": -reading("vehicle.localPosition.vz"),
        "headingRate": (gimbal.is_none() || follows.is_some()).then_some(heading_rate),
        "pitchRate": pitch_rate,
        "rollRate": roll_rate,
        "response": response(backend, &backend.value_fields("vehicle", crate::virtualjoystick::VEHICLE_FIELDS)),
        "roundTripMs": crate::linkhealth::recent_round_trip_ms(),
        "native": native(),
        "draw": drawn(),
        "beam": beam((latitude, longitude), above_home, heading, pitch, CAMERA_FOV_DEG).iter().map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon })).collect::<Vec<_>>(),
    })
}

const PILOT_YAW_MODES: [&str; 6] = ["Stabilize", "Altitude Hold", "Loiter", "Position Hold", "Sport", "Drift"];
const CLIMB_MODES: [&str; 3] = ["Altitude Hold", "Loiter", "Position Hold"];
const RC_HALF_RANGE: f64 = 500.0;
const CM_PER_M: f64 = 100.0;

fn param(backend: &dyn Backend, name: &str) -> Option<f64> {
    raw(backend, &format!("vehicle.parameterManager.getParameter(-1,{name})"))
}

fn gain(backend: &dyn Backend, full: &str, scale: f64, expo: Option<&str>, deadzone: &str) -> Value {
    let read = || Some(json!({ "full": param(backend, full)? / scale, "expo": expo.map_or(Some(0.0), |name| param(backend, name))?, "deadzone": param(backend, deadzone)? / RC_HALF_RANGE }));
    read().unwrap_or(Value::Null)
}

pub fn response(backend: &dyn Backend, vehicle: &Value) -> Value {
    let Some(mode) = crate::virtualjoystick::copter_mode_now(vehicle) else { return json!({ "rateMode": false }) };
    let acro = mode == "Acro";
    let yaw = match (acro, PILOT_YAW_MODES.contains(&mode)) {
        (true, _) => gain(backend, "ACRO_Y_RATE", 1.0, Some("ACRO_Y_EXPO"), "RC4_DZ"),
        (false, true) => gain(backend, "PILOT_Y_RATE", 1.0, Some("PILOT_Y_EXPO"), "RC4_DZ"),
        (false, false) => Value::Null,
    };
    let tilt = |deadzone: &str| if acro { gain(backend, "ACRO_RP_RATE", 1.0, Some("ACRO_RP_EXPO"), deadzone) } else { Value::Null };
    let loiter = |deadzone: &str| if mode == "Loiter" { gain(backend, "LOIT_SPEED", CM_PER_M, None, deadzone) } else { Value::Null };
    let climb = CLIMB_MODES
        .contains(&mode)
        .then(|| {
            let up = param(backend, "PILOT_SPEED_UP")? / CM_PER_M;
            let down = param(backend, "PILOT_SPEED_DN").filter(|down| *down > 0.0).map_or(up, |down| down / CM_PER_M);
            Some(json!({ "up": up, "down": down, "deadzone": param(backend, "THR_DZ")? / RC_HALF_RANGE }))
        })
        .flatten()
        .unwrap_or(Value::Null);
    json!({ "yaw": yaw, "roll": tilt("RC1_DZ"), "pitch": tilt("RC2_DZ"), "forward": loiter("RC2_DZ"), "right": loiter("RC1_DZ"), "climb": climb, "rateMode": acro })
}

fn native() -> bool {
    cfg!(any(native_synthetic, all(feature = "synthetic", target_family = "wasm")))
}

pub fn overlay_deps() -> Vec<String> {
    crate::missionitems::DEPS.iter().chain(crate::fences::DEPS).chain(OVERLAY_DEPS).map(|dep| dep.to_string()).collect()
}

fn located(at: &Value) -> Option<(f64, f64)> {
    let number = |key: &str| at.get(key).and_then(Value::as_f64).filter(|v| v.is_finite());
    number("latitude").zip(number("longitude")).filter(|spot| *spot != (0.0, 0.0) && at.get("valid").and_then(Value::as_bool).unwrap_or(true))
}

fn spot(at: (f64, f64), above_home: f64) -> Value {
    json!({ "latitude": at.0, "longitude": at.1, "aboveHome": above_home })
}

pub fn overlays(home_amsl: Option<f64>, mission: &Value, fences: &Value, fence_top: Option<f64>) -> Value {
    let items = mission["items"].as_array().cloned().unwrap_or_default();
    let home = items.iter().find(|item| item["homePosition"] == true).and_then(|item| located(&item["coordinate"]));
    let placed = |item: &Value| Some((located(&item["coordinate"])?, item["amslEntryAlt"].as_f64().filter(|a| a.is_finite())? - home_amsl?));
    let flown: Vec<(Value, (f64, f64), f64)> = items
        .iter()
        .filter(|item| item["homePosition"] != true)
        .filter_map(|item| placed(item).map(|(at, above)| (item.clone(), at, above)))
        .collect();
    let starts_home = mission["linksStartToHome"] == true;
    let route: Vec<Value> = home
        .filter(|_| starts_home && home_amsl.is_some())
        .map(|at| spot(at, 0.0))
        .into_iter()
        .chain(flown.iter().filter(|(item, _, _)| item["flownLeg"] == true).map(|(_, at, above)| spot(*at, *above)))
        .collect();
    let waypoints: Vec<Value> = flown
        .iter()
        .map(|(item, at, above)| json!({ "latitude": at.0, "longitude": at.1, "aboveHome": above, "label": item["sequence"].as_i64().map_or_else(|| item["index"].to_string(), |n| n.to_string()) }))
        .collect();
    let polygons: Vec<Value> = fences["polygons"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|polygon| json!({ "inclusion": polygon["inclusion"].as_bool().unwrap_or(true), "points": polygon["vertices"].as_array().cloned().unwrap_or_default().iter().filter_map(located).map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon })).collect::<Vec<_>>() }))
        .collect();
    let circles: Vec<Value> = fences["circles"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|circle| Some(json!({ "inclusion": circle["inclusion"].as_bool().unwrap_or(true), "latitude": located(&circle["centre"])?.0, "longitude": located(&circle["centre"])?.1, "radius": circle["radius"].as_f64().filter(|r| *r > 0.0)? })))
        .collect();
    json!({
        "kind": "object",
        "class": "SyntheticOverlays",
        "home": home.map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon })),
        "route": route,
        "waypoints": waypoints,
        "polygons": polygons,
        "circles": circles,
        "fenceTop": fence_top.unwrap_or(DEFAULT_FENCE_TOP_M),
    })
}

pub fn overlays_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let home_amsl = backend.value("vehicle.homePosition").get("altitude").and_then(Value::as_f64).filter(|a| a.is_finite());
    let fence_top = [FENCE_TOP_ARDUPILOT, FENCE_TOP_PX4].iter().find_map(|path| raw(backend, path).filter(|metres| *metres > 0.0));
    overlays(home_amsl, &crate::missionitems::fly_items_view(backend, &["geometry".to_string()]), &crate::fences::fly_fences_view(backend, &[]), fence_top)
}

pub fn synthetic_view(backend: &dyn Backend, _args: &[String]) -> Value {
    synthetic(backend, &crate::gimbal::gimbal_view(backend, &[]), aimed())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    struct Fake(BTreeMap<&'static str, Value>);

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            self.0.get(path).map_or_else(|| json!({ "kind": "null" }), |v| match v {
                Value::Object(_) => v.clone(),
                _ => json!({ "kind": "fact", "rawValue": v, "value": v }),
            }).to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn flying(extra: &[(&'static str, Value)]) -> Fake {
        let base = [
            ("vehicles.activeVehicleAvailable", json!(true)),
            ("vehicle.coordinate", json!({ "latitude": -35.36, "longitude": 149.16, "valid": true })),
            ("vehicle.homePosition", json!({ "latitude": -35.363, "longitude": 149.165, "altitude": 584.0, "valid": true })),
            ("vehicle.altitudeRelative", json!(20.0)),
            ("vehicle.heading", json!(90.0)),
            ("settings.flightMapSettings.mapProvider.rawValue", json!("Esri")),
        ];
        Fake(base.into_iter().chain(extra.iter().cloned()).collect())
    }

    fn no_gimbal() -> Value {
        json!({ "gimbals": [] })
    }

    #[test]
    fn a_flying_drone_is_placed_above_home_and_looks_ahead() {
        let view = synthetic(&flying(&[]), &no_gimbal(), DEFAULT_AIM);
        assert_eq!(view["available"], true);
        assert_eq!((view["latitude"].as_f64(), view["longitude"].as_f64(), view["aboveHome"].as_f64()), (Some(-35.36), Some(149.16), Some(20.0)));
        assert_eq!((view["homeLatitude"].as_f64(), view["homeLongitude"].as_f64()), (Some(-35.363), Some(149.165)), "height is measured from home, so the head anchors it to the terrain it draws there");
        assert_eq!(view["heading"].as_f64(), Some(90.0), "without a gimbal the camera looks where the nose points");
        assert_eq!(view["aimable"], true, "and the pilot aims it from the view");
        assert_eq!(view["imagery"], "Esri World Satellite");
        assert_eq!((view["northSpeed"].as_f64(), view["climbSpeed"].as_f64()), (Some(0.0), Some(0.0)), "no local position yet, so no velocity to fly the view on with");
        let moving = synthetic(&flying(&[("vehicle.localPosition.vx", json!(3.0)), ("vehicle.localPosition.vy", json!(-4.0)), ("vehicle.localPosition.vz", json!(-1.5))]), &no_gimbal(), DEFAULT_AIM);
        assert_eq!((moving["northSpeed"].as_f64(), moving["eastSpeed"].as_f64(), moving["climbSpeed"].as_f64()), (Some(3.0), Some(-4.0), Some(1.5)), "the view carries the drone's NED velocity as north, east and up");
        let params = |pairs: &[(&'static str, f64)]| pairs.iter().map(|(name, value)| (Box::leak(format!("vehicle.parameterManager.getParameter(-1,{name})").into_boxed_str()) as &'static str, json!(*value))).collect::<Vec<_>>();
        let tuned = params(&[("PILOT_Y_RATE", 150.0), ("PILOT_Y_EXPO", 0.0), ("RC4_DZ", 20.0), ("LOIT_SPEED", 1250.0), ("RC1_DZ", 20.0), ("RC2_DZ", 20.0), ("PILOT_SPEED_UP", 250.0), ("PILOT_SPEED_DN", 0.0), ("THR_DZ", 100.0)]);
        let loiter = synthetic(&flying(&[tuned.clone(), vec![copter_in(COPTER_LOITER)]].concat()), &no_gimbal(), DEFAULT_AIM);
        assert_eq!(loiter["response"]["yaw"], json!({ "full": 150.0, "expo": 0.0, "deadzone": 0.04 }), "half a yaw stick asks for PILOT_Y_RATE through the RC deadzone");
        assert_eq!(loiter["response"]["forward"], json!({ "full": 12.5, "expo": 0.0, "deadzone": 0.04 }), "Loiter flies the stick as a speed up to LOIT_SPEED");
        assert_eq!(loiter["response"]["climb"], json!({ "up": 2.5, "down": 2.5, "deadzone": 0.2 }), "PILOT_SPEED_DN 0 means the climb rate both ways");
        assert!(loiter["response"]["roll"].is_null() && loiter["response"]["rateMode"] == false);
        let position_hold = synthetic(&flying(&[tuned.clone(), vec![copter_in(COPTER_POSITION_HOLD)]].concat()), &no_gimbal(), DEFAULT_AIM);
        assert!(position_hold["response"]["yaw"].is_object() && position_hold["response"]["climb"].is_object() && position_hold["response"]["forward"].is_null(), "Position Hold yaws and climbs like Loiter but leans by the stick, so only yaw and climb are predicted");
        let unknown = synthetic(&flying(&[copter_in(COPTER_LOITER)]), &no_gimbal(), DEFAULT_AIM);
        assert!(unknown["response"]["yaw"].is_null() && unknown["response"]["forward"].is_null(), "without the parameters the view does not guess how the drone answers");
        let turning = synthetic(&flying(&[("vehicle.yawRate", json!(30.0))]), &no_gimbal(), DEFAULT_AIM);
        assert_eq!((turning["headingRate"].as_f64(), turning["pitchRate"].as_f64(), turning["rollRate"].as_f64()), (Some(30.0), Some(0.0), Some(0.0)), "level and turning, the view turns with the nose and keeps the pilot's tilt");
        let gimballed = synthetic(&flying(&[("vehicle.yawRate", json!(30.0))]), &json!({ "gimbals": [{ "active": true, "absoluteYaw": 90.0, "pitch": -30.0 }] }), DEFAULT_AIM);
        assert!(gimballed["headingRate"].is_null() && gimballed["pitchRate"].is_null(), "a gimbal aims the view, and its own readings are glided, not predicted from the airframe");
        let following = synthetic(&flying(&[("vehicle.yawRate", json!(30.0))]), &json!({ "gimbals": [{ "active": true, "yawLock": false, "bodyYaw": -20.0, "absoluteYaw": 40.0, "pitch": -30.0 }] }), DEFAULT_AIM);
        assert_eq!((following["heading"].as_f64(), following["headingRate"].as_f64(), following["pitch"].as_f64()), (Some(70.0), Some(30.0), Some(-30.0)), "a gimbal that turns with the nose reports only a few times a second, so the view turns with the nose's 20 Hz heading and keeps the gimbal's offset and tilt");
    }

    #[test]
    fn body_rates_become_heading_pitch_and_roll_rates() {
        let near = |a: (f64, f64, f64), b: (f64, f64, f64)| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9 && (a.2 - b.2).abs() < 1e-9;
        assert!(near(euler_rates(0.0, 0.0, [5.0, 6.0, 7.0]), (5.0, 6.0, 7.0)), "level, body rates are the Euler rates");
        let (_, pitch_rate, heading_rate) = euler_rates(90.0, 0.0, [0.0, 20.0, 0.0]);
        assert!(pitch_rate.abs() < 1e-9 && (heading_rate - 20.0).abs() < 1e-9, "banked 90, pulling back turns the heading, it does not raise the nose");
        assert_eq!(euler_rates(0.0, 90.0, [0.0, 0.0, 10.0]).2, FASTEST_TURN_DEG_S, "straight up the heading rate is unbounded, so it is capped");
    }

    fn copter_in(custom_mode: u32) -> (&'static str, Value) {
        ("vehicle", json!({ "kind": "object", "apmFirmware": true, "multiRotor": true, "customMode": custom_mode }))
    }

    const COPTER_ACRO: u32 = 1;
    const COPTER_LOITER: u32 = 5;
    const COPTER_POSITION_HOLD: u32 = 16;

    #[test]
    fn in_acro_the_view_banks_and_pitches_with_the_drone_so_the_pilot_sees_the_horizon_they_fly_by() {
        let banked = flying(&[("vehicle.pitch", json!(-20.0)), ("vehicle.roll", json!(35.0)), copter_in(COPTER_ACRO)]);
        let view = synthetic(&banked, &no_gimbal(), DEFAULT_AIM);
        assert_eq!((view["pitch"].as_f64(), view["roll"].as_f64()), (Some(-20.0 + FIXED_CAMERA_TILT_DEG), Some(35.0)), "nose down 20 and banked 35, the horizon rises and tilts as it would on a camera bolted to the frame");
        assert_eq!(view["tilt"].as_f64(), Some(FIXED_CAMERA_TILT_DEG), "a drag starts from the pilot's tilt, not from where the nose happens to point");
        let gimballed = synthetic(&banked, &json!({ "gimbals": [{ "active": true, "absoluteYaw": 90.0, "pitch": -30.0 }] }), DEFAULT_AIM);
        assert_eq!((gimballed["heading"].as_f64(), gimballed["pitch"].as_f64(), gimballed["roll"].as_f64()), (Some(90.0), Some(-50.0), Some(35.0)), "a gimbal sets where the view looks, but the horizon still moves with the airframe");
    }

    #[test]
    fn outside_rate_modes_the_view_holds_steady_like_a_gimbal_while_the_drone_leans_into_its_flight() {
        let leaning = flying(&[("vehicle.pitch", json!(-20.0)), ("vehicle.roll", json!(35.0)), copter_in(COPTER_LOITER)]);
        let view = synthetic(&leaning, &no_gimbal(), DEFAULT_AIM);
        assert_eq!((view["pitch"].as_f64(), view["roll"].as_f64()), (Some(FIXED_CAMERA_TILT_DEG), Some(0.0)), "flying forward in Loiter tips the nose down, and the view keeps looking where it was");
        let gimballed = synthetic(&leaning, &json!({ "gimbals": [{ "active": true, "absoluteYaw": 90.0, "pitch": -30.0 }] }), DEFAULT_AIM);
        assert_eq!((gimballed["pitch"].as_f64(), gimballed["roll"].as_f64()), (Some(-30.0), Some(0.0)), "a gimbal's pitch is already measured from the horizon");
        let unknown = synthetic(&flying(&[("vehicle.pitch", json!(-20.0)), ("vehicle.roll", json!(35.0))]), &no_gimbal(), DEFAULT_AIM);
        assert_eq!(unknown["roll"].as_f64(), Some(0.0), "until the mode is known the view stays level");
    }

    #[test]
    fn a_gimbal_aims_the_camera() {
        let gimbals = json!({ "gimbals": [
            { "active": false, "absoluteYaw": 10.0, "pitch": -5.0 },
            { "active": true, "absoluteYaw": 200.0, "pitch": -45.0 },
        ] });
        assert_eq!(gimbal_aim(&gimbals), Some((200.0, -45.0)), "the active gimbal decides, not the first");
        assert_eq!(gimbal_aim(&json!({ "gimbals": [{ "active": true, "absoluteYaw": null, "pitch": -45.0 }] })), None, "a gimbal that has not reported its angles cannot aim");
        let view = synthetic(&flying(&[]), &gimbals, DEFAULT_AIM);
        assert_eq!((view["pitch"].as_f64(), view["aimable"].clone()), (Some(-45.0), json!(false)), "with a gimbal the pilot aims the real camera instead");
    }

    #[test]
    fn the_pilot_aims_the_view_down_to_straight_below_and_round_from_the_nose() {
        assert_eq!(run(SYNTHETIC_AIM, "[-60, -100]")["tilt"], -60.0);
        assert!(take_changed(), "the view is told to redraw");
        let view = synthetic(&flying(&[]), &no_gimbal(), aimed());
        assert_eq!((view["pitch"].as_f64(), view["heading"].as_f64(), view["pan"].as_f64()), (Some(-60.0), Some(350.0), Some(-100.0)), "a heading of 90 panned 100 to the left looks at 350");
        assert_eq!(run(SYNTHETIC_AIM, "[-140, 0]")["tilt"], -90.0);
        assert_eq!(run(SYNTHETIC_AIM, "[30, 0]")["tilt"], 0.0, "it never looks up past the horizon");
        assert_eq!(run(SYNTHETIC_AIM, "[0, 200]")["pan"], -160.0, "a pan past behind wraps round");
        assert_eq!(run(SYNTHETIC_AIM, "[-60]")["reason"], NEEDS_DEGREES);
        assert_eq!(wrapped(-180.0), 180.0);
        run(SYNTHETIC_AIM, &format!("[{}, {}]", DEFAULT_AIM.0, DEFAULT_AIM.1));
    }

    #[test]
    fn the_beam_covers_the_ground_the_view_shows() {
        let drone = (-35.36, 149.16);
        let shape = beam(drone, 100.0, 90.0, -45.0, 70.0);
        let (near, far) = (crate::track::distance_m(drone, shape[1]), crate::track::distance_m(drone, shape[2]));
        assert!((near - 43.5).abs() < 1.0 && (far - 230.0).abs() < 2.0, "from 100 m tilted 45 down, the frame meets the ground from about 43 m to 230 m out: {near} {far}");
        assert!((crate::track::azimuth_deg(drone, shape[2]) - 55.0).abs() < 0.5, "the left edge is half the 70 degree field off the heading");
        assert_eq!((shape[0], shape[5]), (drone, drone), "the beam starts at the drone");
        let level = beam(drone, 100.0, 0.0, 0.0, 70.0);
        assert!((crate::track::distance_m(drone, level[2]) - BEAM_REACH_M).abs() < 5.0, "a frame reaching the horizon is cut at 1.5 km");
        let below = beam(drone, 100.0, 0.0, -90.0, 70.0);
        assert!(crate::track::azimuth_deg(drone, below[1]) > 90.0, "looking straight down, the near edge falls behind the drone");
    }

    #[test]
    fn the_plan_is_placed_in_the_view_at_its_height_above_home() {
        let mission = json!({ "linksStartToHome": true, "items": [
            { "index": 0, "homePosition": true, "coordinate": { "latitude": -35.363, "longitude": 149.165, "valid": true }, "amslEntryAlt": 584.0 },
            { "index": 1, "sequence": 1, "flownLeg": true, "coordinate": { "latitude": -35.362, "longitude": 149.166, "valid": true }, "amslEntryAlt": 614.0 },
            { "index": 2, "sequence": 2, "flownLeg": false, "coordinate": { "latitude": -35.361, "longitude": 149.167, "valid": true }, "amslEntryAlt": 634.0 },
            { "index": 3, "sequence": 3, "coordinate": { "latitude": 0.0, "longitude": 0.0, "valid": false }, "amslEntryAlt": 600.0 },
        ] });
        let fences = json!({
            "polygons": [{ "inclusion": true, "vertices": [{ "latitude": 1.0, "longitude": 2.0 }, { "latitude": 1.0, "longitude": 2.1 }, { "latitude": 1.1, "longitude": 2.0 }] }],
            "circles": [{ "inclusion": false, "centre": { "latitude": 3.0, "longitude": 4.0 }, "radius": 50.0 }, { "centre": null, "radius": 10.0 }],
        });
        let shown = overlays(Some(584.0), &mission, &fences, Some(100.0));
        assert_eq!(shown["route"].as_array().map(|r| r.iter().map(|p| p["aboveHome"].as_f64().unwrap()).collect::<Vec<_>>()), Some(vec![0.0, 30.0]), "the route runs from home through the flown legs only");
        assert_eq!(shown["waypoints"].as_array().map(|w| w.iter().map(|p| (p["label"].as_str().unwrap().to_string(), p["aboveHome"].as_f64().unwrap())).collect::<Vec<_>>()), Some(vec![("1".to_string(), 30.0), ("2".to_string(), 50.0)]), "an item with no position is left out");
        assert_eq!(shown["home"]["latitude"], -35.363);
        assert_eq!((shown["polygons"][0]["points"].as_array().map(Vec::len), shown["circles"].as_array().map(Vec::len)), (Some(3), Some(1)));
        assert_eq!(shown["fenceTop"], 100.0);
        assert_eq!(overlays(None, &mission, &fences, None)["route"].as_array().map(Vec::len), Some(0), "without home's height above sea level there is nothing to measure the plan from");
        assert_eq!(overlays(None, &mission, &fences, None)["fenceTop"], DEFAULT_FENCE_TOP_M);
    }

    #[test]
    fn nothing_to_draw_without_a_position_and_a_home() {
        let unplaced = Fake(BTreeMap::from([("vehicles.activeVehicleAvailable", json!(true))]));
        assert_eq!(synthetic(&unplaced, &no_gimbal(), DEFAULT_AIM)["available"], false);
        let homeless = flying(&[("vehicle.homePosition", json!({ "latitude": 0.0, "longitude": 0.0, "valid": false }))]);
        assert_eq!(synthetic(&homeless, &no_gimbal(), DEFAULT_AIM)["available"], false, "without home there is no ground to measure the height from");
    }

    #[test]
    fn imagery_is_the_providers_satellite_layer() {
        assert_eq!(imagery("Bing"), "Bing Satellite");
        assert_eq!(imagery("Google"), "Google Satellite");
        assert_eq!(imagery("Statkart"), FALLBACK_IMAGERY, "a provider without satellite imagery falls back to Bing's");
        assert_eq!(imagery(""), FALLBACK_IMAGERY);
    }
}
