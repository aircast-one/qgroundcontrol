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

const LANGUAGES: [(i64, &str); 21] = [
    (0, "System"),
    (25, "Azerbaijani (Azerbaijani)"),
    (45, "български (Bulgarian)"),
    (58, "中文 (Chinese)"),
    (72, "Nederlands (Dutch)"),
    (75, "English"),
    (84, "Suomi (Finnish)"),
    (85, "Français (French)"),
    (94, "Deutsche (German)"),
    (96, "Ελληνικά (Greek)"),
    (103, "עברית (Hebrew)"),
    (119, "Italiano (Italian)"),
    (120, "日本語 (Japanese)"),
    (142, "한국어 (Korean)"),
    (209, "Norsk (Norwegian)"),
    (230, "Polskie (Polish)"),
    (231, "Português (Portuguese)"),
    (239, "Pусский (Russian)"),
    (270, "Español (Spanish)"),
    (275, "Svenska (Swedish)"),
    (298, "Türk (Turkish)"),
];
const RELEASE_LANGUAGES: [i64; 7] = [75, 25, 58, 120, 142, 231, 239];
const PARTIAL_LANGUAGES: [i64; 1] = [303];
const FOLLOW_SYSTEM_PALETTE: i64 = 2;

pub fn language_enums() -> Vec<crate::factmeta::EnumEntry> {
    let entry = |value: i64, label: String| crate::factmeta::EnumEntry { label, value: json!(value) };
    let rest = || LANGUAGES.iter();
    std::iter::once(entry(LANGUAGES[0].0, LANGUAGES[0].1.to_string()))
        .chain(rest().filter(|(id, _)| RELEASE_LANGUAGES.contains(id)).map(|(id, name)| entry(*id, (*name).to_string())))
        .chain(rest().filter(|(id, _)| PARTIAL_LANGUAGES.contains(id)).map(|(id, name)| entry(*id, format!("{name} (Partial)"))))
        .chain(rest().filter(|(id, _)| !RELEASE_LANGUAGES.contains(id) && !PARTIAL_LANGUAGES.contains(id)).map(|(id, name)| entry(*id, format!("{name} (Test Only)"))))
        .collect()
}

pub fn hidden_on_this_platform(group: &str, fact: &str) -> bool {
    match (group, fact) {
        ("App", "androidDontSaveToSDCard") => !cfg!(target_os = "android"),
        ("App", "androidUsePosixSerial") => true,
        ("Viewer3D", "enabled") => cfg!(target_os = "android"),
        _ => false,
    }
}

fn text_enums(names: Vec<String>) -> Vec<crate::factmeta::EnumEntry> {
    names.into_iter().map(|name| crate::factmeta::EnumEntry { label: name.clone(), value: json!(name) }).collect()
}

fn platform_meta(group: &str, fact: &str, meta: MetaData) -> MetaData {
    match (group, fact) {
        ("App", "indoorPalette") => MetaData { default: Some(json!(if cfg!(target_os = "android") { FOLLOW_SYSTEM_PALETTE } else if cfg!(target_os = "ios") { 0 } else { 1 })), ..meta },
        ("App", "qLocaleLanguage") => MetaData { enums: language_enums(), ..meta },
        ("FlightMap", "mapProvider") => MetaData { enums: text_enums(crate::maptypes::map_provider_list()), ..meta },
        ("FlightMap", "mapType") => MetaData {
            enums: text_enums(crate::maptypes::map_type_list(&raw_setting("settings.flightMapSettings.mapProvider").and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default())),
            ..meta
        },
        ("Video", "videoSource") => MetaData { enums: text_enums(stream_sources()), default: Some(json!(VIDEO_DISABLED)), ..meta },
        ("FlightMap", "elevationMapProvider") => MetaData { enums: text_enums(crate::maptypes::ELEVATION_PROVIDERS.iter().map(|p| p.to_string()).collect()), ..meta },
        _ => meta,
    }
}

pub fn runtime_fields(path: &str) -> Option<&'static [&'static str]> {
    let short = path.strip_prefix("settings.")?;
    RUNTIME.iter().find(|(name, _)| *name == short).map(|(_, keys)| *keys)
}

pub fn served_by_host(path: &str) -> bool {
    crate::qthost::present() && path.strip_prefix("settings.").is_some_and(|short| RUNTIME_WHOLE.contains(&short))
}

pub fn locate(path: &str) -> Option<(&'static str, &str)> {
    let rest = path.strip_prefix("settings.")?;
    let (object, fact) = rest.split_once('.')?;
    let group = OBJECTS.iter().find(|(name, _)| *name == object)?.1;
    Some((group, fact))
}

pub fn metadata(group: &str, fact: &str) -> Option<MetaData> {
    if let Some(units) = (group == "Units").then(|| crate::units::fact_metadata(fact)).flatten() {
        return Some(units);
    }
    let json = crate::settingsgroups::group(group).map(|g| g.json).or_else(|| EXTRA_GROUPS.iter().find(|(name, _)| *name == group).map(|(_, json)| *json))?;
    crate::factmeta::from_file(json).ok()?.remove(fact)
}

fn with_property(fact: Value, property: &str) -> Value {
    match fact {
        Value::Object(fields) => Value::Object(fields.into_iter().chain(std::iter::once(("property".to_string(), json!(property)))).collect()),
        other => other,
    }
}

fn whole_group(backend: &dyn Backend, path: &str) -> Option<String> {
    let object = path.strip_prefix("settings.").filter(|rest| !rest.contains('.'))?;
    let group = OBJECTS.iter().find(|(name, _)| *name == object)?.1;
    let names = crate::settingsorder::ORDER.iter().find(|(name, _)| *name == group)?.1;
    let facts: Vec<Value> = names
        .iter()
        .filter_map(|fact| {
            let fact_path = format!("{path}.{fact}");
            let at = address(&fact_path)?;
            Some(with_property(described(backend, &at, &fact_path), fact))
        })
        .collect();
    Some(json!({ "kind": "object", "class": format!("{group}Settings"), "facts": facts, "children": [] }).to_string())
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
            false => unsigned_zero(fixed_as_qt(n, usize::try_from(decimals).unwrap_or(0))),
        }),
    }
}

const TIE_DIGITS: usize = 30;

pub fn fixed_as_qt(n: f64, decimals: usize) -> String {
    let expanded = format!("{:.*}", decimals + TIE_DIGITS, n.abs());
    let beyond = &expanded[expanded.len() - TIE_DIGITS..];
    let tie = beyond.starts_with('5') && beyond[1..].bytes().all(|b| b == b'0');
    let scale = 10f64.powi(i32::try_from(decimals).unwrap_or(0));
    match tie && n.is_finite() {
        true => format!("{:.decimals$}", n.signum() * ((n.abs() * scale).floor() + 1.0) / scale),
        false => format!("{n:.decimals$}"),
    }
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

fn elapsed_text(seconds: f64) -> String {
    let whole = (seconds.trunc() as i64).rem_euclid(24 * 3600);
    format!("{:02}:{:02}:{:02}", whole / 3600, whole / 60 % 60, whole % 60)
}

const FACT_DEFAULT_CATEGORY: &str = "Other";
const FACT_DEFAULT_GROUP: &str = "Misc";

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
    let value_string = match (meta.value_type == ValueType::ElapsedSeconds, value.as_f64()) {
        (true, Some(seconds)) => elapsed_text(seconds),
        _ => spelled(&value, decimals, whole),
    };
    let units = unit.map_or_else(|| raw_units.clone(), |u| u.name.to_string());
    let is_number = raw.is_number() || raw.is_null();
    let raw_default = default_of(meta);
    let default = raw_default.as_ref().map(cook);
    json!({
        "kind": "fact",
        "name": meta.name,
        "label": meta.label,
        "shortDescription": meta.short_description,
        "category": meta.category.clone().unwrap_or_else(|| FACT_DEFAULT_CATEGORY.to_string()),
        "group": meta.group.clone().unwrap_or_else(|| FACT_DEFAULT_GROUP.to_string()),
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
        "maxStringLength": meta.max_string_length.unwrap_or(0),
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

static PATH: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);

pub fn written(key: &str, text: &str) {
    if let Some(values) = stored().as_mut() {
        values.insert(key.to_string(), Setting::Text(text.to_string()));
    }
    persist();
}

pub fn forgotten(key: &str) {
    if let Some(values) = stored().as_mut() {
        values.remove(key);
    }
    persist();
}

fn clear_asked(values: &BTreeMap<String, Setting>) -> bool {
    matches!(values.get(&key("App", "clearSettingsNextBoot")), Some(Setting::Text(text)) if text == "true" || text == "1")
}

pub fn cleared_on_boot(values: BTreeMap<String, Setting>) -> BTreeMap<String, Setting> {
    if clear_asked(&values) { BTreeMap::new() } else { values }
}

pub fn open(path: &std::path::Path) {
    let read = crate::settingsini::read(&std::fs::read_to_string(path).unwrap_or_default());
    if clear_asked(&read)
        && let Some(cache) = crate::paramcache::folder_for(path)
    {
        let _ = std::fs::remove_dir_all(cache);
    }
    *stored() = Some(cleared_on_boot(read));
    *PATH.lock().unwrap_or_else(PoisonError::into_inner) = Some(path.to_path_buf());
}

pub fn persist() {
    if crate::qthost::present() {
        return;
    }
    let Some(path) = PATH.lock().unwrap_or_else(PoisonError::into_inner).clone() else { return };
    let Some(text) = stored().as_ref().map(crate::settingsini::write) else { return };
    let staged = path.with_extension("ini.saving");
    if std::fs::write(&staged, text).is_ok() {
        let _ = std::fs::rename(&staged, &path);
    }
}

pub fn replace_group(group: &str, entries: BTreeMap<String, Setting>) {
    let prefix = format!("{group}/");
    if let Some(values) = stored().as_mut() {
        values.retain(|key, _| !key.starts_with(&prefix));
        values.extend(entries);
    }
    persist();
}

pub fn entries_under(group: &str) -> BTreeMap<String, Setting> {
    let prefix = format!("{group}/");
    stored().as_ref().map(|values| values.iter().filter(|(key, _)| key.starts_with(&prefix)).map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default()
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

pub fn raw_setting(path: &str) -> Option<Value> {
    let at = address(path)?;
    Some(raw(at.group, &at.fact, &at.meta))
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
    let meta = match crate::qthost::present() {
        true => meta,
        false => platform_meta(group, &fact, meta),
    };
    Some(Addressed { group, fact, field, meta })
}

fn described(backend: &dyn Backend, at: &Addressed, path: &str) -> Value {
    let hidden = !crate::qthost::present() && hidden_on_this_platform(at.group, &at.fact);
    let value = match (hidden, &at.meta.default) {
        (true, Some(default)) => default.clone(),
        _ => raw(at.group, &at.fact, &at.meta),
    };
    let mut mine = fact_json(&at.meta, &value, unit_for(&at.meta));
    if hidden {
        mine["userVisible"] = json!(false);
        mine["visible"] = json!(false);
    }
    let fact_path = path.split('.').take(3).collect::<Vec<_>>().join(".");
    match crate::qthost::present().then(|| runtime_fields(&fact_path)).flatten() {
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
    if path == "settings.remoteIDSettings.operatorIDValidForRegion" {
        return Some(json!({ "kind": "value", "value": operator_id_valid_for_region() }).to_string());
    }
    if let Some(whole) = whole_group(backend, path) {
        return Some(whole);
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

pub fn file() -> Option<std::path::PathBuf> {
    PATH.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

pub fn folder() -> Option<std::path::PathBuf> {
    PATH.lock().unwrap_or_else(PoisonError::into_inner).as_ref().and_then(|path| path.parent().map(std::path::Path::to_path_buf))
}

pub fn telemetry_save_path() -> Option<String> {
    let root = stored_text(&key("App", "savePath"))?;
    Some(child_save_path(&root, "Telemetry")).filter(|path| !path.is_empty())
}

pub fn default_save_path(application: &str) -> Option<std::path::PathBuf> {
    let home = std::path::PathBuf::from(std::env::var_os("HOME")?);
    Some(home.join("Documents").join(application))
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveRoots {
    pub internal: std::path::PathBuf,
    pub removable: Option<std::path::PathBuf>,
}

static SAVE_ROOTS: Mutex<Option<SaveRoots>> = Mutex::new(None);

fn usable(dir: &std::path::Path) -> bool {
    std::fs::create_dir_all(dir).is_ok() && std::fs::metadata(dir).is_ok_and(|m| m.is_dir() && !m.permissions().readonly())
}

pub fn chosen_save_root(roots: &SaveRoots, dont_save_to_sd_card: bool) -> std::path::PathBuf {
    roots.removable.clone().filter(|sd| !dont_save_to_sd_card && usable(sd)).unwrap_or_else(|| roots.internal.clone())
}

fn dont_save_to_sd_card() -> bool {
    stored_text(&key("App", "androidDontSaveToSDCard")).is_some_and(|text| text == "true" || text == "1")
}

pub fn establish_save_path(given: Option<&str>, removable: Option<&str>, application: &str) {
    let root = match given {
        Some(path) => Some(std::path::PathBuf::from(path)),
        None => match stored_text(&key("App", "savePath")).filter(|path| !path.is_empty()) {
            Some(_) => None,
            None => default_save_path(application),
        },
    };
    let roots = root.clone().map(|internal| SaveRoots { internal, removable: removable.map(std::path::PathBuf::from) });
    *SAVE_ROOTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = roots.clone();
    if let Some(chosen) = roots.map(|roots| chosen_save_root(&roots, dont_save_to_sd_card())).or(root) {
        written(&key("App", "savePath"), &chosen.to_string_lossy());
    }
    create_save_directories();
}

fn create_save_directories() {
    if let Some(saved) = stored_text(&key("App", "savePath")).filter(|path| !path.is_empty()).map(std::path::PathBuf::from)
        && std::fs::create_dir_all(&saved).is_ok()
    {
        SAVE_DIRECTORIES.iter().for_each(|(_, directory)| {
            let _ = std::fs::create_dir_all(saved.join(directory));
        });
    }
}

pub fn parameter_save_path() -> Option<String> {
    let root = stored_text(&key("App", "savePath"))?;
    Some(child_save_path(&root, "Parameters")).filter(|path| !path.is_empty())
}

pub fn video_save_path() -> Option<String> {
    let root = stored_text(&key("App", "savePath"))?;
    Some(child_save_path(&root, "Video")).filter(|path| !path.is_empty())
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

fn store_raw(at: &Addressed, raw_given: &Value) {
    if let Some(new) = typed(&at.meta.value_type, raw_given) {
        let spelled = match &new {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        let before = raw(at.group, &at.fact, &at.meta);
        if let Some(values) = stored().as_mut() {
            values.insert(key(at.group, &at.fact), Setting::Text(spelled));
            follow_ups(at.group, &at.fact, &new).into_iter().filter(|_| before != new).for_each(|(fact, value)| {
                values.insert(key(at.group, fact), Setting::Text(value));
            });
        }
        persist();
        if at.group == "App" && at.fact == "androidDontSaveToSDCard" && before != new {
            create_save_directories();
        }
        if at.group == "App" && at.fact == "defaultMissionItemAltitude" && before != new {
            crate::coreplan::default_altitude_changed();
        }
    }
}

pub fn set_raw(path: &str, raw_given: &Value) {
    if let Some(at) = address(path) {
        store_raw(&at, raw_given);
    }
}

fn enum_index_raw(meta: &MetaData, given: &Value) -> Option<Value> {
    (!meta.bitmask).then_some(())?;
    let index = usize::try_from(given.as_i64()?).ok()?;
    meta.enums.get(index).map(|entry| entry.value.clone())
}

pub fn set(backend: &dyn Backend, path: &str, value: &str) -> Option<String> {
    let at = address(path)?;
    let written = crate::read::object(value);
    let given = written.get("value").cloned().unwrap_or(written);
    let raw_given = match (at.field.as_deref(), unit_for(&at.meta), given.as_f64()) {
        (None | Some("value"), Some(u), Some(n)) => json!((u.base)(n)),
        (None | Some("value" | "rawValue"), _, _) => given,
        (Some("enumIndex"), _, _) => enum_index_raw(&at.meta, &given)?,
        _ => return None,
    };
    if let Some(reason) = refused_write(at.group, &at.fact, &raw_given) {
        return Some(json!({ "ok": false, "reason": reason }).to_string());
    }
    store_raw(&at, &raw_given);
    match crate::qthost::present() {
        true => Some(backend.set(path, value)),
        false => Some(json!({ "ok": true }).to_string()),
    }
}

pub fn unit_system_writes(system: i64) -> Vec<(&'static str, Value)> {
    let presets: [[i64; 5]; 2] = [[1, 1, 1, 1, 0], [0, 0, 5, 2, 1]];
    let facts = ["horizontalDistanceUnits", "verticalDistanceUnits", "areaUnits", "speedUnits", "temperatureUnits"];
    let chosen = usize::try_from(system).ok().and_then(|s| presets.get(s)).map(|preset| facts.iter().zip(preset.iter()).map(|(fact, value)| (*fact, json!(value))).collect::<Vec<_>>()).unwrap_or_default();
    std::iter::once(("customUnits", json!(system == 2))).chain(chosen).collect()
}

fn owned_invoke(path: &str, args: &str) -> Option<String> {
    (!crate::qthost::present() && path == "settings.unitsSettings.setUnitSystem").then_some(())?;
    let system = serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0).and_then(Value::as_i64))?;
    unit_system_writes(system).iter().for_each(|(fact, value)| written(&format!("Units/{fact}"), &value.to_string()));
    Some(json!({ "ok": true }).to_string())
}

const EU_PUBLIC_OPERATOR_ID_LENGTH: usize = 16;

const INVALID_EU_OPERATOR_ID: &str = "Invalid Operator ID. Enter the full 19 or 20 character ID including the 3 secret characters.";

fn refused_write(group: &str, fact: &str, raw: &Value) -> Option<&'static str> {
    let candidate = raw.as_str()?;
    let stored = raw_setting(&format!("settings.remoteIDSettings.{fact}")).and_then(|v| v.as_str().map(str::to_string));
    let legal = candidate.is_empty() || stored.as_deref() == Some(candidate) || crate::remoteid::eu_operator_id_valid(candidate);
    (group == "RemoteID" && fact == "operatorIDEU" && !legal).then_some(INVALID_EU_OPERATOR_ID)
}

fn follow_ups(group: &str, fact: &str, new: &Value) -> Vec<(&'static str, String)> {
    match (group, fact) {
        ("FlightMap", "mapProvider") => new
            .as_str()
            .and_then(|provider| crate::maptypes::map_type_list(provider).into_iter().next())
            .map(|first| vec![("mapType", first)])
            .unwrap_or_default(),
        ("App", "androidDontSaveToSDCard") => SAVE_ROOTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .map(|roots| vec![("savePath", chosen_save_root(&roots, new.as_bool() == Some(true)).to_string_lossy().into_owned())])
            .unwrap_or_default(),
        ("RemoteID", "region") if new.as_i64() == Some(crate::remoteid::REGION_EU) => vec![("sendOperatorID", "true".to_string())],
        ("RemoteID", "region") if new.as_i64() == Some(crate::remoteid::REGION_FAA) => vec![("locationType", crate::remoteid::LOCATION_LIVE.to_string())],
        ("RemoteID", "operatorIDEU") => new
            .as_str()
            .filter(|id| id.chars().count() > EU_PUBLIC_OPERATOR_ID_LENGTH && crate::remoteid::eu_operator_id_valid(id))
            .map(|id| vec![("operatorIDEU", id.chars().take(EU_PUBLIC_OPERATOR_ID_LENGTH).collect())])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn operator_id_valid_for_region() -> bool {
    let eu = raw_setting("settings.remoteIDSettings.region").and_then(|r| r.as_i64()) == Some(crate::remoteid::REGION_EU);
    let fact = if eu { "operatorIDEU" } else { "operatorIDFAA" };
    raw_setting(&format!("settings.remoteIDSettings.{fact}")).and_then(|id| id.as_str().map(|id| !id.is_empty())).unwrap_or(false)
}

pub const VIDEO_DISABLED: &str = "Video Stream Disabled";

const STREAM_SOURCE_ORDER: [&str; 6] = ["RTSP Video Stream", "UDP h.264 Video Stream", "UDP h.265 Video Stream", "TCP-MPEG2 Video Stream", "MPEG-TS Video Stream", "WebRTC (WHEP) Video Stream"];

fn stream_sources() -> Vec<String> {
    std::iter::once(VIDEO_DISABLED).chain(STREAM_SOURCE_ORDER).map(str::to_string).collect()
}
pub const URL_SOURCES: [(&str, &str); 6] = [
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
    video_answer(path, args, Some(stored_text("Video/videoSource").unwrap_or_else(|| VIDEO_DISABLED.to_string())), setting_text)
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
        enabled().then(|| crate::units::invoke(path, args).or_else(|| video_invoke(path, args)).or_else(|| owned_invoke(path, args))).flatten().unwrap_or_else(|| self.0.invoke(path, args))
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
        persist();
        self.0.remember_setting(key, value);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn without_qt_the_video_source_offers_qgcs_stream_sources_in_its_order() {
        let qt: serde_json::Map<String, Value> = serde_json::from_str(include_str!("../tests/fixtures/settings-facts-by-qt.json")).unwrap();
        let listed: Vec<String> = qt["settings.videoSettings.videoSource"]["enumStrings"].as_array().unwrap().iter().filter_map(|v| v.as_str().map(str::to_string)).take(7).collect();
        assert_eq!(stream_sources(), listed, "the stream sources VideoSettings lists before the platform's cameras");
        assert!(STREAM_SOURCE_ORDER.iter().all(|name| URL_SOURCES.iter().any(|(source, _)| source == name)), "every offered stream has a URL setting");
    }

    #[test]
    fn the_platform_rules_answer_what_qt_answered_on_macos() {
        let qt: serde_json::Map<String, Value> = serde_json::from_str(include_str!("../tests/fixtures/settings-facts-by-qt.json")).unwrap();
        let local = |path: &str| {
            let at = address(path).unwrap();
            let hidden = hidden_on_this_platform(at.group, &at.fact);
            let meta = platform_meta(at.group, &at.fact, at.meta.clone());
            let mut fact = fact_json(&meta, &qt[path]["rawValue"], None);
            if hidden {
                fact["userVisible"] = json!(false);
                fact["visible"] = json!(false);
            }
            fact
        };
        ["settings.appSettings.androidDontSaveToSDCard", "settings.appSettings.androidUsePosixSerial"].iter().for_each(|path| {
            assert_eq!((local(path)["userVisible"].clone(), local(path)["visible"].clone()), (qt[*path]["userVisible"].clone(), qt[*path]["visible"].clone()), "{path}");
        });
        let palette = local("settings.appSettings.indoorPalette");
        assert_eq!((palette["defaultValue"].clone(), palette["valueEqualsDefault"].clone()), (qt["settings.appSettings.indoorPalette"]["defaultValue"].clone(), qt["settings.appSettings.indoorPalette"]["valueEqualsDefault"].clone()));
        let languages: Vec<Value> = qt["settings.appSettings.qLocaleLanguage"]["enumValues"].as_array().unwrap().clone();
        let mine: Vec<Value> = local("settings.appSettings.qLocaleLanguage")["enumValues"].as_array().unwrap().clone();
        assert_eq!(mine[..], languages[..mine.len()], "the macOS fixture is a debug build, which adds pseudo-localization after the list");
    }

    #[test]
    fn the_language_list_follows_app_settings_q_locale_language() {
        let listed = language_enums();
        let labels: Vec<&str> = listed.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(labels[..8], ["System", "Azerbaijani (Azerbaijani)", "中文 (Chinese)", "English", "日本語 (Japanese)", "한국어 (Korean)", "Português (Portuguese)", "Pусский (Russian)"], "System first, then the release languages in _rgLanguageInfo order");
        assert_eq!(listed.len(), LANGUAGES.len() + 1, "Ukrainian is partial but has no _rgLanguageInfo entry, and the test-only loop walks System too");
        assert_eq!(labels[8], "System (Test Only)");
        assert!(labels.iter().skip(8).all(|label| label.ends_with(" (Test Only)")));
        assert_eq!(listed[3].value, json!(75), "QLocale::English");
    }

    #[test]
    fn a_number_is_written_as_qt_arg_f_writes_it_from_its_exact_binary_value() {
        assert_eq!(fixed_as_qt(584.05, 1), "584.0", "584.05 is 584.0499... in binary, so it rounds down, as QString::arg does");
        assert_eq!(fixed_as_qt(0.15, 1), "0.1");
        assert_eq!(fixed_as_qt(1.005, 2), "1.00");
        assert_eq!(fixed_as_qt(0.125, 2), "0.13", "an exact tie rounds away from zero, where Rust's formatter would round to even");
        assert_eq!(fixed_as_qt(2.5, 0), "3");
        assert_eq!(fixed_as_qt(-2.5, 0), "-3");
        assert_eq!(fixed_as_qt(0.5, 0), "1");
        assert_eq!(fixed_as_qt(12.34, 1), "12.3");
    }

    use super::*;

    #[test]
    fn data_goes_to_the_sd_card_unless_told_not_to_or_it_cannot_be_written() {
        let base = std::env::temp_dir().join(format!("qgc-save-roots-{}", std::process::id()));
        let roots = SaveRoots { internal: base.join("internal"), removable: Some(base.join("sd")) };
        assert_eq!(chosen_save_root(&roots, false), base.join("sd"), "AppSettings saves to the SD card by default");
        assert_eq!(chosen_save_root(&roots, true), base.join("internal"), "androidDontSaveToSDCard keeps it internal");
        assert_eq!(chosen_save_root(&SaveRoots { removable: None, ..roots.clone() }, false), base.join("internal"), "no card, internal storage");
        let blocked = base.join("file");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(&blocked, b"").unwrap();
        assert_eq!(chosen_save_root(&SaveRoots { removable: Some(blocked.join("sd")), ..roots }, false), base.join("internal"), "a card that cannot be written falls back to internal storage");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn choosing_a_map_provider_moves_the_map_type_to_its_first() {
        assert_eq!(follow_ups("FlightMap", "mapProvider", &json!("Bing")), vec![("mapType", "Road".to_string())], "MapSettings sets mapType to mapTypeList(provider)[0]");
        assert!(follow_ups("FlightMap", "mapProvider", &json!("Nope")).is_empty());
        assert_eq!(text_enums(vec!["Google".into()])[0].value, json!("Google"));
    }

    struct Silent;
    impl Backend for Silent {
        fn get(&self, _path: &str) -> String { String::new() }
        fn get_fields(&self, _path: &str, _fields: &str) -> String { String::new() }
        fn set(&self, _path: &str, _value: &str) -> String { String::new() }
        fn invoke(&self, _path: &str, _args: &str) -> String { String::new() }
        fn watch(&self, _paths: &[String]) {}
    }

    #[test]
    fn a_whole_group_serves_full_facts_a_settings_page_can_edit() {
        let group: Value = serde_json::from_str(&get(&Silent, "settings.appSettings").unwrap()).unwrap();
        let facts = group["facts"].as_array().unwrap();
        let muted = facts.iter().find(|f| f["property"] == "audioMuted").unwrap();
        assert_eq!(muted["readOnly"], json!(false), "a compact fact without readOnly decodes as read-only and the page cannot edit it");
        let palette = facts.iter().find(|f| f["property"] == "indoorPalette").unwrap();
        assert!(palette["enumStrings"].as_array().is_some_and(|e| !e.is_empty()), "an enum keeps its choices");
    }

    #[test]
    fn an_enum_index_write_stores_that_choice_raw_value() {
        let palette = address("settings.appSettings.indoorPalette.enumIndex").unwrap();
        assert_eq!(palette.field.as_deref(), Some("enumIndex"));
        let labels: Vec<_> = palette.meta.enums.iter().map(|e| (e.label.clone(), e.value.clone())).collect();
        let outdoor = labels.iter().position(|(label, _)| label == "Outdoor").unwrap();
        assert_eq!(enum_index_raw(&palette.meta, &json!(outdoor)), Some(labels[outdoor].1.clone()), "the index names a choice; its raw value is what the setting holds");
        assert_eq!(enum_index_raw(&palette.meta, &json!(labels.len())), None, "past the list is not a choice");
        assert_eq!(enum_index_raw(&palette.meta, &json!(-1)), None);
    }

    #[test]
    fn asking_to_clear_settings_empties_them_on_the_next_start() {
        let asked: BTreeMap<String, Setting> = [(key("App", "clearSettingsNextBoot"), Setting::Text("true".into())), ("App/audioMuted".to_string(), Setting::Text("true".into()))].into_iter().collect();
        assert!(cleared_on_boot(asked).is_empty());
        let kept: BTreeMap<String, Setting> = [(key("App", "clearSettingsNextBoot"), Setting::Text("false".into())), ("App/audioMuted".to_string(), Setting::Text("true".into()))].into_iter().collect();
        assert_eq!(cleared_on_boot(kept.clone()), kept);
    }

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
    fn a_unit_system_is_unitssettings_preset() {
        assert_eq!(unit_system_writes(1), vec![("customUnits", json!(false)), ("horizontalDistanceUnits", json!(0)), ("verticalDistanceUnits", json!(0)), ("areaUnits", json!(5)), ("speedUnits", json!(2)), ("temperatureUnits", json!(1))]);
        assert_eq!(unit_system_writes(0)[3], ("areaUnits", json!(1)));
        assert_eq!(unit_system_writes(2), vec![("customUnits", json!(true))], "custom keeps whatever units are set");
    }

    #[test]
    fn remote_id_writes_carry_the_follow_ups_remote_id_settings_applies() {
        assert_eq!(follow_ups("RemoteID", "region", &json!(1)), vec![("sendOperatorID", "true".to_string())], "EU regulation requires broadcasting the operator ID");
        assert_eq!(follow_ups("RemoteID", "region", &json!(0)), vec![("locationType", "1".to_string())], "the FAA requires the live operator position");
        let number = "87astrdge12k";
        let check = crate::remoteid::luhn_mod36(&format!("{number}xyz")).unwrap();
        let full = format!("FIN{number}{check}-xyz");
        assert_eq!(follow_ups("RemoteID", "operatorIDEU", &json!(full)), vec![("operatorIDEU", format!("FIN{number}{check}"))], "the three secret characters are never stored");
        assert!(follow_ups("RemoteID", "operatorIDEU", &json!("FIN87astrdge12kQ-abc")).is_empty());
        assert_eq!(refused_write("RemoteID", "operatorIDEU", &json!("FIN87astrdge12kQ-abc")), Some(INVALID_EU_OPERATOR_ID), "RemoteIDSettings' cooked validator refuses an invalid EU ID, so its secret is never stored or sent");
        assert_eq!(refused_write("RemoteID", "operatorIDEU", &json!(full)), None);
        assert_eq!(refused_write("RemoteID", "operatorIDEU", &json!("")), None, "clearing is always legal");
        assert!(follow_ups("App", "region", &json!(1)).is_empty());
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
                if path.strip_prefix("settings.").is_some_and(|short| RUNTIME_WHOLE.contains(&short)) {
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
        assert_eq!(ask("sourceEnabled", 0, Some(VIDEO_DISABLED)), Some(json!(false)), "VideoSettings defaults videoSource to disabled whenever a source exists, and RTSP always does");
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
