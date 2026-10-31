use std::sync::atomic::{AtomicBool, Ordering};

static PRESENT: AtomicBool = AtomicBool::new(false);
static NATIVE_UI: AtomicBool = AtomicBool::new(false);

pub fn declare() {
    PRESENT.store(true, Ordering::SeqCst);
}

pub fn present() -> bool {
    PRESENT.load(Ordering::SeqCst)
}

pub fn declare_native_ui() {
    NATIVE_UI.store(true, Ordering::SeqCst);
}

pub fn native_ui() -> bool {
    NATIVE_UI.load(Ordering::SeqCst)
}
