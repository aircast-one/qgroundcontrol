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
use super::tileset::{self, GOOGLE_ROOT, ION_GOOGLE_ASSET, Parsed, Request, Tileset};

const WORKERS: usize = 8;
const MAX_IN_FLIGHT: usize = 24;
const CLEARANCE_M: f64 = 2.0;
const FOLLOW_AT_60_HZ: f64 = 0.25;
const LONGEST_STEP: Duration = Duration::from_millis(100);
const PROBE_EVERY: Duration = Duration::from_millis(500);
const RESELECT_EVERY: Duration = Duration::from_secs(1);
const PROBE_ABOVE_M: f64 = 400.0;
const FIRST_PROBE_HEIGHT_M: f64 = 9_000.0;
const PROBE_FOV: f64 = 20.0;
const PROBE_PX: u32 = 256;
const UNKNOWN_GROUND_M: f64 = 1_000.0;
const SETTLED_DEGREES: f64 = 5e-7;
const SETTLED_METRES: f64 = 0.05;
const SETTLED_ANGLE: f64 = 0.05;

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
    ground_meshes: Vec<Arc<Mesh>>,
}

fn wrapped(degrees: f64) -> f64 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

fn toward(from: Pose, to: Pose, alpha: f64) -> Pose {
    let step = |a: f64, b: f64| a + (b - a) * alpha;
    Pose {
        latitude: step(from.latitude, to.latitude),
        longitude: step(from.longitude, to.longitude),
        height: step(from.height, to.height),
        heading: from.heading + wrapped(to.heading - from.heading) * alpha,
        pitch: step(from.pitch, to.pitch),
        roll: step(from.roll, to.roll),
        fov: to.fov,
    }
}

pub fn follow(elapsed: Duration) -> f64 {
    1.0 - (1.0 - FOLLOW_AT_60_HZ).powf(elapsed.min(LONGEST_STEP).as_secs_f64() * 60.0)
}

fn settled(a: &Pose, b: &Pose) -> bool {
    (a.latitude - b.latitude).abs() < SETTLED_DEGREES
        && (a.longitude - b.longitude).abs() < SETTLED_DEGREES
        && (a.height - b.height).abs() < SETTLED_METRES
        && [wrapped(a.heading - b.heading), a.pitch - b.pitch, a.roll - b.roll].iter().all(|turn| turn.abs() < SETTLED_ANGLE)
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
        let counter = bytes.clone();
        let mut pool = Pool::new(WORKERS, "synthetic-fetch", move |job: Job| work(job, &Counted { inner: fetcher.as_ref(), bytes: &counter }), lost)?;
        let google = !token.is_empty();
        if google {
            pool.send(Job::Root(token.to_string()));
        }
        let source = if google { Source::Google(None) } else { Source::Open(OpenScene::new()) };
        Ok(Self { token: token.to_string(), source, pool, bytes, stats: Stats::default(), home: Probe::default(), under: Probe::default(), probed: None, shown: None, ticked: None, selected: None, seen: None, ground_meshes: Vec::new() })
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

    pub fn in_flight(&self) -> usize {
        self.pool.in_flight()
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
        let finished = self.pool.finished();
        let count = finished.len();
        finished.into_iter().for_each(|done| match done {
            Done::Root(Ok((url, root))) => {
                self.stats.loaded += 1;
                if let Source::Google(tiles) = &mut self.source {
                    *tiles = Some(Tileset::new(root, &url));
                }
            }
            Done::Root(Err(error)) => self.failed(format!("tileset root: {error}")),
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
                if let Source::Open(scene) = &mut self.source {
                    scene.deliver(built, now);
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
        let height = known.map_or(FIRST_PROBE_HEIGHT_M, |ground| ground + PROBE_ABOVE_M);
        let camera = Camera::new(&Pose { latitude, longitude, height, heading: 0.0, pitch: -90.0, roll: 0.0, fov: PROBE_FOV }, PROBE_PX, PROBE_PX);
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

    fn probe(&mut self, target: &Target, now: Instant) {
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
        let room = self.room();
        let ground = self.under.height.or(self.home.height).unwrap_or(0.0);
        let (meshes, jobs) = match &mut self.source {
            Source::Google(Some(tiles)) => {
                let (meshes, wanted) = tiles.select(camera, room, now);
                (meshes, wanted.into_iter().map(Job::Tile).collect())
            }
            Source::Open(scene) => {
                let (meshes, wanted) = scene.select(camera, &target.imagery, ground, target.latitude, target.longitude, room, now);
                (meshes, wanted.into_iter().map(Job::Open).collect())
            }
            Source::Google(None) => (Vec::new(), Vec::new()),
        };
        let different = meshes.len() != self.ground_meshes.len() || meshes.iter().zip(&self.ground_meshes).any(|(a, b)| a.id != b.id);
        self.ground_meshes = meshes;
        self.selected = Some(now);
        self.send(jobs);
        different
    }

    pub fn frame(&mut self, target: Option<&Target>, width: u32, height: u32, now: Instant) -> Option<Frame> {
        let delivered = self.collect(now);
        let target = target?;
        if let Source::Google(None) = self.source {
            return None;
        }
        self.probe(target, now);
        let goal = Pose { latitude: target.latitude, longitude: target.longitude, height: camera_height(target, self.home.height, self.under.height), heading: target.heading, pitch: target.pitch, roll: target.roll, fov: target.fov };
        let alpha = self.ticked.map_or(1.0, |at| follow(now.duration_since(at)));
        self.ticked = Some(now);
        let pose = self.shown.map_or(goal, |shown| toward(shown, goal, alpha));
        let moving = self.shown.is_none_or(|before| !settled(&before, &pose));
        self.shown = Some(pose);
        let fresh = self.seen.as_ref() != Some(target);
        self.seen = Some(target.clone());
        let due = self.selected.is_none_or(|at| now.duration_since(at) >= RESELECT_EVERY);
        let reselected = (delivered > 0 || moving || fresh || due) && self.select(&Camera::new(&pose, width, height), target, now);
        Some(Frame { pose, ground: self.ground_meshes.clone(), home_ground: self.home.height, changed: moving || fresh || reselected })
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
