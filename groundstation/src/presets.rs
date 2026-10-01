use serde_json::{Value, json};

use crate::plandoc::{self, Item};
use crate::router::Backend;

pub const DEPS: &[&str] = &[];
pub const SAVE_PRESET: &str = "core.plan.savePreset";
pub const APPLY_PRESET: &str = "core.plan.applyPreset";
pub const DELETE_PRESET: &str = "core.plan.deletePreset";
const PRESETS_KEY: &str = "_presets";
const TRANSECT: &str = "TransectStyleComplexItem";
const GENERATED: [&str; 2] = ["Items", "VisualTransectPoints"];
const SURVEY_KEYS: [&str; 4] = ["angle", "flyAlternateTransects", "splitConcavePolygons", "entryLocation"];
const CORRIDOR_KEYS: [&str; 2] = ["CorridorWidth", "EntryPoint"];

pub fn settings_group(kind: &str) -> Option<&'static str> {
    match kind {
        "survey" => Some("Survey"),
        "CorridorScan" => Some("CorridorScan"),
        _ => None,
    }
}

fn group_of(kind: &str) -> Option<String> {
    settings_group(kind).map(|group| format!("{group}/{PRESETS_KEY}"))
}

pub fn names(kind: &str) -> Vec<String> {
    let Some(group) = group_of(kind) else { return Vec::new() };
    let prefix = format!("{group}/");
    crate::settingsstore::entries_under(&group).keys().filter_map(|key| key.strip_prefix(&prefix).map(str::to_string)).collect()
}

pub fn preset_of(item: &Value) -> Value {
    let mut preset = item.clone();
    if let Some(object) = preset.as_object_mut() {
        ["polygon", "polyline", "coordinate"].iter().for_each(|key| {
            object.remove(*key);
        });
    }
    if let Some(transect) = preset.get_mut(TRANSECT).and_then(Value::as_object_mut) {
        GENERATED.iter().for_each(|key| {
            transect.remove(*key);
        });
    }
    preset
}

pub fn applied(item: &Value, preset: &Value, kind: &str) -> Value {
    let mut changed = item.clone();
    let kept: Vec<(&str, Value)> = GENERATED.iter().filter_map(|key| item.get(TRANSECT)?.get(*key).cloned().map(|v| (*key, v))).collect();
    if let Some(transect) = preset.get(TRANSECT) {
        changed[TRANSECT] = transect.clone();
        kept.into_iter().for_each(|(key, value)| changed[TRANSECT][key] = value);
    }
    let own: &[&str] = if kind == "CorridorScan" { &CORRIDOR_KEYS } else { &SURVEY_KEYS };
    own.iter().filter_map(|key| preset.get(*key).map(|value| (*key, value.clone()))).for_each(|(key, value)| changed[key] = value);
    crate::surveydoc::regenerate_item(&changed)
}

fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && !name.contains('/')
}

fn refused(reason: &str) -> Value {
    json!({ "ok": false, "reason": reason })
}

fn complex_at(doc: &plandoc::Document, index: usize) -> Option<(String, Value)> {
    match doc.items.get(index.checked_sub(1)?)? {
        Item::Complex { kind, json, .. } => Some((kind.clone(), json.clone())),
        Item::Simple(_) => None,
    }
}

pub fn run(path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Vec<Value>>(args).unwrap_or_default();
    let text = |at: usize| given.get(at).and_then(Value::as_str).unwrap_or_default().to_string();
    match path {
        DELETE_PRESET => match group_of(&text(0)).filter(|_| valid_name(&text(1))) {
            Some(group) => {
                crate::settingsstore::forgotten(&format!("{group}/{}", text(1)));
                json!({ "ok": true })
            }
            None => refused("Delete preset takes a pattern kind and a preset name."),
        },
        _ => {
            let (Some(index), name) = (given.first().and_then(Value::as_u64).and_then(|i| usize::try_from(i).ok()), text(1)) else { return refused("Presets need the item index and a preset name.") };
            if !valid_name(&name) {
                return refused("Enter a preset name without '/'.");
            }
            if path == SAVE_PRESET {
                let Some((kind, item)) = complex_at(&crate::coreplan::current_document(), index) else { return refused("That item is not a pattern.") };
                let Some(group) = group_of(&kind) else { return refused("This Pattern does not support Presets.") };
                crate::settingsstore::written(&format!("{group}/{name}"), &preset_of(&item).to_string());
                return json!({ "ok": true });
            }
            crate::coreplan::apply(|doc| {
                let (kind, item) = complex_at(doc, index).ok_or("That item is not a pattern.")?;
                let group = group_of(&kind).ok_or("This Pattern does not support Presets.")?;
                let key = format!("{group}/{name}");
                let preset: Value = crate::settingsstore::stored_text(&key).and_then(|t| serde_json::from_str(&t).ok()).ok_or("That preset no longer exists.")?;
                let changed = applied(&item, &preset, &kind);
                let item_count = plandoc::complex_count(&kind, &changed).unwrap_or(0);
                let at = index - 1;
                Ok(plandoc::Document { items: doc.items.iter().enumerate().map(|(k, it)| if k == at { Item::Complex { kind: kind.clone(), json: changed.clone(), item_count } } else { it.clone() }).collect(), ..doc.clone() })
            })
        }
    }
}

pub fn owns(path: &str) -> bool {
    [SAVE_PRESET, APPLY_PRESET, DELETE_PRESET].contains(&path)
}

pub fn pattern_presets_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let kind = args.first().map(String::as_str).unwrap_or_default();
    json!({ "kind": "object", "class": "PatternPresets", "supported": settings_group(kind).is_some(), "names": names(kind) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_preset_keeps_the_settings_and_drops_the_area() {
        let survey = json!({ "complexItemType": "survey", "polygon": [[1, 2]], "angle": 30, "TransectStyleComplexItem": { "CameraCalc": { "DistanceToSurface": 80 }, "Items": [1], "VisualTransectPoints": [[1, 2]], "TurnAroundDistance": 10 } });
        let preset = preset_of(&survey);
        assert!(preset.get("polygon").is_none() && preset[TRANSECT].get("Items").is_none());
        assert_eq!(preset["angle"], 30);
        assert_eq!(preset[TRANSECT]["CameraCalc"]["DistanceToSurface"], 80);
    }

    #[test]
    fn applying_takes_the_pattern_settings_but_not_the_shape() {
        let item = json!({ "complexItemType": "CorridorScan", "polyline": [[47.0, 8.0], [47.001, 8.0]], "CorridorWidth": 50, "EntryPoint": 0, "TransectStyleComplexItem": { "CameraCalc": { "DistanceToSurface": 40, "AdjustedFootprintSide": 20, "AdjustedFootprintFrontal": 20 }, "TurnAroundDistance": 10 } });
        let preset = json!({ "complexItemType": "CorridorScan", "CorridorWidth": 80, "EntryPoint": 1, "TransectStyleComplexItem": { "CameraCalc": { "DistanceToSurface": 120, "AdjustedFootprintSide": 20, "AdjustedFootprintFrontal": 20 }, "TurnAroundDistance": 0 } });
        let changed = applied(&item, &preset, "CorridorScan");
        assert_eq!(changed["polyline"], item["polyline"], "QGC loads a preset without touching the corridor polyline");
        assert_eq!((changed["CorridorWidth"].as_f64(), changed["EntryPoint"].as_i64()), (Some(80.0), Some(1)));
        assert_eq!(changed[TRANSECT]["CameraCalc"]["DistanceToSurface"], 120);
        assert!(changed[TRANSECT]["Items"].as_array().is_some_and(|items| !items.is_empty()), "the transects are rebuilt for the new settings");
    }

    #[test]
    fn only_surveys_and_corridors_have_presets() {
        assert_eq!((settings_group("survey"), settings_group("CorridorScan"), settings_group("StructureScan")), (Some("Survey"), Some("CorridorScan"), None));
        assert!(!valid_name("a/b") && !valid_name(" ") && valid_name("Mapping 80m"));
    }
}
