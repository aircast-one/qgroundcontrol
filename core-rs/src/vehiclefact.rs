use serde_json::{Value, json};

use crate::batteryfacts::BatteryFacts;
use crate::factmeta::{MetaData, ValueType};

const BATTERY_META: &str = include_str!("../../src/Vehicle/FactGroups/BatteryFact.json");

fn invalid_text(value_type: &ValueType, decimals: i64) -> String {
    match value_type {
        ValueType::Float | ValueType::Double if decimals > 0 => format!("–.{}", "–".repeat(usize::try_from(decimals).unwrap_or(0))),
        ValueType::ElapsedSeconds => "––:––:––".to_string(),
        _ => "–".to_string(),
    }
}

pub fn fact(meta: &MetaData, raw: &Value, property: Option<&str>) -> Value {
    let mut described = crate::settingsstore::fact_json(meta, raw, crate::units::cooking(meta.units.as_deref().unwrap_or("")));
    described["userVisible"] = Value::Null;
    described["visible"] = Value::Null;
    if meta.default.is_none() {
        described["defaultValue"] = Value::Null;
        described["defaultValueString"] = Value::Null;
    }
    if raw.is_null() && matches!(meta.value_type, ValueType::Float | ValueType::Double | ValueType::ElapsedSeconds) {
        let text = invalid_text(&meta.value_type, described["decimalPlaces"].as_i64().unwrap_or(0));
        described["valueString"] = json!(text);
        described["enumOrValueString"] = json!(text);
        described["unknownEnumLabel"] = json!("Unknown: nan");
        described["value"] = Value::Null;
    }
    if let Some(name) = property {
        described["property"] = json!(name);
    }
    described
}

const BATTERY_FACT_NAMES: [&str; 12] = ["id", "batteryFunction", "batteryType", "voltage", "current", "mahConsumed", "temperature", "percentRemaining", "timeRemaining", "timeRemainingStr", "chargeState", "instantPower"];

const BATTERY_PROPERTIES: [(&str, &str); 12] = [
    ("id", "id"),
    ("function", "batteryFunction"),
    ("type", "batteryType"),
    ("temperature", "temperature"),
    ("voltage", "voltage"),
    ("current", "current"),
    ("mahConsumed", "mahConsumed"),
    ("percentRemaining", "percentRemaining"),
    ("timeRemaining", "timeRemaining"),
    ("timeRemainingStr", "timeRemainingStr"),
    ("chargeState", "chargeState"),
    ("instantPower", "instantPower"),
];

fn time_remaining_text(seconds: Option<f64>) -> String {
    seconds.map_or_else(
        || "––:––:––".to_string(),
        |s| {
            let total = s as i64;
            format!("{:02}H:{:02}M:{:02}S", total / 3600, (total % 3600) / 60, total % 60)
        },
    )
}

fn battery_raw(id: u8, battery: &BatteryFacts, name: &str) -> Value {
    let number = |v: Option<f64>| v.map_or(Value::Null, |n| json!(n));
    match name {
        "id" => json!(id),
        "batteryFunction" => json!(battery.function),
        "batteryType" => json!(battery.kind),
        "voltage" => number(battery.voltage),
        "current" => number(battery.current),
        "mahConsumed" => number(battery.mah_consumed),
        "temperature" => number(battery.temperature),
        "percentRemaining" => number(battery.percent_remaining),
        "timeRemaining" => number(battery.time_remaining),
        "timeRemainingStr" => json!(time_remaining_text(battery.time_remaining)),
        "chargeState" => json!(battery.charge_state),
        "instantPower" => number(battery.instant_power),
        _ => Value::Null,
    }
}

pub fn battery_fact(id: u8, battery: &BatteryFacts, name: &str, property: Option<&str>) -> Option<Value> {
    let meta = crate::factmeta::from_file(BATTERY_META).ok()?.remove(name)?;
    Some(fact(&meta, &battery_raw(id, battery, name), property))
}

pub fn battery_group(id: u8, battery: &BatteryFacts) -> Value {
    let facts: Vec<Value> = BATTERY_PROPERTIES.iter().filter_map(|(property, name)| battery_fact(id, battery, name, Some(property))).collect();
    json!({
        "children": [],
        "class": "BatteryFactGroup",
        "factGroupNames": [],
        "factNames": BATTERY_FACT_NAMES,
        "facts": facts,
        "kind": "object",
        "objectName": "",
        "telemetryAvailable": battery.telemetry,
    })
}

pub fn battery_list(batteries: &[(u8, BatteryFacts)]) -> Value {
    json!({
        "children": [],
        "class": "BatteryFactGroupListModel",
        "count": batteries.len(),
        "dirty": true,
        "elements": batteries.iter().map(|(id, b)| battery_group(*id, b)).collect::<Vec<_>>(),
        "facts": [],
        "kind": "object",
        "objectName": "",
    })
}

pub fn battery_by_property(property: &str) -> Option<&'static str> {
    BATTERY_PROPERTIES.iter().find(|(p, _)| *p == property).map(|(_, n)| *n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_reading_reads_as_dashes_and_a_known_one_in_its_decimals() {
        let battery = BatteryFacts { voltage: Some(12.6), charge_state: 1, ..BatteryFacts::default() };
        let voltage = battery_fact(0, &battery, "voltage", None).unwrap();
        assert_eq!((voltage["valueString"].as_str(), voltage["units"].as_str(), voltage["visible"].clone()), (Some("12.60"), Some("v"), Value::Null));
        let current = battery_fact(0, &battery, "current", None).unwrap();
        assert_eq!((current["valueString"].as_str(), current["unknownEnumLabel"].as_str(), current["rawValue"].clone()), (Some("–.––"), Some("Unknown: nan"), Value::Null));
        let charge = battery_fact(0, &battery, "chargeState", None).unwrap();
        assert_eq!((charge["valueString"].as_str(), charge["enumOrValueString"].as_str()), (Some("1"), Some("Ok")));
        assert_eq!(time_remaining_text(Some(3725.0)), "01H:02M:05S");
        assert_eq!(battery_group(0, &battery)["facts"][1]["property"], "function", "a group lists its facts by the property that holds them");
    }
}
