use serde_json::{Value, json};

use crate::control::decode;
use crate::read::{flag, object};
use crate::router::Backend;

pub fn owns(path: &str) -> bool {
    path.starts_with("settings.") && !path.ends_with(".rawValue")
}

fn number(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str().and_then(|s| s.trim().parse().ok())).filter(|v| v.is_finite())
}

fn raw_text(value: &Value) -> String {
    match number(value) {
        Some(n) if n.fract() == 0.0 && n.abs() < 1e15 => format!("{}", n as i64),
        Some(n) => format!("{n}"),
        None => value.as_str().map(str::to_string).unwrap_or_else(|| value.to_string()),
    }
}

pub fn refusal(control: &Value, fact: &Value, asked: &Value) -> Option<(&'static str, String)> {
    if fact.get("readOnly").and_then(Value::as_bool) == Some(true) {
        return Some(("readOnly", "This setting cannot be changed.".to_string()));
    }
    match control["control"].as_str().unwrap_or("number") {
        "toggle" => match asked {
            Value::Bool(_) => None,
            Value::Number(n) if n.as_f64() == Some(0.0) || n.as_f64() == Some(1.0) => None,
            _ => Some(("notAToggle", "This setting is on or off.".to_string())),
        },
        "choice" => {
            let offered = control["options"].as_array().cloned().unwrap_or_default();
            match offered.iter().any(|o| o["raw"] == raw_text(asked).as_str()) {
                true => None,
                false => Some(("notAnOption", format!("Choose one of: {}.", offered.iter().filter_map(|o| o["label"].as_str()).collect::<Vec<_>>().join(", ")))),
            }
        }
        "text" => (!asked.is_string()).then(|| ("notText", "This setting is text.".to_string())),
        "bitmask" => {
            let known: i64 = control["bits"].as_array().map(|b| b.iter().filter_map(|bit| bit["raw"].as_str()?.parse::<i64>().ok()).fold(0, |all, bit| all | bit)).unwrap_or(0);
            match number(asked).filter(|n| n.fract() == 0.0).map(|n| n as i64) {
                Some(bits) if bits >= 0 && bits & !known == 0 => None,
                _ => Some(("unknownBits", "Only the listed options can be set.".to_string())),
            }
        }
        _ => {
            let Some(value) = number(asked) else {
                return Some(("notANumber", "This setting is a number.".to_string()));
            };
            let (minimum, maximum) = (control["minimum"].as_f64(), control["maximum"].as_f64());
            match () {
                _ if control["wholeNumbersOnly"] == true && value.fract() != 0.0 => Some(("notWhole", "This setting takes whole numbers only.".to_string())),
                _ if minimum.is_some_and(|m| value < m) || maximum.is_some_and(|m| value > m) => Some((
                    "outOfRange",
                    format!(
                        "This setting runs from {} to {}.",
                        control["minimumText"].as_str().map(str::to_string).or(minimum.map(|m| m.to_string())).unwrap_or_else(|| "its lowest".to_string()),
                        control["maximumText"].as_str().map(str::to_string).or(maximum.map(|m| m.to_string())).unwrap_or_else(|| "its highest".to_string())
                    ),
                )),
                _ => None,
            }
        }
    }
}

pub fn write(backend: &dyn Backend, path: &str, value: &str) -> Value {
    let fact = object(&backend.get(path));
    if fact.get("kind").and_then(Value::as_str) != Some("fact") {
        return object(&backend.set(path, value));
    }
    let asked = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").cloned()).unwrap_or(Value::Null);
    let control = decode(&fact, path);
    if let Some((token, reason)) = refusal(&control, &fact, &asked) {
        return json!({ "ok": false, "result": false, "refusal": token, "reason": reason, "path": path });
    }
    let answered = flag(&object(&backend.set(path, value)), "ok");
    json!({ "ok": answered, "result": answered, "refusal": Value::Null, "reason": match answered { true => Value::Null, false => json!("The setting was not written.") } })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn fact(extra: Value) -> Value {
        let mut base = json!({ "kind": "fact", "name": "x", "readOnly": false });
        extra.as_object().unwrap().iter().for_each(|(k, v)| base[k] = v.clone());
        base
    }

    fn check(f: Value, asked: Value) -> Option<&'static str> {
        refusal(&decode(&f, "settings.x"), &f, &asked).map(|r| r.0)
    }

    #[test]
    fn a_setting_is_written_only_with_a_value_its_own_metadata_allows() {
        let altitude = fact(json!({ "typeIsInteger": false, "min": 1.0, "max": 1000.0, "minIsDefaultForType": false, "maxIsDefaultForType": false, "minString": "1", "maxString": "1000" }));
        assert_eq!(check(altitude.clone(), json!(50.0)), None);
        assert_eq!(check(altitude.clone(), json!("50")), None, "a field's text reaches the write as a string");
        assert_eq!(check(altitude.clone(), json!(5000.0)), Some("outOfRange"), "the bridge calls setCookedValue, which stores the value without Fact::validate, so a native head's out-of-range entry was kept where QML's FactTextField would have refused it");
        assert_eq!(check(altitude, json!("tall")), Some("notANumber"));
        let whole = fact(json!({ "typeIsInteger": true }));
        assert_eq!(check(whole, json!(3.7)), Some("notWhole"), "the NATIVE_MACOS_REWRITE open item: an integer setting accepted 3.7 and kept 3 in silence");
        let units = fact(json!({ "enumStrings": ["Feet", "Meters", "Unknown: 7"], "enumValues": [0, 1, 7], "unknownEnumLabel": "Unknown: 7" }));
        assert_eq!(check(units.clone(), json!(1)), None);
        assert_eq!(check(units.clone(), json!(7)), Some("notAnOption"), "the synthesised unknown entry is a reading, never a choice");
        assert_eq!(check(units, json!(4)), Some("notAnOption"));
        assert_eq!(check(fact(json!({ "typeIsBool": true })), json!(true)), None);
        assert_eq!(check(fact(json!({ "typeIsBool": true })), json!("yes")), Some("notAToggle"));
        assert_eq!(check(fact(json!({ "typeIsString": true })), json!("udp://:5600")), None);
        let mask = fact(json!({ "bitmaskStrings": ["RC", "Battery"], "bitmaskValues": [1, 2] }));
        assert_eq!(check(mask.clone(), json!(3)), None);
        assert_eq!(check(mask, json!(4)), Some("unknownBits"));
        assert_eq!(check(fact(json!({ "readOnly": true })), json!(1)), Some("readOnly"));
        assert_eq!(check(json!({ "kind": "fact", "name": "x" }), json!(1)), None, "a fact that does not say it is read-only is not refused as one");
    }

    #[test]
    fn a_write_that_is_not_to_a_fact_passes_through_unchanged() {
        struct Settings(RefCell<Vec<String>>);
        impl Backend for Settings {
            fn get(&self, p: &str) -> String {
                match p {
                    "settings.appSettings.defaultMissionItemAltitude" => json!({ "kind": "fact", "name": "defaultMissionItemAltitude", "min": 1.0, "max": 1000.0, "minIsDefaultForType": false, "maxIsDefaultForType": false, "readOnly": false }),
                    _ => json!({ "kind": "object" }),
                }
                .to_string()
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String { String::new() }
            fn set(&self, p: &str, _v: &str) -> String {
                self.0.borrow_mut().push(p.to_string());
                json!({ "ok": true }).to_string()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let settings = Settings(RefCell::new(Vec::new()));
        assert_eq!(write(&settings, "settings.appSettings.defaultMissionItemAltitude", r#"{"value":2000}"#)["refusal"], "outOfRange");
        assert_eq!(write(&settings, "settings.appSettings.defaultMissionItemAltitude", r#"{"value":60}"#)["result"], true);
        assert_eq!(write(&settings, "settings.videoSettings", r#"{"value":1}"#)["ok"], true, "a path that is not a fact is the bridge's to answer");
        assert_eq!(settings.0.borrow().as_slice(), &["settings.appSettings.defaultMissionItemAltitude".to_string(), "settings.videoSettings".to_string()]);
        assert!(owns("settings.appSettings.savePath") && !owns("settings.appSettings.savePath.rawValue") && !owns("vehicle.armed"));
    }
}
