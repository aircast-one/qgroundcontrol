use std::sync::{Mutex, PoisonError};

pub type Job = Box<dyn FnOnce() + Send>;

pub fn worker_pool(name: &str, workers: usize) -> std::sync::mpsc::Sender<Job> {
    let (sender, receiver) = std::sync::mpsc::channel::<Job>();
    let shared = std::sync::Arc::new(Mutex::new(receiver));
    (0..workers).for_each(|_| {
        let shared = std::sync::Arc::clone(&shared);
        let _ = std::thread::Builder::new().name(name.to_string()).spawn(move || {
            std::iter::from_fn(|| shared.lock().unwrap_or_else(PoisonError::into_inner).recv().ok()).for_each(|job| {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
            })
        });
    });
    sender
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};

    #[test]
    fn map_tiles_run_on_a_fixed_pool_so_a_burst_of_requests_cannot_exhaust_threads() {
        let queue = worker_pool("test", 3);
        let barrier = Arc::new(Barrier::new(3));
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let (done, finished) = std::sync::mpsc::channel();
        (0..9).for_each(|_| {
            let (barrier, active, peak, done) = (Arc::clone(&barrier), Arc::clone(&active), Arc::clone(&peak), done.clone());
            queue
                .send(Box::new(move || {
                    let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    barrier.wait();
                    active.fetch_sub(1, Ordering::SeqCst);
                    let _ = done.send(());
                }))
                .unwrap();
        });
        (0..9).for_each(|_| finished.recv().unwrap());
        assert_eq!(peak.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn a_panicking_tile_job_does_not_take_its_worker_down() {
        let queue = worker_pool("test", 1);
        let (done, finished) = std::sync::mpsc::channel();
        queue.send(Box::new(|| panic!("tile job failed"))).unwrap();
        queue.send(Box::new(move || done.send(()).unwrap())).unwrap();
        finished.recv().unwrap();
    }
}
