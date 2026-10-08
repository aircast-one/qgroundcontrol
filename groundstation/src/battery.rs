use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::instruments::display_units;
use crate::read::value_number;
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.batteries.count",
    "settings.batteryIndicatorSettings.threshold1",
    "settings.batteryIndicatorSettings.threshold2",
    "settings.batteryIndicatorSettings.valueDisplay",
];

const MAX_PACKS: usize = 8;
const PACK_FACTS: [&str; 10] = ["voltage", "current", "percentRemaining", "chargeState", "timeRemaining", "timeRemainingStr", "instantPower", "mahConsumed", "temperature", "function"];
const DETAIL_FACTS: [&str; 7] = ["function", "voltage", "current", "instantPower", "mahConsumed", "timeRemainingStr", "temperature"];
const FUNCTION_UNKNOWN: f64 = 0.0;
const FUNCTION_ALL: f64 = 1.0;
static PACKS_SEEN: AtomicUsize = AtomicUsize::new(0);

fn pack_fact_path(index: usize, name: &str) -> String {
    format!("vehicle.batteries.{index}.{name}")
}

fn pack_paths() -> Vec<String> {
    (0..PACKS_SEEN.load(Ordering::Relaxed).clamp(1, MAX_PACKS)).flat_map(|i| PACK_FACTS.iter().map(move |name| pack_fact_path(i, name))).collect()
}

pub fn deps() -> Vec<String> {
    DEPS.iter().map(|d| d.to_string()).chain(pack_paths()).collect()
}

const CHARGE_UNDEFINED: i64 = 0;
const CHARGE_OK: i64 = 1;
const CHARGE_LOW: i64 = 2;
const PERCENT_ROUNDS_TO_FULL: f64 = 98.9;

#[derive(Debug, PartialEq)]
pub struct Pack {
    pub voltage: Option<f64>,
    pub current: Option<f64>,
    pub percent: Option<f64>,
    pub charge_state: i64,
    pub charge_label: String,
    pub percent_text: String,
    pub voltage_text: String,
    pub current_text: String,
    pub time_remaining_text: Option<String>,
}

pub fn level(charge_state: i64, percent: Option<f64>, threshold1: f64, threshold2: f64) -> &'static str {
    match (charge_state, percent) {
        (CHARGE_OK, _) => "normal",
        (CHARGE_UNDEFINED, None) => "normal",
        (CHARGE_UNDEFINED, Some(p)) if p > threshold1 => "normal",
        (CHARGE_UNDEFINED, Some(p)) if p > threshold2 => "caution",
        (CHARGE_UNDEFINED, Some(_)) => "warning",
        (CHARGE_LOW, _) => "warning",
        (3..=6, _) => "critical",
        _ => "normal",
    }
}

pub fn text(pack: &Pack) -> String {
    match (pack.percent, pack.voltage, pack.charge_state) {
        (Some(p), _, _) if p > PERCENT_ROUNDS_TO_FULL => "100%".to_string(),
        (Some(_), _, _) => pack.percent_text.clone(),
        (None, Some(_), _) => pack.voltage_text.clone(),
        (None, None, state) if state != CHARGE_UNDEFINED => pack.charge_label.clone(),
        _ => "n/a".to_string(),
    }
}

pub fn secondary_text(pack: &Pack) -> String {
    match (&pack.time_remaining_text, pack.voltage, pack.charge_state) {
        (Some(t), _, _) => t.clone(),
        (None, Some(_), _) => pack.voltage_text.clone(),
        (None, None, state) if state != CHARGE_UNDEFINED => pack.charge_label.clone(),
        _ => "n/a".to_string(),
    }
}

const SHOW_VOLTAGE: i64 = 1;
const SHOW_BOTH: i64 = 2;

pub fn indicator_lines(pack: &Pack, value_display: i64) -> Vec<String> {
    match value_display {
        SHOW_VOLTAGE => vec![secondary_text(pack)],
        SHOW_BOTH => vec![text(pack), secondary_text(pack)],
        _ => vec![text(pack)],
    }
}

fn pack_count(backend: &dyn Backend) -> usize {
    let count = value_number(&backend.value("vehicle.batteries.count")).map(|n| n as usize).unwrap_or(0).min(MAX_PACKS);
    PACKS_SEEN.fetch_max(count, Ordering::Relaxed);
    count
}

fn remembered(backend: &dyn Backend, index: usize) -> impl Fn(&str) -> Value + '_ {
    let read: std::cell::RefCell<std::collections::HashMap<String, Value>> = std::cell::RefCell::default();
    move |name: &str| read.borrow_mut().entry(name.to_string()).or_insert_with(|| backend.value(&pack_fact_path(index, name))).clone()
}

fn pack(fact: &dyn Fn(&str) -> Value) -> Pack {
    let text = |name: &str, key: &str| fact(name).get(key).and_then(Value::as_str).map(str::to_string).unwrap_or_default();
    let number = |name: &str| fact(name).get("value").and_then(Value::as_f64).filter(|v| v.is_finite());
    let shown = |name: &str| format!("{}{}", text(name, "valueString"), display_units(&text(name, "units")));
    Pack {
        voltage: number("voltage"),
        current: number("current"),
        percent: number("percentRemaining"),
        charge_state: fact("chargeState").get("value").and_then(Value::as_i64).unwrap_or(CHARGE_UNDEFINED),
        charge_label: text("chargeState", "enumOrValueString"),
        percent_text: shown("percentRemaining"),
        voltage_text: shown("voltage"),
        current_text: shown("current"),
        time_remaining_text: number("timeRemaining").map(|_| text("timeRemainingStr", "valueString")).filter(|t| !t.is_empty()),
    }
}

fn detail_facts(read: &dyn Fn(&str) -> Value) -> Value {
    DETAIL_FACTS
        .iter()
        .filter_map(|name| {
            let fact = read(name);
            let raw = fact.get("rawValue").or(fact.get("value")).and_then(Value::as_f64);
            if *name == "function" && raw.is_none_or(|f| f == FUNCTION_UNKNOWN || f == FUNCTION_ALL) {
                return None;
            }
            let key = if *name == "function" { "enumOrValueString" } else { "valueString" };
            let spelled = fact.get(key).and_then(Value::as_str).filter(|s| !s.is_empty())?;
            Some(json!({ "name": name, "valueString": spelled, "value": fact.get("value").cloned().unwrap_or(Value::Null), "units": fact.get("units").and_then(Value::as_str).unwrap_or_default() }))
        })
        .collect()
}

pub fn severity(charge_state: i64) -> i64 {
    match charge_state {
        4..=6 => 3,
        3 => 2,
        CHARGE_LOW => 1,
        _ => 0,
    }
}

pub fn duration_text(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return String::new();
    }
    let total = seconds.round() as i64;
    let (hours, minutes, secs) = (total / 3600, (total % 3600) / 60, total % 60);
    match (total < 60, hours > 0) {
        (true, _) => format!("{total} sec"),
        (false, true) => format!("{hours}:{minutes:02}:{secs:02}"),
        (false, false) => format!("{minutes}:{secs:02}"),
    }
}

pub struct PopupPack {
    pub charge_state: i64,
    pub charge_label: String,
    pub percent: Option<f64>,
    pub time_remaining: Option<f64>,
    pub voltage: Option<String>,
    pub consumed: Option<String>,
    pub temperature: Option<String>,
    pub function: Option<String>,
}

fn limiting_percent(pack: &PopupPack) -> f64 {
    pack.percent.filter(|p| p.is_finite()).unwrap_or(f64::INFINITY)
}

pub fn headline(packs: &[PopupPack]) -> Option<Value> {
    let (index, worst) = packs
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| severity(b.charge_state).cmp(&severity(a.charge_state)).then(limiting_percent(a).total_cmp(&limiting_percent(b))))?;
    let alarming = severity(worst.charge_state) > 0;
    let percent = worst.percent.map(|p| format!("{}%", p.round() as i64));
    let left = worst.time_remaining.map(duration_text).filter(|t| !t.is_empty()).map(|t| format!("{t} left"));
    let lowest = (packs.len() > 1).then(|| format!("lowest of {}", packs.len()));
    let detail = left.into_iter().chain(percent.clone().filter(|_| alarming)).chain(lowest).collect::<Vec<_>>().join("  \u{00b7}  ");
    Some(json!({
        "text": if alarming || percent.is_none() { worst.charge_label.clone() } else { percent.unwrap_or_default() },
        "detail": detail,
        "severity": severity(worst.charge_state),
        "index": index,
    }))
}

const PX4_LOW_BATTERY_WARN: i64 = 0;
const PX4_LOW_BATTERY_RETURN: i64 = 1;
const PX4_LOW_BATTERY_LAND: i64 = 2;
const PX4_LOW_BATTERY_RETURN_THEN_LAND: i64 = 3;

pub fn margin_text(action: Option<i64>, low: Option<f64>, critical: Option<f64>, percent: Option<f64>, seconds_left: Option<f64>) -> Option<String> {
    let (verb, at) = match action? {
        PX4_LOW_BATTERY_WARN => ("Warns", low?),
        PX4_LOW_BATTERY_RETURN | PX4_LOW_BATTERY_RETURN_THEN_LAND => ("Returns home", critical?),
        PX4_LOW_BATTERY_LAND => ("Lands", critical?),
        _ => return None,
    };
    let rule = format!("{verb} at {}%", at.round() as i64);
    let to_go = percent.filter(|p| *p > 0.0).map(|p| match seconds_left.filter(|s| *s > 0.0) {
        _ if p <= at => "reached".to_string(),
        Some(seconds) => format!("about {} min to go", (seconds * (p - at) / p / 60.0).round().max(1.0) as i64),
        None => format!("{} points to go", (p - at).round() as i64),
    });
    Some(to_go.map_or(rule.clone(), |go| format!("{rule}  \u{00b7}  {go}")))
}

fn parameter_value(backend: &dyn Backend, name: &str, field: &str) -> Option<f64> {
    value_number(&backend.value(&format!("vehicle.parameterManager.getParameter(-1,{name}).{field}")))
}

pub fn popup_rows(pack: &PopupPack) -> Vec<Value> {
    let level = severity(pack.charge_state);
    [
        pack.time_remaining.map(|t| ("Time left", duration_text(t), true)),
        pack.percent.map(|p| ("Charge", format!("{}%", p.round() as i64), true)),
        pack.voltage.clone().map(|v| ("Voltage", format!("{v} V"), false)),
        pack.consumed.clone().map(|c| ("Consumed", format!("{c} mAh"), false)),
        pack.temperature.clone().map(|t| ("Temperature", format!("{t}\u{00b0}C"), false)),
        pack.function.clone().map(|f| ("Function", f, false)),
    ]
    .into_iter()
    .flatten()
    .map(|(label, value, coloured)| json!({ "label": label, "value": value, "severity": if coloured && level > 0 { level } else { -1 } }))
    .collect()
}

fn popup_pack(fact: &dyn Fn(&str) -> Value) -> PopupPack {
    let raw = |name: &str| { let f = fact(name); f.get("rawValue").or(f.get("value")).and_then(Value::as_f64).filter(|v| v.is_finite()) };
    let spelled = |name: &str| raw(name).and_then(|_| fact(name).get("valueString").and_then(Value::as_str).map(str::to_string));
    let function = raw("function").filter(|f| *f != FUNCTION_UNKNOWN && *f != FUNCTION_ALL).and_then(|_| fact("function").get("enumOrValueString").and_then(Value::as_str).map(str::to_string));
    PopupPack {
        charge_state: fact("chargeState").get("value").and_then(Value::as_i64).unwrap_or(CHARGE_UNDEFINED),
        charge_label: fact("chargeState").get("enumOrValueString").and_then(Value::as_str).unwrap_or_default().to_string(),
        percent: raw("percentRemaining"),
        time_remaining: raw("timeRemaining"),
        voltage: spelled("voltage"),
        consumed: spelled("mahConsumed"),
        temperature: spelled("temperature"),
        function,
    }
}

const COMBINE_PACKS: &str = "settings.batteryIndicatorSettings.consolidateMultipleBatteries.rawValue";
const COMBINE_PACKS_UNSET: bool = true;

fn severity_rank(level: &Value) -> usize {
    ["critical", "warning", "caution", "normal"].iter().position(|l| level == *l).unwrap_or(3)
}

pub fn indicator_packs(described: &[Value], combine: bool) -> Vec<Value> {
    if !combine || described.len() < 2 {
        return described.to_vec();
    }
    let limiting_percent = |pack: &Value| pack["percent"].as_f64().filter(|p| p.is_finite()).unwrap_or(f64::INFINITY);
    described
        .iter()
        .min_by(|a, b| severity_rank(&a["level"]).cmp(&severity_rank(&b["level"])).then(limiting_percent(a).total_cmp(&limiting_percent(b))))
        .and_then(Value::as_object)
        .map(|limiting| {
            let combined = [(String::from("indicatorLabel"), Value::Null), (String::from("packCount"), json!(described.len()))];
            vec![Value::Object(limiting.clone().into_iter().chain(combined).collect())]
        })
        .unwrap_or_default()
}

pub fn battery_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let readers: Vec<_> = (0..pack_count(backend)).map(|index| remembered(backend, index)).collect();
    let packs: Vec<Pack> = readers.iter().map(|read| pack(read)).collect();
    let popup_packs: Vec<PopupPack> = readers.iter().map(|read| popup_pack(read)).collect();
    let threshold1 = value_number(&backend.value("settings.batteryIndicatorSettings.threshold1.rawValue")).unwrap_or(80.0);
    let threshold2 = value_number(&backend.value("settings.batteryIndicatorSettings.threshold2.rawValue")).unwrap_or(60.0);
    let value_display = value_number(&backend.value("settings.batteryIndicatorSettings.valueDisplay.rawValue")).map(|v| v as i64).unwrap_or(0);
    let numbered = packs.len() > 1;
    let described: Vec<Value> = packs
        .iter()
        .enumerate()
        .map(|(index, p)| {
            json!({
                "index": index,
                "voltage": p.voltage,
                "current": p.current,
                "percent": p.percent,
                "chargeState": p.charge_state,
                "chargeLabel": p.charge_label,
                "level": level(p.charge_state, p.percent, threshold1, threshold2),
                "text": text(p),
                "secondaryText": secondary_text(p),
                "indicatorLabel": numbered.then(|| format!("B{}", index + 1)),
                "indicatorLines": indicator_lines(p, value_display),
                "voltageText": p.voltage_text,
                "currentText": p.current_text,
                "percentText": p.percent_text,
                "facts": detail_facts(&readers[index]),
                "rows": popup_rows(&popup_packs[index]),
            })
        })
        .collect();
    let worst = ["critical", "warning", "caution", "normal"]
        .into_iter()
        .find(|l| described.iter().any(|p| p["level"] == *l))
        .unwrap_or("normal");
    json!({
        "kind": "object",
        "class": "Battery",
        "available": !described.is_empty(),
        "level": worst,
        "text": described.first().map(|p| p["text"].clone()).unwrap_or(Value::String(String::new())),
        "indicatorPacks": indicator_packs(&described, backend.value(COMBINE_PACKS).get("value").and_then(Value::as_bool).unwrap_or(COMBINE_PACKS_UNSET)),
        "packs": described,
        "headline": headline(&popup_packs).map(|mut shown| {
            let index = shown["index"].as_u64().unwrap_or(0) as usize;
            let limiting = &popup_packs[index];
            shown["level"] = described.get(index).map_or(Value::Null, |pack| pack["level"].clone());
            shown["margin"] = margin_text(
                parameter_value(backend, "COM_LOW_BAT_ACT", "rawValue").map(|v| v as i64),
                parameter_value(backend, "BAT_LOW_THR", "value"),
                parameter_value(backend, "BAT_CRIT_THR", "value"),
                limiting.percent,
                limiting.time_remaining,
            )
            .map_or(Value::Null, Value::String);
            shown
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_render_reads_each_battery_fact_once_however_many_rows_spell_it() {
        struct Counted(std::cell::RefCell<Vec<String>>);
        impl Backend for Counted {
            fn get(&self, path: &str) -> String {
                self.0.borrow_mut().push(path.to_string());
                match path {
                    "vehicle.batteries.count" => json!({ "kind": "value", "value": 2 }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let counted = Counted(std::cell::RefCell::default());
        battery_view(&counted, &[]);
        let read = counted.0.borrow();
        let repeated: Vec<&String> = read.iter().filter(|path| read.iter().filter(|other| other == path).count() > 1).collect();
        assert!(repeated.is_empty(), "read more than once in one render: {repeated:?}");
    }

    fn popup(state: i64, label: &str, percent: Option<f64>, left: Option<f64>) -> PopupPack {
        PopupPack { charge_state: state, charge_label: label.into(), percent, time_remaining: left, voltage: Some("15.80".into()), consumed: None, temperature: None, function: None }
    }

    #[test]
    fn durations_read_like_battery_indicator() {
        assert_eq!((duration_text(42.4), duration_text(125.0), duration_text(3725.0), duration_text(-1.0)), ("42 sec".to_string(), "2:05".to_string(), "1:02:05".to_string(), String::new()));
    }

    #[test]
    fn the_headline_follows_the_worst_pack_like_battery_indicator() {
        let calm = headline(&[popup(1, "Ok", Some(72.6), Some(600.0))]).unwrap();
        assert_eq!((calm["text"].as_str(), calm["detail"].as_str(), calm["severity"].as_i64()), (Some("73%"), Some("10:00 left"), Some(0)), "a healthy pack leads with its charge");
        let mixed = headline(&[popup(1, "Ok", Some(80.0), None), popup(3, "Critical", Some(12.0), Some(90.0))]).unwrap();
        assert_eq!((mixed["text"].as_str(), mixed["detail"].as_str(), mixed["severity"].as_i64()), (Some("Critical"), Some("1:30 left  \u{00b7}  12%  \u{00b7}  lowest of 2"), Some(2)), "an alarming pack leads with its state and adds its charge");
        assert!(headline(&[]).is_none());
    }

    #[test]
    fn two_healthy_packs_lead_with_the_lower_one_and_say_so() {
        let both = headline(&[popup(1, "Ok", Some(90.0), Some(810.0)), popup(1, "Ok", Some(79.0), Some(711.0))]).unwrap();
        assert_eq!((both["text"].as_str(), both["detail"].as_str(), both["index"].as_u64()), (Some("79%"), Some("11:51 left  \u{00b7}  lowest of 2"), Some(1)));
    }

    #[test]
    fn the_margin_names_the_failsafe_and_the_time_before_it() {
        assert_eq!(margin_text(Some(3), Some(15.0), Some(7.0), Some(79.0), Some(711.0)).as_deref(), Some("Returns home at 7%  \u{00b7}  about 11 min to go"));
        assert_eq!(margin_text(Some(2), Some(15.0), Some(7.0), Some(50.0), None).as_deref(), Some("Lands at 7%  \u{00b7}  43 points to go"));
        assert_eq!(margin_text(Some(0), Some(15.0), Some(7.0), Some(12.0), Some(300.0)).as_deref(), Some("Warns at 15%  \u{00b7}  reached"));
        assert_eq!(margin_text(None, Some(15.0), Some(7.0), Some(79.0), Some(711.0)), None, "a firmware without these parameters says nothing rather than guess");
        assert_eq!(margin_text(Some(3), None, None, Some(79.0), Some(711.0)), None);
    }

    #[test]
    fn pack_rows_follow_the_popup_order_and_colour_only_time_and_charge() {
        let rows = popup_rows(&popup(2, "Low", Some(30.0), Some(65.0)));
        let labels: Vec<&str> = rows.iter().filter_map(|r| r["label"].as_str()).collect();
        assert_eq!(labels, vec!["Time left", "Charge", "Voltage"]);
        assert_eq!((rows[0]["severity"].as_i64(), rows[2]["severity"].as_i64(), rows[2]["value"].as_str()), (Some(1), Some(-1), Some("15.80 V")));
        assert_eq!(popup_rows(&popup(1, "Ok", Some(80.0), None))[0]["severity"].as_i64(), Some(-1), "a healthy pack's charge is grey, as _packColor defaults to colorGrey");
    }

    #[test]
    fn severity_ranks_charge_states_like_severity_of() {
        assert_eq!((4..=6).map(severity).collect::<Vec<_>>(), vec![3, 3, 3]);
        assert_eq!((severity(3), severity(2), severity(7), severity(0)), (2, 1, 0, 0), "charging is not alarming");
        let tie = headline(&[popup(2, "Low", Some(40.0), None), popup(2, "Low", Some(20.0), None)]).unwrap();
        assert_eq!((tie["detail"].as_str(), tie["index"].as_u64()), (Some("20%  \u{00b7}  lowest of 2"), Some(1)), "an equal severity leads with the lower charge, as the Fly bar ring does");
    }

    fn pack_facts(percent: Option<f64>, state: i64, label: &str) -> Vec<(String, Value)> {
        [
            Some(("voltage".to_string(), json!({ "kind": "fact", "name": "voltage", "value": 15.8, "valueString": "15.80", "units": "V" }))),
            Some(("current".to_string(), json!({ "kind": "fact", "name": "current", "value": 12.5, "valueString": "12.50", "units": "A" }))),
            Some(("chargeState".to_string(), json!({ "kind": "fact", "name": "chargeState", "value": state, "enumOrValueString": label }))),
            percent.map(|p| ("percentRemaining".to_string(), json!({ "kind": "fact", "name": "percentRemaining", "value": p, "valueString": format!("{p:.0}"), "units": "%" }))),
            Some(("instantPower".to_string(), json!({ "kind": "fact", "name": "instantPower", "value": 197.5, "valueString": "197.50", "units": "W" }))),
            Some(("mahConsumed".to_string(), json!({ "kind": "fact", "name": "mahConsumed", "value": 340.0, "valueString": "340", "units": "mAh" }))),
            Some(("temperature".to_string(), json!({ "kind": "fact", "name": "temperature", "value": 0.0, "valueString": "", "units": "\u{00b0}C" }))),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn detail_facts_of(facts: &[(String, Value)]) -> Value {
        struct One(Vec<(String, Value)>);
        impl Backend for One {
            fn get(&self, path: &str) -> String {
                let name = path.rsplit('.').next().unwrap_or_default();
                self.0.iter().find(|(n, _)| n == name).map(|(_, f)| f.clone()).unwrap_or(json!({ "kind": "null" })).to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        detail_facts(&remembered(&One(facts.to_vec()), 0))
    }

    fn pack_of(facts: &[(String, Value)]) -> Pack {
        pack(&|name: &str| facts.iter().find(|(n, _)| n == name).map(|(_, f)| f.clone()).unwrap_or(Value::Null))
    }

    #[test]
    fn details_carry_the_value_for_total_draw_and_a_function_qgc_names() {
        let base = pack_facts(Some(50.0), 1, "OK");
        let shown = detail_facts_of(&base);
        assert_eq!(shown.as_array().unwrap().iter().find(|f| f["name"] == "instantPower").unwrap()["value"], 197.5, "BatteryIndicator sums instantPower, so the number has to travel");
        let propulsion = base.iter().cloned().chain(std::iter::once(("function".to_string(), json!({ "kind": "fact", "name": "function", "value": 2, "enumOrValueString": "Propulsion" })))).collect::<Vec<_>>();
        assert_eq!(detail_facts_of(&propulsion)[0], json!({ "name": "function", "valueString": "Propulsion", "value": 2, "units": "" }));
        let all = base.iter().cloned().chain(std::iter::once(("function".to_string(), json!({ "kind": "fact", "name": "function", "value": 1, "enumOrValueString": "All" })))).collect::<Vec<_>>();
        assert!(detail_facts_of(&all).as_array().unwrap().iter().all(|f| f["name"] != "function"), "UNKNOWN and ALL are not shown");
    }

    #[test]
    fn the_indicator_rule_matches_qgcs_own() {
        assert_eq!(level(CHARGE_OK, Some(5.0), 80.0, 60.0), "normal");
        assert_eq!(level(CHARGE_UNDEFINED, Some(85.0), 80.0, 60.0), "normal");
        assert_eq!(level(CHARGE_UNDEFINED, Some(70.0), 80.0, 60.0), "caution");
        assert_eq!(level(CHARGE_UNDEFINED, Some(55.0), 80.0, 60.0), "warning");
        assert_eq!(level(CHARGE_UNDEFINED, None, 80.0, 60.0), "normal");
        assert_eq!(level(CHARGE_LOW, Some(90.0), 80.0, 60.0), "warning");
        assert_eq!(level(3, Some(90.0), 80.0, 60.0), "critical");
        assert_eq!(level(6, None, 80.0, 60.0), "critical");
        assert_eq!(level(7, None, 80.0, 60.0), "normal");
    }

    #[test]
    fn nearly_full_reads_as_full_and_a_missing_percent_falls_back_to_voltage_then_charge_state() {
        let packs: Vec<Pack> = [pack_facts(Some(99.2), CHARGE_OK, "OK"), pack_facts(Some(72.0), CHARGE_UNDEFINED, "Undefined"), pack_facts(None, CHARGE_LOW, "Low")].iter().map(|f| pack_of(f)).collect();
        assert_eq!(text(&packs[0]), "100%");
        assert_eq!(text(&packs[1]), "72%");
        assert_eq!(text(&packs[2]), "15.80V");
        assert_eq!(packs[0].voltage_text, "15.80V");
        let bare = Pack { voltage: None, current: None, percent: None, charge_state: CHARGE_LOW, charge_label: "Low".into(), percent_text: String::new(), voltage_text: String::new(), current_text: String::new(), time_remaining_text: None };
        assert_eq!(text(&bare), "Low");
        assert_eq!(secondary_text(&bare), "Low");
        let empty = Pack { charge_state: CHARGE_UNDEFINED, charge_label: String::new(), ..bare };
        assert_eq!(text(&empty), "n/a");
    }

    #[test]
    fn indicator_lines_follow_value_display_like_battery_indicator() {
        let pack = Pack { voltage: Some(15.8), current: None, percent: Some(42.0), charge_state: CHARGE_OK, charge_label: "OK".into(), percent_text: "42%".into(), voltage_text: "15.80V".into(), current_text: String::new(), time_remaining_text: Some("00:12:00".into()) };
        assert_eq!(indicator_lines(&pack, 0), ["42%"]);
        assert_eq!(indicator_lines(&pack, 1), ["00:12:00"]);
        assert_eq!(indicator_lines(&pack, 2), ["42%", "00:12:00"]);
    }

    #[test]
    fn the_view_reports_the_worst_pack() {
        struct Fake;
        impl Backend for Fake {
            fn get(&self, path: &str) -> String {
                let packs = [pack_facts(Some(90.0), CHARGE_OK, "OK"), pack_facts(Some(50.0), CHARGE_UNDEFINED, "Undefined")];
                let fact = path.strip_prefix("vehicle.batteries.").and_then(|rest| rest.split_once('.')).and_then(|(index, name)| packs.get(index.parse::<usize>().ok()?)?.iter().find(|(n, _)| n == name).map(|(_, f)| f.clone()));
                match (path, fact) {
                    (_, Some(fact)) => fact,
                    ("vehicle.batteries.count", _) => json!({ "kind": "value", "value": 2 }),
                    ("settings.batteryIndicatorSettings.threshold1.rawValue", _) => json!({ "kind": "value", "value": 80 }),
                    ("settings.batteryIndicatorSettings.threshold2.rawValue", _) => json!({ "kind": "value", "value": 60 }),
                    ("settings.batteryIndicatorSettings.valueDisplay.rawValue", _) => json!({ "kind": "value", "value": 2 }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = battery_view(&Fake, &[]);
        assert!(deps().len() >= 4 + 12, "after a read the packs the vehicle reported are watched");
        assert!(deps().contains(&"vehicle.batteries.1.chargeState".to_string()));
        assert_eq!(view["available"], true);
        assert_eq!(view["level"], "warning");
        assert_eq!(view["text"], "90%");
        assert_eq!(view["packs"][1]["level"], "warning");
        assert_eq!(view["packs"][0]["secondaryText"], "15.80V");
        assert_eq!(view["packs"][0]["currentText"], "12.50A", "the head was formatting this itself as %.2f A, which is locale-independent and prints a full stop where the Fact prints whatever the operator's locale does");
        assert_eq!((view["packs"][0]["indicatorLabel"].clone(), view["packs"][1]["indicatorLabel"].clone()), (json!("B1"), json!("B2")));
        assert_eq!(view["packs"][0]["indicatorLines"], json!(["90%", "15.80V"]));
        assert_eq!(view["packs"][0]["percentText"], "90%", "and this was on the struct already and simply never served, so a head had nothing to read and spelled its own");
        assert_eq!(view["indicatorPacks"].as_array().map(Vec::len), Some(1), "two packs show as one indicator unless the pilot turns combining off");
        assert_eq!(view["indicatorPacks"][0]["level"], "warning");
        assert_eq!(view["indicatorPacks"][0]["packCount"], 2);
    }

    #[test]
    fn the_combined_indicator_shows_the_pack_that_limits_the_flight() {
        let pack = |level: &str, percent: f64| json!({ "level": level, "percent": percent, "indicatorLabel": "B" });
        let packs = [pack("normal", 79.0), pack("normal", 57.0), pack("normal", 91.0)];
        let shown = indicator_packs(&packs, true);
        assert_eq!((shown.len(), shown[0]["percent"].as_f64(), shown[0]["packCount"].clone(), shown[0]["indicatorLabel"].clone()), (1, Some(57.0), json!(3), Value::Null));
        let warned = indicator_packs(&[pack("normal", 40.0), pack("warning", 60.0)], true);
        assert_eq!(warned[0]["level"], "warning", "a pack the vehicle flags outranks a lower charge it does not");
        assert_eq!(indicator_packs(&packs, false).len(), 3);
        assert_eq!(indicator_packs(&packs[..1], true)[0]["indicatorLabel"], "B");
    }

    #[test]
    fn a_pack_carries_the_facts_the_detail_panel_names() {
        let facts = pack_facts(Some(90.0), CHARGE_OK, "OK");
        let view = detail_facts_of(&facts);
        let named: Vec<&str> = view.as_array().unwrap().iter().map(|f| f["name"].as_str().unwrap()).collect();
        assert_eq!(named, ["voltage", "current", "instantPower", "mahConsumed"], "the head reads a list, not scalars, and a fact the vehicle never spelled is absent rather than blank");
        assert_eq!(view[2]["valueString"], "197.50");
        assert_eq!(view[2]["units"], "W", "raw, because the head runs its own Units.display over it");
        assert!(PACK_FACTS.contains(&"temperature"), "watched even when unspelled, or the view never recomputes when it starts arriving");
    }

    #[test]
    fn without_packs_nothing_is_available() {
        struct Empty;
        impl Backend for Empty {
            fn get(&self, _p: &str) -> String { json!({ "kind": "null" }).to_string() }
            fn get_fields(&self, _p: &str, _f: &str) -> String { self.get("") }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = battery_view(&Empty, &[]);
        assert_eq!(view["available"], false);
        assert_eq!(view["level"], "normal");
    }
}
