use serde_json::{Map, Value};
use std::collections::BTreeMap;

use crate::factmeta::{MetaData, ValueType};

pub const BUNDLED: &str = include_str!("../../src/FirmwarePlugin/PX4/PX4ParameterFactMetaData.json");

pub fn bundled() -> &'static BTreeMap<String, MetaData> {
    static PARSED: std::sync::LazyLock<BTreeMap<String, MetaData>> = std::sync::LazyLock::new(|| parse(BUNDLED).unwrap_or_default());
    &PARSED
}

pub fn bare(value_type: ValueType) -> MetaData {
    MetaData {
        bits: Vec::new(),
        name: String::new(),
        value_type,
        label: String::new(),
        short_description: String::new(),
        long_description: String::new(),
        units: None,
        decimal_places: None,
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
        group: None,
        category: None,
    }
}

pub fn post_processed(meta: MetaData) -> MetaData {
    MetaData {
        category: Some(meta.category.clone().filter(|c| !c.is_empty() && c != "Other").unwrap_or_else(|| "Standard".to_string())),
        read_only: meta.read_only || meta.volatile_value,
        short_description: meta.short_description.replace('\n', " "),
        long_description: meta.long_description.replace('\n', " "),
        ..meta
    }
}

pub fn parse(text: &str) -> Result<BTreeMap<String, MetaData>, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    if root.get("version").and_then(Value::as_i64).unwrap_or(0) < 1 {
        return Err("Parameter JSON version too old".to_string());
    }
    let parameters = root.get("parameters").and_then(Value::as_array).ok_or("no parameters array")?;
    let defines = BTreeMap::new();
    Ok(parameters
        .iter()
        .filter_map(Value::as_object)
        .filter(|object: &&Map<String, Value>| object.get("name").and_then(Value::as_str).is_some_and(|name| !name.is_empty()))
        .filter_map(|object| crate::factmeta::from_object(object, &defines).ok())
        .map(|meta| (meta.name.clone(), post_processed(meta)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_px4_metadata_is_the_json_px4_firmware_plugin_loads() {
        let all = parse(BUNDLED).unwrap();
        assert!(all.len() > 1800, "{}", all.len());
        let loss = &all["COM_RC_LOSS_T"];
        assert_eq!((loss.value_type, loss.units.as_deref(), loss.decimal_places), (ValueType::Float, Some("s"), Some(1)));
        assert_eq!(loss.category.as_deref(), Some("Standard"));
        assert!(loss.group.as_deref().is_some_and(|g| !g.is_empty()));
        assert_eq!(all["FW_LND_ANG"].max.as_ref().and_then(Value::as_f64), Some(45.0), "the XML of Dec 2024 still capped it at 15");
        assert!(all.contains_key("COM_ARM_TRAFF"));
        assert!(all.values().any(|m| m.bitmask));
        assert!(all.values().filter(|m| m.volatile_value).all(|m| m.read_only), "_postProcessMetaData makes volatile parameters read-only");
        assert!(all.values().all(|m| !m.short_description.contains('\n')));
    }

    #[test]
    fn an_old_or_broken_file_is_refused() {
        assert!(parse(r#"{"version":0,"parameters":[]}"#).is_err());
        assert!(parse("nope").is_err());
        let twice = parse(r#"{"version":1,"parameters":[{"name":"X","type":"Int32","shortDesc":"first"},{"name":"X","type":"Int32","shortDesc":"second"}]}"#).unwrap();
        assert_eq!(twice["X"].short_description, "second", "PX4ParameterMetaData replaces a duplicate with the later entry");
    }
}
