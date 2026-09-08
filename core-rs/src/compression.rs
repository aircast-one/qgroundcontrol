use std::io::{Cursor, Read};

pub fn inflate_gzip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    flate2::read::MultiGzDecoder::new(bytes).read_to_end(&mut out).map_err(|e| format!("gzip: {e}"))?;
    Ok(out)
}

pub fn inflate_xz(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    lzma_rs::xz_decompress(&mut Cursor::new(bytes), &mut out).map_err(|e| format!("xz: {e:?}"))?;
    Ok(out)
}

pub fn unzip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("zip: {e}"))?;
    (0..archive.len())
        .map(|i| {
            let mut entry = archive.by_index(i).map_err(|e| format!("zip entry {i}: {e}"))?;
            let mut content = Vec::new();
            entry.read_to_end(&mut content).map_err(|e| format!("zip entry {i}: {e}"))?;
            Ok((entry.name().to_string(), content))
        })
        .filter(|entry| !matches!(entry, Ok((name, _)) if name.ends_with('/')))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(format!("{}/../test/Utilities/Compression/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    #[test]
    fn the_three_manifests_inflate_to_the_same_json() {
        let gz = inflate_gzip(&fixture("manifest.json.gz")).unwrap();
        let xz = inflate_xz(&fixture("manifest.json.xz")).unwrap();
        let zipped = unzip(&fixture("manifest.json.zip")).unwrap();
        assert!(serde_json::from_slice::<serde_json::Value>(&gz).is_ok());
        assert_eq!(gz, xz);
        assert_eq!(zipped.len(), 1);
        assert!(zipped[0].0.ends_with("manifest.json"));
        assert_eq!(zipped[0].1, gz);
    }

    #[test]
    fn garbage_is_refused_with_the_codec_named() {
        assert!(inflate_gzip(b"not gzip").unwrap_err().starts_with("gzip:"));
        assert!(inflate_xz(b"not xz").unwrap_err().starts_with("xz:"));
        assert!(unzip(b"not a zip").unwrap_err().starts_with("zip:"));
    }
}
