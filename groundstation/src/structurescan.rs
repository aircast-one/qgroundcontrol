use crate::surveygrid::distance_between;
use crate::surveyitems::{CMD_DO_SET_CAM_TRIGG_DIST, CMD_NAV_WAYPOINT, FRAME_GLOBAL_RELATIVE_ALT, FRAME_MISSION, Item};

pub const CMD_DO_SET_ROI_WPNEXT_OFFSET: u16 = 196;
pub const CMD_DO_SET_ROI_NONE: u16 = 197;

type Point = (f64, f64);

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Plan {
    pub adjusted_side: f64,
    pub adjusted_frontal: f64,
    pub entrance_alt: f64,
    pub scan_bottom_alt: f64,
    pub structure_height: f64,
    pub layers: i64,
    pub start_from_top: bool,
    pub gimbal_pitch: f64,
    pub entry_vertex: usize,
}

pub fn flight_polygon(structure: &[Point], distance_to_surface: f64) -> Option<Vec<Point>> {
    crate::mappolygon::offset(&crate::mappolygon::wound_clockwise(structure), distance_to_surface)
}

pub fn layer_count(structure_height: f64, scan_bottom_alt: f64, adjusted_frontal: f64) -> i64 {
    if adjusted_frontal <= 0.0 {
        return 1;
    }
    let surface = (structure_height - scan_bottom_alt).max(0.0);
    ((surface / adjusted_frontal).ceil() as i64).max(1)
}

pub fn perimeter(flight: &[Point]) -> f64 {
    (0..flight.len()).map(|index| distance_between(flight[index], flight[(index + 1) % flight.len()])).sum()
}

pub fn camera_shots(flight: &[Point], adjusted_side: f64, layers: i64) -> i64 {
    if adjusted_side == 0.0 || flight.len() < 3 {
        return 0;
    }
    let distance = perimeter(flight);
    if distance == 0.0 {
        return 0;
    }
    (distance / adjusted_side) as i64 * layers
}

fn waypoint(at: Point, altitude: f64) -> Item {
    Item { command: CMD_NAV_WAYPOINT, frame: FRAME_GLOBAL_RELATIVE_ALT, params: [Some(0.0), Some(0.0), Some(0.0), None, Some(at.0), Some(at.1), Some(altitude)] }
}

fn trigger(distance: f64, immediate: f64) -> Item {
    Item { command: CMD_DO_SET_CAM_TRIGG_DIST, frame: FRAME_MISSION, params: [Some(distance), Some(0.0), Some(immediate), Some(0.0), Some(0.0), Some(0.0), Some(0.0)] }
}

fn roi(command: u16, params: [Option<f64>; 7]) -> Item {
    Item { command, frame: FRAME_MISSION, params }
}

pub fn items(flight: &[Point], plan: &Plan) -> Vec<Item> {
    if flight.is_empty() {
        return Vec::new();
    }
    let entrance = flight[plan.entry_vertex % flight.len()];
    let half = plan.adjusted_frontal / 2.0;
    let start = if plan.start_from_top { plan.structure_height } else { plan.scan_bottom_alt };
    let first = if plan.start_from_top { start - half } else { start + half };
    let step = if plan.start_from_top { -(half * 2.0) } else { half * 2.0 };
    let ring: Vec<Point> = (0..=flight.len()).map(|offset| flight[(plan.entry_vertex + offset) % flight.len()]).collect();
    let layers = (0..plan.layers)
        .scan(first, |altitude, _| {
            let flown = *altitude;
            *altitude += step;
            Some(flown)
        })
        .flat_map(|altitude| {
            ring.iter()
                .enumerate()
                .flat_map(move |(index, at)| std::iter::once(waypoint(*at, altitude)).chain((index == 0).then(|| trigger(plan.adjusted_side, 1.0))))
                .chain(std::iter::once(trigger(0.0, 0.0)))
        });
    std::iter::once(waypoint(entrance, plan.entrance_alt))
        .chain(std::iter::once(roi(CMD_DO_SET_ROI_WPNEXT_OFFSET, [Some(0.0), Some(0.0), Some(0.0), Some(0.0), Some(plan.gimbal_pitch), Some(0.0), Some(90.0)])))
        .chain(layers)
        .chain(std::iter::once(roi(CMD_DO_SET_ROI_NONE, [Some(0.0); 7])))
        .chain(std::iter::once(waypoint(entrance, plan.entrance_alt)))
        .collect()
}

fn number(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get(key).and_then(serde_json::Value::as_f64)
}

pub fn saved_flight(scan: &serde_json::Value) -> Result<Vec<Point>, String> {
    let calc = scan.get("CameraCalc").ok_or("The structure scan has no camera settings.")?;
    let structure: Vec<Point> = scan.get("polygon").and_then(serde_json::Value::as_array).map(|p| p.iter().filter_map(|v| Some((v.get(0)?.as_f64()?, v.get(1)?.as_f64()?))).collect()).unwrap_or_default();
    if structure.len() < 3 {
        return Ok(Vec::new());
    }
    flight_polygon(&structure, number(calc, "DistanceToSurface").unwrap_or(0.0)).ok_or_else(|| "The structure scan's flight path could not be laid around its structure.".to_string())
}

pub fn saved_plan(scan: &serde_json::Value) -> Plan {
    let calc = scan.get("CameraCalc").cloned().unwrap_or(serde_json::Value::Null);
    Plan {
        adjusted_side: number(&calc, "AdjustedFootprintSide").unwrap_or(0.0),
        adjusted_frontal: number(&calc, "AdjustedFootprintFrontal").unwrap_or(0.0),
        entrance_alt: number(scan, "EntranceAltitude").unwrap_or(0.0),
        scan_bottom_alt: number(scan, "ScanBottomAlt").unwrap_or(0.0),
        structure_height: number(scan, "StructureHeight").unwrap_or(0.0),
        layers: layer_count(number(scan, "StructureHeight").unwrap_or(0.0), number(scan, "ScanBottomAlt").unwrap_or(0.0), number(&calc, "AdjustedFootprintFrontal").unwrap_or(0.0)),
        start_from_top: scan.get("StartFromTop").and_then(serde_json::Value::as_bool).unwrap_or(true),
        gimbal_pitch: number(scan, "GimbalPitch").unwrap_or(0.0),
        entry_vertex: scan.get(ENTRY_VERTEX).and_then(serde_json::Value::as_u64).unwrap_or(0) as usize,
    }
}

pub const ENTRY_VERTEX: &str = "entryVertex";

pub fn entry_vertex(scan: &serde_json::Value) -> usize {
    saved_plan(scan).entry_vertex
}

pub fn rotated_entry(scan: &serde_json::Value) -> Option<serde_json::Value> {
    let count = saved_flight(scan).ok()?.len();
    let next = if entry_vertex(scan) + 1 >= count { 0 } else { entry_vertex(scan) + 1 };
    let mut rotated = scan.clone();
    rotated[ENTRY_VERTEX] = serde_json::json!(next);
    Some(rotated)
}

pub fn saved_items(scan: &serde_json::Value) -> Result<Vec<Item>, String> {
    Ok(items(&saved_flight(scan)?, &saved_plan(scan)))
}

pub fn top_and_bottom(plan: &Plan) -> (f64, f64) {
    let half = plan.adjusted_frontal / 2.0;
    let span = half + (plan.layers - 1) as f64 * plan.adjusted_frontal;
    match plan.start_from_top {
        true => (plan.structure_height - half, plan.structure_height - span),
        false => (plan.scan_bottom_alt + span, plan.scan_bottom_alt + half),
    }
}

pub fn scan_distance(flight: &[Point], plan: &Plan) -> f64 {
    match flight.len() > 2 {
        true => perimeter(flight) * plan.layers as f64 + (plan.structure_height - plan.scan_bottom_alt).max(0.0),
        false => 0.0,
    }
}

pub fn saved_item_json(item: &Item) -> serde_json::Value {
    serde_json::json!({ "type": "SimpleItem", "autoContinue": true, "command": item.command, "frame": item.frame, "params": item.params })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flown_band_is_half_a_layer_inside_the_structure_as_qt_bounds_it() {
        let plan = Plan { adjusted_frontal: 25.0, structure_height: 100.0, scan_bottom_alt: 50.0, layers: 2, start_from_top: true, ..Plan::default() };
        assert_eq!(top_and_bottom(&plan), (87.5, 62.5));
        assert_eq!(top_and_bottom(&Plan { start_from_top: false, ..plan }), (87.5, 62.5), "two layers of 25 m between 50 m and 100 m fly the same band from either end");
        let square = vec![(0.0, 0.0), (0.0, 0.001), (0.001, 0.001), (0.001, 0.0)];
        assert!((scan_distance(&square, &plan) - (perimeter(&square) * 2.0 + 50.0)).abs() < 1e-9, "each layer walks the perimeter and the climb between the ends counts once");
    }
    use serde_json::Value;

    fn oracle() -> Value {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../test/Bridge/fixtures/structure-scan.json")).expect("the oracle is recorded by QGCCoreCTest");
        serde_json::from_str(&text).expect("the oracle is JSON")
    }

    fn parsed(spelled: &str) -> Vec<Point> {
        spelled.split(' ').map(|pair| pair.split_once(',').unwrap()).map(|(lat, lon)| (lat.parse().unwrap(), lon.parse().unwrap())).collect()
    }

    fn spelled(points: &[Point]) -> String {
        points.iter().map(|(lat, lon)| format!("{lat:.7},{lon:.7}")).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn every_recorded_structure_is_flown_around_identically() {
        let cases = oracle();
        let cases = cases.as_object().unwrap();
        let wrong: Vec<String> = cases
            .iter()
            .map(|(name, case)| {
                let structure = parsed(case["structure"].as_str().unwrap());
                let ours = flight_polygon(&structure, case["distanceToSurface"].as_f64().unwrap()).map(|flown| spelled(&flown)).unwrap_or_default();
                (name.clone(), ours, case["flight"].as_str().unwrap().to_string())
            })
            .filter(|(_, ours, theirs)| ours != theirs)
            .map(|(name, ours, theirs)| format!("{name}\n  qt:   {theirs}\n  rust: {ours}"))
            .collect();
        assert!(wrong.is_empty(), "{} of {} flight paths differ from the Qt item:\n{}", wrong.len(), cases.len(), wrong.join("\n"));
        assert_eq!(cases.len(), 18, "three shapes at three distances, each recorded in both windings");
    }

    #[test]
    fn the_vehicle_flies_clear_of_the_structure_whichever_way_it_was_drawn() {
        let cases = oracle();
        let cases = cases.as_object().unwrap();
        let inside: Vec<String> = cases
            .iter()
            .filter(|(_, case)| case["distanceToSurface"].as_f64().unwrap() > 0.0)
            .map(|(name, case)| {
                let structure = parsed(case["structure"].as_str().unwrap());
                let flown = flight_polygon(&structure, case["distanceToSurface"].as_f64().unwrap()).unwrap();
                (name.clone(), crate::mappolygon::area(&flown), crate::mappolygon::area(&structure))
            })
            .filter(|(_, flight, structure)| flight <= structure)
            .map(|(name, flight, structure)| format!("{name} flies a path of {flight} around a structure of {structure}"))
            .collect();
        assert!(inside.is_empty(), "a scan that flies inside the structure is a scan into the tower:\n{}", inside.join("\n"));
        assert_eq!(cases.iter().filter(|(_, case)| case["distanceToSurface"].as_f64().unwrap() > 0.0).count(), 12);
    }

    fn square() -> Vec<Point> {
        vec![(47.3960, 8.5440), (47.3960, 8.5480), (47.3990, 8.5480), (47.3990, 8.5440)]
    }

    fn plan() -> Plan {
        Plan {
            adjusted_side: 30.0,
            adjusted_frontal: 20.0,
            entrance_alt: 5.0,
            scan_bottom_alt: 10.0,
            structure_height: 50.0,
            layers: 2,
            start_from_top: false,
            gimbal_pitch: -15.0,
            entry_vertex: 0,
        }
    }

    #[test]
    fn a_shape_with_no_area_cannot_be_flown_around() {
        assert!(flight_polygon(&[(47.0, 8.0), (47.1, 8.0)], 25.0).is_none());
        assert!(flight_polygon(&[], 25.0).is_none());
        assert_eq!(flight_polygon(&square(), 25.0).unwrap().len(), square().len(), "the flight path keeps a corner for every corner of the structure");
    }

    #[test]
    fn a_layer_is_added_for_every_camera_height_of_structure() {
        assert_eq!(layer_count(50.0, 10.0, 20.0), 2, "forty metres of structure at twenty metres a layer is two layers");
        assert_eq!(layer_count(51.0, 10.0, 20.0), 3, "a partial layer is still flown, so the count rounds up");
        assert_eq!(layer_count(10.0, 10.0, 20.0), 1, "a structure with no height is still scanned once");
        assert_eq!(layer_count(5.0, 10.0, 20.0), 1, "a bottom above the top is no height rather than a negative one");
        assert_eq!(layer_count(50.0, 10.0, 0.0), 1, "a camera that sees nothing would ask for endless layers, so it asks for one");
    }

    #[test]
    fn the_shot_count_is_the_perimeter_walked_once_per_layer() {
        let ring = vec![(47.0000, 8.0000), (47.0000, 8.0010), (47.0010, 8.0010), (47.0010, 8.0000)];
        let walked = perimeter(&ring);
        assert!((walked - 374.0).abs() < 2.0, "this ring is about three hundred and seventy four metres around");
        assert_eq!(camera_shots(&ring, 100.0, 1), 3, "three whole hundred metre intervals fit, and the remaining seventy four metres are dropped rather than rounded up");
        assert_eq!(camera_shots(&ring, 100.0, 3), 9, "each layer flies the same ring");
        assert_eq!(camera_shots(&ring, 374.0, 1), 1);
        assert_eq!(camera_shots(&ring, 375.0, 1), 0, "a trigger distance longer than the ring takes no pictures at all");
        assert_eq!(camera_shots(&ring, 0.0, 2), 0, "a camera that never triggers takes no pictures");
        assert_eq!(camera_shots(&[(47.0, 8.0), (47.1, 8.0)], 30.0, 2), 0, "a path with no area is not a scan");
    }

    #[test]
    fn the_scan_enters_and_leaves_at_the_entrance_altitude() {
        let flown = flight_polygon(&square(), 25.0).unwrap();
        let built = items(&flown, &plan());
        assert_eq!(built[0].command, CMD_NAV_WAYPOINT);
        assert_eq!(built[0].params[6], Some(5.0));
        assert_eq!(built.last().unwrap().command, CMD_NAV_WAYPOINT);
        assert_eq!(built.last().unwrap().params[6], Some(5.0), "the scan comes back to the height it arrived at");
        assert_eq!(built[0].params[4], built.last().unwrap().params[4], "it leaves from the corner it entered by");
        assert_eq!(built[1].command, CMD_DO_SET_ROI_WPNEXT_OFFSET, "the camera is pointed at the structure before the first layer");
        assert_eq!(built[1].params[4], Some(-15.0));
        assert_eq!(built[1].params[6], Some(90.0), "ninety degrees of yaw turns the camera off the flight path and onto the wall");
        assert_eq!(built[built.len() - 2].command, CMD_DO_SET_ROI_NONE, "the camera is released before the vehicle leaves");
    }

    #[test]
    fn the_camera_is_switched_on_after_the_first_waypoint_of_a_layer_and_not_before_it() {
        let flown = flight_polygon(&square(), 25.0).unwrap();
        let built = items(&flown, &plan());
        assert_eq!(built[2].command, CMD_NAV_WAYPOINT, "the vehicle reaches the first corner before the camera is armed");
        assert_eq!(built[3].command, CMD_DO_SET_CAM_TRIGG_DIST, "arming it any earlier fires the camera along the leg in from the entrance");
        assert_eq!(built[3].params[0], Some(30.0));
        assert_eq!(built[3].params[2], Some(1.0), "the camera fires at that corner rather than after the first interval");
        let ring = flown.len() + 1;
        assert_eq!(built[3 + ring].command, CMD_DO_SET_CAM_TRIGG_DIST, "the layer closes with the camera switched off right after its last corner");
        assert_eq!(built[3 + ring].params[0], Some(0.0));
        assert_eq!(built[3 + ring].params[2], Some(0.0), "the closing command carries no immediate flag, which is what the Qt builder writes");
        assert_eq!(built[2 + ring].command, CMD_NAV_WAYPOINT);
    }

    #[test]
    fn each_layer_closes_its_ring() {
        let flown = flight_polygon(&square(), 25.0).unwrap();
        let built = items(&flown, &plan());
        let waypoints: Vec<&Item> = built.iter().filter(|item| item.command == CMD_NAV_WAYPOINT).collect();
        assert_eq!(waypoints.len(), 2 + 2 * (flown.len() + 1), "two layers of five ring waypoints, plus the entrance and the exit");
        let corner = |item: &Item| (item.params[4], item.params[5]);
        assert_eq!(corner(waypoints[1]), corner(waypoints[1 + flown.len()]), "a layer ends back at the corner it started from");
        assert_ne!(corner(waypoints[1]), corner(waypoints[2]), "the corners in between are different corners, which is what makes the closing one worth asserting");
        assert_eq!(built.iter().filter(|item| item.command == CMD_DO_SET_CAM_TRIGG_DIST).count(), 4, "each layer switches the camera on once and off once");
    }

    #[test]
    fn layers_climb_from_the_bottom_and_fall_from_the_top() {
        let flown = flight_polygon(&square(), 25.0).unwrap();
        let heights = |built: &[Item]| -> Vec<f64> {
            built.iter().skip(2).filter(|item| item.command == CMD_NAV_WAYPOINT).map(|item| item.params[6].unwrap()).collect()
        };
        let climbing = heights(&items(&flown, &plan()));
        assert_eq!(climbing[0], 20.0, "the first layer flies half a camera height above the bottom of the scan");
        assert_eq!(climbing[flown.len() + 1], 40.0, "the next layer is a full camera height higher");

        let falling = heights(&items(&flown, &Plan { start_from_top: true, ..plan() }));
        assert_eq!(falling[0], 40.0, "starting from the top begins half a camera height below the structure");
        assert_eq!(falling[flown.len() + 1], 20.0);
    }

    #[test]
    fn rotating_the_entry_vertex_walks_the_flight_ring_and_wraps() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!("../tests/fixtures/structure-inserted-by-qt.json")).unwrap();
        let scan = fixture["structure"].clone();
        let count = saved_flight(&scan).unwrap().len();
        let walked = (0..count).fold(scan.clone(), |at, _| rotated_entry(&at).unwrap());
        assert_eq!(entry_vertex(&rotated_entry(&scan).unwrap()), 1);
        assert_eq!(entry_vertex(&walked), 0, "a full turn comes back to the first vertex");
    }

    #[test]
    fn the_entry_corner_rotates_the_ring_rather_than_reversing_or_shortening_it() {
        let flown = flight_polygon(&square(), 25.0).unwrap();
        let corners = |built: &[Item]| -> Vec<(f64, f64)> {
            built.iter().skip(2).filter(|item| item.command == CMD_NAV_WAYPOINT).take(flown.len() + 1).map(|item| (item.params[4].unwrap(), item.params[5].unwrap())).collect()
        };
        (0..flown.len()).for_each(|entry| {
            let built = items(&flown, &Plan { entry_vertex: entry, ..plan() });
            let expected: Vec<(f64, f64)> = (0..=flown.len()).map(|offset| flown[(entry + offset) % flown.len()]).collect();
            assert_eq!(corners(&built), expected, "entering at corner {entry} must fly the same ring from that corner, in the same direction");
            assert_eq!((built[0].params[4], built[0].params[5]), (Some(flown[entry].0), Some(flown[entry].1)));
        });
    }

    #[test]
    fn an_entry_corner_past_the_last_one_wraps_rather_than_splitting_the_scan() {
        let flown = flight_polygon(&square(), 25.0).unwrap();
        let wrapped = items(&flown, &Plan { entry_vertex: flown.len() + 1, ..plan() });
        let first = items(&flown, &Plan { entry_vertex: 1, ..plan() });
        assert_eq!(wrapped, first, "the entrance and the ring have to agree on which corner they mean, or the vehicle flies to one corner and scans around another");
    }

    #[test]
    fn a_scan_with_no_layers_flies_nowhere_but_still_releases_the_camera() {
        let flown = flight_polygon(&square(), 25.0).unwrap();
        let built = items(&flown, &Plan { layers: 0, ..plan() });
        assert_eq!(built.len(), 4, "an entrance, the gimbal command, the release and the exit");
        assert!(built.iter().all(|item| item.command != CMD_DO_SET_CAM_TRIGG_DIST), "a scan with no layers never arms the camera, so it must not leave it armed either");
        assert_eq!(built[2].command, CMD_DO_SET_ROI_NONE);
    }

    #[test]
    fn a_flight_path_with_no_corners_builds_nothing() {
        assert!(items(&[], &plan()).is_empty(), "the Qt item would emit an entrance and a ring around a corner that does not exist");
    }
}

#[cfg(test)]
mod upload {
    use super::*;
    use serde_json::Value;

    fn oracle() -> Value {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../test/Bridge/fixtures/structure-scan-items.json")).expect("the oracle is recorded by QGCCoreCTest");
        serde_json::from_str(&text).expect("the oracle is JSON")
    }

    fn parsed(spelled: &str) -> Vec<Point> {
        spelled.split(' ').map(|pair| pair.split_once(',').unwrap()).map(|(lat, lon)| (lat.parse().unwrap(), lon.parse().unwrap())).collect()
    }

    fn spelled(item: &Item) -> String {
        let params: Vec<String> = item.params.iter().map(|value| value.map(|v| format!("{v:.7}")).map(|v| format!("\"{v}\"")).unwrap_or_else(|| "null".to_string())).collect();
        format!("{} {} [{}]", item.command, item.frame, params.join(","))
    }

    #[test]
    fn every_recorded_scan_uploads_the_same_items() {
        let cases = oracle();
        let cases = cases.as_object().unwrap();
        let wrong: Vec<String> = cases
            .iter()
            .map(|(name, case)| {
                let flight = parsed(case["flight"].as_str().unwrap());
                let plan = Plan {
                    adjusted_side: case["adjustedFootprintSide"].as_str().unwrap().parse().unwrap(),
                    adjusted_frontal: case["adjustedFootprintFrontal"].as_str().unwrap().parse().unwrap(),
                    entrance_alt: case["entranceAlt"].as_f64().unwrap(),
                    scan_bottom_alt: case["scanBottomAlt"].as_f64().unwrap(),
                    structure_height: case["structureHeight"].as_f64().unwrap(),
                    layers: case["layers"].as_f64().unwrap() as i64,
                    start_from_top: case["startFromTop"].as_bool().unwrap(),
                    gimbal_pitch: case["gimbalPitch"].as_f64().unwrap(),
                    entry_vertex: 0,
                };
                let uploaded: Vec<String> = case["items"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
                let scan: Vec<String> = uploaded.iter().skip_while(|line| !line.starts_with("16 ")).cloned().collect();
                let ours: Vec<String> = items(&flight, &plan).iter().map(spelled).collect();
                let matched = ours.len() == scan.len() && ours.iter().zip(scan.iter()).all(|(a, b)| same_item(a, b));
                (name.clone(), matched, format!("{name}\n  qt:   {}\n  rust: {}", scan.join("\n        "), ours.join("\n        ")))
            })
            .filter(|(_, matched, _)| !matched)
            .map(|(_, _, report)| report)
            .collect();
        assert!(wrong.is_empty(), "{} of {} scans differ from what the vehicle received:\n{}", wrong.len(), cases.len(), wrong.join("\n"));
        assert_eq!(cases.len(), 3);
    }

    #[test]
    fn the_recorded_upload_begins_with_the_plans_own_item_and_not_the_scans() {
        let cases = oracle();
        let cases = cases.as_object().unwrap();
        cases.iter().for_each(|(name, case)| {
            let uploaded: Vec<&str> = case["items"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
            assert!(uploaded[0].starts_with("530 "), "{name} no longer begins with the mission settings item, so skipping to the first waypoint would skip part of the scan");
            assert!(uploaded[1].starts_with("16 "), "{name} has something between the settings item and the scan");
        });
    }

    fn same_item(ours: &str, theirs: &str) -> bool {
        let split = |text: &str| {
            let (head, rest) = text.split_once(" [").unwrap();
            let numbers: Vec<Option<f64>> = rest.trim_end_matches(']').split(',').map(|v| v.trim().trim_matches('"').parse::<f64>().ok()).collect();
            (head.to_string(), numbers)
        };
        let (ours_head, ours_params) = split(ours);
        let (theirs_head, theirs_params) = split(theirs);
        ours_head == theirs_head
            && ours_params.len() == theirs_params.len()
            && ours_params.iter().zip(theirs_params.iter()).all(|(a, b)| match (a, b) {
                (Some(a), Some(b)) => (a - b).abs() < 1e-6,
                (None, None) => true,
                _ => false,
            })
    }
}
