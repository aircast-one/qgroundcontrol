use serde_json::{Value, json};

use crate::router::Backend;
use crate::view::VIEWS;

pub const DEPS: &[&str] = &[];

fn routine_ids() -> Vec<&'static str> {
    vec!["accelerometer", "compass", "levelHorizon", "gyro", "pressure", "airspeed"]
}

pub fn enumerations() -> Value {
    json!({
        "view.guidedActions.actions[].offer": ["hidden", "ready", "blocked"],
        "view.guidedActions.actions[].id": ["arm", "takeoff", "startMission", "continueMission", "pause", "changeAltitude", "changeSpeed", "landAbort", "land", "rtl", "disarm", "grab", "release", "emergencyStop"],
        "view.battery.level": ["normal", "caution", "warning", "critical"],
        "view.battery.packs[].level": ["normal", "caution", "warning", "critical"],
        "view.preflight.groups[].checks[].verdict": ["manual", "passing", "failing", "overridable"],
        "view.vibration.axes[].severity": ["normal", "warning", "danger"],
        "view.vibration.worst": ["normal", "warning", "danger"],
        "view.sensors.sensors[].state": ["healthy", "unhealthy", "disabled"],
        "view.control.control": ["toggle", "choice", "text", "number"],
        "view.plan.sync.state": ["offline", "busy", "ready"],
        "view.plan.readiness.state": [0, 1, 2],
        "view.plan.upload.state": [0, 1, 2, 3],
        "view.links.links[].type": ["tcp", "udp", "serial", "bluetooth", "logReplay", "other"],
        "view.links.links[].editing": ["hostAndPort", "portOnly", "serial", "logFile", "none"],
        "view.warnings.warnings[].id": ["noGpsLock", "prearm"],
        "view.calibration.sides[].stage": ["waiting", "inProgress", "done"],
        "view.calibration.routines[].id": ["accelerometer", "compass", "levelHorizon", "gyro", "pressure"],
        "view.missionKinds.kinds[].geometry": ["area", "line", null],
        "view.missionKinds.kinds[].id": ["waypoint", "takeoff", "land", "roi", "survey", "corridor", "structure"],
        "view.setup.firmware": ["none", "apm", "px4"],
        "view.setup.groups[].pages[].name": crate::setup::PAGES.iter().flat_map(|(_, pages)| pages.iter().copied()).collect::<Vec<_>>(),
        "view.messages.items[].level": ["normal", "warning", "error"],
        "view.coreCalibration.calibration.running": routine_ids(),
        "view.coreCalibration.calibration.last": routine_ids(),
        "view.coreCalibration.calibration.routines[].id": routine_ids(),
        "view.coreCalibration.calibration.sides[].stage": ["waiting", "inProgress", "done"],
        "view.coreCalibration.calibration.outcome": ["success", "cancelled", "failed"],
        "view.fences.polygons[].shape": ["polygon"],
        "view.fences.circles[].shape": ["circle"],
        "view.guidedSpeed.command": ["guidedModeChangeGroundSpeedMetersSecond", "guidedModeChangeEquivalentAirspeedMetersSecond"],
        "view.camera.modeText": ["Photo", "Video", "Survey", "Not set"],
    })
}

pub fn contract_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    json!({
        "kind": "object",
        "class": "Contract",
        "paths": VIEWS.iter().map(|v| v.path).collect::<Vec<_>>(),
        "enumerations": enumerations(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guided::ACTIONS;
    use crate::missionkinds::KINDS;

    #[test]
    fn the_enumerations_agree_with_the_producers() {
        let listed = enumerations();
        let ids: Vec<Value> = ACTIONS.iter().map(|a| serde_json::to_value(a).unwrap()).collect();
        assert_eq!(listed["view.guidedActions.actions[].id"], Value::Array(ids));
        let kinds: Vec<Value> = KINDS.iter().map(|k| json!(k.id)).collect();
        assert_eq!(listed["view.missionKinds.kinds[].id"], Value::Array(kinds));
        assert_eq!(listed["view.battery.level"].as_array().unwrap().len(), 4);
        assert!(VIEWS.iter().any(|v| v.path == "view.contract"));
        let pages: Vec<Value> = crate::setup::PAGES.iter().flat_map(|(_, pages)| pages.iter().map(|p| json!(p))).collect();
        assert_eq!(listed["view.setup.groups[].pages[].name"], Value::Array(pages), "the setup page names are the list the core ships, so a head can pin its glyph table against it");
        let routines: Vec<Value> = routine_ids().iter().map(|id| json!(id)).collect();
        assert_eq!(listed["view.coreCalibration.calibration.routines[].id"], Value::Array(routines));
        assert!(routine_ids().iter().all(|id| crate::sensorcal::Kind::parse(id).is_some()), "every listed routine is a kind the core accepts");
        assert!(["normal", "warning", "error"].iter().all(|level| crate::messages::level_of(match *level { "error" => "#E", "warning" => "#I", _ => "" }) == *level));
    }
}
