use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Value, json};

use crate::router::Backend;

pub const SET_ADVANCED_UI: &str = "advancedUi.set";
const TITLE: &str = "Advanced Mode";
const TURN_ON_TEXT: &str = "WARNING: You are about to enter Advanced Mode. If used incorrectly, this may cause your vehicle to malfunction thus voiding your warranty. You should do so only if instructed by customer support. Are you sure you want to enable Advanced Mode?";
const TURN_OFF_TEXT: &str = "Turn off Advanced Mode?";

static SHOWN: AtomicBool = AtomicBool::new(true);

pub fn shown() -> bool {
    SHOWN.load(Ordering::Relaxed)
}

fn wanted(args: &str) -> Option<bool> {
    serde_json::from_str::<Value>(args).ok()?.get(0)?.as_bool()
}

pub fn set(args: &str) -> Value {
    match wanted(args) {
        Some(show) => {
            SHOWN.store(show, Ordering::Relaxed);
            json!({ "ok": true })
        }
        None => json!({ "ok": false, "reason": "advancedUi.set takes whether Advanced Mode is on" }),
    }
}

pub fn view_of(shown: bool) -> Value {
    json!({
        "kind": "object",
        "class": "AdvancedUi",
        "shown": shown,
        "title": TITLE,
        "confirmation": if shown { TURN_OFF_TEXT } else { TURN_ON_TEXT },
    })
}

pub fn advanced_ui_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    view_of(shown())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_confirmation_asks_for_the_opposite_of_the_current_mode_like_main_window() {
        assert_eq!(view_of(true)["confirmation"], TURN_OFF_TEXT);
        assert!(view_of(false)["confirmation"].as_str().unwrap().starts_with("WARNING: You are about to enter Advanced Mode."));
        assert_eq!(view_of(false)["title"], "Advanced Mode");
    }

    #[test]
    fn the_toggle_takes_a_bool_and_nothing_else() {
        assert_eq!(wanted("[false]"), Some(false));
        assert_eq!(wanted("[true]"), Some(true));
        assert_eq!(wanted("[1]"), None);
        assert_eq!(wanted("[]"), None);
        assert_eq!(set("[\"on\"]")["ok"], false);
    }
}
