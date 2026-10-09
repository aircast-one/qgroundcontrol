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
pub const SYNTHETIC_TILT: &str = "syntheticView.tilt";

pub const FIXED_CAMERA_TILT_DEG: f64 = -15.0;
pub const CAMERA_FOV_DEG: f64 = 70.0;
const FALLBACK_IMAGERY: &str = "Bing Satellite";
const TILT_RANGE_DEG: (f64, f64) = (-90.0, 0.0);
const NEEDS_DEGREES: &str = "Give the tilt in degrees, from 0 (level) to -90 (straight down).";

static TILT: Mutex<f64> = Mutex::new(FIXED_CAMERA_TILT_DEG);
static CHANGED: AtomicBool = AtomicBool::new(false);

pub fn take_changed() -> bool {
    CHANGED.swap(false, Ordering::SeqCst)
}

pub fn owns(path: &str) -> bool {
    path == SYNTHETIC_TILT
}

fn tilt() -> f64 {
    *TILT.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn run(_path: &str, args: &str) -> Value {
    let wanted = serde_json::from_str::<Value>(args).ok().and_then(|given| given.get(0).and_then(Value::as_f64)).filter(|degrees| degrees.is_finite());
    match wanted {
        None => json!({ "ok": false, "reason": NEEDS_DEGREES }),
        Some(degrees) => {
            let clamped = degrees.clamp(TILT_RANGE_DEG.0, TILT_RANGE_DEG.1);
            *TILT.lock().unwrap_or_else(PoisonError::into_inner) = clamped;
            CHANGED.store(true, Ordering::SeqCst);
            json!({ "ok": true, "tilt": clamped })
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

pub fn synthetic(backend: &dyn Backend, gimbals: &Value) -> Value {
    let placed = point(backend, "vehicle.coordinate").zip(point(backend, "vehicle.homePosition")).zip(raw(backend, "vehicle.altitudeRelative"));
    let Some((((latitude, longitude), (home_latitude, home_longitude)), above_home)) = placed else {
        return json!({ "kind": "object", "class": "SyntheticView", "available": false });
    };
    let gimbal = gimbal_aim(gimbals);
    let (heading, pitch) = gimbal.unwrap_or((raw(backend, "vehicle.heading").unwrap_or(0.0), tilt()));
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
        "tiltable": gimbal.is_none(),
        "fov": CAMERA_FOV_DEG,
        "imagery": imagery(&provider),
    })
}

pub fn synthetic_view(backend: &dyn Backend, _args: &[String]) -> Value {
    synthetic(backend, &crate::gimbal::gimbal_view(backend, &[]))
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
        let view = synthetic(&flying(&[]), &no_gimbal());
        assert_eq!(view["available"], true);
        assert_eq!((view["latitude"].as_f64(), view["longitude"].as_f64(), view["aboveHome"].as_f64()), (Some(-35.36), Some(149.16), Some(20.0)));
        assert_eq!((view["homeLatitude"].as_f64(), view["homeLongitude"].as_f64()), (Some(-35.363), Some(149.165)), "height is measured from home, so the head anchors it to the terrain it draws there");
        assert_eq!(view["heading"].as_f64(), Some(90.0), "without a gimbal the camera looks where the nose points");
        assert_eq!(view["tiltable"], true, "and the pilot tilts it from the view");
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
        let view = synthetic(&flying(&[]), &gimbals);
        assert_eq!((view["pitch"].as_f64(), view["tiltable"].clone()), (Some(-45.0), json!(false)), "with a gimbal the pilot aims the real camera instead");
    }

    #[test]
    fn the_pilot_tilts_the_view_between_level_and_straight_down() {
        assert_eq!(run(SYNTHETIC_TILT, "[-60]")["tilt"], -60.0);
        assert!(take_changed(), "the view is told to redraw");
        assert_eq!(synthetic(&flying(&[]), &no_gimbal())["pitch"], -60.0);
        assert_eq!(run(SYNTHETIC_TILT, "[-140]")["tilt"], -90.0);
        assert_eq!(run(SYNTHETIC_TILT, "[30]")["tilt"], 0.0, "it never looks up past the horizon");
        assert_eq!(run(SYNTHETIC_TILT, "[]")["reason"], NEEDS_DEGREES);
        run(SYNTHETIC_TILT, &format!("[{FIXED_CAMERA_TILT_DEG}]"));
    }

    #[test]
    fn nothing_to_draw_without_a_position_and_a_home() {
        let unplaced = Fake(BTreeMap::from([("vehicles.activeVehicleAvailable", json!(true))]));
        assert_eq!(synthetic(&unplaced, &no_gimbal())["available"], false);
        let homeless = flying(&[("vehicle.homePosition", json!({ "latitude": 0.0, "longitude": 0.0, "valid": false }))]);
        assert_eq!(synthetic(&homeless, &no_gimbal())["available"], false, "without home there is no ground to measure the height from");
    }

    #[test]
    fn imagery_is_the_providers_satellite_layer() {
        assert_eq!(imagery("Bing"), "Bing Satellite");
        assert_eq!(imagery("Google"), "Google Satellite");
        assert_eq!(imagery("Statkart"), FALLBACK_IMAGERY, "a provider without satellite imagery falls back to Bing's");
        assert_eq!(imagery(""), FALLBACK_IMAGERY);
    }
}
