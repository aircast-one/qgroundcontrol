use serde_json::{Value, json};
use shapefile::{PolygonRing, Shape};

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
        return Err("Only WGS84 or UTM projections are supported.".to_string());
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

pub fn filtered(points: Vec<(f64, f64)>, closed: bool) -> Vec<(f64, f64)> {
    let trimmed = match (closed, points.first().copied()) {
        (true, Some(first)) => {
            let mut keep = points.len();
            while keep > 3 && metres_between(points[keep - 1], first) < VERTEX_FILTER_METRES {
                keep -= 1;
            }
            points.into_iter().take(keep).collect::<Vec<_>>()
        }
        _ => points,
    };
    let Some((last, body)) = trimmed.split_last() else { return trimmed };
    let mut kept: Vec<(f64, f64)> = body.iter().fold(Vec::new(), |mut kept, p| {
        if kept.last().map(|k| metres_between(*k, *p) >= VERTEX_FILTER_METRES).unwrap_or(true) {
            kept.push(*p);
        }
        kept
    });
    kept.push(*last);
    kept
}

pub fn parse(shp_path: &str) -> Result<(String, usize, Vec<(f64, f64)>), String> {
    if !shp_path.to_lowercase().ends_with(".shp") {
        return Err(format!("File is not a .shp file: {shp_path}"));
    }
    let prj_path = format!("{}.prj", &shp_path[..shp_path.len() - 4]);
    let prj = std::fs::read_to_string(&prj_path).map_err(|_| format!("File not found: {prj_path}"))?;
    let projection = projection(&prj)?;
    let shapes = shapefile::read_shapes(shp_path).map_err(|e| format!("SHPOpen failed: {e}"))?;
    let entities = shapes.len();
    let Some(first) = shapes.into_iter().next() else { return Err("Failed to read polygon object.".to_string()) };
    match first {
        Shape::Polygon(polygon) => {
            let rings: Vec<&PolygonRing<shapefile::Point>> = polygon.rings().iter().collect();
            if rings.len() != 1 {
                return Err("Only single part polygons are supported.".to_string());
            }
            let points: Vec<(f64, f64)> = rings[0].points().iter().map(|p| to_geo(&projection, p.x, p.y)).collect();
            Ok(("polygon".to_string(), entities, filtered(points, true)))
        }
        Shape::Polyline(line) => {
            if line.parts().len() != 1 {
                return Err("Only single part polylines are supported.".to_string());
            }
            let points: Vec<(f64, f64)> = line.parts()[0].iter().map(|p| to_geo(&projection, p.x, p.y)).collect();
            Ok(("polyline".to_string(), entities, filtered(points, false)))
        }
        _ => Err("No supported types found.".to_string()),
    }
}

pub fn shp_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return json!({ "kind": "null" }) };
    match parse(path) {
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
        format!("{}/../test/Utilities/Shape/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn the_projection_line_is_recognised_or_refused() {
        assert_eq!(projection("GEOGCS[\"GCS_WGS_1984\",DATUM[...]]").unwrap(), Projection::Wgs84);
        assert_eq!(projection("PROJCS[\"WGS_1984_UTM_Zone_33N\",GEOGCS[...]]").unwrap(), Projection::Utm { zone: 33, southern: false });
        assert_eq!(projection("PROJCS[\"WGS_1984_UTM_Zone_5S\"").unwrap(), Projection::Utm { zone: 5, southern: true });
        assert!(projection("PROJCS[\"WGS_1984_UTM_Zone_99N\"").unwrap_err().starts_with("UTM projection is not"));
        assert_eq!(projection("PROJCS[\"Something_Else\"").unwrap_err(), "Only WGS84 or UTM projections are supported.");
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
        assert!(parse(&fixture("polygon.kml")).unwrap_err().starts_with("File is not a .shp file"));
        assert!(parse(&fixture("missing.shp")).unwrap_err().starts_with("File not found"));
    }

    #[test]
    fn vertices_closer_than_five_metres_are_dropped() {
        let base = (47.0, 8.0);
        let near = (47.00001, 8.0);
        let far = (47.001, 8.0);
        let farther = (47.002, 8.0);
        assert_eq!(filtered(vec![base, near, far, farther, near], true), vec![base, far, farther]);
        assert_eq!(filtered(vec![base, near, far], false), vec![base, far]);
    }
}
