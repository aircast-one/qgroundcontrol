use serde_json::{Value, json};

use crate::read::{Unit, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["plan.missionController.missionItemCount", "plan.missionController.containsItems", "plan.dirty", "vehicles.activeVehicleAvailable", "plan.missionController@recalcTerrainProfile"];

const FIELDS: &str = "specifiesCoordinate,specifiesAltitudeOnly,altitudeMode,distanceFromStart,amslEntryAlt,terrainAltitude,terrainCollision,sequenceNumber,complexDistance";

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
    pub min_clearance: Option<f64>,
}

pub fn profile(points: Vec<Point>) -> Profile {
    let unknown_terrain = points.iter().filter(|p| p.terrain_altitude.is_none()).count();
    let total_distance = points.iter().map(|p| p.distance).fold(0.0, f64::max);
    let altitudes: Vec<f64> = points.iter().map(|p| p.mission_altitude).chain(points.iter().filter_map(|p| p.terrain_altitude)).collect();
    let low = altitudes.iter().copied().fold(f64::INFINITY, f64::min);
    let high = altitudes.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let (low, high) = if altitudes.is_empty() { (0.0, 0.0) } else { (low, high) };
    let padding = ((high - low) * 0.2).max(5.0);
    let min_clearance = points
        .iter()
        .filter_map(|p| p.terrain_altitude.map(|ground| p.mission_altitude - ground))
        .fold(None, |worst: Option<f64>, clearance| Some(worst.map_or(clearance, |worst| worst.min(clearance))));
    Profile { points, min_altitude: low - padding, max_altitude: high + padding, total_distance, unknown_terrain, min_clearance }
}

// Counting unknown points is not the same rule: an empty profile has none, so it reads complete
// while carrying no figure at all. Completeness is a claim about a number, so there has to be one.
pub fn clearance_complete(profile: &Profile) -> bool {
    profile.min_clearance.is_some() && profile.unknown_terrain == 0
}

const TERRAIN_FRAME: i64 = 4;

fn drawable(item: &Value) -> bool {
    let flag = |key: &str| item.get(key).and_then(Value::as_bool).unwrap_or(false);
    if flag("specifiesCoordinate") {
        return true;
    }
    if !flag("specifiesAltitudeOnly") {
        return false;
    }
    let terrain_framed = item.get("altitudeMode").and_then(Value::as_i64) == Some(TERRAIN_FRAME);
    let ground_known = item.get("terrainAltitude").and_then(Value::as_f64).is_some_and(f64::is_finite);
    !terrain_framed || ground_known
}

pub fn points(model: &Value) -> Vec<Point> {
    model
        .get("elements")
        .and_then(Value::as_array)
        .map(|elements| {
            elements
                .iter()
                .filter(|e| drawable(e))
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

// A pattern is one point on the profile unless its own path is walked. A survey covering a
// kilometre of ground between its entry and its exit draws as a flat line across the hill it is
// flying over, and the operator reads no collision because nothing sampled the middle.
//
// The length of a segment is totalDistance. distanceBetween is the spacing between terrain
// samples along it, and it is zero until a terrain query answers - reading it as the length
// collapses the whole pattern onto its entry point wherever terrain is unknown.
pub fn along_segments(backend: &dyn Backend, index: usize, sequence: i64, start: f64) -> Vec<Point> {
    let segments = object(&backend.get_fields(&format!("plan.missionController.visualItems.{index}.flightPathSegments"), "coord1AMSLAlt,coord2AMSLAlt,amslTerrainHeights,totalDistance,distanceBetween,terrainCollision"));
    let Some(listed) = segments.get("elements").and_then(Value::as_array) else { return Vec::new() };
    listed
        .iter()
        .scan(start, |walked, segment| {
            let number = |key: &str| segment.get(key).and_then(Value::as_f64).filter(|value| value.is_finite());
            let length = number("totalDistance").unwrap_or(0.0);
            let spacing = number("distanceBetween").unwrap_or(0.0);
            let from = *walked;
            *walked += length;
            let (low, high) = (number("coord1AMSLAlt"), number("coord2AMSLAlt"));
            let collision = segment.get("terrainCollision").and_then(Value::as_bool).unwrap_or(false);
            let heights: Vec<Option<f64>> = segment
                .get("amslTerrainHeights")
                .and_then(Value::as_array)
                .map(|heights| heights.iter().map(|h| h.as_f64().filter(|v| v.is_finite())).collect())
                .unwrap_or_default();
            Some((from, length, spacing, low, high, collision, heights))
        })
        .flat_map(|(from, length, spacing, low, high, collision, heights)| {
            let (Some(low), Some(high)) = (low, high) else { return Vec::new() };
            let steps = heights.len().max(2);
            (0..steps)
                .map(|step| {
                    let along = sample_at(step, steps, length, spacing);
                    Point {
                        sequence,
                        distance: from + along,
                        mission_altitude: low + (high - low) * if length > 0.0 { along / length } else { 0.0 },
                        terrain_altitude: heights.get(step).copied().flatten(),
                        collision,
                    }
                })
                .collect()
        })
        .collect()
}

// TerrainTileManager::_pathQueryToCoords interpolates evenly spaced coordinates along the segment
// and then overwrites the last one with the endpoint, so the samples sit at i * distanceBetween
// and the final one at the segment's full length. Spreading them evenly across the length instead
// shears every reading towards the end, which is where a landing approach reads its clearance.
fn sample_at(step: usize, steps: usize, length: f64, spacing: f64) -> f64 {
    match (step + 1 == steps, spacing > 0.0) {
        (true, _) => length,
        (false, true) => (step as f64 * spacing).min(length),
        (false, false) => length * step as f64 / (steps as f64 - 1.0),
    }
}

fn walked(backend: &dyn Backend, model: &Value) -> Vec<Point> {
    let Some(elements) = model.get("elements").and_then(Value::as_array) else { return Vec::new() };
    elements
        .iter()
        .enumerate()
        .flat_map(|(index, element)| {
            let number = |key: &str| element.get(key).and_then(Value::as_f64).filter(|value| value.is_finite());
            let sequence = element.get("sequenceNumber").and_then(Value::as_i64).unwrap_or(-1);
            let start = number("distanceFromStart").unwrap_or(0.0);
            match number("complexDistance").filter(|metres| *metres > 0.0) {
                Some(_) => along_segments(backend, index, sequence, start),
                None => Vec::new(),
            }
        })
        .collect()
}

pub fn terrain_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let model = object(&backend.get_fields("plan.missionController.visualItems", FIELDS));
    let entries = points(&model);
    let inside = walked(backend, &model);
    let mut all: Vec<Point> = entries.into_iter().chain(inside).collect();
    all.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
    let profile = profile(all);
    let vertical = Unit::vertical(backend);
    let usable = profile.points.len() > 1 && profile.max_altitude > profile.min_altitude;
    json!({
        "kind": "object",
        "class": "TerrainProfile",
        "usable": usable,
        "groundKnown": profile.unknown_terrain == 0 && profile.points.len() > 1,
        "hasCollision": profile.points.iter().any(|p| p.collision),
        "minClearanceMetres": profile.min_clearance,
        "clearanceComplete": clearance_complete(&profile),
        "clearanceText": profile.min_clearance.map(|clearance| crate::read::format_measure(vertical.show(clearance.abs()), &vertical.name)),
        "unknownTerrain": profile.unknown_terrain,
        "totalDistanceMeters": profile.total_distance,
        "minAltitudeMeters": profile.min_altitude,
        "maxAltitudeMeters": profile.max_altitude,
        "distanceText": crate::missionsummary::distance_text(profile.total_distance, crate::missionsummary::imperial(backend)),
        "lowestText": crate::read::format_measure(vertical.show(profile.min_altitude), &vertical.name),
        "highestText": crate::read::format_measure(vertical.show(profile.max_altitude), &vertical.name),
        "bandText": crate::read::range_text(profile.min_altitude, profile.max_altitude, &vertical),
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
    fn the_profile_says_how_far_below_the_ground_it_runs_and_not_only_that_it_does() {
        let below = profile(vec![point(0.0, 700.0, Some(600.0)), point(100.0, 500.0, Some(668.0)), point(200.0, 700.0, Some(650.0))]);
        assert_eq!(below.min_clearance, Some(-168.0), "having been told the mission is below terrain the operator has to pick a new altitude, and the worst deficit is the number that choice is made from");

        let clear = profile(vec![point(0.0, 700.0, Some(600.0)), point(100.0, 700.0, Some(690.0))]);
        assert_eq!(clear.min_clearance, Some(10.0), "the same field answers how much room is left when there is room, so a head draws one number rather than two");

        let unknown = profile(vec![point(0.0, 700.0, None), point(100.0, 700.0, None)]);
        assert_eq!(unknown.min_clearance, None, "no ground under any sample is not a clearance of zero, which would read as touching");

        let partial = profile(vec![point(0.0, 700.0, None), point(100.0, 700.0, Some(720.0))]);
        assert_eq!(partial.min_clearance, Some(-20.0), "one sample with ground under it is enough to know the mission is below it somewhere");
    }

    #[test]
    fn a_clearance_measured_over_missing_ground_is_a_bound_and_the_core_says_so() {
        let complete = profile(vec![point(0.0, 700.0, Some(600.0)), point(100.0, 700.0, Some(690.0))]);
        assert_eq!(complete.unknown_terrain, 0);
        assert_eq!(complete.min_clearance, Some(10.0));

        // The smallest clearance measured is not the smallest there is when part of the route has
        // no ground under it, and a reassuring figure there is worse than none. Every head would
        // otherwise have to rediscover that from unknownTerrain, and one of them would not.
        let partial = profile(vec![point(0.0, 700.0, Some(600.0)), point(100.0, 700.0, None), point(200.0, 700.0, Some(690.0))]);
        assert_eq!(partial.min_clearance, Some(10.0), "the samples that do have ground still measure, so the number is a bound rather than nothing");
        assert_eq!(partial.unknown_terrain, 1);

        // Counting unknown points calls an empty profile complete, because there are no points to
        // be unknown about. There is no figure, so there is nothing for completeness to be true of,
        // and a head that reads "complete" and finds no number renders whatever its empty branch
        // does - which for the macOS panel was the wording for a collision.
        assert_eq!(clearance_complete(&complete), true);
        assert_eq!(clearance_complete(&partial), false);

        let nothing = profile(Vec::new());
        assert_eq!(nothing.unknown_terrain, 0, "an empty profile has no unknown points, which is why the count is the wrong instrument");
        assert_eq!(nothing.min_clearance, None);
        assert_eq!(clearance_complete(&nothing), false, "there is no figure, so there is nothing for completeness to be true of");

        let no_ground = profile(vec![point(0.0, 700.0, None), point(100.0, 700.0, None)]);
        assert_eq!(no_ground.min_clearance, None, "ground under nothing is not a clearance of zero");
        assert_eq!(clearance_complete(&no_ground), false);
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

    #[test]
    fn a_takeoff_that_states_only_a_height_is_still_a_height_the_profile_draws() {
        let model = json!({ "elements": [
            { "specifiesCoordinate": true, "sequenceNumber": 0, "distanceFromStart": 0.0, "amslEntryAlt": 100.0, "terrainAltitude": 100.0 },
            { "specifiesCoordinate": false, "specifiesAltitudeOnly": true, "sequenceNumber": 1, "distanceFromStart": 0.0, "amslEntryAlt": 150.0, "terrainAltitude": 100.0 },
            { "specifiesCoordinate": true, "sequenceNumber": 2, "distanceFromStart": 500.0, "amslEntryAlt": 150.0, "terrainAltitude": 140.0 },
        ] });
        let listed = points(&model);
        assert_eq!(listed.iter().map(|p| p.sequence).collect::<Vec<_>>(), vec![0, 1, 2], "ArduPilot declares MAV_CMD_NAV_TAKEOFF specifiesCoordinate false and specifiesAltitudeOnly true, so filtering on a coordinate alone drops the climb from the profile on every APM plan");
        assert_eq!(listed[1].distance, 0.0, "the climb covers no ground, so it shares the launch point's place on the axis rather than being given width it does not have");
        assert_eq!(listed[1].mission_altitude, 150.0);

        let terrain_framed = json!({ "elements": [
            { "specifiesCoordinate": true, "sequenceNumber": 0, "distanceFromStart": 0.0, "amslEntryAlt": 585.0, "terrainAltitude": 585.0 },
            { "specifiesCoordinate": false, "specifiesAltitudeOnly": true, "altitudeMode": 4, "sequenceNumber": 1, "distanceFromStart": 0.0, "amslEntryAlt": 50.0 },
            { "specifiesCoordinate": true, "sequenceNumber": 2, "distanceFromStart": 500.0, "amslEntryAlt": 660.0, "terrainAltitude": 640.0 },
        ] });
        let drawn = points(&terrain_framed);
        assert_eq!(drawn.iter().map(|p| p.sequence).collect::<Vec<_>>(), vec![0, 2], "a terrain-framed altitude is param7 plus the ground under the item, and an item with no place has no ground to query - so its amslEntryAlt is the bare relative height and putting it on an AMSL axis drags the band down by the height of the hill");

        let at_launch: Vec<f64> = listed.iter().filter(|p| p.distance == 0.0).map(|p| p.mission_altitude).collect();
        assert_eq!(at_launch, vec![100.0, 150.0], "two heights at one place is what a vertical climb is; without the second the head interpolates from the ground straight to the first waypoint and draws a diagonal the aircraft never flies");
    }
}

#[cfg(test)]
mod walking {
    use super::*;

    struct Route(Value);

    impl Backend for Route {
        fn get(&self, _p: &str) -> String { String::new() }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "plan.missionController.visualItems" => self.0.to_string(),
                _ => json!({ "kind": "object", "appSettingsVerticalDistanceUnitsString": "m", "appSettingsHorizontalDistanceUnitsString": "m" }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true, "result": 1.0 }).to_string() }
        fn watch(&self, _p: &[String]) {}
    }

    fn leg(distance: f64, mission: f64, ground: f64, collision: bool) -> Value {
        json!({ "specifiesCoordinate": true, "distanceFromStart": distance, "amslEntryAlt": mission, "terrainAltitude": ground, "terrainCollision": collision, "sequenceNumber": 1 })
    }

    #[test]
    fn a_route_that_flies_into_the_ground_says_so_and_one_that_does_not_says_that() {
        // hasCollision is the one field on this view an operator is expected to act on, and
        // nothing asserted it in either direction: pinning it to true and to false both passed.
        let hits = terrain_view(&Route(json!({ "kind": "object", "elements": [leg(0.0, 700.0, 600.0, false), leg(100.0, 500.0, 650.0, true)] })), &[]);
        assert_eq!(hits["hasCollision"], true);
        assert_eq!(hits["minClearanceMetres"], -150.0, "the depth comes from the samples, not from the flag, so the two can disagree and this says they do not");
        assert_eq!(hits["clearanceComplete"], true);

        assert!(hits.get("bandText").is_some(), "the key must reach a head in whatever profile it was built with");
        assert!(hits["bandText"].as_str().unwrap().contains(" to "));
        let clears = terrain_view(&Route(json!({ "kind": "object", "elements": [leg(0.0, 700.0, 600.0, false), leg(100.0, 700.0, 650.0, false)] })), &[]);
        assert_eq!(clears["hasCollision"], false);
        assert_eq!(clears["minClearanceMetres"], 50.0);
        assert_eq!(clears["usable"], true, "a profile a head can draw, which is what makes the false meaningful rather than empty");
    }

    #[test]
    fn a_complete_clearance_always_carries_a_figure_to_state() {
        // A head asked whether clearanceComplete can ever be true with clearanceText empty,
        // because its sentence falls back to the collision wording when there is no magnitude -
        // so a complete clearance with no figure would call a mission that clears it underground.
        // It cannot: completeness requires a measured clearance, and every measured clearance
        // formats to a number and a unit. Pinned so it stays a contract rather than a coincidence.
        let cases = vec![
            vec![leg(0.0, 700.0, 600.0, false), leg(100.0, 700.0, 690.0, false)],
            vec![leg(0.0, 700.0, 700.04, false), leg(100.0, 700.0, 700.0, false)],
            vec![leg(0.0, 500.0, 650.0, true), leg(100.0, 500.0, 668.0, true)],
            vec![leg(0.0, 700.0, 600.0, false)],
        ];
        for points in cases {
            let view = terrain_view(&Route(json!({ "kind": "object", "elements": points })), &[]);
            if view["clearanceComplete"] == true {
                let text = view["clearanceText"].as_str().unwrap_or("");
                assert!(!text.is_empty(), "clearanceComplete was true with no figure to state: {view}");
                assert!(text.chars().any(|c| c.is_ascii_digit()), "the figure has to be a number a head can put in a sentence, not {text:?}");
                assert!(view["minClearanceMetres"].is_number(), "and the signed metres travel with it, or a head cannot tell headroom from depth");
            }
        }
    }

    struct Pattern(Value);

    impl Backend for Pattern {
        fn get(&self, _p: &str) -> String { String::new() }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "plan.missionController.visualItems.1.flightPathSegments" => self.0.to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn segment(low: f64, high: f64, length: f64, heights: Vec<f64>, collision: bool) -> Value {
        json!({ "coord1AMSLAlt": low, "coord2AMSLAlt": high, "totalDistance": length, "distanceBetween": 0.0, "amslTerrainHeights": heights, "terrainCollision": collision })
    }

    #[test]
    fn terrain_samples_sit_where_qt_put_them_rather_than_spread_evenly() {
        let spaced = json!({ "kind": "object", "elements": [json!({
            "coord1AMSLAlt": 100.0, "coord2AMSLAlt": 200.0, "totalDistance": 100.0, "distanceBetween": 30.0,
            "amslTerrainHeights": [10.0, 20.0, 30.0], "terrainCollision": false,
        })] });
        let walked = along_segments(&Pattern(spaced), 1, 3, 0.0);
        let along: Vec<f64> = walked.iter().map(|point| point.distance).collect();
        assert_eq!(along, vec![0.0, 30.0, 100.0], "Qt steps the samples by distanceBetween and closes the last gap, so spreading them evenly moves every one of them");
        assert_eq!(walked[1].mission_altitude, 130.0, "the flown altitude at a sample follows where the sample actually is");
    }

    #[test]
    fn a_pattern_is_sampled_along_its_own_path_rather_than_at_its_entry() {
        let segments = json!({ "kind": "object", "elements": [segment(600.0, 620.0, 400.0, vec![500.0, 540.0, 580.0], false)] });
        let walked = along_segments(&Pattern(segments), 1, 3, 1000.0);
        assert_eq!(walked.len(), 3, "a survey covering four hundred metres of ground is three samples of it, not one point at the corner it started from");
        assert_eq!(walked[0].distance, 1000.0, "the first sample sits where the pattern begins");
        assert_eq!(walked[2].distance, 1400.0, "and the last where it ends, which is the entry plus the distance flown");
        assert_eq!(walked[1].mission_altitude, 610.0, "the flown altitude runs between the ends of the segment");
        assert_eq!(walked[1].terrain_altitude, Some(540.0), "the ground under the middle of the pattern is what a single entry point cannot show");
        assert!(walked.iter().all(|point| point.sequence == 3), "every sample belongs to the item it came from");
    }

    #[test]
    fn a_collision_inside_a_pattern_is_carried_by_the_samples_that_are_in_it() {
        let segments = json!({ "kind": "object", "elements": [segment(600.0, 600.0, 100.0, vec![700.0, 700.0], true), segment(600.0, 600.0, 100.0, vec![500.0, 500.0], false)] });
        let walked = along_segments(&Pattern(segments), 1, 3, 0.0);
        assert_eq!(walked.len(), 4);
        assert!(walked[0].collision && walked[1].collision, "the leg that flies into the hill says so");
        assert!(!walked[2].collision && !walked[3].collision);
        assert_eq!(walked[2].distance, 100.0, "the second segment starts where the first one ended");
        assert!(profile(walked).points.iter().any(|point| point.collision));
    }

    #[test]
    fn a_segment_with_no_terrain_yet_is_sampled_with_none_rather_than_skipped() {
        let segments = json!({ "kind": "object", "elements": [segment(600.0, 620.0, 200.0, Vec::new(), false)] });
        let walked = along_segments(&Pattern(segments), 1, 3, 0.0);
        assert_eq!(walked.len(), 2, "the two ends are known even when the ground between them has not arrived");
        assert!(walked.iter().all(|point| point.terrain_altitude.is_none()));
        assert_eq!(profile(walked).unknown_terrain, 2, "and the profile counts them as unknown rather than as ground at zero");
    }

    #[test]
    fn a_segment_that_has_not_said_how_high_it_flies_contributes_nothing() {
        let segments = json!({ "kind": "object", "elements": [json!({ "totalDistance": 100.0, "amslTerrainHeights": [500.0] })] });
        assert!(along_segments(&Pattern(segments), 1, 3, 0.0).is_empty(), "a sample with no flown altitude cannot be drawn against the ground, and drawing it at zero puts the mission underground");
    }

    #[test]
    fn an_item_with_no_segments_is_left_to_its_entry_point() {
        assert!(along_segments(&Pattern(json!({ "kind": "null" })), 1, 3, 0.0).is_empty());
    }
}
