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
    node.children()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}

fn positions(node: Node) -> Vec<(f64, f64, f64)> {
    node.text()
        .unwrap_or("")
        .split_whitespace()
        .filter_map(|triple| {
            let mut parts = triple.split(',');
            let lon = parts.next()?.parse::<f64>().ok()?;
            let lat = parts.next()?.parse::<f64>().ok()?;
            let alt = parts
                .next()
                .and_then(|a| a.parse::<f64>().ok())
                .unwrap_or(0.0);
            ((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon))
                .then_some((lat, lon, alt))
        })
        .collect()
}

fn coordinates(node: Node) -> Vec<(f64, f64)> {
    positions(node)
        .into_iter()
        .map(|(lat, lon, _)| (lat, lon))
        .collect()
}

pub fn clockwise(points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let sum: f64 = points
        .iter()
        .enumerate()
        .map(|(i, (lat1, lon1))| {
            let (lat2, lon2) = points[(i + 1) % points.len()];
            (lon2 - lon1) * (lat2 + lat1)
        })
        .sum();
    match sum < 0.0 {
        true => points.into_iter().rev().collect(),
        false => points,
    }
}

const VERTEX_FILTER_METRES: f64 = 5.0;

fn filter_vertices(points: Vec<(f64, f64)>, minimum: usize) -> Vec<(f64, f64)> {
    let Some(first) = points.first().copied().filter(|_| points.len() > minimum) else {
        return points;
    };
    points
        .iter()
        .skip(1)
        .fold((vec![first], points.len()), |(kept, count), point| {
            match count > minimum
                && kept.last().is_some_and(|last| {
                    crate::surveygrid::distance_between(*last, *point) < VERTEX_FILTER_METRES
                }) {
                true => (kept, count - 1),
                false => (
                    kept.into_iter().chain(std::iter::once(*point)).collect(),
                    count,
                ),
            }
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
    let document =
        Document::parse(text).map_err(|e| failed(&format!("Unable to parse KML file: {e}")))?;
    let root = document.root();
    let named = |name: &'static str| {
        root.descendants()
            .filter(move |n| n.is_element() && n.tag_name().name() == name)
    };
    let polygon = || {
        named("Polygon")
            .filter_map(|polygon| {
                child(polygon, "outerBoundaryIs")
                    .and_then(|b| child(b, "LinearRing"))
                    .and_then(|r| child(r, "coordinates"))
            })
            .map(coordinates)
            .find(|points| points.len() >= 3)
            .map(|points| {
                Shape::Polygon(filter_vertices(
                    clockwise(without_closing_vertex(points)),
                    3,
                ))
            })
    };
    let line = || {
        named("LineString")
            .filter_map(|line| child(line, "coordinates"))
            .map(coordinates)
            .find(|points| points.len() >= 2)
            .map(|points| Shape::Polyline(filter_vertices(points, 2)))
    };
    let missing = |node: &'static str, plural: &str| {
        failed(&if named(node).next().is_none() {
            format!("Unable to find {node} node in KML")
        } else {
            format!("No valid {plural} found in KML file")
        })
    };
    match wanted {
        Some(true) => line().ok_or_else(|| missing("LineString", "polylines")),
        Some(false) => polygon().ok_or_else(|| missing("Polygon", "polygons")),
        None => polygon()
            .or_else(line)
            .ok_or_else(|| failed("No supported type found in KML file.")),
    }
}

pub fn parse(text: &str) -> Result<Shape, String> {
    shape(text, None)
}

pub fn parse_wanted(text: &str, polyline: bool) -> Result<Shape, String> {
    shape(text, Some(polyline))
}

pub const DENSE_TRACK_VERTICES: usize = 200;
pub const DENSE_TRACK_TOLERANCE_M: f64 = 3.0;
pub const GIMBAL_DATA: &str = "gimbal";
pub const LOOK_AT_DATA: &str = "lookAt";
pub const AREAS_ONLY: &str =
    "This file only has areas. To fly over an area, use Pattern > Survey and load the file there.";

type Position = (f64, f64, f64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteAltitude {
    Ground,
    AboveGround,
    Absolute,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    pub path: Vec<Position>,
    pub altitude: RouteAltitude,
    pub path_count: usize,
    pub source_vertex_count: usize,
    pub placemarks: Vec<Position>,
    pub gimbal: Vec<(f64, f64)>,
    pub look_at: Vec<Option<Position>>,
}

fn load_failed(detail: &str) -> String {
    format!("KML file load failed. {detail}")
}

fn parse_failed(error: &roxmltree::Error) -> String {
    load_failed(&format!(
        "Unable to parse KML file: {error} line: {}",
        error.pos().row
    ))
}

fn recorded<'a>(line: Node<'a, 'a>, name: &str, count: usize) -> Option<Vec<&'a str>> {
    line.parent()?
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "Data")
        .filter(|data| data.attribute("name") == Some(name))
        .filter_map(|data| child(data, "value").and_then(|v| v.text()))
        .map(|value| value.split_whitespace().collect::<Vec<_>>())
        .find(|tokens| tokens.len() == count)
}

fn gimbal_angles(token: &str) -> (f64, f64) {
    let mut values = token.split(',').map(|v| v.parse::<f64>().unwrap_or(0.0));
    (values.next().unwrap_or(0.0), values.next().unwrap_or(0.0))
}

fn look_at_point(token: &str) -> Option<Position> {
    match token.split(',').map(str::parse::<f64>).collect::<Vec<_>>()[..] {
        [Ok(lon), Ok(lat), Ok(alt)] => Some((lat, lon, alt)),
        _ => None,
    }
}

fn subset<T: Clone>(values: Vec<T>, kept: Option<&[usize]>) -> Vec<T> {
    match kept {
        Some(kept) => kept
            .iter()
            .filter_map(|i| values.get(*i).cloned())
            .collect(),
        None => values,
    }
}

pub fn route(text: &str) -> Result<Route, String> {
    let document = Document::parse(text).map_err(|e| parse_failed(&e))?;
    let named = |name: &str| -> Vec<Node> {
        document
            .root()
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == name)
            .collect()
    };
    let (lines, points) = (named("LineString"), named("Point"));
    let is_path = !lines.is_empty();
    let geometries: Vec<(Vec<Position>, &str)> = match is_path {
        true => &lines,
        false => &points,
    }
    .iter()
    .filter_map(|node| {
        let coords = child(*node, "coordinates").map(positions)?;
        let mode = child(*node, "altitudeMode")
            .and_then(|m| m.text())
            .unwrap_or("")
            .trim();
        (!coords.is_empty()).then_some((coords, mode))
    })
    .collect();
    let path: Vec<Position> = geometries
        .iter()
        .flat_map(|(coords, _)| match is_path {
            true => coords.clone(),
            false => coords[..1].to_vec(),
        })
        .collect();
    if path.is_empty() {
        return Err(match named("Polygon").is_empty() {
            true => load_failed("No path or placemarks found in KML file"),
            false => AREAS_ONLY.to_string(),
        });
    }
    let all = |wanted: &str| geometries.iter().all(|(_, mode)| *mode == wanted);
    let altitude = match () {
        _ if !path.iter().any(|(_, _, alt)| *alt > 0.0) => RouteAltitude::Ground,
        _ if all("absolute") => RouteAltitude::Absolute,
        _ if all("relativeToGround") => RouteAltitude::AboveGround,
        _ => RouteAltitude::Ground,
    };
    let placemarks = match is_path {
        true => points
            .iter()
            .filter_map(|n| child(*n, "coordinates").map(positions)?.first().copied())
            .collect(),
        false => Vec::new(),
    };
    let data = |name: &str| match lines[..] {
        [line] => recorded(line, name, path.len()).unwrap_or_default(),
        _ => Vec::new(),
    };
    let gimbal: Vec<(f64, f64)> = data(GIMBAL_DATA).into_iter().map(gimbal_angles).collect();
    let look_at: Vec<Option<Position>> =
        data(LOOK_AT_DATA).into_iter().map(look_at_point).collect();
    let kept = (is_path && path.len() > DENSE_TRACK_VERTICES)
        .then(|| crate::geo::simplify_path_indices(&path, DENSE_TRACK_TOLERANCE_M));
    Ok(Route {
        source_vertex_count: path.len(),
        path: subset(path, kept.as_deref()),
        altitude,
        path_count: if is_path { geometries.len() } else { 0 },
        placemarks,
        gimbal: subset(gimbal, kept.as_deref()),
        look_at: subset(look_at, kept.as_deref()),
    })
}

pub fn is_kml(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".kml") || lower.ends_with(".kmz")
}

pub fn read(path: &str) -> Result<String, String> {
    let bytes = crate::vfs::read(path).map_err(|e| format!("{e} {path}"))?;
    match path.to_lowercase().ends_with(".kmz") {
        false => Ok(String::from_utf8_lossy(&bytes).into_owned()),
        true => {
            let entries = crate::compression::unzip(&bytes).map_err(|e| load_failed(&e))?;
            entries
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("doc.kml"))
                .or_else(|| {
                    entries
                        .iter()
                        .find(|(name, _)| name.to_lowercase().ends_with(".kml"))
                })
                .map(|(_, data)| String::from_utf8_lossy(data).into_owned())
                .ok_or_else(|| load_failed("The KMZ file holds no doc.kml."))
        }
    }
}

pub fn kml_view(_backend: &dyn Backend, args: &[String]) -> Value {
    kml_shape_view(args, None)
}

pub fn kml_shape_view(args: &[String], polyline: Option<bool>) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else {
        return refused(
            "view.kmlFile needs the path of a kml or kmz file to read, as view.kmlFile(/Users/you/area.kml)",
        );
    };
    let text = match crate::vfs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            return json!({ "kind": "object", "class": "KmlFile", "path": path, "readable": false, "error": e.to_string() });
        }
    };
    let points = |p: &[(f64, f64)]| {
        p.iter()
            .map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon }))
            .collect::<Vec<_>>()
    };
    let parsed = match polyline {
        Some(line) => parse_wanted(&text, line),
        None => parse(&text),
    };
    match parsed {
        Err(error) => {
            json!({ "kind": "object", "class": "KmlFile", "path": path, "readable": true, "valid": false, "error": error })
        }
        Ok(Shape::Polygon(p)) => {
            json!({ "kind": "object", "class": "KmlFile", "path": path, "readable": true, "valid": true, "error": "", "shape": "polygon", "count": p.len(), "points": points(&p) })
        }
        Ok(Shape::Polyline(p)) => {
            json!({ "kind": "object", "class": "KmlFile", "path": path, "readable": true, "valid": true, "error": "", "shape": "polyline", "count": p.len(), "points": points(&p) })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/../test/MissionManager/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .or_else(|_| {
            std::fs::read_to_string(format!(
                "{}/../test/Utilities/Geo/{name}",
                env!("CARGO_MANIFEST_DIR")
            ))
        })
        .unwrap()
    }

    fn route_fixture(name: &str) -> Route {
        route(
            &read(&format!(
                "{}/../test/MissionManager/{name}",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn a_ground_path_joins_its_vertices_and_flies_above_the_terrain_like_load_route_from_file() {
        let ground = route_fixture("RouteGroundPath.kml");
        assert_eq!(ground.path.len(), 3);
        assert_eq!(ground.path_count, 1);
        assert_eq!(ground.altitude, RouteAltitude::Ground);
        assert_eq!(ground.path[2].1, 44.8010);
        let kmz = route_fixture("RouteAbsolutePath.kmz");
        assert_eq!(
            kmz.altitude,
            RouteAltitude::Absolute,
            "a kmz is read through its doc.kml"
        );
        assert_eq!((kmz.path[0].2, kmz.path[2].2), (560.5, 590.25));
        let marks = route_fixture("RoutePlacemarks.kml");
        assert_eq!(marks.path.len(), 2, "only points: one waypoint per point");
        assert_eq!((marks.path_count, marks.placemarks.len()), (0, 0));
    }

    #[test]
    fn placemarks_beside_a_path_and_mixed_modes_and_two_lines_are_kept_apart() {
        let towers = route(
            r#"<kml><Document>
  <Placemark><LineString><altitudeMode>relativeToGround</altitudeMode><coordinates>44.79,41.71,30 44.795,41.712,30</coordinates></LineString></Placemark>
  <Placemark><LineString><altitudeMode>relativeToGround</altitudeMode><coordinates>44.80,41.715,40</coordinates></LineString></Placemark>
  <Placemark><Point><coordinates>44.7951,41.7122,0</coordinates></Point></Placemark>
</Document></kml>"#,
        )
        .unwrap();
        assert_eq!((towers.path.len(), towers.path_count), (3, 2));
        assert_eq!(towers.altitude, RouteAltitude::AboveGround);
        assert_eq!(towers.placemarks, vec![(41.7122, 44.7951, 0.0)]);
        let mixed = route(
            r#"<kml><Placemark><LineString><altitudeMode>absolute</altitudeMode><coordinates>8,47,500 8.1,47.1,510</coordinates></LineString></Placemark><Placemark><LineString><coordinates>8.2,47.2,520</coordinates></LineString></Placemark></kml>"#,
        )
        .unwrap();
        assert_eq!(
            mixed.altitude,
            RouteAltitude::Ground,
            "a clampToGround line beside an absolute one sends the whole file to the ground"
        );
    }

    #[test]
    fn an_area_an_empty_file_and_broken_xml_are_refused_with_what_went_wrong() {
        let area = "<kml><Placemark><Polygon><outerBoundaryIs><LinearRing><coordinates>8,47 8.1,47 8.1,47.1 8,47</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark></kml>";
        assert_eq!(route(area).unwrap_err(), AREAS_ONLY);
        assert_eq!(
            route("<kml><Document/></kml>").unwrap_err(),
            "KML file load failed. No path or placemarks found in KML file"
        );
        let broken = route("<kml>\n<Placemark>\n</kml>").unwrap_err();
        assert!(
            broken.starts_with("KML file load failed. Unable to parse KML file:")
                && broken.ends_with("line: 3"),
            "{broken}"
        );
        assert!(read("/no/such/route.kmz").is_err());
    }

    #[test]
    fn a_recorded_track_replays_its_camera_only_when_every_vertex_has_a_token() {
        let recorded = |gimbal: &str, look_at: &str| {
            route(&format!(
                r#"<kml><Placemark><ExtendedData><Data name="gimbal"><value>{gimbal}</value></Data><Data name="lookAt"><value>{look_at}</value></Data></ExtendedData><LineString><altitudeMode>absolute</altitudeMode><coordinates>8,47,500 8.001,47,500 8.002,47,500</coordinates></LineString></Placemark></kml>"#
            ))
            .unwrap()
        };
        let full = recorded(
            "-30.0,10.0 nan,nan -45.0,0.0",
            "- 8.1000000,47.1000000,600.0 -",
        );
        assert_eq!(full.gimbal[0], (-30.0, 10.0));
        assert!(full.gimbal[1].0.is_nan());
        assert_eq!(full.look_at, vec![None, Some((47.1, 8.1, 600.0)), None]);
        let short = recorded("-30.0,10.0", "- -");
        assert!(short.gimbal.is_empty() && short.look_at.is_empty());
    }

    #[test]
    fn a_dense_track_is_simplified_and_its_recording_kept_in_step() {
        let count = 300;
        let coordinates: Vec<String> = (0..count)
            .map(|i| format!("{:.7},47,500", 8.0 + i as f64 * 0.00005))
            .collect();
        let gimbal = vec!["-20.0,0.0"; count].join(" ");
        let dense = route(&format!(
            r#"<kml><Placemark><ExtendedData><Data name="gimbal"><value>{gimbal}</value></Data></ExtendedData><LineString><altitudeMode>absolute</altitudeMode><coordinates>{}</coordinates></LineString></Placemark></kml>"#,
            coordinates.join(" ")
        ))
        .unwrap();
        assert_eq!(dense.source_vertex_count, count);
        assert_eq!(dense.path.len(), 2, "a straight track keeps its ends");
        assert_eq!(dense.gimbal.len(), 2);
    }

    #[test]
    fn a_bad_or_out_of_range_coordinate_is_skipped_like_kml_helper() {
        let kml = "<kml><Placemark><Polygon><outerBoundaryIs><LinearRing><coordinates>8.0,47.0,0 nonsense 8.1,47.0,0 200,47.1,0 8.1,47.1,0 8.0,47.1,0</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark></kml>";
        let Ok(Shape::Polygon(points)) = parse(kml) else {
            panic!("the ring survives its bad triples")
        };
        assert_eq!(
            points.len(),
            4,
            "the malformed triple and the longitude of 200 are dropped, the rest kept: {points:?}"
        );
    }

    #[test]
    fn a_polygon_drops_its_closing_vertex_and_vertices_within_five_metres_as_kml_helper_does() {
        let near = (47.0, 8.00002);
        assert_eq!(
            filter_vertices(vec![(47.0, 8.0), near, (47.0, 8.01), (47.01, 8.01)], 3),
            vec![(47.0, 8.0), (47.0, 8.01), (47.01, 8.01)]
        );
        assert_eq!(
            filter_vertices(vec![(47.0, 8.0), near, (47.0, 8.01)], 3).len(),
            3,
            "never below the minimum"
        );
        assert_eq!(
            without_closing_vertex(vec![(1.0, 1.0), (1.0, 2.0), (2.0, 2.0), (1.0, 1.0)]),
            vec![(1.0, 1.0), (1.0, 2.0), (2.0, 2.0)]
        );
    }

    #[test]
    fn the_good_polygon_parses_and_the_three_bad_ones_are_refused_like_qgc_map_polygon_test() {
        let Shape::Polygon(points) = parse(&fixture("PolygonGood.kml")).unwrap() else {
            panic!("not a polygon")
        };
        assert!(points.len() >= 4);
        assert!(
            parse(&fixture("PolygonBadXml.kml"))
                .unwrap_err()
                .starts_with("KML file load failed. Unable to parse KML file")
        );
        assert_eq!(
            parse(&fixture("PolygonMissingNode.kml")).unwrap_err(),
            "KML file load failed. No supported type found in KML file."
        );
        assert!(parse(&fixture("PolygonBadCoordinatesNode.kml")).is_err());
    }

    #[test]
    fn a_linestring_becomes_a_polyline_and_winding_is_made_clockwise() {
        let Shape::Polyline(points) = parse(&fixture("polyline.kml")).unwrap() else {
            panic!("not a polyline")
        };
        assert!(points.len() >= 2);
        let counter = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let sum: f64 = counter
            .iter()
            .enumerate()
            .map(|(i, (a1, o1))| {
                let (a2, o2) = counter[(i + 1) % 4];
                (o2 - o1) * (a2 + a1)
            })
            .sum();
        let fixed = clockwise(counter.clone());
        assert_eq!(fixed == counter, sum >= 0.0);
        assert_eq!(clockwise(fixed.clone()), fixed);
    }

    #[test]
    fn a_corridor_import_takes_the_line_from_a_file_that_also_holds_a_polygon() {
        let both = r#"<kml><Document><Placemark><Polygon><outerBoundaryIs><LinearRing><coordinates>8.0,47.0 8.01,47.0 8.01,47.01 8.0,47.0</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark><Placemark><LineString><coordinates>8.0,47.0 8.02,47.02</coordinates></LineString></Placemark></Document></kml>"#;
        assert!(
            matches!(parse_wanted(both, true), Ok(Shape::Polyline(_))),
            "QGCMapPolyline::loadKMLFile looks for LineStrings only"
        );
        assert!(matches!(parse_wanted(both, false), Ok(Shape::Polygon(_))));
    }
}
