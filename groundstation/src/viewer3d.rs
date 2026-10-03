use std::collections::BTreeMap;
use std::sync::Mutex;

use serde_json::{Value, json};

use crate::router::Backend;

pub const VIEWER3D_CHANGED: &str = "core.viewer3d@changed";

static PARSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn take_parsed() -> bool {
    PARSED.swap(false, std::sync::atomic::Ordering::SeqCst)
}

pub const DEPS: &[&str] = &[
    VIEWER3D_CHANGED,
    "settings.viewer3DSettings.enabled.rawValue",
    "settings.viewer3DSettings.osmFilePath.rawValue",
    "settings.viewer3DSettings.buildingLevelHeight.rawValue",
];

const SINGLE_STOREY: [&str; 4] = ["bungalow", "shed", "kiosk", "cabin"];
const DOUBLE_STOREY_LEISURE: [&str; 3] = ["stadium", "sports_hall", "sauna"];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Building {
    pub outer: Vec<Vec<(f64, f64)>>,
    pub inner: Vec<Vec<(f64, f64)>>,
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
    text.trim().parse().unwrap_or(0.0)
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
    Building { outer: vec![outer], inner: Vec::new(), levels, height }
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
        .filter(|(_, b)| b.outer.first().is_some_and(|ring| ring.len() > 2))
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
                true => (acc.outer, acc.inner.into_iter().chain(part.outer.clone()).collect()),
                false => (acc.outer.into_iter().chain(part.outer.clone()).collect(), acc.inner),
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
    let extruded: Vec<(f64, f64)> = merged.values().filter(|b| b.levels > 0.0 || b.height > 0.0).flat_map(|b| b.outer.iter().flatten().copied()).collect();
    let bounds = root.children().find(|c| c.has_tag_name("bounds")).and_then(|b| {
        let at = |key: &str| b.attribute(key)?.parse::<f64>().ok();
        Some(((at("minlat")?, at("minlon")?), (at("maxlat")?, at("maxlon")?)))
    }).map(|((south, west), (north, east))| {
        extruded.iter().fold(((south, west), (north, east)), |((s, w), (n, e)), (lat, lon)| ((s.min(*lat), w.min(*lon)), (n.max(*lat), e.max(*lon))))
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

enum Load {
    Parsing,
    Done(Option<std::sync::Arc<CityMap>>),
}

static LOADED: Mutex<Option<(String, std::time::SystemTime, Load)>> = Mutex::new(None);
static RENDERED: Mutex<Option<((String, std::time::SystemTime, u64), Value)>> = Mutex::new(None);

fn locked<T>(held: &'static Mutex<T>) -> std::sync::MutexGuard<'static, T> {
    held.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn accepted(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".osm") || lower.ends_with(".xml")
}

fn loaded(path: &str) -> Option<(std::time::SystemTime, Option<std::sync::Arc<CityMap>>)> {
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
    let mut held = locked(&LOADED);
    match held.as_ref() {
        Some((known, at, Load::Done(map))) if known == path && *at == modified => Some((modified, map.clone())),
        Some((known, at, Load::Parsing)) if known == path && *at == modified => None,
        _ => {
            *held = Some((path.to_string(), modified, Load::Parsing));
            let owned = path.to_string();
            let _ = std::thread::Builder::new().name("osm-parse".into()).spawn(move || {
                let map = std::fs::read_to_string(&owned).ok().and_then(|text| parse(&text)).map(std::sync::Arc::new);
                let mut held = locked(&LOADED);
                if held.as_ref().is_some_and(|(known, at, _)| *known == owned && *at == modified) {
                    *held = Some((owned, modified, Load::Done(map)));
                    PARSED.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            });
            None
        }
    }
}

fn setting(backend: &dyn Backend, name: &str) -> Value {
    crate::read::object(&backend.get(&format!("settings.viewer3DSettings.{name}.rawValue"))).get("value").cloned().unwrap_or(Value::Null)
}

fn ring(points: &[(f64, f64)]) -> Vec<[f64; 2]> {
    points.iter().map(|(lat, lon)| [*lon, *lat]).collect()
}

fn scene(map: &CityMap, level_height: f64) -> Value {
    json!({
        "bounds": { "south": map.south_west.0, "west": map.south_west.1, "north": map.north_east.0, "east": map.north_east.1 },
        "buildings": map.buildings.iter().filter_map(|b| b.extruded_height(level_height).map(|height| json!({
            "outer": b.outer.iter().map(|r| ring(r)).collect::<Vec<_>>(),
            "inner": b.inner.iter().map(|r| ring(r)).collect::<Vec<_>>(),
            "height": height,
        }))).collect::<Vec<_>>(),
    })
}

pub fn viewer3d_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let enabled = setting(backend, "enabled").as_bool().unwrap_or(false);
    let path = setting(backend, "osmFilePath").as_str().unwrap_or_default().to_string();
    let level_height = setting(backend, "buildingLevelHeight").as_f64().unwrap_or(3.0);
    let state = (enabled && accepted(&path)).then(|| loaded(&path));
    let reason = match (enabled, path.is_empty(), &state) {
        (false, _, _) => Some("Turn on the 3D view in Settings."),
        (true, true, _) => Some("Choose an OpenStreetMap file in Settings."),
        (true, false, None) | (true, false, Some(Some((_, None)))) => Some("That OpenStreetMap file could not be read."),
        (true, false, Some(None)) => Some("Loading the OpenStreetMap file..."),
        (true, false, Some(Some((_, Some(_))))) => None,
    };
    let rendered = match &state {
        Some(Some((modified, Some(map)))) => {
            let key = (path.clone(), *modified, level_height.to_bits());
            let mut cache = locked(&RENDERED);
            match cache.as_ref() {
                Some((known, value)) if *known == key => value.clone(),
                _ => {
                    let value = scene(map, level_height);
                    *cache = Some((key, value.clone()));
                    value
                }
            }
        }
        _ => json!({ "bounds": Value::Null, "buildings": [] }),
    };
    json!({
        "kind": "object",
        "class": "Viewer3D",
        "enabled": enabled,
        "available": reason.is_none(),
        "reason": reason,
        "bounds": rendered["bounds"],
        "buildings": rendered["buildings"],
    })
}

pub const PATH_DEPS: &[&str] = &[
    "settings.viewer3DSettings.altitudeBias.rawValue",
    "vehicle.coordinate",
    "vehicle.altitudeRelative",
    crate::coreplan::CHANGED,
    "plan.missionController.visualItems.count",
    "plan.dirty",
];

const WAYPOINT: i64 = 16;
const RETURN_TO_LAUNCH: i64 = 20;
const TAKEOFF: i64 = 22;
const ROI: i64 = 195;
const ROI_DEPRECATED: i64 = 201;

#[derive(Debug, Clone, PartialEq)]
pub struct PathItem {
    pub launch: bool,
    pub takeoff: bool,
    pub command: i64,
    pub specifies_coordinate: bool,
    pub at: (f64, f64),
    pub altitude: f64,
}

fn item_name(item: &PathItem) -> &'static str {
    match () {
        _ if item.launch || item.command == RETURN_TO_LAUNCH => "L",
        _ if item.takeoff => "T",
        _ if item.specifies_coordinate && item.command == WAYPOINT => "W",
        _ if item.specifies_coordinate && item.command == TAKEOFF => "T",
        _ if item.specifies_coordinate && (item.command == ROI || item.command == ROI_DEPRECATED) => "R",
        _ => "",
    }
}

fn marker_colour(name: &str) -> &'static str {
    match name {
        "T" => "green",
        "R" => "red",
        "L" => "orange",
        _ => "black",
    }
}

pub struct Marker {
    pub at: (f64, f64, f64),
    pub name: &'static str,
    pub colour: &'static str,
}

pub struct Segment {
    pub from: (f64, f64, f64),
    pub to: (f64, f64, f64),
    pub rtl: bool,
}

pub fn path(items: &[PathItem], home: Option<(f64, f64)>) -> (Vec<Marker>, Vec<Segment>) {
    let flown: Vec<&PathItem> = items.iter().filter(|i| !i.launch).collect();
    let end = |item: &PathItem, previous: &PathItem| match item.command == RETURN_TO_LAUNCH {
        true => { let (lat, lon) = home.unwrap_or(item.at); (lat, lon, previous.altitude) }
        false => (item.at.0, item.at.1, item.altitude),
    };
    let acceptable = |item: &PathItem| [WAYPOINT, RETURN_TO_LAUNCH, TAKEOFF, ROI, ROI_DEPRECATED].contains(&item.command);
    let (markers, _) = flown.iter().fold((Vec::new(), None::<&PathItem>), |(markers, previous), item| {
        let name = item_name(item);
        let marker = acceptable(item).then(|| match (item.command == RETURN_TO_LAUNCH, previous) {
            (true, None) => None,
            (true, Some(before)) => Some(end(item, before)),
            (false, _) => Some((item.at.0, item.at.1, item.altitude)),
        }).flatten().map(|at| Marker { at, name, colour: marker_colour(name) });
        let next = if name == "L" || name == "W" { Some(*item) } else { previous };
        (markers.into_iter().chain(marker).collect(), next)
    });
    let (segments, _) = flown.iter().fold((Vec::new(), None::<&PathItem>), |(segments, previous), item| match (item.takeoff, previous) {
        (true, _) => (segments, Some(*item)),
        (false, None) => (segments, None),
        (false, Some(before)) if matches!(item_name(item), "L" | "W") => {
            let segment = Segment { from: (before.at.0, before.at.1, before.altitude), to: end(item, before), rtl: item.command == RETURN_TO_LAUNCH };
            (segments.into_iter().chain(std::iter::once(segment)).collect(), Some(*item))
        }
        (false, kept) => (segments, kept),
    });
    (markers, segments)
}

fn path_items(listed: &Value) -> (Vec<PathItem>, Option<(f64, f64)>) {
    let items: Vec<PathItem> = listed["items"].as_array().cloned().unwrap_or_default().iter().filter_map(|item| {
        let coordinate = &item["coordinate"];
        Some(PathItem {
            launch: item["kind"] == "settings",
            takeoff: item["kind"] == "takeoff",
            command: item["command"].as_i64().unwrap_or(0),
            specifies_coordinate: item["specifiesCoordinate"].as_bool().unwrap_or(false),
            at: (coordinate["latitude"].as_f64()?, coordinate["longitude"].as_f64()?),
            altitude: item["altitudeMetres"].as_f64().unwrap_or(0.0),
        })
    }).collect();
    let home = items.iter().find(|i| i.launch).map(|i| i.at);
    (items, home)
}

fn lon_lat_alt((lat, lon, alt): (f64, f64, f64), bias: f64) -> [f64; 3] {
    [lon, lat, alt + bias]
}

pub fn path_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let bias = setting(backend, "altitudeBias").as_f64().unwrap_or(0.0);
    let (items, home) = path_items(&crate::missionitems::fly_items_view(backend, &["geometry".to_string()]));
    let (markers, segments) = path(&items, home);
    let vehicle = crate::read::object(&backend.get_fields("vehicle", "coordinate,altitudeRelative"));
    let at = &vehicle["coordinate"];
    let relative = vehicle["altitudeRelative"].get("rawValue").or(vehicle["altitudeRelative"].get("value")).and_then(Value::as_f64).filter(|v| v.is_finite()).unwrap_or(0.0);
    json!({
        "kind": "object",
        "class": "Viewer3DPath",
        "markers": markers.iter().map(|m| json!({ "at": lon_lat_alt(m.at, bias), "name": m.name, "colour": m.colour })).collect::<Vec<_>>(),
        "segments": segments.iter().map(|s| json!({ "from": lon_lat_alt(s.from, bias), "to": lon_lat_alt(s.to, bias), "colour": if s.rtl { "red" } else { "orange" } })).collect::<Vec<_>>(),
        "vehicle": at["latitude"].as_f64().zip(at["longitude"].as_f64()).filter(|(lat, lon)| lat.is_finite() && lon.is_finite() && (*lat != 0.0 || *lon != 0.0)).map(|(lat, lon)| json!([lon, lat, relative + bias])),
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
  <way id="11"><nd ref="5"/><nd ref="6"/><nd ref="7"/><nd ref="5"/><tag k="building" v="yes"/><tag k="height" v="12"/></way>
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
        let unit = parse(&CITY.replace(r#"v="12""#, r#"v="12 m""#)).unwrap();
        assert!(unit.buildings.iter().any(|b| b.extruded_height(3.0) == Some(6.0)), "QString::toFloat reads '12 m' as 0, so the building falls back to two storeys");
    }

    #[test]
    fn a_multipolygon_relation_merges_its_ways_into_one_building() {
        let text = CITY.replace("</osm>", r#"<relation id="20"><member type="way" ref="10" role="outer"/><member type="way" ref="12" role="inner"/><tag k="type" v="multipolygon"/><tag k="building" v="yes"/></relation></osm>"#);
        let map = parse(&text).unwrap();
        let merged = map.buildings.iter().find(|b| !b.inner.is_empty()).unwrap();
        assert_eq!((merged.outer.len(), merged.outer[0].len(), merged.inner.len(), merged.inner[0].len(), merged.levels), (1, 5, 1, 3, 1.0), "each member way stays its own ring, inner members become holes and the parts' storeys are kept");
        assert_eq!(map.buildings.len(), 3, "the member ways are replaced by the one merged building");
    }

    #[test]
    fn a_file_without_bounds_takes_them_from_its_nodes() {
        let map = parse(&CITY.replace(r#"<bounds minlat="47.0" minlon="8.0" maxlat="47.1" maxlon="8.1"/>"#, "")).unwrap();
        assert_eq!((map.south_west, map.north_east), ((47.01, 8.01), (47.04, 8.04)));
    }

    fn at(launch: bool, takeoff: bool, command: i64, lat: f64, alt: f64) -> PathItem {
        PathItem { launch, takeoff, command, specifies_coordinate: true, at: (lat, 8.0), altitude: alt }
    }

    #[test]
    fn the_path_follows_viewer3d_vehicle_items() {
        let items = vec![at(true, false, 0, 47.0, 0.0), at(false, true, 22, 47.1, 20.0), at(false, false, 16, 47.2, 30.0), at(false, false, 195, 47.25, 0.0), at(false, false, 16, 47.3, 40.0), at(false, false, 20, 0.0, 0.0)];
        let (markers, segments) = path(&items, Some((47.0, 8.0)));
        assert_eq!(markers.iter().map(|m| m.name).collect::<Vec<_>>(), vec!["T", "W", "R", "W", "L"], "the launch item draws nothing; return-to-launch is an L at home");
        assert_eq!(markers.last().unwrap().at, (47.0, 8.0, 40.0), "RTL sits at home at the last waypoint's altitude");
        assert_eq!(segments.len(), 3, "takeoff to W, W to W (the ROI is skipped), W to home");
        assert_eq!((segments[0].from, segments[0].to), ((47.1, 8.0, 20.0), (47.2, 8.0, 30.0)));
        assert!(segments[2].rtl && !segments[1].rtl);
    }
}
