use serde_json::{Value, json};

use crate::read::{flag, integer, object};
use crate::router::Backend;

// A survey, corridor or structure scan edits its own QGCMapPolygon or QGCMapPolyline, reached as
// plan.missionController.visualItems.<n>.<property>.adjustVertex. Neither class checks the index or
// the coordinate: a stale vertex index logs and does nothing while the bridge answers ok, and a
// coordinate off the globe is stored and every transect regenerated around it. The fence polygons
// have had this check since the core claimed them; this is the same one for a mission item's shape.
const SHAPE_ITEMS: &str = "plan.missionController.visualItems.";

pub fn target(path: &str) -> Option<(usize, &str)> {
    let rest = path.strip_prefix(SHAPE_ITEMS)?.strip_suffix(".adjustVertex")?;
    let (index, property) = rest.split_once('.')?;
    let simple = !property.is_empty() && property.bytes().all(|b| b.is_ascii_alphanumeric());
    simple.then_some(())?;
    Some((index.parse().ok()?, property))
}

pub fn owns(path: &str) -> bool {
    target(path).is_some()
}

pub fn adjust_vertex(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let refused = |token: &str, reason: String| json!({ "ok": false, "refusal": token, "reason": reason });
    let Some((index, property)) = target(path) else {
        return refused("malformed", "That is not a mission item shape the core edits.".to_string());
    };
    if flag(&object(&backend.get_fields("plan", "syncInProgress")), "syncInProgress") {
        return refused("busy", "Wait for the sync to finish before changing the plan.".to_string());
    }
    let shape = object(&backend.get_fields(&format!("{SHAPE_ITEMS}{index}.{property}"), "count"));
    if shape.get("kind").and_then(Value::as_str) != Some("object") {
        return refused("noSuchShape", format!("Item {index} has no {property} to edit."));
    }
    let vertices = integer(&shape, "count").unwrap_or(0);
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let Some(vertex) = given.get(0).and_then(Value::as_i64).filter(|v| (0..vertices).contains(v)) else {
        return refused("noSuchVertex", match vertices {
            0 => "The shape has no vertices yet.".to_string(),
            n => format!("The shape has vertices 0 to {}.", n - 1),
        });
    };
    let Some((latitude, longitude)) = crate::fenceedit::point(given.get(1)) else {
        return refused("badCoordinate", "A vertex needs a latitude from -90 to 90 and a longitude from -180 to 180.".to_string());
    };
    let forwarded = json!([vertex, { "latitude": latitude, "longitude": longitude }]);
    let dispatched = flag(&object(&backend.invoke(path, &forwarded.to_string())), "ok");
    json!({ "ok": dispatched, "refusal": Value::Null, "reason": match dispatched { true => Value::Null, false => json!("The shape was not changed.") } })
}

// Tapping a segment's midpoint on the map splits it through the invokable view.polygon names:
// splitPolygonSegment on a QGCMapPolygon, splitSegment on a QGCMapPolyline. Neither bounds-checks
// the index before indexing its path - QGCMapPolygon reads _polygonPath[vertexIndex] for any index,
// and QGCMapPolyline stops only an index past the end - so a segment index gone stale because the
// shape changed under the tap is an out-of-range QList read in the app process, not a refusal.
const SPLITS: [(&str, bool); 2] = [(".splitPolygonSegment", true), (".splitSegment", false)];
const SPLIT_ROOTS: [&str; 2] = ["plan.geoFenceController.polygons.", "plan.missionController.visualItems."];

pub fn split_target(path: &str) -> Option<(&str, bool)> {
    let (shape, ring) = SPLITS.iter().find_map(|(suffix, ring)| path.strip_suffix(suffix).map(|shape| (shape, *ring)))?;
    SPLIT_ROOTS.iter().any(|root| shape.starts_with(root)).then_some((shape, ring))
}

pub fn owns_split(path: &str) -> bool {
    split_target(path).is_some()
}

fn split_refusal(vertices: i64, ring: bool, segment: Option<i64>) -> Option<(&'static str, String)> {
    let segments = match ring {
        true if vertices >= 3 => vertices,
        false if vertices >= 2 => vertices - 1,
        _ => return Some(("tooFewVertices", "The shape has no segment to split yet.".to_string())),
    };
    match segment {
        Some(s) if (0..segments).contains(&s) => None,
        _ => Some(("noSuchSegment", format!("The shape has segments 0 to {}.", segments - 1))),
    }
}

pub fn split(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let refused = |token: &str, reason: String| json!({ "ok": false, "refusal": token, "reason": reason });
    let Some((shape_path, ring)) = split_target(path) else {
        return refused("malformed", "That is not a shape the core splits.".to_string());
    };
    if flag(&object(&backend.get_fields("plan", "syncInProgress")), "syncInProgress") {
        return refused("busy", "Wait for the sync to finish before changing the plan.".to_string());
    }
    let shape = object(&backend.get_fields(shape_path, "count"));
    if shape.get("kind").and_then(Value::as_str) != Some("object") {
        return refused("noSuchShape", format!("There is no shape at {shape_path}."));
    }
    let segment = serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0)?.as_i64());
    if let Some((token, reason)) = split_refusal(integer(&shape, "count").unwrap_or(0), ring, segment) {
        return refused(token, reason);
    }
    let dispatched = flag(&object(&backend.invoke(path, &json!([segment]).to_string())), "ok");
    json!({ "ok": dispatched, "refusal": Value::Null, "reason": match dispatched { true => Value::Null, false => json!("The shape was not split.") } })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Survey {
        syncing: bool,
        calls: RefCell<Vec<String>>,
    }

    impl Backend for Survey {
        fn get(&self, p: &str) -> String { self.get_fields(p, "") }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "plan" => json!({ "kind": "object", "syncInProgress": self.syncing }),
                "plan.missionController.visualItems.1.surveyAreaPolygon" => json!({ "kind": "object", "count": 4 }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, a: &str) -> String {
            self.calls.borrow_mut().push(a.to_string());
            json!({ "ok": true }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    const PATH: &str = "plan.missionController.visualItems.1.surveyAreaPolygon.adjustVertex";

    #[test]
    fn a_survey_vertex_moves_only_to_a_real_place_on_a_real_vertex() {
        assert_eq!(target(PATH), Some((1, "surveyAreaPolygon")));
        assert!(owns("plan.missionController.visualItems.2.corridorPolyline.adjustVertex"));
        assert!(!owns("plan.missionController.visualItems.x.surveyAreaPolygon.adjustVertex") && !owns("plan.missionController.visualItems.1.adjustVertex"));
        let survey = Survey { syncing: false, calls: RefCell::new(Vec::new()) };
        assert_eq!(adjust_vertex(&survey, PATH, r#"[2, {"latitude": 47.4, "longitude": 8.5}]"#)["ok"], true);
        assert_eq!(adjust_vertex(&survey, PATH, r#"[4, {"latitude": 47.4, "longitude": 8.5}]"#)["reason"], "The shape has vertices 0 to 3.", "QGCMapPolygon::adjustVertex logs a stale index and the bridge answered ok");
        assert_eq!(adjust_vertex(&survey, PATH, r#"[1, {"latitude": 147.4, "longitude": 8.5}]"#)["refusal"], "badCoordinate");
        assert_eq!(adjust_vertex(&survey, "plan.missionController.visualItems.3.surveyAreaPolygon.adjustVertex", r#"[0, {"latitude": 47.4, "longitude": 8.5}]"#)["refusal"], "noSuchShape");
        assert_eq!(survey.calls.borrow().as_slice(), &[r#"[2,{"latitude":47.4,"longitude":8.5}]"#.to_string()], "only the valid move reached Qt, stripped to what adjustVertex takes");
        let busy = Survey { syncing: true, calls: RefCell::new(Vec::new()) };
        assert_eq!(adjust_vertex(&busy, PATH, r#"[2, {"latitude": 47.4, "longitude": 8.5}]"#)["refusal"], "busy");
    }

    #[test]
    fn a_split_names_a_segment_the_shape_still_has() {
        assert_eq!(split_target("plan.geoFenceController.polygons.0.splitPolygonSegment"), Some(("plan.geoFenceController.polygons.0", true)));
        assert_eq!(split_target("plan.missionController.visualItems.2.corridorPolyline.splitSegment"), Some(("plan.missionController.visualItems.2.corridorPolyline", false)));
        assert!(!owns_split("vehicle.splitSegment"));
        assert_eq!(split_refusal(4, true, Some(3)), None, "a polygon's last segment closes back to vertex 0");
        assert_eq!(split_refusal(4, true, Some(4)).map(|r| r.0), Some("noSuchSegment"), "QGCMapPolygon reads _polygonPath[4] of four");
        assert_eq!(split_refusal(4, true, Some(-1)).map(|r| r.0), Some("noSuchSegment"));
        assert_eq!(split_refusal(3, false, Some(2)).map(|r| r.0), Some("noSuchSegment"), "a polyline of three has two segments");
        assert_eq!(split_refusal(3, false, Some(1)), None);
        assert_eq!(split_refusal(1, false, Some(0)).map(|r| r.0), Some("tooFewVertices"));
        assert_eq!(split_refusal(4, true, None).map(|r| r.0), Some("noSuchSegment"));

        let survey = Survey { syncing: false, calls: RefCell::new(Vec::new()) };
        assert_eq!(split(&survey, "plan.missionController.visualItems.1.surveyAreaPolygon.splitPolygonSegment", "[3]")["ok"], true);
        assert_eq!(split(&survey, "plan.missionController.visualItems.1.surveyAreaPolygon.splitPolygonSegment", "[7]")["refusal"], "noSuchSegment");
        assert_eq!(survey.calls.borrow().as_slice(), &["[3]".to_string()]);
    }
}
