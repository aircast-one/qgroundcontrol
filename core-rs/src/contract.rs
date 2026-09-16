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

pub const NOTICE_KINDS: [&str; 3] = ["message", "vehicleError", "navigation"];

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
        "host.notices[].kind": NOTICE_KINDS,
    })
}

pub const NULLABLE_UNWITNESSED: &[&str] = &[
    "view.fences.fenceSupported",
    "view.fences.rallySupported",
    "view.plan.fenceSupported",
    "view.plan.rallySupported",
    "view.control(settings.appSettings.audioMuted).changedFromDefault",
    "view.plan.defaults.altitude.changedFromDefault",
    "view.settings(General).sections[0].subsections[0].controls[0].changedFromDefault",
    "view.setup(Safety).sections[0].controls[0].changedFromDefault",
    "view.obstacle.ringMetres",
    "view.obstacle.ringIncrement",
    "view.obstacle.rangeMinMetres",
    "view.obstacle.rangeMaxMetres",
];

pub fn contract_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    json!({
        "kind": "object",
        "class": "Contract",
        "paths": VIEWS.iter().map(|v| v.path).collect::<Vec<_>>(),
        "enumerations": enumerations(),
        "nullableUnwitnessed": NULLABLE_UNWITNESSED,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guided::ACTIONS;
    use crate::missionkinds::KINDS;

    #[test]
    fn a_field_the_rig_can_never_leave_unknown_is_still_declared_nullable() {
        let shapes: Value = serde_json::from_str(include_str!("../../test/Bridge/fixtures/view-shapes.json")).unwrap();
        let declared = |path: &str| -> String {
            let top = shapes.as_object().into_iter().flatten().map(|(key, _)| key.as_str()).filter(|key| path == *key || path.starts_with(&format!("{key}."))).max_by_key(|key| key.len());
            let Some(view) = top else { return format!("no view shape for {path}") };
            path[view.len()..]
                .trim_start_matches('.')
                .split('.')
                .fold(Some(&shapes[view]), |node, step| match step.strip_suffix("[0]") {
                    Some(name) => node?.get(name)?.get(0),
                    None => node?.get(step),
                })
                .map(|node| match node {
                    // A list-typed field is recorded as a one-element array naming its element
                    // type, so the walk has to step into it or every array reads as absent.
                    Value::Array(of) => of.first().and_then(Value::as_str).unwrap_or("empty").to_string(),
                    other => other.as_str().unwrap_or("missing").to_string(),
                })
                .unwrap_or_else(|| "missing".to_string())
        };
        NULLABLE_UNWITNESSED.iter().for_each(|path| {
            assert_ne!(
                declared(path),
                "missing",
                "{path} is listed here as a field the rig cannot witness filled, and it does not resolve in the contract at all - either the path is wrong or the field is gone"
            );
        });

        assert_eq!(crate::plan::capability(&Unknowing, "geoFenceController"), None, "this is the case the recorder cannot create: capability() withholds until capabilitiesKnown, and the rig's vehicle has always answered, so the recorded type says bool and a head is told the value can never be null");
        assert!(
            contract_view(&Unknowing, &[])["nullableUnwitnessed"].as_array().unwrap().iter().any(|p| p == "view.plan.fenceSupported"),
            "the list has to ride on the view a head actually reads, or it is a comment"
        );
    }

    struct Unknowing;
    impl Backend for Unknowing {
        fn get(&self, _p: &str) -> String { String::new() }
        fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "object", "capabilitiesKnown": false, "supported": true }).to_string() }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_enumerations_agree_with_the_producers() {
        let listed = enumerations();
        let ids: Vec<Value> = ACTIONS.iter().map(|a| serde_json::to_value(a).unwrap()).collect();
        assert_eq!(listed["view.guidedActions.actions[].id"], Value::Array(ids));
        let kinds: Vec<Value> = KINDS.iter().map(|k| json!(k.id)).collect();
        assert_eq!(listed["view.missionKinds.kinds[].id"], Value::Array(kinds));
        assert_eq!(listed["view.battery.level"].as_array().unwrap().len(), 4);
        assert_eq!(
            listed["host.notices[].kind"],
            json!(NOTICE_KINDS),
            "both heads read the host root directly and each spells these three kinds itself - Android drops navigation from its banners and routes a tab, macOS keeps it and refuses any destination but setup. Neither policy belongs here, but the DOMAIN does: a fourth kind added to QGCHostNotices::token would otherwise be silently unhandled by whichever head nobody remembered to tell. QGCCoreCTest pins this list against the C++ enum, because nothing in Rust can see it"
        );
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
