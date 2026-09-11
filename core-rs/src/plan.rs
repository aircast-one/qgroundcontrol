use serde_json::{Value, json};

use crate::read::{flag, object, result_integer};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "plan.syncInProgress",
    "plan.offline",
    "plan.dirty",
    "plan.containsItems",
    "plan.currentPlanFile",
    "plan.missionController.containsItems",
    "vehicles.activeVehicleAvailable",
    "vehicle.armed",
    "vehicle.flightMode",
];

pub fn plan_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let plan = object(&backend.get_fields("plan", "syncInProgress,offline,dirty,containsItems,currentPlanFile"));
    let mission = object(&backend.get_fields("plan.missionController", "containsItems"));
    let syncing = flag(&plan, "syncInProgress");
    let offline = flag(&plan, "offline");
    let dirty = flag(&plan, "dirty");
    let contains_items = flag(&plan, "containsItems");
    let has_mission_items = flag(&mission, "containsItems");
    let file = plan.get("currentPlanFile").and_then(Value::as_str).unwrap_or("");
    let name = file.rsplit('/').next().filter(|n| !n.is_empty());
    let readiness = result_integer(&backend.invoke("plan.readyForSaveState", "[]"));
    let upload = result_integer(&backend.invoke("plan.missionController.sendToVehiclePreCheck", "[]"));
    json!({
        "kind": "object",
        "class": "PlanStatus",
        "readiness": readiness_json(readiness),
        "upload": upload_json(upload),
        "actions": {
            "open": !syncing,
            "save": !syncing && contains_items,
            "exportKml": !syncing && has_mission_items,
            "newPlan": !syncing,
            "clearMission": !offline && !syncing,
        },
        "sync": sync_json(offline, syncing),
        "status": status_text(name, dirty, offline),
        "file": name,
        "dirty": dirty,
    })
}

fn readiness_json(state: Option<i64>) -> Value {
    let reason = match state {
        Some(0) => "",
        Some(1) => "Waiting for terrain heights before the plan can be saved or sent.",
        Some(2) => "An item is still being drawn, so the plan cannot be saved or sent.",
        _ => "The plan could not be checked for saving.",
    };
    json!({ "state": state, "ready": state == Some(0), "reason": reason })
}

fn upload_json(state: Option<i64>) -> Value {
    let (refusal, proceed_title) = match state {
        Some(0) => ("", ""),
        Some(1) => ("No vehicle is connected, so there is nowhere to send this plan.", ""),
        Some(2) => ("This plan was made for a different firmware or vehicle type. Uploading it can make the vehicle behave incorrectly.", "Upload anyway"),
        Some(3) => ("The vehicle is flying this mission. It has to be paused before a new plan goes up.", "Pause and upload"),
        _ => ("The plan could not be checked against the vehicle.", ""),
    };
    let can_proceed = matches!(state, Some(2) | Some(3));
    json!({
        "state": state,
        "canSend": state == Some(0),
        "refusal": refusal,
        "heading": if can_proceed { "Upload this plan?" } else { "This plan cannot be uploaded" },
        "proceedTitle": proceed_title,
        "canProceed": can_proceed,
        "pausesFirst": state == Some(3),
    })
}

fn sync_json(offline: bool, syncing: bool) -> Value {
    let (state, refusal) = match (offline, syncing) {
        (true, _) => ("offline", "No vehicle is connected."),
        (false, true) => ("busy", "Already syncing, wait for it to finish."),
        (false, false) => ("ready", ""),
    };
    json!({ "state": state, "refusal": refusal })
}

fn status_text(name: Option<&str>, dirty: bool, offline: bool) -> String {
    match (name, dirty, offline) {
        (None, false, _) => "New plan".to_string(),
        (None, true, _) => "Unsaved plan".to_string(),
        (Some(n), false, _) => n.to_string(),
        (Some(n), true, true) => format!("{n} \u{b7} unsaved changes"),
        (Some(n), true, false) => format!("{n} \u{b7} not uploaded"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    struct Fake {
        fields: BTreeMap<&'static str, Value>,
        readiness: Value,
        upload: Value,
    }

    impl Backend for Fake {
        fn get(&self, _path: &str) -> String {
            String::new()
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            self.fields.get(path).cloned().unwrap_or(json!({ "kind": "null" })).to_string()
        }
        fn set(&self, _path: &str, _value: &str) -> String {
            String::new()
        }
        fn invoke(&self, path: &str, _args: &str) -> String {
            match path {
                "plan.readyForSaveState" => self.readiness.to_string(),
                _ => self.upload.to_string(),
            }
        }
        fn watch(&self, _paths: &[String]) {}
    }

    fn fake(plan: Value, mission_items: bool, readiness: Value, upload: Value) -> Fake {
        Fake {
            fields: BTreeMap::from([("plan", plan), ("plan.missionController", json!({ "kind": "object", "containsItems": mission_items }))]),
            readiness,
            upload,
        }
    }

    #[test]
    fn the_actions_a_plan_offers_are_shown_open_as_well_as_shut() {
        // Every assertion on these said they were unavailable. Pinning save and clearMission to
        // false passed the crate, so a plan editor whose Save was never offered was a passing
        // build - the guards had been shown to refuse and never shown to let anything through.
        let ready = fake(
            json!({ "kind": "object", "syncInProgress": false, "offline": false, "dirty": true, "containsItems": true, "currentPlanFile": "/plans/ridge.plan" }),
            true,
            json!({ "ok": true, "result": 0 }),
            json!({ "ok": true, "result": 0 }),
        );
        let view = plan_view(&ready, &[]);
        assert_eq!(view["actions"]["save"], true, "a connected plan holding items is exactly when saving is the thing the operator wants");
        assert_eq!(view["actions"]["clearMission"], true, "clearing needs a vehicle to clear it from, and there is one");
        assert_eq!(view["actions"]["exportKml"], true);
        assert_eq!(view["actions"]["open"], true);
        assert_eq!(view["file"], "ridge.plan", "the name is the last segment, so a head does not have to split a path it was given whole");

        let syncing = fake(
            json!({ "kind": "object", "syncInProgress": true, "offline": false, "dirty": true, "containsItems": true, "currentPlanFile": "/plans/ridge.plan" }),
            true,
            json!({ "ok": true, "result": 0 }),
            json!({ "ok": true, "result": 0 }),
        );
        let mid = plan_view(&syncing, &[]);
        assert_eq!(mid["actions"]["save"], false, "a plan being written to the vehicle is not a plan to save over");
        assert_eq!(mid["actions"]["clearMission"], false);
        assert_eq!(mid["actions"]["exportKml"], false);
    }

    #[test]
    fn a_fresh_offline_plan_is_ready_and_cannot_upload() {
        let backend = fake(
            json!({ "kind": "object", "syncInProgress": false, "offline": true, "dirty": false, "containsItems": false, "currentPlanFile": "" }),
            false,
            json!({ "ok": true, "result": 0 }),
            json!({ "ok": true, "result": 1 }),
        );
        let view = plan_view(&backend, &[]);
        assert_eq!(view["readiness"]["ready"], true);
        assert_eq!(view["readiness"]["reason"], "");
        assert_eq!(view["upload"]["canSend"], false);
        assert_eq!(view["upload"]["canProceed"], false);
        assert_eq!(view["upload"]["heading"], "This plan cannot be uploaded");
        assert_eq!(view["actions"]["newPlan"], true);
        assert_eq!(view["actions"]["save"], false);
        assert_eq!(view["actions"]["clearMission"], false);
        assert_eq!(view["sync"]["state"], "offline");
        assert_eq!(view["status"], "New plan");
        assert_eq!(view["file"], Value::Null);
    }

    #[test]
    fn terrain_and_data_reasons_follow_the_cpp_enum_order() {
        let base = json!({ "kind": "object", "syncInProgress": false, "offline": false, "dirty": true, "containsItems": true, "currentPlanFile": "/tmp/field.plan" });
        let terrain = plan_view(&fake(base.clone(), true, json!({ "ok": true, "result": 1 }), json!({ "ok": true, "result": 0 })), &[]);
        assert!(terrain["readiness"]["reason"].as_str().unwrap().contains("terrain"));
        let data = plan_view(&fake(base.clone(), true, json!({ "ok": true, "result": 2 }), json!({ "ok": true, "result": 0 })), &[]);
        assert!(data["readiness"]["reason"].as_str().unwrap().contains("still being drawn"));
        assert_eq!(data["status"], "field.plan \u{b7} not uploaded");
        assert_eq!(data["actions"]["exportKml"], true);
        assert_eq!(data["sync"]["state"], "ready");
    }

    #[test]
    fn mismatch_and_active_mission_can_proceed_with_their_own_button() {
        let base = json!({ "kind": "object", "syncInProgress": true, "offline": false, "dirty": true, "containsItems": true, "currentPlanFile": "a.plan" });
        let mismatch = plan_view(&fake(base.clone(), true, json!({ "ok": true, "result": 0 }), json!({ "ok": true, "result": 2 })), &[]);
        assert_eq!(mismatch["upload"]["proceedTitle"], "Upload anyway");
        assert_eq!(mismatch["upload"]["pausesFirst"], false);
        assert_eq!(mismatch["actions"]["open"], false);
        assert_eq!(mismatch["sync"]["state"], "busy");
        let flying = plan_view(&fake(base, true, json!({ "ok": true, "result": 0 }), json!({ "ok": true, "result": 3 })), &[]);
        assert_eq!(flying["upload"]["proceedTitle"], "Pause and upload");
        assert_eq!(flying["upload"]["pausesFirst"], true);
        assert_eq!(flying["upload"]["heading"], "Upload this plan?");
    }

    #[test]
    fn a_failed_check_is_reported_not_assumed_ready() {
        let view = plan_view(&fake(json!({ "kind": "null" }), false, json!({ "ok": false, "reason": "no plan" }), json!({ "ok": false })), &[]);
        assert_eq!(view["readiness"]["ready"], false);
        assert_eq!(view["readiness"]["state"], Value::Null);
        assert!(view["readiness"]["reason"].as_str().unwrap().contains("could not be checked"));
        assert_eq!(view["upload"]["canSend"], false);
    }
}
