use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use serde_json::{Value, json};

use crate::router::Backend;
use crate::tilecache::{Cache, PROVIDERS, Tile, TileSet, provider_hash};

pub const DEPS: &[&str] = &["settings.offlineMapsSettings.maxTilesForDownload"];

pub const START_DOWNLOAD: &str = "offlineMaps.startDownload";
pub const RESUME_DOWNLOAD: &str = "offlineMaps.resume";
pub const CANCEL_DOWNLOAD: &str = "offlineMaps.cancel";
pub const DELETE_SET: &str = "offlineMaps.delete";
pub const RENAME_SET: &str = "offlineMaps.rename";
pub const EXPORT_SETS: &str = "offlineMaps.export";
pub const IMPORT_SETS: &str = "offlineMaps.import";

const MAX_MAP_ZOOM: i32 = 23;
const DEFAULT_AVERAGE_TILE_SIZE: u64 = 13652;
const UNKNOWN_AVERAGE_TILE_SIZE: u64 = 4096;
const AVERAGE_SAMPLE_FLOOR: i64 = 10;
const TILE_BATCH: usize = 256;
const CONCURRENT_DOWNLOADS: usize = 6;
const MAX_TILES_PATH: &str = "settings.offlineMapsSettings.maxTilesForDownload";
const DEFAULT_MAX_TILES: u64 = 100_000;

const AVERAGE_SIZES: &[(&str, u64)] = &[
    ("Bing Hybrid", 19597),
    ("Bing Road", 1297),
    ("Bing Satellite", 19597),
    (crate::maptypes::BUILDING_TILES, 42084),
    ("Copernicus", 2786),
    (crate::maptypes::TERRAIN_TILES, 50000),
    ("Google Hybrid", 56887),
    ("Google Satellite", 56887),
    ("Google Street Map", 4913),
    ("Google Terrain", 19391),
    ("Mapbox Hybrid", 15739),
    ("Mapbox Satellite", 15739),
    ("Mapbox Streets", 5648),
    ("TianDiTu Road", 1297),
    ("TianDiTu Satellite", 19597),
];

static DOWNLOADS: LazyLock<Mutex<BTreeMap<i64, Arc<AtomicBool>>>> = LazyLock::new(|| Mutex::new(BTreeMap::new()));

pub fn average_size(provider: &str) -> u64 {
    AVERAGE_SIZES.iter().find(|(name, _)| *name == provider).map_or(DEFAULT_AVERAGE_TILE_SIZE, |(_, size)| *size)
}

pub fn tile_x(lon: f64, zoom: i32) -> i32 {
    ((lon + 180.0) / 360.0 * 2f64.powi(zoom)).floor() as i32
}

pub fn tile_y(lat: f64, zoom: i32) -> i32 {
    let rad = lat.to_radians();
    ((1.0 - (rad.tan() + 1.0 / rad.cos()).ln() / std::f64::consts::PI) / 2.0 * 2f64.powi(zoom)).floor() as i32
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Region {
    pub top_left_lon: f64,
    pub top_left_lat: f64,
    pub bottom_right_lon: f64,
    pub bottom_right_lat: f64,
}

fn range(region: &Region, zoom: i32) -> (i32, i32, i32, i32, i32) {
    let zoom = zoom.clamp(1, MAX_MAP_ZOOM);
    (zoom, tile_x(region.top_left_lon, zoom), tile_y(region.top_left_lat, zoom), tile_x(region.bottom_right_lon, zoom), tile_y(region.bottom_right_lat, zoom))
}

pub fn tile_count(region: &Region, zoom: i32) -> u64 {
    let (_, x0, y0, x1, y1) = range(region, zoom);
    (i64::from(x1) - i64::from(x0) + 1).max(0) as u64 * (i64::from(y1) - i64::from(y0) + 1).max(0) as u64
}

pub fn estimate(provider: &str, region: &Region, min_zoom: i32, max_zoom: i32) -> (u64, u64) {
    let count: u64 = (min_zoom..=max_zoom).map(|zoom| tile_count(region, zoom)).sum();
    (count, count * average_size(provider))
}

fn tiles(region: &Region, min_zoom: i32, max_zoom: i32) -> Vec<(i32, i32, i32)> {
    (min_zoom..=max_zoom)
        .flat_map(|zoom| {
            let (z, x0, y0, x1, y1) = range(region, zoom);
            (x0..=x1).flat_map(move |x| (y0..=y1).map(move |y| (x, y, z)))
        })
        .collect()
}

pub fn grouped(number: u64) -> String {
    let digits = number.to_string();
    let head = digits.len() % 3;
    digits
        .chars()
        .enumerate()
        .flat_map(|(index, digit)| (index > 0 && (index + 3 - head).is_multiple_of(3)).then_some(',').into_iter().chain(std::iter::once(digit)))
        .collect()
}

fn size_text(size: u64) -> String {
    crate::onboardlogs::big_size_text(size)
}

pub fn unique_name(taken: &[String]) -> String {
    (1..).map(|count| format!("Tile Set {count:03}")).find(|name| !taken.contains(name)).unwrap_or_default()
}

fn open_cache() -> Result<Cache, String> {
    let path = crate::terrainservice::cache_path().ok_or("There is no map tile cache on this device.")?;
    Cache::open(&path).map_err(|error| error.to_string())
}

fn downloading(set: i64) -> bool {
    DOWNLOADS.lock().unwrap_or_else(PoisonError::into_inner).contains_key(&set)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Totals {
    pub saved_count: i64,
    pub saved_size: i64,
    pub unique_count: i64,
    pub unique_size: i64,
    pub total_size: i64,
    pub errors: i64,
}

pub fn totals(cache: &Cache, set: &TileSet) -> rusqlite::Result<Totals> {
    if set.default_set {
        let (unique_count, unique_size) = cache.unique(set.id)?;
        let size = cache.total_size()?;
        return Ok(Totals { saved_count: cache.count()?, saved_size: size, unique_count, unique_size, total_size: size, errors: 0 });
    }
    let (saved_count, saved_size) = cache.saved(set.id)?;
    let (unique_count, unique_size) = cache.unique(set.id)?;
    let fallback = crate::tilecache::provider_named(set.kind).map_or(UNKNOWN_AVERAGE_TILE_SIZE, average_size) as i64;
    let average = if saved_count > AVERAGE_SAMPLE_FLOOR && saved_size > 0 { saved_size / saved_count } else { fallback };
    let total_size = if set.tiles <= saved_count { saved_size } else { average * set.tiles };
    let (unique_count, unique_size) = match unique_count {
        0 => {
            let estimated = (set.tiles - saved_count).max(0);
            (estimated, estimated * average)
        }
        known => (known, unique_size),
    };
    Ok(Totals { saved_count, saved_size, unique_count, unique_size, total_size, errors: cache.errors(set.id)? })
}

fn download_status(set: &TileSet, totals: &Totals) -> String {
    match (set.default_set, set.tiles <= totals.saved_count) {
        (true, _) => size_text(totals.total_size as u64),
        (false, true) => size_text(totals.saved_size as u64),
        (false, false) => format!("{} / {}", size_text(totals.saved_size as u64), size_text(totals.total_size as u64)),
    }
}

fn row_text(status: &str, tiles: i64) -> String {
    match tiles > 0 {
        true => format!("{status} ({tiles} tiles)"),
        false => status.to_string(),
    }
}

fn free_name(wanted: &str, taken: &[String]) -> String {
    std::iter::once(wanted.to_string()).chain((1..).map(|i| format!("{wanted} ({i})"))).find(|name| !taken.contains(name)).unwrap_or_default()
}

fn set_json(cache: &Cache, set: &TileSet) -> Option<Value> {
    let totals = totals(cache, set).ok()?;
    Some(json!({
        "id": set.id,
        "name": set.name,
        "subtitle": if set.default_set { "System Wide Tile Cache" } else { "" },
        "rowText": row_text(&download_status(set, &totals), if set.default_set { totals.unique_count } else { set.tiles }),
        "canDelete": totals.saved_size > 0,
        "mapTypeStr": set.type_str,
        "defaultSet": set.default_set,
        "zoomText": format!("{} - {}", set.min_zoom, set.max_zoom),
        "totalText": format!("{} ({})", grouped(set.tiles.max(0) as u64), size_text(totals.total_size as u64)),
        "uniqueText": format!("{} ({})", grouped(totals.unique_count as u64), size_text(totals.unique_size as u64)),
        "uniqueCount": totals.unique_count,
        "downloadedText": format!("{} ({})", grouped(totals.saved_count as u64), size_text(totals.saved_size as u64)),
        "sizeText": size_text(totals.saved_size as u64),
        "tileCountText": grouped(totals.saved_count as u64),
        "errorCount": totals.errors,
        "errorCountText": grouped(totals.errors as u64),
        "downloadStatus": download_status(set, &totals),
        "downloading": downloading(set.id),
        "complete": set.default_set || set.tiles <= totals.saved_count,
    }))
}

fn number(args: &[String], index: usize) -> Option<f64> {
    args.get(index).and_then(|arg| arg.parse::<f64>().ok()).filter(|value| value.is_finite())
}

fn region_of(args: &[String], first: usize) -> Option<Region> {
    Some(Region { top_left_lon: number(args, first)?, top_left_lat: number(args, first + 1)?, bottom_right_lon: number(args, first + 2)?, bottom_right_lat: number(args, first + 3)? })
}

fn max_tiles(backend: &dyn Backend) -> u64 {
    backend.value(MAX_TILES_PATH).get("value").and_then(Value::as_u64).unwrap_or(DEFAULT_MAX_TILES)
}

pub fn offline_maps_view(backend: &dyn Backend, args: &[String]) -> Value {
    let cache = open_cache();
    let sets = cache.as_ref().ok().and_then(|cache| cache.sets().ok()).unwrap_or_default();
    let taken: Vec<String> = sets.iter().map(|set| set.name.clone()).collect();
    let limit = max_tiles(backend);
    let estimate_json = match (args.first().filter(|provider| provider_hash(provider).is_some()), region_of(args, 1), number(args, 5), number(args, 6)) {
        (Some(provider), Some(region), Some(min_zoom), Some(max_zoom)) => {
            let (image_count, image_size) = estimate(provider, &region, min_zoom as i32, max_zoom as i32);
            let fetch_elevation = args.get(7).is_none_or(|flag| flag != "false") && !crate::maptypes::ELEVATION_PROVIDERS.contains(&provider.as_str());
            let elevation_count = if fetch_elevation { crate::terrainquery::region_tiles((region.top_left_lat, region.top_left_lon), (region.bottom_right_lat, region.bottom_right_lon)).len() as u64 } else { 0 };
            let elevation_size = elevation_count * average_size(crate::maptypes::ELEVATION_PROVIDERS[0]);
            let (companion_count, companion_size) = if fetch_elevation { companions_estimate(&region, max_zoom as i32) } else { (0, 0) };
            let (count, size) = (image_count + elevation_count + companion_count, image_size + elevation_size + companion_size);
            json!({ "tileCount": count, "tileCountText": grouped(count), "tileSizeText": size_text(size), "tooMany": count > limit })
        }
        _ => Value::Null,
    };
    json!({
        "kind": "object",
        "class": "OfflineMaps",
        "available": cache.is_ok(),
        "reason": cache.as_ref().err(),
        "sets": cache.as_ref().ok().map(|cache| sets.iter().filter_map(|set| set_json(cache, set)).collect::<Vec<_>>()).unwrap_or_default(),
        "mapList": PROVIDERS.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        "uniqueName": unique_name(&taken),
        "takenNames": taken,
        "maxTilesForDownload": limit,
        "estimate": estimate_json,
    })
}

#[cfg(not(target_family = "wasm"))]
fn fetched_until_cancelled<T: Sync, R: Send>(batch: &[T], cancel: &AtomicBool, fetch: impl Fn(&T) -> R + Sync) -> Vec<R> {
    std::thread::scope(|scope| {
        batch
            .chunks(batch.len().div_ceil(CONCURRENT_DOWNLOADS).max(1))
            .map(|chunk| scope.spawn(|| chunk.iter().take_while(|_| !cancel.load(Ordering::Relaxed)).map(&fetch).collect::<Vec<_>>()))
            .collect::<Vec<_>>()
            .into_iter()
            .flat_map(|worker| worker.join().unwrap_or_default())
            .collect()
    })
}

#[cfg(not(target_family = "wasm"))]
fn download(set: i64, provider: String, cancel: Arc<AtomicBool>) {
    let keys = crate::mapurls::keys_from_settings();
    let Some(kind) = provider_hash(&provider) else { return };
    let exhausted = |cache: &Cache| -> bool {
        let batch = cache.pending(set, TILE_BATCH).unwrap_or_default();
        if batch.is_empty() || cancel.load(Ordering::Relaxed) {
            return true;
        }
        let fetched: Vec<(String, Option<(String, Vec<u8>)>)> = fetched_until_cancelled(&batch, &cancel, |(hash, x, y, z)| match crate::maptypes::ELEVATION_PROVIDERS.contains(&provider.as_str()) {
            true => (hash.clone(), crate::terrainquery::fetched_tile(*x, *y, &crate::terrainquery::fetch_over_http)),
            false => (hash.clone(), crate::maptiles::fetch(&provider, *x, *y, *z, &keys, None, false, &crate::maptiles::fetch_over_http).and_then(|image| Some((crate::maptiles::tile_format(&provider, &image)?.to_string(), image)))),
        });
        fetched.iter().for_each(|(hash, image)| {
            let stored = image.as_ref().and_then(|(format, image)| cache.complete(set, &Tile { hash: hash.clone(), format: format.clone(), image: image.clone(), kind }).ok());
            if stored.is_none() {
                let _ = cache.mark_error(set, hash);
            }
        });
        false
    };
    if let Ok(cache) = open_cache() {
        while !exhausted(&cache) {}
    }
    DOWNLOADS.lock().unwrap_or_else(PoisonError::into_inner).remove(&set);
    log::info!("Offline map set {set} download stopped");
}

fn start(set: i64, provider: &str) {
    let cancel = Arc::new(AtomicBool::new(false));
    let fresh = DOWNLOADS.lock().unwrap_or_else(PoisonError::into_inner).insert(set, cancel.clone()).is_none();
    if fresh {
        let provider = provider.to_string();
        #[cfg(not(target_family = "wasm"))]
        std::thread::spawn(move || download(set, provider, cancel));
        #[cfg(target_family = "wasm")]
        wasm_bindgen_futures::spawn_local(browser::download(set, provider, cancel));
    }
}

#[cfg(target_family = "wasm")]
pub mod browser {
    use super::*;

    const TILE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
    const RESTORED: &str = "/restored-tiles.db";

    type Memory = sqlite_wasm_rs::MemVfsUtil<sqlite_wasm_rs::WasmOsCallback>;

    async fn fetched(provider: &str, keys: &crate::mapurls::Keys, x: i32, y: i32, z: i32) -> Option<(String, Vec<u8>)> {
        match crate::maptypes::ELEVATION_PROVIDERS.contains(&provider) {
            true => crate::terrainquery::tile_later(x, y).await.ok().map(|tile| (crate::terrainquery::TILE_FORMAT.to_string(), crate::terraintile::encode(&tile))),
            false => {
                let request = crate::mapurls::tile_request(provider, x, y, z, keys)?;
                let image = crate::http::get_later(&request.url, &request.headers, TILE_TIMEOUT).await.ok()?;
                Some((crate::maptiles::tile_format(provider, &image)?.to_string(), image))
            }
        }
    }

    pub async fn download(set: i64, provider: String, cancel: Arc<AtomicBool>) {
        let keys = crate::mapurls::keys_from_settings();
        if let Some(kind) = provider_hash(&provider) {
            loop {
                let batch = open_cache().map(|cache| cache.pending(set, CONCURRENT_DOWNLOADS).unwrap_or_default()).unwrap_or_default();
                if batch.is_empty() || cancel.load(Ordering::Relaxed) {
                    break;
                }
                let fetched = futures_util::future::join_all(batch.iter().map(|(hash, x, y, z)| async { (hash.clone(), fetched(&provider, &keys, *x, *y, *z).await) })).await;
                let Ok(cache) = open_cache() else { break };
                fetched.iter().for_each(|(hash, image)| {
                    let stored = image.as_ref().and_then(|(format, image)| cache.complete(set, &Tile { hash: hash.clone(), format: format.clone(), image: image.clone(), kind }).ok());
                    if stored.is_none() {
                        let _ = cache.mark_error(set, hash);
                    }
                });
            }
        }
        DOWNLOADS.lock().unwrap_or_else(PoisonError::into_inner).remove(&set);
        log::info!("Offline map set {set} download stopped");
        save_to_device();
    }

    fn stored_path() -> Option<String> {
        crate::settingsstore::save_root().map(|root| format!("{root}/Tiles/qgcMapCache.db"))
    }

    pub fn save_to_device() {
        let (Some(path), true) = (stored_path(), crate::device::paired()) else { return };
        match Memory::new().export_db(crate::terrainservice::BROWSER_CACHE) {
            Ok(bytes) => {
                let _ = crate::vfs::write(&path, bytes);
            }
            Err(error) => log::warn!("Could not keep the offline maps on the device: {error}"),
        }
    }

    pub fn handed_to_files(path: &str) -> Result<(), String> {
        let memory = Memory::new();
        let bytes = memory.export_db(path).map_err(|error| error.to_string())?;
        memory.delete_db(path);
        crate::vfs::write(path, bytes).map_err(|error| error.to_string())
    }

    pub fn taken_from_files(path: &str) -> Result<(), String> {
        let bytes = crate::vfs::read(path).map_err(|_| "Error opening import database".to_string())?;
        let memory = Memory::new();
        memory.delete_db(path);
        memory.import_db(path, &bytes).map_err(|error| error.to_string())
    }

    pub fn restore_from_device() {
        let Some(url) = stored_path().and_then(|path| crate::device::files(&path)) else { return };
        wasm_bindgen_futures::spawn_local(async move {
            let Ok(bytes) = crate::http::get_later(&url, &[], std::time::Duration::from_secs(300)).await else { return };
            let memory = Memory::new();
            memory.delete_db(RESTORED);
            let merged = memory.import_db(RESTORED, &bytes).map_err(|error| error.to_string()).and_then(|()| open_cache()?.import(std::path::Path::new(RESTORED), false));
            memory.delete_db(RESTORED);
            match merged {
                Ok(()) => log::info!("Offline maps kept on the device are back in this browser"),
                Err(error) => log::warn!("Could not bring back the offline maps kept on the device: {error}"),
            }
        });
    }
}

fn argument(args: &Value, index: usize) -> Option<&Value> {
    args.get(index)
}

fn float(args: &Value, index: usize) -> Option<f64> {
    argument(args, index).and_then(|value| value.as_f64().or_else(|| value.as_str()?.parse().ok()))
}

pub fn start_download(args: &str) -> Value {
    let args: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let name = argument(&args, 0).and_then(Value::as_str).map(str::trim).unwrap_or_default().to_string();
    let provider = argument(&args, 1).and_then(Value::as_str).unwrap_or_default().to_string();
    let region = (|| Some(Region { top_left_lon: float(&args, 2)?, top_left_lat: float(&args, 3)?, bottom_right_lon: float(&args, 4)?, bottom_right_lat: float(&args, 5)? }))();
    let zooms = float(&args, 6).zip(float(&args, 7)).map(|(min, max)| (min as i32, max as i32));
    let fetch_elevation = argument(&args, 8).and_then(Value::as_bool).unwrap_or(true);
    let (Some(kind), Some(region), Some((min_zoom, max_zoom))) = (provider_hash(&provider), region, zooms) else {
        return json!({ "ok": false, "reason": "offlineMaps.startDownload takes a name, a map type, the region's top-left and bottom-right lon/lat and the zoom range" });
    };
    if name.is_empty() {
        return json!({ "ok": false, "reason": "The tile set needs a name." });
    }
    let (count, _) = estimate(&provider, &region, min_zoom, max_zoom);
    if count == 0 {
        return json!({ "ok": false, "reason": "No tiles to save" });
    }
    let created = open_cache().and_then(|cache| {
        if cache.sets().map_err(|error| error.to_string())?.iter().any(|set| set.name == name) {
            return Err("Tile set with this name already exists".to_string());
        }
        let set = TileSet {
            id: 0,
            name: name.clone(),
            type_str: provider.clone(),
            top_left: (region.top_left_lat, region.top_left_lon),
            bottom_right: (region.bottom_right_lat, region.bottom_right_lon),
            min_zoom,
            max_zoom,
            kind,
            tiles: count as i64,
            default_set: false,
        };
        cache.create_set(&set, &tiles(&region, min_zoom, max_zoom)).map_err(|error| error.to_string())
    });
    match created {
        Ok(id) => {
            log::info!("Offline map set \"{name}\" created with {count} tiles of {provider}");
            start(id, &provider);
            if fetch_elevation && !crate::maptypes::ELEVATION_PROVIDERS.contains(&provider.as_str()) {
                create_elevation_set(&name, &region);
                companions(max_zoom).into_iter().for_each(|companion| create_companion_set(&name, &region, companion));
            }
            json!({ "ok": true, "result": id })
        }
        Err(reason) => json!({ "ok": false, "reason": reason }),
    }
}

#[derive(Clone, Copy)]
struct Companion {
    provider: &'static str,
    label: &'static str,
    min_zoom: i32,
    max_zoom: i32,
}

fn companions(max_zoom: i32) -> Vec<Companion> {
    let terrain = Companion { provider: crate::maptypes::TERRAIN_TILES, label: "Terrain", min_zoom: 0, max_zoom: max_zoom.min(crate::maptypes::TERRAIN_TILES_MAX_ZOOM) };
    let buildings = Companion { provider: crate::maptypes::BUILDING_TILES, label: "Buildings", min_zoom: crate::maptypes::BUILDING_TILES_ZOOM, max_zoom: crate::maptypes::BUILDING_TILES_ZOOM };
    std::iter::once(terrain).chain((max_zoom >= crate::maptypes::BUILDING_TILES_ZOOM).then_some(buildings)).collect()
}

fn companions_estimate(region: &Region, max_zoom: i32) -> (u64, u64) {
    companions(max_zoom).iter().map(|companion| estimate(companion.provider, region, companion.min_zoom, companion.max_zoom)).fold((0, 0), |(count, size), (more, bigger)| (count + more, size + bigger))
}

fn create_companion_set(name: &str, region: &Region, companion: Companion) {
    let Some(kind) = provider_hash(companion.provider) else { return };
    let tiles = tiles(region, companion.min_zoom, companion.max_zoom);
    let set = TileSet {
        id: 0,
        name: format!("{name} {}", companion.label),
        type_str: companion.provider.to_string(),
        top_left: (region.top_left_lat, region.top_left_lon),
        bottom_right: (region.bottom_right_lat, region.bottom_right_lon),
        min_zoom: companion.min_zoom,
        max_zoom: companion.max_zoom,
        kind,
        tiles: tiles.len() as i64,
        default_set: false,
    };
    match open_cache().and_then(|cache| cache.create_set(&set, &tiles).map_err(|error| error.to_string())) {
        Ok(id) => start(id, companion.provider),
        Err(reason) => log::warn!("Offline {} set for \"{name}\" was not created: {reason}", companion.label.to_lowercase()),
    }
}

fn create_elevation_set(name: &str, region: &Region) {
    let provider = crate::maptypes::ELEVATION_PROVIDERS[0];
    let Some(kind) = provider_hash(provider) else { return };
    let tiles = crate::terrainquery::region_tiles((region.top_left_lat, region.top_left_lon), (region.bottom_right_lat, region.bottom_right_lon));
    let set = TileSet {
        id: 0,
        name: format!("{name} Elevation"),
        type_str: provider.to_string(),
        top_left: (region.top_left_lat, region.top_left_lon),
        bottom_right: (region.bottom_right_lat, region.bottom_right_lon),
        min_zoom: 1,
        max_zoom: 1,
        kind,
        tiles: tiles.len() as i64,
        default_set: false,
    };
    match open_cache().and_then(|cache| cache.create_set(&set, &tiles).map_err(|error| error.to_string())) {
        Ok(id) => start(id, provider),
        Err(reason) => log::warn!("Offline elevation set for \"{name}\" was not created: {reason}"),
    }
}

fn set_id(args: &str) -> Option<i64> {
    serde_json::from_str::<Value>(args).ok().and_then(|args| args.get(0).and_then(|id| id.as_i64().or_else(|| id.as_str()?.parse().ok())))
}

fn with_set(args: &str, act: impl FnOnce(&Cache, &TileSet) -> Result<(), String>) -> Value {
    let Some(id) = set_id(args) else { return json!({ "ok": false, "reason": "the tile set's id comes first" }) };
    let outcome = open_cache().and_then(|cache| {
        let set = cache.sets().map_err(|error| error.to_string())?.into_iter().find(|set| set.id == id).ok_or("No such tile set.")?;
        act(&cache, &set)
    });
    self::outcome(outcome)
}

fn cancel(set: i64) {
    if let Some(flag) = DOWNLOADS.lock().unwrap_or_else(PoisonError::into_inner).get(&set) {
        flag.store(true, Ordering::Relaxed);
    }
}

fn cancel_all() {
    DOWNLOADS.lock().unwrap_or_else(PoisonError::into_inner).keys().copied().collect::<Vec<_>>().into_iter().for_each(cancel);
}

fn outcome(result: Result<(), String>) -> Value {
    match result {
        Ok(()) => json!({ "ok": true }),
        Err(reason) => json!({ "ok": false, "reason": reason }),
    }
}

fn export_sets(args: &str) -> Value {
    let args: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let path = argument(&args, 0).and_then(Value::as_str).unwrap_or_default();
    let sets: Vec<i64> = args.as_array().map(|all| all.iter().skip(1).filter_map(Value::as_i64).collect()).unwrap_or_default();
    match (path.is_empty(), sets.is_empty()) {
        (true, _) => json!({ "ok": false, "reason": "offlineMaps.export takes the file to write and the ids of the sets to put in it" }),
        (false, true) => json!({ "ok": false, "reason": "Select at least one tile set to export." }),
        (false, false) => outcome(open_cache().and_then(|cache| cache.export(&sets, std::path::Path::new(path))).and_then(|()| handed_to_files(path))),
    }
}

#[cfg(not(target_family = "wasm"))]
fn handed_to_files(_path: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_family = "wasm"))]
fn taken_from_files(_path: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_family = "wasm")]
fn handed_to_files(path: &str) -> Result<(), String> {
    browser::handed_to_files(path)
}

#[cfg(target_family = "wasm")]
fn taken_from_files(path: &str) -> Result<(), String> {
    browser::taken_from_files(path)
}

fn import_sets(args: &str) -> Value {
    let args: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let path = argument(&args, 0).and_then(Value::as_str).unwrap_or_default();
    let replace = argument(&args, 1).and_then(Value::as_bool).unwrap_or(false);
    if path.is_empty() {
        return json!({ "ok": false, "reason": "offlineMaps.import takes the file to read and whether it replaces the existing sets" });
    }
    if replace {
        cancel_all();
    }
    outcome(taken_from_files(path).and_then(|()| open_cache()?.import(std::path::Path::new(path), replace)))
}

pub fn run(path: &str, args: &str) -> Value {
    let answer = act(path, args);
    #[cfg(target_family = "wasm")]
    if [DELETE_SET, RENAME_SET, IMPORT_SETS].contains(&path) && answer["ok"] == json!(true) {
        browser::save_to_device();
    }
    answer
}

fn act(path: &str, args: &str) -> Value {
    match path {
        EXPORT_SETS => export_sets(args),
        IMPORT_SETS => import_sets(args),
        START_DOWNLOAD => start_download(args),
        RESUME_DOWNLOAD => with_set(args, |cache, set| {
            cache.retry_errors(set.id).map_err(|error| error.to_string())?;
            start(set.id, &set.type_str);
            Ok(())
        }),
        CANCEL_DOWNLOAD => with_set(args, |_, set| {
            cancel(set.id);
            Ok(())
        }),
        DELETE_SET => with_set(args, |cache, set| match set.default_set {
            true => {
                cancel_all();
                cache.reset().map_err(|error| error.to_string())
            }
            false => {
                cancel(set.id);
                cache.delete_set(set.id).map_err(|error| error.to_string())
            }
        }),
        RENAME_SET => with_set(args, |cache, set| {
            if set.default_set {
                return Err("The system wide tile cache keeps its name.".to_string());
            }
            let name = serde_json::from_str::<Value>(args).ok().and_then(|args| args.get(1)?.as_str().map(str::trim).map(str::to_string)).unwrap_or_default();
            let taken: Vec<String> = cache.sets().map_err(|error| error.to_string())?.into_iter().filter(|other| other.id != set.id).map(|other| other.name).collect();
            match name.is_empty() {
                true => Err("The tile set needs a name.".to_string()),
                false => cache.rename_set(set.id, &free_name(&name, &taken)).map_err(|error| error.to_string()),
            }
        }),
        _ => json!({ "ok": false, "reason": format!("{path} is not an offline map action") }),
    }
}

pub fn owns(path: &str) -> bool {
    [START_DOWNLOAD, RESUME_DOWNLOAD, CANCEL_DOWNLOAD, DELETE_SET, RENAME_SET, EXPORT_SETS, IMPORT_SETS].contains(&path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tilecache::tile_hash;

    fn fixture() -> Value {
        let text = std::fs::read_to_string(format!("{}/../test/Bridge/fixtures/tile-providers.json", env!("CARGO_MANIFEST_DIR"))).expect("the fixture is recorded by QGCCoreCTest");
        serde_json::from_str(&text).expect("the fixture is JSON")
    }

    #[test]
    fn every_provider_estimates_with_the_average_size_qt_gives_it() {
        let recorded = fixture();
        let wrong: Vec<String> = recorded["averageSizes"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(name, size)| Some(average_size(name)) != size.as_u64())
            .map(|(name, size)| format!("{name}: qt {size} rust {}", average_size(name)))
            .collect();
        assert!(wrong.is_empty(), "{wrong:?}");
    }

    #[test]
    fn tile_counts_match_the_url_factory() {
        let recorded = fixture();
        let wrong: Vec<String> = recorded["tileCounts"]
            .as_object()
            .unwrap()
            .iter()
            .filter_map(|(key, expected)| {
                let (corners, zoom) = key.split_once(" z").unwrap();
                let c: Vec<f64> = corners.split(',').map(|v| v.parse().unwrap()).collect();
                let region = Region { top_left_lon: c[0], top_left_lat: c[1], bottom_right_lon: c[2], bottom_right_lat: c[3] };
                let ours = tile_count(&region, zoom.parse().unwrap());
                (Some(ours) != expected.as_u64()).then(|| format!("{key}: qt {expected} rust {ours}"))
            })
            .collect();
        assert!(wrong.is_empty(), "{wrong:?}");
        assert_eq!(recorded["tileCounts"].as_object().unwrap().len(), 60);
    }

    #[test]
    fn counts_group_and_names_are_unique() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1000), "1,000");
        assert_eq!(grouped(1234567), "1,234,567");
        assert_eq!(unique_name(&["Tile Set 001".into(), "Tile Set 002".into()]), "Tile Set 003");
    }

    #[test]
    fn a_set_links_what_is_cached_queues_the_rest_and_deletes_only_its_own_tiles() {
        let cache = Cache::open_in_memory().unwrap();
        let kind = provider_hash("Google Satellite").unwrap();
        let png = b"\x89PNG\r\n\x1a\n".to_vec();
        cache.save(&Tile { hash: tile_hash(kind, 1, 1, 2), format: "png".into(), image: png.clone(), kind }, None).unwrap();
        let set = |name: &str| TileSet { id: 0, name: name.into(), type_str: "Google Satellite".into(), top_left: (0.0, 0.0), bottom_right: (0.0, 0.0), min_zoom: 2, max_zoom: 2, kind, tiles: 2, default_set: false };
        let id = cache.create_set(&set("A"), &[(1, 1, 2), (2, 1, 2)]).unwrap();
        assert_eq!(cache.saved(id).unwrap().0, 1, "the cached tile is linked, not downloaded again");
        let pending = cache.pending(id, TILE_BATCH).unwrap();
        assert_eq!(pending, vec![(tile_hash(kind, 2, 1, 2), 2, 1, 2)]);

        cache.mark_error(id, &pending[0].0).unwrap();
        assert_eq!(cache.errors(id).unwrap(), 1);
        assert!(cache.pending(id, TILE_BATCH).unwrap().is_empty(), "an errored tile waits for Resume");
        cache.retry_errors(id).unwrap();
        cache.complete(id, &Tile { hash: pending[0].0.clone(), format: "png".into(), image: png.clone(), kind }).unwrap();
        let stored = cache.sets().unwrap().into_iter().find(|s| s.id == id).unwrap();
        let counted = totals(&cache, &stored).unwrap();
        assert_eq!((counted.saved_count, counted.errors, counted.unique_count), (2, 0, 1), "the shared tile is not unique to the set");

        cache.delete_set(id).unwrap();
        assert!(cache.tile(&tile_hash(kind, 2, 1, 2)).unwrap().is_none(), "the set's own tile goes with it");
        assert!(cache.tile(&tile_hash(kind, 1, 1, 2)).unwrap().is_some(), "the default cache keeps its tile");

        cache.create_set(&set("B"), &[(3, 1, 2)]).unwrap();
        cache.reset().unwrap();
        let left = cache.sets().unwrap();
        assert_eq!((left.len(), left[0].default_set, cache.count().unwrap()), (1, true, 0), "deleting the system wide cache empties it and keeps only its own row");
    }

    #[test]
    fn exported_sets_import_beside_what_is_there_or_in_place_of_it() {
        let folder = std::env::temp_dir().join(format!("qgc-tile-export-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let (ours, file) = (folder.join("ours.db"), folder.join("export.db"));
        let _ = std::fs::remove_file(&ours);
        let kind = provider_hash("Google Satellite").unwrap();
        let png = b"\x89PNG\r\n\x1a\n".to_vec();
        let tile = |x: i32| Tile { hash: tile_hash(kind, x, 1, 2), format: "png".into(), image: png.clone(), kind };
        let set = |name: &str| TileSet { id: 0, name: name.into(), type_str: "Google Satellite".into(), top_left: (1.0, 2.0), bottom_right: (0.5, 2.5), min_zoom: 2, max_zoom: 2, kind, tiles: 2, default_set: false };
        let cache = Cache::open(&ours).unwrap();
        let fields = cache.create_set(&set("Fields"), &[(1, 1, 2), (2, 1, 2)]).unwrap();
        [1, 2].iter().for_each(|x| cache.complete(fields, &tile(*x)).unwrap());
        let empty = cache.create_set(&set("Empty"), &[(7, 1, 2)]).unwrap();
        cache.save(&tile(9), None).unwrap();

        assert_eq!(cache.export(&[fields], &ours), Err("Export path must differ from the active database".into()));
        cache.export(&[fields, empty], &file).unwrap();
        let exported = Cache::open(&file).unwrap();
        let names: Vec<String> = exported.sets().unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(names, ["Default Tile Set", "Empty", "Fields"], "the export carries only the chosen sets beside its own default");
        assert_eq!(exported.count().unwrap(), 2, "the default cache's tile stays home");
        drop(exported);

        cache.import(&file, false).unwrap();
        let after: Vec<(String, i64)> = cache.sets().unwrap().into_iter().map(|s| (s.name, s.tiles)).collect();
        assert_eq!(after, [("Default Tile Set".into(), 0), ("Empty".into(), 2), ("Fields".into(), 2), ("Fields 0001".into(), 2)], "a clashing name is numbered and a set with no tiles is not imported");
        assert_eq!(cache.count().unwrap(), 3, "tiles already cached are linked, not duplicated");

        cache.import(&file, true).unwrap();
        let replaced: Vec<String> = cache.sets().unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(replaced, ["Default Tile Set", "Fields"]);
        assert_eq!(cache.count().unwrap(), 2, "replacing drops what the file does not hold");
        assert_eq!(cache.import(&folder.join("missing.db"), false), Err("Error opening import database".into()));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_default_cache_is_trimmed_to_the_disk_limit() {
        let cache = Cache::open_in_memory().unwrap();
        let kind = provider_hash("Google Satellite").unwrap();
        (0..4).for_each(|x| {
            cache.save(&Tile { hash: tile_hash(kind, x, 0, 3), format: "png".into(), image: vec![0; 1000], kind }, None).unwrap();
        });
        assert_eq!(cache.trim_to(4000).unwrap(), 4000, "a cache within its limit is left alone and its size answered");
        assert_eq!(cache.trim_to(2500).unwrap(), 2000, "two thousand-byte tiles go to pay a debt of 1500, leaving 2000");
        assert_eq!(cache.count().unwrap(), 2);
    }

    #[test]
    fn a_cancelled_download_stops_at_the_next_tile_instead_of_finishing_the_batch() {
        let batch: Vec<usize> = (0..60).collect();
        let (cancel, started) = (AtomicBool::new(false), std::sync::atomic::AtomicUsize::new(0));
        let fetched = fetched_until_cancelled(&batch, &cancel, |tile| {
            if started.fetch_add(1, Ordering::SeqCst) + 1 == 3 {
                cancel.store(true, Ordering::SeqCst);
            }
            *tile
        });
        assert!(started.load(Ordering::SeqCst) <= 3 + CONCURRENT_DOWNLOADS - 1, "each worker finishes at most the tile it holds; {} were fetched", started.load(Ordering::SeqCst));
        assert_eq!(fetched.len(), started.load(Ordering::SeqCst), "tiles never fetched stay pending, so a later start picks them up");
        assert_eq!(fetched_until_cancelled(&batch, &AtomicBool::new(false), |tile| *tile), batch);
    }

    #[test]
    fn a_region_expands_to_every_tile_of_every_zoom() {
        let region = Region { top_left_lon: 8.50, top_left_lat: 47.40, bottom_right_lon: 8.60, bottom_right_lat: 47.35 };
        let listed = tiles(&region, 10, 12);
        assert_eq!(listed.len() as u64, estimate("Google Satellite", &region, 10, 12).0);
        assert_eq!(estimate("Google Satellite", &region, 10, 12).1, listed.len() as u64 * 56887);
    }

    #[test]
    fn terrain_for_the_synthetic_view_is_saved_from_zoom_0_up_to_the_finest_it_has() {
        let region = Region { top_left_lon: 8.50, top_left_lat: 47.40, bottom_right_lon: 8.60, bottom_right_lat: 47.35 };
        let terrain = |max_zoom: i32| companions(max_zoom)[0];
        assert_eq!((terrain(18).min_zoom, terrain(18).max_zoom), (0, crate::maptypes::TERRAIN_TILES_MAX_ZOOM), "terrain stops at zoom 15, where the elevation tiles end");
        assert_eq!(terrain(12).max_zoom, 12, "a coarser map set only needs terrain as fine as itself");
        assert_eq!(companions_estimate(&region, 12).0, tiles(&region, 0, 12).len() as u64);
    }

    #[test]
    fn buildings_for_the_synthetic_view_are_saved_at_the_one_zoom_that_carries_them() {
        let region = Region { top_left_lon: 44.70, top_left_lat: 41.80, bottom_right_lon: 44.90, bottom_right_lat: 41.65 };
        let buildings = companions(18).into_iter().find(|companion| companion.provider == crate::maptypes::BUILDING_TILES).expect("a fine map set brings buildings");
        assert_eq!((buildings.min_zoom, buildings.max_zoom, buildings.label), (14, 14, "Buildings"), "OpenFreeMap carries buildings only at zoom 13 and 14, and 14 holds every one");
        assert!(companions(13).iter().all(|companion| companion.provider != crate::maptypes::BUILDING_TILES), "a coarser map set leaves buildings out like it leaves out fine terrain");
        let terrain = estimate(crate::maptypes::TERRAIN_TILES, &region, 0, 15);
        let building = estimate(crate::maptypes::BUILDING_TILES, &region, 14, 14);
        assert_eq!(companions_estimate(&region, 18), (terrain.0 + building.0, terrain.1 + building.1), "the size shown before a download counts the buildings too");
    }
}
