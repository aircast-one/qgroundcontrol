use serde_json::{Value, json};

use crate::dropsim::Point;
use crate::router::Backend;

pub const ZONE_SETTING: &str = "settings.appSettings.autodropDropZone";
pub const REQUIRED_SETTING: &str = "settings.appSettings.autodropDropZoneRequired";
pub const ADD_CORNER: &str = "dropZone.addCorner";
pub const CLEAR: &str = "dropZone.clear";
pub const NO_ZONE: &str =
    "Draw a drop zone first: long-press the map and add at least three corners.";
pub const OUTSIDE: &str = "outsideDropZone";

pub fn parsed(text: &str) -> Vec<(f64, f64)> {
    serde_json::from_str::<Vec<(f64, f64)>>(text).unwrap_or_default()
}

pub fn corners() -> Vec<(f64, f64)> {
    crate::settingsstore::raw_setting(ZONE_SETTING)
        .and_then(|v| v.as_str().map(parsed))
        .unwrap_or_default()
}

pub fn contains(corners: &[(f64, f64)], point: Point) -> bool {
    let edges = corners.iter().zip(corners.iter().cycle().skip(1));
    edges
        .filter(|((lat_a, lon_a), (lat_b, lon_b))| {
            (lat_a > &point.lat) != (lat_b > &point.lat)
                && point.lon < lon_a + (point.lat - lat_a) / (lat_b - lat_a) * (lon_b - lon_a)
        })
        .count()
        % 2
        == 1
}

pub fn required(simulated: bool) -> bool {
    !simulated
        && crate::settingsstore::raw_setting(REQUIRED_SETTING)
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
}

pub fn refusal(
    corners: &[(f64, f64)],
    landing: Option<Point>,
    required: bool,
) -> Option<&'static str> {
    match (corners.len() >= 3, landing) {
        (false, _) => required.then_some(NO_ZONE),
        (true, Some(point)) if !contains(corners, point) => Some(OUTSIDE),
        _ => None,
    }
}

fn store(backend: &dyn Backend, corners: &[(f64, f64)]) -> Value {
    let written = serde_json::from_str::<Value>(
        &backend.set(ZONE_SETTING, &json!(json!(corners).to_string()).to_string()),
    )
    .unwrap_or(Value::Null);
    match written.get("ok").and_then(Value::as_bool) {
        Some(false) => json!({ "ok": false, "reason": "The drop zone could not be saved." }),
        _ => json!({ "ok": true, "corners": corners.len() }),
    }
}

pub fn add_corner(backend: &dyn Backend, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    match crate::fenceedit::point(given.get(0)) {
        Some(corner) => store(
            backend,
            &corners().into_iter().chain([corner]).collect::<Vec<_>>(),
        ),
        None => {
            json!({ "ok": false, "reason": "A drop zone corner needs a latitude and longitude." })
        }
    }
}

pub fn clear(backend: &dyn Backend) -> Value {
    store(backend, &[])
}

pub fn fence_polygon() -> Option<Value> {
    let zone = corners();
    (zone.len() >= 3).then(|| {
        json!({
            "inclusion": true,
            "dropZone": true,
            "kindText": "Drop zone",
            "vertices": zone.iter().map(|(lat, lon)| json!({ "latitude": lat, "longitude": lon })).collect::<Vec<_>>(),
            "count": zone.len(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: [(f64, f64); 4] = [(47.0, 8.0), (47.0, 8.01), (47.01, 8.01), (47.01, 8.0)];

    #[test]
    fn a_point_inside_the_zone_is_allowed_and_one_outside_is_held() {
        let inside = Point {
            lat: 47.005,
            lon: 8.005,
        };
        let outside = Point {
            lat: 47.02,
            lon: 8.005,
        };
        assert!(contains(&SQUARE, inside) && !contains(&SQUARE, outside));
        assert_eq!(refusal(&SQUARE, Some(inside), true), None);
        assert_eq!(refusal(&SQUARE, Some(outside), true), Some(OUTSIDE));
    }

    #[test]
    fn without_a_zone_a_real_vehicle_is_refused_and_a_simulated_one_is_not() {
        assert_eq!(refusal(&[], None, true), Some(NO_ZONE));
        assert_eq!(
            refusal(&[(47.0, 8.0), (47.0, 8.01)], None, true),
            Some(NO_ZONE)
        );
        assert_eq!(refusal(&[], None, false), None);
        assert!(!required(true), "a simulated vehicle never needs one");
    }

    #[test]
    fn the_stored_zone_reads_back_and_garbage_reads_as_no_zone() {
        assert_eq!(parsed("[[47.0,8.0],[47.0,8.01],[47.01,8.0]]").len(), 3);
        assert!(parsed("not json").is_empty());
    }
}
