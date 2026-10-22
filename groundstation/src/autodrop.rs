use serde_json::{Value, json};

use crate::dropsim::{self, Point, Target, Velocity};
use crate::geo::{geo_to_ned, ned_to_geo};
use crate::gimbalgeo;
use crate::gpstarget;
use crate::read::flag;
use crate::router::Backend;

pub const COMMAND_LATENCY_S: f64 = 0.15;
const LATENCY_RANGE_S: std::ops::RangeInclusive<f64> = 0.05..=1.0;
const LATENCY_LEARNING: f64 = 0.5;

pub const RELEASE_LATENCY_SETTING: &str = "settings.appSettings.autodropReleaseLatencyMs";

pub fn calibrated_latency_s() -> f64 {
    crate::settingsstore::raw_setting(RELEASE_LATENCY_SETTING)
        .and_then(|ms| ms.as_f64())
        .map_or(COMMAND_LATENCY_S, |ms| {
            (ms / 1000.0).clamp(*LATENCY_RANGE_S.start(), *LATENCY_RANGE_S.end())
        })
}

pub fn learned_latency_s(held_s: f64, measured_s: f64) -> f64 {
    (held_s + LATENCY_LEARNING * (measured_s - held_s))
        .clamp(*LATENCY_RANGE_S.start(), *LATENCY_RANGE_S.end())
}
pub const MAX_RUN_S: f64 = 90.0;
pub const AUTODROP_TICK_MS: u64 = 150;
const GUIDED_GOTO_LIMIT_M: f64 = 950.0;
pub const CRUISE_MPS: f64 = 10.0;
pub const APPROACH_SETTLE_MS: u64 = 2_000;
const SETTLE_MPS: f64 = 3.0;
const CLOSING_MPS: f64 = 3.0;
const ENGAGE_M: f64 = 100.0;
pub const ENGAGE_RANGE_SETTING: &str = "settings.appSettings.autodropEngageRangeM";
pub const REAIM_EVERY_MS: u64 = 1000;
pub const REAIM_UNTIL_S: f64 = 2.0;
pub const FIRE_WITHIN_M: f64 = 2.0;
pub const CAR_LENGTH_M: f64 = 4.4;
pub const CAR_WIDTH_M: f64 = 1.8;
pub const RELEASE_MARGIN_M: f64 = 0.5;
pub const TURNED_DEG: f64 = 15.0;
pub const FRESH_SIGHTING_MS: u64 = 300;
pub const MAX_FRESH_SIGHTING_MS: u64 = 700;
pub const TURN_WINDOW_MS: u64 = 2_000;
const PLAN_STEP_S: f64 = 0.05;
const PLAN_HORIZON_S: f64 = 60.0;
const REFINE_STEPS: i32 = 10;
const BRANCHES_WITHIN_S: f64 = 1.0;

pub const CORNER_LATERAL_MPS2: f64 = 2.0;
pub const TIMED_OUT: &str = "no release window opened within the time limit";
pub const HOLD_AFTER_FIRE_MS: u64 = 1_000;
const MOVING_MPS: f64 = 1.0;
const CAR_LABELS: [&str; 5] = ["car", "van", "truck", "bus", "pickup"];

#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    pub target: String,
    pub distance_m: Option<f64>,
    pub speed_mps: Option<f64>,
    pub searching: bool,
    pub release_in_s: Option<f64>,
    pub holding: Option<&'static str>,
    pub time_left_s: f64,
}

impl Progress {
    pub fn json(&self) -> Value {
        json!({
            "target": self.target,
            "distanceMeters": self.distance_m,
            "speedMps": self.speed_mps,
            "moving": self.speed_mps.map(|speed| speed >= MOVING_MPS),
            "searching": self.searching,
            "releaseInSeconds": self.release_in_s,
            "holding": self.holding,
            "secondsLeft": self.time_left_s.max(0.0).round(),
        })
    }
}

pub fn label(source: TargetSource, seen_label: Option<&str>) -> String {
    match source {
        TargetSource::Tracked => seen_label
            .map(str::trim)
            .filter(|seen| !seen.is_empty() && !seen.eq_ignore_ascii_case("tracked"))
            .map(
                |seen| match CAR_LABELS.iter().any(|car| seen.eq_ignore_ascii_case(car)) {
                    true => "car".to_string(),
                    false => seen.to_lowercase(),
                },
            )
            .unwrap_or_else(|| "target".to_string()),
        TargetSource::Vehicle(id) => format!("vehicle {id}"),
        TargetSource::Manual => "marked point".to_string(),
        TargetSource::Gimbal => "gimbal point".to_string(),
    }
}

pub fn hold_reason(plan: Plan, turned_deg: f64) -> &'static str {
    match (plan.junction, turned_deg > TURNED_DEG) {
        (true, _) => "junction",
        (false, true) => "turning",
        (false, false) => "offTarget",
    }
}

pub fn next_bay(bays: i64, empty: &[i64]) -> Option<(i64, usize)> {
    let loaded: Vec<i64> = match bays > 1 {
        true => (1..=bays).filter(|bay| !empty.contains(bay)).collect(),
        false => (!empty.contains(&0)).then_some(0).into_iter().collect(),
    };
    loaded.first().map(|bay| (*bay, loaded.len()))
}

fn payload_of(backend: &dyn Backend) -> (i64, Vec<i64>) {
    let bays =
        crate::read::value_number(&backend.value("settings.flyViewSettings.payloadBays.rawValue"))
            .map_or(1, |bays| bays as i64)
            .max(1);
    let empty = backend.value("vehicle.payloadSetup")["value"]["empty"]
        .as_array()
        .map(|empty| empty.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default();
    (bays, empty)
}

pub fn payload_json(backend: &dyn Backend) -> Value {
    let (bays, empty) = payload_of(backend);
    let next = next_bay(bays, &empty);
    json!({
        "bays": bays,
        "bay": next.map(|(bay, _)| bay),
        "left": next.map_or(0, |(_, loaded)| loaded),
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan {
    pub release_in_s: f64,
    pub miss_m: f64,
    pub along_m: f64,
    pub across_m: f64,
    pub moving: bool,
    pub junction: bool,
    pub target_at_impact: Point,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Course {
    north_m: f64,
    east_m: f64,
    heading_deg: Option<f64>,
    junctions: usize,
}

fn steps_for(seconds: f64) -> usize {
    (seconds / PLAN_STEP_S).ceil() as usize + 1
}

fn courses_on_road(
    target: Target,
    roads: Option<&crate::roads::Network>,
    seconds: f64,
    branching: bool,
) -> Option<Vec<Vec<Course>>> {
    let heading = gimbalgeo::heading_of(target.velocity)?;
    let network = roads?;
    let here = (target.position.lat, target.position.lon);
    let matched = network.matched(here, heading)?;
    let speed = target.velocity.north_mps.hypot(target.velocity.east_mps);
    let steps = steps_for(seconds);
    let routes = match branching {
        true => network.branches(matched, speed, CORNER_LATERAL_MPS2, PLAN_STEP_S, steps),
        false => vec![network.ahead(matched, speed, CORNER_LATERAL_MPS2, PLAN_STEP_S, steps)],
    };
    Some(
        routes
            .into_iter()
            .map(|route| {
                route
                    .into_iter()
                    .map(|ahead| {
                        let (north_m, east_m, _) =
                            geo_to_ned(ahead.at.0, ahead.at.1, 0.0, (here.0, here.1, 0.0));
                        Course {
                            north_m,
                            east_m,
                            heading_deg: Some(ahead.heading_deg),
                            junctions: ahead.junctions,
                        }
                    })
                    .collect()
            })
            .collect(),
    )
}

fn road_course(
    target: Target,
    roads: Option<&crate::roads::Network>,
    seconds: f64,
) -> Option<Vec<Course>> {
    courses_on_road(target, roads, seconds, false)?
        .into_iter()
        .next()
}

fn every_branch_hits(
    target: Target,
    roads: Option<&crate::roads::Network>,
    flown_s: f64,
    drop_offset: (f64, f64),
) -> bool {
    let heading_now = gimbalgeo::heading_of(target.velocity);
    courses_on_road(target, roads, flown_s, true).is_some_and(|branches| {
        branches.iter().all(|branch| {
            let there = sampled(branch, flown_s);
            let (miss_n, miss_e) = (drop_offset.0 + there.north_m, drop_offset.1 + there.east_m);
            let (along_m, across_m, moving) = framed(miss_n, miss_e, there.heading_deg);
            let straight_on = heading_now
                .zip(there.heading_deg)
                .is_some_and(|(now, then)| gimbalgeo::turn_between(now, then) <= TURNED_DEG);
            straight_on
                && on_car(
                    along_m,
                    across_m,
                    miss_n.hypot(miss_e),
                    moving,
                    RELEASE_MARGIN_M,
                )
        })
    })
}

fn course(target: Target, roads: Option<&crate::roads::Network>, seconds: f64) -> Vec<Course> {
    road_course(target, roads, seconds).unwrap_or_else(|| {
        (0..=steps_for(seconds))
            .map(|step| {
                let flown = step as f64 * PLAN_STEP_S;
                Course {
                    north_m: target.velocity.north_mps * flown,
                    east_m: target.velocity.east_mps * flown,
                    heading_deg: gimbalgeo::heading_of(target.velocity),
                    junctions: 0,
                }
            })
            .collect()
    })
}

fn sampled(course: &[Course], seconds: f64) -> Course {
    let last = course.len() - 1;
    let at = (seconds / PLAN_STEP_S).max(0.0);
    let before = (at.floor() as usize).min(last);
    let after = (before + 1).min(last);
    let share = (at - before as f64).clamp(0.0, 1.0);
    Course {
        north_m: course[before].north_m + (course[after].north_m - course[before].north_m) * share,
        east_m: course[before].east_m + (course[after].east_m - course[before].east_m) * share,
        ..course[after]
    }
}

pub fn plan(
    release: dropsim::Release,
    target: Target,
    roads: Option<&crate::roads::Network>,
) -> Plan {
    let carried = dropsim::fall(release, dropsim::air());
    let fall = carried.seconds;
    let origin = (release.position.lat, release.position.lon, 0.0);
    let (north, east, _) = geo_to_ned(target.position.lat, target.position.lon, 0.0, origin);
    let course = course(target, roads, PLAN_HORIZON_S + fall);
    let dropped_from = |release_in_s: f64| {
        (
            release.velocity.north_mps * release_in_s + carried.north_m,
            release.velocity.east_mps * release_in_s + carried.east_m,
        )
    };
    let at = |release_in_s: f64| {
        let flown = release_in_s + fall;
        let there = sampled(&course, flown);
        let landed = dropped_from(release_in_s);
        let miss = (
            north + there.north_m - landed.0,
            east + there.east_m - landed.1,
        );
        (release_in_s, miss, there)
    };
    let closest = |times: &mut dyn Iterator<Item = f64>| {
        times
            .map(at)
            .min_by(|a, b| a.1.0.hypot(a.1.1).total_cmp(&b.1.0.hypot(b.1.1)))
            .expect("the plan always tries at least one release time")
    };
    let steps = (PLAN_HORIZON_S / PLAN_STEP_S) as usize;
    let (coarse, _, _) = closest(&mut (0..=steps).map(|step| step as f64 * PLAN_STEP_S));
    let (release_in_s, (miss_n, miss_e), there) = closest(
        &mut (-REFINE_STEPS..=REFINE_STEPS)
            .map(|step| coarse + f64::from(step) * PLAN_STEP_S / f64::from(REFINE_STEPS))
            .filter(|release_in_s| *release_in_s >= 0.0),
    );
    let (along_m, across_m, moving) = framed(miss_n, miss_e, there.heading_deg);
    let (lat, lon, _) = ned_to_geo(north + there.north_m, east + there.east_m, 0.0, origin);
    let flown = release_in_s + fall;
    let landed = dropped_from(release_in_s);
    let drop_offset = (north - landed.0, east - landed.1);
    let junction = there.junctions > 0
        && (release_in_s > BRANCHES_WITHIN_S
            || !every_branch_hits(target, roads, flown, drop_offset));
    Plan {
        release_in_s,
        miss_m: miss_n.hypot(miss_e),
        along_m,
        across_m,
        moving,
        junction,
        target_at_impact: Point { lat, lon },
    }
}

pub fn coasted(target: Target, roads: Option<&crate::roads::Network>, seconds: f64) -> Target {
    let there = sampled(&course(target, roads, seconds), seconds);
    let (lat, lon, _) = ned_to_geo(
        there.north_m,
        there.east_m,
        0.0,
        (target.position.lat, target.position.lon, 0.0),
    );
    let speed = target.velocity.north_mps.hypot(target.velocity.east_mps);
    Target {
        position: Point { lat, lon },
        velocity: there.heading_deg.map_or(target.velocity, |heading| {
            let (sin, cos) = heading.to_radians().sin_cos();
            Velocity {
                north_mps: speed * cos,
                east_mps: speed * sin,
            }
        }),
    }
}

pub fn lead_now(
    release: dropsim::Release,
    target: Target,
    roads: Option<&crate::roads::Network>,
) -> Point {
    let fall = dropsim::fall(release, dropsim::air()).seconds;
    let there = sampled(&course(target, roads, fall), fall);
    let (lat, lon, _) = ned_to_geo(
        there.north_m,
        there.east_m,
        0.0,
        (target.position.lat, target.position.lon, 0.0),
    );
    Point { lat, lon }
}

pub fn roads_under(target: Target) -> Option<std::sync::Arc<crate::roads::Network>> {
    gimbalgeo::heading_of(target.velocity)
        .and_then(|_| crate::roads::near(target.position.lat, target.position.lon))
}

fn framed(miss_n: f64, miss_e: f64, heading_deg: Option<f64>) -> (f64, f64, bool) {
    match heading_deg {
        Some(heading) => {
            let (sin, cos) = heading.to_radians().sin_cos();
            (
                miss_n * cos + miss_e * sin,
                -miss_n * sin + miss_e * cos,
                true,
            )
        }
        None => (miss_n.hypot(miss_e), miss_n.hypot(miss_e), false),
    }
}

fn on_car(along_m: f64, across_m: f64, miss_m: f64, moving: bool, margin_m: f64) -> bool {
    match moving {
        true => {
            along_m.abs() <= CAR_LENGTH_M / 2.0 - margin_m
                && across_m.abs() <= CAR_WIDTH_M / 2.0 - margin_m
        }
        false => miss_m <= CAR_WIDTH_M / 2.0 - margin_m,
    }
}

pub fn sighting_window_ms(interval_ms: Option<u64>) -> u64 {
    interval_ms.map_or(FRESH_SIGHTING_MS, |interval| {
        (interval * 5 / 2).clamp(FRESH_SIGHTING_MS, MAX_FRESH_SIGHTING_MS)
    })
}

pub fn sighted(sighting: Option<(u64, Option<u64>)>) -> bool {
    sighting.is_some_and(|(age, interval)| age <= sighting_window_ms(interval))
}

pub fn releasable(plan: Plan, turned_deg: f64) -> bool {
    on_car(
        plan.along_m,
        plan.across_m,
        plan.miss_m,
        plan.moving,
        RELEASE_MARGIN_M,
    ) && !plan.junction
        && turned_deg <= TURNED_DEG
}

pub fn judged(landed: Point, target: Target) -> Impact {
    let (miss_n, miss_e, _) = geo_to_ned(
        landed.lat,
        landed.lon,
        0.0,
        (target.position.lat, target.position.lon, 0.0),
    );
    let miss_m = miss_n.hypot(miss_e);
    let (along_m, across_m, moving) =
        framed(miss_n, miss_e, gimbalgeo::heading_of(target.velocity));
    Impact::Seen {
        miss_m,
        on_target: on_car(along_m, across_m, miss_m, moving, 0.0),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Impact {
    Falling,
    Seen { miss_m: f64, on_target: bool },
    Unseen,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    Approaching,
    WaitingForCruise,
    WaitingForRelease,
}

pub fn engage_range_m() -> f64 {
    crate::settingsstore::raw_setting(ENGAGE_RANGE_SETTING)
        .and_then(|value| value.as_f64())
        .filter(|metres| metres.is_finite() && *metres > 0.0)
        .unwrap_or(ENGAGE_M)
}

pub fn approach_speed(target_speed_mps: f64, settling: bool) -> f64 {
    match settling {
        true => SETTLE_MPS,
        false => (target_speed_mps + CLOSING_MPS).clamp(SETTLE_MPS, CRUISE_MPS),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetSource {
    Gimbal,
    Vehicle(u8),
    Manual,
    Tracked,
}

impl TargetSource {
    pub fn of(args: &Value) -> Option<TargetSource> {
        match args.get(0).and_then(Value::as_str) {
            None | Some("gimbal") => Some(TargetSource::Gimbal),
            Some("manual") => Some(TargetSource::Manual),
            Some("tracked") => Some(TargetSource::Tracked),
            Some("vehicle") => args
                .get(1)
                .and_then(Value::as_u64)
                .and_then(|id| u8::try_from(id).ok())
                .map(TargetSource::Vehicle),
            Some(_) => None,
        }
    }

    fn json(self) -> Value {
        match self {
            TargetSource::Gimbal => json!("gimbal"),
            TargetSource::Vehicle(id) => json!({ "vehicle": id }),
            TargetSource::Manual => json!("manual"),
            TargetSource::Tracked => json!("tracked"),
        }
    }

    pub fn kind(self) -> &'static str {
        match self {
            TargetSource::Gimbal => "gimbal",
            TargetSource::Vehicle(_) => "vehicle",
            TargetSource::Manual => "manual",
            TargetSource::Tracked => "tracked",
        }
    }

    fn args(self) -> Value {
        match self {
            TargetSource::Gimbal => json!(["gimbal"]),
            TargetSource::Vehicle(id) => json!(["vehicle", id]),
            TargetSource::Manual => json!(["manual"]),
            TargetSource::Tracked => json!(["tracked"]),
        }
    }

    pub(crate) fn fallback_order(self_id: Option<u8>) -> impl Iterator<Item = TargetSource> {
        [TargetSource::Tracked, TargetSource::Manual]
            .into_iter()
            .chain(
                self_id
                    .and_then(gpstarget::only_other)
                    .map(TargetSource::Vehicle),
            )
            .chain(std::iter::once(TargetSource::Gimbal))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Waiting {
        phase: Phase,
        source: TargetSource,
        started_ms: u64,
        due_ms: u64,
    },
    Fired {
        predicted_miss_m: f64,
        bottle: usize,
        impact: Impact,
        at_ms: u64,
    },
    GaveUp {
        reason: String,
        at_ms: u64,
    },
}

impl State {
    pub fn json(&self) -> Value {
        match self {
            State::Waiting { phase, source, .. } => json!({
                "state": "waiting",
                "phase": match phase {
                    Phase::Approaching => "approaching",
                    Phase::WaitingForCruise => "waitingForCruise",
                    Phase::WaitingForRelease => "waitingForRelease",
                },
                "source": source.json(),
            }),
            State::Fired {
                predicted_miss_m,
                impact,
                at_ms,
                ..
            } => {
                let (seen, miss_m, on_target) = match impact {
                    Impact::Falling => ("falling", None, None),
                    Impact::Seen { miss_m, on_target } => ("seen", Some(*miss_m), Some(*on_target)),
                    Impact::Unseen => ("unseen", None, None),
                };
                json!({
                    "state": "fired",
                    "predictedMissMeters": predicted_miss_m,
                    "atMs": at_ms,
                    "impact": seen,
                    "estimatedMissMeters": miss_m,
                    "estimatedOnTarget": on_target,
                })
            }
            State::GaveUp { reason, at_ms } => {
                json!({ "state": "gaveUp", "reason": reason, "timedOut": reason == TIMED_OUT, "atMs": at_ms })
            }
        }
    }
}

pub fn aim_point(
    position: Point,
    target: Target,
    roads: Option<&crate::roads::Network>,
) -> Option<Point> {
    road_lead(position, target, roads)
        .or_else(|| {
            dropsim::intercept(position, CRUISE_MPS, target).map(|solved| solved.lead_point)
        })
        .map(|lead| far_point_on_bearing(position, lead, GUIDED_GOTO_LIMIT_M))
}

fn road_lead(
    position: Point,
    target: Target,
    roads: Option<&crate::roads::Network>,
) -> Option<Point> {
    let origin = (position.lat, position.lon, 0.0);
    let (north, east, _) = geo_to_ned(target.position.lat, target.position.lon, 0.0, origin);
    road_course(target, roads, PLAN_HORIZON_S)?
        .into_iter()
        .enumerate()
        .find(|(step, there)| {
            (north + there.north_m).hypot(east + there.east_m)
                <= CRUISE_MPS * *step as f64 * PLAN_STEP_S
        })
        .map(|(_, there)| {
            let (lat, lon, _) = ned_to_geo(north + there.north_m, east + there.east_m, 0.0, origin);
            Point { lat, lon }
        })
}

pub fn along_bearing(from: Point, bearing_deg: f64) -> Point {
    let (sin, cos) = bearing_deg.to_radians().sin_cos();
    let (lat, lon, _) = ned_to_geo(
        cos * GUIDED_GOTO_LIMIT_M,
        sin * GUIDED_GOTO_LIMIT_M,
        0.0,
        (from.lat, from.lon, 0.0),
    );
    Point { lat, lon }
}

fn far_point_on_bearing(from: Point, towards: Point, distance_m: f64) -> Point {
    let origin = (from.lat, from.lon, 0.0);
    let (n0, e0, _) = geo_to_ned(towards.lat, towards.lon, 0.0, origin);
    let mag = n0.hypot(e0).max(1e-6);
    let (lat, lon, _) = ned_to_geo(n0 / mag * distance_m, e0 / mag * distance_m, 0.0, origin);
    Point { lat, lon }
}

fn active_gimbal(backend: &dyn Backend) -> Option<(f64, f64)> {
    let snapshot = crate::gimbal::gimbal_view(backend, &[]);
    let gimbals = snapshot["gimbals"].as_array()?;
    let active = gimbals
        .iter()
        .find(|g| flag(g, "active"))
        .or(gimbals.first())?;
    Some((active["pitch"].as_f64()?, active["absoluteYaw"].as_f64()?))
}

fn release_position(backend: &dyn Backend) -> Option<(Point, f64)> {
    let (lat, lon) = backend.point("vehicle.coordinate")?;
    let altitude_agl_m = backend.raw("vehicle.altitudeRelative")?;
    Some((Point { lat, lon }, altitude_agl_m))
}

fn vehicle_id(backend: &dyn Backend) -> Option<Value> {
    backend
        .value("vehicle.id")
        .get("value")
        .cloned()
        .filter(Value::is_u64)
}

fn manual_target(backend: &dyn Backend) -> Option<Target> {
    let (lat, lon) = backend.point("vehicle.manualTarget")?;
    Some(Target {
        position: Point { lat, lon },
        velocity: Velocity::default(),
    })
}

fn target_from(
    backend: &dyn Backend,
    source: TargetSource,
    position: Point,
    altitude_agl_m: f64,
) -> Result<Target, &'static str> {
    match source {
        TargetSource::Gimbal => {
            let (pitch, yaw) = active_gimbal(backend).ok_or("no gimbal to aim with")?;
            gimbalgeo::track(position, altitude_agl_m, yaw, pitch, crate::hub::now_ms())
                .ok_or("the gimbal is not looking far enough below the horizon to reach the ground")
        }
        TargetSource::Vehicle(id) => {
            gpstarget::target_of(id).ok_or("no live telemetry recorded yet for that vehicle id")
        }
        TargetSource::Manual => {
            manual_target(backend).ok_or("no target has been marked on the map yet")
        }
        TargetSource::Tracked => crate::hub::lock()
            .active()
            .and_then(|vehicle| {
                vehicle.target_for_autodrop(
                    TargetSource::Tracked,
                    position,
                    altitude_agl_m,
                    crate::hub::now_ms(),
                )
            })
            .ok_or("the camera is not tracking anything"),
    }
}

pub(crate) fn start_order(self_id: Option<u8>, tracker_locked: bool) -> Vec<TargetSource> {
    match tracker_locked {
        true => vec![TargetSource::Tracked],
        false => TargetSource::fallback_order(self_id).collect(),
    }
}

fn tracker_locked() -> bool {
    crate::devicetrack::locked()
        || crate::hub::lock().active().is_some_and(|vehicle| {
            vehicle.camera_tracking_geo.is_some() || vehicle.camera_tracking_image.is_some()
        })
}

pub fn run(backend: &dyn Backend, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let Some(id) = vehicle_id(backend) else {
        return json!({ "ok": false, "reason": "No vehicle is connected." });
    };
    if given.get(0).and_then(Value::as_str) == Some("stop") {
        return match backend.core_guided(&json!({"action": "autodropStop", "vehicle": id})) {
            Some(Ok(())) => json!({ "ok": true, "stopped": true }),
            Some(Err(reason)) => json!({ "ok": false, "reason": reason }),
            None => json!({ "ok": false, "reason": "this build has no core vehicle to stop" }),
        };
    }

    let (bays, empty) = payload_of(backend);
    let Some((bay, _)) = next_bay(bays, &empty) else {
        return json!({ "ok": false, "reason": "No bottles left" });
    };
    let Some((position, altitude_agl_m)) = release_position(backend) else {
        return json!({ "ok": false, "reason": "no vehicle position yet" });
    };

    let (source, target) = if given.get(0).and_then(Value::as_str).is_some() {
        let Some(source) = TargetSource::of(&given) else {
            return json!({ "ok": false, "reason": "the target source must be gimbal, manual, or vehicle with an id" });
        };
        match target_from(backend, source, position, altitude_agl_m) {
            Ok(target) => (source, target),
            Err(reason) => return json!({ "ok": false, "reason": reason }),
        }
    } else {
        let self_id = id.as_u64().and_then(|n| u8::try_from(n).ok());
        let locked = tracker_locked();
        let Some(resolved) = start_order(self_id, locked).into_iter().find_map(|source| {
            target_from(backend, source, position, altitude_agl_m)
                .ok()
                .map(|target| (source, target))
        }) else {
            return json!({ "ok": false, "reason": match locked {
                true => "The tracker has a lock but hasn't placed the target on the ground yet. Try again in a moment.",
                false => "nothing tracked by the camera, and no marked target, other vehicle, or gimbal to aim with",
            } });
        };
        resolved
    };

    let Some(aim_at) = aim_point(position, target, roads_under(target).as_deref()) else {
        return json!({ "ok": false, "reason": "the target cannot be intercepted at this speed on this heading" });
    };

    match backend.core_guided(&json!({
        "action": "goto",
        "latitude": aim_at.lat,
        "longitude": aim_at.lon,
        "autodrop": true,
        "targetSource": source.args(),
        "bay": bay,
        "vehicle": id,
    })) {
        Some(Ok(())) => json!({ "ok": true, "started": true }),
        Some(Err(reason)) => json!({ "ok": false, "reason": reason }),
        None => json!({ "ok": false, "reason": "this build has no core vehicle to steer" }),
    }
}

pub fn status_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let Some(id) = vehicle_id(backend).and_then(|v| v.as_u64()) else {
        return json!({ "kind": "object", "class": "AutodropStatus", "state": "idle" });
    };
    let hub = crate::hub::lock();
    let vehicle = hub.vehicle(id as u8);
    let mut status = vehicle
        .and_then(|v| v.autodrop.as_ref())
        .map(State::json)
        .unwrap_or_else(|| json!({ "state": "idle" }));
    status["progress"] = vehicle
        .and_then(|v| v.autodrop_progress.as_ref())
        .filter(|_| status["state"] != "idle")
        .map_or(Value::Null, Progress::json);
    drop(hub);
    status["payload"] = payload_json(backend);
    status["kind"] = json!("object");
    status["class"] = json!("AutodropStatus");
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_a_fork_it_releases_when_every_way_the_car_can_go_still_hits() {
        let north = |metres: f64| crate::geo::ned_to_geo(metres, 0.0, 0.0, (47.0, 8.0, 0.0));
        let east_of = |n: f64, e: f64| crate::geo::ned_to_geo(n, e, 0.0, (47.0, 8.0, 0.0));
        let at = |(lat, lon, _): (f64, f64, f64)| (lat, lon);
        let fork = crate::roads::Network::of(
            vec![
                at(north(0.0)),
                at(north(100.0)),
                at(north(200.0)),
                at(east_of(100.0, 100.0)),
            ],
            vec![vec![1], vec![2, 3], vec![], vec![]],
        );
        let (lat, lon, _) = north(80.0);
        let car = Target {
            position: Point { lat, lon },
            velocity: Velocity {
                north_mps: 5.0,
                east_mps: 0.0,
            },
        };
        let straight = |flown_s: f64| {
            sampled(
                &road_course(car, Some(&fork), flown_s).expect("on the road"),
                flown_s,
            )
        };
        let aimed_at = |flown_s: f64| {
            let there = straight(flown_s);
            (-there.north_m, -there.east_m)
        };
        assert!(
            straight(2.0).junctions > 0,
            "10 m from the fork the old gate already held"
        );
        assert!(
            every_branch_hits(car, Some(&fork), 2.0, aimed_at(2.0)),
            "but the car is still where both roads put it"
        );
        assert!(
            !every_branch_hits(car, Some(&fork), 6.0, aimed_at(6.0)),
            "past the fork, straight on and the right turn put the car in different places"
        );
    }

    #[test]
    fn far_point_on_bearing_keeps_the_direction_and_sets_the_distance() {
        let origin = (47.0, 8.0);
        let from = Point {
            lat: origin.0,
            lon: origin.1,
        };
        let (lat, lon, _) = ned_to_geo(100.0, 0.0, 0.0, (origin.0, origin.1, 0.0));
        let towards = Point { lat, lon };

        let far = far_point_on_bearing(from, towards, 950.0);

        let (n, e, _) = geo_to_ned(far.lat, far.lon, 0.0, (origin.0, origin.1, 0.0));
        assert!(
            (n - 950.0).abs() < 1e-3 && e.abs() < 1e-6,
            "950m due north should stay due north, got n={n} e={e}"
        );
    }

    const ORIGIN: (f64, f64, f64) = (47.0, 8.0, 0.0);

    fn at(north: f64, east: f64) -> Point {
        let (lat, lon, _) = ned_to_geo(north, east, 0.0, ORIGIN);
        Point { lat, lon }
    }

    fn north_east(point: Point) -> (f64, f64) {
        let (north, east, _) = geo_to_ned(point.lat, point.lon, 0.0, ORIGIN);
        (north, east)
    }

    fn corner() -> crate::roads::Network {
        let node = |north: f64, east: f64| {
            let point = at(north, east);
            (point.lat, point.lon)
        };
        crate::roads::Network::of(
            vec![node(0.0, 0.0), node(100.0, 0.0), node(100.0, 300.0)],
            vec![vec![1], vec![0, 2], vec![1]],
        )
    }

    fn hovering_over(point: Point) -> dropsim::Release {
        dropsim::Release {
            position: point,
            altitude_agl_m: 40.0,
            velocity: Velocity::default(),
            climb_mps: 0.0,
        }
    }

    fn northbound() -> Target {
        Target {
            position: at(20.0, 0.0),
            velocity: Velocity {
                north_mps: 8.0,
                east_mps: 0.0,
            },
        }
    }

    #[test]
    fn a_plan_on_a_road_waits_for_the_car_round_the_corner() {
        let roads = corner();
        let release = hovering_over(at(100.0, 60.0));
        let on_road = plan(release, northbound(), Some(&roads));
        assert!(on_road.miss_m < 0.5, "{on_road:?}");
        assert!(on_road.release_in_s > 5.0, "{on_road:?}");
        let (north, east) = north_east(on_road.target_at_impact);
        assert!((north - 100.0).abs() < 1.0 && (east - 60.0).abs() < 1.0);
        let straight = plan(release, northbound(), None);
        assert!(
            straight.miss_m > 50.0,
            "the straight-line guess drives on north: {straight:?}"
        );
        let (lead_north, lead_east) = north_east(lead_now(release, northbound(), Some(&roads)));
        assert!(lead_north > 20.0 && lead_north < 50.0 && lead_east.abs() < 1.0);
    }

    #[test]
    fn the_drone_is_aimed_at_where_the_car_will_be_on_its_road() {
        let roads = corner();
        let drone = at(160.0, 120.0);
        let aim = aim_point(drone, northbound(), Some(&roads)).expect("reachable");
        let (north, east) = north_east(aim);
        let (from_north, from_east) = (north - 160.0, east - 120.0);
        assert!(
            from_north < 0.0,
            "it heads down to the eastbound leg, not north: {north} {east}"
        );
        assert!(
            from_east.abs() < from_north.abs() * 2.0,
            "{from_north} {from_east}"
        );
    }

    #[test]
    fn a_release_waits_out_a_junction_a_fresh_turn_or_a_miss_near_the_car_edge() {
        let fits = Plan {
            release_in_s: 0.0,
            miss_m: 0.5,
            along_m: 0.5,
            across_m: 0.1,
            moving: true,
            junction: false,
            target_at_impact: at(0.0, 0.0),
        };
        assert!(releasable(fits, 0.0));
        assert!(!releasable(
            Plan {
                junction: true,
                ..fits
            },
            0.0
        ));
        assert!(!releasable(fits, TURNED_DEG + 1.0));
        assert!(!releasable(
            Plan {
                across_m: CAR_WIDTH_M / 2.0 - 0.1,
                ..fits
            },
            0.0
        ));
        assert!(
            releasable(
                Plan {
                    along_m: 1.5,
                    miss_m: 1.5,
                    ..fits
                },
                0.0
            ),
            "the car is long"
        );
        assert!(
            !releasable(
                Plan {
                    moving: false,
                    miss_m: 1.5,
                    ..fits
                },
                0.0
            ),
            "parked: no heading"
        );
    }

    #[test]
    fn state_json_names_each_variant() {
        assert_eq!(
            State::Waiting {
                phase: Phase::WaitingForCruise,
                source: TargetSource::Gimbal,
                started_ms: 0,
                due_ms: 0
            }
            .json()["phase"],
            "waitingForCruise"
        );
        assert_eq!(
            State::Fired {
                predicted_miss_m: 1.5,
                bottle: 0,
                impact: Impact::Falling,
                at_ms: 10
            }
            .json()["state"],
            "fired"
        );
        assert_eq!(
            State::Fired {
                predicted_miss_m: 0.4,
                bottle: 0,
                impact: Impact::Seen {
                    miss_m: 1.2,
                    on_target: true
                },
                at_ms: 10
            }
            .json(),
            json!({ "state": "fired", "predictedMissMeters": 0.4, "atMs": 10, "impact": "seen", "estimatedMissMeters": 1.2, "estimatedOnTarget": true })
        );
        assert_eq!(
            State::GaveUp {
                reason: "too far".to_string(),
                at_ms: 10
            }
            .json()["reason"],
            "too far"
        );
    }

    #[test]
    fn target_source_defaults_to_gimbal_and_reads_a_vehicle_id() {
        assert_eq!(TargetSource::of(&json!([])), Some(TargetSource::Gimbal));
        assert_eq!(
            TargetSource::of(&TargetSource::Tracked.args()),
            Some(TargetSource::Tracked),
            "a run resolved to the camera's tracked object hands the tick the same source"
        );
        assert_eq!(
            TargetSource::of(&json!(["gimbal"])),
            Some(TargetSource::Gimbal)
        );
        assert_eq!(
            TargetSource::of(&json!(["manual"])),
            Some(TargetSource::Manual)
        );
        assert_eq!(
            TargetSource::of(&json!(["vehicle", 129])),
            Some(TargetSource::Vehicle(129))
        );
        assert_eq!(
            TargetSource::of(&json!(["vehicle"])),
            None,
            "vehicle needs an id"
        );
        assert_eq!(TargetSource::of(&json!(["vacuum"])), None);
    }

    #[test]
    fn a_vehicle_source_reads_the_live_fix_and_a_manual_source_reads_the_marked_point() {
        let _fixes_lock = gpstarget::TEST_FIXES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut fixes = std::collections::BTreeMap::new();
        fixes.insert(
            129,
            gpstarget::Fix {
                position: Point {
                    lat: 47.4,
                    lon: 8.5,
                },
                ground_speed_mps: 5.0,
                heading_deg: 0.0,
                on_ground: false,
            },
        );
        gpstarget::refresh(fixes);

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

        let here = Point {
            lat: 47.0,
            lon: 8.0,
        };
        let from_vehicle = target_from(&NoBackend, TargetSource::Vehicle(129), here, 10.0);
        assert_eq!(
            from_vehicle.map(|t| t.position),
            Ok(Point {
                lat: 47.4,
                lon: 8.5
            })
        );
        assert_eq!(
            target_from(&NoBackend, TargetSource::Vehicle(250), here, 10.0),
            Err("no live telemetry recorded yet for that vehicle id"),
            "an id the snapshot never carried is refused by name, not silently answered as 129's"
        );
        assert_eq!(
            target_from(&NoBackend, TargetSource::Manual, here, 10.0),
            Err("no target has been marked on the map yet"),
            "NoBackend reports no vehicle.manualTarget fact"
        );
    }

    struct EmptyBayBackend;
    impl Backend for EmptyBayBackend {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.id" => json!({"kind": "value", "value": 1}).to_string(),
                "vehicle.payloadSetup" => {
                    json!({"kind": "value", "value": {"empty": [0]}}).to_string()
                }
                _ => "{}".to_string(),
            }
        }
        fn get_fields(&self, _path: &str, _fields: &str) -> String {
            "{}".to_string()
        }
        fn set(&self, _path: &str, _value: &str) -> String {
            "{}".to_string()
        }
        fn invoke(&self, _path: &str, _args: &str) -> String {
            "{}".to_string()
        }
        fn watch(&self, _paths: &[String]) {}
    }

    #[test]
    fn a_run_is_refused_up_front_when_the_payload_bay_is_already_empty() {
        let answer = run(&EmptyBayBackend, "[]");
        assert_eq!(
            (answer["ok"].as_bool(), answer["reason"].as_str()),
            (Some(false), Some("No bottles left")),
            "an empty bay should refuse before ever steering the vehicle, not fly the course and fire nothing: {answer}"
        );
    }

    #[test]
    fn it_drops_the_next_loaded_bay_and_counts_what_is_left() {
        assert_eq!(next_bay(1, &[]), Some((0, 1)), "one gripper is bay 0");
        assert_eq!(
            next_bay(1, &[0]),
            None,
            "and once it has dropped there is nothing left"
        );
        assert_eq!(next_bay(4, &[]), Some((1, 4)));
        assert_eq!(
            next_bay(4, &[1, 2]),
            Some((3, 2)),
            "bays 1 and 2 are gone: bay 3, two loaded"
        );
        assert_eq!(next_bay(4, &[1, 2, 3, 4]), None);
    }

    #[test]
    fn the_target_is_named_the_way_a_pilot_would_say_it() {
        assert_eq!(label(TargetSource::Tracked, Some("Truck")), "car");
        assert_eq!(label(TargetSource::Tracked, Some("person")), "person");
        assert_eq!(label(TargetSource::Tracked, None), "target");
        assert_eq!(label(TargetSource::Vehicle(129), None), "vehicle 129");
        assert_eq!(
            label(TargetSource::Tracked, Some("tracked")),
            "target",
            "the device's visual tracker follows a box, not a class"
        );
        assert_eq!(label(TargetSource::Manual, Some("car")), "marked point");
    }

    #[test]
    fn the_fallback_order_prefers_the_camera_s_tracked_object_then_manual_then_a_lone_vehicle_then_the_gimbal()
     {
        let _fixes_lock = gpstarget::TEST_FIXES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        gpstarget::refresh(std::collections::BTreeMap::new());
        assert_eq!(
            TargetSource::fallback_order(Some(7)).collect::<Vec<_>>(),
            vec![
                TargetSource::Tracked,
                TargetSource::Manual,
                TargetSource::Gimbal
            ],
            "no other vehicle is known - the chain skips straight from manual to gimbal"
        );

        let mut fixes = std::collections::BTreeMap::new();
        fixes.insert(
            129,
            gpstarget::Fix {
                position: Point {
                    lat: 47.4,
                    lon: 8.5,
                },
                ground_speed_mps: 5.0,
                heading_deg: 0.0,
                on_ground: false,
            },
        );
        gpstarget::refresh(fixes.clone());
        assert_eq!(
            TargetSource::fallback_order(Some(7)).collect::<Vec<_>>(),
            vec![
                TargetSource::Tracked,
                TargetSource::Manual,
                TargetSource::Vehicle(129),
                TargetSource::Gimbal
            ]
        );

        fixes.insert(
            130,
            gpstarget::Fix {
                position: Point { lat: 0.0, lon: 0.0 },
                ground_speed_mps: 0.0,
                heading_deg: 0.0,
                on_ground: false,
            },
        );
        gpstarget::refresh(fixes);
        assert_eq!(
            TargetSource::fallback_order(Some(7)).collect::<Vec<_>>(),
            vec![
                TargetSource::Tracked,
                TargetSource::Manual,
                TargetSource::Gimbal
            ],
            "two other vehicles is ambiguous, so the vehicle step drops out entirely"
        );
    }

    struct FakeBackend {
        manual_target: Option<(f64, f64)>,
    }

    impl Backend for FakeBackend {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.id" => "{\"value\": 7}".to_string(),
                "vehicle.coordinate" => "{\"latitude\": 47.0, \"longitude\": 8.0}".to_string(),
                "vehicle.altitudeRelative" => "{\"rawValue\": 20.0}".to_string(),
                "vehicle.manualTarget" => self
                    .manual_target
                    .map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon }).to_string())
                    .unwrap_or_default(),
                _ => String::new(),
            }
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
    fn triggering_with_no_source_refuses_only_once_manual_vehicle_and_gimbal_all_fail() {
        let _fixes_lock = gpstarget::TEST_FIXES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        gpstarget::refresh(std::collections::BTreeMap::new());
        let answer = run(
            &FakeBackend {
                manual_target: None,
            },
            "[]",
        );
        assert_eq!(answer["ok"], false);
        assert_eq!(
            answer["reason"],
            "nothing tracked by the camera, and no marked target, other vehicle, or gimbal to aim with"
        );
    }

    #[test]
    fn a_manually_marked_target_lets_a_no_source_trigger_run_with_no_gimbal_at_all() {
        let _fixes_lock = gpstarget::TEST_FIXES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        gpstarget::refresh(std::collections::BTreeMap::new());
        let answer = run(
            &FakeBackend {
                manual_target: Some((47.001, 8.0)),
            },
            "[]",
        );
        assert_eq!(
            answer["reason"], "this build has no core vehicle to steer",
            "it must have resolved a target and reached the core_guided call, not been refused for lack of a gimbal: {answer}"
        );
    }

    #[test]
    fn while_a_tracker_holds_a_lock_a_run_only_ever_chases_what_it_tracks() {
        let _fixes = gpstarget::TEST_FIXES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        gpstarget::refresh(std::collections::BTreeMap::from([(
            129,
            gpstarget::Fix {
                position: Point {
                    lat: 47.4,
                    lon: 8.5,
                },
                ground_speed_mps: 5.0,
                heading_deg: 0.0,
                on_ground: true,
            },
        )]));
        assert_eq!(start_order(Some(7), true), vec![TargetSource::Tracked]);
        assert!(
            start_order(Some(7), false).contains(&TargetSource::Vehicle(129)),
            "without a lock the other vehicle stays a fallback"
        );
    }

    #[test]
    fn a_release_needs_the_target_seen_in_the_last_300_ms() {
        assert!(sighted(Some((0, None))) && sighted(Some((FRESH_SIGHTING_MS, None))));
        assert!(
            !sighted(Some((FRESH_SIGHTING_MS + 1, Some(100)))),
            "a 1.5 s coast is a prediction, not a sighting"
        );
        assert!(!sighted(None), "never seen");
        assert!(
            sighted(Some((600, Some(250)))) && !sighted(Some((650, Some(250)))),
            "a 4 frame-a-second detector gets 2.5 frames of slack, 625 ms"
        );
        assert!(
            sighted(Some((300, Some(100)))) && !sighted(Some((320, Some(100)))),
            "a 10 frame-a-second detector keeps the 300 ms floor"
        );
        assert_eq!(
            sighting_window_ms(Some(1_000)),
            MAX_FRESH_SIGHTING_MS,
            "a very slow detector never gets more than 700 ms"
        );
    }
}
