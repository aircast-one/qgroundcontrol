use serde_json::{Value, json};

use crate::router::Backend;
use crate::view::VIEWS;

pub const DEPS: &[&str] = &[];

fn routine_ids() -> Vec<&'static str> {
    crate::sensorcal::KINDS.iter().map(|(_, id, _)| *id).collect()
}

fn routine_ids_or_null() -> Vec<Value> {
    routine_ids().iter().map(|id| json!(id)).chain(std::iter::once(Value::Null)).collect()
}

pub const ORDERS: [&str; 1] = ["oldestFirst"];

pub fn enumerations() -> Value {
    json!({
        "view.guidedActions.actions[].offer": ["hidden", "ready", "blocked"],
        "view.guidedActions.actions[].id": ["arm", "takeoff", "startMission", "continueMission", "resumeMission", "cancelRoi", "pause", "changeAltitude", "changeSpeed", "landAbort", "land", "rtl", "disarm", "grab", "release", "emergencyStop", "vtolTransitionToFixedWing", "vtolTransitionToMultiRotor", "forceArm"],
        "view.battery.level": ["normal", "caution", "warning", "critical"],
        "view.battery.packs[].level": ["normal", "caution", "warning", "critical"],
        "view.preflight.groups[].checks[].verdict": ["manual", "passing", "failing", "overridable"],
        "view.vibration.axes[].severity": ["normal", "warning", "danger"],
        "view.vibration.worst": ["normal", "warning", "danger"],
        "view.sensors.sensors[].state": ["healthy", "unhealthy", "disabled"],
        "view.control.control": ["toggle", "choice", "bitmask", "text", "number"],
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
        "view.altitudeModes.context": ["mission", "item"],
        "view.altitudeModes.modes[].raw": [0, 1, 2, 3, 4],
        "view.setup.groups[].pages[].name": crate::setup::PAGES.iter().flat_map(|(_, pages)| pages.iter().copied()).collect::<Vec<_>>(),
        "view.messages.items[].level": ["normal", "warning", "error"],
        "view.messages.order": ORDERS,
        "view.track.order": ORDERS,
        "view.coreCalibration.calibration.running": routine_ids_or_null(),
        "view.coreCalibration.calibration.last": routine_ids_or_null(),
        "view.coreCalibration.calibration.routines[].id": routine_ids(),
        "view.coreCalibration.calibration.sides[].stage": ["waiting", "inProgress", "done"],
        "view.coreCalibration.calibration.outcome": ["success", "cancelled", "failed", null],
        "view.fences.polygons[].shape": ["polygon"],
        "view.fences.circles[].shape": ["circle"],
        "view.guidedSpeed.command": ["guidedModeChangeGroundSpeedMetersSecond", "guidedModeChangeEquivalentAirspeedMetersSecond"],
        "view.camera.modeText": ["Photo", "Video", "Survey", "Not set"],
        "view.flyState.state": crate::flystate::STATES.iter().map(|s| s.token()).collect::<Vec<_>>(),
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
        let routines: Vec<Value> = crate::sensorcal::KINDS.iter().map(|(_, id, _)| json!(id)).collect();
        assert_eq!(listed["view.coreCalibration.calibration.routines[].id"], Value::Array(routines), "every calibration the core accepts is listed, and nothing else is");
        assert!(listed["view.coreCalibration.calibration.running"].as_array().unwrap().contains(&Value::Null), "a vehicle that has never calibrated answers null, so null is part of the contract");
        let bit = |mask: u8, index: u8| mask & (1 << index) != 0;
        let reachable: std::collections::BTreeSet<&str> = (0u8..32).map(|mask| crate::flystate::state_of(bit(mask, 0), bit(mask, 1), bit(mask, 2), bit(mask, 3), bit(mask, 4)).token()).collect();
        let listed_raws: Vec<i64> = listed["view.altitudeModes.modes[].raw"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
        assert_eq!(listed_raws, vec![crate::altitudemodes::MIXED, crate::altitudemodes::RELATIVE, crate::altitudemodes::ABSOLUTE, crate::altitudemodes::CALC_ABOVE_TERRAIN, crate::altitudemodes::TERRAIN_FRAME], "the raw values are written back through plan.missionController, so a reordering upstream changes what every mode means");
        let listed_states: std::collections::BTreeSet<&str> = listed["view.flyState.state"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect();
        assert_eq!(listed_states, reachable, "the contract lists the states the view can answer, not a hand-kept parallel list");
        assert!(listed["view.messages.items[].level"].as_array().unwrap().iter().all(|level| crate::messages::LEVELS.contains(&level.as_str().unwrap())));
        ["view.messages.order", "view.track.order"].iter().for_each(|path| {
            assert_eq!(listed[path], json!([crate::view::ORDER]), "{path} is the field whose whole purpose is to stop a head guessing which end is newest, so it is the one that most needs pinning");
        });
    }
}
