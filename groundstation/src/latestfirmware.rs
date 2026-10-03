use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const PX4_RELEASES: &str = "https://api.github.com/repos/PX4/Firmware/releases?per_page=10";
const APM_VERSION: &str = "http://firmware.ardupilot.org/{}/stable/Pixhawk1/git-version.txt";
const VERSION_FETCH_TIMEOUT: Duration = Duration::from_secs(20);
const VERSION_RETRY_AFTER: Duration = Duration::from_secs(60);

enum Fetch {
    Pending,
    Done(String),
    Failed(Instant),
}

static FETCHED: Mutex<BTreeMap<String, Fetch>> = Mutex::new(BTreeMap::new());

pub fn version_url(px4: bool, apm_vehicle: Option<&str>) -> Option<String> {
    match (px4, apm_vehicle) {
        (true, _) => Some(PX4_RELEASES.to_string()),
        (false, Some(vehicle)) => Some(APM_VERSION.replace("{}", vehicle)),
        (false, None) => None,
    }
}

pub fn parse(px4: bool, contents: &str) -> Option<String> {
    let pattern = if px4 { r"v([0-9.]+)(?: -)? Stable" } else { r"(?m) V([0-9,\.]*)$" };
    regex::Regex::new(pattern).ok()?.captures(contents)?.get(1).map(|m| m.as_str().to_string())
}

fn numbers(version: &str) -> Vec<u64> {
    version.split(['.', ',']).map(|part| part.trim().parse().unwrap_or(0)).collect()
}

pub fn older(running: &str, latest: &str) -> bool {
    let (a, b) = (numbers(running), numbers(latest));
    (0..3).map(|i| (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0))).find(|(x, y)| x != y).is_some_and(|(x, y)| x < y)
}

pub fn update_text(running: &str, latest: &str) -> String {
    format!("Update available: this vehicle is running {running}, latest stable is {latest}.")
}

fn fetches() -> bool {
    !cfg!(test) && std::env::var_os("QGC_CORE_OFFLINE").is_none()
}

fn fetched() -> std::sync::MutexGuard<'static, BTreeMap<String, Fetch>> {
    FETCHED.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn text(url: &str) -> Option<String> {
    let mut held = fetched();
    let due = match held.get(url) {
        Some(Fetch::Done(body)) => return Some(body.clone()),
        Some(Fetch::Pending) => false,
        Some(Fetch::Failed(at)) => at.elapsed() >= VERSION_RETRY_AFTER,
        None => true,
    };
    if due && fetches() {
        held.insert(url.to_string(), Fetch::Pending);
        let url = url.to_string();
        let _ = std::thread::Builder::new().name("firmware-versions".into()).spawn(move || {
            let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(VERSION_FETCH_TIMEOUT)).build().into();
            let body = agent.get(&url).call().ok().and_then(|mut answer| answer.body_mut().read_to_string().ok());
            fetched().insert(url, body.map_or_else(|| Fetch::Failed(Instant::now()), Fetch::Done));
        });
    }
    None
}

pub fn latest(url: &str, px4: bool) -> Option<String> {
    text(url).and_then(|body| parse(px4, &body))
}

pub fn px4_release_names(releases: &str) -> (Option<String>, Option<String>) {
    let listed: Vec<serde_json::Value> = serde_json::from_str(releases).unwrap_or_default();
    let first = |prerelease: bool| listed.iter().find(|r| r["prerelease"].as_bool().unwrap_or(false) == prerelease).and_then(|r| r["name"].as_str()).map(str::to_string);
    (first(false), first(true))
}

pub fn px4_releases() -> (Option<String>, Option<String>) {
    text(PX4_RELEASES).map(|body| px4_release_names(&body)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_read_and_compared_like_firmware_plugin() {
        assert_eq!(parse(true, r#"[{"name":"v1.17.0 - Stable Release"},{"name":"v1.14.4 Stable Release"}]"#).as_deref(), Some("1.17.0"), "PX4 names releases \"v1.17.0 - Stable Release\" since 1.15; QGC's pattern skips those");
        assert_eq!(parse(true, r#"[{"name":"v1.14.4 Stable Release"}]"#).as_deref(), Some("1.14.4"));
        assert_eq!(parse(false, "APMVERSION: ArduCopter V4.5.7\nother").as_deref(), Some("4.5.7"), "APMFirmwarePlugin::_versionRegex");
        assert!(older("4.5.6", "4.5.7") && older("1.9.9", "1.15.0") && !older("4.5.7", "4.5.7") && !older("4.6.0", "4.5.7"));
        assert_eq!(version_url(false, Some("Copter")).as_deref(), Some("http://firmware.ardupilot.org/Copter/stable/Pixhawk1/git-version.txt"));
        assert_eq!(version_url(false, None), None);
        assert_eq!(
            px4_release_names(r#"[{"name":"v1.16.0-beta2","prerelease":true},{"name":"v1.15.4","prerelease":false},{"name":"v1.15.3","prerelease":false}]"#),
            (Some("v1.15.4".to_string()), Some("v1.16.0-beta2".to_string())),
            "FirmwareUpgradeController: the first non-prerelease is stable, the first prerelease is beta"
        );
        assert_eq!(update_text("4.5.6", "4.5.7"), "Update available: this vehicle is running 4.5.6, latest stable is 4.5.7.");
    }
}
