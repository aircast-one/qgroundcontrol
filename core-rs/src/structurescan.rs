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
    crate::mappolygon::offset(structure, distance_to_surface)
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
    let entrance = flight[plan.entry_vertex.min(flight.len() - 1)];
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn the_flight_path_is_the_structure_pushed_out_by_the_camera_distance() {
        let flown = flight_polygon(&square(), 25.0).unwrap();
        assert_eq!(flown.len(), square().len(), "the flight path keeps a corner for every corner of the structure");
        let structure_perimeter = perimeter(&square());
        assert!(perimeter(&flown) < structure_perimeter, "this winding pulls the path inside the structure, which is the direction the Qt offset takes");
        assert!(flight_polygon(&[(47.0, 8.0), (47.1, 8.0)], 25.0).is_none());
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
        let flown = flight_polygon(&square(), 25.0).unwrap();
        let one = camera_shots(&flown, 30.0, 1);
        assert_eq!(camera_shots(&flown, 30.0, 3), one * 3, "each layer flies the same ring, so the shots multiply by the layers");
        assert_eq!(one, (perimeter(&flown) / 30.0) as i64, "the last part shot is dropped rather than rounded up");
        assert_eq!(camera_shots(&flown, 0.0, 2), 0, "a camera that never triggers takes no pictures");
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
    fn each_layer_closes_its_ring_and_switches_the_camera_off_at_the_end() {
        let flown = flight_polygon(&square(), 25.0).unwrap();
        let built = items(&flown, &plan());
        let waypoints: Vec<&Item> = built.iter().filter(|item| item.command == CMD_NAV_WAYPOINT).collect();
        assert_eq!(waypoints.len(), 2 + 2 * (flown.len() + 1), "two layers of five ring waypoints, plus the entrance and the exit");
        let corner = |item: &Item| (item.params[4], item.params[5]);
        assert_eq!(corner(waypoints[1]), corner(waypoints[1 + flown.len()]), "a layer ends back at the corner it started from");
        assert_ne!(corner(waypoints[1]), corner(waypoints[2]), "the corners in between are different corners, which is what makes the closing one worth asserting");
        let triggers: Vec<&Item> = built.iter().filter(|item| item.command == CMD_DO_SET_CAM_TRIGG_DIST).collect();
        assert_eq!(triggers.len(), 4, "each layer switches the camera on once and off once");
        assert_eq!(triggers[0].params[0], Some(30.0));
        assert_eq!(triggers[0].params[2], Some(1.0), "the camera fires on the first waypoint rather than after the first interval");
        assert_eq!(triggers[1].params[0], Some(0.0));
        assert_eq!(triggers[1].params[2], Some(0.0), "the closing command carries no immediate flag, which is what the Qt builder writes");
    }

    #[test]
    fn layers_climb_from_the_bottom_and_fall_from_the_top() {
        let flown = flight_polygon(&square(), 25.0).unwrap();
        let up = items(&flown, &plan());
        let heights = |built: &[Item]| -> Vec<f64> {
            built.iter().skip(2).filter(|item| item.command == CMD_NAV_WAYPOINT).map(|item| item.params[6].unwrap()).collect()
        };
        let climbing = heights(&up);
        assert_eq!(climbing[0], 20.0, "the first layer flies half a camera height above the bottom of the scan");
        assert_eq!(climbing[flown.len() + 1], 40.0, "the next layer is a full camera height higher");

        let down = items(&flown, &Plan { start_from_top: true, ..plan() });
        let falling = heights(&down);
        assert_eq!(falling[0], 40.0, "starting from the top begins half a camera height below the structure");
        assert_eq!(falling[flown.len() + 1], 20.0);
    }

    #[test]
    fn the_entry_corner_rotates_the_ring_without_shortening_it() {
        let flown = flight_polygon(&square(), 25.0).unwrap();
        let first = items(&flown, &plan());
        let third = items(&flown, &Plan { entry_vertex: 2, ..plan() });
        assert_eq!(first.len(), third.len(), "entering by another corner flies the same ring, not a shorter one");
        assert_eq!(third[0].params[4], Some(flown[2].0), "the scan starts at the corner it was told to start at");
        assert_ne!(first[0].params[4], third[0].params[4]);
    }
}
