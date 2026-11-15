use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};

use groundstation::nativehost::{qgc_bridge_free, qgc_bridge_get};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime, WebviewWindow};

const CLOSE_CHECKS: &str = "view.closeChecks";

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

pub fn close_prompts(checks: &Value) -> Vec<String> {
    checks["prompts"]
        .as_array()
        .map(|prompts| {
            prompts
                .iter()
                .filter_map(|prompt| prompt["message"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn holds_close<R: Runtime>(app: &AppHandle<R>) -> bool {
    if CLOSING.load(Ordering::Relaxed) {
        return false;
    }
    let asked = close_prompts(&close_checks());
    if asked.is_empty() {
        return false;
    }
    log::info!(target: "desktop.window", "event=close_held reasons={}", asked.len());
    if let Err(error) = app.emit("close-blocked", asked) {
        log::warn!(target: "desktop.window", "event=close_prompt_failed reason=\"{error}\"");
        return false;
    }
    true
}

#[tauri::command]
pub fn close_anyway<R: Runtime>(app: AppHandle<R>) {
    log::info!(target: "desktop.window", "event=closed_anyway");
    CLOSING.store(true, Ordering::Relaxed);
    app.exit(0);
}

#[cfg(target_os = "macos")]
pub fn guard_quit<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
    const QUIT: &str = "guarded-quit";
    const APPLICATION_MENU: usize = 0;
    let name = app.package_info().name.clone();
    let about = AboutMetadata {
        name: Some(name.clone()),
        version: Some(app.package_info().version.to_string()),
        ..Default::default()
    };
    let application = Submenu::with_items(
        app,
        &name,
        true,
        &[
            &PredefinedMenuItem::about(app, None, Some(about))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, QUIT, format!("Quit {name}"), true, Some("CmdOrCtrl+Q"))?,
        ],
    )?;
    let menu = Menu::default(app)?;
    menu.remove_at(APPLICATION_MENU)?;
    menu.insert(&application, APPLICATION_MENU)?;
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
    fn every_close_prompt_holds_the_window_open_as_the_native_heads_do() {
        let checks = json!({ "prompts": [
            { "id": "activeConnections", "message": "The vehicle is connected.", "losesWork": false },
            { "id": "unsavedMission", "message": "The mission has unsaved changes.", "losesWork": true },
            { "id": "pendingWrites", "message": "Parameter writes are still pending.", "losesWork": true },
        ]});
        assert_eq!(
            close_prompts(&checks),
            [
                "The vehicle is connected.",
                "The mission has unsaved changes.",
                "Parameter writes are still pending."
            ],
            "a live connection asks too, so a Cmd+Q in flight is never silent"
        );
        assert!(close_prompts(&json!({ "prompts": [] })).is_empty());
        assert!(close_prompts(&Value::Null).is_empty());
    }
}
