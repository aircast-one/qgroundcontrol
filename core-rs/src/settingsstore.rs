use serde_json::{Value, json};

use crate::factmeta::{MetaData, ValueType};

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

pub const RUNTIME: [(&str, &[&str]); 7] = [
    ("appSettings.androidDontSaveToSDCard", &["userVisible", "visible"]),
    ("appSettings.androidUsePosixSerial", &["userVisible", "visible"]),
    ("appSettings.indoorPalette", &["defaultValue", "defaultValueString", "valueEqualsDefault"]),
    ("appSettings.qLocaleLanguage", &["enumIndex", "enumOrValueString", "enumStrings", "enumValues"]),
    ("batteryIndicatorSettings.valueDisplay", &["defaultValue", "defaultValueString", "valueEqualsDefault"]),
    ("videoSettings.aspectRatio", &["defaultValue", "valueEqualsDefault"]),
    ("videoSettings.forceVideoDecoder", &["enumStrings", "enumValues"]),
];

pub const RUNTIME_WHOLE: [&str; 1] = ["videoSettings.videoSource"];

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
            false => format!("{n:.prec$}", prec = usize::try_from(decimals).unwrap_or(0)),
        }),
    }
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
        "defaultValueAvailable": meta.default.is_some(),
        "defaultValue": meta.default,
        "defaultValueString": meta.default.as_ref().map_or_else(String::new, |d| spelled(d, decimals, whole)),
        "valueEqualsDefault": meta.default.as_ref().is_some_and(|d| d == raw || d.as_f64().zip(raw.as_f64()).is_some_and(|(a, b)| a == b)),
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
