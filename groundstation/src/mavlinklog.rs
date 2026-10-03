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
    pending: Vec<String>,
}

static UPLOADS: Mutex<Uploads> = Mutex::new(Uploads { running: false, current: None, progress: 0.0, message: None, cancel: None, feedback: String::new(), was_running: None, pending: Vec::new() });

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

fn persistence() -> bool {
    !crate::settingsstore::raw_setting("settings.appSettings.disableAllPersistence").and_then(|v| v.as_bool()).unwrap_or(false)
}

fn stem(name: &str) -> String {
    name.strip_suffix(MAVLINK_LOG_EXTENSION).unwrap_or(name).to_string()
}

fn file_of(name: &str) -> String {
    format!("{}{MAVLINK_LOG_EXTENSION}", stem(name))
}

fn writing_file() -> Option<String> {
    let state = crate::hub::lock().active().map(|vehicle| vehicle.log_snapshot())?;
    state["running"].as_bool().filter(|running| *running)?;
    state["file"].as_str().and_then(|f| Path::new(f).file_name().map(|n| n.to_string_lossy().into_owned()))
}

pub fn configure_hub() {
    let request = json!({ "path": folder().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(), "extension": MAVLINK_LOG_EXTENSION, "autoStart": flag("enableAutoStart") && persistence() });
    let _ = crate::hub::lock().log_request(None, &request, crate::hub::now_ms());
}

fn files() -> Vec<Value> {
    let Some(folder) = folder() else { return Vec::new() };
    let writing = writing_file();
    let listed: std::collections::BTreeMap<String, (u64, bool)> = std::fs::read_dir(&folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.to_string_lossy().ends_with(MAVLINK_LOG_EXTENSION))
                .map(|path| {
                    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    (path.file_name().unwrap_or_default().to_string_lossy().into_owned(), (size, uploaded_marker(&path).exists()))
                })
                .collect()
        })
        .unwrap_or_default();
    listed.into_iter().map(|(name, (size, uploaded))| json!({ "name": stem(&name), "size": size, "uploaded": uploaded, "writing": writing.as_deref() == Some(name.as_str()) })).collect()
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

struct Sending<'a> {
    body: std::io::Cursor<Vec<u8>>,
    cancel: &'a AtomicBool,
}

impl std::io::Read for Sending<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(std::io::Error::other("upload cancelled"));
        }
        let read = self.body.read(buf)?;
        UPLOADS.lock().unwrap_or_else(PoisonError::into_inner).progress = sent_fraction(self.body.position(), self.body.get_ref().len());
        Ok(read)
    }
}

fn sent_fraction(sent: u64, total: usize) -> f64 {
    match total {
        0 => 0.0,
        total => sent as f64 / total as f64,
    }
}

fn send(path: &Path, feedback: &str, cancel: &AtomicBool) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let body = multipart(&form_fields(feedback), &name, &bytes);
    let length = body.len();
    let mut sending = Sending { body: std::io::Cursor::new(body), cancel };
    let response = ureq::post(&text("uploadURL"))
        .header("Content-Type", &format!("multipart/form-data; boundary={BOUNDARY}"))
        .header("Content-Length", &length.to_string())
        .send(ureq::SendBody::from_reader(&mut sending))
        .map_err(|e| format!("Log Upload Error: {e}"))?;
    match response.status().as_u16() {
        200 => Ok(()),
        code => Err(format!("Log Upload Error: status {code}")),
    }
}

fn upload_one(folder: &Path, name: &str, feedback: &str, cancel: &AtomicBool) -> Result<(), String> {
    {
        let mut uploads = UPLOADS.lock().unwrap_or_else(PoisonError::into_inner);
        uploads.current = Some(stem(name));
        uploads.progress = 0.0;
    }
    let path = folder.join(name);
    send(&path, feedback, cancel)?;
    match flag("deleteAfterUpload") {
        true => {
            let _ = std::fs::remove_file(&path);
        }
        false => {
            let _ = std::fs::write(uploaded_marker(&path), b"");
        }
    }
    Ok(())
}

fn upload(names: Vec<String>) -> Result<(), String> {
    if text("emailAddress").is_empty() {
        return Err("Please enter an email address before uploading MAVLink log files.".to_string());
    }
    let folder = folder().ok_or("There is no log folder.")?;
    let writing = writing_file();
    let names: Vec<String> = names.iter().map(|name| file_of(name)).filter(|name| !uploaded_marker(&folder.join(name)).exists() && writing.as_deref() != Some(name.as_str())).collect();
    let cancel = Arc::new(AtomicBool::new(false));
    let feedback = {
        let mut uploads = UPLOADS.lock().unwrap_or_else(PoisonError::into_inner);
        if uploads.running {
            uploads.pending.extend(names);
            return Ok(());
        }
        uploads.running = true;
        uploads.message = None;
        uploads.cancel = Some(cancel.clone());
        uploads.feedback.clone()
    };
    std::thread::spawn(move || {
        let failures: Vec<String> = names.iter().take_while(|_| !cancel.load(Ordering::Relaxed)).filter_map(|name| upload_one(&folder, name, &feedback, &cancel).err()).collect();
        let cancelled = cancel.load(Ordering::Relaxed);
        let pending = {
            let mut uploads = UPLOADS.lock().unwrap_or_else(PoisonError::into_inner);
            uploads.running = false;
            uploads.current = None;
            uploads.cancel = None;
            uploads.progress = 0.0;
            uploads.message = (!failures.is_empty() && !cancelled).then(|| failures.join("\n"));
            let queued = std::mem::take(&mut uploads.pending);
            if cancelled { Vec::new() } else { queued }
        };
        if let Err(reason) = (!pending.is_empty()).then(|| upload(pending)).unwrap_or(Ok(())) {
            UPLOADS.lock().unwrap_or_else(PoisonError::into_inner).message = Some(reason);
        }
    });
    Ok(())
}

pub fn tick() {
    let state = crate::hub::lock().active().map(|vehicle| vehicle.log_snapshot());
    let running = state.as_ref().and_then(|s| s["running"].as_bool()).unwrap_or(false);
    let failed = state.as_ref().is_some_and(|s| !s["error"].is_null());
    let file = state.as_ref().and_then(|s| s["file"].as_str().map(str::to_string));
    let finished = {
        let mut uploads = UPLOADS.lock().unwrap_or_else(PoisonError::into_inner);
        let was = uploads.was_running.take();
        uploads.was_running = running.then(|| file.clone()).flatten();
        was.filter(|_| !running)
    };
    if let Some(done) = finished.filter(|done| flag("enableAutoUpload") && !failed && Path::new(done).exists()) {
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
        "persistence": persistence(),
        "settings": Value::Object(texts.into_iter().chain(flags).chain([("feedback".to_string(), json!(uploads.feedback))]).collect()),
        "files": files(),
        "uploading": uploads.running,
        "uploadingFile": uploads.current,
        "uploadProgress": uploads.progress,
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
    let text = if field == "uploadURL" && text.trim().is_empty() { DEFAULT_URL.to_string() } else { text };
    crate::settingsstore::written(&key(name), &text);
    if field == "enableAutoStart" {
        configure_hub();
    }
    Ok(())
}

pub fn run(path: &str, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    match path {
        MAVLINK_LOG_START if !persistence() => json!({ "ok": false, "reason": "MAVLink logging is off while all persistence is disabled." }),
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
            let writing = writing_file();
            names(&given).iter().map(|name| file_of(name)).filter(|name| writing.as_deref() != Some(name.as_str())).filter_map(|name| folder.as_ref().map(|f| f.join(name))).for_each(|path| {
                let _ = std::fs::remove_file(uploaded_marker(&path));
                let _ = std::fs::remove_file(&path);
            });
            json!({ "ok": true })
        }
        MAVLINK_LOG_CANCEL => {
            let mut uploads = UPLOADS.lock().unwrap_or_else(PoisonError::into_inner);
            uploads.pending.clear();
            if let Some(cancel) = uploads.cancel.as_ref() {
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
    fn upload_progress_is_the_sent_share_of_the_body_and_cancel_aborts_the_body() {
        assert_eq!((sent_fraction(0, 0), sent_fraction(25, 100), sent_fraction(100, 100)), (0.0, 0.25, 1.0));
        let cancel = AtomicBool::new(true);
        let mut sending = Sending { body: std::io::Cursor::new(vec![1, 2, 3]), cancel: &cancel };
        assert!(std::io::Read::read(&mut sending, &mut [0; 3]).is_err(), "QGC aborts the reply in flight on cancelUpload");
    }

    #[test]
    fn an_uploaded_log_is_marked_by_a_sidecar_next_to_it() {
        assert_eq!(uploaded_marker(Path::new("/logs/001-2026.ulg")), PathBuf::from("/logs/001-2026.uploaded"));
    }
}
