use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::instruments::display_units;
use crate::read::{object, value_number};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.batteries.count",
    "settings.batteryIndicatorSettings.threshold1",
    "settings.batteryIndicatorSettings.threshold2",
];

const MAX_PACKS: usize = 8;
const PACK_FACTS: [&str; 6] = ["voltage", "current", "percentRemaining", "chargeState", "timeRemaining", "timeRemainingStr"];
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

fn pack_count(backend: &dyn Backend) -> usize {
    let count = value_number(&backend.get("vehicle.batteries.count")).map(|n| n as usize).unwrap_or(0).min(MAX_PACKS);
    PACKS_SEEN.fetch_max(count, Ordering::Relaxed);
    count
}

fn packs(backend: &dyn Backend) -> Vec<Pack> {
    (0..pack_count(backend)).map(|index| pack(&|name: &str| object(&backend.get(&pack_fact_path(index, name))))).collect()
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
        time_remaining_text: number("timeRemaining").map(|_| text("timeRemainingStr", "valueString")).filter(|t| !t.is_empty()),
    }
}

pub fn battery_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let packs = packs(backend);
    let threshold1 = value_number(&backend.get("settings.batteryIndicatorSettings.threshold1.rawValue")).unwrap_or(80.0);
    let threshold2 = value_number(&backend.get("settings.batteryIndicatorSettings.threshold2.rawValue")).unwrap_or(60.0);
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
                "voltageText": p.voltage_text,
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
        "packs": described,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack_facts(percent: Option<f64>, state: i64, label: &str) -> Vec<(String, Value)> {
        [
            Some(("voltage".to_string(), json!({ "kind": "fact", "name": "voltage", "value": 15.8, "valueString": "15.80", "units": "V" }))),
            Some(("chargeState".to_string(), json!({ "kind": "fact", "name": "chargeState", "value": state, "enumOrValueString": label }))),
            percent.map(|p| ("percentRemaining".to_string(), json!({ "kind": "fact", "name": "percentRemaining", "value": p, "valueString": format!("{p:.0}"), "units": "%" }))),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn pack_of(facts: &[(String, Value)]) -> Pack {
        pack(&|name: &str| facts.iter().find(|(n, _)| n == name).map(|(_, f)| f.clone()).unwrap_or(Value::Null))
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
        let bare = Pack { voltage: None, current: None, percent: None, charge_state: CHARGE_LOW, charge_label: "Low".into(), percent_text: String::new(), voltage_text: String::new(), time_remaining_text: None };
        assert_eq!(text(&bare), "Low");
        assert_eq!(secondary_text(&bare), "Low");
        let empty = Pack { charge_state: CHARGE_UNDEFINED, charge_label: String::new(), ..bare };
        assert_eq!(text(&empty), "n/a");
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
