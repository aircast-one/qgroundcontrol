use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

pub const RECORD_SETTING: &str = "settings.appSettings.detectionsRecord";
pub const EXTENSION: &str = "detections.jsonl";

static FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

pub fn wall_us() -> u64 {
    chrono::Utc::now().timestamp_micros().max(0) as u64
}

fn enabled() -> bool {
    crate::settingsstore::raw_setting(RECORD_SETTING)
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn opened() -> Option<std::fs::File> {
    let folder = crate::settingsstore::telemetry_save_path()
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())?;
    std::fs::create_dir_all(&folder).ok()?;
    let stamp = chrono::Local::now().format("%Y-%m-%d %H-%M-%S");
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(folder.join(format!("{stamp}.{EXTENSION}")))
        .ok()
}

fn append(record: Value) {
    let mut file = FILE.lock().unwrap_or_else(PoisonError::into_inner);
    if !enabled() {
        *file = None;
        return;
    }
    if file.is_none() {
        *file = opened();
    }
    if let Some(open) = file.as_mut()
        && writeln!(open, "{record}").is_err()
    {
        *file = None;
    }
}

pub fn detection_record(payload: &Value, wall_us: u64, latency_ms: u64) -> Value {
    json!({
        "wallUs": wall_us,
        "capturedWallUs": wall_us.saturating_sub(latency_ms * 1000),
        "payload": payload,
    })
}

pub fn detection(payload: &Value, received_ms: u64, captured_ms: u64) {
    append(detection_record(
        payload,
        wall_us(),
        received_ms.saturating_sub(captured_ms),
    ));
}

pub fn video_started(file: &str) {
    append(json!({ "wallUs": wall_us(), "video": file }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_detection_is_stamped_on_the_tlog_clock_at_its_capture_time() {
        let record = detection_record(&json!({ "boxes": [] }), 1_700_000_000_000_000, 250);
        assert_eq!(record["capturedWallUs"], 1_699_999_999_750_000_u64);
        assert_eq!(record["payload"]["boxes"], json!([]));
    }
}
