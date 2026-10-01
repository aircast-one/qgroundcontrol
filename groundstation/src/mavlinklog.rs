use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Value, json};

use crate::router::Backend;

pub const MAVLINK_LOG_START: &str = "mavlinkLog.start";
pub const MAVLINK_LOG_STOP: &str = "mavlinkLog.stop";
pub const MAVLINK_LOG_SET: &str = "mavlinkLog.set";
pub const MAVLINK_LOG_UPLOAD: &str = "mavlinkLog.upload";
pub const MAVLINK_LOG_DELETE: &str = "mavlinkLog.delete";
pub const MAVLINK_LOG_CANCEL: &str = "mavlinkLog.cancelUpload";

const MAVLINK_LOG_GROUP: &str = "MAVLinkLogGroup";
const MAVLINK_LOG_EXTENSION: &str = ".ulg";
const SIDECAR: &str = ".uploaded";
const DEFAULT_URL: &str = "https://logs.px4.io/upload";
const DEFAULT_DESCRIPTION: &str = "QGroundControl Session";
const BOUNDARY: &str = "----QGroundControlLogBoundary7f3a";

const TEXT_KEYS: [(&str, &str, &str); 6] = [
    ("emailAddress", "Email", ""),
    ("description", "Description", DEFAULT_DESCRIPTION),
    ("uploadURL", "LogURL", DEFAULT_URL),
    ("videoURL", "VideoURL", ""),
    ("rating", "RateKey", "notset"),
    ("windSpeed", "WindSpeed", "-1"),
];
const FLAG_KEYS: [(&str, &str, bool); 4] = [
    ("enableAutoUpload", "EnableAutoUpload", true),
    ("enableAutoStart", "EnableAutoStart", false),
    ("deleteAfterUpload", "EnableDelete", false),
    ("publicLog", "PublicLog", true),
];

#[derive(Default)]
struct Uploads {
    running: bool,
    current: Option<String>,
    progress: f64,
    message: Option<String>,
    cancel: Option<Arc<AtomicBool>>,
    feedback: String,
    was_running: Option<String>,
}

static UPLOADS: Mutex<Uploads> = Mutex::new(Uploads { running: false, current: None, progress: 0.0, message: None, cancel: None, feedback: String::new(), was_running: None });

fn key(name: &str) -> String {
    format!("{MAVLINK_LOG_GROUP}/{name}")
}

fn text(field: &str) -> String {
    let (_, stored, default) = TEXT_KEYS.iter().find(|(f, _, _)| *f == field).copied().unwrap_or(("", "", ""));
    crate::settingsstore::stored_text(&key(stored)).unwrap_or_else(|| default.to_string())
}

fn flag(field: &str) -> bool {
    let (_, stored, default) = FLAG_KEYS.iter().find(|(f, _, _)| *f == field).copied().unwrap_or(("", "", false));
    crate::settingsstore::stored_text(&key(stored)).map_or(default, |t| t == "true")
}

fn folder() -> Option<PathBuf> {
    crate::settingsstore::log_save_path().map(PathBuf::from)
}

fn uploaded_marker(log: &Path) -> PathBuf {
    log.with_extension(SIDECAR.trim_start_matches('.'))
}

pub fn configure_hub() {
    let request = json!({ "path": folder().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(), "extension": MAVLINK_LOG_EXTENSION, "autoStart": flag("enableAutoStart") });
    let _ = crate::hub::lock().log_request(None, &request, crate::hub::now_ms());
}

fn files() -> Vec<Value> {
    let Some(folder) = folder() else { return Vec::new() };
    let listed: std::collections::BTreeMap<std::cmp::Reverse<String>, (u64, bool)> = std::fs::read_dir(&folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.to_string_lossy().ends_with(MAVLINK_LOG_EXTENSION))
                .map(|path| {
                    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    (std::cmp::Reverse(path.file_name().unwrap_or_default().to_string_lossy().into_owned()), (size, uploaded_marker(&path).exists()))
                })
                .collect()
        })
        .unwrap_or_default();
    listed.into_iter().map(|(std::cmp::Reverse(name), (size, uploaded))| json!({ "name": name, "size": size, "uploaded": uploaded })).collect()
}

pub fn multipart(fields: &[(&str, String)], file_name: &str, file: &[u8]) -> Vec<u8> {
    let parts: Vec<u8> = fields
        .iter()
        .flat_map(|(name, value)| format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").into_bytes())
        .collect();
    let header = format!("--{BOUNDARY}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; name=\"filearg\"; filename=\"{file_name}\"\r\n\r\n");
    [parts, header.into_bytes(), file.to_vec(), format!("\r\n--{BOUNDARY}--\r\n").into_bytes()].concat()
}

fn form_fields(feedback: &str) -> Vec<(&'static str, String)> {
    vec![
        ("email", text("emailAddress")),
        ("description", text("description")),
        ("source", "QGroundControl".to_string()),
        ("version", env!("CARGO_PKG_VERSION").to_string()),
        ("type", "flightreport".to_string()),
        ("windSpeed", text("windSpeed")),
        ("rating", text("rating")),
        ("public", flag("publicLog").to_string()),
        ("feedback", if feedback.is_empty() { "None Given".to_string() } else { feedback.to_string() }),
        ("videoUrl", Some(text("videoURL")).filter(|v| !v.is_empty()).unwrap_or_else(|| "None".to_string())),
    ]
}

fn send(path: &Path, feedback: &str) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let body = multipart(&form_fields(feedback), &name, &bytes);
    let response = ureq::post(&text("uploadURL"))
        .header("Content-Type", &format!("multipart/form-data; boundary={BOUNDARY}"))
        .send(&body[..])
        .map_err(|e| format!("Log Upload Error: {e}"))?;
    match response.status().as_u16() {
        200 => Ok(()),
        code => Err(format!("Log Upload Error: status {code}")),
    }
}

fn upload(names: Vec<String>) -> Result<(), String> {
    if text("emailAddress").is_empty() {
        return Err("Please enter an email address before uploading MAVLink log files.".to_string());
    }
    let folder = folder().ok_or("There is no log folder.")?;
    let cancel = Arc::new(AtomicBool::new(false));
    let feedback = {
        let mut uploads = UPLOADS.lock().unwrap_or_else(PoisonError::into_inner);
        if uploads.running {
            return Err("An upload is already running.".to_string());
        }
        uploads.running = true;
        uploads.message = None;
        uploads.cancel = Some(cancel.clone());
        uploads.feedback.clone()
    };
    std::thread::spawn(move || {
        let outcome = names.iter().take_while(|_| !cancel.load(Ordering::Relaxed)).try_for_each(|name| {
            {
                let mut uploads = UPLOADS.lock().unwrap_or_else(PoisonError::into_inner);
                uploads.current = Some(name.clone());
                uploads.progress = 0.0;
            }
            let path = folder.join(name);
            send(&path, &feedback)?;
            match flag("deleteAfterUpload") {
                true => {
                    let _ = std::fs::remove_file(&path);
                }
                false => {
                    let _ = std::fs::write(uploaded_marker(&path), b"");
                }
            }
            Ok::<(), String>(())
        });
        let mut uploads = UPLOADS.lock().unwrap_or_else(PoisonError::into_inner);
        uploads.running = false;
        uploads.current = None;
        uploads.cancel = None;
        uploads.message = outcome.err();
    });
    Ok(())
}

pub fn tick() {
    let state = crate::hub::lock().active().map(|vehicle| vehicle.log_snapshot());
    let running = state.as_ref().and_then(|s| s["running"].as_bool()).unwrap_or(false);
    let file = state.as_ref().and_then(|s| s["file"].as_str().map(str::to_string));
    let finished = {
        let mut uploads = UPLOADS.lock().unwrap_or_else(PoisonError::into_inner);
        let was = uploads.was_running.take();
        uploads.was_running = running.then(|| file.clone()).flatten();
        was.filter(|_| !running)
    };
    if let Some(done) = finished.filter(|_| flag("enableAutoUpload")) {
        let name = Path::new(&done).file_name().unwrap_or_default().to_string_lossy().into_owned();
        let _ = upload(vec![name]);
    }
}

pub fn mavlink_log_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    let (px4, state) = {
        let hub = crate::hub::lock();
        let active = hub.active();
        (active.is_some_and(|v| v.autopilot == crate::modes::AUTOPILOT_PX4), active.map(|v| v.log_snapshot()))
    };
    let running = state.as_ref().and_then(|s| s["running"].as_bool()).unwrap_or(false);
    let denied = state.as_ref().and_then(|s| s["denied"].as_bool()).unwrap_or(false);
    let uploads = UPLOADS.lock().unwrap_or_else(PoisonError::into_inner);
    let texts: serde_json::Map<String, Value> = TEXT_KEYS.iter().map(|(field, _, _)| (field.to_string(), json!(text(field)))).collect();
    let flags: serde_json::Map<String, Value> = FLAG_KEYS.iter().map(|(field, _, _)| (field.to_string(), json!(flag(field)))).collect();
    json!({
        "kind": "object",
        "class": "MavlinkLog",
        "vehiclePx4": px4,
        "logRunning": running,
        "canStartLog": px4 && !running && !denied,
        "persistence": !crate::settingsstore::raw_setting("settings.appSettings.disableAllPersistence").and_then(|v| v.as_bool()).unwrap_or(false),
        "settings": Value::Object(texts.into_iter().chain(flags).chain([("feedback".to_string(), json!(uploads.feedback))]).collect()),
        "files": files(),
        "uploading": uploads.running,
        "uploadingFile": uploads.current,
        "message": uploads.message,
        "error": state.as_ref().map(|s| s["error"].clone()).unwrap_or(Value::Null),
    })
}

pub fn owns(path: &str) -> bool {
    [MAVLINK_LOG_START, MAVLINK_LOG_STOP, MAVLINK_LOG_SET, MAVLINK_LOG_UPLOAD, MAVLINK_LOG_DELETE, MAVLINK_LOG_CANCEL].contains(&path)
}

fn names(args: &Value) -> Vec<String> {
    args.as_array().map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default()
}

fn outcome(result: Result<(), String>) -> Value {
    match result {
        Ok(()) => json!({ "ok": true }),
        Err(reason) => json!({ "ok": false, "reason": reason }),
    }
}

fn set(field: &str, value: &Value) -> Result<(), String> {
    if field == "feedback" {
        UPLOADS.lock().unwrap_or_else(PoisonError::into_inner).feedback = value.as_str().unwrap_or_default().to_string();
        return Ok(());
    }
    let stored = TEXT_KEYS.iter().find(|(f, _, _)| *f == field).map(|(_, k, _)| (*k, value.as_str().map(str::to_string).or_else(|| value.as_i64().map(|n| n.to_string()))))
        .or_else(|| FLAG_KEYS.iter().find(|(f, _, _)| *f == field).map(|(_, k, _)| (*k, value.as_bool().map(|b| b.to_string()))))
        .ok_or_else(|| format!("{field} is not a log transfer setting"))?;
    let (name, text) = (stored.0, stored.1.ok_or("That value does not fit the setting.")?);
    crate::settingsstore::written(&key(name), &text);
    if field == "enableAutoStart" {
        configure_hub();
    }
    Ok(())
}

pub fn run(path: &str, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    match path {
        MAVLINK_LOG_START | MAVLINK_LOG_STOP => {
            configure_hub();
            let request = json!({ "action": if path == MAVLINK_LOG_START { "start" } else { "stop" } });
            let sent = crate::hub::lock().log_request(None, &request, crate::hub::now_ms());
            outcome(sent.map(|outbound| {
                outbound.iter().for_each(|(link, bytes)| {
                    crate::linkhost::write(&crate::linkhost::TRANSPORTS, *link, bytes);
                });
            }))
        }
        MAVLINK_LOG_SET => outcome(set(given.get(0).and_then(Value::as_str).unwrap_or_default(), given.get(1).unwrap_or(&Value::Null))),
        MAVLINK_LOG_UPLOAD => outcome(upload(names(&given))),
        MAVLINK_LOG_DELETE => {
            let folder = folder();
            names(&given).iter().filter_map(|name| folder.as_ref().map(|f| f.join(name))).for_each(|path| {
                let _ = std::fs::remove_file(uploaded_marker(&path));
                let _ = std::fs::remove_file(&path);
            });
            json!({ "ok": true })
        }
        MAVLINK_LOG_CANCEL => {
            if let Some(cancel) = UPLOADS.lock().unwrap_or_else(PoisonError::into_inner).cancel.as_ref() {
                cancel.store(true, Ordering::Relaxed);
            }
            json!({ "ok": true })
        }
        _ => json!({ "ok": false, "reason": format!("{path} is not a log transfer action") }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flight_review_form_carries_qgcs_fields_and_the_log_as_filearg() {
        let body = String::from_utf8(multipart(&[("email", "a@b.c".to_string()), ("type", "flightreport".to_string())], "001-x.ulg", b"ULog")).unwrap();
        assert!(body.contains("name=\"email\"\r\n\r\na@b.c\r\n"));
        assert!(body.contains("name=\"filearg\"; filename=\"001-x.ulg\"\r\n\r\nULog\r\n"));
        assert!(body.ends_with(&format!("--{BOUNDARY}--\r\n")));
        let fields = form_fields("");
        let value = |name: &str| fields.iter().find(|(n, _)| *n == name).map(|(_, v)| v.clone()).unwrap();
        assert_eq!((value("feedback"), value("videoUrl"), value("source"), value("rating")), ("None Given".to_string(), "None".to_string(), "QGroundControl".to_string(), "notset".to_string()));
        assert_eq!(value("public"), "true", "PublicLog defaults on as in MAVLinkLogManager");
    }

    #[test]
    fn an_uploaded_log_is_marked_by_a_sidecar_next_to_it() {
        assert_eq!(uploaded_marker(Path::new("/logs/001-2026.ulg")), PathBuf::from("/logs/001-2026.uploaded"));
    }
}
