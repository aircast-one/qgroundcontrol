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
    "settings.flightMapSettings.mapProvider",
    crate::gimbal::GIMBAL_CHANGED,
    SYNTHETIC_CHANGED,
];

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

static AIM: Mutex<(f64, f64)> = Mutex::new(DEFAULT_AIM);
static CHANGED: AtomicBool = AtomicBool::new(false);

pub fn take_changed() -> bool {
    CHANGED.swap(false, Ordering::SeqCst)
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

pub fn gimbal_aim(gimbals: &Value) -> Option<(f64, f64)> {
    let all = gimbals["gimbals"].as_array().cloned().unwrap_or_default();
    all.iter()
        .find(|gimbal| flag(gimbal, "active"))
        .or(all.first())
        .and_then(|gimbal| Some((gimbal["absoluteYaw"].as_f64()?, gimbal["pitch"].as_f64()?)))
        .filter(|(yaw, pitch)| yaw.is_finite() && pitch.is_finite())
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
        return json!({ "kind": "object", "class": "SyntheticView", "available": false });
    };
    let gimbal = gimbal_aim(gimbals);
    let (tilt, pan) = aim;
    let (heading, pitch) = gimbal.unwrap_or(((raw(backend, "vehicle.heading").unwrap_or(0.0) + pan).rem_euclid(360.0), tilt));
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
        "roll": 0.0,
        "aimable": gimbal.is_none(),
        "pan": if gimbal.is_none() { pan } else { 0.0 },
        "fov": CAMERA_FOV_DEG,
        "imagery": imagery(&provider),
        "beam": beam((latitude, longitude), above_home, heading, pitch, CAMERA_FOV_DEG).iter().map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon })).collect::<Vec<_>>(),
    })
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
