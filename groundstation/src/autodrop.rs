use serde_json::{Value, json};

use crate::dropsim::{self, Point, Target, Velocity};
use crate::geo::{geo_to_ned, ned_to_geo};
use crate::gimbalgeo;
use crate::gpstarget;
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

/// Where the tracked target's position+velocity comes from, each tick. Chosen once when the
/// run is triggered, carried in `State::Waiting` for the whole run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetSource {
    Gimbal,
    Vehicle(u8),
    Manual,
}

impl TargetSource {
    pub fn of(args: &Value) -> Option<TargetSource> {
        match args.get(0).and_then(Value::as_str) {
            None | Some("gimbal") => Some(TargetSource::Gimbal),
            Some("manual") => Some(TargetSource::Manual),
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
        }
    }

    /// The `args`-array shape `of` parses back - used to hand a source already resolved by
    /// `run`'s fallback chain down to `hub.rs`'s mission handler without it re-guessing.
    fn args(self) -> Value {
        match self {
            TargetSource::Gimbal => json!(["gimbal"]),
            TargetSource::Vehicle(id) => json!(["vehicle", id]),
            TargetSource::Manual => json!(["manual"]),
        }
    }

    /// The order an auto-intercept trigger with no explicit source falls back through: a
    /// manually-marked point first (the operator just set it on purpose), then whichever other
    /// vehicle this one alone is tracking, then the gimbal - refusing only once none of the
    /// three actually produces a target.
    pub(crate) fn fallback_order(self_id: Option<u8>) -> impl Iterator<Item = TargetSource> {
        std::iter::once(TargetSource::Manual)
            .chain(
                self_id
                    .and_then(gpstarget::only_other)
                    .map(TargetSource::Vehicle),
            )
            .chain(std::iter::once(TargetSource::Gimbal))
    }
}

/// Lives on `hub::Vehicle`, advanced one tick at a time by `Vehicle::tick_autodrop` - no
/// blocking sleeps anywhere, same shape as the `rc_due`/`gimbal_rate_due` repeating behaviours
/// already in `hub.rs`. `None` means idle (never run, or its last result already read).
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
                    Phase::WaitingForCruise => "waitingForCruise",
                    Phase::WaitingForRelease => "waitingForRelease",
                },
                "source": source.json(),
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
    }
}

/// Designate a target - aim the camera at it, track another connected vehicle, or mark a point
/// on the map - call this, and the drone flies the intercept and drops the payload on its own:
/// no further input. One snapshot of the chosen source decides the heading to fly; everything
/// after that - waiting to reach cruise speed, polling the release window, firing the gripper -
/// happens tick by tick inside the core (`hub::Vehicle::tick_autodrop`), re-reading the same
/// source fresh each tick, not in this function. `args == ["stop"]` cancels a run in flight.
/// `args[0]` picks the source: `"gimbal"`, `"manual"`, or `"vehicle"` with the target vehicle's
/// id as `args[1]`. Omitted, it falls back through `TargetSource::fallback_order` instead of
/// assuming the gimbal - an explicit choice is still honored exactly, refusing if that one
/// source alone can't be read.
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

    // ponytail: checks only bay 0, the one every drop this app has ever used in practice.
    // A multi-bay rig with bay 0 empty but another bay still loaded would wrongly refuse here;
    // upgrade to checking every bay once a real multi-bay autodrop scenario exists.
    if backend
        .value("vehicle.payloadSetup")
        .get("empty")
        .and_then(Value::as_array)
        .is_some_and(|empty| empty.iter().any(|bay| bay.as_i64() == Some(0)))
    {
        return json!({ "ok": false, "reason": "the payload bay is already empty" });
    }
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
        let Some(resolved) = TargetSource::fallback_order(self_id).find_map(|source| {
            target_from(backend, source, position, altitude_agl_m)
                .ok()
                .map(|target| (source, target))
        }) else {
            return json!({ "ok": false, "reason": "no manually-marked target, tracked vehicle, or gimbal is available to aim with" });
        };
        resolved
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
        "targetSource": source.args(),
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

    #[test]
    fn target_source_defaults_to_gimbal_and_reads_a_vehicle_id() {
        assert_eq!(TargetSource::of(&json!([])), Some(TargetSource::Gimbal));
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
                "vehicle.payloadSetup" => json!({"empty": [0]}).to_string(),
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
            (Some(false), Some("the payload bay is already empty")),
            "an empty bay should refuse before ever steering the vehicle, not fly the course and fire nothing: {answer}"
        );
    }

    #[test]
    fn the_fallback_order_prefers_manual_then_a_lone_tracked_vehicle_then_the_gimbal() {
        let _fixes_lock = gpstarget::TEST_FIXES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        gpstarget::refresh(std::collections::BTreeMap::new());
        assert_eq!(
            TargetSource::fallback_order(Some(7)).collect::<Vec<_>>(),
            vec![TargetSource::Manual, TargetSource::Gimbal],
            "no other vehicle is tracked - the chain skips straight from manual to gimbal"
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
            },
        );
        gpstarget::refresh(fixes.clone());
        assert_eq!(
            TargetSource::fallback_order(Some(7)).collect::<Vec<_>>(),
            vec![
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
            },
        );
        gpstarget::refresh(fixes);
        assert_eq!(
            TargetSource::fallback_order(Some(7)).collect::<Vec<_>>(),
            vec![TargetSource::Manual, TargetSource::Gimbal],
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
            "no manually-marked target, tracked vehicle, or gimbal is available to aim with"
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
}
