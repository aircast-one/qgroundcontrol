use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::router::Backend;

pub const SETUP_CHANGED: &str = "core.deepLinkSetup@changed";
pub const DEPS: &[&str] = &[SETUP_CHANGED];
pub const ACCEPT: &str = "deepLinkSetup.accept";
pub const DECLINE: &str = "deepLinkSetup.decline";

static PENDING: Mutex<Option<String>> = Mutex::new(None);
static DIRTY: AtomicBool = AtomicBool::new(false);

fn pending() -> std::sync::MutexGuard<'static, Option<String>> {
    PENDING.lock().unwrap_or_else(PoisonError::into_inner)
}

fn replace(next: Option<String>) -> Option<String> {
    let before = std::mem::replace(&mut *pending(), next);
    DIRTY.store(true, Ordering::Relaxed);
    before
}

pub fn offer(host: &str) {
    let mut held = pending();
    if held.is_none() {
        *held = Some(host.to_string());
        DIRTY.store(true, Ordering::Relaxed);
    }
}

pub fn take_changed() -> bool {
    DIRTY.swap(false, Ordering::Relaxed)
}

pub fn owns(path: &str) -> bool {
    path == ACCEPT || path == DECLINE
}

pub fn view_of(host: Option<&str>) -> Value {
    json!({
        "kind": "object",
        "class": "DeepLinkSetup",
        "show": host.is_some(),
        "host": host,
        "title": "Set up from this device?",
        "text": host.map(|host| format!("A link asks Aircast to set itself up from {host}. Accepting adds that device's cameras and connects to the telemetry and Aircast account server it names. Only accept a link you opened from your own device.")),
        "accept": "Set up",
        "decline": "Ignore",
    })
}

pub fn deep_link_setup_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    view_of(pending().as_deref())
}

pub fn setup_from_device(host: &str) {
    if let Some(config) = crate::devicesetup::fetch(host, crate::devicesetup::STREAM_CONFIG) {
        let via =
            crate::devicesetup::fetch(host, crate::devicesetup::WATCH_VIA).unwrap_or(Value::Null);
        crate::cameras::adopt_device(
            crate::devicesetup::bare_host(host),
            crate::devicesetup::device_cameras(host, &config, &via),
        );
    }
    let Some(telemetry) = crate::devicesetup::fetch(host, crate::devicesetup::TELEMETRY_CONFIG)
    else {
        return;
    };
    if let Some((api_base, link)) = crate::devicesetup::cloud_link(host, &telemetry) {
        crate::account::set_api_base(&api_base);
        crate::corelinks::replace_and_connect(link);
    }
    if let Some(link) = crate::devicesetup::telemetry_link(host, &telemetry) {
        crate::corelinks::replace_and_connect(link);
    }
}

pub const DEVICE_SETUP_UNAVAILABLE: &str = "Setting up a device is not available in this app yet.";

pub fn run(path: &str, args: &str, setup: impl FnOnce(String) + Send + 'static) -> Value {
    let shown = serde_json::from_str::<Value>(args)
        .ok()
        .and_then(|a| a.get(0)?.as_str().map(str::to_string));
    let waiting = pending().clone();
    match (path, waiting) {
        (ACCEPT, Some(host)) if shown.as_deref() == Some(host.as_str()) => {
            replace(None);
            match crate::background::spawn("groundstation-device-setup", move || setup(host)) {
                Ok(()) => json!({ "ok": true }),
                Err(_) => json!({ "ok": false, "reason": DEVICE_SETUP_UNAVAILABLE }),
            }
        }
        (ACCEPT, Some(_)) => {
            json!({ "ok": false, "reason": "The device waiting to set up is not the one that was shown." })
        }
        (ACCEPT, None) => json!({ "ok": false, "reason": "There is no device setup waiting." }),
        _ => {
            replace(None);
            json!({ "ok": true })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_link_waits_for_the_pilot_and_runs_only_when_accepted() {
        assert_eq!(view_of(None)["show"], false);
        let shown = view_of(Some("10.0.0.5"));
        assert_eq!(
            (shown["show"].as_bool(), shown["host"].as_str()),
            (Some(true), Some("10.0.0.5"))
        );
        offer("evil.example");
        assert!(take_changed());
        assert_eq!(
            run(DECLINE, "[]", |_| panic!("a declined link sets nothing up")),
            json!({ "ok": true })
        );
        assert!(pending().is_none());
        assert_eq!(
            run(ACCEPT, r#"["10.0.0.5"]"#, |_| panic!("nothing is waiting"))["ok"],
            false
        );
        offer("10.0.0.5");
        offer("evil.example");
        assert_eq!(
            pending().as_deref(),
            Some("10.0.0.5"),
            "a second link never replaces the one on screen"
        );
        assert_eq!(
            run(ACCEPT, r#"["evil.example"]"#, |_| panic!(
                "only the host shown is accepted"
            ))["ok"],
            false
        );
        let (sent, got) = std::sync::mpsc::channel();
        assert_eq!(
            run(ACCEPT, r#"["10.0.0.5"]"#, move |host| sent
                .send(host)
                .unwrap())["ok"],
            true
        );
        assert_eq!(
            got.recv_timeout(std::time::Duration::from_secs(2)).unwrap(),
            "10.0.0.5"
        );
    }
}
