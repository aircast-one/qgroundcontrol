use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

const ACTIONS_FILE_TYPE: &str = "MavlinkActions";
const ACTIONS_FILE_VERSION: i64 = 1;
const ACTION_DEFAULT_COMPONENT: u8 = 1;
pub const JOYSTICK_FILE: &str = "settings.mavlinkActionsSettings.joystickActionsFile";
pub const FLY_VIEW_FILE: &str = "settings.mavlinkActionsSettings.flyViewActionsFile";
pub const DEPS: &[&str] = &[FLY_VIEW_FILE, JOYSTICK_FILE, "vehicles.activeVehicleAvailable"];
pub const SEND_MAVLINK_ACTION: &str = "mavlinkActions.send";
const ACTIONS_SAVE_PATH: &str = "settings.appSettings.mavlinkActionsSavePath";

#[derive(Debug, Clone, PartialEq)]
pub struct Action {
    pub label: String,
    pub description: String,
    pub command: u16,
    pub component: u8,
    pub params: [f64; 7],
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "NULL",
        Value::Bool(_) => "Bool",
        Value::Number(_) => "Double",
        Value::String(_) => "String",
        Value::Array(_) => "Array",
        Value::Object(_) => "Object",
    }
}

fn format_error(detail: String) -> String {
    format!("Custom actions file - incorrect format: {detail}")
}

fn number(entry: &Value, key: &str, default: f64) -> Result<f64, String> {
    match entry.get(key) {
        None => Ok(default),
        Some(value) => value.as_f64().ok_or_else(|| format_error(format!("Incorrect value type - key:type:expected {key}:{}:Double", type_name(value)))),
    }
}

pub fn parse(path: &str, text: &str) -> Result<Vec<Action>, String> {
    let failed = |detail: String| format!("Failed to load custom actions file: `{path}` error: `{detail}`");
    let file: Value = serde_json::from_str(text).map_err(|e| failed(format!("Unable to parse json file: {path} error: {e}")))?;
    if !file.is_object() {
        return Err(failed(format!("Root of json file is not object: {path}")));
    }
    let in_file = |detail: String| failed(format!("Json file: '{path}'. {detail}"));
    if file["fileType"].as_str() != Some(ACTIONS_FILE_TYPE) {
        return Err(in_file(format!("Incorrect file type key expected:{ACTIONS_FILE_TYPE} actual:{}", file["fileType"].as_str().unwrap_or(""))));
    }
    let version = file["version"].as_f64().map_or(0, |v| v as i64);
    if version < ACTIONS_FILE_VERSION {
        return Err(in_file(format!("File version {version} is no longer supported")));
    }
    if version > ACTIONS_FILE_VERSION {
        return Err(in_file(format!("File version {version} is newer than current supported version {ACTIONS_FILE_VERSION}")));
    }
    let actions = file["actions"].as_array().ok_or_else(|| format_error("actions is not an array".to_string()))?;
    actions
        .iter()
        .map(|entry| {
            if !entry.is_object() {
                return Err(format_error("JsonValue not an object".to_string()));
            }
            let label = entry["label"].as_str().ok_or_else(|| format_error("label is required".to_string()))?;
            let description = entry["description"].as_str().ok_or_else(|| format_error("description is required".to_string()))?;
            let command = entry["mavCmd"].as_f64().ok_or_else(|| format_error("mavCmd is required".to_string()))?;
            let params: Vec<f64> = (1..=7).map(|i| number(entry, &format!("param{i}"), 0.0)).collect::<Result<_, _>>()?;
            Ok(Action {
                label: label.to_string(),
                description: description.to_string(),
                command: command as u16,
                component: number(entry, "compId", f64::from(ACTION_DEFAULT_COMPONENT))? as u8,
                params: std::array::from_fn(|i| params[i]),
            })
        })
        .collect()
}

fn text(backend: &dyn Backend, path: &str) -> String {
    object(&backend.get(path)).get("value").and_then(Value::as_str).unwrap_or("").to_string()
}

pub fn joystick_actions(backend: &dyn Backend) -> Vec<Action> {
    actions_from(backend, JOYSTICK_FILE)
}

pub fn fly_view_actions(backend: &dyn Backend) -> Vec<Action> {
    actions_from(backend, FLY_VIEW_FILE)
}

pub fn action_files(backend: &dyn Backend) -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(text(backend, ACTIONS_SAVE_PATH))
        .map(|dir| dir.filter_map(Result::ok).filter_map(|entry| entry.file_name().into_string().ok()).filter(|name| name.to_lowercase().ends_with(".json")).collect())
        .unwrap_or_default();
    files.sort();
    files
}

fn actions_from(backend: &dyn Backend, setting: &str) -> Vec<Action> {
    let text = |path: &str| text(backend, path);
    let file = text(setting);
    if file.is_empty() {
        return Vec::new();
    }
    let path = std::path::Path::new(&text(ACTIONS_SAVE_PATH)).join(&file);
    let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    let mut cache = CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((cached_path, cached_at, actions)) = cache.as_ref()
        && *cached_path == path
        && *cached_at == modified
    {
        return actions.clone();
    }
    let shown = path.display().to_string();
    let actions = match std::fs::read_to_string(&path).map_err(|e| format!("Failed to load custom actions file: `{shown}` error: `{e}`")).and_then(|t| parse(&shown, &t)) {
        Ok(actions) => actions,
        Err(error) => {
            crate::noticeboard::post(crate::noticeboard::MESSAGE, "", &error);
            Vec::new()
        }
    };
    *cache = Some((path, modified, actions.clone()));
    actions
}

type Cached = Option<(std::path::PathBuf, Option<std::time::SystemTime>, Vec<Action>)>;

static CACHE: std::sync::Mutex<Cached> = std::sync::Mutex::new(None);

pub fn mavlink_actions_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = crate::read::flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
    json!({
        "kind": "object",
        "class": "MavlinkActions",
        "files": action_files(backend),
        "flyViewFile": text(backend, FLY_VIEW_FILE),
        "joystickFile": text(backend, JOYSTICK_FILE),
        "flyViewPath": FLY_VIEW_FILE,
        "joystickPath": JOYSTICK_FILE,
        "actions": if vehicle { fly_view_actions(backend).iter().map(|a| json!({ "label": a.label, "description": a.description })).collect::<Vec<_>>() } else { Vec::new() },
    })
}

pub fn send(backend: &dyn Backend, args: &str) -> Value {
    let index = serde_json::from_str::<Vec<Value>>(args).ok().and_then(|a| a.first().and_then(Value::as_u64)).map(|i| i as usize);
    match index.and_then(|i| fly_view_actions(backend).get(i).cloned()) {
        Some(action) => crate::guided::dispatch(backend, Some(command(&action)), crate::guided::active_id(backend), "", "[]"),
        None => json!({ "ok": false, "reason": "There is no such action in the Fly View actions file." }),
    }
}

pub fn command(action: &Action) -> Value {
    json!({ "action": "mavlinkCommand", "command": action.command, "component": action.component, "params": action.params })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_custom_actions_file_reads_like_mavlink_action_manager() {
        let actions = parse("a.json", r#"{"fileType":"MavlinkActions","version":1,"actions":[{"label":"Lights","description":"toggle","mavCmd":183,"param1":9,"param2":1900},{"label":"Payload","description":"drop","mavCmd":211,"compId":25}]}"#).unwrap();
        assert_eq!(actions[0], Action { label: "Lights".into(), description: "toggle".into(), command: 183, component: 1, params: [9.0, 1900.0, 0.0, 0.0, 0.0, 0.0, 0.0] });
        assert_eq!(actions[1].component, 25);
        assert!(parse("a.json", r#"{"fileType":"Plan","version":1,"actions":[]}"#).is_err());
        assert!(parse("a.json", r#"{"fileType":"MavlinkActions","version":1,"actions":[{"label":"x","mavCmd":1}]}"#).is_err(), "description is required");
    }

    #[test]
    fn a_bad_custom_actions_file_reports_qgc_errors() {
        let error = |text: &str| parse("a.json", text).unwrap_err();
        assert_eq!(error(r#"{"fileType":"MavlinkActions","version":2,"actions":[]}"#), "Failed to load custom actions file: `a.json` error: `Json file: 'a.json'. File version 2 is newer than current supported version 1`");
        assert!(error(r#"{"fileType":"MavlinkActions","version":0.5,"actions":[]}"#).ends_with("File version 0 is no longer supported`"));
        assert!(parse("a.json", r#"{"fileType":"MavlinkActions","version":1.9,"actions":[]}"#).is_ok());
        assert_eq!(error(r#"{"fileType":"MavlinkActions","version":1,"actions":[{"label":"x","description":"y","mavCmd":1,"compId":"25"}]}"#), "Custom actions file - incorrect format: Incorrect value type - key:type:expected compId:String:Double");
        assert!(error(r#"{"fileType":"MavlinkActions","version":1,"actions":[{"label":"x","description":"y","mavCmd":1,"param3":true}]}"#).contains("param3:Bool:Double"));
        assert!(error("[]").contains("Root of json file is not object: a.json"));
    }

    struct Saved(String);
    impl Backend for Saved {
        fn get(&self, path: &str) -> String {
            match path {
                ACTIONS_SAVE_PATH => json!({ "kind": "value", "value": self.0 }),
                FLY_VIEW_FILE => json!({ "kind": "fact", "value": "fly.json" }),
                _ => json!({ "kind": "fact", "value": "" }),
            }
            .to_string()
        }
        fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "activeVehicleAvailable": true }).to_string() }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_fly_view_lists_its_file_actions_and_the_settings_offer_the_json_files() {
        let dir = std::env::temp_dir().join(format!("mavlinkactions-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("fly.json"), r#"{"fileType":"MavlinkActions","version":1,"actions":[{"label":"Lights","description":"toggle","mavCmd":183}]}"#).unwrap();
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        let backend = Saved(dir.to_string_lossy().into_owned());
        let view = mavlink_actions_view(&backend, &[]);
        assert_eq!(view["files"], json!(["fly.json"]));
        assert_eq!(view["actions"], json!([{ "label": "Lights", "description": "toggle" }]));
        assert_eq!(send(&backend, "[5]")["ok"], false);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
