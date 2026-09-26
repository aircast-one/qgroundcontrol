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

fn renamed_value(controller: &Value, old: &str) -> Option<Value> {
    match old {
        "complexMissionItemNames" => Some(pattern_names(controller.get("complexMissionItems"))),
        "globalAltitudeMode" => controller.get("globalAltitudeFrame").cloned(),
        _ => None,
    }
}

const OLD_NAMES: &[(&str, &str)] = &[("complexMissionItemNames", "complexMissionItems"), ("globalAltitudeMode", "globalAltitudeFrame")];

pub fn read(backend: &dyn Backend, path: &str) -> Option<String> {
    let old = path.strip_prefix(MISSION)?.strip_prefix('.')?;
    let (_, new) = OLD_NAMES.iter().find(|(name, _)| *name == old)?;
    let controller = object(&backend.get_fields(MISSION, new));
    if controller.get("kind").and_then(Value::as_str) != Some("object") {
        return Some(json!({ "kind": "null" }).to_string());
    }
    Some(json!({ "kind": "value", "value": renamed_value(&controller, old).unwrap_or(Value::Null) }).to_string())
}

pub fn patch(path: &str, raw: String) -> String {
    if path != MISSION {
        return raw;
    }
    let mut controller = object(&raw);
    if controller.get("kind").and_then(Value::as_str) != Some("object") {
        return raw;
    }
    OLD_NAMES.iter().for_each(|(old, _)| {
        if controller.get(*old).is_none() {
            if let Some(value) = renamed_value(&controller, old) {
                controller[*old] = value;
            }
        }
    });
    controller.to_string()
}

pub fn fields(path: &str, fields: &str) -> Option<String> {
    if path != MISSION {
        return None;
    }
    let asked: Vec<&str> = fields.split(',').map(str::trim).collect();
    if !asked.iter().any(|f| OLD_NAMES.iter().any(|(old, _)| old == f)) {
        return None;
    }
    let mut widened: Vec<&str> = asked.iter().map(|f| OLD_NAMES.iter().find(|(old, _)| old == f).map_or(*f, |(_, new)| *new)).collect();
    asked.iter().filter(|f| !OLD_NAMES.iter().any(|(old, _)| old == *f)).for_each(|f| widened.push(f));
    widened.sort_unstable();
    widened.dedup();
    Some(widened.join(","))
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
}
