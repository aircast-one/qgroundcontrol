use serde_json::{Value, json};

use crate::read::{flag, integer, object};
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
    armed && !mission_mode.is_empty() && mode == mission_mode && progress.last > 0 && (1..=progress.last).contains(&progress.current)
}

pub fn fraction(progress: &Progress) -> f64 {
    (progress.current as f64 / progress.last.max(1) as f64).clamp(0.0, 1.0)
}

fn last_sequence(backend: &dyn Backend) -> i64 {
    let count = integer(&object(&backend.get("planFly.missionController.visualItems.count")), "value").unwrap_or(0);
    (count > 0)
        .then(|| integer(&object(&backend.get_fields(&format!("planFly.missionController.visualItems.{}", count - 1), "lastSequenceNumber")), "lastSequenceNumber"))
        .flatten()
        .unwrap_or(0)
}

pub fn mission_progress_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let available = flag(&object(&backend.get("vehicles.activeVehicleAvailable")), "value");
    let vehicle = object(&backend.get_fields("vehicle", "armed,flightMode,missionFlightMode"));
    let text = |key: &str| vehicle.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
    let current = integer(&object(&backend.get_fields("planFly.missionController", "currentMissionIndex")), "currentMissionIndex").unwrap_or(-1);
    let progress = Progress { current, last: last_sequence(backend) };
    let visible = available && shown(flag(&vehicle, "armed"), &text("flightMode"), &text("missionFlightMode"), &progress);
    let distance = object(&backend.get("vehicle.distanceToNextWP"));
    json!({
        "kind": "object",
        "class": "MissionProgress",
        "shown": visible,
        "current": progress.current,
        "last": progress.last,
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
    fn the_card_shows_only_while_armed_on_a_mission_with_items_left() {
        let on = Progress { current: 3, last: 6 };
        assert!(shown(true, "Auto", "Auto", &on));
        assert!(!shown(false, "Auto", "Auto", &on), "disarmed");
        assert!(!shown(true, "Guided", "Auto", &on), "MissionController's current index still moves in other modes, but the mission is not being flown");
        assert!(!shown(true, "", "", &on), "a firmware with no mission mode never shows it");
        assert!(!shown(true, "Auto", "Auto", &Progress { current: 0, last: 6 }), "home is not a waypoint to fly to");
        assert!(!shown(true, "Auto", "Auto", &Progress { current: 1, last: 0 }), "no mission");
        assert_eq!(fraction(&on), 0.5);
    }
}
