use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex, PoisonError};

use crate::read::{flag, integer};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.id", "vehicle.armed", "vehicle.coordinate"];

const DISTANCE_TOLERANCE_M: f64 = 2.0;
const AZIMUTH_TOLERANCE_DEG: f64 = 1.5;
pub const TAIL_POINTS: usize = 128;

#[derive(Debug, Default)]
pub struct Track {
    points: Vec<(f64, f64)>,
    last_azimuth: Option<f64>,
    generation: u64,
    armed: bool,
    flight_distance: f64,
}

fn offset(from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    let (north, east, _) = crate::geo::geo_to_ned(to.0, to.1, 0.0, (from.0, from.1, 0.0));
    (north, east)
}

pub fn distance_m(from: (f64, f64), to: (f64, f64)) -> f64 {
    let (north, east) = offset(from, to);
    north.hypot(east)
}

pub fn azimuth_deg(from: (f64, f64), to: (f64, f64)) -> f64 {
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
        self.generation += 1;
    }

    pub fn observe(&mut self, armed: bool, position: Option<(f64, f64)>) {
        let armed_edge = armed && !self.armed;
        self.armed = armed;
        if armed_edge {
            self.restart();
            self.flight_distance = 0.0;
        }
        let Some(position) = position.filter(|_| armed) else { return };
        let Some(&last) = self.points.last() else {
            self.points.push(position);
            return;
        };
        let moved = distance_m(last, position);
        if moved <= DISTANCE_TOLERANCE_M {
            return;
        }
        self.flight_distance += moved;
        let azimuth = azimuth_deg(last, position);
        match self.points.last_mut() {
            Some(end) if self.last_azimuth.is_some_and(|anchor| turn_deg(anchor, azimuth) <= AZIMUTH_TOLERANCE_DEG) => *end = position,
            _ => {
                self.last_azimuth = Some(azimuth);
                self.points.push(position);
            }
        }
    }

    fn mirror(&mut self, armed: bool, listed: Vec<(f64, f64)>) {
        let cleared = !self.points.is_empty() && (listed.len() < self.points.len() || listed.first() != self.points.first());
        if cleared {
            self.generation += 1;
        }
        self.armed = armed;
        self.points = listed;
    }

    pub fn snapshot(&self, vehicle: Option<i64>, tail: bool) -> Value {
        let from = if tail { self.points.len().saturating_sub(TAIL_POINTS) } else { 0 };
        json!({
            "kind": "object",
            "class": "Track",
            "order": crate::view::ORDER,
            "available": vehicle.is_some(),
            "vehicleId": vehicle,
            "recording": self.armed,
            "generation": self.generation,
            "count": self.points.len(),
            "from": from,
            "points": self.points[from..].iter().map(|(latitude, longitude)| json!({ "latitude": latitude, "longitude": longitude })).collect::<Vec<_>>(),
        })
    }
}

static TRACKS: LazyLock<Mutex<BTreeMap<i64, Track>>> = LazyLock::new(|| Mutex::new(BTreeMap::new()));
static QT_TRACK: LazyLock<Mutex<Option<(i64, Track)>>> = LazyLock::new(|| Mutex::new(None));

pub fn observe(vehicle: i64, armed: bool, position: Option<(f64, f64)>) {
    TRACKS.lock().unwrap_or_else(PoisonError::into_inner).entry(vehicle).or_default().observe(armed, position);
}

pub fn flight_distance(vehicle: i64) -> Option<f64> {
    TRACKS.lock().unwrap_or_else(PoisonError::into_inner).get(&vehicle).map(|track| track.flight_distance)
}

pub fn clear(vehicle: i64) {
    if let Some(track) = TRACKS.lock().unwrap_or_else(PoisonError::into_inner).get_mut(&vehicle) {
        track.restart();
    }
}

pub fn forget(vehicle: i64) {
    TRACKS.lock().unwrap_or_else(PoisonError::into_inner).remove(&vehicle);
}

fn qt_points(reply: &str) -> Vec<(f64, f64)> {
    crate::read::ok_result(reply)
        .and_then(|listed| listed.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|point| crate::read::nested_coordinate(&json!({ "coordinate": point })))
        .collect()
}

fn qt_snapshot(backend: &dyn Backend, vehicle: i64, armed: bool, tail: bool) -> Value {
    let listed = qt_points(&backend.invoke("vehicle.trajectoryPoints.list", "[]"));
    let mut held = QT_TRACK.lock().unwrap_or_else(PoisonError::into_inner);
    let generation = held.as_ref().map_or(0, |(_, track)| track.generation);
    let track = match held.take() {
        Some((id, track)) if id == vehicle => track,
        _ => Track { generation: generation + 1, ..Track::default() },
    };
    let (_, track) = held.insert((vehicle, track));
    track.mirror(armed, listed);
    track.snapshot(Some(vehicle), tail)
}

pub fn track_view(backend: &dyn Backend, args: &[String]) -> Value {
    let tail = args.first().is_some_and(|arg| arg == "tail");
    let vehicles = backend.value_fields("vehicles", "activeVehicleAvailable");
    let vehicle = backend.value_fields("vehicle", "id,armed");
    let present = flag(&vehicles, "activeVehicleAvailable") && vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let Some(id) = present.then(|| integer(&vehicle, "id")).flatten() else { return Track::default().snapshot(None, tail) };
    if crate::qthost::present() {
        return qt_snapshot(backend, id, flag(&vehicle, "armed"), tail);
    }
    TRACKS.lock().unwrap_or_else(PoisonError::into_inner).get(&id).map_or_else(|| Track::default().snapshot(Some(id), tail), |track| track.snapshot(Some(id), tail))
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
    fn flight_distance_adds_each_move_past_the_tolerance_and_restarts_on_arm_like_trajectory_points() {
        let start = (47.397, 8.545);
        let mut track = Track::default();
        track.observe(true, Some(start));
        track.observe(true, Some(north_of(start, 1.0)));
        assert_eq!(track.flight_distance, 0.0, "inside the tolerance nothing is flown");
        track.observe(true, Some(north_of(start, 10.0)));
        track.observe(true, Some(north_of(start, 20.0)));
        assert!((track.flight_distance - 20.0).abs() < 0.5, "{}", track.flight_distance);
        track.restart();
        assert!(track.flight_distance > 19.0, "clearing the trail keeps the distance flown");
        track.observe(false, None);
        track.observe(true, Some(start));
        assert_eq!(track.flight_distance, 0.0, "a new arming starts a new flight");
    }

    #[test]
    fn a_mission_transfer_clears_the_trail_like_vehicle_connecting_trajectory_clear() {
        let start = (47.397, 8.545);
        let mut flown = Track::default();
        flown.observe(true, Some(start));
        flown.observe(true, Some(north_of(start, 50.0)));
        TRACKS.lock().unwrap_or_else(PoisonError::into_inner).insert(9_901, flown);
        clear(9_901);
        let cleared = TRACKS.lock().unwrap_or_else(PoisonError::into_inner).remove(&9_901).unwrap();
        assert!(cleared.points.is_empty());
        clear(9_902);
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
        assert_eq!(track.snapshot(Some(1), false)["count"], 0, "a disarmed vehicle lays no track, as QGC starts the trail on arming");
        track.observe(true, Some(start));
        track.observe(true, Some(north_of(start, 10.0)));
        let flight = track.snapshot(Some(1), false);
        assert_eq!((flight["count"].as_u64(), flight["recording"].as_bool()), (Some(2), Some(true)));
        let generation = flight["generation"].as_u64().unwrap();
        track.observe(false, Some(north_of(start, 20.0)));
        assert_eq!(track.snapshot(Some(1), false)["count"], 2, "landing keeps the track that was flown");
        assert_eq!(track.snapshot(Some(1), false)["generation"].as_u64(), Some(generation), "the same flight keeps its generation");
        track.observe(true, Some(start));
        assert_eq!(track.snapshot(Some(1), false)["count"], 1, "arming again is a new flight");
        assert!(track.snapshot(Some(1), false)["generation"].as_u64().unwrap() > generation);
    }

    #[test]
    fn a_long_flight_keeps_every_point_and_the_tail_serves_only_the_end() {
        let start = (47.397, 8.545);
        let mut track = Track::default();
        track.observe(true, Some(start));
        let generation = track.generation;
        (0..6_000).for_each(|i| {
            let along = north_of(start, 10.0 * (i + 1) as f64);
            track.observe(true, Some(if i % 2 == 0 { along } else { east_of(along, 40.0) }));
        });
        assert_eq!(track.points.len(), 6_001, "TrajectoryPoints has no cap, so neither does the trail");
        assert_eq!(track.generation, generation);
        let whole = track.snapshot(Some(1), false);
        assert_eq!((whole["from"].as_u64(), whole["points"].as_array().map(Vec::len)), (Some(0), Some(6_001)));
        let tail = track.snapshot(Some(1), true);
        assert_eq!(tail["count"], 6_001);
        assert_eq!(tail["from"].as_u64(), Some((6_001 - TAIL_POINTS) as u64));
        assert_eq!(tail["points"].as_array().map(Vec::len), Some(TAIL_POINTS), "a head watching the tail is not sent the whole flight on every fix");
        assert_eq!(tail["points"][TAIL_POINTS - 1], whole["points"][6_000]);
    }

    #[test]
    fn a_qt_trail_that_restarts_is_a_new_generation_and_one_that_grows_is_not() {
        let start = (47.397, 8.545);
        let mut track = Track::default();
        track.mirror(true, vec![start]);
        let generation = track.generation;
        track.mirror(true, vec![start, north_of(start, 10.0)]);
        track.mirror(true, vec![start, north_of(start, 20.0)]);
        assert_eq!(track.generation, generation, "a moved end point is the same flight");
        track.mirror(true, vec![]);
        assert!(track.generation > generation, "Qt cleared its trail after a mission transfer");
        let cleared = track.generation;
        track.mirror(true, vec![east_of(start, 30.0)]);
        assert_eq!(track.generation, cleared, "the first point after a clear is not another clear");
        track.mirror(true, vec![north_of(start, 90.0), start, start]);
        assert!(track.generation > cleared, "a trail that starts somewhere else was cleared between reads");
    }

    #[test]
    fn qt_trajectory_points_read_as_plain_positions() {
        let reply = json!({ "ok": true, "result": [
            { "valid": true, "latitude": 47.1, "longitude": 8.1, "altitude": 10.0 },
            null,
            { "valid": true, "latitude": 47.2, "longitude": 8.2, "altitude": 11.0 },
        ] })
        .to_string();
        assert_eq!(qt_points(&reply), vec![(47.1, 8.1), (47.2, 8.2)]);
        assert!(qt_points(&json!({ "ok": false }).to_string()).is_empty());
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
    fn each_vehicle_keeps_its_own_trail_and_distance_however_often_the_view_is_read() {
        let start = (47.4, 8.5);
        observe(71, true, Some(start));
        observe(71, true, Some(north_of(start, 50.0)));
        observe(71, true, Some(east_of(north_of(start, 100.0), 40.0)));
        observe(72, true, Some(east_of(start, 500.0)));
        let other = (0..40).map(|_| track_view(&flying(72, east_of(start, 500.0), true), &[])).last().unwrap();
        assert_eq!((other["vehicleId"].as_i64(), other["count"].as_u64()), (Some(72), Some(1)), "a second vehicle has its own trail");
        observe(71, true, Some(north_of(start, 200.0)));
        assert_eq!(track_view(&flying(71, start, true), &[])["count"], 4, "switching back shows the first vehicle's whole trail, as QGC keeps one per vehicle");
        assert!(flight_distance(71).unwrap() > 200.0, "reading another vehicle's trail never restarts this one's flight");
        let gone = track_view(&Fake { vehicle: json!({ "kind": "null" }) }, &[]);
        assert_eq!((gone["available"].as_bool(), gone["count"].as_u64()), (Some(false), Some(0)));
        forget(71);
        forget(72);
        assert_eq!(flight_distance(71), None, "a removed vehicle takes its trail with it");
        assert_eq!(track_view(&flying(71, start, true), &["tail".into()])["count"], 0);
    }
}
