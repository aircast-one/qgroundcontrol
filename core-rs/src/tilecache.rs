use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

pub const PROVIDERS: [(&str, i32); 37] = [
    ("Bing Hybrid", 308415137),
    ("Bing Road", -201109620),
    ("Bing Satellite", -636113614),
    ("Copernicus", 1270231390),
    ("CustomURL Custom", 641461492),
    ("Eniro Topo", -1719639304),
    ("Esri Terrain", 1177283281),
    ("Esri World Satellite", 584748914),
    ("Esri World Street", 324741496),
    ("Google Hybrid", -180572015),
    ("Google Labels", -459453944),
    ("Google Satellite", -688114322),
    ("Google Street Map", -649361553),
    ("Google Terrain", 1651207517),
    ("Japan-GSI Anaglyph", -1397826394),
    ("Japan-GSI Contour", 134850610),
    ("Japan-GSI Relief", -577685280),
    ("Japan-GSI Seamless", 1266637725),
    ("Japan-GSI Slope", 1550280749),
    ("LINZ Basemap", -921597794),
    ("MapQuest Map", 245015981),
    ("MapQuest Sat", 1797877876),
    ("Mapbox Bright", 2043432917),
    ("Mapbox Custom", 1974515743),
    ("Mapbox Dark", -1238823924),
    ("Mapbox Hybrid", -257067776),
    ("Mapbox Light", 2017851366),
    ("Mapbox Outdoors", 441564750),
    ("Mapbox Satellite", 1543086237),
    ("Mapbox Streets", 687470815),
    ("Mapbox StreetsBasic", -770441679),
    ("Statkart Basemap", 1789023079),
    ("Statkart Topo", 29830293),
    ("Street Map", -1436319509),
    ("Svalbard Topo", 1502309240),
    ("VWorld Satellite Map", 557265812),
    ("VWorld Street Map", 1897091046),
];

pub const DEFAULT_SET_NAME: &str = "Default Tile Set";

#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    pub hash: String,
    pub format: String,
    pub image: Vec<u8>,
    pub kind: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileSet {
    pub id: i64,
    pub name: String,
    pub type_str: String,
    pub top_left: (f64, f64),
    pub bottom_right: (f64, f64),
    pub min_zoom: i32,
    pub max_zoom: i32,
    pub kind: i32,
    pub tiles: i64,
    pub default_set: bool,
}

pub fn provider_hash(name: &str) -> Option<i32> {
    PROVIDERS.iter().find(|(known, _)| *known == name).map(|(_, hash)| *hash)
}

pub fn provider_named(hash: i32) -> Option<&'static str> {
    PROVIDERS.iter().find(|(_, known)| *known == hash).map(|(name, _)| *name)
}

pub fn tile_hash(provider: i32, x: i32, y: i32, z: i32) -> String {
    format!("{provider:010}{x:08}{y:08}{z:03}")
}

pub fn provider_of(hash: &str) -> Option<i32> {
    hash.get(..10).and_then(|head| head.trim_start_matches('0').parse::<i32>().ok().or_else(|| head.parse::<i32>().ok()))
}

pub struct Cache {
    connection: Connection,
}

const SCHEMA: [&str; 5] = [
    "CREATE TABLE IF NOT EXISTS Tiles (tileID INTEGER PRIMARY KEY NOT NULL, hash TEXT NOT NULL UNIQUE, format TEXT NOT NULL, tile BLOB NULL, size INTEGER, type INTEGER, date INTEGER DEFAULT 0)",
    "CREATE INDEX IF NOT EXISTS hash ON Tiles ( hash, size, type ) ",
    "CREATE TABLE IF NOT EXISTS TileSets (setID INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL UNIQUE, typeStr TEXT, topleftLat REAL DEFAULT 0.0, topleftLon REAL DEFAULT 0.0, bottomRightLat REAL DEFAULT 0.0, bottomRightLon REAL DEFAULT 0.0, minZoom INTEGER DEFAULT 3, maxZoom INTEGER DEFAULT 3, type INTEGER DEFAULT -1, numTiles INTEGER DEFAULT 0, defaultSet INTEGER DEFAULT 0, date INTEGER DEFAULT 0)",
    "CREATE TABLE IF NOT EXISTS SetTiles (setID INTEGER, tileID INTEGER)",
    "CREATE TABLE IF NOT EXISTS TilesDownload (setID INTEGER, hash TEXT NOT NULL UNIQUE, type INTEGER, x INTEGER, y INTEGER, z INTEGER, state INTEGER DEFAULT 0)",
];

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|since| since.as_secs() as i64).unwrap_or(0)
}

impl Cache {
    pub fn open(path: &Path) -> rusqlite::Result<Cache> {
        let cache = Cache { connection: Connection::open(path)? };
        cache.prepare()?;
        Ok(cache)
    }

    pub fn open_in_memory() -> rusqlite::Result<Cache> {
        let cache = Cache { connection: Connection::open_in_memory()? };
        cache.prepare()?;
        Ok(cache)
    }

    fn prepare(&self) -> rusqlite::Result<()> {
        SCHEMA.iter().try_for_each(|statement| self.connection.execute_batch(statement))?;
        let existing: Option<i64> = self
            .connection
            .query_row("SELECT setID FROM TileSets WHERE name = ?1", params![DEFAULT_SET_NAME], |row| row.get(0))
            .optional()?;
        match existing {
            Some(_) => Ok(()),
            None => self
                .connection
                .execute("INSERT INTO TileSets(name, defaultSet, date) VALUES(?1, 1, ?2)", params![DEFAULT_SET_NAME, now_secs()])
                .map(|_| ()),
        }
    }

    pub fn default_set(&self) -> i64 {
        self.connection.query_row("SELECT setID FROM TileSets WHERE defaultSet = 1", [], |row| row.get(0)).unwrap_or(1)
    }

    pub fn tile(&self, hash: &str) -> Option<Tile> {
        self.connection
            .query_row("SELECT tile, format, type FROM Tiles WHERE hash = ?1", params![hash], |row| {
                Ok(Tile { hash: hash.to_string(), image: row.get(0)?, format: row.get(1)?, kind: row.get(2)? })
            })
            .optional()
            .ok()
            .flatten()
    }

    pub fn save(&self, tile: &Tile, set: Option<i64>) -> rusqlite::Result<bool> {
        let inserted = self.connection.execute(
            "INSERT OR IGNORE INTO Tiles(hash, format, tile, size, type, date) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![tile.hash, tile.format, tile.image, tile.image.len() as i64, tile.kind, now_secs()],
        )?;
        if inserted == 0 {
            return Ok(false);
        }
        let id = self.connection.last_insert_rowid();
        self.connection.execute("INSERT INTO SetTiles(tileID, setID) VALUES(?1, ?2)", params![id, set.unwrap_or_else(|| self.default_set())])?;
        Ok(true)
    }

    pub fn sets(&self) -> rusqlite::Result<Vec<TileSet>> {
        let mut statement = self.connection.prepare(
            "SELECT setID, name, typeStr, topleftLat, topleftLon, bottomRightLat, bottomRightLon, minZoom, maxZoom, type, numTiles, defaultSet FROM TileSets ORDER BY defaultSet DESC, name ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(TileSet {
                id: row.get(0)?,
                name: row.get(1)?,
                type_str: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                top_left: (row.get(3)?, row.get(4)?),
                bottom_right: (row.get(5)?, row.get(6)?),
                min_zoom: row.get(7)?,
                max_zoom: row.get(8)?,
                kind: row.get(9)?,
                tiles: row.get(10)?,
                default_set: row.get::<_, i64>(11)? != 0,
            })
        })?;
        rows.collect()
    }

    pub fn total_size(&self) -> i64 {
        self.connection.query_row("SELECT SUM(size) FROM Tiles", [], |row| row.get::<_, Option<i64>>(0)).ok().flatten().unwrap_or(0)
    }

    pub fn count(&self) -> i64 {
        self.connection.query_row("SELECT COUNT(*) FROM Tiles", [], |row| row.get(0)).unwrap_or(0)
    }

    pub fn prune(&self, keep_bytes: i64) -> rusqlite::Result<i64> {
        let over = self.total_size() - keep_bytes;
        if over <= 0 {
            return Ok(0);
        }
        let mut statement = self.connection.prepare("SELECT tileID, size FROM Tiles ORDER BY date ASC")?;
        let aged: Vec<(i64, i64)> = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
        let doomed: Vec<i64> = aged
            .iter()
            .scan(0i64, |freed, (id, size)| {
                let before = *freed;
                *freed += size;
                Some((*id, before))
            })
            .take_while(|(_, before)| *before < over)
            .map(|(id, _)| id)
            .collect();
        doomed.iter().try_for_each(|id| {
            self.connection.execute("DELETE FROM SetTiles WHERE tileID = ?1", params![id])?;
            self.connection.execute("DELETE FROM Tiles WHERE tileID = ?1", params![id]).map(|_| ())
        })?;
        Ok(doomed.len() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture(name: &str) -> Value {
        let text = std::fs::read_to_string(format!("../test/Bridge/fixtures/{name}")).expect("the fixture is recorded by QGCCoreCTest");
        serde_json::from_str(&text).expect("the fixture is JSON")
    }

    #[test]
    fn every_provider_qt_knows_has_a_hash_the_cache_can_key_on() {
        let recorded = fixture("tile-providers.json");
        let providers = recorded["providers"].as_object().unwrap();
        let missing: Vec<&String> = providers.keys().filter(|name| provider_hash(name).is_none()).collect();
        assert!(missing.is_empty(), "Qt serves map providers the core cannot key, so their tiles would be re-downloaded: {missing:?}");
        let wrong: Vec<String> = providers
            .iter()
            .filter(|(name, hash)| provider_hash(name) != hash.as_i64().map(|value| value as i32))
            .map(|(name, hash)| format!("{name} qt {hash} rust {:?}", provider_hash(name)))
            .collect();
        assert!(wrong.is_empty(), "{wrong:?}");
        assert_eq!(PROVIDERS.len(), providers.len(), "the core carries a provider Qt no longer serves, or the recording is stale");
    }

    #[test]
    fn every_recorded_tile_hash_is_spelled_identically() {
        let recorded = fixture("tile-providers.json");
        let samples = recorded["tileHashes"].as_object().unwrap();
        let wrong: Vec<String> = samples
            .iter()
            .map(|(key, expected)| {
                let (name, tile) = key.rsplit_once(' ').unwrap();
                let numbers: Vec<i32> = tile.split('/').map(|value| value.parse().unwrap()).collect();
                let ours = tile_hash(provider_hash(name).unwrap(), numbers[0], numbers[1], numbers[2]);
                (key.clone(), ours, expected.as_str().unwrap().to_string())
            })
            .filter(|(_, ours, theirs)| ours != theirs)
            .map(|(key, ours, theirs)| format!("{key}\n  qt:   {theirs}\n  rust: {ours}"))
            .collect();
        assert!(wrong.is_empty(), "{} of {} tile hashes differ from Qt, and a differing hash is a tile downloaded again:\n{}", wrong.len(), samples.len(), wrong.join("\n"));
        assert_eq!(samples.len(), 148);
    }

    #[test]
    fn a_negative_provider_hash_still_names_its_provider() {
        let bing = provider_hash("Bing Road").unwrap();
        assert!(bing < 0, "provider hashes are signed and this one is negative, which is what makes the padding worth a test");
        let hash = tile_hash(bing, 1, 2, 3);
        assert_eq!(hash.len(), 29, "the sign takes the place of a padding zero rather than adding a character");
        assert_eq!(provider_of(&hash), Some(bing));
        assert_eq!(provider_named(bing), Some("Bing Road"));

        let padded = provider_hash("Statkart Topo").unwrap();
        assert!(padded > 0 && padded < 100_000_000, "this provider hashes short enough that the format has to pad it, which is the other half of the padding rule");
        assert!(tile_hash(padded, 1, 2, 3).starts_with('0'));
        assert_eq!(tile_hash(padded, 1, 2, 3).len(), 29);
        assert_eq!(provider_of(&tile_hash(padded, 1, 2, 3)), Some(padded));

        assert_eq!(provider_hash("Esri World Street Map"), None, "a name close to a real provider is not a provider; the recorded list is the only authority on which names exist");
    }

    #[test]
    fn the_schema_is_the_one_the_qt_engine_writes() {
        let recorded = fixture("tile-cache-schema.json");
        let cache = Cache::open_in_memory().unwrap();
        let mut statement = cache.connection.prepare("SELECT type, name, sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY name").unwrap();
        let ours: Vec<(String, String)> = statement
            .query_map([], |row| Ok((row.get::<_, String>(1)?, format!("{} {}", row.get::<_, String>(0)?, row.get::<_, String>(2)?))))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let wrong: Vec<String> = ours
            .iter()
            .filter(|(name, sql)| recorded[name].as_str() != Some(sql.as_str()))
            .map(|(name, sql)| format!("{name}\n  qt:   {}\n  rust: {sql}", recorded[name].as_str().unwrap_or("(absent)")))
            .collect();
        assert!(wrong.is_empty(), "the Rust cache writes a schema Qt would not recognise:\n{}", wrong.join("\n"));
        assert_eq!(ours.len(), recorded.as_object().unwrap().len() - 1, "every table and index Qt creates is created here too");
    }

    #[test]
    fn a_cache_opens_with_the_default_set_qt_expects() {
        let cache = Cache::open_in_memory().unwrap();
        let sets = cache.sets().unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].name, DEFAULT_SET_NAME);
        assert!(sets[0].default_set);
        assert_eq!(cache.default_set(), sets[0].id);
    }

    fn tile(hash: &str, bytes: usize) -> Tile {
        Tile { hash: hash.to_string(), format: "png".to_string(), image: vec![7u8; bytes], kind: 3 }
    }

    #[test]
    fn a_saved_tile_comes_back_whole() {
        let cache = Cache::open_in_memory().unwrap();
        let hash = tile_hash(provider_hash("Bing Road").unwrap(), 8523, 5606, 14);
        assert!(cache.tile(&hash).is_none(), "an empty cache serves nothing rather than an empty tile");
        assert!(cache.save(&tile(&hash, 2048), None).unwrap());
        let served = cache.tile(&hash).unwrap();
        assert_eq!(served.image.len(), 2048);
        assert_eq!(served.format, "png");
        assert_eq!(served.kind, 3);
        assert_eq!(cache.total_size(), 2048);
    }

    #[test]
    fn the_same_tile_arriving_twice_is_stored_once() {
        let cache = Cache::open_in_memory().unwrap();
        let hash = tile_hash(provider_hash("Bing Road").unwrap(), 1, 1, 1);
        assert!(cache.save(&tile(&hash, 100), None).unwrap());
        assert!(!cache.save(&tile(&hash, 100), None).unwrap(), "QtLocation asks for the same tile twice in a row, and the second arrival is not an error");
        assert_eq!(cache.count(), 1);
    }

    #[test]
    fn pruning_drops_the_oldest_tiles_until_the_cache_fits() {
        let cache = Cache::open_in_memory().unwrap();
        let hashes: Vec<String> = (0..5).map(|index| tile_hash(provider_hash("Bing Road").unwrap(), index, 0, 10)).collect();
        hashes.iter().enumerate().for_each(|(index, hash)| {
            cache.save(&tile(hash, 1000), None).unwrap();
            cache.connection.execute("UPDATE Tiles SET date = ?1 WHERE hash = ?2", params![index as i64, hash]).unwrap();
        });
        assert_eq!(cache.total_size(), 5000);
        assert_eq!(cache.prune(5000).unwrap(), 0, "a cache already within its limit loses nothing");
        assert_eq!(cache.prune(2500).unwrap(), 3, "three of five thousand-byte tiles go to bring five thousand under two and a half");
        assert!(cache.tile(&hashes[0]).is_none(), "the oldest tile is the first to go");
        assert!(cache.tile(&hashes[4]).is_some(), "the newest tile stays");
        assert_eq!(cache.connection.query_row("SELECT COUNT(*) FROM SetTiles", [], |row| row.get::<_, i64>(0)).unwrap(), 2, "a pruned tile leaves no row behind in its set");
    }
}
