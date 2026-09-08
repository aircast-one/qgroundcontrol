use roxmltree::{Document, Node};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::factmeta::{EnumEntry, MetaData, ValueType};

pub const DEFAULT_GROUP: &str = "Misc";
pub const LIBRARIES: &str = "libraries";
const VEHICLE_CATEGORIES: [&str; 6] = ["ArduCopter", "ArduPlane", "APMrover2", "Rover", "ArduSub", "AntennaTracker"];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Raw {
    pub name: String,
    pub category: String,
    pub group: String,
    pub short_description: String,
    pub long_description: String,
    pub units: String,
    pub min: String,
    pub max: String,
    pub increment: String,
    pub read_only: bool,
    pub reboot_required: bool,
    pub values: Vec<(String, String)>,
    pub bitmask: Vec<(String, String)>,
}

pub type Definitions = BTreeMap<String, BTreeMap<String, Raw>>;

pub fn group_from_name(name: &str) -> String {
    name.split('_').next().unwrap_or("").trim_end_matches(|c: char| c.is_ascii_digit()).to_string()
}

pub fn vehicle_category(mav_type: u8) -> &'static str {
    match mav_type {
        1 | 19 | 20 | 21 | 22 | 23 | 24 | 25 => "ArduPlane",
        2 | 3 | 4 | 13 | 14 | 15 => "ArduCopter",
        5 => "Antenna Tracker",
        10 | 11 => "Rover",
        12 => "ArduSub",
        _ => "",
    }
}

fn range(text: &str) -> Option<(String, String)> {
    let trimmed = text.trim();
    let halves = [" ", "to", "-"].iter().map(|sep| trimmed.split(sep).collect::<Vec<_>>()).find(|parts| parts.len() == 2)?;
    let first_word = |s: &str| s.trim().split(' ').next().unwrap_or("").to_string();
    Some((first_word(halves[0]), first_word(halves[1])))
}

fn pairs(text: &str) -> Vec<(String, String)> {
    let parsed: Option<Vec<(String, String)>> = text.split(',').map(|item| item.split_once(':').map(|(a, b)| (a.to_string(), b.to_string()))).collect();
    parsed.unwrap_or_default()
}

fn raw(node: Node) -> Option<Raw> {
    let full = node.attribute("name")?;
    let name = full.rsplit(':').next().unwrap_or(full).to_string();
    let field = |wanted: &str| node.children().find(|c| c.has_tag_name("field") && c.attribute("name") == Some(wanted)).and_then(|c| c.text()).map(str::to_string);
    let (min, max) = field("Range").and_then(|r| range(&r)).unwrap_or_default();
    let truthy = |value: Option<String>| value.is_some_and(|v| v.trim().eq_ignore_ascii_case("true"));
    Some(Raw {
        group: group_from_name(&name),
        name,
        category: node.attribute("user").unwrap_or("").to_string(),
        short_description: node.attribute("humanName").unwrap_or("").to_string(),
        long_description: node.attribute("documentation").unwrap_or("").to_string(),
        units: field("Units").unwrap_or_default(),
        min,
        max,
        increment: field("Increment").unwrap_or_default(),
        read_only: truthy(field("ReadOnly")),
        reboot_required: truthy(field("RebootRequired")),
        values: node
            .children()
            .filter(|c| c.has_tag_name("values"))
            .flat_map(|values| values.children().filter(|v| v.has_tag_name("value")).filter_map(|v| Some((v.attribute("code")?.to_string(), v.text().unwrap_or("").to_string()))).collect::<Vec<_>>())
            .collect(),
        bitmask: field("Bitmask").map(|b| pairs(&b)).unwrap_or_default(),
    })
}

fn demote_single_member_groups(mut params: BTreeMap<String, Raw>) -> BTreeMap<String, Raw> {
    let members = params.values().fold(BTreeMap::<String, usize>::new(), |mut m, r| {
        *m.entry(r.group.clone()).or_default() += 1;
        m
    });
    params.values_mut().filter(|r| members.get(&r.group) == Some(&1)).for_each(|r| r.group = DEFAULT_GROUP.to_string());
    params
}

pub fn parse(xml: &str) -> Result<Definitions, String> {
    let document = Document::parse(xml).map_err(|e| format!("Badly formed XML: {e}"))?;
    let root = document.root_element();
    if !root.has_tag_name("paramfile") {
        return Err("Badly formed XML: no paramfile root".to_string());
    }
    let blocks = root.children().filter(|c| c.has_tag_name("vehicles") || c.has_tag_name("libraries")).flat_map(|section| {
        let in_libraries = section.has_tag_name("libraries");
        section.children().filter(|p| p.has_tag_name("parameters")).filter_map(move |parameters| {
            let name = parameters.attribute("name").unwrap_or("");
            let category = match (VEHICLE_CATEGORIES.iter().any(|c| name.contains(c)), in_libraries) {
                (true, _) => name.to_string(),
                (false, true) => LIBRARIES.to_string(),
                (false, false) => return None,
            };
            Some((category, parameters))
        })
    });
    Ok(blocks.fold(Definitions::new(), |mut all, (category, parameters)| {
        let block = parameters.children().filter(|p| p.has_tag_name("param")).filter_map(raw).fold(BTreeMap::new(), |mut m, r| {
            m.insert(r.name.clone(), r);
            m
        });
        all.entry(category).or_default().extend(demote_single_member_groups(block));
        all
    }))
}

fn typed(value_type: ValueType, text: &str) -> Option<Value> {
    let parsed: f64 = text.trim().parse().ok()?;
    Some(match value_type {
        ValueType::Float | ValueType::Double => Value::from(parsed),
        _ => Value::from(parsed.round() as i64),
    })
}

fn bit_value(value_type: ValueType, bit: u32) -> Option<Value> {
    let set = 1u32.checked_shl(bit)?;
    Some(match value_type {
        ValueType::Int8 => Value::from(set as i8),
        ValueType::Int16 => Value::from(set as i16),
        ValueType::Int32 | ValueType::Int64 => Value::from(set as i32),
        ValueType::Uint8 | ValueType::Uint16 | ValueType::Uint32 | ValueType::Uint64 => Value::from(set),
        _ => return None,
    })
}

pub fn metadata(definitions: &Definitions, mav_type: u8, name: &str, value_type: ValueType) -> MetaData {
    let category = vehicle_category(mav_type);
    let categories = if category == "Rover" { vec!["Rover", "APMrover2"] } else { vec![category] };
    let raw = categories.iter().find_map(|c| definitions.get(*c).and_then(|m| m.get(name)).or_else(|| definitions.get(LIBRARIES).and_then(|m| m.get(name))));
    let mut meta = MetaData {
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
        group: Some(group_from_name(name)),
        category: Some("Advanced".to_string()),
    };
    let Some(raw) = raw else { return meta };
    let enums: Vec<EnumEntry> = raw.values.iter().map(|(code, label)| typed(value_type, code).map(|value| EnumEntry { label: label.clone(), value })).collect::<Option<_>>().unwrap_or_default();
    let bits: Vec<EnumEntry> = raw
        .bitmask
        .iter()
        .map(|(bit, label)| bit.trim().parse::<u32>().ok().and_then(|b| bit_value(value_type, b)).map(|value| EnumEntry { label: label.clone(), value }))
        .collect::<Option<_>>()
        .unwrap_or_default();
    let gain = (name.ends_with("_P") || name.ends_with("_I") || name.ends_with("_D")) && matches!(value_type, ValueType::Float | ValueType::Double);
    meta.name = raw.name.clone();
    meta.category = Some(if raw.category.is_empty() { "Other".to_string() } else { raw.category.clone() });
    meta.group = Some(raw.group.clone());
    meta.vehicle_reboot_required = raw.reboot_required;
    meta.read_only = raw.read_only;
    meta.short_description = raw.short_description.clone();
    meta.long_description = raw.long_description.clone();
    meta.units = (!raw.units.is_empty()).then(|| raw.units.clone());
    meta.min = typed(value_type, &raw.min);
    meta.max = typed(value_type, &raw.max);
    meta.bitmask = !bits.is_empty();
    meta.enums = if enums.is_empty() { bits } else { enums };
    meta.increment = raw.increment.trim().parse().ok();
    meta.decimal_places = gain.then_some(6);
    meta
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copter() -> Definitions {
        parse(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/FirmwarePlugin/APM/ArduPilot-Parameter-Repository/Copter-4.6/apm.pdef.xml")).unwrap()).unwrap()
    }

    #[test]
    fn the_copter_definitions_resolve_units_ranges_enums_and_bitmasks() {
        let defs = copter();
        assert!(defs["ArduCopter"].len() > 50 && defs[LIBRARIES].len() > 1000);
        let rtl = metadata(&defs, 2, "RTL_ALT", ValueType::Float);
        assert_eq!((rtl.units.as_deref(), rtl.min.clone(), rtl.max.clone(), rtl.increment), (Some("cm"), Some(Value::from(30.0)), Some(Value::from(300000.0)), Some(1.0)));
        assert_eq!((rtl.category.as_deref(), rtl.group.as_deref(), rtl.short_description.as_str()), (Some("Standard"), Some("RTL"), "RTL Altitude"));
        let cone = metadata(&defs, 2, "RTL_CONE_SLOPE", ValueType::Float);
        assert_eq!(cone.enums.iter().map(|e| e.value.as_f64().unwrap()).collect::<Vec<_>>(), vec![0.0, 1.0, 3.0]);
        let masked = defs.values().flat_map(|m| m.values()).find(|r| r.bitmask.len() >= 3).unwrap().clone();
        let meta = metadata(&defs, 2, &masked.name, ValueType::Int32);
        assert!(meta.bitmask && meta.enums.iter().all(|e| e.value.as_i64().unwrap().count_ones() == 1));
        let gain = metadata(&defs, 2, "ATC_RAT_RLL_P", ValueType::Float);
        assert_eq!(gain.decimal_places, Some(6));
        let unknown = metadata(&defs, 2, "NOPE_X1", ValueType::Int32);
        assert_eq!((unknown.category.as_deref(), unknown.group.as_deref(), unknown.name.as_str()), (Some("Advanced"), Some("NOPE"), ""));
    }

    #[test]
    fn rover_falls_back_to_the_old_name_and_single_member_groups_are_misc() {
        let xml = r#"<paramfile><vehicles><parameters name="APMrover2"><param name="APMrover2:CRUISE_SPEED" humanName="Cruise" user="Standard"><field name="Range">0 to 100</field></param><param name="APMrover2:CRUISE_THROTTLE"/><param name="APMrover2:LONE_X" humanName="Lone"/></parameters><parameters name="Other"><param name="Other:SKIP"/></parameters></vehicles><libraries><parameters name="Lib"><param name="Lib:BATT_MONITOR" humanName="Monitor"><values><value code="0">Off</value><value code="4">Analog</value></values></param><param name="Lib:BATT2_MONITOR"/></parameters></libraries></paramfile>"#;
        let defs = parse(xml).unwrap();
        assert!(!defs.contains_key("Other"));
        let cruise = metadata(&defs, 10, "CRUISE_SPEED", ValueType::Float);
        assert_eq!((cruise.min.clone(), cruise.max.clone(), cruise.group.as_deref()), (Some(Value::from(0.0)), Some(Value::from(100.0)), Some("CRUISE")));
        assert_eq!(metadata(&defs, 10, "LONE_X", ValueType::Int8).group.as_deref(), Some(DEFAULT_GROUP));
        let monitor = metadata(&defs, 10, "BATT_MONITOR", ValueType::Int8);
        assert_eq!(monitor.enums.iter().map(|e| e.value.as_i64().unwrap()).collect::<Vec<_>>(), vec![0, 4]);
        assert_eq!(monitor.group.as_deref(), Some("BATT"));
        assert_eq!(group_from_name("BATT2_MONITOR"), "BATT");
        assert!(parse("<nope/>").is_err());
    }
}
