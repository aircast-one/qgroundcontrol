#[cfg(not(target_family = "wasm"))]
pub fn spawn(name: &str, job: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
    std::thread::Builder::new().name(name.to_string()).spawn(job).map(drop)
}

#[cfg(not(target_family = "wasm"))]
pub fn poll(name: &str, every: std::time::Duration, mut step: impl FnMut() -> bool + Send + 'static) -> std::io::Result<()> {
    spawn(name, move || {
        let _ = std::iter::repeat_with(|| {
            std::thread::sleep(every);
            step()
        })
        .find(|done| *done);
    })
}

#[cfg(target_family = "wasm")]
thread_local! {
    static QUEUED: std::cell::RefCell<Vec<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(Vec::new()) };
    static POLLING: std::cell::RefCell<Vec<Box<dyn FnMut() -> bool>>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(target_family = "wasm")]
pub fn spawn(_name: &str, job: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
    QUEUED.with(|queued| queued.borrow_mut().push(Box::new(job)));
    Ok(())
}

#[cfg(target_family = "wasm")]
pub fn poll(_name: &str, every: std::time::Duration, mut step: impl FnMut() -> bool + Send + 'static) -> std::io::Result<()> {
    let mut due = web_time::Instant::now() + every;
    POLLING.with(|polling| {
        polling.borrow_mut().push(Box::new(move || {
            let now = web_time::Instant::now();
            if now < due {
                return false;
            }
            due = now + every;
            step()
        }))
    });
    Ok(())
}

#[cfg(target_family = "wasm")]
pub fn run_queued() {
    let jobs = QUEUED.with(|queued| std::mem::take(&mut *queued.borrow_mut()));
    jobs.into_iter().for_each(|job| job());
    let steps = POLLING.with(|polling| std::mem::take(&mut *polling.borrow_mut()));
    let pending: Vec<Box<dyn FnMut() -> bool>> = steps.into_iter().filter_map(|mut step| (!step()).then_some(step)).collect();
    POLLING.with(|polling| {
        let added = std::mem::take(&mut *polling.borrow_mut());
        *polling.borrow_mut() = pending.into_iter().chain(added).collect();
    });
}

#[cfg(target_family = "wasm")]
pub async fn pause(wait: std::time::Duration) {
    use wasm_bindgen::JsCast;
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let scope: web_sys::WorkerGlobalScope = js_sys::global().unchecked_into();
        let _ = scope.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, i32::try_from(wait.as_millis()).unwrap_or(i32::MAX));
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}
