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

pub const VELOCITY_WINDOW_MS: u64 = 1000;

static FIXES: Mutex<Vec<Fix>> = Mutex::new(Vec::new());

#[cfg(test)]
pub(crate) static TEST_FIXES: Mutex<()> = Mutex::new(());

pub const MAX_JUMP_M: f64 = 20.0;

fn window_target(fixes: &[Fix], current: Fix) -> Target {
    let velocity = fixes
        .first()
        .and_then(|oldest| velocity_from(*oldest, current))
        .unwrap_or_default();
    Target {
        position: current.position,
        velocity,
    }
}

pub fn windowed(fixes: &[Fix], current: Fix) -> (Target, Vec<Fix>) {
    let recent: Vec<Fix> = fixes
        .iter()
        .copied()
        .filter(|f| f.at_ms < current.at_ms && current.at_ms - f.at_ms <= VELOCITY_WINDOW_MS)
        .collect();
    let predicted = recent
        .last()
        .filter(|_| recent.len() >= 2)
        .map(|last| carried(window_target(&recent, *last), last.at_ms, current.at_ms));
    match predicted.filter(|expected| {
        crate::track::distance_m(
            (expected.position.lat, expected.position.lon),
            (current.position.lat, current.position.lon),
        ) > MAX_JUMP_M
    }) {
        Some(expected) => (expected, recent),
        None => {
            let kept: Vec<Fix> = recent.into_iter().chain(std::iter::once(current)).collect();
            (window_target(&kept, current), kept)
        }
    }
}

pub fn track(
    vehicle: Point,
    altitude_agl_m: f64,
    yaw_deg: f64,
    pitch_deg: f64,
    now_ms: u64,
) -> Option<Target> {
    ground_point(vehicle, altitude_agl_m, yaw_deg, pitch_deg)
        .map(|position| track_point(position, now_ms))
}

pub fn track_point(position: Point, at_ms: u64) -> Target {
    let mut fixes = FIXES.lock().unwrap_or_else(PoisonError::into_inner);
    let (target, kept) = windowed(&fixes, Fix { position, at_ms });
    *fixes = kept;
    target
}

pub const IMAGE_ASPECT: f64 = 16.0 / 9.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lens {
    pub horizontal_deg: f64,
    pub vertical_deg: f64,
}

static LENS: Mutex<Option<Lens>> = Mutex::new(None);

fn usable(degrees: Option<f64>) -> Option<f64> {
    degrees.filter(|d| d.is_finite() && *d > 0.0 && *d < 180.0)
}

fn vertical_for(horizontal_deg: f64) -> f64 {
    2.0 * ((horizontal_deg / 2.0).to_radians().tan() / IMAGE_ASPECT)
        .atan()
        .to_degrees()
}

pub fn lens_from(horizontal: Option<f64>, vertical: Option<f64>) -> Lens {
    let horizontal_deg = usable(horizontal).unwrap_or(crate::synthview::CAMERA_FOV_DEG);
    Lens {
        horizontal_deg,
        vertical_deg: usable(vertical).unwrap_or_else(|| vertical_for(horizontal_deg)),
    }
}

pub fn lens_of(backend: &dyn Backend) -> Lens {
    match crate::cameras::synthetic_picked(backend) {
        true => lens_from(Some(crate::synthview::CAMERA_FOV_DEG), None),
        false => {
            let (horizontal, vertical, _) = crate::lens::field_of_view(backend);
            lens_from(horizontal, vertical)
        }
    }
}

pub fn set_lens(lens: Lens) {
    *LENS.lock().unwrap_or_else(PoisonError::into_inner) = Some(lens);
}

pub fn lens() -> Lens {
    LENS.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .unwrap_or_else(|| lens_from(None, None))
}

fn half_tan(degrees: f64) -> f64 {
    (degrees / 2.0).to_radians().tan()
}

pub fn picture_to_ground(
    lens: Lens,
    vehicle: Point,
    altitude_agl_m: f64,
    (yaw_deg, pitch_deg): (f64, f64),
    (x, y): (f64, f64),
) -> Option<Point> {
    let camera = crate::videoar::Camera {
        heading: yaw_deg,
        pitch: pitch_deg,
        roll: 0.0,
        fov: lens.horizontal_deg,
    };
    crate::videoar::ground_point(
        (vehicle.lat, vehicle.lon),
        altitude_agl_m,
        (camera, Some(lens.vertical_deg)),
        IMAGE_ASPECT,
        (2.0 * x - 1.0, 1.0 - 2.0 * y),
        &|_| 0.0,
    )
    .map(|(lat, lon)| Point { lat, lon })
}

pub fn ground_to_picture(
    lens: Lens,
    vehicle: Point,
    altitude_agl_m: f64,
    (yaw_deg, pitch_deg): (f64, f64),
    target: Point,
) -> Option<(f64, f64)> {
    let camera = crate::videoar::Camera {
        heading: yaw_deg,
        pitch: pitch_deg,
        roll: 0.0,
        fov: lens.horizontal_deg,
    };
    let (north, east, _) = geo_to_ned(target.lat, target.lon, 0.0, (vehicle.lat, vehicle.lon, 0.0));
    let (across, above, ahead) = crate::videoar::in_camera(camera, (east, north, -altitude_agl_m));
    (ahead > 0.0).then(|| {
        (
            0.5 + across / ahead * 0.5 / half_tan(lens.horizontal_deg),
            0.5 - above / ahead * 0.5 / half_tan(lens.vertical_deg),
        )
    })
}

pub fn carried(target: Target, from_ms: u64, to_ms: u64) -> Target {
    let seconds = to_ms.saturating_sub(from_ms) as f64 / 1000.0;
    let (lat, lon, _) = ned_to_geo(
        target.velocity.north_mps * seconds,
        target.velocity.east_mps * seconds,
        0.0,
        (target.position.lat, target.position.lon, 0.0),
    );
    Target {
        position: Point { lat, lon },
        ..target
    }
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
    let Some(v) = numbers(args, 8) else {
        return crate::read::refused(
            "this needs eight numbers: vehicle latitude, longitude, altitude AGL, north speed, east speed, climb speed (positive is up), gimbal yaw and gimbal pitch (negative is down)",
        );
    };
    let vehicle = Point {
        lat: v[0],
        lon: v[1],
    };
    let Some(target) = track(vehicle, v[2], v[6], v[7], crate::hub::now_ms()) else {
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
            climb_mps: v[5],
        },
        target,
    )
    .json()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_point_and_its_ground_point_map_back_onto_each_other() {
        let drone = Point {
            lat: 47.0,
            lon: 8.0,
        };
        let aim = (30.0, -50.0);
        [lens_from(None, None), lens_from(Some(40.0), Some(30.0))]
            .iter()
            .for_each(|lens| {
                let ground = picture_to_ground(*lens, drone, 40.0, aim, (0.3, 0.7))
                    .expect("below the horizon");
                let (x, y) = ground_to_picture(*lens, drone, 40.0, aim, ground)
                    .expect("in front of the camera");
                assert!(
                    (x - 0.3).abs() < 0.01 && (y - 0.7).abs() < 0.01,
                    "round trip lands back on the picked point through {lens:?}, got ({x}, {y})"
                );
            });
    }

    #[test]
    fn a_narrower_lens_puts_the_bottom_edge_of_the_picture_nearer_straight_ahead() {
        let drone = Point {
            lat: 47.0,
            lon: 8.0,
        };
        let ahead_m = |lens: Lens| {
            let ground = picture_to_ground(lens, drone, 40.0, (0.0, -45.0), (0.5, 1.0)).unwrap();
            geo_to_ned(ground.lat, ground.lon, 0.0, (drone.lat, drone.lon, 0.0)).0
        };
        let expected = |vertical: f64| 40.0 / (45.0 + vertical / 2.0).to_radians().tan();
        assert!(close(
            ahead_m(lens_from(Some(60.0), Some(20.0))),
            expected(20.0),
            0.01
        ));
        assert!(close(
            ahead_m(lens_from(Some(60.0), Some(50.0))),
            expected(50.0),
            0.01
        ));
        assert!(
            close(
                lens_from(None, None).horizontal_deg,
                crate::synthview::CAMERA_FOV_DEG,
                1e-9
            ) && close(
                lens_from(Some(0.0), Some(-5.0)).vertical_deg,
                lens_from(None, None).vertical_deg,
                1e-9
            ),
            "nothing usable known falls back to the synthetic camera's lens"
        );
    }

    #[test]
    fn a_fix_far_from_where_the_track_is_heading_is_held_off_until_the_track_goes_quiet() {
        let north = |metres: f64| Point {
            lat: 47.0 + (metres / 6_371_000.0).to_degrees(),
            lon: 8.0,
        };
        let steady: Vec<Fix> = (0_u32..5)
            .map(|step| Fix {
                position: north(f64::from(step) * 0.5),
                at_ms: 1_000 + u64::from(step) * 100,
            })
            .collect();
        let (target, kept) = windowed(
            &steady,
            Fix {
                position: north(-282.0),
                at_ms: 1_500,
            },
        );
        let (ahead, _, _) = geo_to_ned(
            target.position.lat,
            target.position.lon,
            0.0,
            (47.0, 8.0, 0.0),
        );
        assert!(
            (ahead - 2.5).abs() < 0.1 && kept == steady,
            "a 284 m jump in a tenth of a second is a bad fix: the track carries on at 5 m/s instead, got {ahead}"
        );
        let (later, _) = windowed(
            &kept,
            Fix {
                position: north(-282.0),
                at_ms: 2_500,
            },
        );
        assert_eq!(
            later.position,
            north(-282.0),
            "once the old fixes age out of the window the new position is taken"
        );
    }

    #[test]
    fn a_target_is_carried_forward_along_its_velocity() {
        let start = Target {
            position: Point {
                lat: 47.0,
                lon: 8.0,
            },
            velocity: Velocity {
                north_mps: 5.0,
                east_mps: 0.0,
            },
        };
        let later = carried(start, 1_000, 1_400);
        let (north, east, _) = geo_to_ned(
            later.position.lat,
            later.position.lon,
            0.0,
            (47.0, 8.0, 0.0),
        );
        assert!(close(north, 2.0, 1e-6) && close(east, 0.0, 1e-6));
        assert_eq!(later.velocity, start.velocity);
    }

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
        let (target, fixes) = windowed(
            &[],
            Fix {
                position: first,
                at_ms: 1_000,
            },
        );
        assert_eq!(
            target.velocity,
            Velocity::default(),
            "a single fix carries no velocity yet"
        );

        let second = ground_point(vehicle, 50.0, 90.0, -45.0).unwrap();
        let (target, fixes) = windowed(
            &fixes,
            Fix {
                position: second,
                at_ms: 1_800,
            },
        );
        assert!(
            close(target.velocity.north_mps, -62.5, 1e-3),
            "the ground point swept from 50m north to 50m east over 0.8s, got {}",
            target.velocity.north_mps
        );
        assert!(
            close(target.velocity.east_mps, 62.5, 1e-3),
            "got {}",
            target.velocity.east_mps
        );

        let (again, _) = windowed(
            &fixes,
            Fix {
                position: second,
                at_ms: 1_800,
            },
        );
        assert_eq!(
            again.velocity, target.velocity,
            "the same frame read twice is the same observation, not a stop"
        );

        let (late, kept) = windowed(
            &fixes,
            Fix {
                position: second,
                at_ms: 2_500,
            },
        );
        assert_eq!(kept.len(), 2, "the fix from 1.5 s ago left the window");
        assert_eq!(
            late.velocity,
            Velocity::default(),
            "sitting still for 0.7 s"
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
