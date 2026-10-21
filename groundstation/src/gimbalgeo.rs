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

pub const MAX_JUMP_M: f64 = 20.0;
pub const ASSOCIATE_M: f64 = 6.0;
const GATE_GROWTH_MPS: f64 = 3.0;
pub const REACQUIRE_MS: u64 = 10_000;
pub const CONFIRM_FIXES: u8 = 3;
pub const OBJECT_HALF_HEIGHT_M: f64 = 0.75;
pub const COAST_MS: u64 = 1_500;
const ARRIVAL_SAMPLES: usize = 5;
const SMOOTHING: f64 = 0.75;
const POSITION_GAIN: f64 = 1.0 - SMOOTHING * SMOOTHING;
const VELOCITY_GAIN: f64 = (1.0 - SMOOTHING) * (1.0 - SMOOTHING);
const MOVING_MPS: f64 = 1.0;
const MAX_ACCEL_MPS2: f64 = 6.0;
const HEADING_SAMPLES: usize = 12;
const HEADING_EVERY_MS: u64 = 250;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sensor {
    Gimbal,
    CameraGeo,
    CameraImage,
    Detector,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Track {
    sensor: Sensor,
    origin: Point,
    north_m: f64,
    east_m: f64,
    velocity: Velocity,
    at_ms: u64,
    seen_ms: u64,
    gaps: [u64; ARRIVAL_SAMPLES],
    headings: [Option<(u64, f64)>; HEADING_SAMPLES],
    steady: u8,
}

pub fn heading_of(velocity: Velocity) -> Option<f64> {
    (velocity.north_mps.hypot(velocity.east_mps) >= MOVING_MPS)
        .then(|| velocity.east_mps.atan2(velocity.north_mps).to_degrees())
}

fn wrapped(degrees: f64) -> f64 {
    (degrees + 540.0).rem_euclid(360.0) - 180.0
}

pub fn turn_between(from_deg: f64, to_deg: f64) -> f64 {
    wrapped(to_deg - from_deg).abs()
}

impl Track {
    pub fn start(sensor: Sensor, fix: Fix, velocity: Option<Velocity>) -> Track {
        Track {
            sensor,
            origin: fix.position,
            north_m: 0.0,
            east_m: 0.0,
            velocity: velocity.unwrap_or_default(),
            at_ms: fix.at_ms,
            seen_ms: fix.at_ms,
            gaps: [0; ARRIVAL_SAMPLES],
            headings: [None; HEADING_SAMPLES],
            steady: 0,
        }
    }

    fn predicted(&self, at_ms: u64) -> (f64, f64) {
        let seconds = at_ms.saturating_sub(self.at_ms) as f64 / 1000.0;
        (
            self.north_m + self.velocity.north_mps * seconds,
            self.east_m + self.velocity.east_mps * seconds,
        )
    }

    fn remembered(&self, at_ms: u64, velocity: Velocity) -> [Option<(u64, f64)>; HEADING_SAMPLES] {
        let latest = self.headings.iter().flatten().map(|(t, _)| *t).max();
        match (heading_of(velocity), latest) {
            (Some(heading), last) if last.is_none_or(|t| at_ms >= t + HEADING_EVERY_MS) => {
                let oldest = (0..HEADING_SAMPLES)
                    .min_by_key(|&i| self.headings[i].map_or(0, |(t, _)| t))
                    .unwrap_or(0);
                std::array::from_fn(|i| match i == oldest {
                    true => Some((at_ms, heading)),
                    false => self.headings[i],
                })
            }
            _ => self.headings,
        }
    }

    pub fn updated(self, fix: Fix) -> Track {
        let seconds = fix.at_ms.saturating_sub(self.at_ms) as f64 / 1000.0;
        let (north, east) = self.predicted(fix.at_ms);
        let (seen_north, seen_east, _) = geo_to_ned(
            fix.position.lat,
            fix.position.lon,
            0.0,
            (self.origin.lat, self.origin.lon, 0.0),
        );
        let (miss_north, miss_east) = (seen_north - north, seen_east - east);
        if seconds <= 0.0 || miss_north.hypot(miss_east) > MAX_JUMP_M {
            return self;
        }
        let (change_north, change_east) = (
            VELOCITY_GAIN * miss_north / seconds,
            VELOCITY_GAIN * miss_east / seconds,
        );
        let limit =
            (MAX_ACCEL_MPS2 * seconds / change_north.hypot(change_east).max(f64::EPSILON)).min(1.0);
        let velocity = Velocity {
            north_mps: self.velocity.north_mps + change_north * limit,
            east_mps: self.velocity.east_mps + change_east * limit,
        };
        Track {
            north_m: north + POSITION_GAIN * miss_north,
            east_m: east + POSITION_GAIN * miss_east,
            velocity,
            at_ms: fix.at_ms,
            headings: self.remembered(fix.at_ms, velocity),
            steady: match miss_north.hypot(miss_east) <= ASSOCIATE_M {
                true => self.steady.saturating_add(1),
                false => 0,
            },
            ..self
        }
    }

    pub fn target(&self, at_ms: u64) -> Target {
        let (north, east) = self.predicted(at_ms);
        let (lat, lon, _) = ned_to_geo(north, east, 0.0, (self.origin.lat, self.origin.lon, 0.0));
        Target {
            position: Point { lat, lon },
            velocity: self.velocity,
        }
    }

    pub fn turned_deg(&self, at_ms: u64, window_ms: u64) -> f64 {
        heading_of(self.velocity).map_or(0.0, |now| {
            self.headings
                .iter()
                .flatten()
                .filter(|(t, _)| at_ms.saturating_sub(*t) <= window_ms)
                .map(|(_, heading)| wrapped(now - heading).abs())
                .fold(0.0, f64::max)
        })
    }

    fn fresh(&self, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.at_ms) <= COAST_MS
    }
}

pub fn stepped(
    held: Option<Track>,
    seen: Option<(Sensor, Fix, Option<Velocity>)>,
) -> Option<Track> {
    match (held, seen) {
        (Some(track), Some((sensor, fix, _)))
            if track.sensor == sensor && track.fresh(fix.at_ms) =>
        {
            Some(track.updated(fix))
        }
        (Some(track), Some((sensor, fix, velocity))) if track.sensor == sensor => {
            Some(Track::start(sensor, fix, velocity.or(Some(track.velocity))))
        }
        (_, Some((sensor, fix, velocity))) => Some(Track::start(sensor, fix, velocity)),
        (held, None) => held,
    }
}

static GIMBAL_TRACK: Mutex<Option<Track>> = Mutex::new(None);
static PICKED_TRACK: Mutex<Option<Track>> = Mutex::new(None);

#[cfg(test)]
pub(crate) static TEST_FIXES: Mutex<()> = Mutex::new(());

#[cfg(test)]
pub(crate) fn forget() {
    *GIMBAL_TRACK.lock().unwrap_or_else(PoisonError::into_inner) = None;
    *PICKED_TRACK.lock().unwrap_or_else(PoisonError::into_inner) = None;
}

fn followed(
    slot: &Mutex<Option<Track>>,
    seen: Option<(Sensor, Fix, Option<Velocity>)>,
    now_ms: u64,
) -> Option<Target> {
    let mut held = slot.lock().unwrap_or_else(PoisonError::into_inner);
    let before = *held;
    *held = stepped(*held, seen).map(|track| match before {
        Some(old) if (old.at_ms, old.origin) == (track.at_ms, track.origin) => track,
        Some(old) => Track {
            seen_ms: now_ms,
            gaps: pushed(old.gaps, now_ms.saturating_sub(old.seen_ms)),
            ..track
        },
        None => Track {
            seen_ms: now_ms,
            ..track
        },
    });
    held.filter(|track| track.fresh(now_ms))
        .map(|track| track.target(now_ms))
}

pub fn tracked(seen: Option<(Sensor, Fix, Option<Velocity>)>, now_ms: u64) -> Option<Target> {
    followed(&PICKED_TRACK, seen, now_ms)
}

pub fn release_tracked() {
    *PICKED_TRACK.lock().unwrap_or_else(PoisonError::into_inner) = None;
}

fn metres(a: Point, b: Point) -> f64 {
    let (north, east, _) = geo_to_ned(a.lat, a.lon, 0.0, (b.lat, b.lon, 0.0));
    north.hypot(east)
}

pub fn associated(seen: &[crate::detections::Seen], at_ms: u64) -> Option<Point> {
    let held = PICKED_TRACK
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .filter(|track| {
            track.sensor == Sensor::Detector
                && track.steady >= CONFIRM_FIXES
                && at_ms.saturating_sub(track.at_ms) <= REACQUIRE_MS
        });
    match held {
        Some(track) => {
            let gap_s = at_ms.saturating_sub(track.at_ms) as f64 / 1000.0;
            let last = track.target(track.at_ms);
            let roads = crate::autodrop::roads_under(last);
            let expected = crate::autodrop::coasted(last, roads.as_deref(), gap_s).position;
            let gate = (ASSOCIATE_M + GATE_GROWTH_MPS * gap_s).min(MAX_JUMP_M);
            seen.iter()
                .map(|s| (metres(s.position, expected), s.position))
                .filter(|(off, _)| *off <= gate)
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, position)| position)
        }
        None => seen.iter().find(|s| s.target).map(|s| s.position),
    }
}

fn pushed(gaps: [u64; ARRIVAL_SAMPLES], gap: u64) -> [u64; ARRIVAL_SAMPLES] {
    std::array::from_fn(|i| gaps.get(i + 1).copied().unwrap_or(gap))
}

fn median_gap(gaps: [u64; ARRIVAL_SAMPLES]) -> Option<u64> {
    let mut known: Vec<u64> = gaps.into_iter().filter(|gap| *gap > 0).collect();
    known.sort_unstable();
    known.get(known.len() / 2).copied()
}

pub fn tracked_sighting_age_ms(now_ms: u64) -> Option<u64> {
    tracked_sighting(now_ms).map(|(age, _)| age)
}

pub fn tracked_sighting(now_ms: u64) -> Option<(u64, Option<u64>)> {
    PICKED_TRACK
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .map(|track| (now_ms.saturating_sub(track.seen_ms), median_gap(track.gaps)))
}

pub fn tracked_turn_deg(now_ms: u64, window_ms: u64) -> Option<f64> {
    PICKED_TRACK
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .filter(|track| track.fresh(now_ms))
        .map(|track| track.turned_deg(now_ms, window_ms))
}

pub fn track(
    vehicle: Point,
    altitude_agl_m: f64,
    yaw_deg: f64,
    pitch_deg: f64,
    now_ms: u64,
) -> Option<Target> {
    let seen = ground_point(vehicle, altitude_agl_m, yaw_deg, pitch_deg).map(|position| {
        (
            Sensor::Gimbal,
            Fix {
                position,
                at_ms: now_ms,
            },
            None,
        )
    });
    seen.and_then(|seen| followed(&GIMBAL_TRACK, Some(seen), now_ms))
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
    aim: (f64, f64),
    point: (f64, f64),
) -> Option<Point> {
    picture_to_plane(lens, vehicle, altitude_agl_m, aim, point, 0.0)
}

pub fn picture_to_plane(
    lens: Lens,
    vehicle: Point,
    altitude_agl_m: f64,
    (yaw_deg, pitch_deg): (f64, f64),
    (x, y): (f64, f64),
    plane_m: f64,
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
        &|_| plane_m,
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

    fn north_of(metres: f64) -> Point {
        Point {
            lat: 47.0 + (metres / 6_371_000.0_f64).to_degrees(),
            lon: 8.0,
        }
    }

    fn east_of(at: Point, metres: f64) -> Point {
        Point {
            lon: at.lon + (metres / (6_371_000.0 * at.lat.to_radians().cos())).to_degrees(),
            ..at
        }
    }

    fn driven_north_for_a_second() {
        (0_u32..=10).for_each(|step| {
            let at_ms = u64::from(step) * 100;
            tracked(
                Some((
                    Sensor::Detector,
                    Fix {
                        position: north_of(f64::from(step) * 0.5),
                        at_ms,
                    },
                    None,
                )),
                at_ms,
            );
        });
    }

    #[test]
    fn a_track_started_from_a_bad_fix_takes_the_devices_pick_until_it_has_settled() {
        let _fixes = TEST_FIXES.lock().unwrap_or_else(PoisonError::into_inner);
        forget();
        tracked(
            Some((
                Sensor::Detector,
                Fix {
                    position: east_of(north_of(0.0), 18.0),
                    at_ms: 0,
                },
                None,
            )),
            0,
        );
        let car = crate::detections::Seen {
            position: north_of(0.5),
            target: true,
        };
        assert_eq!(
            associated(&[car], 100),
            Some(car.position),
            "one fix taken while the camera swung is no reason to refuse where the device sees the car"
        );
        forget();
    }

    #[test]
    fn the_core_follows_its_own_track_and_ignores_a_decoy_the_device_marks_as_the_target() {
        let _fixes = TEST_FIXES.lock().unwrap_or_else(PoisonError::into_inner);
        forget();
        let car = north_of(5.5);
        let decoy = crate::detections::Seen {
            position: east_of(car, 15.0),
            target: true,
        };
        let real = crate::detections::Seen {
            position: car,
            target: false,
        };
        assert_eq!(
            associated(&[decoy, real], 1_100),
            Some(decoy.position),
            "with no track yet, the device's pick starts it"
        );
        driven_north_for_a_second();
        assert_eq!(
            associated(&[decoy, real], 1_100),
            Some(car),
            "once tracking, the box where the car should be wins over the one flagged as the target"
        );
        forget();
    }

    #[test]
    fn a_lost_track_reacquires_where_the_car_should_be_not_where_it_was_last_seen() {
        let _fixes = TEST_FIXES.lock().unwrap_or_else(PoisonError::into_inner);
        forget();
        driven_north_for_a_second();
        let left_behind = crate::detections::Seen {
            position: north_of(5.0),
            target: true,
        };
        let moved_on = crate::detections::Seen {
            position: north_of(20.0),
            target: false,
        };
        assert_eq!(
            associated(&[left_behind, moved_on], 4_000),
            Some(moved_on.position),
            "three seconds at 5 m/s later, the car is 15 m on"
        );
        assert_eq!(
            associated(&[left_behind], 4_000 + REACQUIRE_MS),
            Some(left_behind.position),
            "a track lost for too long hands the pick back to the device"
        );
        forget();
    }

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

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    fn metres_north(n: f64) -> Point {
        Point {
            lat: 47.0 + (n / 6_371_000.0).to_degrees(),
            lon: 8.0,
        }
    }

    fn north_of_start(target: Target) -> f64 {
        geo_to_ned(
            target.position.lat,
            target.position.lon,
            0.0,
            (47.0, 8.0, 0.0),
        )
        .0
    }

    fn noise(step: u32) -> f64 {
        (f64::from(step) * 12.9898).sin() * 1.0
    }

    fn driven(steps: u32) -> Option<Track> {
        (0..steps).fold(None, |held, step| {
            let at_ms = 1_000 + u64::from(step) * 100;
            let seen = metres_north(5.0 * f64::from(step) / 10.0 + noise(step));
            stepped(
                held,
                Some((
                    Sensor::Detector,
                    Fix {
                        position: seen,
                        at_ms,
                    },
                    None,
                )),
            )
        })
    }

    #[test]
    fn noisy_fixes_along_a_straight_line_settle_on_the_true_speed_and_steady_the_position() {
        let track = driven(60).expect("a track");
        assert!(
            (track.velocity.north_mps - 5.0).abs() < 0.5 && track.velocity.east_mps.abs() < 1e-9,
            "6 s of fixes with 1 m noise at 10 Hz settle near 5 m/s north, got {:?}",
            track.velocity
        );
        let truth = 5.0 * 5.9;
        let error = (north_of_start(track.target(track.at_ms)) - truth).abs();
        assert!(
            error < 0.8,
            "the filtered position beats the raw 1 m noise, got {error}"
        );
    }

    #[test]
    fn between_fixes_the_target_moves_on_smoothly_at_its_speed() {
        let track = driven(60).unwrap();
        let positions: Vec<f64> = (0..5)
            .map(|step| north_of_start(track.target(track.at_ms + step * 20)))
            .collect();
        let steps: Vec<f64> = positions.windows(2).map(|pair| pair[1] - pair[0]).collect();
        assert!(
            steps
                .iter()
                .all(|step| (step - track.velocity.north_mps * 0.02).abs() < 1e-6),
            "every 20 ms the prediction advances by speed x 20 ms, got {steps:?}"
        );
    }

    #[test]
    fn the_detector_s_frame_interval_is_measured_from_accepted_detections() {
        let _fixes = TEST_FIXES.lock().unwrap_or_else(PoisonError::into_inner);
        let fed = |every_ms: u64| {
            forget();
            (0..8).for_each(|step| {
                let at = 1_000 + step * every_ms;
                tracked(
                    Some((
                        Sensor::Detector,
                        Fix {
                            position: metres_north(step as f64 * 0.5),
                            at_ms: at,
                        },
                        None,
                    )),
                    at + 40,
                );
            });
            tracked_sighting(1_000 + 7 * every_ms + 40).map(|(_, interval)| interval)
        };
        assert_eq!(fed(100), Some(Some(100)), "10 frames a second");
        assert_eq!(fed(250), Some(Some(250)), "4 frames a second");
    }

    #[test]
    fn only_an_accepted_detection_counts_as_seeing_the_target() {
        let _fixes = TEST_FIXES.lock().unwrap_or_else(PoisonError::into_inner);
        forget();
        let seen = |at_ms: u64, north: f64| {
            Some((
                Sensor::Detector,
                Fix {
                    position: metres_north(north),
                    at_ms,
                },
                None,
            ))
        };
        tracked(seen(1_000, 0.0), 1_300);
        tracked(seen(1_100, 0.5), 1_400);
        assert_eq!(
            tracked_sighting_age_ms(1_500),
            Some(100),
            "counted from when the detection arrived, not from when the frame was taken"
        );
        tracked(None, 1_800);
        assert_eq!(
            tracked_sighting_age_ms(1_800),
            Some(400),
            "a missed frame is not a sighting"
        );
        tracked(seen(1_200, 500.0), 1_900);
        assert_eq!(
            tracked_sighting_age_ms(1_900),
            Some(500),
            "a box rejected as a jump is not the target seen again"
        );
    }

    #[test]
    fn a_missed_frame_coasts_and_a_long_gap_drops_the_track() {
        let _fixes = TEST_FIXES.lock().unwrap_or_else(PoisonError::into_inner);
        forget();
        let seen = |at_ms: u64, north: f64| {
            Some((
                Sensor::Detector,
                Fix {
                    position: metres_north(north),
                    at_ms,
                },
                None,
            ))
        };
        tracked(seen(1_000, 0.0), 1_000);
        let moving = tracked(seen(2_000, 5.0), 2_000).unwrap();
        let coasting = tracked(None, 2_500).expect("half a second without a frame still coasts");
        assert!(north_of_start(coasting) > north_of_start(moving));
        assert_eq!(
            tracked(None, 2_000 + COAST_MS + 1),
            None,
            "a target unseen for longer than the coast window is no target"
        );
        assert_eq!(
            tracked(seen(2_000 + COAST_MS + 100, 300.0), 2_000 + COAST_MS + 100)
                .map(north_of_start)
                .map(f64::round),
            Some(300.0),
            "the next fix after the track dropped starts a new one where it is seen"
        );
    }

    #[test]
    fn a_track_that_lapsed_restarts_where_it_is_seen_but_keeps_its_speed() {
        let track = driven(60).unwrap();
        let later = track.at_ms + COAST_MS + 500;
        let restarted = stepped(
            Some(track),
            Some((
                Sensor::Detector,
                Fix {
                    position: metres_north(100.0),
                    at_ms: later,
                },
                None,
            )),
        )
        .unwrap();
        assert_eq!(restarted.velocity, track.velocity);
        assert_eq!(north_of_start(restarted.target(later)).round(), 100.0);
    }

    #[test]
    fn a_wild_fix_is_gated_out_and_the_same_frame_read_twice_changes_nothing() {
        let track = driven(30).unwrap();
        let wild = track.updated(Fix {
            position: metres_north(-284.0),
            at_ms: track.at_ms + 100,
        });
        assert_eq!(
            wild, track,
            "a 284 m jump in a tenth of a second is a bad fix"
        );
        let again = track.updated(Fix {
            position: metres_north(1_000.0),
            at_ms: track.at_ms,
        });
        assert_eq!(
            again, track,
            "a frame already folded in is not a new observation"
        );
    }

    #[test]
    fn a_track_reports_how_far_its_heading_swung_in_the_recent_window() {
        let at = |north: f64, east: f64| {
            let (lat, lon, _) = ned_to_geo(north, east, 0.0, (47.0, 8.0, 0.0));
            Point { lat, lon }
        };
        let path = |step: u32| match f64::from(step) {
            s if s < 40.0 => at(s, 0.0),
            s => at(40.0, s - 40.0),
        };
        let track = |steps: u32| {
            {
                (0..steps).fold(None, |held, step| {
                    stepped(
                        held,
                        Some((
                            Sensor::Detector,
                            Fix {
                                position: path(step),
                                at_ms: 1_000 + u64::from(step) * 100,
                            },
                            None,
                        )),
                    )
                })
            }
            .expect("a track")
        };
        let straight = track(40);
        assert!(straight.turned_deg(straight.at_ms, 2_000) < 5.0);
        let turned = track(60);
        let swung = turned.turned_deg(turned.at_ms, 2_000);
        assert!(
            swung > 60.0,
            "a right-angle turn shows within two seconds, got {swung}"
        );
        assert!(
            turned.turned_deg(turned.at_ms + 10_000, 2_000) < 1.0,
            "an old turn ages out"
        );
    }

    #[test]
    fn a_bad_fix_inside_the_gate_bends_the_speed_no_faster_than_a_car_can() {
        let track = driven(60).unwrap();
        let (north, _) = track.predicted(track.at_ms + 100);
        let bad = track.updated(Fix {
            position: metres_north(north + 15.0),
            at_ms: track.at_ms + 100,
        });
        let change = bad.velocity.north_mps - track.velocity.north_mps;
        assert!(
            change > 0.0 && change <= MAX_ACCEL_MPS2 * 0.1 + 1e-9,
            "a 15 m jump a tenth of a second on moves the speed by at most {} m/s, got {change}",
            MAX_ACCEL_MPS2 * 0.1
        );
    }

    #[test]
    fn a_camera_report_with_its_own_speed_starts_the_track_at_that_speed() {
        let started = Track::start(
            Sensor::CameraGeo,
            Fix {
                position: metres_north(0.0),
                at_ms: 1_000,
            },
            Some(Velocity {
                north_mps: 3.0,
                east_mps: -1.0,
            }),
        );
        assert_eq!(
            (started.velocity.north_mps, started.velocity.east_mps),
            (3.0, -1.0)
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
