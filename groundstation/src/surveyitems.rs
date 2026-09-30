use crate::surveygrid::{Coord, Kind};

type Point = (f64, f64);

pub const CMD_NAV_WAYPOINT: u16 = 16;
pub const CMD_DO_SET_CAM_TRIGG_DIST: u16 = 206;
pub const FRAME_GLOBAL: u8 = 0;
pub const FRAME_MISSION: u8 = 2;
pub const FRAME_GLOBAL_RELATIVE_ALT: u8 = 3;
pub const FRAME_GLOBAL_TERRAIN_ALT: u8 = 10;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Item {
    pub command: u16,
    pub frame: u8,
    pub params: [Option<f64>; 7],
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Plan {
    pub altitude: f64,
    pub trigger_distance: f64,
    pub altitude_mode: i64,
    pub images_in_turnaround: bool,
}

pub fn frame_for(altitude_mode: i64) -> u8 {
    match altitude_mode {
        crate::altitudemodes::ABSOLUTE | crate::altitudemodes::CALC_ABOVE_TERRAIN => FRAME_GLOBAL,
        crate::altitudemodes::TERRAIN_FRAME => FRAME_GLOBAL_TERRAIN_ALT,
        _ => FRAME_GLOBAL_RELATIVE_ALT,
    }
}

fn waypoint(at: Point, altitude: f64, frame: u8, hold: f64) -> Item {
    Item { command: CMD_NAV_WAYPOINT, frame, params: [Some(hold), Some(0.0), Some(0.0), None, Some(at.0), Some(at.1), Some(altitude)] }
}

fn trigger(distance: f64) -> Item {
    Item { command: CMD_DO_SET_CAM_TRIGG_DIST, frame: FRAME_MISSION, params: [Some(distance), Some(0.0), Some(1.0), Some(0.0), Some(0.0), Some(0.0), Some(0.0)] }
}

pub fn items(transects: &[Vec<Coord>], plan: &Plan) -> Vec<Item> {
    let frame = frame_for(plan.altitude_mode);
    let triggering = plan.trigger_distance > 0.0;
    let flight: Vec<Coord> = transects.iter().flatten().copied().collect();
    let last = flight.len().saturating_sub(1);
    flight
        .iter()
        .enumerate()
        .flat_map(|(index, coord)| {
            let first_turnaround = plan.images_in_turnaround && coord.kind == Kind::Turnaround && index == 0;
            let opens = triggering && (coord.kind == Kind::SurveyEntry || first_turnaround);
            let closes = triggering && index == last;
            std::iter::once(waypoint(coord.at, plan.altitude, frame, 0.0))
                .chain(opens.then(|| trigger(plan.trigger_distance)))
                .chain(closes.then(|| trigger(0.0)))
        })
        .collect()
}

pub fn spelled(item: &Item) -> String {
    let params: Vec<String> = item.params.iter().map(|value| value.map(|v| format!("{v}")).unwrap_or_else(|| "null".to_string())).collect();
    format!("{} {} [{}]", item.command, item.frame, params.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn oracle() -> Value {
        let text = std::fs::read_to_string("../test/Bridge/fixtures/survey-transects.json").expect("the oracle is recorded by QGCCoreCTest");
        serde_json::from_str(&text).expect("the oracle is JSON")
    }

    #[test]
    fn every_recorded_plan_builds_the_same_mission_items() {
        let cases = oracle();
        let cases = cases.as_object().expect("the oracle is an object of cases");
        let checked: Vec<(String, bool, String)> = cases
            .iter()
            .filter(|(_, case)| case["kind"] == "items")
            .map(|(name, case)| {
                let polygon: Vec<Point> = case["polygon"].as_array().unwrap().iter().map(|v| (v["latitude"].as_f64().unwrap(), v["longitude"].as_f64().unwrap())).collect();
                let grid = crate::surveygrid::Params {
                    grid_angle: case["gridAngle"].as_f64().unwrap(),
                    grid_spacing: case["gridSpacing"].as_f64().unwrap(),
                    turnaround: case["turnAround"].as_f64().unwrap(),
                    refly: false,
                    alternate: false,
                    entry: crate::altitudemodes::MIXED,
                };
                let transects = crate::surveygrid::typed_transects(&polygon, &grid);
                let plan = Plan { altitude: case["distanceToSurface"].as_f64().unwrap(), trigger_distance: 40.0, altitude_mode: crate::altitudemodes::RELATIVE, images_in_turnaround: true };
                let ours: Vec<String> = items(&transects, &plan).iter().map(spelled).collect();
                let expected: Vec<String> = case["items"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
                let matched = ours.len() == expected.len() && ours.iter().zip(expected.iter()).all(|(ours, theirs)| same_item(ours, theirs));
                (name.clone(), matched, format!("{name}\n  qt:   {}\n  rust: {}", expected.join(" | "), ours.join(" | ")))
            })
            .collect();
        assert_eq!(checked.len(), 3, "every recorded plan is checked, the turnaround one included");
        let wrong: Vec<&str> = checked.iter().filter(|(_, matched, _)| !matched).map(|(_, _, report)| report.as_str()).collect();
        assert!(wrong.is_empty(), "{} of {} plans differ from the Qt builder:\n{}", wrong.len(), checked.len(), wrong.join("\n"));
    }

    fn same_item(ours: &str, theirs: &str) -> bool {
        let split = |text: &str| {
            let (head, rest) = text.split_once(" [").unwrap();
            let numbers: Vec<Option<f64>> = rest.trim_end_matches(']').split(',').map(|v| v.trim().parse::<f64>().ok()).collect();
            (head.to_string(), numbers)
        };
        let (ours_head, ours_params) = split(ours);
        let (theirs_head, theirs_params) = split(theirs);
        ours_head == theirs_head
            && ours_params.len() == theirs_params.len()
            && ours_params.iter().zip(theirs_params.iter()).all(|(a, b)| match (a, b) {
                (Some(a), Some(b)) => (a - b).abs() < 1e-7,
                (None, None) => true,
                _ => false,
            })
    }

    #[test]
    fn every_recorded_corridor_plan_builds_the_same_mission_items() {
        let cases = oracle();
        let cases = cases.as_object().expect("the oracle is an object of cases");
        let checked: Vec<(String, bool, String)> = cases
            .iter()
            .filter(|(_, case)| case["kind"] == "corridorItems")
            .map(|(name, case)| {
                let polyline: Vec<Point> = case["polyline"].as_array().unwrap().iter().map(|v| (v["latitude"].as_f64().unwrap(), v["longitude"].as_f64().unwrap())).collect();
                let corridor = crate::corridorscan::Params {
                    width: case["corridorWidth"].as_f64().unwrap(),
                    spacing: case["gridSpacing"].as_f64().unwrap(),
                    turnaround: case["turnAround"].as_f64().unwrap(),
                    entry: 0,
                };
                let transects = crate::corridorscan::typed_transects(&polyline, &corridor);
                let plan = Plan {
                    altitude: case["distanceToSurface"].as_f64().unwrap(),
                    trigger_distance: case["triggerDistance"].as_f64().unwrap(),
                    altitude_mode: crate::altitudemodes::RELATIVE,
                    images_in_turnaround: true,
                };
                let ours: Vec<String> = items(&transects, &plan).iter().map(spelled).collect();
                let expected: Vec<String> = case["items"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
                let matched = ours.len() == expected.len() && ours.iter().zip(expected.iter()).all(|(ours, theirs)| same_item(ours, theirs));
                (name.clone(), matched, format!("{name}\n  qt:   {}\n  rust: {}", expected.join(" | "), ours.join(" | ")))
            })
            .collect();
        assert_eq!(checked.len(), 3, "every recorded corridor plan is checked");
        let wrong: Vec<&str> = checked.iter().filter(|(_, matched, _)| !matched).map(|(_, _, report)| report.as_str()).collect();
        assert!(wrong.is_empty(), "{} of {} corridor plans differ from the Qt builder:\n{}", wrong.len(), checked.len(), wrong.join("\n"));
    }

    #[test]
    fn the_camera_is_switched_on_at_each_entry_and_off_once_at_the_end() {
        let two = vec![crate::surveygrid::typed(vec![(47.0, 8.0), (47.1, 8.0)], 0.0), crate::surveygrid::typed(vec![(47.1, 8.1), (47.0, 8.1)], 0.0)];
        let built = items(&two, &Plan { altitude: 60.0, trigger_distance: 40.0, altitude_mode: crate::altitudemodes::RELATIVE, images_in_turnaround: true });
        let commands: Vec<u16> = built.iter().map(|item| item.command).collect();
        assert_eq!(commands, [16, 206, 16, 16, 206, 16, 206], "a trigger follows each entry, and one last trigger turns the camera off");
        assert_eq!(built.last().unwrap().params[0], Some(0.0), "the closing trigger is a distance of zero, which is what stops the camera");
        assert_eq!(built[1].params[0], Some(40.0));
    }

    #[test]
    fn a_survey_with_no_camera_is_waypoints_alone() {
        let one = vec![crate::surveygrid::typed(vec![(47.0, 8.0), (47.1, 8.0)], 0.0)];
        let built = items(&one, &Plan { altitude: 60.0, trigger_distance: 0.0, altitude_mode: crate::altitudemodes::RELATIVE, images_in_turnaround: true });
        assert!(built.iter().all(|item| item.command == CMD_NAV_WAYPOINT), "no trigger distance means no camera commands at all");
        assert_eq!(built.len(), 2);
    }

    #[test]
    fn the_frame_follows_the_altitude_mode() {
        assert_eq!(frame_for(crate::altitudemodes::RELATIVE), FRAME_GLOBAL_RELATIVE_ALT);
        assert_eq!(frame_for(crate::altitudemodes::ABSOLUTE), FRAME_GLOBAL);
        assert_eq!(frame_for(crate::altitudemodes::CALC_ABOVE_TERRAIN), FRAME_GLOBAL, "terrain is converted to a height above sea level before upload, so it flies as an absolute frame");
        assert_eq!(frame_for(crate::altitudemodes::TERRAIN_FRAME), FRAME_GLOBAL_TERRAIN_ALT);
        assert_eq!(frame_for(crate::altitudemodes::MIXED), FRAME_GLOBAL_RELATIVE_ALT, "a mode that cannot be flown falls back to the relative frame, as the Qt builder does after warning");
    }
}
