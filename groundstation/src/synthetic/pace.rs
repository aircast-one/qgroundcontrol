use std::time::Duration;
use web_time::Instant;

pub const SOFTWARE_PAINT_EVERY: Duration = Duration::from_millis(200);

pub fn paint_due(software: bool, last_paint: Option<Instant>, now: Instant) -> bool {
    !software
        || last_paint.is_none_or(|at| now.saturating_duration_since(at) >= SOFTWARE_PAINT_EVERY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_software_adapter_paints_at_most_five_times_a_second() {
        let start = Instant::now();
        assert!(paint_due(true, None, start));
        assert!(!paint_due(
            true,
            Some(start),
            start + Duration::from_millis(16)
        ));
        assert!(!paint_due(
            true,
            Some(start),
            start + Duration::from_millis(199)
        ));
        assert!(paint_due(true, Some(start), start + SOFTWARE_PAINT_EVERY));
        assert!(paint_due(
            false,
            Some(start),
            start + Duration::from_millis(16)
        ));
        assert!(paint_due(false, Some(start), start));
    }
}
