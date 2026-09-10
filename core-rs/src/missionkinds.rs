use serde_json::{Value, json};

use crate::router::Backend;

pub const DEPS: &[&str] = &[];
const DEFAULT_AREA_METRES: f64 = 150.0;
const METRES_PER_DEGREE: f64 = 111_320.0;

pub struct Kind {
    pub id: &'static str,
    pub title: &'static str,
    pub invokable: &'static str,
    pub complex_name: Option<&'static str>,
    pub geometry: Option<(&'static str, &'static str)>,
    pub placement_hint: &'static str,
}

pub const KINDS: &[Kind] = &[
    Kind { id: "waypoint", title: "Waypoint", invokable: "insertSimpleMissionItem", complex_name: None, geometry: None, placement_hint: "Click the map to place a waypoint." },
    Kind { id: "takeoff", title: "Takeoff", invokable: "insertTakeoffItem", complex_name: None, geometry: None, placement_hint: "Click the map to place a takeoff." },
    Kind { id: "land", title: "Land", invokable: "insertLandItem", complex_name: None, geometry: None, placement_hint: "Click the map to place a land." },
    Kind { id: "roi", title: "Region of Interest", invokable: "insertROIMissionItem", complex_name: None, geometry: None, placement_hint: "Click the map to place a region of interest." },
    Kind { id: "survey", title: "Survey", invokable: "insertComplexMissionItem", complex_name: Some("Survey"), geometry: Some(("area", "surveyAreaPolygon")), placement_hint: "Click the map to place a survey area." },
    Kind { id: "corridor", title: "Corridor Scan", invokable: "insertComplexMissionItem", complex_name: Some("Corridor Scan"), geometry: Some(("line", "corridorPolyline")), placement_hint: "Click the map to place a corridor to scan along." },
    Kind { id: "structure", title: "Structure Scan", invokable: "insertComplexMissionItem", complex_name: Some("Structure Scan"), geometry: Some(("area", "structurePolygon")), placement_hint: "Click the map to place a structure to scan around." },
];

pub fn lookup(id_or_name: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|k| k.id == id_or_name || k.complex_name == Some(id_or_name))
}

fn kind_json(kind: &Kind) -> Value {
    json!({
        "id": kind.id,
        "title": kind.title,
        "invokable": kind.invokable,
        "complexName": kind.complex_name,
        "geometry": kind.geometry.map(|(shape, _)| shape),
        "geometryProperty": kind.geometry.map(|(_, property)| property),
        "shapeNoun": match kind.geometry { Some(("line", _)) => "path", Some(("area", _)) => "area", _ => "shape" },
        "placementHint": kind.placement_hint,
        "simple": kind.complex_name.is_none(),
    })
}

pub fn kinds_view(_backend: &dyn Backend, args: &[String]) -> Value {
    match args.first() {
        Some(wanted) => lookup(wanted).map(kind_json).unwrap_or(json!({ "kind": "null" })),
        None => json!({ "kind": "object", "class": "MissionKinds", "kinds": KINDS.iter().map(kind_json).collect::<Vec<_>>() }),
    }
}

pub fn default_area(latitude: f64, longitude: f64) -> Vec<(f64, f64)> {
    let latitude_span = DEFAULT_AREA_METRES / METRES_PER_DEGREE;
    let longitude_span = DEFAULT_AREA_METRES / (METRES_PER_DEGREE * latitude.to_radians().cos().max(0.01));
    [
        (latitude - latitude_span, longitude - longitude_span),
        (latitude - latitude_span, longitude + longitude_span),
        (latitude + latitude_span, longitude + longitude_span),
        (latitude + latitude_span, longitude - longitude_span),
    ]
    .into_iter()
    .map(|(lat, lon)| crate::geo::wrap(lat, lon))
    .collect()
}

pub fn default_line(latitude: f64, longitude: f64) -> Vec<(f64, f64)> {
    let span = DEFAULT_AREA_METRES / METRES_PER_DEGREE;
    [(latitude - span, longitude), (latitude + span, longitude)].into_iter().map(|(lat, lon)| crate::geo::wrap(lat, lon)).collect()
}

pub fn seed_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let number = |i: usize| args.get(i).and_then(|a| a.parse::<f64>().ok()).filter(|v| v.is_finite());
    let (Some(kind), Some(latitude), Some(longitude)) = (args.first().and_then(|k| lookup(k)), number(1), number(2)) else {
        return json!({ "kind": "null" });
    };
    let Some((shape, property)) = kind.geometry else { return json!({ "kind": "null" }) };
    let points = match shape {
        "line" => default_line(latitude, longitude),
        _ => default_area(latitude, longitude),
    };
    json!({
        "kind": "object",
        "class": "MissionSeed",
        "id": kind.id,
        "property": property,
        "points": points.iter().map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon, "altitude": 0 })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Nothing;
    impl Backend for Nothing {
        fn get(&self, _p: &str) -> String { String::new() }
        fn get_fields(&self, _p: &str, _f: &str) -> String { String::new() }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_catalogue_names_the_invokable_and_the_geometry() {
        let all = kinds_view(&Nothing, &[]);
        assert_eq!(all["kinds"].as_array().unwrap().len(), 7);
        let corridor = kinds_view(&Nothing, &["Corridor Scan".to_string()]);
        assert_eq!(corridor["id"], "corridor");
        assert_eq!(corridor["invokable"], "insertComplexMissionItem");
        assert_eq!(corridor["geometryProperty"], "corridorPolyline");
        assert_eq!(corridor["shapeNoun"], "path");
        let waypoint = kinds_view(&Nothing, &["waypoint".to_string()]);
        assert_eq!(waypoint["simple"], true);
        assert_eq!(waypoint["geometry"], Value::Null);
        assert_eq!(kinds_view(&Nothing, &["nope".to_string()])["kind"], "null");
    }

    #[test]
    fn a_seed_is_a_square_for_areas_and_a_north_south_line_for_corridors() {
        let survey = seed_view(&Nothing, &["survey".to_string(), "47.0".to_string(), "8.0".to_string()]);
        assert_eq!(survey["property"], "surveyAreaPolygon");
        assert_eq!(survey["points"].as_array().unwrap().len(), 4);
        let width = survey["points"][1]["longitude"].as_f64().unwrap() - survey["points"][0]["longitude"].as_f64().unwrap();
        assert!(width > 2.0 * DEFAULT_AREA_METRES / METRES_PER_DEGREE);
        let corridor = seed_view(&Nothing, &["corridor".to_string(), "0".to_string(), "0".to_string()]);
        assert_eq!(corridor["points"].as_array().unwrap().len(), 2);
        assert_eq!(corridor["points"][0]["longitude"], 0.0);
        assert_eq!(seed_view(&Nothing, &["waypoint".to_string(), "0".to_string(), "0".to_string()])["kind"], "null");
        assert_eq!(seed_view(&Nothing, &["survey".to_string()])["kind"], "null");
    }

    #[test]
    fn a_seed_beside_the_dateline_stays_on_the_map() {
        let area = default_area(-16.5, 179.999);
        assert!(area.iter().all(|(lat, lon)| (-90.0..=90.0).contains(lat) && (-180.0..=180.0).contains(lon)), "a survey seeded in Fiji must not carry a longitude no autopilot will accept: {area:?}");
        assert!(area.iter().any(|(_, lon)| *lon < 0.0) && area.iter().any(|(_, lon)| *lon > 0.0), "the area still straddles the dateline rather than collapsing to one side");
        let pole = default_area(89.999, 8.5);
        assert!(pole.iter().all(|(lat, _)| (-90.0..=90.0).contains(lat)));
        let line = default_line(89.999, 179.999);
        assert!(line.iter().all(|(lat, lon)| (-90.0..=90.0).contains(lat) && (-180.0..=180.0).contains(lon)));
        let ordinary = default_area(47.4, 8.5);
        assert!(ordinary.iter().all(|(lat, lon)| *lat > 47.0 && *lat < 48.0 && *lon > 8.0 && *lon < 9.0), "an ordinary seed is unchanged");
    }
}
