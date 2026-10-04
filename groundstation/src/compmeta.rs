use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::compression;

pub const MSG_COMPONENT_METADATA: u32 = 397;
pub const MSG_COMPONENT_INFORMATION: u32 = 395;
pub const TYPE_GENERAL: u8 = 0;
pub const TYPE_PARAMETER: u8 = 1;
pub const TYPE_EVENTS: u8 = 4;
pub const TYPE_ACTUATORS: u8 = 5;
pub const FTP_ACK_TIMEOUT_MS: u64 = 1000;
pub const SLOW_AFTER_MS: u64 = 10_000;
pub const SLOW_LIMIT_MS: u64 = 40_000;
pub const CACHE_MAX_FILES: usize = 50;
const CACHE_FOLDER: &str = "QGCCompInfoCache";
const CACHE_EXTENSION: &str = "cache";
const MAVLINK_FTP_SCHEME: &str = "mftp://";
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uris {
    pub uri: String,
    pub crc: Option<u64>,
    pub fallback: Option<Source>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub uri: String,
    pub crc: Option<u64>,
}

pub fn parse_general(text: &str) -> Result<BTreeMap<u8, Uris>, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| format!("general metadata is not JSON: {e}"))?;
    if root.get("version").and_then(Value::as_i64) != Some(1) {
        return Err("general metadata version is not 1".to_string());
    }
    let types = root.get("metadataTypes").and_then(Value::as_array).ok_or("general metadata has no metadataTypes")?;
    Ok(types
        .iter()
        .filter_map(|entry| {
            let kind = u8::try_from(entry.get("type").and_then(Value::as_u64)?).ok()?;
            let uri = entry.get("uri").and_then(Value::as_str).filter(|u| !u.is_empty())?.to_string();
            let crc = entry.get("fileCrc").and_then(Value::as_u64);
            let fallback = entry.get("uriFallback").and_then(Value::as_str).filter(|u| !u.is_empty()).map(|u| Source { uri: u.to_string(), crc: entry.get("fileCrcFallback").and_then(Value::as_u64) });
            crc.map(|_| (kind, Uris { uri, crc, fallback }))
        })
        .collect())
}

pub fn inflate(uri: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
    let lower = uri.to_ascii_lowercase();
    match lower.ends_with(".xz") || lower.ends_with(".lzma") {
        true => compression::inflate_xz(bytes),
        false => Ok(bytes.to_vec()),
    }
}

pub fn over_mavlink_ftp(uri: &str) -> bool {
    uri.get(..MAVLINK_FTP_SCHEME.len()).is_some_and(|scheme| scheme.eq_ignore_ascii_case(MAVLINK_FTP_SCHEME))
}

pub fn download_over_http(uri: &str) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(HTTP_TIMEOUT)).build().into();
    let mut answer = agent.get(uri).call().map_err(|e| format!("Component metadata download failed: {e}"))?;
    answer.body_mut().read_to_vec().map_err(|e| format!("Component metadata download failed: {e}"))
}

pub fn cache_tag(kind: u8, crc: u64) -> String {
    format!("{:08x}_{kind:02}_0", crc as u32)
}

pub fn cache_folder() -> Option<PathBuf> {
    Some(crate::terrainservice::cache_path()?.parent()?.parent()?.join(CACHE_FOLDER))
}

fn cache_file(folder: &Path, tag: &str) -> PathBuf {
    folder.join(tag).with_extension(CACHE_EXTENSION)
}

pub fn cached(folder: &Path, tag: &str) -> Option<Vec<u8>> {
    let path = cache_file(folder, tag);
    let bytes = std::fs::read(&path).ok()?;
    let _ = std::fs::File::options().write(true).open(&path).and_then(|f| f.set_modified(SystemTime::now()));
    Some(bytes)
}

pub fn insert(folder: &Path, tag: &str, bytes: &[u8]) {
    let path = cache_file(folder, tag);
    if path.exists() || std::fs::create_dir_all(folder).and_then(|()| std::fs::write(&path, bytes)).is_err() {
        return;
    }
    let others: Vec<(SystemTime, PathBuf)> = std::fs::read_dir(folder)
        .map(|dir| {
            dir.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| *p != path && p.extension().is_some_and(|x| x == CACHE_EXTENSION))
                .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
                .collect()
        })
        .unwrap_or_default();
    let excess = (others.len() + 1).saturating_sub(CACHE_MAX_FILES);
    others.into_iter().collect::<std::collections::BTreeSet<_>>().into_iter().take(excess).for_each(|(_, p)| {
        let _ = std::fs::remove_file(p);
    });
}

pub fn too_slow(elapsed_ms: u64, progress: f64) -> bool {
    elapsed_ms > SLOW_AFTER_MS && progress < 0.5 && progress > 0.0 && (elapsed_ms as f64 / progress) as u64 > SLOW_LIMIT_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_general_file_lists_the_typed_uris_and_skips_entries_without_a_crc() {
        let types = parse_general(r#"{"version":1,"metadataTypes":[{"type":1,"uri":"mftp://etc/extras/parameters.json.xz","fileCrc":3735928559},{"type":2,"uri":"mftp://x"},{"type":4,"uri":"","fileCrc":1},{"type":257,"uri":"mftp://y","fileCrc":2}]}"#).unwrap();
        assert_eq!(types.len(), 1);
        assert_eq!(types[&TYPE_PARAMETER], Uris { uri: "mftp://etc/extras/parameters.json.xz".into(), crc: Some(3735928559), fallback: None });
        let fallen = parse_general(r#"{"version":1,"metadataTypes":[{"type":4,"uri":"mftp://etc/extras/events.json.xz","fileCrc":1,"uriFallback":"https://px4.io/events.json.xz","fileCrcFallback":2},{"type":5,"uri":"mftp://a","fileCrc":3,"uriFallback":""}]}"#).unwrap();
        assert_eq!(fallen[&TYPE_EVENTS].fallback, Some(Source { uri: "https://px4.io/events.json.xz".into(), crc: Some(2) }), "CompInfoGeneral reads uriFallback and fileCrcFallback beside the primary");
        assert_eq!(fallen[&TYPE_ACTUATORS].fallback, None);
        assert!(over_mavlink_ftp("MFTP://etc/x") && over_mavlink_ftp("mftp://[;comp=100]x") && !over_mavlink_ftp("https://x") && !over_mavlink_ftp("mft"));
        assert_eq!(cache_tag(TYPE_PARAMETER, 0xdeadbeef), "deadbeef_01_0", "ComponentInformationManager::_getFileCacheTag");
        assert!(parse_general("{}").is_err());
        assert!(parse_general(r#"{"version":2,"metadataTypes":[]}"#).is_err());
        assert_eq!(inflate("a.json", b"{}").unwrap(), b"{}");
        assert!(inflate("a.json.xz", b"nope").is_err());
        assert!(!too_slow(9_000, 0.1));
        assert!(too_slow(11_000, 0.1));
        assert!(!too_slow(11_000, 0.6));
    }

    #[test]
    fn the_cache_keeps_the_fifty_most_recently_used_files_like_component_information_cache() {
        let folder = std::env::temp_dir().join(format!("compinfo-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(cached(&folder, "missing"), None);
        let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        (0..CACHE_MAX_FILES).for_each(|i| {
            insert(&folder, &format!("t{i:02}"), format!("{i}").as_bytes());
            let _ = std::fs::File::options().write(true).open(cache_file(&folder, &format!("t{i:02}"))).and_then(|f| f.set_modified(old + Duration::from_secs(i as u64)));
        });
        assert_eq!(cached(&folder, "t00").as_deref(), Some(b"0".as_slice()), "an access marks the entry as recently used");
        insert(&folder, "t00", b"other");
        assert_eq!(cached(&folder, "t00").as_deref(), Some(b"0".as_slice()), "an existing entry is not replaced");
        insert(&folder, "new", b"n");
        assert_eq!(cached(&folder, "new").as_deref(), Some(b"n".as_slice()));
        assert_eq!(cached(&folder, "t01"), None, "the least recently used entry went to keep fifty");
        assert!(cached(&folder, "t00").is_some() && cached(&folder, "t02").is_some());
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), CACHE_MAX_FILES);
        let _ = std::fs::remove_dir_all(&folder);
    }
}
