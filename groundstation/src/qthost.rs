use std::sync::atomic::{AtomicBool, Ordering};

static PRESENT: AtomicBool = AtomicBool::new(false);

pub fn declare() {
    PRESENT.store(true, Ordering::SeqCst);
}

pub fn present() -> bool {
    PRESENT.load(Ordering::SeqCst)
}
