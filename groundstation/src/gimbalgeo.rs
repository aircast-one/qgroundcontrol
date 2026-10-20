use serde_json::{Value, json};
use std::sync::{Mutex, PoisonError};

use crate::dropsim::{Point, Release, Target, Velocity};
use crate::geo::{geo_to_ned, ned_to_geo};
use crate::router::Backend;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fix {
    pub position: Point,
    pub at_ms: u64,
}

pub fn ground_point(
    vehicle: Point,
    altitude_agl_m: f64,
    yaw_deg: f64,
    pitch_deg: f64,
) -> Option<Point> {
    let depression = -pitch_deg;
    if altitude_agl_m <= 0.0 || depression <= 0.0 || depression > 90.0 {
        return None;
    }
    let horizontal_m = altitude_agl_m / depression.to_radians().tan();
    let (sin_yaw, cos_yaw) = yaw_deg.to_radians().sin_cos();
    let origin = (vehicle.lat, vehicle.lon, 0.0);
    let (lat, lon, _) = ned_to_geo(horizontal_m * cos_yaw, horizontal_m * sin_yaw, 0.0, origin);
    Some(Point { lat, lon })
}

pub fn velocity_from(previous: Fix, current: Fix) -> Option<Velocity> {
    let dt_s = current.at_ms.checked_sub(previous.at_ms)? as f64 / 1000.0;
    (dt_s > 0.0).then(|| {
        let origin = (previous.position.lat, previous.position.lon, 0.0);
        let (north_m, east_m, _) =
            geo_to_ned(current.position.lat, current.position.lon, 0.0, origin);
        Velocity {
            north_mps: north_m / dt_s,
            east_mps: east_m / dt_s,
        }
    })
}

static LAST_FIX: Mutex<Option<Fix>> = Mutex::new(None);

pub fn tracked(position: Point, at_ms: u64, last: Option<Fix>) -> (Target, Fix) {
    let current = Fix { position, at_ms };
    let velocity = last
        .and_then(|previous| velocity_from(previous, current))
        .unwrap_or_default();
    (Target { position, velocity }, current)
}

pub fn track(
    vehicle: Point,
    altitude_agl_m: f64,
    yaw_deg: f64,
    pitch_deg: f64,
    now_ms: u64,
) -> Option<Target> {
    let position = ground_point(vehicle, altitude_agl_m, yaw_deg, pitch_deg)?;
    let mut last = LAST_FIX.lock().unwrap_or_else(PoisonError::into_inner);
    let (target, current) = tracked(position, now_ms, *last);
    *last = Some(current);
    Some(target)
}

fn numbers(args: &[String], count: usize) -> Option<Vec<f64>> {
    let parsed: Vec<f64> = args
        .iter()
        .take(count)
        .filter_map(|a| a.parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .collect();
    (parsed.len() == count).then_some(parsed)
}

const NO_GROUND: &str =
    "the gimbal is not looking far enough below the horizon to reach the ground from this altitude";

pub fn gimbal_ground_point_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(v) = numbers(args, 5) else {
        return crate::read::refused(
            "this needs five numbers: vehicle latitude, longitude, altitude AGL, gimbal yaw and gimbal pitch (negative is down)",
        );
    };
    let vehicle = Point {
        lat: v[0],
        lon: v[1],
    };
    match ground_point(vehicle, v[2], v[3], v[4]) {
        Some(point) => {
            json!({"kind": "coordinate", "valid": true, "latitude": point.lat, "longitude": point.lon})
        }
        None => json!({"kind": "coordinate", "valid": false, "reason": NO_GROUND}),
    }
}

pub fn gimbal_drop_lead_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(v) = numbers(args, 7) else {
        return crate::read::refused(
            "this needs seven numbers: vehicle latitude, longitude, altitude AGL, north speed, east speed, gimbal yaw and gimbal pitch (negative is down)",
        );
    };
    let vehicle = Point {
        lat: v[0],
        lon: v[1],
    };
    let Some(target) = track(vehicle, v[2], v[5], v[6], crate::hub::now_ms()) else {
        return crate::read::refused(NO_GROUND);
    };
    crate::dropsim::lead(
        Release {
            position: vehicle,
            altitude_agl_m: v[2],
            velocity: Velocity {
                north_mps: v[3],
                east_mps: v[4],
            },
        },
        target,
    )
    .json()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn straight_down_lands_directly_below_the_vehicle() {
        let vehicle = Point {
            lat: 47.0,
            lon: 8.0,
        };
        let point =
            ground_point(vehicle, 50.0, 0.0, -90.0).expect("looking straight down hits the ground");
        assert!(close(point.lat, vehicle.lat, 1e-9) && close(point.lon, vehicle.lon, 1e-9));
    }

    #[test]
    fn a_45_degree_look_north_lands_one_altitude_ahead() {
        let vehicle = Point {
            lat: 47.0,
            lon: 8.0,
        };
        let point =
            ground_point(vehicle, 50.0, 0.0, -45.0).expect("45 degrees down reaches the ground");
        let (north_m, east_m, _) =
            geo_to_ned(point.lat, point.lon, 0.0, (vehicle.lat, vehicle.lon, 0.0));
        assert!(
            close(north_m, 50.0, 1e-6),
            "north offset should equal the altitude at 45 degrees, got {north_m}"
        );
        assert!(close(east_m, 0.0, 1e-6));
    }

    #[test]
    fn looking_level_or_up_never_reaches_the_ground() {
        let vehicle = Point {
            lat: 47.0,
            lon: 8.0,
        };
        assert!(ground_point(vehicle, 50.0, 0.0, 0.0).is_none(), "level");
        assert!(
            ground_point(vehicle, 50.0, 0.0, 10.0).is_none(),
            "tilted up"
        );
        assert!(
            ground_point(vehicle, 0.0, 0.0, -45.0).is_none(),
            "on the ground already, nothing to ray-cast from"
        );
    }

    #[test]
    fn two_fixes_on_a_moving_target_produce_its_velocity() {
        let vehicle = Point {
            lat: 47.0,
            lon: 8.0,
        };
        let first = ground_point(vehicle, 50.0, 0.0, -45.0).unwrap();
        let (target, fix1) = tracked(first, 1_000, None);
        assert_eq!(
            target.velocity,
            Velocity::default(),
            "a single fix carries no velocity yet"
        );

        let second = ground_point(vehicle, 50.0, 90.0, -45.0).unwrap();
        let (target, _fix2) = tracked(second, 3_000, Some(fix1));
        assert!(
            close(target.velocity.north_mps, -25.0, 1e-3),
            "the ground point swept from 50m north to 50m east over 2s, got {}",
            target.velocity.north_mps
        );
        assert!(
            close(target.velocity.east_mps, 25.0, 1e-3),
            "got {}",
            target.velocity.east_mps
        );
    }

    struct NoBackend;
    impl Backend for NoBackend {
        fn get(&self, _p: &str) -> String {
            String::new()
        }
        fn get_fields(&self, _p: &str, _f: &str) -> String {
            String::new()
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            String::new()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_ground_point_view_reports_an_invalid_fix_instead_of_refusing_outright() {
        let args: Vec<String> = ["47.0", "8.0", "50.0", "0.0", "0.0"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let answer = gimbal_ground_point_view(&NoBackend, &args);
        assert_eq!(answer["valid"], false);
    }

    #[test]
    fn the_ground_point_view_geolocates_with_five_numbers() {
        let args: Vec<String> = ["47.0", "8.0", "50.0", "0.0", "-90.0"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let answer = gimbal_ground_point_view(&NoBackend, &args);
        assert_eq!(answer["valid"], true);
        assert!(close(answer["latitude"].as_f64().unwrap(), 47.0, 1e-9));
    }
}
