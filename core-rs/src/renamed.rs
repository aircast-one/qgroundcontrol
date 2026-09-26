use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

const MISSION: &str = "plan.missionController";

fn pattern_names(items: Option<&Value>) -> Value {
    let names: Vec<Value> = items
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(|item| item.as_str().or_else(|| item.get("canonicalName")?.as_str())).map(|name| json!(name)).collect())
        .unwrap_or_default();
    Value::Array(names)
}

const ITEMS: &str = "plan.missionController.visualItems.";

fn renames(object_path: &str) -> &'static [(&'static str, &'static str)] {
    let item = object_path.strip_prefix(ITEMS).is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()));
    match () {
        _ if object_path == MISSION => &[("complexMissionItemNames", "complexMissionItems"), ("globalAltitudeMode", "globalAltitudeFrame")],
        _ if item => &[("altitudeMode", "altitudeFrame")],
        _ => &[],
    }
}

fn renamed_value(controller: &Value, old: &str, new: &str) -> Option<Value> {
    match old {
        "complexMissionItemNames" => Some(pattern_names(controller.get(new))),
        _ => controller.get(new).cloned(),
    }
}

fn split(path: &str) -> Option<(&str, &str)> {
    path.rsplit_once('.')
}

pub fn read(backend: &dyn Backend, path: &str) -> Option<String> {
    let (object_path, old) = split(path)?;
    let (_, new) = renames(object_path).iter().find(|(name, _)| *name == old)?;
    let controller = object(&backend.get_fields(object_path, new));
    if controller.get("kind").and_then(Value::as_str) != Some("object") {
        return Some(json!({ "kind": "null" }).to_string());
    }
    Some(json!({ "kind": "value", "value": renamed_value(&controller, old, new).unwrap_or(Value::Null) }).to_string())
}

pub fn patch(path: &str, raw: String) -> String {
    let table = renames(path);
    if table.is_empty() {
        return raw;
    }
    let mut controller = object(&raw);
    if controller.get("kind").and_then(Value::as_str) != Some("object") {
        return raw;
    }
    table.iter().for_each(|(old, new)| {
        if controller.get(*old).is_none() {
            if let Some(value) = renamed_value(&controller, old, new) {
                controller[*old] = value;
            }
        }
    });
    controller.to_string()
}

pub fn fields(path: &str, fields: &str) -> Option<String> {
    let table = renames(path);
    let asked: Vec<&str> = fields.split(',').map(str::trim).collect();
    if !asked.iter().any(|f| table.iter().any(|(old, _)| old == f)) {
        return None;
    }
    let mut widened: Vec<&str> = asked.iter().map(|f| table.iter().find(|(old, _)| old == f).map_or(*f, |(_, new)| *new)).collect();
    widened.sort_unstable();
    widened.dedup();
    Some(widened.join(","))
}

pub fn write_path(path: &str) -> Option<String> {
    let (object_path, old) = split(path)?;
    let (_, new) = renames(object_path).iter().find(|(name, _)| *name == old)?;
    (old != "complexMissionItemNames").then(|| format!("{object_path}.{new}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Controller;
    impl Backend for Controller {
        fn get(&self, _p: &str) -> String { self.get_fields(MISSION, "") }
        fn get_fields(&self, _p: &str, _f: &str) -> String {
            json!({ "kind": "object", "globalAltitudeFrame": 1, "complexMissionItems": [
                { "canonicalName": "Survey", "translatedName": "Vermessung" },
                { "canonicalName": "Corridor Scan", "translatedName": "Korridor" },
            ] })
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_head_reading_the_pre_merge_names_is_answered_from_the_properties_they_became() {
        assert_eq!(
            object(&read(&Controller, "plan.missionController.complexMissionItemNames").unwrap())["value"],
            json!(["Survey", "Corridor Scan"]),
            "MissionController has no complexMissionItemNames since the upstream merge, so both heads read an empty list and offered no survey, corridor or structure scan at all; the canonical name is served because that is what insertComplexMissionItem takes"
        );
        assert_eq!(object(&read(&Controller, "plan.missionController.globalAltitudeMode").unwrap())["value"], 1);
        assert_eq!(read(&Controller, "plan.missionController.visualItems"), None, "a path that was not renamed passes through untouched");
        assert_eq!(read(&Controller, "plan.missionControllerX.globalAltitudeMode"), None);

        let group = object(&patch(MISSION, Controller.get(MISSION)));
        assert_eq!((&group["complexMissionItemNames"], &group["globalAltitudeMode"]), (&json!(["Survey", "Corridor Scan"]), &json!(1)), "the macOS head reads both off the whole controller object");
        assert_eq!(patch("plan", r#"{"kind":"object"}"#.to_string()), r#"{"kind":"object"}"#);
        assert_eq!(pattern_names(Some(&json!(["Survey"]))), json!(["Survey"]), "a build that still serves plain names reads the same");

        assert_eq!(fields(MISSION, "complexMissionItemNames,containsItems"), Some("complexMissionItems,containsItems".to_string()));
        assert_eq!(fields(MISSION, "containsItems"), None);
        assert_eq!(fields("vehicle", "globalAltitudeMode"), None, "only the controller that was renamed is rewritten");
    }

    #[test]
    fn the_router_serves_the_old_names_through_every_read_a_head_makes() {
        let core = crate::router::Core::new(Controller);
        assert_eq!(object(&core.get("plan.missionController.complexMissionItemNames"))["value"], json!(["Survey", "Corridor Scan"]), "PlanFiles.kt reads the full path");
        assert_eq!(object(&core.get(MISSION))["complexMissionItemNames"], json!(["Survey", "Corridor Scan"]), "Mission.swift reads the whole controller");
        assert_eq!(object(&core.get_fields(MISSION, "globalAltitudeMode"))["globalAltitudeMode"], 1);
    }

    #[test]
    fn a_mission_item_read_or_written_by_its_pre_merge_altitude_name_reaches_altitude_frame() {
        struct Item;
        impl Backend for Item {
            fn get(&self, _p: &str) -> String { self.get_fields("", "") }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "object", "altitudeFrame": 2, "sequenceNumber": 3 }).to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let path = "plan.missionController.visualItems.4";
        assert_eq!(object(&patch(path, Item.get(path)))["altitudeMode"], 2, "SimpleMissionItem has no altitudeMode since the upstream merge; the property is altitudeFrame, and Mission.swift reads the old name off the item object");
        assert_eq!(object(&read(&Item, "plan.missionController.visualItems.4.altitudeMode").unwrap())["value"], 2);
        assert_eq!(write_path("plan.missionController.visualItems.4.altitudeMode").as_deref(), Some("plan.missionController.visualItems.4.altitudeFrame"), "and writes the old name, which reached nothing");
        assert_eq!(write_path("plan.missionController.globalAltitudeMode").as_deref(), Some("plan.missionController.globalAltitudeFrame"));
        assert_eq!(write_path("plan.missionController.visualItems.x.altitudeMode"), None, "only an item by position is an item");
        assert_eq!(write_path("plan.missionController.visualItems.4.altitude"), None);
        assert_eq!(write_path("plan.missionController.complexMissionItemNames"), None, "a list of names has no write to rename");
        assert_eq!(patch("plan.missionController.visualItems", r#"{"kind":"object"}"#.to_string()), r#"{"kind":"object"}"#);
    }
}
