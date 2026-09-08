use serde_json::{Value, json};

use crate::read::{Unit, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["plan.missionController.missionItemCount", "plan.missionController.containsItems", "plan.dirty", "vehicles.activeVehicleAvailable"];

const FIELDS: &str = "specifiesCoordinate,distanceFromStart,amslEntryAlt,terrainAltitude,terrainCollision,sequenceNumber";

#[derive(Debug, PartialEq, Clone)]
pub struct Point {
    pub sequence: i64,
    pub distance: f64,
    pub mission_altitude: f64,
    pub terrain_altitude: Option<f64>,
    pub collision: bool,
}

#[derive(Debug, PartialEq)]
pub struct Profile {
    pub points: Vec<Point>,
    pub min_altitude: f64,
    pub max_altitude: f64,
    pub total_distance: f64,
    pub unknown_terrain: usize,
}

pub fn profile(points: Vec<Point>) -> Profile {
    let unknown_terrain = points.iter().filter(|p| p.terrain_altitude.is_none()).count();
    let total_distance = points.iter().map(|p| p.distance).fold(0.0, f64::max);
    let altitudes: Vec<f64> = points.iter().map(|p| p.mission_altitude).chain(points.iter().filter_map(|p| p.terrain_altitude)).collect();
    let low = altitudes.iter().copied().fold(f64::INFINITY, f64::min);
    let high = altitudes.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let (low, high) = if altitudes.is_empty() { (0.0, 0.0) } else { (low, high) };
    let padding = ((high - low) * 0.2).max(5.0);
    Profile { points, min_altitude: low - padding, max_altitude: high + padding, total_distance, unknown_terrain }
}

pub fn points(model: &Value) -> Vec<Point> {
    model
        .get("elements")
        .and_then(Value::as_array)
        .map(|elements| {
            elements
                .iter()
                .filter(|e| e.get("specifiesCoordinate").and_then(Value::as_bool).unwrap_or(false))
                .filter_map(|e| {
                    let number = |key: &str| e.get(key).and_then(Value::as_f64).filter(|v| v.is_finite());
                    Some(Point {
                        sequence: e.get("sequenceNumber").and_then(Value::as_i64).unwrap_or(-1),
                        distance: number("distanceFromStart").unwrap_or(0.0),
                        mission_altitude: number("amslEntryAlt")?,
                        terrain_altitude: number("terrainAltitude"),
                        collision: e.get("terrainCollision").and_then(Value::as_bool).unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn terrain_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let model = object(&backend.get_fields("plan.missionController.visualItems", FIELDS));
    let profile = profile(points(&model));
    let horizontal = Unit::horizontal(backend);
    let vertical = Unit::vertical(backend);
    let usable = profile.points.len() > 1 && profile.max_altitude > profile.min_altitude;
    json!({
        "kind": "object",
        "class": "TerrainProfile",
        "usable": usable,
        "groundKnown": profile.unknown_terrain == 0 && profile.points.len() > 1,
        "hasCollision": profile.points.iter().any(|p| p.collision),
        "unknownTerrain": profile.unknown_terrain,
        "totalDistanceMeters": profile.total_distance,
        "minAltitudeMeters": profile.min_altitude,
        "maxAltitudeMeters": profile.max_altitude,
        "distanceText": format!("{:.0} {}", horizontal.show(profile.total_distance), horizontal.name),
        "lowestText": format!("{:.0} {}", vertical.show(profile.min_altitude), vertical.name),
        "highestText": format!("{:.0} {}", vertical.show(profile.max_altitude), vertical.name),
        "points": profile.points.iter().map(|p| json!({
            "sequence": p.sequence,
            "distance": p.distance,
            "x": if profile.total_distance > 0.0 { p.distance / profile.total_distance } else { 0.0 },
            "missionAltitude": p.mission_altitude,
            "terrainAltitude": p.terrain_altitude,
            "collision": p.collision,
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(distance: f64, mission: f64, terrain: Option<f64>) -> Point {
        Point { sequence: 1, distance, mission_altitude: mission, terrain_altitude: terrain, collision: false }
    }

    #[test]
    fn a_flat_mission_still_gets_a_band_to_draw_in() {
        let flat = profile(vec![point(0.0, 100.0, Some(50.0)), point(200.0, 100.0, Some(50.0))]);
        assert_eq!(flat.min_altitude, 40.0);
        assert_eq!(flat.max_altitude, 110.0);
        assert_eq!(flat.total_distance, 200.0);
        assert_eq!(flat.unknown_terrain, 0);
    }

    #[test]
    fn the_band_pads_twenty_percent_of_the_range_and_counts_unknown_ground() {
        let hilly = profile(vec![point(0.0, 100.0, None), point(500.0, 150.0, Some(20.0))]);
        assert_eq!(hilly.unknown_terrain, 1);
        assert_eq!(hilly.min_altitude, 20.0 - 26.0);
        assert_eq!(hilly.max_altitude, 150.0 + 26.0);
    }

    #[test]
    fn points_come_only_from_items_with_a_position_and_an_altitude() {
        let model = json!({ "elements": [
            { "specifiesCoordinate": false, "sequenceNumber": 0, "distanceFromStart": 0.0, "amslEntryAlt": 500.0 },
            { "specifiesCoordinate": true, "sequenceNumber": 1, "distanceFromStart": 0.0, "amslEntryAlt": 120.0, "terrainAltitude": 100.0, "terrainCollision": false },
            { "specifiesCoordinate": true, "sequenceNumber": 2, "distanceFromStart": 300.0, "amslEntryAlt": null, "terrainAltitude": 100.0 },
            { "specifiesCoordinate": true, "sequenceNumber": 3, "distanceFromStart": 600.0, "amslEntryAlt": 90.0, "terrainAltitude": 110.0, "terrainCollision": true },
        ] });
        let listed = points(&model);
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[1].sequence, 3);
        assert!(listed[1].collision);
    }
}
