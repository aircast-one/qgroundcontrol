use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};

use crate::control::decode;
use crate::filejobs::Outcome;
use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.parameterManager.parametersReady"];
pub const OPEN_SCRIPTS: &str = "scripting.open";
pub const REFRESH_SCRIPTS: &str = "scripting.refresh";
pub const UPLOAD_SCRIPT: &str = "scripting.upload";
pub const DOWNLOAD_SCRIPT: &str = "scripting.download";
pub const DELETE_SCRIPT: &str = "scripting.delete";
pub const CANCEL_TRANSFER: &str = "scripting.cancel";
pub const SCRIPT_ROOT: &str = "/APM/scripts/";
const SCRIPTING_ENABLE_PARAM: &str = "SCR_ENABLE";
const OPERATION_BUSY: &str = "Another FTP operation is in progress";

#[derive(Debug, Default)]
struct Page {
    seen_generation: u64,
    scripts: Vec<String>,
    status: String,
    enabled_seen: Option<bool>,
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

pub fn refusal_text(reason: &str, own_transfer_running: bool, failure: String) -> String {
    match reason == crate::hub::FILES_BUSY && own_transfer_running {
        true => OPERATION_BUSY.to_string(),
        false => failure,
    }
}

fn own_transfer_running() -> bool {
    crate::hub::lock().active().and_then(|v| v.files.job.as_ref().map(|job| job.path.starts_with(SCRIPT_ROOT))).unwrap_or(false)
}

fn finished_generation() -> u64 {
    crate::hub::lock().active().map(|v| v.files.generation).unwrap_or(0)
}

fn ftp(backend: &dyn Backend, action: Value) -> Value {
    crate::guided::dispatch(backend, Some(action), crate::guided::active_id(backend), "", "[]")
}

fn refused(answer: &Value, failure: String) -> Option<String> {
    match answer.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        true => None,
        false => Some(refusal_text(answer.get("reason").and_then(Value::as_str).unwrap_or_default(), own_transfer_running(), failure)),
    }
}

fn refusal(text: String) -> Value {
    json!({ "ok": false, "refusal": Value::Null, "reason": text })
}

fn list(backend: &dyn Backend) -> Value {
    let answer = ftp(backend, json!({ "action": "ftp", "op": "list", "path": SCRIPT_ROOT }));
    match refused(&answer, format!("Failed to list {SCRIPT_ROOT}")) {
        Some(text) if text == OPERATION_BUSY => refusal(text),
        Some(text) => {
            page().scripts.clear();
            refusal(text)
        }
        None => {
            page().scripts.clear();
            answer
        }
    }
}

fn transfer(backend: &dyn Backend, action: Value, failure: String) -> Value {
    let answer = ftp(backend, action);
    match refused(&answer, failure) {
        Some(text) => {
            page().status = text.clone();
            refusal(text)
        }
        None => answer,
    }
}

fn args(text: &str) -> Vec<String> {
    serde_json::from_str::<Vec<Value>>(text).unwrap_or_default().iter().map(|v| v.as_str().unwrap_or_default().to_string()).collect()
}

static DOWNLOAD_TARGET: Mutex<String> = Mutex::new(String::new());

struct Enable {
    path: String,
    fact: Value,
    supported: bool,
    enabled: bool,
}

fn enable(backend: &dyn Backend) -> Enable {
    let path = format!("vehicle.parameterManager.getParameter(-1,{SCRIPTING_ENABLE_PARAM})");
    let fact = object(&backend.get(&path));
    let supported = fact.get("kind").and_then(Value::as_str) == Some("fact") && fact.get("name").and_then(Value::as_str).is_some_and(|n| !n.is_empty());
    let enabled = fact.get("rawValue").or(fact.get("value")).and_then(Value::as_f64).is_some_and(|v| v != 0.0);
    Enable { path, fact, supported, enabled }
}

pub fn run(backend: &dyn Backend, path: &str, text: &str) -> Value {
    let given = args(text);
    let arg = |i: usize| given.get(i).cloned().unwrap_or_default();
    match path {
        OPEN_SCRIPTS => {
            let enabled = enable(backend).enabled;
            *page() = Page { seen_generation: finished_generation(), enabled_seen: Some(enabled), ..Page::default() };
            match enabled {
                true => list(backend),
                false => json!({ "ok": true, "refusal": Value::Null, "reason": Value::Null }),
            }
        }
        REFRESH_SCRIPTS => list(backend),
        UPLOAD_SCRIPT => match std::path::Path::new(&arg(0)).is_file() {
            false => {
                let text = format!("File {} does not exist", arg(0));
                page().status = text.clone();
                refusal(text)
            }
            true => transfer(backend, json!({ "action": "ftp", "op": "upload", "path": format!("{SCRIPT_ROOT}{}", remote_name(&arg(1))), "file": arg(0) }), format!("Failed to upload {}", arg(1))),
        },
        DOWNLOAD_SCRIPT => {
            *DOWNLOAD_TARGET.lock().unwrap_or_else(PoisonError::into_inner) = Some(arg(2)).filter(|shown| !shown.is_empty()).unwrap_or_else(|| arg(1));
            let remote = format!("{SCRIPT_ROOT}{}", arg(0));
            transfer(backend, json!({ "action": "ftp", "op": "download", "path": remote, "file": arg(1) }), format!("Failed to download {remote}"))
        }
        DELETE_SCRIPT => {
            let remote = format!("{SCRIPT_ROOT}{}", arg(0));
            transfer(backend, json!({ "action": "ftp", "op": "delete", "path": remote }), format!("Failed to delete {remote}"))
        }
        CANCEL_TRANSFER => match own_transfer_running() {
            true => ftp(backend, json!({ "action": "ftp", "op": "cancel" })),
            false => json!({ "ok": true, "refusal": Value::Null, "reason": Value::Null }),
        },
        _ => json!({ "ok": false, "reason": format!("{path} is not a scripting action") }),
    }
}

pub fn owns(path: &str) -> bool {
    [OPEN_SCRIPTS, REFRESH_SCRIPTS, UPLOAD_SCRIPT, DOWNLOAD_SCRIPT, DELETE_SCRIPT, CANCEL_TRANSFER].contains(&path)
}

fn noticed(page: &mut Page, enabled: bool, generation: u64, last: Option<(String, Result<Outcome, String>)>, local: &str) -> bool {
    let switched_on = page.enabled_seen == Some(false) && enabled;
    if page.enabled_seen == Some(true) && !enabled {
        page.status.clear();
    }
    page.enabled_seen = Some(enabled);
    let fresh = generation != page.seen_generation;
    page.seen_generation = generation;
    let finished = last.filter(|_| fresh).is_some_and(|(path, result)| {
        if let Ok(Outcome::Listed(entries)) = &result {
            page.scripts = script_files(entries);
        }
        let (status, refresh) = status_for(&path, &result, local);
        if !status.is_empty() {
            page.status = status;
        }
        refresh
    });
    switched_on || finished
}

pub fn scripting_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let Enable { path: enable_path, fact, supported, enabled } = enable(backend);
    let files = crate::hub::lock().active().map(|v| (v.files.job.as_ref().is_some_and(|job| job.path.starts_with(SCRIPT_ROOT)), v.files.progress, v.files.generation, v.files.last.clone()));
    let (busy, progress, generation, last) = files.unwrap_or((false, 0.0, 0, None));
    let local = DOWNLOAD_TARGET.lock().unwrap_or_else(PoisonError::into_inner).clone();
    if noticed(&mut page(), enabled, generation, last, &local) {
        run(backend, REFRESH_SCRIPTS, "[]");
    }
    let page = page();
    json!({
        "kind": "object",
        "class": "Scripting",
        "available": supported,
        "enabled": enabled,
        "enable": if supported { decode(&fact, &enable_path) } else { Value::Null },
        "scripts": page.scripts.clone(),
        "busy": busy,
        "progress": if busy { progress } else { 0.0 },
        "status": page.status.clone(),
        "unsupportedText": "Scripting is not supported by this version of firmware.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Nothing;
    impl Backend for Nothing {
        fn get(&self, _: &str) -> String { json!({ "kind": "null" }).to_string() }
        fn get_fields(&self, _: &str, _: &str) -> String { json!({ "kind": "null" }).to_string() }
        fn set(&self, _: &str, _: &str) -> String { String::new() }
        fn invoke(&self, _: &str, _: &str) -> String { String::new() }
        fn watch(&self, _: &[String]) {}
    }

    #[test]
    fn a_download_reports_the_file_the_operator_saved_not_the_staging_copy() {
        run(&Nothing, DOWNLOAD_SCRIPT, r#"["hello.lua", "/cache/download-hello.lua", "hello.lua"]"#);
        let local = DOWNLOAD_TARGET.lock().unwrap().clone();
        assert_eq!(status_for("/APM/scripts/hello.lua", &Ok(Outcome::Downloaded(Vec::new())), &local).0, "Download succeeded: hello.lua", "ScriptingComponent shows the path the user picked");
    }

    #[test]
    fn refusals_read_like_ftp_controller() {
        assert_eq!(refusal_text(crate::hub::FILES_BUSY, true, "Failed to list /APM/scripts/".into()), "Another FTP operation is in progress", "the page's own transfer is FTPController's _operation check");
        assert_eq!(refusal_text(crate::hub::FILES_BUSY, false, "Failed to list /APM/scripts/".into()), "Failed to list /APM/scripts/", "another user of the FTP manager makes FTPManager refuse");
        assert_eq!(refusal_text("The vehicle was not sent the command.", false, "Failed to delete /APM/scripts/a.lua".into()), "Failed to delete /APM/scripts/a.lua");
        let missing = run(&Nothing, UPLOAD_SCRIPT, r#"["/no/such/upload-a.lua", "a.lua"]"#);
        assert_eq!(missing["reason"], "File /no/such/upload-a.lua does not exist");
    }

    #[test]
    fn switching_scripting_on_lists_and_off_clears_the_status() {
        let mut page = Page { enabled_seen: Some(false), status: "Upload succeeded: /APM/scripts/a.lua".into(), scripts: vec!["a.lua".into()], ..Page::default() };
        assert!(noticed(&mut page, true, 0, None, ""), "SCR_ENABLE going on lists the scripts");
        assert!(!noticed(&mut page, true, 0, None, ""));
        assert!(!noticed(&mut page, false, 0, None, ""));
        assert_eq!(page.status, "", "SCR_ENABLE going off clears the status line");
        assert_eq!(page.scripts, ["a.lua"], "the list stays, greyed out, like the disabled Flow in ScriptingComponent");
    }

    #[test]
    fn a_finished_transfer_is_read_once() {
        let mut page = Page { enabled_seen: Some(true), ..Page::default() };
        let uploaded = Some(("/APM/scripts/a.lua".to_string(), Ok(Outcome::Uploaded)));
        assert!(noticed(&mut page, true, 1, uploaded.clone(), ""), "an upload refreshes the directory");
        assert!(!noticed(&mut page, true, 1, uploaded, ""), "the same result is not acted on twice");
        let listed = Some(("/APM/scripts/".to_string(), Ok(Outcome::Listed(vec!["Fb.lua\t1".into()]))));
        assert!(!noticed(&mut page, true, 2, listed, ""));
        assert_eq!((page.scripts.as_slice(), page.status.as_str()), (["b.lua".to_string()].as_slice(), "Upload succeeded: /APM/scripts/a.lua"));
    }

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
