use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, Once, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};

use crate::applog::Entry;
use crate::router::Backend;

pub const ACCEPT_OFFER: &str = "betaDiagnostics.accept";
pub const DISMISS_OFFER: &str = "betaDiagnostics.dismiss";
pub const DEPS: &[&str] = &[SHOWN_PATH, ENABLED_PATH];
pub const DEFAULT_ENDPOINT: &str = "otel.aircast.one:443";
const ENABLED_PATH: &str = "settings.logManagerSettings.remoteLoggingEnabled";
const ENDPOINT_PATH: &str = "settings.logManagerSettings.remoteLoggingEndpoint";
const TOKEN_PATH: &str = "settings.logManagerSettings.remoteLoggingToken";
const SHOWN_PATH: &str = "settings.appSettings.firstRunPromptIdsShown";
const PROMPT_ID: i64 = 1000;
const BUILT_IN_TOKEN: Option<&str> = option_env!("OTEL_BETA_INGEST_TOKEN");
const SERVICE_NAME: &str = "aircast-qgc";
const SEND_EVERY: Duration = Duration::from_secs(5);
const SEND_TIMEOUT: Duration = Duration::from_secs(10);
const PENDING_CAP: usize = 2000;
const BATCH: usize = 500;
const SEVERITIES: [(u8, &str); 5] = [
    (5, "DEBUG"),
    (9, "INFO"),
    (13, "WARN"),
    (17, "ERROR"),
    (21, "FATAL"),
];

static PENDING: Mutex<Vec<Entry>> = Mutex::new(Vec::new());
static ON: AtomicBool = AtomicBool::new(false);
static STARTED: Once = Once::new();
static SESSION: LazyLock<String> = LazyLock::new(|| {
    format!(
        "{:x}",
        chrono::Local::now()
            .timestamp_nanos_opt()
            .unwrap_or_default()
    )
});

pub fn owns(path: &str) -> bool {
    path == ACCEPT_OFFER || path == DISMISS_OFFER
}

fn setting_text(path: &str) -> Option<String> {
    crate::settingsstore::raw_setting(path)
        .and_then(|value| value.as_str().map(str::trim).map(str::to_string))
        .filter(|text| !text.is_empty())
}

fn token() -> Option<String> {
    setting_text(TOKEN_PATH).or_else(|| BUILT_IN_TOKEN.map(str::to_string))
}

fn endpoint() -> String {
    setting_text(ENDPOINT_PATH).unwrap_or_else(|| DEFAULT_ENDPOINT.to_string())
}

fn enabled() -> bool {
    crate::settingsstore::raw_setting(ENABLED_PATH)
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

pub fn head() -> &'static str {
    match cfg!(target_family = "wasm") {
        true => "web",
        false => std::env::consts::OS,
    }
}

pub fn beta(version: &str) -> bool {
    crate::appupdate::parsed(version)
        .is_some_and(|parsed| crate::appupdate::channel(&parsed) == "beta")
}

pub fn offered(version: &str, shown: &str, enabled: bool, has_token: bool) -> bool {
    has_token
        && !enabled
        && beta(version)
        && crate::firstrun::shown_ids(shown).contains(&crate::firstrun::INITIAL_SETUP_PROMPT_ID)
        && !crate::firstrun::shown_ids(shown).contains(&PROMPT_ID)
}

fn shown_text(backend: &dyn Backend) -> String {
    backend
        .value(SHOWN_PATH)
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub fn beta_diagnostics_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let enabled = backend
        .value(ENABLED_PATH)
        .get("value")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let endpoint = endpoint();
    json!({
        "kind": "object",
        "class": "BetaDiagnostics",
        "show": offered(&crate::mavlinklog::application_version(), &shown_text(backend), enabled, token().is_some()),
        "title": "Help us debug this beta",
        "text": format!("Share this app's log with Aircast so we can fix the problems you hit. Logs stream to {endpoint} and carry no video or passwords. You can turn this off any time under Settings › App Logging."),
        "accept": "Share logs",
        "dismiss": "Not now",
        "endpoint": endpoint,
    })
}

fn mark_shown(backend: &dyn Backend) -> Value {
    let shown = shown_text(backend);
    let ids = crate::firstrun::shown_ids(&shown);
    if ids.contains(&PROMPT_ID) {
        return json!({ "ok": true });
    }
    let updated = ids
        .iter()
        .chain([&PROMPT_ID])
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(",");
    crate::factwrite::write(
        backend,
        SHOWN_PATH,
        &json!({ "value": updated }).to_string(),
    )
}

pub fn accept(backend: &dyn Backend) -> Value {
    let written =
        crate::factwrite::write(backend, ENABLED_PATH, &json!({ "value": true }).to_string());
    if !crate::read::flag(&written, "ok") {
        return written;
    }
    ON.store(true, Ordering::Relaxed);
    mark_shown(backend)
}

pub fn dismiss(backend: &dyn Backend) -> Value {
    mark_shown(backend)
}

pub fn record(entry: &Entry) {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    let mut pending = PENDING.lock().unwrap_or_else(PoisonError::into_inner);
    if pending.len() < PENDING_CAP {
        pending.push(entry.clone());
    }
}

pub fn url(endpoint: &str) -> String {
    let base = endpoint.trim().trim_end_matches('/');
    match base.starts_with("http://") || base.starts_with("https://") {
        true => format!("{base}/v1/logs"),
        false => format!("https://{base}/v1/logs"),
    }
}

fn attribute(key: &str, value: Value) -> Value {
    json!({ "key": key, "value": value })
}

fn text(value: &str) -> Value {
    json!({ "stringValue": value })
}

fn record_json(entry: &Entry) -> Value {
    let (severity, label) = SEVERITIES[entry.level.min(SEVERITIES.len() - 1)];
    json!({
        "timeUnixNano": entry.timestamp.timestamp_nanos_opt().unwrap_or_default().to_string(),
        "severityNumber": severity,
        "severityText": label,
        "body": text(&entry.message),
        "attributes": [
            attribute("category", text(&entry.category)),
            attribute("code.filepath", text(&entry.file)),
            attribute("code.lineno", json!({ "intValue": entry.line.to_string() })),
        ],
    })
}

pub fn payload(entries: &[Entry], version: &str, head: &str, session: &str) -> Value {
    json!({
        "resourceLogs": [{
            "resource": {
                "attributes": [
                    attribute("service.name", text(SERVICE_NAME)),
                    attribute("service.version", text(version)),
                    attribute("aircast.head", text(head)),
                    attribute("session.id", text(session)),
                ],
            },
            "scopeLogs": [{
                "scope": { "name": "applog" },
                "logRecords": entries.iter().map(record_json).collect::<Vec<_>>(),
            }],
        }],
    })
}

fn take_batch() -> Vec<Entry> {
    let mut pending = PENDING.lock().unwrap_or_else(PoisonError::into_inner);
    let count = pending.len().min(BATCH);
    pending.drain(..count).collect()
}

fn put_back(batch: Vec<Entry>) {
    let mut pending = PENDING.lock().unwrap_or_else(PoisonError::into_inner);
    let kept: Vec<Entry> = batch
        .into_iter()
        .chain(std::mem::take(&mut *pending))
        .take(PENDING_CAP)
        .collect();
    *pending = kept;
}

pub fn refresh() {
    let on = enabled() && token().is_some();
    ON.store(on, Ordering::Relaxed);
    if !on {
        PENDING
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
}

fn send_pending() {
    refresh();
    let (Some(token), true) = (token(), ON.load(Ordering::Relaxed)) else {
        return;
    };
    let batch = take_batch();
    if batch.is_empty() {
        return;
    }
    let body = payload(
        &batch,
        &crate::mavlinklog::application_version(),
        head(),
        &SESSION,
    )
    .to_string();
    let sent = crate::http::send(
        "POST",
        &url(&endpoint()),
        &[
            ("Authorization", &format!("Bearer {token}")),
            ("Content-Type", "application/json"),
        ],
        Some(body.as_bytes()),
        SEND_TIMEOUT,
    );
    match sent {
        Ok((status, _)) if (200..300).contains(&status) => {}
        Ok((status, _)) if (400..500).contains(&status) => {
            eprintln!(
                "beta diagnostics: {status} from {}, dropping {} entries",
                endpoint(),
                batch.len()
            );
        }
        _ => put_back(batch),
    }
}

pub fn start() {
    refresh();
    STARTED.call_once(|| {
        let _ = crate::background::poll("beta-diagnostics", SEND_EVERY, || {
            send_pending();
            false
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn the_offer_goes_only_to_beta_builds_that_can_send_and_have_not_answered() {
        assert!(offered("5.6.5-beta.36", "1,3", false, true));
        assert!(offered("v5.6.5-beta.36", "3", false, true));
        assert!(
            !offered("5.6.5-beta.36", "", false, true),
            "it waits for the first-run preferences, so the two never open together"
        );
        assert!(
            !offered("5.6.5", "", false, true),
            "a stable build is never asked"
        );
        assert!(
            !offered("5.6.5-beta.36", "3", true, true),
            "already sharing"
        );
        assert!(
            !offered("5.6.5-beta.36", "3,1000", false, true),
            "already answered"
        );
        assert!(
            !offered("5.6.5-beta.36", "3", false, false),
            "nothing to authenticate with"
        );
        assert!(!offered("dev", "3", false, true));
    }

    #[test]
    fn the_collector_url_accepts_a_host_or_a_full_origin() {
        assert_eq!(
            url("otel.aircast.one:443"),
            "https://otel.aircast.one:443/v1/logs"
        );
        assert_eq!(
            url("http://localhost:4319/"),
            "http://localhost:4319/v1/logs"
        );
    }

    #[test]
    fn a_log_entry_becomes_an_otlp_record_with_its_source_and_severity() {
        let entry = Entry {
            sequence: 7,
            timestamp: chrono::Local
                .with_ymd_and_hms(2026, 11, 8, 10, 0, 0)
                .unwrap(),
            elapsed_ms: 10,
            level: 2,
            category: "groundstation::hub".into(),
            message: "link lost".into(),
            file: "src/hub.rs".into(),
            line: 12,
        };
        let body = payload(&[entry.clone()], "5.6.5-beta.36", "android", "abc");
        let resource = &body["resourceLogs"][0]["resource"]["attributes"];
        assert_eq!(resource[0]["value"]["stringValue"], "aircast-qgc");
        assert_eq!(resource[1]["value"]["stringValue"], "5.6.5-beta.36");
        assert_eq!(resource[2]["value"]["stringValue"], "android");
        assert_eq!(resource[3]["value"]["stringValue"], "abc");
        let record = &body["resourceLogs"][0]["scopeLogs"][0]["logRecords"][0];
        assert_eq!(record["severityNumber"], 13);
        assert_eq!(record["severityText"], "WARN");
        assert_eq!(record["body"]["stringValue"], "link lost");
        assert_eq!(
            record["attributes"][0]["value"]["stringValue"],
            "groundstation::hub"
        );
        assert_eq!(record["attributes"][2]["value"]["intValue"], "12");
        assert_eq!(
            record["timeUnixNano"],
            entry.timestamp.timestamp_nanos_opt().unwrap().to_string()
        );
    }

    #[test]
    fn a_failed_batch_returns_to_the_front_without_growing_past_the_cap() {
        let _held = TEST_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let entry = |n: u64| Entry {
            sequence: n,
            timestamp: chrono::Local::now(),
            elapsed_ms: 0,
            level: 1,
            category: String::new(),
            message: n.to_string(),
            file: String::new(),
            line: 0,
        };
        *PENDING.lock().unwrap() = (0..PENDING_CAP as u64).map(entry).collect();
        let batch = take_batch();
        assert_eq!(batch.len(), BATCH);
        PENDING.lock().unwrap().push(entry(9999));
        put_back(batch);
        let pending = PENDING.lock().unwrap();
        assert_eq!(pending.len(), PENDING_CAP);
        assert_eq!(pending[0].sequence, 0, "the failed batch goes first");
        assert!(
            pending.iter().all(|e| e.sequence != 9999),
            "the newest entry is the one dropped"
        );
        drop(pending);
        PENDING.lock().unwrap().clear();
    }

    #[test]
    fn nothing_is_kept_while_sharing_is_off() {
        let _held = TEST_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        ON.store(false, Ordering::Relaxed);
        PENDING.lock().unwrap().clear();
        record(&Entry {
            sequence: 0,
            timestamp: chrono::Local::now(),
            elapsed_ms: 0,
            level: 1,
            category: String::new(),
            message: String::new(),
            file: String::new(),
            line: 0,
        });
        assert!(PENDING.lock().unwrap().is_empty());
    }
}
