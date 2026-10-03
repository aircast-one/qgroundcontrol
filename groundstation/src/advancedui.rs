use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Value, json};

use crate::router::Backend;

pub const SET_ADVANCED_UI: &str = "advancedUi.set";

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
    })
}

pub fn advanced_ui_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    view_of(shown())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_view_serves_only_the_flag_since_help_settings_toggles_without_asking() {
        assert_eq!(view_of(false), json!({ "kind": "object", "class": "AdvancedUi", "shown": false }));
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
