use serde_json::{Map, Value};
use std::collections::BTreeMap;

use crate::factmeta::{self, MetaData, ValueType};

pub const INDEXED_NAME_TAG: &str = "{n}";
const PARAMETERS_KEY: &str = "parameters";
pub const AUTOPILOT_COMPONENT: u8 = 1;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComponentParameters {
    pub named: BTreeMap<String, MetaData>,
    pub indexed: Vec<MetaData>,
}

pub fn parse(text: &str) -> Result<ComponentParameters, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    if root.get("version").and_then(Value::as_i64) != Some(1) {
        return Err("component parameter metadata version is not 1".to_string());
    }
    let list = root.get(PARAMETERS_KEY).and_then(Value::as_array).ok_or(format!("no {PARAMETERS_KEY} array"))?;
    let defines = BTreeMap::new();
    let entries: Vec<MetaData> = list.iter().map(|entry| entry.as_object().ok_or("parameter entry is not an object".to_string()).and_then(|o: &Map<String, Value>| factmeta::from_object(o, &defines))).collect::<Result<_, _>>()?;
    let (indexed, named): (Vec<MetaData>, Vec<MetaData>) = entries.into_iter().partition(|m| m.name.contains(INDEXED_NAME_TAG));
    Ok(ComponentParameters { named: named.into_iter().map(|m| (m.name.clone(), m)).collect(), indexed })
}

fn indexed_match<'a>(template: &'a str, name: &str) -> Option<String> {
    let (prefix, suffix) = template.split_once(INDEXED_NAME_TAG)?;
    let digits = name.strip_prefix(prefix)?.strip_suffix(suffix)?;
    (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())).then(|| digits.to_string())
}

fn bare(name: &str, value_type: ValueType, component: u8) -> MetaData {
    MetaData {
        name: name.to_string(),
        value_type,
        short_description: String::new(),
        long_description: String::new(),
        units: None,
        decimal_places: None,
        default: None,
        min: None,
        max: None,
        increment: None,
        enums: Vec::new(),
        bitmask: false,
        has_control: true,
        qgc_reboot_required: false,
        vehicle_reboot_required: false,
        volatile_value: false,
        read_only: false,
        group: name.find('_').filter(|i| *i > 0).map(|i| name[..i].to_string()),
        category: (component != AUTOPILOT_COMPONENT).then(|| format!("Component {component}")),
    }
}

impl ComponentParameters {
    pub fn metadata_for(&self, name: &str, value_type: ValueType, component: u8) -> MetaData {
        if let Some(found) = self.named.get(name) {
            return found.clone();
        }
        let indexed = self.indexed.iter().find_map(|template| {
            let index = indexed_match(&template.name, name)?;
            Some(MetaData {
                name: name.to_string(),
                short_description: template.short_description.replace(INDEXED_NAME_TAG, &index),
                long_description: template.short_description.replace(INDEXED_NAME_TAG, &index),
                ..template.clone()
            })
        });
        indexed.unwrap_or_else(|| bare(name, value_type, component))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"version":1,"parameters":[
        {"name":"GIMBAL_ROLL","type":"float","shortDesc":"Gimbal roll","units":"deg","min":-90,"max":90},
        {"name":"CAM_{n}_MODE","type":"uint8","shortDesc":"Camera {n} mode","enumStrings":"Off,On","enumValues":"0,1"}
    ]}"#;

    #[test]
    fn named_indexed_and_unknown_parameters_resolve_like_the_component_info() {
        let parsed = parse(SAMPLE).unwrap();
        assert_eq!((parsed.named.len(), parsed.indexed.len()), (1, 1));
        let roll = parsed.metadata_for("GIMBAL_ROLL", ValueType::Float, AUTOPILOT_COMPONENT);
        assert_eq!((roll.units.as_deref(), roll.min.clone()), (Some("deg"), Some(Value::from(-90.0))));
        let cam = parsed.metadata_for("CAM_3_MODE", ValueType::Uint8, 100);
        assert_eq!((cam.name.as_str(), cam.short_description.as_str(), cam.long_description.as_str()), ("CAM_3_MODE", "Camera 3 mode", "Camera 3 mode"));
        assert_eq!(cam.enums.len(), 2);
        let unknown = parsed.metadata_for("BATT_CAPACITY", ValueType::Int32, 100);
        assert_eq!((unknown.group.as_deref(), unknown.category.as_deref(), unknown.value_type), (Some("BATT"), Some("Component 100"), ValueType::Int32));
        assert_eq!(parsed.metadata_for("X", ValueType::Int32, AUTOPILOT_COMPONENT).category, None);
        assert_eq!(parsed.metadata_for("CAM_X_MODE", ValueType::Uint8, 1).short_description, "");
    }

    #[test]
    fn a_wrong_version_or_shape_is_refused() {
        assert!(parse(r#"{"version":2,"parameters":[]}"#).unwrap_err().contains("version"));
        assert!(parse(r#"{"version":1}"#).unwrap_err().contains("parameters"));
        assert!(parse(r#"{"version":1,"parameters":[{"name":"A"}]}"#).is_err());
    }
}
