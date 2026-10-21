use std::collections::VecDeque;
use std::sync::{Arc, PoisonError};

use mavlink::MavHeader;
use mavlink::dialects::ardupilotmega::*;
use serde_json::{Value, json};

use crate::autodrop::{CAR_LENGTH_M, CAR_WIDTH_M, TURN_WINDOW_MS, TURNED_DEG};
use crate::dropsim::Point;
use crate::geo::geo_to_ned;
use crate::hub::{Hub, Origin};
use crate::mockcamera::StreamKind;
use crate::mocklink::{Options, Out, Sim, Vehicle};

const COPTER: u8 = 128;
const CAR: u8 = 129;
const LINK: u32 = 7;
const STEP_MS: u64 = 10;
const SIM_TICK_MS: u64 = 100;
const HUB_TICK_PHASE_MS: u64 = 50;
const CAR_HEIGHT_M: f64 = 1.45;
const ALTITUDE_M: f32 = 40.0;
const CONNECT_LIMIT_MS: u64 = 60_000;
const CHASE_LIMIT_MS: u64 = 60_000;
const CAR_SETTLES_MS: u64 = 3_000;

#[derive(Debug, Clone, Copy)]
pub struct Detector {
    pub latency_ms: u64,
    pub noise: f64,
    pub dropout: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Outcome {
    pub dropped: bool,
    pub hit: bool,
    pub miss_m: f64,
    pub along_m: f64,
    pub across_m: f64,
    pub predicted_m: f64,
    pub turned: bool,
    pub track_err_m: f64,
    pub speed_err_mps: f64,
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1_u64 << 53) as f64
    }

    fn gauss(&mut self) -> f64 {
        let (u, v) = (self.next().max(1e-12), self.next());
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }
}

fn options(vehicle: Vehicle, gear: bool) -> Options {
    Options {
        vehicle,
        send_status_text: false,
        enable_camera: gear,
        enable_gimbal: gear,
        enable_proximity: false,
        apm_start_fresh_params: false,
        increment_vehicle_id: false,
        video: StreamKind::None,
    }
}

fn gcs() -> MavHeader {
    MavHeader {
        system_id: crate::mavout::gcs_system(),
        component_id: 190,
        sequence: 0,
    }
}

fn command(sim: &mut Sim, command: MavCmd, params: [f32; 7]) -> Vec<Out> {
    let [param1, param2, param3, param4, param5, param6, param7] = params;
    sim.receive(
        gcs(),
        &MavMessage::COMMAND_LONG(COMMAND_LONG_DATA {
            param1,
            param2,
            param3,
            param4,
            param5,
            param6,
            param7,
            command,
            target_system: COPTER,
            target_component: 1,
            confirmation: 0,
        }),
    )
}

fn origin() -> Origin {
    Origin {
        link: LINK,
        replay: false,
        v2: true,
    }
}

fn decode(bytes: &[u8]) -> Option<MavMessage> {
    mavlink::read_versioned_msg::<MavMessage, _>(
        &mut mavlink::peek_reader::PeekReader::new(bytes),
        mavlink::ReadVersion::Any,
    )
    .ok()
    .map(|(_, message)| message)
}

struct World {
    hub: Hub,
    copter: Sim,
    rover: Sim,
    car: VecDeque<(u64, (f64, f64), f64)>,
    sequence: u8,
}

impl World {
    fn to_hub(&mut self, outs: Vec<Out>, now_ms: u64) {
        let replies: Vec<Out> = outs
            .into_iter()
            .flat_map(|(component, message)| {
                self.sequence = self.sequence.wrapping_add(1);
                let header = MavHeader {
                    system_id: COPTER,
                    component_id: component,
                    sequence: self.sequence,
                };
                let sent = self
                    .hub
                    .on_frame(origin(), &header, &message, now_ms * 1000, now_ms);
                self.to_copter(sent)
            })
            .collect();
        if !replies.is_empty() {
            self.to_hub(replies, now_ms);
        }
    }

    fn to_copter(&mut self, sent: Vec<(u32, Vec<u8>)>) -> Vec<Out> {
        sent.iter()
            .filter_map(|(_, bytes)| decode(bytes))
            .flat_map(|message| self.copter.receive(gcs(), &message))
            .collect()
    }

    fn vehicle(&self) -> Option<&crate::hub::Vehicle> {
        self.hub.vehicle(COPTER)
    }

    fn car_at(&self, at_ms: u64) -> ((f64, f64), f64) {
        let after = self.car.iter().position(|(t, _, _)| *t >= at_ms);
        match after {
            Some(0) | None => {
                let (_, at, heading) = self.car.back().copied().expect("the car is driving");
                (at, heading)
            }
            Some(index) => {
                let (t0, a, h0) = self.car[index - 1];
                let (t1, b, h1) = self.car[index];
                let share = (at_ms - t0) as f64 / (t1 - t0).max(1) as f64;
                let turn = ((h1 - h0 + 540.0) % 360.0) - 180.0;
                (
                    (a.0 + (b.0 - a.0) * share, a.1 + (b.1 - a.1) * share),
                    h0 + turn * share,
                )
            }
        }
    }
}

fn half_tan(degrees: f64) -> f64 {
    (degrees / 2.0).to_radians().tan()
}

fn seen_box(
    lens: crate::gimbalgeo::Lens,
    drone: (f64, f64),
    altitude: f64,
    aim: (f64, f64),
    car: ((f64, f64), f64),
) -> Option<(f64, f64, f64, f64)> {
    let ((lat, lon), heading) = car;
    let (north, east, _) = geo_to_ned(lat, lon, 0.0, (drone.0, drone.1, 0.0));
    let (sin, cos) = heading.to_radians().sin_cos();
    let camera = crate::videoar::Camera {
        heading: aim.0,
        pitch: aim.1,
        roll: 0.0,
        fov: lens.horizontal_deg,
    };
    let corners: Option<Vec<(f64, f64)>> = [-1.0, 1.0]
        .iter()
        .flat_map(|&along| [-1.0, 1.0].map(|across| (along, across)))
        .flat_map(|(along, across)| [0.0, CAR_HEIGHT_M].map(|up| (along, across, up)))
        .map(|(along, across, up)| {
            let (forward, side) = (along * CAR_LENGTH_M / 2.0, across * CAR_WIDTH_M / 2.0);
            let n = north + forward * cos - side * sin;
            let e = east + forward * sin + side * cos;
            let (x, y, ahead) = crate::videoar::in_camera(camera, (e, n, up - altitude));
            (ahead > 0.5).then(|| {
                (
                    0.5 + x / ahead * 0.5 / half_tan(lens.horizontal_deg),
                    0.5 - y / ahead * 0.5 / half_tan(lens.vertical_deg),
                )
            })
        })
        .collect();
    let corners = corners?;
    let fold = |pick: fn(&(f64, f64)) -> f64, f: fn(f64, f64) -> f64, start: f64| {
        corners.iter().map(pick).fold(start, f)
    };
    let (left, right) = (
        fold(|c| c.0, f64::min, f64::MAX),
        fold(|c| c.0, f64::max, f64::MIN),
    );
    let (top, bottom) = (
        fold(|c| c.1, f64::min, f64::MAX),
        fold(|c| c.1, f64::max, f64::MIN),
    );
    let inside = right > 0.0 && left < 1.0 && bottom > 0.0 && top < 1.0;
    inside.then(|| {
        let (l, t) = (left.max(0.0), top.max(0.0));
        (l, t, right.min(1.0) - l, bottom.min(1.0) - t)
    })
}

fn heading_change(world: &World, from_ms: u64, to_ms: u64) -> f64 {
    let headings: Vec<f64> = world
        .car
        .iter()
        .filter(|(t, _, _)| *t >= from_ms && *t <= to_ms)
        .map(|(_, _, h)| *h)
        .collect();
    headings
        .first()
        .map(|&first| {
            headings
                .iter()
                .map(|h| (((h - first + 540.0) % 360.0) - 180.0).abs())
                .fold(0.0, f64::max)
        })
        .unwrap_or(0.0)
}

pub fn run(
    network: Arc<crate::roads::Network>,
    seed: u64,
    detector: Detector,
    on_roads: bool,
) -> Outcome {
    crate::roads::provide(on_roads.then(|| network.clone()));
    crate::gimbalgeo::forget();
    *crate::detections::lock() = Default::default();
    crate::detections::lock().retarget(crate::detections::parse("rtsp://bench/cam0", None));
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut world = World {
        hub: Hub::default(),
        copter: Sim::new(options(Vehicle::Copter, true), COPTER, None),
        rover: Sim::new(options(Vehicle::Rover, false), CAR, None),
        car: VecDeque::new(),
        sequence: 0,
    };
    let bearing = rng.next() * 360.0;
    let distance = 60.0 + rng.next() * 60.0;
    let mut driving: Option<u64> = None;

    let setup: Vec<Out> = [
        command(
            &mut world.copter,
            MavCmd::MAV_CMD_DO_SET_MODE,
            [1.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
        command(
            &mut world.copter,
            MavCmd::MAV_CMD_COMPONENT_ARM_DISARM,
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
        command(
            &mut world.copter,
            MavCmd::MAV_CMD_NAV_TAKEOFF,
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, ALTITUDE_M],
        ),
    ]
    .concat();
    world.to_hub(setup, 0);

    let lens = crate::gimbalgeo::lens();
    let mut pending: VecDeque<(u64, u64, (f64, f64, f64, f64))> = VecDeque::new();
    let mut triggered: Option<u64> = None;
    let mut released: Option<(u64, crate::payloadfall::Release, f64)> = None;
    let mut estimate: Option<crate::dropsim::Target> = None;
    let mut now = 0_u64;
    let limit = |driving: Option<u64>, triggered: Option<u64>| match (driving, triggered) {
        (_, Some(at)) => at + CHASE_LIMIT_MS,
        (Some(at), None) => at + CAR_SETTLES_MS * 4,
        (None, None) => CONNECT_LIMIT_MS,
    };
    while now < limit(driving, triggered) {
        now += STEP_MS;
        if driving.is_none() && now % SIM_TICK_MS == 0 {
            let ((lat, lon), alt, _, _) = world.copter.truth();
            let ready = alt >= f64::from(ALTITUDE_M) - 1.0
                && world
                    .vehicle()
                    .and_then(|v| v.picture_to_ground_at((0.5, 0.5), now))
                    .is_some();
            if ready {
                let start =
                    crate::surveygrid::at_distance_and_azimuth((lat, lon), distance, bearing);
                world.rover.drive_roads_from(network.clone(), start, seed);
                driving = Some(now);
            }
        }
        if now % SIM_TICK_MS == 0 {
            world.rover.tick(now);
            let ((lat, lon), _, heading, _) = world.rover.truth();
            world.car.push_back((now, (lat, lon), heading));
            let outs = world.copter.tick(now);
            let ((dlat, dlon), alt, _, _) = world.copter.truth();
            let aim = world.copter.camera_aim();
            if triggered.is_none() && alt >= f64::from(ALTITUDE_M) - 1.0 {
                let (aim_heading, aim_pitch) =
                    crate::synthview::aim_at((dlat, dlon, alt), (lat, lon, 0.0));
                world.copter.point_camera(aim_pitch, aim_heading);
            }
            let seen =
                aim.and_then(|aim| seen_box(lens, (dlat, dlon), alt, aim, ((lat, lon), heading)));
            if let Some(found) = seen.filter(|_| rng.next() >= detector.dropout) {
                let jitter = |rng: &mut Rng| rng.gauss() * detector.noise;
                let (x, y, w, h) = found;
                let noisy = (x + jitter(&mut rng), y + jitter(&mut rng), w, h);
                pending.push_back((now + detector.latency_ms, now, noisy));
            }
            world.to_hub(outs, now);
        }
        while pending.front().is_some_and(|(due, _, _)| *due <= now) {
            let (due, captured, (x, y, w, h)) = pending.pop_front().expect("checked");
            let payload = json!({
                "ageMs": due - captured,
                "boxes": [{"x": x, "y": y, "w": w, "h": h, "label": "car", "conf": 0.9, "target": true}],
            });
            let frame = crate::detections::frame_from(&payload, now, 0);
            if let Some(vehicle) = world.vehicle() {
                let grounded = crate::detections::grounded(frame, |centre| {
                    vehicle.object_at(centre, captured)
                });
                crate::detections::lock().receive(1, grounded);
            }
        }
        loop {
            let queued = world.copter.pump();
            if queued.is_empty() {
                break;
            }
            world.to_hub(queued, now);
        }
        if now % SIM_TICK_MS == HUB_TICK_PHASE_MS
            || world
                .hub
                .next_autodrop_due_ms()
                .is_some_and(|due| now >= due)
        {
            let sent = world.hub.tick(now);
            let replies = world.to_copter(sent);
            world.to_hub(replies, now);
        }
        if triggered.is_none()
            && driving.is_some_and(|at| now >= at + CAR_SETTLES_MS)
            && now % 100 == 0
        {
            let aim = world.vehicle().and_then(|vehicle| {
                vehicle.release_now().and_then(|release| {
                    let target = vehicle.target_for_autodrop(
                        crate::autodrop::TargetSource::Tracked,
                        release.position,
                        release.altitude_agl_m,
                        now,
                    )?;
                    crate::autodrop::aim_point(
                        release.position,
                        target,
                        crate::autodrop::roads_under(target).as_deref(),
                    )
                })
            });
            if let Some(aim) = aim {
                triggered = Some(now);
                let sent = world
                    .hub
                    .guided(
                        Some(COPTER),
                        &json!({"action": "goto", "latitude": aim.lat, "longitude": aim.lon, "autodrop": true, "targetSource": ["tracked"], "vehicle": COPTER}),
                        now,
                    )
                    .unwrap_or_default();
                let replies = world.to_copter(sent);
                world.to_hub(replies, now);
            }
        }
        if released.is_none() {
            estimate = world.vehicle().and_then(|vehicle| {
                let release = vehicle.release_now()?;
                vehicle.target_for_autodrop(
                    crate::autodrop::TargetSource::Tracked,
                    release.position,
                    release.altitude_agl_m,
                    now,
                )
            });
            released = world
                .vehicle()
                .and_then(|vehicle| vehicle.releases.last().copied())
                .map(|release| {
                    let predicted = match world
                        .vehicle()
                        .and_then(|vehicle| vehicle.autodrop.as_ref())
                    {
                        Some(crate::autodrop::State::Fired {
                            predicted_miss_m, ..
                        }) => *predicted_miss_m,
                        _ => f64::NAN,
                    };
                    (now, release, predicted)
                });
        }
        if let Some((at, release, _)) = released
            && now >= at + (release.falls_for_s() * 1000.0) as u64 + SIM_TICK_MS * 2
        {
            break;
        }
        if matches!(
            world
                .vehicle()
                .and_then(|vehicle| vehicle.autodrop.as_ref()),
            Some(crate::autodrop::State::GaveUp { .. })
        ) {
            break;
        }
    }
    released.map_or(
        Outcome {
            dropped: false,
            hit: false,
            miss_m: f64::NAN,
            along_m: f64::NAN,
            across_m: f64::NAN,
            predicted_m: f64::NAN,
            turned: false,
            track_err_m: f64::NAN,
            speed_err_mps: f64::NAN,
        },
        |(at, release, predicted)| {
            let falls_ms = (release.falls_for_s() * 1000.0) as u64;
            let landed = release.at(release.at_ms + falls_ms as i64 + 1);
            let impact = Point {
                lat: landed["latitude"].as_f64().unwrap_or(f64::NAN),
                lon: landed["longitude"].as_f64().unwrap_or(f64::NAN),
            };
            let land_ms = at + falls_ms;
            let ((clat, clon), heading) = world.car_at(land_ms);
            let (north, east, _) = geo_to_ned(impact.lat, impact.lon, 0.0, (clat, clon, 0.0));
            let (sin, cos) = heading.to_radians().sin_cos();
            let along = north * cos + east * sin;
            let across = -north * sin + east * cos;
            let ((rlat, rlon), _) = world.car_at(at);
            let true_speed = world
                .car
                .iter()
                .find(|(t, _, _)| *t >= at)
                .zip(world.car.iter().find(|(t, _, _)| *t >= at + 1000))
                .map(|((_, a, _), (_, b, _))| geo_to_ned(b.0, b.1, 0.0, (a.0, a.1, 0.0)))
                .map_or((0.0, 0.0), |(n, e, _)| (n, e));
            let (track_err, speed_err) = estimate.map_or((f64::NAN, f64::NAN), |t| {
                let (tn, te, _) =
                    geo_to_ned(t.position.lat, t.position.lon, 0.0, (rlat, rlon, 0.0));
                (
                    tn.hypot(te),
                    (t.velocity.north_mps - true_speed.0).hypot(t.velocity.east_mps - true_speed.1),
                )
            });
            Outcome {
                dropped: true,
                hit: along.abs() <= CAR_LENGTH_M / 2.0 && across.abs() <= CAR_WIDTH_M / 2.0,
                miss_m: north.hypot(east),
                along_m: along,
                across_m: across,
                predicted_m: predicted,
                track_err_m: track_err,
                speed_err_mps: speed_err,
                turned: heading_change(&world, at.saturating_sub(TURN_WINDOW_MS), land_ms)
                    > TURNED_DEG,
            }
        },
    )
}

pub fn network() -> Arc<crate::roads::Network> {
    let raw = std::fs::read(format!(
        "{}/../test/Bridge/fixtures/openfreemap-14-8580-5736-roads-around-mock-home.pbf",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("the recorded roads around the mock home");
    Arc::new(crate::roads::network(&[(8580, 5736, raw)]))
}

pub fn summary(outcomes: &[Outcome]) -> Value {
    let rate = |set: Vec<&Outcome>| {
        let dropped: Vec<&&Outcome> = set.iter().filter(|o| o.dropped).collect();
        let hits = dropped.iter().filter(|o| o.hit).count();
        let mut misses: Vec<f64> = dropped.iter().map(|o| o.miss_m).collect();
        misses.sort_by(f64::total_cmp);
        let at = |q: f64| {
            misses
                .get(((misses.len() as f64 - 1.0) * q).round() as usize)
                .copied()
        };
        json!({
            "runs": set.len(),
            "dropped": dropped.len(),
            "hits": hits,
            "hitRate": if dropped.is_empty() { 0.0 } else { hits as f64 / dropped.len() as f64 },
            "missMedian": at(0.5),
            "miss90": at(0.9),
            "alongMean": dropped.iter().map(|o| o.along_m).sum::<f64>() / dropped.len().max(1) as f64,
            "acrossMean": dropped.iter().map(|o| o.across_m).sum::<f64>() / dropped.len().max(1) as f64,
            "trackErrMean": dropped.iter().map(|o| o.track_err_m).sum::<f64>() / dropped.len().max(1) as f64,
            "speedErrMean": dropped.iter().map(|o| o.speed_err_mps).sum::<f64>() / dropped.len().max(1) as f64,
            "predictedMean": dropped.iter().map(|o| o.predicted_m).sum::<f64>() / dropped.len().max(1) as f64,
        })
    };
    json!({
        "all": rate(outcomes.iter().collect()),
        "straight": rate(outcomes.iter().filter(|o| !o.turned).collect()),
        "turning": rate(outcomes.iter().filter(|o| o.turned).collect()),
    })
}

type Exclusive = (
    std::sync::MutexGuard<'static, ()>,
    std::sync::MutexGuard<'static, ()>,
    std::sync::MutexGuard<'static, ()>,
);

fn exclusive() -> Exclusive {
    (
        crate::gimbalgeo::TEST_FIXES
            .lock()
            .unwrap_or_else(PoisonError::into_inner),
        crate::detections::TEST_FEED
            .lock()
            .unwrap_or_else(PoisonError::into_inner),
        crate::gpstarget::TEST_FIXES
            .lock()
            .unwrap_or_else(PoisonError::into_inner),
    )
}

fn alone(
    network: Arc<crate::roads::Network>,
    seed: u64,
    detector: Detector,
    on_roads: bool,
) -> Outcome {
    let outcome = run(network, seed, detector, on_roads);
    crate::gimbalgeo::forget();
    *crate::detections::lock() = Default::default();
    crate::gpstarget::refresh(std::collections::BTreeMap::new());
    crate::roads::provide(None);
    outcome
}

pub fn bench(runs: u64, detector: Detector, on_roads: bool) -> Value {
    let _exclusive = exclusive();
    let network = network();
    let outcomes: Vec<Outcome> = (1..=runs)
        .map(|seed| alone(network.clone(), seed, detector, on_roads))
        .collect();
    summary(&outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DETECTOR: Detector = Detector {
        latency_ms: 300,
        noise: 0.004,
        dropout: 0.1,
    };

    #[test]
    #[ignore = "the full bench takes minutes; run with --release -- --ignored --nocapture"]
    fn intercept_bench() {
        let env = |name: &str| std::env::var(name).ok().and_then(|v| v.parse::<f64>().ok());
        let runs = env("BENCH_RUNS").map_or(200, |n| n as u64);
        let detector = Detector {
            latency_ms: env("BENCH_LATENCY").map_or(DETECTOR.latency_ms, |v| v as u64),
            noise: env("BENCH_NOISE").unwrap_or(DETECTOR.noise),
            dropout: env("BENCH_DROPOUT").unwrap_or(DETECTOR.dropout),
        };
        let on_roads = env("BENCH_ROADS").is_none_or(|v| v != 0.0);
        println!(
            "{}",
            serde_json::to_string_pretty(&bench(runs, detector, on_roads)).unwrap()
        );
    }

    #[test]
    fn one_bench_run_drops_on_a_road_car() {
        let _exclusive = exclusive();
        let outcome = alone(network(), 3, DETECTOR, true);
        assert!(outcome.dropped && outcome.hit, "{outcome:?}");
    }
}
