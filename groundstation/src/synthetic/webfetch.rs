use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::{Rc, Weak};
use std::time::Duration;
use web_time::Instant;

use url::Url;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use super::fetch::{Fetcher, Network};

const WAITING: &str = "waiting for the network";
const ANSWER_TIMEOUT_MS: u32 = 20_000;
const WORK_PER_TICK: Duration = Duration::from_millis(8);
const IN_FLIGHT: usize = 6;

enum Fetched {
    InFlight,
    Done(Result<Vec<u8>, String>),
}

type Store = Rc<RefCell<HashMap<String, Fetched>>>;

struct Attempt {
    store: Store,
    missed: bool,
    touched: Vec<String>,
}

thread_local! {
    static ATTEMPT: RefCell<Option<Attempt>> = const { RefCell::new(None) };
    static LINE: RefCell<Line> = RefCell::new(Line::default());
}

fn failed(error: wasm_bindgen::JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}

async fn download(url: &str, headers: &[(String, String)]) -> Result<Vec<u8>, String> {
    let sent = web_sys::Headers::new().map_err(failed)?;
    headers.iter().try_for_each(|(name, value)| sent.append(name, value)).map_err(failed)?;
    let init = web_sys::RequestInit::new();
    init.set_headers(&sent);
    init.set_signal(Some(&web_sys::AbortSignal::timeout_with_u32(ANSWER_TIMEOUT_MS)));
    let scope: web_sys::WorkerGlobalScope = js_sys::global().unchecked_into();
    let response: web_sys::Response = JsFuture::from(scope.fetch_with_str_and_init(url, &init)).await.map_err(failed)?.unchecked_into();
    if !response.ok() {
        return Err(format!("http status: {}", response.status()));
    }
    let body = JsFuture::from(response.array_buffer().map_err(failed)?).await.map_err(failed)?;
    Ok(js_sys::Uint8Array::new(&body).to_vec())
}

struct Download {
    store: Weak<RefCell<HashMap<String, Fetched>>>,
    url: String,
    headers: Vec<(String, String)>,
}

#[derive(Default)]
struct Line {
    busy: usize,
    queued: VecDeque<Download>,
}

fn queue(download: Download) {
    let start = LINE.with(|line| {
        let mut line = line.borrow_mut();
        match line.busy < IN_FLIGHT {
            true => {
                line.busy += 1;
                Some(download)
            }
            false => {
                line.queued.push_back(download);
                None
            }
        }
    });
    if let Some(download) = start {
        run(download);
    }
}

fn next() -> Option<Download> {
    LINE.with(|line| {
        let mut line = line.borrow_mut();
        let next = std::iter::from_fn(|| line.queued.pop_front()).find(|download| download.store.strong_count() > 0);
        if next.is_none() {
            line.busy = line.busy.saturating_sub(1);
        }
        next
    })
}

fn run(download: Download) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = self::download(&download.url, &download.headers).await;
        if let Some(store) = download.store.upgrade() {
            store.borrow_mut().insert(download.url, Fetched::Done(result));
        }
        if let Some(next) = next() {
            run(next);
        }
    });
}

impl Attempt {
    fn fetch(&mut self, url: &str, headers: &[(String, String)]) -> Result<Vec<u8>, String> {
        self.touched.push(url.to_string());
        let mut store = self.store.borrow_mut();
        match store.get(url) {
            Some(Fetched::Done(result)) => result.clone(),
            Some(Fetched::InFlight) => {
                self.missed = true;
                Err(WAITING.to_string())
            }
            None if self.missed => Err(WAITING.to_string()),
            None => {
                self.missed = true;
                store.insert(url.to_string(), Fetched::InFlight);
                queue(Download { store: Rc::downgrade(&self.store), url: url.to_string(), headers: headers.to_vec() });
                Err(WAITING.to_string())
            }
        }
    }
}

fn fetched(url: &str, headers: &[(String, String)]) -> Result<Vec<u8>, String> {
    ATTEMPT.with(|attempt| attempt.borrow_mut().as_mut().map_or_else(|| Err("the synthetic view fetches only inside its jobs".to_string()), |attempt| attempt.fetch(url, headers)))
}

impl Fetcher for Network {
    fn url(&self, url: &Url) -> Result<Vec<u8>, String> {
        fetched(url.as_str(), &[])
    }

    fn map(&self, provider: &str, z: u32, x: u32, y: u32) -> Result<Vec<u8>, String> {
        let signed = |value: u32| i32::try_from(value).map_err(|_| format!("tile coordinate {value} out of range"));
        let (sx, sy, sz) = (signed(x)?, signed(y)?, signed(z)?);
        let http = |request: &crate::mapurls::TileRequest| fetched(&request.url, &request.headers);
        crate::maptiles::fetch(provider, sx, sy, sz, &crate::mapurls::keys_from_settings(), None, false, &http)
            .and_then(|tile| if provider == crate::maptypes::BUILDING_TILES { crate::buildings::tile_json(&tile) } else { Some(tile) })
            .ok_or_else(|| format!("no {provider} tile {z}/{x}/{y}"))
    }
}

struct Waiting<J> {
    job: J,
    touched: Vec<String>,
}

pub struct Pool<J, D> {
    work: Box<dyn Fn(J) -> D>,
    store: Store,
    waiting: Vec<Waiting<J>>,
}

impl<J: Clone + 'static, D: 'static> Pool<J, D> {
    pub fn new(_workers: usize, _name: &str, work: impl Fn(J) -> D + Send + Sync + 'static, _lost: impl Fn(&J) -> D + Send + Sync + 'static) -> Result<Self, String> {
        Ok(Self { work: Box::new(work), store: Store::default(), waiting: Vec::new() })
    }

    pub fn send(&mut self, job: J) {
        self.waiting.push(Waiting { job, touched: Vec::new() });
    }

    fn ready(&self, waiting: &Waiting<J>) -> bool {
        let store = self.store.borrow();
        waiting.touched.iter().all(|url| !matches!(store.get(url), Some(Fetched::InFlight)))
    }

    fn attempt(&self, waiting: Waiting<J>) -> Result<D, Waiting<J>> {
        ATTEMPT.with(|attempt| *attempt.borrow_mut() = Some(Attempt { store: self.store.clone(), missed: false, touched: Vec::new() }));
        let done = (self.work)(waiting.job.clone());
        let Some(Attempt { missed, touched, .. }) = ATTEMPT.with(|attempt| attempt.borrow_mut().take()) else { return Err(waiting) };
        match missed {
            true => Err(Waiting { job: waiting.job, touched }),
            false => {
                let mut store = self.store.borrow_mut();
                touched.iter().for_each(|url| drop(store.remove(url)));
                Ok(done)
            }
        }
    }

    pub fn finished(&mut self) -> Vec<D> {
        let started = Instant::now();
        let (done, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut self.waiting)
            .into_iter()
            .map(|waiting| match started.elapsed() < WORK_PER_TICK && self.ready(&waiting) {
                true => self.attempt(waiting),
                false => Err(waiting),
            })
            .partition(Result::is_ok);
        self.waiting = waiting.into_iter().filter_map(Result::err).collect();
        done.into_iter().filter_map(Result::ok).collect()
    }

    pub fn room(&self, limit: usize) -> usize {
        limit.saturating_sub(self.waiting.len())
    }

    pub fn in_flight(&self) -> usize {
        self.waiting.len()
    }
}
