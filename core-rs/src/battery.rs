use serde_json::{Value, json};

use crate::read::{object, value_number};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.batteries",
    "settings.batteryIndicatorSettings.threshold1",
    "settings.batteryIndicatorSettings.threshold2",
];

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
}

pub fn level(charge_state: i64, percent: Option<f64>, threshold1: f64, threshold2: f64) -> &'static str {
    match (charge_state, percent) {
        (CHARGE_OK, _) => "normal",
        (CHARGE_UNDEFINED, None) => "normal",
        (CHARGE_UNDEFINED, Some(p)) if p > threshold1 => "normal",
        (CHARGE_UNDEFINED, Some(p)) if p > threshold2 => "caution",
        (CHARGE_UNDEFINED, Some(_)) => "critical",
        (CHARGE_LOW, _) => "caution",
        _ => "critical",
    }
}

pub fn text(pack: &Pack) -> String {
    match (pack.percent, pack.charge_state) {
        (Some(p), _) if p > PERCENT_ROUNDS_TO_FULL => "100%".to_string(),
        (Some(_), _) if !pack.percent_text.is_empty() => pack.percent_text.clone(),
        (_, state) if state != CHARGE_UNDEFINED => pack.charge_label.clone(),
        _ => String::new(),
    }
}

pub fn packs(batteries: &Value) -> Vec<Pack> {
    batteries
        .get("elements")
        .and_then(Value::as_array)
        .map(|elements| elements.iter().map(pack).collect())
        .unwrap_or_default()
}

fn pack(element: &Value) -> Pack {
    let fact = |name: &str| element.get("facts").and_then(Value::as_array).and_then(|f| f.iter().find(|x| x.get("name").and_then(Value::as_str) == Some(name)));
    let number = |name: &str| fact(name).and_then(|f| f.get("value")).and_then(Value::as_f64).filter(|v| v.is_finite());
    let shown = |name: &str| {
        fact(name)
            .map(|f| format!("{}{}", f.get("valueString").and_then(Value::as_str).unwrap_or(""), f.get("units").and_then(Value::as_str).unwrap_or("")))
            .unwrap_or_default()
    };
    Pack {
        voltage: number("voltage"),
        current: number("current"),
        percent: number("percentRemaining"),
        charge_state: fact("chargeState").and_then(|f| f.get("value")).and_then(Value::as_i64).unwrap_or(CHARGE_UNDEFINED),
        charge_label: fact("chargeState").and_then(|f| f.get("enumOrValueString")).and_then(Value::as_str).unwrap_or("").to_string(),
        percent_text: shown("percentRemaining"),
        voltage_text: shown("voltage"),
    }
}

pub fn battery_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let batteries = object(&backend.get("vehicle.batteries"));
    let packs = packs(&batteries);
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
                "voltageText": p.voltage_text,
            })
        })
        .collect();
    let worst = ["critical", "caution", "normal"]
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

    fn pack_json(percent: Option<f64>, state: i64, label: &str) -> Value {
        let facts: Vec<Value> = [
            Some(json!({ "name": "voltage", "value": 15.8, "valueString": "15.80", "units": "V" })),
            Some(json!({ "name": "chargeState", "value": state, "enumOrValueString": label })),
            percent.map(|p| json!({ "name": "percentRemaining", "value": p, "valueString": format!("{p:.0}"), "units": "%" })),
        ]
        .into_iter()
        .flatten()
        .collect();
        json!({ "facts": facts })
    }

    #[test]
    fn the_indicator_rule_matches_qgcs_own() {
        assert_eq!(level(CHARGE_OK, Some(5.0), 80.0, 60.0), "normal");
        assert_eq!(level(CHARGE_UNDEFINED, Some(85.0), 80.0, 60.0), "normal");
        assert_eq!(level(CHARGE_UNDEFINED, Some(70.0), 80.0, 60.0), "caution");
        assert_eq!(level(CHARGE_UNDEFINED, Some(55.0), 80.0, 60.0), "critical");
        assert_eq!(level(CHARGE_UNDEFINED, None, 80.0, 60.0), "normal");
        assert_eq!(level(CHARGE_LOW, Some(90.0), 80.0, 60.0), "caution");
        assert_eq!(level(3, Some(90.0), 80.0, 60.0), "critical");
        assert_eq!(level(6, None, 80.0, 60.0), "critical");
    }

    #[test]
    fn nearly_full_reads_as_full_and_a_missing_percent_falls_back_to_the_charge_state() {
        let packs = packs(&json!({ "elements": [pack_json(Some(99.2), CHARGE_OK, "OK"), pack_json(Some(72.0), CHARGE_UNDEFINED, "Undefined"), pack_json(None, CHARGE_LOW, "Low")] }));
        assert_eq!(text(&packs[0]), "100%");
        assert_eq!(text(&packs[1]), "72%");
        assert_eq!(text(&packs[2]), "Low");
        assert_eq!(packs[0].voltage_text, "15.80V");
    }

    #[test]
    fn the_view_reports_the_worst_pack() {
        struct Fake;
        impl Backend for Fake {
            fn get(&self, path: &str) -> String {
                match path {
                    "vehicle.batteries" => json!({ "kind": "object", "elements": [pack_json(Some(90.0), CHARGE_OK, "OK"), pack_json(Some(50.0), CHARGE_UNDEFINED, "Undefined")] }),
                    "settings.batteryIndicatorSettings.threshold1.rawValue" => json!({ "kind": "value", "value": 80 }),
                    "settings.batteryIndicatorSettings.threshold2.rawValue" => json!({ "kind": "value", "value": 60 }),
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
        assert_eq!(view["available"], true);
        assert_eq!(view["level"], "critical");
        assert_eq!(view["text"], "90%");
        assert_eq!(view["packs"][1]["level"], "critical");
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
