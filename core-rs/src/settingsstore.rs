use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex, PoisonError};

use serde_json::{Value, json};

use crate::factmeta::{MetaData, ValueType};
use crate::router::Backend;
use crate::settingsini::Setting;

const OBJECTS: [(&str, &str); 25] = [
    ("adsbVehicleManagerSettings", "ADSBVehicleManager"),
    ("packetRadioSettings", "PacketRadio"),
    ("apmMavlinkStreamRateSettings", "APMMavlinkStreamRate"),
    ("appSettings", "App"),
    ("autoConnectSettings", "AutoConnect"),
    ("batteryIndicatorSettings", "BatteryIndicator"),
    ("mavlinkActionsSettings", "MavlinkActions"),
    ("firmwareUpgradeSettings", "FirmwareUpgrade"),
    ("flightMapSettings", "FlightMap"),
    ("flightModeSettings", "FlightMode"),
    ("flyViewSettings", "FlyView"),
    ("gimbalControllerSettings", "GimbalController"),
    ("mapsSettings", "Maps"),
    ("offlineMapsSettings", "OfflineMaps"),
    ("planViewSettings", "PlanView"),
    ("remoteIDSettings", "RemoteID"),
    ("rtkSettings", "RTK"),
    ("unitsSettings", "Units"),
    ("ntripSettings", "NTRIP"),
    ("videoSettings", "Video"),
    ("mavlinkSettings", "Mavlink"),
    ("joystickManagerSettings", "JoystickManager"),
    ("logManagerSettings", "LogManager"),
    ("logViewerSettings", "LogViewer"),
    ("viewer3DSettings", "Viewer3D"),
];

const EXTRA_GROUPS: [(&str, &str); 4] = [
    ("JoystickManager", include_str!("../../src/Settings/JoystickManager.SettingsGroup.json")),
    ("LogManager", include_str!("../../src/Settings/LogManager.SettingsGroup.json")),
    ("LogViewer", include_str!("../../src/Settings/LogViewer.SettingsGroup.json")),
    ("NTRIP", include_str!("../../src/Settings/NTRIP.SettingsGroup.json")),
];

const DEFAULT_DECIMAL_PLACES: i64 = 3;

pub const RUNTIME: [(&str, &[&str]); 5] = [
    ("appSettings.androidDontSaveToSDCard", &["userVisible", "visible"]),
    ("appSettings.androidUsePosixSerial", &["userVisible", "visible"]),
    ("appSettings.indoorPalette", &["defaultValue", "defaultValueString", "valueEqualsDefault"]),
    ("appSettings.qLocaleLanguage", &["enumIndex", "enumOrValueString", "enumStrings", "enumValues"]),
    ("videoSettings.forceVideoDecoder", &["enumStrings", "enumValues"]),
];

pub const RUNTIME_WHOLE: [&str; 1] = ["videoSettings.videoSource"];

pub const UNEXPOSED: [&str; 3] = ["autoConnectSettings.autoConnectZeroConf", "flightModeSettings.px4HiddenFlightModes", "videoSettings.videoSavePath"];

pub fn runtime_fields(path: &str) -> Option<&'static [&'static str]> {
    let short = path.strip_prefix("settings.")?;
    RUNTIME.iter().find(|(name, _)| *name == short).map(|(_, keys)| *keys)
}

pub fn served_by_host(path: &str) -> bool {
    path.strip_prefix("settings.").is_some_and(|short| RUNTIME_WHOLE.contains(&short))
}

pub fn locate(path: &str) -> Option<(&'static str, &str)> {
    let rest = path.strip_prefix("settings.")?;
    let (object, fact) = rest.split_once('.')?;
    let group = OBJECTS.iter().find(|(name, _)| *name == object)?.1;
    Some((group, fact))
}

pub fn metadata(group: &str, fact: &str) -> Option<MetaData> {
    let json = crate::settingsgroups::group(group).map(|g| g.json).or_else(|| EXTRA_GROUPS.iter().find(|(name, _)| *name == group).map(|(_, json)| *json))?;
    crate::factmeta::from_file(json).ok()?.remove(fact)
}

fn integer(value_type: &ValueType) -> bool {
    matches!(value_type, ValueType::Uint8 | ValueType::Int8 | ValueType::Uint16 | ValueType::Int16 | ValueType::Uint32 | ValueType::Int32 | ValueType::Uint64 | ValueType::Int64)
}

fn type_limits(value_type: &ValueType) -> (f64, f64) {
    match value_type {
        ValueType::Bool => (0.0, 1.0),
        ValueType::String => (0.0, 0.0),
        ValueType::Uint8 => (0.0, f64::from(u8::MAX)),
        ValueType::Int8 => (f64::from(i8::MIN), f64::from(i8::MAX)),
        ValueType::Uint16 => (0.0, f64::from(u16::MAX)),
        ValueType::Int16 => (f64::from(i16::MIN), f64::from(i16::MAX)),
        ValueType::Uint32 => (0.0, f64::from(u32::MAX)),
        ValueType::Int32 => (f64::from(i32::MIN), f64::from(i32::MAX)),
        ValueType::Uint64 => (0.0, u64::MAX as f64),
        ValueType::Int64 => (i64::MIN as f64, i64::MAX as f64),
        ValueType::Float => (f64::from(-f32::MAX), f64::from(f32::MAX)),
        _ => (-f64::MAX, f64::MAX),
    }
}

fn spelled(value: &Value, decimals: i64, whole: bool) -> String {
    match value {
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        other => other.as_f64().map_or_else(String::new, |n| match whole {
            true => format!("{}", n as i64),
            false => half_away(n, usize::try_from(decimals).unwrap_or(0)),
        }),
    }
}

fn half_away(n: f64, decimals: usize) -> String {
    let scale = 10f64.powi(i32::try_from(decimals).unwrap_or(0));
    let rounded = (n * scale).round() / scale;
    format!("{:.decimals$}", if rounded.is_finite() { rounded } else { n })
}

fn number_json(value: f64, whole: bool) -> Value {
    match whole || (value.fract() == 0.0 && value.abs() < 1e15) {
        true => json!(value as i64),
        false => json!(value),
    }
}

pub fn fact_json(meta: &MetaData, raw: &Value, units: &crate::surveydoc::Units) -> Value {
    let whole = integer(&meta.value_type);
    let decimals = meta.decimal_places.unwrap_or(DEFAULT_DECIMAL_PLACES);
    let limits = type_limits(&meta.value_type);
    let raw_units = meta.units.clone().unwrap_or_default();
    let unit = match raw_units.as_str() {
        "vertical m" => Some(units.vertical),
        "m" | "meter" | "meters" | "horizontal m" => Some(units.horizontal),
        _ => None,
    };
    let cooked = |v: f64| unit.map_or(v, |u| u.show(v));
    let bound = |v: &Option<Value>| v.as_ref().and_then(Value::as_f64);
    let (min, max) = (bound(&meta.min), bound(&meta.max));
    let (shown_min, shown_max) = (min.map(cooked).unwrap_or(limits.0), max.map(cooked).unwrap_or(limits.1));
    let bool_typed = meta.value_type == ValueType::Bool;
    let bound_text = |v: f64| match bool_typed {
        true => (v != 0.0).to_string(),
        false => spelled(&json!(v), decimals, whole || meta.value_type == ValueType::String),
    };
    let labels: Vec<String> = meta.enums.iter().map(|e| e.label.clone()).collect();
    let values: Vec<Value> = meta.enums.iter().map(|e| e.value.clone()).collect();
    let enum_index = values.iter().position(|v| v == raw || v.as_f64().zip(raw.as_f64()).is_some_and(|(a, b)| a == b)).map_or(-1, |i| i as i64);
    let value = match (unit, raw.as_f64()) {
        (Some(u), Some(v)) => number_json(u.show(v), whole),
        _ => raw.clone(),
    };
    let value_string = spelled(&value, decimals, whole);
    let units = unit.map_or_else(|| raw_units.clone(), |u| u.name.clone());
    let is_number = raw.is_number();
    let default = default_of(meta);
    json!({
        "kind": "fact",
        "name": meta.name,
        "shortDescription": meta.short_description,
        "longDescription": meta.long_description,
        "value": value,
        "rawValue": raw,
        "valueString": value_string,
        "enumOrValueString": usize::try_from(enum_index).ok().and_then(|i| labels.get(i).cloned()).unwrap_or_else(|| value_string.clone()),
        "enumStrings": labels,
        "enumValues": values,
        "enumIndex": enum_index,
        "units": units,
        "rawUnits": raw_units,
        "decimalPlaces": decimals,
        "defaultValueAvailable": default.is_some(),
        "defaultValue": default,
        "defaultValueString": default.as_ref().map_or_else(String::new, |d| spelled(d, decimals, whole)),
        "valueEqualsDefault": default.as_ref().is_some_and(|d| d == raw || d.as_f64().zip(raw.as_f64()).is_some_and(|(a, b)| a == b)),
        "min": number_json(shown_min, whole || !is_number),
        "max": number_json(shown_max, whole || !is_number),
        "minString": bound_text(shown_min),
        "maxString": bound_text(shown_max),
        "minIsDefaultForType": shown_min == limits.0,
        "maxIsDefaultForType": shown_max == limits.1,
        "typeIsBool": meta.value_type == ValueType::Bool,
        "typeIsInteger": whole,
        "typeIsString": meta.value_type == ValueType::String,
        "readOnly": meta.read_only,
        "qgcRebootRequired": meta.qgc_reboot_required,
        "vehicleRebootRequired": meta.vehicle_reboot_required,
        "unknownEnumLabel": format!("Unknown: {}", crate::control::raw_text(raw)),
        "bitmaskStrings": [],
        "bitmaskValues": [],
        "userVisible": true,
        "visible": true,
    })
}

static SWITCHED_ON: LazyLock<bool> = LazyLock::new(|| std::env::var("QGC_CORE_SETTINGS").is_ok_and(|v| v == "1"));

static STORED: Mutex<Option<BTreeMap<String, Setting>>> = Mutex::new(None);

fn stored() -> std::sync::MutexGuard<'static, Option<BTreeMap<String, Setting>>> {
    STORED.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn open(path: &std::path::Path) {
    *stored() = Some(crate::settingsini::read(&std::fs::read_to_string(path).unwrap_or_default()));
}

pub fn stored_text(key: &str) -> Option<String> {
    match stored().as_ref()?.get(key)? {
        Setting::Text(text) => Some(text.clone()),
        _ => None,
    }
}

pub fn enabled() -> bool {
    *SWITCHED_ON && stored().is_some()
}

fn key(group: &str, fact: &str) -> String {
    crate::settingsgroups::settings_key(group, fact).unwrap_or_else(|| format!("{group}/{fact}"))
}

pub fn typed(value_type: &ValueType, value: &Value) -> Option<Value> {
    let number = value.as_f64().or_else(|| value.as_bool().map(f64::from)).or_else(|| value.as_str().and_then(|t| t.trim().parse().ok()));
    match value_type {
        ValueType::Bool => value.as_bool().or_else(|| value.as_str().and_then(|t| match t {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        })).or_else(|| number.map(|n| n != 0.0)).map(Value::Bool),
        ValueType::String => Some(value.as_str().map_or_else(|| crate::control::raw_text(value), str::to_string)).map(Value::String),
        t if integer(t) => number.map(|n| json!(n.round() as i64)),
        ValueType::Float => number.filter(|n| n.is_finite()).map(|n| number_json(f64::from(n as f32), false)),
        _ => number.filter(|n| n.is_finite()).map(|n| number_json(n, false)),
    }
}

fn default_of(meta: &MetaData) -> Option<Value> {
    meta.default.as_ref().map(|d| typed(&meta.value_type, d).unwrap_or_else(|| d.clone()))
}

fn raw(group: &str, fact: &str, meta: &MetaData) -> Value {
    let held = match stored().as_ref().and_then(|values| values.get(&key(group, fact)).cloned()) {
        Some(Setting::Text(text)) => typed(&meta.value_type, &Value::String(text)),
        _ => None,
    };
    held.or_else(|| default_of(meta)).unwrap_or(Value::Null)
}

fn units_for(backend: &dyn Backend, meta: &MetaData) -> (crate::read::Unit, crate::read::Unit) {
    let metres = || crate::read::Unit { name: "m".to_string(), factor: 1.0 };
    match meta.units.as_deref() {
        Some("vertical m") => (crate::read::Unit::vertical(backend), metres()),
        Some("m" | "meter" | "meters" | "horizontal m") => (metres(), crate::read::Unit::horizontal(backend)),
        _ => (metres(), metres()),
    }
}

struct Addressed {
    group: &'static str,
    fact: String,
    field: Option<String>,
    meta: MetaData,
}

fn address(path: &str) -> Option<Addressed> {
    let (group, rest) = locate(path)?;
    let (fact, field) = match rest.split_once('.') {
        Some((fact, field)) => (fact.to_string(), Some(field.to_string())),
        None => (rest.to_string(), None),
    };
    let fact_path = path.strip_suffix(&field.as_ref().map(|f| format!(".{f}")).unwrap_or_default()).unwrap_or(path).to_string();
    (!served_by_host(&fact_path)).then_some(())?;
    let meta = metadata(group, &fact)?;
    Some(Addressed { group, fact, field, meta })
}

fn described(backend: &dyn Backend, at: &Addressed, path: &str) -> Value {
    let (vertical, horizontal) = units_for(backend, &at.meta);
    let mine = fact_json(&at.meta, &raw(at.group, &at.fact, &at.meta), &crate::surveydoc::Units { vertical: &vertical, horizontal: &horizontal });
    let fact_path = path.split('.').take(3).collect::<Vec<_>>().join(".");
    match runtime_fields(&fact_path) {
        Some(keys) => {
            let host = crate::read::object(&backend.get_fields(&fact_path, &keys.join(",")));
            let mut merged = mine;
            keys.iter().filter_map(|k| host.get(*k).map(|v| (*k, v.clone()))).for_each(|(k, v)| merged[k] = v);
            merged
        }
        None => mine,
    }
}

fn unexposed(path: &str) -> bool {
    path.strip_prefix("settings.").is_some_and(|short| UNEXPOSED.iter().any(|name| short == *name || short.starts_with(&format!("{name}."))))
}

pub fn get(backend: &dyn Backend, path: &str) -> Option<String> {
    if unexposed(path) {
        return Some(json!({ "found": false, "kind": "value", "value": null }).to_string());
    }
    let at = address(path)?;
    let fact = described(backend, &at, path);
    match &at.field {
        None => Some(fact.to_string()),
        Some(field) => fact.get(field.as_str()).map(|v| json!({ "kind": "value", "value": v }).to_string()),
    }
}

pub fn get_fields(backend: &dyn Backend, path: &str, fields: &str) -> Option<String> {
    let at = address(path)?;
    at.field.is_none().then_some(())?;
    let fact = described(backend, &at, path);
    let asked: Vec<&str> = fields.split(',').map(str::trim).filter(|f| !f.is_empty()).collect();
    let everything = asked.contains(&"*");
    let picked: serde_json::Map<String, Value> = fact.as_object()?.iter().filter(|(k, _)| everything || k.as_str() == "kind" || asked.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
    let unknown: Vec<&str> = asked.iter().filter(|f| **f != "*" && fact.get(**f).is_none()).copied().collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    let mut answer = Value::Object(picked);
    if !unknown.is_empty() {
        answer["unknownFields"] = json!(unknown);
    }
    Some(answer.to_string())
}

pub fn set(backend: &dyn Backend, path: &str, value: &str) -> Option<String> {
    let at = address(path)?;
    let cooked = match at.field.as_deref() {
        None | Some("value") => true,
        Some("rawValue") => false,
        Some(_) => return None,
    };
    let written = crate::read::object(value);
    let given = written.get("value").cloned().unwrap_or(written);
    let (vertical, horizontal) = units_for(backend, &at.meta);
    let unit = match at.meta.units.as_deref() {
        Some("vertical m") => Some(vertical),
        Some("m" | "meter" | "meters" | "horizontal m") => Some(horizontal),
        _ => None,
    };
    let raw_given = match (cooked, unit, given.as_f64()) {
        (true, Some(u), Some(n)) => json!(u.meters(n)),
        _ => given,
    };
    if let Some(new) = typed(&at.meta.value_type, &raw_given) {
        let spelled = match &new {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        if let Some(values) = stored().as_mut() {
            values.insert(key(at.group, &at.fact), Setting::Text(spelled));
        }
    }
    Some(backend.set(path, value))
}

pub struct Owner<B>(pub B);

impl<B: Backend> Backend for Owner<B> {
    fn get(&self, path: &str) -> String {
        enabled().then(|| get(&self.0, path)).flatten().unwrap_or_else(|| self.0.get(path))
    }
    fn get_fields(&self, path: &str, fields: &str) -> String {
        enabled().then(|| get_fields(&self.0, path, fields)).flatten().unwrap_or_else(|| self.0.get_fields(path, fields))
    }
    fn set(&self, path: &str, value: &str) -> String {
        enabled().then(|| set(&self.0, path, value)).flatten().unwrap_or_else(|| self.0.set(path, value))
    }
    fn invoke(&self, path: &str, args: &str) -> String {
        self.0.invoke(path, args)
    }
    fn watch(&self, paths: &[String]) {
        self.0.watch(paths);
    }
    fn core_guided(&self, action: &Value) -> Option<Result<(), String>> {
        self.0.core_guided(action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by_value(value: &Value) -> Value {
        match value {
            Value::Number(n) => json!(n.as_f64().map(|f| format!("{f:.12e}"))),
            Value::Array(items) => Value::Array(items.iter().map(by_value).collect()),
            Value::Object(fields) => Value::Object(fields.iter().map(|(k, v)| (k.clone(), by_value(v))).collect()),
            other => other.clone(),
        }
    }

    #[test]
    fn a_stored_value_is_read_as_the_fact_type_qt_converts_it_to() {
        assert_eq!(typed(&ValueType::Bool, &json!("true")), Some(json!(true)));
        assert_eq!(typed(&ValueType::Uint32, &json!(false)), Some(json!(0)), "a false default on an integer fact reads as 0");
        assert_eq!(typed(&ValueType::Float, &json!(1.777777)), Some(json!(1.7777769565582275)), "a float fact holds single precision");
        assert_eq!(typed(&ValueType::Double, &json!("15")), Some(json!(15)));
        assert_eq!(typed(&ValueType::Uint8, &json!("abc")), None);
        assert_eq!(spelled(&json!(17.25), 1, false), "17.3", "Qt rounds a written half away from zero");
    }

    #[test]
    fn every_setting_fact_is_described_as_the_bridge_describes_it() {
        let qt: serde_json::Map<String, Value> = serde_json::from_str(include_str!("../tests/fixtures/settings-facts-by-qt.json")).unwrap();
        let differing: Vec<String> = qt
            .iter()
            .filter_map(|(path, expected)| {
                let Some((group, fact)) = locate(path) else { return Some(format!("{path}: no group")) };
                let Some(meta) = metadata(group, fact) else { return Some(format!("{path}: no metadata")) };
                if served_by_host(path) {
                    return None;
                }
                let metres = crate::read::Unit { name: "m".to_string(), factor: 1.0 };
                let mine = by_value(&fact_json(&meta, &expected["rawValue"], &crate::surveydoc::Units { vertical: &metres, horizontal: &metres }));
                let expected = by_value(expected);
                let host = runtime_fields(path).unwrap_or(&[]);
                let keys: Vec<String> = expected.as_object().unwrap().iter().filter(|(k, v)| !host.contains(&k.as_str()) && mine.get(k.as_str()) != Some(v)).map(|(k, _)| k.clone()).collect();
                (!keys.is_empty()).then(|| format!("{path}: {}", keys.join(",")))
            })
            .collect();
        assert!(differing.is_empty(), "{} of {} differ:\n{}", differing.len(), qt.len(), differing.join("\n"));
    }

    #[test]
    fn every_field_left_to_the_host_really_differs_from_the_metadata() {
        let qt: serde_json::Map<String, Value> = serde_json::from_str(include_str!("../tests/fixtures/settings-facts-by-qt.json")).unwrap();
        let metres = crate::read::Unit { name: "m".to_string(), factor: 1.0 };
        let units = crate::surveydoc::Units { vertical: &metres, horizontal: &metres };
        RUNTIME.iter().for_each(|(short, keys)| {
            let path = format!("settings.{short}");
            let (group, fact) = locate(&path).unwrap();
            let mine = by_value(&fact_json(&metadata(group, fact).unwrap(), &qt[&path]["rawValue"], &units));
            let expected = by_value(&qt[&path]);
            assert!(keys.iter().any(|k| mine.get(*k) != expected.get(*k)), "{path} is listed as runtime but the metadata already answers it - take it off the list");
        });
    }
}
