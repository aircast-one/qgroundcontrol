use std::collections::BTreeMap;
use std::sync::Mutex;

use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "settings.viewer3DSettings.enabled.rawValue",
    "settings.viewer3DSettings.osmFilePath.rawValue",
    "settings.viewer3DSettings.buildingLevelHeight.rawValue",
    "settings.viewer3DSettings.altitudeBias.rawValue",
];

const SINGLE_STOREY: [&str; 4] = ["bungalow", "shed", "kiosk", "cabin"];
const DOUBLE_STOREY_LEISURE: [&str; 3] = ["stadium", "sports_hall", "sauna"];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Building {
    pub outer: Vec<(f64, f64)>,
    pub inner: Vec<(f64, f64)>,
    pub levels: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CityMap {
    pub buildings: Vec<Building>,
    pub south_west: (f64, f64),
    pub north_east: (f64, f64),
}

impl Building {
    pub fn extruded_height(&self, level_height: f64) -> Option<f64> {
        match (self.height > 0.0, self.levels > 0.0) {
            (true, _) => Some(self.height),
            (false, true) => Some(self.levels * level_height),
            (false, false) => None,
        }
    }
}

fn number(text: &str) -> f64 {
    text.trim().split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).next().and_then(|n| n.parse().ok()).unwrap_or(0.0)
}

fn way_building(way: roxmltree::Node, nodes: &BTreeMap<i64, (f64, f64)>) -> Building {
    let outer: Vec<(f64, f64)> = way.children().filter(|c| c.has_tag_name("nd")).filter_map(|nd| nd.attribute("ref")?.parse::<i64>().ok()).filter(|id| *id > 0).filter_map(|id| nodes.get(&id).copied()).collect();
    let tagged = way.children().filter(|c| c.has_tag_name("tag")).filter_map(|t| Some((t.attribute("k")?, t.attribute("v")?)));
    let (levels, height) = tagged.fold((0.0, 0.0), |(levels, height), (key, value)| match key {
        "building:levels" => (number(value), height),
        "height" => (levels, number(value)),
        "building" if levels == 0.0 && height == 0.0 => (if SINGLE_STOREY.contains(&value) { 1.0 } else { 2.0 }, height),
        "leisure" if levels == 0.0 && height == 0.0 && DOUBLE_STOREY_LEISURE.contains(&value) => (2.0, height),
        _ => (levels, height),
    });
    Building { outer, inner: Vec::new(), levels, height }
}

pub fn parse(text: &str) -> Option<CityMap> {
    let document = roxmltree::Document::parse(text).ok()?;
    let root = document.root_element();
    let nodes: BTreeMap<i64, (f64, f64)> = root
        .children()
        .filter(|c| c.has_tag_name("node"))
        .filter_map(|n| Some((n.attribute("id")?.parse::<i64>().ok().filter(|id| *id > 0)?, (n.attribute("lat")?.parse().ok()?, n.attribute("lon")?.parse().ok()?))))
        .collect();
    let ways: BTreeMap<i64, Building> = root
        .children()
        .filter(|c| c.has_tag_name("way"))
        .filter_map(|w| Some((w.attribute("id")?.parse::<i64>().ok().filter(|id| *id != 0)?, way_building(w, &nodes))))
        .filter(|(_, b)| b.outer.len() > 2)
        .collect();
    let merged = root.children().filter(|c| c.has_tag_name("relation")).fold(ways, |ways, relation| {
        let tags: Vec<(&str, &str)> = relation.children().filter(|c| c.has_tag_name("tag")).filter_map(|t| Some((t.attribute("k")?, t.attribute("v")?))).collect();
        let multipolygon = tags.iter().any(|(k, v)| *k == "type" && *v == "multipolygon");
        let is_building = tags.iter().any(|(k, _)| *k == "building");
        let members: Vec<(i64, bool)> = relation
            .children()
            .filter(|c| c.has_tag_name("member") && c.attribute("type") == Some("way"))
            .filter_map(|m| Some((m.attribute("ref")?.parse::<i64>().ok()?, m.attribute("role") == Some("inner"))))
            .filter(|(id, _)| ways.contains_key(id))
            .collect();
        let combined = members.iter().fold(Building::default(), |acc, (id, inner)| {
            let part = &ways[id];
            let (outer, inner_points) = match inner {
                true => (acc.outer, [acc.inner, part.outer.clone()].concat()),
                false => ([acc.outer, part.outer.clone()].concat(), acc.inner),
            };
            Building { outer, inner: inner_points, levels: acc.levels.max(part.levels), height: acc.height.max(part.height) }
        });
        let combined = match is_building && combined.height == 0.0 && combined.levels == 0.0 {
            true => Building { levels: 2.0, ..combined },
            false => combined,
        };
        match (multipolygon, members.first()) {
            (true, Some((first, _))) => ways.into_iter().filter(|(id, _)| !members.iter().any(|(m, _)| m == id)).chain(std::iter::once((*first, combined))).collect(),
            _ => ways,
        }
    });
    let bounds = root.children().find(|c| c.has_tag_name("bounds")).and_then(|b| {
        let at = |key: &str| b.attribute(key)?.parse::<f64>().ok();
        Some(((at("minlat")?, at("minlon")?), (at("maxlat")?, at("maxlon")?)))
    });
    let (south_west, north_east) = bounds.or_else(|| {
        let all: Vec<&(f64, f64)> = nodes.values().collect();
        (!all.is_empty()).then(|| {
            let lat = all.iter().map(|p| p.0);
            let lon = all.iter().map(|p| p.1);
            ((lat.clone().fold(f64::MAX, f64::min), lon.clone().fold(f64::MAX, f64::min)), (lat.fold(f64::MIN, f64::max), lon.fold(f64::MIN, f64::max)))
        })
    })?;
    Some(CityMap { buildings: merged.into_values().collect(), south_west, north_east })
}

static LOADED: Mutex<Option<(String, std::time::SystemTime, Option<CityMap>)>> = Mutex::new(None);

fn loaded(path: &str) -> Option<CityMap> {
    let lower = path.to_lowercase();
    if !(lower.ends_with(".osm") || lower.ends_with(".xml")) {
        return None;
    }
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
    let mut held = LOADED.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    match held.as_ref() {
        Some((known, at, map)) if known == path && *at == modified => map.clone(),
        _ => {
            let map = std::fs::read_to_string(path).ok().and_then(|text| parse(&text));
            *held = Some((path.to_string(), modified, map.clone()));
            map
        }
    }
}

fn setting(backend: &dyn Backend, name: &str) -> Value {
    crate::read::object(&backend.get(&format!("settings.viewer3DSettings.{name}.rawValue"))).get("value").cloned().unwrap_or(Value::Null)
}

fn ring(points: &[(f64, f64)]) -> Vec<[f64; 2]> {
    points.iter().map(|(lat, lon)| [*lon, *lat]).collect()
}

pub fn viewer3d_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let enabled = setting(backend, "enabled").as_bool().unwrap_or(false);
    let path = setting(backend, "osmFilePath").as_str().unwrap_or_default().to_string();
    let level_height = setting(backend, "buildingLevelHeight").as_f64().unwrap_or(3.0);
    let bias = setting(backend, "altitudeBias").as_f64().unwrap_or(0.0);
    let map = (enabled && !path.is_empty()).then(|| loaded(&path)).flatten();
    let reason = match (enabled, path.is_empty(), map.is_some()) {
        (false, _, _) => Some("Turn on the 3D view in Settings."),
        (true, true, _) => Some("Choose an OpenStreetMap file in Settings."),
        (true, false, false) => Some("That OpenStreetMap file could not be read."),
        (true, false, true) => None,
    };
    json!({
        "kind": "object",
        "class": "Viewer3D",
        "enabled": enabled,
        "available": map.is_some(),
        "reason": reason,
        "altitudeBias": bias,
        "bounds": map.as_ref().map(|m| json!({ "south": m.south_west.0, "west": m.south_west.1, "north": m.north_east.0, "east": m.north_east.1 })),
        "buildings": map.as_ref().map(|m| m.buildings.iter().filter_map(|b| b.extruded_height(level_height).map(|height| json!({ "outer": ring(&b.outer), "inner": ring(&b.inner), "height": height }))).collect::<Vec<_>>()).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CITY: &str = r#"<?xml version="1.0"?>
<osm version="0.6">
  <bounds minlat="47.0" minlon="8.0" maxlat="47.1" maxlon="8.1"/>
  <node id="1" lat="47.01" lon="8.01"/><node id="2" lat="47.01" lon="8.02"/><node id="3" lat="47.02" lon="8.02"/><node id="4" lat="47.02" lon="8.01"/>
  <node id="5" lat="47.03" lon="8.03"/><node id="6" lat="47.03" lon="8.04"/><node id="7" lat="47.04" lon="8.04"/>
  <way id="10"><nd ref="1"/><nd ref="2"/><nd ref="3"/><nd ref="4"/><nd ref="1"/><tag k="building" v="shed"/></way>
  <way id="11"><nd ref="5"/><nd ref="6"/><nd ref="7"/><nd ref="5"/><tag k="building" v="yes"/><tag k="height" v="12 m"/></way>
  <way id="12"><nd ref="1"/><nd ref="2"/><nd ref="3"/><tag k="highway" v="service"/></way>
  <way id="13"><nd ref="5"/><nd ref="6"/><nd ref="7"/><tag k="leisure" v="stadium"/></way>
</osm>"#;

    #[test]
    fn buildings_are_read_with_qgcs_storey_and_height_rules() {
        let map = parse(CITY).unwrap();
        assert_eq!((map.south_west, map.north_east), ((47.0, 8.0), (47.1, 8.1)), "the file's bounds win, as OsmParserThread reads the header first");
        let height_of = |count: usize, level: f64| map.buildings.iter().filter_map(|b| b.extruded_height(level)).filter(|h| (*h - count as f64).abs() < 1e-9).count();
        assert_eq!(height_of(3, 3.0), 1, "a shed is one storey of buildingLevelHeight");
        assert_eq!(height_of(12, 3.0), 1, "an explicit height wins over storeys");
        assert_eq!(height_of(6, 3.0), 1, "a stadium is two storeys");
        assert_eq!(map.buildings.iter().filter(|b| b.extruded_height(3.0).is_none()).count(), 1, "a way that is no building is kept but never extruded, as buildingToMesh skips it");
    }

    #[test]
    fn a_multipolygon_relation_merges_its_ways_into_one_building() {
        let text = CITY.replace("</osm>", r#"<relation id="20"><member type="way" ref="10" role="outer"/><member type="way" ref="12" role="inner"/><tag k="type" v="multipolygon"/><tag k="building" v="yes"/></relation></osm>"#);
        let map = parse(&text).unwrap();
        let merged = map.buildings.iter().find(|b| !b.inner.is_empty()).unwrap();
        assert_eq!((merged.outer.len(), merged.inner.len(), merged.levels), (5, 3, 1.0), "inner members become holes and the parts' storeys are kept");
        assert_eq!(map.buildings.len(), 3, "the member ways are replaced by the one merged building");
    }

    #[test]
    fn a_file_without_bounds_takes_them_from_its_nodes() {
        let map = parse(&CITY.replace(r#"<bounds minlat="47.0" minlon="8.0" maxlat="47.1" maxlon="8.1"/>"#, "")).unwrap();
        assert_eq!((map.south_west, map.north_east), ((47.01, 8.01), (47.04, 8.04)));
    }
}
