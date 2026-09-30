use std::io::{Cursor, Read};

pub const INFLATE_LIMIT: u64 = 256 * 1024 * 1024;

fn bounded(mut reader: impl Read, codec: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    reader.by_ref().take(INFLATE_LIMIT + 1).read_to_end(&mut out).map_err(|e| format!("{codec}: {e}"))?;
    (out.len() as u64 <= INFLATE_LIMIT).then_some(out).ok_or_else(|| format!("{codec}: inflated payload exceeds {INFLATE_LIMIT} bytes"))
}

struct LimitedSink<'a> {
    out: &'a mut Vec<u8>,
}

impl std::io::Write for LimitedSink<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if (self.out.len() + buf.len()) as u64 > INFLATE_LIMIT {
            return Err(std::io::Error::other(format!("inflated payload exceeds {INFLATE_LIMIT} bytes")));
        }
        self.out.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn inflate_gzip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    bounded(flate2::read::MultiGzDecoder::new(bytes), "gzip")
}

pub fn inflate_xz(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut sink = LimitedSink { out: &mut out };
    lzma_rs::xz_decompress(&mut Cursor::new(bytes), &mut sink).map_err(|e| format!("xz: {e:?}"))?;
    Ok(out)
}

pub fn unzip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("zip: {e}"))?;
    (0..archive.len())
        .map(|i| {
            let entry = archive.by_index(i).map_err(|e| format!("zip entry {i}: {e}"))?;
            let name = entry.name().to_string();
            Ok((name, bounded(entry, "zip")?))
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
