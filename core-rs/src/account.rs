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

pub fn object() -> Value {
    let pending = PENDING.lock().unwrap_or_else(PoisonError::into_inner).clone();
    json!({
        "kind": "object",
        "apiBase": api_base(),
        "signedIn": signed_in(),
        "signingIn": pending.is_some(),
        "userCode": pending.as_ref().map(|p| p.user_code.clone()).unwrap_or_default(),
        "verificationUrl": pending.map(|p| p.verification_url).unwrap_or_default(),
        "status": STATUS.lock().unwrap_or_else(PoisonError::into_inner).clone(),
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
    std::thread::spawn(move || {
        let current = || ATTEMPT.load(Ordering::SeqCst) == attempt;
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
        let outcome = std::iter::repeat_with(|| {
            std::thread::sleep(Duration::from_secs(interval));
            current().then(|| poll_outcome(&post(&endpoint(&base, "/v1/oauth2/cli/token"), &body)))
        })
        .find(|polled| polled.as_ref() != Some(&Poll::Wait))
        .flatten();
        match outcome {
            Some(Poll::Token(token)) => {
                if let Some(key) = crate::cloudlink::token_key(&base) {
                    crate::settingsstore::written(&key, &token);
                }
                finish("Signed in.");
            }
            Some(Poll::Refused(reason)) => finish(&reason),
            _ => {}
        }
    });
}

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
    let answer = ureq::post(url).header("Content-Type", "application/json").config().http_status_as_error(false).build().send(body.to_string()).map_err(|e| e.to_string())?;
    let status = answer.status();
    let parsed: Value = serde_json::from_str(&answer.into_body().read_to_string().map_err(|e| e.to_string())?).unwrap_or(Value::Null);
    match status.is_success() || parsed.get("error").is_some() {
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
}
