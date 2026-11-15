use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};

use groundstation::nativehost::{qgc_bridge_free, qgc_bridge_get};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime, WebviewWindow};

const CLOSE_CHECKS: &str = "view.closeChecks";
const LOST_ON_LEAVING: [&str; 2] = ["unsavedMission", "pendingWrites"];

static CLOSING: AtomicBool = AtomicBool::new(false);

fn close_checks() -> Value {
    let path = CString::new(CLOSE_CHECKS).unwrap_or_default();
    let raw = unsafe { qgc_bridge_get(path.as_ptr()) };
    if raw.is_null() {
        return Value::Null;
    }
    let text = unsafe { std::ffi::CStr::from_ptr(raw) }
        .to_string_lossy()
        .into_owned();
    unsafe { qgc_bridge_free(raw) };
    serde_json::from_str(&text).unwrap_or(Value::Null)
}

pub fn work_lost_on_leaving(checks: &Value) -> Vec<String> {
    checks["prompts"]
        .as_array()
        .map(|prompts| {
            prompts
                .iter()
                .filter(|prompt| {
                    prompt["id"]
                        .as_str()
                        .is_some_and(|id| LOST_ON_LEAVING.contains(&id))
                })
                .filter_map(|prompt| prompt["message"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn holds_close<R: Runtime>(app: &AppHandle<R>) -> bool {
    if CLOSING.load(Ordering::Relaxed) {
        return false;
    }
    let lost = work_lost_on_leaving(&close_checks());
    if lost.is_empty() {
        return false;
    }
    let _ = app.emit("close-blocked", lost);
    true
}

#[tauri::command]
pub fn close_anyway<R: Runtime>(app: AppHandle<R>) {
    CLOSING.store(true, Ordering::Relaxed);
    app.exit(0);
}

#[cfg(target_os = "macos")]
pub fn guard_quit<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    const QUIT: &str = "guarded-quit";
    let menu = Menu::default(app)?;
    if let Some(application) = menu.items()?.first().and_then(|item| item.as_submenu()) {
        let last = application.items()?.len().saturating_sub(1);
        application.remove_at(last)?;
        application.append(&MenuItem::with_id(
            app,
            QUIT,
            format!("Quit {}", app.package_info().name),
            true,
            Some("CmdOrCtrl+Q"),
        )?)?;
    }
    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        if event.id() == QUIT && !holds_close(app) {
            app.exit(0);
        }
    });
    Ok(())
}

#[tauri::command]
pub fn full_screen<R: Runtime>(window: WebviewWindow<R>) -> bool {
    window.is_fullscreen().unwrap_or(false)
}

#[tauri::command]
pub fn toggle_full_screen<R: Runtime>(window: WebviewWindow<R>) -> Result<bool, String> {
    let full = !window.is_fullscreen().map_err(|error| error.to_string())?;
    window
        .set_fullscreen(full)
        .map_err(|error| error.to_string())?;
    Ok(full)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_unsaved_missions_and_pending_writes_hold_the_window_open() {
        let checks = json!({ "prompts": [
            { "id": "armed", "message": "The vehicle is armed." },
            { "id": "unsavedMission", "message": "The mission has unsaved changes." },
            { "id": "pendingWrites", "message": "Parameter writes are still pending." },
        ]});
        assert_eq!(
            work_lost_on_leaving(&checks),
            [
                "The mission has unsaved changes.",
                "Parameter writes are still pending."
            ]
        );
        assert!(work_lost_on_leaving(&json!({ "prompts": [] })).is_empty());
        assert!(work_lost_on_leaving(&Value::Null).is_empty());
    }
}
