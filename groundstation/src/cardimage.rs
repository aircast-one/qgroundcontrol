use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MIN_CARD_BYTES: u64 = 3_500_000_000;
pub const CANCELLED_BY_USER: &str = "Cancelled";
pub const RELEASE_CHANNELS: [(&str, &str); 3] = [("stable", "downloads.aircast.one"), ("staging", "downloads.stage.aircast.one"), ("development", "downloads-dev.aircast.one")];
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const ATTEMPT_LIMIT: Duration = Duration::from_secs(60);
const ATTEMPTS: u32 = 30;
const BACKOFF_CAP_SECS: u64 = 30;
const IMAGE_CHUNK: usize = 256 * 1024;
const PIPE_DEPTH: usize = 16;
const XZ_FOOTER_LEN: u64 = 12;
const XZ_HEADER_LEN: u64 = 12;
const XZ_PADDING_SCAN: u64 = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Image {
    pub filename: String,
    pub extension: String,
    pub size: u64,
    #[serde(default)]
    pub uncompressed_size: Option<u64>,
    pub download_url: String,
    pub checksum_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    pub version: String,
    pub prerelease: bool,
    pub created_at: String,
    pub image: Image,
}

#[derive(Deserialize)]
struct Listing {
    releases: Vec<Release>,
}

pub fn releases_url(channel: &str) -> Option<String> {
    RELEASE_CHANNELS.iter().find(|(name, _)| *name == channel).map(|(_, host)| format!("https://{host}/lite/releases.json"))
}

pub fn parse_releases(json: &str) -> Result<Vec<Release>, String> {
    serde_json::from_str::<Listing>(json).map(|listing| listing.releases).map_err(|e| format!("The release list did not parse: {e}"))
}

pub fn parse_sha256(text: &str) -> Result<String, String> {
    text.split_whitespace()
        .next()
        .map(str::to_ascii_lowercase)
        .filter(|digest| digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| "The checksum file does not hold a SHA-256 digest".to_string())
}

fn agent(limit: Option<Duration>) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_connect(Some(CONNECT_TIMEOUT)).timeout_recv_body(limit).http_status_as_error(false).build().into()
}

fn text(url: &str) -> Result<String, String> {
    let mut answer = agent(Some(CONNECT_TIMEOUT)).get(url).call().map_err(|e| format!("{url}: {e}"))?;
    match answer.status().as_u16() {
        200 => answer.body_mut().read_to_string().map_err(|e| format!("{url}: {e}")),
        status => Err(format!("{url} answered HTTP {status}")),
    }
}

pub fn fetch_releases(channel: &str) -> Result<Vec<Release>, String> {
    releases_url(channel).ok_or_else(|| format!("Unknown channel {channel}")).and_then(|url| text(&url)).and_then(|json| parse_releases(&json))
}

pub fn sha256_of(path: &Path, cancelled: &dyn Fn() -> bool) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; IMAGE_CHUNK];
    loop {
        if cancelled() {
            return Err(CANCELLED_BY_USER.to_string());
        }
        match file.read(&mut buf).map_err(|e| format!("{}: {e}", path.display()))? {
            0 => return Ok(crate::signingkeys::to_hex(&hash.finalize())),
            n => hash.update(&buf[..n]),
        }
    }
}

pub fn cached(folder: &Path, filename: &str, expected: &str) -> Option<PathBuf> {
    let image = folder.join(filename);
    let marker = std::fs::read_to_string(folder.join(format!("{filename}.sha256"))).ok()?;
    (image.is_file() && marker.trim() == expected).then_some(image)
}

pub fn download(image: &Image, folder: &Path, cancelled: &dyn Fn() -> bool, progress: &mut dyn FnMut(u64, u64)) -> Result<PathBuf, String> {
    let expected = parse_sha256(&text(&image.checksum_url)?)?;
    if let Some(found) = cached(folder, &image.filename, &expected) {
        progress(image.size, image.size);
        return Ok(found);
    }
    std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let part = folder.join(format!("{}.part", image.filename));
    fetch_with_retries(&image.download_url, &part, image.size, cancelled, progress, 0)?;
    match sha256_of(&part, cancelled)? == expected {
        true => {
            let finished = folder.join(&image.filename);
            std::fs::rename(&part, &finished).map_err(|e| format!("{}: {e}", finished.display()))?;
            std::fs::write(folder.join(format!("{}.sha256", image.filename)), &expected).map_err(|e| format!("{}: {e}", folder.display()))?;
            Ok(finished)
        }
        false => {
            let _ = std::fs::remove_file(&part);
            Err("The download is corrupt (its checksum does not match). Try again.".to_string())
        }
    }
}

fn fetch_with_retries(url: &str, part: &Path, size_hint: u64, cancelled: &dyn Fn() -> bool, progress: &mut dyn FnMut(u64, u64), attempt: u32) -> Result<(), String> {
    match fetch_into(url, part, size_hint, cancelled, progress) {
        Ok(()) => Ok(()),
        Err(e) if e == CANCELLED_BY_USER || attempt + 1 >= ATTEMPTS => Err(e),
        Err(_) => {
            wait_unless_cancelled(Duration::from_secs((1u64 << attempt.min(5)).min(BACKOFF_CAP_SECS)), cancelled);
            match cancelled() {
                true => Err(CANCELLED_BY_USER.to_string()),
                false => fetch_with_retries(url, part, size_hint, cancelled, progress, attempt + 1),
            }
        }
    }
}

fn wait_unless_cancelled(total: Duration, cancelled: &dyn Fn() -> bool) {
    let step = Duration::from_millis(100);
    let _ = (0..total.as_millis() / step.as_millis()).take_while(|_| !cancelled()).for_each(|_| std::thread::sleep(step));
}

fn fetch_into(url: &str, part: &Path, size_hint: u64, cancelled: &dyn Fn() -> bool, progress: &mut dyn FnMut(u64, u64)) -> Result<(), String> {
    let start = std::fs::metadata(part).map(|m| m.len()).unwrap_or(0);
    let request = agent(Some(ATTEMPT_LIMIT)).get(url);
    let request = match start {
        0 => request,
        n => request.header("Range", &format!("bytes={n}-")),
    };
    let mut answer = request.call().map_err(|e| format!("{url}: {e}"))?;
    let length = answer.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok());
    let (resume_at, total) = match answer.status().as_u16() {
        206 => (start, length.map(|l| start + l).unwrap_or(size_hint)),
        200 => (0, length.unwrap_or(size_hint)),
        416 if start > 0 => return Ok(()),
        status => return Err(format!("{url} answered HTTP {status}")),
    };
    let mut file = OpenOptions::new().create(true).write(true).truncate(resume_at == 0).open(part).map_err(|e| format!("{}: {e}", part.display()))?;
    file.seek(SeekFrom::Start(resume_at)).map_err(|e| format!("{}: {e}", part.display()))?;
    let mut reader = answer.body_mut().as_reader();
    let mut done = resume_at;
    let mut buf = vec![0u8; IMAGE_CHUNK];
    progress(done, total);
    loop {
        if cancelled() {
            return Err(CANCELLED_BY_USER.to_string());
        }
        match reader.read(&mut buf).map_err(|e| format!("{url}: {e}"))? {
            0 => break,
            n => {
                file.write_all(&buf[..n]).map_err(|e| format!("{}: {e}", part.display()))?;
                done += n as u64;
                progress(done, total);
            }
        }
    }
    match done >= total || total == 0 {
        true => Ok(()),
        false => Err(format!("{url}: the download stopped at {done} of {total} bytes")),
    }
}

fn multibyte(bytes: &[u8]) -> Option<(u64, usize)> {
    bytes
        .iter()
        .take(9)
        .enumerate()
        .try_fold(0u64, |value, (i, &b)| {
            let value = value | (u64::from(b & 0x7f) << (7 * i));
            match b & 0x80 {
                0 => Err((value, i + 1)),
                _ => Ok(value),
            }
        })
        .err()
}

pub fn xz_uncompressed_len(file: &mut File) -> Option<u64> {
    let len = file.metadata().ok()?.len();
    let scan = len.min(XZ_PADDING_SCAN);
    let mut tail = vec![0u8; scan as usize];
    file.seek(SeekFrom::Start(len - scan)).ok()?;
    file.read_exact(&mut tail).ok()?;
    let padding = tail.iter().rev().take_while(|&&b| b == 0).count() as u64 / 4 * 4;
    let footer_end = len - padding;
    let footer_at = footer_end.checked_sub(XZ_FOOTER_LEN)?;
    let footer = &tail[(footer_at - (len - scan)) as usize..(footer_end - (len - scan)) as usize];
    (footer[10..12] == *b"YZ").then_some(())?;
    let index_len = (u64::from(u32::from_le_bytes([footer[4], footer[5], footer[6], footer[7]])) + 1) * 4;
    let index_at = footer_at.checked_sub(index_len)?;
    let mut index = vec![0u8; index_len as usize];
    file.seek(SeekFrom::Start(index_at)).ok()?;
    file.read_exact(&mut index).ok()?;
    (index.first() == Some(&0)).then_some(())?;
    let (records, used) = multibyte(&index[1..])?;
    let (blocks, uncompressed, _) = (0..records).try_fold((0u64, 0u64, 1 + used), |(blocks, uncompressed, at), _| {
        let (unpadded, a) = multibyte(index.get(at..)?)?;
        let (size, b) = multibyte(index.get(at + a..)?)?;
        Some((blocks + unpadded.div_ceil(4) * 4, uncompressed + size, at + a + b))
    })?;
    (XZ_HEADER_LEN + blocks + index_len + XZ_FOOTER_LEN == footer_end).then_some(uncompressed)
}

pub fn uncompressed_len(path: &Path) -> Result<u64, String> {
    let name = path.to_string_lossy().to_ascii_lowercase();
    let mut file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    match (name.ends_with(".xz"), name.ends_with(".gz")) {
        (true, _) => match xz_uncompressed_len(&mut file) {
            Some(len) => Ok(len),
            None => count(open_image(path)?),
        },
        (_, true) => count(open_image(path)?),
        _ => file.metadata().map(|m| m.len()).map_err(|e| format!("{}: {e}", path.display())),
    }
}

fn count(mut reader: Box<dyn Read + Send>) -> Result<u64, String> {
    io::copy(&mut reader, &mut io::sink()).map_err(|e| format!("The image did not decompress: {e}"))
}

struct ChunkSender(SyncSender<io::Result<Vec<u8>>>);

impl Write for ChunkSender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(Ok(buf.to_vec())).map(|_| buf.len()).map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "the image reader went away"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct ChunkReader {
    chunks: Receiver<io::Result<Vec<u8>>>,
    current: Vec<u8>,
    at: usize,
}

impl Read for ChunkReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while self.at == self.current.len() {
            match self.chunks.recv() {
                Ok(Ok(chunk)) => {
                    self.current = chunk;
                    self.at = 0;
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => return Ok(0),
            }
        }
        let n = buf.len().min(self.current.len() - self.at);
        buf[..n].copy_from_slice(&self.current[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

pub fn open_image(path: &Path) -> Result<Box<dyn Read + Send>, String> {
    let name = path.to_string_lossy().to_ascii_lowercase();
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    match (name.ends_with(".xz"), name.ends_with(".gz")) {
        (true, _) => {
            let (sender, chunks) = sync_channel(PIPE_DEPTH);
            std::thread::Builder::new()
                .name("card-image-xz".into())
                .spawn(move || {
                    let failed = sender.clone();
                    if let Err(e) = lzma_rs::xz_decompress(&mut BufReader::with_capacity(IMAGE_CHUNK, file), &mut ChunkSender(sender)) {
                        let _ = failed.send(Err(io::Error::new(io::ErrorKind::InvalidData, format!("xz: {e:?}"))));
                    }
                })
                .map_err(|e| format!("could not start the decompressor: {e}"))?;
            Ok(Box::new(ChunkReader { chunks, current: Vec::new(), at: 0 }))
        }
        (_, true) => Ok(Box::new(flate2::read::MultiGzDecoder::new(BufReader::with_capacity(IMAGE_CHUNK, file)))),
        _ => Ok(Box::new(BufReader::with_capacity(IMAGE_CHUNK, file))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = r#"{"releases":[{"version":"v0.3.5","prerelease":false,"created_at":"2026-08-22T15:56:38Z","image":{"filename":"aircast-lite-arm64-v0.3.5.img.xz","extension":"img.xz","size":576627168,"download_url":"https://downloads.aircast.one/lite/v0.3.5/aircast-lite-arm64-v0.3.5.img.xz","checksum_url":"https://downloads.aircast.one/lite/v0.3.5/aircast-lite-arm64-v0.3.5.img.xz.sha256"}}]}"#;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cardimage-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample(len: usize) -> Vec<u8> {
        (0..len).map(|i| ((i * 31) % 251) as u8).collect()
    }

    fn xz(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        lzma_rs::xz_compress(&mut io::Cursor::new(data), &mut out).unwrap();
        out
    }

    #[test]
    fn the_release_list_parses_the_published_shape() {
        let releases = parse_releases(LISTING).unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].version, "v0.3.5");
        assert_eq!(releases[0].image.uncompressed_size, None);
        assert_eq!(releases[0].image.size, 576_627_168);
    }

    #[test]
    fn every_channel_has_a_releases_url_and_unknown_ones_do_not() {
        assert_eq!(releases_url("stable").as_deref(), Some("https://downloads.aircast.one/lite/releases.json"));
        assert_eq!(releases_url("development").as_deref(), Some("https://downloads-dev.aircast.one/lite/releases.json"));
        assert_eq!(releases_url("nightly"), None);
    }

    #[test]
    fn a_checksum_file_yields_its_digest_and_junk_is_refused() {
        let digest = "A".repeat(64);
        assert_eq!(parse_sha256(&format!("{digest}  aircast.img.xz\n")).unwrap(), "a".repeat(64));
        assert!(parse_sha256("<html>not found</html>").is_err());
        assert!(parse_sha256("").is_err());
    }

    #[test]
    fn the_xz_index_gives_the_uncompressed_size_without_decompressing() {
        let dir = scratch("index");
        let data = sample(3 * 1024 * 1024 + 17);
        let path = dir.join("image.img.xz");
        std::fs::write(&path, xz(&data)).unwrap();
        assert_eq!(xz_uncompressed_len(&mut File::open(&path).unwrap()), Some(data.len() as u64));
        assert_eq!(uncompressed_len(&path).unwrap(), data.len() as u64);
    }

    #[test]
    fn stream_padding_after_the_footer_is_skipped() {
        let dir = scratch("padding");
        let data = sample(100_000);
        let path = dir.join("padded.img.xz");
        std::fs::write(&path, [xz(&data), vec![0u8; 8]].concat()).unwrap();
        assert_eq!(xz_uncompressed_len(&mut File::open(&path).unwrap()), Some(data.len() as u64));
    }

    #[test]
    fn a_file_that_is_not_xz_has_no_index() {
        let dir = scratch("notxz");
        let path = dir.join("fake.img.xz");
        std::fs::write(&path, sample(5000)).unwrap();
        assert_eq!(xz_uncompressed_len(&mut File::open(&path).unwrap()), None);
    }

    #[test]
    fn images_stream_back_byte_for_byte_whatever_the_compression() {
        let dir = scratch("stream");
        let data = sample(2 * 1024 * 1024 + 5);
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(&data).unwrap();
        [("a.img.xz", xz(&data)), ("a.img.gz", gz.finish().unwrap()), ("a.img", data.clone())].iter().for_each(|(name, bytes)| {
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
            let mut out = Vec::new();
            open_image(&path).unwrap().read_to_end(&mut out).unwrap();
            assert_eq!(out, data, "{name}");
            assert_eq!(uncompressed_len(&path).unwrap(), data.len() as u64, "{name}");
        });
    }

    #[test]
    fn a_truncated_xz_reports_an_error_instead_of_a_short_image() {
        let dir = scratch("truncated");
        let noise: Vec<u8> = (0..500_000u64).map(|i| (i.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) >> 33) as u8).collect();
        let bytes = xz(&noise);
        let path = dir.join("broken.img.xz");
        std::fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();
        let mut out = Vec::new();
        assert!(open_image(&path).unwrap().read_to_end(&mut out).is_err());
    }

    #[test]
    fn a_verified_download_is_reused_and_a_wrong_marker_is_not() {
        let dir = scratch("cache");
        std::fs::write(dir.join("a.img.xz"), b"image").unwrap();
        std::fs::write(dir.join("a.img.xz.sha256"), "abc").unwrap();
        assert_eq!(cached(&dir, "a.img.xz", "abc"), Some(dir.join("a.img.xz")));
        assert_eq!(cached(&dir, "a.img.xz", "def"), None);
        assert_eq!(cached(&dir, "missing.img.xz", "abc"), None);
    }

    #[test]
    fn hashing_a_file_matches_sha256_and_stops_when_cancelled() {
        let dir = scratch("hash");
        std::fs::write(dir.join("f"), b"abc").unwrap();
        assert_eq!(sha256_of(&dir.join("f"), &|| false).unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(sha256_of(&dir.join("f"), &|| true).unwrap_err(), CANCELLED_BY_USER);
    }

    #[test]
    fn multibyte_integers_decode_like_the_xz_spec() {
        assert_eq!(multibyte(&[0x05]), Some((5, 1)));
        assert_eq!(multibyte(&[0x80, 0x01]), Some((128, 2)));
        assert_eq!(multibyte(&[0xff, 0xff, 0x03]), Some((65535, 3)));
        assert_eq!(multibyte(&[0x80]), None);
    }
}
