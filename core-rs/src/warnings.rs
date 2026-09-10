use serde_json::{Value, json};

use crate::read::{flag, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.requiresGpsFix",
    "vehicle.armed",
    "vehicle.prearmError",
    "vehicle.coordinate",
    "vehicle.allSensorsHealthy",
    "vehicle.readyToFlyAvailable",
    "vehicle.readyToFly",
    "vehicle.healthAndArmingCheckReport.supported",
];

#[derive(Default)]
pub struct State {
    pub connected: bool,
    pub requires_gps_fix: bool,
    pub has_position: bool,
    pub armed: bool,
    pub prearm_error: String,
    pub report_supported: bool,
    pub all_sensors_healthy: bool,
    pub ready_to_fly_available: bool,
    pub ready_to_fly: bool,
}

pub fn warnings(s: &State) -> Vec<Value> {
    let no_gps = (s.connected && s.requires_gps_fix && !s.has_position).then(|| json!({
        "id": "noGpsLock",
        "text": "No GPS lock for vehicle",
        "detail": "This vehicle needs a position fix before it will arm.",
    }));
    let prearm = (s.connected && !s.armed && !s.prearm_error.is_empty() && !s.report_supported).then(|| json!({
        "id": "prearm",
        "text": s.prearm_error,
        "detail": "The vehicle has failed a pre-arm check. In order to arm the vehicle, resolve the failure.",
    }));
    [no_gps, prearm].into_iter().flatten().collect()
}

pub fn arming_blocker(s: &State) -> Option<String> {
    match s {
        State { connected: false, .. } => None,
        State { armed: true, .. } => None,
        State { prearm_error, report_supported: false, .. } if !prearm_error.is_empty() => Some(prearm_error.clone()),
        State { requires_gps_fix: true, has_position: false, .. } => Some("No GPS lock. This vehicle needs a position fix before it will arm.".to_string()),
        State { all_sensors_healthy: false, .. } => Some("A sensor is reporting unhealthy. The vehicle will refuse to arm.".to_string()),
        State { ready_to_fly_available: true, ready_to_fly: false, .. } => Some("The vehicle is not ready to fly yet.".to_string()),
        _ => None,
    }
}

pub fn warnings_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let state = read_state(backend);
    let listed = warnings(&state);
    json!({
        "kind": "object",
        "class": "VehicleWarnings",
        "warnings": listed,
        "armingBlocker": arming_blocker(&state),
    })
}

fn read_state(backend: &dyn Backend) -> State {
    let vehicle = object(&backend.get_fields("vehicle", "requiresGpsFix,armed,prearmError,coordinate,allSensorsHealthy,readyToFlyAvailable,readyToFly"));
    if vehicle.get("kind") != Some(&Value::String("object".into())) {
        return State::default();
    }
    let report = object(&backend.get_fields("vehicle.healthAndArmingCheckReport", "supported"));
    State {
        connected: true,
        requires_gps_fix: flag(&vehicle, "requiresGpsFix"),
        has_position: vehicle.get("coordinate").map(|c| flag(c, "valid")).unwrap_or(false),
        armed: flag(&vehicle, "armed"),
        prearm_error: text(&vehicle, "prearmError"),
        report_supported: flag(&report, "supported"),
        all_sensors_healthy: flag(&vehicle, "allSensorsHealthy"),
        ready_to_fly_available: flag(&vehicle, "readyToFlyAvailable"),
        ready_to_fly: flag(&vehicle, "readyToFly"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy() -> State {
        State { connected: true, has_position: true, all_sensors_healthy: true, ready_to_fly_available: true, ready_to_fly: true, ..State::default() }
    }

    #[test]
    fn a_healthy_vehicle_shows_nothing() {
        assert!(warnings(&healthy()).is_empty());
        assert_eq!(arming_blocker(&healthy()), None);
        assert!(warnings(&State::default()).is_empty());
    }

    #[test]
    fn the_gps_warning_follows_the_qml_rule() {
        let lost = State { requires_gps_fix: true, has_position: false, ..healthy() };
        assert_eq!(warnings(&lost)[0]["id"], "noGpsLock");
        assert!(arming_blocker(&lost).unwrap().starts_with("No GPS lock"));
        let armed = State { armed: true, ..lost };
        assert_eq!(warnings(&armed).len(), 1);
        assert_eq!(arming_blocker(&armed), None);
    }

    #[test]
    fn a_prearm_error_is_suppressed_in_both_places_when_a_health_report_supersedes_it() {
        let failing = State { prearm_error: "PreArm: Compass not calibrated".into(), ..healthy() };
        assert_eq!(warnings(&failing)[0]["id"], "prearm");
        assert_eq!(arming_blocker(&failing).as_deref(), Some("PreArm: Compass not calibrated"));
        let reported = State { report_supported: true, prearm_error: failing.prearm_error.clone(), ..healthy() };
        assert!(warnings(&reported).is_empty());
        assert_ne!(arming_blocker(&reported).as_deref(), Some("PreArm: Compass not calibrated"), "a head reading only the blocker must not be shown a string the list has decided is superseded");
        assert_eq!(arming_blocker(&reported), None, "a report that says the vehicle is ready outranks a stale prearm string, and both heads now agree there is nothing to say");
        let reported_and_blocked = State { report_supported: true, ready_to_fly: false, prearm_error: failing.prearm_error.clone(), ..healthy() };
        assert_eq!(arming_blocker(&reported_and_blocked).as_deref(), Some("The vehicle is not ready to fly yet."), "where the report does block, it is the report's words a head shows, not the raw string beneath it");
    }

    #[test]
    fn the_blocker_has_a_fixed_priority() {
        let sensors = State { all_sensors_healthy: false, ready_to_fly: false, ..healthy() };
        assert!(arming_blocker(&sensors).unwrap().starts_with("A sensor"));
        let not_ready = State { ready_to_fly: false, ..healthy() };
        assert_eq!(arming_blocker(&not_ready).as_deref(), Some("The vehicle is not ready to fly yet."));
        assert!(warnings(&not_ready).is_empty());
    }
}
