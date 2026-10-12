use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use url::Url;

use super::geo::{Camera, Pose};
use super::glb::Mesh;
use super::ground;
use super::open::{OpenScene, TileKey, Want};
use super::tileset::{self, Request, Tileset, GOOGLE_ROOT, ION_GOOGLE_ASSET};

const WORKERS: usize = 8;
const MAX_IN_FLIGHT: usize = 24;
const CLEARANCE_M: f64 = 2.0;
const FOLLOW: f64 = 0.25;
const PROBE_EVERY: Duration = Duration::from_millis(500);
const PROBE_ABOVE_M: f64 = 400.0;
const FIRST_PROBE_HEIGHT_M: f64 = 9_000.0;
const PROBE_FOV: f64 = 20.0;
const PROBE_PX: u32 = 256;
const UNKNOWN_GROUND_M: f64 = 1_000.0;

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
            heading: number("heading").unwrap_or(0.0),
            pitch: number("pitch").unwrap_or(-15.0),
            roll: number("roll").unwrap_or(0.0),
            fov: number("fov").unwrap_or(60.0),
            imagery: view.get("imagery").and_then(serde_json::Value::as_str).unwrap_or("Bing Satellite").to_string(),
        })
    }
}

enum Job {
    Root(String),
    Tile(Request),
    Map(Want),
}

enum Done {
    Root(Option<(Url, Vec<u8>)>),
    Tile(Vec<usize>, Option<Vec<u8>>),
    Map(String, TileKey, Option<Vec<u8>>),
}

fn fetch(url: &Url) -> Option<Vec<u8>> {
    ureq::get(url.as_str()).call().ok()?.body_mut().with_config().limit(64 * 1024 * 1024).read_to_vec().ok()
}

pub fn root_url(token: &str, fetch: impl Fn(&Url) -> Option<Vec<u8>>) -> Option<Url> {
    match token.starts_with("eyJ") {
        true => {
            let endpoint = Url::parse_with_params(&format!("https://api.cesium.com/v1/assets/{ION_GOOGLE_ASSET}/endpoint"), [("access_token", token)]).ok()?;
            let reply: serde_json::Value = serde_json::from_slice(&fetch(&endpoint)?).ok()?;
            Url::parse(reply.get("options")?.get("url")?.as_str()?).ok()
        }
        false => Url::parse_with_params(GOOGLE_ROOT, [("key", token)]).ok(),
    }
}

fn map_tile(want: &Want) -> Option<Vec<u8>> {
    let cache = crate::terrainservice::cache_path().and_then(|path| crate::tilecache::Cache::open(&path).ok());
    let persist = !crate::settingsstore::raw_setting("settings.appSettings.disableAllPersistence").and_then(|v| v.as_bool()).unwrap_or(false);
    let (x, y, z) = (i32::try_from(want.key.x).ok()?, i32::try_from(want.key.y).ok()?, i32::try_from(want.key.z).ok()?);
    crate::maptiles::fetch_remembered(&want.provider, x, y, z, cache.as_ref(), persist)
}

fn work(job: Job) -> Done {
    match job {
        Job::Root(token) => Done::Root(root_url(&token, fetch).and_then(|url| Some((url.clone(), fetch(&url)?)))),
        Job::Tile(request) => Done::Tile(request.path, fetch(&request.url)),
        Job::Map(want) => {
            let bytes = map_tile(&want);
            Done::Map(want.provider, want.key, bytes)
        }
    }
}

fn workers() -> (Sender<Job>, Receiver<Done>) {
    let (jobs, queue) = mpsc::channel::<Job>();
    let (done, results) = mpsc::channel::<Done>();
    let queue = Arc::new(Mutex::new(queue));
    (0..WORKERS).for_each(|index| {
        let (queue, done) = (queue.clone(), done.clone());
        let _ = std::thread::Builder::new().name(format!("photoreal-fetch-{index}")).spawn(move || {
            while let Ok(job) = queue.lock().map_err(|_| ()).and_then(|queue| queue.recv().map_err(|_| ())) {
                if done.send(work(job)).is_err() {
                    break;
                }
            }
        });
    });
    (jobs, results)
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

pub struct Session {
    token: String,
    source: Source,
    jobs: Sender<Job>,
    results: Receiver<Done>,
    in_flight: usize,
    home: Probe,
    under: Probe,
    probed: Option<Instant>,
    shown: Option<Pose>,
}

fn wrapped(degrees: f64) -> f64 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

fn toward(from: Pose, to: Pose) -> Pose {
    let step = |a: f64, b: f64| a + (b - a) * FOLLOW;
    Pose {
        latitude: step(from.latitude, to.latitude),
        longitude: step(from.longitude, to.longitude),
        height: step(from.height, to.height),
        heading: from.heading + wrapped(to.heading - from.heading) * FOLLOW,
        pitch: step(from.pitch, to.pitch),
        roll: step(from.roll, to.roll),
        fov: to.fov,
    }
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
    pub fn new(token: &str) -> Self {
        let (jobs, results) = workers();
        let google = !token.is_empty();
        if google {
            let _ = jobs.send(Job::Root(token.to_string()));
        }
        let source = if google { Source::Google(None) } else { Source::Open(OpenScene::new()) };
        Self { token: token.to_string(), source, jobs, results, in_flight: usize::from(google), home: Probe::default(), under: Probe::default(), probed: None, shown: None }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    fn collect(&mut self) {
        let finished: Vec<Done> = self.results.try_iter().collect();
        self.in_flight = self.in_flight.saturating_sub(finished.len());
        finished.into_iter().for_each(|done| match (done, &mut self.source) {
            (Done::Root(Some((url, json))), Source::Google(tiles)) => *tiles = tileset::root(&json, &url).map(|root| Tileset::new(root, &url)),
            (Done::Tile(path, bytes), Source::Google(Some(tiles))) => tiles.deliver(&path, bytes),
            (Done::Map(provider, key, bytes), Source::Open(scene)) => scene.deliver(&provider, key, bytes),
            _ => {}
        });
    }

    fn send(&mut self, jobs: Vec<Job>) {
        self.in_flight += jobs.len();
        jobs.into_iter().for_each(|job| {
            let _ = self.jobs.send(job);
        });
    }

    fn ask(&mut self, wanted: Vec<Request>) {
        self.send(wanted.into_iter().map(Job::Tile).collect());
    }

    fn budget(&self) -> usize {
        MAX_IN_FLIGHT.saturating_sub(self.in_flight)
    }

    fn probe(&mut self, latitude: f64, longitude: f64, known: Option<f64>) -> Option<f64> {
        let height = known.map_or(FIRST_PROBE_HEIGHT_M, |ground| ground + PROBE_ABOVE_M);
        let camera = Camera::new(&Pose { latitude, longitude, height, heading: 0.0, pitch: -90.0, roll: 0.0, fov: PROBE_FOV }, PROBE_PX, PROBE_PX);
        let budget = self.budget();
        let Source::Google(Some(tiles)) = &mut self.source else { return known };
        let (meshes, wanted) = tiles.select(&camera, budget);
        self.ask(wanted);
        ground::height_at(&meshes, latitude, longitude).or(known)
    }

    fn open_ground(&mut self, latitude: f64, longitude: f64, known: Option<f64>) -> Option<f64> {
        let budget = self.budget();
        let Source::Open(scene) = &mut self.source else { return known };
        let mut wanted = Vec::new();
        let found = scene.ground(latitude, longitude, &mut wanted, budget);
        self.send(wanted.into_iter().map(Job::Map).collect());
        found.or(known)
    }

    pub fn frame(&mut self, target: Option<&Target>, width: u32, height: u32) -> Option<(Camera, Vec<Arc<Mesh>>)> {
        self.collect();
        let target = target?;
        let open = matches!(self.source, Source::Open(_));
        if let Source::Google(None) = self.source {
            return None;
        }
        self.home.for_spot(target.home_latitude, target.home_longitude);
        self.under.for_spot(target.latitude, target.longitude);
        let due = open || self.probed.is_none_or(|at| at.elapsed() >= PROBE_EVERY);
        if due {
            self.probed = Some(Instant::now());
            self.home.height = match open {
                true => self.open_ground(target.home_latitude, target.home_longitude, self.home.height),
                false => self.probe(target.home_latitude, target.home_longitude, self.home.height),
            };
            self.under.height = match open {
                true => self.open_ground(target.latitude, target.longitude, self.under.height),
                false => self.probe(target.latitude, target.longitude, self.under.height.or(self.home.height)),
            };
        }
        let goal = Pose { latitude: target.latitude, longitude: target.longitude, height: camera_height(target, self.home.height, self.under.height), heading: target.heading, pitch: target.pitch, roll: target.roll, fov: target.fov };
        let shown = self.shown.map_or(goal, |shown| toward(shown, goal));
        self.shown = Some(shown);
        let camera = Camera::new(&shown, width, height);
        let budget = self.budget();
        let ground = self.under.height.or(self.home.height).unwrap_or(0.0);
        let meshes = match &mut self.source {
            Source::Google(Some(tiles)) => {
                let (meshes, wanted) = tiles.select(&camera, budget);
                self.ask(wanted);
                meshes
            }
            Source::Open(scene) => {
                let (meshes, wanted) = scene.select(&camera, &target.imagery, ground, target.latitude, target.longitude, budget);
                self.send(wanted.into_iter().map(Job::Map).collect());
                meshes
            }
            Source::Google(None) => Vec::new(),
        };
        Some((camera, meshes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(above_home: f64) -> Target {
        Target { latitude: 0.0, longitude: 0.0, above_home, home_latitude: 0.0, home_longitude: 0.0, heading: 0.0, pitch: -15.0, roll: 0.0, fov: 60.0, imagery: String::new() }
    }

    #[test]
    fn the_camera_flies_at_home_ground_plus_height_above_home_but_never_into_the_ground() {
        assert_eq!(camera_height(&target(30.0), Some(600.0), Some(590.0)), 630.0);
        assert_eq!(camera_height(&target(-2.0), Some(600.0), Some(605.0)), 607.0, "landed lower than the 3D ground, the camera sits on it");
        assert_eq!(camera_height(&target(30.0), None, Some(590.0)), 622.0);
    }

    #[test]
    fn a_google_key_goes_straight_to_google_and_an_ion_token_asks_ion() {
        let google = root_url("AIzaKey", |_| None).expect("a google url");
        assert_eq!(google.as_str(), "https://tile.googleapis.com/v1/3dtiles/root.json?key=AIzaKey");
        let asked = std::cell::RefCell::new(None);
        let ion = root_url("eyJtoken", |url| {
            *asked.borrow_mut() = Some(url.clone());
            Some(br#"{"options":{"url":"https://tile.googleapis.com/v1/3dtiles/root.json?key=FromIon"}}"#.to_vec())
        });
        assert_eq!(asked.into_inner().map(|url| url.host_str().map(str::to_string)), Some(Some("api.cesium.com".to_string())));
        assert_eq!(ion.map(|url| url.to_string()), Some("https://tile.googleapis.com/v1/3dtiles/root.json?key=FromIon".to_string()));
    }

    #[test]
    fn the_view_reads_into_a_target_only_when_the_aircraft_is_placed() {
        let view = serde_json::json!({ "available": true, "latitude": -35.0, "longitude": 149.0, "aboveHome": 20.0, "homeLatitude": -35.1, "homeLongitude": 149.1, "heading": 90.0, "pitch": -15.0, "fov": 60.0 });
        assert_eq!(Target::from_view(&view).map(|t| (t.above_home, t.heading)), Some((20.0, 90.0)));
        assert_eq!(Target::from_view(&serde_json::json!({ "available": false })), None);
    }
}
