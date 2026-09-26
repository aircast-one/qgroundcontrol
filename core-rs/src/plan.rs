use serde_json::{Value, json};

use crate::read::{flag, object, result_integer};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "plan.syncInProgress",
    "plan.offline",
    "plan.dirty",
    "plan.containsItems",
    "plan.currentPlanFile",
    "plan.canUndo",
    "plan.canRedo",
    "plan.missionController.containsItems",
    "vehicles.activeVehicleAvailable",
    "vehicle.armed",
    "vehicle.flightMode",
    "plan.managerVehicle.capabilitiesKnown",
    "plan.geoFenceController.supported",
    "plan.rallyPointController.supported",
    "plan.controllerVehicle.vehicleTypeString",
    "plan.controllerVehicle.firmwareTypeString",
    "plan.controllerVehicle.multiRotor",
    "plan.controllerVehicle.vtol",
    "plan.controllerVehicle.apmFirmware",
    "plan.controllerVehicle.homePosition",
    "settings.appSettings.defaultMissionItemAltitude",
    "settings.appSettings.offlineEditingCruiseSpeed",
    "settings.appSettings.offlineEditingHoverSpeed",
];

fn planning_for(backend: &dyn Backend) -> Value {
    // type and firmware alone left the reader on plan.controllerVehicle for the three flags it
    // branches on, so the view existed and retired nothing. A view retires a path when it carries
    // every field the reader dereferences, not when it carries the natural summary.
    let read = object(&backend.get_fields("plan.controllerVehicle", "vehicleTypeString,firmwareTypeString,multiRotor,vtol,apmFirmware,homePosition"));
    let text = |key: &str| read.get(key).and_then(Value::as_str).filter(|value| !value.is_empty()).map(str::to_string);
    match text("vehicleTypeString").zip(text("firmwareTypeString")) {
        Some((kind, firmware)) => json!({
            "type": kind,
            "firmware": firmware,
            "multiRotor": flag(&read, "multiRotor"),
            "vtol": flag(&read, "vtol"),
            "apmFirmware": flag(&read, "apmFirmware"),
            // LaunchPosition(home:) reads home["valid"] and nothing else, so an object carrying
            // only the coordinate answers false for every vehicle that has a home. Serving the
            // path the reader names is not enough; it has to carry the keys the reader subscripts.
            "home": crate::read::nested_coordinate_at(&read, "homePosition").map(|(latitude, longitude)| json!({ "kind": "coordinate", "valid": true, "latitude": latitude, "longitude": longitude })),
        }),
        None => Value::Null,
    }
}

pub fn capability(backend: &dyn Backend, controller: &str) -> Option<bool> {
    let known = flag(&object(&backend.get_fields("plan.managerVehicle", "capabilitiesKnown")), "capabilitiesKnown");
    let supported = flag(&object(&backend.get_fields(&format!("plan.{controller}"), "supported")), "supported");
    known.then_some(supported)
}

// Mission.swift reads these three as raw Facts and rebuilds their units and bounds itself, then
// picks a speed unit with `cruise.units ?? hover.units ?? "m/s"` - a guess for a case that cannot
// arise, since App.SettingsGroup.json declares units on both. The guess is not the defect; the
// re-derivation is. These go through control::decode, the same serialiser the settings controls
// use, so the bounds and the unit are resolved once here rather than per reader.
fn plan_default(backend: &dyn Backend, name: &str) -> Value {
    let path = format!("settings.appSettings.{name}");
    crate::control::decode(&object(&backend.get(&path)), &path)
}

fn defaults_json(backend: &dyn Backend) -> Value {
    let altitude = plan_default(backend, "defaultMissionItemAltitude");
    let cruise = plan_default(backend, "offlineEditingCruiseSpeed");
    let hover = plan_default(backend, "offlineEditingHoverSpeed");
    // One unit for both speeds, and null rather than a guess when the two disagree or neither
    // reports - a speed drawn in an invented unit is worse than a speed drawn with none.
    let unit_of = |c: &Value| c.get("units").and_then(Value::as_str).filter(|u| !u.is_empty()).map(str::to_string);
    let speed_units = match (unit_of(&cruise), unit_of(&hover)) {
        (Some(c), Some(h)) if c == h => json!(c),
        (Some(c), None) => json!(c),
        (None, Some(h)) => json!(h),
        _ => Value::Null,
    };
    json!({ "altitude": altitude, "cruise": cruise, "hover": hover, "speedUnits": speed_units })
}

pub fn plan_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let plan = object(&backend.get_fields("plan", "syncInProgress,offline,dirty,containsItems,currentPlanFile,canUndo,canRedo"));
    let mission = object(&backend.get_fields("plan.missionController", "containsItems"));
    let syncing = flag(&plan, "syncInProgress");
    let offline = flag(&plan, "offline");
    let dirty = flag(&plan, "dirty");
    let contains_items = flag(&plan, "containsItems");
    let has_mission_items = flag(&mission, "containsItems");
    let file = plan.get("currentPlanFile").and_then(Value::as_str).unwrap_or("");
    let name = file.rsplit('/').next().filter(|n| !n.is_empty());
    let (fences, rally) = (capability(backend, "geoFenceController"), capability(backend, "rallyPointController"));
    let (offers_fence, offers_rally) = (fences.unwrap_or(true), rally.unwrap_or(true));
    let readiness = result_integer(&backend.invoke("plan.readyForSaveState", "[]"));
    let upload = result_integer(&backend.invoke("plan.missionController.sendToVehiclePreCheck", "[]"));
    json!({
        "kind": "object",
        "class": "PlanStatus",
        // The plan is edited against a vehicle even with none connected: offline it is the
        // controllerVehicle, and a head showing "which aircraft is this plan for" had to read that
        // object itself. Empty strings mean the controller has not resolved one, which is not the
        // same as a plan for no vehicle.
        "planningFor": planning_for(backend),
        "defaults": defaults_json(backend),
        "readiness": readiness_json(readiness),
        "upload": upload_json(upload),
        "actions": {
            "open": !syncing,
            "save": !syncing && contains_items,
            "exportKml": !syncing && has_mission_items,
            "newPlan": !syncing,
            "clearMission": !offline && !syncing,
            "addFence": offers_fence && !syncing,
            "addRally": offers_rally && !syncing,
        },
        "fenceSupported": fences,
        "rallySupported": rally,
        "unsupportedReason": match (fences, rally) {
            (None, _) | (_, None) => Some("This vehicle has not said what it accepts yet."),
            (Some(false), Some(false)) => Some("This link accepts neither a geofence nor rally points."),
            (Some(false), Some(true)) => Some("This link does not accept a geofence."),
            (Some(true), Some(false)) => Some("This link does not accept rally points."),
            (Some(true), Some(true)) => None,
        },
        "sync": sync_json(offline, syncing),
        "status": status_text(name, dirty, offline, contains_items),
        "file": name,
        "dirty": dirty,
        "canUndo": flag(&plan, "canUndo"),
        "canRedo": flag(&plan, "canRedo"),
    })
}

fn readiness_json(state: Option<i64>) -> Value {
    let reason = match state {
        Some(0) => None,
        Some(1) => Some("Waiting for terrain heights before the plan can be saved or sent."),
        Some(2) => Some("An item is still being drawn, so the plan cannot be saved or sent."),
        _ => Some("The plan could not be checked for saving."),
    };
    json!({ "state": state, "ready": state == Some(0), "reason": reason })
}

fn upload_json(state: Option<i64>) -> Value {
    let (refusal, proceed_title) = match state {
        Some(0) => (None, None),
        Some(1) => (Some("No vehicle is connected, so there is nowhere to send this plan."), None),
        Some(2) => (Some("This plan was made for a different firmware or vehicle type. Uploading it can make the vehicle behave incorrectly."), Some("Upload anyway")),
        Some(3) => (Some("The vehicle is flying this mission. It has to be paused before a new plan goes up."), Some("Pause and upload")),
        _ => (Some("The plan could not be checked against the vehicle."), None),
    };
    let can_proceed = matches!(state, Some(2) | Some(3));
    json!({
        "state": state,
        "canSend": state == Some(0),
        "refusal": refusal,
        "heading": refusal.map(|_| if can_proceed { "Upload this plan?" } else { "This plan cannot be uploaded" }),
        "proceedTitle": proceed_title,
        "canProceed": can_proceed,
        "pausesFirst": state == Some(3),
    })
}

fn sync_json(offline: bool, syncing: bool) -> Value {
    let (state, refusal) = match (offline, syncing) {
        (true, _) => ("offline", Some("No vehicle is connected.")),
        (false, true) => ("busy", Some("Already syncing, wait for it to finish.")),
        (false, false) => ("ready", None),
    };
    json!({ "state": state, "refusal": refusal })
}

fn status_text(name: Option<&str>, dirty: bool, offline: bool, has_items: bool) -> String {
    match (name, dirty, offline, has_items) {
        (None, _, _, false) => "New plan".to_string(),
        (None, _, true, true) => "Unsaved plan".to_string(),
        (None, true, false, true) => "Not uploaded".to_string(),
        (None, false, false, true) => "Sent to the vehicle".to_string(),
        (Some(n), false, _, _) => n.to_string(),
        (Some(n), true, true, _) => format!("{n} \u{b7} unsaved changes"),
        (Some(n), true, false, _) => format!("{n} \u{b7} not uploaded"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanAction {
    Send,
    Download,
    SaveCurrent,
    SaveFile,
    SaveKml,
}

fn folder_refusal(path: Option<&str>) -> Option<(&'static str, String)> {
    let Some(path) = path.filter(|p| !p.trim().is_empty()) else {
        return Some(("noFile", "Choose a file to save to.".to_string()));
    };
    match std::path::Path::new(path).parent().filter(|d| !d.as_os_str().is_empty()) {
        Some(folder) if !folder.is_dir() => Some(("folderMissing", format!("{} is not a folder that exists.", folder.display()))),
        _ => None,
    }
}

fn plan_refusal(action: PlanAction, view: &Value, path: Option<&str>) -> Option<(&'static str, String)> {
    let text = |v: &Value| v.as_str().unwrap_or("").to_string();
    let allowed = |name: &str| view["actions"][name].as_bool() == Some(true);
    let sync_refusal = || (view["sync"]["state"] != "ready").then(|| (if view["sync"]["state"] == "offline" { "offline" } else { "busy" }, text(&view["sync"]["refusal"])));
    let not_ready = || (view["readiness"]["ready"] != true).then(|| ("notReady", text(&view["readiness"]["reason"])));
    match action {
        PlanAction::Download => sync_refusal(),
        PlanAction::Send => sync_refusal().or_else(not_ready).or_else(|| match view["upload"]["state"].as_i64() {
            Some(0 | 2 | 3) => None,
            _ => Some(("cannotUpload", text(&view["upload"]["refusal"]))),
        }),
        PlanAction::SaveCurrent | PlanAction::SaveFile => not_ready()
            .or_else(|| (!allowed("save")).then(|| ("nothingToSave", "There is nothing in this plan to save, or a sync is running.".to_string())))
            .or_else(|| match action {
                PlanAction::SaveCurrent => view["file"].is_null().then(|| ("noFile", "This plan has not been saved to a file yet.".to_string())),
                _ => folder_refusal(path),
            }),
        PlanAction::SaveKml => (!allowed("exportKml"))
            .then(|| ("nothingToExport", "There are no mission items to export, or a sync is running.".to_string()))
            .or_else(|| folder_refusal(path)),
    }
}

pub fn plan_action(backend: &dyn Backend, action: PlanAction, path: &str, args: &str) -> Value {
    let file = serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0)?.as_str().map(str::to_string));
    let returns = matches!(action, PlanAction::SaveCurrent | PlanAction::SaveFile);
    if let Some((token, reason)) = plan_refusal(action, &plan_view(backend, &[]), file.as_deref()) {
        return json!({ "ok": false, "result": returns.then_some(false), "refusal": token, "reason": reason });
    }
    let forwarded = match (action, &file) {
        (PlanAction::SaveFile | PlanAction::SaveKml, Some(file)) => json!([file]).to_string(),
        _ => "[]".to_string(),
    };
    let answer = object(&backend.invoke(path, &forwarded));
    let done = match returns {
        true => flag(&answer, "result"),
        false => flag(&answer, "ok"),
    };
    json!({
        "ok": done,
        "result": returns.then_some(done),
        "refusal": Value::Null,
        "reason": match done { true => Value::Null, false => json!("The plan controller did not carry it out.") },
    })
}

#[cfg(test)]
mod tests {

    fn declared(name: &str, value: f64, shown: &str, units: &str) -> Value {
        const APP: &str = include_str!("../../src/Settings/App.SettingsGroup.json");
        let facts: Vec<Value> = serde_json::from_str::<Value>(APP).unwrap()["QGC.MetaData.Facts"].as_array().cloned().unwrap();
        let fact = facts.iter().find(|f| f["name"] == json!(name)).unwrap_or_else(|| panic!("{name} is not declared in App.SettingsGroup.json"));
        json!({
            "kind": "fact", "name": name, "value": value, "valueString": shown, "units": units,
            "min": fact["min"], "minIsDefaultForType": fact.get("min").is_none(),
            "max": fact.get("max").cloned().unwrap_or(json!(f64::MAX)), "maxIsDefaultForType": fact.get("max").is_none(),
            "decimalPlaces": fact["decimalPlaces"],
        })
    }

    #[test]
    fn none_of_the_three_plan_defaults_declares_a_ceiling() {
        const APP: &str = include_str!("../../src/Settings/App.SettingsGroup.json");
        let facts: Vec<Value> = serde_json::from_str::<Value>(APP).unwrap()["QGC.MetaData.Facts"].as_array().cloned().unwrap();
        let named = |name: &str| facts.iter().find(|f| f["name"] == json!(name)).unwrap_or_else(|| panic!("{name} is not declared in App.SettingsGroup.json"));

        ["defaultMissionItemAltitude", "offlineEditingCruiseSpeed", "offlineEditingHoverSpeed"].iter().for_each(|name| {
            assert!(
                named(name).get("max").is_none(),
                "{name} now declares a max, so view.plan.defaults stops serving a null ceiling and the fake in the test below no longer matches what QGC writes - update both together"
            );
        });
    }

    #[test]
    fn the_plan_defaults_resolve_their_unit_once_instead_of_per_reader() {
        struct Defaults(&'static str, &'static str);
        impl Backend for Defaults {
            fn get(&self, path: &str) -> String {
                let speed = |units: &str| declared("offlineEditingCruiseSpeed", 15.0, "15.00", units);
                match path {
                    "settings.appSettings.defaultMissionItemAltitude" => declared("defaultMissionItemAltitude", 50.0, "50.0", "m").to_string(),
                    "settings.appSettings.offlineEditingCruiseSpeed" => speed(self.0).to_string(),
                    "settings.appSettings.offlineEditingHoverSpeed" => speed(self.1).to_string(),
                    _ => json!({ "kind": "null" }).to_string(),
                }
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        let agreed = defaults_json(&Defaults("m/s", "m/s"));
        assert_eq!(agreed["speedUnits"], "m/s");
        assert_eq!(agreed["altitude"]["units"], "m", "the altitude keeps its own unit; only the two speeds share one");
        assert_eq!(agreed["cruise"]["valueString"], "15.00");
        assert_eq!(agreed["altitude"]["minimum"], 0.0, "the bound travels decoded, so a head does not rebuild FactRange from min and minIsDefaultForType");
        assert_eq!(
            (agreed["altitude"]["maximum"].clone(), agreed["cruise"]["maximum"].clone(), agreed["hover"]["maximum"].clone()),
            (Value::Null, Value::Null, Value::Null),
            "App.SettingsGroup.json declares a min on all three and a max on none, so maxIsDefaultForType is the type's own limit rather than a real one and offering it as a ceiling invents a rule the setting does not have. This used to assert the altitude ceiling was 1000, a number the fake invented and QGC has never declared"
        );

        let disagreeing = defaults_json(&Defaults("m/s", "ft/s"));
        assert_eq!(disagreeing["speedUnits"], Value::Null, "two speeds in different units have no shared unit, and picking the first would draw one of them wrong");
        assert_eq!(disagreeing["cruise"]["units"], "m/s", "each still carries its own, so nothing is lost by refusing to pick");

        let silent = defaults_json(&Defaults("", ""));
        assert_eq!(silent["speedUnits"], Value::Null, "Mission.swift falls back to \"m/s\" here; a guessed unit on a number an operator flies by is worse than none");
    }
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

    fn supporting(plan: Value, fences: bool, rally: bool) -> Fake {
        told(plan, fences, rally, true)
    }

    fn told(plan: Value, fences: bool, rally: bool, known: bool) -> Fake {
        Fake {
            fields: BTreeMap::from([
                ("plan", plan),
                ("plan.managerVehicle", json!({ "kind": "object", "capabilitiesKnown": known })),
                ("plan.controllerVehicle", json!({ "kind": "object", "vehicleTypeString": "Multi-Rotor", "firmwareTypeString": "PX4 Pro", "multiRotor": true, "vtol": false, "apmFirmware": false, "homePosition": { "valid": true, "latitude": 47.397, "longitude": 8.546, "altitude": 12.0 } })),
                ("plan.missionController", json!({ "kind": "object", "containsItems": true })),
                ("plan.geoFenceController", json!({ "kind": "object", "supported": fences })),
                ("plan.rallyPointController", json!({ "kind": "object", "supported": rally })),
            ]),
            readiness: json!({ "ok": true, "result": 0 }),
            upload: json!({ "ok": true, "result": 0 }),
        }
    }

    #[test]
    fn a_vehicle_that_refuses_a_geofence_says_so_before_the_button_is_pressed() {
        let connected = json!({ "kind": "object", "syncInProgress": false, "offline": false, "dirty": false, "containsItems": true, "currentPlanFile": "" });

        let takes_both = plan_view(&supporting(connected.clone(), true, true), &[]);
        assert_eq!(takes_both["actions"]["addFence"], true);
        assert_eq!(takes_both["unsupportedReason"], Value::Null, "nothing to explain when the vehicle accepts both, and an empty sentence is a sentence a head will draw");

        let neither = plan_view(&supporting(connected.clone(), false, false), &[]);
        assert_eq!(neither["actions"]["addFence"], false, "PlanElementController::supported is a vehicle capability, and both heads were offering a button for something the vehicle refuses when pressed");
        assert_eq!(neither["actions"]["addRally"], false);
        assert_eq!(neither["fenceSupported"], false);
        assert!(neither["unsupportedReason"].as_str().unwrap().contains("neither"));
        assert!(neither["unsupportedReason"].as_str().unwrap().starts_with("This link"), "GeoFenceController::supported is a capability bit AND maxProtoVersion >= 200, so a false can mean the vehicle lacks the feature or that the link speaks MAVLink 1 - naming the vehicle picks one of the two causes without reading either, and the operator acts on the wrong one");

        let fence_only = plan_view(&supporting(connected.clone(), true, false), &[]);
        assert_eq!(fence_only["actions"]["addFence"], true, "the two are separate capabilities and a vehicle can accept one and not the other");
        assert_eq!(fence_only["actions"]["addRally"], false);
        assert!(fence_only["unsupportedReason"].as_str().unwrap().contains("rally points"));

        let unasked = plan_view(&told(connected, true, true, false), &[]);
        assert_eq!(unasked["fenceSupported"], Value::Null, "Vehicle.cc initialises _capabilityBits to MISSION_FENCE|MISSION_RALLY and GeoFenceController::supported never consults capabilitiesKnown, so a vehicle that has said nothing reads as one that accepts both - true here would be a default answering for someone who was never asked");
        assert_eq!(unasked["rallySupported"], Value::Null);
        assert_eq!(unasked["actions"]["addFence"], true, "the button stays, because QGC itself lets you draw a fence for a vehicle that has not answered - withholding it would remove a capability that works");
        assert!(unasked["unsupportedReason"].as_str().unwrap().contains("not said"));
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
        assert_eq!(view["readiness"]["reason"], Value::Null);
        assert_eq!(view["upload"]["canSend"], false);
        assert_eq!(view["upload"]["canProceed"], false);
        assert_eq!(view["upload"]["heading"], "This plan cannot be uploaded", "state 1 is a genuine refusal - no vehicle - so there is a heading to draw");
        assert_eq!(view["actions"]["newPlan"], true);
        assert_eq!(view["actions"]["save"], false);
        assert_eq!(view["actions"]["clearMission"], false);
        assert_eq!(view["sync"]["state"], "offline");
        assert_eq!(view["status"], "New plan");

        let unnamed = |dirty: bool, offline: bool, items: bool| status_text(None, dirty, offline, items);
        assert_eq!(unnamed(false, false, true), "Sent to the vehicle", "with a vehicle connected dirty means not synced rather than not saved, so a plan whose items have just gone up read New plan - the operator's finished work described as though they had not started");
        assert_eq!(unnamed(true, false, true), "Not uploaded");
        assert_eq!(unnamed(true, false, false), "New plan", "a plan just cleared to nothing is dirty because clearing is a change, and it read Unsaved plan - the blank canvas claiming work was at risk");
        assert_eq!(unnamed(false, false, false), "New plan");
        assert_eq!(unnamed(true, true, true), "Unsaved plan", "offline the flag means what the words say, and this is the case the sentence was written for");
        assert_eq!(unnamed(true, true, false), "New plan");
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
    fn the_plan_names_the_vehicle_it_is_being_edited_against() {
        let view = plan_view(&supporting(json!({ "kind": "object", "offline": true, "dirty": false, "containsItems": true }), true, true), &[]);
        assert_eq!(view["planningFor"]["type"], "Multi-Rotor", "a plan is edited against a vehicle even with none connected, and a head asking which one had to read the controller object itself");
        assert_eq!(view["planningFor"]["firmware"], "PX4 Pro");
        assert_eq!((view["planningFor"]["multiRotor"].clone(), view["planningFor"]["vtol"].clone(), view["planningFor"]["apmFirmware"].clone()), (json!(true), json!(false), json!(false)), "the reader branches on these three, so serving the names alone left it on plan.controllerVehicle and retired nothing");
        assert!(view["defaults"]["altitude"]["path"].is_string(), "plan_view has to CARRY the defaults block; a head reads view.plan and never calls defaults_json, so testing that function alone leaves the wiring unpinned");
        assert_eq!(view["planningFor"]["home"]["valid"], true, "LaunchPosition subscripts home[\"valid\"]; serving the coordinate without it answers no-home for every vehicle that has one");
        assert_eq!(view["planningFor"]["home"]["latitude"], 47.397, "the sixth read on that group - five of six is not retirement");

        let mut homeless = supporting(json!({ "kind": "object", "offline": true, "dirty": false, "containsItems": true }), true, true);
        homeless.fields.insert("plan.controllerVehicle", json!({ "kind": "object", "vehicleTypeString": "Multi-Rotor", "firmwareTypeString": "PX4 Pro", "multiRotor": true, "vtol": false, "apmFirmware": false }));
        assert_eq!(plan_view(&homeless, &[])["planningFor"]["home"], Value::Null, "an unset home is absent rather than a point at nowhere");

        let mut blank = supporting(json!({ "kind": "object", "offline": true, "dirty": false, "containsItems": true }), true, true);
        blank.fields.insert("plan.controllerVehicle", json!({ "kind": "object", "vehicleTypeString": "", "firmwareTypeString": "" }));
        assert_eq!(plan_view(&blank, &[])["planningFor"], Value::Null, "an unresolved controller is not a plan for a vehicle with an empty name");
    }

    #[test]
    fn a_failed_check_is_reported_not_assumed_ready() {
        let view = plan_view(&fake(json!({ "kind": "null" }), false, json!({ "ok": false, "reason": "no plan" }), json!({ "ok": false })), &[]);
        assert_eq!(view["readiness"]["ready"], false);
        assert_eq!(view["readiness"]["state"], Value::Null);
        assert!(view["readiness"]["reason"].as_str().unwrap().contains("could not be checked"));
        assert_eq!(view["upload"]["canSend"], false);
    }
    #[test]
    fn the_undo_stacks_travel_with_the_plan_so_the_head_stops_reading_the_raw_object() {
        let with_history = json!({ "kind": "object", "syncInProgress": false, "offline": true, "dirty": true, "containsItems": true, "currentPlanFile": "", "canUndo": true, "canRedo": false });
        let view = plan_view(&fake(with_history, true, json!({ "ok": true, "result": 0 }), json!({ "ok": true, "result": 0 })), &[]);
        assert_eq!(
            (view["canUndo"].clone(), view["canRedo"].clone()), (json!(true), json!(false)),
            "Mission.swift took five keys off Bridge.group(\"plan\") and view.plan already served three of them; these two were the whole reason the raw read survived, and a head mixing a view with a separate raw snapshot reads one plan at two instants"
        );

        let fresh = json!({ "kind": "object", "syncInProgress": false, "offline": true, "dirty": false, "containsItems": false, "currentPlanFile": "" });
        let empty = plan_view(&fake(fresh, false, json!({ "ok": true, "result": 0 }), json!({ "ok": true, "result": 0 })), &[]);
        assert_eq!((empty["canUndo"].clone(), empty["canRedo"].clone()), (json!(false), json!(false)), "an object that does not report them has nothing to undo, which is what a fresh plan is");
    }

    #[test]
    fn the_state_that_means_nothing_is_wrong_has_no_refusal_to_show() {
        let ready = upload_json(Some(0));
        assert_eq!(ready["canSend"], true);
        assert_eq!(ready["heading"], Value::Null, "heading was a two-way choice on canProceed, so the state that means send it fell through the else and got the refusal sentence: canSend true beside 'This plan cannot be uploaded'");
        assert_eq!(ready["refusal"], Value::Null);
        assert_eq!(ready["proceedTitle"], Value::Null);

        let no_vehicle = upload_json(Some(1));
        assert_eq!(no_vehicle["heading"], "This plan cannot be uploaded", "a refusal with nothing to offer still needs a heading");
        assert_eq!(no_vehicle["proceedTitle"], Value::Null, "and no button title, because there is no button");

        let wrong_firmware = upload_json(Some(2));
        assert_eq!(wrong_firmware["heading"], "Upload this plan?", "a question is only a question when there is something to answer");
        assert_eq!(wrong_firmware["proceedTitle"], "Upload anyway");

        let unchecked = upload_json(None);
        assert_eq!(unchecked["heading"], "This plan cannot be uploaded", "an unknown state is a refusal, not a green light");
    }

    #[test]
    fn a_plan_is_not_sent_or_saved_while_the_view_says_it_cannot_be() {
        let view = |sync: &str, ready: bool, upload: i64, file: Value| json!({
            "sync": { "state": sync, "refusal": if sync == "ready" { Value::Null } else { json!("No vehicle is connected.") } },
            "readiness": { "ready": ready, "reason": if ready { Value::Null } else { json!("Waiting for terrain heights before the plan can be saved or sent.") } },
            "upload": { "state": upload, "refusal": if upload == 0 { Value::Null } else { json!("No vehicle is connected, so there is nowhere to send this plan.") } },
            "actions": { "save": true, "exportKml": true },
            "file": file,
        });
        let token = |action, v: &Value, path: Option<&str>| plan_refusal(action, v, path).map(|(t, _)| t);
        let good = view("ready", true, 0, json!("ridge.plan"));
        assert_eq!(token(PlanAction::Send, &good, None), None);
        assert_eq!(
            plan_refusal(PlanAction::Send, &view("ready", false, 0, Value::Null), None),
            Some(("notReady", "Waiting for terrain heights before the plan can be saved or sent.".to_string())),
            "PlanMasterController::sendToVehicle checks offline and syncing only, so a plan with its terrain heights still pending went up"
        );
        assert_eq!(token(PlanAction::Send, &view("ready", true, 2, Value::Null), None), None, "a firmware mismatch is a warning the head has already put to the operator before it sends");
        assert_eq!(token(PlanAction::Send, &view("ready", true, 3, Value::Null), None), None, "and the head pauses first for a mission in flight, after which the pre-check can still read 3 for a moment");
        assert_eq!(token(PlanAction::Send, &view("ready", true, 1, Value::Null), None), Some("cannotUpload"));
        assert_eq!(token(PlanAction::Send, &view("busy", true, 0, Value::Null), None), Some("busy"));
        assert_eq!(token(PlanAction::Download, &view("offline", true, 0, Value::Null), None), Some("offline"));
        assert_eq!(token(PlanAction::SaveCurrent, &view("ready", true, 0, Value::Null), None), Some("noFile"));
        assert_eq!(token(PlanAction::SaveCurrent, &good, None), None);
        assert_eq!(token(PlanAction::SaveFile, &good, Some("/no/such/folder/ridge.plan")), Some("folderMissing"));
        let here = std::env::temp_dir().join("ridge.plan");
        assert_eq!(token(PlanAction::SaveFile, &good, here.to_str()), None);
        assert_eq!(token(PlanAction::SaveKml, &good, Some("")), Some("noFile"));
        assert_eq!(token(PlanAction::SaveFile, &view("ready", false, 0, Value::Null), here.to_str()), Some("notReady"));
    }
}
