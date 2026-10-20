use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::dropsim::{self, Point, Release, Target, Velocity};
use crate::geo::{geo_to_ned, ned_to_geo};
use crate::gimbalgeo;
use crate::read::flag;
use crate::router::Backend;

const COMMAND_LATENCY_S: f64 = 0.15;
const MAX_RUN_S: f64 = 90.0;
const AUTODROP_TICK_MS: u64 = 150;
const GUIDED_GOTO_LIMIT_M: f64 = 950.0;
// ponytail: this mock's observed ArduCopter guided-mode cruise speed (measured live,
// repeatedly, this session). No cleaner source found: tuning.cruise in mocklink.rs is a
// fixed default, not read from a vehicle parameter. Upgrade to a real cruise-speed fact
// or parameter read if one is ever exposed.
const CRUISE_MPS: f64 = 10.0;

static CANCELLED: AtomicBool = AtomicBool::new(false);

fn active_gimbal(backend: &dyn Backend) -> Option<(f64, f64)> {
    let snapshot = crate::gimbal::gimbal_view(backend, &[]);
    let gimbals = snapshot["gimbals"].as_array()?;
    let active = gimbals
        .iter()
        .find(|g| flag(g, "active"))
        .or(gimbals.first())?;
    Some((active["pitch"].as_f64()?, active["absoluteYaw"].as_f64()?))
}

fn release_state(backend: &dyn Backend) -> Option<(Point, f64, Velocity, f64)> {
    let (lat, lon) = backend.point("vehicle.coordinate")?;
    let altitude_agl_m = backend.raw("vehicle.altitudeRelative")?;
    let ground_speed = backend.raw("vehicle.groundSpeed").unwrap_or(0.0);
    let heading = backend.raw("vehicle.heading").unwrap_or(0.0);
    let climb_mps = backend.raw("vehicle.climbRate").unwrap_or(0.0);
    let (sin, cos) = heading.to_radians().sin_cos();
    Some((
        Point { lat, lon },
        altitude_agl_m,
        Velocity {
            north_mps: ground_speed * cos,
            east_mps: ground_speed * sin,
        },
        climb_mps,
    ))
}

fn tracked_target(backend: &dyn Backend, position: Point, altitude_agl_m: f64) -> Option<Target> {
    let (pitch, yaw) = active_gimbal(backend)?;
    gimbalgeo::track(position, altitude_agl_m, yaw, pitch, crate::hub::now_ms())
}

fn far_point_on_bearing(from: Point, towards: Point, distance_m: f64) -> Point {
    let origin = (from.lat, from.lon, 0.0);
    let (n0, e0, _) = geo_to_ned(towards.lat, towards.lon, 0.0, origin);
    let mag = n0.hypot(e0).max(1e-6);
    let (lat, lon, _) = ned_to_geo(n0 / mag * distance_m, e0 / mag * distance_m, 0.0, origin);
    Point { lat, lon }
}

fn send_goto(backend: &dyn Backend, at: Point) -> Value {
    crate::actions::run(
        backend,
        "vehicle.guidedModeGotoLocation",
        &json!([{"latitude": at.lat, "longitude": at.lon}]).to_string(),
    )
}

fn fire_gripper(backend: &dyn Backend) -> Value {
    crate::actions::run(backend, "vehicle.sendGripperAction", "[0,0]")
}

/// Aim the camera at the target, call this, and the drone flies the intercept and drops
/// the payload on its own: no further input. `args == ["stop"]` cancels a run in flight.
pub fn run(backend: &dyn Backend, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    if given.get(0).and_then(Value::as_str) == Some("stop") {
        CANCELLED.store(true, Ordering::Relaxed);
        return json!({ "ok": true, "stopped": true });
    }
    CANCELLED.store(false, Ordering::Relaxed);

    let Some((position, altitude_agl_m, _velocity, _climb_mps)) = release_state(backend) else {
        return json!({ "ok": false, "reason": "no vehicle position yet" });
    };
    let Some(target) = tracked_target(backend, position, altitude_agl_m) else {
        return json!({ "ok": false, "reason": "the gimbal is not looking far enough below the horizon to reach the ground" });
    };
    let Some(solved) = dropsim::intercept(position, CRUISE_MPS, target) else {
        return json!({ "ok": false, "reason": "the target cannot be intercepted at this speed on this heading" });
    };

    let aim_at = far_point_on_bearing(position, solved.lead_point, GUIDED_GOTO_LIMIT_M);
    let sent = send_goto(backend, aim_at);
    if sent["ok"].as_bool() != Some(true) {
        return json!({ "ok": false, "reason": format!("could not steer onto the intercept heading: {}", sent["reason"]) });
    }

    let started = Instant::now();
    // A guided goto takes a moment to actually move the vehicle. Checking dropLead's
    // constant-velocity release math before the drone is up to speed reads a near-zero
    // velocity and solves a nonsense (usually immediate) release - this is the exact bug
    // that fired the gripper on the very first attempt testing this live.
    while started.elapsed().as_secs_f64() < MAX_RUN_S {
        if CANCELLED.load(Ordering::Relaxed) {
            return json!({ "ok": false, "reason": "cancelled" });
        }
        let speed = release_state(backend)
            .map(|(_, _, v, _)| v.north_mps.hypot(v.east_mps))
            .unwrap_or(0.0);
        if speed >= 0.8 * CRUISE_MPS {
            break;
        }
        thread::sleep(Duration::from_millis(AUTODROP_TICK_MS));
    }

    loop {
        if CANCELLED.load(Ordering::Relaxed) {
            return json!({ "ok": false, "reason": "cancelled" });
        }
        if started.elapsed().as_secs_f64() >= MAX_RUN_S {
            return json!({ "ok": false, "reason": "gave up: no release window opened within the time limit" });
        }
        let Some((position, altitude_agl_m, velocity, climb_mps)) = release_state(backend) else {
            thread::sleep(Duration::from_millis(AUTODROP_TICK_MS));
            continue;
        };
        let Some(target) = tracked_target(backend, position, altitude_agl_m) else {
            thread::sleep(Duration::from_millis(AUTODROP_TICK_MS));
            continue;
        };
        let plan = dropsim::lead(
            Release {
                position,
                altitude_agl_m,
                velocity,
                climb_mps,
            },
            target,
        );
        if plan.release_in_s <= COMMAND_LATENCY_S {
            let fired = fire_gripper(backend);
            return json!({
                "ok": fired["ok"].as_bool().unwrap_or(false),
                "reason": fired["reason"],
                "predictedMissMeters": plan.miss_m,
                "waitedSeconds": started.elapsed().as_secs_f64(),
            });
        }
        thread::sleep(
            Duration::from_millis(AUTODROP_TICK_MS).min(Duration::from_secs_f64(
                plan.release_in_s - COMMAND_LATENCY_S,
            )),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn stop_cancels_without_reading_any_vehicle_state() {
        struct Unreachable;
        impl Backend for Unreachable {
            fn get(&self, _p: &str) -> String {
                panic!("stop must not read any vehicle state")
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String {
                panic!("stop must not read any vehicle state")
            }
            fn set(&self, _p: &str, _v: &str) -> String {
                String::new()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String {
                String::new()
            }
            fn watch(&self, _p: &[String]) {}
        }

        let answer = run(&Unreachable, "[\"stop\"]");
        assert_eq!(answer, json!({ "ok": true, "stopped": true }));
        assert!(CANCELLED.load(Ordering::Relaxed));
        CANCELLED.store(false, Ordering::Relaxed);
    }
}
