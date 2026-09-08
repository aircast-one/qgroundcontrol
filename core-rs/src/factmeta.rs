use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Uint8,
    Int8,
    Uint16,
    Int16,
    Uint32,
    Int32,
    Uint64,
    Int64,
    Float,
    Double,
    String,
    Bool,
    ElapsedSeconds,
    Custom,
}

const TYPE_NAMES: [(&str, ValueType); 14] = [
    ("uint8", ValueType::Uint8),
    ("int8", ValueType::Int8),
    ("uint16", ValueType::Uint16),
    ("int16", ValueType::Int16),
    ("uint32", ValueType::Uint32),
    ("int32", ValueType::Int32),
    ("uint64", ValueType::Uint64),
    ("int64", ValueType::Int64),
    ("float", ValueType::Float),
    ("double", ValueType::Double),
    ("string", ValueType::String),
    ("bool", ValueType::Bool),
    ("elapsedseconds", ValueType::ElapsedSeconds),
    ("custom", ValueType::Custom),
];

pub fn value_type(name: &str) -> Option<ValueType> {
    let lowered = name.to_ascii_lowercase();
    TYPE_NAMES.iter().find(|(n, _)| *n == lowered).map(|(_, t)| *t)
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumEntry {
    pub label: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetaData {
    pub name: String,
    pub value_type: ValueType,
    pub short_description: String,
    pub long_description: String,
    pub units: Option<String>,
    pub decimal_places: Option<i64>,
    pub default: Option<Value>,
    pub min: Option<Value>,
    pub max: Option<Value>,
    pub increment: Option<f64>,
    pub enums: Vec<EnumEntry>,
    pub bitmask: bool,
    pub has_control: bool,
    pub qgc_reboot_required: bool,
    pub vehicle_reboot_required: bool,
    pub volatile_value: bool,
    pub read_only: bool,
    pub group: Option<String>,
    pub category: Option<String>,
}

pub const DEFINES_KEY: &str = "QGC.MetaData.Defines";
pub const FACTS_KEY: &str = "QGC.MetaData.Facts";

pub fn split_translated_list(text: &str) -> Vec<String> {
    text.split([',', '，', '、']).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
}

fn defines(root: &Value) -> BTreeMap<String, String> {
    root.get(DEFINES_KEY)
        .and_then(Value::as_object)
        .map(|o| o.iter().filter_map(|(k, v)| v.as_str().map(|s| (format!("{DEFINES_KEY}.{k}"), s.to_string()))).collect())
        .unwrap_or_default()
}

fn typed(value_type: ValueType, raw: &Value) -> Value {
    match (value_type, raw) {
        (ValueType::Bool, Value::Number(n)) => Value::Bool(n.as_f64().unwrap_or(0.0) != 0.0),
        (ValueType::String, Value::Number(n)) => Value::String(n.to_string()),
        (ValueType::Float | ValueType::Double, Value::String(s)) => s.parse::<f64>().map(Value::from).unwrap_or(Value::Null),
        (ValueType::Uint8 | ValueType::Int8 | ValueType::Uint16 | ValueType::Int16 | ValueType::Uint32 | ValueType::Int32 | ValueType::Uint64 | ValueType::Int64, Value::String(s)) => {
            s.parse::<f64>().ok().map(|n| Value::from(n.round() as i64)).unwrap_or(Value::Null)
        }
        (ValueType::Uint8 | ValueType::Int8 | ValueType::Uint16 | ValueType::Int16 | ValueType::Uint32 | ValueType::Int32 | ValueType::Uint64 | ValueType::Int64, Value::Number(n)) => {
            Value::from(n.as_f64().unwrap_or(0.0).round() as i64)
        }
        (_, other) => other.clone(),
    }
}

fn labelled_array(json: &Map<String, Value>, key: &str, value_key: &str) -> Result<Vec<(String, f64)>, String> {
    json.get(key)
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .map(|entry| {
                    let label = entry.get("description").and_then(Value::as_str).ok_or(format!("{key} entry without description"))?;
                    let value = entry.get(value_key).and_then(Value::as_f64).ok_or(format!("{key} entry without {value_key}"))?;
                    Ok((label.to_string(), value))
                })
                .collect()
        })
        .unwrap_or(Ok(Vec::new()))
}

fn enums(name: &str, json: &Map<String, Value>, defines: &BTreeMap<String, String>, value_type: ValueType) -> Result<(Vec<EnumEntry>, bool), String> {
    let values = labelled_array(json, "values", "value")?;
    if !values.is_empty() {
        return Ok((values.into_iter().map(|(label, v)| EnumEntry { label, value: typed(value_type, &Value::from(v)) }).collect(), false));
    }
    let bitmask = labelled_array(json, "bitmask", "index")?;
    if !bitmask.is_empty() {
        return Ok((bitmask.into_iter().map(|(label, i)| EnumEntry { label, value: Value::from(1i64 << (i as i64)) }).collect(), true));
    }
    let Some(strings) = json.get("enumStrings").and_then(Value::as_str) else { return Ok((Vec::new(), false)) };
    let resolve = |text: &str| defines.get(text).cloned().unwrap_or_else(|| text.to_string());
    let labels = split_translated_list(&resolve(strings));
    let raws = split_translated_list(&resolve(json.get("enumValues").and_then(Value::as_str).unwrap_or("")));
    if labels.len() != raws.len() {
        return Err(format!("Enum strings/values count mismatch - name: '{name}' strings: {} values: {}", labels.len(), raws.len()));
    }
    Ok((labels.into_iter().zip(raws).map(|(label, raw)| EnumEntry { label, value: typed(value_type, &Value::String(raw)) }).collect(), false))
}

pub fn from_object(json: &Map<String, Value>, defines: &BTreeMap<String, String>) -> Result<MetaData, String> {
    let name = json.get("name").and_then(Value::as_str).ok_or("fact without a name")?.to_string();
    let type_name = json.get("type").and_then(Value::as_str).ok_or(format!("fact {name} without a type"))?;
    let value_type = value_type(type_name).ok_or(format!("Unknown type {type_name}"))?;
    let (enums, bitmask) = enums(&name, json, defines, value_type)?;
    let text = |key: &str| json.get(key).and_then(Value::as_str).map(str::to_string);
    let flag = |key: &str, fallback: bool| json.get(key).and_then(Value::as_bool).unwrap_or(fallback);
    let number = |key: &str| json.get(key).map(|v| typed(value_type, v));
    Ok(MetaData {
        default: json.get("default").map(|v| match (v, value_type) {
            (Value::Null, ValueType::Float | ValueType::Double) => Value::Null,
            _ => typed(value_type, v),
        }),
        min: number("min"),
        max: number("max"),
        increment: json.get("increment").and_then(Value::as_f64),
        decimal_places: json.get("decimalPlaces").and_then(Value::as_i64),
        short_description: text("shortDesc").unwrap_or_default(),
        long_description: text("longDesc").unwrap_or_default(),
        units: text("units"),
        has_control: flag("control", true),
        qgc_reboot_required: flag("qgcRebootRequired", false),
        vehicle_reboot_required: flag("rebootRequired", false),
        volatile_value: flag("volatile", false),
        read_only: false,
        group: text("group"),
        category: text("category"),
        name,
        value_type,
        enums,
        bitmask,
    })
}

pub fn from_file(text: &str) -> Result<BTreeMap<String, MetaData>, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let defines = defines(&root);
    let facts = root.get(FACTS_KEY).and_then(Value::as_array).ok_or(format!("no {FACTS_KEY} array"))?;
    facts.iter().filter_map(Value::as_object).try_fold(BTreeMap::new(), |mut map, object| {
        let meta = from_object(object, &defines)?;
        match map.contains_key(&meta.name) {
            true => Err(format!("Duplicate fact name: {}", meta.name)),
            false => {
                map.insert(meta.name.clone(), meta);
                Ok(map)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> Vec<std::path::PathBuf> {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                if path.is_dir() {
                    walk(&path, out);
                } else if name.ends_with(".SettingsGroup.json") || name.ends_with(".FactMetaData.json") {
                    out.push(path);
                }
            }
        }
        let mut out = Vec::new();
        walk(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../src")), &mut out);
        out
    }

    #[test]
    fn every_bundled_metadata_file_loads_with_unique_names() {
        let files = files();
        assert!(files.len() >= 38, "found {} files", files.len());
        let loaded: Vec<(String, Result<BTreeMap<String, MetaData>, String>)> = files.iter().map(|p| (p.display().to_string(), from_file(&std::fs::read_to_string(p).unwrap()))).collect();
        let failures: Vec<&(String, Result<_, _>)> = loaded.iter().filter(|(_, r)| r.is_err()).collect();
        assert!(failures.is_empty(), "{failures:?}");
        assert!(loaded.iter().all(|(_, r)| !r.as_ref().unwrap().is_empty()));
    }

    #[test]
    fn the_app_settings_carry_typed_enums_defaults_and_bounds() {
        let app = from_file(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/Settings/App.SettingsGroup.json")).unwrap()).unwrap();
        let firmware = &app["offlineEditingFirmwareClass"];
        assert_eq!(firmware.value_type, ValueType::Uint32);
        assert_eq!(firmware.enums.iter().map(|e| e.value.as_i64().unwrap()).collect::<Vec<_>>(), vec![3, 12, 0]);
        assert_eq!(firmware.enums[1].label, "PX4 Pro");
        assert_eq!(firmware.default, Some(Value::from(12)));
        let cruise = &app["offlineEditingCruiseSpeed"];
        assert_eq!((cruise.value_type, cruise.units.as_deref(), cruise.decimal_places), (ValueType::Double, Some("m/s"), Some(2)));
        assert_eq!((cruise.min.clone(), cruise.default.clone()), (Some(Value::from(1.0)), Some(Value::from(15.0))));
        assert!(cruise.has_control && !cruise.qgc_reboot_required);
    }

    #[test]
    fn enum_values_come_from_defines_and_mismatches_are_refused() {
        let defines = BTreeMap::from([(format!("{DEFINES_KEY}.Modes"), "A, B".to_string()), (format!("{DEFINES_KEY}.ModeValues"), "1，2".to_string())]);
        let object: Map<String, Value> = serde_json::from_str(&format!(r#"{{"name":"m","type":"Uint8","enumStrings":"{DEFINES_KEY}.Modes","enumValues":"{DEFINES_KEY}.ModeValues","default":"2"}}"#)).unwrap();
        let meta = from_object(&object, &defines).unwrap();
        assert_eq!(meta.enums, vec![EnumEntry { label: "A".into(), value: Value::from(1) }, EnumEntry { label: "B".into(), value: Value::from(2) }]);
        assert_eq!(meta.default, Some(Value::from(2)));
        let broken: Map<String, Value> = serde_json::from_str(r#"{"name":"m","type":"uint8","enumStrings":"A,B","enumValues":"1"}"#).unwrap();
        assert!(from_object(&broken, &defines).unwrap_err().contains("count mismatch"));
        let unknown: Map<String, Value> = serde_json::from_str(r#"{"name":"m","type":"quad"}"#).unwrap();
        assert_eq!(from_object(&unknown, &defines).unwrap_err(), "Unknown type quad");
        let bits: Map<String, Value> = serde_json::from_str(r#"{"name":"b","type":"uint32","bitmask":[{"index":0,"description":"one"},{"index":3,"description":"eight"}]}"#).unwrap();
        let meta = from_object(&bits, &defines).unwrap();
        assert!(meta.bitmask);
        assert_eq!(meta.enums.iter().map(|e| e.value.as_i64().unwrap()).collect::<Vec<_>>(), vec![1, 8]);
    }
}
