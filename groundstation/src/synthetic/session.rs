use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use web_time::Instant;

use url::Url;

use super::fetch::{Fetcher, Pool};
use super::httpcache::Answer;
use glam::{DQuat, DVec3, EulerRot};

use super::geo::{Camera, Pose};
use super::ground;
use super::mesh::Mesh;
use super::open::{Built, OpenScene, Want};
use super::overlay::{self, Bottle, Overlays};
use super::predict::{self, Gain, Lag, Predicted, Step};
use super::tileset::{self, GOOGLE_ROOT, ION_GOOGLE_ASSET, Parsed, Request, Selection, Tileset};
use crate::virtualjoystick::StickSample;

const WORKERS: usize = 8;
const MAX_IN_FLIGHT: usize = 24;
const PROBE_SLOTS: usize = 4;
const CLEARANCE_M: f64 = 2.0;
const FOLLOW_AT_60_HZ: f64 = 0.25;
const LONGEST_STEP: Duration = Duration::from_millis(100);
const PROBE_EVERY: Duration = Duration::from_millis(500);
const RESELECT_EVERY: Duration = Duration::from_secs(1);
const PROBE_HEIGHT_M: f64 = 9_000.0;
const PROBE_FOV: f64 = 1.0;
const PROBE_PX: u32 = 256;
const UNKNOWN_GROUND_M: f64 = 1_000.0;
const SETTLED_DEGREES: f64 = 5e-7;
const SETTLED_METRES: f64 = 0.05;
const SETTLED_ANGLE: f64 = 0.05;
const TURN_EVERY_GUESS: Duration = Duration::from_millis(90);
const TURN_SHORTEST: Duration = Duration::from_millis(30);
const TURN_LONGEST: Duration = Duration::from_millis(300);
const TURN_STRETCH: f64 = 1.3;
const DEG_S_PER_DETAIL_STEP: f64 = 15.0;
const COARSEST_WHILE_TURNING: f64 = 3.0;
const COARSER_UNTIL_HANDOVER: f64 = 4.0;
const SPIN_SMOOTHING: f64 = 0.3;
const GOOGLE_READY_COARSEST: f64 = 1_000.0;
const STAND_IN_WORKERS: usize = 3;
const STAND_IN_IN_FLIGHT: usize = 6;
const TERRAIN_UNDER_EVERY: Duration = Duration::from_millis(500);
const TERRAIN_UNDER_COARSER: f64 = 4.0;
const LONGEST_GUESS: Duration = Duration::from_secs(1);
const LONGEST_TURN_GUESS: Duration = Duration::from_millis(600);
const DISPLAY_LEAD: Duration = Duration::from_millis(25);
const STICKS_LIVE: Duration = Duration::from_millis(300);
const SURROUND_EVERY: Duration = Duration::from_millis(500);
const SURROUND_COARSER: f64 = 8.0;
const SURROUND_SLOTS: usize = 4;
const SURROUND_TURNS: [f64; 3] = [90.0, 180.0, 270.0];

#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub latitude: f64,
    pub longitude: f64,
    pub above_home: f64,
    pub home_latitude: f64,
    pub home_longitude: f64,
    pub heading: f64,
    pub pitch: f64,
    pub roll: f64,
    pub fov: f64,
    pub aim: (f64, f64),
    pub imagery: String,
    pub north_speed: f64,
    pub east_speed: f64,
    pub climb_speed: f64,
    pub heading_rate: Option<f64>,
    pub pitch_rate: Option<f64>,
    pub roll_rate: f64,
    pub response: Response,
    pub round_trip: Option<Duration>,
    pub bottles: Vec<Bottle>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Climb {
    pub up: f64,
    pub down: f64,
    pub deadzone: f64,
}

impl Climb {
    pub fn commanded(&self, thrust: f64) -> f64 {
        let stick = (thrust - 0.5) * 2.0;
        let past = (stick.abs() - self.deadzone).max(0.0) / (1.0 - self.deadzone).max(f64::EPSILON);
        past.min(1.0) * if stick > 0.0 { self.up } else { -self.down }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Response {
    pub yaw: Option<Gain>,
    pub roll: Option<Gain>,
    pub pitch: Option<Gain>,
    pub forward: Option<Gain>,
    pub right: Option<Gain>,
    pub climb: Option<Climb>,
    pub rate_mode: bool,
}

impl Response {
    pub fn from_view(view: &serde_json::Value) -> Self {
        let number =
            |at: &serde_json::Value, key: &str| at.get(key).and_then(serde_json::Value::as_f64);
        let gain = |key: &str| {
            view.get(key).and_then(|at| {
                Some(Gain {
                    full: number(at, "full")?,
                    expo: number(at, "expo")?,
                    deadzone: number(at, "deadzone")?,
                })
            })
        };
        let climb = view.get("climb").and_then(|at| {
            Some(Climb {
                up: number(at, "up")?,
                down: number(at, "down")?,
                deadzone: number(at, "deadzone")?,
            })
        });
        Self {
            yaw: gain("yaw"),
            roll: gain("roll"),
            pitch: gain("pitch"),
            forward: gain("forward"),
            right: gain("right"),
            climb,
            rate_mode: view.get("rateMode").and_then(serde_json::Value::as_bool) == Some(true),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Telemetry {
    latitude: f64,
    longitude: f64,
    above_home: f64,
    velocity: (f64, f64, f64),
    rates: (Option<f64>, Option<f64>, f64),
    roll: f64,
}

impl Target {
    pub fn telemetry(&self) -> Telemetry {
        Telemetry {
            latitude: self.latitude,
            longitude: self.longitude,
            above_home: self.above_home,
            velocity: (self.north_speed, self.east_speed, self.climb_speed),
            rates: (self.heading_rate, self.pitch_rate, self.roll_rate),
            roll: self.roll,
        }
    }

    pub fn from_view(view: &serde_json::Value) -> Option<Self> {
        let number = |key: &str| {
            view.get(key)
                .and_then(serde_json::Value::as_f64)
                .filter(|v| v.is_finite())
        };
        (view.get("available").and_then(serde_json::Value::as_bool) == Some(true)).then_some(())?;
        Some(Self {
            latitude: number("latitude")?,
            longitude: number("longitude")?,
            above_home: number("aboveHome")?,
            home_latitude: number("homeLatitude")?,
            home_longitude: number("homeLongitude")?,
            heading: number("heading")?,
            pitch: number("pitch")?,
            roll: number("roll")?,
            fov: number("fov")?,
            aim: match view.get("aimable").and_then(serde_json::Value::as_bool) {
                Some(true) => (number("pan").unwrap_or(0.0), number("tilt").unwrap_or(0.0)),
                _ => (0.0, 0.0),
            },
            imagery: view.get("imagery")?.as_str()?.to_string(),
            north_speed: number("northSpeed")?,
            east_speed: number("eastSpeed")?,
            climb_speed: number("climbSpeed")?,
            heading_rate: number("headingRate"),
            pitch_rate: number("pitchRate"),
            roll_rate: number("rollRate")?,
            response: view
                .get("response")
                .map(Response::from_view)
                .unwrap_or_default(),
            round_trip: view
                .get("roundTripMs")
                .and_then(serde_json::Value::as_u64)
                .map(Duration::from_millis),
            bottles: Bottle::listed(view),
        })
    }
}

fn surround(
    tiles: &mut Tileset,
    pose: &Pose,
    width: u32,
    height: u32,
    budget: usize,
    now: Instant,
) -> Vec<Request> {
    SURROUND_TURNS.iter().fold(Vec::new(), |found, turn| {
        let camera = Camera::new(
            &Pose {
                heading: pose.heading + turn,
                ..*pose
            },
            width,
            height,
        );
        let more = tiles
            .select_measured(
                &camera,
                budget.saturating_sub(found.len()),
                SURROUND_COARSER,
                now,
            )
            .wanted;
        found.into_iter().chain(more).collect()
    })
}

pub fn root_url(token: &str, fetcher: &dyn Fetcher) -> Result<Url, String> {
    match token.starts_with("eyJ") {
        true => {
            let endpoint = Url::parse_with_params(
                &format!("https://api.cesium.com/v1/assets/{ION_GOOGLE_ASSET}/endpoint"),
                [("access_token", token)],
            )
            .map_err(|error| error.to_string())?;
            let reply: serde_json::Value = serde_json::from_slice(&fetcher.url(&endpoint)?)
                .map_err(|error| format!("Cesium ion answered unreadable JSON: {error}"))?;
            let url = reply
                .get("options")
                .and_then(|options| options.get("url"))
                .and_then(serde_json::Value::as_str)
                .ok_or("Cesium ion gave no tileset url; check the token can read asset 2275207")?;
            Url::parse(url).map_err(|error| error.to_string())
        }
        false => {
            Url::parse_with_params(GOOGLE_ROOT, [("key", token)]).map_err(|error| error.to_string())
        }
    }
}

#[derive(Clone)]
enum Job {
    Root(String),
    Tile(Request),
    Open(Want),
}

enum Done {
    Root(Result<(Url, tileset::Tile), String>),
    Tile(Vec<usize>, Result<Parsed, String>),
    Open(Built, Option<String>),
}

#[derive(Default)]
struct Tally {
    bytes: AtomicU64,
    from_disk: AtomicU64,
}

struct Counted<'a> {
    inner: &'a dyn Fetcher,
    tally: &'a Tally,
}

impl Fetcher for Counted<'_> {
    fn url(&self, url: &Url) -> Result<Vec<u8>, String> {
        self.inner.url(url).inspect(|bytes| {
            self.tally
                .bytes
                .fetch_add(bytes.len() as u64, Ordering::Relaxed);
        })
    }

    fn tile(&self, url: &Url) -> Result<(Vec<u8>, Answer), String> {
        self.inner.tile(url).inspect(|(bytes, answer)| {
            self.tally
                .bytes
                .fetch_add(bytes.len() as u64, Ordering::Relaxed);
            self.tally
                .from_disk
                .fetch_add(u64::from(*answer != Answer::Downloaded), Ordering::Relaxed);
        })
    }

    fn root(&self, url: &Url) -> Result<Vec<u8>, String> {
        self.inner.root(url).inspect(|bytes| {
            self.tally
                .bytes
                .fetch_add(bytes.len() as u64, Ordering::Relaxed);
        })
    }

    fn map(&self, provider: &str, z: u32, x: u32, y: u32) -> Result<Vec<u8>, String> {
        self.inner.map(provider, z, x, y).inspect(|bytes| {
            self.tally
                .bytes
                .fetch_add(bytes.len() as u64, Ordering::Relaxed);
        })
    }
}

fn work(job: Job, fetcher: &dyn Fetcher) -> Done {
    match job {
        Job::Root(token) => Done::Root(root_url(&token, fetcher).and_then(|url| {
            let json = fetcher.root(&url)?;
            tileset::root(&json, &url)
                .map(|root| (url, root))
                .ok_or_else(|| "the root tileset was unreadable".to_string())
        })),
        Job::Tile(request) => Done::Tile(
            request.path,
            fetcher
                .tile(&request.url)
                .and_then(|(bytes, _)| tileset::parse(&request.url, &bytes)),
        ),
        Job::Open(want) => {
            let what = want.describe();
            let built = want.build(fetcher);
            let error = match &built {
                Built::Heights(_, Err(error))
                | Built::Houses(_, Err(error))
                | Built::Terrain {
                    mesh: Err(error), ..
                } => Some(format!("{what}: {error}")),
                Built::Terrain {
                    mesh: Ok((_, false)),
                    ..
                } => Some(format!("{what}: no imagery")),
                _ => None,
            };
            Done::Open(built, error)
        }
    }
}

fn lost(job: &Job) -> Done {
    match job {
        Job::Root(_) => Done::Root(Err("the tileset worker crashed".to_string())),
        Job::Tile(request) => Done::Tile(
            request.path.clone(),
            Err("the tile worker crashed".to_string()),
        ),
        Job::Open(want) => Done::Open(
            want.failed("the tile worker crashed"),
            Some(format!("{}: the tile worker crashed", want.describe())),
        ),
    }
}

#[derive(Default)]
struct Probe {
    at: Option<(f64, f64)>,
    height: Option<f64>,
}

impl Probe {
    fn for_spot(&mut self, latitude: f64, longitude: f64) {
        self.height = self
            .height
            .filter(|_| self.at == Some((latitude, longitude)));
        self.at = Some((latitude, longitude));
    }
}

enum Source {
    Google(Option<Tileset>),
    Open(OpenScene),
}

struct Fallback {
    scene: OpenScene,
    pool: Pool<Job, Done>,
    home: Probe,
    under: Probe,
    terrain: Vec<Arc<Mesh>>,
    picked: Option<Instant>,
}

impl Fallback {
    fn room(&self) -> usize {
        self.pool.room(STAND_IN_IN_FLIGHT)
    }
}

fn workers(
    count: usize,
    name: &str,
    fetcher: Arc<dyn Fetcher>,
    tally: Arc<Tally>,
) -> Result<Pool<Job, Done>, String> {
    Pool::new(
        count,
        name,
        move |job: Job| {
            work(
                job,
                &Counted {
                    inner: fetcher.as_ref(),
                    tally: &tally,
                },
            )
        },
        lost,
    )
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stats {
    pub requested: u64,
    pub loaded: u64,
    pub failed: u64,
    pub bytes: u64,
    pub from_disk: u64,
    pub errors: BTreeMap<String, u64>,
    pub waits_ms: Vec<u64>,
}

pub struct Frame {
    pub pose: Pose,
    pub ground: Vec<Arc<Mesh>>,
    pub backfill: Vec<Arc<Mesh>>,
    pub warm: Vec<Arc<Mesh>>,
    pub home_ground: Option<f64>,
    pub under_ground: Option<f64>,
    pub bottles: Vec<Bottle>,
    pub changed: bool,
    pub reading_age: Duration,
    pub stick_to_frame: Option<Duration>,
}

impl Frame {
    pub fn view(
        &self,
        overlays: Option<&Overlays>,
        width: u32,
        height: u32,
    ) -> (Camera, Vec<Arc<Mesh>>) {
        let camera = Camera::new(&self.pose, width, height);
        let marks = overlays
            .zip(self.home_ground)
            .map(|(overlays, ground)| overlay::meshes(overlays, ground, &camera))
            .unwrap_or_default();
        let bottles = self
            .home_ground
            .and_then(|ground| overlay::bottles(&self.bottles, ground, &camera));
        (
            camera,
            self.ground
                .iter()
                .cloned()
                .chain(marks)
                .chain(bottles)
                .collect(),
        )
    }

    pub fn ground_above_home(&self) -> Option<f64> {
        self.under_ground
            .zip(self.home_ground)
            .map(|(under, home)| under - home)
    }
}

pub struct Session {
    token: String,
    source: Source,
    pool: Pool<Job, Done>,
    tally: Arc<Tally>,
    stats: Stats,
    home: Probe,
    under: Probe,
    probed: Option<Instant>,
    surrounded: Option<Instant>,
    shown: Option<Pose>,
    ticked: Option<Instant>,
    selected: Option<Instant>,
    seen: Option<Target>,
    telemetry_since: Option<(Telemetry, Instant)>,
    shown_touch: Option<Instant>,
    turn: Option<Turn>,
    aim: (f64, f64),
    spot_since: Option<((f64, f64), Instant)>,
    height_since: Option<(f64, Instant)>,
    attitude_since: Option<((f64, f64, f64), Instant)>,
    spin: f64,
    ground_meshes: Vec<Arc<Mesh>>,
    backfill: Vec<Arc<Mesh>>,
    bottle_ground: Vec<((f64, f64), f64)>,
    bottles_probed: Option<Instant>,
    fallback: Option<Fallback>,
    coarsest: f64,
    holes: usize,
    first_frame: Option<Instant>,
    handed_over_after: Option<Duration>,
    warming: Option<usize>,
    root_due: Option<Instant>,
}

fn ground_at(grounds: &[((f64, f64), f64)], spot: (f64, f64)) -> Option<f64> {
    grounds
        .iter()
        .find(|(at, _)| *at == spot)
        .map(|(_, height)| *height)
}

fn wrapped(degrees: f64) -> f64 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aim {
    pub heading: f64,
    pub pitch: f64,
}

impl Aim {
    fn apart(&self, other: &Aim) -> f64 {
        wrapped(other.heading - self.heading)
            .abs()
            .max((other.pitch - self.pitch).abs())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Turn {
    from: Aim,
    to: Aim,
    at: Instant,
    every: Duration,
}

impl Turn {
    pub fn aim_at(&self, now: Instant) -> Aim {
        let progress = (now.saturating_duration_since(self.at).as_secs_f64()
            / (self.every.as_secs_f64() * TURN_STRETCH))
            .min(1.0);
        Aim {
            heading: self.from.heading + wrapped(self.to.heading - self.from.heading) * progress,
            pitch: self.from.pitch + (self.to.pitch - self.from.pitch) * progress,
        }
    }
}

pub fn turned(turn: Option<Turn>, aim: Aim, now: Instant) -> Turn {
    match turn {
        None => Turn {
            from: aim,
            to: aim,
            at: now,
            every: TURN_EVERY_GUESS,
        },
        Some(turn) if aim.apart(&turn.to) < SETTLED_ANGLE => turn,
        Some(turn) => {
            let gap = now
                .saturating_duration_since(turn.at)
                .clamp(TURN_SHORTEST, TURN_LONGEST);
            Turn {
                from: turn.aim_at(now),
                to: aim,
                at: now,
                every: (turn.every + gap) / 2,
            }
        }
    }
}

fn since<T: PartialEq>(
    seen: Option<(T, Instant)>,
    reading: T,
    now: Instant,
    ticked: Option<Instant>,
) -> (T, Instant) {
    let noticed = now
        - ticked.map_or(Duration::ZERO, |before| {
            now.saturating_duration_since(before).min(LONGEST_STEP) / 2
        });
    let at = seen
        .as_ref()
        .filter(|(before, _)| *before == reading)
        .map_or(noticed, |(_, at)| *at);
    (reading, at)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Motion {
    pub north: f64,
    pub east: f64,
    pub climb: f64,
    pub heading: f64,
    pub pitch: f64,
    pub roll: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub spot: Instant,
    pub height: Instant,
    pub attitude: Instant,
}

fn moved(at: (f64, f64), north: f64, east: f64) -> (f64, f64) {
    crate::surveygrid::at_distance_and_azimuth(
        at,
        north.hypot(east),
        east.atan2(north).to_degrees(),
    )
}

pub fn ahead(
    target: &Target,
    sticks: &[StickSample],
    read: Reading,
    now: Instant,
) -> (Target, Motion) {
    let half = target.round_trip.map_or(Duration::ZERO, |trip| trip / 2);
    let driven = sticks
        .last()
        .is_some_and(|last| now.saturating_duration_since(last.at) < STICKS_LIVE);
    let axis = |start: f64,
                read_at: Instant,
                longest: Duration,
                lag: Lag,
                command: Option<&dyn Fn(&StickSample) -> f64>|
     -> Predicted {
        let from = read_at.checked_sub(half).unwrap_or(read_at);
        let delay = half + lag.dead;
        let steps: Vec<Step> = command
            .filter(|_| driven)
            .map(|command| {
                sticks
                    .iter()
                    .map(|sample| Step {
                        at: sample.at,
                        value: command(sample),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let until = now
            + DISPLAY_LEAD
            + if steps.is_empty() {
                Duration::ZERO
            } else {
                delay
            };
        let capped = until.min(from + longest);
        let ahead = predict::predicted(start, from, capped, &steps, delay, lag);
        Predicted {
            value: if until > capped { 0.0 } else { ahead.value },
            ..ahead
        }
    };
    let response = target.response;
    let yaw_lag = if response.rate_mode {
        predict::ACRO_YAW
    } else {
        predict::LOITER_YAW
    };
    let yaw = response
        .yaw
        .filter(|_| target.heading_rate.is_some())
        .map(|gain| move |sample: &StickSample| gain.commanded(sample.yaw));
    let roll = response
        .roll
        .map(|gain| move |sample: &StickSample| gain.commanded(sample.roll));
    let pitch = response
        .pitch
        .map(|gain| move |sample: &StickSample| -gain.commanded(sample.pitch));
    let (sin, cos) = target.heading.to_radians().sin_cos();
    let along = response.forward.zip(response.right);
    let north = along.map(|(forward, right)| {
        move |sample: &StickSample| {
            forward.commanded(sample.pitch) * cos - right.commanded(sample.roll) * sin
        }
    });
    let east = along.map(|(forward, right)| {
        move |sample: &StickSample| {
            forward.commanded(sample.pitch) * sin + right.commanded(sample.roll) * cos
        }
    });
    let climb = response
        .climb
        .map(|climb| move |sample: &StickSample| climb.commanded(sample.thrust));
    let yaw = yaw.as_ref().map(|f| f as &dyn Fn(&StickSample) -> f64);
    let pitch = pitch.as_ref().map(|f| f as &dyn Fn(&StickSample) -> f64);
    let roll = roll.as_ref().map(|f| f as &dyn Fn(&StickSample) -> f64);
    let airframe = response
        .rate_mode
        .then(|| target.heading_rate.zip(target.pitch_rate))
        .flatten();
    let ((heading, tilt, bank), (heading_rate, pitch_rate, roll_rate)) = match airframe {
        Some((heading_rate, pitch_rate)) => {
            let [p, q, r] = crate::synthview::body_rates(
                target.roll,
                target.pitch,
                (target.roll_rate, pitch_rate, heading_rate),
            );
            let spin = [
                axis(
                    p,
                    read.attitude,
                    LONGEST_TURN_GUESS,
                    predict::ACRO_TILT,
                    roll,
                ),
                axis(
                    q,
                    read.attitude,
                    LONGEST_TURN_GUESS,
                    predict::ACRO_TILT,
                    pitch,
                ),
                axis(r, read.attitude, LONGEST_TURN_GUESS, yaw_lag, yaw),
            ];
            rotated(target, spin)
        }
        None => {
            let turning = axis(
                target.heading_rate.unwrap_or(0.0),
                read.attitude,
                LONGEST_TURN_GUESS,
                yaw_lag,
                yaw,
            );
            let nodding = axis(
                target.pitch_rate.unwrap_or(0.0),
                read.attitude,
                LONGEST_TURN_GUESS,
                predict::ACRO_TILT,
                pitch,
            );
            let banking = axis(
                target.roll_rate,
                read.attitude,
                LONGEST_TURN_GUESS,
                predict::ACRO_TILT,
                roll,
            );
            (
                (
                    target.heading + turning.travelled,
                    target.pitch + nodding.travelled,
                    target.roll + banking.travelled,
                ),
                (turning.value, nodding.value, banking.value),
            )
        }
    };
    let northing = axis(
        target.north_speed,
        read.spot,
        LONGEST_GUESS,
        predict::LOITER_SPEED,
        north.as_ref().map(|f| f as &dyn Fn(&StickSample) -> f64),
    );
    let easting = axis(
        target.east_speed,
        read.spot,
        LONGEST_GUESS,
        predict::LOITER_SPEED,
        east.as_ref().map(|f| f as &dyn Fn(&StickSample) -> f64),
    );
    let climbing = axis(
        target.climb_speed,
        read.height,
        LONGEST_GUESS,
        predict::CLIMB,
        climb.as_ref().map(|f| f as &dyn Fn(&StickSample) -> f64),
    );
    let (latitude, longitude) = moved(
        (target.latitude, target.longitude),
        northing.travelled,
        easting.travelled,
    );
    let guess = Target {
        latitude,
        longitude,
        above_home: target.above_home + climbing.travelled,
        heading,
        pitch: tilt,
        roll: bank,
        ..target.clone()
    };
    let motion = Motion {
        north: northing.value,
        east: easting.value,
        climb: climbing.value,
        heading: heading_rate,
        pitch: pitch_rate,
        roll: roll_rate,
    };
    (guess, motion)
}

fn rotated(target: &Target, [p, q, r]: [Predicted; 3]) -> ((f64, f64, f64), (f64, f64, f64)) {
    let start = DQuat::from_euler(
        EulerRot::ZYX,
        target.heading.to_radians(),
        target.pitch.to_radians(),
        target.roll.to_radians(),
    );
    let turned = start
        * DQuat::from_scaled_axis(
            DVec3::new(p.travelled, q.travelled, r.travelled) * std::f64::consts::PI / 180.0,
        );
    let (heading, pitch, roll) = turned.to_euler(EulerRot::ZYX);
    let (pitch, roll) = (pitch.to_degrees(), roll.to_degrees());
    let heading =
        target.heading + (heading.to_degrees() - target.heading + 180.0).rem_euclid(360.0) - 180.0;
    let (roll_rate, pitch_rate, heading_rate) =
        crate::synthview::euler_rates(roll, pitch, [p.value, q.value, r.value]);
    (
        (heading, pitch, roll),
        (heading_rate, pitch_rate, roll_rate),
    )
}

pub fn advanced(pose: Pose, motion: &Motion, elapsed: Duration) -> Pose {
    let seconds = elapsed.min(LONGEST_STEP).as_secs_f64();
    let (latitude, longitude) = moved(
        (pose.latitude, pose.longitude),
        motion.north * seconds,
        motion.east * seconds,
    );
    Pose {
        latitude,
        longitude,
        height: pose.height + motion.climb * seconds,
        heading: (pose.heading + motion.heading * seconds).rem_euclid(360.0),
        pitch: pose.pitch + motion.pitch * seconds,
        roll: pose.roll + motion.roll * seconds,
        fov: pose.fov,
    }
}

fn toward(from: Pose, to: Pose, alpha: f64) -> Pose {
    let step = |a: f64, b: f64| a + (b - a) * alpha;
    Pose {
        latitude: step(from.latitude, to.latitude),
        longitude: step(from.longitude, to.longitude),
        height: step(from.height, to.height),
        heading: from.heading + wrapped(to.heading - from.heading) * alpha,
        pitch: step(from.pitch, to.pitch),
        roll: from.roll + wrapped(to.roll - from.roll) * alpha,
        fov: to.fov,
    }
}

pub fn turn_detail(spin_deg_s: f64) -> f64 {
    (1.0 + spin_deg_s.max(0.0) / DEG_S_PER_DETAIL_STEP).min(COARSEST_WHILE_TURNING)
}

pub fn spin_of(before: &Pose, after: &Pose, elapsed: Duration) -> f64 {
    wrapped(after.heading - before.heading).hypot(after.pitch - before.pitch)
        / elapsed.as_secs_f64().max(1e-3)
}

fn spun(previous: f64, latest: Option<f64>) -> f64 {
    latest.map_or(previous, |rate| {
        previous + (rate - previous) * SPIN_SMOOTHING
    })
}

pub fn follow(elapsed: Duration) -> f64 {
    1.0 - (1.0 - FOLLOW_AT_60_HZ).powf(elapsed.min(LONGEST_STEP).as_secs_f64() * 60.0)
}

fn settled(a: &Pose, b: &Pose) -> bool {
    (a.latitude - b.latitude).abs() < SETTLED_DEGREES
        && (a.longitude - b.longitude).abs() < SETTLED_DEGREES
        && (a.height - b.height).abs() < SETTLED_METRES
        && [
            wrapped(a.heading - b.heading),
            a.pitch - b.pitch,
            wrapped(a.roll - b.roll),
        ]
        .iter()
        .all(|turn| turn.abs() < SETTLED_ANGLE)
}

pub fn google_ready(
    loaded: bool,
    coarsest: f64,
    home_ground: Option<f64>,
    still_to_upload: Option<usize>,
) -> bool {
    loaded
        && coarsest <= GOOGLE_READY_COARSEST
        && home_ground.is_some()
        && still_to_upload.is_none_or(|left| left == 0)
}

pub fn camera_height(target: &Target, home_ground: Option<f64>, under_ground: Option<f64>) -> f64 {
    let above_home = home_ground.map(|ground| ground + target.above_home);
    let clear = under_ground.map(|ground| ground + CLEARANCE_M);
    match (above_home, clear) {
        (Some(planned), Some(floor)) => planned.max(floor),
        (Some(planned), None) => planned,
        (None, Some(floor)) => floor + target.above_home.max(0.0),
        (None, None) => UNKNOWN_GROUND_M + target.above_home,
    }
}

impl Session {
    pub fn new(token: &str, fetcher: Arc<dyn Fetcher>) -> Result<Self, String> {
        let tally = Arc::new(Tally::default());
        let mut pool = workers(WORKERS, "synthetic-fetch", fetcher.clone(), tally.clone())?;
        let google = !token.is_empty();
        if google {
            pool.send(Job::Root(token.to_string()));
        }
        let source = if google {
            Source::Google(None)
        } else {
            Source::Open(OpenScene::new())
        };
        let fallback = match google {
            true => Some(Fallback {
                scene: OpenScene::new(),
                pool: workers(
                    STAND_IN_WORKERS,
                    "synthetic-stand-in",
                    fetcher,
                    tally.clone(),
                )?,
                home: Probe::default(),
                under: Probe::default(),
                terrain: Vec::new(),
                picked: None,
            }),
            false => None,
        };
        Ok(Self {
            token: token.to_string(),
            source,
            pool,
            tally,
            stats: Stats::default(),
            home: Probe::default(),
            under: Probe::default(),
            probed: None,
            surrounded: None,
            shown: None,
            ticked: None,
            selected: None,
            seen: None,
            telemetry_since: None,
            shown_touch: None,
            turn: None,
            aim: (0.0, 0.0),
            spot_since: None,
            height_since: None,
            attitude_since: None,
            spin: 0.0,
            ground_meshes: Vec::new(),
            backfill: Vec::new(),
            bottle_ground: Vec::new(),
            bottles_probed: None,
            fallback,
            coarsest: f64::INFINITY,
            holes: 0,
            first_frame: None,
            handed_over_after: None,
            warming: None,
            root_due: None,
        })
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn source(&self) -> &'static str {
        match self.source {
            Source::Google(_) => "google",
            Source::Open(_) => "open",
        }
    }

    pub fn showing(&self) -> &'static str {
        match (&self.source, self.fallback.is_some() && !self.handed_over()) {
            (Source::Google(_), true) => "open_fallback",
            (Source::Google(_), false) => "google",
            (Source::Open(_), _) => "open",
        }
    }

    fn handed_over(&self) -> bool {
        self.handed_over_after.is_some()
    }

    pub fn coarsest(&self) -> f64 {
        self.coarsest
    }

    pub fn holes(&self) -> usize {
        self.holes
    }

    pub fn handover(&self) -> Option<Duration> {
        self.handed_over_after
    }

    pub fn warming(&self) -> Option<usize> {
        self.warming
    }

    pub fn warmed(&mut self, still_to_upload: usize) {
        self.warming = Some(still_to_upload);
    }

    pub fn in_flight(&self) -> usize {
        self.pool.in_flight()
            + self
                .fallback
                .as_ref()
                .map_or(0, |fallback| fallback.pool.in_flight())
    }

    pub fn loaded_tiles(&self) -> usize {
        match &self.source {
            Source::Google(tiles) => tiles.as_ref().map_or(0, Tileset::loaded_tiles),
            Source::Open(scene) => scene.loaded_tiles(),
        }
    }

    pub fn take_stats(&mut self) -> Stats {
        Stats {
            bytes: self.tally.bytes.swap(0, Ordering::Relaxed),
            from_disk: self.tally.from_disk.swap(0, Ordering::Relaxed),
            ..std::mem::take(&mut self.stats)
        }
    }

    fn failed(&mut self, error: String) {
        self.stats.failed += 1;
        *self.stats.errors.entry(error).or_insert(0) += 1;
    }

    fn collect(&mut self, now: Instant) -> usize {
        let stood_in = self
            .fallback
            .as_mut()
            .map(|fallback| fallback.pool.finished())
            .unwrap_or_default();
        let finished: Vec<Done> = self.pool.finished().into_iter().chain(stood_in).collect();
        let count = finished.len();
        finished.into_iter().for_each(|done| match done {
            Done::Root(Ok((url, root))) => {
                self.stats.loaded += 1;
                if let Source::Google(tiles) = &mut self.source {
                    *tiles = Some(Tileset::new(root, &url));
                }
            }
            Done::Root(Err(error)) => {
                self.failed(format!("tileset root: {error}"));
                self.root_due = Some(now + tileset::RETRY_AFTER);
            }
            Done::Tile(path, parsed) => {
                match &parsed {
                    Ok(_) => self.stats.loaded += 1,
                    Err(error) => self.failed(format!("google tile: {error}")),
                }
                if let Source::Google(Some(tiles)) = &mut self.source {
                    let waited = tiles.deliver(&path, parsed, now);
                    self.stats.waits_ms.extend(
                        waited.map(|waited| u64::try_from(waited.as_millis()).unwrap_or(u64::MAX)),
                    );
                }
            }
            Done::Open(built, error) => {
                match error {
                    Some(error) => self.failed(error),
                    None => self.stats.loaded += 1,
                }
                match (&mut self.source, &mut self.fallback) {
                    (Source::Open(scene), _) => scene.deliver(built, now),
                    (_, Some(fallback)) => fallback.scene.deliver(built, now),
                    _ => {}
                }
            }
        });
        count
    }

    fn send(&mut self, jobs: Vec<Job>) {
        self.stats.requested += jobs.len() as u64;
        jobs.into_iter().for_each(|job| self.pool.send(job));
    }

    fn room(&self) -> usize {
        self.pool.room(MAX_IN_FLIGHT)
    }

    fn google_ground(
        &mut self,
        latitude: f64,
        longitude: f64,
        known: Option<f64>,
        now: Instant,
    ) -> Option<f64> {
        let camera = Camera::new(
            &Pose {
                latitude,
                longitude,
                height: PROBE_HEIGHT_M,
                heading: 0.0,
                pitch: -90.0,
                roll: 0.0,
                fov: PROBE_FOV,
            },
            PROBE_PX,
            PROBE_PX,
        );
        let room = self.room();
        let Source::Google(Some(tiles)) = &mut self.source else {
            return known;
        };
        let (meshes, wanted) = tiles.select(&camera, room, now);
        self.send(wanted.into_iter().map(Job::Tile).collect());
        ground::height_at(&meshes, latitude, longitude).or(known)
    }

    fn open_ground(
        &mut self,
        latitude: f64,
        longitude: f64,
        known: Option<f64>,
        now: Instant,
    ) -> Option<f64> {
        let room = self.room();
        let Source::Open(scene) = &mut self.source else {
            return known;
        };
        let (found, wanted) = scene.ground(latitude, longitude, room, now);
        self.send(wanted.into_iter().map(Job::Open).collect());
        found.or(known)
    }

    fn stand_in(&mut self, jobs: Vec<Want>) {
        self.stats.requested += jobs.len() as u64;
        if let Some(fallback) = self.fallback.as_mut() {
            jobs.into_iter()
                .for_each(|want| fallback.pool.send(Job::Open(want)));
        }
    }

    fn probe_fallback(&mut self, target: &Target, now: Instant) {
        let Some(fallback) = self.fallback.as_mut() else {
            return;
        };
        let room = fallback.room();
        fallback
            .home
            .for_spot(target.home_latitude, target.home_longitude);
        fallback.under.for_spot(target.latitude, target.longitude);
        let (home, home_wanted) =
            fallback
                .scene
                .ground(target.home_latitude, target.home_longitude, room, now);
        let (under, under_wanted) = fallback.scene.ground(
            target.latitude,
            target.longitude,
            room.saturating_sub(home_wanted.len()),
            now,
        );
        fallback.home.height = home.or(fallback.home.height);
        fallback.under.height = under.or(fallback.under.height);
        self.stand_in(home_wanted.into_iter().chain(under_wanted).collect());
    }

    fn probe(&mut self, target: &Target, now: Instant) {
        if !self.handed_over() {
            self.probe_fallback(target, now);
        }
        let open = matches!(self.source, Source::Open(_));
        self.home
            .for_spot(target.home_latitude, target.home_longitude);
        self.under.for_spot(target.latitude, target.longitude);
        if !(open
            || self
                .probed
                .is_none_or(|at| now.duration_since(at) >= PROBE_EVERY))
        {
            return;
        }
        self.probed = Some(now);
        let (home_known, under_known) = (self.home.height, self.under.height);
        self.home.height = match open {
            true => self.open_ground(target.home_latitude, target.home_longitude, home_known, now),
            false => {
                self.google_ground(target.home_latitude, target.home_longitude, home_known, now)
            }
        };
        self.under.height = match open {
            true => self.open_ground(target.latitude, target.longitude, under_known, now),
            false => self.google_ground(
                target.latitude,
                target.longitude,
                under_known.or(self.home.height),
                now,
            ),
        };
    }

    fn select(
        &mut self,
        pose: &Pose,
        width: u32,
        height: u32,
        target: &Target,
        now: Instant,
    ) -> bool {
        let camera = &Camera::new(pose, width, height);
        let room = self.room().saturating_sub(PROBE_SLOTS);
        let detail = turn_detail(self.spin);
        let handed_over = self.handed_over();
        let google_ground = self.under.height.or(self.home.height);
        let (stand_in, stand_in_jobs) = match self.fallback.as_mut() {
            Some(fallback) => {
                let room = fallback.room();
                let (meshes, wanted) = match handed_over {
                    true if fallback
                        .picked
                        .is_some_and(|at| now.duration_since(at) < TERRAIN_UNDER_EVERY) =>
                    {
                        (fallback.terrain.clone(), Vec::new())
                    }
                    true => {
                        let (terrain, wanted) = fallback.scene.select_terrain(
                            camera,
                            &target.imagery,
                            google_ground.unwrap_or(0.0),
                            detail * TERRAIN_UNDER_COARSER,
                            room,
                            now,
                        );
                        fallback.terrain = terrain.clone();
                        fallback.picked = Some(now);
                        (terrain, wanted)
                    }
                    false => {
                        let ground = fallback
                            .under
                            .height
                            .or(fallback.home.height)
                            .unwrap_or(0.0);
                        fallback.scene.select(
                            camera,
                            &target.imagery,
                            ground,
                            target.latitude,
                            target.longitude,
                            detail,
                            room,
                            now,
                        )
                    }
                };
                (Some(meshes), wanted)
            }
            None => (None, Vec::new()),
        };
        let ground = self.under.height.or(self.home.height).unwrap_or(0.0);
        let (meshes, jobs, coarsest, holes, backfill) = match &mut self.source {
            Source::Google(Some(tiles)) => {
                let due = handed_over
                    && self
                        .surrounded
                        .is_none_or(|at| now.duration_since(at) >= SURROUND_EVERY);
                let around = match due {
                    true => surround(tiles, pose, width, height, room.min(SURROUND_SLOTS), now),
                    false => Vec::new(),
                };
                self.surrounded = if due { Some(now) } else { self.surrounded };
                let kept = if handed_over { SURROUND_SLOTS } else { 0 };
                let Selection {
                    meshes,
                    wanted,
                    coarsest,
                    holes,
                    backfill,
                } = tiles.select_measured(
                    camera,
                    room.saturating_sub(kept),
                    if handed_over {
                        detail
                    } else {
                        detail * COARSER_UNTIL_HANDOVER
                    },
                    now,
                );
                tiles.trim(now, self.probed);
                (
                    meshes,
                    around.into_iter().chain(wanted).map(Job::Tile).collect(),
                    coarsest,
                    holes,
                    backfill,
                )
            }
            Source::Open(scene) => {
                let (meshes, wanted) = scene.select(
                    camera,
                    &target.imagery,
                    ground,
                    target.latitude,
                    target.longitude,
                    detail,
                    room,
                    now,
                );
                (
                    meshes,
                    wanted.into_iter().map(Job::Open).collect(),
                    0.0,
                    0,
                    Vec::new(),
                )
            }
            Source::Google(None) => (Vec::new(), Vec::new(), f64::INFINITY, 0, Vec::new()),
        };
        self.coarsest = coarsest;
        self.holes = holes;
        let (drawn, underneath) = match (stand_in, handed_over) {
            (Some(terrain), true) => (meshes, terrain.into_iter().chain(backfill).collect()),
            (Some(stand_in), false) => (stand_in, Vec::new()),
            (None, _) => (meshes, backfill),
        };
        let changed = |now: &[Arc<Mesh>], before: &[Arc<Mesh>]| {
            now.len() != before.len() || now.iter().zip(before).any(|(a, b)| a.id != b.id)
        };
        let different =
            changed(&drawn, &self.ground_meshes) || changed(&underneath, &self.backfill);
        self.backfill = underneath;
        self.ground_meshes = drawn;
        self.selected = Some(now);
        self.stand_in(stand_in_jobs);
        self.send(jobs);
        different
    }

    fn grounded(
        &mut self,
        bottles: &[Bottle],
        home_ground: Option<f64>,
        now: Instant,
    ) -> Vec<Bottle> {
        let due = self
            .bottles_probed
            .is_none_or(|at| now.duration_since(at) >= PROBE_EVERY);
        self.bottles_probed = due.then_some(now).or(self.bottles_probed);
        let ground: Vec<((f64, f64), f64)> = bottles
            .iter()
            .filter(|bottle| bottle.landed)
            .map(|bottle| (bottle.latitude, bottle.longitude))
            .filter_map(|spot| {
                ground_at(&self.bottle_ground, spot)
                    .filter(|_| !due)
                    .or_else(|| ground::height_at(&self.ground_meshes, spot.0, spot.1))
                    .map(|height| (spot, height))
            })
            .collect();
        self.bottle_ground = ground;
        bottles
            .iter()
            .map(|bottle| {
                let drawn = ground_at(&self.bottle_ground, (bottle.latitude, bottle.longitude))
                    .filter(|_| bottle.landed)
                    .zip(home_ground);
                Bottle {
                    above_home: drawn.map_or(bottle.above_home, |(height, home)| height - home),
                    ..bottle.clone()
                }
            })
            .collect()
    }

    pub fn frame(
        &mut self,
        target: Option<&Target>,
        sticks: &[StickSample],
        width: u32,
        height: u32,
        now: Instant,
    ) -> Option<Frame> {
        let delivered = self.collect(now);
        if matches!(self.source, Source::Google(None))
            && self.root_due.is_some_and(|due| now >= due)
        {
            self.root_due = None;
            self.pool.send(Job::Root(self.token.clone()));
        }
        let target = target?;
        let started = *self.first_frame.get_or_insert(now);
        if self.fallback.is_some()
            && !self.handed_over()
            && google_ready(
                matches!(self.source, Source::Google(Some(_))),
                self.coarsest,
                self.home.height,
                self.warming,
            )
        {
            self.shown = None;
            self.handed_over_after = Some(now.saturating_duration_since(started));
        }
        if matches!(self.source, Source::Google(None)) && self.fallback.is_none() {
            return None;
        }
        self.probe(target, now);
        let turn = turned(
            self.turn,
            Aim {
                heading: target.heading,
                pitch: target.pitch,
            },
            now,
        );
        self.turn = Some(turn);
        let aim = turn.aim_at(now);
        let (home_ground, under_ground) =
            match self.fallback.as_ref().filter(|_| !self.handed_over()) {
                Some(fallback) => (fallback.home.height, fallback.under.height),
                None => (self.home.height, self.under.height),
            };
        let telemetry = since(self.telemetry_since, target.telemetry(), now, self.ticked);
        self.telemetry_since = Some(telemetry);
        let spot = since(
            self.spot_since,
            (target.latitude, target.longitude),
            now,
            self.ticked,
        );
        let altitude = since(self.height_since, target.above_home, now, self.ticked);
        let attitude = since(
            self.attitude_since,
            (target.heading, target.pitch, target.roll),
            now,
            self.ticked,
        );
        self.spot_since = Some(spot);
        self.height_since = Some(altitude);
        self.attitude_since = Some(attitude);
        let (guess, motion) = ahead(
            target,
            sticks,
            Reading {
                spot: spot.1,
                height: altitude.1,
                attitude: attitude.1,
            },
            now,
        );
        let touched = sticks
            .iter()
            .rev()
            .find_map(|sample| sample.touched)
            .filter(|touched| self.shown_touch.is_none_or(|shown| *touched > shown));
        self.shown_touch = touched.or(self.shown_touch);
        let goal = Pose {
            latitude: guess.latitude,
            longitude: guess.longitude,
            height: camera_height(&guess, home_ground, under_ground),
            heading: target.heading_rate.map_or(aim.heading, |_| guess.heading),
            pitch: target.pitch_rate.map_or(aim.pitch, |_| guess.pitch),
            roll: guess.roll,
            fov: target.fov,
        };
        let elapsed = self.ticked.map(|at| now.duration_since(at));
        self.ticked = Some(now);
        let reaimed = (
            wrapped(target.aim.0 - self.aim.0),
            target.aim.1 - self.aim.1,
        );
        self.aim = target.aim;
        let pose = self.shown.zip(elapsed).map_or(goal, |(shown, elapsed)| {
            let aimed = Pose {
                heading: shown.heading + reaimed.0,
                pitch: shown.pitch + reaimed.1,
                ..shown
            };
            toward(advanced(aimed, &motion, elapsed), goal, follow(elapsed))
        });
        let moving = self.shown.is_none_or(|before| !settled(&before, &pose));
        self.spin = spun(
            self.spin,
            self.shown
                .zip(elapsed)
                .map(|(before, elapsed)| spin_of(&before, &pose, elapsed)),
        );
        self.shown = Some(pose);
        let fresh = self.seen.as_ref() != Some(target);
        self.seen = Some(target.clone());
        let due = self
            .selected
            .is_none_or(|at| now.duration_since(at) >= RESELECT_EVERY);
        let reselected = (delivered > 0 || moving || fresh || due)
            && self.select(&pose, width, height, target, now);
        let warm = match &mut self.source {
            Source::Google(Some(tiles)) => tiles.take_fresh(),
            _ => Vec::new(),
        };
        let bottles = self.grounded(&target.bottles, home_ground, now);
        Some(Frame {
            pose,
            ground: self.ground_meshes.clone(),
            backfill: self.backfill.clone(),
            warm,
            home_ground,
            under_ground,
            bottles,
            changed: moving || fresh || reselected,
            reading_age: now.saturating_duration_since(telemetry.1),
            stick_to_frame: touched.map(|at| now.saturating_duration_since(at)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::open::TERRAIN;

    struct Fake;

    impl Fetcher for Fake {
        fn url(&self, url: &Url) -> Result<Vec<u8>, String> {
            match url.host_str() {
                Some("api.cesium.com") => Ok(br#"{"options":{"url":"https://tile.googleapis.com/v1/3dtiles/root.json?key=FromIon"}}"#.to_vec()),
                _ => Err("offline".into()),
            }
        }

        fn map(&self, provider: &str, z: u32, x: u32, y: u32) -> Result<Vec<u8>, String> {
            match provider {
                TERRAIN => {
                    let mut png = std::io::Cursor::new(Vec::new());
                    image::RgbImage::from_pixel(4, 4, image::Rgb([130, 92, 0]))
                        .write_to(&mut png, image::ImageFormat::Png)
                        .map_err(|e| e.to_string())?;
                    Ok(png.into_inner())
                }
                "Panics" => panic!("a broken provider"),
                _ => Err(format!("no {provider} {z}/{x}/{y}")),
            }
        }
    }

    fn target(imagery: &str) -> Target {
        Target {
            latitude: -35.36213,
            longitude: 149.16514,
            above_home: 30.0,
            home_latitude: -35.363262,
            home_longitude: 149.165237,
            heading: 200.0,
            pitch: -15.0,
            roll: 0.0,
            fov: 70.0,
            aim: (0.0, 0.0),
            imagery: imagery.to_string(),
            north_speed: 0.0,
            east_speed: 0.0,
            climb_speed: 0.0,
            heading_rate: Some(0.0),
            pitch_rate: Some(0.0),
            roll_rate: 0.0,
            response: Response::default(),
            round_trip: None,
            bottles: Vec::new(),
        }
    }

    fn settle(session: &mut Session, target: &Target, start: Instant) -> Option<Frame> {
        let deadline = Instant::now() + Duration::from_secs(10);
        (0u64..)
            .take_while(|_| Instant::now() < deadline)
            .map(|round| {
                (
                    round,
                    session.frame(
                        Some(target),
                        &[],
                        640,
                        360,
                        start + Duration::from_millis(round * 50),
                    ),
                    session.in_flight(),
                )
            })
            .find(|(round, frame, in_flight)| {
                *round > 20
                    && *in_flight == 0
                    && frame.as_ref().is_some_and(|frame| !frame.ground.is_empty())
            })
            .and_then(|(_, frame, _)| frame)
    }

    fn heading(degrees: f64) -> Aim {
        Aim {
            heading: degrees,
            pitch: -15.0,
        }
    }

    #[test]
    fn ground_behind_the_camera_is_fetched_coarse_so_a_turn_has_something_to_show() {
        let pose = Pose {
            latitude: 0.0,
            longitude: 0.0,
            height: 100.0,
            heading: 0.0,
            pitch: 0.0,
            roll: 0.0,
            fov: 70.0,
        };
        let centre = |latitude: f64| crate::synthetic::geo::ecef(latitude, 0.0, 0.0);
        let boxed = |at: glam::DVec3, half: f64| {
            serde_json::json!([
                at.x, at.y, at.z, half, 0.0, 0.0, 0.0, half, 0.0, 0.0, 0.0, half
            ])
        };
        let leaf = |latitude: f64, name: &str| serde_json::json!({"boundingVolume": {"box": boxed(centre(latitude), 50.0)}, "geometricError": 1.0, "content": {"uri": format!("{name}.glb")}});
        let json = serde_json::json!({"root": {"boundingVolume": {"box": boxed(centre(0.0), 5_000.0)}, "geometricError": 1e6, "children": [leaf(0.003, "ahead"), leaf(-0.003, "behind")]}});
        let url = Url::parse("https://x/root.json").expect("url");
        let mut tiles = Tileset::new(
            tileset::root(json.to_string().as_bytes(), &url).expect("tileset"),
            &url,
        );
        let paths = |wanted: Vec<Request>| {
            wanted
                .into_iter()
                .map(|request| request.url.path().to_string())
                .collect::<Vec<_>>()
        };
        let now = Instant::now();
        assert_eq!(
            paths(
                tiles
                    .select_measured(&Camera::new(&pose, 1280, 720), 8, 1.0, now)
                    .wanted
            ),
            ["/ahead.glb"]
        );
        assert_eq!(
            paths(surround(&mut tiles, &pose, 1280, 720, 8, now)),
            ["/behind.glb"],
            "a yaw on the phone swung onto ground never fetched and showed up to 13 holes"
        );
    }

    #[test]
    fn a_turn_glides_between_heading_readings_instead_of_jumping_to_each() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let first = turned(None, heading(100.0), at(0));
        assert_eq!(first.aim_at(at(50)).heading, 100.0);
        let second = turned(Some(first), heading(108.0), at(90));
        let halfway = second.aim_at(at(90 + 58)).heading;
        assert!(
            halfway > 102.0 && halfway < 106.0,
            "half an interval later the view is part way there, not at 108 ({halfway})"
        );
        assert_eq!(second.aim_at(at(90 + 300)).heading, 108.0, "and it arrives");
        let early = turned(Some(second), heading(116.0), at(150));
        assert!(
            (early.aim_at(at(150)).heading - second.aim_at(at(150)).heading).abs() < 1e-9,
            "a reading that lands mid-glide starts from where the view is, so nothing jumps"
        );
        assert_eq!(
            turned(Some(second), heading(108.0), at(200)),
            second,
            "the same heading again changes nothing, so the glide finishes"
        );
    }

    #[test]
    fn a_tilt_glides_between_gimbal_readings_too() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let level = turned(
            None,
            Aim {
                heading: 30.0,
                pitch: -23.0,
            },
            at(0),
        );
        let tilted = turned(
            Some(level),
            Aim {
                heading: 30.0,
                pitch: -15.0,
            },
            at(330),
        );
        let midway = tilted.aim_at(at(330 + 150)).pitch;
        assert!(
            midway > -21.0 && midway < -17.0,
            "a gimbal reporting 8 degrees of tilt every third of a second is drawn part way between, not in steps ({midway})"
        );
        assert_eq!(
            tilted.aim_at(at(330 + 600)),
            Aim {
                heading: 30.0,
                pitch: -15.0
            },
            "and it arrives"
        );
    }

    #[test]
    fn the_view_draws_coarser_while_it_turns_fast_and_full_detail_once_it_stops() {
        assert_eq!(turn_detail(0.0), 1.0);
        assert_eq!(turn_detail(15.0), 2.0);
        assert_eq!(
            turn_detail(90.0),
            COARSEST_WHILE_TURNING,
            "a fast yaw does not keep coarsening without end"
        );
        let pose = |heading: f64| Pose {
            latitude: 0.0,
            longitude: 0.0,
            height: 30.0,
            heading,
            pitch: -15.0,
            roll: 0.0,
            fov: 70.0,
        };
        assert!(
            (spin_of(&pose(358.0), &pose(2.0), Duration::from_millis(100)) - 40.0).abs() < 1e-9,
            "four degrees through north in 100 ms is 40 deg/s"
        );
        let settling = (0..20).fold(45.0, |spin, _| spun(spin, Some(0.0)));
        assert!(
            turn_detail(settling) < 1.01,
            "a stopped turn is back to full detail within a second of frames ({settling} deg/s left)"
        );
    }

    #[test]
    fn a_turn_through_north_goes_the_short_way() {
        let start = Instant::now();
        let turn = turned(
            Some(turned(None, heading(355.0), start)),
            heading(5.0),
            start + Duration::from_millis(90),
        );
        let midway = turn.aim_at(start + Duration::from_millis(90 + 60)).heading;
        assert!(
            (midway - 355.0).rem_euclid(360.0) < 10.0,
            "from 355 to 5 is ten degrees through north, not 350 back round ({midway})"
        );
    }

    #[test]
    fn the_camera_flies_at_home_ground_plus_height_above_home_but_never_into_the_ground() {
        assert_eq!(camera_height(&target(""), Some(600.0), Some(590.0)), 630.0);
        assert_eq!(
            camera_height(
                &Target {
                    above_home: -2.0,
                    ..target("")
                },
                Some(600.0),
                Some(605.0)
            ),
            607.0,
            "landed lower than the 3D ground, the camera sits on it"
        );
        assert_eq!(camera_height(&target(""), None, Some(590.0)), 622.0);
    }

    #[test]
    fn a_google_key_goes_straight_to_google_and_an_ion_token_asks_ion() {
        assert_eq!(
            root_url("AIzaKey", &Fake).map(|u| u.to_string()),
            Ok("https://tile.googleapis.com/v1/3dtiles/root.json?key=AIzaKey".to_string())
        );
        assert_eq!(
            root_url("eyJtoken", &Fake).map(|u| u.to_string()),
            Ok("https://tile.googleapis.com/v1/3dtiles/root.json?key=FromIon".to_string())
        );
    }

    #[test]
    fn the_view_reads_into_a_target_only_when_complete() {
        let view = serde_json::json!({ "available": true, "latitude": -35.0, "longitude": 149.0, "aboveHome": 20.0, "homeLatitude": -35.1, "homeLongitude": 149.1, "heading": 90.0, "pitch": -15.0, "roll": 0.0, "fov": 70.0, "imagery": "Bing Satellite", "northSpeed": 3.0, "eastSpeed": -4.0, "climbSpeed": 1.0, "headingRate": 12.0, "pitchRate": null, "rollRate": 0.0 });
        assert_eq!(
            Target::from_view(&view).map(|t| (
                t.above_home,
                t.heading,
                t.north_speed,
                t.east_speed,
                t.climb_speed
            )),
            Some((20.0, 90.0, 3.0, -4.0, 1.0))
        );
        assert_eq!(
            Target::from_view(&view).map(|t| (t.heading_rate, t.pitch_rate)),
            Some((Some(12.0), None)),
            "a gimbal-aimed axis has no rate, so it glides instead"
        );
        assert_eq!(
            Target::from_view(&serde_json::json!({ "available": false })),
            None
        );
        let mut missing = view.clone();
        missing.as_object_mut().unwrap().remove("fov");
        assert_eq!(
            Target::from_view(&missing),
            None,
            "a pose without a field of view is refused, not guessed"
        );
    }

    #[test]
    fn the_reading_age_follows_telemetry_not_the_pilots_tilt() {
        let mut session = Session::new("", Arc::new(Fake)).expect("workers");
        let reading = target("Bing Satellite");
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        session
            .frame(Some(&reading), &[], 640, 360, at(0))
            .expect("a frame");
        let tilted = Target {
            pitch: -40.0,
            ..reading.clone()
        };
        assert_eq!(
            session
                .frame(Some(&tilted), &[], 640, 360, at(500))
                .expect("a frame")
                .reading_age,
            Duration::from_millis(500),
            "dragging the tilt is not new telemetry"
        );
        let moved = Target {
            north_speed: 1.5,
            ..tilted
        };
        assert_eq!(
            session
                .frame(Some(&moved), &[], 640, 360, at(516))
                .expect("a frame")
                .reading_age,
            Duration::from_millis(8),
            "a new velocity is, dated midway between the ticks that saw it"
        );
    }

    fn stick(at: Instant, roll: f64, pitch: f64, yaw: f64, thrust: f64) -> StickSample {
        StickSample {
            at,
            roll,
            pitch,
            yaw,
            thrust,
            touched: None,
        }
    }

    fn loiter() -> Response {
        let speed = Gain {
            full: 12.5,
            expo: 0.0,
            deadzone: 0.0,
        };
        Response {
            yaw: Some(Gain {
                full: 150.0,
                expo: 0.0,
                deadzone: 0.0,
            }),
            forward: Some(speed),
            right: Some(speed),
            climb: Some(Climb {
                up: 2.5,
                down: 2.5,
                deadzone: 0.0,
            }),
            ..Response::default()
        }
    }

    #[test]
    fn the_view_turns_with_the_stick_before_telemetry_does() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let hovering = Target {
            response: loiter(),
            round_trip: Some(Duration::from_millis(40)),
            ..target("Bing Satellite")
        };
        let read = Reading {
            spot: at(970),
            height: at(970),
            attitude: at(970),
        };
        let still = [
            stick(at(500), 0.0, 0.0, 0.0, 0.5),
            stick(at(960), 0.0, 0.0, 0.0, 0.5),
        ];
        let pushed = [still[0], still[1], stick(at(1_000), 0.0, 0.0, 0.5, 0.5)];
        let (resting, calm) = ahead(&hovering, &still, read, at(1_000));
        let (turning, motion) = ahead(&hovering, &pushed, read, at(1_000));
        assert_eq!((resting.heading, calm.heading), (200.0, 0.0));
        assert!(
            motion.heading > 5.0 && turning.heading > resting.heading,
            "the frame drawn as the stick moves already turns, ahead of any telemetry: {motion:?}"
        );
        let (_, stale) = ahead(&hovering, &pushed, read, at(1_400));
        assert_eq!(
            stale.heading, 0.0,
            "sticks that stopped sending 400 ms ago no longer steer the view"
        );
    }

    #[test]
    fn in_acro_a_yaw_while_banked_also_tips_the_nose_as_the_drone_does() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let gain = |full: f64| {
            Some(Gain {
                full,
                expo: 0.0,
                deadzone: 0.0,
            })
        };
        let acro = Response {
            yaw: gain(200.0),
            roll: gain(360.0),
            pitch: gain(360.0),
            rate_mode: true,
            ..Response::default()
        };
        let read = Reading {
            spot: at(970),
            height: at(970),
            attitude: at(970),
        };
        let pushed = [
            stick(at(500), 0.0, 0.0, 0.0, 0.5),
            stick(at(900), 0.0, 0.0, 1.0, 0.5),
        ];
        let level = Target {
            pitch: 0.0,
            response: acro,
            round_trip: Some(Duration::from_millis(40)),
            ..target("Bing Satellite")
        };
        let (flat, flat_motion) = ahead(&level, &pushed, read, at(1_000));
        assert!(
            flat.heading > level.heading + 5.0
                && flat.pitch.abs() < 1e-6
                && flat_motion.pitch.abs() < 1e-6,
            "level, a yaw is only a turn: {flat_motion:?}"
        );
        let banked = Target {
            roll: 45.0,
            ..level.clone()
        };
        let (guess, motion) = ahead(&banked, &pushed, read, at(1_000));
        assert!(
            motion.pitch < -0.6 * motion.heading && guess.pitch < -3.0,
            "banked 45 deg, a body yaw drops the nose about as fast as it turns; drawing it as a pure turn snapped back at every reading: {guess:?} {motion:?}"
        );
        assert!(
            guess.heading > banked.heading
                && guess.heading - banked.heading < flat.heading - level.heading,
            "and turns the heading less than when level"
        );
    }

    #[test]
    fn body_and_euler_rates_convert_back_and_forth() {
        let rates = crate::synthview::euler_rates(
            30.0,
            -20.0,
            crate::synthview::body_rates(30.0, -20.0, (10.0, -25.0, 40.0)),
        );
        assert!(
            (rates.0 - 10.0).abs() < 1e-9
                && (rates.1 + 25.0).abs() < 1e-9
                && (rates.2 - 40.0).abs() < 1e-9,
            "{rates:?}"
        );
    }

    #[test]
    fn in_loiter_the_stick_flies_the_view_along_the_nose_and_up() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let east_facing = Target {
            heading: 90.0,
            response: loiter(),
            round_trip: Some(Duration::from_millis(40)),
            ..target("Bing Satellite")
        };
        let read = Reading {
            spot: at(900),
            height: at(900),
            attitude: at(900),
        };
        let pushed = [
            stick(at(500), 0.0, 0.0, 0.0, 0.5),
            stick(at(980), 0.0, 1.0, 0.0, 1.0),
        ];
        let (guess, motion) = ahead(&east_facing, &pushed, read, at(1_000));
        assert!(
            motion.east > 0.1 && motion.north.abs() < 1e-6,
            "forward on a nose pointing east is east: {motion:?}"
        );
        assert!(
            guess.longitude > east_facing.longitude && motion.climb > 0.1,
            "{motion:?}"
        );
    }

    #[test]
    fn a_gimbal_aimed_or_unknown_vehicle_is_never_steered_by_the_sticks() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let unknown = Target {
            round_trip: Some(Duration::from_millis(40)),
            ..target("Bing Satellite")
        };
        let read = Reading {
            spot: at(970),
            height: at(970),
            attitude: at(970),
        };
        let pushed = [
            stick(at(500), 1.0, 1.0, 1.0, 1.0),
            stick(at(990), 1.0, 1.0, 1.0, 1.0),
        ];
        let (guess, motion) = ahead(&unknown, &pushed, read, at(1_000));
        assert!(
            (guess.heading - unknown.heading).abs() < 1e-9
                && (guess.latitude - unknown.latitude).abs() < 1e-9
                && (guess.above_home - unknown.above_home).abs() < 1e-9
                && motion.heading == 0.0,
            "no known response, no feed-forward: {motion:?}"
        );
        let loiter = Response {
            yaw: Some(Gain {
                full: 150.0,
                expo: 0.0,
                deadzone: 0.0,
            }),
            ..Response::default()
        };
        let held = Target {
            heading_rate: None,
            response: loiter,
            ..unknown
        };
        let (guess, motion) = ahead(&held, &pushed, read, at(1_000));
        assert!(
            (guess.heading - held.heading).abs() < 1e-9 && motion.heading == 0.0,
            "a gimbal holding its own heading does not turn with the yaw stick; predicting it fought the glide toward the gimbal's readings and shook the view: {motion:?}"
        );
    }

    #[test]
    fn a_frame_reports_how_long_after_the_touch_it_was_drawn() {
        let mut session = Session::new("", Arc::new(Fake)).expect("workers");
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let touched = [StickSample {
            touched: Some(at(990)),
            ..stick(at(995), 0.0, 0.0, 0.2, 0.5)
        }];
        let first = session
            .frame(
                Some(&target("Bing Satellite")),
                &touched,
                640,
                360,
                at(1_000),
            )
            .expect("a frame");
        let next = session
            .frame(
                Some(&target("Bing Satellite")),
                &touched,
                640,
                360,
                at(1_016),
            )
            .expect("a frame");
        assert_eq!(
            (first.stick_to_frame, next.stick_to_frame),
            (Some(Duration::from_millis(10)), None),
            "each touch is counted once, on the first frame that shows it"
        );
    }

    #[test]
    fn between_attitude_readings_the_view_turns_on_at_the_reported_rate() {
        let turning = Target {
            heading_rate: Some(50.0),
            ..target("Bing Satellite")
        };
        let start = Instant::now();
        let read = Reading {
            spot: start,
            height: start,
            attitude: start,
        };
        let at = |ms: u64| start + Duration::from_millis(ms);
        assert!(
            (ahead(&turning, &[], read, at(175)).0.heading - 210.0).abs() < 1e-9,
            "a fifth of a second at 50 deg/s is 10 degrees on, counting the 25 ms the display takes"
        );
        assert!(
            (ahead(&turning, &[], read, at(5_000)).0.heading - 230.0).abs() < 1e-9,
            "a quiet link stops the turn after 600 ms rather than spinning the view"
        );
        assert_eq!(
            ahead(&turning, &[], read, at(5_000)).1.heading,
            0.0,
            "and the drawn view stops turning with it"
        );
    }

    #[test]
    fn a_steady_turn_is_drawn_where_the_drone_points_now_not_where_it_pointed() {
        let mut session = Session::new("", Arc::new(Fake)).expect("workers");
        let start = Instant::now();
        let errors: Vec<f64> = (0u64..120)
            .map(|tick| {
                let ms = tick * 16;
                let reading = Target {
                    heading: 200.0 + 60.0 * (ms / 50 * 50) as f64 / 1000.0,
                    heading_rate: Some(60.0),
                    ..target("Bing Satellite")
                };
                let drawn = session
                    .frame(
                        Some(&reading),
                        &[],
                        640,
                        360,
                        start + Duration::from_millis(ms),
                    )
                    .expect("a frame")
                    .pose
                    .heading;
                wrapped(drawn - (200.0 + 60.0 * (ms + 25) as f64 / 1000.0)).abs()
            })
            .collect();
        let worst = errors[60..].iter().copied().fold(0.0, f64::max);
        assert!(
            worst < 0.5,
            "a 60 deg/s turn read 20 times a second and drawn at 60 fps sits within half a degree of where it points when the frame reaches the screen ({worst})"
        );
    }

    #[test]
    fn between_position_readings_the_camera_flies_on_at_the_reported_velocity() {
        let flying = Target {
            north_speed: 10.0,
            east_speed: 0.0,
            climb_speed: 2.0,
            ..target("Bing Satellite")
        };
        let start = Instant::now();
        let read = Reading {
            spot: start,
            height: start,
            attitude: start,
        };
        let half = ahead(&flying, &[], read, start + Duration::from_millis(475)).0;
        assert!(
            ((half.latitude - flying.latitude) * 111_195.0 - 5.0).abs() < 0.05,
            "half a second at 10 m/s north is 5 m north ({})",
            (half.latitude - flying.latitude) * 111_195.0
        );
        assert!(
            (half.longitude - flying.longitude).abs() < 1e-9
                && (half.above_home - 31.0).abs() < 1e-9
        );
        let lost = ahead(&flying, &[], read, start + Duration::from_secs(30)).0;
        assert!(
            ((lost.latitude - flying.latitude) * 111_195.0 - 10.0).abs() < 0.1,
            "a link that goes quiet stops the guess after a second instead of flying the camera away"
        );
    }

    #[test]
    fn a_moving_drone_keeps_the_view_moving_between_its_position_readings() {
        let mut session = Session::new("", Arc::new(Fake)).expect("workers");
        let flying = Target {
            north_speed: 10.0,
            ..target("Bing Satellite")
        };
        let start = Instant::now();
        let frames: Vec<Frame> = (0..20)
            .map(|tick| {
                session
                    .frame(
                        Some(&flying),
                        &[],
                        640,
                        360,
                        start + Duration::from_millis(tick * 16),
                    )
                    .expect("a frame")
            })
            .collect();
        assert!(
            frames
                .windows(2)
                .all(|pair| pair[1].pose.latitude > pair[0].pose.latitude && pair[1].changed),
            "one reading at 10 m/s keeps every frame moving north instead of settling on the reading"
        );
    }

    #[test]
    fn the_pilots_own_aim_is_drawn_at_once_while_telemetry_is_still_eased_in() {
        let mut session = Session::new("", Arc::new(Fake)).expect("workers");
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let still = target("Bing Satellite");
        session
            .frame(Some(&still), &[], 640, 360, at(0))
            .expect("a frame");
        let dragged = Target {
            heading: still.heading + 30.0,
            pitch: still.pitch - 10.0,
            aim: (30.0, -10.0),
            ..still.clone()
        };
        let pose = session
            .frame(Some(&dragged), &[], 640, 360, at(20))
            .expect("a frame")
            .pose;
        assert!(
            wrapped(pose.heading - dragged.heading).abs() < 1e-6
                && (pose.pitch - dragged.pitch).abs() < 1e-6,
            "a drag turns the view the whole way in the next frame ({}, {})",
            pose.heading,
            pose.pitch
        );
        let turned = Target {
            heading: dragged.heading + 30.0,
            ..dragged.clone()
        };
        let pose = session
            .frame(Some(&turned), &[], 640, 360, at(40))
            .expect("a frame")
            .pose;
        assert!(
            wrapped(turned.heading - pose.heading) > 1.0,
            "the drone's own turn is still eased in ({})",
            pose.heading
        );
    }

    #[test]
    fn smoothing_covers_the_same_ground_per_second_at_any_frame_rate() {
        let per_second = |hz: u32| {
            (0..hz).fold(0.0, |done: f64, _| {
                done + (1.0 - done) * follow(Duration::from_secs_f64(1.0 / f64::from(hz)))
            })
        };
        assert!((per_second(30) - per_second(60)).abs() < 1e-9);
    }

    #[test]
    fn a_roll_through_upside_down_goes_the_short_way() {
        let level = Pose {
            latitude: 0.0,
            longitude: 0.0,
            height: 30.0,
            heading: 0.0,
            pitch: 0.0,
            roll: 170.0,
            fov: 70.0,
        };
        let rolled = toward(
            level,
            Pose {
                roll: -170.0,
                ..level
            },
            0.5,
        )
        .roll;
        assert!(
            (wrapped(rolled) - 180.0).abs() < 1e-9 || (wrapped(rolled) + 180.0).abs() < 1e-9,
            "from 170 to -170 the view passes upside down, not back through level ({rolled})"
        );
        assert!(settled(
            &Pose {
                roll: 180.0,
                ..level
            },
            &Pose {
                roll: -180.0,
                ..level
            }
        ));
    }

    #[test]
    fn an_open_session_loads_offline_terrain_draws_it_and_then_stops_asking() {
        let mut session = Session::new("", Arc::new(Fake)).expect("workers");
        let frame =
            settle(&mut session, &target("Bing Satellite"), Instant::now()).expect("terrain drawn");
        assert!(!frame.ground.is_empty());
        let stats = session.take_stats();
        assert!(
            stats.loaded > 0 && stats.failed > 0 && stats.bytes > 0,
            "{stats:?}"
        );
        assert!(
            stats
                .errors
                .keys()
                .any(|error| error.contains("no imagery")),
            "missing imagery is reported, not hidden: {:?}",
            stats.errors
        );
    }

    #[test]
    fn a_frame_draws_the_dropped_bottles_and_knows_the_ground_under_the_drone() {
        let mut session = Session::new("", Arc::new(Fake)).expect("workers");
        let bottle = |above_home: f64, landed: bool| Bottle {
            latitude: -35.36213,
            longitude: 149.16514,
            above_home,
            heading: 0.0,
            tilt: if landed { 90.0 } else { 30.0 },
            landed,
        };
        let dropped = Target {
            bottles: vec![bottle(-3.0, true), bottle(5.0, false)],
            ..target("Bing Satellite")
        };
        let frame = settle(&mut session, &dropped, Instant::now()).expect("terrain drawn");
        let drawn = ground::height_at(&frame.ground, -35.36213, 149.16514).unwrap()
            - frame.home_ground.unwrap();
        assert!(
            (frame.bottles[0].above_home - drawn).abs() < 0.01,
            "a bottle that landed where the ground is higher than under the drone lies on the drawn ground, not under it: {} vs {drawn}",
            frame.bottles[0].above_home
        );
        assert_eq!(
            frame.bottles[1],
            bottle(5.0, false),
            "a falling bottle keeps its flight"
        );
        let (_, meshes) = frame.view(None, 640, 360);
        assert_eq!(meshes.len(), frame.ground.len() + 1);
        assert!(
            frame
                .ground_above_home()
                .is_some_and(|metres| metres.abs() < 1.0),
            "the test terrain is flat: {:?}",
            frame.ground_above_home()
        );
    }

    #[test]
    fn a_still_view_stops_asking_for_redraws_once_its_tiles_are_in() {
        let mut session = Session::new("", Arc::new(Fake)).expect("workers");
        let still = target("Bing Satellite");
        let start = Instant::now();
        settle(&mut session, &still, start).expect("terrain drawn");
        let later = start + Duration::from_secs(2);
        let drained = std::iter::repeat_with(|| {
            session.frame(Some(&still), &[], 640, 360, later);
            session.in_flight()
        })
        .take_while(|in_flight| *in_flight > 0)
        .count();
        let frame = session
            .frame(Some(&still), &[], 640, 360, later)
            .expect("a frame");
        assert!(
            !frame.changed,
            "nothing moved and nothing arrived, so the GPU can rest (drained {drained} frames)"
        );
        let moved = Target {
            heading: 210.0,
            ..still
        };
        assert!(
            session
                .frame(Some(&moved), &[], 640, 360, later)
                .expect("a frame")
                .changed,
            "a turn redraws"
        );
    }

    #[test]
    fn google_takes_over_once_its_near_tiles_are_real_geometry_not_globe_tiles() {
        assert!(
            !google_ready(true, 31_974_277.8, Some(10.0), None),
            "globe-level tiles under the camera are what drew haze and smears"
        );
        assert!(
            google_ready(true, 53.5, Some(10.0), None),
            "a 1 km tile still refining is already a city on screen"
        );
        assert!(
            !google_ready(true, f64::INFINITY, Some(10.0), None),
            "nothing drawn yet"
        );
        assert!(
            !google_ready(true, 2.0, None, None),
            "without home's ground the camera height is a guess"
        );
        assert!(!google_ready(false, 2.0, Some(10.0), None));
        assert!(
            !google_ready(true, 2.0, Some(10.0), Some(40)),
            "uploading 790 meshes in the handover frame froze the phone for 2 s"
        );
        assert!(
            google_ready(true, 2.0, Some(10.0), Some(0)),
            "once they are on the GPU the switch is one ordinary frame"
        );
    }

    #[test]
    fn with_a_token_the_open_scene_stands_in_until_google_answers() {
        let mut session = Session::new("eyJtoken", Arc::new(Fake)).expect("workers");
        let frame = settle(&mut session, &target("Bing Satellite"), Instant::now())
            .expect("open terrain drawn while Google's root is unreachable");
        assert!(!frame.ground.is_empty());
        assert_eq!(session.showing(), "open_fallback");
        assert_eq!(session.handover(), None);
        assert!(
            session
                .take_stats()
                .errors
                .keys()
                .any(|error| error.starts_with("tileset root")),
            "the Google failure is still reported"
        );
    }

    #[test]
    fn an_unanswered_google_root_is_asked_for_again_instead_of_leaving_the_stand_in_forever() {
        let mut session = Session::new("eyJtoken", Arc::new(Fake)).expect("workers");
        let (start, deadline) = (Instant::now(), Instant::now() + Duration::from_secs(10));
        let retried = (0u64..)
            .take_while(|_| Instant::now() < deadline)
            .any(|round| {
                session.frame(
                    Some(&target("Bing Satellite")),
                    &[],
                    640,
                    360,
                    start + Duration::from_secs(round * 2),
                );
                session
                    .stats
                    .errors
                    .iter()
                    .filter(|(error, _)| error.starts_with("tileset root"))
                    .map(|(_, count)| *count)
                    .sum::<u64>()
                    >= 2
            });
        assert!(
            retried,
            "one failed root request left Google off for the whole session: {:?}",
            session.stats.errors
        );
    }

    #[test]
    fn a_provider_that_crashes_its_worker_is_reported_and_frees_the_budget() {
        let mut session = Session::new("", Arc::new(Fake)).expect("workers");
        let (start, deadline) = (Instant::now(), Instant::now() + Duration::from_secs(10));
        let settled = (0u64..)
            .take_while(|_| Instant::now() < deadline)
            .any(|round| {
                session.frame(
                    Some(&target("Panics")),
                    &[],
                    640,
                    360,
                    start + Duration::from_millis(round * 50),
                );
                round > 20
                    && session.in_flight() == 0
                    && session
                        .stats
                        .errors
                        .keys()
                        .any(|error| error.contains("crashed"))
            });
        assert!(
            settled,
            "the crash is counted and every job comes back: {:?}",
            session.stats.errors
        );
    }
}
