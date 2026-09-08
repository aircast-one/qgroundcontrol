use roxmltree::{Document, Node};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::factmeta::{EnumEntry, MetaData, ValueType, value_type};

pub const BUNDLED: &str = include_str!("../../src/FirmwarePlugin/PX4/PX4ParameterFactMetaData.xml");
const MIN_VERSION: i64 = 3;

fn text(node: Node) -> String {
    node.text().unwrap_or("").replace('\n', " ")
}

fn number(value_type: ValueType, raw: &str) -> Option<Value> {
    let parsed: f64 = raw.trim().parse().ok()?;
    Some(match value_type {
        ValueType::Float | ValueType::Double => Value::from(parsed),
        _ => Value::from(parsed.round() as i64),
    })
}

fn bare(value_type: ValueType) -> MetaData {
    MetaData {
        name: String::new(),
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
        group: None,
        category: None,
    }
}

fn parameter(node: Node, group: &str) -> Result<MetaData, String> {
    let name = node.attribute("name").ok_or("parameter without a name")?;
    let type_name = node.attribute("type").ok_or(format!("parameter {name} without a type"))?;
    let value_type = value_type(type_name).ok_or(format!("Parameter meta data with bad type: {type_name} name: {name}"))?;
    let volatile_value = node.attribute("volatile") == Some("true");
    let read_only = volatile_value || node.attribute("readonly") == Some("true");
    let children: Vec<Node> = node.children().filter(Node::is_element).collect();
    let child = |tag: &str| children.iter().find(|c| c.has_tag_name(tag)).copied();
    let value_entries: Vec<EnumEntry> = child("values")
        .map(|values| values.children().filter(|c| c.has_tag_name("value")).filter_map(|v| number(value_type, v.attribute("code")?).map(|code| EnumEntry { label: text(v), value: code })).collect())
        .unwrap_or_default();
    let bit_entries: Vec<EnumEntry> = child("bitmask")
        .map(|mask| {
            mask.children()
                .filter(|c| c.has_tag_name("bit"))
                .filter_map(|b| b.attribute("index")?.parse::<u8>().ok().filter(|i| *i < 31).map(|i| EnumEntry { label: text(b), value: Value::from(1i64 << i) }))
                .collect()
        })
        .unwrap_or_default();
    let boolean: Vec<EnumEntry> = child("boolean").map(|_| vec![EnumEntry { label: "Enabled".into(), value: Value::from(1) }, EnumEntry { label: "Disabled".into(), value: Value::from(0) }]).unwrap_or_default();
    let bitmask = !bit_entries.is_empty();
    Ok(MetaData {
        name: name.to_string(),
        value_type,
        short_description: child("short_desc").map(text).unwrap_or_default(),
        long_description: child("long_desc").map(text).unwrap_or_default(),
        units: child("unit").map(text),
        decimal_places: child("decimal").and_then(|d| text(d).trim().parse::<u32>().ok()).map(i64::from),
        default: node.attribute("default").filter(|d| !d.is_empty()).and_then(|d| number(value_type, d)),
        min: child("min").and_then(|m| number(value_type, &text(m))),
        max: child("max").and_then(|m| number(value_type, &text(m))),
        increment: child("increment").and_then(|i| text(i).trim().parse().ok()),
        enums: value_entries.into_iter().chain(boolean).chain(bit_entries).collect(),
        bitmask,
        has_control: true,
        qgc_reboot_required: false,
        vehicle_reboot_required: child("reboot_required").is_some_and(|r| text(r).trim().eq_ignore_ascii_case("true")),
        volatile_value,
        read_only,
        group: Some(group.to_string()),
        category: Some(node.attribute("category").filter(|c| !c.is_empty()).unwrap_or("Standard").to_string()),
    })
}

pub fn parse(xml: &str) -> Result<BTreeMap<String, MetaData>, String> {
    let document = Document::parse(xml).map_err(|e| format!("Badly formed XML: {e}"))?;
    let root = document.root_element();
    if !root.has_tag_name("parameters") {
        return Err("Badly formed XML: no parameters root".to_string());
    }
    let version: i64 = root.children().find(|c| c.has_tag_name("version")).and_then(|v| v.text()).and_then(|t| t.trim().parse().ok()).ok_or("Badly formed XML: no version")?;
    if version < MIN_VERSION {
        return Err(format!("Parameter version stamp too old, skipping load. Found: {version} Want: {MIN_VERSION}"));
    }
    root.children()
        .filter(|g| g.has_tag_name("group"))
        .try_fold(BTreeMap::new(), |map, group| {
            let group_name = group.attribute("name").ok_or("Badly formed XML: group without a name")?;
            group.children().filter(|p| p.has_tag_name("parameter")).try_fold(map, |mut map, node| {
                let meta = parameter(node, group_name)?;
                let entry = if map.contains_key(&meta.name) { bare(meta.value_type) } else { meta.clone() };
                map.insert(meta.name.clone(), entry);
                Ok(map)
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_px4_metadata_loads_with_its_units_bounds_and_enums() {
        let all = parse(BUNDLED).unwrap();
        assert!(all.len() > 2000, "{}", all.len());
        let loss = &all["COM_RC_LOSS_T"];
        assert_eq!((loss.value_type, loss.units.as_deref(), loss.decimal_places, loss.increment), (ValueType::Float, Some("s"), Some(1), Some(0.1)));
        assert_eq!((loss.default.clone(), loss.min.clone(), loss.max.clone()), (Some(Value::from(0.5)), Some(Value::from(0.0)), Some(Value::from(35.0))));
        assert_eq!(loss.category.as_deref(), Some("Standard"));
        assert!(loss.group.as_deref().is_some_and(|g| !g.is_empty()));
        let bitmasked = all.values().find(|m| m.bitmask).unwrap();
        assert!(bitmasked.enums.iter().all(|e| e.value.as_i64().is_some_and(|v| v.count_ones() == 1)));
        assert!(all.values().any(|m| m.read_only));
    }

    #[test]
    fn old_or_broken_files_are_refused_and_duplicates_go_bare() {
        assert!(parse("<parameters><version>2</version></parameters>").unwrap_err().contains("too old"));
        assert!(parse("<nope/>").unwrap_err().starts_with("Badly formed XML"));
        assert!(parse("<parameters><version>3</version><group name=\"g\"><parameter name=\"X\" type=\"QUAD\"/></group></parameters>").unwrap_err().contains("bad type"));
        let twice = parse("<parameters><version>3</version><group name=\"g\"><parameter name=\"X\" type=\"INT32\" default=\"1\"><short_desc>first</short_desc></parameter><parameter name=\"X\" type=\"INT32\"/></group></parameters>").unwrap();
        assert_eq!(twice["X"].short_description, "");
        assert_eq!(twice["X"].name, "");
        let boolean = parse("<parameters><version>3</version><group name=\"g\"><parameter name=\"B\" type=\"INT32\"><boolean/></parameter></group></parameters>").unwrap();
        assert_eq!(boolean["B"].enums.iter().map(|e| (e.label.as_str(), e.value.as_i64().unwrap())).collect::<Vec<_>>(), vec![("Enabled", 1), ("Disabled", 0)]);
    }
}
