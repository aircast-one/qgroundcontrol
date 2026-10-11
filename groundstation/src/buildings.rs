use geo_types::{Geometry, LineString, Polygon};
use mvt_reader::Reader;
use mvt_reader::feature::{Feature, Value as Property};
use serde_json::{Value, json};

const LAYER: &str = "building";
const HEIGHT_STEP: f64 = 10.0;

fn number(feature: &Feature, name: &str) -> Option<f64> {
    match feature.properties.as_ref()?.get(name)? {
        Property::Float(value) => Some(f64::from(*value)),
        Property::Double(value) => Some(*value),
        Property::Int(value) | Property::SInt(value) => Some(*value as f64),
        Property::UInt(value) => Some(*value as f64),
        _ => None,
    }
}

fn hidden(feature: &Feature) -> bool {
    matches!(feature.properties.as_ref().and_then(|properties| properties.get("hide_3d")), Some(Property::Bool(true)))
}

fn colour(feature: &Feature) -> Option<String> {
    match feature.properties.as_ref()?.get("colour")? {
        Property::String(colour) if !colour.is_empty() => Some(colour.clone()),
        _ => None,
    }
}

fn ring(line: &LineString<f32>) -> Vec<i32> {
    let points = line.0.len().saturating_sub(usize::from(line.is_closed()));
    line.0.iter().take(points).flat_map(|point| [point.x.round() as i32, point.y.round() as i32]).collect()
}

fn polygons(geometry: &Geometry<f32>) -> Vec<&Polygon<f32>> {
    match geometry {
        Geometry::Polygon(polygon) => vec![polygon],
        Geometry::MultiPolygon(many) => many.0.iter().collect(),
        _ => Vec::new(),
    }
}

fn rounded(metres: f64) -> f64 {
    (metres * HEIGHT_STEP).round() / HEIGHT_STEP
}

fn enclosing(ring: &[i32]) -> bool {
    ring.len() >= 6
}

fn building(feature: &Feature, polygon: &Polygon<f32>) -> Option<Value> {
    let height = number(feature, "render_height")?;
    let base = number(feature, "render_min_height").unwrap_or(0.0);
    let outline = Some(ring(polygon.exterior())).filter(|outline| enclosing(outline) && height > base)?;
    let rings: Vec<Vec<i32>> = std::iter::once(outline).chain(polygon.interiors().iter().map(ring).filter(|hole| enclosing(hole))).collect();
    let fields = [("rings", json!(rings)), ("height", json!(rounded(height))), ("base", json!(rounded(base)))].into_iter().chain(colour(feature).map(|colour| ("colour", json!(colour))));
    Some(Value::Object(fields.map(|(name, value)| (name.to_string(), value)).collect()))
}

pub fn tile_json(raw: &[u8]) -> Option<Vec<u8>> {
    let reader = Reader::new(raw.to_vec()).ok()?;
    let layers = reader.get_layer_metadata().ok()?;
    let found = layers.iter().find(|layer| layer.name == LAYER);
    let extent = found.map_or(4096, |layer| layer.extent);
    let features = found.map(|layer| reader.get_features(layer.layer_index).unwrap_or_default()).unwrap_or_default();
    let buildings: Vec<Value> = features
        .iter()
        .filter(|feature| !hidden(feature))
        .flat_map(|feature| polygons(&feature.geometry).into_iter().filter_map(move |polygon| building(feature, polygon)))
        .collect();
    serde_json::to_vec(&json!({ "extent": extent, "buildings": buildings })).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile() -> Vec<u8> {
        std::fs::read(format!("{}/../test/Bridge/fixtures/openfreemap-14-10240-6105.pbf", env!("CARGO_MANIFEST_DIR"))).expect("a recorded OpenFreeMap tile")
    }

    fn served(raw: &[u8]) -> Value {
        serde_json::from_slice(&tile_json(raw).expect("the tile decodes")).expect("the answer is JSON")
    }

    #[test]
    fn an_openfreemap_tile_becomes_footprints_with_heights_in_tile_units() {
        let shown = served(&tile());
        let buildings = shown["buildings"].as_array().expect("a building list");
        assert_eq!(shown["extent"], 4096);
        assert!(!buildings.is_empty(), "the recorded tile is a suburb of Tbilisi with houses in it");
        assert!(buildings.iter().all(|b| b["height"].as_f64().unwrap() > b["base"].as_f64().unwrap()), "every footprint stands up off its base");
        assert!(buildings.iter().all(|b| b["rings"][0].as_array().unwrap().len() >= 6), "an outline has at least three corners");
        assert!(
            buildings.iter().all(|b| b["rings"].as_array().unwrap().iter().all(|r| r.as_array().unwrap().len() % 2 == 0)),
            "rings are flat x,y pairs"
        );
    }

    #[test]
    fn a_closing_corner_is_dropped_so_the_outline_does_not_repeat_its_start() {
        let square = LineString::from(vec![(0.0f32, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0), (0.0, 0.0)]);
        assert_eq!(ring(&square), vec![0, 0, 10, 0, 10, 10, 0, 10]);
    }

    #[test]
    fn something_that_is_not_a_vector_tile_serves_nothing_and_a_tile_without_buildings_serves_an_empty_list() {
        assert_eq!(tile_json(b"\x89PNG\r\n\x1a\nnot a vector tile"), None);
        assert_eq!(served(&[])["buildings"], json!([]));
    }
}
