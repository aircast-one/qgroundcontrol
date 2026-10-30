use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};
use web_time::Instant;

const RELEASES_URL: &str =
    "https://api.github.com/repos/aircast-one/qgroundcontrol/releases?per_page=20";
const TAG_PREFIX: &str = "aircast-v";
const NATIVE_PREFIX: &str = "AircastQGCNative-";
const ANDROID_PREFIX: &str = "AircastAndroid-";
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);
pub const FIRST_CHECK_AFTER: Duration = Duration::from_secs(15);
pub const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
pub const RETRY_AFTER_FAILURE: Duration = Duration::from_secs(60 * 60);
const TICK: Duration = Duration::from_millis(250);
const AUTO_CHECK_KEY: &str = "AppUpdate/autoCheck";
const SNOOZED_KEY: &str = "AppUpdate/snoozed";
const DOWNLOAD_DIR: &str = "Updates";
const CHUNK: usize = 64 * 1024;
const CANCELLED: &str = "cancelled";
pub const CHANGED: &str = "core.appUpdate@changed";
pub const DEPS: &[&str] = &[CHANGED, "vehicles.activeVehicleAvailable"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Ready,
    Failed,
}

impl Phase {
    fn token(self) -> &'static str {
        match self {
            Phase::Idle => "idle",
            Phase::Checking => "checking",
            Phase::UpToDate => "upToDate",
            Phase::Available => "available",
            Phase::Downloading => "downloading",
            Phase::Ready => "ready",
            Phase::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub checksum_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: semver::Version,
    pub notes: String,
    pub url: String,
    pub asset: Option<Asset>,
}

#[derive(Default)]
struct State {
    current: String,
    started: Option<Instant>,
    phase: Phase,
    latest: Option<Release>,
    progress: f64,
    file: String,
    error: String,
    checked_at: String,
    last_check: Option<(Instant, bool)>,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));
static CANCEL: AtomicBool = AtomicBool::new(false);
static DIRTY: AtomicBool = AtomicBool::new(false);
static POLLING: std::sync::Once = std::sync::Once::new();

fn state() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

struct Changing(MutexGuard<'static, State>);

impl std::ops::Deref for Changing {
    type Target = State;
    fn deref(&self) -> &State {
        &self.0
    }
}

impl std::ops::DerefMut for Changing {
    fn deref_mut(&mut self) -> &mut State {
        &mut self.0
    }
}

impl Drop for Changing {
    fn drop(&mut self) {
        DIRTY.store(true, Ordering::SeqCst);
    }
}

fn changing() -> Changing {
    Changing(state())
}

fn take_changed() -> bool {
    DIRTY.swap(false, Ordering::SeqCst)
}

fn vehicle_connected() -> bool {
    !crate::hub::lock().vehicle_ids().is_empty()
}

pub fn parsed(version: &str) -> Option<semver::Version> {
    semver::Version::parse(version.trim().trim_start_matches('v')).ok()
}

pub fn channel(current: &semver::Version) -> &'static str {
    match current.pre.is_empty() {
        true => "stable",
        false => "beta",
    }
}

pub fn asset_pattern(os: &str, arch: &str) -> Option<(&'static str, &'static str)> {
    match (os, arch) {
        ("windows", "x86_64") => Some((NATIVE_PREFIX, "-windows-x64-setup.exe")),
        ("windows", "aarch64") => Some((NATIVE_PREFIX, "-windows-arm64-setup.exe")),
        ("android", _) => Some((ANDROID_PREFIX, ".apk")),
        ("macos", _) => Some((NATIVE_PREFIX, ".dmg")),
        _ => None,
    }
}

fn host_pattern() -> Option<(&'static str, &'static str)> {
    asset_pattern(std::env::consts::OS, std::env::consts::ARCH)
        .filter(|_| !crate::qthost::present() || cfg!(target_os = "android"))
}

fn asset_of(assets: &[Value], (prefix, suffix): (&str, &str)) -> Option<Asset> {
    let found = assets.iter().find(|a| {
        a["name"]
            .as_str()
            .is_some_and(|n| n.starts_with(prefix) && n.ends_with(suffix))
    })?;
    let name = found["name"].as_str()?;
    let checksum = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(&format!("{name}.sha256")))?;
    Some(Asset {
        name: name.to_string(),
        url: found["browser_download_url"].as_str()?.to_string(),
        size: found["size"].as_u64().unwrap_or(0),
        checksum_url: checksum["browser_download_url"].as_str()?.to_string(),
    })
}

fn release_of(entry: &Value, stable_only: bool, pattern: Option<(&str, &str)>) -> Option<Release> {
    let drafted = entry["draft"].as_bool().unwrap_or(false);
    let prerelease = entry["prerelease"].as_bool().unwrap_or(false);
    (!drafted && (!stable_only || !prerelease)).then_some(())?;
    let version =
        semver::Version::parse(entry["tag_name"].as_str()?.strip_prefix(TAG_PREFIX)?).ok()?;
    let assets = entry["assets"].as_array().cloned().unwrap_or_default();
    Some(Release {
        version,
        notes: entry["body"].as_str().unwrap_or_default().to_string(),
        url: entry["html_url"].as_str().unwrap_or_default().to_string(),
        asset: pattern.and_then(|pattern| asset_of(&assets, pattern)),
    })
}

pub fn newest(
    releases: &str,
    current: &semver::Version,
    pattern: Option<(&str, &str)>,
) -> Result<Option<Release>, String> {
    let listed: Vec<Value> = serde_json::from_str(releases)
        .map_err(|e| format!("GitHub did not answer with a release list: {e}"))?;
    Ok(listed
        .iter()
        .filter_map(|entry| release_of(entry, current.pre.is_empty(), pattern))
        .filter(|release| release.version > *current)
        .max_by(|a, b| a.version.cmp(&b.version)))
}

pub fn expected_digest(checksum_file: &str) -> Option<String> {
    checksum_file
        .split_whitespace()
        .next()
        .filter(|hex| hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()))
        .map(str::to_ascii_lowercase)
}

pub fn digest_matches(expected: &str, digest: &[u8]) -> bool {
    crate::signingkeys::to_hex(digest) == expected.to_ascii_lowercase()
}

pub fn effective_snooze(stored: Option<&str>, latest: Option<&semver::Version>) -> String {
    let snoozed = stored.and_then(parsed);
    match (snoozed, latest) {
        (Some(snoozed), Some(latest)) if *latest > snoozed => String::new(),
        (Some(snoozed), _) => snoozed.to_string(),
        (None, _) => String::new(),
    }
}

pub fn due(now: Instant, started: Instant, last_check: Option<(Instant, bool)>) -> bool {
    match last_check {
        None => now.duration_since(started) >= FIRST_CHECK_AFTER,
        Some((at, failed)) => {
            now.duration_since(at)
                >= match failed {
                    true => RETRY_AFTER_FAILURE,
                    false => CHECK_EVERY,
                }
        }
    }
}

fn auto_check() -> bool {
    crate::settingsstore::stored_text(AUTO_CHECK_KEY).is_none_or(|text| text != "false")
}

fn now_text() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn download_dir() -> Option<PathBuf> {
    crate::settingsstore::save_root().map(|root| PathBuf::from(root).join(DOWNLOAD_DIR))
}

fn fetches() -> bool {
    !cfg!(test) && std::env::var_os("QGC_CORE_OFFLINE").is_none()
}

fn settle(found: Result<Option<Release>, String>) {
    let mut held = changing();
    held.checked_at = now_text();
    held.last_check = Some((Instant::now(), found.is_err()));
    match found {
        Err(error) => {
            held.phase = Phase::Failed;
            held.error = error;
        }
        Ok(None) => {
            held.phase = Phase::UpToDate;
            held.latest = None;
        }
        Ok(Some(release)) => {
            held.phase = Phase::Available;
            held.latest = Some(release);
        }
    }
}

pub fn check() -> Result<(), String> {
    let mut held = changing();
    if matches!(held.phase, Phase::Checking | Phase::Downloading) {
        return Err("A check or download is already running.".to_string());
    }
    let current = parsed(&held.current).ok_or_else(|| {
        format!(
            "{} is not a release version, so there is nothing to compare against.",
            match held.current.is_empty() {
                true => "This build",
                false => held.current.as_str(),
            }
        )
    })?;
    held.error.clear();
    held.file.clear();
    held.progress = 0.0;
    let Some(pattern) = host_pattern() else {
        held.phase = Phase::UpToDate;
        held.checked_at = now_text();
        return Ok(());
    };
    held.phase = Phase::Checking;
    drop(held);
    if !fetches() {
        settle(Ok(None));
        return Ok(());
    }
    fetch_releases(current, pattern)
}

#[cfg(not(target_family = "wasm"))]
fn fetch_releases(
    current: semver::Version,
    pattern: (&'static str, &'static str),
) -> Result<(), String> {
    crate::background::spawn("app-update-check", move || {
        settle(
            crate::http::get_text(RELEASES_URL, FETCH_TIMEOUT)
                .map_err(|e| format!("GitHub could not be reached to check for updates ({e})."))
                .and_then(|body| newest(&body, &current, Some(pattern))),
        );
    })
    .map_err(|e| e.to_string())
}

#[cfg(target_family = "wasm")]
fn fetch_releases(_current: semver::Version, _pattern: (&str, &str)) -> Result<(), String> {
    settle(Ok(None));
    Ok(())
}

pub fn download() -> Result<(), String> {
    let mut held = changing();
    if matches!(held.phase, Phase::Checking | Phase::Downloading) {
        return Err("A check or download is already running.".to_string());
    }
    let asset = held
        .latest
        .as_ref()
        .and_then(|release| release.asset.clone())
        .ok_or_else(|| {
            "No installer is published for this computer; open the release page instead."
                .to_string()
        })?;
    let dir =
        download_dir().ok_or_else(|| "No save folder is set up to download into.".to_string())?;
    held.phase = Phase::Downloading;
    held.progress = 0.0;
    held.file.clear();
    held.error.clear();
    drop(held);
    CANCEL.store(false, Ordering::SeqCst);
    fetch_asset(asset, dir)
}

fn finished(outcome: Result<PathBuf, String>) {
    let mut held = changing();
    match outcome {
        Ok(file) => {
            held.phase = Phase::Ready;
            held.progress = 1.0;
            held.file = file.to_string_lossy().into_owned();
        }
        Err(reason) if reason == CANCELLED => {
            held.phase = Phase::Available;
            held.progress = 0.0;
        }
        Err(reason) => {
            held.phase = Phase::Failed;
            held.error = reason;
        }
    }
}

#[cfg(not(target_family = "wasm"))]
fn fetch_asset(asset: Asset, dir: PathBuf) -> Result<(), String> {
    crate::background::spawn("app-update-download", move || {
        finished(download_verified(&asset, &dir));
    })
    .map_err(|e| e.to_string())
}

#[cfg(target_family = "wasm")]
fn fetch_asset(_asset: Asset, _dir: PathBuf) -> Result<(), String> {
    finished(Err(
        "The browser build updates itself when the page reloads.".to_string(),
    ));
    Ok(())
}

#[cfg(not(target_family = "wasm"))]
fn download_verified(asset: &Asset, dir: &std::path::Path) -> Result<PathBuf, String> {
    use sha2::{Digest, Sha256};
    use std::io::{Read, Write};

    let expected = crate::http::get_text(&asset.checksum_url, FETCH_TIMEOUT)
        .ok()
        .as_deref()
        .and_then(expected_digest)
        .ok_or_else(|| format!("{}.sha256 could not be read.", asset.name))?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let partial = dir.join(format!("{}.part", asset.name));
    let target = dir.join(&asset.name);
    let streamed = (|| {
        let interrupted = |e: String| format!("The download was interrupted ({e}). Try again.");
        let (length, mut body) =
            crate::http::stream(&asset.url, DOWNLOAD_TIMEOUT).map_err(interrupted)?;
        let total = length.unwrap_or(asset.size).max(1) as f64;
        let mut out = std::fs::File::create(&partial).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut buffer = vec![0u8; CHUNK];
        let mut written = 0u64;
        loop {
            if CANCEL.load(Ordering::SeqCst) {
                return Err(CANCELLED.to_string());
            }
            let n = body
                .read(&mut buffer)
                .map_err(|e| interrupted(e.to_string()))?;
            if n == 0 {
                break;
            }
            out.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
            hash.update(&buffer[..n]);
            written += n as u64;
            changing().progress = (written as f64 / total).min(0.99);
        }
        out.flush().map_err(|e| e.to_string())?;
        digest_matches(&expected, &hash.finalize())
            .then_some(())
            .ok_or_else(|| {
                format!(
                    "{} did not match its published checksum, so it was discarded.",
                    asset.name
                )
            })?;
        std::fs::rename(&partial, &target).map_err(|e| e.to_string())?;
        Ok(target)
    })();
    if streamed.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    streamed
}

pub fn start(version: &str, announce: fn()) {
    {
        let mut held = changing();
        held.current = version.trim().to_string();
        held.started.get_or_insert_with(Instant::now);
    }
    POLLING.call_once(|| {
        if let Some(dir) = download_dir() {
            let _ = std::fs::remove_dir_all(dir);
        }
        let _ = crate::background::poll("app-update", TICK, move || {
            if take_changed() {
                announce();
            }
            tick();
            false
        });
    });
}

fn tick() {
    let held = state();
    let idle = matches!(
        held.phase,
        Phase::Idle | Phase::UpToDate | Phase::Available | Phase::Failed
    );
    let wanted = idle
        && parsed(&held.current).is_some()
        && held
            .started
            .is_some_and(|started| due(Instant::now(), started, held.last_check));
    drop(held);
    if wanted && auto_check() && !vehicle_connected() {
        let _ = check();
    }
}

pub fn view(_backend: &dyn crate::router::Backend, _args: &[String]) -> Value {
    let can_install = !vehicle_connected();
    let held = state();
    let latest = held.latest.as_ref();
    let asset = latest.and_then(|release| release.asset.as_ref());
    json!({
        "kind": "object",
        "class": "AppUpdate",
        "current": held.current,
        "channel": parsed(&held.current).as_ref().map_or("stable", channel),
        "phase": held.phase.token(),
        "latest": latest.map(|r| r.version.to_string()).unwrap_or_default(),
        "notes": latest.map(|r| r.notes.clone()).unwrap_or_default(),
        "releaseUrl": latest.map(|r| r.url.clone()).unwrap_or_default(),
        "assetName": asset.map(|a| a.name.clone()).unwrap_or_default(),
        "assetUrl": asset.map(|a| a.url.clone()).unwrap_or_default(),
        "sizeBytes": asset.map_or(0, |a| a.size),
        "progress": held.progress,
        "file": held.file,
        "error": held.error,
        "checkedAt": held.checked_at,
        "autoCheck": auto_check(),
        "snoozed": effective_snooze(
            crate::settingsstore::stored_text(SNOOZED_KEY).as_deref(),
            latest.map(|r| &r.version),
        ),
        "canInstall": can_install,
    })
}

fn answer(outcome: Result<(), String>) -> Value {
    match outcome {
        Ok(()) => json!({ "ok": true }),
        Err(reason) => json!({ "ok": false, "reason": reason }),
    }
}

pub fn invoke(path: &str, args: &str) -> Option<Value> {
    let given: Vec<Value> = serde_json::from_str(args).unwrap_or_default();
    Some(match path {
        "appUpdate.check" => answer(check()),
        "appUpdate.download" => answer(download()),
        "appUpdate.cancel" => {
            CANCEL.store(true, Ordering::SeqCst);
            answer(Ok(()))
        }
        "appUpdate.setAutoCheck" => answer(match given.first().and_then(Value::as_bool) {
            Some(on) => {
                crate::settingsstore::written(AUTO_CHECK_KEY, if on { "true" } else { "false" });
                drop(changing());
                Ok(())
            }
            None => Err("appUpdate.setAutoCheck takes true or false".to_string()),
        }),
        "appUpdate.snooze" => answer(match changing().latest.as_ref() {
            Some(release) => {
                crate::settingsstore::written(SNOOZED_KEY, &release.version.to_string());
                Ok(())
            }
            None => Err("There is no update to snooze.".to_string()),
        }),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, draft: bool, prerelease: bool, assets: &[&str]) -> Value {
        json!({
            "tag_name": tag,
            "draft": draft,
            "prerelease": prerelease,
            "html_url": format!("https://github.com/aircast-one/qgroundcontrol/releases/tag/{tag}"),
            "body": format!("notes for {tag}"),
            "assets": assets.iter().map(|name| json!({
                "name": name,
                "browser_download_url": format!("https://dl/{name}"),
                "size": 1234,
            })).collect::<Vec<_>>(),
        })
    }

    fn listing() -> String {
        Value::Array(vec![
            release(
                "aircast-v5.6.5-beta.22",
                false,
                true,
                &[
                    "AircastQGCNative-v5.6.5-beta.22-windows-arm64-setup.exe",
                    "AircastQGCNative-v5.6.5-beta.22-windows-arm64-setup.exe.sha256",
                    "AircastQGCNative-v5.6.5-beta.22-windows-x64-setup.exe",
                    "AircastQGCNative-v5.6.5-beta.22.dmg",
                    "AircastQGCNative-v5.6.5-beta.22.dmg.sha256",
                    "AircastAndroid-v5.6.5-beta.22.apk",
                    "AircastAndroid-v5.6.5-beta.22.apk.sha256",
                ],
            ),
            release("aircast-v5.6.5-beta.23", true, true, &[]),
            release(
                "aircast-v5.6.4",
                false,
                false,
                &[
                    "AircastQGCNative-v5.6.4.dmg",
                    "AircastQGCNative-v5.6.4.dmg.sha256",
                ],
            ),
            release("aircast-v5.6.5-beta.21", false, true, &[]),
            release("aircast-v5.5.9", false, false, &[]),
            release("v9.9.9", false, false, &[]),
        ])
        .to_string()
    }

    #[test]
    fn a_prerelease_build_is_on_the_beta_channel_and_a_release_build_on_stable() {
        assert_eq!(channel(&parsed("5.6.5-beta.21").unwrap()), "beta");
        assert_eq!(channel(&parsed("v5.6.4").unwrap()), "stable");
        assert_eq!(parsed("dev"), None);
    }

    #[test]
    fn the_newest_eligible_release_above_the_running_version_wins() {
        let beta = parsed("5.6.5-beta.21").unwrap();
        let found = newest(&listing(), &beta, None).unwrap().unwrap();
        assert_eq!(
            found.version.to_string(),
            "5.6.5-beta.22",
            "the draft beta.23 and the foreign v9.9.9 tag are ignored"
        );
        assert_eq!(
            found.url,
            "https://github.com/aircast-one/qgroundcontrol/releases/tag/aircast-v5.6.5-beta.22"
        );
        assert_eq!(found.notes, "notes for aircast-v5.6.5-beta.22");
        let stable = parsed("5.6.3").unwrap();
        assert_eq!(
            newest(&listing(), &stable, None)
                .unwrap()
                .unwrap()
                .version
                .to_string(),
            "5.6.4",
            "a stable build never sees prereleases"
        );
        assert_eq!(
            newest(&listing(), &parsed("5.6.4").unwrap(), None).unwrap(),
            None,
            "the same version is not an update"
        );
        assert_eq!(
            newest(&listing(), &parsed("5.6.5-beta.22").unwrap(), None).unwrap(),
            None
        );
        assert!(newest("<html>", &beta, None).is_err());
    }

    #[test]
    fn the_asset_is_matched_per_host_and_needs_its_checksum_sibling() {
        let beta = parsed("5.6.5-beta.21").unwrap();
        let on = |os: &str, arch: &str| {
            newest(&listing(), &beta, asset_pattern(os, arch))
                .unwrap()
                .unwrap()
                .asset
        };
        assert_eq!(
            on("windows", "aarch64").unwrap().name,
            "AircastQGCNative-v5.6.5-beta.22-windows-arm64-setup.exe"
        );
        assert_eq!(
            on("windows", "aarch64").unwrap().checksum_url,
            "https://dl/AircastQGCNative-v5.6.5-beta.22-windows-arm64-setup.exe.sha256"
        );
        assert_eq!(
            on("windows", "x86_64"),
            None,
            "the x64 installer has no .sha256 beside it, so it is not offered"
        );
        assert_eq!(
            on("macos", "aarch64").unwrap().name,
            "AircastQGCNative-v5.6.5-beta.22.dmg"
        );
        assert_eq!(
            on("android", "aarch64").unwrap().name,
            "AircastAndroid-v5.6.5-beta.22.apk"
        );
        assert_eq!(on("android", "aarch64").unwrap().size, 1234);
        assert_eq!(asset_pattern("linux", "x86_64"), None);
        assert_eq!(asset_pattern("ios", "aarch64"), None);
        assert_eq!(asset_pattern("unknown", "wasm32"), None);
    }

    #[test]
    fn the_checksum_file_is_parsed_and_compared_case_insensitively() {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(b"installer bytes");
        let hex = crate::signingkeys::to_hex(&digest);
        let file = format!(
            "{}  AircastQGCNative-v5.6.5-beta.22.dmg\n",
            hex.to_uppercase()
        );
        let expected = expected_digest(&file).unwrap();
        assert!(digest_matches(&expected, &digest));
        assert!(!digest_matches(&expected, &Sha256::digest(b"tampered")));
        assert_eq!(expected_digest("not a digest"), None);
        assert_eq!(expected_digest(""), None);
    }

    #[test]
    fn a_snooze_hides_that_version_until_a_newer_one_appears() {
        let v22 = parsed("5.6.5-beta.22").unwrap();
        let v23 = parsed("5.6.5-beta.23").unwrap();
        assert_eq!(
            effective_snooze(Some("5.6.5-beta.22"), Some(&v22)),
            "5.6.5-beta.22"
        );
        assert_eq!(effective_snooze(Some("5.6.5-beta.22"), Some(&v23)), "");
        assert_eq!(
            effective_snooze(Some("5.6.5-beta.22"), None),
            "5.6.5-beta.22"
        );
        assert_eq!(effective_snooze(None, Some(&v22)), "");
        assert_eq!(effective_snooze(Some("garbage"), Some(&v22)), "");
    }

    #[test]
    fn the_first_check_waits_after_start_then_every_six_hours_and_hourly_after_a_failure() {
        let started = Instant::now();
        assert!(!due(started + Duration::from_secs(5), started, None));
        assert!(due(started + FIRST_CHECK_AFTER, started, None));
        let checked = started + FIRST_CHECK_AFTER;
        assert!(!due(
            checked + Duration::from_secs(3600),
            started,
            Some((checked, false))
        ));
        assert!(due(checked + CHECK_EVERY, started, Some((checked, false))));
        assert!(!due(
            checked + Duration::from_secs(600),
            started,
            Some((checked, true))
        ));
        assert!(due(
            checked + RETRY_AFTER_FAILURE,
            started,
            Some((checked, true))
        ));
    }
}
