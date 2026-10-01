use serde_json::{Value, json};

use crate::surveygrid::{at_distance_and_azimuth, distance_between};

type Point = (f64, f64);

const INSET: f64 = 0.75;
const LARGEST_HALF_METRES: f64 = 1500.0;
const FEWEST_VERTICES: usize = 3;
pub const RALLY_DEFAULT_ALTITUDE: f64 = 0.0;

fn window(top_left: Point, bottom_right: Point) -> (Point, f64, f64) {
    let top_right = (top_left.0, bottom_right.1);
    let bottom_left = (bottom_right.0, top_left.1);
    let half_width = distance_between(top_left, top_right) / 2.0;
    let half_height = distance_between(top_left, bottom_left) / 2.0;
    let left_edge = at_distance_and_azimuth(top_left, half_height, 180.0);
    let top_edge = at_distance_and_azimuth(top_left, half_width, 90.0);
    ((left_edge.0, top_edge.1), half_width, half_height)
}

fn list(section: &Value, key: &str) -> Vec<Value> {
    section.get(key).and_then(Value::as_array).cloned().unwrap_or_default()
}

fn with(section: &Value, key: &str, values: Vec<Value>) -> Value {
    let mut changed = section.clone();
    changed[key] = Value::Array(values);
    changed
}

fn replaced_at(values: &[Value], index: usize, change: impl FnOnce(&Value) -> Option<Value>) -> Option<Vec<Value>> {
    let new = change(values.get(index)?)?;
    Some(values.iter().enumerate().map(|(i, v)| if i == index { new.clone() } else { v.clone() }).collect())
}

fn without(values: &[Value], index: usize) -> Option<Vec<Value>> {
    (index < values.len()).then(|| values.iter().enumerate().filter(|(i, _)| *i != index).map(|(_, v)| v.clone()).collect())
}

pub fn add_polygon(fence: &Value, top_left: Point, bottom_right: Point) -> Value {
    let (centre, half_width, half_height) = window(top_left, bottom_right);
    let (w, h) = ((half_width * INSET).min(LARGEST_HALF_METRES), (half_height * INSET).min(LARGEST_HALF_METRES));
    let corner = |across: f64, up: f64| at_distance_and_azimuth(at_distance_and_azimuth(centre, w, across), h, up);
    let vertices = [corner(-90.0, 0.0), corner(90.0, 0.0), corner(90.0, 180.0), corner(-90.0, 180.0)];
    let polygon = json!({ "inclusion": true, "polygon": vertices.iter().map(|(lat, lon)| json!([lat, lon])).collect::<Vec<_>>(), "version": 1 });
    with(fence, "polygons", list(fence, "polygons").into_iter().chain(std::iter::once(polygon)).collect())
}

pub fn add_circle(fence: &Value, top_left: Point, bottom_right: Point) -> Value {
    let (centre, half_width, half_height) = window(top_left, bottom_right);
    let radius = (half_width.min(half_height) * INSET).min(LARGEST_HALF_METRES);
    let circle = json!({ "inclusion": true, "circle": { "center": [centre.0, centre.1], "radius": radius }, "version": 1 });
    with(fence, "circles", list(fence, "circles").into_iter().chain(std::iter::once(circle)).collect())
}

pub fn delete(fence: &Value, key: &str, index: usize) -> Option<Value> {
    without(&list(fence, key), index).map(|kept| with(fence, key, kept))
}

pub fn set_inclusion(fence: &Value, key: &str, index: usize, inclusion: bool) -> Option<Value> {
    replaced_at(&list(fence, key), index, |shape| {
        let mut changed = shape.clone();
        changed["inclusion"] = json!(inclusion);
        Some(changed)
    })
    .map(|shapes| with(fence, key, shapes))
}

pub fn move_vertex(fence: &Value, polygon: usize, vertex: usize, at: Point) -> Option<Value> {
    replaced_at(&list(fence, "polygons"), polygon, |shape| {
        let vertices = replaced_at(&list(shape, "polygon"), vertex, |_| Some(json!([at.0, at.1])))?;
        Some(with(shape, "polygon", vertices))
    })
    .map(|polygons| with(fence, "polygons", polygons))
}

pub fn remove_vertex(fence: &Value, polygon: usize, vertex: usize) -> Option<Value> {
    replaced_at(&list(fence, "polygons"), polygon, |shape| {
        let vertices = list(shape, "polygon");
        (vertices.len() > FEWEST_VERTICES).then_some(())?;
        Some(with(shape, "polygon", without(&vertices, vertex)?))
    })
    .map(|polygons| with(fence, "polygons", polygons))
}

pub fn set_circle(fence: &Value, index: usize, centre: Option<Point>, radius: Option<f64>) -> Option<Value> {
    replaced_at(&list(fence, "circles"), index, |shape| {
        let mut changed = shape.clone();
        if let Some((lat, lon)) = centre {
            changed["circle"]["center"] = json!([lat, lon]);
        }
        if let Some(r) = radius {
            changed["circle"]["radius"] = json!(r);
        }
        Some(changed)
    })
    .map(|circles| with(fence, "circles", circles))
}

pub fn set_breach_return(fence: &Value, at: Point, altitude: Option<f64>) -> Value {
    let mut changed = fence.clone();
    changed["breachReturn"] = json!([at.0, at.1, altitude]);
    changed
}

pub fn clear_breach_return(fence: &Value) -> Value {
    let mut changed = fence.clone();
    if let Value::Object(map) = &mut changed {
        map.remove("breachReturn");
    }
    changed
}

pub fn set_breach_altitude(fence: &Value, altitude: f64) -> Option<Value> {
    let back = fence.get("breachReturn").and_then(Value::as_array).filter(|b| b.len() >= 2)?;
    let mut changed = fence.clone();
    changed["breachReturn"] = json!([back[0], back[1], altitude]);
    Some(changed)
}

pub fn add_rally(rally: &Value, at: Point, fixed_wing: bool, mission_altitude: f64) -> Value {
    let points = list(rally, "points");
    let altitude = points.last().and_then(|last| last.get(2)).and_then(Value::as_f64).unwrap_or(if fixed_wing { mission_altitude } else { RALLY_DEFAULT_ALTITUDE });
    with(rally, "points", points.into_iter().chain(std::iter::once(json!([at.0, at.1, altitude]))).collect())
}

pub fn remove_rally(rally: &Value, index: usize) -> Option<Value> {
    without(&list(rally, "points"), index).map(|kept| with(rally, "points", kept))
}

pub fn move_rally(rally: &Value, index: usize, at: Point, altitude: Option<f64>) -> Option<Value> {
    replaced_at(&list(rally, "points"), index, |point| {
        let kept = altitude.or_else(|| point.get(2).and_then(Value::as_f64));
        Some(json!([at.0, at.1, kept]))
    })
    .map(|points| with(rally, "points", points))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_breach_return_point_is_set_raised_and_removed() {
        let fence = json!({ "version": 2, "polygons": [], "circles": [] });
        let set = set_breach_return(&fence, (47.4, 8.5), Some(30.0));
        assert_eq!(set_breach_altitude(&set, 45.0).unwrap()["breachReturn"], json!([47.4, 8.5, 45.0]));
        assert!(set_breach_altitude(&fence, 45.0).is_none(), "no point, no altitude to set");
        assert!(clear_breach_return(&set).get("breachReturn").is_none());
    }

    fn empty_fence() -> Value {
        json!({ "circles": [], "polygons": [], "version": 2 })
    }

    #[test]
    fn a_new_polygon_takes_three_quarters_of_the_window() {
        let fence = add_polygon(&empty_fence(), (-35.36, 149.16), (-35.37, 149.17));
        let vertices = fence["polygons"][0]["polygon"].as_array().unwrap().clone();
        let point = |v: &Value| (v[0].as_f64().unwrap(), v[1].as_f64().unwrap());
        let (tl, tr, bl) = (point(&vertices[0]), point(&vertices[1]), point(&vertices[3]));
        let (window_width, window_height) = (distance_between((-35.36, 149.16), (-35.36, 149.17)), distance_between((-35.36, 149.16), (-35.37, 149.16)));
        assert!((distance_between(tl, tr) / window_width - 0.75).abs() < 1e-3);
        assert!((distance_between(tl, bl) / window_height - 0.75).abs() < 1e-3);
        assert_eq!(fence["polygons"][0]["inclusion"], true);
    }

    #[test]
    fn a_wide_window_caps_the_new_shapes_at_fifteen_hundred_metres_a_side() {
        let circle = add_circle(&empty_fence(), (-35.0, 149.0), (-36.0, 150.0));
        assert_eq!(circle["circles"][0]["circle"]["radius"], 1500.0);
    }

    #[test]
    fn a_polygon_keeps_three_vertices() {
        let fence = add_polygon(&empty_fence(), (-35.36, 149.16), (-35.37, 149.17));
        let three = remove_vertex(&fence, 0, 0).unwrap();
        assert_eq!(three["polygons"][0]["polygon"].as_array().unwrap().len(), 3);
        assert!(remove_vertex(&three, 0, 0).is_none());
    }

    #[test]
    fn a_rally_point_climbs_to_the_last_one_or_to_the_airframe_default() {
        let rally = json!({ "points": [], "version": 2 });
        let copter = add_rally(&rally, (1.0, 2.0), false, 75.0);
        assert_eq!(copter["points"][0][2], 0.0);
        assert_eq!(add_rally(&rally, (1.0, 2.0), true, 75.0)["points"][0][2], 75.0, "a plane's first rally point takes the mission altitude");
        let second = add_rally(&move_rally(&copter, 0, (1.0, 2.0), Some(40.0)).unwrap(), (3.0, 4.0), false, 75.0);
        assert_eq!(second["points"][1][2], 40.0, "a later point copies the one before");
    }
}
