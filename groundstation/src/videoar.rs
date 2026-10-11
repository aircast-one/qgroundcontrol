use serde_json::{Value, json};

use crate::attitude::{point, raw};
use crate::read::{Unit, format_measure};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.coordinate",
    "vehicle.homePosition",
    "vehicle.altitudeRelative",
    "vehicle.heading",
    "vehicle.roll",
    "vehicle.pitch",
    "settings.unitsSettings.horizontalDistanceUnits",
    crate::gimbal::GIMBAL_CHANGED,
];

const EARTH_RADIUS_M: f64 = 6_371_000.0;
const NEAR_M: f64 = 1.0;
const HOME_LABEL: &str = "H";

type Space = (f64, f64, f64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub heading: f64,
    pub pitch: f64,
    pub roll: f64,
    pub fov: f64,
}

pub fn deps() -> Vec<String> {
    DEPS.iter().map(|dep| dep.to_string()).chain(crate::synthview::overlay_deps()).collect()
}

fn offset(from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    let east = (to.1 - from.1).to_radians() * EARTH_RADIUS_M * from.0.to_radians().cos();
    let north = (to.0 - from.0).to_radians() * EARTH_RADIUS_M;
    (east, north)
}

pub fn in_camera(camera: Camera, (east, north, up): Space) -> Space {
    let (heading, pitch, roll) = (camera.heading.to_radians(), camera.pitch.to_radians(), camera.roll.to_radians());
    let forward = (heading.sin() * pitch.cos(), heading.cos() * pitch.cos(), pitch.sin());
    let right = (heading.cos(), -heading.sin(), 0.0);
    let upward = (-heading.sin() * pitch.sin(), -heading.cos() * pitch.sin(), pitch.cos());
    let dot = |axis: Space| east * axis.0 + north * axis.1 + up * axis.2;
    let (across, above, ahead) = (dot(right), dot(upward), dot(forward));
    (across * roll.cos() - above * roll.sin(), above * roll.cos() + across * roll.sin(), ahead)
}

fn framed(camera: Camera, (across, above, ahead): Space) -> Value {
    let scale = 0.5 / (camera.fov / 2.0).to_radians().tan();
    json!({ "x": 0.5 + across / ahead * scale, "up": above / ahead * scale })
}

fn seen(drone: (f64, f64), drone_above: f64, camera: Camera, at: (f64, f64), above: f64) -> Space {
    let (east, north) = offset(drone, at);
    in_camera(camera, (east, north, above - drone_above))
}

fn toward(kept: Space, gone: Space) -> Space {
    let share = (kept.2 - NEAR_M) / (kept.2 - gone.2);
    (kept.0 + (gone.0 - kept.0) * share, kept.1 + (gone.1 - kept.1) * share, NEAR_M)
}

pub fn clipped(from: Space, to: Space) -> Option<(Space, Space)> {
    match (from.2 > NEAR_M, to.2 > NEAR_M) {
        (true, true) => Some((from, to)),
        (true, false) => Some((from, toward(from, to))),
        (false, true) => Some((toward(to, from), to)),
        (false, false) => None,
    }
}

fn spot_of(point: &Value) -> Option<((f64, f64), f64)> {
    Some(((point["latitude"].as_f64()?, point["longitude"].as_f64()?), point["aboveHome"].as_f64().unwrap_or(0.0)))
}

pub fn ar(drone: (f64, f64), drone_above: f64, camera: Camera, overlays: &Value, distance: &dyn Fn(f64) -> String) -> Value {
    let looked = |at: (f64, f64), above: f64| seen(drone, drone_above, camera, at, above);
    let ahead = |space: Space| (space.2 > NEAR_M).then(|| framed(camera, space));
    let home = spot_of(&overlays["home"]).and_then(|(at, above)| {
        ahead(looked(at, above)).map(|mut shown| {
            shown["label"] = json!(HOME_LABEL);
            shown["text"] = json!(distance(crate::track::distance_m(drone, at)));
            shown
        })
    });
    let waypoints: Vec<Value> = overlays["waypoints"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|point| {
            let (at, above) = spot_of(point)?;
            ahead(looked(at, above)).map(|mut shown| {
                shown["label"] = point["label"].clone();
                shown
            })
        })
        .collect();
    let route: Vec<Space> = overlays["route"].as_array().cloned().unwrap_or_default().iter().filter_map(spot_of).map(|(at, above)| looked(at, above)).collect();
    let legs: Vec<Value> = route
        .windows(2)
        .filter_map(|leg| clipped(leg[0], leg[1]))
        .map(|(from, to)| json!({ "from": framed(camera, from), "to": framed(camera, to) }))
        .collect();
    json!({ "kind": "object", "class": "VideoAr", "available": true, "home": home, "waypoints": waypoints, "route": legs })
}

pub fn camera(backend: &dyn Backend, gimbals: &Value) -> Camera {
    match crate::synthview::gimbal_aim(gimbals) {
        Some((heading, pitch)) => Camera { heading, pitch, roll: 0.0, fov: crate::synthview::CAMERA_FOV_DEG },
        None => Camera {
            heading: raw(backend, "vehicle.heading").unwrap_or(0.0),
            pitch: raw(backend, "vehicle.pitch").unwrap_or(0.0) + crate::synthview::FIXED_CAMERA_TILT_DEG,
            roll: raw(backend, "vehicle.roll").unwrap_or(0.0),
            fov: crate::synthview::CAMERA_FOV_DEG,
        },
    }
}

pub fn video_ar_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let placed = point(backend, "vehicle.coordinate").zip(point(backend, "vehicle.homePosition")).zip(raw(backend, "vehicle.altitudeRelative"));
    let Some(((drone, _), drone_above)) = placed else {
        return json!({ "kind": "object", "class": "VideoAr", "available": false });
    };
    let unit = Unit::horizontal(backend);
    let distance = |metres: f64| format_measure(unit.show(metres), &unit.name);
    ar(drone, drone_above, camera(backend, &crate::gimbal::gimbal_view(backend, &[])), &crate::synthview::overlays_view(backend, &[]), &distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEVEL_NORTH: Camera = Camera { heading: 0.0, pitch: 0.0, roll: 0.0, fov: 90.0 };

    fn metres(value: f64) -> String {
        format!("{value:.0} m")
    }

    #[test]
    fn a_point_straight_ahead_sits_in_the_middle_of_the_frame() {
        let (across, above, ahead) = in_camera(LEVEL_NORTH, (0.0, 100.0, 0.0));
        assert!(across.abs() < 1e-9 && above.abs() < 1e-9 && (ahead - 100.0).abs() < 1e-9);
        let shown = framed(LEVEL_NORTH, (0.0, 0.0, 100.0));
        assert_eq!((shown["x"].as_f64(), shown["up"].as_f64()), (Some(0.5), Some(0.0)));
    }

    #[test]
    fn a_point_at_the_edge_of_the_field_reaches_the_edge_of_the_frame() {
        let edge = framed(LEVEL_NORTH, in_camera(LEVEL_NORTH, (100.0, 100.0, 0.0)));
        assert!((edge["x"].as_f64().unwrap() - 1.0).abs() < 1e-9, "45 degrees right with a 90 degree field is the right edge");
        let tilted = Camera { pitch: -45.0, ..LEVEL_NORTH };
        let below = in_camera(tilted, (0.0, 100.0, -100.0));
        assert!(below.0.abs() < 1e-9 && below.1.abs() < 1e-9, "a camera tilted 45 down looks straight at a point 45 below the horizon");
        let east = in_camera(Camera { heading: 90.0, ..LEVEL_NORTH }, (100.0, 0.0, 0.0));
        assert!((east.2 - 100.0).abs() < 1e-9, "turning to 90 looks east");
        let rolled = in_camera(Camera { roll: 90.0, ..LEVEL_NORTH }, (0.0, 100.0, 50.0));
        assert!((rolled.0 + 50.0).abs() < 1e-9, "rolled right wing down a quarter turn, what was above now shows to the left");
    }

    #[test]
    fn a_leg_running_behind_the_camera_is_cut_where_it_leaves_the_view() {
        let (from, to) = clipped((0.0, 0.0, 10.0), (0.0, 0.0, -10.0)).unwrap();
        assert_eq!((from.2, to.2), (10.0, NEAR_M));
        assert_eq!(clipped((0.0, 0.0, -1.0), (0.0, 0.0, -5.0)), None, "a leg wholly behind is not drawn");
    }

    #[test]
    fn home_and_the_plan_are_placed_on_the_live_picture() {
        let drone = (-35.0, 149.0);
        let north_100 = (-35.0 + (100.0 / EARTH_RADIUS_M).to_degrees(), 149.0);
        let south_100 = (-35.0 - (100.0 / EARTH_RADIUS_M).to_degrees(), 149.0);
        let overlays = json!({
            "home": { "latitude": south_100.0, "longitude": south_100.1 },
            "waypoints": [{ "latitude": north_100.0, "longitude": north_100.1, "aboveHome": 50.0, "label": "1" }],
            "route": [{ "latitude": south_100.0, "longitude": south_100.1, "aboveHome": 0.0 }, { "latitude": north_100.0, "longitude": north_100.1, "aboveHome": 50.0 }],
        });
        let shown = ar(drone, 50.0, LEVEL_NORTH, &overlays, &metres);
        assert_eq!(shown["home"], Value::Null, "home behind the camera is not drawn");
        let waypoint = &shown["waypoints"][0];
        assert!((waypoint["x"].as_f64().unwrap() - 0.5).abs() < 1e-6 && waypoint["up"].as_f64().unwrap().abs() < 1e-6, "a waypoint dead ahead at the camera's height is centred");
        assert_eq!(shown["route"].as_array().map(Vec::len), Some(1), "the leg from home behind to the waypoint ahead is drawn from where it enters the view");
        let back = ar(drone, 50.0, Camera { heading: 180.0, ..LEVEL_NORTH }, &overlays, &metres);
        assert_eq!((back["home"]["label"].as_str(), back["home"]["text"].as_str()), (Some("H"), Some("100 m")), "facing home it is drawn with its distance");
    }
}
