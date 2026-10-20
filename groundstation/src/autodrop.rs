use serde_json::{Value, json};

use crate::dropsim::{self, Point};
use crate::geo::{geo_to_ned, ned_to_geo};
use crate::gimbalgeo;
use crate::read::flag;
use crate::router::Backend;

pub const COMMAND_LATENCY_S: f64 = 0.15;
pub const MAX_RUN_S: f64 = 90.0;
pub const AUTODROP_TICK_MS: u64 = 150;
const GUIDED_GOTO_LIMIT_M: f64 = 950.0;
// ponytail: this mock's observed ArduCopter guided-mode cruise speed (measured live,
// repeatedly, across a long session). No cleaner source found: tuning.cruise in mocklink.rs
// is a fixed default, not read from a vehicle parameter. Upgrade to a real cruise-speed fact
// or parameter read if one is ever exposed.
pub const CRUISE_MPS: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    WaitingForCruise,
    WaitingForRelease,
}

/// Lives on `hub::Vehicle`, advanced one tick at a time by `Vehicle::tick_autodrop` - no
/// blocking sleeps anywhere, same shape as the `rc_due`/`gimbal_rate_due` repeating behaviours
/// already in `hub.rs`. `None` means idle (never run, or its last result already read).
#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Waiting {
        phase: Phase,
        started_ms: u64,
        due_ms: u64,
    },
    Fired {
        predicted_miss_m: f64,
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
            State::Waiting { phase, .. } => json!({
                "state": "waiting",
                "phase": match phase {
                    Phase::WaitingForCruise => "waitingForCruise",
                    Phase::WaitingForRelease => "waitingForRelease",
                },
            }),
            State::Fired {
                predicted_miss_m,
                at_ms,
            } => {
                json!({ "state": "fired", "predictedMissMeters": predicted_miss_m, "atMs": at_ms })
            }
            State::GaveUp { reason, at_ms } => {
                json!({ "state": "gaveUp", "reason": reason, "atMs": at_ms })
            }
        }
    }
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

/// Aim the camera at the target, call this, and the drone flies the intercept and drops the
/// payload on its own: no further input. One snapshot of the live gimbal aim decides the
/// heading to fly; everything after that - waiting to reach cruise speed, polling the release
/// window, firing the gripper - happens tick by tick inside the core (`hub::Vehicle::tick_autodrop`),
/// not in this function. `args == ["stop"]` cancels a run in flight.
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

    let Some((position, altitude_agl_m)) = release_position(backend) else {
        return json!({ "ok": false, "reason": "no vehicle position yet" });
    };
    let Some((pitch, yaw)) = active_gimbal(backend) else {
        return json!({ "ok": false, "reason": "no gimbal to aim with" });
    };
    let Some(target) = gimbalgeo::track(position, altitude_agl_m, yaw, pitch, crate::hub::now_ms())
    else {
        return json!({ "ok": false, "reason": "the gimbal is not looking far enough below the horizon to reach the ground" });
    };
    let Some(solved) = dropsim::intercept(position, CRUISE_MPS, target) else {
        return json!({ "ok": false, "reason": "the target cannot be intercepted at this speed on this heading" });
    };
    let aim_at = far_point_on_bearing(position, solved.lead_point, GUIDED_GOTO_LIMIT_M);

    match backend.core_guided(&json!({
        "action": "goto",
        "latitude": aim_at.lat,
        "longitude": aim_at.lon,
        "autodrop": true,
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
    let mut status = crate::hub::lock()
        .vehicle(id as u8)
        .and_then(|v| v.autodrop.as_ref())
        .map(State::json)
        .unwrap_or_else(|| json!({ "state": "idle" }));
    status["kind"] = json!("object");
    status["class"] = json!("AutodropStatus");
    status
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
    fn state_json_names_each_variant() {
        assert_eq!(
            State::Waiting {
                phase: Phase::WaitingForCruise,
                started_ms: 0,
                due_ms: 0
            }
            .json()["phase"],
            "waitingForCruise"
        );
        assert_eq!(
            State::Fired {
                predicted_miss_m: 1.5,
                at_ms: 10
            }
            .json()["state"],
            "fired"
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
}
