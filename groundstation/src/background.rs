#[cfg(not(target_family = "wasm"))]
pub fn spawn(name: &str, job: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
    std::thread::Builder::new().name(name.to_string()).spawn(job).map(drop)
}

#[cfg(target_family = "wasm")]
thread_local! {
    static QUEUED: std::cell::RefCell<Vec<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(target_family = "wasm")]
pub fn spawn(_name: &str, job: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
    QUEUED.with(|queued| queued.borrow_mut().push(Box::new(job)));
    Ok(())
}

#[cfg(target_family = "wasm")]
pub fn run_queued() {
    let jobs = QUEUED.with(|queued| std::mem::take(&mut *queued.borrow_mut()));
    jobs.into_iter().for_each(|job| job());
}
