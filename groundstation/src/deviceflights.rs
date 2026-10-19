use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use chrono::TimeZone;
use serde_json::{Value, json};

use crate::router::Backend;

pub const FLIGHT_CHART: &str = "deviceFlight.chart";
pub const FLIGHT_RENDER: &str = "deviceFlight.render";
pub const RENDER_CHANGED: &str = "core.deviceRender@changed";
pub const RENDER_DEPS: &[&str] = &[RENDER_CHANGED];

const PLOT_TOP: f64 = 5.0;
const PLOT_HEIGHT: f64 = 90.0;
const PLOT_BOTTOM: f64 = PLOT_TOP + PLOT_HEIGHT;
const GRID_FRACTIONS: [f64; 3] = [0.25, 0.5, 0.75];
const MAX_TICKS: i64 = 5;
const SECONDS_PER_DAY: i64 = 86_400;
const DEVICE_LEVEL_NEUTRAL: i64 = 3;
const UNREAD_SAMPLE: &str = "no reading";
const CHART_TAKES: &str = "deviceFlight.chart takes a flight's trace as the device serves it, then the zoom's from and to as fractions of the flight, or nothing for the whole flight.";
const RENDER_TAKES: &str =
    "deviceFlight.render takes the camera, from and to of a sortie's share version.";
const RENDER_POLL: Duration = Duration::from_secs(2);
const CLOUD_FAILED: &str = "The cloud could not render this video.";
const NOT_SENT: &str = "Could not send the video.";
const LOST_TRACK: &str = "Lost track of the render.";
const BURNING_IN: &str =
    "Burning the telemetry in… this takes a few minutes; you can leave this page.";

fn text<'a>(json: &'a Value, key: &str) -> &'a str {
    json.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn number(json: &Value, key: &str, fallback: f64) -> f64 {
    json.get(key).and_then(Value::as_f64).unwrap_or(fallback)
}

fn integer(json: &Value, key: &str) -> i64 {
    json.get(key).and_then(Value::as_i64).unwrap_or_default()
}

fn list<'a>(json: &'a Value, key: &str) -> &'a [Value] {
    json.get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn objects<'a>(json: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    list(json, key).iter().filter(|it| it.is_object())
}

fn tenth(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn percent(part: f64, of: f64) -> f64 {
    tenth(part / of * 100.0)
}

fn window(count: usize, zoom: Option<(f64, f64)>) -> Range<usize> {
    match zoom {
        Some((low, high)) if count >= 2 => {
            let last = (count - 1) as f64;
            let from = (low * last).round().clamp(0.0, last - 1.0) as usize;
            let to = (high * last).round().clamp(from as f64 + 1.0, last) as usize;
            from..to + 1
        }
        _ => 0..count,
    }
}

fn scaled(values: &[f64], max: f64) -> bool {
    max > 0.0 || values.iter().any(|value| *value > 0.0)
}

fn ceiling(values: &[f64], max: f64) -> f64 {
    let peak = values.iter().copied().fold(0.0, f64::max);
    match (max > 0.0, peak > 0.0) {
        (true, _) => max,
        (false, true) => peak,
        (false, false) => 1.0,
    }
}

fn lines(values: &[f64], ceiling: f64) -> Vec<Vec<[f64; 2]>> {
    let last = (values.len().max(2) - 1) as f64;
    let points: Vec<Option<[f64; 2]>> = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            (*value >= 0.0).then(|| {
                [
                    percent(index as f64, last),
                    tenth(PLOT_BOTTOM - PLOT_HEIGHT * value.min(ceiling) / ceiling),
                ]
            })
        })
        .collect();
    points
        .split(Option::is_none)
        .filter(|run| run.len() > 1)
        .map(|run| run.iter().flatten().copied().collect())
        .collect()
}

fn rate(bytes_per_second: f64) -> String {
    match bytes_per_second * 8.0 {
        bits if bits >= 1e6 => format!("{:.1} Mbps", bits / 1e6),
        bits if bits >= 1e3 => format!("{} kbps", (bits / 1e3).round()),
        bits => format!("{} bps", bits.round()),
    }
}

fn reading(value: f64, unit: &str, imperial: bool) -> String {
    match unit {
        "rate" => rate(value),
        "meters" => crate::missionsummary::distance_text(value, imperial),
        "count" => format!("{}", value.round()),
        "mb" if value >= 1024.0 => format!("{:.1} GB", value / 1024.0),
        "mb" => format!("{} MB", value.round()),
        "seconds" if value >= 60.0 => format!("{} m", (value / 60.0).round()),
        "seconds" => format!("{} s", value.round()),
        "number" => format!("{}", tenth(value)),
        _ => format!("{}%", value.round()),
    }
}

fn sample_text(value: f64, note: &str, unit: &str, imperial: bool) -> String {
    let read = match value < 0.0 {
        true => UNREAD_SAMPLE.to_string(),
        false => reading(value, unit, imperial),
    };
    match note.is_empty() {
        true => read,
        false => format!("{read} · {note}"),
    }
}

fn clock(unix: i64, tz_offset: i64, with_seconds: bool) -> String {
    let day = (unix + tz_offset).rem_euclid(SECONDS_PER_DAY);
    let minutes = format!("{:02}:{:02}", day / 3600, day % 3600 / 60);
    match with_seconds {
        true => format!("{minutes}:{:02}", day % 60),
        false => minutes,
    }
}

fn span_label(seconds: i64) -> String {
    let minutes = (seconds as f64 / 60.0).round() as i64;
    match minutes >= 60 {
        true => format!("{}h {}m", minutes / 60, minutes % 60),
        false => format!("{minutes}m"),
    }
}

fn grid(ceiling: f64, unit: &str, imperial: bool) -> Vec<Value> {
    let labels: Vec<String> = GRID_FRACTIONS
        .iter()
        .map(|fraction| reading(ceiling * fraction, unit, imperial))
        .collect();
    let distinct = labels
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        == labels.len();
    GRID_FRACTIONS
        .iter()
        .zip(labels)
        .filter(|_| distinct)
        .map(|(fraction, label)| json!({ "y": PLOT_BOTTOM - PLOT_HEIGHT * fraction, "label": label }))
        .collect()
}

fn ticks(start: i64, step: i64, count: usize, tz_offset: i64) -> Vec<Value> {
    let span = step * (count as i64 - 1);
    let want = (span / 60 + 1).clamp(2, MAX_TICKS);
    match count > 1 && step > 0 {
        false => Vec::new(),
        true => (0..want)
            .map(|i| {
                json!({ "x": percent(i as f64, (want - 1) as f64), "label": clock(start + span * i / (want - 1), tz_offset, false) })
            })
            .collect(),
    }
}

fn markers(trace: &Value, step: i64, count: usize, shift: i64) -> Vec<Value> {
    let span = step * (count as i64 - 1);
    objects(trace, "markers")
        .enumerate()
        .filter_map(|(index, marker)| {
            let offset = integer(marker, "offset") - shift;
            (span > 0 && (0..=span).contains(&offset)).then(|| {
                json!({
                    "x": percent(offset as f64, span as f64),
                    "label": text(marker, "label"),
                    "level": marker.get("level").and_then(Value::as_i64).unwrap_or(DEVICE_LEVEL_NEUTRAL),
                    "index": index,
                })
            })
        })
        .collect()
}

fn lane(lane: &Value, trace: &Value, zoom: Option<(f64, f64)>, imperial: bool) -> Value {
    let all: Vec<f64> = list(lane, "values")
        .iter()
        .map(|value| value.as_f64().unwrap_or(-1.0))
        .collect();
    let shown = window(all.len(), zoom);
    let values = &all[shown.clone()];
    let notes: Vec<&str> = list(lane, "notes")
        .iter()
        .skip(shown.start)
        .map(|note| note.as_str().unwrap_or_default())
        .collect();
    let step = integer(trace, "step");
    let tz_offset = integer(trace, "tzOffset");
    let start = integer(trace, "startUnix") + shown.start as i64 * step;
    let top = ceiling(values, number(lane, "max", 0.0));
    let unit = text(lane, "unit");
    let readings: Vec<String> = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            format!(
                "{} · {}",
                clock(start + index as i64 * step, tz_offset, true),
                sample_text(
                    *value,
                    notes.get(index).copied().unwrap_or_default(),
                    unit,
                    imperial
                )
            )
        })
        .collect();
    let span = match values.len() > 1 {
        true => format!("{} of flight", span_label(step * (values.len() as i64 - 1))),
        false => String::new(),
    };
    json!({
        "key": text(lane, "key"),
        "label": text(lane, "label"),
        "chip": text(lane, "chip"),
        "peak": text(lane, "peak"),
        "default": lane.get("default").and_then(Value::as_bool).unwrap_or(false),
        "lines": lines(values, top),
        "grid": scaled(values, number(lane, "max", 0.0)).then(|| grid(top, unit, imperial)).unwrap_or_default(),
        "markers": markers(trace, step, values.len(), shown.start as i64 * step),
        "readings": readings,
        "ticks": ticks(start, step, values.len(), tz_offset),
        "span": span,
    })
}

fn chart_of(trace: &Value, zoom: Option<(f64, f64)>, imperial: bool) -> Value {
    json!({
        "lanes": objects(trace, "lanes").map(|it| lane(it, trace, zoom, imperial)).collect::<Vec<Value>>(),
        "spans": objects(trace, "spans")
            .map(|span| json!({ "label": text(span, "label"), "from": number(span, "from", 0.0), "to": number(span, "to", 1.0) }))
            .collect::<Vec<Value>>(),
    })
}

fn answer(args: &str, imperial: bool) -> Value {
    let args = serde_json::from_str::<Vec<Value>>(args).unwrap_or_default();
    let zoom = args
        .get(1)
        .and_then(Value::as_f64)
        .zip(args.get(2).and_then(Value::as_f64));
    match args.first().filter(|trace| trace.is_object()) {
        Some(trace) => json!({ "ok": true, "result": chart_of(trace, zoom, imperial) }),
        None => json!({ "ok": false, "reason": CHART_TAKES }),
    }
}

pub fn chart(backend: &dyn Backend, args: &str) -> Value {
    answer(args, crate::missionsummary::imperial(backend))
}

type Share = [String; 3];

#[derive(Clone, Debug, Default, PartialEq)]
struct Render {
    id: String,
    state: String,
    sent: u64,
    total: u64,
    url: String,
    expires_at: String,
    error: String,
}

impl Render {
    fn running(&self) -> bool {
        matches!(self.state.as_str(), "uploading" | "rendering")
    }
}

struct Renders {
    jobs: BTreeMap<Share, Render>,
    dirty: bool,
}

static RENDERS: Mutex<Renders> = Mutex::new(Renders {
    jobs: BTreeMap::new(),
    dirty: false,
});

fn renders() -> MutexGuard<'static, Renders> {
    RENDERS.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn take_render_changed() -> bool {
    std::mem::take(&mut renders().dirty)
}

fn failed(error: &str) -> Render {
    Render {
        state: "failed".into(),
        error: error.into(),
        ..Render::default()
    }
}

fn job(json: &Value) -> Render {
    let state = text(json, "state");
    Render {
        id: text(json, "id").into(),
        state: state.into(),
        sent: json.get("sent").and_then(Value::as_u64).unwrap_or_default(),
        total: json
            .get("total")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
        url: text(json, "url").into(),
        expires_at: text(json, "expiresAt").into(),
        error: match (state, text(json, "error")) {
            ("failed", "") => CLOUD_FAILED.into(),
            ("failed", error) => error.into(),
            _ => String::new(),
        },
    }
}

fn started(answer: &Value) -> Render {
    match answer["ok"] == true {
        true => job(&answer["result"]),
        false => failed(
            Some(text(answer, "reason"))
                .filter(|reason| !reason.is_empty())
                .unwrap_or(NOT_SENT),
        ),
    }
}

fn polled(answer: &Value) -> Render {
    match answer["ok"] == true {
        true => job(&answer["result"]),
        false => failed(LOST_TRACK),
    }
}

fn record(share: &Share, job: Render) -> Option<String> {
    let next = (job.running() && !job.id.is_empty()).then(|| job.id.clone());
    let mut held = renders();
    held.jobs.insert(share.clone(), job);
    held.dirty = true;
    next
}

fn claim(share: &Share) -> bool {
    let mut held = renders();
    let free = !held.jobs.get(share).is_some_and(Render::running);
    if free {
        held.jobs.insert(
            share.clone(),
            Render {
                state: "uploading".into(),
                ..Render::default()
            },
        );
        held.dirty = true;
    }
    free
}

fn start_args(share: &Share) -> String {
    json!(["POST", "/api/renders", { "camera": share[0], "from": share[1], "to": share[2] }])
        .to_string()
}

fn read_args(id: &str) -> String {
    let segment = url::form_urlencoded::byte_serialize(id.as_bytes())
        .collect::<String>()
        .replace('+', "%20");
    json!([format!("/api/renders/{segment}")]).to_string()
}

#[cfg(not(target_family = "wasm"))]
fn render(share: &Share, every: Duration, send: impl Fn(&str, &str) -> Value) {
    let mut polling = record(
        share,
        started(&send(crate::deviceviews::CALL, &start_args(share))),
    );
    while let Some(id) = polling {
        std::thread::sleep(every);
        polling = record(
            share,
            polled(&send(crate::deviceviews::READ, &read_args(&id))),
        );
    }
}

#[cfg(not(target_family = "wasm"))]
fn follow_render(share: Share) {
    render(&share, RENDER_POLL, crate::deviceviews::run);
}

#[cfg(target_family = "wasm")]
fn follow_render(share: Share) {
    wasm_bindgen_futures::spawn_local(async move {
        let mut polling = record(
            &share,
            started(
                &crate::deviceviews::run_later(crate::deviceviews::CALL, &start_args(&share)).await,
            ),
        );
        while let Some(id) = polling {
            crate::background::pause(RENDER_POLL).await;
            polling = record(
                &share,
                polled(
                    &crate::deviceviews::run_later(crate::deviceviews::READ, &read_args(&id)).await,
                ),
            );
        }
    });
}

fn share_of(args: &str) -> Option<Share> {
    let given: Vec<String> = serde_json::from_str::<Vec<Value>>(args)
        .unwrap_or_default()
        .iter()
        .map(|it| it.as_str().unwrap_or_default().to_string())
        .collect();
    Share::try_from(given)
        .ok()
        .filter(|share| share.iter().all(|part| !part.is_empty()))
}

pub fn request_render(args: &str) -> Value {
    match share_of(args) {
        None => json!({ "ok": false, "reason": RENDER_TAKES }),
        Some(share) => {
            let followed = share.clone();
            if claim(&share)
                && crate::background::spawn("groundstation-device-render", move || {
                    follow_render(followed)
                })
                .is_err()
            {
                record(&share, failed(NOT_SENT));
            }
            json!({ "ok": true })
        }
    }
}

fn progress(job: &Render) -> String {
    match job.state.as_str() {
        "uploading" => format!(
            "Sending to Aircast cloud… {}%",
            (job.sent * 100).checked_div(job.total).unwrap_or_default()
        ),
        "rendering" => BURNING_IN.to_string(),
        _ => String::new(),
    }
}

fn expiry<Tz: TimeZone>(expires_at: &str, zone: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    chrono::DateTime::parse_from_rfc3339(expires_at)
        .map(|at| {
            format!(
                "Link works until {}",
                at.with_timezone(zone).format("%-d %b")
            )
        })
        .unwrap_or_default()
}

fn progress_view<Tz: TimeZone>(job: &Render, zone: &Tz) -> Value
where
    Tz::Offset: std::fmt::Display,
{
    let done = job.state == "done";
    json!({
        "kind": "object",
        "class": "DeviceRender",
        "canStart": matches!(job.state.as_str(), "" | "failed"),
        "progress": progress(job),
        "error": job.error,
        "url": if done { job.url.as_str() } else { "" },
        "expiry": if done { expiry(&job.expires_at, zone) } else { String::new() },
    })
}

pub fn render_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let share: Share = [0, 1, 2].map(|index| args.get(index).cloned().unwrap_or_default());
    let job = renders().jobs.get(&share).cloned().unwrap_or_default();
    progress_view(&job, &chrono::Local)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(values: Value, unit: &str, max: f64) -> Value {
        json!({
            "step": 30,
            "startUnix": 1_700_000_000,
            "tzOffset": 0,
            "lanes": [{ "key": "signal", "label": "Modem signal", "chip": "Signal", "values": values, "notes": [], "unit": unit, "max": max, "default": true, "peak": "41%" }],
        })
    }

    fn first_lane(chart: &Value) -> &Value {
        &chart["lanes"][0]
    }

    #[test]
    fn the_line_breaks_where_the_value_is_unknown() {
        assert_eq!(
            lines(&[100.0, 50.0, -1.0, 0.0, 100.0], 100.0),
            vec![
                vec![[0.0, 5.0], [25.0, 50.0]],
                vec![[75.0, 95.0], [100.0, 5.0]]
            ]
        );
        let chart = chart_of(
            &trace(json!([100, 50, null, 0, 100]), "percent", 100.0),
            None,
            false,
        );
        assert_eq!(
            first_lane(&chart)["lines"],
            json!([[[0.0, 5.0], [25.0, 50.0]], [[75.0, 95.0], [100.0, 5.0]]])
        );
    }

    #[test]
    fn without_a_ceiling_the_line_scales_to_its_own_peak() {
        let values = [0.0, 250_000.0, 500_000.0];
        assert_eq!(
            lines(&values, ceiling(&values, 0.0)),
            vec![vec![[0.0, 95.0], [50.0, 50.0], [100.0, 5.0]]]
        );
        assert_eq!(
            ceiling(&[-1.0, 0.0], 0.0),
            1.0,
            "a lane with nothing above zero still has a scale"
        );
    }

    #[test]
    fn a_single_point_draws_nothing() {
        assert!(lines(&[42.0], 100.0).is_empty());
        assert!(lines(&[], 100.0).is_empty());
    }

    #[test]
    fn a_lane_gets_evenly_spaced_clock_ticks_from_first_to_last_sample() {
        let ticked = ticks(1_700_000_000, 30, 13, 0);
        assert_eq!(
            ticked
                .iter()
                .map(|tick| tick["x"].as_f64().unwrap())
                .collect::<Vec<f64>>(),
            vec![0.0, 25.0, 50.0, 75.0, 100.0]
        );
        assert_eq!(
            (ticked[0]["label"].as_str(), ticked[4]["label"].as_str()),
            (Some("22:13"), Some("22:19"))
        );
        assert!(ticks(1, 30, 1, 0).is_empty());
        assert_eq!(
            ticks(1_700_000_000, 30, 2, 0).len(),
            2,
            "a short span still marks both ends"
        );
    }

    #[test]
    fn markers_land_where_their_moment_falls_on_the_series_or_not_at_all() {
        let traced = json!({ "markers": [
            { "offset": 0, "label": "a" },
            { "offset": 180, "label": "b" },
            { "offset": 361, "label": "late" },
            { "offset": -1, "label": "early" },
        ] });
        assert_eq!(
            markers(&traced, 30, 13, 0),
            vec![
                json!({ "x": 0.0, "label": "a", "level": DEVICE_LEVEL_NEUTRAL, "index": 0 }),
                json!({ "x": 50.0, "label": "b", "level": DEVICE_LEVEL_NEUTRAL, "index": 1 }),
            ]
        );
    }

    #[test]
    fn a_zoomed_lane_keeps_each_marker_on_its_moment_and_its_index_on_its_line() {
        let chart = chart_of(
            &json!({
                "step": 30,
                "startUnix": 0,
                "lanes": [{ "key": "cpu", "values": [1, 2, 3, 4, 5] }],
                "markers": [{ "offset": 30, "label": "before" }, { "offset": 90, "label": "inside", "level": 2 }],
            }),
            Some((0.5, 1.0)),
            false,
        );
        assert_eq!(
            first_lane(&chart)["markers"],
            json!([{ "x": 50.0, "label": "inside", "level": 2, "index": 1 }])
        );
    }

    #[test]
    fn the_caption_names_the_span_in_minutes_or_hours() {
        assert_eq!(span_label(360), "6m");
        assert_eq!(span_label(4500), "1h 15m");
        assert_eq!(
            span_label(21_590),
            "6h 0m",
            "rounding up to a whole hour carries into the hour"
        );
        assert_eq!(span_label(3_590), "1h 0m");
        let chart = chart_of(
            &trace(
                json!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]),
                "count",
                0.0,
            ),
            None,
            false,
        );
        assert_eq!(first_lane(&chart)["span"], "6m of flight");
        let single = chart_of(&trace(json!([1]), "count", 0.0), None, false);
        assert_eq!(first_lane(&single)["span"], "");
    }

    #[test]
    fn the_axis_reads_in_the_devices_clock_not_the_viewers() {
        assert_eq!(clock(1_700_000_000, 0, false), "22:13");
        assert_eq!(clock(1_700_000_000, 3 * 3600, true), "01:13:20");
        assert_eq!(clock(0, -3600, false), "23:00");
    }

    #[test]
    fn gridlines_that_would_read_the_same_are_left_out() {
        assert_eq!(
            grid(0.4, "meters", false),
            Vec::<Value>::new(),
            "a few decimetres of drift would label every gridline 0 m"
        );
        assert_eq!(grid(400.0, "meters", false).len(), 3);
    }

    #[test]
    fn a_lane_with_nothing_above_zero_and_no_declared_maximum_draws_no_gridlines() {
        assert!(
            !scaled(&[0.0, 0.0, 0.0], 0.0),
            "an all-zero distance lane would label every gridline 0 m"
        );
        assert!(scaled(&[0.0, 0.0], 100.0), "a declared maximum is a scale");
        assert!(scaled(&[0.0, 2.5], 0.0));
    }

    #[test]
    fn gridlines_sit_at_quarters_of_the_ceiling_with_the_units_label() {
        let labels = |ceiling: f64, unit: &str| {
            grid(ceiling, unit, false)
                .iter()
                .map(|line| line["label"].as_str().unwrap().to_string())
                .collect::<Vec<String>>()
        };
        assert_eq!(labels(100.0, "percent"), ["25%", "50%", "75%"]);
        assert_eq!(
            labels(250_000.0, "rate"),
            ["500 kbps", "1.0 Mbps", "1.5 Mbps"]
        );
        assert_eq!(labels(2000.0, "meters"), ["500 m", "1.00 km", "1.50 km"]);
        assert_eq!(
            grid(100.0, "percent", false)
                .iter()
                .map(|line| line["y"].as_f64().unwrap())
                .collect::<Vec<f64>>(),
            vec![72.5, 50.0, 27.5]
        );
    }

    #[test]
    fn a_cursor_reads_each_sample_with_its_clock_and_note() {
        assert_eq!(sample_text(18.0, "3G", "percent", false), "18% · 3G");
        assert_eq!(sample_text(-1.0, "", "percent", false), "no reading");
        let chart = chart_of(
            &json!({
                "step": 30,
                "startUnix": 1_700_000_000,
                "tzOffset": 0,
                "lanes": [{ "key": "signal", "unit": "percent", "values": [80, 41, 18, "x"], "notes": ["4G (LTE)", "3G", "3G"] }],
            }),
            None,
            false,
        );
        assert_eq!(
            first_lane(&chart)["readings"],
            json!([
                "22:13:20 · 80% · 4G (LTE)",
                "22:13:50 · 41% · 3G",
                "22:14:20 · 18% · 3G",
                "22:14:50 · no reading"
            ])
        );
    }

    #[test]
    fn a_zoomed_cursor_reads_the_moments_inside_the_zoom() {
        let chart = chart_of(
            &json!({
                "step": 60,
                "startUnix": 0,
                "lanes": [{ "values": [1, 2, 3, 4, 5], "notes": ["a", "b", "c", "d", "e"], "unit": "count" }],
            }),
            Some((0.5, 0.75)),
            false,
        );
        assert_eq!(
            first_lane(&chart)["readings"],
            json!(["00:02:00 · 3 · c", "00:03:00 · 4 · d"])
        );
        assert_eq!(first_lane(&chart)["ticks"][1]["label"], "00:03");
    }

    #[test]
    fn counts_read_as_bare_numbers() {
        assert_eq!(reading(512.4, "count", false), "512");
        assert_eq!(reading(4.0, "count", false), "4");
    }

    #[test]
    fn distances_read_in_the_apps_distance_units() {
        assert_eq!(reading(640.0, "meters", false), "640 m");
        assert_eq!(reading(1250.0, "meters", false), "1.25 km");
        assert_eq!(reading(640.0, "meters", true), "2100 ft");
        assert_eq!(reading(2000.0, "meters", true), "1.24 mi");
    }

    #[test]
    fn a_zoom_window_picks_a_run_of_samples_and_never_fewer_than_two() {
        assert_eq!(window(14, None), 0..14);
        assert_eq!(window(14, Some((0.25, 0.5))), 3..8);
        assert_eq!(window(14, Some((0.99, 1.0))), 12..14);
        assert_eq!(window(14, Some((0.3, 0.31))), 4..6);
        assert_eq!(window(1, Some((0.3, 0.6))), 0..1);
        assert_eq!(window(0, Some((0.3, 0.6))), 0..0);
    }

    #[test]
    fn each_lanes_unit_is_spelled_the_way_that_lane_is_read() {
        assert_eq!(reading(0.72, "number", false), "0.7");
        assert_eq!(reading(3.0, "number", false), "3");
        assert_eq!(reading(2048.0, "mb", false), "2.0 GB");
        assert_eq!(reading(512.0, "mb", false), "512 MB");
        assert_eq!(reading(45.0, "seconds", false), "45 s");
        assert_eq!(reading(180.0, "seconds", false), "3 m");
        assert_eq!(reading(73.0, "percent", false), "73%");
        assert_eq!(reading(100.0, "rate", false), "800 bps");
    }

    #[test]
    fn a_chart_carries_each_lanes_chip_and_the_sorties_to_jump_to() {
        let chart = chart_of(
            &json!({
                "step": 30,
                "lanes": [{ "key": "signal", "label": "Modem signal", "chip": "Signal", "values": [80, 41], "unit": "percent", "max": 100, "default": true, "peak": "41%" }, "junk"],
                "spans": [{ "label": "Sortie 1", "from": 0.1, "to": 0.9 }, { "label": "Sortie 2" }],
            }),
            None,
            false,
        );
        let lane = first_lane(&chart);
        assert_eq!(
            (
                lane["key"].as_str(),
                lane["label"].as_str(),
                lane["chip"].as_str(),
                lane["peak"].as_str(),
                lane["default"].as_bool()
            ),
            (
                Some("signal"),
                Some("Modem signal"),
                Some("Signal"),
                Some("41%"),
                Some(true)
            )
        );
        assert_eq!(chart["lanes"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            chart["spans"],
            json!([{ "label": "Sortie 1", "from": 0.1, "to": 0.9 }, { "label": "Sortie 2", "from": 0.0, "to": 1.0 }])
        );
    }

    #[test]
    fn a_chart_needs_the_trace_and_zooms_only_with_both_ends() {
        assert_eq!(
            answer("[]", false),
            json!({ "ok": false, "reason": CHART_TAKES })
        );
        assert_eq!(answer(r#"["trace"]"#, false)["ok"], false);
        let traced = trace(json!([1, 2, 3, 4, 5]), "count", 0.0);
        let readings = |args: Value| {
            answer(&args.to_string(), false)["result"]["lanes"][0]["readings"]
                .as_array()
                .map(Vec::len)
        };
        assert_eq!(readings(json!([traced, 0.5])), Some(5));
        assert_eq!(readings(json!([traced, 0.5, 1.0])), Some(3));
        assert_eq!(readings(json!([traced, null, null])), Some(5));
    }

    fn share(camera: &str) -> Share {
        [camera.to_string(), "a".to_string(), "b".to_string()]
    }

    fn gmt4() -> chrono::FixedOffset {
        chrono::FixedOffset::east_opt(4 * 3600).unwrap()
    }

    #[test]
    fn a_render_reports_how_far_it_got_and_why_it_stopped() {
        let uploading =
            job(&json!({ "id": "r1", "state": "uploading", "sent": 300, "total": 1200 }));
        assert!(uploading.running());
        assert_eq!(progress(&uploading), "Sending to Aircast cloud… 25%");
        assert_eq!(
            progress(&job(&json!({ "state": "uploading" }))),
            "Sending to Aircast cloud… 0%"
        );
        assert_eq!(progress(&job(&json!({ "state": "rendering" }))), BURNING_IN);
        assert!(!job(&json!({ "state": "done", "url": "https://r2/x.mp4" })).running());
        assert_eq!(job(&json!({ "state": "failed" })).error, CLOUD_FAILED);
        assert_eq!(
            job(&json!({ "state": "failed", "error": "the cloud could not render this video" }))
                .error,
            "the cloud could not render this video"
        );
    }

    #[test]
    fn a_finished_render_offers_its_link_until_it_expires() {
        let done = job(
            &json!({ "state": "done", "url": "https://r2/x.mp4", "expiresAt": "2026-10-22T22:30:00Z" }),
        );
        let shown = progress_view(&done, &gmt4());
        assert_eq!(
            (
                shown["canStart"].as_bool(),
                shown["url"].as_str(),
                shown["expiry"].as_str(),
                shown["progress"].as_str()
            ),
            (
                Some(false),
                Some("https://r2/x.mp4"),
                Some("Link works until 23 Oct"),
                Some("")
            ),
            "the date is the viewer's, past midnight there"
        );
        assert_eq!(expiry("", &gmt4()), "");
        assert_eq!(expiry("next week", &gmt4()), "");
        let sending = progress_view(
            &job(&json!({ "state": "uploading", "url": "https://r2/x.mp4" })),
            &gmt4(),
        );
        assert_eq!(
            (sending["canStart"].as_bool(), sending["url"].as_str()),
            (Some(false), Some(""))
        );
        let failed_view = progress_view(&failed(LOST_TRACK), &gmt4());
        assert_eq!(
            (
                failed_view["canStart"].as_bool(),
                failed_view["error"].as_str()
            ),
            (Some(true), Some(LOST_TRACK)),
            "a failed render can be sent again"
        );
        assert_eq!(progress_view(&Render::default(), &gmt4())["canStart"], true);
    }

    #[test]
    fn a_render_is_sent_then_followed_until_the_cloud_has_it() {
        let sent = std::cell::RefCell::new(Vec::new());
        let answers = std::cell::RefCell::new(vec![
            json!({ "ok": true, "result": { "id": "r 1", "state": "uploading", "sent": 0, "total": 10 } }),
            json!({ "ok": true, "result": { "id": "r 1", "state": "rendering" } }),
            json!({ "ok": true, "result": { "id": "r 1", "state": "done", "url": "https://r2/x.mp4" } }),
        ]);
        let followed = share("followed");
        render(&followed, Duration::ZERO, |path, args| {
            sent.borrow_mut().push(format!("{path} {args}"));
            answers.borrow_mut().remove(0)
        });
        assert_eq!(
            sent.into_inner(),
            vec![
                r#"device.call ["POST","/api/renders",{"camera":"followed","from":"a","to":"b"}]"#
                    .to_string(),
                r#"device.read ["/api/renders/r%201"]"#.to_string(),
                r#"device.read ["/api/renders/r%201"]"#.to_string(),
            ]
        );
        assert_eq!(renders().jobs[&followed].url, "https://r2/x.mp4");
        assert!(take_render_changed());
    }

    #[test]
    fn a_refused_send_or_a_lost_render_says_so_and_stops() {
        let calls = std::cell::Cell::new(0);
        let refused = share("refused");
        render(&refused, Duration::ZERO, |_, _| {
            calls.set(calls.get() + 1);
            json!({ "ok": false, "reason": "No Aircast device is connected.", "field": "" })
        });
        assert_eq!(
            calls.get(),
            1,
            "nothing is polled after the send is refused"
        );
        assert_eq!(
            renders().jobs[&refused].error,
            "No Aircast device is connected."
        );
        assert_eq!(started(&json!({ "ok": false })).error, NOT_SENT);
        let lost = share("lost");
        let answers = std::cell::RefCell::new(vec![
            json!({ "ok": true, "result": { "id": "r2", "state": "rendering" } }),
            json!({ "ok": false, "reason": "The device did not answer." }),
        ]);
        render(&lost, Duration::ZERO, |_, _| answers.borrow_mut().remove(0));
        assert_eq!(renders().jobs[&lost], failed(LOST_TRACK));
    }

    #[test]
    fn one_render_runs_per_share_version_at_a_time() {
        let once = share("once");
        assert!(claim(&once));
        assert_eq!(
            progress(&renders().jobs[&once]),
            "Sending to Aircast cloud… 0%"
        );
        assert!(!claim(&once), "a second press while it runs starts nothing");
        record(&once, failed(CLOUD_FAILED));
        assert!(claim(&once), "a failed render can be sent again");
        assert_eq!(request_render(r#"["cam2","a"]"#)["reason"], RENDER_TAKES);
        assert_eq!(request_render(r#"["cam2","","b"]"#)["ok"], false);
        assert_eq!(share_of(r#"["cam2","a","b"]"#), Some(share("cam2")));
    }
}
