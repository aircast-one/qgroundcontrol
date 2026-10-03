use roxmltree::{Document, Node};
use serde_json::{Value, json};

use crate::read::refused;
use crate::router::Backend;

pub const DEPS: &[&str] = &[];

#[derive(Debug, PartialEq)]
pub enum Shape {
    Polygon(Vec<(f64, f64)>),
    Polyline(Vec<(f64, f64)>),
}

fn child<'a>(node: Node<'a, 'a>, name: &str) -> Option<Node<'a, 'a>> {
    node.children().find(|n| n.is_element() && n.tag_name().name() == name)
}

fn coordinates(node: Node) -> Vec<(f64, f64)> {
    node.text()
        .unwrap_or("")
        .split_whitespace()
        .filter_map(|triple| {
            let mut parts = triple.split(',');
            let lon = parts.next()?.parse::<f64>().ok()?;
            let lat = parts.next()?.parse::<f64>().ok()?;
            ((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)).then_some((lat, lon))
        })
        .collect()
}

pub fn clockwise(points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let sum: f64 = points.iter().enumerate().map(|(i, (lat1, lon1))| {
        let (lat2, lon2) = points[(i + 1) % points.len()];
        (lon2 - lon1) * (lat2 + lat1)
    }).sum();
    match sum < 0.0 {
        true => points.into_iter().rev().collect(),
        false => points,
    }
}

const VERTEX_FILTER_METRES: f64 = 5.0;

fn filter_vertices(points: Vec<(f64, f64)>, minimum: usize) -> Vec<(f64, f64)> {
    let Some(first) = points.first().copied().filter(|_| points.len() > minimum) else { return points };
    points
        .iter()
        .skip(1)
        .fold((vec![first], points.len()), |(kept, count), point| match count > minimum && kept.last().is_some_and(|last| crate::surveygrid::distance_between(*last, *point) < VERTEX_FILTER_METRES) {
            true => (kept, count - 1),
            false => (kept.into_iter().chain(std::iter::once(*point)).collect(), count),
        })
        .0
}

fn without_closing_vertex(points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    match points.len() > 3 && points.first() == points.last() {
        true => points[..points.len() - 1].to_vec(),
        false => points,
    }
}

fn shape(text: &str, wanted: Option<bool>) -> Result<Shape, String> {
    let failed = |detail: &str| format!("KML file load failed. {detail}");
    let document = Document::parse(text).map_err(|e| failed(&format!("Unable to parse KML file: {e}")))?;
    let root = document.root();
    let named = |name: &'static str| root.descendants().filter(move |n| n.is_element() && n.tag_name().name() == name);
    let polygon = || {
        named("Polygon")
            .filter_map(|polygon| child(polygon, "outerBoundaryIs").and_then(|b| child(b, "LinearRing")).and_then(|r| child(r, "coordinates")))
            .map(coordinates)
            .find(|points| points.len() >= 3)
            .map(|points| Shape::Polygon(filter_vertices(clockwise(without_closing_vertex(points)), 3)))
    };
    let line = || {
        named("LineString")
            .filter_map(|line| child(line, "coordinates"))
            .map(coordinates)
            .find(|points| points.len() >= 2)
            .map(|points| Shape::Polyline(filter_vertices(points, 2)))
    };
    let missing = |node: &'static str, plural: &str| failed(&if named(node).next().is_none() { format!("Unable to find {node} node in KML") } else { format!("No valid {plural} found in KML file") });
    match wanted {
        Some(true) => line().ok_or_else(|| missing("LineString", "polylines")),
        Some(false) => polygon().ok_or_else(|| missing("Polygon", "polygons")),
        None => polygon().or_else(line).ok_or_else(|| failed("No supported type found in KML file.")),
    }
}

pub fn parse(text: &str) -> Result<Shape, String> {
    shape(text, None)
}

pub fn parse_wanted(text: &str, polyline: bool) -> Result<Shape, String> {
    shape(text, Some(polyline))
}

pub fn kml_view(_backend: &dyn Backend, args: &[String]) -> Value {
    kml_shape_view(args, None)
}

pub fn kml_shape_view(args: &[String], polyline: Option<bool>) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return refused("view.kmlFile needs the path of a kml or kmz file to read, as view.kmlFile(/Users/you/area.kml)") };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return json!({ "kind": "object", "class": "KmlFile", "path": path, "readable": false, "error": e.to_string() }),
    };
    let points = |p: &[(f64, f64)]| p.iter().map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon })).collect::<Vec<_>>();
    let parsed = match polyline {
        Some(line) => parse_wanted(&text, line),
        None => parse(&text),
    };
    match parsed {
        Err(error) => json!({ "kind": "object", "class": "KmlFile", "path": path, "readable": true, "valid": false, "error": error }),
        Ok(Shape::Polygon(p)) => json!({ "kind": "object", "class": "KmlFile", "path": path, "readable": true, "valid": true, "error": "", "shape": "polygon", "count": p.len(), "points": points(&p) }),
        Ok(Shape::Polyline(p)) => json!({ "kind": "object", "class": "KmlFile", "path": path, "readable": true, "valid": true, "error": "", "shape": "polyline", "count": p.len(), "points": points(&p) }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{}/../test/MissionManager/{name}", env!("CARGO_MANIFEST_DIR"))).or_else(|_| std::fs::read_to_string(format!("{}/../test/Utilities/Geo/{name}", env!("CARGO_MANIFEST_DIR")))).unwrap()
    }

    #[test]
    fn a_bad_or_out_of_range_coordinate_is_skipped_like_kml_helper() {
        let kml = "<kml><Placemark><Polygon><outerBoundaryIs><LinearRing><coordinates>8.0,47.0,0 nonsense 8.1,47.0,0 200,47.1,0 8.1,47.1,0 8.0,47.1,0</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark></kml>";
        let Ok(Shape::Polygon(points)) = parse(kml) else { panic!("the ring survives its bad triples") };
        assert_eq!(points.len(), 4, "the malformed triple and the longitude of 200 are dropped, the rest kept: {points:?}");
    }

    #[test]
    fn a_polygon_drops_its_closing_vertex_and_vertices_within_five_metres_as_kml_helper_does() {
        let near = (47.0, 8.00002);
        assert_eq!(filter_vertices(vec![(47.0, 8.0), near, (47.0, 8.01), (47.01, 8.01)], 3), vec![(47.0, 8.0), (47.0, 8.01), (47.01, 8.01)]);
        assert_eq!(filter_vertices(vec![(47.0, 8.0), near, (47.0, 8.01)], 3).len(), 3, "never below the minimum");
        assert_eq!(without_closing_vertex(vec![(1.0, 1.0), (1.0, 2.0), (2.0, 2.0), (1.0, 1.0)]), vec![(1.0, 1.0), (1.0, 2.0), (2.0, 2.0)]);
    }

    #[test]
    fn the_good_polygon_parses_and_the_three_bad_ones_are_refused_like_qgc_map_polygon_test() {
        let Shape::Polygon(points) = parse(&fixture("PolygonGood.kml")).unwrap() else { panic!("not a polygon") };
        assert!(points.len() >= 4);
        assert!(parse(&fixture("PolygonBadXml.kml")).unwrap_err().starts_with("KML file load failed. Unable to parse KML file"));
        assert_eq!(parse(&fixture("PolygonMissingNode.kml")).unwrap_err(), "KML file load failed. No supported type found in KML file.");
        assert!(parse(&fixture("PolygonBadCoordinatesNode.kml")).is_err());
    }

    #[test]
    fn a_linestring_becomes_a_polyline_and_winding_is_made_clockwise() {
        let Shape::Polyline(points) = parse(&fixture("polyline.kml")).unwrap() else { panic!("not a polyline") };
        assert!(points.len() >= 2);
        let counter = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let sum: f64 = counter.iter().enumerate().map(|(i, (a1, o1))| { let (a2, o2) = counter[(i + 1) % 4]; (o2 - o1) * (a2 + a1) }).sum();
        let fixed = clockwise(counter.clone());
        assert_eq!(fixed == counter, sum >= 0.0);
        assert_eq!(clockwise(fixed.clone()), fixed);
    }

    #[test]
    fn a_corridor_import_takes_the_line_from_a_file_that_also_holds_a_polygon() {
        let both = r#"<kml><Document><Placemark><Polygon><outerBoundaryIs><LinearRing><coordinates>8.0,47.0 8.01,47.0 8.01,47.01 8.0,47.0</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark><Placemark><LineString><coordinates>8.0,47.0 8.02,47.02</coordinates></LineString></Placemark></Document></kml>"#;
        assert!(matches!(parse_wanted(both, true), Ok(Shape::Polyline(_))), "QGCMapPolyline::loadKMLFile looks for LineStrings only");
        assert!(matches!(parse_wanted(both, false), Ok(Shape::Polygon(_))));
    }
}
