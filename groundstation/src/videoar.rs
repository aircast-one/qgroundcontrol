use serde_json::{Value, json};

use crate::attitude::{point, raw};
use crate::read::value_number;
use crate::read::{Unit, format_measure};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.coordinate",
    "vehicle.homePosition",
    "vehicle.altitudeRelative",
    "vehicle.heading",
    "vehicle.pitch",
    "vehicle.roll",
    "video.activeVideoSource",
    crate::cameras::CAMERAS_PATH,
    crate::cameras::ACTIVE_PATH,
    "settings.unitsSettings.horizontalDistanceUnits",
    "settings.gimbalControllerSettings.cameraHFov",
    crate::gimbal::GIMBAL_CHANGED,
];

const EARTH_RADIUS_M: f64 = 6_371_000.0;
const NEAR_M: f64 = 1.0;
const HOME_LABEL: &str = "H";
const AIM_LABEL: &str = "Target";
const IMPACT_LABEL: &str = "Drop";
const CAMERA_HFOV: &str = "settings.gimbalControllerSettings.cameraHFov.rawValue";
pub const GROUND_AT: &str = "videoAr.groundAt";
pub const OUTLINE: &str = "videoAr.outline";
const OUTLINE_SEGMENTS: usize = 36;
const REACH_M: f64 = 2000.0;
const STEP_M: f64 = 5.0;
const PICTURE_ACTIONS: [&str; 4] = ["GoTo", "Orbit", "Roi", "Heading"];

type Space = (f64, f64, f64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub heading: f64,
    pub pitch: f64,
    pub roll: f64,
    pub fov: f64,
}

pub fn deps() -> Vec<String> {
    DEPS.iter()
        .chain(crate::orbit::DEPS)
        .map(|dep| dep.to_string())
        .chain(crate::synthview::overlay_deps())
        .collect()
}

fn offset(from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    let east = (to.1 - from.1).to_radians() * EARTH_RADIUS_M * from.0.to_radians().cos();
    let north = (to.0 - from.0).to_radians() * EARTH_RADIUS_M;
    (east, north)
}

fn axes(camera: Camera) -> [Space; 3] {
    let (heading, pitch, roll) = (
        camera.heading.to_radians(),
        camera.pitch.to_radians(),
        camera.roll.to_radians(),
    );
    let forward = (
        heading.sin() * pitch.cos(),
        heading.cos() * pitch.cos(),
        pitch.sin(),
    );
    let level = (heading.cos(), -heading.sin(), 0.0);
    let raised = (
        -heading.sin() * pitch.sin(),
        -heading.cos() * pitch.sin(),
        pitch.cos(),
    );
    let blend = |a: Space, b: Space, (x, y): (f64, f64)| {
        (a.0 * x + b.0 * y, a.1 * x + b.1 * y, a.2 * x + b.2 * y)
    };
    [
        blend(level, raised, (roll.cos(), -roll.sin())),
        blend(raised, level, (roll.cos(), roll.sin())),
        forward,
    ]
}

pub fn in_camera(camera: Camera, (east, north, up): Space) -> Space {
    let [right, upward, forward] = axes(camera);
    let dot = |axis: Space| east * axis.0 + north * axis.1 + up * axis.2;
    (dot(right), dot(upward), dot(forward))
}

pub fn out_of_camera(camera: Camera, (across, above, ahead): Space) -> Space {
    let [right, upward, forward] = axes(camera);
    let along = |pick: fn(Space) -> f64| {
        across * pick(right) + above * pick(upward) + ahead * pick(forward)
    };
    (along(|a| a.0), along(|a| a.1), along(|a| a.2))
}

pub fn ground_hit(height: f64, ray: Space, ground: &dyn Fn(f64, f64) -> f64) -> Option<(f64, f64)> {
    let length = (ray.0.powi(2) + ray.1.powi(2) + ray.2.powi(2)).sqrt();
    let at = |slant: f64| {
        let share = slant / length;
        let (east, north) = (ray.0 * share, ray.1 * share);
        ((east, north), height + ray.2 * share - ground(east, north))
    };
    let path: Vec<(f64, f64)> = (0..=(REACH_M / STEP_M) as usize)
        .map(|step| step as f64 * STEP_M)
        .map(|slant| (slant, at(slant).1))
        .collect();
    path.windows(2)
        .filter(|_| length > 0.0)
        .find(|pair| pair[0].1 > 0.0 && pair[1].1 <= 0.0)
        .map(|pair| at(pair[0].0 + STEP_M * pair[0].1 / (pair[0].1 - pair[1].1)).0)
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
    (
        kept.0 + (gone.0 - kept.0) * share,
        kept.1 + (gone.1 - kept.1) * share,
        NEAR_M,
    )
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
    Some((
        (point["latitude"].as_f64()?, point["longitude"].as_f64()?),
        point["aboveHome"].as_f64().unwrap_or(0.0),
    ))
}

pub fn ar(
    drone: (f64, f64),
    drone_above: f64,
    camera: Camera,
    overlays: &Value,
    distance: &dyn Fn(f64) -> String,
) -> Value {
    let looked = |at: (f64, f64), above: f64| seen(drone, drone_above, camera, at, above);
    let ahead = |space: Space| (space.2 > NEAR_M).then(|| framed(camera, space));
    let marked = |key: &str, label: &str| {
        spot_of(&overlays[key]).and_then(|(at, above)| {
            ahead(looked(at, above)).map(|mut shown| {
                shown["label"] = json!(label);
                shown["text"] = json!(distance(crate::track::distance_m(drone, at)));
                shown
            })
        })
    };
    let home = marked("home", HOME_LABEL);
    let aim = marked("aim", AIM_LABEL);
    let impact = marked("impact", IMPACT_LABEL);
    let placed = |key: &str| spot_of(&overlays[key]).map(|(at, above)| looked(at, above));
    let lead = placed("lead").and_then(ahead);
    let lead_line = placed("aim")
        .zip(placed("lead"))
        .and_then(|(from, to)| clipped(from, to))
        .map(|(from, to)| json!({ "from": framed(camera, from), "to": framed(camera, to) }));
    let waypoints: Vec<Value> = overlays["waypoints"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|point| {
            let (at, above) = spot_of(point)?;
            ahead(looked(at, above)).map(|mut shown| {
                shown["label"] = point["label"].clone();
                shown
            })
        })
        .collect();
    let route: Vec<Space> = overlays["route"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(spot_of)
        .map(|(at, above)| looked(at, above))
        .collect();
    let legs: Vec<Value> = route
        .windows(2)
        .filter_map(|leg| clipped(leg[0], leg[1]))
        .map(|(from, to)| json!({ "from": framed(camera, from), "to": framed(camera, to) }))
        .collect();
    json!({ "kind": "object", "class": "VideoAr", "available": true, "home": home, "aim": aim, "impact": impact, "lead": lead, "leadLine": lead_line, "waypoints": waypoints, "route": legs })
}

pub fn circle_outline(
    drone: (f64, f64),
    drone_above: f64,
    camera: Camera,
    (centre, radius): ((f64, f64), f64),
    segments: usize,
    ground: &dyn Fn((f64, f64)) -> f64,
) -> Vec<Value> {
    (0..=segments)
        .map(|i| {
            let at = crate::surveygrid::at_distance_and_azimuth(
                centre,
                radius.abs(),
                360.0 * i as f64 / segments as f64,
            );
            let space = seen(drone, drone_above, camera, at, ground(at));
            match space.2 > NEAR_M {
                true => framed(camera, space),
                false => Value::Null,
            }
        })
        .collect()
}

fn orbiting(backend: &dyn Backend) -> Option<((f64, f64), f64)> {
    let orbit = crate::orbit::orbit_view(backend, &[]);
    let centre = &orbit["centre"];
    (orbit["orbiting"] == true)
        .then(|| {
            Some((
                (centre["latitude"].as_f64()?, centre["longitude"].as_f64()?),
                orbit["radiusMetres"].as_f64()?,
            ))
        })
        .flatten()
}

pub fn camera(backend: &dyn Backend, gimbals: &Value, fixed_tilt: Option<f64>) -> Option<Camera> {
    let fov = value_number(&backend.value(CAMERA_HFOV)).filter(|fov| *fov > 0.0 && *fov < 180.0)?;
    match fixed_tilt {
        Some(tilt) => Some(Camera {
            heading: raw(backend, "vehicle.heading")?,
            pitch: raw(backend, "vehicle.pitch").unwrap_or(0.0) + tilt,
            roll: raw(backend, "vehicle.roll").unwrap_or(0.0),
            fov,
        }),
        None => crate::synthview::gimbal_aim(gimbals).map(|(heading, pitch)| Camera {
            heading,
            pitch,
            roll: 0.0,
            fov,
        }),
    }
}

fn unavailable() -> Value {
    json!({ "kind": "object", "class": "VideoAr", "available": false })
}

pub fn video_ar(
    backend: &dyn Backend,
    gimbals: &Value,
    overlays: &Value,
    fixed_tilt: Option<f64>,
) -> Value {
    let placed = point(backend, "vehicle.coordinate")
        .zip(point(backend, "vehicle.homePosition"))
        .zip(raw(backend, "vehicle.altitudeRelative"));
    let Some((((drone, _), drone_above), camera)) =
        placed.zip(camera(backend, gimbals, fixed_tilt))
    else {
        return unavailable();
    };
    let unit = Unit::horizontal(backend);
    let distance = |metres: f64| format_measure(unit.show(metres), &unit.name);
    let orbit = orbiting(backend).map(|(centre, radius)| {
        json!({
            "centre": { "latitude": centre.0, "longitude": centre.1 },
            "radiusMetres": radius,
            "outline": circle_outline(drone, drone_above, camera, (centre, radius), OUTLINE_SEGMENTS, &ground_above_home(backend)),
        })
    });
    let shown = ar(drone, drone_above, camera, overlays, &distance);
    Value::Object(
        shown
            .as_object()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .chain([("orbit".to_string(), orbit.unwrap_or(Value::Null))])
            .collect(),
    )
}

fn half_tan(degrees: f64) -> f64 {
    (degrees / 2.0).to_radians().tan()
}

pub fn ground_point(
    drone: (f64, f64),
    height: f64,
    lens: (Camera, Option<f64>),
    aspect: f64,
    (pan, tilt): (f64, f64),
    ground: &dyn Fn((f64, f64)) -> f64,
) -> Option<(f64, f64)> {
    let (camera, vertical_fov) = lens;
    let across = pan * half_tan(camera.fov);
    let above = tilt * vertical_fov.map_or(half_tan(camera.fov) / aspect, half_tan);
    let ray = out_of_camera(camera, (across, above, 1.0));
    let place = |east: f64, north: f64| {
        crate::surveygrid::at_distance_and_azimuth(
            drone,
            east.hypot(north),
            east.atan2(north).to_degrees().rem_euclid(360.0),
        )
    };
    ground_hit(height, ray, &|east, north| ground(place(east, north)))
        .map(|(east, north)| place(east, north))
}

fn shown_lens(backend: &dyn Backend, synthetic: bool) -> Option<(Camera, Option<f64>)> {
    let angle = |degrees: Option<f64>| degrees.filter(|fov| *fov > 0.0 && *fov < 180.0);
    match synthetic {
        true => {
            let pose = crate::synthview::synthetic_view(backend, &[]);
            let read = |key: &str| pose[key].as_f64().filter(|v| v.is_finite());
            Some((
                Camera {
                    heading: read("heading")?,
                    pitch: read("pitch")?,
                    roll: read("roll")?,
                    fov: angle(read("fov"))?,
                },
                None,
            ))
        }
        false => {
            let (horizontal, vertical, _) = crate::lens::field_of_view(backend);
            camera(
                backend,
                &crate::gimbal::gimbal_view(backend, &[]),
                crate::cameras::active_tilt(backend),
            )
            .map(|aimed| {
                (
                    Camera {
                        fov: angle(horizontal).unwrap_or(aimed.fov),
                        ..aimed
                    },
                    angle(vertical),
                )
            })
        }
    }
}

fn ground_above_home(backend: &dyn Backend) -> impl Fn((f64, f64)) -> f64 {
    let home = point(backend, "vehicle.homePosition")
        .and_then(|(latitude, longitude)| crate::terrainservice::height(latitude, longitude));
    move |(latitude, longitude)| {
        home.zip(crate::terrainservice::height(latitude, longitude))
            .map_or(0.0, |(home, here)| here - home)
    }
}

pub fn ground_at(backend: &dyn Backend, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let number = |index: usize| {
        given
            .get(index)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
    };
    let fraction = |index: usize| number(index).filter(|v| (-1.0..=1.0).contains(v));
    let refused =
        |token: &str, reason: String| json!({ "ok": false, "refusal": token, "reason": reason });
    let Some(((pan, tilt), aspect)) = fraction(0)
        .zip(fraction(1))
        .zip(number(2).filter(|aspect| *aspect > 0.0))
    else {
        return refused(
            "badPoint",
            "Give the point as fractions of the picture from -1 to 1, then the picture's width over its height.".to_string(),
        );
    };
    let synthetic = given.get(3).and_then(Value::as_bool).unwrap_or(false);
    let placed = point(backend, "vehicle.coordinate").zip(raw(backend, "vehicle.altitudeRelative"));
    let Some(((drone, height), lens)) = placed.zip(shown_lens(backend, synthetic)) else {
        return refused(
            "notPlaced",
            "Where the aircraft is and where its camera looks are not known yet.".to_string(),
        );
    };
    let unit = Unit::horizontal(backend);
    match ground_point(
        drone,
        height,
        lens,
        aspect,
        (pan, tilt),
        &ground_above_home(backend),
    ) {
        Some((latitude, longitude)) => {
            json!({ "ok": true, "latitude": latitude, "longitude": longitude, "actions": PICTURE_ACTIONS })
        }
        None => refused(
            "notGround",
            format!(
                "That is above the horizon, or farther than {}.",
                format_measure(unit.show(REACH_M), &unit.name)
            ),
        ),
    }
}

pub fn outline(backend: &dyn Backend, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let number = |index: usize| {
        given
            .get(index)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
    };
    let refused = |token: &str, reason: &str| json!({ "ok": false, "refusal": token, "reason": reason, "outline": [] });
    let circle = number(0)
        .filter(|lat| (-90.0..=90.0).contains(lat))
        .zip(number(1).filter(|lon| (-180.0..=180.0).contains(lon)))
        .zip(number(2).filter(|radius| *radius != 0.0));
    let segments = match given.get(3) {
        None | Some(Value::Null) => Some(OUTLINE_SEGMENTS),
        Some(count) => count
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| *n >= 3),
    };
    let Some((circle, segments)) = circle.zip(segments) else {
        return refused(
            "badCircle",
            "Give the circle as a latitude, a longitude and a radius in metres, then how many segments from 3 up.",
        );
    };
    let placed = point(backend, "vehicle.coordinate").zip(raw(backend, "vehicle.altitudeRelative"));
    let aimed = camera(
        backend,
        &crate::gimbal::gimbal_view(backend, &[]),
        crate::cameras::active_tilt(backend),
    );
    let Some(((drone, drone_above), camera)) = placed.zip(aimed) else {
        return refused(
            "notPlaced",
            "Where the aircraft is and where its camera looks are not known yet.",
        );
    };
    json!({
        "ok": true,
        "refusal": Value::Null,
        "reason": Value::Null,
        "outline": circle_outline(drone, drone_above, camera, circle, segments, &ground_above_home(backend)),
    })
}

pub fn video_ar_view(backend: &dyn Backend, _args: &[String]) -> Value {
    video_ar(
        backend,
        &crate::gimbal::gimbal_view(backend, &[]),
        &crate::synthview::overlays_view(backend, &[]),
        crate::cameras::active_tilt(backend),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    struct Fake(BTreeMap<&'static str, Value>);

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            self.0
                .get(path)
                .map_or_else(
                    || json!({ "kind": "null" }),
                    |v| match v {
                        Value::Object(_) => v.clone(),
                        _ => json!({ "kind": "fact", "rawValue": v, "value": v }),
                    },
                )
                .to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            self.get(p)
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            String::new()
        }
        fn watch(&self, _p: &[String]) {}
    }

    fn flying(fov: Option<f64>) -> Fake {
        let base = [
            (
                "vehicle.coordinate",
                json!({ "latitude": -35.0, "longitude": 149.0, "valid": true }),
            ),
            (
                "vehicle.homePosition",
                json!({ "latitude": -35.001, "longitude": 149.0, "valid": true }),
            ),
            ("vehicle.altitudeRelative", json!(50.0)),
        ];
        Fake(
            base.into_iter()
                .chain(fov.map(|fov| (CAMERA_HFOV, json!(fov))))
                .collect(),
        )
    }

    fn gimbal(yaw: f64, pitch: f64) -> Value {
        json!({ "gimbals": [{ "active": true, "absoluteYaw": yaw, "pitch": pitch }] })
    }

    const LEVEL_NORTH: Camera = Camera {
        heading: 0.0,
        pitch: 0.0,
        roll: 0.0,
        fov: 90.0,
    };

    fn metres(value: f64) -> String {
        format!("{value:.0} m")
    }

    #[test]
    fn a_point_straight_ahead_sits_in_the_middle_of_the_frame() {
        let (across, above, ahead) = in_camera(LEVEL_NORTH, (0.0, 100.0, 0.0));
        assert!(across.abs() < 1e-9 && above.abs() < 1e-9 && (ahead - 100.0).abs() < 1e-9);
        let shown = framed(LEVEL_NORTH, (0.0, 0.0, 100.0));
        assert_eq!(
            (shown["x"].as_f64(), shown["up"].as_f64()),
            (Some(0.5), Some(0.0))
        );
    }

    #[test]
    fn a_point_at_the_edge_of_the_field_reaches_the_edge_of_the_frame() {
        let edge = framed(LEVEL_NORTH, in_camera(LEVEL_NORTH, (100.0, 100.0, 0.0)));
        assert!(
            (edge["x"].as_f64().unwrap() - 1.0).abs() < 1e-9,
            "45 degrees right with a 90 degree field is the right edge"
        );
        let tilted = Camera {
            pitch: -45.0,
            ..LEVEL_NORTH
        };
        let below = in_camera(tilted, (0.0, 100.0, -100.0));
        assert!(
            below.0.abs() < 1e-9 && below.1.abs() < 1e-9,
            "a camera tilted 45 down looks straight at a point 45 below the horizon"
        );
        let east = in_camera(
            Camera {
                heading: 90.0,
                ..LEVEL_NORTH
            },
            (100.0, 0.0, 0.0),
        );
        assert!((east.2 - 100.0).abs() < 1e-9, "turning to 90 looks east");
        let rolled = in_camera(
            Camera {
                roll: 90.0,
                ..LEVEL_NORTH
            },
            (0.0, 100.0, 50.0),
        );
        assert!(
            (rolled.0 + 50.0).abs() < 1e-9,
            "rolled right wing down a quarter turn, what was above now shows to the left"
        );
    }

    #[test]
    fn the_gimbal_aims_the_camera_and_the_operators_field_of_view_frames_it() {
        assert_eq!(
            camera(&flying(Some(62.0)), &gimbal(200.0, -30.0), None),
            Some(Camera {
                heading: 200.0,
                pitch: -30.0,
                roll: 0.0,
                fov: 62.0
            })
        );
        assert_eq!(
            camera(&flying(Some(62.0)), &json!({ "gimbals": [] }), None),
            None,
            "a camera with no gimbal and no tilt set could only be guessed at"
        );
        assert_eq!(
            camera(&flying(None), &gimbal(200.0, -30.0), None),
            None,
            "without a field of view the marks cannot be scaled to the picture"
        );
        assert_eq!(
            camera(&flying(Some(0.0)), &gimbal(200.0, -30.0), None),
            None
        );
    }

    #[test]
    fn a_camera_fixed_to_the_frame_turns_with_the_drone_at_its_set_tilt() {
        let banked = Fake(
            flying(Some(80.0))
                .0
                .into_iter()
                .chain([
                    ("vehicle.heading", json!(90.0)),
                    ("vehicle.pitch", json!(5.0)),
                    ("vehicle.roll", json!(-10.0)),
                ])
                .collect(),
        );
        assert_eq!(
            camera(&banked, &json!({ "gimbals": [] }), Some(-30.0)),
            Some(Camera {
                heading: 90.0,
                pitch: -25.0,
                roll: -10.0,
                fov: 80.0
            })
        );
        assert_eq!(
            camera(&banked, &gimbal(200.0, -45.0), Some(-30.0)).map(|aimed| aimed.heading),
            Some(90.0),
            "a tilt set on the camera on screen means it is not the one on the gimbal"
        );
        assert_eq!(
            camera(&flying(Some(80.0)), &json!({ "gimbals": [] }), Some(-30.0)),
            None,
            "without a heading a fixed camera cannot be pointed"
        );
    }

    #[test]
    fn the_marks_are_withheld_until_the_drone_its_home_and_the_camera_are_known() {
        let overlays = json!({ "home": { "latitude": -35.001, "longitude": 149.0 } });
        assert_eq!(
            video_ar(
                &Fake(BTreeMap::new()),
                &gimbal(180.0, -20.0),
                &overlays,
                None
            )["available"],
            false
        );
        assert_eq!(
            video_ar(
                &flying(Some(80.0)),
                &json!({ "gimbals": [] }),
                &overlays,
                None
            )["available"],
            false
        );
        let shown = video_ar(&flying(Some(80.0)), &gimbal(180.0, -20.0), &overlays, None);
        assert_eq!(
            (
                shown["available"].as_bool(),
                shown["home"]["label"].as_str()
            ),
            (Some(true), Some(HOME_LABEL)),
            "home behind the drone shows once the gimbal looks back at it"
        );
    }

    #[test]
    fn a_ground_circle_below_is_outlined_on_the_picture_and_one_behind_is_not() {
        let drone = (-35.0, 149.0);
        let looking_down = Camera {
            pitch: -90.0,
            ..LEVEL_NORTH
        };
        let flat = |_: (f64, f64)| 0.0;
        let below = circle_outline(drone, 50.0, looking_down, (drone, 20.0), 4, &flat);
        assert_eq!(
            below.len(),
            5,
            "closed: the first point is repeated at the end"
        );
        assert_eq!(below[0], below[4]);
        let north = &below[0];
        assert!(
            (north["x"].as_f64().unwrap() - 0.5).abs() < 1e-6
                && (north["up"].as_f64().unwrap() - 0.2).abs() < 1e-3,
            "20 m north of a camera 50 m up looking straight down at a 90 degree lens: {north}"
        );
        assert!(
            (below[1]["x"].as_f64().unwrap() - 0.7).abs() < 1e-3,
            "east is to the right"
        );
        let south = (-35.0 - (300.0 / EARTH_RADIUS_M).to_degrees(), 149.0);
        assert!(
            circle_outline(drone, 50.0, LEVEL_NORTH, (south, 20.0), 8, &flat)
                .iter()
                .all(Value::is_null),
            "a circle behind the camera has nothing to draw"
        );
    }

    #[test]
    fn the_running_orbit_is_outlined_in_the_view_and_a_circle_is_outlined_on_request() {
        let orbiting = Fake(
            flying(Some(80.0))
                .0
                .into_iter()
                .chain([
                    ("vehicle", json!({ "kind": "object", "orbitActive": true })),
                    (
                        "vehicle.vehicleLinkManager",
                        json!({ "kind": "object", "communicationLost": false }),
                    ),
                    (
                        "vehicle.orbitMapCircle",
                        json!({ "kind": "object", "center": { "latitude": -34.999, "longitude": 149.0 }, "clockwiseRotation": true, "facts": [{ "property": "radius", "rawValue": 30.0 }] }),
                    ),
                ])
                .collect(),
        );
        let shown = video_ar(&orbiting, &gimbal(0.0, -30.0), &json!({}), None);
        assert_eq!(shown["orbit"]["radiusMetres"], 30.0);
        assert_eq!(
            shown["orbit"]["outline"].as_array().map(Vec::len),
            Some(OUTLINE_SEGMENTS + 1)
        );
        assert_eq!(
            video_ar(&flying(Some(80.0)), &gimbal(0.0, -30.0), &json!({}), None)["orbit"],
            Value::Null,
            "no orbit, no circle"
        );
        assert_eq!(outline(&orbiting, "[-35.0, 149.0]")["refusal"], "badCircle");
        assert_eq!(
            outline(&orbiting, "[-35.0, 149.0, 30.0, 2]")["refusal"],
            "badCircle"
        );
        assert_eq!(
            outline(&Fake(BTreeMap::new()), "[-35.0, 149.0, 30.0]")["refusal"],
            "notPlaced"
        );
    }

    #[test]
    fn a_wider_field_of_view_draws_the_same_point_nearer_the_middle() {
        let point = (100.0, 100.0, 0.0);
        let narrow = framed(LEVEL_NORTH, in_camera(LEVEL_NORTH, point));
        let wide = Camera {
            fov: 120.0,
            ..LEVEL_NORTH
        };
        let widened = framed(wide, in_camera(wide, point));
        assert!(widened["x"].as_f64().unwrap() < narrow["x"].as_f64().unwrap());
    }

    #[test]
    fn a_leg_running_behind_the_camera_is_cut_where_it_leaves_the_view() {
        let (from, to) = clipped((0.0, 0.0, 10.0), (0.0, 0.0, -10.0)).unwrap();
        assert_eq!((from.2, to.2), (10.0, NEAR_M));
        assert_eq!(
            clipped((0.0, 0.0, -1.0), (0.0, 0.0, -5.0)),
            None,
            "a leg wholly behind is not drawn"
        );
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
        assert_eq!(
            shown["home"],
            Value::Null,
            "home behind the camera is not drawn"
        );
        let waypoint = &shown["waypoints"][0];
        assert!(
            (waypoint["x"].as_f64().unwrap() - 0.5).abs() < 1e-6
                && waypoint["up"].as_f64().unwrap().abs() < 1e-6,
            "a waypoint dead ahead at the camera's height is centred"
        );
        assert_eq!(
            shown["route"].as_array().map(Vec::len),
            Some(1),
            "the leg from home behind to the waypoint ahead is drawn from where it enters the view"
        );
        let back = ar(
            drone,
            50.0,
            Camera {
                heading: 180.0,
                ..LEVEL_NORTH
            },
            &overlays,
            &metres,
        );
        assert_eq!(
            (
                back["home"]["label"].as_str(),
                back["home"]["text"].as_str()
            ),
            (Some("H"), Some("100 m")),
            "facing home it is drawn with its distance"
        );
    }

    #[test]
    fn the_drop_target_and_the_predicted_impact_are_placed_on_the_ground_ahead() {
        let drone = (-35.0, 149.0);
        let north = |metres: f64| (-35.0 + (metres / EARTH_RADIUS_M).to_degrees(), 149.0);
        let overlays = json!({
            "aim": { "latitude": north(100.0).0, "longitude": 149.0 },
            "impact": { "latitude": north(50.0).0, "longitude": 149.0 },
        });
        let looking_down = Camera {
            pitch: -45.0,
            ..LEVEL_NORTH
        };
        let shown = ar(drone, 50.0, looking_down, &overlays, &metres);
        let up = |key: &str| shown[key]["up"].as_f64().unwrap();
        assert_eq!(
            (
                shown["aim"]["label"].as_str(),
                shown["aim"]["text"].as_str()
            ),
            (Some("Target"), Some("100 m"))
        );
        assert!(
            up("impact").abs() < 1e-6,
            "50 m ahead from 50 m up is dead centre of a 45° down camera, got {}",
            up("impact")
        );
        assert!(
            up("aim") > 0.0,
            "the target farther ahead sits above the impact point in the picture"
        );
        assert_eq!(
            ar(drone, 50.0, LEVEL_NORTH, &json!({}), &metres)["aim"],
            Value::Null,
            "no target, no mark"
        );
    }

    #[test]
    fn the_lead_point_is_drawn_ahead_of_the_target_with_a_line_from_the_target_to_it() {
        let drone = (-35.0, 149.0);
        let north = |metres: f64| (-35.0 + (metres / EARTH_RADIUS_M).to_degrees(), 149.0);
        let overlays = json!({
            "aim": { "latitude": north(60.0).0, "longitude": 149.0 },
            "lead": { "latitude": north(75.0).0, "longitude": 149.0 },
        });
        let looking_down = Camera {
            pitch: -45.0,
            ..LEVEL_NORTH
        };
        let shown = ar(drone, 50.0, looking_down, &overlays, &metres);
        let (aim_up, lead_up) = (
            shown["aim"]["up"].as_f64().unwrap(),
            shown["lead"]["up"].as_f64().unwrap(),
        );
        assert!(
            lead_up > aim_up,
            "the car is heading away, so where it will be sits higher in the picture: {aim_up} vs {lead_up}"
        );
        assert!(
            (shown["leadLine"]["from"]["up"].as_f64().unwrap() - aim_up).abs() < 1e-9
                && (shown["leadLine"]["to"]["up"].as_f64().unwrap() - lead_up).abs() < 1e-9,
            "the line runs from the target to the lead: {}",
            shown["leadLine"]
        );
        assert_eq!(
            ar(
                drone,
                50.0,
                looking_down,
                &json!({ "aim": overlays["aim"] }),
                &metres
            )["leadLine"],
            Value::Null,
            "no lead, no line"
        );
    }

    const DRONE: (f64, f64) = (-35.0, 149.0);

    fn flat(_: (f64, f64)) -> f64 {
        0.0
    }

    fn looking(heading: f64, pitch: f64, fov: f64) -> (Camera, Option<f64>) {
        (
            Camera {
                heading,
                pitch,
                roll: 0.0,
                fov,
            },
            None,
        )
    }

    fn ahead_and_across(at: (f64, f64)) -> (f64, f64) {
        let (east, north) = offset(DRONE, at);
        (north, east)
    }

    fn close(found: (f64, f64), wanted: (f64, f64)) -> bool {
        (found.0 - wanted.0).abs() < 0.05 && (found.1 - wanted.1).abs() < 0.05
    }

    #[test]
    fn a_direction_on_the_picture_turns_back_into_the_direction_in_the_world() {
        let banked = Camera {
            heading: 130.0,
            pitch: -35.0,
            roll: 12.0,
            fov: 60.0,
        };
        let world = (30.0, -45.0, -20.0);
        let back = out_of_camera(banked, in_camera(banked, world));
        assert!(
            (back.0 - world.0).abs() < 1e-9
                && (back.1 - world.1).abs() < 1e-9
                && (back.2 - world.2).abs() < 1e-9
        );
    }

    #[test]
    fn the_middle_of_a_picture_looking_45_down_from_20_m_is_20_m_ahead() {
        let hit = ground_point(
            DRONE,
            20.0,
            looking(0.0, -45.0, 70.0),
            16.0 / 9.0,
            (0.0, 0.0),
            &flat,
        );
        assert!(
            close(ahead_and_across(hit.unwrap()), (20.0, 0.0)),
            "{:?}",
            hit.map(ahead_and_across)
        );
        let east = ground_point(
            DRONE,
            20.0,
            looking(90.0, -45.0, 70.0),
            16.0 / 9.0,
            (0.0, 0.0),
            &flat,
        );
        assert!(
            close(ahead_and_across(east.unwrap()), (0.0, 20.0)),
            "turned east it lands 20 m east"
        );
    }

    #[test]
    fn looking_straight_down_the_middle_of_the_picture_is_under_the_drone() {
        let hit = ground_point(
            DRONE,
            20.0,
            looking(0.0, -90.0, 70.0),
            16.0 / 9.0,
            (0.0, 0.0),
            &flat,
        );
        assert!(close(ahead_and_across(hit.unwrap()), (0.0, 0.0)));
    }

    #[test]
    fn the_sky_and_ground_past_2_km_are_not_offered_as_a_point() {
        assert_eq!(
            ground_point(
                DRONE,
                20.0,
                looking(0.0, 0.0, 70.0),
                16.0 / 9.0,
                (0.0, 0.5),
                &flat
            ),
            None,
            "the upper half of a level picture is sky"
        );
        assert_eq!(
            ground_point(
                DRONE,
                20.0,
                looking(0.0, -0.3, 70.0),
                16.0 / 9.0,
                (0.0, 0.0),
                &flat
            ),
            None,
            "0.3 degrees down from 20 m meets the ground nearly 4 km out"
        );
        assert!(
            ground_point(
                DRONE,
                20.0,
                looking(0.0, -1.0, 70.0),
                16.0 / 9.0,
                (0.0, 0.0),
                &flat
            )
            .is_some()
        );
    }

    #[test]
    fn rising_ground_is_met_before_the_flat_ground_would_be() {
        let hill = |at: (f64, f64)| match ahead_and_across(at).0 > 15.0 {
            true => 10.0,
            false => 0.0,
        };
        let hit = ground_point(
            DRONE,
            20.0,
            looking(0.0, -30.0, 70.0),
            16.0 / 9.0,
            (0.0, 0.0),
            &hill,
        );
        let (ahead, _) = ahead_and_across(hit.unwrap());
        assert!(
            (ahead - 10.0 / (30.0f64).to_radians().tan()).abs() < 0.5,
            "a 10 m rise 15 m out meets the ray 10 m below the drone, at {ahead}"
        );
        let valley = ground_point(
            DRONE,
            20.0,
            looking(0.0, -45.0, 70.0),
            16.0 / 9.0,
            (0.0, 0.0),
            &|_| -10.0,
        );
        assert!(
            close(ahead_and_across(valley.unwrap()), (30.0, 0.0)),
            "ground 10 m below home is 30 m away at 45 degrees"
        );
    }

    #[test]
    fn a_long_press_lands_where_a_tap_would_aim_the_gimbal() {
        let (pan, tilt, h_fov, v_fov, pitch) = (0.6, -0.4, 60.0, 34.0, -30.0);
        let (yaw, aimed) = crate::gimbal::aim_through_lens(pan, tilt, h_fov, v_fov, pitch);
        let hit = ground_point(
            DRONE,
            20.0,
            (
                Camera {
                    heading: 0.0,
                    pitch: f64::from(pitch),
                    roll: 0.0,
                    fov: f64::from(h_fov),
                },
                Some(f64::from(v_fov)),
            ),
            16.0 / 9.0,
            (f64::from(pan), f64::from(tilt)),
            &flat,
        )
        .unwrap();
        let (ahead, across) = ahead_and_across(hit);
        let reach = 20.0 / f64::from(-aimed).to_radians().tan();
        assert!(
            (across.atan2(ahead).to_degrees() - f64::from(yaw)).abs() < 0.05
                && (ahead.hypot(across) - reach).abs() < 0.1,
            "bearing {} reach {} against the aim's {yaw} and {reach}",
            across.atan2(ahead).to_degrees(),
            ahead.hypot(across)
        );
    }

    #[test]
    fn without_a_measured_vertical_field_the_picture_shape_sets_it() {
        let lens = looking(0.0, -45.0, 90.0);
        let square = ground_point(DRONE, 20.0, lens, 1.0, (0.0, -1.0), &flat);
        assert!(
            close(ahead_and_across(square.unwrap()), (0.0, 0.0)),
            "the bottom edge of a square 90 degree picture looking 45 down is straight below"
        );
        let wide = ground_point(DRONE, 20.0, lens, 2.0, (0.0, -1.0), &flat).map(ahead_and_across);
        let bottom_edge = (45.0 + 0.5f64.atan().to_degrees()).to_radians();
        assert!(
            wide.is_some_and(|(ahead, _)| (ahead - 20.0 / bottom_edge.tan()).abs() < 0.05),
            "a picture twice as wide as tall spans half the tangent above and below the middle"
        );
    }

    #[test]
    fn a_ground_question_without_a_point_or_an_aircraft_says_why() {
        let empty = Fake(BTreeMap::new());
        assert_eq!(ground_at(&empty, "[]")["refusal"], "badPoint");
        assert_eq!(ground_at(&empty, "[0, 2, 1.7]")["refusal"], "badPoint");
        assert_eq!(
            ground_at(&empty, "[0, 0, 1.7, true]")["refusal"],
            "notPlaced"
        );
    }
}
