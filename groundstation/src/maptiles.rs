use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex, PoisonError, TryLockError};
use std::time::{Duration, Instant};

use once_cell::sync::OnceCell;
use quick_cache::Weighter;
use rayon_core::{ThreadPool, ThreadPoolBuildError, ThreadPoolBuilder};

use crate::mapurls::{Keys, TileRequest, tile_request};
use crate::tilecache::{Cache, Tile, provider_hash, tile_hash};

const TILE_TIMEOUT: Duration = Duration::from_secs(10);
const TILE_THREADS: usize = 6;
const TILE_THREAD_NAME: &str = "qgc-tiles";
const TILES_WAITING: usize = 64;
const BING_NO_TILE: &[u8] = include_bytes!("../../resources/BingNoTileBytes.dat");
const ELEVATION_PROVIDER: &str = "Copernicus";
const DISK_LIMIT_PATH: &str = "settings.mapsSettings.maxCacheDiskSize";
const DEFAULT_DISK_LIMIT_MB: u64 = 1024;
const LIMIT_CHECK_EVERY: Duration = Duration::from_secs(2);
const RECOUNT_EVERY: Duration = Duration::from_secs(300);
const MEMORY_LIMIT_PATH: &str = "settings.mapsSettings.maxCacheMemorySize";
const MEGABYTE: u64 = 1024 * 1024;
const TYPICAL_TILE_BYTES: u64 = 20 * 1024;

fn disk_limit_bytes() -> i64 {
    let megabytes = crate::settingsstore::raw_setting(DISK_LIMIT_PATH).and_then(|value| value.as_u64()).unwrap_or(DEFAULT_DISK_LIMIT_MB);
    (megabytes * 1024 * 1024) as i64
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DiskTally {
    bytes: i64,
    counted: Instant,
    trimmed: Option<Instant>,
}

static DISK_TALLY: Mutex<Option<DiskTally>> = Mutex::new(None);
static UNTALLIED: AtomicI64 = AtomicI64::new(0);

fn tallied(held: Option<DiskTally>, added: i64, limit: i64, now: Instant, count: impl FnOnce() -> Option<i64>, trim: impl FnOnce(i64) -> Option<i64>) -> Option<DiskTally> {
    let kept = held.filter(|tally| now.duration_since(tally.counted) < RECOUNT_EVERY).map(|tally| DiskTally { bytes: tally.bytes + added, ..tally });
    let tally = kept.or_else(|| count().map(|bytes| DiskTally { bytes, counted: now, trimmed: None }))?;
    let due = tally.bytes > limit && tally.trimmed.is_none_or(|at| now.duration_since(at) >= LIMIT_CHECK_EVERY);
    Some(match due {
        true => trim(limit).map_or(tally, |bytes| DiskTally { bytes, counted: now, trimmed: Some(now) }),
        false => tally,
    })
}

fn keep_within_disk_limit(cache: &Cache, added: i64) {
    UNTALLIED.fetch_add(added, Ordering::SeqCst);
    let held = match DISK_TALLY.try_lock() {
        Ok(held) => Some(held),
        Err(TryLockError::Poisoned(poisoned)) => Some(poisoned.into_inner()),
        Err(TryLockError::WouldBlock) => None,
    };
    if let Some(mut held) = held {
        *held = tallied(*held, UNTALLIED.swap(0, Ordering::SeqCst), disk_limit_bytes(), Instant::now(), || cache.default_bytes().ok(), |limit| cache.trim_to(limit).ok());
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
    if let Some(cache) = cache.filter(|_| persist)
        && cache.save(&Tile { hash, format: format.to_string(), image: image.clone(), kind }, None).unwrap_or(false)
    {
        keep_within_disk_limit(cache, image.len() as i64);
    }
    Some(image)
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(timeout)).build().into()
}

static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| agent(TILE_TIMEOUT));

pub fn fetch_over_http(request: &TileRequest) -> Result<Vec<u8>, String> {
    fetch_through(&AGENT, request)
}

fn fetch_through(agent: &ureq::Agent, request: &TileRequest) -> Result<Vec<u8>, String> {
    let asked = request.headers.iter().fold(agent.get(&request.url), |asked, (name, value)| asked.header(name, value));
    asked.call().map_err(|e| e.to_string())?.body_mut().read_to_vec().map_err(|e| e.to_string())
}

pub struct TileWorkers {
    pool: ThreadPool,
    in_flight: Arc<AtomicUsize>,
    limit: usize,
}

impl TileWorkers {
    pub fn start() -> Result<TileWorkers, ThreadPoolBuildError> {
        ThreadPoolBuilder::new().num_threads(TILE_THREADS).thread_name(|_| TILE_THREAD_NAME.to_string()).build().map(|pool| TileWorkers::over(pool, TILES_WAITING))
    }

    fn over(pool: ThreadPool, waiting: usize) -> TileWorkers {
        let limit = pool.current_num_threads() + waiting;
        TileWorkers { pool, in_flight: Arc::new(AtomicUsize::new(0)), limit }
    }

    fn run(&self, job: impl FnOnce() -> Option<Vec<u8>> + Send + 'static, reply: impl FnOnce(Option<Vec<u8>>) + Send + 'static) {
        if self.in_flight.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |held| (held < self.limit).then_some(held + 1)).is_err() {
            return reply(None);
        }
        let in_flight = Arc::clone(&self.in_flight);
        self.pool.spawn(move || {
            let image = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)).ok().flatten();
            in_flight.fetch_sub(1, Ordering::SeqCst);
            reply(image);
        });
    }
}

fn fetch_in_background(
    workers: &OnceCell<TileWorkers>,
    start: impl FnOnce() -> Result<TileWorkers, ThreadPoolBuildError>,
    job: impl FnOnce() -> Option<Vec<u8>> + Send + 'static,
    reply: impl FnOnce(Option<Vec<u8>>) + Send + 'static,
) {
    match workers.get_or_try_init(start) {
        Ok(workers) => workers.run(job, reply),
        Err(error) => {
            log::warn!("Map tile workers did not start: {error}");
            reply(None)
        }
    }
}

#[derive(Default)]
pub struct Tickets {
    issued: AtomicU64,
    open: Mutex<BTreeMap<u64, bool>>,
}

impl Tickets {
    pub const fn new() -> Tickets {
        Tickets { issued: AtomicU64::new(0), open: Mutex::new(BTreeMap::new()) }
    }

    fn held(&self) -> std::sync::MutexGuard<'_, BTreeMap<u64, bool>> {
        self.open.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn issue(&self) -> u64 {
        let ticket = self.issued.fetch_add(1, Ordering::SeqCst) + 1;
        self.held().insert(ticket, false);
        ticket
    }

    pub fn cancel(&self, ticket: u64) {
        if let Some(cancelled) = self.held().get_mut(&ticket) {
            *cancelled = true;
        }
    }

    fn cancelled(&self, ticket: u64) -> bool {
        self.held().get(&ticket).copied().unwrap_or(false)
    }

    fn settle(&self, ticket: u64) {
        self.held().remove(&ticket);
    }
}

pub fn fetch_ticketed(
    tickets: &'static Tickets,
    workers: &OnceCell<TileWorkers>,
    start: impl FnOnce() -> Result<TileWorkers, ThreadPoolBuildError>,
    fetch: impl FnOnce() -> Option<Vec<u8>> + Send + 'static,
    reply: impl FnOnce(Option<Vec<u8>>) + Send + 'static,
) -> u64 {
    let ticket = tickets.issue();
    let job = move || (!tickets.cancelled(ticket)).then(fetch).flatten();
    fetch_in_background(workers, start, job, move |image| {
        tickets.settle(ticket);
        reply(image)
    });
    ticket
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

    #[test]
    fn the_disk_limit_is_kept_by_a_running_tally_so_a_save_costs_no_full_read() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let unread = || -> Option<i64> { panic!("a fresh tally needs no full read") };
        let untrimmed = |_: i64| -> Option<i64> { panic!("nothing is trimmed under the limit or within two seconds of a trim") };
        let first = tallied(None, 20, 2_000, start, || Some(1_000), untrimmed);
        assert_eq!(first.map(|tally| tally.bytes), Some(1_000), "the first save counts the cache once, its own tile included");
        let filling = (1..=40).fold(first, |held, n| tallied(held, 20, 2_000, at(n * 10), unread, untrimmed));
        assert_eq!(filling.map(|tally| tally.bytes), Some(1_800));
        let trimmed = tallied(filling, 400, 2_000, at(500), unread, |limit| Some(limit - 100));
        assert_eq!(trimmed.map(|tally| (tally.bytes, tally.trimmed)), Some((1_900, Some(at(500)))), "going over trims to the limit and keeps the size the trim answered");
        let soon = tallied(trimmed, 300, 2_000, at(1_500), unread, untrimmed);
        assert_eq!(soon.map(|tally| tally.bytes), Some(2_200), "a cache already trimmed is left over its limit until two seconds pass, as before");
        assert_eq!(tallied(soon, 0, 2_000, at(2_500), unread, |limit| Some(limit)).map(|tally| tally.bytes), Some(2_000));
        let stale = at(500) + RECOUNT_EVERY;
        assert_eq!(tallied(soon, 20, 2_000, stale, || Some(700), untrimmed).map(|tally| (tally.bytes, tally.counted)), Some((700, stale)), "every few minutes the tally is recounted, so offline sets and deletions cannot let it drift");
    }

    #[test]
    fn a_tile_cancelled_while_it_waits_is_answered_with_nothing_and_never_fetched() {
        static TICKETS: Tickets = Tickets::new();
        let held = OnceCell::new();
        let (gate, wait) = std::sync::mpsc::channel::<()>();
        let (answer, answers) = std::sync::mpsc::channel();
        let first = answer.clone();
        fetch_ticketed(&TICKETS, &held, || Ok(workers(1, 4)), move || wait.recv().ok().map(|()| PNG.to_vec()), move |image| first.send(image).unwrap());
        let fetched = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let touched = fetched.clone();
        let ticket = fetch_ticketed(&TICKETS, &held, || unreachable!(), move || Some(PNG.to_vec()).filter(|_| !touched.swap(true, Ordering::SeqCst)), move |image| answer.send(image).unwrap());
        TICKETS.cancel(ticket);
        gate.send(()).unwrap();
        assert_eq!(answers.iter().collect::<Vec<_>>(), vec![Some(PNG.to_vec()), None], "the dropped tile still answers once, so the head's reply is released");
        assert!(!fetched.load(Ordering::SeqCst), "a tile the map panned away from never reaches the network");
        TICKETS.cancel(ticket);
        assert!(TICKETS.held().is_empty(), "an answered ticket is forgotten, so a late cancel changes nothing and tickets never pile up");
    }

    type Job = Box<dyn FnOnce() -> Option<Vec<u8>> + Send>;
    type Answers = std::sync::mpsc::Receiver<Option<Vec<u8>>>;

    fn workers(threads: usize, waiting: usize) -> TileWorkers {
        TileWorkers::over(ThreadPoolBuilder::new().num_threads(threads).build().unwrap(), waiting)
    }

    fn answered(workers: &TileWorkers, jobs: Vec<Job>) -> Answers {
        let (answer, answers) = std::sync::mpsc::channel();
        jobs.into_iter().for_each(|job| {
            let answer = answer.clone();
            workers.run(job, move |image| answer.send(image).unwrap());
        });
        answers
    }

    #[test]
    fn every_job_is_answered_exactly_once_even_when_it_panics() {
        let jobs: Vec<Job> = vec![Box::new(|| panic!("tile job failed")), Box::new(|| None), Box::new(|| Some(PNG.to_vec()))];
        let answers = answered(&workers(1, 3), jobs);
        assert_eq!(answers.iter().collect::<Vec<_>>(), vec![None, None, Some(PNG.to_vec())], "a panic answers nothing so the head releases its reply and falls back, and the worker lives on for the next job");
    }

    #[test]
    fn a_full_pool_answers_nothing_at_once_instead_of_queueing_the_job() {
        let (gates, waits): (Vec<_>, Vec<_>) = (0..3).map(|_| std::sync::mpsc::channel::<()>()).unzip();
        let jobs: Vec<Job> = waits.into_iter().map(|wait| Box::new(move || wait.recv().ok().map(|_| PNG.to_vec())) as Job).collect();
        let answers = answered(&workers(1, 1), jobs);
        assert_eq!(answers.recv().unwrap(), None, "one job running and one waiting fill a single worker with room for one more");
        gates[..2].iter().for_each(|gate| gate.send(()).unwrap());
        assert_eq!(answers.iter().collect::<Vec<_>>(), vec![Some(PNG.to_vec()), Some(PNG.to_vec())]);
    }

    #[test]
    fn a_server_that_never_answers_holds_its_worker_only_until_the_timeout() {
        assert_eq!(AGENT.config().timeouts().global, Some(TILE_TIMEOUT), "QGCTileFetchReply gives a tile ten seconds");
        let silent = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let request = TileRequest { url: format!("http://{}/1/2/3.png", silent.local_addr().unwrap()), headers: Vec::new() };
        let impatient = agent(Duration::from_millis(200));
        let started = Instant::now();
        let answers = answered(&workers(1, 1), vec![Box::new(move || fetch_through(&impatient, &request).ok()), Box::new(|| Some(PNG.to_vec()))]);
        assert_eq!(answers.recv_timeout(Duration::from_secs(10)).unwrap(), None);
        assert_eq!(answers.recv_timeout(Duration::from_secs(10)).unwrap(), Some(PNG.to_vec()), "the next job gets the worker back");
        assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
    }

    #[test]
    fn workers_that_cannot_start_answer_nothing_and_leave_the_next_job_to_start_them() {
        let held = OnceCell::new();
        let (answer, answers) = std::sync::mpsc::channel();
        let refused = || ThreadPoolBuilder::new().num_threads(1).spawn_handler(|_| Err(std::io::Error::other("no threads"))).build().map(|pool| TileWorkers::over(pool, 0));
        let first = answer.clone();
        fetch_in_background(&held, refused, || panic!("a pool that never started runs nothing"), move |image| first.send(image).unwrap());
        assert_eq!(answers.recv().unwrap(), None);
        assert!(held.get().is_none(), "a failed start is not kept, or every later job would be refused for the rest of the session");
        fetch_in_background(&held, || Ok(workers(1, 0)), || Some(PNG.to_vec()), move |image| answer.send(image).unwrap());
        assert_eq!(answers.recv().unwrap(), Some(PNG.to_vec()));
    }
}
