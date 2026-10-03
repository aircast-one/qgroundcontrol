use serde_json::{Value, json};
use shapefile::{Shape, ShapeType};

use crate::router::Backend;

pub const DEPS: &[&str] = &[];
const VERTEX_FILTER_METRES: f64 = 5.0;
const EARTH_RADIUS_METRES: f64 = 6_371_000.0;

#[derive(Debug, PartialEq)]
pub enum Projection {
    Wgs84,
    Utm { zone: u8, southern: bool },
}

pub fn projection(prj: &str) -> Result<Projection, String> {
    let line = prj.lines().next().unwrap_or("").trim();
    if line.starts_with("GEOGCS[\"GCS_WGS_1984\"") {
        return Ok(Projection::Wgs84);
    }
    let Some(rest) = line.strip_prefix("PROJCS[\"WGS_1984_UTM_Zone_") else {
        let named = ["GEOGCS[\"", "PROJCS[\""].iter().find_map(|prefix| line.strip_prefix(prefix)).and_then(|rest| rest.split('"').next()).filter(|name| !name.is_empty());
        return Err(match named {
            Some(name) => format!("Unsupported projection: {name}. Supported projections are: WGS84 (GEOGCS[\"GCS_WGS_1984\"]) and UTM (PROJCS[\"WGS_1984_UTM_Zone_##N/S\"]). Convert your shapefile to WGS84 using QGIS or ogr2ogr."),
            None => "Unable to parse projection from PRJ file. Supported projections are: WGS84 (GEOGCS[\"GCS_WGS_1984\"]) and UTM (PROJCS[\"WGS_1984_UTM_Zone_##N/S\"]).".to_string(),
        });
    };
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let hemisphere = rest.chars().nth(digits.len());
    match (digits.parse::<u8>().ok().filter(|z| (1..=60).contains(z)), hemisphere) {
        (Some(zone), Some('N')) => Ok(Projection::Utm { zone, southern: false }),
        (Some(zone), Some('S')) => Ok(Projection::Utm { zone, southern: true }),
        _ => Err("UTM projection is not in supported format. Must be PROJCS[\"WGS_1984_UTM_Zone_##N/S".to_string()),
    }
}

fn metres_between(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (lat1, lat2) = (a.0.to_radians(), b.0.to_radians());
    let (dlat, dlon) = ((b.0 - a.0).to_radians(), (b.1 - a.1).to_radians());
    let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_METRES * h.sqrt().min(1.0).asin()
}

fn to_geo(projection: &Projection, x: f64, y: f64) -> (f64, f64) {
    match projection {
        Projection::Wgs84 => (y, x),
        Projection::Utm { zone, southern } => {
            let letter = if *southern { 'M' } else { 'N' };
            utm::wsg84_utm_to_lat_lon(x, y, *zone, letter).unwrap_or((y, x))
        }
    }
}

const CLOSURE_METRES: f64 = 0.01;

pub fn filtered(points: Vec<(f64, f64)>, closed: bool) -> Vec<(f64, f64)> {
    let minimum = if closed { 3 } else { 2 };
    let total = points.len();
    let closes = |kept: &[(f64, f64)]| kept.first().zip(kept.last()).is_some_and(|(first, last)| metres_between(*first, *last) < CLOSURE_METRES);
    let explicit_closure = closed && closes(&points);
    let kept = points.iter().enumerate().fold(Vec::new(), |kept: Vec<(f64, f64)>, (index, point)| match kept.last() {
        Some(last) if kept.len() + (total - index) > minimum && metres_between(*last, *point) < VERTEX_FILTER_METRES => kept,
        _ => kept.into_iter().chain(std::iter::once(*point)).collect(),
    });
    match explicit_closure && kept.len() > 3 && closes(&kept) {
        true => kept[..kept.len() - 1].to_vec(),
        false => kept,
    }
}

pub fn parse(shp_path: &str) -> Result<(String, usize, Vec<(f64, f64)>), String> {
    parse_wanted(shp_path, None)
}

fn type_name(kind: ShapeType) -> &'static str {
    match kind {
        ShapeType::NullShape => "NullShape",
        ShapeType::Point => "Point",
        ShapeType::Polyline => "Arc",
        ShapeType::Polygon => "Polygon",
        ShapeType::Multipoint => "MultiPoint",
        ShapeType::PointZ => "PointZ",
        ShapeType::PolylineZ => "ArcZ",
        ShapeType::PolygonZ => "PolygonZ",
        ShapeType::MultipointZ => "MultiPointZ",
        ShapeType::PointM => "PointM",
        ShapeType::PolylineM => "ArcM",
        ShapeType::PolygonM => "PolygonM",
        ShapeType::MultipointM => "MultiPointM",
        ShapeType::Multipatch => "MultiPatch",
    }
}

fn outline(shape: &Shape) -> Option<(bool, Vec<(f64, f64)>)> {
    match shape {
        Shape::Polygon(p) => Some((false, p.rings().first()?.points().iter().map(|v| (v.x, v.y)).collect())),
        Shape::PolygonZ(p) => Some((false, p.rings().first()?.points().iter().map(|v| (v.x, v.y)).collect())),
        Shape::Polyline(l) => Some((true, l.parts().first()?.iter().map(|v| (v.x, v.y)).collect())),
        Shape::PolylineZ(l) => Some((true, l.parts().first()?.iter().map(|v| (v.x, v.y)).collect())),
        _ => None,
    }
}

pub fn parse_wanted(shp_path: &str, polyline: Option<bool>) -> Result<(String, usize, Vec<(f64, f64)>), String> {
    let failed = |detail: String| format!("SHP file load failed. {detail}");
    if !shp_path.to_lowercase().ends_with(".shp") {
        return Err(failed(format!("File is not a .shp file: {shp_path}")));
    }
    let prj_path = format!("{}.prj", &shp_path[..shp_path.len() - 4]);
    let prj = std::fs::read_to_string(&prj_path).map_err(|_| failed(format!("File not found: {prj_path}")))?;
    let projection = projection(&prj).map_err(failed)?;
    let kind = shapefile::ShapeReader::from_path(shp_path).map_err(|_| failed("SHPOpen failed.".to_string()))?.header().shape_type;
    let shapes = shapefile::read_shapes(shp_path).map_err(|_| failed("SHPOpen failed.".to_string()))?;
    let entities = shapes.len();
    if entities == 0 && polyline.is_none() {
        return Err(failed("No entities found.".to_string()));
    }
    let file_kind = match kind {
        ShapeType::Polygon | ShapeType::PolygonZ => Some(false),
        ShapeType::Polyline | ShapeType::PolylineZ => Some(true),
        _ => None,
    };
    let file_is_line = match (file_kind, polyline) {
        (Some(found), Some(wanted)) if found == wanted => found,
        (_, Some(wanted)) => return Err(failed(format!("File contains {}, expected {}.", type_name(kind), if wanted { "Arc" } else { "Polygon" }))),
        (Some(found), None) => found,
        (None, None) => return Err(failed("No supported types found.".to_string())),
    };
    let minimum = if file_is_line { 2 } else { 3 };
    let points = shapes
        .iter()
        .filter_map(outline)
        .map(|(_, raw)| raw.into_iter().map(|(x, y)| to_geo(&projection, x, y)).collect::<Vec<_>>())
        .find(|points| points.len() >= minimum)
        .ok_or_else(|| failed(format!("No valid {} found.", if file_is_line { "polylines" } else { "polygons" })))?;
    Ok(match file_is_line {
        true => ("polyline".to_string(), entities, filtered(points, false)),
        false => ("polygon".to_string(), entities, filtered(crate::kml::clockwise(points), true)),
    })
}

pub fn shp_view(_backend: &dyn Backend, args: &[String]) -> Value {
    shp_shape_view(args, None)
}

fn shape_file_view(args: &[String], polyline: bool) -> Value {
    match args.first().is_some_and(|path| path.to_lowercase().ends_with(".shp")) {
        true => shp_shape_view(args, Some(polyline)),
        false => crate::kml::kml_shape_view(args, Some(polyline)),
    }
}

pub fn area_file_view(_backend: &dyn Backend, args: &[String]) -> Value {
    shape_file_view(args, false)
}

pub fn line_file_view(_backend: &dyn Backend, args: &[String]) -> Value {
    shape_file_view(args, true)
}

fn shp_shape_view(args: &[String], wanted: Option<bool>) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return crate::read::refused("this needs the path of a shapefile to read, and none was given") };
    match parse_wanted(path, wanted) {
        Err(error) => json!({ "kind": "object", "class": "ShapeFile", "path": path, "valid": false, "error": error }),
        Ok((shape, entities, points)) => json!({
            "kind": "object",
            "class": "ShapeFile",
            "path": path,
            "valid": true,
            "error": "",
            "shape": shape,
            "entities": entities,
            "count": points.len(),
            "points": points.iter().map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon })).collect::<Vec<_>>(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        format!("{}/../test/Utilities/Geo/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn the_projection_line_is_recognised_or_refused() {
        assert_eq!(projection("GEOGCS[\"GCS_WGS_1984\",DATUM[...]]").unwrap(), Projection::Wgs84);
        assert_eq!(projection("PROJCS[\"WGS_1984_UTM_Zone_33N\",GEOGCS[...]]").unwrap(), Projection::Utm { zone: 33, southern: false });
        assert_eq!(projection("PROJCS[\"WGS_1984_UTM_Zone_5S\"").unwrap(), Projection::Utm { zone: 5, southern: true });
        assert!(projection("PROJCS[\"WGS_1984_UTM_Zone_99N\"").unwrap_err().starts_with("UTM projection is not"));
        assert!(projection("PROJCS[\"Something_Else\"").unwrap_err().starts_with("Unsupported projection: Something_Else. Supported projections are"));
        assert!(projection("garbage").unwrap_err().starts_with("Unable to parse projection from PRJ file."));
    }

    #[test]
    fn the_shape_test_fixtures_load_like_shape_test_cc() {
        let (shape, entities, points) = parse(&fixture("polygon.shp")).expect("polygon.shp");
        assert_eq!(shape, "polygon");
        assert_eq!(entities, 474);
        assert!(points.len() >= 3);
        assert!(metres_between(points[0], *points.last().unwrap()) >= VERTEX_FILTER_METRES);
        let (shape, _, line) = parse(&fixture("pline.shp")).expect("pline.shp");
        assert_eq!(shape, "polyline");
        assert!(line.len() >= 2);
        assert!(parse(&fixture("polygon.kml")).unwrap_err().starts_with("SHP file load failed. File is not a .shp file"));
        assert!(parse(&fixture("missing.shp")).unwrap_err().starts_with("SHP file load failed. File not found"));
        assert_eq!(parse_wanted(&fixture("polygon.shp"), Some(true)).unwrap_err(), "SHP file load failed. File contains Polygon, expected Arc.");
        assert_eq!(parse_wanted(&fixture("pline.shp"), Some(false)).unwrap_err(), "SHP file load failed. File contains Arc, expected Polygon.");
    }

    #[test]
    fn vertices_closer_than_five_metres_are_dropped() {
        let base = (47.0, 8.0);
        let near = (47.00001, 8.0);
        let far = (47.001, 8.0);
        let farther = (47.002, 8.0);
        assert_eq!(filtered(vec![base, near, far, farther, near], true), vec![base, far, farther, near], "SHPFileHelper drops only consecutive close vertices, not a distinct last one near the first");
        assert_eq!(filtered(vec![base, far, farther, (47.0015, 8.001), base], true), vec![base, far, farther, (47.0015, 8.001)], "an exact closing vertex is removed");
        assert_eq!(filtered(vec![base, near, far], false), vec![base, far]);
    }
}
