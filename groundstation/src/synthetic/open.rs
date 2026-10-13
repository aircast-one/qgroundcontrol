use std::collections::{BTreeMap, HashMap};
use std::f64::consts::{FRAC_1_SQRT_2, PI};
use std::sync::Arc;
use std::time::{Duration, Instant};

use glam::{DMat3, DMat4, DVec3, DVec4};

use super::fetch::Fetcher;
use super::geo::{Camera, east_north_up, ecef};
use super::ground;
use super::mesh::{Layer, Mesh, solid};
use super::tileset::{RETRY_AFTER, cutoff, frustum, kept};

pub const TERRAIN: &str = "Terrarium";
pub const BUILDINGS: &str = "Buildings";
const TERRAIN_ZOOM: u32 = 15;
const MAX_ZOOM: u32 = 19;
const BORROWED_LEVELS: u32 = 3;
const GRID: usize = 16;
const TEXELS: f64 = 256.0;
const MAX_TEXEL_PX: f64 = 1.5;
const UNKNOWN_RELIEF_M: f64 = 2_000.0;
const KNOWN_RELIEF_MARGIN_M: f64 = 50.0;
const REACH_M: f64 = 60_000.0;
const BUILDING_ZOOM: u32 = 14;
const BUILDING_REACH_M: f64 = 1_500.0;
const DEFAULT_EXTENT: f64 = 4096.0;
const KEEP_TERRAIN_FOR: Duration = Duration::from_secs(20);
const KEEP_HOUSES_FOR: Duration = Duration::from_secs(60);
const KEEP_DATA_FOR: Duration = Duration::from_secs(120);
const MAX_TERRAIN_TILES: usize = 800;
const MAX_HEIGHT_TILES: usize = 500;
const UNIMAGED: [u8; 4] = [118, 124, 104, 255];
const ROOF: [u8; 4] = [217, 211, 199, 255];
const WALL: [u8; 4] = [200, 194, 182, 255];
const ROOF_U: f32 = 0.25;
const WALL_U: f32 = 0.75;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TileKey {
    pub z: u32,
    pub x: u32,
    pub y: u32,
}

impl TileKey {
    fn children(self) -> [TileKey; 4] {
        let (z, x, y) = (self.z + 1, self.x * 2, self.y * 2);
        [TileKey { z, x, y }, TileKey { z, x: x + 1, y }, TileKey { z, x, y: y + 1 }, TileKey { z, x: x + 1, y: y + 1 }]
    }

    fn ancestor(self, zoom: u32) -> (TileKey, f64, f64, f64) {
        let up = self.z.saturating_sub(zoom);
        let scale = f64::from(1u32 << up);
        let key = TileKey { z: self.z - up, x: self.x >> up, y: self.y >> up };
        (key, f64::from(self.x) / scale - f64::from(key.x), f64::from(self.y) / scale - f64::from(key.y), 1.0 / scale)
    }

    fn longitude(self, u: f64) -> f64 {
        (f64::from(self.x) + u) / f64::from(1u32 << self.z) * 360.0 - 180.0
    }

    fn latitude(self, v: f64) -> f64 {
        (PI * (1.0 - 2.0 * (f64::from(self.y) + v) / f64::from(1u32 << self.z))).sinh().atan().to_degrees()
    }

    fn width_m(self) -> f64 {
        2.0 * PI * 6_378_137.0 * self.latitude(0.5).to_radians().cos() / f64::from(1u32 << self.z)
    }

    pub fn containing(latitude: f64, longitude: f64, z: u32) -> (TileKey, f64, f64) {
        let n = f64::from(1u32 << z);
        let x = (longitude + 180.0) / 360.0 * n;
        let y = (1.0 - latitude.to_radians().tan().asinh() / PI) / 2.0 * n;
        let (tx, ty) = (x.floor().clamp(0.0, n - 1.0), y.floor().clamp(0.0, n - 1.0));
        (TileKey { z, x: tx as u32, y: ty as u32 }, x - tx, y - ty)
    }
}

pub struct Heights {
    size: usize,
    metres: Vec<f32>,
}

impl Heights {
    pub fn from_terrarium(png: &[u8]) -> Result<Self, String> {
        let image = image::load_from_memory(png).map_err(|error| format!("terrarium tile: {error}"))?.to_rgb8();
        let size = image.width() as usize;
        if image.height() as usize != size || size < 2 {
            return Err(format!("terrarium tile is {}x{}, not square", image.width(), image.height()));
        }
        let metres = image.pixels().map(|p| f32::from(p[0]) * 256.0 + f32::from(p[1]) + f32::from(p[2]) / 256.0 - 32_768.0).collect();
        Ok(Self { size, metres })
    }

    pub fn at(&self, u: f64, v: f64) -> f64 {
        let last = (self.size - 1) as f64;
        let (fx, fy) = ((u * last).clamp(0.0, last), (v * last).clamp(0.0, last));
        let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.size - 1), (y0 + 1).min(self.size - 1));
        let (ax, ay) = (fx - x0 as f64, fy - y0 as f64);
        let h = |x: usize, y: usize| f64::from(self.metres[y * self.size + x]);
        (h(x0, y0) * (1.0 - ax) + h(x1, y0) * ax) * (1.0 - ay) + (h(x0, y1) * (1.0 - ax) + h(x1, y1) * ax) * ay
    }

    fn range(&self, du: f64, dv: f64, span: f64) -> (f64, f64) {
        let last = (self.size - 1) as f64;
        let pixel = |t: f64| ((t * last).round() as usize).min(self.size - 1);
        let (x0, x1, y0, y1) = (pixel(du), pixel(du + span), pixel(dv), pixel(dv + span));
        (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| f64::from(self.metres[y * self.size + x]))).fold((f64::MAX, f64::MIN), |(low, high), h| (low.min(h), high.max(h)))
    }
}

#[derive(Clone)]
pub struct Patch {
    heights: Arc<Heights>,
    zoom: u32,
    du: f64,
    dv: f64,
    span: f64,
}

impl Patch {
    fn at(&self, u: f64, v: f64) -> f64 {
        self.heights.at(self.du + u * self.span, self.dv + v * self.span)
    }
}

pub enum Want {
    Heights(TileKey),
    Terrain { key: TileKey, imagery: String, patch: Patch },
    Houses { key: TileKey, patch: Patch },
}

pub enum Built {
    Heights(TileKey, Result<Heights, String>),
    Terrain { key: TileKey, imagery: String, zoom: u32, mesh: Result<(Mesh, bool), String> },
    Houses(TileKey, Result<Option<Mesh>, String>),
}

impl Want {
    pub fn failed(&self, reason: &str) -> Built {
        match self {
            Want::Heights(key) => Built::Heights(*key, Err(reason.to_string())),
            Want::Terrain { key, imagery, patch } => Built::Terrain { key: *key, imagery: imagery.clone(), zoom: patch.zoom, mesh: Err(reason.to_string()) },
            Want::Houses { key, .. } => Built::Houses(*key, Err(reason.to_string())),
        }
    }

    pub fn build(self, fetcher: &dyn Fetcher) -> Built {
        match self {
            Want::Heights(key) => Built::Heights(key, fetcher.map(TERRAIN, key.z, key.x, key.y).and_then(|png| Heights::from_terrarium(&png))),
            Want::Terrain { key, imagery, patch } => {
                let found = picture(fetcher, &imagery, key);
                let imaged = found.as_ref().is_some_and(|(_, own)| *own);
                Built::Terrain { key, imagery, zoom: patch.zoom, mesh: Ok((terrain_mesh(key, Some(&patch), found.map(|(picture, _)| picture)), imaged)) }
            }
            Want::Houses { key, patch } => Built::Houses(key, fetcher.map(BUILDINGS, key.z, key.x, key.y).map(|json| building_mesh(key, &json, Some(&patch)))),
        }
    }

    fn priority(&self) -> (u32, u8, u32, u32) {
        match self {
            Want::Terrain { key, .. } => (key.z, 0, key.x, key.y),
            Want::Heights(key) => (key.z, 1, key.x, key.y),
            Want::Houses { key, .. } => (key.z, 2, key.x, key.y),
        }
    }

    pub fn describe(&self) -> String {
        let (kind, key) = match self {
            Want::Heights(key) => (TERRAIN.to_string(), key),
            Want::Terrain { key, imagery, .. } => (imagery.clone(), key),
            Want::Houses { key, .. } => (BUILDINGS.to_string(), key),
        };
        format!("{kind} {}/{}/{}", key.z, key.x, key.y)
    }
}

enum Slot<T> {
    Asked,
    Have(Arc<T>),
    Failed(Instant),
}

struct Store<T> {
    slots: HashMap<TileKey, (Slot<T>, Instant)>,
}

enum Lookup<T> {
    Ready(Arc<T>),
    Waiting,
    Missing,
}

impl<T> Store<T> {
    fn new() -> Self {
        Self { slots: HashMap::new() }
    }

    fn lookup(&mut self, key: TileKey, now: Instant) -> Lookup<T> {
        match self.slots.get_mut(&key) {
            None => Lookup::Missing,
            Some((slot, used)) => {
                *used = now;
                match slot {
                    Slot::Have(value) => Lookup::Ready(value.clone()),
                    Slot::Failed(at) if now.duration_since(*at) >= RETRY_AFTER => Lookup::Missing,
                    _ => Lookup::Waiting,
                }
            }
        }
    }

    fn ask(&mut self, key: TileKey, now: Instant) {
        self.slots.insert(key, (Slot::Asked, now));
    }

    fn put(&mut self, key: TileKey, value: Result<T, String>, now: Instant) {
        self.slots.insert(key, (value.map_or(Slot::Failed(now), |value| Slot::Have(Arc::new(value))), now));
    }

    fn trim(&mut self, now: Instant, keep: Duration, most: usize) {
        self.slots.retain(|_, (slot, used)| matches!(slot, Slot::Asked) || now.duration_since(*used) < keep);
        let oldest = cutoff(self.slots.values().map(|(_, used)| *used).collect(), most);
        oldest.into_iter().for_each(|at| self.slots.retain(|_, (slot, used)| matches!(slot, Slot::Asked) || kept(*used, at, now)));
    }
}

fn awaited(asked: Option<Instant>, now: Instant) -> bool {
    asked.is_some_and(|at| now.duration_since(at) < RETRY_AFTER)
}

struct TerrainEntry {
    mesh: Option<Arc<Mesh>>,
    zoom: u32,
    imaged: bool,
    asked: Option<Instant>,
    used: Instant,
}

struct HouseEntry {
    mesh: Option<Arc<Mesh>>,
    loaded: bool,
    asked: Option<Instant>,
    used: Instant,
}

struct Sight<'a> {
    camera: &'a Camera,
    planes: [DVec4; 5],
    ground: f64,
    detail: f64,
}

pub struct OpenScene {
    imagery: String,
    heights: Store<Heights>,
    terrain: HashMap<TileKey, TerrainEntry>,
    houses: HashMap<TileKey, HouseEntry>,
}

fn frame_at(latitude: f64, longitude: f64) -> (DMat4, DMat3, DVec3) {
    let enu = east_north_up(latitude, longitude);
    let origin = ecef(latitude, longitude, 0.0);
    (DMat4::from_mat3_translation(enu, origin), enu.transpose(), origin)
}

fn picture(fetcher: &dyn Fetcher, imagery: &str, key: TileKey) -> Option<(image::RgbaImage, bool)> {
    (0..=BORROWED_LEVELS.min(key.z)).find_map(|up| {
        let (source, du, dv, span) = key.ancestor(key.z - up);
        let whole = image::load_from_memory(&fetcher.map(imagery, source.z, source.x, source.y).ok()?).ok()?.to_rgba8();
        let (width, height) = whole.dimensions();
        let pixels = |t: f64, size: u32| (t * f64::from(size)).round() as u32;
        let part = image::imageops::crop_imm(&whole, pixels(du, width), pixels(dv, height), pixels(span, width).max(1), pixels(span, height).max(1)).to_image();
        Some((part, up == 0))
    })
}

fn terrain_mesh(key: TileKey, patch: Option<&Patch>, picture: Option<image::RgbaImage>) -> Mesh {
    let height_at = |u: f64, v: f64| patch.map_or(0.0, |patch| patch.at(u, v));
    let (to_ecef, to_local, origin) = frame_at(key.latitude(0.5), key.longitude(0.5));
    let local = |u: f64, v: f64, height: f64| -> [f32; 3] { (to_local * (ecef(key.latitude(v), key.longitude(u), height) - origin)).as_vec3().to_array() };
    let skirt = key.width_m() / GRID as f64;
    let steps = GRID + 1;
    let at = |index: usize| ((index % steps) as f64 / GRID as f64, (index / steps) as f64 / GRID as f64);
    let grid: Vec<([f32; 3], [f32; 2])> = (0..steps * steps).map(at).map(|(u, v)| (local(u, v, height_at(u, v)), [u as f32, v as f32])).collect();
    let edge: Vec<usize> = (0..GRID).chain((0..GRID).map(|i| i * steps + GRID)).chain((0..GRID).map(|i| GRID * steps + GRID - i)).chain((0..GRID).map(|i| (GRID - i) * steps)).collect();
    let lowered = edge.iter().map(|&index| {
        let (u, v) = at(index);
        (local(u, v, height_at(u, v) - skirt), grid[index].1)
    });
    let cells = (0..GRID).flat_map(|row| (0..GRID).map(move |column| row * steps + column)).flat_map(|i| [i, i + 1, i + steps, i + 1, i + steps + 1, i + steps]);
    let ring = edge.len();
    let skirts = (0..ring).flat_map(|i| {
        let (top_a, top_b) = (edge[i], edge[(i + 1) % ring]);
        let (low_a, low_b) = (grid.len() + i, grid.len() + (i + 1) % ring);
        [top_a, top_b, low_a, top_b, low_b, low_a]
    });
    let indices: Vec<u32> = cells.chain(skirts).map(|i| i as u32).collect();
    let (positions, uvs): (Vec<[f32; 3]>, Vec<[f32; 2]>) = grid.iter().copied().chain(lowered).unzip();
    let step = 1.0 / GRID as f64;
    let point = |u: f64, v: f64| DVec3::from_array(local(u, v, height_at(u, v)).map(f64::from));
    let upward: Vec<[f32; 3]> = (0..steps * steps).map(at).map(|(u, v)| (point(u, v + step) - point(u, v - step)).cross(point(u + step, v) - point(u - step, v)).normalize_or_zero().as_vec3().to_array()).collect();
    let normals = upward.iter().copied().chain(edge.iter().map(|&index| upward[index])).collect();
    Mesh::new(to_ecef, positions, uvs, indices, picture.unwrap_or_else(|| solid(UNIMAGED)), Layer::Opaque).with_normals(normals)
}

fn palette() -> image::RgbaImage {
    image::RgbaImage::from_fn(2, 1, |x, _| image::Rgba(if x == 0 { ROOF } else { WALL }))
}

#[derive(Default)]
struct Part {
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

fn joined(parts: impl Iterator<Item = Part>) -> Part {
    parts.fold(Part::default(), |whole, part| {
        let offset = whole.positions.len() as u32;
        Part {
            positions: whole.positions.into_iter().chain(part.positions).collect(),
            uvs: whole.uvs.into_iter().chain(part.uvs).collect(),
            normals: whole.normals.into_iter().chain(part.normals).collect(),
            indices: whole.indices.into_iter().chain(part.indices.into_iter().map(|i| i + offset)).collect(),
        }
    })
}

fn wall(a: DVec3, b: DVec3, low: f64, high: f64) -> Part {
    let up = |p: DVec3, h: f64| (p + DVec3::Z * h).as_vec3().to_array();
    let along = b - a;
    let outward = DVec3::new(-along.y, along.x, 0.0).normalize_or_zero().as_vec3().to_array();
    Part { positions: vec![up(a, low), up(b, low), up(b, high), up(a, high)], uvs: vec![[WALL_U, 0.5]; 4], normals: vec![outward; 4], indices: vec![0, 1, 2, 0, 2, 3] }
}

fn roof(rings: &[Vec<DVec3>], height: f64) -> Part {
    let flat: Vec<[f64; 2]> = rings.iter().flatten().map(|p| [p.x, p.y]).collect();
    let holes: Vec<u32> = rings.iter().scan(0u32, |at, ring| {
        let start = *at;
        *at += ring.len() as u32;
        Some(start)
    })
    .skip(1)
    .collect();
    let mut triangles: Vec<u32> = Vec::new();
    earcut::Earcut::<f64>::new().earcut(flat.iter().copied(), &holes, &mut triangles);
    Part { positions: rings.iter().flatten().map(|p| (*p + DVec3::Z * height).as_vec3().to_array()).collect(), uvs: vec![[ROOF_U, 0.5]; flat.len()], normals: vec![[0.0, 0.0, 1.0]; flat.len()], indices: triangles }
}

fn building_mesh(key: TileKey, json: &[u8], patch: Option<&Patch>) -> Option<Mesh> {
    let tile: serde_json::Value = serde_json::from_slice(json).ok()?;
    let extent = tile.get("extent").and_then(serde_json::Value::as_f64).unwrap_or(DEFAULT_EXTENT);
    let ground = |u: f64, v: f64| patch.map_or(0.0, |patch| patch.at(u, v));
    let (to_ecef, to_local, origin) = frame_at(key.latitude(0.5), key.longitude(0.5));
    let flat_at = |u: f64, v: f64| -> DVec3 { to_local * (ecef(key.latitude(v), key.longitude(u), 0.0) - origin) };
    let parts = tile.get("buildings").and_then(serde_json::Value::as_array).into_iter().flatten().filter_map(|building| {
        let rings: Vec<Vec<(f64, f64)>> = building.get("rings")?.as_array()?.iter().filter_map(serde_json::Value::as_array).map(|flat| flat.chunks_exact(2).filter_map(|p| Some((p[0].as_f64()? / extent, p[1].as_f64()? / extent))).collect()).collect();
        let outline = rings.first().filter(|ring| ring.len() >= 3)?;
        let top = building.get("height").and_then(serde_json::Value::as_f64)?;
        let base = building.get("base").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
        let floor = ground(outline[0].0, outline[0].1);
        let placed: Vec<Vec<DVec3>> = rings.iter().map(|ring| ring.iter().map(|&(u, v)| flat_at(u, v)).collect()).collect();
        let walls = placed.iter().flat_map(|ring| (0..ring.len()).map(move |i| wall(ring[i], ring[(i + 1) % ring.len()], floor + base, floor + top)));
        Some(joined(walls.chain(std::iter::once(roof(&placed, floor + top)))))
    });
    let whole = joined(parts);
    (!whole.indices.is_empty()).then(|| Mesh::new(to_ecef, whole.positions, whole.uvs, whole.indices, palette(), Layer::Opaque).with_normals(whole.normals))
}

impl Default for OpenScene {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenScene {
    pub fn new() -> Self {
        Self { imagery: String::new(), heights: Store::new(), terrain: HashMap::new(), houses: HashMap::new() }
    }

    pub fn deliver(&mut self, built: Built, now: Instant) {
        match built {
            Built::Heights(key, heights) => self.heights.put(key, heights, now),
            Built::Terrain { key, imagery, zoom, mesh: Ok((mesh, imaged)) } if imagery == self.imagery => {
                self.terrain.insert(key, TerrainEntry { mesh: Some(Arc::new(mesh)), zoom, imaged, asked: (!imaged).then_some(now), used: now });
            }
            Built::Terrain { .. } => {}
            Built::Houses(key, Ok(mesh)) => {
                self.houses.insert(key, HouseEntry { mesh: mesh.map(Arc::new), loaded: true, asked: None, used: now });
            }
            Built::Houses(..) => {}
        }
    }

    fn issue(&mut self, candidates: Vec<Want>, budget: usize, now: Instant) -> Vec<Want> {
        let chosen: Vec<Want> = candidates.into_iter().map(|want| (want.priority(), want)).collect::<BTreeMap<_, _>>().into_values().take(budget).collect();
        chosen.iter().for_each(|want| match want {
            Want::Heights(key) => self.heights.ask(*key, now),
            Want::Terrain { key, .. } => self.terrain.entry(*key).and_modify(|entry| entry.asked = Some(now)).or_insert(TerrainEntry { mesh: None, zoom: 0, imaged: false, asked: Some(now), used: now }).used = now,
            Want::Houses { key, .. } => self.houses.entry(*key).and_modify(|entry| entry.asked = Some(now)).or_insert(HouseEntry { mesh: None, loaded: false, asked: Some(now), used: now }).used = now,
        });
        chosen
    }

    fn patch(&mut self, key: TileKey, wanted: &mut Vec<Want>, now: Instant) -> Option<Patch> {
        let finest = key.z.min(TERRAIN_ZOOM);
        (0..=finest).rev().fold((None, false), |(found, asked): (Option<Patch>, bool), zoom| {
            if found.is_some() {
                return (found, asked);
            }
            let (source, du, dv, span) = key.ancestor(zoom);
            match self.heights.lookup(source, now) {
                Lookup::Ready(heights) => (Some(Patch { heights, zoom, du, dv, span }), asked),
                Lookup::Missing if !asked => {
                    wanted.push(Want::Heights(source));
                    (None, true)
                }
                _ => (None, asked),
            }
        })
        .0
    }

    pub fn ground(&mut self, latitude: f64, longitude: f64, budget: usize, now: Instant) -> (Option<f64>, Vec<Want>) {
        let (key, u, v) = TileKey::containing(latitude, longitude, TERRAIN_ZOOM);
        let mut candidates = Vec::new();
        let terrain = self.patch(key, &mut candidates, now).map(|patch| patch.at(u, v));
        let houses: Vec<Arc<Mesh>> = self.houses.values().filter_map(|entry| entry.mesh.clone()).collect();
        let roof = ground::height_at(&houses, latitude, longitude);
        (terrain.map(|terrain| roof.map_or(terrain, |roof| roof.max(terrain))), self.issue(candidates, budget, now))
    }

    fn terrain(&mut self, key: TileKey, wanted: &mut Vec<Want>, now: Instant) -> Option<Arc<Mesh>> {
        let patch = self.patch(key, wanted, now);
        let entry = self.terrain.entry(key).or_insert(TerrainEntry { mesh: None, zoom: 0, imaged: false, asked: None, used: now });
        entry.used = now;
        let stale = entry.mesh.is_none() || !entry.imaged || patch.as_ref().is_some_and(|patch| patch.zoom > entry.zoom);
        let idle = entry.asked.is_none_or(|asked| now.duration_since(asked) >= RETRY_AFTER);
        if let Some(patch) = patch.filter(|_| stale && idle) {
            wanted.push(Want::Terrain { key, imagery: self.imagery.clone(), patch });
        }
        entry.mesh.clone()
    }

    fn visible(&mut self, key: TileKey, camera: &Camera, planes: &[DVec4; 5], ground: f64, now: Instant) -> Option<f64> {
        let (source, du, dv, span) = key.ancestor(key.z.min(TERRAIN_ZOOM));
        let (low, high) = match self.heights.lookup(source, now) {
            Lookup::Ready(heights) => {
                let (low, high) = heights.range(du, dv, span);
                (low - KNOWN_RELIEF_MARGIN_M, high + KNOWN_RELIEF_MARGIN_M)
            }
            _ => (ground - UNKNOWN_RELIEF_M, ground + UNKNOWN_RELIEF_M),
        };
        let center = ecef(key.latitude(0.5), key.longitude(0.5), (low + high) / 2.0);
        let radius = key.width_m() * FRAC_1_SQRT_2 + (high - low) / 2.0;
        let distance = (center - camera.eye).length();
        let inside = distance - radius < REACH_M && !planes.iter().any(|plane| plane.truncate().dot(center - camera.eye) + plane.w < -radius);
        inside.then_some((distance - radius).max(1.0))
    }

    fn visit(&mut self, key: TileKey, sight: &Sight, wanted: &mut Vec<Want>, now: Instant) -> Option<Vec<Arc<Mesh>>> {
        let Some(distance) = self.visible(key, sight.camera, &sight.planes, sight.ground, now) else { return Some(Vec::new()) };
        let refine = key.z < MAX_ZOOM && sight.camera.screen_space_error(key.width_m() / TEXELS, distance) > MAX_TEXEL_PX * sight.detail;
        let own = self.terrain(key, wanted, now);
        let below = refine.then(|| {
            key.children()
                .iter()
                .map(|child| self.visit(*child, sight, wanted, now))
                .collect::<Vec<_>>()
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .map(|lists| lists.into_iter().flatten().collect::<Vec<_>>())
        });
        below.flatten().or_else(|| own.map(|mesh| vec![mesh]))
    }

    fn house(&mut self, key: TileKey, wanted: &mut Vec<Want>, now: Instant) -> Option<Arc<Mesh>> {
        let (loaded, idle) = {
            let entry = self.houses.entry(key).or_insert(HouseEntry { mesh: None, loaded: false, asked: None, used: now });
            entry.used = now;
            (entry.loaded, entry.asked.is_none_or(|asked| now.duration_since(asked) >= RETRY_AFTER))
        };
        if !loaded
            && idle
            && let Some(patch) = self.patch(key, wanted, now)
        {
            wanted.push(Want::Houses { key, patch });
        }
        self.houses.get(&key).and_then(|entry| entry.mesh.clone())
    }

    fn houses_around(&mut self, latitude: f64, longitude: f64, wanted: &mut Vec<Want>, now: Instant) -> Vec<Arc<Mesh>> {
        let (center, _, _) = TileKey::containing(latitude, longitude, BUILDING_ZOOM);
        let span = (BUILDING_REACH_M / center.width_m()).ceil() as i64;
        let around: Vec<TileKey> = (-span..=span)
            .flat_map(|dy| (-span..=span).map(move |dx| (dx, dy)))
            .filter_map(|(dx, dy)| Some(TileKey { z: BUILDING_ZOOM, x: u32::try_from(i64::from(center.x) + dx).ok()?, y: u32::try_from(i64::from(center.y) + dy).ok()? }))
            .collect();
        around.into_iter().filter_map(|key| self.house(key, wanted, now)).collect()
    }

    pub fn loaded_tiles(&self) -> usize {
        self.terrain.values().filter(|entry| entry.mesh.is_some()).count() + self.houses.values().filter(|entry| entry.mesh.is_some()).count()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn select(&mut self, camera: &Camera, imagery: &str, ground: f64, latitude: f64, longitude: f64, detail: f64, budget: usize, now: Instant) -> (Vec<Arc<Mesh>>, Vec<Want>) {
        if imagery != self.imagery {
            self.imagery = imagery.to_string();
            self.terrain.clear();
        }
        let sight = Sight { camera, planes: frustum(camera.culling()), ground, detail };
        let mut candidates = Vec::new();
        let ground_meshes = self.visit(TileKey { z: 0, x: 0, y: 0 }, &sight, &mut candidates, now).unwrap_or_default();
        let houses = self.houses_around(latitude, longitude, &mut candidates, now);
        let wanted = self.issue(candidates, budget, now);
        self.terrain.retain(|_, entry| now.duration_since(entry.used) < KEEP_TERRAIN_FOR);
        let oldest = cutoff(self.terrain.values().map(|entry| entry.used).collect(), MAX_TERRAIN_TILES);
        oldest.into_iter().for_each(|at| self.terrain.retain(|_, entry| kept(entry.used, at, now) || awaited(entry.asked, now)));
        self.houses.retain(|_, entry| now.duration_since(entry.used) < KEEP_HOUSES_FOR);
        self.heights.trim(now, KEEP_DATA_FOR, MAX_HEIGHT_TILES);
        (ground_meshes.into_iter().chain(houses).collect(), wanted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::geo::Pose;

    struct Offline;

    impl Fetcher for Offline {
        fn url(&self, _: &url::Url) -> Result<Vec<u8>, String> {
            Err("offline".into())
        }
        fn map(&self, provider: &str, z: u32, x: u32, y: u32) -> Result<Vec<u8>, String> {
            match provider {
                TERRAIN => {
                    let mut png = std::io::Cursor::new(Vec::new());
                    image::RgbImage::from_pixel(4, 4, image::Rgb([130, 92, 0])).write_to(&mut png, image::ImageFormat::Png).map_err(|e| e.to_string())?;
                    Ok(png.into_inner())
                }
                _ => Err(format!("no {provider} {z}/{x}/{y}")),
            }
        }
    }

    fn camera() -> Camera {
        Camera::new(&Pose { latitude: -35.36213, longitude: 149.16514, height: 330.0, heading: 200.0, pitch: -15.0, roll: 0.0, fov: 70.0 }, 1280, 720)
    }

    fn run(scene: &mut OpenScene, camera: &Camera, rounds: usize, start: Instant) -> Vec<Vec<Arc<Mesh>>> {
        (0..rounds)
            .map(|round| {
                let now = start + Duration::from_millis(round as u64 * 100);
                let (meshes, wanted) = scene.select(camera, "Bing Satellite", 300.0, -35.36213, 149.16514, 1.0, 24, now);
                wanted.into_iter().map(|want| want.build(&Offline)).for_each(|built| scene.deliver(built, now));
                meshes
            })
            .collect()
    }

    struct Served;

    impl Fetcher for Served {
        fn url(&self, _: &url::Url) -> Result<Vec<u8>, String> {
            Err("offline".into())
        }
        fn map(&self, provider: &str, z: u32, x: u32, y: u32) -> Result<Vec<u8>, String> {
            let mut png = std::io::Cursor::new(Vec::new());
            match provider {
                TERRAIN => image::RgbImage::from_pixel(4, 4, image::Rgb([130, 92, 0])).write_to(&mut png, image::ImageFormat::Png).map_err(|e| e.to_string())?,
                BUILDINGS => return Err(format!("no buildings {z}/{x}/{y}")),
                _ => image::RgbImage::from_pixel(1, 1, image::Rgb([90, 110, 70])).write_to(&mut png, image::ImageFormat::Png).map_err(|e| e.to_string())?,
            }
            Ok(png.into_inner())
        }
    }

    fn low_camera(heading: f64) -> Camera {
        let ground = 130.0 * 256.0 + 92.0 - 32_768.0;
        Camera::new(&Pose { latitude: -35.36213, longitude: 149.16514, height: ground + 30.0, heading, pitch: -15.0, roll: 0.0, fov: 70.0 }, 2280, 1080)
    }

    fn turn_once_round(scene: &mut OpenScene, detail: f64, start: Instant) -> usize {
        let ground = 130.0 * 256.0 + 92.0 - 32_768.0;
        (0..180u64).fold(0, |requested, frame| {
            let now = start + Duration::from_millis(frame * 45);
            let (_, wanted) = scene.select(&low_camera(frame as f64 * 2.0), "Bing Satellite", ground, -35.36213, 149.16514, detail, 24, now);
            let terrain = wanted.iter().filter(|want| matches!(want, Want::Terrain { .. })).count();
            wanted.into_iter().map(|want| want.build(&Served)).for_each(|built| scene.deliver(built, now));
            requested + terrain
        })
    }

    #[test]
    fn a_full_turn_keeps_tile_memory_bounded_without_dropping_what_the_frame_uses() {
        let mut scene = OpenScene::new();
        let begun = Instant::now();
        turn_once_round(&mut scene, 1.0, begun);
        let last = begun + Duration::from_millis(179 * 45);
        let in_use = scene.terrain.values().filter(|entry| entry.used == last).count();
        assert!(scene.terrain.len() <= MAX_TERRAIN_TILES.max(in_use), "a 360 degree turn kept {} terrain tiles with {in_use} in use; before the cap it kept about 5000", scene.terrain.len());
        assert!(scene.heights.slots.len() <= MAX_HEIGHT_TILES, "and {} height tiles; before the cap about 1300", scene.heights.slots.len());
        let now = Instant::now() + Duration::from_secs(9);
        let ground = 130.0 * 256.0 + 92.0 - 32_768.0;
        let (drawn, _) = scene.select(&low_camera(0.0), "Bing Satellite", ground, -35.36213, 149.16514, 1.0, 24, now);
        assert!(drawn.iter().all(|mesh| scene.terrain.values().any(|entry| entry.mesh.as_ref().is_some_and(|kept| kept.id == mesh.id))), "every mesh the frame draws is still held");
    }

    #[test]
    fn a_fast_turn_asks_for_far_fewer_fine_tiles() {
        let still = turn_once_round(&mut OpenScene::new(), 1.0, Instant::now());
        let turning = turn_once_round(&mut OpenScene::new(), 3.0, Instant::now());
        assert!(turning * 2 < still, "coarser detail while turning asked for {turning} tiles against {still}");
    }

    #[test]
    fn the_cap_keeps_the_newest_and_always_the_tiles_this_frame_uses() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        assert_eq!(cutoff(vec![at(1), at(5), at(3)], 2), Some(at(3)));
        assert_eq!(cutoff(vec![at(1), at(2)], 5), None);
        assert!(kept(at(9), at(9), at(9)), "a tile used in this frame stays even when this frame alone is past the cap");
        assert!(!kept(at(4), at(4), at(9)), "an older frame's tiles at the cutoff go, so the cap is not overshot by a whole frame");
        assert!(kept(at(5), at(4), at(9)));
        assert!(awaited(Some(at(9)), at(9) + Duration::from_secs(1)), "a tile still on its way is not evicted, or the next frame asks for it again");
        assert!(!awaited(Some(at(9)), at(9) + RETRY_AFTER), "one asked so long ago it would be retried anyway can go");
        assert!(!awaited(None, at(9)));
    }

    #[test]
    fn web_mercator_tiles_cover_the_point_they_are_asked_for() {
        let (key, u, v) = TileKey::containing(-35.363262, 149.165237, 15);
        assert_eq!((key.z, key.x, key.y), (15, 29961, 19829));
        assert!((key.longitude(u) - 149.165237).abs() < 1e-9 && (key.latitude(v) - -35.363262).abs() < 1e-9);
    }

    #[test]
    fn a_deep_tile_reads_its_heights_from_the_matching_corner_of_a_terrarium_ancestor() {
        let (ancestor, du, dv, span) = TileKey { z: 17, x: 4 * 29961 + 3, y: 4 * 19806 + 1 }.ancestor(15);
        assert_eq!((ancestor.x, ancestor.y, du, dv, span), (29961, 19806, 0.75, 0.25, 0.25));
    }

    #[test]
    fn terrarium_pixels_decode_to_metres_and_a_patch_knows_its_range() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_fn(2, 2, |x, _| image::Rgb([128, x as u8, 0])).write_to(&mut png, image::ImageFormat::Png).unwrap();
        let heights = Heights::from_terrarium(&png.into_inner()).unwrap();
        assert_eq!(heights.at(0.0, 0.0), 0.0);
        assert_eq!(heights.range(0.0, 0.0, 1.0), (0.0, 1.0));
        assert!(Heights::from_terrarium(b"not a png").is_err());
    }

    #[test]
    fn the_coarse_world_draws_first_then_finer_tiles_take_over_even_without_imagery() {
        let mut scene = OpenScene::new();
        let frames = run(&mut scene, &camera(), 40, Instant::now());
        let first_drawn = frames.iter().position(|meshes| !meshes.is_empty()).expect("something draws");
        assert!(first_drawn <= 3, "a coarse tile shows within a few frames, got frame {first_drawn}");
        assert!(frames.last().unwrap().len() > frames[first_drawn].len(), "finer tiles replace the coarse one as they load");
    }

    #[test]
    fn coarse_tiles_are_asked_for_first_so_a_far_sibling_is_never_starved_by_near_detail() {
        let mut scene = OpenScene::new();
        let patch = Patch { heights: Arc::new(Heights { size: 1, metres: vec![0.0] }), zoom: 9, du: 0.0, dv: 0.0, span: 1.0 };
        let terrain = |z, x, y| Want::Terrain { key: TileKey { z, x, y }, imagery: "Bing Satellite".to_string(), patch: patch.clone() };
        let candidates = vec![terrain(17, 5, 5), Want::Heights(TileKey { z: 9, x: 1, y: 1 }), terrain(9, 1, 1), terrain(9, 1, 1)];
        let chosen: Vec<String> = scene.issue(candidates, 2, Instant::now()).iter().map(Want::describe).collect();
        assert_eq!(chosen, vec!["Bing Satellite 9/1/1", "Terrarium 9/1/1"]);
    }

    #[test]
    fn a_missing_fine_height_tile_falls_back_to_a_coarser_one_instead_of_sea_level() {
        let mut scene = OpenScene::new();
        let now = Instant::now();
        let coarse = TileKey { z: 10, x: 936, y: 619 };
        scene.heights.put(coarse, Heights::from_terrarium(&{
            let mut png = std::io::Cursor::new(Vec::new());
            image::RgbImage::from_pixel(2, 2, image::Rgb([130, 92, 0])).write_to(&mut png, image::ImageFormat::Png).unwrap();
            png.into_inner()
        }), now);
        (11..=15).for_each(|zoom| scene.heights.put(TileKey { z: zoom, x: 936 << (zoom - 10), y: 619 << (zoom - 10) }, Err("offline".into()), now));
        let patch = scene.patch(TileKey { z: 17, x: 936 << 7, y: 619 << 7 }, &mut Vec::new(), now).expect("a coarser patch");
        assert_eq!(patch.zoom, 10);
        assert_eq!(patch.at(0.0, 0.0), 130.0 * 256.0 + 92.0 - 32_768.0);
    }

    #[test]
    fn a_terrain_tile_without_imagery_still_draws_with_its_skirt() {
        let mesh = terrain_mesh(TileKey { z: 15, x: 29961, y: 19806 }, None, None);
        assert_eq!(mesh.positions.len(), (GRID + 1) * (GRID + 1) + 4 * GRID);
        assert_eq!(mesh.indices.len(), GRID * GRID * 6 + 4 * GRID * 6);
        assert!(mesh.indices.iter().all(|&i| (i as usize) < mesh.positions.len()));
    }

    #[test]
    fn flat_ground_faces_straight_up_and_a_slope_leans_downhill() {
        let flat = terrain_mesh(TileKey { z: 15, x: 29961, y: 19806 }, None, None);
        assert_eq!(flat.normals.len(), flat.positions.len(), "every vertex, skirt included, has a normal");
        assert!(flat.normals.iter().all(|n| n[2] > 0.999), "flat ground faces up");
        let rising_east = Heights { size: 2, metres: vec![0.0, 1000.0, 0.0, 1000.0] };
        let patch = Patch { heights: Arc::new(rising_east), zoom: 15, du: 0.0, dv: 0.0, span: 1.0 };
        let slope = terrain_mesh(TileKey { z: 15, x: 29961, y: 19806 }, Some(&patch), None);
        let middle = slope.normals[(GRID + 1) * (GRID / 2) + GRID / 2];
        assert!(middle[0] < -0.5 && middle[2] > 0.0 && middle[1].abs() < 1e-3, "ground rising to the east faces up and west, got {middle:?}");
    }

    #[test]
    fn a_tile_with_no_imagery_of_its_own_borrows_the_matching_corner_of_its_parent() {
        struct ParentOnly;
        impl Fetcher for ParentOnly {
            fn url(&self, _: &url::Url) -> Result<Vec<u8>, String> {
                Err("offline".into())
            }
            fn map(&self, _: &str, z: u32, _: u32, _: u32) -> Result<Vec<u8>, String> {
                let mut png = std::io::Cursor::new(Vec::new());
                let quarters = image::RgbaImage::from_fn(4, 4, |x, y| image::Rgba([if x < 2 { 10 } else { 200 }, if y < 2 { 10 } else { 200 }, 0, 255]));
                (z == 18).then_some(()).ok_or("no imagery")?;
                quarters.write_to(&mut png, image::ImageFormat::Png).map_err(|e| e.to_string())?;
                Ok(png.into_inner())
            }
        }
        let (borrowed, own) = picture(&ParentOnly, "Bing Satellite", TileKey { z: 19, x: 3, y: 2 }).expect("the parent's picture");
        assert!(!own, "a borrowed picture is retried later");
        assert_eq!(borrowed.dimensions(), (2, 2));
        assert_eq!(borrowed.get_pixel(0, 0).0, [200, 10, 0, 255], "x 3 is the right half, y 2 the top half of parent 1/1");
        assert!(picture(&ParentOnly, "Bing Satellite", TileKey { z: 22, x: 0, y: 0 }).is_none(), "nothing within three levels up");
    }

    #[test]
    fn standing_inside_a_building_footprint_puts_the_ground_on_its_roof() {
        let mut scene = OpenScene::new();
        let now = Instant::now();
        let key = TileKey { z: BUILDING_ZOOM, x: 8580, y: 5733 };
        let flat = || {
            let mut png = std::io::Cursor::new(Vec::new());
            image::RgbImage::from_pixel(2, 2, image::Rgb([130, 92, 0])).write_to(&mut png, image::ImageFormat::Png).unwrap();
            Heights::from_terrarium(&png.into_inner()).unwrap()
        };
        scene.heights.put(key, Ok(flat()), now);
        let patch = Patch { heights: Arc::new(flat()), zoom: BUILDING_ZOOM, du: 0.0, dv: 0.0, span: 1.0 };
        let json = serde_json::json!({ "extent": 4096, "buildings": [{ "rings": [[1000, 1000, 3000, 1000, 3000, 3000, 1000, 3000]], "height": 9.0 }] });
        let house = building_mesh(key, &serde_json::to_vec(&json).unwrap(), Some(&patch)).expect("a building");
        scene.houses.insert(key, HouseEntry { mesh: Some(Arc::new(house)), loaded: true, asked: None, used: now });
        let ground = |scene: &mut OpenScene, u: f64| scene.ground(key.latitude(u), key.longitude(u), 8, now).0.expect("a height");
        let roof = ground(&mut scene, 0.5);
        assert!((roof - 613.0).abs() < 0.1, "on the roof, 9 m above the 604 m street, got {roof}");
        assert!((ground(&mut scene, 0.1) - 604.0).abs() < 0.1, "beside the building, the street");
    }

    #[test]
    fn a_square_building_gets_four_walls_and_a_two_triangle_roof() {
        let json = serde_json::json!({ "extent": 4096, "buildings": [{ "rings": [[1000, 1000, 1100, 1000, 1100, 1100, 1000, 1100]], "height": 9.0, "base": 0.0 }] });
        let mesh = building_mesh(TileKey { z: 14, x: 14980, y: 9903 }, &serde_json::to_vec(&json).unwrap(), None).expect("a building");
        assert_eq!(mesh.indices.len(), 4 * 6 + 2 * 3);
        assert!((mesh.high[2] - 9.0).abs() < 0.5, "the roof sits at the building's height, got {}", mesh.high[2]);
        assert!(mesh.indices.iter().all(|&i| (i as usize) < mesh.positions.len()));
        let facing: Vec<[f32; 3]> = mesh.normals.chunks(4).take(4).map(|wall| wall[0]).collect();
        let centre = |axis: usize| (mesh.low[axis] + mesh.high[axis]) / 2.0;
        let middles: Vec<[f32; 2]> = mesh.positions.chunks(4).take(4).map(|wall| [(wall[0][0] + wall[1][0]) / 2.0 - centre(0), (wall[0][1] + wall[1][1]) / 2.0 - centre(1)]).collect();
        assert!(facing.iter().zip(&middles).all(|(n, m)| n[0] * m[0] + n[1] * m[1] > 0.0 && n[2] == 0.0), "each wall faces away from the middle of the building: {facing:?}");
        assert!(mesh.normals[16..].iter().all(|n| *n == [0.0, 0.0, 1.0]), "the roof faces up");
    }
}
