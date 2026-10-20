use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use crate::dropsim::{self, Point, Release, Target, Velocity};
use crate::router::Backend;

/// One vehicle's live telemetry, as needed to stand in as a `dropsim::Target`. Refreshed once
/// per hub tick from every vehicle's own facts - no position-delta tracking needed the way
/// `gimbalgeo` requires, because a real vehicle already reports its own velocity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fix {
    pub position: Point,
    pub ground_speed_mps: f64,
    pub heading_deg: f64,
}

impl Fix {
    pub fn target(self) -> Target {
        let (sin, cos) = self.heading_deg.to_radians().sin_cos();
        Target {
            position: self.position,
            velocity: Velocity {
                north_mps: self.ground_speed_mps * cos,
                east_mps: self.ground_speed_mps * sin,
            },
        }
    }
}

static FIXES: Mutex<BTreeMap<u8, Fix>> = Mutex::new(BTreeMap::new());

pub fn refresh(fixes: BTreeMap<u8, Fix>) {
    *FIXES.lock().unwrap_or_else(PoisonError::into_inner) = fixes;
}

pub fn target_of(id: u8) -> Option<Target> {
    FIXES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&id)
        .map(|fix| fix.target())
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

const NO_FIX: &str = "no live telemetry recorded yet for that vehicle id - it needs to be connected and heard from at least once";

pub fn drop_lead_from_vehicle_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(v) = numbers(args, 7) else {
        return crate::read::refused(
            "this needs seven numbers: release latitude, longitude, altitude AGL, north speed, east speed, climb speed (positive is up), then the target vehicle id",
        );
    };
    let Some(target_id) = u8::try_from(v[6] as i64).ok() else {
        return crate::read::refused("the target vehicle id has to be a small positive number");
    };
    let Some(target) = target_of(target_id) else {
        return crate::read::refused(NO_FIX);
    };
    dropsim::lead(
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
    fn a_fix_heading_north_has_no_east_velocity() {
        let fix = Fix {
            position: Point {
                lat: 47.0,
                lon: 8.0,
            },
            ground_speed_mps: 5.0,
            heading_deg: 0.0,
        };
        let target = fix.target();
        assert!(close(target.velocity.north_mps, 5.0, 1e-9));
        assert!(close(target.velocity.east_mps, 0.0, 1e-9));
    }

    #[test]
    fn a_fix_heading_east_has_no_north_velocity() {
        let fix = Fix {
            position: Point {
                lat: 47.0,
                lon: 8.0,
            },
            ground_speed_mps: 5.0,
            heading_deg: 90.0,
        };
        let target = fix.target();
        assert!(close(target.velocity.north_mps, 0.0, 1e-6));
        assert!(close(target.velocity.east_mps, 5.0, 1e-9));
    }

    #[test]
    fn refresh_then_target_of_round_trips_and_an_unknown_id_answers_none() {
        let mut fixes = BTreeMap::new();
        fixes.insert(
            129,
            Fix {
                position: Point {
                    lat: 47.4,
                    lon: 8.5,
                },
                ground_speed_mps: 5.0,
                heading_deg: 0.0,
            },
        );
        refresh(fixes);
        assert!(target_of(129).is_some());
        assert!(
            target_of(250).is_none(),
            "a vehicle id the snapshot never carried answers None, not a stale fix from a previous test"
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
    fn the_view_refuses_anything_other_than_seven_numbers() {
        let answer = drop_lead_from_vehicle_view(&NoBackend, &["1".to_string()]);
        assert_eq!(answer["kind"], "null");
        assert!(answer["reason"].as_str().unwrap().contains("seven numbers"));
    }

    #[test]
    fn the_view_reports_a_real_target_s_fix_against_an_explicit_release() {
        let mut fixes = BTreeMap::new();
        fixes.insert(
            7,
            Fix {
                position: Point {
                    lat: 47.00018,
                    lon: 8.0,
                },
                ground_speed_mps: 0.0,
                heading_deg: 0.0,
            },
        );
        refresh(fixes);
        let args: Vec<String> = ["47.0", "8.0", "19.6133", "10", "0", "0", "7"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let answer = drop_lead_from_vehicle_view(&NoBackend, &args);

        assert!(
            answer["missMeters"].as_f64().unwrap() < 0.1,
            "a drone flying straight at a known-stationary target's live fix should find a near-exact release, got {answer}"
        );
    }

    #[test]
    fn the_view_refuses_a_vehicle_id_with_no_recorded_fix() {
        refresh(BTreeMap::new());
        let args: Vec<String> = ["47.0", "8.0", "20.0", "0", "0", "0", "99"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let answer = drop_lead_from_vehicle_view(&NoBackend, &args);

        assert_eq!(answer["kind"], "null");
        assert!(
            answer["reason"]
                .as_str()
                .unwrap()
                .contains("no live telemetry")
        );
    }
}
