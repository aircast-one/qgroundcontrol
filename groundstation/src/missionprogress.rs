use serde_json::{Value, json};

use crate::read::{flag, integer};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.armed",
    "vehicle.flightMode",
    "vehicle.distanceToNextWP",
    "planFly.missionController.currentMissionIndex",
    "planFly.missionController.visualItems.count",
];

pub struct Progress {
    pub current: i64,
    pub last: i64,
}

pub fn shown(armed: bool, mode: &str, mission_mode: &str, progress: &Progress) -> bool {
    armed
        && !mission_mode.is_empty()
        && mode == mission_mode
        && progress.last > 0
        && (1..=progress.last).contains(&progress.current)
}

pub fn progress_line(row: Option<&crate::coreplan::FlownRow>, progress: &Progress) -> String {
    match row {
        Some(row) if row.arrived => format!("At {}", row.place),
        Some(row) => format!("To {}", row.place),
        None => format!("Mission item {} of {}", progress.current, progress.last),
    }
}

pub fn set_waypoint_line(row: Option<&crate::coreplan::FlownRow>, sequence: i64) -> String {
    match row {
        Some(row) => format!("Continue the mission from {}", row.place),
        None => format!("Continue the mission from mission item {sequence}"),
    }
}

pub fn set_waypoint_view(_backend: &dyn Backend, args: &[String]) -> Value {
    match args.first().and_then(|given| given.parse::<i64>().ok()) {
        Some(sequence) => json!({
            "kind": "object",
            "sequence": sequence,
            "line": set_waypoint_line(crate::coreplan::flown_row(sequence).as_ref(), sequence),
        }),
        None => json!({
            "ok": false,
            "reason": "view.setWaypoint needs the mission sequence, as view.setWaypoint(4)",
        }),
    }
}

pub fn fraction(progress: &Progress) -> f64 {
    (progress.current as f64 / progress.last.max(1) as f64).clamp(0.0, 1.0)
}

pub fn last_sequence(backend: &dyn Backend) -> i64 {
    let count = integer(
        &backend.value("planFly.missionController.visualItems.count"),
        "value",
    )
    .unwrap_or(0);
    (count > 0)
        .then(|| {
            integer(
                &backend.value_fields(
                    &format!("planFly.missionController.visualItems.{}", count - 1),
                    "lastSequenceNumber",
                ),
                "lastSequenceNumber",
            )
            .unwrap_or(count - 1)
        })
        .unwrap_or(0)
}

pub fn mission_progress_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let available = flag(&backend.value("vehicles.activeVehicleAvailable"), "value");
    let vehicle = backend.value_fields("vehicle", "armed,flightMode,missionFlightMode");
    let text = |key: &str| {
        vehicle
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    let current = integer(
        &backend.value_fields("planFly.missionController", "currentMissionIndex"),
        "currentMissionIndex",
    )
    .unwrap_or(-1);
    let progress = Progress {
        current,
        last: last_sequence(backend),
    };
    let visible = available
        && shown(
            flag(&vehicle, "armed"),
            &text("flightMode"),
            &text("missionFlightMode"),
            &progress,
        );
    let row = visible
        .then(|| crate::coreplan::flown_row(progress.current))
        .flatten();
    let distance = Some(backend.value("vehicle.distanceToNextWP"))
        .filter(|fact| crate::read::value_number(fact).is_some_and(|metres| metres > 0.0))
        .unwrap_or(Value::Null);
    json!({
        "kind": "object",
        "class": "MissionProgress",
        "shown": visible,
        "current": progress.current,
        "last": progress.last,
        "line": progress_line(row.as_ref(), &progress),
        "fraction": fraction(&progress),
        "distanceToNext": distance.get("valueString").cloned().unwrap_or(Value::Null),
        "distanceUnits": distance.get("units").cloned().unwrap_or(Value::Null),
        "canSkip": visible && progress.current < progress.last,
        "skipTo": (progress.current + 1).min(progress.last),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_core_fly_mission_counts_home_so_its_last_sequence_is_count_less_one() {
        struct Core;
        impl Backend for Core {
            fn get(&self, path: &str) -> String {
                match path {
                    "planFly.missionController.visualItems.count" => {
                        json!({ "kind": "value", "value": 4 })
                    }
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String {
                json!({ "kind": "null" }).to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String {
                String::new()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String {
                String::new()
            }
            fn watch(&self, _p: &[String]) {}
        }
        assert_eq!(
            last_sequence(&Core),
            3,
            "home plus three waypoints, and the core serves no visual items to read a lastSequenceNumber from"
        );
    }

    struct Distance(Value);
    impl Backend for Distance {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.distanceToNextWP" => self.0.clone(),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, _p: &str, _f: &str) -> String {
            json!({ "kind": "null" }).to_string()
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
    fn a_distance_the_vehicle_never_reported_is_left_off_rather_than_shown_as_zero() {
        let fact = |metres: f64, text: &str| json!({ "kind": "fact", "value": metres, "valueString": text, "units": "m" });
        let unsent = mission_progress_view(&Distance(fact(0.0, "0")), &[]);
        assert_eq!(
            (&unsent["distanceToNext"], &unsent["distanceUnits"]),
            (&Value::Null, &Value::Null),
            "distanceToNextWP stays at its default 0 until NAV_CONTROLLER_OUTPUT arrives, and 0 m to the next waypoint tells the pilot nothing"
        );
        let sent = mission_progress_view(&Distance(fact(84.0, "84")), &[]);
        assert_eq!(
            (&sent["distanceToNext"], &sent["distanceUnits"]),
            (&json!("84"), &json!("m"))
        );
    }

    #[test]
    fn the_card_shows_only_while_armed_on_a_mission_with_items_left() {
        let on = Progress {
            current: 3,
            last: 6,
        };
        assert!(shown(true, "Auto", "Auto", &on));
        assert!(!shown(false, "Auto", "Auto", &on), "disarmed");
        assert!(
            !shown(true, "Guided", "Auto", &on),
            "MissionController's current index still moves in other modes, but the mission is not being flown"
        );
        assert!(
            !shown(true, "", "", &on),
            "a firmware with no mission mode never shows it"
        );
        assert!(
            !shown(
                true,
                "Auto",
                "Auto",
                &Progress {
                    current: 0,
                    last: 6
                }
            ),
            "home is not a waypoint to fly to"
        );
        assert!(
            !shown(
                true,
                "Auto",
                "Auto",
                &Progress {
                    current: 1,
                    last: 0
                }
            ),
            "no mission"
        );
        assert_eq!(fraction(&on), 0.5);
    }
}
