use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex, PoisonError};

use crate::terraintile::Tile;

pub const TERRAIN_CHANGED: &str = "core.terrain@changed";
pub const FAILED_BACKOFF_MS: u64 = 5000;

type Key = (i32, i32);
type Notify = Box<dyn Fn() + Send + Sync>;

#[derive(Debug)]
enum Slot {
    Pending,
    Ready(Tile),
    Failed(u64),
}

#[derive(Debug, Default)]
pub struct TileTable {
    slots: BTreeMap<Key, Slot>,
}

#[derive(Debug, PartialEq)]
pub enum Lookup {
    Known(Option<f64>),
    Waiting,
    Fetch(Key),
}

impl TileTable {
    pub fn lookup(&mut self, latitude: f64, longitude: f64, now_ms: u64) -> Lookup {
        let key = crate::terrainquery::tile_xy(latitude, longitude);
        match self.slots.get(&key) {
            Some(Slot::Ready(tile)) => Lookup::Known(tile.elevation(latitude, longitude)),
            Some(Slot::Pending) => Lookup::Waiting,
            Some(Slot::Failed(at)) if now_ms.saturating_sub(*at) < FAILED_BACKOFF_MS => Lookup::Known(None),
            _ => {
                self.slots.insert(key, Slot::Pending);
                Lookup::Fetch(key)
            }
        }
    }

    pub fn store(&mut self, key: Key, fetched: Result<Tile, String>, now_ms: u64) {
        self.slots.insert(key, fetched.map_or(Slot::Failed(now_ms), Slot::Ready));
    }
}

static TABLE: LazyLock<Mutex<TileTable>> = LazyLock::new(|| Mutex::new(TileTable::default()));
static NOTIFY: Mutex<Option<Notify>> = Mutex::new(None);
static CACHE_PATH: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);

pub fn set_cache_path(path: Option<std::path::PathBuf>) {
    *CACHE_PATH.lock().unwrap_or_else(PoisonError::into_inner) = path;
}

pub fn load(key: Key, cache_path: Option<&std::path::Path>, fetch: &dyn Fn(&str) -> Result<String, String>) -> Result<Tile, String> {
    let cache = cache_path.and_then(|path| crate::tilecache::Cache::open(path).ok());
    crate::terrainquery::load_tile(key.0, key.1, cache.as_ref(), fetch)
}

pub fn set_notify(notify: Notify) {
    *NOTIFY.lock().unwrap_or_else(PoisonError::into_inner) = Some(notify);
}

fn fetch_in_background(key: Key) {
    std::thread::spawn(move || {
        let path = CACHE_PATH.lock().unwrap_or_else(PoisonError::into_inner).clone();
        let fetched = load(key, path.as_deref(), &crate::terrainquery::fetch_over_http);
        TABLE.lock().unwrap_or_else(PoisonError::into_inner).store(key, fetched, crate::hub::now_ms());
        if let Some(notify) = NOTIFY.lock().unwrap_or_else(PoisonError::into_inner).as_ref() {
            notify();
        }
    });
}

pub fn height(latitude: f64, longitude: f64) -> Option<f64> {
    let lookup = TABLE.lock().unwrap_or_else(PoisonError::into_inner).lookup(latitude, longitude, crate::hub::now_ms());
    match lookup {
        Lookup::Known(height) => height,
        Lookup::Waiting => None,
        Lookup::Fetch(key) => {
            fetch_in_background(key);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(height: i16) -> Tile {
        let (x, y) = crate::terrainquery::tile_xy(47.0, 8.0);
        let (sw_lon, sw_lat) = (f64::from(x) * 0.01 - 180.0, f64::from(y) * 0.01 - 90.0);
        Tile { sw_lat, sw_lon, ne_lat: sw_lat + 0.01, ne_lon: sw_lon + 0.01, min_elevation: height, max_elevation: height, avg_elevation: f64::from(height), grid_lat: 2, grid_lon: 2, elevations: vec![height; 4] }
    }

    #[test]
    fn a_tile_fetched_once_is_read_back_from_the_map_cache_file_without_the_network() {
        let path = std::env::temp_dir().join(format!("terrainservice-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let key = crate::terrainquery::tile_xy(47.6334, -122.0907);
        let online = |_: &str| Ok(include_str!("../tests/fixtures/copernicus-sectiontest.json").to_string());
        let offline = |_: &str| Err("offline".to_string());
        assert!(load(key, Some(&path), &offline).is_err(), "nothing cached and no network is no tile");
        assert_eq!(load(key, Some(&path), &online).map(|t| t.elevation(47.6334, -122.0907)), Ok(Some(35.0)));
        assert_eq!(load(key, Some(&path), &offline).map(|t| t.elevation(47.6334, -122.0907)), Ok(Some(35.0)), "the tile QGC's map cache holds answers with the network gone");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_tile_is_fetched_once_answers_every_point_in_it_and_a_failure_backs_off() {
        let mut table = TileTable::default();
        let key = crate::terrainquery::tile_xy(47.0001, 8.0001);
        assert_eq!(table.lookup(47.0001, 8.0001, 0), Lookup::Fetch(key));
        assert_eq!(table.lookup(47.0002, 8.0002, 1), Lookup::Waiting, "a second point in a tile already being fetched does not fetch it again");
        table.store(key, Ok(flat(412)), 2);
        assert_eq!(table.lookup(47.0003, 8.0003, 3), Lookup::Known(Some(412.0)));
        let far = crate::terrainquery::tile_xy(10.0, 10.0);
        assert_eq!(table.lookup(10.0, 10.0, 0), Lookup::Fetch(far));
        table.store(far, Err("offline".to_string()), 100);
        assert_eq!(table.lookup(10.0, 10.0, 100 + FAILED_BACKOFF_MS - 1), Lookup::Known(None), "TerrainTileManager does not hammer a server that just failed");
        assert_eq!(table.lookup(10.0, 10.0, 100 + FAILED_BACKOFF_MS), Lookup::Fetch(far));
    }
}
