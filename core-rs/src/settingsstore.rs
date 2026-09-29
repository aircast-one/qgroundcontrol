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

const I64_RANGE: f64 = 9_223_372_036_854_775_808.0;

fn unsigned_zero(text: String) -> String {
    match text.strip_prefix('-') {
        Some(rest) if rest.chars().all(|c| c == '0' || c == '.') => rest.to_string(),
        _ => text,
    }
}

fn spelled(value: &Value, decimals: i64, whole: bool) -> String {
    match value {
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        other => other.as_f64().map_or_else(String::new, |n| match whole {
            true if n.abs() > I64_RANGE => format!("{n}"),
            true => format!("{}", n as i64),
            false => unsigned_zero(half_away(n, usize::try_from(decimals).unwrap_or(0))),
        }),
    }
}

fn half_away(n: f64, decimals: usize) -> String {
    let scale = 10f64.powi(i32::try_from(decimals).unwrap_or(0));
    let rounded = (n * scale).round() / scale;
    format!("{:.decimals$}", if rounded.is_finite() { rounded } else { n })
}

fn decimal_places(meta: &MetaData, cooked: impl Fn(f64) -> f64) -> i64 {
    let from_increment = meta.increment.map(|increment| cooked(increment).fract().abs()).map(|fraction| if fraction == 0.0 { 0 } else { -(fraction.log10().ceil() as i64) });
    meta.decimal_places.or(from_increment).unwrap_or_else(|| ((DEFAULT_DECIMAL_PLACES as f64 - cooked(1.0).log10()) as i64).clamp(0, 25))
}

fn number_json(value: f64, whole: bool) -> Value {
    if !value.is_finite() {
        return Value::Null;
    }
    match (whole && value.abs() <= I64_RANGE) || (value.fract() == 0.0 && value.abs() < 1e15) {
        true => json!(value as i64),
        false => json!(value),
    }
}

pub fn fact_json(meta: &MetaData, raw: &Value, unit: Option<crate::units::Conversion>) -> Value {
    let whole = integer(&meta.value_type);
    let limits = type_limits(&meta.value_type);
    let raw_units = meta.units.clone().unwrap_or_default();
    let cooked = |v: f64| unit.map_or(v, |u| (u.shown)(v));
    let decimals = decimal_places(meta, cooked);
    let bound = |v: &Option<Value>| v.as_ref().and_then(Value::as_f64);
    let (raw_min, raw_max) = (bound(&meta.min).unwrap_or(limits.0), bound(&meta.max).unwrap_or(limits.1));
    let (shown_min, shown_max) = (cooked(raw_min), cooked(raw_max));
    let bool_typed = meta.value_type == ValueType::Bool;
    let single = meta.value_type == ValueType::Float;
    let bound_text = |v: f64| match (bool_typed, if single { f64::from(v as f32) } else { v }.is_finite()) {
        (true, _) => (v != 0.0).to_string(),
        (false, false) => if v > 0.0 { "inf" } else { "-inf" }.to_string(),
        (false, true) => spelled(&json!(if single { f64::from(v as f32) } else { v }), decimals, whole || meta.value_type == ValueType::String),
    };
    let real = matches!(meta.value_type, ValueType::Float | ValueType::Double);
    let matches = |v: &Value| v == raw || v.as_f64().zip(raw.as_f64()).is_some_and(|(a, b)| a == b || (real && (a - b).abs() < 1e-6));
    let unknown_label = format!("Unknown: {}", crate::control::raw_text(raw));
    let (listed, bits): (&[crate::factmeta::EnumEntry], &[crate::factmeta::EnumEntry]) = match (meta.bitmask, meta.bits.is_empty()) {
        (true, true) => (&[], &meta.enums),
        _ => (&meta.enums, &meta.bits),
    };
    let unknown = !listed.is_empty() && !listed.iter().any(|e| matches(&e.value));
    let labels: Vec<String> = listed.iter().map(|e| e.label.clone()).chain(unknown.then(|| unknown_label.clone())).collect();
    let values: Vec<Value> = listed.iter().map(|e| e.value.clone()).chain(unknown.then(|| raw.clone())).collect();
    let enum_index = values.iter().position(matches).map_or(-1, |i| i as i64);
    let cook = |given: &Value| match (unit, given.as_f64()) {
        (Some(_), Some(v)) => number_json(cooked(v), whole),
        _ => given.clone(),
    };
    let value = cook(raw);
    let value_string = spelled(&value, decimals, whole);
    let units = unit.map_or_else(|| raw_units.clone(), |u| u.name.to_string());
    let is_number = raw.is_number() || raw.is_null();
    let raw_default = default_of(meta);
    let default = raw_default.as_ref().map(cook);
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
        "valueEqualsDefault": raw_default.as_ref().is_some_and(|d| d == raw || d.as_f64().zip(raw.as_f64()).is_some_and(|(a, b)| a == b)),
        "min": number_json(shown_min, whole || !is_number),
        "max": number_json(shown_max, whole || !is_number),
        "minString": bound_text(shown_min),
        "maxString": bound_text(shown_max),
        "minIsDefaultForType": raw_min == limits.0,
        "maxIsDefaultForType": raw_max == limits.1,
        "typeIsBool": meta.value_type == ValueType::Bool,
        "typeIsInteger": whole,
        "typeIsString": meta.value_type == ValueType::String,
        "readOnly": meta.read_only,
        "qgcRebootRequired": meta.qgc_reboot_required,
        "vehicleRebootRequired": meta.vehicle_reboot_required,
        "unknownEnumLabel": unknown_label,
        "bitmaskStrings": bits.iter().map(|e| e.label.clone()).collect::<Vec<_>>(),
        "bitmaskValues": bits.iter().map(|e| e.value.clone()).collect::<Vec<_>>(),
        "userVisible": true,
        "visible": true,
    })
}

#[cfg(not(test))]
const ON_WITHOUT_SWITCH: bool = true;
#[cfg(test)]
const ON_WITHOUT_SWITCH: bool = false;

static SWITCHED_ON: LazyLock<bool> = LazyLock::new(|| std::env::var("QGC_CORE_SETTINGS").map_or(ON_WITHOUT_SWITCH, |v| v == "1"));

static STORED: Mutex<Option<BTreeMap<String, Setting>>> = Mutex::new(None);

fn stored() -> std::sync::MutexGuard<'static, Option<BTreeMap<String, Setting>>> {
    STORED.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn written(key: &str, text: &str) {
    if let Some(values) = stored().as_mut() {
        values.insert(key.to_string(), Setting::Text(text.to_string()));
    }
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

fn unit_for(meta: &MetaData) -> Option<crate::units::Conversion> {
    crate::units::for_fact(meta, crate::units::cooking)
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
    let mine = fact_json(&at.meta, &raw(at.group, &at.fact, &at.meta), unit_for(&at.meta));
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

const SAVE_DIRECTORIES: [(&str, &str); 9] = [
    ("parameterSavePath", "Parameters"),
    ("telemetrySavePath", "Telemetry"),
    ("missionSavePath", "Missions"),
    ("logSavePath", "Logs"),
    ("videoSavePath", "Video"),
    ("photoSavePath", "Photo"),
    ("crashSavePath", "CrashLogs"),
    ("mavlinkActionsSavePath", "MavlinkActions"),
    ("settingsSavePath", "Settings"),
];

fn child_save_path(root: &str, directory: &str) -> String {
    let root = std::path::Path::new(root);
    match !root.as_os_str().is_empty() && root.is_dir() {
        true => root.join(directory).to_string_lossy().into_owned(),
        false => String::new(),
    }
}

fn object_fields(backend: &dyn Backend, path: &str, fields: &str) -> Option<String> {
    let object = path.strip_prefix("settings.").filter(|rest| !rest.contains('.'))?;
    let group = OBJECTS.iter().find(|(name, _)| *name == object)?.1;
    let all: Vec<&str> = fields.split(',').map(str::trim).filter(|f| !f.is_empty()).collect();
    let (paths, asked): (Vec<&str>, Vec<&str>) = all.iter().partition(|f| object == "appSettings" && SAVE_DIRECTORIES.iter().any(|(name, _)| name == *f));
    let root = match paths.is_empty() {
        true => None,
        false => Some(stored_text(&key("App", "savePath"))?),
    };
    let saved: serde_json::Map<String, Value> = paths
        .iter()
        .filter_map(|name| SAVE_DIRECTORIES.iter().find(|(n, _)| n == name))
        .map(|(name, directory)| (name.to_string(), json!(child_save_path(root.as_deref().unwrap_or(""), directory))))
        .collect();
    let facts: Option<Vec<Value>> = asked
        .iter()
        .map(|fact| {
            let fact_path = format!("{path}.{fact}");
            let at = address(&fact_path)?;
            Some(crate::vehiclefact::compact(&described(backend, &at, &fact_path), fact))
        })
        .collect();
    let facts = facts.filter(|f| !f.is_empty() || !saved.is_empty())?;
    let mut answer = json!({ "kind": "object", "class": format!("{group}Settings"), "facts": facts, "children": [] });
    saved.into_iter().for_each(|(name, value)| answer[name] = value);
    Some(answer.to_string())
}

pub fn log_save_path() -> Option<String> {
    let root = stored_text(&key("App", "savePath"))?;
    Some(child_save_path(&root, "Logs")).filter(|path| !path.is_empty())
}

fn save_path(path: &str) -> Option<String> {
    let name = path.strip_prefix("settings.appSettings.")?;
    let (_, directory) = SAVE_DIRECTORIES.iter().find(|(n, _)| *n == name)?;
    let root = stored_text(&key("App", "savePath"))?;
    Some(json!({ "kind": "value", "value": child_save_path(&root, directory) }).to_string())
}

pub fn get_fields(backend: &dyn Backend, path: &str, fields: &str) -> Option<String> {
    if let Some(answered) = object_fields(backend, path, fields) {
        return Some(answered);
    }
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
    let raw_given = match (cooked, unit_for(&at.meta), given.as_f64()) {
        (true, Some(u), Some(n)) => json!((u.base)(n)),
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

const VIDEO_DISABLED: &str = "Video Stream Disabled";
const URL_SOURCES: [(&str, &str); 6] = [
    ("UDP h.264 Video Stream", "udpUrl"),
    ("UDP h.265 Video Stream", "udpUrl"),
    ("MPEG-TS Video Stream", "udpUrl"),
    ("RTSP Video Stream", "rtspUrl"),
    ("TCP-MPEG2 Video Stream", "tcpUrl"),
    ("WebRTC (WHEP) Video Stream", "whepUrl"),
];

fn setting_text(fact: &str) -> Option<String> {
    let meta = metadata("Video", fact)?;
    Some(crate::control::raw_text(&raw("Video", fact, &meta)))
}

fn video_source_at(index: i64, source: Option<String>, setting: &impl Fn(&str) -> Option<String>) -> Option<(String, String)> {
    let primary = || {
        let source = source.clone()?;
        let url = URL_SOURCES.iter().find(|(name, _)| *name == source).and_then(|(_, fact)| setting(fact)).unwrap_or_default();
        Some((source, url.trim().to_string()))
    };
    let extras: Vec<Value> = serde_json::from_str(&setting("extraVideoSources")?).unwrap_or_default();
    match (index > 0, usize::try_from(index - 1).ok().and_then(|at| extras.get(at))) {
        (false, _) => primary(),
        (true, Some(extra)) => Some((extra["source"].as_str().unwrap_or("").to_string(), extra["url"].as_str().unwrap_or("").trim().to_string())),
        (true, None) => primary().map(|(source, _)| (source, String::new())),
    }
}

fn video_answer(path: &str, args: &str, source: Option<String>, setting: impl Fn(&str) -> Option<String>) -> Option<String> {
    let index = serde_json::from_str::<Value>(args).ok()?.get(0)?.as_i64()?;
    let (source, url) = video_source_at(index, source, &setting)?;
    let result = match path {
        "settings.videoSettings.sourceEnabled" => source != VIDEO_DISABLED,
        "settings.videoSettings.sourceConfigured" => !URL_SOURCES.iter().any(|(name, _)| *name == source) || !url.is_empty(),
        _ => return None,
    };
    Some(json!({ "ok": true, "result": result }).to_string())
}

pub fn video_invoke(path: &str, args: &str) -> Option<String> {
    video_answer(path, args, stored_text("Video/videoSource"), setting_text)
}

pub struct Owner<B>(pub B);

impl<B: Backend> Backend for Owner<B> {
    fn get(&self, path: &str) -> String {
        enabled().then(|| crate::units::get(path).or_else(|| save_path(path)).or_else(|| get(&self.0, path))).flatten().unwrap_or_else(|| self.0.get(path))
    }
    fn get_fields(&self, path: &str, fields: &str) -> String {
        enabled().then(|| crate::units::fields(path, fields).or_else(|| get_fields(&self.0, path, fields))).flatten().unwrap_or_else(|| self.0.get_fields(path, fields))
    }
    fn set(&self, path: &str, value: &str) -> String {
        enabled().then(|| set(&self.0, path, value)).flatten().unwrap_or_else(|| self.0.set(path, value))
    }
    fn invoke(&self, path: &str, args: &str) -> String {
        enabled().then(|| crate::units::invoke(path, args).or_else(|| video_invoke(path, args))).flatten().unwrap_or_else(|| self.0.invoke(path, args))
    }
    fn watch(&self, paths: &[String]) {
        self.0.watch(paths);
    }
    fn core_guided(&self, action: &Value) -> Option<Result<(), String>> {
        self.0.core_guided(action)
    }
    fn remember_setting(&self, key: &str, value: &Value) {
        if let Some(values) = stored().as_mut() {
            values.insert(key.to_string(), Setting::Text(value.as_str().map_or_else(|| value.to_string(), str::to_string)));
        }
        self.0.remember_setting(key, value);
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
    fn a_child_save_path_exists_only_under_an_existing_root() {
        let root = std::env::temp_dir();
        assert_eq!(child_save_path(&root.to_string_lossy(), "Logs"), root.join("Logs").to_string_lossy());
        assert_eq!(child_save_path("/no/such/qgc/root", "Logs"), "", "Qt answers an empty path when the root folder is missing");
        assert_eq!(child_save_path("", "Logs"), "");
    }

    #[test]
    fn a_uint64_limit_is_the_double_qt_holds_it_in() {
        let meta = crate::factmeta::from_file(r#"{"QGC.MetaData.Facts":[{"name":"status","type":"uint64"}]}"#).unwrap().remove("status").unwrap();
        let fact = fact_json(&meta, &json!(4), None);
        assert_eq!((fact["max"].as_f64(), fact["maxString"].as_str()), (Some(18_446_744_073_709_551_615.0), Some("18446744073709552000")));
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
                let mine = by_value(&fact_json(&meta, &expected["rawValue"], crate::units::for_fact(&meta, crate::units::metric)));
                let expected = by_value(expected);
                let host = runtime_fields(path).unwrap_or(&[]);
                let keys: Vec<String> = expected.as_object().unwrap().iter().filter(|(k, v)| !host.contains(&k.as_str()) && mine.get(k.as_str()) != Some(v)).map(|(k, _)| k.clone()).collect();
                (!keys.is_empty()).then(|| format!("{path}: {}", keys.join(",")))
            })
            .collect();
        assert!(differing.is_empty(), "{} of {} differ:\n{}", differing.len(), qt.len(), differing.join("\n"));
    }

    #[test]
    fn an_imperial_fact_cooks_its_default_bounds_and_decimals_as_qgc_does() {
        let meta = metadata("FlyView", "guidedMinimumAltitude").unwrap();
        let feet = crate::units::cooking_with("vertical m", |_| None, crate::units::IMPERIAL_US);
        let fact = fact_json(&meta, &json!(2.0), feet);
        assert_eq!((fact["decimalPlaces"].as_i64(), fact["valueString"].as_str(), fact["units"].as_str()), (Some(2), Some("6.56"), Some("ft")), "three places less log10 of 3.28 ft to the metre");
        assert_eq!((fact["max"].clone(), fact["maxString"].as_str(), fact["maxIsDefaultForType"].as_bool()), (Value::Null, Some("inf"), Some(true)), "the type's own limit is converted too and overflows");
        assert_eq!(fact["defaultValueString"], "6.56");
        let speed = fact_json(&metadata("App", "offlineEditingCruiseSpeed").unwrap(), &json!(15.0), crate::units::cooking_with("m/s", |_| None, crate::units::IMPERIAL_US));
        assert_eq!(speed["units"], "mph");
    }

    #[test]
    fn a_video_source_is_enabled_unless_disabled_and_configured_once_its_url_is_set() {
        let setting = |fact: &str| match fact {
            "rtspUrl" => Some(String::new()),
            "extraVideoSources" => Some(json!([{ "source": "Video Stream Disabled", "url": "" }, { "source": "UDP h.264 Video Stream", "url": "0.0.0.0:5601" }]).to_string()),
            _ => None,
        };
        let ask = |path: &str, index: i64, source: Option<&str>| video_answer(&format!("settings.videoSettings.{path}"), &format!("[{index}]"), source.map(str::to_string), setting).map(|r| crate::read::object(&r)["result"].clone());
        let rtsp = Some("RTSP Video Stream");
        assert_eq!((ask("sourceEnabled", 0, rtsp), ask("sourceConfigured", 0, rtsp)), (Some(json!(true)), Some(json!(false))), "an RTSP source with no URL is on but not configured");
        assert_eq!(ask("sourceEnabled", 1, rtsp), Some(json!(false)));
        assert_eq!(ask("sourceConfigured", 2, rtsp), Some(json!(true)));
        assert_eq!(ask("sourceConfigured", 9, rtsp), Some(json!(false)), "past the extras Qt names the primary source but gives it no URL");
        assert_eq!(ask("sourceEnabled", 0, None), None, "with no stored source the default is Qt's to compute");
    }

    #[test]
    fn every_field_left_to_the_host_really_differs_from_the_metadata() {
        let qt: serde_json::Map<String, Value> = serde_json::from_str(include_str!("../tests/fixtures/settings-facts-by-qt.json")).unwrap();
        RUNTIME.iter().for_each(|(short, keys)| {
            let path = format!("settings.{short}");
            let (group, fact) = locate(&path).unwrap();
            let meta = metadata(group, fact).unwrap();
            let mine = by_value(&fact_json(&meta, &qt[&path]["rawValue"], crate::units::for_fact(&meta, crate::units::metric)));
            let expected = by_value(&qt[&path]);
            assert!(keys.iter().any(|k| mine.get(*k) != expected.get(*k)), "{path} is listed as runtime but the metadata already answers it - take it off the list");
        });
    }
}
