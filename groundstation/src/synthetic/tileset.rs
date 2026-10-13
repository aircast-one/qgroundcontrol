use std::sync::Arc;
use std::time::{Duration, Instant};

use glam::{DMat4, DVec3, DVec4};
use serde_json::Value;
use url::Url;

use super::geo::Camera;
use super::glb;
use super::mesh::Mesh;

pub const GOOGLE_ROOT: &str = "https://tile.googleapis.com/v1/3dtiles/root.json";
pub const ION_GOOGLE_ASSET: u64 = 2_275_207;
pub const RETRY_AFTER: Duration = Duration::from_secs(10);
pub const UNLOAD_AFTER: Duration = Duration::from_secs(30);
const UNLOAD_SWEEP_EVERY: Duration = Duration::from_secs(1);
const MAXIMUM_SCREEN_SPACE_ERROR: f64 = 16.0;
const DETAIL_PIXELS: f64 = 1280.0 * 720.0;
const MAX_LOADED_TILES: usize = 500;
const LOWEST_GROUND_M: f64 = 1_000.0;
const OCCLUDER_RADII: DVec3 = DVec3::new(6_378_137.0 - LOWEST_GROUND_M, 6_378_137.0 - LOWEST_GROUND_M, 6_356_752.314_245 - LOWEST_GROUND_M);

#[derive(Clone, Copy, Debug)]
pub struct Obb {
    pub center: DVec3,
    pub half: [DVec3; 3],
}

impl Obb {
    pub fn from_box(values: &[f64]) -> Option<Self> {
        let [cx, cy, cz, ax, ay, az, bx, by, bz, ux, uy, uz] = <[f64; 12]>::try_from(values).ok()?;
        Some(Self { center: DVec3::new(cx, cy, cz), half: [DVec3::new(ax, ay, az), DVec3::new(bx, by, bz), DVec3::new(ux, uy, uz)] })
    }

    pub fn distance(&self, point: DVec3) -> f64 {
        let offset = point - self.center;
        self.half
            .iter()
            .map(|axis| {
                let length = axis.length();
                let along = if length > 0.0 { offset.dot(*axis) / length } else { 0.0 };
                (along.abs() - length).max(0.0).powi(2)
            })
            .sum::<f64>()
            .sqrt()
    }

    fn corners(&self) -> [DVec3; 8] {
        std::array::from_fn(|corner| {
            let sign = |bit: usize| if corner & bit == 0 { -1.0 } else { 1.0 };
            self.center + self.half[0] * sign(1) + self.half[1] * sign(2) + self.half[2] * sign(4)
        })
    }

    pub fn outside(&self, plane: DVec4, eye: DVec3) -> bool {
        let normal = plane.truncate();
        let reach: f64 = self.half.iter().map(|axis| normal.dot(*axis).abs()).sum();
        normal.dot(self.center - eye) + plane.w < -reach
    }
}

struct Horizon {
    eye: DVec3,
    beyond: f64,
}

impl Horizon {
    fn from(eye: DVec3) -> Self {
        let scaled = eye / OCCLUDER_RADII;
        Self { eye: scaled, beyond: scaled.length_squared() - 1.0 }
    }

    fn hides_point(&self, point: DVec3) -> bool {
        let towards = point / OCCLUDER_RADII - self.eye;
        let along = -towards.dot(self.eye);
        self.beyond > 0.0 && along > self.beyond && along * along / towards.length_squared() > self.beyond
    }

    fn hides(&self, bounds: &Obb) -> bool {
        bounds.corners().iter().all(|corner| self.hides_point(*corner))
    }
}

pub fn frustum(culling: DMat4) -> [DVec4; 5] {
    let rows = [culling.row(0), culling.row(1), culling.row(2), culling.row(3)];
    [rows[3] + rows[0], rows[3] - rows[0], rows[3] + rows[1], rows[3] - rows[1], rows[2]].map(|plane| plane / plane.truncate().length())
}

pub enum Parsed {
    External(Box<Tile>),
    Meshes(Vec<Mesh>),
}

pub fn parse(url: &Url, bytes: &[u8]) -> Result<Parsed, String> {
    match url.path().ends_with(".json") {
        true => root(bytes, url).map(Box::new).map(Parsed::External).ok_or_else(|| "unreadable tileset json".to_string()),
        false => Ok(Parsed::Meshes(glb::meshes(bytes))),
    }
}

enum Content {
    None,
    Pending,
    Requested,
    External,
    Meshes(Vec<Arc<Mesh>>),
    Failed(Instant),
}

pub struct Tile {
    bounds: Obb,
    geometric_error: f64,
    url: Option<Url>,
    content: Content,
    children: Vec<Tile>,
    used: Option<Instant>,
}

pub struct Request {
    pub path: Vec<usize>,
    pub url: Url,
}

pub struct Tileset {
    root: Tile,
    session: Option<(String, String)>,
    keep: Vec<(String, String)>,
    swept: Option<Instant>,
}

fn query(url: &Url) -> Vec<(String, String)> {
    url.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect()
}

fn tile(value: &Value, base: &Url) -> Option<Tile> {
    let bounds = Obb::from_box(&value.get("boundingVolume")?.get("box")?.as_array()?.iter().filter_map(Value::as_f64).collect::<Vec<_>>())?;
    let url = value.get("content").and_then(|content| content.get("uri").or_else(|| content.get("url"))).and_then(Value::as_str).and_then(|uri| base.join(uri).ok());
    let children = value.get("children").and_then(Value::as_array).map(|children| children.iter().filter_map(|child| tile(child, base)).collect()).unwrap_or_default();
    let content = if url.is_some() { Content::Pending } else { Content::None };
    Some(Tile { bounds, geometric_error: value.get("geometricError").and_then(Value::as_f64).unwrap_or(0.0), url, content, children, used: None })
}

pub fn root(json: &[u8], url: &Url) -> Option<Tile> {
    tile(serde_json::from_slice::<Value>(json).ok()?.get("root")?, url)
}

fn empty() -> Tile {
    Tile { bounds: Obb { center: DVec3::ZERO, half: [DVec3::ZERO; 3] }, geometric_error: 0.0, url: None, content: Content::None, children: Vec::new(), used: None }
}

fn unload(tile: &mut Tile, now: Instant) {
    let stale = tile.used.is_none_or(|used| now.duration_since(used) > UNLOAD_AFTER);
    if stale && tile.url.is_some() && matches!(tile.content, Content::Meshes(_) | Content::External) {
        if matches!(tile.content, Content::External) {
            tile.children.clear();
        }
        tile.content = Content::Pending;
    }
    tile.children.iter_mut().for_each(|child| unload(child, now));
}

pub(super) fn cutoff(mut used: Vec<Instant>, most: usize) -> Option<Instant> {
    (used.len() > most && most > 0).then(|| *used.select_nth_unstable_by(most - 1, |a, b| b.cmp(a)).1)
}

fn used_meshes(tile: &Tile) -> Vec<Instant> {
    tile.used.filter(|_| matches!(tile.content, Content::Meshes(_))).into_iter().chain(tile.children.iter().flat_map(used_meshes)).collect()
}

pub(super) fn kept(used: Instant, cutoff: Instant, now: Instant) -> bool {
    used > cutoff || used == now
}

fn evict(tile: &mut Tile, cutoff: Instant, seen: &[Instant]) {
    if matches!(tile.content, Content::Meshes(_)) && tile.url.is_some() && !tile.used.is_some_and(|used| seen.iter().any(|at| kept(used, cutoff, *at))) {
        tile.content = Content::Pending;
    }
    tile.children.iter_mut().for_each(|child| evict(child, cutoff, seen));
}

fn refine_above(camera: &Camera) -> f64 {
    MAXIMUM_SCREEN_SPACE_ERROR * (camera.height_px * camera.height_px * camera.aspect / DETAIL_PIXELS).sqrt().max(1.0)
}

fn loaded(tile: &Tile) -> usize {
    usize::from(matches!(tile.content, Content::Meshes(_))) + tile.children.iter().map(loaded).sum::<usize>()
}

impl Tileset {
    pub fn new(root: Tile, url: &Url) -> Self {
        Self { root, session: None, keep: query(url), swept: None }
    }

    fn signed(&mut self, url: &Url) -> Url {
        let have = query(url);
        let session = have.iter().find(|(k, _)| k == "session").cloned();
        self.session = session.or_else(|| self.session.clone());
        let missing: Vec<(String, String)> = self.keep.iter().chain(self.session.iter()).filter(|(k, _)| !have.iter().any(|(h, _)| h == k)).cloned().collect();
        let mut signed = url.clone();
        signed.query_pairs_mut().extend_pairs(missing);
        signed
    }

    pub fn loaded_tiles(&self) -> usize {
        loaded(&self.root)
    }

    pub fn select(&mut self, camera: &Camera, budget: usize, now: Instant) -> (Vec<Arc<Mesh>>, Vec<Request>) {
        let planes = frustum(camera.culling());
        let horizon = Horizon::from(camera.eye);
        let mut wanted = Vec::new();
        let mut root = std::mem::replace(&mut self.root, empty());
        let drawn = self.visit(&mut root, &mut Vec::new(), camera, (&planes, &horizon), budget, &mut wanted, now).unwrap_or_default();
        if self.swept.is_none_or(|swept| now.duration_since(swept) >= UNLOAD_SWEEP_EVERY) {
            unload(&mut root, now);
            self.swept = Some(now);
        }
        self.root = root;
        (drawn, wanted)
    }

    pub fn trim(&mut self, now: Instant, probed: Option<Instant>) {
        if let Some(oldest) = cutoff(used_meshes(&self.root), MAX_LOADED_TILES) {
            evict(&mut self.root, oldest, &[Some(now), probed].into_iter().flatten().collect::<Vec<_>>());
        }
    }

    pub fn deliver(&mut self, path: &[usize], parsed: Result<Parsed, String>, now: Instant) {
        let Some(tile) = path.iter().try_fold(&mut self.root, |tile, &child| tile.children.get_mut(child)) else { return };
        if !matches!(tile.content, Content::Requested) {
            return;
        }
        tile.content = match parsed {
            Err(_) => Content::Failed(now),
            Ok(Parsed::External(child)) => {
                tile.children.push(*child);
                Content::External
            }
            Ok(Parsed::Meshes(meshes)) => Content::Meshes(meshes.into_iter().map(Arc::new).collect()),
        };
    }

    fn request(&mut self, tile: &mut Tile, path: &[usize], budget: usize, wanted: &mut Vec<Request>, now: Instant) {
        if let Content::Failed(at) = tile.content
            && now.duration_since(at) >= RETRY_AFTER
        {
            tile.content = Content::Pending;
        }
        let (Content::Pending, Some(url)) = (&tile.content, tile.url.clone()) else { return };
        if wanted.len() >= budget {
            return;
        }
        wanted.push(Request { path: path.to_vec(), url: self.signed(&url) });
        tile.content = Content::Requested;
    }

    #[allow(clippy::too_many_arguments)]
    fn visit(&mut self, tile: &mut Tile, path: &mut Vec<usize>, camera: &Camera, sight: (&[DVec4; 5], &Horizon), budget: usize, wanted: &mut Vec<Request>, now: Instant) -> Option<Vec<Arc<Mesh>>> {
        let (planes, horizon) = sight;
        if planes.iter().any(|plane| tile.bounds.outside(*plane, camera.eye)) || horizon.hides(&tile.bounds) {
            return Some(Vec::new());
        }
        tile.used = Some(now);
        self.request(tile, path, budget, wanted, now);
        let refine = camera.screen_space_error(tile.geometric_error, tile.bounds.distance(camera.eye)) > refine_above(camera);
        let external = matches!(tile.content, Content::External);
        let children = (refine || external) && !tile.children.is_empty();
        let below: Option<Vec<Arc<Mesh>>> = match children {
            true => tile
                .children
                .iter_mut()
                .enumerate()
                .map(|(index, child)| {
                    path.push(index);
                    let drawn = self.visit(child, path, camera, sight, budget, wanted, now);
                    path.pop();
                    drawn
                })
                .collect::<Vec<Option<Vec<Arc<Mesh>>>>>()
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .map(|lists| lists.into_iter().flatten().collect()),
            false => None,
        };
        let own = match &tile.content {
            Content::Meshes(meshes) => Some(meshes.clone()),
            Content::None if !children => Some(Vec::new()),
            _ => None,
        };
        below.or(own)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::geo::Pose;
    use crate::synthetic::mesh::Layer;

    fn camera() -> Camera {
        Camera::new(&Pose { latitude: 0.0, longitude: 0.0, height: 100.0, heading: 0.0, pitch: 0.0, roll: 0.0, fov: 60.0 }, 1280, 720)
    }

    fn mesh() -> Mesh {
        Mesh::new(DMat4::IDENTITY, Vec::new(), Vec::new(), Vec::new(), image::RgbaImage::new(1, 1), Layer::Opaque)
    }

    fn ahead(camera: &Camera, metres: f64) -> Obb {
        Obb { center: camera.eye + camera.forward * metres, half: [DVec3::X * 50.0, DVec3::Y * 50.0, DVec3::Z * 50.0] }
    }

    fn leaf(camera: &Camera, name: &str) -> Tile {
        Tile { bounds: ahead(camera, 200.0), geometric_error: 1.0, url: Some(Url::parse(&format!("https://x/{name}.glb")).unwrap()), content: Content::Pending, children: Vec::new(), used: None }
    }

    fn parent(camera: &Camera) -> (Tileset, Arc<Mesh>) {
        let shown = Arc::new(mesh());
        let root = Tile { bounds: ahead(camera, 200.0), geometric_error: 1000.0, url: None, content: Content::Meshes(vec![shown.clone()]), children: vec![leaf(camera, "a"), leaf(camera, "b")], used: None };
        (Tileset::new(root, &Url::parse("https://x/root.json").unwrap()), shown)
    }

    #[test]
    fn past_the_tile_budget_the_least_recently_seen_go_but_what_is_drawn_stays() {
        let camera = camera();
        let now = Instant::now();
        let seen = |name: &str, ago: u64| Tile { content: Content::Meshes(vec![Arc::new(mesh())]), used: Some(now - Duration::from_secs(ago)), ..leaf(&camera, name) };
        let mut root = Tile { content: Content::None, url: None, children: vec![seen("a", 5), seen("b", 0), seen("c", 9)], ..leaf(&camera, "root") };
        let oldest = cutoff(used_meshes(&root), 1).expect("three tiles is over a budget of one");
        evict(&mut root, oldest, &[now]);
        assert_eq!(root.children.iter().map(|tile| matches!(tile.content, Content::Meshes(_))).collect::<Vec<_>>(), [false, true, false], "the tile drawn this frame stays");
        let mut probed = Tile { content: Content::None, url: None, children: vec![seen("view", 0), seen("probe", 1), seen("old", 9)], ..leaf(&camera, "root") };
        let oldest = cutoff(used_meshes(&probed), 1).expect("over budget");
        evict(&mut probed, oldest, &[now, now - Duration::from_secs(1)]);
        assert_eq!(probed.children.iter().map(|tile| matches!(tile.content, Content::Meshes(_))).collect::<Vec<_>>(), [true, true, false], "the ground probe's tiles from its last look stay as well");
    }

    #[test]
    fn a_phone_screen_asks_for_the_tiles_a_720p_one_would() {
        let sized = |width: u32, height: u32| refine_above(&Camera::new(&Pose { latitude: 0.0, longitude: 0.0, height: 100.0, heading: 0.0, pitch: 0.0, roll: 0.0, fov: 60.0 }, width, height));
        assert!((sized(1280, 720) - MAXIMUM_SCREEN_SPACE_ERROR).abs() < 1e-9);
        assert_eq!(sized(640, 360), MAXIMUM_SCREEN_SPACE_ERROR, "a small view keeps full detail");
        assert!((sized(2560, 1440) - MAXIMUM_SCREEN_SPACE_ERROR * 2.0).abs() < 1e-9, "four times the pixels of 720p; at full detail a 2280x1080 phone needed 2250 tiles and 5 GB");
    }

    #[test]
    fn a_tile_behind_the_curve_of_the_earth_is_neither_drawn_nor_fetched() {
        let camera = camera();
        let horizon = Horizon::from(camera.eye);
        let small = [DVec3::X * 50.0, DVec3::Y * 50.0, DVec3::Z * 50.0];
        assert!(!horizon.hides(&ahead(&camera, 200.0)));
        assert!(!horizon.hides(&Obb { center: crate::synthetic::geo::ecef(0.2, 0.0, 0.0), half: small }), "22 km away is still above the horizon from 100 m");
        assert!(horizon.hides(&Obb { center: crate::synthetic::geo::ecef(9.0, 0.0, 0.0), half: small }), "1000 km away is below it");
        assert!(horizon.hides(&Obb { center: -camera.eye, half: small }), "the far side of the planet was being drawn through the ground");
        assert!(!horizon.hides(&Obb { center: DVec3::ZERO, half: [DVec3::X * 7e6, DVec3::Y * 7e6, DVec3::Z * 7e6] }), "a tile around the whole globe holds what is in view");
    }

    #[test]
    fn a_box_measures_zero_from_inside_and_the_gap_from_outside() {
        let unit = Obb { center: DVec3::ZERO, half: [DVec3::X, DVec3::Y, DVec3::Z] };
        assert_eq!(unit.distance(DVec3::new(0.5, 0.0, 0.0)), 0.0);
        assert!((unit.distance(DVec3::new(3.0, 0.0, 0.0)) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn a_box_behind_the_camera_is_culled_and_one_ahead_is_kept() {
        let camera = camera();
        let planes = frustum(camera.culling());
        let front = ahead(&camera, 500.0);
        let behind = Obb { center: camera.eye - camera.forward * 500.0, ..front };
        assert!(!planes.iter().any(|plane| front.outside(*plane, camera.eye)));
        assert!(planes.iter().any(|plane| behind.outside(*plane, camera.eye)));
    }

    #[test]
    fn every_request_carries_the_key_and_the_session_the_root_handed_out() {
        let mut tiles = Tileset::new(empty(), &Url::parse("https://tile.googleapis.com/v1/3dtiles/root.json?key=K").unwrap());
        let first = tiles.signed(&Url::parse("https://tile.googleapis.com/v1/3dtiles/datasets/a/files/b.json?session=S").unwrap());
        let later = tiles.signed(&Url::parse("https://tile.googleapis.com/v1/3dtiles/datasets/a/files/c.glb").unwrap());
        assert_eq!(first.query(), Some("session=S&key=K"));
        assert_eq!(later.query(), Some("key=K&session=S"));
    }

    #[test]
    fn a_parent_stays_drawn_until_every_visible_child_has_loaded_and_a_failure_is_retried_later() {
        let camera = camera();
        let (mut tiles, shown) = parent(&camera);
        let start = Instant::now();
        let (drawn, wanted) = tiles.select(&camera, 8, start);
        assert!(drawn.len() == 1 && Arc::ptr_eq(&drawn[0], &shown), "with nothing loaded below, the coarse parent fills the view");
        assert_eq!(wanted.iter().map(|request| request.path.clone()).collect::<Vec<_>>(), vec![vec![0], vec![1]], "both children are asked for");
        tiles.deliver(&[0], Err("offline".into()), start);
        tiles.deliver(&[1], Ok(Parsed::Meshes(vec![mesh()])), start);
        let (drawn, wanted) = tiles.select(&camera, 8, start + Duration::from_secs(1));
        assert!(drawn.len() == 1 && Arc::ptr_eq(&drawn[0], &shown) && wanted.is_empty(), "a failed child keeps the parent up and is not asked again at once");
        let (_, wanted) = tiles.select(&camera, 8, start + RETRY_AFTER);
        assert_eq!(wanted.iter().map(|request| request.path.clone()).collect::<Vec<_>>(), vec![vec![0]], "after the backoff the failed child is asked again");
        tiles.deliver(&[0], Ok(Parsed::Meshes(vec![mesh()])), start + RETRY_AFTER);
        let (drawn, _) = tiles.select(&camera, 8, start + RETRY_AFTER);
        assert_eq!(drawn.len(), 2, "both children replace the parent once loaded");
    }

    #[test]
    fn content_nobody_has_looked_at_for_a_while_is_unloaded_and_asked_for_again() {
        let camera = camera();
        let (mut tiles, _) = parent(&camera);
        let start = Instant::now();
        tiles.select(&camera, 8, start);
        tiles.deliver(&[0], Ok(Parsed::Meshes(vec![mesh()])), start);
        tiles.deliver(&[1], Ok(Parsed::Meshes(vec![mesh()])), start);
        assert_eq!(tiles.loaded_tiles(), 3);
        let away = Camera::new(&Pose { latitude: 50.0, longitude: 50.0, height: 100.0, heading: 0.0, pitch: 0.0, roll: 0.0, fov: 60.0 }, 1280, 720);
        tiles.select(&away, 8, start + UNLOAD_AFTER + Duration::from_secs(1));
        assert_eq!(tiles.loaded_tiles(), 1, "fetched tiles out of view for longer than the unload delay give their meshes back; the built-in root keeps its own");
        let (_, wanted) = tiles.select(&camera, 8, start + UNLOAD_AFTER + Duration::from_secs(2));
        assert_eq!(wanted.len(), 2, "coming back asks for them again");
    }
}
