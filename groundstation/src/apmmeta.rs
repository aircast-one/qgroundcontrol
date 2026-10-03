use serde_json::Value;
use std::collections::BTreeMap;

use crate::factmeta::{EnumEntry, MetaData, ValueType};

pub const DEFAULT_GROUP: &str = "Misc";

pub fn group_from_name(name: &str) -> String {
    name.split('_').next().unwrap_or("").trim_end_matches(|c: char| c.is_ascii_digit()).to_string()
}

include!(concat!(env!("OUT_DIR"), "/apm_parameter_files.rs"));

#[derive(Debug, Default)]
pub struct JsonParameters {
    pub by_name: BTreeMap<String, (String, serde_json::Map<String, Value>)>,
}

pub fn parse_json(text: &str) -> Result<JsonParameters, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let groups = root.as_object().ok_or("the parameter file is not an object")?;
    let by_name: BTreeMap<String, (String, serde_json::Map<String, Value>)> = groups
        .values()
        .filter_map(Value::as_object)
        .flat_map(|params| params.iter().filter_map(|(name, fields)| Some((name.clone(), (group_from_name(name), fields.as_object()?.clone())))))
        .collect();
    let members = by_name.values().fold(BTreeMap::<String, usize>::new(), |mut counts, (group, _)| {
        *counts.entry(group.clone()).or_default() += 1;
        counts
    });
    let by_name = by_name.into_iter().map(|(name, (group, fields))| (name, (if members.get(&group) == Some(&1) { DEFAULT_GROUP.to_string() } else { group }, fields))).collect();
    Ok(JsonParameters { by_name })
}

pub fn vehicle_file_name(mav_type: u8) -> Option<&'static str> {
    Some(match crate::modes::vehicle_class(mav_type) {
        crate::modes::VehicleClass::MultiRotor => "Copter",
        crate::modes::VehicleClass::FixedWing => "Plane",
        crate::modes::VehicleClass::Rover => "Rover",
        crate::modes::VehicleClass::Sub => "Sub",
        crate::modes::VehicleClass::Other => return None,
    })
}

fn embedded(vehicle: &str, major: i64, minor: i64) -> Option<(&'static str, &'static [u8])> {
    let wanted = format!("{vehicle}-{major}.{minor}");
    APM_PARAMETER_FILES.iter().find(|(name, _)| *name == wanted).copied()
}

pub fn file_for(vehicle: &str, major: i64, minor: i64) -> Option<(&'static str, &'static [u8])> {
    let newest = std::iter::successors(Some((major, minor)), |&(major, minor)| Some(if minor - 1 == 0 { (major - 1, 10) } else { (major, minor - 1) }))
        .take_while(|&(major, minor)| major >= 4 && minor > 0)
        .find_map(|(major, minor)| embedded(vehicle, major, minor));
    newest.or_else(|| (0..10).find_map(|minor| embedded(vehicle, 4, minor)))
}

static LOADED: std::sync::LazyLock<std::sync::Mutex<BTreeMap<&'static str, std::sync::Arc<JsonParameters>>>> = std::sync::LazyLock::new(Default::default);

pub fn load(vehicle: &str, major: i64, minor: i64) -> Option<std::sync::Arc<JsonParameters>> {
    let (name, gz) = file_for(vehicle, major, minor)?;
    let mut loaded = LOADED.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(found) = loaded.get(name) {
        return Some(found.clone());
    }
    let mut text = String::new();
    std::io::Read::read_to_string(&mut flate2::read::GzDecoder::new(gz), &mut text).ok()?;
    let parsed = std::sync::Arc::new(parse_json(&text).ok()?);
    loaded.insert(name, parsed.clone());
    Some(parsed)
}

fn raw_converted(value_type: ValueType, text: &str) -> Option<Value> {
    let text = text.trim();
    let whole = |low: f64, high: f64| text.parse::<i64>().ok().filter(|v| (low..=high).contains(&(*v as f64))).map(Value::from);
    match value_type {
        ValueType::Uint8 => whole(0.0, f64::from(u8::MAX)),
        ValueType::Int8 => whole(f64::from(i8::MIN), f64::from(i8::MAX)),
        ValueType::Uint16 => whole(0.0, f64::from(u16::MAX)),
        ValueType::Int16 => whole(f64::from(i16::MIN), f64::from(i16::MAX)),
        ValueType::Uint32 => whole(0.0, f64::from(u32::MAX)),
        ValueType::Int32 => whole(f64::from(i32::MIN), f64::from(i32::MAX)),
        ValueType::Uint64 | ValueType::Int64 => text.parse::<i64>().ok().map(Value::from),
        ValueType::Float => text.parse::<f64>().ok().filter(|v| v.is_finite() && v.abs() <= f64::from(f32::MAX)).map(|v| Value::from(f64::from(v as f32))),
        _ => text.parse::<f64>().ok().filter(|v| v.is_finite()).map(Value::from),
    }
}

fn width(value_type: ValueType) -> u32 {
    match value_type {
        ValueType::Uint8 | ValueType::Int8 => 8,
        ValueType::Uint16 | ValueType::Int16 => 16,
        ValueType::Uint32 | ValueType::Int32 | ValueType::Float => 32,
        _ => 64,
    }
}

fn sorted_pairs(object: &serde_json::Map<String, Value>) -> Vec<(String, String)> {
    let mut keyed: Vec<(f64, String, String)> = object.iter().filter_map(|(key, label)| Some((key.parse::<f64>().ok()?, key.clone(), label.as_str().unwrap_or("").to_string()))).collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
    keyed.into_iter().map(|(_, key, label)| (key, label)).collect()
}

fn json_flag(value: &Value) -> bool {
    value.as_bool().unwrap_or_else(|| value.as_str().is_some_and(|text| text.eq_ignore_ascii_case("true")))
}

pub fn json_metadata(parameters: &JsonParameters, name: &str, value_type: ValueType) -> MetaData {
    let gain = (name.ends_with("_P") || name.ends_with("_I") || name.ends_with("_D")) && matches!(value_type, ValueType::Float | ValueType::Double);
    let bare = MetaData {
        bits: Vec::new(),
        name: name.to_string(),
        value_type,
        label: String::new(),
        short_description: String::new(),
        long_description: String::new(),
        units: None,
        decimal_places: gain.then_some(6),
        default: None,
        min: None,
        max: None,
        increment: None,
        max_string_length: None,
        user_min: None,
        user_max: None,
        enums: Vec::new(),
        bitmask: false,
        has_control: true,
        qgc_reboot_required: false,
        vehicle_reboot_required: false,
        volatile_value: false,
        read_only: false,
        group: Some(group_from_name(name)),
        category: Some("Advanced".to_string()),
    };
    let Some((group, fields)) = parameters.by_name.get(name) else { return bare };
    let text = |key: &str| fields.get(key).and_then(Value::as_str).filter(|t| !t.is_empty()).map(str::to_string);
    let range = fields.get("Range").and_then(Value::as_object);
    let bound = |key: &str| range.and_then(|r| r.get(key)).and_then(Value::as_str).and_then(|t| raw_converted(value_type, t));
    let enums = fields
        .get("Values")
        .and_then(Value::as_object)
        .map(sorted_pairs)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(code, label)| {
            let signed = match (value_type, code.parse::<i64>()) {
                (ValueType::Int8, Ok(unsigned)) if (128..=255).contains(&unsigned) => (unsigned - 256).to_string(),
                _ => code,
            };
            raw_converted(value_type, &signed).map(|value| EnumEntry { label, value })
        })
        .collect();
    let bits = fields
        .get("Bitmask")
        .and_then(Value::as_object)
        .map(sorted_pairs)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(bit, label)| {
            let bit = bit.trim().parse::<u32>().ok().filter(|b| *b < 64)?;
            let set = 1u64 << bit;
            let wrapped = match value_type {
                ValueType::Int8 => (set as u8 as i8).to_string(),
                ValueType::Int16 => (set as u16 as i16).to_string(),
                ValueType::Int32 => (set as u32 as i32).to_string(),
                _ => set.to_string(),
            };
            (u32::try_from(set.ilog2()).ok()? < width(value_type)).then_some(())?;
            raw_converted(value_type, &wrapped).map(|value| EnumEntry { label, value })
        })
        .collect();
    MetaData {
        name: name.to_string(),
        label: String::new(),
        short_description: text("DisplayName").unwrap_or_default(),
        long_description: text("Description").unwrap_or_default(),
        units: text("Units"),
        category: Some(text("User").unwrap_or_else(|| "Other".to_string())),
        group: Some(group.clone()),
        read_only: fields.get("ReadOnly").is_some_and(json_flag),
        vehicle_reboot_required: fields.get("RebootRequired").is_some_and(json_flag),
        increment: text("Increment").and_then(|t| t.trim().parse().ok()),
        min: bound("low"),
        max: bound("high"),
        enums,
        bits,
        ..bare
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_metadata_file_is_the_newest_at_or_below_the_firmware_then_the_oldest() {
        assert_eq!(file_for("Copter", 4, 5).map(|(n, _)| n), Some("Copter-4.5"));
        let newest = APM_PARAMETER_FILES.iter().filter(|(n, _)| n.starts_with("Copter-4.")).map(|(n, _)| *n).max_by_key(|n| n["Copter-4.".len()..].parse::<u32>().unwrap_or(0)).unwrap();
        assert_eq!(file_for("Copter", 4, 10).map(|(n, _)| n), Some(newest), "a firmware newer than any file takes the newest one below it");
        let oldest = (0..10).find_map(|minor| APM_PARAMETER_FILES.iter().find(|(n, _)| *n == format!("Copter-4.{minor}"))).map(|(n, _)| *n);
        assert_eq!(file_for("Copter", -1, -1).map(|(n, _)| n), oldest, "an unknown firmware falls back to the oldest 4.x");
    }

    #[test]
    fn a_copter_flight_mode_reads_its_enum_from_the_json_file() {
        let parameters = load("Copter", 4, 5).unwrap();
        let mode = json_metadata(&parameters, "FLTMODE1", ValueType::Int8);
        assert_eq!((mode.short_description.as_str(), mode.enums.first().map(|e| e.label.as_str()), mode.enums.len()), ("Flight Mode 1", Some("Stabilize"), 25));
        let checks = json_metadata(&parameters, "ARMING_CHECK", ValueType::Int32);
        assert!(checks.enums.is_empty() && checks.bits.len() > 10 && checks.bits[0].value == Value::from(1));
        assert_eq!(json_metadata(&parameters, "NOT_A_PARAM", ValueType::Float).category.as_deref(), Some("Advanced"));
    }
}
