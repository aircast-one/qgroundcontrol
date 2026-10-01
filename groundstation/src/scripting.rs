use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};

use crate::control::decode;
use crate::filejobs::Outcome;
use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.parameterManager.parametersReady"];
pub const REFRESH_SCRIPTS: &str = "scripting.refresh";
pub const UPLOAD_SCRIPT: &str = "scripting.upload";
pub const DOWNLOAD_SCRIPT: &str = "scripting.download";
pub const DELETE_SCRIPT: &str = "scripting.delete";
pub const CANCEL_TRANSFER: &str = "scripting.cancel";
pub const SCRIPT_ROOT: &str = "/APM/scripts/";
const SCRIPTING_ENABLE_PARAM: &str = "SCR_ENABLE";

#[derive(Debug, Default)]
struct Page {
    seen_generation: u64,
    scripts: Vec<String>,
    status: String,
}

static PAGE: LazyLock<Mutex<Page>> = LazyLock::new(|| Mutex::new(Page::default()));

fn page() -> MutexGuard<'static, Page> {
    PAGE.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn script_files(entries: &[String]) -> Vec<String> {
    entries.iter().filter(|entry| entry.len() >= 2 && entry.starts_with('F')).map(|entry| entry[1..].split('\t').next().unwrap_or_default().to_string()).collect()
}

pub fn remote_name(local_name: &str) -> String {
    match local_name.to_lowercase().ends_with(".lua") {
        true => local_name.to_string(),
        false => format!("{local_name}.lua"),
    }
}

fn status_for(path: &str, result: &Result<Outcome, String>, local: &str) -> (String, bool) {
    match result {
        Err(error) => (error.clone(), false),
        Ok(Outcome::Uploaded) => (format!("Upload succeeded: {path}"), true),
        Ok(Outcome::Deleted) => (format!("Delete succeeded: {path}"), true),
        Ok(Outcome::Downloaded(_)) => (format!("Download succeeded: {local}"), false),
        Ok(Outcome::Listed(_)) => (String::new(), false),
    }
}

fn ftp(backend: &dyn Backend, action: Value) -> Value {
    crate::guided::dispatch(backend, Some(action), crate::guided::active_id(backend), "", "[]")
}

fn args(text: &str) -> Vec<String> {
    serde_json::from_str::<Vec<Value>>(text).unwrap_or_default().iter().map(|v| v.as_str().unwrap_or_default().to_string()).collect()
}

static DOWNLOAD_TARGET: Mutex<String> = Mutex::new(String::new());

pub fn run(backend: &dyn Backend, path: &str, text: &str) -> Value {
    let given = args(text);
    let arg = |i: usize| given.get(i).cloned().unwrap_or_default();
    match path {
        REFRESH_SCRIPTS => ftp(backend, json!({ "action": "ftp", "op": "list", "path": SCRIPT_ROOT })),
        UPLOAD_SCRIPT => ftp(backend, json!({ "action": "ftp", "op": "upload", "path": format!("{SCRIPT_ROOT}{}", remote_name(&arg(1))), "file": arg(0) })),
        DOWNLOAD_SCRIPT => {
            *DOWNLOAD_TARGET.lock().unwrap_or_else(PoisonError::into_inner) = arg(1);
            ftp(backend, json!({ "action": "ftp", "op": "download", "path": format!("{SCRIPT_ROOT}{}", arg(0)), "file": arg(1) }))
        }
        DELETE_SCRIPT => ftp(backend, json!({ "action": "ftp", "op": "delete", "path": format!("{SCRIPT_ROOT}{}", arg(0)) })),
        CANCEL_TRANSFER => ftp(backend, json!({ "action": "ftp", "op": "cancel" })),
        _ => json!({ "ok": false, "reason": format!("{path} is not a scripting action") }),
    }
}

pub fn owns(path: &str) -> bool {
    [REFRESH_SCRIPTS, UPLOAD_SCRIPT, DOWNLOAD_SCRIPT, DELETE_SCRIPT, CANCEL_TRANSFER].contains(&path)
}

pub fn scripting_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let enable_path = format!("vehicle.parameterManager.getParameter(-1,{SCRIPTING_ENABLE_PARAM})");
    let fact = object(&backend.get(&enable_path));
    let supported = fact.get("kind").and_then(Value::as_str) == Some("fact") && fact.get("name").and_then(Value::as_str).is_some_and(|n| !n.is_empty());
    let enabled = fact.get("rawValue").or(fact.get("value")).and_then(Value::as_f64).is_some_and(|v| v != 0.0);
    let files = crate::hub::lock().active().map(|v| (v.files.busy(), v.files.progress, v.files.generation, v.files.last.clone()));
    let (busy, progress, generation, last) = files.unwrap_or((false, 0.0, 0, None));
    let refresh = {
        let mut page = page();
        let fresh = generation != page.seen_generation && !busy;
        if fresh {
            page.seen_generation = generation;
        }
        match last.as_ref().filter(|_| fresh) {
            None => false,
            Some((path, result)) => {
                if let Ok(Outcome::Listed(entries)) = result {
                    page.scripts = script_files(entries);
                }
                let local = DOWNLOAD_TARGET.lock().unwrap_or_else(PoisonError::into_inner).clone();
                let (status, refresh) = status_for(path, result, &local);
                if !status.is_empty() {
                    page.status = status;
                }
                refresh && enabled
            }
        }
    };
    if refresh {
        run(backend, REFRESH_SCRIPTS, "[]");
    }
    let page = page();
    json!({
        "kind": "object",
        "class": "Scripting",
        "available": supported,
        "enabled": enabled,
        "enable": if supported { decode(&fact, &enable_path) } else { Value::Null },
        "scripts": if enabled { page.scripts.clone() } else { Vec::new() },
        "busy": busy,
        "progress": progress,
        "status": if enabled { page.status.clone() } else { String::new() },
        "unsupportedText": "Scripting is not supported by this version of firmware.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_files_are_listed_and_uploads_get_the_lua_extension() {
        let entries = vec!["Fhello.lua\t120".to_string(), "Dmodules".to_string(), "F".to_string(), "Fnotes.txt\t3".to_string()];
        assert_eq!(script_files(&entries), ["hello.lua", "notes.txt"]);
        assert_eq!(remote_name("mission"), "mission.lua");
        assert_eq!(remote_name("Mission.LUA"), "Mission.LUA");
    }

    #[test]
    fn a_finished_transfer_reads_like_qgc() {
        assert_eq!(status_for("/APM/scripts/a.lua", &Ok(Outcome::Uploaded), ""), ("Upload succeeded: /APM/scripts/a.lua".to_string(), true));
        assert_eq!(status_for("/APM/scripts/a.lua", &Ok(Outcome::Deleted), ""), ("Delete succeeded: /APM/scripts/a.lua".to_string(), true));
        assert_eq!(status_for("/APM/scripts/a.lua", &Ok(Outcome::Downloaded(Vec::new())), "/cache/a.lua"), ("Download succeeded: /cache/a.lua".to_string(), false));
        assert_eq!(status_for("x", &Err("Delete failed".into()), ""), ("Delete failed".to_string(), false));
    }
}
