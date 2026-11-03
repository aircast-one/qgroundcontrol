use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

pub const MAP3D_PAN: &str = "map3d.pan";
pub const MAP3D_TURN: &str = "map3d.turn";
pub const MAP3D_TILT: &str = "map3d.tilt";
pub const MAP3D_ZOOM: &str = "map3d.zoom";
pub const MAP3D_FOLLOW: &str = "map3d.follow";

pub const FOV_DEG: f64 = 60.0;
const TILT_DEG: (f64, f64) = (10.0, 89.0);
const RANGE_M: (f64, f64) = (30.0, 30_000.0);
const FLATTEST_SINE: f64 = 0.25;
const TILT_PER_VIEW_HEIGHT_DEG: f64 = 90.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    pub heading: f64,
    pub tilt: f64,
    pub range: f64,
    pub focus: Option<(f64, f64)>,
}

pub const START: Orbit = Orbit {
    heading: 0.0,
    tilt: 35.0,
    range: 500.0,
    focus: None,
};

static ORBIT: Mutex<Orbit> = Mutex::new(START);

pub fn owns(path: &str) -> bool {
    [MAP3D_PAN, MAP3D_TURN, MAP3D_TILT, MAP3D_ZOOM, MAP3D_FOLLOW].contains(&path)
}

pub fn orbit() -> Orbit {
    *ORBIT.lock().unwrap_or_else(PoisonError::into_inner)
}

fn view_width_m(range: f64) -> f64 {
    2.0 * range * (FOV_DEG / 2.0).to_radians().tan()
}

pub fn panned(orbit: Orbit, from: (f64, f64), [right, down, width]: [f64; 3]) -> Orbit {
    let metres_per_px = view_width_m(orbit.range) / width.max(1.0);
    let ahead = down * metres_per_px / orbit.tilt.to_radians().sin().max(FLATTEST_SINE);
    let forward =
        crate::surveygrid::at_distance_and_azimuth(from, ahead, orbit.heading.rem_euclid(360.0));
    Orbit {
        focus: Some(crate::surveygrid::at_distance_and_azimuth(
            forward,
            right * metres_per_px,
            (orbit.heading + 270.0).rem_euclid(360.0),
        )),
        ..orbit
    }
}

pub fn turned(orbit: Orbit, clockwise_deg: f64) -> Orbit {
    Orbit {
        heading: (orbit.heading - clockwise_deg).rem_euclid(360.0),
        ..orbit
    }
}

pub fn tilted(orbit: Orbit, [up, height]: [f64; 2]) -> Orbit {
    Orbit {
        tilt: (orbit.tilt - up / height.max(1.0) * TILT_PER_VIEW_HEIGHT_DEG)
            .clamp(TILT_DEG.0, TILT_DEG.1),
        ..orbit
    }
}

pub fn zoomed(orbit: Orbit, scale: f64) -> Orbit {
    Orbit {
        range: (orbit.range / scale).clamp(RANGE_M.0, RANGE_M.1),
        ..orbit
    }
}

pub fn moved(
    orbit: Orbit,
    path: &str,
    given: &[f64],
    aircraft: Option<(f64, f64)>,
) -> Option<Orbit> {
    match (path, given) {
        (MAP3D_PAN, [right, down, width]) => orbit
            .focus
            .or(aircraft)
            .map(|from| panned(orbit, from, [*right, *down, *width])),
        (MAP3D_TURN, [clockwise]) => Some(turned(orbit, *clockwise)),
        (MAP3D_TILT, [up, height]) => Some(tilted(orbit, [*up, *height])),
        (MAP3D_ZOOM, [scale]) if *scale > 0.0 => Some(zoomed(orbit, *scale)),
        (MAP3D_FOLLOW, []) => Some(Orbit {
            focus: None,
            ..orbit
        }),
        _ => None,
    }
}

pub fn run(path: &str, args: &str, aircraft: Option<(f64, f64)>) -> Value {
    let given: Vec<f64> = serde_json::from_str::<Vec<f64>>(args)
        .unwrap_or_default()
        .into_iter()
        .filter(|number| number.is_finite())
        .collect();
    let mut held = ORBIT.lock().unwrap_or_else(PoisonError::into_inner);
    match moved(*held, path, &given, aircraft) {
        Some(orbit) => {
            *held = orbit;
            json!({ "ok": true, "heading": orbit.heading, "tilt": orbit.tilt, "range": orbit.range, "following": orbit.focus.is_none() })
        }
        None => json!({ "ok": false, "reason": format!("{path} needs {}", needs(path)) }),
    }
}

fn needs(path: &str) -> &'static str {
    match path {
        MAP3D_PAN => {
            "the drag right and down and the view width, in pixels, and an aircraft or spot to move from"
        }
        MAP3D_TURN => "the twist in degrees, clockwise",
        MAP3D_TILT => "the drag up and the view height, in pixels",
        MAP3D_ZOOM => "the pinch scale, above 0",
        _ => "no arguments",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AIRCRAFT: (f64, f64) = (47.397, 8.5455);

    fn metres(a: (f64, f64), b: (f64, f64)) -> f64 {
        crate::track::distance_m(a, b)
    }

    #[test]
    fn a_drag_pins_the_view_to_the_ground_and_moves_it_against_the_finger() {
        let width = 1000.0;
        let across = view_width_m(START.range);
        let dragged_left = moved(
            START,
            MAP3D_PAN,
            &[-width / 2.0, 0.0, width],
            Some(AIRCRAFT),
        )
        .unwrap();
        let spot = dragged_left
            .focus
            .expect("a drag leaves the aircraft and holds a spot");
        assert!((metres(spot, AIRCRAFT) - across / 2.0).abs() < 1.0);
        assert!(
            spot.1 > AIRCRAFT.1,
            "looking north, a drag to the left shows what lies east"
        );

        let dragged_down = moved(START, MAP3D_PAN, &[0.0, 100.0, width], Some(AIRCRAFT)).unwrap();
        assert!(
            dragged_down.focus.unwrap().0 > AIRCRAFT.0,
            "pulling the ground down brings what lies ahead into view"
        );
    }

    #[test]
    fn a_second_drag_carries_on_from_the_held_spot_not_from_the_aircraft() {
        let once = moved(START, MAP3D_PAN, &[-100.0, 0.0, 1000.0], Some(AIRCRAFT)).unwrap();
        let twice = moved(once, MAP3D_PAN, &[-100.0, 0.0, 1000.0], Some((0.0, 0.0))).unwrap();
        assert!(
            (metres(twice.focus.unwrap(), AIRCRAFT) - 2.0 * metres(once.focus.unwrap(), AIRCRAFT))
                .abs()
                < 1.0
        );
    }

    #[test]
    fn with_no_aircraft_and_no_spot_a_drag_has_nothing_to_move() {
        assert_eq!(moved(START, MAP3D_PAN, &[10.0, 0.0, 1000.0], None), None);
    }

    #[test]
    fn a_twist_turns_the_map_with_the_fingers() {
        assert_eq!(
            moved(START, MAP3D_TURN, &[30.0], None).unwrap().heading,
            330.0
        );
    }

    #[test]
    fn two_fingers_up_tilt_toward_the_horizon_and_stop_short_of_it_and_of_straight_down() {
        assert_eq!(
            moved(START, MAP3D_TILT, &[100.0, 900.0], None)
                .unwrap()
                .tilt,
            25.0
        );
        assert_eq!(
            moved(START, MAP3D_TILT, &[900.0, 900.0], None)
                .unwrap()
                .tilt,
            TILT_DEG.0
        );
        assert_eq!(
            moved(START, MAP3D_TILT, &[-900.0, 900.0], None)
                .unwrap()
                .tilt,
            TILT_DEG.1
        );
    }

    #[test]
    fn a_pinch_out_comes_closer_within_the_range_limits() {
        assert_eq!(moved(START, MAP3D_ZOOM, &[2.0], None).unwrap().range, 250.0);
        assert_eq!(
            moved(START, MAP3D_ZOOM, &[1000.0], None).unwrap().range,
            RANGE_M.0
        );
        assert_eq!(moved(START, MAP3D_ZOOM, &[0.0], None), None);
    }

    #[test]
    fn follow_goes_back_to_the_aircraft_and_keeps_the_angle() {
        let away = Orbit {
            focus: Some((1.0, 2.0)),
            heading: 90.0,
            ..START
        };
        assert_eq!(
            moved(away, MAP3D_FOLLOW, &[], None),
            Some(Orbit {
                focus: None,
                ..away
            })
        );
    }
}
