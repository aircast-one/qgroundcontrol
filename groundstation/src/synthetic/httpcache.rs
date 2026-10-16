use std::cell::Cell;
use std::path::Path;
use std::time::Duration;
use web_time::{SystemTime, UNIX_EPOCH};

use http_cache_semantics::{AfterResponse, BeforeRequest, CacheOptions, CachePolicy};
use rusqlite::{Connection, OptionalExtension, params};
use ureq::http::{HeaderMap, Request, Response, StatusCode};
use url::Url;

const SIGNING: [&str; 2] = ["key", "session"];
const TRIM_EVERY: u64 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Fresh,
    Revalidated,
    Downloaded,
}

pub struct HttpCache {
    db: Connection,
    limit: u64,
    stored: Cell<u64>,
}

fn unsigned(url: &Url) -> Url {
    let kept: Vec<(String, String)> = url.query_pairs().filter(|(name, _)| !SIGNING.contains(&name.as_ref())).map(|(name, value)| (name.into_owned(), value.into_owned())).collect();
    let query = (!kept.is_empty()).then(|| url::form_urlencoded::Serializer::new(String::new()).extend_pairs(kept).finish());
    let mut bare = url.clone();
    bare.set_query(query.as_deref());
    bare
}

fn private() -> CacheOptions {
    CacheOptions { shared: false, ..CacheOptions::default() }
}

fn seconds(at: SystemTime) -> i64 {
    at.duration_since(UNIX_EPOCH).map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX))
}

impl HttpCache {
    pub fn open(path: &Path, limit: u64) -> rusqlite::Result<Self> {
        let db = Connection::open(path)?;
        db.busy_timeout(Duration::from_secs(5))?;
        db.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        db.execute_batch(
            "PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS responses (key TEXT PRIMARY KEY NOT NULL, policy TEXT NOT NULL, body BLOB NOT NULL, size INTEGER NOT NULL, used INTEGER NOT NULL, fetched INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS responses_used ON responses (used);",
        )?;
        let cache = Self { db, limit, stored: Cell::new(0) };
        cache.trim()?;
        Ok(cache)
    }

    pub fn fetch(&self, url: &Url, now: SystemTime, reuse_body_for: Duration, send: impl FnOnce(&HeaderMap) -> Result<Response<Vec<u8>>, String>) -> Result<(Vec<u8>, Answer), String> {
        let key = unsigned(url).to_string();
        let asked = Request::get(key.as_str()).body(()).map_err(|error| error.to_string())?;
        match self.stored(&key, now, reuse_body_for) {
            Some((policy, body)) => match policy.before_request(&asked, now) {
                BeforeRequest::Fresh(_) => {
                    self.note(self.db.execute("UPDATE responses SET used = ?2 WHERE key = ?1", params![key, seconds(now)]));
                    Ok((body, Answer::Fresh))
                }
                BeforeRequest::Stale { request, matches } => {
                    let response = send(&request.headers)?;
                    match policy.after_response(&asked, &response, now) {
                        AfterResponse::NotModified(renewed, _) if matches => {
                            self.note(serde_json::to_string(&renewed).map_err(|error| error.to_string()).and_then(|renewed| {
                                self.db.execute("UPDATE responses SET policy = ?2, used = ?3 WHERE key = ?1", params![key, renewed, seconds(now)]).map_err(|error| error.to_string())
                            }));
                            Ok((body, Answer::Revalidated))
                        }
                        _ => self.keep(&key, &asked, response, now),
                    }
                }
            },
            None => self.keep(&key, &asked, send(&HeaderMap::new())?, now),
        }
    }

    fn stored(&self, key: &str, now: SystemTime, reuse_body_for: Duration) -> Option<(CachePolicy, Vec<u8>)> {
        self.db
            .query_row("SELECT policy, body, fetched FROM responses WHERE key = ?1", params![key], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?, row.get::<_, i64>(2)?)))
            .optional()
            .ok()
            .flatten()
            .filter(|(_, _, fetched)| Duration::from_secs(u64::try_from(seconds(now) - fetched).unwrap_or(0)) <= reuse_body_for)
            .and_then(|(policy, body, _)| serde_json::from_str(&policy).ok().map(|policy| (policy, body)))
    }

    fn keep(&self, key: &str, asked: &Request<()>, response: Response<Vec<u8>>, now: SystemTime) -> Result<(Vec<u8>, Answer), String> {
        if response.status() != StatusCode::OK {
            return Err(format!("the server answered {}", response.status()));
        }
        let policy = CachePolicy::new_options(asked, &response, now, private());
        let body = response.into_body();
        if policy.is_storable() {
            self.note(serde_json::to_string(&policy).map_err(|error| error.to_string()).and_then(|policy| {
                self.db
                    .execute("INSERT OR REPLACE INTO responses (key, policy, body, size, used, fetched) VALUES (?1, ?2, ?3, ?4, ?5, ?5)", params![key, policy, body, body.len() as i64, seconds(now)])
                    .map_err(|error| error.to_string())
            }));
            self.stored.set(self.stored.get() + 1);
            if self.stored.get().is_multiple_of(TRIM_EVERY) {
                self.note(self.trim());
            }
        }
        Ok((body, Answer::Downloaded))
    }

    fn trim(&self) -> rusqlite::Result<usize> {
        self.db.execute(
            "DELETE FROM responses WHERE key IN (SELECT key FROM (SELECT key, SUM(size) OVER (ORDER BY used DESC, key) AS running FROM responses) WHERE running > ?1)",
            params![i64::try_from(self.limit).unwrap_or(i64::MAX)],
        )
    }

    fn note<T, E: std::fmt::Display>(&self, done: Result<T, E>) {
        if let Err(error) = done {
            log::warn!(target: "synthetic", "event=tile_cache_failed reason=\"{error}\"");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const BODY: &[u8] = b"glb bytes";

    fn answer(status: u16, cache_control: &str, body: &[u8]) -> Response<Vec<u8>> {
        Response::builder().status(status).header("cache-control", cache_control).header("etag", "\"E1\"").body(body.to_vec()).unwrap()
    }

    fn tile(session: &str) -> Url {
        Url::parse(&format!("https://tile.googleapis.com/v1/3dtiles/datasets/D/files/F.glb?session={session}&key=K")).unwrap()
    }

    fn cache(limit: u64) -> HttpCache {
        HttpCache::open(Path::new(":memory:"), limit).unwrap()
    }

    #[test]
    fn a_tile_is_served_from_disk_while_fresh_whatever_session_and_key_sign_it() {
        let cache = cache(1 << 30);
        let start = SystemTime::now();
        let sent = RefCell::new(Vec::<HeaderMap>::new());
        let send = |status: u16, body: &'static [u8]| {
            let sent = &sent;
            move |headers: &HeaderMap| {
                sent.borrow_mut().push(headers.clone());
                Ok(answer(status, "private, max-age=14400, must-revalidate, no-transform", body))
            }
        };
        assert_eq!(cache.fetch(&tile("S1"), start, Duration::MAX, send(200, BODY)).unwrap(), (BODY.to_vec(), Answer::Downloaded));
        assert_eq!(cache.fetch(&tile("S2"), start + Duration::from_secs(3_600), Duration::MAX, send(200, b"unused")).unwrap(), (BODY.to_vec(), Answer::Fresh), "a new session is the same tile");
        assert_eq!(sent.borrow().len(), 1, "a fresh tile never touches the network");
        let later = start + Duration::from_secs(5 * 3_600);
        assert_eq!(cache.fetch(&tile("S3"), later, Duration::MAX, send(304, b"")).unwrap(), (BODY.to_vec(), Answer::Revalidated), "past max-age a 304 keeps the stored body");
        assert_eq!(sent.borrow()[1].get("if-none-match").and_then(|value| value.to_str().ok()), Some("\"E1\""), "the stale tile is revalidated with its ETag");
        assert_eq!(cache.fetch(&tile("S3"), later + Duration::from_secs(60), Duration::MAX, send(200, b"unused")).unwrap().1, Answer::Fresh, "the 304 renewed it for another max-age");
        assert_eq!(cache.fetch(&tile("S4"), later + Duration::from_secs(5 * 3_600), Duration::MAX, send(200, b"new")).unwrap(), (b"new".to_vec(), Answer::Downloaded), "a changed tile replaces the old one");
    }

    #[test]
    fn what_the_server_forbids_storing_is_fetched_every_time() {
        let cache = cache(1 << 30);
        let now = SystemTime::now();
        let sends = Cell::new(0);
        let fetch = || {
            cache
                .fetch(&tile("S"), now, Duration::MAX, |_| {
                    sends.set(sends.get() + 1);
                    Ok(answer(200, "no-store", BODY))
                })
                .unwrap()
        };
        assert_eq!(fetch().1, Answer::Downloaded);
        assert_eq!(fetch().1, Answer::Downloaded);
        assert_eq!(sends.get(), 2);
    }

    #[test]
    fn an_error_answer_is_an_error_and_is_not_stored() {
        let cache = cache(1 << 30);
        let now = SystemTime::now();
        assert!(cache.fetch(&tile("S"), now, Duration::MAX, |_| Ok(answer(503, "private, max-age=14400", b"busy"))).is_err());
        assert_eq!(cache.fetch(&tile("S"), now, Duration::MAX, |_| Ok(answer(200, "private, max-age=14400", BODY))).unwrap().1, Answer::Downloaded);
    }

    #[test]
    fn past_the_size_limit_the_least_recently_used_tiles_go() {
        let cache = cache(25);
        let start = SystemTime::now();
        let url = |name: &str| Url::parse(&format!("https://tile.googleapis.com/v1/3dtiles/datasets/D/files/{name}.glb?key=K")).unwrap();
        let ten = |_: &HeaderMap| Ok(answer(200, "private, max-age=14400", &[0; 10]));
        ["a", "b", "c"].iter().enumerate().for_each(|(age, name)| {
            cache.fetch(&url(name), start + Duration::from_secs(age as u64), Duration::MAX, ten).unwrap();
        });
        cache.fetch(&url("a"), start + Duration::from_secs(10), Duration::MAX, ten).unwrap();
        cache.trim().unwrap();
        let kept: Vec<bool> = ["a", "b", "c"].iter().map(|name| cache.stored(unsigned(&url(name)).as_str(), start, Duration::MAX).is_some()).collect();
        assert_eq!(kept, [true, false, true], "b was used least recently; a was read again after c arrived");
    }

    #[test]
    fn a_root_is_revalidated_while_its_session_is_young_and_fetched_anew_once_it_is_old() {
        let cache = cache(1 << 30);
        let start = SystemTime::now();
        let root = Url::parse("https://tile.googleapis.com/v1/3dtiles/root.json?key=K").unwrap();
        let hour = Duration::from_secs(3_600);
        let conditional = Cell::new(None::<bool>);
        let send = |status: u16, body: &'static [u8]| {
            let conditional = &conditional;
            move |headers: &HeaderMap| {
                conditional.set(Some(headers.contains_key("if-none-match")));
                Ok(answer(status, "private, max-age=0, must-revalidate, no-transform", body))
            }
        };
        cache.fetch(&root, start, hour, send(200, b"session A")).unwrap();
        assert_eq!(cache.fetch(&root, start + hour / 2, hour, send(304, b"")).unwrap(), (b"session A".to_vec(), Answer::Revalidated), "the same session keeps every tile path the same");
        assert_eq!(cache.fetch(&root, start + hour * 2, hour, send(200, b"session B")).unwrap(), (b"session B".to_vec(), Answer::Downloaded));
        assert_eq!(conditional.get(), Some(false), "a session past its reuse window is not offered to the server again");
    }
}
