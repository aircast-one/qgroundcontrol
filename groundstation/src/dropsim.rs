use serde_json::{Value, json};

use crate::geo::{geo_to_ned, ned_to_geo};
use crate::router::Backend;

const GRAVITY_MPS2: f64 = 9.81;
const AIR_DENSITY_KGM3: f64 = 1.225;
const FALL_STEP_S: f64 = 0.005;
const FALL_LIMIT_S: f64 = 120.0;
pub const PAYLOAD_MASS_SETTING: &str = "settings.appSettings.autodropPayloadMassG";
pub const PAYLOAD_DRAG_AREA_SETTING: &str = "settings.appSettings.autodropPayloadDragAreaCm2";
pub const WIND_MANUAL_SETTING: &str = "settings.appSettings.autodropWindManual";
pub const WIND_SPEED_SETTING: &str = "settings.appSettings.autodropWindSpeedMps";
pub const WIND_FROM_SETTING: &str = "settings.appSettings.autodropWindFromDeg";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Velocity {
    pub north_mps: f64,
    pub east_mps: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Release {
    pub position: Point,
    pub altitude_agl_m: f64,
    pub velocity: Velocity,
    pub climb_mps: f64,
}

pub fn fall_time_s(altitude_agl_m: f64, climb_mps: f64) -> f64 {
    let h = altitude_agl_m.max(0.0);
    (climb_mps + (climb_mps * climb_mps + 2.0 * GRAVITY_MPS2 * h).sqrt()) / GRAVITY_MPS2
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Payload {
    pub mass_kg: f64,
    pub drag_area_m2: f64,
}

impl Payload {
    fn drag_per_mass(self) -> f64 {
        match self.mass_kg > 0.0 && self.drag_area_m2 > 0.0 {
            true => 0.5 * AIR_DENSITY_KGM3 * self.drag_area_m2 / self.mass_kg,
            false => 0.0,
        }
    }

    pub fn terminal_speed_mps(self) -> f64 {
        match self.drag_per_mass() {
            0.0 => f64::INFINITY,
            k => (GRAVITY_MPS2 / k).sqrt(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Air {
    pub payload: Payload,
    pub wind: Velocity,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fall {
    pub seconds: f64,
    pub north_m: f64,
    pub east_m: f64,
}

type State = [f64; 6];

fn slope(s: State, k: f64, wind: Velocity) -> State {
    let rel = [s[3] - wind.north_mps, s[4] - wind.east_mps, s[5]];
    let speed = rel.iter().map(|v| v * v).sum::<f64>().sqrt();
    [
        s[3],
        s[4],
        s[5],
        -k * speed * rel[0],
        -k * speed * rel[1],
        -GRAVITY_MPS2 - k * speed * rel[2],
    ]
}

fn stepped(s: State, k: f64, wind: Velocity) -> State {
    let shifted =
        |from: State, by: State, h: f64| -> State { std::array::from_fn(|i| from[i] + by[i] * h) };
    let k1 = slope(s, k, wind);
    let k2 = slope(shifted(s, k1, FALL_STEP_S / 2.0), k, wind);
    let k3 = slope(shifted(s, k2, FALL_STEP_S / 2.0), k, wind);
    let k4 = slope(shifted(s, k3, FALL_STEP_S), k, wind);
    std::array::from_fn(|i| s[i] + FALL_STEP_S / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]))
}

pub fn trajectory(
    velocity: (f64, f64, f64),
    air: Air,
) -> impl Iterator<Item = (f64, [f64; 3])> + Clone {
    let k = air.payload.drag_per_mass();
    let start: State = [0.0, 0.0, 0.0, velocity.0, velocity.1, velocity.2];
    std::iter::successors(Some((0.0, start)), move |&(t, s)| {
        Some((t + FALL_STEP_S, stepped(s, k, air.wind)))
    })
    .take((FALL_LIMIT_S / FALL_STEP_S) as usize)
    .map(|(t, s)| (t, [s[0], s[1], s[2]]))
}

pub fn fall(release: Release, air: Air) -> Fall {
    let drop_m = release.altitude_agl_m.max(0.0);
    if air.payload.drag_per_mass() == 0.0 {
        let seconds = fall_time_s(drop_m, release.climb_mps);
        return Fall {
            seconds,
            north_m: release.velocity.north_mps * seconds,
            east_m: release.velocity.east_mps * seconds,
        };
    }
    let path = trajectory(
        (
            release.velocity.north_mps,
            release.velocity.east_mps,
            release.climb_mps,
        ),
        air,
    );
    let ((t0, a), (_, b)) = path
        .clone()
        .zip(path.skip(1))
        .find(|(_, (_, b))| b[2] <= -drop_m)
        .unwrap_or(((0.0, [0.0; 3]), (0.0, [0.0; 3])));
    let share = match a[2] - b[2] {
        0.0 => 0.0,
        dz => (a[2] + drop_m) / dz,
    };
    Fall {
        seconds: t0 + share * FALL_STEP_S,
        north_m: a[0] + share * (b[0] - a[0]),
        east_m: a[1] + share * (b[1] - a[1]),
    }
}

fn number_setting(path: &str) -> f64 {
    crate::settingsstore::raw_setting(path)
        .and_then(|v| v.as_f64())
        .filter(|v| v.is_finite())
        .unwrap_or(0.0)
}

pub fn calibrated_payload() -> Payload {
    Payload {
        mass_kg: number_setting(PAYLOAD_MASS_SETTING) / 1000.0,
        drag_area_m2: number_setting(PAYLOAD_DRAG_AREA_SETTING) / 10_000.0,
    }
}

pub fn blowing(speed_mps: f64, from_deg: f64) -> Velocity {
    let (sin, cos) = from_deg.to_radians().sin_cos();
    Velocity {
        north_mps: -speed_mps * cos,
        east_mps: -speed_mps * sin,
    }
}

static WIND: std::sync::Mutex<Velocity> = std::sync::Mutex::new(Velocity {
    north_mps: 0.0,
    east_mps: 0.0,
});

pub fn report_wind(wind: Velocity) {
    *WIND
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = wind;
}

static SIM_PAYLOAD: std::sync::Mutex<Option<Payload>> = std::sync::Mutex::new(None);

pub fn set_sim_payload(payload: Payload) {
    *SIM_PAYLOAD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(payload);
}

pub fn falling_air() -> Air {
    let sim = *SIM_PAYLOAD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Air {
        payload: sim.unwrap_or_else(calibrated_payload),
        ..air()
    }
}

pub fn air() -> Air {
    let manual = crate::settingsstore::raw_setting(WIND_MANUAL_SETTING)
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Air {
        payload: calibrated_payload(),
        wind: match manual {
            true => blowing(
                number_setting(WIND_SPEED_SETTING),
                number_setting(WIND_FROM_SETTING),
            ),
            false => *WIND
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    pub position: Point,
    pub velocity: Velocity,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drop {
    pub fall_s: f64,
    pub impact: Point,
    pub target_at_impact: Point,
    pub miss_north_m: f64,
    pub miss_east_m: f64,
    pub miss_m: f64,
}

impl Drop {
    pub fn json(&self) -> Value {
        json!({
            "fallSeconds": self.fall_s,
            "impact": {"lat": self.impact.lat, "lon": self.impact.lon},
            "targetAtImpact": {"lat": self.target_at_impact.lat, "lon": self.target_at_impact.lon},
            "missNorthMeters": self.miss_north_m,
            "missEastMeters": self.miss_east_m,
            "missMeters": self.miss_m,
        })
    }
}

pub fn simulate(release: Release, target: Target) -> Drop {
    simulate_in(release, target, air())
}

pub fn simulate_in(release: Release, target: Target, air: Air) -> Drop {
    let carried = fall(release, air);
    let fall_s = carried.seconds;
    let origin = (release.position.lat, release.position.lon, 0.0);

    let impact_north_m = carried.north_m;
    let impact_east_m = carried.east_m;
    let (impact_lat, impact_lon, _) = ned_to_geo(impact_north_m, impact_east_m, 0.0, origin);

    let (target_north0_m, target_east0_m, _) =
        geo_to_ned(target.position.lat, target.position.lon, 0.0, origin);
    let target_north_m = target_north0_m + target.velocity.north_mps * fall_s;
    let target_east_m = target_east0_m + target.velocity.east_mps * fall_s;
    let (target_lat, target_lon, _) = ned_to_geo(target_north_m, target_east_m, 0.0, origin);

    let miss_north_m = target_north_m - impact_north_m;
    let miss_east_m = target_east_m - impact_east_m;

    Drop {
        fall_s,
        impact: Point {
            lat: impact_lat,
            lon: impact_lon,
        },
        target_at_impact: Point {
            lat: target_lat,
            lon: target_lon,
        },
        miss_north_m,
        miss_east_m,
        miss_m: miss_north_m.hypot(miss_east_m),
    }
}

pub fn impact(release: Release) -> Point {
    simulate(
        release,
        Target {
            position: release.position,
            velocity: Velocity::default(),
        },
    )
    .impact
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lead {
    pub release_in_s: f64,
    pub fall_s: f64,
    pub miss_m: f64,
    pub release_point: Point,
    pub impact: Point,
    pub target_at_impact: Point,
}

impl Lead {
    pub fn json(&self) -> Value {
        json!({
            "releaseInSeconds": self.release_in_s,
            "fallSeconds": self.fall_s,
            "missMeters": self.miss_m,
            "releasePoint": {"lat": self.release_point.lat, "lon": self.release_point.lon},
            "impact": {"lat": self.impact.lat, "lon": self.impact.lon},
            "targetAtImpact": {"lat": self.target_at_impact.lat, "lon": self.target_at_impact.lon},
        })
    }
}

pub fn lead(release: Release, target: Target) -> Lead {
    lead_in(release, target, air())
}

pub fn lead_in(release: Release, target: Target, air: Air) -> Lead {
    let carried = fall(release, air);
    let fall_s = carried.seconds;
    let origin = (release.position.lat, release.position.lon, 0.0);

    let (target_north0_m, target_east0_m, _) =
        geo_to_ned(target.position.lat, target.position.lon, 0.0, origin);
    let v_d = (release.velocity.north_mps, release.velocity.east_mps);
    let v_t = (target.velocity.north_mps, target.velocity.east_mps);
    let w = (v_t.0 - v_d.0, v_t.1 - v_d.1);
    let gap = (
        target_north0_m + v_t.0 * fall_s - carried.north_m,
        target_east0_m + v_t.1 * fall_s - carried.east_m,
    );
    let w_dot_w = w.0 * w.0 + w.1 * w.1;
    let release_in_s = match w_dot_w > 1e-9 {
        true => -(gap.0 * w.0 + gap.1 * w.1) / w_dot_w,
        false => -fall_s,
    };
    let s = release_in_s + fall_s;
    let miss = (gap.0 + w.0 * release_in_s, gap.1 + w.1 * release_in_s);
    let release_offset = (v_d.0 * release_in_s, v_d.1 * release_in_s);
    let impact_offset = (
        release_offset.0 + carried.north_m,
        release_offset.1 + carried.east_m,
    );
    let target_offset = (target_north0_m + v_t.0 * s, target_east0_m + v_t.1 * s);

    let point = |offset: (f64, f64)| {
        let (lat, lon, _) = ned_to_geo(offset.0, offset.1, 0.0, origin);
        Point { lat, lon }
    };

    Lead {
        release_in_s,
        fall_s,
        miss_m: miss.0.hypot(miss.1),
        release_point: point(release_offset),
        impact: point(impact_offset),
        target_at_impact: point(target_offset),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Intercept {
    pub heading_deg: f64,
    pub speed_mps: f64,
    pub time_to_intercept_s: f64,
    pub lead_point: Point,
}

impl Intercept {
    pub fn json(&self) -> Value {
        json!({
            "headingDegrees": self.heading_deg,
            "speedMetersPerSecond": self.speed_mps,
            "timeToInterceptSeconds": self.time_to_intercept_s,
            "leadPoint": {"lat": self.lead_point.lat, "lon": self.lead_point.lon},
        })
    }
}

pub fn intercept(drone_position: Point, speed_mps: f64, target: Target) -> Option<Intercept> {
    let origin = (drone_position.lat, drone_position.lon, 0.0);
    let (dn, de, _) = geo_to_ned(target.position.lat, target.position.lon, 0.0, origin);
    let (vn, ve) = (target.velocity.north_mps, target.velocity.east_mps);

    let a = vn * vn + ve * ve - speed_mps * speed_mps;
    let b = 2.0 * (dn * vn + de * ve);
    let c = dn * dn + de * de;

    let t = if a.abs() > 1e-9 {
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            return None;
        }
        let sq = disc.sqrt();
        [(-b + sq) / (2.0 * a), (-b - sq) / (2.0 * a)]
            .into_iter()
            .filter(|t| *t >= 0.0)
            .fold(f64::INFINITY, f64::min)
    } else if b.abs() > 1e-9 {
        -c / b
    } else if c < 1e-9 {
        0.0
    } else {
        f64::INFINITY
    };
    if !t.is_finite() {
        return None;
    }

    let lead_n = dn + vn * t;
    let lead_e = de + ve * t;
    let (lead_lat, lead_lon, _) = ned_to_geo(lead_n, lead_e, 0.0, origin);

    Some(Intercept {
        heading_deg: lead_e.atan2(lead_n).to_degrees().rem_euclid(360.0),
        speed_mps,
        time_to_intercept_s: t,
        lead_point: Point {
            lat: lead_lat,
            lon: lead_lon,
        },
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

const VIEW_USAGE: &str = "this needs ten numbers: release latitude, longitude, altitude AGL, north speed, east speed and climb speed (positive is up), then target latitude, longitude, north speed and east speed";

fn release_and_target(v: &[f64]) -> (Release, Target) {
    (
        Release {
            position: Point {
                lat: v[0],
                lon: v[1],
            },
            altitude_agl_m: v[2],
            velocity: Velocity {
                north_mps: v[3],
                east_mps: v[4],
            },
            climb_mps: v[5],
        },
        Target {
            position: Point {
                lat: v[6],
                lon: v[7],
            },
            velocity: Velocity {
                north_mps: v[8],
                east_mps: v[9],
            },
        },
    )
}

pub fn drop_sim_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(v) = numbers(args, 10) else {
        return crate::read::refused(VIEW_USAGE);
    };
    let (release, target) = release_and_target(&v);
    simulate(release, target).json()
}

pub fn drop_lead_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(v) = numbers(args, 10) else {
        return crate::read::refused(VIEW_USAGE);
    };
    let (release, target) = release_and_target(&v);
    lead(release, target).json()
}

pub fn intercept_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(v) = numbers(args, 7) else {
        return crate::read::refused(
            "this needs seven numbers: drone latitude, longitude, speed, then target latitude, longitude, north speed and east speed",
        );
    };
    let drone = Point {
        lat: v[0],
        lon: v[1],
    };
    let target = Target {
        position: Point {
            lat: v[3],
            lon: v[4],
        },
        velocity: Velocity {
            north_mps: v[5],
            east_mps: v[6],
        },
    };
    match intercept(drone, v[2], target) {
        Some(solved) => solved.json(),
        None => {
            json!({ "kind": "object", "reachable": false, "reason": "the target cannot be intercepted at this speed on this heading" })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offset(origin: (f64, f64), north_m: f64, east_m: f64) -> Point {
        let (lat, lon, _) = ned_to_geo(north_m, east_m, 0.0, (origin.0, origin.1, 0.0));
        Point { lat, lon }
    }

    #[test]
    fn a_stationary_target_already_at_the_impact_point_is_hit() {
        let origin = (47.0, 8.0);
        let release = Release {
            position: Point {
                lat: origin.0,
                lon: origin.1,
            },
            altitude_agl_m: 19.6133,
            climb_mps: 0.0,
            velocity: Velocity {
                north_mps: 10.0,
                east_mps: 0.0,
            },
        };
        let target = Target {
            position: offset(origin, 20.0, 0.0),
            velocity: Velocity::default(),
        };

        let drop = simulate(release, target);

        assert!(
            (drop.fall_s - 2.0).abs() < 1e-3,
            "a 19.6133m drop should take ~2s to fall, got {}",
            drop.fall_s
        );
        assert!(
            drop.miss_m < 0.01,
            "a target sitting exactly at the impact point should show near-zero miss, got {}",
            drop.miss_m
        );
    }

    #[test]
    fn a_target_drifting_sideways_during_the_fall_is_missed_by_its_own_drift() {
        let origin = (47.0, 8.0);
        let release = Release {
            position: Point {
                lat: origin.0,
                lon: origin.1,
            },
            altitude_agl_m: 19.6133,
            climb_mps: 0.0,
            velocity: Velocity {
                north_mps: 10.0,
                east_mps: 0.0,
            },
        };
        let target = Target {
            position: offset(origin, 20.0, 0.0),
            velocity: Velocity {
                north_mps: 0.0,
                east_mps: 5.0,
            },
        };

        let drop = simulate(release, target);

        assert!(
            (drop.miss_m - 10.0).abs() < 0.01,
            "5 m/s of crosswise target drift over a 2s fall should miss by 10m, got {}",
            drop.miss_m
        );
    }

    #[test]
    fn flying_straight_at_a_stationary_target_finds_the_release_time_that_hits_it() {
        let origin = (47.0, 8.0);
        let release = Release {
            position: Point {
                lat: origin.0,
                lon: origin.1,
            },
            altitude_agl_m: 19.6133,
            climb_mps: 0.0,
            velocity: Velocity {
                north_mps: 10.0,
                east_mps: 0.0,
            },
        };
        let target = Target {
            position: offset(origin, 100.0, 0.0),
            velocity: Velocity::default(),
        };

        let solved = lead(release, target);

        assert!(
            (solved.release_in_s - 8.0).abs() < 1e-3,
            "100m at 10 m/s minus the 2s fall is 8s out, got {}",
            solved.release_in_s
        );
        assert!(
            solved.miss_m < 1e-3,
            "flying straight at a stationary target has an exact solution, got {}",
            solved.miss_m
        );
    }

    #[test]
    fn a_crossing_target_is_missed_by_the_closest_this_heading_can_manage() {
        let origin = (47.0, 8.0);
        let release = Release {
            position: Point {
                lat: origin.0,
                lon: origin.1,
            },
            altitude_agl_m: 19.6133,
            climb_mps: 0.0,
            velocity: Velocity {
                north_mps: 10.0,
                east_mps: 0.0,
            },
        };
        let target = Target {
            position: offset(origin, 100.0, 0.0),
            velocity: Velocity {
                north_mps: 0.0,
                east_mps: 5.0,
            },
        };

        let solved = lead(release, target);

        assert!(
            (solved.release_in_s - 6.0).abs() < 1e-3,
            "got {}",
            solved.release_in_s
        );
        assert!(
            (solved.miss_m - 2000f64.sqrt()).abs() < 1e-2,
            "a due-north course can't converge on an eastbound target; this is the closest point of approach, got {}",
            solved.miss_m
        );
    }

    #[test]
    fn two_courses_at_the_same_velocity_never_converge_so_now_is_as_good_as_any_time() {
        let origin = (47.0, 8.0);
        let release = Release {
            position: Point {
                lat: origin.0,
                lon: origin.1,
            },
            altitude_agl_m: 19.6133,
            climb_mps: 0.0,
            velocity: Velocity {
                north_mps: 10.0,
                east_mps: 0.0,
            },
        };
        let target = Target {
            position: offset(origin, 0.0, 30.0),
            velocity: Velocity {
                north_mps: 10.0,
                east_mps: 0.0,
            },
        };

        let solved = lead(release, target);

        assert!(
            (solved.release_in_s - (-solved.fall_s)).abs() < 1e-9,
            "with matched velocities the gap never closes, so there is no better time than now minus the time already spent falling, got {}",
            solved.release_in_s
        );
        assert!(
            (solved.miss_m - 30.0).abs() < 1e-6,
            "the separation this course started with is the separation it keeps, got {}",
            solved.miss_m
        );
    }

    #[test]
    fn the_fall_time_satisfies_the_height_equation_whether_climbing_or_descending() {
        for (h, climb) in [(50.0, 8.0), (50.0, -8.0), (20.0, 0.0), (0.0, 12.0)] {
            let t = fall_time_s(h, climb);
            let height_at_landing = h + climb * t - 0.5 * GRAVITY_MPS2 * t * t;
            assert!(
                height_at_landing.abs() < 1e-6,
                "h={h} climb={climb}: at t={t}s the kinematic height should be ~0, got {height_at_landing}"
            );
        }
    }

    #[test]
    fn climbing_at_release_takes_longer_to_land_than_descending_from_the_same_height() {
        let level = fall_time_s(50.0, 0.0);
        let climbing = fall_time_s(50.0, 8.0);
        let descending = fall_time_s(50.0, -8.0);
        assert!(
            climbing > level,
            "a payload released while climbing should hang longer, got {climbing} vs level {level}"
        );
        assert!(
            descending < level,
            "a payload released while descending should land sooner, got {descending} vs level {level}"
        );
    }

    #[test]
    fn simulate_wires_the_release_climb_rate_into_the_fall_time() {
        let origin = (47.0, 8.0);
        let level = Release {
            position: Point {
                lat: origin.0,
                lon: origin.1,
            },
            altitude_agl_m: 50.0,
            velocity: Velocity::default(),
            climb_mps: 0.0,
        };
        let climbing = Release {
            climb_mps: 8.0,
            ..level
        };
        let target = Target {
            position: offset(origin, 0.0, 0.0),
            velocity: Velocity::default(),
        };

        assert!(
            (simulate(climbing, target).fall_s - fall_time_s(50.0, 8.0)).abs() < 1e-9,
            "simulate() should use the same fall time a direct call to fall_time_s computes"
        );
        assert!(
            simulate(level, target).fall_s < simulate(climbing, target).fall_s,
            "a climbing release should fall for longer than a level one from the same altitude"
        );
    }

    #[test]
    fn a_stationary_drone_leads_a_target_moving_directly_away_within_reach() {
        let origin = (47.0, 8.0);
        let drone = Point {
            lat: origin.0,
            lon: origin.1,
        };
        let target = Target {
            position: offset(origin, 100.0, 0.0),
            velocity: Velocity {
                north_mps: 3.0,
                east_mps: 0.0,
            },
        };

        let solved = intercept(drone, 10.0, target).expect("3 m/s is outrun by a 10 m/s chaser");

        assert!(
            (solved.time_to_intercept_s - 100.0 / 7.0).abs() < 1e-6,
            "closing speed is 10-3=7 m/s over the initial 100m gap, got {}",
            solved.time_to_intercept_s
        );
        assert!(
            solved.heading_deg.min(360.0 - solved.heading_deg) < 1e-6,
            "the target never leaves the north line, so the lead point is still due north, got {}",
            solved.heading_deg
        );
        let closing = solved.speed_mps * solved.time_to_intercept_s;
        let (lead_n, _, _) = geo_to_ned(
            solved.lead_point.lat,
            solved.lead_point.lon,
            0.0,
            (origin.0, origin.1, 0.0),
        );
        assert!(
            (closing - lead_n).abs() < 1e-6,
            "flying speed_mps for time_to_intercept_s should land exactly on the lead point"
        );
    }

    #[test]
    fn a_target_outrunning_the_chase_speed_while_moving_away_has_no_intercept() {
        let origin = (47.0, 8.0);
        let drone = Point {
            lat: origin.0,
            lon: origin.1,
        };
        let target = Target {
            position: offset(origin, 100.0, 0.0),
            velocity: Velocity {
                north_mps: 15.0,
                east_mps: 0.0,
            },
        };

        assert!(
            intercept(drone, 10.0, target).is_none(),
            "a target already ahead and faster than the chaser can never be caught"
        );
    }

    #[test]
    fn a_target_already_at_the_drone_is_intercepted_instantly() {
        let origin = (47.0, 8.0);
        let drone = Point {
            lat: origin.0,
            lon: origin.1,
        };
        let target = Target {
            position: drone,
            velocity: Velocity {
                north_mps: 4.0,
                east_mps: 0.0,
            },
        };

        let solved = intercept(drone, 10.0, target).expect("already there");
        assert!(solved.time_to_intercept_s.abs() < 1e-9);
    }

    #[test]
    fn a_crossing_target_is_led_to_a_point_off_its_current_bearing() {
        let origin = (47.0, 8.0);
        let drone = Point {
            lat: origin.0,
            lon: origin.1,
        };
        let target = Target {
            position: offset(origin, 100.0, 0.0),
            velocity: Velocity {
                north_mps: 0.0,
                east_mps: 5.0,
            },
        };

        let solved = intercept(drone, 10.0, target)
            .expect("5 m/s crossing is within a 10 m/s chaser's reach");

        assert!(
            solved.heading_deg > 1.0,
            "a target crossing east of a straight-north bearing has to be led east of it, got {}",
            solved.heading_deg
        );
        let closing = solved.speed_mps * solved.time_to_intercept_s;
        let (lead_n, lead_e, _) = geo_to_ned(
            solved.lead_point.lat,
            solved.lead_point.lon,
            0.0,
            (origin.0, origin.1, 0.0),
        );
        assert!(
            (closing - lead_n.hypot(lead_e)).abs() < 1e-6,
            "flying speed_mps for time_to_intercept_s should land exactly on the lead point"
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
    fn the_view_refuses_anything_other_than_ten_numbers() {
        let answer = drop_sim_view(&NoBackend, &["1".to_string(), "2".to_string()]);
        assert_eq!(answer["kind"], "null");
        assert!(answer["reason"].as_str().unwrap().contains("ten numbers"));
    }

    #[test]
    fn the_view_reports_the_same_miss_as_calling_simulate_directly() {
        let origin = (47.0, 8.0);
        let target = offset(origin, 20.0, 0.0);
        let args: Vec<String> = [
            origin.0, origin.1, 19.6133, 10.0, 0.0, 0.0, target.lat, target.lon, 0.0, 5.0,
        ]
        .iter()
        .map(|n| n.to_string())
        .collect();

        let answer = drop_sim_view(&NoBackend, &args);

        assert!(
            (answer["missMeters"].as_f64().unwrap() - 10.0).abs() < 0.01,
            "{answer}"
        );
    }

    fn released(altitude_agl_m: f64, north_mps: f64, climb_mps: f64) -> Release {
        Release {
            position: Point {
                lat: 47.0,
                lon: 8.0,
            },
            altitude_agl_m,
            velocity: Velocity {
                north_mps,
                east_mps: 0.0,
            },
            climb_mps,
        }
    }

    fn payload(mass_kg: f64, drag_area_m2: f64) -> Air {
        Air {
            payload: Payload {
                mass_kg,
                drag_area_m2,
            },
            wind: Velocity::default(),
        }
    }

    #[test]
    fn the_integrator_with_next_to_no_drag_lands_where_the_closed_form_does() {
        let release = released(40.0, 10.0, 1.5);
        let integrated = fall(release, payload(1000.0, 1e-6));
        let closed = fall(release, Air::default());
        assert!(
            (integrated.north_m - closed.north_m).abs() < 0.01
                && (integrated.seconds - closed.seconds).abs() < 0.001,
            "{integrated:?} vs {closed:?}"
        );
    }

    #[test]
    fn a_vertical_drop_with_drag_matches_the_exact_quadratic_drag_solution() {
        let air = payload(0.5, 0.01);
        let terminal = air.payload.terminal_speed_mps();
        let height = 60.0;
        let exact = terminal / GRAVITY_MPS2
            * (height * GRAVITY_MPS2 / (terminal * terminal))
                .exp()
                .acosh();
        let integrated = fall(released(height, 0.0, 0.0), air).seconds;
        assert!(
            (integrated - exact).abs() < 0.002,
            "terminal {terminal:.1} m/s: integrated {integrated:.4} s, exact {exact:.4} s"
        );
        assert!(
            integrated > fall_time_s(height, 0.0),
            "drag lengthens the fall"
        );
    }

    #[test]
    fn drag_shortens_how_far_a_moving_release_carries() {
        let release = released(40.0, 10.0, 0.0);
        let dragged = fall(release, payload(0.5, 0.01));
        assert!(
            dragged.north_m < 10.0 * fall_time_s(40.0, 0.0) - 1.0,
            "{dragged:?}"
        );
    }

    #[test]
    fn wind_carries_a_light_payload_with_it_and_barely_moves_a_dense_one() {
        let windy = |air: Air| Air {
            wind: blowing(5.0, 270.0),
            ..air
        };
        let release = released(40.0, 0.0, 0.0);
        let light = windy(payload(0.05, 0.05));
        let drifted = fall(release, light);
        assert!(
            drifted.east_m > 0.8 * 5.0 * drifted.seconds,
            "a payload with a {:.1} m/s terminal speed rides a 5 m/s west wind east: {drifted:?}",
            light.payload.terminal_speed_mps()
        );
        let dense = fall(release, windy(payload(5.0, 0.002)));
        assert!(
            dense.east_m > 0.0 && dense.east_m < 0.1 * 5.0 * dense.seconds,
            "{dense:?}"
        );
    }

    #[test]
    fn a_lead_with_drag_releases_when_the_carried_impact_meets_the_target() {
        let release = released(40.0, 10.0, 0.0);
        let air = payload(0.5, 0.01);
        let target = Target {
            position: offset((47.0, 8.0), 120.0, 0.0),
            velocity: Velocity {
                north_mps: 5.0,
                east_mps: 0.0,
            },
        };
        let lead = lead_in(release, target, air);
        let later = Release {
            position: offset((47.0, 8.0), 10.0 * lead.release_in_s, 0.0),
            ..release
        };
        let moved = Target {
            position: offset((47.0, 8.0), 120.0 + 5.0 * lead.release_in_s, 0.0),
            ..target
        };
        assert!(lead.miss_m < 0.01, "{lead:?}");
        assert!(simulate_in(later, moved, air).miss_m < 0.05);
    }
}
