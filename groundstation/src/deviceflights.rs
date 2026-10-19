use std::ops::Range;

use serde_json::{Value, json};

use crate::router::Backend;

pub const FLIGHT_CHART: &str = "deviceFlight.chart";

const PLOT_TOP: f64 = 5.0;
const PLOT_HEIGHT: f64 = 90.0;
const PLOT_BOTTOM: f64 = PLOT_TOP + PLOT_HEIGHT;
const GRID_FRACTIONS: [f64; 3] = [0.25, 0.5, 0.75];
const MAX_TICKS: i64 = 5;
const SECONDS_PER_DAY: i64 = 86_400;
const DEVICE_LEVEL_NEUTRAL: i64 = 3;
const UNREAD_SAMPLE: &str = "no reading";
const CHART_TAKES: &str = "deviceFlight.chart takes a flight's trace as the device serves it, then the zoom's from and to as fractions of the flight, or nothing for the whole flight.";

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
    match seconds >= 3600 {
        true => format!(
            "{}h {}m",
            seconds / 3600,
            ((seconds % 3600) as f64 / 60.0).round()
        ),
        false => format!("{}m", (seconds as f64 / 60.0).round()),
    }
}

fn grid(ceiling: f64, unit: &str, imperial: bool) -> Vec<Value> {
    GRID_FRACTIONS
        .iter()
        .map(|fraction| {
            json!({ "y": PLOT_BOTTOM - PLOT_HEIGHT * fraction, "label": reading(ceiling * fraction, unit, imperial) })
        })
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
        "grid": grid(top, unit, imperial),
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
}
