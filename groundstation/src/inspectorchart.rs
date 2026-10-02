use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::read::refused;
use crate::router::Backend;

pub const CHART_ADD: &str = "mavlinkInspector.chart.add";
pub const CHART_REMOVE: &str = "mavlinkInspector.chart.remove";
pub const CHART_RANGE_X: &str = "mavlinkInspector.chart.rangeX";
pub const CHART_RANGE_Y: &str = "mavlinkInspector.chart.rangeY";

const CHART_TIME_SCALES: [(&str, u64); 6] = [("5 Sec", 5_000), ("10 Sec", 10_000), ("30 Sec", 30_000), ("60 Sec", 60_000), ("2 Min", 120_000), ("5 Min", 300_000)];
const CHART_RANGES: [(&str, f64); 10] = [("Auto", 0.0), ("10,000", 10_000.0), ("1,000", 1_000.0), ("100", 100.0), ("10", 10.0), ("1", 1.0), ("0.1", 0.1), ("0.01", 0.01), ("0.001", 0.001), ("0.0001", 0.0001)];
const CHART_MAX_PLOTS: usize = 6;
const CHART_COUNT: usize = 2;
const CHART_MIN_DELTA: f64 = 1e-6;
const CHART_KEEP_MS: u64 = 300_000;

#[derive(Debug, Clone, PartialEq)]
struct Plot {
    system: u8,
    component: u8,
    message: u32,
    instance: String,
    field: String,
    label: String,
    samples: VecDeque<(u64, f64)>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Chart {
    plots: Vec<Plot>,
    range_x: usize,
    range_y: usize,
}

static STATE: Mutex<[Chart; CHART_COUNT]> = Mutex::new([Chart { plots: Vec::new(), range_x: 0, range_y: 0 }, Chart { plots: Vec::new(), range_x: 0, range_y: 0 }]);

fn charts() -> std::sync::MutexGuard<'static, [Chart; CHART_COUNT]> {
    STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn record(system: u8, component: u8, message: u32, payload: &[u8], now_ms: u64) {
    let instance = crate::mavinspect::instance_value(message, payload);
    let mut held = charts();
    held.iter_mut().flat_map(|chart| chart.plots.iter_mut()).filter(|p| p.system == system && p.component == component && p.message == message && p.instance == instance).for_each(|plot| {
        if let Some(value) = crate::mavinspect::field_number(message, payload, &plot.field) {
            plot.samples.push_back((now_ms, value));
        }
        while plot.samples.front().is_some_and(|(at, _)| now_ms.saturating_sub(*at) > CHART_KEEP_MS) {
            plot.samples.pop_front();
        }
    });
}

fn charted(held: &[Chart; CHART_COUNT], system: u8, component: u8, message: u32, instance: &str, field: &str) -> Option<usize> {
    held.iter().position(|chart| chart.plots.iter().any(|p| p.system == system && p.component == component && p.message == message && p.instance == instance && p.field == field))
}

pub fn charts_message(system: u8, component: u8, message: u32, instance: &str) -> bool {
    charts().iter().any(|chart| chart.plots.iter().any(|p| p.system == system && p.component == component && p.message == message && p.instance == instance))
}

fn args(given: &str) -> (Option<usize>, Value) {
    let parsed: Value = serde_json::from_str(given).unwrap_or(Value::Null);
    (parsed.get(0).and_then(Value::as_u64).map(|c| c as usize).filter(|c| *c < CHART_COUNT), parsed.get(1).cloned().unwrap_or(Value::Null))
}

pub fn owns(path: &str) -> bool {
    [CHART_ADD, CHART_REMOVE, CHART_RANGE_X, CHART_RANGE_Y].contains(&path)
}

pub fn run(path: &str, given: &str) -> Value {
    let (chart, second) = args(given);
    let Some(chart) = chart else { return refused("A chart is 0 or 1.") };
    let selected = {
        let inspector = crate::mavinspect::lock();
        inspector.selected_target().zip(inspector.selected_name()).zip(inspector.selected_instance())
    };
    let mut held = charts();
    match path {
        CHART_ADD => {
            let (Some((((system, component, message), name), instance)), Some(field)) = (selected, second.as_str()) else { return refused("Choose a message and one of its fields to chart.") };
            if !crate::mavinspect::field_chartable(message, field) {
                return refused(&format!("{field} is not a number that can be charted."));
            }
            if charted(&held, system, component, message, &instance, field).is_some() {
                return refused(&format!("{field} is already charted."));
            }
            if held[chart].plots.len() >= CHART_MAX_PLOTS {
                return refused("This chart is full.");
            }
            held[chart].plots.push(Plot { system, component, message, instance, field: field.to_string(), label: format!("{name}.{field}"), samples: VecDeque::new() });
        }
        CHART_REMOVE => {
            let (Some((((system, component, message), _), instance)), Some(field)) = (selected, second.as_str()) else { return refused("Name the charted field to remove.") };
            held[chart].plots.retain(|p| !(p.system == system && p.component == component && p.message == message && p.instance == instance && p.field == field));
        }
        CHART_RANGE_X => match second.as_u64().map(|i| i as usize).filter(|i| *i < CHART_TIME_SCALES.len()) {
            Some(index) => held[chart].range_x = index,
            None => return refused("No such time scale."),
        },
        _ => match second.as_u64().map(|i| i as usize).filter(|i| *i < CHART_RANGES.len()) {
            Some(index) => held[chart].range_y = index,
            None => return refused("No such range."),
        },
    }
    json!({ "ok": true })
}

fn y_range(chart: &Chart, visible: &[Vec<(u64, f64)>]) -> Option<(f64, f64)> {
    match CHART_RANGES[chart.range_y].1 {
        range if range > 0.0 => Some((-range, range)),
        _ => {
            let values: Vec<f64> = visible.iter().flatten().map(|(_, v)| *v).collect();
            let (low, high) = values.iter().fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
            (low <= high).then(|| match (high - low).abs() < CHART_MIN_DELTA {
                true => (low - 1.0, high + 1.0),
                false => (low - (high - low) * 0.05, high + (high - low) * 0.05),
            })
        }
    }
}

fn chart_json(chart: &Chart, now_ms: u64) -> Value {
    let window = CHART_TIME_SCALES[chart.range_x].1;
    let visible: Vec<Vec<(u64, f64)>> = chart.plots.iter().map(|p| p.samples.iter().filter(|(at, _)| now_ms.saturating_sub(*at) <= window).copied().collect()).collect();
    let range = y_range(chart, &visible);
    json!({
        "rangeX": chart.range_x,
        "rangeY": chart.range_y,
        "windowMs": window,
        "yMin": range.map(|r| r.0),
        "yMax": range.map(|r| r.1),
        "room": chart.plots.len() < CHART_MAX_PLOTS,
        "plots": chart.plots.iter().zip(visible).enumerate().map(|(index, (plot, points))| json!({
            "label": plot.label,
            "field": plot.field,
            "colour": index,
            "points": points.iter().map(|(at, v)| json!([now_ms.saturating_sub(*at), v])).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

pub fn charts_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    let now_ms = crate::hub::now_ms();
    let selected = {
        let inspector = crate::mavinspect::lock();
        inspector.selected_target().zip(inspector.selected_instance())
    };
    let held = charts();
    json!({
        "kind": "object",
        "class": "InspectorCharts",
        "timeScales": CHART_TIME_SCALES.iter().map(|(label, _)| label).collect::<Vec<_>>(),
        "ranges": CHART_RANGES.iter().map(|(label, _)| label).collect::<Vec<_>>(),
        "charts": held.iter().map(|chart| chart_json(chart, now_ms)).collect::<Vec<_>>(),
        "selectedCharted": selected.map(|((system, component, message), instance)| {
            held.iter().enumerate().flat_map(|(index, chart)| chart.plots.iter().filter(|p| p.system == system && p.component == component && p.message == message && p.instance == instance).map(move |p| json!({ "field": p.field, "chart": index }))).collect::<Vec<_>>()
        }).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plot(samples: &[(u64, f64)]) -> Plot {
        Plot { system: 1, component: 1, message: 30, instance: String::new(), field: "roll".into(), label: "ATTITUDE.roll".into(), samples: samples.iter().copied().collect() }
    }

    #[test]
    fn auto_range_pads_the_visible_values_and_a_fixed_range_is_centred() {
        let chart = Chart { plots: vec![plot(&[])], range_x: 0, range_y: 0 };
        assert_eq!(y_range(&chart, &[vec![(0, 0.0), (1, 10.0)]]), Some((-0.5, 10.5)));
        assert_eq!(y_range(&chart, &[vec![(0, 3.0)]]), Some((2.0, 4.0)));
        assert_eq!(y_range(&chart, &[vec![]]), None);
        assert_eq!(y_range(&Chart { range_y: 3, ..chart }, &[vec![]]), Some((-100.0, 100.0)));
    }

    #[test]
    fn a_chart_shows_only_the_samples_inside_its_time_scale() {
        let chart = Chart { plots: vec![plot(&[(1_000, 1.0), (8_000, 2.0), (9_500, 3.0)])], range_x: 0, range_y: 0 };
        let shown = chart_json(&chart, 10_000);
        assert_eq!(shown["plots"][0]["points"], json!([[2000, 2.0], [500, 3.0]]));
        assert_eq!(shown["windowMs"], 5000);
    }

    #[test]
    fn attitude_roll_reads_as_a_number_and_text_fields_do_not() {
        use mavlink::dialects::ardupilotmega::{ATTITUDE_DATA, MavMessage};
        let mut raw = Vec::new();
        mavlink::write_v2_msg(&mut raw, mavlink::MavHeader::default(), &MavMessage::ATTITUDE(ATTITUDE_DATA { roll: 0.5, ..Default::default() })).unwrap();
        let payload = crate::mavinspect::payload(&raw, true);
        assert_eq!(crate::mavinspect::field_number(30, &payload, "roll"), Some(0.5));
        assert!(!crate::mavinspect::field_chartable(253, "text"), "STATUSTEXT text is a char array");
        assert!(crate::mavinspect::field_chartable(253, "severity") && crate::mavinspect::field_chartable(30, "roll"));
        assert_eq!(crate::mavinspect::field_number(30, &payload, "nope"), None);
    }
}
