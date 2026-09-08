use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["plan.geoFenceController.polygons", "plan.geoFenceController.circles", "plan.rallyPointController.points"];
const METRES_PER_DEGREE: f64 = 111_320.0;

fn point(json: &Value) -> Option<(f64, f64)> {
    let lat = json.get("latitude")?.as_f64().filter(|v| v.is_finite())?;
    let lon = json.get("longitude")?.as_f64().filter(|v| v.is_finite())?;
    Some((lat, lon))
}

fn points(json: Option<&Value>) -> Vec<(f64, f64)> {
    json.and_then(Value::as_array).map(|a| a.iter().filter_map(point).collect()).unwrap_or_default()
}

pub fn area_text(area: f64) -> String {
    match area >= 10000.0 {
        true => format!("{:.2} km\u{b2}", area / 1_000_000.0),
        false => format!("{area:.0} m\u{b2}"),
    }
}

fn elements(backend: &dyn Backend, path: &str) -> Vec<Value> {
    object(&backend.get(path)).get("elements").and_then(Value::as_array).cloned().unwrap_or_default()
}

fn radius_fact(json: &Value) -> (f64, String) {
    json.get("facts")
        .and_then(Value::as_array)
        .and_then(|f| f.iter().find(|x| x.get("name").and_then(Value::as_str) == Some("Radius")))
        .map(|f| (f.get("value").and_then(Value::as_f64).unwrap_or(0.0), f.get("units").and_then(Value::as_str).unwrap_or("m").to_string()))
        .unwrap_or((0.0, "m".to_string()))
}

fn polygon_json(index: usize, json: &Value) -> Value {
    let inclusion = json.get("inclusion").and_then(Value::as_bool).unwrap_or(true);
    let vertices = points(json.get("path"));
    let count = json.get("count").and_then(Value::as_i64).unwrap_or(vertices.len() as i64);
    let area = json.get("area").and_then(Value::as_f64).unwrap_or(0.0);
    let vertex_text = format!("{count} vertice{}", if count == 1 { "" } else { "s" });
    json!({
        "index": index,
        "path": format!("plan.geoFenceController.polygons.{index}"),
        "shape": "polygon",
        "inclusion": inclusion,
        "kindText": if inclusion { "Keep-in polygon" } else { "Keep-out polygon" },
        "detailText": if area > 0.0 { format!("{vertex_text} \u{b7} {}", area_text(area)) } else { vertex_text },
        "vertices": vertices.iter().map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon })).collect::<Vec<_>>(),
        "usable": vertices.len() >= 3,
        "framing": vertices.iter().map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon })).collect::<Vec<_>>(),
    })
}

fn circle_json(index: usize, json: &Value) -> Value {
    let inclusion = json.get("inclusion").and_then(Value::as_bool).unwrap_or(true);
    let centre = json.get("center").and_then(point);
    let (radius, units) = radius_fact(json);
    let framing: Vec<Value> = centre
        .map(|(lat, lon)| {
            let lat_span = radius / METRES_PER_DEGREE;
            let lon_span = radius / (METRES_PER_DEGREE * lat.to_radians().cos().max(0.01));
            vec![json!({ "latitude": lat - lat_span, "longitude": lon - lon_span }), json!({ "latitude": lat + lat_span, "longitude": lon + lon_span })]
        })
        .unwrap_or_default();
    json!({
        "index": index,
        "path": format!("plan.geoFenceController.circles.{index}"),
        "shape": "circle",
        "inclusion": inclusion,
        "kindText": if inclusion { "Keep-in circle" } else { "Keep-out circle" },
        "detailText": format!("{radius:.0} {units} radius"),
        "centre": centre.map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon })),
        "centreText": centre.map(|(lat, lon)| format!("{lat:.6}, {lon:.6}")).unwrap_or("\u{2014}".to_string()),
        "radius": radius,
        "radiusUnits": units,
        "usable": centre.is_some(),
        "framing": framing,
    })
}

pub fn fences_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let polygons: Vec<Value> = elements(backend, "plan.geoFenceController.polygons").iter().enumerate().map(|(i, p)| polygon_json(i, p)).collect();
    let circles: Vec<Value> = elements(backend, "plan.geoFenceController.circles").iter().enumerate().map(|(i, c)| circle_json(i, c)).collect();
    let rally: Vec<Value> = elements(backend, "plan.rallyPointController.points")
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let (lat, lon) = p.get("coordinate").and_then(point)?;
            let altitude = object(&backend.get(&format!("plan.rallyPointController.points.{i}.textFieldFacts.2")));
            let is_fact = altitude.get("kind").and_then(Value::as_str) == Some("fact");
            Some(json!({
                "index": i,
                "path": format!("plan.rallyPointController.points.{i}"),
                "latitude": lat,
                "longitude": lon,
                "altitude": if is_fact { altitude.get("value").cloned().unwrap_or(Value::Null) } else { Value::Null },
                "altitudeUnits": if is_fact { altitude.get("units").and_then(Value::as_str).unwrap_or("") } else { "" },
                "altitudePath": format!("plan.rallyPointController.points.{i}.textFieldFacts.2"),
            }))
        })
        .collect();
    json!({
        "kind": "object",
        "class": "Fences",
        "polygons": polygons,
        "circles": circles,
        "rallyPoints": rally,
        "count": polygons.len() + circles.len(),
    })
}

pub fn polygon_view(backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return json!({ "kind": "null" }) };
    let ring = args.get(1).map(|r| r != "line").unwrap_or(true);
    let json = object(&backend.get(path));
    if json.get("kind").and_then(Value::as_str) != Some("object") {
        return json!({ "kind": "null" });
    }
    let vertices = points(json.get("path"));
    let minimum = json.get("minVertexCount").and_then(Value::as_i64).map(|m| m as usize).unwrap_or(if ring { 3 } else { 2 });
    let closed = vertices.len() >= minimum;
    let segments = match (closed, ring) {
        (false, _) => 0,
        (true, true) => vertices.len(),
        (true, false) => vertices.len().saturating_sub(1),
    };
    let midpoints: Vec<Value> = (0..segments)
        .map(|i| {
            let (a, b) = (vertices[i], vertices[(i + 1) % vertices.len()]);
            json!({ "latitude": (a.0 + b.0) / 2.0, "longitude": (a.1 + b.1) / 2.0 })
        })
        .collect();
    json!({
        "kind": "object",
        "class": "EditablePolygon",
        "path": path,
        "ring": ring,
        "minimumVertices": minimum,
        "closed": closed,
        "canRemoveVertex": vertices.len() > minimum,
        "segments": segments,
        "splitInvokable": if ring { "splitPolygonSegment" } else { "splitSegment" },
        "adjustInvokable": "adjustVertex",
        "removeInvokable": "removeVertex",
        "vertices": vertices.iter().map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon })).collect::<Vec<_>>(),
        "midpoints": midpoints,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;
    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            match path {
                "plan.geoFenceController.polygons" => json!({ "kind": "object", "elements": [ { "inclusion": false, "count": 4, "area": 25000.0, "path": [ {"latitude": 1.0, "longitude": 1.0}, {"latitude": 1.0, "longitude": 2.0}, {"latitude": 2.0, "longitude": 2.0}, {"latitude": 2.0, "longitude": 1.0} ] } ] }),
                "plan.geoFenceController.circles" => json!({ "kind": "object", "elements": [ { "inclusion": true, "center": {"latitude": 47.0, "longitude": 8.0}, "facts": [ { "name": "Radius", "value": 150.0, "units": "m" } ] } ] }),
                "plan.rallyPointController.points" => json!({ "kind": "object", "elements": [ { "coordinate": {"latitude": 47.1, "longitude": 8.1} } ] }),
                "plan.rallyPointController.points.0.textFieldFacts.2" => json!({ "kind": "fact", "name": "RelativeAltitude", "value": 50.0, "units": "m" }),
                "poly" => json!({ "kind": "object", "path": [ {"latitude": 0.0, "longitude": 0.0}, {"latitude": 0.0, "longitude": 2.0}, {"latitude": 2.0, "longitude": 2.0} ] }),
                "line" => json!({ "kind": "object", "path": [ {"latitude": 0.0, "longitude": 0.0}, {"latitude": 0.0, "longitude": 2.0} ], "minVertexCount": 2 }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn fences_are_described_and_framed() {
        let view = fences_view(&Fake, &[]);
        assert_eq!(view["count"], 2);
        assert_eq!(view["polygons"][0]["kindText"], "Keep-out polygon");
        assert_eq!(view["polygons"][0]["detailText"], "4 vertices \u{b7} 0.03 km\u{b2}");
        assert_eq!(view["circles"][0]["detailText"], "150 m radius");
        assert_eq!(view["circles"][0]["centreText"], "47.000000, 8.000000");
        assert_eq!(view["circles"][0]["framing"].as_array().unwrap().len(), 2);
        assert_eq!(view["rallyPoints"][0]["path"], "plan.rallyPointController.points.0");
        assert_eq!((view["rallyPoints"][0]["altitude"].clone(), view["rallyPoints"][0]["altitudeUnits"].clone()), (json!(50.0), json!("m")));
        assert_eq!(area_text(9999.0), "9999 m\u{b2}");
    }

    #[test]
    fn a_ring_has_as_many_segments_as_vertices_and_a_line_one_fewer() {
        let ring = polygon_view(&Fake, &["poly".to_string()]);
        assert_eq!(ring["closed"], true);
        assert_eq!(ring["canRemoveVertex"], false);
        assert_eq!(ring["segments"], 3);
        assert_eq!(ring["midpoints"][2]["longitude"], 1.0);
        assert_eq!(ring["splitInvokable"], "splitPolygonSegment");
        let line = polygon_view(&Fake, &["line".to_string(), "line".to_string()]);
        assert_eq!(line["segments"], 1);
        assert_eq!(line["splitInvokable"], "splitSegment");
        assert_eq!(line["canRemoveVertex"], false);
        assert_eq!(polygon_view(&Fake, &["nope".to_string()])["kind"], "null");
    }
}
