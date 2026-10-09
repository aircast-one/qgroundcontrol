use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::router::Backend;

const SAMPLE_MS: u64 = 1000;

struct Sampled {
    last: Option<(u64, i64)>,
    text: String,
}

static STATS: Mutex<Sampled> = Mutex::new(Sampled { last: None, text: String::new() });

pub fn format_video_stats(latency_ms: i64, fps: i64, height: i64) -> String {
    [(latency_ms >= 0).then(|| format!("{latency_ms} ms")), Some(format!("{fps} fps")), (height > 0).then(|| format!("{height}p"))]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ")
}

pub fn fps_between(from: (u64, i64), to: (u64, i64)) -> i64 {
    let elapsed = to.0.saturating_sub(from.0).max(1) as i64;
    ((to.1 - from.1).max(0) * 1000 + elapsed / 2) / elapsed
}

pub fn sample(running: bool, frames: i64, height: i64, now_ms: u64) {
    let mut stats = STATS.lock().unwrap_or_else(PoisonError::into_inner);
    match (running && frames > 0, stats.last) {
        (false, _) => *stats = Sampled { last: None, text: String::new() },
        (true, None) => stats.last = Some((now_ms, frames)),
        (true, Some(last)) if now_ms.saturating_sub(last.0) >= SAMPLE_MS => {
            *stats = Sampled { last: Some((now_ms, frames)), text: format_video_stats(-1, fps_between(last, (now_ms, frames)), height) };
        }
        (true, Some(_)) => {}
    }
}

pub fn video_stats_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    json!({ "kind": "object", "class": "VideoStats", "text": STATS.lock().unwrap_or_else(PoisonError::into_inner).text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_read_like_video_manager_and_leave_out_unknowns() {
        assert_eq!(format_video_stats(-1, 30, 720), "30 fps · 720p");
        assert_eq!(format_video_stats(85, 29, 0), "85 ms · 29 fps");
        assert_eq!(fps_between((0, 100), (1000, 130)), 30);
        assert_eq!(fps_between((0, 100), (2000, 159)), 30);
    }

    #[test]
    fn every_head_samples_the_stats_through_its_native_video_report() {
        sample(true, 1, 720, 0);
        sample(true, 31, 720, SAMPLE_MS);
        assert_eq!(STATS.lock().unwrap_or_else(PoisonError::into_inner).text, "30 fps · 720p");
        crate::videohost::invoke("video.reportNative", &json!([false, 0, 0, 0, "", null, false, crate::videohost::MAIN_CHANNEL]).to_string());
        assert_eq!(STATS.lock().unwrap_or_else(PoisonError::into_inner).text, "", "a stopped main channel clears the pill on iOS and macOS too, not only through the Android driver");
    }
}
