use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};
use web_time::Instant;

use crate::router::Backend;

pub const MAP3D_PAN: &str = "map3d.pan";
pub const MAP3D_TURN: &str = "map3d.turn";
pub const MAP3D_TILT: &str = "map3d.tilt";
pub const MAP3D_ZOOM: &str = "map3d.zoom";
pub const MAP3D_FOLLOW: &str = "map3d.follow";
pub const MAP3D_NORTH: &str = "map3d.north";
pub const MAP3D_OPEN: &str = "map3d.open";
pub const MAP3D_LOOK: &str = "map3d.look";
pub const MAP3D_PICK: &str = "map3d.pick";
pub const MAP3D_FLING: &str = "map3d.fling";
pub const MAP3D_SPIN: &str = "map3d.spin";
pub const MAP3D_STOP: &str = "map3d.stop";
pub const MAP3D_CHANGED: &str = "core.map3d@changed";
pub const DEPS: &[&str] = &[
    MAP3D_CHANGED,
    "vehicles.activeVehicleAvailable",
    "vehicle.coordinate",
];

pub const FOV_DEG: f64 = 60.0;
const TILT_DEG: (f64, f64) = (10.0, 89.0);
const RANGE_M: (f64, f64) = (30.0, 30_000.0);
const FLATTEST_SINE: f64 = 0.25;
const TILT_PER_VIEW_HEIGHT_DEG: f64 = 90.0;
const FOLLOW_RANGE_PER_HEIGHT: f64 = 4.0;
const FOLLOW_RANGE_M: (f64, f64) = (150.0, 1000.0);
const GLIDE_DECAY_S: f64 = 0.35;
const GLIDE_SPEED_PX_S: (f64, f64) = (60.0, 5000.0);
const GLIDE_TURN_DEG_S: (f64, f64) = (30.0, 360.0);
const GLIDE_ZOOM_PER_S: (f64, f64) = (0.5, 4.0);
const GLIDE_SETTLED_S: f64 = 8.0 * GLIDE_DECAY_S;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    pub heading: f64,
    pub tilt: f64,
    pub range: f64,
    pub focus: Option<(f64, f64)>,
    pub resting: bool,
    pub framed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aircraft {
    pub at: (f64, f64),
    pub heading: f64,
}

pub const START: Orbit = Orbit {
    heading: 0.0,
    tilt: 15.0,
    range: 500.0,
    focus: None,
    resting: false,
    framed: true,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Motion {
    Pan {
        right: f64,
        down: f64,
        width: f64,
    },
    Spin {
        clockwise: f64,
        up: f64,
        height: f64,
        zoom: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glide {
    from: Orbit,
    motion: Motion,
    at: Instant,
}

static ORBIT: Mutex<Orbit> = Mutex::new(START);
static GLIDE: Mutex<Option<Glide>> = Mutex::new(None);
static CHANGED: AtomicBool = AtomicBool::new(false);
static SHOWN_RANGE: AtomicU64 = AtomicU64::new(0);

impl Orbit {
    pub fn around(self, aircraft: bool) -> Orbit {
        let focus = self.focus.filter(|_| !(self.resting && aircraft));
        Orbit {
            focus,
            resting: self.resting && focus.is_some(),
            ..self
        }
    }

    pub fn at_height(self, above_ground: Option<f64>) -> Orbit {
        match (self.framed, self.focus, above_ground) {
            (true, None, Some(height)) => Orbit {
                range: (FOLLOW_RANGE_PER_HEIGHT * height.max(0.0))
                    .clamp(FOLLOW_RANGE_M.0, FOLLOW_RANGE_M.1),
                ..self
            },
            _ => self,
        }
    }
}

pub fn shown(range: f64) {
    SHOWN_RANGE.store(range.to_bits(), Ordering::Relaxed);
}

fn shown_range() -> Option<f64> {
    Some(f64::from_bits(SHOWN_RANGE.load(Ordering::Relaxed))).filter(|range| *range > 0.0)
}

pub fn owns(path: &str) -> bool {
    [
        MAP3D_PAN,
        MAP3D_TURN,
        MAP3D_TILT,
        MAP3D_ZOOM,
        MAP3D_FOLLOW,
        MAP3D_NORTH,
        MAP3D_OPEN,
        MAP3D_LOOK,
        MAP3D_PICK,
        MAP3D_FLING,
        MAP3D_SPIN,
        MAP3D_STOP,
    ]
    .contains(&path)
}

pub fn orbit() -> Orbit {
    let held = *ORBIT.lock().unwrap_or_else(PoisonError::into_inner);
    GLIDE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .map_or(held, |glide| {
            let now = Instant::now();
            if glide.reshapes_view(now) {
                CHANGED.store(true, Ordering::SeqCst);
            }
            glided(&glide, now)
        })
}

impl Glide {
    fn reshapes_view(&self, now: Instant) -> bool {
        matches!(self.motion, Motion::Spin { .. })
            && now.saturating_duration_since(self.at).as_secs_f64() < GLIDE_SETTLED_S
    }
}

pub fn glided(glide: &Glide, now: Instant) -> Orbit {
    let elapsed = now.saturating_duration_since(glide.at).as_secs_f64();
    let reach = GLIDE_DECAY_S * (1.0 - (-elapsed / GLIDE_DECAY_S).exp());
    match glide.motion {
        Motion::Pan { right, down, width } => glide.from.focus.map_or(glide.from, |spot| {
            panned(glide.from, spot, [right * reach, down * reach, width])
        }),
        Motion::Spin {
            clockwise,
            up,
            height,
            zoom,
        } => {
            let swung = tilted(turned(glide.from, clockwise * reach), [up * reach, height]);
            if zoom == 0.0 {
                swung
            } else {
                zoomed(swung, (zoom * reach).exp())
            }
        }
    }
}

pub fn flung(orbit: Orbit, [right, down, width]: [f64; 3], at: Instant) -> Option<Glide> {
    let speed = right.hypot(down);
    let scale = speed.min(GLIDE_SPEED_PX_S.1) / speed.max(f64::EPSILON);
    (speed >= GLIDE_SPEED_PX_S.0 && orbit.focus.is_some() && width > 0.0).then_some(Glide {
        from: orbit,
        motion: Motion::Pan {
            right: right * scale,
            down: down * scale,
            width,
        },
        at,
    })
}

pub fn spun(orbit: Orbit, [clockwise, up, height, zoom]: [f64; 4], at: Instant) -> Option<Glide> {
    let clockwise = brisk(clockwise, GLIDE_TURN_DEG_S);
    let up = brisk(up, GLIDE_SPEED_PX_S);
    let zoom = brisk(zoom, GLIDE_ZOOM_PER_S);
    (height > 0.0 && [clockwise, up, zoom].iter().any(|rate| *rate != 0.0)).then_some(Glide {
        from: orbit,
        motion: Motion::Spin {
            clockwise,
            up,
            height,
            zoom,
        },
        at,
    })
}

fn brisk(rate: f64, (least, most): (f64, f64)) -> f64 {
    if rate.abs() < least {
        0.0
    } else {
        rate.clamp(-most, most)
    }
}

pub fn take_changed() -> bool {
    CHANGED.swap(false, Ordering::SeqCst)
}

pub fn aircraft(backend: &dyn Backend) -> Option<Aircraft> {
    backend.point("vehicle.coordinate").map(|at| Aircraft {
        at,
        heading: backend.raw("vehicle.heading").unwrap_or(0.0),
    })
}

pub fn map3d_view(backend: &dyn Backend, _args: &[String]) -> Value {
    view_of(orbit(), aircraft(backend).is_some())
}

fn view_of(orbit: Orbit, aircraft: bool) -> Value {
    json!({
        "kind": "object",
        "class": "Map3d",
        "heading": orbit.heading,
        "tilt": orbit.tilt,
        "range": orbit.range,
        "following": orbit.around(aircraft).focus.is_none(),
        "aircraft": aircraft,
    })
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
        resting: false,
        framed: false,
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
        framed: false,
        ..orbit
    }
}

pub fn opened(orbit: Orbit, aircraft: Option<Aircraft>, map_centre: Option<(f64, f64)>) -> Orbit {
    match aircraft {
        Some(aircraft) if orbit == START => Orbit {
            heading: aircraft.heading.rem_euclid(360.0),
            ..orbit
        },
        Some(_) => orbit.around(true),
        None => Orbit {
            focus: orbit.focus.or(map_centre),
            resting: orbit.focus.map_or(map_centre.is_some(), |_| orbit.resting),
            ..orbit
        },
    }
}

pub fn moved(orbit: Orbit, path: &str, given: &[f64], aircraft: Option<Aircraft>) -> Option<Orbit> {
    match (path, given) {
        (MAP3D_PAN, [right, down, width]) => orbit
            .around(aircraft.is_some())
            .focus
            .or(aircraft.map(|aircraft| aircraft.at))
            .map(|from| panned(orbit, from, [*right, *down, *width])),
        (MAP3D_TURN, [clockwise]) => Some(turned(orbit, *clockwise)),
        (MAP3D_TILT, [up, height]) => Some(tilted(orbit, [*up, *height])),
        (MAP3D_ZOOM, [scale]) if *scale > 0.0 => Some(zoomed(orbit, *scale)),
        (MAP3D_FOLLOW, []) => Some(Orbit {
            focus: None,
            resting: false,
            framed: true,
            ..orbit
        }),
        (MAP3D_NORTH, []) => Some(Orbit {
            heading: 0.0,
            ..orbit
        }),
        (MAP3D_OPEN, []) => Some(opened(orbit, aircraft, None)),
        (MAP3D_OPEN, [latitude, longitude]) if on_globe(*latitude, *longitude) => {
            Some(opened(orbit, aircraft, Some((*latitude, *longitude))))
        }
        (MAP3D_LOOK, [latitude, longitude]) if on_globe(*latitude, *longitude) => Some(Orbit {
            focus: Some((*latitude, *longitude)),
            resting: false,
            framed: false,
            ..orbit
        }),
        _ => None,
    }
}

fn on_globe(latitude: f64, longitude: f64) -> bool {
    (-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude)
}

fn numbers(args: &str) -> Vec<f64> {
    let parsed: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    crate::fenceedit::point(parsed.get(0))
        .map(|(latitude, longitude)| vec![latitude, longitude])
        .or_else(|| {
            parsed
                .as_array()
                .and_then(|items| items.iter().map(Value::as_f64).collect::<Option<Vec<_>>>())
        })
        .unwrap_or_default()
        .into_iter()
        .filter(|number| number.is_finite())
        .collect()
}

#[cfg(feature = "synthetic")]
fn pick(given: &[f64]) -> Value {
    crate::synthetic::pick::run(given)
}

#[cfg(not(feature = "synthetic"))]
fn pick(_given: &[f64]) -> Value {
    json!({ "ok": false, "reason": "This build has no 3D map." })
}

pub fn run(path: &str, args: &str, aircraft: Option<Aircraft>) -> Value {
    let given = numbers(args);
    if path == MAP3D_PICK {
        return pick(&given);
    }
    let mut held = ORBIT.lock().unwrap_or_else(PoisonError::into_inner);
    let mut glide = GLIDE.lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    *held = glide.take().map_or(*held, |gliding| glided(&gliding, now));
    match (path, given.as_slice()) {
        (MAP3D_STOP, _) => return json!({ "ok": true }),
        (MAP3D_FLING, [right, down, width]) => {
            *glide = flung(*held, [*right, *down, *width], now);
            return json!({ "ok": true, "gliding": glide.is_some() });
        }
        (MAP3D_SPIN, [clockwise, up, height, zoom]) => {
            let from = if held.framed && *zoom != 0.0 {
                Orbit {
                    range: shown_range().unwrap_or(held.range),
                    ..*held
                }
            } else {
                *held
            };
            *glide = spun(from, [*clockwise, *up, *height, *zoom], now);
            return json!({ "ok": true, "gliding": glide.is_some() });
        }
        _ => {}
    }
    let from = match path {
        MAP3D_PAN | MAP3D_ZOOM | MAP3D_LOOK if held.framed => Orbit {
            range: shown_range().unwrap_or(held.range),
            ..*held
        },
        _ => *held,
    };
    match moved(from, path, &given, aircraft) {
        Some(orbit) => {
            if orbit != *held {
                CHANGED.store(true, Ordering::SeqCst);
            }
            *held = orbit;
            json!({ "ok": true, "heading": orbit.heading, "tilt": orbit.tilt, "range": orbit.range, "following": orbit.around(aircraft.is_some()).focus.is_none() })
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
        MAP3D_OPEN => {
            "nothing, or the latitude (-90 to 90) and longitude (-180 to 180) the 2D map was centred on"
        }
        MAP3D_LOOK => "the latitude (-90 to 90) and longitude (-180 to 180) of the spot to look at",
        MAP3D_FLING => "the release velocity right and down and the view width, in pixels",
        MAP3D_SPIN => {
            "the release rates: twist in degrees clockwise, drag up in pixels and the view height in pixels, and pinch as the log of the scale, each per second"
        }
        _ => "no arguments",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AIRCRAFT: Aircraft = Aircraft {
        at: (47.397, 8.5455),
        heading: 120.0,
    };

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
        assert!((metres(spot, AIRCRAFT.at) - across / 2.0).abs() < 1.0);
        assert!(
            spot.1 > AIRCRAFT.at.1,
            "looking north, a drag to the left shows what lies east"
        );

        let dragged_down = moved(START, MAP3D_PAN, &[0.0, 100.0, width], Some(AIRCRAFT)).unwrap();
        assert!(
            dragged_down.focus.unwrap().0 > AIRCRAFT.at.0,
            "pulling the ground down brings what lies ahead into view"
        );
    }

    #[test]
    fn a_second_drag_carries_on_from_the_held_spot_not_from_the_aircraft() {
        let once = moved(START, MAP3D_PAN, &[-100.0, 0.0, 1000.0], Some(AIRCRAFT)).unwrap();
        let twice = moved(
            once,
            MAP3D_PAN,
            &[-100.0, 0.0, 1000.0],
            Some(Aircraft {
                at: (0.0, 0.0),
                heading: 0.0,
            }),
        )
        .unwrap();
        assert!(
            (metres(twice.focus.unwrap(), AIRCRAFT.at)
                - 2.0 * metres(once.focus.unwrap(), AIRCRAFT.at))
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
        let steep = Orbit {
            tilt: 35.0,
            ..START
        };
        assert_eq!(
            moved(steep, MAP3D_TILT, &[100.0, 900.0], None)
                .unwrap()
                .tilt,
            25.0
        );
        assert_eq!(
            moved(steep, MAP3D_TILT, &[900.0, 900.0], None)
                .unwrap()
                .tilt,
            TILT_DEG.0
        );
        assert_eq!(
            moved(steep, MAP3D_TILT, &[-900.0, 900.0], None)
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

    #[test]
    fn opening_with_an_aircraft_first_looks_along_its_nose_then_keeps_the_pilots_angle() {
        let opened = moved(START, MAP3D_OPEN, &[], Some(AIRCRAFT)).unwrap();
        assert_eq!(opened.heading, AIRCRAFT.heading);
        let turned_away = Orbit {
            heading: 10.0,
            ..opened
        };
        assert_eq!(
            moved(turned_away, MAP3D_OPEN, &[], Some(AIRCRAFT)),
            Some(turned_away),
            "a view the pilot has turned is not turned back"
        );
    }

    #[test]
    fn opening_with_no_aircraft_looks_at_where_the_2d_map_was_and_keeps_a_spot_already_held() {
        let opened = moved(START, MAP3D_OPEN, &[46.5, 7.9], None).unwrap();
        assert_eq!(opened.focus, Some((46.5, 7.9)));
        assert_eq!(
            moved(opened, MAP3D_OPEN, &[1.0, 2.0], None).unwrap().focus,
            Some((46.5, 7.9))
        );
        assert_eq!(moved(START, MAP3D_OPEN, &[], None), Some(START));
    }

    #[test]
    fn a_spot_the_view_only_opened_on_gives_way_to_the_aircraft_when_it_appears() {
        let opened = moved(START, MAP3D_OPEN, &[46.5, 7.9], None).unwrap();
        assert_eq!(view_of(opened, false)["following"], false);
        assert_eq!(
            view_of(opened, true)["following"],
            true,
            "the aircraft connected after the view opened: show it, no Back to aircraft"
        );
        let dragged = moved(opened, MAP3D_PAN, &[-100.0, 0.0, 1000.0], Some(AIRCRAFT)).unwrap();
        assert!(
            metres(dragged.focus.unwrap(), AIRCRAFT.at) < metres((46.5, 7.9), AIRCRAFT.at),
            "a drag starts from the aircraft, not from the old spot"
        );
        assert_eq!(
            view_of(dragged, true)["following"],
            false,
            "a spot the pilot dragged to stays put"
        );
        assert_eq!(
            moved(opened, MAP3D_OPEN, &[], Some(AIRCRAFT))
                .unwrap()
                .focus,
            None,
            "reopened with an aircraft, the old spot is dropped"
        );
    }

    #[test]
    fn following_frames_the_aircraft_from_four_times_its_height_until_the_pilot_zooms() {
        assert_eq!(START.at_height(Some(60.0)).range, 240.0);
        assert_eq!(
            START.at_height(Some(0.0)).range,
            FOLLOW_RANGE_M.0,
            "on the ground it still stands back far enough to see around"
        );
        assert_eq!(START.at_height(Some(5000.0)).range, FOLLOW_RANGE_M.1);
        assert_eq!(START.at_height(None), START);
        let zoomed_in = moved(START, MAP3D_ZOOM, &[2.0], None).unwrap();
        assert_eq!(
            zoomed_in.at_height(Some(60.0)),
            zoomed_in,
            "a pinch fixes the distance"
        );
        let back = moved(zoomed_in, MAP3D_FOLLOW, &[], None).unwrap();
        assert_eq!(back.at_height(Some(60.0)).range, 240.0);
        let dragged = moved(START, MAP3D_PAN, &[0.0, 10.0, 1000.0], Some(AIRCRAFT)).unwrap();
        assert_eq!(dragged.at_height(Some(60.0)), dragged);
    }

    #[test]
    fn look_here_holds_the_chosen_spot_at_the_distance_on_screen() {
        let looked = moved(START, MAP3D_LOOK, &[46.5, 7.9], Some(AIRCRAFT)).unwrap();
        assert_eq!(looked.focus, Some((46.5, 7.9)));
        assert!(
            !looked.resting && !looked.framed,
            "a spot the pilot chose stays put when the aircraft moves"
        );
        assert_eq!(view_of(looked, true)["following"], false);
        assert_eq!(moved(START, MAP3D_LOOK, &[91.0, 7.9], None), None);
    }

    #[test]
    fn the_map_menu_passes_its_point_as_latitude_and_longitude_fields() {
        assert_eq!(
            numbers(r#"[{"latitude":46.5,"longitude":7.9}]"#),
            vec![46.5, 7.9]
        );
        assert_eq!(numbers("[1, 2, 3]"), vec![1.0, 2.0, 3.0]);
        assert_eq!(numbers(r#"[{"latitude":46.5}]"#), Vec::<f64>::new());
        assert_eq!(numbers(r#"[1, "two"]"#), Vec::<f64>::new());
    }

    #[test]
    fn a_flick_keeps_the_view_moving_the_way_the_finger_went_and_slows_to_a_stop() {
        let pinned = moved(START, MAP3D_PAN, &[0.0, 0.0, 1000.0], Some(AIRCRAFT)).unwrap();
        let spot = pinned.focus.unwrap();
        let at = Instant::now();
        let glide = flung(pinned, [-800.0, 0.0, 1000.0], at).expect("a quick flick glides");
        assert_eq!(
            glided(&glide, at),
            pinned,
            "it starts where the finger let go"
        );
        let travelled = |seconds: f64| {
            metres(
                glided(&glide, at + std::time::Duration::from_secs_f64(seconds))
                    .focus
                    .unwrap(),
                spot,
            )
        };
        assert!(travelled(0.1) > 0.0 && travelled(0.1) < travelled(0.5));
        assert!(travelled(0.5) < travelled(3.0));
        let settled = panned(pinned, spot, [-800.0 * GLIDE_DECAY_S, 0.0, 1000.0]);
        assert!(
            metres(
                glided(&glide, at + std::time::Duration::from_secs(10))
                    .focus
                    .unwrap(),
                settled.focus.unwrap()
            ) < 0.01,
            "it comes to rest one decay time's worth of the release speed away"
        );
        assert!(
            glided(&glide, at + std::time::Duration::from_secs(1))
                .focus
                .unwrap()
                .1
                > spot.1,
            "flicked left while looking north, it carries on east like the drag did"
        );
    }

    #[test]
    fn only_a_quick_flick_of_a_held_spot_glides_and_never_faster_than_the_cap() {
        let at = Instant::now();
        let pinned = moved(START, MAP3D_PAN, &[0.0, 0.0, 1000.0], Some(AIRCRAFT)).unwrap();
        assert!(
            flung(pinned, [30.0, 20.0, 1000.0], at).is_none(),
            "a slow release just stops"
        );
        assert!(
            flung(START, [800.0, 0.0, 1000.0], at).is_none(),
            "following, there is no spot to carry"
        );
        let Motion::Pan { right, down, .. } = flung(pinned, [30_000.0, 40_000.0, 1000.0], at)
            .unwrap()
            .motion
        else {
            panic!("a flick pans")
        };
        assert!((right.hypot(down) - GLIDE_SPEED_PX_S.1).abs() < 1e-6);
        assert!(
            (down / right - 40.0 / 30.0).abs() < 1e-9,
            "the cap keeps the direction"
        );
    }

    #[test]
    fn a_twist_tilt_or_pinch_let_go_quickly_carries_on_and_slows_to_a_stop() {
        let at = Instant::now();
        let after = |glide: &Glide, seconds: f64| {
            glided(glide, at + std::time::Duration::from_secs_f64(seconds))
        };
        let twist = spun(START, [180.0, 0.0, 800.0, 0.0], at).expect("a quick twist spins");
        assert_eq!(
            after(&twist, 0.0),
            START,
            "it starts where the fingers let go"
        );
        let turned_by =
            |seconds: f64| (START.heading - after(&twist, seconds).heading).rem_euclid(360.0);
        assert!(turned_by(0.1) > 0.0 && turned_by(0.1) < turned_by(0.5));
        assert!((turned_by(10.0) - 180.0 * GLIDE_DECAY_S).abs() < 1e-6);
        assert!(
            after(&twist, 10.0).framed,
            "a twist alone keeps framing the aircraft"
        );

        let raised = Orbit {
            tilt: 45.0,
            ..START
        };
        let tilt = spun(raised, [0.0, 800.0, 800.0, 0.0], at).unwrap();
        assert!(after(&tilt, 0.1).tilt < raised.tilt);
        assert!(
            (after(&tilt, 10.0).tilt - tilted(raised, [800.0 * GLIDE_DECAY_S, 800.0]).tilt).abs()
                < 1e-6
        );
        let toward_ground = spun(raised, [0.0, -50_000.0, 800.0, 0.0], at).unwrap();
        assert_eq!(
            after(&toward_ground, 10.0).tilt,
            TILT_DEG.1,
            "it stops at straight down"
        );

        let pinch = spun(START, [0.0, 0.0, 800.0, 2.0], at).unwrap();
        assert!(after(&pinch, 0.1).range < START.range);
        assert!(
            (after(&pinch, 10.0).range - START.range / (2.0 * GLIDE_DECAY_S).exp()).abs() < 1e-6
        );
        assert!(!after(&pinch, 0.1).framed, "a pinch fixes the distance");
    }

    #[test]
    fn only_a_brisk_release_spins_and_never_faster_than_the_cap() {
        let at = Instant::now();
        assert!(
            spun(START, [10.0, 30.0, 800.0, 0.2], at).is_none(),
            "a slow release just stops"
        );
        assert!(spun(START, [180.0, 0.0, 0.0, 0.0], at).is_none());
        assert_eq!(
            spun(START, [100_000.0, 20.0, 800.0, -0.1], at)
                .unwrap()
                .motion,
            Motion::Spin {
                clockwise: GLIDE_TURN_DEG_S.1,
                up: 0.0,
                height: 800.0,
                zoom: 0.0
            },
            "only the brisk part carries on, capped"
        );
        assert_eq!(
            spun(START, [0.0, 0.0, 800.0, -100.0], at).unwrap().motion,
            Motion::Spin {
                clockwise: 0.0,
                up: 0.0,
                height: 800.0,
                zoom: -GLIDE_ZOOM_PER_S.1
            }
        );
    }

    #[test]
    fn heads_hear_about_a_spin_until_it_settles_but_not_about_a_pan() {
        let at = Instant::now();
        let spin = spun(START, [180.0, 0.0, 800.0, 0.0], at).unwrap();
        assert!(spin.reshapes_view(at + std::time::Duration::from_millis(100)));
        assert!(!spin.reshapes_view(at + std::time::Duration::from_secs_f64(GLIDE_SETTLED_S)));
        let pinned = moved(START, MAP3D_PAN, &[0.0, 0.0, 1000.0], Some(AIRCRAFT)).unwrap();
        let pan = flung(pinned, [-800.0, 0.0, 1000.0], at).unwrap();
        assert!(
            !pan.reshapes_view(at),
            "a pan glide moves the picture, not the compass"
        );
    }

    #[test]
    fn north_turns_the_view_to_look_north_and_keeps_everything_else() {
        let turned = Orbit {
            heading: 250.0,
            focus: Some((1.0, 2.0)),
            ..START
        };
        assert_eq!(
            moved(turned, MAP3D_NORTH, &[], None),
            Some(Orbit {
                heading: 0.0,
                ..turned
            })
        );
    }

    #[test]
    fn a_map_centre_off_the_globe_is_refused() {
        assert_eq!(moved(START, MAP3D_OPEN, &[95.0, 7.9], None), None);
        assert_eq!(moved(START, MAP3D_OPEN, &[46.5, 181.0], None), None);
    }

    #[test]
    fn a_command_moves_the_shared_orbit_says_so_and_the_view_reports_it() {
        let answer = run(MAP3D_OPEN, "[46.5, 7.9]", None);
        assert_eq!(answer["ok"], true);
        assert_eq!(orbit().focus, Some((46.5, 7.9)));
        assert!(take_changed(), "a change is announced to the heads");
        assert_eq!(run(MAP3D_OPEN, "[1.0, 2.0]", None)["ok"], true);
        assert!(
            !take_changed(),
            "a command that changes nothing announces nothing"
        );
        assert_eq!(run(MAP3D_ZOOM, "[0]", None)["ok"], false);
        let view = view_of(orbit(), false);
        assert_eq!(
            (view["following"].clone(), view["aircraft"].clone()),
            (json!(false), json!(false))
        );
        assert_eq!(run(MAP3D_FOLLOW, "[]", None)["ok"], true);
        assert_eq!(view_of(orbit(), false)["following"], true);
    }
}
