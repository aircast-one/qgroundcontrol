use serde_json::{Value, json};

use crate::control::decode;
use crate::read::{flag, object};
use crate::router::Backend;

const FACT_ROOTS: &[&str] = &["settings.", "vehicle.parameterManager.getParameter(", "plan.missionController.visualItems."];

pub fn owns(path: &str) -> bool {
    FACT_ROOTS.iter().any(|root| path.starts_with(root)) && !path.ends_with(".rawValue")
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

// A value the fact already holds is written back as it is. Some hold one their own metadata would
// refuse - a survey's minTriggerInterval of 0 under a declared minimum of 0.1, an unmeasured
// amslAltAboveTerrain that is NaN and serialises as null - and a head writing a field back untouched
// must not be told its own reading is out of range.
fn unchanged(fact: &Value, asked: &Value) -> bool {
    let held = fact.get("value").unwrap_or(&Value::Null);
    match (number(held), number(asked)) {
        (Some(h), Some(a)) => h == a,
        _ => held == asked,
    }
}

const ENUM_INDEX: &str = ".enumIndex";
const VALIDATE: &str = ".validate";

pub fn owns_validate(path: &str) -> bool {
    path.strip_suffix(VALIDATE).is_some_and(owns)
}

fn is_fact(fact: &Value) -> bool {
    fact.get("kind").and_then(Value::as_str) == Some("fact")
}

// SettingsScreen.kt asked Fact::validate before writing and the core's own metadata check after, so
// a value could pass the first and be refused by the second in different words. This answers
// validate with the check the write will apply, in Qt's shape: the result is the reason, or empty.
// convertOnly asks only whether the text is the right kind of value, as Fact::validate does.
pub fn validate(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let fact_path = path.strip_suffix(VALIDATE).unwrap_or(path);
    let fact = object(&backend.get(fact_path));
    if !is_fact(&fact) {
        return object(&backend.invoke(path, args));
    }
    let entered = given.get(0).and_then(Value::as_str).unwrap_or_default();
    let convert_only = given.get(1).and_then(Value::as_bool).unwrap_or(false);
    let asked = match fact.get("typeIsString").and_then(Value::as_bool) {
        Some(true) => json!(entered),
        _ => serde_json::from_str::<Value>(entered.trim()).ok().filter(|v| !v.is_string()).unwrap_or_else(|| json!(entered)),
    };
    let found = refusal(&decode(&fact, fact_path), &fact, &asked)
        .filter(|_| !unchanged(&fact, &asked))
        .filter(|(token, _)| !convert_only || matches!(*token, "notANumber" | "notText" | "notAToggle" | "notWhole"));
    json!({
        "ok": true,
        "result": found.as_ref().map_or(String::new(), |(_, reason)| reason.clone()),
        "refusal": found.map(|(token, _)| token),
    })
}

// QGC writes enumIndex straight into the fact, so an index past the list, or the synthesised
// "Unknown: N" entry that is a reading and never a choice, was stored as asked.
fn enum_index_refusal(fact: &Value, index: Option<i64>) -> Option<String> {
    let strings = fact.get("enumStrings").and_then(Value::as_array).cloned().unwrap_or_default();
    let unknown = fact.get("unknownEnumLabel").and_then(Value::as_str).filter(|l| !l.is_empty());
    let chosen = index.and_then(|i| usize::try_from(i).ok()).and_then(|i| strings.get(i));
    match chosen {
        None => Some(format!("Choose one of the {} options.", strings.iter().filter(|s| s.as_str() != unknown).count())),
        Some(label) if label.as_str() == unknown => Some("That entry is the current unknown reading, not a choice.".to_string()),
        Some(_) => None,
    }
}

fn write_enum_index(backend: &dyn Backend, path: &str, value: &str) -> Value {
    let fact = object(&backend.get(path.strip_suffix(ENUM_INDEX).unwrap_or(path)));
    if !is_fact(&fact) {
        return object(&backend.set(path, value));
    }
    if fact.get("readOnly").and_then(Value::as_bool) == Some(true) {
        return json!({ "ok": false, "result": false, "refusal": "readOnly", "reason": "This setting cannot be changed." });
    }
    let index = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value")?.as_i64());
    if let Some(reason) = enum_index_refusal(&fact, index) {
        return json!({ "ok": false, "result": false, "refusal": "notAnOption", "reason": reason });
    }
    let answered = flag(&object(&backend.set(path, value)), "ok");
    json!({ "ok": answered, "result": answered, "refusal": Value::Null, "reason": match answered { true => Value::Null, false => json!("The setting was not written.") } })
}

pub fn write(backend: &dyn Backend, path: &str, value: &str) -> Value {
    if path.ends_with(ENUM_INDEX) {
        return write_enum_index(backend, path, value);
    }
    let renamed = crate::renamed::write_path(path);
    let path = renamed.as_deref().unwrap_or(path);
    let fact = object(&backend.get(path));
    if fact.get("kind").and_then(Value::as_str) != Some("fact") {
        let mut answered = object(&backend.set(path, value));
        if !flag(&answered, "ok") && answered.get("reason").and_then(Value::as_str).is_none_or(str::is_empty) {
            answered = json!({ "ok": false, "result": false, "reason": format!("{path} did not take the write.") });
        }
        return answered;
    }
    let asked = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").cloned()).unwrap_or(Value::Null);
    let control = decode(&fact, path);
    if let Some((token, reason)) = refusal(&control, &fact, &asked).filter(|_| !unchanged(&fact, &asked)) {
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
        assert!(owns("vehicle.parameterManager.getParameter(1,RTL_ALT)"), "a parameter is a Fact the vehicle keeps, so a value outside its metadata goes to the autopilot");
        assert!(owns("plan.missionController.visualItems.3.altitude"));
        assert!(!owns("plan.missionController.globalAltitudeFrame"));
    }

    #[test]
    fn a_parameter_or_item_fact_is_checked_and_a_renamed_item_field_still_lands() {
        struct Vehicle(RefCell<Vec<String>>);
        impl Backend for Vehicle {
            fn get(&self, p: &str) -> String {
                match p {
                    "vehicle.parameterManager.getParameter(1,SERVO_RATE)" => json!({ "kind": "fact", "name": "SERVO_RATE", "typeIsInteger": true, "min": 25, "max": 400, "minIsDefaultForType": false, "maxIsDefaultForType": false, "readOnly": false }),
                    _ => json!({ "kind": "null" }),
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
        let vehicle = Vehicle(RefCell::new(Vec::new()));
        let path = "vehicle.parameterManager.getParameter(1,SERVO_RATE)";
        assert_eq!(write(&vehicle, path, r#"{"value":50.5}"#)["refusal"], "notWhole", "NATIVE_MACOS_REWRITE's open item: an integer parameter accepted a fractional entry and the vehicle was sent the truncation");
        assert_eq!(write(&vehicle, path, r#"{"value":900}"#)["refusal"], "outOfRange");
        assert_eq!(write(&vehicle, path, r#"{"value":50}"#)["result"], true);
        assert_eq!(write(&vehicle, "plan.missionController.visualItems.2.altitudeMode", r#"{"value":1}"#)["ok"], true);
        assert!(unchanged(&json!({ "value": 0.0 }), &json!(0)) && unchanged(&json!({ "value": null }), &Value::Null));
        assert!(!unchanged(&json!({ "value": 0.0 }), &json!(0.05)) && !unchanged(&json!({ "value": null }), &json!(3)));
        assert_eq!(
            vehicle.0.borrow().as_slice(),
            &[path.to_string(), "plan.missionController.visualItems.2.altitudeFrame".to_string()],
            "an item's renamed field is still rewritten when the fact check takes the write before the router's own rename does"
        );
    }

    #[test]
    fn a_value_the_fact_already_holds_is_written_back_even_where_its_metadata_would_refuse_it() {
        struct Survey(RefCell<Vec<String>>);
        impl Backend for Survey {
            fn get(&self, p: &str) -> String {
                match p {
                    "plan.missionController.visualItems.3.cameraCalc.minTriggerInterval" => json!({ "kind": "fact", "name": "MinTriggerInterval", "value": 0.0, "min": 0.1, "max": 100.0, "minIsDefaultForType": false, "maxIsDefaultForType": false, "readOnly": false }),
                    _ => json!({ "kind": "fact", "name": "Alt above terrain", "value": null, "readOnly": false }),
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
        let survey = Survey(RefCell::new(Vec::new()));
        let interval = "plan.missionController.visualItems.3.cameraCalc.minTriggerInterval";
        assert_eq!(write(&survey, interval, r#"{"value":0}"#)["ok"], true, "QGC holds 0 under a declared minimum of 0.1, and writing a field back untouched is not an entry out of range");
        assert_eq!(write(&survey, interval, r#"{"value":0.05}"#)["refusal"], "outOfRange", "a new value below the minimum is still refused");
        assert_eq!(write(&survey, "plan.missionController.visualItems.1.amslAltAboveTerrain", r#"{"value":null}"#)["ok"], true, "an unmeasured height is NaN and reads as null");
        assert_eq!(survey.0.borrow().len(), 2);
    }

    struct Units(RefCell<Vec<String>>);
    impl Backend for Units {
        fn get(&self, p: &str) -> String {
            match p {
                "settings.unitsSettings.verticalDistanceUnits" => json!({ "kind": "fact", "name": "verticalDistanceUnits", "enumStrings": ["Feet", "Meters", "Unknown: 7"], "enumValues": [0, 1, 7], "enumIndex": 2, "value": 7, "unknownEnumLabel": "Unknown: 7", "readOnly": false }),
                "settings.appSettings.defaultMissionItemAltitude" => json!({ "kind": "fact", "name": "defaultMissionItemAltitude", "value": 50.0, "typeIsInteger": false, "min": 1.0, "max": 1000.0, "minIsDefaultForType": false, "maxIsDefaultForType": false, "minString": "1", "maxString": "1000", "readOnly": false }),
                "settings.appSettings.indoorPaletteName" => json!({ "kind": "fact", "name": "indoorPaletteName", "value": "Dark", "typeIsString": true, "readOnly": false }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, _p: &str, _f: &str) -> String { String::new() }
        fn set(&self, p: &str, _v: &str) -> String {
            self.0.borrow_mut().push(p.to_string());
            json!({ "ok": true }).to_string()
        }
        fn invoke(&self, p: &str, _a: &str) -> String {
            self.0.borrow_mut().push(p.to_string());
            json!({ "ok": true, "result": "" }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn an_enum_index_is_written_only_for_a_real_choice() {
        let units = Units(RefCell::new(Vec::new()));
        let path = "settings.unitsSettings.verticalDistanceUnits.enumIndex";
        assert!(crate::actions::owns_write(path), "the enumIndex write reaches the core through the fact roots");
        assert_eq!(write(&units, path, r#"{"value":1}"#)["ok"], true);
        assert_eq!(write(&units, path, r#"{"value":2}"#)["refusal"], "notAnOption", "the synthesised Unknown: 7 is a reading, never a choice");
        assert_eq!(write(&units, path, r#"{"value":9}"#)["reason"], "Choose one of the 2 options.");
        assert_eq!(write(&units, path, r#"{"value":-1}"#)["refusal"], "notAnOption");
        assert_eq!(units.0.borrow().as_slice(), &[path.to_string()], "only the real choice was written");
    }

    #[test]
    fn validate_answers_with_the_check_the_write_will_apply() {
        let units = Units(RefCell::new(Vec::new()));
        let altitude = "settings.appSettings.defaultMissionItemAltitude.validate";
        assert!(owns_validate(altitude) && !owns_validate("vehicle.armed.validate"));
        let asked = |text: &str, convert_only: bool| validate(&units, altitude, &json!([text, convert_only]).to_string());
        assert_eq!((&asked("60", false)["ok"], &asked("60", false)["result"]), (&json!(true), &json!("")), "Qt's shape: an empty result is a valid entry");
        assert_eq!(asked("5000", false)["refusal"], "outOfRange");
        assert_eq!(asked("5000", false)["result"], "This setting runs from 1 to 1000.", "the same sentence the write gives");
        assert_eq!(asked("5000", true)["result"], "", "convertOnly asks only whether the text is a number");
        assert_eq!(asked("tall", true)["refusal"], "notANumber");
        let palette = validate(&units, "settings.appSettings.indoorPaletteName.validate", r#"["1.5",false]"#);
        assert_eq!(palette["result"], "", "a text setting takes digits as text");
        assert!(units.0.borrow().is_empty(), "a fact is validated by the core without asking Qt");
        let _ = validate(&units, "settings.appSettings.notAFact.validate", r#"["1",false]"#);
        assert_eq!(units.0.borrow().as_slice(), &["settings.appSettings.notAFact.validate".to_string()], "a path that is not a fact is Qt's to answer");
    }
}
