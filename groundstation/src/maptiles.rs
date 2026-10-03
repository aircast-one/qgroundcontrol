use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use quick_cache::Weighter;

use crate::mapurls::{Keys, TileRequest, tile_request};
use crate::tilecache::{Cache, Tile, provider_hash, tile_hash};

const BING_NO_TILE: &[u8] = include_bytes!("../../resources/BingNoTileBytes.dat");
const ELEVATION_PROVIDER: &str = "Copernicus";
const DISK_LIMIT_PATH: &str = "settings.mapsSettings.maxCacheDiskSize";
const DEFAULT_DISK_LIMIT_MB: u64 = 1024;
const LIMIT_CHECK_EVERY: Duration = Duration::from_secs(2);
const MEMORY_LIMIT_PATH: &str = "settings.mapsSettings.maxCacheMemorySize";
const MEGABYTE: u64 = 1024 * 1024;
const TYPICAL_TILE_BYTES: u64 = 20 * 1024;

static LAST_LIMIT_CHECK: LazyLock<Mutex<Option<Instant>>> = LazyLock::new(|| Mutex::new(None));

fn disk_limit_bytes() -> i64 {
    let megabytes = crate::settingsstore::raw_setting(DISK_LIMIT_PATH).and_then(|value| value.as_u64()).unwrap_or(DEFAULT_DISK_LIMIT_MB);
    (megabytes * 1024 * 1024) as i64
}

fn keep_within_disk_limit(cache: &Cache) {
    let due = {
        let mut last = LAST_LIMIT_CHECK.lock().unwrap_or_else(PoisonError::into_inner);
        let due = last.is_none_or(|checked| checked.elapsed() >= LIMIT_CHECK_EVERY);
        if due {
            *last = Some(Instant::now());
        }
        due
    };
    if due {
        let _ = cache.trim_to(disk_limit_bytes());
    }
}

type TileKey = (String, i32, i32, i32);

#[derive(Clone)]
struct ImageBytes;

impl Weighter<TileKey, Arc<[u8]>> for ImageBytes {
    fn weight(&self, _key: &TileKey, image: &Arc<[u8]>) -> u64 {
        image.len() as u64
    }
}

pub struct MemoryCache(quick_cache::sync::Cache<TileKey, Arc<[u8]>, ImageBytes>);

impl MemoryCache {
    pub fn with_limit_mb(megabytes: u64) -> MemoryCache {
        let bytes = megabytes.clamp(1, 1024) * MEGABYTE;
        MemoryCache(quick_cache::sync::Cache::with_weighter((bytes / TYPICAL_TILE_BYTES) as usize, bytes, ImageBytes))
    }

    pub fn through(&self, provider: &str, x: i32, y: i32, zoom: i32, fetch: impl FnOnce() -> Option<Vec<u8>>) -> Option<Vec<u8>> {
        let key = (provider.to_string(), x, y, zoom);
        self.0.get(&key).map(|image| image.to_vec()).or_else(|| {
            let image = fetch()?;
            self.0.insert(key, Arc::from(image.as_slice()));
            Some(image)
        })
    }
}

static MEMORY: LazyLock<MemoryCache> = LazyLock::new(|| {
    MemoryCache::with_limit_mb(crate::settingsstore::raw_setting(MEMORY_LIMIT_PATH).and_then(|value| value.as_u64()).unwrap_or(0))
});

pub fn fetch_remembered(provider: &str, x: i32, y: i32, zoom: i32, cache: Option<&Cache>, persist: bool) -> Option<Vec<u8>> {
    MEMORY.through(provider, x, y, zoom, || fetch(provider, x, y, zoom, &crate::mapurls::keys_from_settings(), cache, persist, &fetch_over_http))
}

pub fn image_format(image: &[u8]) -> Option<&'static str> {
    [(&b"\x89PNG\r\n\x1a\n"[..], "png"), (&b"\xff\xd8\xff"[..], "jpg"), (&b"GIF8"[..], "gif")]
        .iter()
        .find(|(signature, _)| image.len() >= 3 && image.starts_with(signature))
        .map(|(_, format)| *format)
}

pub fn fetch(provider: &str, x: i32, y: i32, zoom: i32, keys: &Keys, cache: Option<&Cache>, persist: bool, http: &dyn Fn(&TileRequest) -> Result<Vec<u8>, String>) -> Option<Vec<u8>> {
    let kind = provider_hash(provider).filter(|_| provider != ELEVATION_PROVIDER)?;
    let hash = tile_hash(kind, x, y, zoom);
    if let Some(tile) = cache.and_then(|c| c.tile(&hash).ok().flatten()) {
        return Some(tile.image);
    }
    let request = tile_request(provider, x, y, zoom, keys)?;
    let image = http(&request).ok().filter(|image| !image.is_empty())?;
    if provider.starts_with("Bing") && image == BING_NO_TILE {
        return None;
    }
    let format = image_format(&image)?;
    if let Some(cache) = cache.filter(|_| persist) {
        let _ = cache.save(&Tile { hash, format: format.to_string(), image: image.clone(), kind }, None);
        keep_within_disk_limit(cache);
    }
    Some(image)
}

pub fn fetch_over_http(request: &TileRequest) -> Result<Vec<u8>, String> {
    let asked = request.headers.iter().fold(ureq::get(&request.url), |asked, (name, value)| asked.header(name, value));
    asked.call().map_err(|e| e.to_string())?.body_mut().read_to_vec().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> Keys {
        Keys { mapbox_token: String::new(), mapbox_account: String::new(), mapbox_style: String::new(), esri_token: String::new(), custom_url: String::new(), tianditu_token: String::new(), openaip_token: String::new(), vworld_token: String::new(), language: "en-US".into() }
    }

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n rest";

    #[test]
    fn a_tile_held_in_memory_is_served_without_asking_the_disk_or_network_again() {
        let memory = MemoryCache::with_limit_mb(1);
        assert_eq!(memory.through("Google Satellite", 1, 2, 3, || Some(PNG.to_vec())), Some(PNG.to_vec()));
        assert_eq!(memory.through("Google Satellite", 1, 2, 3, || panic!("held tiles never reach the fetch")), Some(PNG.to_vec()));
        assert_eq!(memory.through("Google Satellite", 1, 2, 4, || None), None);
        assert_eq!(memory.through("Google Satellite", 1, 2, 4, || Some(PNG.to_vec())), Some(PNG.to_vec()), "a missing tile is not remembered as missing");
    }

    #[test]
    fn the_memory_limit_is_clamped_like_qgeofiletilecacheqgc_and_evicts_past_it() {
        let memory = MemoryCache::with_limit_mb(0);
        assert_eq!(memory.0.capacity(), MEGABYTE);
        assert_eq!(MemoryCache::with_limit_mb(5000).0.capacity(), 1024 * MEGABYTE);
        let tile = vec![0u8; 300 * 1024];
        (0..8).for_each(|x| {
            memory.through("Google Satellite", x, 0, 1, || Some(tile.clone()));
        });
        assert!(memory.0.weight() <= MEGABYTE);
        assert!(memory.0.len() < 8);
    }

    #[test]
    fn a_fetched_tile_is_cached_and_the_next_request_never_reaches_the_network() {
        let cache = Cache::open_in_memory().unwrap();
        let calls = std::cell::Cell::new(0);
        let network = |_: &TileRequest| {
            calls.set(calls.get() + 1);
            Ok(PNG.to_vec())
        };
        assert_eq!(fetch("Google Satellite", 1, 2, 3, &keys(), Some(&cache), true, &network), Some(PNG.to_vec()));
        assert_eq!(fetch("Google Satellite", 1, 2, 3, &keys(), Some(&cache), true, &network), Some(PNG.to_vec()));
        assert_eq!(calls.get(), 1);
        let offline = |_: &TileRequest| Err("offline".to_string());
        assert_eq!(fetch("Google Satellite", 9, 9, 9, &keys(), Some(&cache), true, &offline), None);
        assert!(fetch("Google Satellite", 5, 5, 5, &keys(), Some(&cache), false, &network).is_some());
        assert_eq!(fetch("Google Satellite", 5, 5, 5, &keys(), Some(&cache), false, &offline), None, "disableAllPersistence keeps tiles out of the cache");
    }

    #[test]
    fn bings_placeholder_and_unknown_formats_are_not_tiles() {
        let bing = |_: &TileRequest| Ok(BING_NO_TILE.to_vec());
        assert_eq!(fetch("Bing Road", 1, 1, 20, &keys(), None, true, &bing), None);
        let text = |_: &TileRequest| Ok(b"<html>".to_vec());
        assert_eq!(fetch("Esri World Street", 1, 1, 3, &keys(), None, true, &text), None);
        assert_eq!(image_format(b"\xff\xd8\xff\xe0"), Some("jpg"));
        assert_eq!(fetch("Copernicus", 1, 1, 3, &keys(), None, true, &text), None, "elevation tiles are terrain data, not map images");
    }
}
