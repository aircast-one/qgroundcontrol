use std::sync::atomic::{AtomicU64, Ordering};

static LAST_PANIC_REPORT_MS: AtomicU64 = AtomicU64::new(0);
const PANIC_REPORT_EVERY_MS: u64 = 10_000;

fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|text| (*text).to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_default()
}

pub fn guarded(step: &str, work: impl FnOnce()) {
    let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) else {
        return;
    };
    let now = crate::hub::now_ms();
    let last = LAST_PANIC_REPORT_MS.load(Ordering::Relaxed);
    let due = last == 0 || now.saturating_sub(last) >= PANIC_REPORT_EVERY_MS;
    if due
        && LAST_PANIC_REPORT_MS
            .compare_exchange(last, now.max(1), Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    {
        log::error!(
            "pump step {step} panicked and was skipped this tick: {}",
            panic_message(panic.as_ref())
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panicking_pump_step_is_skipped_and_the_next_step_still_runs() {
        let ran = std::sync::atomic::AtomicBool::new(false);
        guarded("broken", || panic!("boom"));
        guarded("heartbeat", || {
            ran.store(true, std::sync::atomic::Ordering::Relaxed)
        });
        assert!(ran.load(std::sync::atomic::Ordering::Relaxed));
        let caught = std::panic::catch_unwind(|| panic!("index out of range")).unwrap_err();
        assert_eq!(panic_message(caught.as_ref()), "index out of range");
        let formatted = std::panic::catch_unwind(|| panic!("{} of {}", 3, 2)).unwrap_err();
        assert_eq!(panic_message(formatted.as_ref()), "3 of 2");
    }
}
