use std::sync::Mutex;
use std::sync::PoisonError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime, WebviewWindow};

const CLOSE_CHECKS: &str = "view.closeChecks";
const UNANSWERED: Duration = Duration::from_secs(2);

static CLOSING: AtomicBool = AtomicBool::new(false);
static PROMPT: Mutex<Prompt> = Mutex::new(Prompt::Idle);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Prompt {
    Idle,
    Asked(Instant),
    Shown,
}

#[derive(Debug, PartialEq)]
pub enum Close {
    Go,
    Ask,
}

pub fn close_decision(prompt: Prompt, prompts: usize, now: Instant) -> (Close, Prompt) {
    match (prompts, prompt) {
        (0, _) => (Close::Go, Prompt::Idle),
        (_, Prompt::Asked(at)) if now.duration_since(at) >= UNANSWERED => (Close::Go, Prompt::Idle),
        (_, Prompt::Asked(at)) => (Close::Ask, Prompt::Asked(at)),
        _ => (Close::Ask, Prompt::Asked(now)),
    }
}

fn prompt() -> std::sync::MutexGuard<'static, Prompt> {
    PROMPT.lock().unwrap_or_else(PoisonError::into_inner)
}

fn prompt_messages(checks: &Value, wanted: impl Fn(&Value) -> bool) -> Vec<String> {
    checks["prompts"]
        .as_array()
        .map(|prompts| {
            prompts
                .iter()
                .filter(|prompt| wanted(prompt))
                .filter_map(|prompt| prompt["message"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn close_prompts(checks: &Value) -> Vec<String> {
    prompt_messages(checks, |_| true)
}

pub fn work_lost(checks: &Value) -> Vec<String> {
    prompt_messages(checks, |prompt| prompt["losesWork"] == true)
}

pub fn work_at_risk() -> Vec<String> {
    work_lost(&crate::bridge::core_value(CLOSE_CHECKS))
}

pub fn holds_close<R: Runtime>(app: &AppHandle<R>) -> bool {
    if CLOSING.load(Ordering::Relaxed) {
        return false;
    }
    let asked = close_prompts(&crate::bridge::core_value(CLOSE_CHECKS));
    let mut held = prompt();
    let (close, next) = close_decision(*held, asked.len(), Instant::now());
    if matches!(*held, Prompt::Asked(_)) && close == Close::Go {
        log::warn!(target: "desktop.window", "event=close_prompt_unanswered reasons={}", asked.len());
        CLOSING.store(true, Ordering::Relaxed);
    }
    *held = next;
    drop(held);
    if close == Close::Go {
        return false;
    }
    log::info!(target: "desktop.window", "event=close_held reasons={}", asked.len());
    if let Err(error) = app.emit("close-blocked", asked) {
        log::warn!(target: "desktop.window", "event=close_prompt_failed reason=\"{error}\"");
        *prompt() = Prompt::Idle;
        return false;
    }
    true
}

#[tauri::command]
pub fn close_prompt_shown() {
    *prompt() = Prompt::Shown;
}

#[tauri::command]
pub fn close_prompt_dismissed() {
    *prompt() = Prompt::Idle;
}

#[tauri::command]
pub fn close_anyway<R: Runtime>(app: AppHandle<R>) {
    log::info!(target: "desktop.window", "event=closed_anyway");
    quit_now(&app);
}

pub fn quit_now<R: Runtime>(app: &AppHandle<R>) {
    CLOSING.store(true, Ordering::Relaxed);
    app.exit(0);
}

#[cfg(target_os = "macos")]
mod terminate {
    use std::sync::OnceLock;

    use objc2::ffi::class_addMethod;
    use objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
    use objc2::sel;

    const TERMINATE_CANCEL: usize = 0;
    const TERMINATE_NOW: usize = 1;

    type Holds = Box<dyn Fn() -> bool + Send + Sync>;

    static HOLDS: OnceLock<Holds> = OnceLock::new();

    extern "C-unwind" fn should_terminate(_: &AnyObject, _: Sel, _: *mut AnyObject) -> usize {
        match HOLDS.get().is_some_and(|holds| holds()) {
            true => TERMINATE_CANCEL,
            false => TERMINATE_NOW,
        }
    }

    pub fn guard(holds: Holds) -> Result<(), String> {
        let _ = HOLDS.set(holds);
        let delegate = AnyClass::get(c"TaoAppDelegateParent")
            .ok_or("The app delegate class is missing, so a Quit cannot be held.")?;
        type Answer = extern "C-unwind" fn(&AnyObject, Sel, *mut AnyObject) -> usize;
        let answer: Answer = should_terminate;
        let added = unsafe {
            class_addMethod(
                std::ptr::from_ref(delegate).cast_mut(),
                sel!(applicationShouldTerminate:),
                std::mem::transmute::<Answer, Imp>(answer),
                c"Q@:@".as_ptr(),
            )
        };
        added
            .as_bool()
            .then_some(())
            .ok_or_else(|| "The app delegate already answers Quit.".to_string())
    }
}

#[cfg(target_os = "macos")]
pub fn guard_terminate<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    if let Err(reason) = terminate::guard(Box::new(move || holds_close(&app))) {
        log::warn!(target: "desktop.window", "event=quit_unguarded reason=\"{reason}\"");
    }
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
        assert_eq!(
            work_lost(&checks),
            [
                "The mission has unsaved changes.",
                "Parameter writes are still pending."
            ],
            "an update asks to save these first, as installing closes the app"
        );
        assert!(close_prompts(&json!({ "prompts": [] })).is_empty());
        assert!(close_prompts(&Value::Null).is_empty());
    }

    #[test]
    fn a_prompt_the_page_never_showed_lets_a_later_close_through() {
        let asked = Instant::now();
        assert_eq!(
            close_decision(Prompt::Idle, 0, asked),
            (Close::Go, Prompt::Idle)
        );
        assert_eq!(
            close_decision(Prompt::Idle, 1, asked),
            (Close::Ask, Prompt::Asked(asked))
        );
        assert_eq!(
            close_decision(Prompt::Shown, 1, asked),
            (Close::Ask, Prompt::Asked(asked)),
            "a close while the dialog is up asks again"
        );
        assert_eq!(
            close_decision(Prompt::Asked(asked), 1, asked + UNANSWERED / 4),
            (Close::Ask, Prompt::Asked(asked)),
            "a held Cmd+Q or a double click lands before the page can answer, so it asks again"
        );
        assert_eq!(
            close_decision(Prompt::Asked(asked), 1, asked + UNANSWERED),
            (Close::Go, Prompt::Idle),
            "a page that never answered the prompt cannot hold the window, so a later close goes through"
        );
    }
}
