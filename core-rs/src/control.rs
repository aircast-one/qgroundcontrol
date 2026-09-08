use serde_json::{Value, json};

use crate::label::humanise;
use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &[];
const UNKNOWN_ENUM_PREFIX: &str = "Unknown: ";

pub fn control_view(backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return json!({ "kind": "null" }) };
    let fact = object(&backend.get(path));
    match fact.get("kind").and_then(Value::as_str) {
        Some("fact") => decode(&fact, path),
        _ => json!({ "kind": "null" }),
    }
}

pub fn raw_text(value: &Value) -> String {
    match value.as_f64() {
        Some(n) if n == n.round() && n.abs() < 1e15 => format!("{}", n as i64),
        Some(n) => format!("{n}"),
        None => value.as_str().map(str::to_string).unwrap_or_else(|| value.to_string()),
    }
}

pub fn decode(fact: &Value, path: &str) -> Value {
    let text = |key: &str| fact.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let flag = |key: &str| fact.get(key).and_then(Value::as_bool).unwrap_or(false);
    let name = text("name");
    let labels: Vec<String> = fact.get("enumStrings").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
    let raws: Vec<Value> = fact
        .get("enumValues")
        .and_then(Value::as_array)
        .filter(|r| r.len() == labels.len())
        .cloned()
        .unwrap_or_else(|| (0..labels.len()).map(|i| json!(i)).collect());
    let options: Vec<Value> = labels
        .iter()
        .zip(raws.iter())
        .filter(|(label, _)| !label.starts_with(UNKNOWN_ENUM_PREFIX))
        .map(|(label, raw)| json!({ "label": label, "raw": raw_text(raw) }))
        .collect();
    let control = match (flag("typeIsBool"), labels.is_empty(), flag("typeIsString")) {
        (true, _, _) => "toggle",
        (false, false, _) => "choice",
        (false, true, true) => "text",
        _ => "number",
    };
    let enum_index = fact.get("enumIndex").and_then(Value::as_i64).unwrap_or(-1);
    let display = labels
        .get(usize::try_from(enum_index).unwrap_or(usize::MAX))
        .filter(|l| !l.starts_with(UNKNOWN_ENUM_PREFIX))
        .cloned()
        .unwrap_or_else(|| text("valueString"));
    let bound = |key: &str, default_flag: &str| (!flag(default_flag)).then(|| fact.get(key).and_then(Value::as_f64).filter(|v| v.is_finite())).flatten();
    let described = text("shortDescription");
    json!({
        "kind": "object",
        "class": "Control",
        "path": path,
        "name": name,
        "label": if described.is_empty() { humanise(&name) } else { described },
        "control": control,
        "value": fact.get("value").cloned().unwrap_or(Value::Null),
        "valueString": text("valueString"),
        "display": display,
        "units": text("units"),
        "readOnly": flag("readOnly"),
        "options": options,
        "decimalPlaces": fact.get("decimalPlaces").and_then(Value::as_i64).unwrap_or(0),
        "minimum": bound("min", "minIsDefaultForType"),
        "maximum": bound("max", "maxIsDefaultForType"),
        "rebootRequired": flag("vehicleRebootRequired") || flag("qgcRebootRequired"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bool_fact_is_a_toggle_and_an_enum_a_choice_without_unknowns() {
        let toggle = decode(&json!({ "kind": "fact", "name": "audioMuted", "typeIsBool": true, "value": true, "valueString": "true" }), "p");
        assert_eq!(toggle["control"], "toggle");
        assert_eq!(toggle["label"], "Audio Muted");
        let choice = decode(&json!({ "kind": "fact", "name": "verticalDistanceUnits", "shortDescription": "Vertical distance", "enumStrings": ["Feet", "Meters", "Unknown: 7"], "enumValues": [0, 1, 7], "enumIndex": 2, "valueString": "7" }), "p");
        assert_eq!(choice["control"], "choice");
        assert_eq!(choice["options"].as_array().unwrap().len(), 2);
        assert_eq!(choice["options"][1]["raw"], "1");
        assert_eq!(choice["display"], "7");
        assert_eq!(choice["label"], "Vertical distance");
    }

    #[test]
    fn number_bounds_follow_the_bridges_default_for_type_flags_not_a_magic_size() {
        let bounded = decode(&json!({ "kind": "fact", "name": "RTL_ALT", "min": 0.0, "max": 5e9, "minIsDefaultForType": false, "maxIsDefaultForType": false, "decimalPlaces": 1 }), "p");
        assert_eq!(bounded["control"], "number");
        assert_eq!(bounded["minimum"], 0.0);
        assert_eq!(bounded["maximum"], 5e9);
        let open = decode(&json!({ "kind": "fact", "name": "X", "min": -32768, "max": 32767, "minIsDefaultForType": true, "maxIsDefaultForType": true }), "p");
        assert_eq!(open["minimum"], Value::Null);
        assert_eq!(open["maximum"], Value::Null);
    }

    #[test]
    fn strings_and_enum_positions_fall_back_sensibly() {
        let text = decode(&json!({ "kind": "fact", "name": "rtspUrl", "typeIsString": true, "valueString": "rtsp://x" }), "p");
        assert_eq!(text["control"], "text");
        let positional = decode(&json!({ "kind": "fact", "name": "mode", "enumStrings": ["A", "B"], "enumIndex": 1 }), "p");
        assert_eq!(positional["options"][1]["raw"], "1");
        assert_eq!(positional["display"], "B");
        assert_eq!(raw_text(&json!(2.5)), "2.5");
        assert_eq!(raw_text(&json!("x")), "x");
    }
}
