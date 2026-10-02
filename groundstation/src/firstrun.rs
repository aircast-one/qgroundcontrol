use serde_json::{Value, json};

use crate::control::decode;
use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "settings.appSettings.firstRunPromptIdsShown",
    "settings.appSettings.preferredFirmwareClass",
    "settings.appSettings.preferredVehicleClass",
];

pub const MARK_SHOWN: &str = "firstRun.markShown";
const INITIAL_SETUP_PROMPT_ID: i64 = 3;
const SHOWN_PATH: &str = "settings.appSettings.firstRunPromptIdsShown";
const PREFERENCES: [(&str, &str); 2] = [("preferredFirmwareClass", "Preferred Firmware"), ("preferredVehicleClass", "Preferred Vehicle")];

pub fn shown_ids(text: &str) -> Vec<i64> {
    text.split(',').filter_map(|id| id.trim().parse().ok()).collect()
}

fn shown_text(backend: &dyn Backend) -> String {
    object(&backend.get(SHOWN_PATH)).get("value").and_then(Value::as_str).unwrap_or_default().to_string()
}

pub fn first_run_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let show = !shown_ids(&shown_text(backend)).contains(&INITIAL_SETUP_PROMPT_ID);
    let preferences: Vec<Value> = PREFERENCES
        .iter()
        .filter_map(|(name, label)| {
            let path = format!("settings.appSettings.{name}");
            let fact = object(&backend.get(&path));
            (fact.get("kind").and_then(Value::as_str) == Some("fact")).then(|| {
                let mut control = decode(&fact, &path);
                control["label"] = json!(label);
                control
            })
        })
        .collect();
    json!({
        "kind": "object",
        "class": "FirstRun",
        "show": show,
        "promptId": INITIAL_SETUP_PROMPT_ID,
        "title": "Preferences",
        "vehicleHeading": "Vehicle Preferences",
        "vehicleDescription": "Select the firmware and vehicle type you typically use.",
        "vehiclePreferences": preferences,
        "unitsHeading": "Measurement Units",
        "unitsDescription": "Choose the measurement units you want to use. You can also change it later in General Settings.",
    })
}

pub fn mark_shown(backend: &dyn Backend) -> Value {
    let ids = shown_ids(&shown_text(backend));
    if ids.contains(&INITIAL_SETUP_PROMPT_ID) {
        return json!({ "ok": true });
    }
    [("preferredFirmwareClass", "offlineEditingFirmwareClass"), ("preferredVehicleClass", "offlineEditingVehicleClass")].iter().for_each(|(preferred, offline)| {
        let chosen = object(&backend.get(&format!("settings.appSettings.{preferred}")));
        if let Some(class) = chosen.get("rawValue").or(chosen.get("value")).and_then(Value::as_i64).filter(|class| *class != 0) {
            crate::factwrite::write(backend, &format!("settings.appSettings.{offline}"), &json!({ "value": class }).to_string());
        }
    });
    let updated = ids.iter().chain([&INITIAL_SETUP_PROMPT_ID]).map(i64::to_string).collect::<Vec<_>>().join(",");
    crate::factwrite::write(backend, SHOWN_PATH, &json!({ "value": updated }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Settings(RefCell<String>);

    impl Backend for Settings {
        fn get(&self, path: &str) -> String {
            match path {
                SHOWN_PATH => json!({ "kind": "fact", "name": "firstRunPromptIdsShown", "value": *self.0.borrow(), "typeIsString": true }),
                p if p.ends_with("Class") => json!({ "kind": "fact", "name": p.rsplit('.').next().unwrap(), "value": 0, "enumStrings": ["ArduPilot", "PX4"], "enumValues": [3, 12], "enumIndex": 0 }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, value: &str) -> String {
            *self.0.borrow_mut() = serde_json::from_str::<Value>(value).unwrap()["value"].as_str().unwrap().to_string();
            json!({ "ok": true }).to_string()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_prompt_shows_until_its_id_is_marked() {
        let settings = Settings(RefCell::new("1".to_string()));
        let view = first_run_view(&settings, &[]);
        assert_eq!(view["show"], true);
        assert_eq!(view["vehiclePreferences"][0]["label"], "Preferred Firmware");
        assert_eq!(mark_shown(&settings)["ok"], true);
        assert_eq!(*settings.0.borrow(), "1,3", "the id joins the list AppSettings keeps");
        assert_eq!(first_run_view(&settings, &[])["show"], false);
        assert_eq!(shown_ids(" 3 ,x,4"), vec![3, 4]);
    }
}
