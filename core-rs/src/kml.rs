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

fn descendant<'a>(node: Node<'a, 'a>, name: &str) -> Option<Node<'a, 'a>> {
    node.descendants().find(|n| n.is_element() && n.tag_name().name() == name)
}

fn child<'a>(node: Node<'a, 'a>, name: &str) -> Option<Node<'a, 'a>> {
    node.children().find(|n| n.is_element() && n.tag_name().name() == name)
}

fn coordinates(node: Node) -> Result<Vec<(f64, f64)>, String> {
    node.text()
        .unwrap_or("")
        .split_whitespace()
        .map(|triple| {
            let mut parts = triple.split(',');
            let lon = parts.next().and_then(|v| v.parse::<f64>().ok());
            let lat = parts.next().and_then(|v| v.parse::<f64>().ok());
            match (lat, lon) {
                (Some(lat), Some(lon)) => Ok((lat, lon)),
                _ => Err(format!("bad coordinate: {triple}")),
            }
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

pub fn parse(text: &str) -> Result<Shape, String> {
    let document = Document::parse(text).map_err(|e| format!("Unable to parse KML: {e}"))?;
    let root = document.root();
    if let Some(polygon) = descendant(root, "Polygon") {
        let node = child(polygon, "outerBoundaryIs").and_then(|b| child(b, "LinearRing")).and_then(|r| child(r, "coordinates")).ok_or("Unable to find coordinates node in KML")?;
        return Ok(Shape::Polygon(clockwise(coordinates(node)?)));
    }
    if let Some(line) = descendant(root, "LineString") {
        let node = child(line, "coordinates").ok_or("Unable to find coordinates node in KML")?;
        return Ok(Shape::Polyline(coordinates(node)?));
    }
    Err("No supported type found in KML file.".to_string())
}

pub fn kml_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return refused("view.kml needs the path of the item to describe") };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return json!({ "kind": "object", "class": "KmlFile", "path": path, "readable": false, "error": e.to_string() }),
    };
    let points = |p: &[(f64, f64)]| p.iter().map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon })).collect::<Vec<_>>();
    match parse(&text) {
        Err(error) => json!({ "kind": "object", "class": "KmlFile", "path": path, "readable": true, "valid": false, "error": error }),
        Ok(Shape::Polygon(p)) => json!({ "kind": "object", "class": "KmlFile", "path": path, "readable": true, "valid": true, "error": "", "shape": "polygon", "count": p.len(), "points": points(&p) }),
        Ok(Shape::Polyline(p)) => json!({ "kind": "object", "class": "KmlFile", "path": path, "readable": true, "valid": true, "error": "", "shape": "polyline", "count": p.len(), "points": points(&p) }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{}/../test/MissionManager/{name}", env!("CARGO_MANIFEST_DIR"))).or_else(|_| std::fs::read_to_string(format!("{}/../test/Utilities/Shape/{name}", env!("CARGO_MANIFEST_DIR")))).unwrap()
    }

    #[test]
    fn the_good_polygon_parses_and_the_three_bad_ones_are_refused_like_qgc_map_polygon_test() {
        let Shape::Polygon(points) = parse(&fixture("PolygonGood.kml")).unwrap() else { panic!("not a polygon") };
        assert!(points.len() >= 4);
        assert!(parse(&fixture("PolygonBadXml.kml")).unwrap_err().starts_with("Unable to parse KML"));
        assert_eq!(parse(&fixture("PolygonMissingNode.kml")).unwrap_err(), "No supported type found in KML file.");
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
}
