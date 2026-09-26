use std::path::Path;

use serde_json::{Value, json};

use crate::read::{flag, object, text};
use crate::router::Backend;

const CONTROLLER: &str = "geoTag";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Start,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    LogFile,
    ImageDirectory,
    SaveDirectory,
}

impl Field {
    fn property(self) -> &'static str {
        match self {
            Field::LogFile => "logFile",
            Field::ImageDirectory => "imageDirectory",
            Field::SaveDirectory => "saveDirectory",
        }
    }
}

pub fn local_path(raw: &str) -> String {
    match url::Url::parse(raw).ok().filter(|u| u.scheme() == "file").and_then(|u| u.to_file_path().ok()) {
        Some(path) => path.to_string_lossy().into_owned(),
        None => raw.to_string(),
    }
}

fn path_refusal(field: Field, path: &str) -> Option<(&'static str, &'static str)> {
    let place = Path::new(path);
    match field {
        _ if path.trim().is_empty() => Some(("emptyPath", "Choose a path first.")),
        _ if !place.exists() => Some(("notFound", "Nothing exists at that path.")),
        Field::LogFile if !place.is_file() => Some(("notAFile", "The flight log has to be a file.")),
        Field::ImageDirectory | Field::SaveDirectory if !place.is_dir() => Some(("notADirectory", "That path is not a folder.")),
        _ => None,
    }
}

#[derive(Debug, Default)]
struct Job {
    running: bool,
    log_file: String,
    image_directory: String,
    save_directory: String,
}

fn job(controller: &Value) -> Job {
    Job {
        running: flag(controller, "inProgress"),
        log_file: text(controller, "logFile"),
        image_directory: text(controller, "imageDirectory"),
        save_directory: text(controller, "saveDirectory"),
    }
}

fn refusal(action: Action, state: &Job) -> Option<(&'static str, &'static str)> {
    match action {
        Action::Cancel if !state.running => Some(("idle", "No tagging is running.")),
        Action::Cancel => None,
        Action::Start if state.running => Some(("busy", "Tagging is already running.")),
        Action::Start if state.image_directory.is_empty() => Some(("noImageDirectory", "Choose the folder holding the images.")),
        Action::Start if state.log_file.is_empty() => Some(("noLogFile", "Choose the flight log.")),
        Action::Start if !Path::new(&state.image_directory).is_dir() => Some(("imageDirectoryMissing", "The image folder is no longer there.")),
        Action::Start if !state.save_directory.is_empty() && !Path::new(&state.save_directory).is_dir() => Some(("saveDirectoryMissing", "The destination folder is no longer there.")),
        Action::Start => None,
    }
}

pub fn act(backend: &dyn Backend, action: Action, path: &str) -> Value {
    let controller = object(&backend.get_fields(CONTROLLER, "inProgress,logFile,imageDirectory,saveDirectory"));
    if controller.get("kind").and_then(Value::as_str) != Some("object") {
        return json!({ "ok": false, "refusal": "unavailable", "reason": "The geotagging controller is not available." });
    }
    if let Some((token, reason)) = refusal(action, &job(&controller)) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let dispatched = flag(&object(&backend.invoke(path, "[]")), "ok");
    json!({
        "ok": dispatched,
        "refusal": Value::Null,
        "reason": match dispatched { true => Value::Null, false => json!("The geotagging controller did not take the request.") },
    })
}

pub fn write(backend: &dyn Backend, field: Field, path: &str, value: &str) -> Value {
    let Some(asked) = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value")?.as_str().map(local_path)) else {
        return json!({ "ok": false, "result": false, "refusal": "notText", "reason": "A path has to be text." });
    };
    let property = field.property();
    let controller = object(&backend.get_fields(CONTROLLER, &format!("inProgress,{property}")));
    if flag(&controller, "inProgress") {
        return json!({ "ok": false, "result": false, "refusal": "busy", "reason": "Wait for tagging to finish, or cancel it, before changing paths." });
    }
    if let Some((token, reason)) = path_refusal(field, &asked) {
        return json!({ "ok": false, "result": false, "refusal": token, "reason": reason, "path": asked });
    }
    let dispatched = flag(&object(&backend.set(path, &json!({ "value": asked }).to_string())), "ok");
    let held = text(&object(&backend.get_fields(CONTROLLER, property)), property);
    let took = dispatched && held == asked;
    json!({
        "ok": took,
        "result": took,
        "refusal": Value::Null,
        "path": asked,
        "reason": match took { true => Value::Null, false => json!("The geotagging controller did not keep that path.") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Controller {
        state: RefCell<Value>,
        keeps: bool,
        invoked: RefCell<Vec<String>>,
    }

    impl Controller {
        fn new(state: Value, keeps: bool) -> Self {
            Self { state: RefCell::new(state), keeps, invoked: RefCell::new(Vec::new()) }
        }
    }

    impl Backend for Controller {
        fn get(&self, p: &str) -> String { self.get_fields(p, "") }
        fn get_fields(&self, _p: &str, _f: &str) -> String { self.state.borrow().to_string() }
        fn set(&self, p: &str, v: &str) -> String {
            if self.keeps {
                let value = object(v)["value"].clone();
                self.state.borrow_mut()[p.trim_start_matches("geoTag.")] = value;
            }
            json!({ "ok": true }).to_string()
        }
        fn invoke(&self, p: &str, _a: &str) -> String {
            self.invoked.borrow_mut().push(p.to_string());
            json!({ "ok": true }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    fn scratch() -> (String, String) {
        let dir = std::env::temp_dir().join(format!("qgc-geotagjob-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("flight.tlog");
        std::fs::write(&log, b"").unwrap();
        (dir.to_string_lossy().into_owned(), log.to_string_lossy().into_owned())
    }

    #[test]
    fn a_path_the_controller_would_drop_is_refused_and_one_it_dropped_anyway_is_reported() {
        let (dir, log) = scratch();
        let fresh = || Controller::new(json!({ "kind": "object", "inProgress": false, "logFile": "", "imageDirectory": "" }), true);

        let backend = fresh();
        assert_eq!(write(&backend, Field::LogFile, "geoTag.logFile", &json!({ "value": dir }).to_string())["refusal"], "notAFile", "setLogFile sets errorMessage and keeps the old file, while the bridge answers the property write ok");
        assert_eq!(write(&backend, Field::ImageDirectory, "geoTag.imageDirectory", &json!({ "value": log }).to_string())["refusal"], "notADirectory");
        assert_eq!(write(&backend, Field::SaveDirectory, "geoTag.saveDirectory", r#"{"value":"/no/such/folder"}"#)["refusal"], "notFound");
        assert_eq!(write(&backend, Field::LogFile, "geoTag.logFile", r#"{"value":""}"#)["refusal"], "emptyPath");

        let url = url::Url::from_file_path(&log).unwrap().to_string();
        let taken = write(&backend, Field::LogFile, "geoTag.logFile", &json!({ "value": url }).to_string());
        assert_eq!((&taken["ok"], &taken["result"], &taken["path"]), (&json!(true), &json!(true), &json!(log)), "a file URL is written as the local path toLocalPath would have made of it, so the read-back compares like with like");

        let deaf = Controller::new(json!({ "kind": "object", "inProgress": false }), false);
        let dropped = write(&deaf, Field::ImageDirectory, "geoTag.imageDirectory", &json!({ "value": dir }).to_string());
        assert_eq!((&dropped["ok"], &dropped["result"]), (&json!(false), &json!(false)), "the value is read back, because a dispatched write is not a kept one");

        let busy = Controller::new(json!({ "kind": "object", "inProgress": true }), true);
        assert_eq!(write(&busy, Field::ImageDirectory, "geoTag.imageDirectory", &json!({ "value": dir }).to_string())["refusal"], "busy");
    }

    #[test]
    fn tagging_starts_only_when_it_has_what_it_needs_and_cancels_only_what_runs() {
        let (dir, log) = scratch();
        let ready = Job { running: false, log_file: log.clone(), image_directory: dir.clone(), save_directory: String::new() };
        let token = |action, state: &Job| refusal(action, state).map(|(t, _)| t);
        assert_eq!(token(Action::Start, &ready), None);
        assert_eq!(token(Action::Start, &Job { running: true, ..Job { log_file: log.clone(), image_directory: dir.clone(), ..Default::default() } }), Some("busy"), "startTagging answers a second start with a log warning and nothing else");
        assert_eq!(token(Action::Start, &Job { image_directory: String::new(), log_file: log.clone(), ..Default::default() }), Some("noImageDirectory"));
        assert_eq!(token(Action::Start, &Job { image_directory: dir.clone(), ..Default::default() }), Some("noLogFile"));
        assert_eq!(token(Action::Start, &Job { image_directory: "/no/such/folder".into(), log_file: log.clone(), ..Default::default() }), Some("imageDirectoryMissing"));
        assert_eq!(token(Action::Start, &Job { save_directory: "/no/such/folder".into(), log_file: log.clone(), image_directory: dir.clone(), ..Default::default() }), Some("saveDirectoryMissing"));
        assert_eq!(token(Action::Cancel, &ready), Some("idle"));
        assert_eq!(token(Action::Cancel, &Job { running: true, ..Default::default() }), None);

        let backend = Controller::new(json!({ "kind": "object", "inProgress": false, "logFile": log, "imageDirectory": dir }), true);
        assert_eq!(act(&backend, Action::Cancel, "geoTag.cancelTagging")["refusal"], "idle");
        assert_eq!(act(&backend, Action::Start, "geoTag.startTagging")["ok"], true);
        assert_eq!(backend.invoked.borrow().as_slice(), &["geoTag.startTagging".to_string()]);
        assert_eq!(act(&Controller::new(json!({ "kind": "null" }), true), Action::Start, "geoTag.startTagging")["refusal"], "unavailable");
    }
}
