use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

pub const PROVIDERS: [(&str, i32); 40] = [
    ("Bing Hybrid", 8),
    ("Bing Road", 6),
    ("Bing Satellite", 7),
    ("Copernicus", 40),
    ("CustomURL Custom", 39),
    ("Eniro Topo", 14),
    ("Esri Terrain", 17),
    ("Esri World Satellite", 16),
    ("Esri World Street", 15),
    ("Google Hybrid", 4),
    ("Google Labels", 5),
    ("Google Satellite", 2),
    ("Google Street Map", 1),
    ("Google Terrain", 3),
    ("Japan-GSI Anaglyph", 33),
    ("Japan-GSI Contour", 31),
    ("Japan-GSI Relief", 35),
    ("Japan-GSI Seamless", 32),
    ("Japan-GSI Slope", 34),
    ("LINZ Basemap", 36),
    ("MapQuest Map", 27),
    ("MapQuest Sat", 28),
    ("Mapbox Bright", 25),
    ("Mapbox Custom", 26),
    ("Mapbox Dark", 20),
    ("Mapbox Hybrid", 22),
    ("Mapbox Light", 19),
    ("Mapbox Outdoors", 24),
    ("Mapbox Satellite", 21),
    ("Mapbox Streets", 18),
    ("Mapbox StreetsBasic", 23),
    ("OpenAIP", 38),
    ("Statkart Basemap", 12),
    ("Statkart Topo", 11),
    ("Street Map", 37),
    ("Svalbard Topo", 13),
    ("TianDiTu Road", 9),
    ("TianDiTu Satellite", 10),
    ("VWorld Satellite Map", 30),
    ("VWorld Street Map", 29),
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

pub const CORE_ONLY_PROVIDERS: &[(&str, i32)] = &[(crate::maptypes::TERRAIN_TILES, 41)];

pub fn provider_hash(name: &str) -> Option<i32> {
    PROVIDERS.iter().chain(CORE_ONLY_PROVIDERS).find(|(known, _)| *known == name).map(|(_, hash)| *hash)
}

pub fn provider_named(hash: i32) -> Option<&'static str> {
    PROVIDERS.iter().chain(CORE_ONLY_PROVIDERS).find(|(_, known)| *known == hash).map(|(name, _)| *name)
}

pub fn tile_hash(provider: i32, x: i32, y: i32, z: i32) -> String {
    format!("{provider:010}{x:08}{y:08}{z:03}")
}

const TILE_DIGITS: usize = 19;

pub fn provider_of(hash: &str) -> Option<i32> {
    hash.len().checked_sub(TILE_DIGITS).and_then(|head| hash.get(..head)).and_then(|head| head.parse::<i32>().ok())
}

const READONLY_RETRIES: u64 = 5;
const RETRY_PAUSE_MS: u64 = 20;

fn recovering_a_journal(error: &rusqlite::Error) -> bool {
    matches!(error.sqlite_error_code(), Some(rusqlite::ErrorCode::ReadOnly))
}

const UNIQUE_TILES: &str = "SELECT A.tileID FROM SetTiles A JOIN SetTiles B ON A.tileID = B.tileID WHERE B.setID = ?1 GROUP BY A.tileID HAVING COUNT(A.tileID) = 1";

pub struct Cache {
    connection: Connection,
}

const SCHEMA: [&str; 10] = [
    "CREATE TABLE IF NOT EXISTS Tiles (tileID INTEGER PRIMARY KEY NOT NULL, hash TEXT NOT NULL UNIQUE, format TEXT NOT NULL, tile BLOB NULL, size INTEGER, type INTEGER, date INTEGER DEFAULT 0)",
    "CREATE TABLE IF NOT EXISTS TileSets (setID INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL UNIQUE, typeStr TEXT, topleftLat REAL DEFAULT 0.0, topleftLon REAL DEFAULT 0.0, bottomRightLat REAL DEFAULT 0.0, bottomRightLon REAL DEFAULT 0.0, minZoom INTEGER DEFAULT 3, maxZoom INTEGER DEFAULT 3, type INTEGER DEFAULT -1, numTiles INTEGER DEFAULT 0, defaultSet INTEGER DEFAULT 0, date INTEGER DEFAULT 0)",
    "CREATE TABLE IF NOT EXISTS SetTiles (setID INTEGER NOT NULL REFERENCES TileSets(setID) ON DELETE CASCADE, tileID INTEGER NOT NULL REFERENCES Tiles(tileID) ON DELETE CASCADE)",
    "CREATE TABLE IF NOT EXISTS TilesDownload (setID INTEGER NOT NULL REFERENCES TileSets(setID) ON DELETE CASCADE, hash TEXT NOT NULL, type INTEGER, x INTEGER, y INTEGER, z INTEGER, state INTEGER DEFAULT 0)",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_settiles_unique ON SetTiles(tileID, setID)",
    "CREATE INDEX IF NOT EXISTS idx_settiles_setid ON SetTiles(setID)",
    "CREATE INDEX IF NOT EXISTS idx_settiles_tileid ON SetTiles(tileID)",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_tilesdownload_setid_hash ON TilesDownload(setID, hash)",
    "CREATE INDEX IF NOT EXISTS idx_tilesdownload_setid_state ON TilesDownload(setID, state)",
    "CREATE INDEX IF NOT EXISTS idx_tiles_date ON Tiles(date)",
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

    pub fn serve(path: &Path) -> rusqlite::Result<Cache> {
        let connection = Connection::open_with_flags(
            &format!("file:{}?mode=ro&cache=private&readonly_shm=1", uri_escaped(path)),
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
                Ok(Tile { hash: hash.to_string(), image: row.get(0)?, format: row.get(1)?, kind: row.get::<_, Option<i64>>(2)?.unwrap_or(-1) as i32 })
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

    pub fn busiest_provider(&self) -> rusqlite::Result<Option<i32>> {
        self.connection
            .query_row("SELECT type FROM Tiles GROUP BY type ORDER BY COUNT(*) DESC LIMIT 1", [], |row| row.get::<_, Option<i32>>(0))
            .optional()
            .map(Option::flatten)
    }

    pub fn total_size(&self) -> rusqlite::Result<i64> {
        self.connection.query_row("SELECT SUM(size) FROM Tiles", [], |row| row.get::<_, Option<i64>>(0)).map(|total| total.unwrap_or(0))
    }

    pub fn count(&self) -> rusqlite::Result<i64> {
        self.connection.query_row("SELECT COUNT(*) FROM Tiles", [], |row| row.get(0))
    }

    pub fn prune(&self, free_bytes: i64) -> rusqlite::Result<i64> {
        self.prune_owing(free_bytes).map(|(deleted, _)| deleted)
    }

    fn prune_owing(&self, free_bytes: i64) -> rusqlite::Result<(i64, i64)> {
        let (deleted, left) = self.prune_batch(free_bytes)?;
        match deleted > 0 && left > 0 {
            true => self.prune_owing(left).map(|(more, still)| (deleted + more, still)),
            false => Ok((deleted, left)),
        }
    }

    fn prune_batch(&self, free_bytes: i64) -> rusqlite::Result<(i64, i64)> {
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
            .take_while(|(_, owed)| *owed > 0)
            .map(|(id, _)| id)
            .collect();
        let freed: i64 = aged.iter().filter(|(id, _)| doomed.contains(id)).map(|(_, size)| size).sum();
        doomed.iter().try_for_each(|id| {
            self.connection.execute("DELETE FROM SetTiles WHERE tileID = ?1", params![id])?;
            self.connection.execute("DELETE FROM Tiles WHERE tileID = ?1", params![id]).map(|_| ())
        })?;
        Ok((doomed.len() as i64, free_bytes - freed))
    }

    pub fn create_set(&self, set: &TileSet, tiles: &[(i32, i32, i32)]) -> rusqlite::Result<i64> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO TileSets(name, typeStr, topleftLat, topleftLon, bottomRightLat, bottomRightLon, minZoom, maxZoom, type, numTiles, date) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![set.name, set.type_str, set.top_left.0, set.top_left.1, set.bottom_right.0, set.bottom_right.1, set.min_zoom, set.max_zoom, set.kind, set.tiles, now_secs()],
        )?;
        let id = transaction.last_insert_rowid();
        tiles.iter().try_for_each(|(x, y, z)| {
            let hash = tile_hash(set.kind, *x, *y, *z);
            let linked = transaction.execute("INSERT OR IGNORE INTO SetTiles(tileID, setID) SELECT tileID, ?1 FROM Tiles WHERE hash = ?2", params![id, hash])?;
            match linked {
                0 => transaction
                    .execute("INSERT OR IGNORE INTO TilesDownload(setID, hash, type, x, y, z, state) VALUES(?1, ?2, ?3, ?4, ?5, ?6, 0)", params![id, hash, set.kind, x, y, z])
                    .map(|_| ()),
                _ => Ok(()),
            }
        })?;
        transaction.commit()?;
        Ok(id)
    }

    pub fn delete_set(&self, set: i64) -> rusqlite::Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute("DELETE FROM TilesDownload WHERE setID = ?1", params![set])?;
        let unique: Vec<i64> = transaction
            .prepare(&format!("SELECT tileID FROM SetTiles WHERE tileID IN ({UNIQUE_TILES})"))?
            .query_map(params![set], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        transaction.execute("DELETE FROM SetTiles WHERE setID = ?1", params![set])?;
        unique.iter().try_for_each(|id| transaction.execute("DELETE FROM Tiles WHERE tileID = ?1", params![id]).map(|_| ()))?;
        transaction.execute("DELETE FROM TileSets WHERE setID = ?1", params![set])?;
        transaction.commit()
    }

    pub fn reset(&self) -> rusqlite::Result<()> {
        self.connection.execute_batch("BEGIN; DELETE FROM TilesDownload; DELETE FROM SetTiles; DELETE FROM Tiles; DELETE FROM TileSets WHERE defaultSet = 0; COMMIT;")
    }

    pub fn rename_set(&self, set: i64, name: &str) -> rusqlite::Result<()> {
        self.connection.execute("UPDATE TileSets SET name = ?1 WHERE setID = ?2", params![name, set]).map(|_| ())
    }

    pub fn pending(&self, set: i64, count: usize) -> rusqlite::Result<Vec<(String, i32, i32, i32)>> {
        self.connection
            .prepare("SELECT hash, x, y, z FROM TilesDownload WHERE setID = ?1 AND state = 0 LIMIT ?2")?
            .query_map(params![set, count as i64], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
            .collect()
    }

    pub fn mark_error(&self, set: i64, hash: &str) -> rusqlite::Result<()> {
        self.connection.execute("UPDATE TilesDownload SET state = 2 WHERE setID = ?1 AND hash = ?2", params![set, hash]).map(|_| ())
    }

    pub fn complete(&self, set: i64, tile: &Tile) -> rusqlite::Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT OR IGNORE INTO Tiles(hash, format, tile, size, type, date) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![tile.hash, tile.format, tile.image, tile.image.len() as i64, tile.kind, now_secs()],
        )?;
        transaction.execute("INSERT OR IGNORE INTO SetTiles(tileID, setID) SELECT tileID, ?1 FROM Tiles WHERE hash = ?2", params![set, tile.hash])?;
        transaction.execute("DELETE FROM TilesDownload WHERE setID = ?1 AND hash = ?2", params![set, tile.hash])?;
        transaction.commit()
    }

    pub fn retry_errors(&self, set: i64) -> rusqlite::Result<()> {
        self.connection.execute("UPDATE TilesDownload SET state = 0 WHERE setID = ?1", params![set]).map(|_| ())
    }

    pub fn errors(&self, set: i64) -> rusqlite::Result<i64> {
        self.connection.query_row("SELECT COUNT(*) FROM TilesDownload WHERE setID = ?1 AND state = 2", params![set], |row| row.get(0))
    }

    pub fn unique(&self, set: i64) -> rusqlite::Result<(i64, i64)> {
        self.connection.query_row(&format!("SELECT COUNT(size), SUM(size) FROM Tiles WHERE tileID IN ({UNIQUE_TILES})"), params![set], |row| {
            Ok((row.get(0)?, row.get::<_, Option<i64>>(1)?.unwrap_or(0)))
        })
    }

    pub fn default_bytes(&self) -> rusqlite::Result<i64> {
        self.unique(self.default_set()?).map(|(_, size)| size)
    }

    pub fn trim_to(&self, max_bytes: i64) -> rusqlite::Result<i64> {
        let size = self.default_bytes()?;
        match size > max_bytes {
            true => self.prune_owing(size - max_bytes).map(|(_, still)| max_bytes + still),
            false => Ok(size),
        }
    }

    fn is_own_file(&self, path: &Path) -> bool {
        let own = self.connection.path().and_then(|own| std::fs::canonicalize(own).ok());
        std::fs::canonicalize(path).ok().zip(own).is_some_and(|(asked, own)| asked == own)
    }

    pub fn export(&self, sets: &[i64], path: &Path) -> Result<(), String> {
        if self.is_own_file(path) {
            return Err("Export path must differ from the active database".to_string());
        }
        let _ = std::fs::remove_file(path);
        let created = Connection::open(path).and_then(|out| SCHEMA.iter().try_for_each(|statement| out.execute_batch(statement)));
        created.map_err(|_| "Error creating export database".to_string())?;
        self.connection.execute("ATTACH DATABASE ?1 AS export", params![path.to_string_lossy()]).map_err(|_| "Error opening export database".to_string())?;
        let copied = self.copy_sets_out(sets).map_err(|_| "Error adding tile set to exported database".to_string());
        let _ = self.connection.execute_batch("DETACH DATABASE export");
        copied
    }

    fn copy_sets_out(&self, sets: &[i64]) -> rusqlite::Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        sets.iter().try_for_each(|set| {
            transaction.execute(
                "INSERT INTO export.TileSets(name, typeStr, topleftLat, topleftLon, bottomRightLat, bottomRightLon, minZoom, maxZoom, type, numTiles, defaultSet, date) \
                 SELECT name, typeStr, topleftLat, topleftLon, bottomRightLat, bottomRightLon, minZoom, maxZoom, type, numTiles, defaultSet, date FROM main.TileSets WHERE setID = ?1",
                params![set],
            )?;
            let exported = transaction.last_insert_rowid();
            transaction.execute(
                "INSERT OR IGNORE INTO export.Tiles(hash, format, tile, size, type, date) \
                 SELECT T.hash, T.format, T.tile, length(T.tile), T.type, T.date FROM main.Tiles T INNER JOIN main.SetTiles S ON T.tileID = S.tileID WHERE S.setID = ?1",
                params![set],
            )?;
            transaction
                .execute(
                    "INSERT OR IGNORE INTO export.SetTiles(tileID, setID) \
                     SELECT E.tileID, ?2 FROM export.Tiles E INNER JOIN main.Tiles T ON E.hash = T.hash INNER JOIN main.SetTiles S ON T.tileID = S.tileID WHERE S.setID = ?1",
                    params![set, exported],
                )
                .map(|_| ())
        })?;
        transaction.commit()
    }

    pub fn import(&self, path: &Path, replace: bool) -> Result<(), String> {
        if !path.is_file() {
            return Err("Error opening import database".to_string());
        }
        if self.is_own_file(path) {
            return Err("Import path must differ from the active database".to_string());
        }
        self.connection.execute("ATTACH DATABASE ?1 AS import", params![path.to_string_lossy()]).map_err(|_| "Error opening import database".to_string())?;
        let merged = self.merge_imported(replace);
        let _ = self.connection.execute_batch("DETACH DATABASE import");
        merged
    }

    fn merge_imported(&self, replace: bool) -> Result<(), String> {
        let tiles: i64 = self.connection.query_row("SELECT COUNT(tileID) FROM import.Tiles", [], |row| row.get(0)).map_err(|_| "Error opening import database".to_string())?;
        if tiles == 0 {
            return Err("No unique tiles in imported database".to_string());
        }
        let imported: Vec<(i64, String, bool)> = self
            .connection
            .prepare("SELECT setID, name, defaultSet FROM import.TileSets ORDER BY defaultSet DESC, name ASC")
            .and_then(|mut statement| statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i64>(2)? != 0)))?.collect())
            .map_err(|_| "No tile set in database".to_string())?;
        if replace {
            self.reset().map_err(|error| error.to_string())?;
        }
        let linked = imported.iter().try_fold(false, |any, (source, name, default_set)| {
            self.merge_set(*source, name, *default_set).map(|linked| any || linked)
        });
        match linked.map_err(|_| "Error adding imported tile set to database".to_string())? {
            true => Ok(()),
            false => Err("No unique tiles in imported database".to_string()),
        }
    }

    fn merge_set(&self, source: i64, name: &str, default_set: bool) -> rusqlite::Result<bool> {
        let transaction = self.connection.unchecked_transaction()?;
        let target = match default_set {
            true => self.default_set()?,
            false => {
                transaction.execute(
                    "INSERT INTO main.TileSets(name, typeStr, topleftLat, topleftLon, bottomRightLat, bottomRightLon, minZoom, maxZoom, type, numTiles, defaultSet, date) \
                     SELECT ?2, typeStr, topleftLat, topleftLon, bottomRightLat, bottomRightLon, minZoom, maxZoom, type, numTiles, defaultSet, ?3 FROM import.TileSets WHERE setID = ?1",
                    params![source, self.unused_name(name)?, now_secs()],
                )?;
                transaction.last_insert_rowid()
            }
        };
        transaction.execute(
            "INSERT OR IGNORE INTO main.Tiles(hash, format, tile, size, type, date) \
             SELECT T.hash, T.format, T.tile, length(T.tile), T.type, T.date FROM import.Tiles T INNER JOIN import.SetTiles S ON T.tileID = S.tileID WHERE S.setID = ?1",
            params![source],
        )?;
        let linked = transaction.execute(
            "INSERT OR IGNORE INTO main.SetTiles(tileID, setID) \
             SELECT M.tileID, ?2 FROM main.Tiles M INNER JOIN import.Tiles T ON M.hash = T.hash INNER JOIN import.SetTiles S ON T.tileID = S.tileID WHERE S.setID = ?1",
            params![source, target],
        )?;
        match (linked, default_set) {
            (0, false) => transaction.execute("DELETE FROM main.TileSets WHERE setID = ?1", params![target]).map(|_| ())?,
            (0, true) => (),
            _ => transaction
                .execute("UPDATE main.TileSets SET numTiles = (SELECT COUNT(*) FROM main.SetTiles WHERE setID = ?1) WHERE setID = ?1", params![target])
                .map(|_| ())?,
        }
        transaction.commit()?;
        Ok(linked > 0)
    }

    fn unused_name(&self, name: &str) -> rusqlite::Result<String> {
        let taken: std::collections::HashSet<String> = self.sets()?.into_iter().map(|set| set.name).collect();
        Ok(std::iter::once(name.to_string())
            .chain((1..=9999).map(|n| format!("{name} {n:04}")))
            .find(|candidate| !taken.contains(candidate))
            .unwrap_or_else(|| format!("{name} {}", now_secs())))
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
    use super::{Cache, Tile, recovering_a_journal, tile_hash};

    #[test]
    fn the_busiest_provider_is_the_one_with_the_most_cached_tiles() {
        let cache = Cache::open_in_memory().unwrap();
        assert_eq!(cache.busiest_provider().unwrap(), None, "an empty cache names no provider, so a failed fetch falls through to nothing");
        let save = |provider: i32, x: i32| cache.save(&Tile { hash: tile_hash(provider, x, 7, 12), format: "png".into(), image: vec![provider as u8, x as u8], kind: provider }, None).unwrap();
        [(42, 1), (42, 2), (42, 3), (7, 1)].into_iter().for_each(|(provider, x)| { save(provider, x); });
        assert_eq!(cache.busiest_provider().unwrap(), Some(42), "like Android's QgcTileCache.providers().first(), the offline fallback reads the provider with the most tiles");
        assert_eq!(cache.tile(&tile_hash(42, 2, 7, 12)).unwrap().map(|tile| tile.image), Some(vec![42, 2]));
    }

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
    fn terrain_tiles_have_a_key_of_their_own_outside_qts_list() {
        assert_eq!(provider_hash(crate::maptypes::TERRAIN_TILES), Some(41));
        assert_eq!(provider_named(41), Some(crate::maptypes::TERRAIN_TILES));
        assert!(PROVIDERS.iter().all(|(name, hash)| *name != crate::maptypes::TERRAIN_TILES && *hash != 41), "the key must not collide with a provider Qt serves");
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
        assert!(widths.iter().all(|width| *width == 29), "{widths:?}");
        assert_eq!(provider_of("short"), None);
        assert_eq!(provider_of(""), None);
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
        Tile { hash: hash.to_string(), format: "png".to_string(), image: vec![7u8; bytes], kind: provider_hash("Bing Road").unwrap() }
    }

    fn hash_for(x: i32) -> String {
        tile_hash(provider_hash("Bing Road").unwrap(), x, 5606, 14)
    }

    #[test]
    fn a_database_another_writer_owns_is_opened_without_writing_to_it() {
        let scratch = std::env::temp_dir().join(format!("groundstation-serve-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&scratch);
        let hash = hash_for(8523);
        {
            let owner = Cache::open(&scratch).unwrap();
            owner.save(&tile(&hash, 512), None).unwrap();
        }
        let reader = Cache::serve(&scratch).unwrap();
        assert_eq!(reader.tile(&hash).unwrap().unwrap().image.len(), 512);
        assert!(reader.save(&tile(&hash_for(1), 10), None).is_err(), "a reader must not be able to write into a database it does not own");
        assert!(Cache::serve(&std::env::temp_dir().join("groundstation-not-here.db")).is_err(), "and it must not create one that is not there");
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
        assert_eq!(served.kind, provider_hash("Bing Road").unwrap(), "the provider id is what the Qt worker writes into this column");
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
        assert_eq!(full.prune(0).unwrap(), 0, "QGCTileCacheDatabase::pruneCache stops once nothing is owed");
    }

    #[test]
    fn pruning_loops_batches_until_the_debt_is_paid_and_stops_on_an_exact_hit() {
        let (cache, _) = aged_cache(200);
        assert_eq!(cache.prune(1_000_000).unwrap(), 200, "batches of 128 repeat until the overage is gone");
        let (exact, _) = aged_cache(5);
        assert_eq!(exact.prune(2000).unwrap(), 2, "two 1000-byte tiles pay 2000 exactly, so a third is not taken");
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
