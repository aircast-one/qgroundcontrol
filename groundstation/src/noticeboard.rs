use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};

pub const NOTICES_CHANGED: &str = "core.notices@changed";
pub const MESSAGE: &str = "message";
pub const VEHICLE_ERROR: &str = "vehicleError";
pub const REBOOT_VEHICLE_TEXT: &str = "Reboot vehicle for changes to take effect.";
pub const RESTART_APPLICATION_TEXT: &str = "Restart application for changes to take effect.";
const REBOOT_DEBOUNCE_MS: i64 = 2 * 60 * 1000;
static LAST_REBOOT_NOTICE_MS: std::sync::Mutex<Option<i64>> = std::sync::Mutex::new(None);

pub fn reboot_debounced(last: Option<i64>, now_ms: i64) -> bool {
    last.is_some_and(|previous| now_ms - previous < REBOOT_DEBOUNCE_MS)
}

pub fn reboot_notice_after_write(vehicle_reboot: bool, application_restart: bool, now_ms: i64) {
    let text = match (vehicle_reboot, application_restart) {
        (true, _) => REBOOT_VEHICLE_TEXT,
        (false, true) => RESTART_APPLICATION_TEXT,
        (false, false) => return,
    };
    let debounced = {
        let mut last = LAST_REBOOT_NOTICE_MS.lock().unwrap_or_else(PoisonError::into_inner);
        let debounced = reboot_debounced(*last, now_ms);
        *last = Some(now_ms);
        debounced
    };
    if !debounced {
        post(MESSAGE, "", text);
    }
}

pub const KINDS: [&str; 3] = ["message", "vehicleError", "navigation"];
const MAX_NOTICES: usize = 64;
const KEEP_OLDEST: usize = 8;

#[derive(Debug, Clone, PartialEq)]
struct Notice {
    id: i64,
    kind: String,
    title: String,
    text: String,
    repeated: i64,
    at: i64,
    last_at: i64,
}

#[derive(Debug)]
pub struct Board {
    notices: Vec<Notice>,
    next_id: i64,
    dropped: i64,
}

impl Default for Board {
    fn default() -> Self {
        Board { notices: Vec::new(), next_id: 1, dropped: 0 }
    }
}

impl Board {
    pub fn post(&mut self, kind: &str, title: &str, text: &str, now_ms: i64) -> bool {
        if !KINDS.contains(&kind) {
            return false;
        }
        if let Some(newest) = self.notices.last_mut().filter(|n| n.kind == kind && n.title == title && n.text == text) {
            newest.repeated += 1;
            newest.last_at = now_ms;
            return true;
        }
        while self.notices.len() >= MAX_NOTICES {
            self.notices.remove(KEEP_OLDEST.min(self.notices.len() - 1));
            self.dropped += 1;
        }
        self.notices.push(Notice { id: self.next_id, kind: kind.to_string(), title: title.to_string(), text: text.to_string(), repeated: 0, at: now_ms, last_at: now_ms });
        self.next_id += 1;
        true
    }

    pub fn acknowledge(&mut self, id: i64) -> bool {
        let before = self.notices.len();
        self.notices.retain(|n| n.id != id);
        self.notices.len() != before
    }

    pub fn acknowledge_through(&mut self, id: i64) -> i64 {
        let before = self.notices.len();
        self.notices.retain(|n| n.id > id);
        (before - self.notices.len()) as i64
    }

    pub fn object(&self) -> Value {
        let notices: Vec<Value> = self
            .notices
            .iter()
            .map(|n| json!({ "id": n.id, "kind": n.kind, "repeated": n.repeated, "title": n.title, "text": n.text, "at": n.at, "lastAt": n.last_at }))
            .collect();
        json!({ "kind": "object", "notices": notices, "count": self.notices.len(), "dropped": self.dropped, "order": "oldestFirst" })
    }
}

static BOARD: LazyLock<Mutex<Board>> = LazyLock::new(|| Mutex::new(Board::default()));
static CHANGED_SINCE: AtomicBool = AtomicBool::new(false);
static APPLICATION_NAME: Mutex<String> = Mutex::new(String::new());

pub fn application_name() -> String {
    APPLICATION_NAME.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

pub fn set_application_name(name: &str) {
    *APPLICATION_NAME.lock().unwrap_or_else(PoisonError::into_inner) = name.to_string();
}

fn silenced(kind: &str, text: &str) -> bool {
    kind == VEHICLE_ERROR && (text.starts_with("PreArm") || text.get(..9).is_some_and(|head| head.eq_ignore_ascii_case("preflight")))
}

pub fn post_from_vehicle(kind: &str, text: &str) -> bool {
    let title = application_name();
    !crate::qthost::present() && !silenced(kind, text) && post(kind, &title, text)
}

pub fn lock() -> MutexGuard<'static, Board> {
    BOARD.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn mark_changed() {
    CHANGED_SINCE.store(true, Ordering::SeqCst);
}

pub fn take_changed() -> bool {
    CHANGED_SINCE.swap(false, Ordering::SeqCst)
}

pub fn post(kind: &str, title: &str, text: &str) -> bool {
    let posted = lock().post(kind, title, text, chrono::Utc::now().timestamp_millis());
    if posted {
        mark_changed();
    }
    posted
}

pub fn get(path: &str) -> Option<Value> {
    let whole = lock().object();
    match path {
        "host" => Some(whole),
        _ => {
            let field = path.strip_prefix("host.")?;
            Some(match whole.get(field).filter(|_| field != "kind") {
                Some(value) => json!({ "kind": "value", "value": value }),
                None => json!({ "kind": "value", "value": null, "found": false }),
            })
        }
    }
}

pub fn invoke(path: &str, args: &str) -> Option<Value> {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let id = || given.get(0).and_then(Value::as_i64);
    let text = |i: usize| given.get(i).and_then(Value::as_str).unwrap_or_default().to_string();
    let result = match path {
        "host.postNotice" => json!(post(&text(0), &text(1), &text(2))),
        "host.acknowledge" => json!(id().is_some_and(|id| lock().acknowledge(id))),
        "host.acknowledgeThrough" => json!(id().map_or(0, |id| lock().acknowledge_through(id))),
        _ => return None,
    };
    mark_changed();
    Some(json!({ "ok": true, "result": result }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn reboot_prompts_debounce_for_two_minutes_like_qgc_application() {
        assert!(!super::reboot_debounced(None, 1_000));
        assert!(super::reboot_debounced(Some(1_000), 1_000 + 119_999));
        assert!(!super::reboot_debounced(Some(1_000), 1_000 + 120_000));
    }

    use super::*;

    #[test]
    fn a_vehicle_error_about_prearm_is_left_to_the_health_report() {
        assert!(silenced(VEHICLE_ERROR, "PreArm: RC not calibrated") && silenced(VEHICLE_ERROR, "PREFLIGHT fail"));
        assert!(!silenced(MESSAGE, "PreArm: RC not calibrated"), "showAppMessage has no such filter");
        assert!(!silenced(VEHICLE_ERROR, "Vehicle 2: PreArm: RC not calibrated"), "QGCApplication checks the text after Vehicle has prefixed it");
    }

    #[test]
    fn a_run_of_identical_notices_folds_and_a_flood_keeps_the_oldest_eight() {
        let mut board = Board::default();
        assert!(board.post("message", "Parameters", "missing", 10));
        assert!(board.post("message", "Parameters", "missing", 20));
        assert!(!board.post("shouting", "x", "y", 30));
        let listed = board.object();
        assert_eq!((listed["count"].as_i64(), listed["notices"][0]["repeated"].as_i64(), listed["notices"][0]["lastAt"].as_i64()), (Some(1), Some(1), Some(20)));
        assert!(board.acknowledge(1) && !board.acknowledge(1));
        (0..70).for_each(|i| {
            board.post("message", "", &format!("message {i}"), i);
        });
        let flooded = board.object();
        let text = |i: usize| flooded["notices"][i]["text"].as_str().unwrap().to_string();
        assert_eq!((flooded["count"].as_i64(), flooded["dropped"].as_i64()), (Some(64), Some(6)));
        assert_eq!((text(0), text(7), text(8), text(63)), ("message 0".into(), "message 7".into(), "message 14".into(), "message 69".into()));
        assert_eq!(board.acknowledge_through(flooded["notices"][9]["id"].as_i64().unwrap()), 10);
    }
}
