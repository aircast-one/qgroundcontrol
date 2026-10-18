use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};

const CLIENT_ID: &str = "aircast-qgc";
const DEVICE_CODE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
const API_BASE_KEY: &str = "AircastAccount/apiBase";
const DEFAULT_POLL_SECS: u64 = 5;

#[derive(Debug, Clone, Default, PartialEq)]
struct Pending {
    user_code: String,
    verification_url: String,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

#[derive(Debug, Clone, Default, PartialEq)]
struct Drones {
    base: String,
    list: Vec<Value>,
    status: String,
    asked_ms: Option<i64>,
}

static DRONES: Mutex<Drones> = Mutex::new(Drones { base: String::new(), list: Vec::new(), status: String::new(), asked_ms: None });
const DRONES_REFRESH_MS: i64 = 30_000;
const DRONES_TIMEOUT: Duration = Duration::from_secs(15);
static STATUS: Mutex<String> = Mutex::new(String::new());
static ATTEMPT: AtomicU64 = AtomicU64::new(0);

type Post = dyn Fn(&str, &Value) -> Result<Value, String> + Send + Sync;

#[derive(Debug, Clone, PartialEq)]
pub enum Poll {
    Token(String),
    Wait,
    Refused(String),
}

pub fn endpoint(api_base: &str, path: &str) -> String {
    format!("{}{path}", api_base.trim_end_matches('/'))
}

pub fn poll_outcome(answer: &Result<Value, String>) -> Poll {
    let body = answer.as_ref().ok();
    let token = body.and_then(|b| b.get("access_token")).and_then(Value::as_str).filter(|t| !t.is_empty());
    let error = body.and_then(|b| b.get("error")).and_then(Value::as_str).unwrap_or_default();
    match (token, error) {
        (Some(token), _) => Poll::Token(token.to_string()),
        (None, "authorization_pending" | "slow_down") => Poll::Wait,
        (None, "") if answer.is_err() => Poll::Wait,
        (None, "expired_token") => Poll::Refused("The sign-in code expired \u{2014} start again.".to_string()),
        (None, _) => Poll::Refused(format!("Sign-in failed: {}", body.and_then(|b| b.get("error_description")).and_then(Value::as_str).unwrap_or_default())),
    }
}

pub fn api_base() -> String {
    crate::settingsstore::stored_text(API_BASE_KEY).unwrap_or_default()
}

fn token_key() -> Option<String> {
    crate::cloudlink::token_key(&api_base())
}

fn signed_in() -> bool {
    token_key().and_then(|key| crate::settingsstore::stored_text(&key)).is_some_and(|t| !t.is_empty())
}

fn set_status(status: &str) {
    *STATUS.lock().unwrap_or_else(PoisonError::into_inner) = status.to_string();
}

fn finish(status: &str) {
    ATTEMPT.fetch_add(1, Ordering::SeqCst);
    *PENDING.lock().unwrap_or_else(PoisonError::into_inner) = None;
    set_status(status);
}

pub fn seen_text(ago: chrono::Duration) -> String {
    let minutes = ago.num_minutes().max(0);
    match (minutes, minutes / 60, minutes / 1440) {
        (0, _, _) => "seen just now".to_string(),
        (m, 0, _) => format!("seen {m} min ago"),
        (_, h, 0) => format!("seen {h} h ago"),
        (_, _, d) => format!("seen {d} d ago"),
    }
}

pub fn drones_from(body: &Value, now: chrono::DateTime<chrono::Utc>) -> Vec<Value> {
    let seen = |device: &Value| device.get("last_seen_at").and_then(Value::as_str).and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok()).map(|t| t.with_timezone(&chrono::Utc));
    body.as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|device| {
            let id = device.get("id").and_then(Value::as_str).filter(|id| !id.trim().is_empty())?.to_string();
            let name = device.get("name").and_then(Value::as_str).filter(|n| !n.trim().is_empty()).unwrap_or(&id).to_string();
            let last = seen(device);
            let line = last.map_or_else(|| "never seen".to_string(), |at| seen_text(now - at));
            Some(((std::cmp::Reverse(last.map_or(i64::MIN, |at| at.timestamp())), id.clone()), json!({ "id": id, "name": name, "seen": line })))
        })
        .collect::<std::collections::BTreeMap<_, _>>()
        .into_values()
        .collect()
}

fn drones_due(signed_in: bool, base: &str, now_ms: i64) -> bool {
    let held = DRONES.lock().unwrap_or_else(PoisonError::into_inner);
    signed_in && (held.base != base || held.asked_ms.is_none_or(|at| now_ms - at >= DRONES_REFRESH_MS))
}

pub fn refresh_drones() {
    let base = api_base();
    let Some(token) = token_key().and_then(|key| crate::settingsstore::stored_text(&key)).filter(|t| !t.is_empty()) else { return };
    let asked = crate::hub::wall_ms();
    {
        let mut held = DRONES.lock().unwrap_or_else(PoisonError::into_inner);
        if held.base != base {
            *held = Drones { base: base.clone(), ..Drones::default() };
        }
        held.asked_ms = Some(asked);
    }
    let _ = crate::background::spawn("groundstation-account-drones", move || {
        let bearer = format!("Bearer {token}");
        let answer = crate::http::send("GET", &endpoint(&base, "/v1/user/devices"), &[("Authorization", bearer.as_str()), ("Accept", "application/json")], None, DRONES_TIMEOUT);
        let mut held = DRONES.lock().unwrap_or_else(PoisonError::into_inner);
        if held.base != base {
            return;
        }
        match answer {
            Ok((status, body)) if (200..300).contains(&status) => {
                held.list = drones_from(&serde_json::from_slice(&body).unwrap_or(Value::Null), chrono::Utc::now());
                held.status = if held.list.is_empty() { "No drones on this account yet. Connect one to the cloud from its own page.".to_string() } else { String::new() };
            }
            Ok((401 | 403, _)) => held.status = "Sign in again to see your drones.".to_string(),
            Ok((status, _)) => held.status = format!("Couldn't list your drones (HTTP {status})."),
            Err(reason) => held.status = format!("Couldn't list your drones: {reason}"),
        }
    });
}

pub fn object() -> Value {
    let pending = PENDING.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let (base, signed) = (api_base(), signed_in());
    if drones_due(signed, &base, crate::hub::wall_ms()) {
        refresh_drones();
    }
    let drones = DRONES.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let current = signed && drones.base == base;
    json!({
        "kind": "object",
        "apiBase": api_base(),
        "signedIn": signed_in(),
        "signingIn": pending.is_some(),
        "userCode": pending.as_ref().map(|p| p.user_code.clone()).unwrap_or_default(),
        "verificationUrl": pending.map(|p| p.verification_url).unwrap_or_default(),
        "status": STATUS.lock().unwrap_or_else(PoisonError::into_inner).clone(),
        "drones": if current { drones.list } else { Vec::new() },
        "dronesStatus": if current { drones.status } else { String::new() },
    })
}

pub fn sign_in(post: &'static Post) {
    let base = api_base();
    if url::Url::parse(&base).ok().and_then(|u| u.host_str().map(str::to_string)).is_none_or(|h| h.is_empty()) {
        set_status("Set up from an Aircast device first \u{2014} it tells QGroundControl which account server to use.");
        return;
    }
    let attempt = ATTEMPT.fetch_add(1, Ordering::SeqCst) + 1;
    *PENDING.lock().unwrap_or_else(PoisonError::into_inner) = None;
    set_status("Asking the Aircast account server for a sign-in code\u{2026}");
    let started = crate::background::spawn("groundstation-sign-in", move || {
        let current = move || ATTEMPT.load(Ordering::SeqCst) == attempt;
        let code = post(&endpoint(&base, "/v1/oauth2/cli/code"), &json!({ "client_id": CLIENT_ID }));
        let Some((device_code, user_code, url, interval)) = code.as_ref().ok().and_then(|c| {
            let text = |key: &str| c.get(key).and_then(Value::as_str).map(str::to_string);
            Some((text("device_code").filter(|d| !d.is_empty())?, text("user_code").unwrap_or_default(), text("verification_uri_complete").unwrap_or_default(), c.get("interval").and_then(Value::as_u64).unwrap_or(DEFAULT_POLL_SECS).max(1)))
        }) else {
            if current() {
                finish(&format!("Couldn't start sign-in: {}", code.err().unwrap_or_default()));
            }
            return;
        };
        if !current() {
            return;
        }
        *PENDING.lock().unwrap_or_else(PoisonError::into_inner) = Some(Pending { user_code: user_code.clone(), verification_url: url });
        set_status(&format!("Approve code {user_code} in the browser to sign in."));
        let body = json!({ "grant_type": DEVICE_CODE_GRANT, "device_code": device_code, "client_id": CLIENT_ID });
        let polling = crate::background::poll("groundstation-sign-in-poll", Duration::from_secs(interval), move || {
            if !current() {
                return true;
            }
            match poll_outcome(&post(&endpoint(&base, "/v1/oauth2/cli/token"), &body)) {
                Poll::Wait => false,
                Poll::Token(token) => {
                    if let Some(key) = crate::cloudlink::token_key(&base) {
                        crate::settingsstore::written(&key, &token);
                    }
                    finish("Signed in.");
                    refresh_drones();
                    true
                }
                Poll::Refused(reason) => {
                    finish(&reason);
                    true
                }
            }
        });
        if polling.is_err() {
            finish(SIGN_IN_UNAVAILABLE);
        }
    });
    if started.is_err() {
        set_status(SIGN_IN_UNAVAILABLE);
    }
}

pub const SIGN_IN_UNAVAILABLE: &str = "Signing in is not available in this app yet.";

pub fn sign_out() {
    if let Some(key) = token_key() {
        crate::settingsstore::forgotten(&key);
    }
    finish("Signed out.");
}

pub fn set_api_base(base: &str) {
    crate::settingsstore::written(API_BASE_KEY, base.trim());
}

pub fn post_json(url: &str, body: &Value) -> Result<Value, String> {
    let (status, answer) = crate::http::post(url, "application/json", body.to_string().as_bytes(), std::time::Duration::from_secs(20))?;
    let parsed: Value = serde_json::from_slice(&answer).unwrap_or(Value::Null);
    match (200..300).contains(&status) || parsed.get("error").is_some() {
        true => Ok(parsed),
        false => Err(format!("HTTP {status}")),
    }
}

pub fn get(path: &str) -> Option<Value> {
    (!crate::qthost::present()).then_some(())?;
    let whole = object();
    match path {
        "account" => Some(whole),
        _ => Some(json!({ "kind": "value", "value": whole.get(path.strip_prefix("account.")?)?.clone() })),
    }
}

pub fn invoke(path: &str) -> Option<Value> {
    (!crate::qthost::present()).then_some(())?;
    match path {
        "account.signIn" => sign_in(&post_json),
        "account.cancelSignIn" => finish(""),
        "account.signOut" => sign_out(),
        "account.refreshDrones" => refresh_drones(),
        _ => return None,
    }
    Some(json!({ "ok": true }))
}

pub fn set(path: &str, value: &str) -> Option<Value> {
    (!crate::qthost::present() && path == "account.apiBase").then_some(())?;
    let given = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").and_then(Value::as_str).map(str::to_string)).unwrap_or_default();
    set_api_base(&given);
    Some(json!({ "ok": true }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_token_poll_reads_as_aircast_account_does() {
        assert_eq!(poll_outcome(&Ok(json!({ "access_token": "t1" }))), Poll::Token("t1".into()));
        assert_eq!(poll_outcome(&Ok(json!({ "error": "authorization_pending" }))), Poll::Wait);
        assert_eq!(poll_outcome(&Ok(json!({ "error": "slow_down" }))), Poll::Wait);
        assert_eq!(poll_outcome(&Err("connection refused".into())), Poll::Wait, "a network error keeps polling");
        assert_eq!(poll_outcome(&Ok(json!({ "error": "expired_token" }))), Poll::Refused("The sign-in code expired \u{2014} start again.".into()));
        assert_eq!(poll_outcome(&Ok(json!({ "error": "access_denied", "error_description": "no" }))), Poll::Refused("Sign-in failed: no".into()));
        assert_eq!(endpoint("https://api.aircast.one//", "/v1/oauth2/cli/code"), "https://api.aircast.one/v1/oauth2/cli/code");
    }

    #[test]
    fn the_account_lists_its_drones_most_recently_seen_first() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-19T12:00:00Z").unwrap().with_timezone(&chrono::Utc);
        let body = json!([
            { "id": "old", "name": "Hangar spare", "last_seen_at": "2026-10-12T12:00:00Z" },
            { "id": "a-1", "name": "Vampire 1", "last_seen_at": "2026-10-19T11:58:30Z" },
            { "id": "b-2", "name": "", "last_seen_at": "2026-10-19T09:00:00Z" },
            { "id": "", "name": "no id" },
            { "id": "c-3", "name": "Never flown" }
        ]);
        assert_eq!(
            drones_from(&body, now),
            vec![
                json!({ "id": "a-1", "name": "Vampire 1", "seen": "seen 1 min ago" }),
                json!({ "id": "b-2", "name": "b-2", "seen": "seen 3 h ago" }),
                json!({ "id": "old", "name": "Hangar spare", "seen": "seen 7 d ago" }),
                json!({ "id": "c-3", "name": "Never flown", "seen": "never seen" }),
            ],
            "a drone with no name is shown by its id, and one with no id cannot be linked to"
        );
        assert_eq!(seen_text(chrono::Duration::seconds(20)), "seen just now");
        assert!(drones_from(&json!({ "error": "x" }), now).is_empty());
    }
}
