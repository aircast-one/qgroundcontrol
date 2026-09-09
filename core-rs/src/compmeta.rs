use serde_json::Value;
use std::collections::BTreeMap;

use crate::compression;

pub const MSG_COMPONENT_METADATA: u32 = 397;
pub const TYPE_GENERAL: u8 = 0;
pub const TYPE_PARAMETER: u8 = 1;
pub const FTP_ACK_TIMEOUT_MS: u64 = 1000;
pub const SLOW_AFTER_MS: u64 = 10_000;
pub const SLOW_LIMIT_MS: u64 = 40_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uris {
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
            crc.map(|_| (kind, Uris { uri, crc }))
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
        assert_eq!(types[&TYPE_PARAMETER], Uris { uri: "mftp://etc/extras/parameters.json.xz".into(), crc: Some(3735928559) });
        assert!(parse_general("{}").is_err());
        assert!(parse_general(r#"{"version":2,"metadataTypes":[]}"#).is_err());
        assert_eq!(inflate("a.json", b"{}").unwrap(), b"{}");
        assert!(inflate("a.json.xz", b"nope").is_err());
        assert!(!too_slow(9_000, 0.1));
        assert!(too_slow(11_000, 0.1));
        assert!(!too_slow(11_000, 0.6));
    }
}
