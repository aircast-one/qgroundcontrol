use std::collections::BTreeSet;
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
const TRIM_EVERY: Duration = Duration::from_secs(1);
const MAXIMUM_SCREEN_SPACE_ERROR: f64 = 16.0;
const DETAIL_PIXELS: f64 = 1280.0 * 720.0;
const MAX_LOADED_TILES: usize = 500;
const NEAR_M: f64 = 1_500.0;
const FALLBACK_TOO_COARSE: f64 = 8.0;
const BACKFILL_TOO_COARSE: f64 = 64.0;
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
    wanted: Option<Instant>,
}

struct Candidate {
    distance: f64,
    path: Vec<usize>,
}

pub struct Request {
    pub path: Vec<usize>,
    pub url: Url,
}

pub struct Tileset {
    root: Tile,
    session: Option<(String, String)>,
    keep: Vec<(String, String)>,
    held: BTreeSet<Vec<usize>>,
    swept: Option<Instant>,
    trimmed: Option<Instant>,
    coarsest: f64,
    holes: usize,
    refine_at: f64,
    fresh: Vec<Arc<Mesh>>,
    backfill: Vec<(usize, Vec<Arc<Mesh>>)>,
}

pub struct Selection {
    pub meshes: Vec<Arc<Mesh>>,
    pub wanted: Vec<Request>,
    pub coarsest: f64,
    pub holes: usize,
    pub backfill: Vec<Arc<Mesh>>,
}

fn query(url: &Url) -> Vec<(String, String)> {
    url.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect()
}

fn tile(value: &Value, base: &Url) -> Option<Tile> {
    let bounds = Obb::from_box(&value.get("boundingVolume")?.get("box")?.as_array()?.iter().filter_map(Value::as_f64).collect::<Vec<_>>())?;
    let url = value.get("content").and_then(|content| content.get("uri").or_else(|| content.get("url"))).and_then(Value::as_str).and_then(|uri| base.join(uri).ok());
    let children = value.get("children").and_then(Value::as_array).map(|children| children.iter().filter_map(|child| tile(child, base)).collect()).unwrap_or_default();
    let content = if url.is_some() { Content::Pending } else { Content::None };
    Some(Tile { bounds, geometric_error: value.get("geometricError").and_then(Value::as_f64).unwrap_or(0.0), url, content, children, used: None, wanted: None })
}

pub fn root(json: &[u8], url: &Url) -> Option<Tile> {
    tile(serde_json::from_slice::<Value>(json).ok()?.get("root")?, url)
}

fn empty() -> Tile {
    Tile { bounds: Obb { center: DVec3::ZERO, half: [DVec3::ZERO; 3] }, geometric_error: 0.0, url: None, content: Content::None, children: Vec::new(), used: None, wanted: None }
}

fn holds(tile: &Tile) -> bool {
    tile.url.is_some() && matches!(tile.content, Content::Meshes(_) | Content::External)
}

fn held_in(tile: &Tile, path: &mut Vec<usize>, found: &mut BTreeSet<Vec<usize>>) {
    if holds(tile) {
        found.insert(path.clone());
    }
    tile.children.iter().enumerate().for_each(|(index, child)| {
        path.push(index);
        held_in(child, path, found);
        path.pop();
    });
}

fn at<'a>(root: &'a Tile, path: &[usize]) -> Option<&'a Tile> {
    path.iter().try_fold(root, |tile, &child| tile.children.get(child))
}

fn at_mut<'a>(root: &'a mut Tile, path: &[usize]) -> Option<&'a mut Tile> {
    path.iter().try_fold(root, |tile, &child| tile.children.get_mut(child))
}

fn release(tile: &mut Tile) {
    if matches!(tile.content, Content::External) {
        tile.children.clear();
    }
    tile.content = Content::Pending;
}

pub(super) fn cutoff(mut used: Vec<Instant>, most: usize) -> Option<Instant> {
    (used.len() > most && most > 0).then(|| *used.select_nth_unstable_by(most - 1, |a, b| b.cmp(a)).1)
}

pub(super) fn kept(used: Instant, cutoff: Instant, now: Instant) -> bool {
    used > cutoff || used == now
}

fn refine_above(camera: &Camera) -> f64 {
    MAXIMUM_SCREEN_SPACE_ERROR * (camera.height_px * camera.height_px * camera.aspect / DETAIL_PIXELS).sqrt().max(1.0)
}

fn loaded(tile: &Tile) -> usize {
    usize::from(matches!(tile.content, Content::Meshes(_))) + tile.children.iter().map(loaded).sum::<usize>()
}

impl Tileset {
    pub fn new(root: Tile, url: &Url) -> Self {
        let mut held = BTreeSet::new();
        held_in(&root, &mut Vec::new(), &mut held);
        Self { root, session: None, keep: query(url), held, swept: None, trimmed: None, coarsest: f64::INFINITY, holes: 0, refine_at: MAXIMUM_SCREEN_SPACE_ERROR, fresh: Vec::new(), backfill: Vec::new() }
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

    pub fn take_fresh(&mut self) -> Vec<Arc<Mesh>> {
        std::mem::take(&mut self.fresh)
    }

    pub fn select(&mut self, camera: &Camera, budget: usize, now: Instant) -> (Vec<Arc<Mesh>>, Vec<Request>) {
        let Selection { meshes, wanted, .. } = self.select_measured(camera, budget, 1.0, now);
        (meshes, wanted)
    }

    pub fn select_measured(&mut self, camera: &Camera, budget: usize, detail: f64, now: Instant) -> Selection {
        self.coarsest = 0.0;
        self.holes = 0;
        self.refine_at = refine_above(camera) * detail;
        self.backfill.clear();
        let planes = frustum(camera.culling());
        let horizon = Horizon::from(camera.eye);
        let mut wanted = Vec::new();
        let mut root = std::mem::replace(&mut self.root, empty());
        let drawn = self.visit(&mut root, &mut Vec::new(), camera, (&planes, &horizon), &mut wanted, now).unwrap_or_default();
        self.root = root;
        if self.swept.is_none_or(|swept| now.duration_since(swept) >= UNLOAD_SWEEP_EVERY) {
            self.sweep(now);
            self.swept = Some(now);
        }
        wanted.sort_by(|a, b| a.distance.total_cmp(&b.distance).then(a.path.len().cmp(&b.path.len())));
        let wanted = wanted.into_iter().take(budget).filter_map(|candidate| self.claim(candidate.path)).collect();
        let coarsest = if drawn.is_empty() { f64::INFINITY } else { self.coarsest };
        let mut layered = std::mem::take(&mut self.backfill);
        layered.sort_by_key(|(depth, _)| *depth);
        let backfill = layered.into_iter().flat_map(|(_, meshes)| meshes).collect();
        Selection { meshes: drawn, wanted, coarsest, holes: self.holes, backfill }
    }

    fn claim(&mut self, path: Vec<usize>) -> Option<Request> {
        let url = {
            let tile = path.iter().try_fold(&mut self.root, |tile, &child| tile.children.get_mut(child))?;
            tile.content = Content::Requested;
            tile.url.clone()?
        };
        Some(Request { url: self.signed(&url), path })
    }

    fn sweep(&mut self, now: Instant) {
        let root = &mut self.root;
        self.held.retain(|path| {
            let Some(tile) = at_mut(root, path).filter(|tile| holds(tile)) else { return false };
            let stale = tile.used.is_none_or(|used| now.duration_since(used) > UNLOAD_AFTER);
            if stale {
                release(tile);
            }
            !stale
        });
    }

    pub fn trim(&mut self, now: Instant, probed: Option<Instant>) {
        if self.trimmed.is_some_and(|trimmed| now.duration_since(trimmed) < TRIM_EVERY) {
            return;
        }
        self.trimmed = Some(now);
        self.trim_to(MAX_LOADED_TILES, &[Some(now), probed].into_iter().flatten().collect::<Vec<_>>());
    }

    fn trim_to(&mut self, most: usize, seen: &[Instant]) {
        let meshed = |tile: &&Tile| matches!(tile.content, Content::Meshes(_));
        let used = self.held.iter().filter_map(|path| at(&self.root, path)).filter(meshed).filter_map(|tile| tile.used).collect();
        let Some(oldest) = cutoff(used, most) else { return };
        let root = &mut self.root;
        self.held.retain(|path| {
            let Some(tile) = at_mut(root, path).filter(|tile| holds(tile)) else { return false };
            let evicted = matches!(tile.content, Content::Meshes(_)) && !tile.used.is_some_and(|used| seen.iter().any(|at| kept(used, oldest, *at)));
            if evicted {
                tile.content = Content::Pending;
            }
            !evicted
        });
    }

    pub fn deliver(&mut self, path: &[usize], parsed: Result<Parsed, String>, now: Instant) -> Option<Duration> {
        let tile = path.iter().try_fold(&mut self.root, |tile, &child| tile.children.get_mut(child))?;
        if !matches!(tile.content, Content::Requested) {
            return None;
        }
        let waited = tile.wanted.take().map(|wanted| now.saturating_duration_since(wanted));
        let fetched = parsed.is_ok() && tile.url.is_some();
        tile.content = match parsed {
            Err(_) => Content::Failed(now),
            Ok(Parsed::External(child)) => {
                tile.children.push(*child);
                Content::External
            }
            Ok(Parsed::Meshes(meshes)) => {
                let arrived: Vec<Arc<Mesh>> = meshes.into_iter().map(Arc::new).collect();
                self.fresh.extend(arrived.iter().cloned());
                Content::Meshes(arrived)
            }
        };
        if fetched {
            self.held.insert(path.to_vec());
        }
        waited
    }

    fn consider(tile: &mut Tile, path: &[usize], distance: f64, wanted: &mut Vec<Candidate>, now: Instant) {
        if let Content::Failed(at) = tile.content
            && now.duration_since(at) >= RETRY_AFTER
        {
            tile.content = Content::Pending;
        }
        if matches!(tile.content, Content::Pending) && tile.url.is_some() {
            tile.wanted.get_or_insert(now);
            wanted.push(Candidate { distance, path: path.to_vec() });
        }
    }

    fn visit(&mut self, tile: &mut Tile, path: &mut Vec<usize>, camera: &Camera, sight: (&[DVec4; 5], &Horizon), wanted: &mut Vec<Candidate>, now: Instant) -> Option<Vec<Arc<Mesh>>> {
        let (planes, horizon) = sight;
        if planes.iter().any(|plane| tile.bounds.outside(*plane, camera.eye)) || horizon.hides(&tile.bounds) {
            return Some(Vec::new());
        }
        if tile.used.is_some_and(|used| now.duration_since(used) > UNLOAD_AFTER) {
            tile.wanted = None;
        }
        tile.used = Some(now);
        let distance = tile.bounds.distance(camera.eye);
        Self::consider(tile, path, distance, wanted, now);
        let error = camera.screen_space_error(tile.geometric_error, distance) / self.refine_at;
        let refine = error > 1.0;
        let external = matches!(tile.content, Content::External);
        let children = (refine || external) && !tile.children.is_empty();
        let parts: Option<Vec<Option<Vec<Arc<Mesh>>>>> = children.then(|| {
            tile.children
                .iter_mut()
                .enumerate()
                .map(|(index, child)| {
                    path.push(index);
                    let drawn = self.visit(child, path, camera, sight, wanted, now);
                    path.pop();
                    drawn
                })
                .collect()
        });
        let complete = parts.as_ref().is_some_and(|parts| parts.iter().all(Option::is_some));
        let missing = parts.as_ref().map_or(0, |parts| parts.iter().filter(|part| part.is_none()).count());
        let gathered = parts.map(|parts| parts.into_iter().flatten().flatten().collect::<Vec<_>>());
        let (below, partial) = match complete {
            true => (gathered, None),
            false => (None, gathered.filter(|meshes| !meshes.is_empty())),
        };
        let own = match &tile.content {
            Content::Meshes(meshes) => Some(meshes.clone()),
            Content::None if !children => Some(Vec::new()),
            _ => None,
        };
        let loaded_parent = below.is_none() && own.as_ref().is_some_and(|meshes| !meshes.is_empty());
        if loaded_parent && refine && !tile.children.is_empty() && distance < NEAR_M {
            self.coarsest = self.coarsest.max(error);
        }
        let stands_in = loaded_parent && (error <= FALLBACK_TOO_COARSE || partial.is_none());
        if loaded_parent && !stands_in && partial.is_some() && error <= BACKFILL_TOO_COARSE {
            self.backfill.push((path.len(), own.clone().unwrap_or_default()));
        }
        let too_coarse = children && error > BACKFILL_TOO_COARSE;
        let gap = match stands_in {
            true => too_coarse,
            false => below.is_none() && partial.is_some(),
        };
        if gap && distance < NEAR_M {
            self.holes += missing;
        }
        match stands_in {
            true if too_coarse => Some(Vec::new()),
            true => own,
            false => below.or(partial).or(own),
        }
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
        Tile { bounds: ahead(camera, 200.0), geometric_error: 1.0, url: Some(Url::parse(&format!("https://x/{name}.glb")).unwrap()), content: Content::Pending, children: Vec::new(), used: None, wanted: None }
    }

    fn parent(camera: &Camera) -> (Tileset, Arc<Mesh>) {
        let shown = Arc::new(mesh());
        let root = Tile { bounds: ahead(camera, 200.0), geometric_error: 6.0, url: None, content: Content::Meshes(vec![shown.clone()]), children: vec![leaf(camera, "a"), leaf(camera, "b")], used: None, wanted: None };
        (Tileset::new(root, &Url::parse("https://x/root.json").unwrap()), shown)
    }

    #[test]
    fn past_the_tile_budget_the_least_recently_seen_go_but_what_is_drawn_stays() {
        let camera = camera();
        let now = Instant::now();
        let seen = |name: &str, ago: u64| Tile { content: Content::Meshes(vec![Arc::new(mesh())]), used: Some(now - Duration::from_secs(ago)), ..leaf(&camera, name) };
        let url = Url::parse("https://x/root.json").unwrap();
        let tree = |children: Vec<Tile>| Tileset::new(Tile { content: Content::None, url: None, children, ..leaf(&camera, "root") }, &url);
        let meshed = |tiles: &Tileset| tiles.root.children.iter().map(|tile| matches!(tile.content, Content::Meshes(_))).collect::<Vec<_>>();
        let mut drawn = tree(vec![seen("a", 5), seen("b", 0), seen("c", 9)]);
        drawn.trim_to(1, &[now]);
        assert_eq!(meshed(&drawn), [false, true, false], "the tile drawn this frame stays");
        assert_eq!(drawn.held.len(), 1, "evicted tiles leave the index, so the next trim does not look at them");
        let mut probed = tree(vec![seen("view", 0), seen("probe", 1), seen("old", 9)]);
        probed.trim_to(1, &[now, now - Duration::from_secs(1)]);
        assert_eq!(meshed(&probed), [true, true, false], "the ground probe's tiles from its last look stay as well");
    }

    #[test]
    fn a_parent_drawn_in_place_of_its_unloaded_children_is_measured_as_coarse_until_they_arrive() {
        let camera = camera();
        let (mut tiles, _) = parent(&camera);
        let start = Instant::now();
        let waiting = tiles.select_measured(&camera, 8, 1.0, start);
        assert_eq!(waiting.wanted.len(), 2);
        assert!(waiting.coarsest > 1.0, "the parent wanted to refine and could not ({})", waiting.coarsest);
        tiles.deliver(&[0], Ok(Parsed::Meshes(vec![mesh()])), start);
        tiles.deliver(&[1], Ok(Parsed::Meshes(vec![mesh()])), start);
        assert_eq!(tiles.select_measured(&camera, 8, 1.0, start).coarsest, 0.0, "leaves that need no finer tile are full detail");
        let away = Camera::new(&Pose { heading: 180.0, ..Pose { latitude: 0.0, longitude: 0.0, height: 100.0, heading: 0.0, pitch: 0.0, roll: 0.0, fov: 60.0 } }, 1280, 720);
        assert_eq!(tiles.select_measured(&away, 8, 1.0, start).coarsest, f64::INFINITY, "nothing drawn is not ready");
    }

    #[test]
    fn a_tile_with_nothing_of_its_own_draws_the_children_it_has_instead_of_nothing() {
        let camera = camera();
        let shown = Arc::new(mesh());
        let ready = Tile { content: Content::Meshes(vec![shown.clone()]), ..leaf(&camera, "ready") };
        let index = Tile { bounds: ahead(&camera, 200.0), geometric_error: 1000.0, url: None, content: Content::None, children: vec![ready, leaf(&camera, "coming")], used: None, wanted: None };
        let mut tiles = Tileset::new(index, &Url::parse("https://x/root.json").unwrap());
        let selected = tiles.select_measured(&camera, 8, 1.0, Instant::now());
        assert!(selected.meshes.len() == 1 && Arc::ptr_eq(&selected.meshes[0], &shown), "one tile still loading blanked every loaded sibling, and climbing showed haze for 24 s");
        assert_eq!(selected.wanted.len(), 1, "the missing one is still asked for");
        assert_eq!(selected.holes, 1, "the missing one is counted as a hole in the picture");
    }

    #[test]
    fn the_nearest_missing_tile_is_fetched_first_wherever_it_sits_in_the_tree() {
        let camera = camera();
        let far = Tile { bounds: ahead(&camera, 900.0), ..leaf(&camera, "far") };
        let near = Tile { bounds: ahead(&camera, 100.0), ..leaf(&camera, "near") };
        let index = Tile { bounds: ahead(&camera, 500.0), geometric_error: 1000.0, url: None, content: Content::None, children: vec![far, near], used: None, wanted: None };
        let mut tiles = Tileset::new(index, &Url::parse("https://x/root.json").unwrap());
        let (_, wanted) = tiles.select(&camera, 1, Instant::now());
        assert_eq!(wanted.iter().map(|request| request.path.clone()).collect::<Vec<_>>(), vec![vec![1]], "in tree order, tiles in the first branches took every free slot and the ground ahead of a low, fast drone stayed missing");
        let (_, wanted) = tiles.select(&camera, 1, Instant::now());
        assert_eq!(wanted.iter().map(|request| request.path.clone()).collect::<Vec<_>>(), vec![vec![0]], "the far one follows once a slot frees up");
    }

    #[test]
    fn a_far_too_coarse_parent_does_not_replace_the_children_it_has_but_a_close_enough_one_covers_the_gap() {
        let camera = camera();
        let child = Arc::new(mesh());
        let parent_mesh = Arc::new(mesh());
        let tree = |geometric_error: f64| Tile {
            bounds: ahead(&camera, 200.0),
            geometric_error,
            url: Some(Url::parse("https://x/parent.glb").unwrap()),
            content: Content::Meshes(vec![parent_mesh.clone()]),
            children: vec![Tile { content: Content::Meshes(vec![child.clone()]), ..leaf(&camera, "ready") }, leaf(&camera, "coming")],
            used: None,
            wanted: None,
        };
        let selected = |geometric_error: f64| Tileset::new(tree(geometric_error), &Url::parse("https://x/root.json").unwrap()).select_measured(&camera, 8, 1.0, Instant::now());
        let coarse = selected(100.0);
        assert!(coarse.meshes.len() == 1 && Arc::ptr_eq(&coarse.meshes[0], &child), "a much coarser parent does not replace the city it would cover");
        assert!(coarse.backfill.len() == 1 && Arc::ptr_eq(&coarse.backfill[0], &parent_mesh), "it fills the hole the missing child leaves, drawn behind everything");
        let globe = selected(1_000_000.0);
        assert!(globe.meshes.len() == 1 && Arc::ptr_eq(&globe.meshes[0], &child));
        assert!(globe.backfill.is_empty(), "a globe tile's flat chord runs kilometres under the ground and hazes to sky, so it fills nothing");
        let mut nothing_below = Tileset::new(Tile { children: vec![leaf(&camera, "a"), leaf(&camera, "b")], ..tree(1_000_000.0) }, &Url::parse("https://x/root.json").unwrap());
        assert!(nothing_below.select(&camera, 8, Instant::now()).0.is_empty(), "with no child loaded it is not drawn either; the open-data terrain shows instead of a sky-coloured slab");
        let close = selected(3.0);
        assert!(close.meshes.len() == 1 && Arc::ptr_eq(&close.meshes[0], &parent_mesh), "a parent nearly fine enough covers the gap rather than leave a hole");
        assert!(close.backfill.is_empty());
    }

    #[test]
    fn the_finest_tile_under_a_landed_drone_is_drawn_however_close_the_camera_sits() {
        let camera = camera();
        let ground = Arc::new(mesh());
        let under = Tile { bounds: Obb { center: camera.eye, half: [DVec3::X * 20.0, DVec3::Y * 20.0, DVec3::Z * 1.0] }, geometric_error: 2.0, content: Content::Meshes(vec![ground.clone()]), ..leaf(&camera, "under") };
        let selected = Tileset::new(under, &Url::parse("https://x/root.json").unwrap()).select_measured(&camera, 8, 1.0, Instant::now());
        assert!(selected.meshes.len() == 1 && Arc::ptr_eq(&selected.meshes[0], &ground), "a leaf within 2 m counted as far too coarse, so on the ground the tile under the drone vanished and the backfill showed");
        assert_eq!(selected.holes, 0);
    }

    #[test]
    fn arrived_meshes_are_handed_out_once_for_upload_not_walked_every_frame() {
        let camera = camera();
        let (mut tiles, _) = parent(&camera);
        let start = Instant::now();
        tiles.select(&camera, 8, start);
        tiles.deliver(&[0], Ok(Parsed::Meshes(vec![mesh(), mesh()])), start);
        assert_eq!(tiles.take_fresh().len(), 2);
        assert!(tiles.take_fresh().is_empty(), "walking every loaded tile each frame to find them cost a third of the phone's render thread");
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
        assert_eq!(tiles.deliver(&[1], Ok(Parsed::Meshes(vec![mesh()])), start + Duration::from_millis(300)), Some(Duration::from_millis(300)), "the wait runs from the first frame that wanted the tile");
        assert_eq!(tiles.deliver(&[1], Ok(Parsed::Meshes(vec![mesh()])), start), None, "a tile nobody asked for again is not measured");
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

    #[test]
    fn an_unloaded_tileset_takes_the_tiles_under_it_out_of_the_index() {
        let camera = camera();
        let url = Url::parse("https://x/root.json").unwrap();
        let external = Tile { url: Some(Url::parse("https://x/sub.json").unwrap()), ..leaf(&camera, "sub") };
        let mut tiles = Tileset::new(Tile { content: Content::None, url: None, geometric_error: 1000.0, children: vec![external], ..leaf(&camera, "root") }, &url);
        let start = Instant::now();
        tiles.select(&camera, 8, start);
        tiles.deliver(&[0], Ok(Parsed::External(Box::new(leaf(&camera, "inner")))), start);
        tiles.select(&camera, 8, start);
        tiles.deliver(&[0, 0], Ok(Parsed::Meshes(vec![mesh()])), start);
        assert_eq!(tiles.held.len(), 2);
        let away = Camera::new(&Pose { latitude: 50.0, longitude: 50.0, height: 100.0, heading: 0.0, pitch: 0.0, roll: 0.0, fov: 60.0 }, 1280, 720);
        tiles.select(&away, 8, start + UNLOAD_AFTER + Duration::from_secs(1));
        assert!(tiles.held.is_empty() && tiles.root.children[0].children.is_empty(), "walking the whole tree every second to find these took 20-44 ms a frame on the phone");
    }

    #[test]
    fn a_fast_turn_keeps_the_coarse_tile_it_has_instead_of_fetching_finer_ones() {
        let camera = camera();
        let (mut steady, _) = parent(&camera);
        assert_eq!(steady.select_measured(&camera, 8, 1.0, Instant::now()).wanted.len(), 2, "held still, the view refines");
        let (mut turning, shown) = parent(&camera);
        let coarse = turning.select_measured(&camera, 8, 3.0, Instant::now());
        assert!(coarse.wanted.is_empty(), "a yaw on the phone reloaded 2,500 tiles in 10 s because finer tiles were fetched and evicted every turn");
        assert!(coarse.meshes.len() == 1 && Arc::ptr_eq(&coarse.meshes[0], &shown));
    }
}
