use std::io::Write;
use std::sync::{Mutex, PoisonError};

use serde_json::Value;

use crate::router::Backend;

const SAMPLE_MS: u64 = 1000;
const BASE_WIDTH: u32 = 640;
const BASE_FONT_SIZE: u32 = 12;
const OFFSET_FACTOR: i64 = 100;
const ROWS: usize = 3;
const FALLBACK_SIZE: (u32, u32) = (1280, 720);

pub fn path_for(video_file: &str) -> String {
    let video = std::path::Path::new(video_file);
    let stem = video.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    video.with_file_name(format!("{stem}.ass")).to_string_lossy().into_owned()
}

pub fn header(width: u32, height: u32) -> String {
    let font = width * BASE_FONT_SIZE / BASE_WIDTH;
    format!(
        "[Script Info]\nTitle: QGroundControl Subtitle Telemetry file\nScriptType: v4.00+\nWrapStyle: 0\nScaledBorderAndShadow: yes\nYCbCr Matrix: TV.601\nPlayResX: {width}\nPlayResY: {height}\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Monospace,{font},&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,2,1,10,10,10,1\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n"
    )
}

pub fn timestamp(ms: u64) -> String {
    format!("{}:{:02}:{:02}.{:02}", ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000 / 10)
}

pub fn events(start_ms: u64, end_ms: u64, rows: &[(String, String)], size: (u32, u32), date: &str) -> String {
    let (width, height) = (i64::from(size.0), i64::from(size.1));
    let row_width = (width + OFFSET_FACTOR) / (ROWS as i64 + 1);
    let per_row = rows.len().div_ceil(ROWS);
    let (start, end) = (timestamp(start_ms), timestamp(end_ms));
    let columns: String = (0..ROWS)
        .map(|i| {
            let chunk: Vec<&(String, String)> = rows.iter().skip(i * per_row).take(per_row).collect();
            let names = chunk.iter().map(|(name, _)| format!("{name}:")).collect::<Vec<_>>().join("\\N");
            let values = chunk.iter().map(|(_, value)| value.clone()).collect::<Vec<_>>().join("\\N");
            let x = -OFFSET_FACTOR / 2 + row_width * (i as i64 + 1);
            format!(
                "Dialogue: 0,{start},{end},Default,,0,0,0,,{{\\an3\\pos({},{})}}{names}\nDialogue: 0,{start},{end},Default,,0,0,0,,{{\\pos({x},{})}}{values}\n",
                x - 10,
                height - 30,
                height - 30,
            )
        })
        .collect();
    format!("{columns}Dialogue: 0,{start},{end},Default,,0,0,0,,{{\\pos(10,35)}}{date}\n")
}

struct Capture {
    file: std::fs::File,
    size: (u32, u32),
    started_ms: u64,
    written_ms: u64,
    shown: Option<Vec<String>>,
}

static CAPTURE: Mutex<Option<Capture>> = Mutex::new(None);

const CHOSEN_KEY: &str = "Subtitles/instruments";

pub const SET_INSTRUMENTS: &str = "subtitles.setInstruments";

pub fn set_instruments(vehicle_class: &str, chosen: &str) {
    crate::settingsstore::written(&format!("{CHOSEN_KEY}/{vehicle_class}"), chosen);
}

fn chosen_for(vehicle_class: &str) -> Vec<String> {
    crate::settingsstore::stored_text(&format!("{CHOSEN_KEY}/{vehicle_class}"))
        .map(|csv| csv.split(',').filter(|n| !n.is_empty()).map(str::to_string).collect())
        .unwrap_or_default()
}

pub fn start(video_file: &str, size: Option<(u32, u32)>, now_ms: u64) {
    let size = size.filter(|(w, h)| *w > 0 && *h > 0).unwrap_or(FALLBACK_SIZE);
    let opened = std::fs::File::create(path_for(video_file)).and_then(|mut file| file.write_all(header(size.0, size.1).as_bytes()).map(|()| file));
    *CAPTURE.lock().unwrap_or_else(PoisonError::into_inner) = match opened {
        Ok(file) => Some(Capture { file, size, started_ms: now_ms, written_ms: 0, shown: None }),
        Err(error) => {
            log::warn!("Unable to write subtitle data to file: {error}");
            None
        }
    };
}

pub fn stop() {
    *CAPTURE.lock().unwrap_or_else(PoisonError::into_inner) = None;
}

fn rows(backend: &dyn Backend, shown: &[String]) -> Vec<(String, String)> {
    let view = crate::instruments::instruments_view(backend, shown);
    view["items"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|item| {
            let text = |key: &str| item.get(key).and_then(Value::as_str).unwrap_or("").to_string();
            (text("label"), format!("{} {}", text("value"), text("units")).trim_end().to_string())
        })
        .collect()
}

pub fn tick(backend: &dyn Backend, now_ms: u64) {
    let due = CAPTURE.lock().unwrap_or_else(PoisonError::into_inner).as_ref().and_then(|active| {
        let elapsed = now_ms.saturating_sub(active.started_ms) / SAMPLE_MS * SAMPLE_MS;
        (elapsed > active.written_ms).then(|| (active.written_ms, elapsed, active.size, active.shown.clone()))
    });
    let Some((start, end, size, chosen)) = due else { return };
    if !crate::vehiclefacade::switched_on() || crate::hub::lock().active().is_none() {
        return;
    }
    let shown = chosen.unwrap_or_else(|| {
        let class = crate::instruments::instruments_view(backend, &[])["vehicleClass"].as_str().unwrap_or("generic").to_string();
        chosen_for(&class)
    });
    let date = chrono::Local::now().format("%x").to_string();
    let text = events(start, end, &rows(backend, &shown), size, &date);
    if let Some(active) = CAPTURE.lock().unwrap_or_else(PoisonError::into_inner).as_mut().filter(|active| active.written_ms == start) {
        active.written_ms = end;
        active.shown = Some(shown);
        let _ = active.file.write_all(text.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_sits_beside_the_video_with_qgcs_header() {
        assert_eq!(path_for("/v/2026-10-02_10.00.00.mkv"), "/v/2026-10-02_10.00.00.ass");
        assert!(header(1280, 720).contains("PlayResX: 1280\nPlayResY: 720"));
        assert!(header(1280, 720).contains("Style: Default,Monospace,24,"), "the font scales with the width from 12pt at 640");
    }

    #[test]
    fn a_second_of_telemetry_is_three_columns_and_the_date_like_subtitlewriter() {
        let rows: Vec<(String, String)> = ["Alt", "Speed", "Dist", "Climb"].iter().map(|n| (n.to_string(), "1 m".to_string())).collect();
        let written = events(0, 1000, &rows, (640, 480), "10/2/26");
        assert_eq!(timestamp(3_723_450), "1:02:03.45", "H:mm:ss.zz, the last digit chopped as QTime's zzz is");
        assert_eq!(written.lines().count(), 7, "a names and a values line per column, then the date");
        assert!(written.starts_with("Dialogue: 0,0:00:00.00,0:00:01.00,Default,,0,0,0,,{\\an3\\pos(125,450)}Alt:\\NSpeed:\n"));
        assert!(written.ends_with("{\\pos(10,35)}10/2/26\n"));
    }
}
