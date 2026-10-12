use std::cell::OnceCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::Duration;

use url::Url;

const MAX_BODY_BYTES: u64 = 64 * 1024 * 1024;
const NO_PERSISTENCE: &str = "settings.appSettings.disableAllPersistence";
const FETCH_TIMEOUT: Duration = Duration::from_secs(20);

static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| ureq::Agent::config_builder().timeout_global(Some(FETCH_TIMEOUT)).build().into());

pub trait Fetcher: Send + Sync {
    fn url(&self, url: &Url) -> Result<Vec<u8>, String>;
    fn map(&self, provider: &str, z: u32, x: u32, y: u32) -> Result<Vec<u8>, String>;
}

pub struct Network;

thread_local! {
    #[allow(clippy::missing_const_for_thread_local)]
    static CACHE: OnceCell<Option<crate::tilecache::Cache>> = const { OnceCell::new() };
}

impl Fetcher for Network {
    fn url(&self, url: &Url) -> Result<Vec<u8>, String> {
        AGENT.get(url.as_str()).call().map_err(|error| error.to_string())?.body_mut().with_config().limit(MAX_BODY_BYTES).read_to_vec().map_err(|error| error.to_string())
    }

    fn map(&self, provider: &str, z: u32, x: u32, y: u32) -> Result<Vec<u8>, String> {
        let persist = !crate::settingsstore::raw_setting(NO_PERSISTENCE).and_then(|value| value.as_bool()).unwrap_or(false);
        let signed = |value: u32| i32::try_from(value).map_err(|_| format!("tile coordinate {value} out of range"));
        let (sx, sy, sz) = (signed(x)?, signed(y)?, signed(z)?);
        CACHE.with(|cache| {
            let cache = cache.get_or_init(|| crate::terrainservice::cache_path().and_then(|path| crate::tilecache::Cache::open(&path).ok()));
            crate::maptiles::fetch_remembered(provider, sx, sy, sz, cache.as_ref(), persist).ok_or_else(|| format!("no {provider} tile {z}/{x}/{y}"))
        })
    }
}

pub struct Pool<J, D> {
    jobs: Sender<J>,
    done: Receiver<D>,
    in_flight: usize,
}

impl<J: Send + 'static, D: Send + 'static> Pool<J, D> {
    pub fn new(workers: usize, name: &str, work: impl Fn(J) -> D + Send + Sync + 'static, lost: impl Fn(&J) -> D + Send + Sync + 'static) -> Result<Self, String> {
        let (jobs, queue) = mpsc::channel::<J>();
        let (finished, done) = mpsc::channel::<D>();
        let queue = Arc::new(Mutex::new(queue));
        let shared = Arc::new((work, lost));
        (0..workers)
            .map(|index| {
                let (queue, finished, shared) = (queue.clone(), finished.clone(), shared.clone());
                std::thread::Builder::new().name(format!("{name}-{index}")).spawn(move || {
                    let next = || queue.lock().unwrap_or_else(PoisonError::into_inner).recv().ok();
                    while let Some(job) = next() {
                        let (work, lost) = (&shared.0, &shared.1);
                        let fallback = lost(&job);
                        let done = catch_unwind(AssertUnwindSafe(|| work(job))).unwrap_or(fallback);
                        if finished.send(done).is_err() {
                            break;
                        }
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("could not start {name} workers: {error}"))?;
        Ok(Self { jobs, done, in_flight: 0 })
    }

    pub fn send(&mut self, job: J) {
        if self.jobs.send(job).is_ok() {
            self.in_flight += 1;
        }
    }

    pub fn finished(&mut self) -> Vec<D> {
        let done: Vec<D> = self.done.try_iter().collect();
        self.in_flight = self.in_flight.saturating_sub(done.len());
        done
    }

    pub fn room(&self, limit: usize) -> usize {
        limit.saturating_sub(self.in_flight)
    }

    pub fn in_flight(&self) -> usize {
        self.in_flight
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn drained<D: Send + 'static>(pool: &mut Pool<u32, D>, expected: usize) -> Vec<D> {
        let deadline = Instant::now() + Duration::from_secs(10);
        std::iter::repeat_with(|| pool.finished()).take_while(|_| Instant::now() < deadline).flatten().take(expected).collect()
    }

    #[test]
    fn a_job_that_panics_still_reports_back_so_the_budget_never_leaks() {
        let mut pool = Pool::new(2, "test-pool", |job: u32| if job == 2 { panic!("boom") } else { Ok(job) }, |job: &u32| Err(format!("job {job} panicked"))).expect("workers");
        (1..=3).for_each(|job| pool.send(job));
        let seen: Vec<Result<u32, String>> = drained(&mut pool, 3);
        assert_eq!(seen.iter().filter(|done| done.is_ok()).count(), 2);
        assert!(seen.contains(&Err("job 2 panicked".to_string())));
        assert_eq!(pool.in_flight(), 0);
        assert_eq!(pool.room(4), 4);
    }

    #[test]
    fn workers_fetch_side_by_side_instead_of_one_at_a_time() {
        let (signal, heard) = mpsc::channel::<()>();
        let heard = Mutex::new(heard);
        let work = move |job: u32| match job {
            0 => heard.lock().unwrap_or_else(PoisonError::into_inner).recv_timeout(Duration::from_secs(5)).is_ok(),
            _ => signal.send(()).is_ok(),
        };
        let mut pool = Pool::new(2, "test-pair", work, |_: &u32| false).expect("workers");
        (0..2).for_each(|job| pool.send(job));
        assert_eq!(drained(&mut pool, 2), vec![true, true], "the waiting job heard its neighbour, so both ran at once");
    }
}
