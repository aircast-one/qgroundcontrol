use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{LazyLock, Mutex, PoisonError};

use crate::read::{flag, integer, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.id", "vehicle.armed", "vehicle.coordinate"];

const DISTANCE_TOLERANCE_M: f64 = 2.0;
const AZIMUTH_TOLERANCE_DEG: f64 = 1.5;
pub const MAX_POINTS: usize = 500;
const MAX_TRACKED_VEHICLES: usize = 8;

#[derive(Debug, Default)]
pub struct Track {
    points: VecDeque<(f64, f64)>,
    last_azimuth: Option<f64>,
    generation: u64,
    dropped: u64,
    armed: bool,
    touched: u64,
}

fn offset(from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    let (north, east, _) = crate::geo::geo_to_ned(to.0, to.1, 0.0, (from.0, from.1, 0.0));
    (north, east)
}

fn distance_m(from: (f64, f64), to: (f64, f64)) -> f64 {
    let (north, east) = offset(from, to);
    north.hypot(east)
}

fn azimuth_deg(from: (f64, f64), to: (f64, f64)) -> f64 {
    let (north, east) = offset(from, to);
    east.atan2(north).to_degrees().rem_euclid(360.0)
}

fn turn_deg(from: f64, to: f64) -> f64 {
    let apart = (to - from).abs().rem_euclid(360.0);
    apart.min(360.0 - apart)
}

impl Track {
    fn restart(&mut self) {
        self.points.clear();
        self.last_azimuth = None;
        self.dropped = 0;
        self.generation += 1;
    }

    fn append(&mut self, position: (f64, f64)) {
        if self.points.len() == MAX_POINTS {
            self.points.pop_front();
            self.dropped += 1;
        }
        self.points.push_back(position);
    }

    pub fn observe(&mut self, armed: bool, position: Option<(f64, f64)>) {
        let armed_edge = armed && !self.armed;
        self.armed = armed;
        if armed_edge {
            self.restart();
        }
        let Some(position) = position.filter(|_| armed) else { return };
        let Some(&last) = self.points.back() else {
            self.append(position);
            return;
        };
        if distance_m(last, position) <= DISTANCE_TOLERANCE_M {
            return;
        }
        let azimuth = azimuth_deg(last, position);
        if self.last_azimuth.is_some_and(|anchor| turn_deg(anchor, azimuth) <= AZIMUTH_TOLERANCE_DEG) {
            let end = self.points.len() - 1;
            self.points[end] = position;
            return;
        }
        self.last_azimuth = Some(azimuth);
        self.append(position);
    }

    pub fn snapshot(&self, vehicle: Option<i64>) -> Value {
        json!({
            "kind": "object",
            "class": "Track",
            "order": crate::view::ORDER,
            "available": vehicle.is_some(),
            "vehicleId": vehicle,
            "recording": self.armed,
            "generation": self.generation,
            "dropped": self.dropped,
            "count": self.points.len(),
            "points": self.points.iter().map(|(latitude, longitude)| json!({ "latitude": latitude, "longitude": longitude })).collect::<Vec<_>>(),
        })
    }
}

static TRACKS: LazyLock<Mutex<BTreeMap<i64, Track>>> = LazyLock::new(|| Mutex::new(BTreeMap::new()));

fn coordinate(vehicle: &Value) -> Option<(f64, f64)> {
    let coordinate = vehicle.get("coordinate")?;
    if !flag(coordinate, "valid") {
        return None;
    }
    let number = |key: &str| coordinate.get(key).and_then(Value::as_f64).filter(|v| v.is_finite());
    number("latitude").zip(number("longitude")).filter(|(lat, lon)| *lat != 0.0 || *lon != 0.0)
}

pub fn track_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicles = object(&backend.get_fields("vehicles", "activeVehicleAvailable"));
    let vehicle = object(&backend.get_fields("vehicle", "id,armed,coordinate"));
    let present = flag(&vehicles, "activeVehicleAvailable") && vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let id = present.then(|| integer(&vehicle, "id")).flatten();
    let mut tracks = TRACKS.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(id) = id else { return Track::default().snapshot(None) };
    let clock = tracks.values().map(|t| t.touched).max().unwrap_or(0) + 1;
    let track = tracks.entry(id).or_default();
    track.touched = clock;
    track.observe(flag(&vehicle, "armed"), coordinate(&vehicle));
    let snapshot = track.snapshot(Some(id));
    let stale: Vec<i64> = tracks.iter().filter(|(_, t)| clock.saturating_sub(t.touched) >= MAX_TRACKED_VEHICLES as u64).map(|(id, _)| *id).collect();
    stale.iter().for_each(|id| {
        tracks.remove(id);
    });
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;

    fn north_of(origin: (f64, f64), metres: f64) -> (f64, f64) {
        let (lat, lon, _) = crate::geo::ned_to_geo(metres, 0.0, 0.0, (origin.0, origin.1, 0.0));
        (lat, lon)
    }

    fn east_of(origin: (f64, f64), metres: f64) -> (f64, f64) {
        let (lat, lon, _) = crate::geo::ned_to_geo(0.0, metres, 0.0, (origin.0, origin.1, 0.0));
        (lat, lon)
    }

    fn circle(centre: (f64, f64), radius: f64, steps: usize) -> Vec<(f64, f64)> {
        (0..steps)
            .map(|i| {
                let angle = std::f64::consts::TAU * i as f64 / steps as f64;
                let (lat, lon, _) = crate::geo::ned_to_geo(radius * angle.cos(), radius * angle.sin(), 0.0, (centre.0, centre.1, 0.0));
                (lat, lon)
            })
            .collect()
    }

    #[test]
    fn a_straight_leg_stays_one_point_and_a_turn_adds_one() {
        let start = (47.397, 8.545);
        let mut track = Track::default();
        track.observe(true, Some(start));
        assert_eq!(track.points.len(), 1);
        track.observe(true, Some(north_of(start, 1.0)));
        assert_eq!(track.points.len(), 1, "a metre of drift is inside the tolerance and is not a point");
        track.observe(true, Some(north_of(start, 10.0)));
        assert_eq!(track.points.len(), 2);
        let twenty = north_of(start, 20.0);
        track.observe(true, Some(twenty));
        assert_eq!(track.points.len(), 2, "flying on down the same leg moves the end rather than appending, as the Qt trail does");
        assert_eq!(track.points[1], twenty);
        track.observe(true, Some(east_of(twenty, 30.0)));
        assert_eq!(track.points.len(), 3, "a turn is a new point");
    }

    #[test]
    fn a_slow_turn_draws_an_arc_rather_than_a_chord() {
        let centre = (47.397, 8.545);
        let mut track = Track::default();
        circle(centre, 100.0, 300).iter().for_each(|point| track.observe(true, Some(*point)));
        assert!(track.points.len() > 100, "the heading is anchored at the last appended point, as TrajectoryPoints anchors it, so a constant turn keeps appending; got {}", track.points.len());
    }

    #[test]
    fn flying_due_north_does_not_spell_every_sample_as_a_turn() {
        assert_eq!(turn_deg(359.5, 0.5), 1.0, "a heading either side of north is a one degree turn, not a 359 degree one");
        assert_eq!(turn_deg(0.5, 359.5), 1.0);
        assert_eq!(turn_deg(10.0, 200.0), 170.0);
        let start = (47.397, 8.545);
        let mut track = Track::default();
        track.observe(true, Some(start));
        (1..40).for_each(|i| {
            let wobble = if i % 2 == 0 { 0.05 } else { -0.05 };
            track.observe(true, Some(north_of(east_of(start, wobble), 10.0 * i as f64)));
        });
        assert!(track.points.len() < 6, "a due north leg that wobbles either side of the meridian is one segment, not one point per sample; got {}", track.points.len());
    }

    #[test]
    fn a_track_belongs_to_one_flight() {
        let start = (47.397, 8.545);
        let mut track = Track::default();
        track.observe(false, Some(start));
        assert_eq!(track.snapshot(Some(1))["count"], 0, "a disarmed vehicle lays no track, as QGC starts the trail on arming");
        track.observe(true, Some(start));
        track.observe(true, Some(north_of(start, 10.0)));
        let flight = track.snapshot(Some(1));
        assert_eq!((flight["count"].as_u64(), flight["recording"].as_bool()), (Some(2), Some(true)));
        let generation = flight["generation"].as_u64().unwrap();
        track.observe(false, Some(north_of(start, 20.0)));
        assert_eq!(track.snapshot(Some(1))["count"], 2, "landing keeps the track that was flown");
        assert_eq!(track.snapshot(Some(1))["generation"].as_u64(), Some(generation), "the same flight keeps its generation");
        track.observe(true, Some(start));
        assert_eq!(track.snapshot(Some(1))["count"], 1, "arming again is a new flight");
        assert!(track.snapshot(Some(1))["generation"].as_u64().unwrap() > generation);
    }

    #[test]
    fn a_long_flight_drops_its_oldest_points_rather_than_its_whole_trail() {
        let start = (47.397, 8.545);
        let mut track = Track::default();
        track.observe(true, Some(start));
        let generation = track.generation;
        (0..MAX_POINTS + 40).for_each(|i| {
            let along = north_of(start, 10.0 * (i + 1) as f64);
            track.observe(true, Some(if i % 2 == 0 { along } else { east_of(along, 40.0) }));
        });
        assert_eq!(track.points.len(), MAX_POINTS, "the trail is capped, unlike TrajectoryPoints which has no cap at all");
        assert!(track.dropped > 0, "a head is told how many points fell off the front");
        assert_eq!(track.generation, generation, "reaching the cap is not a new flight, so a head keeps drawing rather than starting over");
    }

    struct Fake {
        vehicle: Value,
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            self.get_fields(path, "")
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": self.vehicle.get("kind") == Some(&json!("object")) }).to_string(),
                "vehicle" => self.vehicle.to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _path: &str, _value: &str) -> String { String::new() }
        fn invoke(&self, _path: &str, _args: &str) -> String { String::new() }
        fn watch(&self, _paths: &[String]) {}
    }

    fn flying(id: i64, at: (f64, f64), valid: bool) -> Fake {
        Fake { vehicle: json!({ "kind": "object", "id": id, "armed": true, "coordinate": { "valid": valid, "latitude": at.0, "longitude": at.1 } }) }
    }

    #[test]
    fn each_vehicle_keeps_its_own_trail_across_a_switch() {
        let start = (47.4, 8.5);
        let leg = |i: f64| {
            let along = north_of(start, 50.0 * i);
            track_view(&flying(71, if i as i64 % 2 == 0 { along } else { east_of(along, 40.0) }, true), &[])
        };
        leg(0.0);
        leg(1.0);
        assert_eq!(leg(2.0)["count"], 3);
        let other = track_view(&flying(72, east_of(start, 500.0), true), &[]);
        assert_eq!((other["vehicleId"].as_i64(), other["count"].as_u64()), (Some(72), Some(1)), "a second vehicle starts its own trail");
        assert_eq!(leg(3.0)["count"], 4, "switching back shows the first vehicle's trail, as QGC keeps one per vehicle");
        let gone = track_view(&Fake { vehicle: json!({ "kind": "null" }) }, &[]);
        assert_eq!((gone["available"].as_bool(), gone["count"].as_u64()), (Some(false), Some(0)));
    }

    #[test]
    fn a_position_the_vehicle_calls_invalid_is_not_a_point() {
        let start = (47.5, 8.6);
        assert_eq!(track_view(&flying(81, start, false), &[])["count"], 0, "an invalid coordinate is not a place the vehicle has been");
        assert_eq!(track_view(&flying(81, (0.0, 0.0), true), &[])["count"], 0, "null island is what an autopilot reports before it has a fix");
        assert_eq!(track_view(&flying(81, start, true), &[])["count"], 1);
    }
}
