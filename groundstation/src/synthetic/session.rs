use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use url::Url;

use super::fetch::{Fetcher, Pool};
use super::geo::{Camera, Pose};
use super::ground;
use super::mesh::Mesh;
use super::open::{Built, OpenScene, Want};
use super::overlay::{self, Overlays};
use super::tileset::{self, GOOGLE_ROOT, ION_GOOGLE_ASSET, Parsed, Request, Selection, Tileset};

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
const SPIN_SMOOTHING: f64 = 0.3;
const GOOGLE_READY_COARSEST: f64 = 1_000.0;
const STAND_IN_WORKERS: usize = 3;
const STAND_IN_IN_FLIGHT: usize = 6;

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
    pub imagery: String,
}

impl Target {
    pub fn from_view(view: &serde_json::Value) -> Option<Self> {
        let number = |key: &str| view.get(key).and_then(serde_json::Value::as_f64).filter(|v| v.is_finite());
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
            imagery: view.get("imagery")?.as_str()?.to_string(),
        })
    }
}

pub fn root_url(token: &str, fetcher: &dyn Fetcher) -> Result<Url, String> {
    match token.starts_with("eyJ") {
        true => {
            let endpoint = Url::parse_with_params(&format!("https://api.cesium.com/v1/assets/{ION_GOOGLE_ASSET}/endpoint"), [("access_token", token)]).map_err(|error| error.to_string())?;
            let reply: serde_json::Value = serde_json::from_slice(&fetcher.url(&endpoint)?).map_err(|error| format!("Cesium ion answered unreadable JSON: {error}"))?;
            let url = reply.get("options").and_then(|options| options.get("url")).and_then(serde_json::Value::as_str).ok_or("Cesium ion gave no tileset url; check the token can read asset 2275207")?;
            Url::parse(url).map_err(|error| error.to_string())
        }
        false => Url::parse_with_params(GOOGLE_ROOT, [("key", token)]).map_err(|error| error.to_string()),
    }
}

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

struct Counted<'a> {
    inner: &'a dyn Fetcher,
    bytes: &'a AtomicU64,
}

impl Fetcher for Counted<'_> {
    fn url(&self, url: &Url) -> Result<Vec<u8>, String> {
        self.inner.url(url).inspect(|bytes| {
            self.bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);
        })
    }

    fn map(&self, provider: &str, z: u32, x: u32, y: u32) -> Result<Vec<u8>, String> {
        self.inner.map(provider, z, x, y).inspect(|bytes| {
            self.bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);
        })
    }
}

fn work(job: Job, fetcher: &dyn Fetcher) -> Done {
    match job {
        Job::Root(token) => Done::Root(root_url(&token, fetcher).and_then(|url| {
            let json = fetcher.url(&url)?;
            tileset::root(&json, &url).map(|root| (url, root)).ok_or_else(|| "the root tileset was unreadable".to_string())
        })),
        Job::Tile(request) => Done::Tile(request.path, fetcher.url(&request.url).and_then(|bytes| tileset::parse(&request.url, &bytes))),
        Job::Open(want) => {
            let what = want.describe();
            let built = want.build(fetcher);
            let error = match &built {
                Built::Heights(_, Err(error)) | Built::Houses(_, Err(error)) | Built::Terrain { mesh: Err(error), .. } => Some(format!("{what}: {error}")),
                Built::Terrain { mesh: Ok((_, false)), .. } => Some(format!("{what}: no imagery")),
                _ => None,
            };
            Done::Open(built, error)
        }
    }
}

fn lost(job: &Job) -> Done {
    match job {
        Job::Root(_) => Done::Root(Err("the tileset worker crashed".to_string())),
        Job::Tile(request) => Done::Tile(request.path.clone(), Err("the tile worker crashed".to_string())),
        Job::Open(want) => Done::Open(want.failed("the tile worker crashed"), Some(format!("{}: the tile worker crashed", want.describe()))),
    }
}

#[derive(Default)]
struct Probe {
    at: Option<(f64, f64)>,
    height: Option<f64>,
}

impl Probe {
    fn for_spot(&mut self, latitude: f64, longitude: f64) {
        self.height = self.height.filter(|_| self.at == Some((latitude, longitude)));
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
}

impl Fallback {
    fn room(&self) -> usize {
        self.pool.room(STAND_IN_IN_FLIGHT)
    }
}

fn workers(count: usize, name: &str, fetcher: Arc<dyn Fetcher>, bytes: Arc<AtomicU64>) -> Result<Pool<Job, Done>, String> {
    Pool::new(count, name, move |job: Job| work(job, &Counted { inner: fetcher.as_ref(), bytes: &bytes }), lost)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stats {
    pub requested: u64,
    pub loaded: u64,
    pub failed: u64,
    pub bytes: u64,
    pub errors: BTreeMap<String, u64>,
}

pub struct Frame {
    pub pose: Pose,
    pub ground: Vec<Arc<Mesh>>,
    pub warm: Vec<Arc<Mesh>>,
    pub home_ground: Option<f64>,
    pub changed: bool,
}

impl Frame {
    pub fn view(&self, overlays: Option<&Overlays>, width: u32, height: u32) -> (Camera, Vec<Arc<Mesh>>) {
        let camera = Camera::new(&self.pose, width, height);
        let marks = overlays.zip(self.home_ground).map(|(overlays, ground)| overlay::meshes(overlays, ground, &camera)).unwrap_or_default();
        (camera, self.ground.iter().cloned().chain(marks).collect())
    }
}

pub struct Session {
    token: String,
    source: Source,
    pool: Pool<Job, Done>,
    bytes: Arc<AtomicU64>,
    stats: Stats,
    home: Probe,
    under: Probe,
    probed: Option<Instant>,
    shown: Option<Pose>,
    ticked: Option<Instant>,
    selected: Option<Instant>,
    seen: Option<Target>,
    turn: Option<Turn>,
    spin: f64,
    ground_meshes: Vec<Arc<Mesh>>,
    fallback: Option<Fallback>,
    coarsest: f64,
    first_frame: Option<Instant>,
    handed_over_after: Option<Duration>,
    warming: Option<usize>,
    root_due: Option<Instant>,
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
        wrapped(other.heading - self.heading).abs().max((other.pitch - self.pitch).abs())
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
        let progress = (now.saturating_duration_since(self.at).as_secs_f64() / (self.every.as_secs_f64() * TURN_STRETCH)).min(1.0);
        Aim { heading: self.from.heading + wrapped(self.to.heading - self.from.heading) * progress, pitch: self.from.pitch + (self.to.pitch - self.from.pitch) * progress }
    }
}

pub fn turned(turn: Option<Turn>, aim: Aim, now: Instant) -> Turn {
    match turn {
        None => Turn { from: aim, to: aim, at: now, every: TURN_EVERY_GUESS },
        Some(turn) if aim.apart(&turn.to) < SETTLED_ANGLE => turn,
        Some(turn) => {
            let gap = now.saturating_duration_since(turn.at).clamp(TURN_SHORTEST, TURN_LONGEST);
            Turn { from: turn.aim_at(now), to: aim, at: now, every: (turn.every + gap) / 2 }
        }
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
    wrapped(after.heading - before.heading).hypot(after.pitch - before.pitch) / elapsed.as_secs_f64().max(1e-3)
}

fn spun(previous: f64, latest: Option<f64>) -> f64 {
    latest.map_or(previous, |rate| previous + (rate - previous) * SPIN_SMOOTHING)
}

pub fn follow(elapsed: Duration) -> f64 {
    1.0 - (1.0 - FOLLOW_AT_60_HZ).powf(elapsed.min(LONGEST_STEP).as_secs_f64() * 60.0)
}

fn settled(a: &Pose, b: &Pose) -> bool {
    (a.latitude - b.latitude).abs() < SETTLED_DEGREES
        && (a.longitude - b.longitude).abs() < SETTLED_DEGREES
        && (a.height - b.height).abs() < SETTLED_METRES
        && [wrapped(a.heading - b.heading), a.pitch - b.pitch, wrapped(a.roll - b.roll)].iter().all(|turn| turn.abs() < SETTLED_ANGLE)
}

pub fn google_ready(loaded: bool, coarsest: f64, home_ground: Option<f64>, still_to_upload: Option<usize>) -> bool {
    loaded && coarsest <= GOOGLE_READY_COARSEST && home_ground.is_some() && still_to_upload.is_none_or(|left| left == 0)
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
        let bytes = Arc::new(AtomicU64::new(0));
        let mut pool = workers(WORKERS, "synthetic-fetch", fetcher.clone(), bytes.clone())?;
        let google = !token.is_empty();
        if google {
            pool.send(Job::Root(token.to_string()));
        }
        let source = if google { Source::Google(None) } else { Source::Open(OpenScene::new()) };
        let fallback = match google {
            true => Some(Fallback { scene: OpenScene::new(), pool: workers(STAND_IN_WORKERS, "synthetic-stand-in", fetcher, bytes.clone())?, home: Probe::default(), under: Probe::default() }),
            false => None,
        };
        Ok(Self {
            token: token.to_string(),
            source,
            pool,
            bytes,
            stats: Stats::default(),
            home: Probe::default(),
            under: Probe::default(),
            probed: None,
            shown: None,
            ticked: None,
            selected: None,
            seen: None,
            turn: None,
            spin: 0.0,
            ground_meshes: Vec::new(),
            fallback,
            coarsest: f64::INFINITY,
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
        match (&self.source, self.fallback.is_some()) {
            (Source::Google(_), true) => "open_fallback",
            (Source::Google(_), false) => "google",
            (Source::Open(_), _) => "open",
        }
    }

    pub fn coarsest(&self) -> f64 {
        self.coarsest
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
        self.pool.in_flight() + self.fallback.as_ref().map_or(0, |fallback| fallback.pool.in_flight())
    }

    pub fn loaded_tiles(&self) -> usize {
        match &self.source {
            Source::Google(tiles) => tiles.as_ref().map_or(0, Tileset::loaded_tiles),
            Source::Open(scene) => scene.loaded_tiles(),
        }
    }

    pub fn take_stats(&mut self) -> Stats {
        Stats { bytes: self.bytes.swap(0, Ordering::Relaxed), ..std::mem::take(&mut self.stats) }
    }

    fn failed(&mut self, error: String) {
        self.stats.failed += 1;
        *self.stats.errors.entry(error).or_insert(0) += 1;
    }

    fn collect(&mut self, now: Instant) -> usize {
        let stood_in = self.fallback.as_mut().map(|fallback| fallback.pool.finished()).unwrap_or_default();
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
                    tiles.deliver(&path, parsed, now);
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

    fn google_ground(&mut self, latitude: f64, longitude: f64, known: Option<f64>, now: Instant) -> Option<f64> {
        let camera = Camera::new(&Pose { latitude, longitude, height: PROBE_HEIGHT_M, heading: 0.0, pitch: -90.0, roll: 0.0, fov: PROBE_FOV }, PROBE_PX, PROBE_PX);
        let room = self.room();
        let Source::Google(Some(tiles)) = &mut self.source else { return known };
        let (meshes, wanted) = tiles.select(&camera, room, now);
        self.send(wanted.into_iter().map(Job::Tile).collect());
        ground::height_at(&meshes, latitude, longitude).or(known)
    }

    fn open_ground(&mut self, latitude: f64, longitude: f64, known: Option<f64>, now: Instant) -> Option<f64> {
        let room = self.room();
        let Source::Open(scene) = &mut self.source else { return known };
        let (found, wanted) = scene.ground(latitude, longitude, room, now);
        self.send(wanted.into_iter().map(Job::Open).collect());
        found.or(known)
    }

    fn stand_in(&mut self, jobs: Vec<Want>) {
        self.stats.requested += jobs.len() as u64;
        if let Some(fallback) = self.fallback.as_mut() {
            jobs.into_iter().for_each(|want| fallback.pool.send(Job::Open(want)));
        }
    }

    fn probe_fallback(&mut self, target: &Target, now: Instant) {
        let Some(fallback) = self.fallback.as_mut() else { return };
        let room = fallback.room();
        fallback.home.for_spot(target.home_latitude, target.home_longitude);
        fallback.under.for_spot(target.latitude, target.longitude);
        let (home, home_wanted) = fallback.scene.ground(target.home_latitude, target.home_longitude, room, now);
        let (under, under_wanted) = fallback.scene.ground(target.latitude, target.longitude, room.saturating_sub(home_wanted.len()), now);
        fallback.home.height = home.or(fallback.home.height);
        fallback.under.height = under.or(fallback.under.height);
        self.stand_in(home_wanted.into_iter().chain(under_wanted).collect());
    }

    fn probe(&mut self, target: &Target, now: Instant) {
        self.probe_fallback(target, now);
        let open = matches!(self.source, Source::Open(_));
        self.home.for_spot(target.home_latitude, target.home_longitude);
        self.under.for_spot(target.latitude, target.longitude);
        if !(open || self.probed.is_none_or(|at| now.duration_since(at) >= PROBE_EVERY)) {
            return;
        }
        self.probed = Some(now);
        let (home_known, under_known) = (self.home.height, self.under.height);
        self.home.height = match open {
            true => self.open_ground(target.home_latitude, target.home_longitude, home_known, now),
            false => self.google_ground(target.home_latitude, target.home_longitude, home_known, now),
        };
        self.under.height = match open {
            true => self.open_ground(target.latitude, target.longitude, under_known, now),
            false => self.google_ground(target.latitude, target.longitude, under_known.or(self.home.height), now),
        };
    }

    fn select(&mut self, camera: &Camera, target: &Target, now: Instant) -> bool {
        let room = self.room().saturating_sub(PROBE_SLOTS);
        let detail = turn_detail(self.spin);
        let (stand_in, stand_in_jobs) = match self.fallback.as_mut() {
            Some(fallback) => {
                let ground = fallback.under.height.or(fallback.home.height).unwrap_or(0.0);
                let room = fallback.room();
                let (meshes, wanted) = fallback.scene.select(camera, &target.imagery, ground, target.latitude, target.longitude, detail, room, now);
                (Some(meshes), wanted)
            }
            None => (None, Vec::new()),
        };
        let ground = self.under.height.or(self.home.height).unwrap_or(0.0);
        let (meshes, jobs, coarsest) = match &mut self.source {
            Source::Google(Some(tiles)) => {
                let Selection { meshes, wanted, coarsest } = tiles.select_measured(camera, room, now);
                tiles.trim(now, self.probed);
                (meshes, wanted.into_iter().map(Job::Tile).collect(), coarsest)
            }
            Source::Open(scene) => {
                let (meshes, wanted) = scene.select(camera, &target.imagery, ground, target.latitude, target.longitude, detail, room, now);
                (meshes, wanted.into_iter().map(Job::Open).collect(), 0.0)
            }
            Source::Google(None) => (Vec::new(), Vec::new(), f64::INFINITY),
        };
        self.coarsest = coarsest;
        let drawn = stand_in.unwrap_or(meshes);
        let different = drawn.len() != self.ground_meshes.len() || drawn.iter().zip(&self.ground_meshes).any(|(a, b)| a.id != b.id);
        self.ground_meshes = drawn;
        self.selected = Some(now);
        self.stand_in(stand_in_jobs);
        self.send(jobs);
        different
    }



    pub fn frame(&mut self, target: Option<&Target>, width: u32, height: u32, now: Instant) -> Option<Frame> {
        let delivered = self.collect(now);
        if matches!(self.source, Source::Google(None)) && self.root_due.is_some_and(|due| now >= due) {
            self.root_due = None;
            self.pool.send(Job::Root(self.token.clone()));
        }
        let target = target?;
        let started = *self.first_frame.get_or_insert(now);
        if self.fallback.is_some() && google_ready(matches!(self.source, Source::Google(Some(_))), self.coarsest, self.home.height, self.warming) {
            self.fallback = None;
            self.shown = None;
            self.handed_over_after = Some(now.saturating_duration_since(started));
        }
        if matches!(self.source, Source::Google(None)) && self.fallback.is_none() {
            return None;
        }
        self.probe(target, now);
        let turn = turned(self.turn, Aim { heading: target.heading, pitch: target.pitch }, now);
        self.turn = Some(turn);
        let aim = turn.aim_at(now);
        let (home_ground, under_ground) = self.fallback.as_ref().map_or((self.home.height, self.under.height), |fallback| (fallback.home.height, fallback.under.height));
        let goal = Pose { latitude: target.latitude, longitude: target.longitude, height: camera_height(target, home_ground, under_ground), heading: aim.heading, pitch: aim.pitch, roll: target.roll, fov: target.fov };
        let elapsed = self.ticked.map(|at| now.duration_since(at));
        let alpha = elapsed.map_or(1.0, follow);
        self.ticked = Some(now);
        let pose = self.shown.map_or(goal, |shown| toward(shown, goal, alpha));
        let moving = self.shown.is_none_or(|before| !settled(&before, &pose));
        self.spin = spun(self.spin, self.shown.zip(elapsed).map(|(before, elapsed)| spin_of(&before, &pose, elapsed)));
        self.shown = Some(pose);
        let fresh = self.seen.as_ref() != Some(target);
        self.seen = Some(target.clone());
        let due = self.selected.is_none_or(|at| now.duration_since(at) >= RESELECT_EVERY);
        let reselected = (delivered > 0 || moving || fresh || due) && self.select(&Camera::new(&pose, width, height), target, now);
        let warm = match &mut self.source {
            Source::Google(Some(tiles)) => tiles.take_fresh(),
            _ => Vec::new(),
        };
        Some(Frame { pose, ground: self.ground_meshes.clone(), warm, home_ground, changed: moving || fresh || reselected })
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
                    image::RgbImage::from_pixel(4, 4, image::Rgb([130, 92, 0])).write_to(&mut png, image::ImageFormat::Png).map_err(|e| e.to_string())?;
                    Ok(png.into_inner())
                }
                "Panics" => panic!("a broken provider"),
                _ => Err(format!("no {provider} {z}/{x}/{y}")),
            }
        }
    }

    fn target(imagery: &str) -> Target {
        Target { latitude: -35.36213, longitude: 149.16514, above_home: 30.0, home_latitude: -35.363262, home_longitude: 149.165237, heading: 200.0, pitch: -15.0, roll: 0.0, fov: 70.0, imagery: imagery.to_string() }
    }

    fn settle(session: &mut Session, target: &Target, start: Instant) -> Option<Frame> {
        let deadline = Instant::now() + Duration::from_secs(10);
        (0u64..)
            .take_while(|_| Instant::now() < deadline)
            .map(|round| (round, session.frame(Some(target), 640, 360, start + Duration::from_millis(round * 50)), session.in_flight()))
            .find(|(round, frame, in_flight)| *round > 20 && *in_flight == 0 && frame.as_ref().is_some_and(|frame| !frame.ground.is_empty()))
            .and_then(|(_, frame, _)| frame)
    }

    fn heading(degrees: f64) -> Aim {
        Aim { heading: degrees, pitch: -15.0 }
    }

    #[test]
    fn a_turn_glides_between_heading_readings_instead_of_jumping_to_each() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let first = turned(None, heading(100.0), at(0));
        assert_eq!(first.aim_at(at(50)).heading, 100.0);
        let second = turned(Some(first), heading(108.0), at(90));
        let halfway = second.aim_at(at(90 + 58)).heading;
        assert!(halfway > 102.0 && halfway < 106.0, "half an interval later the view is part way there, not at 108 ({halfway})");
        assert_eq!(second.aim_at(at(90 + 300)).heading, 108.0, "and it arrives");
        let early = turned(Some(second), heading(116.0), at(150));
        assert!((early.aim_at(at(150)).heading - second.aim_at(at(150)).heading).abs() < 1e-9, "a reading that lands mid-glide starts from where the view is, so nothing jumps");
        assert_eq!(turned(Some(second), heading(108.0), at(200)), second, "the same heading again changes nothing, so the glide finishes");
    }

    #[test]
    fn a_tilt_glides_between_gimbal_readings_too() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let level = turned(None, Aim { heading: 30.0, pitch: -23.0 }, at(0));
        let tilted = turned(Some(level), Aim { heading: 30.0, pitch: -15.0 }, at(330));
        let midway = tilted.aim_at(at(330 + 150)).pitch;
        assert!(midway > -21.0 && midway < -17.0, "a gimbal reporting 8 degrees of tilt every third of a second is drawn part way between, not in steps ({midway})");
        assert_eq!(tilted.aim_at(at(330 + 600)), Aim { heading: 30.0, pitch: -15.0 }, "and it arrives");
    }

    #[test]
    fn the_view_draws_coarser_while_it_turns_fast_and_full_detail_once_it_stops() {
        assert_eq!(turn_detail(0.0), 1.0);
        assert_eq!(turn_detail(15.0), 2.0);
        assert_eq!(turn_detail(90.0), COARSEST_WHILE_TURNING, "a fast yaw does not keep coarsening without end");
        let pose = |heading: f64| Pose { latitude: 0.0, longitude: 0.0, height: 30.0, heading, pitch: -15.0, roll: 0.0, fov: 70.0 };
        assert!((spin_of(&pose(358.0), &pose(2.0), Duration::from_millis(100)) - 40.0).abs() < 1e-9, "four degrees through north in 100 ms is 40 deg/s");
        let settling = (0..20).fold(45.0, |spin, _| spun(spin, Some(0.0)));
        assert!(turn_detail(settling) < 1.01, "a stopped turn is back to full detail within a second of frames ({settling} deg/s left)");
    }

    #[test]
    fn a_turn_through_north_goes_the_short_way() {
        let start = Instant::now();
        let turn = turned(Some(turned(None, heading(355.0), start)), heading(5.0), start + Duration::from_millis(90));
        let midway = turn.aim_at(start + Duration::from_millis(90 + 60)).heading;
        assert!((midway - 355.0).rem_euclid(360.0) < 10.0, "from 355 to 5 is ten degrees through north, not 350 back round ({midway})");
    }

    #[test]
    fn the_camera_flies_at_home_ground_plus_height_above_home_but_never_into_the_ground() {
        assert_eq!(camera_height(&target(""), Some(600.0), Some(590.0)), 630.0);
        assert_eq!(camera_height(&Target { above_home: -2.0, ..target("") }, Some(600.0), Some(605.0)), 607.0, "landed lower than the 3D ground, the camera sits on it");
        assert_eq!(camera_height(&target(""), None, Some(590.0)), 622.0);
    }

    #[test]
    fn a_google_key_goes_straight_to_google_and_an_ion_token_asks_ion() {
        assert_eq!(root_url("AIzaKey", &Fake).map(|u| u.to_string()), Ok("https://tile.googleapis.com/v1/3dtiles/root.json?key=AIzaKey".to_string()));
        assert_eq!(root_url("eyJtoken", &Fake).map(|u| u.to_string()), Ok("https://tile.googleapis.com/v1/3dtiles/root.json?key=FromIon".to_string()));
    }

    #[test]
    fn the_view_reads_into_a_target_only_when_complete() {
        let view = serde_json::json!({ "available": true, "latitude": -35.0, "longitude": 149.0, "aboveHome": 20.0, "homeLatitude": -35.1, "homeLongitude": 149.1, "heading": 90.0, "pitch": -15.0, "roll": 0.0, "fov": 70.0, "imagery": "Bing Satellite" });
        assert_eq!(Target::from_view(&view).map(|t| (t.above_home, t.heading)), Some((20.0, 90.0)));
        assert_eq!(Target::from_view(&serde_json::json!({ "available": false })), None);
        let mut missing = view.clone();
        missing.as_object_mut().unwrap().remove("fov");
        assert_eq!(Target::from_view(&missing), None, "a pose without a field of view is refused, not guessed");
    }

    #[test]
    fn smoothing_covers_the_same_ground_per_second_at_any_frame_rate() {
        let per_second = |hz: u32| (0..hz).fold(0.0, |done: f64, _| done + (1.0 - done) * follow(Duration::from_secs_f64(1.0 / f64::from(hz))));
        assert!((per_second(30) - per_second(60)).abs() < 1e-9);
    }

    #[test]
    fn a_roll_through_upside_down_goes_the_short_way() {
        let level = Pose { latitude: 0.0, longitude: 0.0, height: 30.0, heading: 0.0, pitch: 0.0, roll: 170.0, fov: 70.0 };
        let rolled = toward(level, Pose { roll: -170.0, ..level }, 0.5).roll;
        assert!((wrapped(rolled) - 180.0).abs() < 1e-9 || (wrapped(rolled) + 180.0).abs() < 1e-9, "from 170 to -170 the view passes upside down, not back through level ({rolled})");
        assert!(settled(&Pose { roll: 180.0, ..level }, &Pose { roll: -180.0, ..level }));
    }

    #[test]
    fn an_open_session_loads_offline_terrain_draws_it_and_then_stops_asking() {
        let mut session = Session::new("", Arc::new(Fake)).expect("workers");
        let frame = settle(&mut session, &target("Bing Satellite"), Instant::now()).expect("terrain drawn");
        assert!(!frame.ground.is_empty());
        let stats = session.take_stats();
        assert!(stats.loaded > 0 && stats.failed > 0 && stats.bytes > 0, "{stats:?}");
        assert!(stats.errors.keys().any(|error| error.contains("no imagery")), "missing imagery is reported, not hidden: {:?}", stats.errors);
    }

    #[test]
    fn a_still_view_stops_asking_for_redraws_once_its_tiles_are_in() {
        let mut session = Session::new("", Arc::new(Fake)).expect("workers");
        let still = target("Bing Satellite");
        let start = Instant::now();
        settle(&mut session, &still, start).expect("terrain drawn");
        let later = start + Duration::from_secs(2);
        let drained = std::iter::repeat_with(|| {
            session.frame(Some(&still), 640, 360, later);
            session.in_flight()
        })
        .take_while(|in_flight| *in_flight > 0)
        .count();
        let frame = session.frame(Some(&still), 640, 360, later).expect("a frame");
        assert!(!frame.changed, "nothing moved and nothing arrived, so the GPU can rest (drained {drained} frames)");
        let moved = Target { heading: 210.0, ..still };
        assert!(session.frame(Some(&moved), 640, 360, later).expect("a frame").changed, "a turn redraws");
    }

    #[test]
    fn google_takes_over_once_its_near_tiles_are_real_geometry_not_globe_tiles() {
        assert!(!google_ready(true, 31_974_277.8, Some(10.0), None), "globe-level tiles under the camera are what drew haze and smears");
        assert!(google_ready(true, 53.5, Some(10.0), None), "a 1 km tile still refining is already a city on screen");
        assert!(!google_ready(true, f64::INFINITY, Some(10.0), None), "nothing drawn yet");
        assert!(!google_ready(true, 2.0, None, None), "without home's ground the camera height is a guess");
        assert!(!google_ready(false, 2.0, Some(10.0), None));
        assert!(!google_ready(true, 2.0, Some(10.0), Some(40)), "uploading 790 meshes in the handover frame froze the phone for 2 s");
        assert!(google_ready(true, 2.0, Some(10.0), Some(0)), "once they are on the GPU the switch is one ordinary frame");
    }

    #[test]
    fn with_a_token_the_open_scene_stands_in_until_google_answers() {
        let mut session = Session::new("eyJtoken", Arc::new(Fake)).expect("workers");
        let frame = settle(&mut session, &target("Bing Satellite"), Instant::now()).expect("open terrain drawn while Google's root is unreachable");
        assert!(!frame.ground.is_empty());
        assert_eq!(session.showing(), "open_fallback");
        assert_eq!(session.handover(), None);
        assert!(session.take_stats().errors.keys().any(|error| error.starts_with("tileset root")), "the Google failure is still reported");
    }

    #[test]
    fn an_unanswered_google_root_is_asked_for_again_instead_of_leaving_the_stand_in_forever() {
        let mut session = Session::new("eyJtoken", Arc::new(Fake)).expect("workers");
        let (start, deadline) = (Instant::now(), Instant::now() + Duration::from_secs(10));
        let retried = (0u64..).take_while(|_| Instant::now() < deadline).any(|round| {
            session.frame(Some(&target("Bing Satellite")), 640, 360, start + Duration::from_secs(round * 2));
            session.stats.errors.iter().filter(|(error, _)| error.starts_with("tileset root")).map(|(_, count)| *count).sum::<u64>() >= 2
        });
        assert!(retried, "one failed root request left Google off for the whole session: {:?}", session.stats.errors);
    }

    #[test]
    fn a_provider_that_crashes_its_worker_is_reported_and_frees_the_budget() {
        let mut session = Session::new("", Arc::new(Fake)).expect("workers");
        let (start, deadline) = (Instant::now(), Instant::now() + Duration::from_secs(10));
        let settled = (0u64..).take_while(|_| Instant::now() < deadline).any(|round| {
            session.frame(Some(&target("Panics")), 640, 360, start + Duration::from_millis(round * 50));
            round > 20 && session.in_flight() == 0 && session.stats.errors.keys().any(|error| error.contains("crashed"))
        });
        assert!(settled, "the crash is counted and every job comes back: {:?}", session.stats.errors);
    }
}
