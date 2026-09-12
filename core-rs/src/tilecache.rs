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
    pub kind: String,
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

const TILE_DIGITS: usize = 19;

pub fn provider_of(hash: &str) -> Option<i32> {
    hash.len().checked_sub(TILE_DIGITS).and_then(|head| hash.get(..head)).and_then(|head| head.parse::<i32>().ok())
}

// The database uses a rollback journal, not WAL. When a read-only connection meets a journal the
// Qt worker has open mid-transaction, SQLite has to roll it back before it can present a
// consistent database - and rolling back is a write, which a read-only connection cannot do. It
// answers SQLITE_READONLY, "attempt to write a readonly database", for a plain SELECT.
//
// busy_timeout does not cover this: it waits on a lock, and this is not a lock. The window is one
// write transaction wide, so a bounded retry closes it without giving up read-only, which is the
// property that keeps this connection unable to disturb the worker.
const READONLY_RETRIES: u64 = 5;
const RETRY_PAUSE_MS: u64 = 20;

fn recovering_a_journal(error: &rusqlite::Error) -> bool {
    matches!(error.sqlite_error_code(), Some(rusqlite::ErrorCode::ReadOnly))
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

fn uri_escaped(path: &Path) -> String {
    path.to_string_lossy().chars().map(|c| if c == '?' || c == '#' { format!("%{:02X}", c as u8) } else { c.to_string() }).collect()
}

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|since| since.as_secs() as i64).unwrap_or(0)
}

impl Cache {
    pub fn open(path: &Path) -> rusqlite::Result<Cache> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        let cache = Cache { connection };
        cache.prepare()?;
        Ok(cache)
    }

    /// Opens a database another writer owns. No schema is created and nothing is written, so this
    /// takes no write lock and cannot collide with the Qt worker holding the same file.
    pub fn serve(path: &Path) -> rusqlite::Result<Cache> {
        let connection = Connection::open_with_flags(
            &format!("file:{}?mode=ro&cache=private", uri_escaped(path)),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(Cache { connection })
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

    pub fn default_set(&self) -> rusqlite::Result<i64> {
        self.connection.query_row("SELECT setID FROM TileSets WHERE defaultSet = 1", [], |row| row.get(0))
    }

    pub fn tile(&self, hash: &str) -> rusqlite::Result<Option<Tile>> {
        (0..READONLY_RETRIES).find_map(|attempt| match self.read_tile(hash) {
            Err(error) if recovering_a_journal(&error) => {
                std::thread::sleep(std::time::Duration::from_millis(RETRY_PAUSE_MS * (attempt + 1)));
                None
            }
            answer => Some(answer),
        })
        .unwrap_or_else(|| self.read_tile(hash))
    }

    fn read_tile(&self, hash: &str) -> rusqlite::Result<Option<Tile>> {
        self.connection
            .query_row("SELECT tile, format, type FROM Tiles WHERE hash = ?1", params![hash], |row| {
                Ok(Tile { hash: hash.to_string(), image: row.get(0)?, format: row.get(1)?, kind: row.get::<_, Option<String>>(2)?.unwrap_or_default() })
            })
            .optional()
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
        let set = match set {
            Some(set) => set,
            None => self.default_set()?,
        };
        self.connection.execute("INSERT INTO SetTiles(tileID, setID) VALUES(?1, ?2)", params![id, set])?;
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

    pub fn total_size(&self) -> rusqlite::Result<i64> {
        self.connection.query_row("SELECT SUM(size) FROM Tiles", [], |row| row.get::<_, Option<i64>>(0)).map(|total| total.unwrap_or(0))
    }

    pub fn count(&self) -> rusqlite::Result<i64> {
        self.connection.query_row("SELECT COUNT(*) FROM Tiles", [], |row| row.get(0))
    }

    pub fn prune(&self, free_bytes: i64) -> rusqlite::Result<i64> {
        let mut statement = self.connection.prepare(
            "SELECT tileID, size FROM Tiles WHERE tileID IN (SELECT A.tileID FROM SetTiles A join SetTiles B on A.tileID = B.tileID WHERE B.setID = ?1 GROUP by A.tileID HAVING COUNT(A.tileID) = 1) ORDER BY date ASC LIMIT 128",
        )?;
        let aged: Vec<(i64, i64)> = statement.query_map(params![self.default_set()?], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
        let doomed: Vec<i64> = aged
            .iter()
            .scan(free_bytes, |remaining, (id, size)| {
                let owed = *remaining;
                *remaining -= size;
                Some((*id, owed))
            })
            .take_while(|(_, owed)| *owed >= 0)
            .map(|(id, _)| id)
            .collect();
        doomed.iter().try_for_each(|id| {
            self.connection.execute("DELETE FROM SetTiles WHERE tileID = ?1", params![id])?;
            self.connection.execute("DELETE FROM Tiles WHERE tileID = ?1", params![id]).map(|_| ())
        })?;
        Ok(doomed.len() as i64)
    }

    pub fn saved(&self, set: i64) -> rusqlite::Result<(i64, i64)> {
        self.connection.query_row(
            "SELECT COUNT(size), SUM(size) FROM Tiles A INNER JOIN SetTiles B on A.tileID = B.tileID WHERE B.setID = ?1",
            params![set],
            |row| Ok((row.get(0)?, row.get::<_, Option<i64>>(1)?.unwrap_or(0))),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::recovering_a_journal;

    #[test]
    fn only_a_readonly_refusal_is_worth_retrying() {
        let readonly = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error { code: rusqlite::ErrorCode::ReadOnly, extended_code: 8 },
            Some("attempt to write a readonly database".to_string()),
        );
        assert!(recovering_a_journal(&readonly), "this is the error a hot journal produces, and the whole retry hangs off recognising it");

        let busy = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error { code: rusqlite::ErrorCode::DatabaseBusy, extended_code: 5 },
            Some("database is locked".to_string()),
        );
        assert!(!recovering_a_journal(&busy), "busy_timeout already waits on a lock, and retrying it here would double the wait");

        let corrupt = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error { code: rusqlite::ErrorCode::DatabaseCorrupt, extended_code: 11 },
            Some("database disk image is malformed".to_string()),
        );
        assert!(!recovering_a_journal(&corrupt), "a broken database does not get better by asking again");
    }

    use super::*;
    use serde_json::Value;

    fn fixture(name: &str) -> Value {
        let text = std::fs::read_to_string(format!("{}/../test/Bridge/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).expect("the fixture is recorded by QGCCoreCTest");
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
        assert_eq!(samples.len(), recorded["providers"].as_object().unwrap().len() * 4, "every provider is sampled at every recorded tile");
    }

    #[test]
    fn every_provider_can_be_read_back_out_of_a_tile_hash_it_keyed() {
        let unreadable: Vec<String> = PROVIDERS
            .iter()
            .filter(|(_, hash)| provider_of(&tile_hash(*hash, 8523, 5606, 14)) != Some(*hash))
            .map(|(name, hash)| format!("{name} {hash} reads back as {:?}", provider_of(&tile_hash(*hash, 8523, 5606, 14))))
            .collect();
        assert!(unreadable.is_empty(), "{unreadable:?}");
        let widths: Vec<usize> = PROVIDERS.iter().map(|(_, hash)| tile_hash(*hash, 8523, 5606, 14).len()).collect();
        assert!(widths.contains(&29) && widths.contains(&30), "provider hashes are not all the same width, which is why the provider is read from the end rather than the start");
        assert_eq!(provider_of("short"), None);
        assert_eq!(provider_of(""), None);
    }

    #[test]
    fn qts_own_decoder_misreads_the_widest_provider_hashes() {
        let widest: Vec<&str> = PROVIDERS.iter().filter(|(_, hash)| tile_hash(*hash, 8523, 5606, 14).len() > 29).map(|(name, _)| *name).collect();
        assert_eq!(widest.len(), 4, "four provider hashes need eleven characters, so Qt's tileHashToType reads ten of them and answers a provider that is not there");
        let hash = tile_hash(provider_hash(widest[0]).unwrap(), 8523, 5606, 14);
        let qt_reads: i32 = hash[..10].parse().unwrap();
        assert_ne!(qt_reads, provider_hash(widest[0]).unwrap());
        assert_eq!(provider_named(qt_reads), None, "the number Qt reads names no provider at all, so this shows up as a lookup failure rather than a wrong map");
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
        let objects = recorded.as_object().unwrap().keys().filter(|key| !key.starts_with('_')).count();
        assert_eq!(ours.len(), objects, "every table and index Qt creates is created here too");
    }

    #[test]
    fn a_cache_opens_with_the_default_set_qt_named() {
        let recorded: Value = serde_json::from_str(fixture("tile-cache-schema.json")["_tileSets"].as_str().unwrap()).unwrap();
        let cache = Cache::open_in_memory().unwrap();
        let sets = cache.sets().unwrap();
        let theirs = recorded.as_object().unwrap();
        assert_eq!(sets.len(), theirs.len());
        assert_eq!(sets[0].name, *theirs.keys().next().unwrap(), "the default set carries the name Qt gave it, or a database written here opens in Qt with two default sets");
        assert!(sets[0].default_set);
        assert_eq!(cache.default_set().unwrap(), sets[0].id);
    }

    #[test]
    fn the_default_set_is_listed_first_however_it_is_named() {
        let cache = Cache::open_in_memory().unwrap();
        ["Alps", "Aaa Downloaded"].iter().for_each(|name| {
            cache.connection.execute("INSERT INTO TileSets(name, defaultSet, date) VALUES(?1, 0, 0)", params![name]).unwrap();
        });
        let listed: Vec<String> = cache.sets().unwrap().into_iter().map(|set| set.name).collect();
        assert_eq!(listed[0], DEFAULT_SET_NAME, "the default set leads the list even though its name sorts last");
        assert_eq!(listed[1..], ["Aaa Downloaded", "Alps"], "the rest are alphabetical");
    }

    fn tile(hash: &str, bytes: usize) -> Tile {
        Tile { hash: hash.to_string(), format: "png".to_string(), image: vec![7u8; bytes], kind: "Bing Road".to_string() }
    }

    fn hash_for(x: i32) -> String {
        tile_hash(provider_hash("Bing Road").unwrap(), x, 5606, 14)
    }

    #[test]
    fn a_database_another_writer_owns_is_opened_without_writing_to_it() {
        let scratch = std::env::temp_dir().join(format!("qgc-core-serve-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&scratch);
        let hash = hash_for(8523);
        {
            let owner = Cache::open(&scratch).unwrap();
            owner.save(&tile(&hash, 512), None).unwrap();
        }
        let reader = Cache::serve(&scratch).unwrap();
        assert_eq!(reader.tile(&hash).unwrap().unwrap().image.len(), 512);
        assert!(reader.save(&tile(&hash_for(1), 10), None).is_err(), "a reader must not be able to write into a database it does not own");
        assert!(Cache::serve(&std::env::temp_dir().join("qgc-core-not-here.db")).is_err(), "and it must not create one that is not there");
        let odd = std::env::temp_dir().join(format!("qgc core serve?{}.db", std::process::id()));
        let _ = std::fs::remove_file(&odd);
        Cache::open(&odd).unwrap().save(&tile(&hash, 8), None).unwrap();
        assert_eq!(Cache::serve(&odd).unwrap().tile(&hash).unwrap().unwrap().image.len(), 8, "a path with a question mark in it is a path, not the start of a query string");
        let _ = std::fs::remove_file(&odd);
        let _ = std::fs::remove_file(&scratch);
    }

    #[test]
    fn a_saved_tile_comes_back_whole() {
        let cache = Cache::open_in_memory().unwrap();
        let hash = hash_for(8523);
        assert!(cache.tile(&hash).unwrap().is_none(), "an empty cache serves nothing rather than an empty tile");
        assert!(cache.save(&tile(&hash, 2048), None).unwrap());
        let served = cache.tile(&hash).unwrap().unwrap();
        assert_eq!(served.image.len(), 2048);
        assert_eq!(served.format, "png");
        assert_eq!(served.kind, "Bing Road", "the provider name is what the Qt worker writes into this column, whatever the schema calls it");
        assert_eq!(cache.total_size().unwrap(), 2048);
        assert_eq!(cache.saved(cache.default_set().unwrap()).unwrap(), (1, 2048), "the set a tile was filed under counts it");
    }

    #[test]
    fn a_tile_with_no_bytes_round_trips_rather_than_reading_as_a_miss() {
        let cache = Cache::open_in_memory().unwrap();
        let hash = hash_for(1);
        assert!(cache.save(&tile(&hash, 0), None).unwrap());
        assert_eq!(cache.tile(&hash).unwrap().unwrap().image.len(), 0, "an empty tile is a tile the server sent, and re-fetching it would be endless");
        assert_eq!(cache.total_size().unwrap(), 0);
    }

    #[test]
    fn the_same_tile_arriving_twice_is_stored_once() {
        let cache = Cache::open_in_memory().unwrap();
        let hash = hash_for(1);
        assert!(cache.save(&tile(&hash, 100), None).unwrap());
        assert!(!cache.save(&tile(&hash, 100), None).unwrap(), "QtLocation asks for the same tile twice in a row, and the second arrival is not an error");
        assert_eq!(cache.count().unwrap(), 1);
    }

    fn aged_cache(tiles: i32) -> (Cache, Vec<String>) {
        let cache = Cache::open_in_memory().unwrap();
        let hashes: Vec<String> = (0..tiles).map(hash_for).collect();
        hashes.iter().enumerate().for_each(|(index, hash)| {
            cache.save(&tile(hash, 1000), None).unwrap();
            cache.connection.execute("UPDATE Tiles SET date = ?1 WHERE hash = ?2", params![index as i64, hash]).unwrap();
        });
        (cache, hashes)
    }

    #[test]
    fn pruning_frees_what_it_was_asked_for_starting_with_the_oldest() {
        let (cache, hashes) = aged_cache(5);
        assert_eq!(cache.total_size().unwrap(), 5000);
        assert_eq!(cache.prune(2500).unwrap(), 3, "tiles go until the debt is paid, so three thousand bytes cover a debt of two and a half");
        assert!(cache.tile(&hashes[0]).unwrap().is_none(), "the oldest tile is the first to go");
        assert!(cache.tile(&hashes[3]).unwrap().is_some(), "the newest tiles stay");
        assert_eq!(cache.total_size().unwrap(), 2000);
        assert_eq!(cache.connection.query_row("SELECT COUNT(*) FROM SetTiles", [], |row| row.get::<_, i64>(0)).unwrap(), 2, "a pruned tile leaves no row behind in its set");
    }

    #[test]
    fn pruning_never_touches_a_tile_a_downloaded_set_also_holds() {
        let (cache, hashes) = aged_cache(3);
        cache.connection.execute("INSERT INTO TileSets(name, defaultSet, date) VALUES('Flight Area', 0, 0)", []).unwrap();
        let offline: i64 = cache.connection.query_row("SELECT setID FROM TileSets WHERE name = 'Flight Area'", [], |row| row.get(0)).unwrap();
        let oldest: i64 = cache.connection.query_row("SELECT tileID FROM Tiles WHERE hash = ?1", params![hashes[0]], |row| row.get(0)).unwrap();
        cache.connection.execute("INSERT INTO SetTiles(tileID, setID) VALUES(?1, ?2)", params![oldest, offline]).unwrap();

        assert_eq!(cache.prune(3000).unwrap(), 2, "only the two tiles nothing else holds can go");
        assert!(cache.tile(&hashes[0]).unwrap().is_some(), "the oldest tile is in an offline set the operator downloaded for a flight, and pruning must not take it");
        assert_eq!(cache.saved(offline).unwrap(), (1, 1000));
    }

    #[test]
    fn pruning_an_empty_cache_asks_for_nothing() {
        let cache = Cache::open_in_memory().unwrap();
        assert_eq!(cache.prune(5000).unwrap(), 0);
        let (full, _) = aged_cache(2);
        assert_eq!(full.prune(0).unwrap(), 1, "a debt of nothing still takes one tile, which is what the Qt worker does rather than looping forever on a full cache");
    }

    #[test]
    fn pruning_takes_no_more_than_a_batch_at_a_time() {
        let (cache, _) = aged_cache(200);
        assert_eq!(cache.prune(1_000_000).unwrap(), 128, "the Qt worker prunes a hundred and twenty eight at a time and is called again, rather than holding a write lock over the whole cache");
    }
}

#[cfg(test)]
mod real_cache {
    use super::*;

    #[test]
    fn a_cache_an_operator_already_has_opens_and_serves_without_downloading() {
        let Ok(path) = std::env::var("QGC_REAL_TILE_CACHE") else {
            return;
        };
        let cache = Cache::open(Path::new(&path)).expect("an existing qgcMapCache.db opens");
        let mut listed = cache
            .connection
            .prepare("SELECT hash, length(tile) FROM Tiles WHERE tile IS NOT NULL")
            .expect("the schema is the one Qt writes");
        let rows: Vec<(String, i64)> = listed
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(!rows.is_empty(), "the cache holds tiles, so there is something to serve");

        let served = rows
            .iter()
            .filter(|(hash, bytes)| {
                cache.tile(hash).ok().flatten().is_some_and(|tile| tile.image.len() as i64 == *bytes)
            })
            .count();
        assert_eq!(served, rows.len(), "every tile Qt wrote is one the core reads back under the hash Qt keyed it by, with the same byte count; anything less is a tile the operator would have to download again");
    }
}
