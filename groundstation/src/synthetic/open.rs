use std::collections::{BTreeMap, HashMap, HashSet};
use std::f64::consts::{FRAC_1_SQRT_2, PI, TAU};
use std::sync::Arc;
use std::time::Duration;
use web_time::Instant;

use ::geo::{Area, MinimumRotatedRect};
use glam::{DMat3, DMat4, DVec3, DVec4};

use super::fetch::Fetcher;
use super::geo::{Camera, east_north_up, ecef};
use super::ground;
use super::mesh::{Layer, Mesh, solid};
use super::tileset::{RETRY_AFTER, cutoff, frustum, kept};

pub const TERRAIN: &str = "Terrarium";
pub const BUILDINGS: &str = "Buildings";
const TERRAIN_ZOOM: u32 = 15;
const MAX_ZOOM: u32 = 20;
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
const WALL_SHADE: f32 = 0.75;
const WALL_GREY: f32 = 0.3;
const WALL_VARIETY: f32 = 0.08;
const PALETTE_WIDTH: u32 = 64;
const SAMPLED_CORNERS: usize = 15;
const ROOF_PICTURE_ZOOM: u32 = 16;
const ROOF_LIFT: f32 = 1.5;
const WALL_TAG: f32 = 2.0;
const SHOP_TAG: f32 = 0.5;
const BLANK: f32 = -1.0;
const PARAPET_M: f64 = 1.0;
const PARAPET_FROM_M: f64 = 6.0;
const OVERHANG_M: f64 = 0.4;
const HOUSE_AREA_M2: (f64, f64) = (12.0, 300.0);
const HOUSE_MAX_HEIGHT_M: f64 = 15.0;
const TILED_MAX_AREA_M2: f64 = 2500.0;
const TILED_MAX_HEIGHT_M: f64 = 30.0;
const TILE_RED_OVER_GREEN: f32 = 1.08;
const TILE_RED_OVER_BLUE: f32 = 1.25;
const MAX_RIDGE_M: f64 = 7.0;
const HOUSE_MIN_FILL: f64 = 0.85;
const GABLE_ASPECT: f64 = 1.3;
const PITCH_DEGREES: (f64, f64) = (30.0, 40.0);
const STOREY_M: (f64, f64) = (3.0, 3.5);
const BAY_M: (f64, f64) = (2.6, 3.8);
const SHOP_FROM_STOREYS: f64 = 3.0;
const SHOP_SHARE: f64 = 0.6;
const ROOFTOP_FROM_M2: f64 = 600.0;
const ROOFTOP_EVERY_M2: f64 = 500.0;
const MAX_ROOFTOPS: usize = 6;
const ROOFTOP_SIZE_M: (f64, f64) = (2.0, 4.5);
const ROOFTOP_HEIGHT_M: (f64, f64) = (1.2, 3.0);
const FACADE_TONE_WEIGHT: f32 = 0.75;
const FACADE_TONES: [[f32; 3]; 8] = [
    [226.0, 218.0, 200.0],
    [236.0, 232.0, 222.0],
    [196.0, 190.0, 178.0],
    [176.0, 172.0, 166.0],
    [204.0, 178.0, 140.0],
    [214.0, 186.0, 160.0],
    [158.0, 98.0, 74.0],
    [118.0, 122.0, 126.0],
];
const BOUNDS_BLOCK: usize = 16;
const TREE_ZOOM: u32 = 16;
const TREE_NEAR_ZOOM: u32 = 18;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Woods {
    pub reach_m: f64,
    pub split_m: [f64; 2],
    pub far_keep: f64,
}

impl Woods {
    pub const FULL: Woods = Woods {
        reach_m: 1_000.0,
        split_m: [600.0, 250.0],
        far_keep: 0.5,
    };
}
const FAR_SPREAD: f64 = 1.35;
const TREE_BRIGHTEST: f32 = 120.0;
const TREE_DARKEST: f32 = 65.0;
const CANOPY_SAMPLE_M: f64 = 4.0;
const HOME_CLEARING_M: f64 = 25.0;
const TREE_LIFT: f32 = 1.85;
const TREE_SATURATION: f32 = 0.95;
const TREE_CHROMA: f32 = 0.45;
const FOREST_GREEN: [f32; 3] = [0.8, 1.0, 0.9];
const FOREST_PULL: f32 = 0.4;
const TREE_VARIETY: f32 = 0.08;
const SHADES: [f32; 4] = [1.1, 1.0, 0.92, 0.8];
const CONIFER_TINT: [f32; 3] = [0.86, 0.95, 1.0];
const HIGHLAND_M: f64 = 800.0;
const HIGHLAND_CONIFERS: f64 = 0.1;
const BROADLEAF_SHORTER: f64 = 0.85;
const CROWN_LUMPS: f64 = 0.12;
const CROWN_BOTTOM: f64 = 0.32;
const FOLIAGE_LIFT: f64 = 0.6;
const TRUNK_RADIUS: f64 = 0.034;
const TRUNK: [u8; 4] = [74, 58, 44, 255];
const NEAR_LOBES: [(f64, f64, f64, f64); 3] = [
    (0.0, 0.0, 0.3, 0.66),
    (0.0, 0.4, -0.15, 0.64),
    (0.5, 0.4, -0.15, 0.64),
];
const FAR_LOBES: [(f64, f64, f64, f64); 1] = [(0.0, 0.0, 0.0, 1.0)];
const NEAR_TIERS: [(f64, f64, f64); 3] = [(0.08, 0.6, 1.0), (0.36, 0.82, 0.74), (0.6, 1.0, 0.48)];
const FAR_TIERS: [(f64, f64, f64); 2] = [(0.08, 0.72, 1.0), (0.45, 1.0, 0.62)];
const NEAR_SIDES: u32 = 7;
const FAR_SIDES: u32 = 5;
const FOREST: Stand = Stand {
    spacing_m: 7.0,
    heights_m: (18.0, 30.0),
    conifers: 0.3,
    brightest: 140.0,
};
const PARK: Stand = Stand {
    spacing_m: 22.0,
    heights_m: (10.0, 18.0),
    conifers: 0.2,
    brightest: 105.0,
};
const GOLDEN: f64 = 1.618_033_988_749_895;
const ICOSAHEDRON: [[f64; 3]; 12] = [
    [-1.0, GOLDEN, 0.0],
    [1.0, GOLDEN, 0.0],
    [-1.0, -GOLDEN, 0.0],
    [1.0, -GOLDEN, 0.0],
    [0.0, -1.0, GOLDEN],
    [0.0, 1.0, GOLDEN],
    [0.0, -1.0, -GOLDEN],
    [0.0, 1.0, -GOLDEN],
    [GOLDEN, 0.0, -1.0],
    [GOLDEN, 0.0, 1.0],
    [-GOLDEN, 0.0, -1.0],
    [-GOLDEN, 0.0, 1.0],
];
const ICOSAHEDRON_FACES: [[u32; 3]; 20] = [
    [0, 11, 5],
    [0, 5, 1],
    [0, 1, 7],
    [0, 7, 10],
    [0, 10, 11],
    [1, 5, 9],
    [5, 11, 4],
    [11, 10, 2],
    [10, 7, 6],
    [7, 1, 8],
    [3, 9, 4],
    [3, 4, 2],
    [3, 2, 6],
    [3, 6, 8],
    [3, 8, 9],
    [4, 9, 5],
    [2, 4, 11],
    [6, 2, 10],
    [8, 6, 7],
    [9, 8, 1],
];
const WATER: [u8; 4] = [52, 78, 96, 255];
const WATER_LIFT_M: f64 = 1.0;
const WATER_SAMPLES: u32 = 12;
const NO_HEIGHTS: (f32, f32) = (f32::MAX, f32::MIN);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TileKey {
    pub z: u32,
    pub x: u32,
    pub y: u32,
}

impl TileKey {
    fn children(self) -> [TileKey; 4] {
        let (z, x, y) = (self.z + 1, self.x * 2, self.y * 2);
        [
            TileKey { z, x, y },
            TileKey { z, x: x + 1, y },
            TileKey { z, x, y: y + 1 },
            TileKey {
                z,
                x: x + 1,
                y: y + 1,
            },
        ]
    }

    fn ancestor(self, zoom: u32) -> (TileKey, f64, f64, f64) {
        let up = self.z.saturating_sub(zoom);
        let scale = f64::from(1u32 << up);
        let key = TileKey {
            z: self.z - up,
            x: self.x >> up,
            y: self.y >> up,
        };
        (
            key,
            f64::from(self.x) / scale - f64::from(key.x),
            f64::from(self.y) / scale - f64::from(key.y),
            1.0 / scale,
        )
    }

    fn longitude(self, u: f64) -> f64 {
        (f64::from(self.x) + u) / f64::from(1u32 << self.z) * 360.0 - 180.0
    }

    fn latitude(self, v: f64) -> f64 {
        (PI * (1.0 - 2.0 * (f64::from(self.y) + v) / f64::from(1u32 << self.z)))
            .sinh()
            .atan()
            .to_degrees()
    }

    fn width_m(self) -> f64 {
        2.0 * PI * 6_378_137.0 * self.latitude(0.5).to_radians().cos() / f64::from(1u32 << self.z)
    }

    pub fn containing(latitude: f64, longitude: f64, z: u32) -> (TileKey, f64, f64) {
        let n = f64::from(1u32 << z);
        let x = (longitude + 180.0) / 360.0 * n;
        let y = (1.0 - latitude.to_radians().tan().asinh() / PI) / 2.0 * n;
        let (tx, ty) = (x.floor().clamp(0.0, n - 1.0), y.floor().clamp(0.0, n - 1.0));
        (
            TileKey {
                z,
                x: tx as u32,
                y: ty as u32,
            },
            x - tx,
            y - ty,
        )
    }
}

pub struct Heights {
    size: usize,
    metres: Vec<f32>,
    bounds: Vec<Bounds>,
}

struct Bounds {
    cells: usize,
    low_high: Vec<(f32, f32)>,
}

fn widened(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    (a.0.min(b.0), a.1.max(b.1))
}

impl Bounds {
    fn of_pixels(size: usize, metres: &[f32]) -> Self {
        let cells = size.div_ceil(BOUNDS_BLOCK);
        let pixels = |cell: usize| cell * BOUNDS_BLOCK..((cell + 1) * BOUNDS_BLOCK).min(size);
        let low_high = (0..cells * cells)
            .map(|cell| {
                pixels(cell / cells)
                    .flat_map(|y| pixels(cell % cells).map(move |x| metres[y * size + x]))
                    .fold(NO_HEIGHTS, |bounds, h| widened(bounds, (h, h)))
            })
            .collect();
        Self { cells, low_high }
    }

    fn halved(&self) -> Self {
        let cells = self.cells.div_ceil(2);
        let low_high = (0..cells * cells)
            .map(|cell| {
                let (cx, cy) = (cell % cells * 2, cell / cells * 2);
                [(cx, cy), (cx + 1, cy), (cx, cy + 1), (cx + 1, cy + 1)]
                    .into_iter()
                    .filter(|(x, y)| *x < self.cells && *y < self.cells)
                    .map(|(x, y)| self.low_high[y * self.cells + x])
                    .fold(NO_HEIGHTS, widened)
            })
            .collect();
        Self { cells, low_high }
    }
}

impl Heights {
    fn new(size: usize, metres: Vec<f32>) -> Self {
        let bounds = std::iter::successors(Some(Bounds::of_pixels(size, &metres)), |level| {
            (level.cells > 1).then(|| level.halved())
        })
        .collect();
        Self {
            size,
            metres,
            bounds,
        }
    }

    pub fn from_terrarium(png: &[u8]) -> Result<Self, String> {
        let image = image::load_from_memory(png)
            .map_err(|error| format!("terrarium tile: {error}"))?
            .to_rgb8();
        let size = image.width() as usize;
        if image.height() as usize != size || size < 2 {
            return Err(format!(
                "terrarium tile is {}x{}, not square",
                image.width(),
                image.height()
            ));
        }
        let metres = image
            .pixels()
            .map(|p| f32::from(p[0]) * 256.0 + f32::from(p[1]) + f32::from(p[2]) / 256.0 - 32_768.0)
            .collect();
        Ok(Self::new(size, metres))
    }

    pub fn at(&self, u: f64, v: f64) -> f64 {
        let last = (self.size - 1) as f64;
        let (fx, fy) = ((u * last).clamp(0.0, last), (v * last).clamp(0.0, last));
        let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.size - 1), (y0 + 1).min(self.size - 1));
        let (ax, ay) = (fx - x0 as f64, fy - y0 as f64);
        let h = |x: usize, y: usize| f64::from(self.metres[y * self.size + x]);
        (h(x0, y0) * (1.0 - ax) + h(x1, y0) * ax) * (1.0 - ay)
            + (h(x0, y1) * (1.0 - ax) + h(x1, y1) * ax) * ay
    }

    fn range(&self, du: f64, dv: f64, span: f64) -> (f64, f64) {
        let last = (self.size - 1) as f64;
        let pixel = |t: f64| ((t * last).round() as usize).min(self.size - 1);
        let (x0, x1, y0, y1) = (pixel(du), pixel(du + span), pixel(dv), pixel(dv + span));
        let wide = (x1 - x0).max(y1 - y0) + 1;
        let (low, high) = match self
            .bounds
            .iter()
            .enumerate()
            .rev()
            .find(|(level, _)| BOUNDS_BLOCK << level <= wide / 2)
        {
            Some((level, bounds)) => {
                let cell = BOUNDS_BLOCK << level;
                (y0 / cell..=y1 / cell)
                    .flat_map(|cy| {
                        (x0 / cell..=x1 / cell)
                            .map(move |cx| bounds.low_high[cy * bounds.cells + cx])
                    })
                    .fold(NO_HEIGHTS, widened)
            }
            None => (y0..=y1)
                .flat_map(|y| (x0..=x1).map(move |x| self.metres[y * self.size + x]))
                .fold(NO_HEIGHTS, |bounds, h| widened(bounds, (h, h))),
        };
        (f64::from(low), f64::from(high))
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
        self.heights
            .at(self.du + u * self.span, self.dv + v * self.span)
    }
}

#[derive(Clone)]
pub enum Want {
    Heights(TileKey),
    Terrain {
        key: TileKey,
        imagery: String,
        patch: Patch,
    },
    Houses {
        key: TileKey,
        imagery: String,
        patch: Patch,
    },
    Trees {
        key: TileKey,
        imagery: String,
        patch: Patch,
        groves: Arc<Vec<Grove>>,
        home: Option<(f64, f64)>,
        far_keep: f64,
    },
}

pub enum Built {
    Heights(TileKey, Result<Heights, String>),
    Terrain {
        key: TileKey,
        imagery: String,
        zoom: u32,
        mesh: Result<(Mesh, bool), String>,
    },
    Houses(TileKey, Result<(Vec<Mesh>, Vec<Grove>), String>),
    Trees(TileKey, Result<Vec<Mesh>, String>),
}

impl Want {
    pub fn failed(&self, reason: &str) -> Built {
        match self {
            Want::Heights(key) => Built::Heights(*key, Err(reason.to_string())),
            Want::Terrain {
                key,
                imagery,
                patch,
            } => Built::Terrain {
                key: *key,
                imagery: imagery.clone(),
                zoom: patch.zoom,
                mesh: Err(reason.to_string()),
            },
            Want::Houses { key, .. } => Built::Houses(*key, Err(reason.to_string())),
            Want::Trees { key, .. } => Built::Trees(*key, Err(reason.to_string())),
        }
    }

    pub fn build(self, fetcher: &dyn Fetcher) -> Built {
        match self {
            Want::Heights(key) => Built::Heights(
                key,
                fetcher
                    .map(TERRAIN, key.z, key.x, key.y)
                    .and_then(|png| Heights::from_terrarium(&png)),
            ),
            Want::Terrain {
                key,
                imagery,
                patch,
            } => {
                let found = picture(fetcher, &imagery, key);
                let imaged = found.as_ref().is_some_and(|(_, own)| *own);
                Built::Terrain {
                    key,
                    imagery,
                    zoom: patch.zoom,
                    mesh: Ok((
                        terrain_mesh(key, Some(&patch), found.map(|(picture, _)| picture)),
                        imaged,
                    )),
                }
            }
            Want::Houses {
                key,
                imagery,
                patch,
            } => Built::Houses(
                key,
                fetcher.map(BUILDINGS, key.z, key.x, key.y).map(|json| {
                    place_meshes(key, &json, Some(&patch), &|under| {
                        picture(fetcher, &imagery, under).map(|(picture, _)| picture)
                    })
                }),
            ),
            Want::Trees {
                key,
                imagery,
                patch,
                groves,
                home,
                far_keep,
            } => Built::Trees(
                key,
                picture(fetcher, &imagery, key)
                    .map(|(picture, _)| {
                        tree_mesh(key, &groves, &patch, &picture, home, far_keep)
                            .into_iter()
                            .collect()
                    })
                    .ok_or_else(|| "no imagery".to_string()),
            ),
        }
    }

    fn priority(&self) -> (u32, u8, u32, u32) {
        match self {
            Want::Terrain { key, .. } => (key.z, 0, key.x, key.y),
            Want::Heights(key) => (key.z, 1, key.x, key.y),
            Want::Houses { key, .. } => (key.z, 2, key.x, key.y),
            Want::Trees { key, .. } => (
                BUILDING_ZOOM,
                3 + (TREE_NEAR_ZOOM - key.z) as u8,
                key.x,
                key.y,
            ),
        }
    }

    pub fn describe(&self) -> String {
        let (kind, key) = match self {
            Want::Heights(key) => (TERRAIN.to_string(), key),
            Want::Terrain { key, imagery, .. } => (imagery.clone(), key),
            Want::Houses { key, .. } => (BUILDINGS.to_string(), key),
            Want::Trees { key, .. } => ("Trees".to_string(), key),
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
        Self {
            slots: HashMap::new(),
        }
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
        self.slots.insert(
            key,
            (
                value.map_or(Slot::Failed(now), |value| Slot::Have(Arc::new(value))),
                now,
            ),
        );
    }

    fn trim(&mut self, now: Instant, keep: Duration, most: usize) {
        self.slots.retain(|_, (slot, used)| {
            matches!(slot, Slot::Asked) || now.duration_since(*used) < keep
        });
        let oldest = cutoff(self.slots.values().map(|(_, used)| *used).collect(), most);
        oldest.into_iter().for_each(|at| {
            self.slots
                .retain(|_, (slot, used)| matches!(slot, Slot::Asked) || kept(*used, at, now))
        });
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
    meshes: Vec<Arc<Mesh>>,
    groves: Arc<Vec<Grove>>,
    loaded: bool,
    asked: Option<Instant>,
    used: Instant,
}

impl HouseEntry {
    fn empty(asked: Option<Instant>, now: Instant) -> Self {
        Self {
            meshes: Vec::new(),
            groves: Arc::default(),
            loaded: false,
            asked,
            used: now,
        }
    }

    fn loaded(meshes: Vec<Mesh>, groves: Vec<Grove>, now: Instant) -> Self {
        Self {
            meshes: meshes.into_iter().map(Arc::new).collect(),
            groves: Arc::new(groves),
            loaded: true,
            asked: None,
            used: now,
        }
    }

    fn ask(entries: &mut HashMap<TileKey, HouseEntry>, key: TileKey, now: Instant) {
        entries
            .entry(key)
            .and_modify(|entry| entry.asked = Some(now))
            .or_insert_with(|| HouseEntry::empty(Some(now), now))
            .used = now;
    }
}

struct Sight<'a> {
    camera: &'a Camera,
    planes: [DVec4; 5],
    ground: f64,
    detail: f64,
}

pub struct OpenScene {
    woods: Woods,
    imagery: String,
    heights: Store<Heights>,
    terrain: HashMap<TileKey, TerrainEntry>,
    houses: HashMap<TileKey, HouseEntry>,
    trees: HashMap<TileKey, HouseEntry>,
    home: Option<(f64, f64)>,
}

fn frame_at(latitude: f64, longitude: f64) -> (DMat4, DMat3, DVec3) {
    let enu = east_north_up(latitude, longitude);
    let origin = ecef(latitude, longitude, 0.0);
    (
        DMat4::from_mat3_translation(enu, origin),
        enu.transpose(),
        origin,
    )
}

fn picture(fetcher: &dyn Fetcher, imagery: &str, key: TileKey) -> Option<(image::RgbaImage, bool)> {
    (0..=BORROWED_LEVELS.min(key.z)).find_map(|up| {
        let (source, du, dv, span) = key.ancestor(key.z - up);
        let whole =
            image::load_from_memory(&fetcher.map(imagery, source.z, source.x, source.y).ok()?)
                .ok()?
                .to_rgba8();
        let (width, height) = whole.dimensions();
        let pixels = |t: f64, size: u32| (t * f64::from(size)).round() as u32;
        let part = image::imageops::crop_imm(
            &whole,
            pixels(du, width),
            pixels(dv, height),
            pixels(span, width).max(1),
            pixels(span, height).max(1),
        )
        .to_image();
        Some((part, up == 0))
    })
}

fn terrain_mesh(key: TileKey, patch: Option<&Patch>, picture: Option<image::RgbaImage>) -> Mesh {
    let height_at = |u: f64, v: f64| patch.map_or(0.0, |patch| patch.at(u, v));
    let (to_ecef, to_local, origin) = frame_at(key.latitude(0.5), key.longitude(0.5));
    let local = |u: f64, v: f64, height: f64| -> [f32; 3] {
        (to_local * (ecef(key.latitude(v), key.longitude(u), height) - origin))
            .as_vec3()
            .to_array()
    };
    let skirt = key.width_m() / GRID as f64;
    let steps = GRID + 1;
    let at = |index: usize| {
        (
            (index % steps) as f64 / GRID as f64,
            (index / steps) as f64 / GRID as f64,
        )
    };
    let grid: Vec<([f32; 3], [f32; 2])> = (0..steps * steps)
        .map(at)
        .map(|(u, v)| (local(u, v, height_at(u, v)), [u as f32, v as f32]))
        .collect();
    let edge: Vec<usize> = (0..GRID)
        .chain((0..GRID).map(|i| i * steps + GRID))
        .chain((0..GRID).map(|i| GRID * steps + GRID - i))
        .chain((0..GRID).map(|i| (GRID - i) * steps))
        .collect();
    let lowered = edge.iter().map(|&index| {
        let (u, v) = at(index);
        (local(u, v, height_at(u, v) - skirt), grid[index].1)
    });
    let cells = (0..GRID)
        .flat_map(|row| (0..GRID).map(move |column| row * steps + column))
        .flat_map(|i| [i, i + 1, i + steps, i + 1, i + steps + 1, i + steps]);
    let ring = edge.len();
    let skirts = (0..ring).flat_map(|i| {
        let (top_a, top_b) = (edge[i], edge[(i + 1) % ring]);
        let (low_a, low_b) = (grid.len() + i, grid.len() + (i + 1) % ring);
        [top_a, top_b, low_a, top_b, low_b, low_a]
    });
    let indices: Vec<u32> = cells.chain(skirts).map(|i| i as u32).collect();
    let (positions, uvs): (Vec<[f32; 3]>, Vec<[f32; 2]>) =
        grid.iter().copied().chain(lowered).unzip();
    let step = 1.0 / GRID as f64;
    let point = |u: f64, v: f64| DVec3::from_array(local(u, v, height_at(u, v)).map(f64::from));
    let upward: Vec<[f32; 3]> = (0..steps * steps)
        .map(at)
        .map(|(u, v)| {
            (point(u, v + step) - point(u, v - step))
                .cross(point(u + step, v) - point(u - step, v))
                .normalize_or_zero()
                .as_vec3()
                .to_array()
        })
        .collect();
    let normals = upward
        .iter()
        .copied()
        .chain(edge.iter().map(|&index| upward[index]))
        .collect();
    Mesh::new(
        to_ecef,
        positions,
        uvs,
        indices,
        picture.unwrap_or_else(|| solid(UNIMAGED)),
        Layer::Opaque,
    )
    .with_normals(normals)
}

fn palette(colours: &[[u8; 4]]) -> image::RgbaImage {
    let rows = (colours.len() as u32).div_ceil(PALETTE_WIDTH).max(1);
    image::RgbaImage::from_fn(PALETTE_WIDTH, rows, |x, y| {
        image::Rgba(
            colours
                .get((y * PALETTE_WIDTH + x) as usize)
                .copied()
                .unwrap_or(ROOF),
        )
    })
}

fn palette_uv(index: usize, count: usize) -> [f32; 2] {
    let rows = (count as u32).div_ceil(PALETTE_WIDTH).max(1);
    [
        (index as u32 % PALETTE_WIDTH) as f32 / PALETTE_WIDTH as f32 + 0.5 / PALETTE_WIDTH as f32,
        (index as u32 / PALETTE_WIDTH) as f32 / rows as f32 + 0.5 / rows as f32,
    ]
}

fn centre_of(outline: &[(f64, f64)]) -> (f64, f64) {
    let count = outline.len() as f64;
    outline
        .iter()
        .fold((0.0, 0.0), |(u, v), p| (u + p.0 / count, v + p.1 / count))
}

fn under(key: TileKey, outline: &[(f64, f64)]) -> (TileKey, f64, f64) {
    under_at(key, centre_of(outline))
}

fn under_at(key: TileKey, (u, v): (f64, f64)) -> (TileKey, f64, f64) {
    let scale = f64::from(1u32 << ROOF_PICTURE_ZOOM.saturating_sub(key.z));
    let (du, dv) = (
        (u * scale).floor().clamp(0.0, scale - 1.0),
        (v * scale).floor().clamp(0.0, scale - 1.0),
    );
    (
        TileKey {
            z: key.z.max(ROOF_PICTURE_ZOOM),
            x: key.x * scale as u32 + du as u32,
            y: key.y * scale as u32 + dv as u32,
        },
        du,
        dv,
    )
}

fn roof_colour(
    key: TileKey,
    pictures: &HashMap<TileKey, Option<image::RgbaImage>>,
    outline: &[(f64, f64)],
    fallback: [u8; 4],
    lift: f32,
) -> [u8; 4] {
    let (sub, du, dv) = under(key, outline);
    let Some(Some(picture)) = pictures.get(&sub) else {
        return fallback;
    };
    let scale = f64::from(1u32 << (sub.z - key.z));
    let (width, height) = picture.dimensions();
    let samples: Vec<[u8; 4]> = std::iter::once(centre_of(outline))
        .chain(outline.iter().copied().take(SAMPLED_CORNERS))
        .map(|(u, v)| {
            let x = (((u * scale - du).clamp(0.0, 1.0) * f64::from(width)) as u32).min(width - 1);
            let y = (((v * scale - dv).clamp(0.0, 1.0) * f64::from(height)) as u32).min(height - 1);
            picture.get_pixel(x, y).0
        })
        .collect();
    std::array::from_fn(|channel| {
        let mean =
            samples.iter().map(|s| f32::from(s[channel])).sum::<f32>() / samples.len() as f32;
        (if channel == 3 { mean } else { mean * lift }).min(255.0) as u8
    })
}

fn wall_colour(roof: [u8; 4], outline: &[(f64, f64)]) -> [u8; 4] {
    let seed = seed_of(outline);
    let variety = 1.0 + WALL_VARIETY * ((seed % 1000) as f32 / 500.0 - 1.0);
    let grey = (f32::from(roof[0]) + f32::from(roof[1]) + f32::from(roof[2])) / 3.0;
    let shade = |channel: u8| {
        ((f32::from(channel) * (1.0 - WALL_GREY) + grey * WALL_GREY) * WALL_SHADE * variety)
            .clamp(0.0, 255.0) as u8
    };
    [shade(roof[0]), shade(roof[1]), shade(roof[2]), 255]
}

fn osm_colour(text: &str) -> Option<[u8; 4]> {
    let digits: Vec<u8> = text
        .strip_prefix('#')?
        .chars()
        .map(|c| c.to_digit(16).map(|d| d as u8))
        .collect::<Option<_>>()?;
    let channels: Vec<u8> = match digits.len() {
        3 => digits.iter().map(|d| d * 17).collect(),
        6 => digits
            .chunks(2)
            .map(|pair| pair[0] * 16 + pair[1])
            .collect(),
        _ => return None,
    };
    Some([channels[0], channels[1], channels[2], 255])
}

fn seed_of(outline: &[(f64, f64)]) -> u64 {
    outline[0].0.to_bits() ^ outline[0].1.to_bits().rotate_left(17)
}

fn span((low, high): (f64, f64), seed: u64) -> f64 {
    low + (high - low) * unit(seed)
}

fn facade_colour(osm: Option<[u8; 4]>, imaged: [u8; 4], seed: u64) -> [u8; 4] {
    osm.unwrap_or_else(|| {
        let tone = FACADE_TONES[(mixed(seed, 11) % FACADE_TONES.len() as u64) as usize];
        let variety = 1.0 + WALL_VARIETY * (unit(mixed(seed, 12)) as f32 * 2.0 - 1.0);
        let [r, g, b]: [u8; 3] = std::array::from_fn(|channel| {
            ((tone[channel] * FACADE_TONE_WEIGHT
                + f32::from(imaged[channel]) * (1.0 - FACADE_TONE_WEIGHT))
                * variety)
                .clamp(0.0, 255.0) as u8
        });
        [r, g, b, 255]
    })
}

#[derive(Default)]
struct Footprint {
    rings: Vec<Vec<(f64, f64)>>,
    base: f64,
    top: f64,
    colour: Option<[u8; 4]>,
}

#[derive(Clone, Copy, Debug)]
struct Rectangle {
    centre: DVec3,
    along: DVec3,
    across: DVec3,
    length: f64,
    width: f64,
}

impl Rectangle {
    fn at(&self, along: f64, across: f64, height: f64) -> [f32; 3] {
        (self.centre + self.along * along + self.across * across + DVec3::Z * height)
            .as_vec3()
            .to_array()
    }
}

#[derive(Debug)]
enum Roof {
    Flat,
    Gable(Rectangle),
    Hip(Rectangle),
}

struct Facade {
    base: f64,
    storey: f64,
    bay: f64,
    tag: f32,
}

fn polygon_of(placed: &[Vec<DVec3>]) -> ::geo::Polygon<f64> {
    let line = |ring: &Vec<DVec3>| {
        ::geo::LineString::from(ring.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>())
    };
    ::geo::Polygon::new(line(&placed[0]), placed[1..].iter().map(line).collect())
}

fn bounding(outline: &[DVec3]) -> Option<Rectangle> {
    let corners: Vec<DVec3> = polygon_of(&[outline.to_vec()])
        .minimum_rotated_rect()?
        .exterior()
        .points()
        .take(4)
        .map(|p| DVec3::new(p.x(), p.y(), 0.0))
        .collect();
    let [a, b, c, _] = corners[..] else {
        return None;
    };
    let (first, second) = (b - a, c - b);
    let long = if first.length() >= second.length() {
        first
    } else {
        second
    };
    let along = long.normalize_or_zero();
    Some(Rectangle {
        centre: (a + c) / 2.0,
        along,
        across: DVec3::new(-along.y, along.x, 0.0),
        length: first.length().max(second.length()),
        width: first.length().min(second.length()),
    })
    .filter(|rectangle| rectangle.width > 1.0)
}

fn tiled(roof: [u8; 4]) -> bool {
    let [r, g, b, _] = roof.map(f32::from);
    r >= g * TILE_RED_OVER_GREEN && r >= b * TILE_RED_OVER_BLUE
}

fn roof_for(placed: &[Vec<DVec3>], base: f64, top: f64, colour: [u8; 4]) -> Roof {
    let area = polygon_of(placed).unsigned_area();
    let solid = placed.len() == 1 && base < 0.5;
    let house =
        top - base <= HOUSE_MAX_HEIGHT_M && (HOUSE_AREA_M2.0..=HOUSE_AREA_M2.1).contains(&area);
    let block = tiled(colour) && top - base <= TILED_MAX_HEIGHT_M && area <= TILED_MAX_AREA_M2;
    match (solid && (house || block))
        .then(|| bounding(&placed[0]))
        .flatten()
        .filter(|rectangle| area / (rectangle.length * rectangle.width) >= HOUSE_MIN_FILL)
    {
        Some(rectangle) if rectangle.length >= rectangle.width * GABLE_ASPECT => {
            Roof::Gable(rectangle)
        }
        Some(rectangle) => Roof::Hip(rectangle),
        None => Roof::Flat,
    }
}

struct Part {
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

fn joined(parts: impl Iterator<Item = Part>) -> Part {
    let parts: Vec<Part> = parts.collect();
    let offsets = parts.iter().scan(0u32, |at, part| {
        let start = *at;
        *at += part.positions.len() as u32;
        Some(start)
    });
    Part {
        positions: parts
            .iter()
            .flat_map(|part| part.positions.iter().copied())
            .collect(),
        uvs: parts
            .iter()
            .flat_map(|part| part.uvs.iter().copied())
            .collect(),
        normals: parts
            .iter()
            .flat_map(|part| part.normals.iter().copied())
            .collect(),
        indices: parts
            .iter()
            .zip(offsets)
            .flat_map(|(part, offset)| part.indices.iter().map(move |index| index + offset))
            .collect(),
    }
}

fn wall(a: DVec3, b: DVec3, low: f64, high: f64, uv: [f32; 2]) -> Part {
    let up = |p: DVec3, h: f64| (p + DVec3::Z * h).as_vec3().to_array();
    let along = b - a;
    let outward = DVec3::new(-along.y, along.x, 0.0)
        .normalize_or_zero()
        .as_vec3()
        .to_array();
    Part {
        positions: vec![up(a, low), up(b, low), up(b, high), up(a, high)],
        uvs: vec![uv; 4],
        normals: vec![outward; 4],
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}

fn roof(rings: &[Vec<DVec3>], height: f64, uv: [f32; 2]) -> Part {
    let flat: Vec<[f64; 2]> = rings.iter().flatten().map(|p| [p.x, p.y]).collect();
    let holes: Vec<u32> = rings
        .iter()
        .scan(0u32, |at, ring| {
            let start = *at;
            *at += ring.len() as u32;
            Some(start)
        })
        .skip(1)
        .collect();
    let mut triangles: Vec<u32> = Vec::new();
    earcut::Earcut::<f64>::new().earcut(flat.iter().copied(), &holes, &mut triangles);
    Part {
        positions: rings
            .iter()
            .flatten()
            .map(|p| (*p + DVec3::Z * height).as_vec3().to_array())
            .collect(),
        uvs: vec![uv; flat.len()],
        normals: vec![[0.0, 0.0, 1.0]; flat.len()],
        indices: triangles,
    }
}

fn facade_wall(a: DVec3, b: DVec3, low: f64, high: f64, facade: &Facade) -> Part {
    let up = |p: DVec3, h: f64| (p + DVec3::Z * h).as_vec3().to_array();
    let along = b - a;
    let bays = (along.length() / facade.bay).round();
    let across = |at: f64| {
        if bays >= 1.0 {
            (at * bays) as f32
        } else {
            BLANK
        }
    };
    let storeys = |h: f64| ((h - facade.base) / facade.storey) as f32;
    let outward = DVec3::new(-along.y, along.x, 0.0).normalize_or_zero();
    Part {
        positions: vec![up(a, low), up(b, low), up(b, high), up(a, high)],
        uvs: vec![
            [across(0.0), storeys(low)],
            [across(1.0), storeys(low)],
            [across(1.0), storeys(high)],
            [across(0.0), storeys(high)],
        ],
        normals: vec![[outward.x as f32, outward.y as f32, facade.tag]; 4],
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}

fn slab(corners: &[[f32; 3]], normal: DVec3, uv: [f32; 2]) -> Part {
    Part {
        positions: corners.to_vec(),
        uvs: vec![uv; corners.len()],
        normals: vec![normal.as_vec3().to_array(); corners.len()],
        indices: (1..corners.len() as u32 - 1)
            .flat_map(|i| [0, i, i + 1])
            .collect(),
    }
}

fn pitched(
    rectangle: &Rectangle,
    eave: f64,
    pitch: f64,
    hipped: bool,
    uv: [f32; 2],
    tag: f32,
) -> Part {
    let pitch = pitch.min((MAX_RIDGE_M * 2.0 / rectangle.width).atan());
    let rise = pitch.tan();
    let ridge = eave + rise * rectangle.width / 2.0;
    let low = eave - rise * OVERHANG_M;
    let (long, wide) = (
        rectangle.length / 2.0 + OVERHANG_M,
        rectangle.width / 2.0 + OVERHANG_M,
    );
    let inset = if hipped {
        (rectangle.length - rectangle.width) / 2.0
    } else {
        long
    };
    let sloped = |toward: DVec3| toward * pitch.sin() + DVec3::Z * pitch.cos();
    let at = |along: f64, across: f64, height: f64| rectangle.at(along, across, height);
    let sides = [1.0, -1.0].map(|s| {
        slab(
            &[
                at(-s * long, s * wide, low),
                at(s * long, s * wide, low),
                at(s * inset, 0.0, ridge),
                at(-s * inset, 0.0, ridge),
            ],
            sloped(rectangle.across * s),
            uv,
        )
    });
    let ends = [1.0, -1.0].map(|s| {
        if hipped {
            slab(
                &[
                    at(s * long, s * wide, low),
                    at(s * long, -s * wide, low),
                    at(s * inset, 0.0, ridge),
                ],
                sloped(rectangle.along * s),
                uv,
            )
        } else {
            let (end, half) = (s * rectangle.length / 2.0, rectangle.width / 2.0);
            let outward = rectangle.along * s;
            Part {
                normals: vec![[outward.x as f32, outward.y as f32, tag]; 3],
                uvs: vec![[BLANK, 0.0]; 3],
                ..slab(
                    &[
                        at(end, s * half, eave),
                        at(end, -s * half, eave),
                        at(end, 0.0, ridge),
                    ],
                    outward,
                    uv,
                )
            }
        }
    });
    joined(sides.into_iter().chain(ends))
}

fn rooftops(placed: &[Vec<DVec3>], level: f64, seed: u64, uv: [f32; 2]) -> Vec<Part> {
    let area = polygon_of(placed).unsigned_area();
    let count = ((area / ROOFTOP_EVERY_M2) as usize).min(MAX_ROOFTOPS);
    let Some(rectangle) = bounding(&placed[0]).filter(|_| area >= ROOFTOP_FROM_M2) else {
        return Vec::new();
    };
    let rings: Vec<Vec<(f64, f64)>> = placed
        .iter()
        .map(|ring| ring.iter().map(|p| (p.x, p.y)).collect())
        .collect();
    (0..(MAX_ROOFTOPS * 4) as u64)
        .map(|candidate| {
            let pick = |salt: u64| unit(mixed(seed, candidate * 8 + salt));
            let half = span(ROOFTOP_SIZE_M, mixed(seed, candidate * 8 + 3)) / 2.0;
            let centre = rectangle.centre
                + rectangle.along * (pick(1) - 0.5) * (rectangle.length - 2.0 * half).max(0.0)
                + rectangle.across * (pick(2) - 0.5) * (rectangle.width - 2.0 * half).max(0.0);
            let corners: Vec<DVec3> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .iter()
                .map(|(a, b)| centre + rectangle.along * a * half + rectangle.across * b * half)
                .collect();
            (
                corners,
                span(ROOFTOP_HEIGHT_M, mixed(seed, candidate * 8 + 4)),
            )
        })
        .filter(|(corners, _)| corners.iter().all(|p| within(&rings, (p.x, p.y))))
        .take(count)
        .map(|(corners, height)| {
            joined(
                (0..4)
                    .map(|i| wall(corners[i], corners[(i + 1) % 4], level, level + height, uv))
                    .chain(std::iter::once(roof(
                        std::slice::from_ref(&corners),
                        level + height,
                        uv,
                    ))),
            )
        })
        .collect()
}

fn house(
    placed: &[Vec<DVec3>],
    floor: f64,
    footprint: &Footprint,
    roof_colour: [u8; 4],
    uv: [f32; 2],
    wall: usize,
) -> Part {
    let seed = seed_of(&footprint.rings[0]);
    let (low, eave) = (floor + footprint.base, floor + footprint.top);
    let shape = roof_for(placed, footprint.base, footprint.top, roof_colour);
    let parapet = if matches!(shape, Roof::Flat) && footprint.top - footprint.base >= PARAPET_FROM_M
    {
        PARAPET_M
    } else {
        0.0
    };
    let walled = footprint.top - footprint.base - parapet;
    let storeys = (walled / span(STOREY_M, mixed(seed, 1))).round().max(1.0);
    let shop = storeys >= SHOP_FROM_STOREYS && unit(mixed(seed, 2)) < SHOP_SHARE;
    let tag = WALL_TAG + wall as f32 + if shop { SHOP_TAG } else { 0.0 };
    let facade = Facade {
        base: low,
        storey: walled / storeys,
        bay: span(BAY_M, mixed(seed, 3)),
        tag,
    };
    let walls = placed.iter().flat_map(|ring| {
        let facade = &facade;
        (0..ring.len())
            .map(move |i| facade_wall(ring[i], ring[(i + 1) % ring.len()], low, eave, facade))
    });
    let pitch = span(PITCH_DEGREES, mixed(seed, 4)).to_radians();
    let top = match shape {
        Roof::Flat => joined(
            std::iter::once(roof(placed, eave - parapet, uv)).chain(rooftops(
                placed,
                eave - parapet,
                seed,
                uv,
            )),
        ),
        Roof::Gable(rectangle) => pitched(&rectangle, eave, pitch, false, uv, tag),
        Roof::Hip(rectangle) => pitched(&rectangle, eave, pitch, true, uv, tag),
    };
    joined(walls.chain(std::iter::once(top)))
}

fn extent_of(tile: &serde_json::Value) -> f64 {
    tile.get("extent")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(DEFAULT_EXTENT)
}

fn rings_of(rings: &serde_json::Value, extent: f64) -> Vec<Vec<(f64, f64)>> {
    rings
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_array)
        .map(|flat| {
            flat.chunks_exact(2)
                .filter_map(|p| Some((p[0].as_f64()? / extent, p[1].as_f64()? / extent)))
                .collect()
        })
        .collect()
}

fn areas_of(tile: &serde_json::Value, name: &str) -> Vec<Vec<Vec<(f64, f64)>>> {
    let extent = extent_of(tile);
    tile.get(name)
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .map(|area| rings_of(area, extent))
        .filter(|rings| rings.first().is_some_and(|ring| ring.len() >= 3))
        .collect()
}

struct Placement {
    to_ecef: DMat4,
    to_local: DMat3,
    origin: DVec3,
    key: TileKey,
}

impl Placement {
    fn new(key: TileKey) -> Self {
        let (to_ecef, to_local, origin) = frame_at(key.latitude(0.5), key.longitude(0.5));
        Self {
            to_ecef,
            to_local,
            origin,
            key,
        }
    }

    fn flat_at(&self, u: f64, v: f64) -> DVec3 {
        self.to_local * (ecef(self.key.latitude(v), self.key.longitude(u), 0.0) - self.origin)
    }
}

fn place_meshes(
    key: TileKey,
    json: &[u8],
    patch: Option<&Patch>,
    pictures: &dyn Fn(TileKey) -> Option<image::RgbaImage>,
) -> (Vec<Mesh>, Vec<Grove>) {
    serde_json::from_slice::<serde_json::Value>(json)
        .ok()
        .map(|tile| {
            (
                [
                    building_mesh(key, &tile, patch, pictures),
                    water_mesh(key, &tile, patch, pictures),
                ]
                .into_iter()
                .flatten()
                .collect(),
                areas_of(&tile, "woods")
                    .into_iter()
                    .map(|area| (area, &FOREST))
                    .chain(
                        areas_of(&tile, "parks")
                            .into_iter()
                            .map(|area| (area, &PARK)),
                    )
                    .collect(),
            )
        })
        .unwrap_or_default()
}

fn mixed(a: u64, b: u64) -> u64 {
    let z = (a ^ b.rotate_left(32)).wrapping_add(0x9E37_79B9_7F4A_7C15);
    let z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn unit(seed: u64) -> f64 {
    (seed >> 11) as f64 / (1u64 << 53) as f64
}

fn inside(ring: &[(f64, f64)], (u, v): (f64, f64)) -> bool {
    ring.iter()
        .zip(ring.iter().cycle().skip(1))
        .filter(|((au, av), (bu, bv))| {
            (av > &v) != (bv > &v) && u < (bu - au) * (v - av) / (bv - av) + au
        })
        .count()
        % 2
        == 1
}

fn within(area: &[Vec<(f64, f64)>], point: (f64, f64)) -> bool {
    inside(&area[0], point) && !area[1..].iter().any(|hole| inside(hole, point))
}

fn tile_box(ring: &[(f64, f64)]) -> ((f64, f64), (f64, f64)) {
    ring.iter()
        .fold(((1.0, 1.0), (0.0, 0.0)), |((lu, lv), (hu, hv)), &(u, v)| {
            (
                (lu.min(u).max(0.0), lv.min(v).max(0.0)),
                (hu.max(u).min(1.0), hv.max(v).min(1.0)),
            )
        })
}

pub struct Stand {
    spacing_m: f64,
    heights_m: (f64, f64),
    conifers: f64,
    brightest: f32,
}

pub type Grove = (Vec<Vec<(f64, f64)>>, &'static Stand);

struct Tree {
    at: DVec3,
    height: f64,
    spread: f64,
    conifer: bool,
    seed: u64,
    colour: [f32; 3],
}

fn scatter(key: TileKey, groves: &[Grove]) -> Vec<(f64, f64, u64, &'static Stand)> {
    let (grid, gu, gv, gspan) = key.ancestor(TREE_ZOOM);
    let (_, du, dv, span) = grid.ancestor(BUILDING_ZOOM);
    let width = grid.width_m();
    let salt = mixed(
        u64::from(grid.x) << 32 | u64::from(grid.y),
        u64::from(grid.z),
    );
    let mine = move |u: f64, v: f64| (gu..gu + gspan).contains(&u) && (gv..gv + gspan).contains(&v);
    groves
        .iter()
        .map(|(area, stand)| {
            let local: Vec<Vec<(f64, f64)>> = area
                .iter()
                .map(|ring| {
                    ring.iter()
                        .map(|&(u, v)| ((u - du) / span, (v - dv) / span))
                        .collect()
                })
                .collect();
            let ((lu, lv), (hu, hv)) = tile_box(&local[0]);
            let reach = (
                (lu.max(gu), lv.max(gv)),
                (hu.min(gu + gspan), hv.min(gv + gspan)),
            );
            (local, reach, *stand)
        })
        .filter(|(_, ((lu, lv), (hu, hv)), _)| lu < hu && lv < hv)
        .flat_map(|(local, ((lu, lv), (hu, hv)), stand)| {
            let step = stand.spacing_m / width;
            let cells =
                |low: f64, high: f64| (low / step).floor() as i64..=(high / step).floor() as i64;
            cells(lv, hv)
                .flat_map(|row| cells(lu, hu).map(move |column| (column, row)))
                .map(|(column, row)| {
                    let seed = mixed(salt, (column as u64) << 32 ^ row as u64);
                    (
                        (column as f64 + unit(seed)) * step,
                        (row as f64 + unit(mixed(seed, 1))) * step,
                        seed,
                    )
                })
                .filter(|&(u, v, _)| mine(u, v) && within(&local, (u, v)))
                .map(|(u, v, seed)| ((u - gu) / gspan, (v - gv) / gspan, seed, stand))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn canopy(
    picture: &image::RgbaImage,
    (u, v): (f64, f64),
    reach: f64,
    stand: &Stand,
) -> Option<[f32; 3]> {
    let (width, height) = picture.dimensions();
    let pixel =
        |at: f64, size: u32| ((at * f64::from(size)) as i64).clamp(0, i64::from(size) - 1) as u32;
    let samples: Vec<[u8; 4]> = (-2..=2)
        .flat_map(|dy| (-2..=2).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| {
            picture
                .get_pixel(
                    pixel(u + f64::from(dx) * reach / 2.0, width),
                    pixel(v + f64::from(dy) * reach / 2.0, height),
                )
                .0
        })
        .collect();
    let mean: [f32; 3] = std::array::from_fn(|channel| {
        samples.iter().map(|s| f32::from(s[channel])).sum::<f32>() / samples.len() as f32
    });
    (luma(mean) <= stand.brightest).then_some(mean)
}

fn forest_green(mean: [f32; 3]) -> [f32; 3] {
    let grey = luma(mean);
    let scale = grey / luma(FOREST_GREEN);
    let pulled: [f32; 3] = std::array::from_fn(|channel| {
        let own = grey + (mean[channel] - grey) * TREE_SATURATION;
        own + (FOREST_GREEN[channel] * scale - own) * FOREST_PULL
    });
    let chroma = pulled.iter().fold(0.0_f32, |widest, channel| {
        widest.max((channel - grey).abs())
    });
    let kept = (TREE_CHROMA * grey / chroma.max(f32::EPSILON)).min(1.0);
    pulled.map(|channel| grey + (channel - grey) * kept)
}

fn luma(colour: [f32; 3]) -> f32 {
    0.299 * colour[0] + 0.587 * colour[1] + 0.114 * colour[2]
}

fn shaded(colour: [f32; 3]) -> [[u8; 4]; 4] {
    SHADES.map(|shade| {
        let [r, g, b] = colour.map(|channel| (channel * shade).clamp(0.0, 255.0) as u8);
        [r, g, b, 255]
    })
}

fn shaded_palette(blocks: &[[[u8; 4]; 4]]) -> image::RgbaImage {
    let across = PALETTE_WIDTH / 2;
    let rows = 2 * (blocks.len() as u32).div_ceil(across).max(1);
    image::RgbaImage::from_fn(PALETTE_WIDTH, rows, |x, y| {
        image::Rgba(
            blocks
                .get(((y / 2) * across + x / 2) as usize)
                .map_or(ROOF, |block| block[((y % 2) * 2 + x % 2) as usize]),
        )
    })
}

fn shade_uv(block: usize, corner: usize, blocks: usize) -> [f32; 2] {
    let across = (PALETTE_WIDTH / 2) as usize;
    let rows = 2 * blocks.div_ceil(across).max(1);
    [
        ((block % across) * 2 + corner % 2) as f32 / PALETTE_WIDTH as f32
            + 0.5 / PALETTE_WIDTH as f32,
        ((block / across) * 2 + corner / 2) as f32 / rows as f32 + 0.5 / rows as f32,
    ]
}

fn corner_at(rise: f64, seed: u64) -> usize {
    if rise > 0.62 {
        0
    } else if rise < 0.3 {
        3
    } else {
        1 + (seed & 1) as usize
    }
}

fn trunk(at: DVec3, radius: f64, top: f64, uv: [f32; 2]) -> Part {
    let around: Vec<DVec3> = (0..3)
        .map(|side| {
            let angle = f64::from(side) * TAU / 3.0;
            DVec3::new(angle.cos(), angle.sin(), 0.0)
        })
        .collect();
    Part {
        positions: around
            .iter()
            .map(|d| at + *d * radius)
            .chain(
                around
                    .iter()
                    .map(|d| at + *d * radius * 0.7 + DVec3::Z * top),
            )
            .map(|p| p.as_vec3().to_array())
            .collect(),
        uvs: vec![uv; 6],
        normals: around
            .iter()
            .chain(around.iter())
            .map(|d| d.as_vec3().to_array())
            .collect(),
        indices: (0..3u32)
            .flat_map(|side| {
                let next = (side + 1) % 3;
                [side, next, 3 + next, side, 3 + next, 3 + side]
            })
            .collect(),
    }
}

fn lobe(centre: DVec3, radii: DVec3, seed: u64, shade: &dyn Fn(f64, u64) -> [f32; 2]) -> Part {
    let turn = DMat3::from_rotation_z(unit(seed) * TAU);
    let directions = ICOSAHEDRON.map(|corner| turn * DVec3::from_array(corner).normalize());
    let positions: Vec<DVec3> = directions
        .iter()
        .zip(0u64..)
        .map(|(direction, corner)| {
            let lump = 1.0 + CROWN_LUMPS * (2.0 * unit(mixed(seed, 10 + corner)) - 1.0);
            centre + *direction * radii * lump
        })
        .collect();
    Part {
        uvs: positions
            .iter()
            .zip(0u64..)
            .map(|(p, corner)| shade(p.z, mixed(seed, corner)))
            .collect(),
        positions: positions.iter().map(|p| p.as_vec3().to_array()).collect(),
        normals: directions
            .iter()
            .map(|direction| {
                (*direction + DVec3::Z * FOLIAGE_LIFT)
                    .normalize()
                    .as_vec3()
                    .to_array()
            })
            .collect(),
        indices: ICOSAHEDRON_FACES.iter().flatten().copied().collect(),
    }
}

fn broadleaf(
    tree: &Tree,
    lobes: &[(f64, f64, f64, f64)],
    near: bool,
    uv: &dyn Fn(usize) -> [f32; 2],
) -> Part {
    let height = tree.height * BROADLEAF_SHORTER;
    let turn = unit(mixed(tree.seed, 4)) * TAU;
    let width = height * (0.48 + 0.14 * unit(mixed(tree.seed, 5))) * tree.spread;
    let (bottom, top) = (tree.at.z + height * CROWN_BOTTOM, tree.at.z + height);
    let centre = tree.at.with_z((bottom + top) / 2.0);
    let radii = DVec3::new(width / 2.0, width / 2.0, (top - bottom) / 2.0);
    let shade = |z: f64, seed: u64| uv(corner_at((z - bottom) / (top - bottom), seed));
    let crown = lobes
        .iter()
        .zip(0u64..)
        .map(|(&(around, reach, rise, size), index)| {
            let angle = turn + around * TAU;
            let offset = DVec3::new(
                angle.cos() * reach * radii.x,
                angle.sin() * reach * radii.y,
                rise * radii.z,
            );
            lobe(
                centre + offset,
                radii * size,
                mixed(tree.seed, 30 + index),
                &shade,
            )
        });
    joined(crown.chain(near.then(|| {
        trunk(
            tree.at,
            height * TRUNK_RADIUS,
            height * (CROWN_BOTTOM + (1.0 - CROWN_BOTTOM) * 0.25),
            uv(4),
        )
    })))
}

fn cone(
    at: DVec3,
    (base, apex): (f64, f64),
    (radius, sides): (f64, u32),
    seed: u64,
    ring_corner: &dyn Fn(u32) -> usize,
    uv: &dyn Fn(usize) -> [f32; 2],
) -> Part {
    let turn = unit(seed) * TAU;
    let rise = apex - base;
    let ring: Vec<(DVec3, DVec3)> = (0..sides)
        .map(|side| {
            let angle = turn + f64::from(side) * TAU / f64::from(sides);
            let outward = DVec3::new(angle.cos(), angle.sin(), 0.0);
            let reach =
                radius * (1.0 + CROWN_LUMPS * (2.0 * unit(mixed(seed, u64::from(side))) - 1.0));
            (
                at + outward * reach + DVec3::Z * base,
                (outward * rise + DVec3::Z * reach).normalize(),
            )
        })
        .collect();
    Part {
        positions: ring
            .iter()
            .map(|(p, _)| *p)
            .chain(std::iter::once(at + DVec3::Z * apex))
            .map(|p| p.as_vec3().to_array())
            .collect(),
        uvs: (0..sides)
            .map(|side| uv(ring_corner(side)))
            .chain(std::iter::once(uv(0)))
            .collect(),
        normals: ring
            .iter()
            .map(|(_, n)| *n)
            .chain(std::iter::once(DVec3::Z))
            .map(|n| n.as_vec3().to_array())
            .collect(),
        indices: (0..sides)
            .flat_map(|side| [side, (side + 1) % sides, sides])
            .collect(),
    }
}

fn conifer(
    tree: &Tree,
    tiers: &[(f64, f64, f64)],
    sides: u32,
    uv: &dyn Fn(usize) -> [f32; 2],
) -> Part {
    let radius = tree.height * (0.17 + 0.05 * unit(mixed(tree.seed, 5))) * tree.spread;
    joined(
        tiers
            .iter()
            .zip(0u64..)
            .map(|(&(base, apex, scale), tier)| {
                cone(
                    tree.at,
                    (tree.height * base, tree.height * apex),
                    (radius * scale, sides),
                    mixed(tree.seed, 20 + tier),
                    &|side| {
                        if tier == 0 {
                            3
                        } else {
                            1 + (side % 2) as usize
                        }
                    },
                    uv,
                )
            }),
    )
}

fn tree_mesh(
    key: TileKey,
    groves: &[Grove],
    patch: &Patch,
    picture: &image::RgbaImage,
    home: Option<(f64, f64)>,
    far_keep: f64,
) -> Option<Mesh> {
    let near = key.z >= TREE_NEAR_ZOOM;
    let far = key.z == TREE_ZOOM;
    let placement = Placement::new(key);
    let home = home.map(|(latitude, longitude)| {
        let (tile, u, v) = TileKey::containing(latitude, longitude, key.z);
        placement.flat_at(
            f64::from(tile.x) - f64::from(key.x) + u,
            f64::from(tile.y) - f64::from(key.y) + v,
        )
    });
    let trees: Vec<Tree> = scatter(key, groves)
        .into_iter()
        .filter(|&(.., seed, _)| !far || unit(mixed(seed, 7)) < far_keep)
        .filter(|&(u, v, ..)| {
            home.is_none_or(|home| (placement.flat_at(u, v) - home).length() > HOME_CLEARING_M)
        })
        .filter_map(|(u, v, seed, stand)| {
            let mean = canopy(picture, (u, v), CANOPY_SAMPLE_M / key.width_m(), stand)?;
            let ground = patch.at(u, v);
            let conifers = stand.conifers
                + if ground > HIGHLAND_M {
                    HIGHLAND_CONIFERS
                } else {
                    0.0
                };
            let conifer = unit(mixed(seed, 3)) < conifers;
            let tint = if conifer { CONIFER_TINT } else { [1.0; 3] };
            let variety = 1.0 + TREE_VARIETY * (2.0 * unit(mixed(seed, 6)) as f32 - 1.0);
            let green = forest_green(mean);
            let colour: [f32; 3] =
                std::array::from_fn(|channel| green[channel] * TREE_LIFT * variety * tint[channel]);
            let dimmed = (luma(colour).clamp(TREE_DARKEST, TREE_BRIGHTEST)) / luma(colour).max(1.0);
            Some(Tree {
                at: placement.flat_at(u, v) + DVec3::Z * ground,
                height: stand.heights_m.0
                    + (stand.heights_m.1 - stand.heights_m.0) * unit(mixed(seed, 2)),
                spread: if far { FAR_SPREAD } else { 1.0 },
                conifer,
                seed,
                colour: colour.map(|channel| channel * dimmed),
            })
        })
        .collect();
    let blocks: Vec<[[u8; 4]; 4]> = std::iter::once([TRUNK; 4])
        .chain(trees.iter().map(|tree| shaded(tree.colour)))
        .collect();
    let whole = joined(trees.iter().zip(1..).map(|(tree, block)| {
        let uv = |corner: usize| match corner {
            4 => shade_uv(0, 0, blocks.len()),
            _ => shade_uv(block, corner, blocks.len()),
        };
        match (tree.conifer, near) {
            (true, true) => conifer(tree, &NEAR_TIERS, NEAR_SIDES, &uv),
            (true, false) => conifer(tree, &FAR_TIERS, FAR_SIDES, &uv),
            (false, true) => broadleaf(tree, &NEAR_LOBES, true, &uv),
            (false, false) => broadleaf(tree, &FAR_LOBES, false, &uv),
        }
    }));
    (!whole.indices.is_empty()).then(|| {
        Mesh::new(
            placement.to_ecef,
            whole.positions,
            whole.uvs,
            whole.indices,
            shaded_palette(&blocks),
            Layer::Foliage,
        )
        .with_normals(whole.normals)
    })
}

fn median(mut heights: Vec<f64>) -> f64 {
    heights.sort_by(f64::total_cmp);
    heights.get(heights.len() / 2).copied().unwrap_or(0.0)
}

fn interior(area: &[Vec<(f64, f64)>]) -> Vec<(f64, f64)> {
    let ((lu, lv), (hu, hv)) = tile_box(&area[0]);
    let steps = 0..WATER_SAMPLES;
    steps
        .clone()
        .flat_map(|row| steps.clone().map(move |column| (column, row)))
        .map(|(column, row)| {
            (
                lu + (hu - lu) * (f64::from(column) + 0.5) / f64::from(WATER_SAMPLES),
                lv + (hv - lv) * (f64::from(row) + 0.5) / f64::from(WATER_SAMPLES),
            )
        })
        .filter(|&point| within(area, point))
        .collect()
}

fn water_level(area: &[Vec<(f64, f64)>], ground: &dyn Fn(f64, f64) -> f64) -> f64 {
    let inside: Vec<f64> = interior(area)
        .into_iter()
        .map(|(u, v)| ground(u, v))
        .collect();
    let shore = || area[0].iter().map(|&(u, v)| ground(u, v)).collect();
    median(if inside.is_empty() { shore() } else { inside }) + WATER_LIFT_M
}

fn water_colour(
    key: TileKey,
    pictures: &HashMap<TileKey, Option<image::RgbaImage>>,
    area: &[Vec<(f64, f64)>],
) -> [u8; 4] {
    let scale = f64::from(1u32 << ROOF_PICTURE_ZOOM.saturating_sub(key.z));
    let samples: Vec<[u8; 4]> = interior(area)
        .into_iter()
        .filter_map(|(u, v)| {
            let (sub, du, dv) = under_at(key, (u, v));
            let picture = pictures.get(&sub)?.as_ref()?;
            let (width, height) = picture.dimensions();
            let x = (((u * scale - du).clamp(0.0, 1.0) * f64::from(width)) as u32).min(width - 1);
            let y = (((v * scale - dv).clamp(0.0, 1.0) * f64::from(height)) as u32).min(height - 1);
            Some(picture.get_pixel(x, y).0)
        })
        .collect();
    if samples.is_empty() {
        return roof_colour(key, pictures, &area[0], WATER, 1.0);
    }
    std::array::from_fn(|channel| {
        median(
            samples
                .iter()
                .map(|sample| f64::from(sample[channel]))
                .collect(),
        ) as u8
    })
}

fn water_mesh(
    key: TileKey,
    tile: &serde_json::Value,
    patch: Option<&Patch>,
    pictures: &dyn Fn(TileKey) -> Option<image::RgbaImage>,
) -> Option<Mesh> {
    let lakes = areas_of(tile, "water");
    let placement = Placement::new(key);
    let ground = |u: f64, v: f64| patch.map_or(0.0, |patch| patch.at(u, v));
    let fetched: HashMap<TileKey, Option<image::RgbaImage>> = lakes
        .iter()
        .flat_map(|area| {
            std::iter::once(under(key, &area[0]).0).chain(
                interior(area)
                    .into_iter()
                    .map(|point| under_at(key, point).0),
            )
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .map(|sub| (sub, pictures(sub)))
        .collect();
    let colours: Vec<[u8; 4]> = lakes
        .iter()
        .map(|area| water_colour(key, &fetched, area))
        .collect();
    let whole = joined(lakes.iter().enumerate().map(|(index, area)| {
        let level = water_level(area, &ground);
        let placed: Vec<Vec<DVec3>> = area
            .iter()
            .map(|ring| ring.iter().map(|&(u, v)| placement.flat_at(u, v)).collect())
            .collect();
        roof(&placed, level, palette_uv(index, lakes.len()))
    }));
    (!whole.indices.is_empty()).then(|| {
        Mesh::new(
            placement.to_ecef,
            whole.positions,
            whole.uvs,
            whole.indices,
            palette(&colours),
            Layer::Water,
        )
        .with_normals(whole.normals)
    })
}

fn building_mesh(
    key: TileKey,
    tile: &serde_json::Value,
    patch: Option<&Patch>,
    pictures: &dyn Fn(TileKey) -> Option<image::RgbaImage>,
) -> Option<Mesh> {
    let extent = extent_of(tile);
    let ground = |u: f64, v: f64| patch.map_or(0.0, |patch| patch.at(u, v));
    let placement = Placement::new(key);
    let flat_at = |u: f64, v: f64| placement.flat_at(u, v);
    let footprints: Vec<Footprint> = tile
        .get("buildings")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|building| {
            let rings = rings_of(building.get("rings")?, extent);
            rings.first().filter(|ring| ring.len() >= 3)?;
            let top = building.get("height").and_then(serde_json::Value::as_f64)?;
            let base = building
                .get("base")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let colour = building
                .get("colour")
                .and_then(serde_json::Value::as_str)
                .and_then(osm_colour);
            Some(Footprint {
                rings,
                base,
                top,
                colour,
            })
        })
        .collect();
    let slots = footprints.len() * 2;
    let fetched: HashMap<TileKey, Option<image::RgbaImage>> = footprints
        .iter()
        .map(|footprint| under(key, &footprint.rings[0]).0)
        .collect::<HashSet<_>>()
        .into_iter()
        .map(|sub| (sub, pictures(sub)))
        .collect();
    let colours: Vec<[u8; 4]> = footprints
        .iter()
        .flat_map(|footprint| {
            let roof = roof_colour(key, &fetched, &footprint.rings[0], ROOF, ROOF_LIFT);
            [
                roof,
                facade_colour(
                    footprint.colour,
                    wall_colour(roof, &footprint.rings[0]),
                    seed_of(&footprint.rings[0]),
                ),
            ]
        })
        .collect();
    let parts = footprints.iter().enumerate().map(|(index, footprint)| {
        let floor = ground(footprint.rings[0][0].0, footprint.rings[0][0].1);
        let placed: Vec<Vec<DVec3>> = footprint
            .rings
            .iter()
            .map(|ring| ring.iter().map(|&(u, v)| flat_at(u, v)).collect())
            .collect();
        house(
            &placed,
            floor,
            footprint,
            colours[index * 2],
            palette_uv(index * 2, slots),
            index * 2 + 1,
        )
    });
    let whole = joined(parts);
    (!whole.indices.is_empty()).then(|| {
        Mesh::new(
            placement.to_ecef,
            whole.positions,
            whole.uvs,
            whole.indices,
            palette(&colours),
            Layer::Building,
        )
        .with_normals(whole.normals)
    })
}

impl Default for OpenScene {
    fn default() -> Self {
        Self::new(Woods::FULL)
    }
}

impl OpenScene {
    pub fn new(woods: Woods) -> Self {
        Self {
            woods,
            imagery: String::new(),
            heights: Store::new(),
            terrain: HashMap::new(),
            houses: HashMap::new(),
            trees: HashMap::new(),
            home: None,
        }
    }

    pub fn clear_home(&mut self, latitude: f64, longitude: f64) {
        if self.home != Some((latitude, longitude)) {
            self.home = Some((latitude, longitude));
            self.trees.clear();
        }
    }

    pub fn deliver(&mut self, built: Built, now: Instant) {
        match built {
            Built::Heights(key, heights) => self.heights.put(key, heights, now),
            Built::Terrain {
                key,
                imagery,
                zoom,
                mesh: Ok((mesh, imaged)),
            } if imagery == self.imagery => {
                self.terrain.insert(
                    key,
                    TerrainEntry {
                        mesh: Some(Arc::new(mesh)),
                        zoom,
                        imaged,
                        asked: (!imaged).then_some(now),
                        used: now,
                    },
                );
            }
            Built::Terrain { .. } => {}
            Built::Houses(key, Ok((meshes, groves))) => {
                self.houses
                    .insert(key, HouseEntry::loaded(meshes, groves, now));
            }
            Built::Trees(key, Ok(meshes)) => {
                self.trees
                    .insert(key, HouseEntry::loaded(meshes, Vec::new(), now));
            }
            Built::Houses(..) | Built::Trees(..) => {}
        }
    }

    fn issue(&mut self, candidates: Vec<Want>, budget: usize, now: Instant) -> Vec<Want> {
        let chosen: Vec<Want> = candidates
            .into_iter()
            .map(|want| (want.priority(), want))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .take(budget)
            .collect();
        chosen.iter().for_each(|want| match want {
            Want::Heights(key) => self.heights.ask(*key, now),
            Want::Terrain { key, .. } => {
                self.terrain
                    .entry(*key)
                    .and_modify(|entry| entry.asked = Some(now))
                    .or_insert(TerrainEntry {
                        mesh: None,
                        zoom: 0,
                        imaged: false,
                        asked: Some(now),
                        used: now,
                    })
                    .used = now
            }
            Want::Houses { key, .. } => HouseEntry::ask(&mut self.houses, *key, now),
            Want::Trees { key, .. } => HouseEntry::ask(&mut self.trees, *key, now),
        });
        chosen
    }

    fn patch(&mut self, key: TileKey, wanted: &mut Vec<Want>, now: Instant) -> Option<Patch> {
        let finest = key.z.min(TERRAIN_ZOOM);
        (0..=finest)
            .rev()
            .fold(
                (None, false),
                |(found, asked): (Option<Patch>, bool), zoom| {
                    if found.is_some() {
                        return (found, asked);
                    }
                    let (source, du, dv, span) = key.ancestor(zoom);
                    match self.heights.lookup(source, now) {
                        Lookup::Ready(heights) => (
                            Some(Patch {
                                heights,
                                zoom,
                                du,
                                dv,
                                span,
                            }),
                            asked,
                        ),
                        Lookup::Missing if !asked => {
                            wanted.push(Want::Heights(source));
                            (None, true)
                        }
                        _ => (None, asked),
                    }
                },
            )
            .0
    }

    pub fn ground(
        &mut self,
        latitude: f64,
        longitude: f64,
        budget: usize,
        now: Instant,
    ) -> (Option<f64>, Vec<Want>) {
        let (key, u, v) = TileKey::containing(latitude, longitude, TERRAIN_ZOOM);
        let mut candidates = Vec::new();
        let terrain = self
            .patch(key, &mut candidates, now)
            .map(|patch| patch.at(u, v));
        let houses: Vec<Arc<Mesh>> = self
            .houses
            .values()
            .flat_map(|entry| entry.meshes.iter().cloned())
            .collect();
        let roof = ground::height_at(&houses, latitude, longitude);
        (
            terrain.map(|terrain| roof.map_or(terrain, |roof| roof.max(terrain))),
            self.issue(candidates, budget, now),
        )
    }

    fn terrain(&mut self, key: TileKey, wanted: &mut Vec<Want>, now: Instant) -> Option<Arc<Mesh>> {
        let patch = self.patch(key, wanted, now);
        let entry = self.terrain.entry(key).or_insert(TerrainEntry {
            mesh: None,
            zoom: 0,
            imaged: false,
            asked: None,
            used: now,
        });
        entry.used = now;
        let stale = entry.mesh.is_none()
            || !entry.imaged
            || patch.as_ref().is_some_and(|patch| patch.zoom > entry.zoom);
        let idle = entry
            .asked
            .is_none_or(|asked| now.duration_since(asked) >= RETRY_AFTER);
        if let Some(patch) = patch.filter(|_| stale && idle) {
            wanted.push(Want::Terrain {
                key,
                imagery: self.imagery.clone(),
                patch,
            });
        }
        entry.mesh.clone()
    }

    fn visible(
        &mut self,
        key: TileKey,
        camera: &Camera,
        planes: &[DVec4; 5],
        ground: f64,
        now: Instant,
    ) -> Option<f64> {
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
        let inside = distance - radius < REACH_M
            && !planes
                .iter()
                .any(|plane| plane.truncate().dot(center - camera.eye) + plane.w < -radius);
        inside.then_some((distance - radius).max(1.0))
    }

    fn visit(
        &mut self,
        key: TileKey,
        sight: &Sight,
        wanted: &mut Vec<Want>,
        now: Instant,
    ) -> Option<Vec<Arc<Mesh>>> {
        let Some(distance) = self.visible(key, sight.camera, &sight.planes, sight.ground, now)
        else {
            return Some(Vec::new());
        };
        let own = self.terrain(key, wanted, now);
        let unimaged = key.z >= TERRAIN_ZOOM
            && own.is_some()
            && self.terrain.get(&key).is_some_and(|entry| !entry.imaged);
        let refine = key.z < MAX_ZOOM
            && !unimaged
            && sight
                .camera
                .screen_space_error(key.width_m() / TEXELS, distance)
                > MAX_TEXEL_PX * sight.detail;
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

    fn placed(
        &mut self,
        key: TileKey,
        entries: fn(&mut OpenScene) -> &mut HashMap<TileKey, HouseEntry>,
        want: impl FnOnce(Patch) -> Want,
        wanted: &mut Vec<Want>,
        now: Instant,
    ) -> Vec<Arc<Mesh>> {
        let (loaded, idle) = {
            let entry = entries(self)
                .entry(key)
                .or_insert_with(|| HouseEntry::empty(None, now));
            entry.used = now;
            (
                entry.loaded,
                entry
                    .asked
                    .is_none_or(|asked| now.duration_since(asked) >= RETRY_AFTER),
            )
        };
        if !loaded
            && idle
            && let Some(patch) = self
                .patch(key, wanted, now)
                .filter(|patch| patch.zoom == key.z.min(TERRAIN_ZOOM))
        {
            wanted.push(want(patch));
        }
        entries(self)
            .get(&key)
            .map(|entry| entry.meshes.clone())
            .unwrap_or_default()
    }

    fn house(&mut self, key: TileKey, wanted: &mut Vec<Want>, now: Instant) -> Vec<Arc<Mesh>> {
        let imagery = self.imagery.clone();
        self.placed(
            key,
            |scene| &mut scene.houses,
            |patch| Want::Houses {
                key,
                imagery,
                patch,
            },
            wanted,
            now,
        )
    }

    fn tree(&mut self, key: TileKey, wanted: &mut Vec<Want>, now: Instant) -> Vec<Arc<Mesh>> {
        let (parent, ..) = key.ancestor(BUILDING_ZOOM);
        let Some(groves) = self
            .houses
            .get(&parent)
            .filter(|entry| entry.loaded && !entry.groves.is_empty())
            .map(|entry| entry.groves.clone())
        else {
            return Vec::new();
        };
        let (imagery, home, far_keep) = (self.imagery.clone(), self.home, self.woods.far_keep);
        self.placed(
            key,
            |scene| &mut scene.trees,
            |patch| Want::Trees {
                key,
                imagery,
                patch,
                groves,
                home,
                far_keep,
            },
            wanted,
            now,
        )
    }

    fn trees_around(
        &mut self,
        latitude: f64,
        longitude: f64,
        wanted: &mut Vec<Want>,
        now: Instant,
    ) -> Vec<Arc<Mesh>> {
        let gap_m = |key: TileKey| {
            let (eye, u, v) = TileKey::containing(latitude, longitude, key.z);
            let gap = |low: f64, at: f64| (low - at).max(at - low - 1.0).max(0.0);
            gap(f64::from(key.x) - f64::from(eye.x), u)
                .hypot(gap(f64::from(key.y) - f64::from(eye.y), v))
                * key.width_m()
        };
        fn split(key: TileKey, gap_m: &dyn Fn(TileKey) -> f64, split_m: [f64; 2]) -> Vec<TileKey> {
            if key.z < TREE_NEAR_ZOOM && gap_m(key) <= split_m[(key.z - TREE_ZOOM) as usize] {
                key.children()
                    .into_iter()
                    .flat_map(|child| split(child, gap_m, split_m))
                    .collect()
            } else {
                vec![key]
            }
        }
        let (centre, ..) = TileKey::containing(latitude, longitude, TREE_ZOOM);
        let Woods {
            reach_m, split_m, ..
        } = self.woods;
        let span = (reach_m / centre.width_m()).ceil() as i64;
        let around: Vec<TileKey> = (-span..=span)
            .flat_map(|dy| (-span..=span).map(move |dx| (dx, dy)))
            .filter_map(|(dx, dy)| {
                Some(TileKey {
                    z: TREE_ZOOM,
                    x: u32::try_from(i64::from(centre.x) + dx).ok()?,
                    y: u32::try_from(i64::from(centre.y) + dy).ok()?,
                })
            })
            .filter(|&key| gap_m(key) <= reach_m)
            .flat_map(|key| split(key, &gap_m, split_m))
            .collect();
        around
            .into_iter()
            .flat_map(|key| self.tree(key, wanted, now))
            .collect()
    }

    fn houses_around(
        &mut self,
        latitude: f64,
        longitude: f64,
        wanted: &mut Vec<Want>,
        now: Instant,
    ) -> Vec<Arc<Mesh>> {
        let (center, _, _) = TileKey::containing(latitude, longitude, BUILDING_ZOOM);
        let span = (BUILDING_REACH_M / center.width_m()).ceil() as i64;
        let around: Vec<TileKey> = (-span..=span)
            .flat_map(|dy| (-span..=span).map(move |dx| (dx, dy)))
            .filter_map(|(dx, dy)| {
                Some(TileKey {
                    z: BUILDING_ZOOM,
                    x: u32::try_from(i64::from(center.x) + dx).ok()?,
                    y: u32::try_from(i64::from(center.y) + dy).ok()?,
                })
            })
            .collect();
        around
            .into_iter()
            .flat_map(|key| self.house(key, wanted, now))
            .collect()
    }

    pub fn loaded_tiles(&self) -> usize {
        self.terrain
            .values()
            .filter(|entry| entry.mesh.is_some())
            .count()
            + self
                .houses
                .values()
                .chain(self.trees.values())
                .filter(|entry| !entry.meshes.is_empty())
                .count()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn select(
        &mut self,
        camera: &Camera,
        imagery: &str,
        ground: f64,
        latitude: f64,
        longitude: f64,
        detail: f64,
        budget: usize,
        now: Instant,
    ) -> (Vec<Arc<Mesh>>, Vec<Want>) {
        self.pick(
            camera,
            imagery,
            ground,
            Some((latitude, longitude)),
            detail,
            budget,
            now,
        )
    }

    pub fn select_terrain(
        &mut self,
        camera: &Camera,
        imagery: &str,
        ground: f64,
        detail: f64,
        budget: usize,
        now: Instant,
    ) -> (Vec<Arc<Mesh>>, Vec<Want>) {
        self.pick(camera, imagery, ground, None, detail, budget, now)
    }

    fn pick(
        &mut self,
        camera: &Camera,
        imagery: &str,
        ground: f64,
        houses_at: Option<(f64, f64)>,
        detail: f64,
        budget: usize,
        now: Instant,
    ) -> (Vec<Arc<Mesh>>, Vec<Want>) {
        if imagery != self.imagery {
            self.imagery = imagery.to_string();
            self.terrain.clear();
        }
        let sight = Sight {
            camera,
            planes: frustum(camera.culling()),
            ground,
            detail,
        };
        let mut candidates = Vec::new();
        let ground_meshes = self
            .visit(TileKey { z: 0, x: 0, y: 0 }, &sight, &mut candidates, now)
            .unwrap_or_default();
        let houses = houses_at
            .map(|(latitude, longitude)| {
                let houses = self.houses_around(latitude, longitude, &mut candidates, now);
                let trees = self.trees_around(latitude, longitude, &mut candidates, now);
                houses.into_iter().chain(trees).collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let wanted = self.issue(candidates, budget, now);
        self.terrain
            .retain(|_, entry| now.duration_since(entry.used) < KEEP_TERRAIN_FOR);
        let oldest = cutoff(
            self.terrain.values().map(|entry| entry.used).collect(),
            MAX_TERRAIN_TILES,
        );
        oldest.into_iter().for_each(|at| {
            self.terrain
                .retain(|_, entry| kept(entry.used, at, now) || awaited(entry.asked, now))
        });
        self.houses
            .retain(|_, entry| now.duration_since(entry.used) < KEEP_HOUSES_FOR);
        self.trees
            .retain(|_, entry| now.duration_since(entry.used) < KEEP_HOUSES_FOR);
        self.heights.trim(now, KEEP_DATA_FOR, MAX_HEIGHT_TILES);
        (ground_meshes.into_iter().chain(houses).collect(), wanted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::geo::Pose;

    const GREY: [u8; 4] = [120, 120, 118, 255];
    const TILES: [u8; 4] = [160, 104, 82, 255];

    fn box_ring(length: f64, width: f64, turn: f64) -> Vec<DVec3> {
        let (along, across) = (
            DVec3::new(turn.cos(), turn.sin(), 0.0),
            DVec3::new(-turn.sin(), turn.cos(), 0.0),
        );
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .iter()
            .map(|(a, b)| along * a * length / 2.0 + across * b * width / 2.0)
            .collect()
    }

    #[test]
    fn houses_get_pitched_roofs_and_blocks_only_when_their_roof_looks_tiled() {
        let gable = |ring: Vec<DVec3>, top: f64, colour| roof_for(&[ring], 0.0, top, colour);
        let Roof::Gable(long) = gable(box_ring(16.0, 8.0, 0.4), 6.0, GREY) else {
            panic!("an elongated house is gabled");
        };
        assert!(
            (long
                .along
                .dot(DVec3::new(0.4f64.cos(), 0.4f64.sin(), 0.0))
                .abs()
                - 1.0)
                .abs()
                < 1e-6
        );
        assert!(matches!(
            gable(box_ring(10.0, 10.0, 0.0), 6.0, GREY),
            Roof::Hip(_)
        ));
        assert!(matches!(
            gable(box_ring(40.0, 40.0, 0.0), 12.0, GREY),
            Roof::Flat
        ));
        assert!(matches!(
            gable(box_ring(40.0, 40.0, 0.0), 12.0, TILES),
            Roof::Hip(_)
        ));
        assert!(matches!(
            gable(box_ring(10.0, 10.0, 0.0), 20.0, GREY),
            Roof::Flat
        ));
        let l_shape = vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(14.0, 0.0, 0.0),
            DVec3::new(14.0, 4.0, 0.0),
            DVec3::new(4.0, 4.0, 0.0),
            DVec3::new(4.0, 14.0, 0.0),
            DVec3::new(0.0, 14.0, 0.0),
        ];
        assert!(matches!(gable(l_shape, 6.0, GREY), Roof::Flat));
        let courtyard = [box_ring(20.0, 12.0, 0.0), box_ring(4.0, 4.0, 0.0)];
        assert!(matches!(roof_for(&courtyard, 0.0, 6.0, TILES), Roof::Flat));
    }

    fn rectangle(length: f64, width: f64) -> Rectangle {
        bounding(&box_ring(length, width, 0.3)).expect("a rectangle bounds itself")
    }

    #[test]
    fn a_gable_closes_on_one_ridge_along_the_long_axis_above_the_walls() {
        let (eave, pitch) = (6.0, 35f64.to_radians());
        let roof = pitched(&rectangle(16.0, 8.0), eave, pitch, false, [0.0; 2], 9.0);
        let ridge = eave + pitch.tan() * 4.0;
        let top: Vec<DVec3> = roof
            .positions
            .iter()
            .map(|p| DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2])))
            .filter(|p| (p.z - ridge).abs() < 1e-3)
            .collect();
        let line = (top[0] - top[1]).normalize();
        assert!((line.dot(DVec3::new(0.3f64.cos(), 0.3f64.sin(), 0.0)).abs() - 1.0).abs() < 1e-4);
        let distinct = |points: &[DVec3]| {
            points.iter().fold(Vec::<DVec3>::new(), |seen, p| {
                if seen.iter().any(|q| (*q - *p).length() < 1e-3) {
                    seen
                } else {
                    [seen, vec![*p]].concat()
                }
            })
        };
        assert_eq!(
            distinct(&top).len(),
            4,
            "both slopes and both gables meet on one ridge"
        );
        assert!(
            roof.positions
                .iter()
                .all(|p| f64::from(p[2]) >= eave - pitch.tan() * OVERHANG_M - 1e-3)
        );
        let gables = roof.normals.iter().filter(|n| n[2] == 9.0).count();
        assert_eq!(gables, 6, "two gable triangles carry the wall tag");
    }

    #[test]
    fn a_hip_roof_has_a_ridge_shorter_by_the_width_and_every_slope_faces_up() {
        let (eave, pitch) = (6.0, 35f64.to_radians());
        let roof = pitched(&rectangle(16.0, 8.0), eave, pitch, true, [0.0; 2], 9.0);
        let ridge = eave + pitch.tan() * 4.0;
        let top: Vec<DVec3> = roof
            .positions
            .iter()
            .map(|p| DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2])))
            .filter(|p| (p.z - ridge).abs() < 1e-3)
            .collect();
        let longest = top
            .iter()
            .flat_map(|a| top.iter().map(move |b| (*a - *b).length()))
            .fold(0.0, f64::max);
        assert!((longest - 8.0).abs() < 1e-3);
        assert!(roof.normals.iter().all(|n| n[2] > 0.5 && n[2] <= 1.0));
    }

    #[test]
    fn facades_fit_whole_bays_to_each_wall_and_blank_the_short_ones() {
        let facade = Facade {
            base: 2.0,
            storey: 3.0,
            bay: 3.0,
            tag: WALL_TAG + 7.5,
        };
        let long = facade_wall(DVec3::ZERO, DVec3::new(10.0, 0.0, 0.0), 2.0, 11.0, &facade);
        assert_eq!(long.uvs[1][0], 3.0);
        assert_eq!(long.uvs[2][1], 3.0);
        assert!(long.normals.iter().all(|n| n[2] == WALL_TAG + 7.5));
        let short = facade_wall(DVec3::ZERO, DVec3::new(1.2, 0.0, 0.0), 2.0, 11.0, &facade);
        assert!(short.uvs.iter().all(|uv| uv[0] == BLANK));
    }

    #[test]
    fn osm_colours_read_both_hex_forms() {
        assert_eq!(osm_colour("#eca"), Some([238, 204, 170, 255]));
        assert_eq!(osm_colour("#b7b1a6"), Some([183, 177, 166, 255]));
        assert_eq!(osm_colour("brick"), None);
    }

    struct Offline;

    impl Fetcher for Offline {
        fn url(&self, _: &url::Url) -> Result<Vec<u8>, String> {
            Err("offline".into())
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
                _ => Err(format!("no {provider} {z}/{x}/{y}")),
            }
        }
    }

    fn camera() -> Camera {
        Camera::new(
            &Pose {
                latitude: -35.36213,
                longitude: 149.16514,
                height: 330.0,
                heading: 200.0,
                pitch: -15.0,
                roll: 0.0,
                fov: 70.0,
            },
            1280,
            720,
        )
    }

    fn run(
        scene: &mut OpenScene,
        camera: &Camera,
        rounds: usize,
        start: Instant,
    ) -> Vec<Vec<Arc<Mesh>>> {
        (0..rounds)
            .map(|round| {
                let now = start + Duration::from_millis(round as u64 * 100);
                let (meshes, wanted) = scene.select(
                    camera,
                    "Bing Satellite",
                    300.0,
                    -35.36213,
                    149.16514,
                    1.0,
                    24,
                    now,
                );
                wanted
                    .into_iter()
                    .map(|want| want.build(&Offline))
                    .for_each(|built| scene.deliver(built, now));
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
                TERRAIN => image::RgbImage::from_pixel(4, 4, image::Rgb([130, 92, 0]))
                    .write_to(&mut png, image::ImageFormat::Png)
                    .map_err(|e| e.to_string())?,
                BUILDINGS => return Err(format!("no buildings {z}/{x}/{y}")),
                _ => image::RgbImage::from_pixel(1, 1, image::Rgb([90, 110, 70]))
                    .write_to(&mut png, image::ImageFormat::Png)
                    .map_err(|e| e.to_string())?,
            }
            Ok(png.into_inner())
        }
    }

    fn low_camera(heading: f64) -> Camera {
        let ground = 130.0 * 256.0 + 92.0 - 32_768.0;
        Camera::new(
            &Pose {
                latitude: -35.36213,
                longitude: 149.16514,
                height: ground + 30.0,
                heading,
                pitch: -15.0,
                roll: 0.0,
                fov: 70.0,
            },
            2280,
            1080,
        )
    }

    fn turn_once_round(scene: &mut OpenScene, detail: f64, start: Instant) -> usize {
        let ground = 130.0 * 256.0 + 92.0 - 32_768.0;
        (0..180u64).fold(0, |requested, frame| {
            let now = start + Duration::from_millis(frame * 45);
            let (_, wanted) = scene.select(
                &low_camera(frame as f64 * 2.0),
                "Bing Satellite",
                ground,
                -35.36213,
                149.16514,
                detail,
                24,
                now,
            );
            let terrain = wanted
                .iter()
                .filter(|want| matches!(want, Want::Terrain { .. }))
                .count();
            wanted
                .into_iter()
                .map(|want| want.build(&Served))
                .for_each(|built| scene.deliver(built, now));
            requested + terrain
        })
    }

    #[test]
    fn a_full_turn_keeps_tile_memory_bounded_without_dropping_what_the_frame_uses() {
        let mut scene = OpenScene::default();
        let begun = Instant::now();
        turn_once_round(&mut scene, 1.0, begun);
        let last = begun + Duration::from_millis(179 * 45);
        let in_use = scene
            .terrain
            .values()
            .filter(|entry| entry.used == last)
            .count();
        assert!(
            scene.terrain.len() <= MAX_TERRAIN_TILES.max(in_use),
            "a 360 degree turn kept {} terrain tiles with {in_use} in use; before the cap it kept about 5000",
            scene.terrain.len()
        );
        assert!(
            scene.heights.slots.len() <= MAX_HEIGHT_TILES,
            "and {} height tiles; before the cap about 1300",
            scene.heights.slots.len()
        );
        let now = Instant::now() + Duration::from_secs(9);
        let ground = 130.0 * 256.0 + 92.0 - 32_768.0;
        let (drawn, _) = scene.select(
            &low_camera(0.0),
            "Bing Satellite",
            ground,
            -35.36213,
            149.16514,
            1.0,
            24,
            now,
        );
        assert!(
            drawn.iter().all(|mesh| scene
                .terrain
                .values()
                .any(|entry| entry.mesh.as_ref().is_some_and(|kept| kept.id == mesh.id))),
            "every mesh the frame draws is still held"
        );
    }

    #[test]
    fn a_fast_turn_asks_for_far_fewer_fine_tiles() {
        let still = turn_once_round(&mut OpenScene::default(), 1.0, Instant::now());
        let turning = turn_once_round(&mut OpenScene::default(), 3.0, Instant::now());
        assert!(
            turning * 2 < still,
            "coarser detail while turning asked for {turning} tiles against {still}"
        );
    }

    #[test]
    fn the_cap_keeps_the_newest_and_always_the_tiles_this_frame_uses() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        assert_eq!(cutoff(vec![at(1), at(5), at(3)], 2), Some(at(3)));
        assert_eq!(cutoff(vec![at(1), at(2)], 5), None);
        assert!(
            kept(at(9), at(9), at(9)),
            "a tile used in this frame stays even when this frame alone is past the cap"
        );
        assert!(
            !kept(at(4), at(4), at(9)),
            "an older frame's tiles at the cutoff go, so the cap is not overshot by a whole frame"
        );
        assert!(kept(at(5), at(4), at(9)));
        assert!(
            awaited(Some(at(9)), at(9) + Duration::from_secs(1)),
            "a tile still on its way is not evicted, or the next frame asks for it again"
        );
        assert!(
            !awaited(Some(at(9)), at(9) + RETRY_AFTER),
            "one asked so long ago it would be retried anyway can go"
        );
        assert!(!awaited(None, at(9)));
    }

    #[test]
    fn web_mercator_tiles_cover_the_point_they_are_asked_for() {
        let (key, u, v) = TileKey::containing(-35.363262, 149.165237, 15);
        assert_eq!((key.z, key.x, key.y), (15, 29961, 19829));
        assert!(
            (key.longitude(u) - 149.165237).abs() < 1e-9
                && (key.latitude(v) - -35.363262).abs() < 1e-9
        );
    }

    #[test]
    fn a_deep_tile_reads_its_heights_from_the_matching_corner_of_a_terrarium_ancestor() {
        let (ancestor, du, dv, span) = TileKey {
            z: 17,
            x: 4 * 29961 + 3,
            y: 4 * 19806 + 1,
        }
        .ancestor(15);
        assert_eq!(
            (ancestor.x, ancestor.y, du, dv, span),
            (29961, 19806, 0.75, 0.25, 0.25)
        );
    }

    #[test]
    fn terrarium_pixels_decode_to_metres_and_a_patch_knows_its_range() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_fn(2, 2, |x, _| image::Rgb([128, x as u8, 0]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let heights = Heights::from_terrarium(&png.into_inner()).unwrap();
        assert_eq!(heights.at(0.0, 0.0), 0.0);
        assert_eq!(heights.range(0.0, 0.0, 1.0), (0.0, 1.0));
        assert!(Heights::from_terrarium(b"not a png").is_err());
    }

    #[test]
    fn a_patch_range_read_from_the_bounds_pyramid_holds_every_height_in_the_patch() {
        let size = 256;
        let metres: Vec<f32> = (0..size * size)
            .map(|i| ((i * 7919) % 4001) as f32 - 2000.0 + (i / size) as f32)
            .collect();
        let heights = Heights::new(size, metres.clone());
        let exact = |du: f64, dv: f64, span: f64| {
            let pixel = |t: f64| ((t * (size - 1) as f64).round() as usize).min(size - 1);
            (pixel(dv)..=pixel(dv + span))
                .flat_map(|y| (pixel(du)..=pixel(du + span)).map(move |x| (y, x)))
                .map(|(y, x)| f64::from(metres[y * size + x]))
                .fold((f64::MAX, f64::MIN), |(low, high), h| {
                    (low.min(h), high.max(h))
                })
        };
        assert_eq!(
            heights.range(0.0, 0.0, 1.0),
            exact(0.0, 0.0, 1.0),
            "a whole tile reads its exact range"
        );
        (1..=6u32).for_each(|depth| {
            let parts = 1u32 << depth;
            let span = 1.0 / f64::from(parts);
            (0..parts * parts).for_each(|part| {
                let (du, dv) = (
                    f64::from(part % parts) * span,
                    f64::from(part / parts) * span,
                );
                let ((low, high), (true_low, true_high)) =
                    (heights.range(du, dv, span), exact(du, dv, span));
                assert!(
                    low <= true_low && high >= true_high,
                    "patch {du},{dv} span {span}: {low}..{high} misses {true_low}..{true_high}"
                );
            });
        });
    }

    #[test]
    fn the_coarse_world_draws_first_then_finer_tiles_take_over_even_without_imagery() {
        let mut scene = OpenScene::default();
        let frames = run(&mut scene, &camera(), 40, Instant::now());
        let first_drawn = frames
            .iter()
            .position(|meshes| !meshes.is_empty())
            .expect("something draws");
        assert!(
            first_drawn <= 3,
            "a coarse tile shows within a few frames, got frame {first_drawn}"
        );
        assert!(
            frames.last().unwrap().len() > frames[first_drawn].len(),
            "finer tiles replace the coarse one as they load"
        );
    }

    #[test]
    fn imagery_stops_refining_where_the_provider_has_no_finer_pictures() {
        let mut scene = OpenScene::default();
        let start = Instant::now();
        let ground = 130.0 * 256.0 + 92.0 - 32_768.0;
        let finest = (0..60u64)
            .map(|frame| {
                let now = start + Duration::from_millis(frame * 100);
                let (_, wanted) = scene.select(
                    &low_camera(0.0),
                    "Bing Satellite",
                    ground,
                    -35.36213,
                    149.16514,
                    1.0,
                    24,
                    now,
                );
                let finest = wanted
                    .iter()
                    .filter_map(|want| match want {
                        Want::Terrain { key, .. } => Some(key.z),
                        _ => None,
                    })
                    .max();
                wanted
                    .into_iter()
                    .map(|want| want.build(&Offline))
                    .for_each(|built| scene.deliver(built, now));
                finest
            })
            .flatten()
            .max();
        assert_eq!(
            finest,
            Some(TERRAIN_ZOOM),
            "a tile that borrowed its picture never asks for children"
        );
    }

    #[test]
    fn coarse_tiles_are_asked_for_first_so_a_far_sibling_is_never_starved_by_near_detail() {
        let mut scene = OpenScene::default();
        let patch = Patch {
            heights: Arc::new(Heights::new(1, vec![0.0])),
            zoom: 9,
            du: 0.0,
            dv: 0.0,
            span: 1.0,
        };
        let terrain = |z, x, y| Want::Terrain {
            key: TileKey { z, x, y },
            imagery: "Bing Satellite".to_string(),
            patch: patch.clone(),
        };
        let candidates = vec![
            terrain(17, 5, 5),
            Want::Heights(TileKey { z: 9, x: 1, y: 1 }),
            terrain(9, 1, 1),
            terrain(9, 1, 1),
        ];
        let chosen: Vec<String> = scene
            .issue(candidates, 2, Instant::now())
            .iter()
            .map(Want::describe)
            .collect();
        assert_eq!(chosen, vec!["Bing Satellite 9/1/1", "Terrarium 9/1/1"]);
    }

    #[test]
    fn a_missing_fine_height_tile_falls_back_to_a_coarser_one_instead_of_sea_level() {
        let mut scene = OpenScene::default();
        let now = Instant::now();
        let coarse = TileKey {
            z: 10,
            x: 936,
            y: 619,
        };
        scene.heights.put(
            coarse,
            Heights::from_terrarium(&{
                let mut png = std::io::Cursor::new(Vec::new());
                image::RgbImage::from_pixel(2, 2, image::Rgb([130, 92, 0]))
                    .write_to(&mut png, image::ImageFormat::Png)
                    .unwrap();
                png.into_inner()
            }),
            now,
        );
        (11..=15).for_each(|zoom| {
            scene.heights.put(
                TileKey {
                    z: zoom,
                    x: 936 << (zoom - 10),
                    y: 619 << (zoom - 10),
                },
                Err("offline".into()),
                now,
            )
        });
        let patch = scene
            .patch(
                TileKey {
                    z: 17,
                    x: 936 << 7,
                    y: 619 << 7,
                },
                &mut Vec::new(),
                now,
            )
            .expect("a coarser patch");
        assert_eq!(patch.zoom, 10);
        assert_eq!(patch.at(0.0, 0.0), 130.0 * 256.0 + 92.0 - 32_768.0);
    }

    #[test]
    fn a_terrain_tile_without_imagery_still_draws_with_its_skirt() {
        let mesh = terrain_mesh(
            TileKey {
                z: 15,
                x: 29961,
                y: 19806,
            },
            None,
            None,
        );
        assert_eq!(mesh.positions.len(), (GRID + 1) * (GRID + 1) + 4 * GRID);
        assert_eq!(mesh.indices.len(), GRID * GRID * 6 + 4 * GRID * 6);
        assert!(
            mesh.indices
                .iter()
                .all(|&i| (i as usize) < mesh.positions.len())
        );
    }

    #[test]
    fn flat_ground_faces_straight_up_and_a_slope_leans_downhill() {
        let flat = terrain_mesh(
            TileKey {
                z: 15,
                x: 29961,
                y: 19806,
            },
            None,
            None,
        );
        assert_eq!(
            flat.normals.len(),
            flat.positions.len(),
            "every vertex, skirt included, has a normal"
        );
        assert!(
            flat.normals.iter().all(|n| n[2] > 0.999),
            "flat ground faces up"
        );
        let rising_east = Heights::new(2, vec![0.0, 1000.0, 0.0, 1000.0]);
        let patch = Patch {
            heights: Arc::new(rising_east),
            zoom: 15,
            du: 0.0,
            dv: 0.0,
            span: 1.0,
        };
        let slope = terrain_mesh(
            TileKey {
                z: 15,
                x: 29961,
                y: 19806,
            },
            Some(&patch),
            None,
        );
        let middle = slope.normals[(GRID + 1) * (GRID / 2) + GRID / 2];
        assert!(
            middle[0] < -0.5 && middle[2] > 0.0 && middle[1].abs() < 1e-3,
            "ground rising to the east faces up and west, got {middle:?}"
        );
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
                let quarters = image::RgbaImage::from_fn(4, 4, |x, y| {
                    image::Rgba([
                        if x < 2 { 10 } else { 200 },
                        if y < 2 { 10 } else { 200 },
                        0,
                        255,
                    ])
                });
                (z == 18).then_some(()).ok_or("no imagery")?;
                quarters
                    .write_to(&mut png, image::ImageFormat::Png)
                    .map_err(|e| e.to_string())?;
                Ok(png.into_inner())
            }
        }
        let (borrowed, own) = picture(&ParentOnly, "Bing Satellite", TileKey { z: 19, x: 3, y: 2 })
            .expect("the parent's picture");
        assert!(!own, "a borrowed picture is retried later");
        assert_eq!(borrowed.dimensions(), (2, 2));
        assert_eq!(
            borrowed.get_pixel(0, 0).0,
            [200, 10, 0, 255],
            "x 3 is the right half, y 2 the top half of parent 1/1"
        );
        assert!(
            picture(&ParentOnly, "Bing Satellite", TileKey { z: 22, x: 0, y: 0 }).is_none(),
            "nothing within three levels up"
        );
    }

    fn flat_patch() -> Patch {
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(2, 2, image::Rgb([130, 92, 0]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        Patch {
            heights: Arc::new(Heights::from_terrarium(&png.into_inner()).unwrap()),
            zoom: TERRAIN_ZOOM,
            du: 0.0,
            dv: 0.0,
            span: 1.0,
        }
    }

    #[test]
    fn trees_scatter_the_same_way_every_time_and_only_inside_the_wood() {
        let key = TileKey {
            z: TREE_ZOOM,
            x: 8581 * 4 + 1,
            y: 5738 * 4 + 2,
        };
        let wood: Grove = (
            vec![
                vec![(0.25, 0.5), (0.5, 0.5), (0.5, 0.75), (0.25, 0.75)],
                vec![(0.3, 0.55), (0.35, 0.55), (0.35, 0.6), (0.3, 0.6)],
            ],
            &FOREST,
        );
        let once = scatter(key, std::slice::from_ref(&wood));
        let again = scatter(key, std::slice::from_ref(&wood));
        let spots = |trees: &[(f64, f64, u64, &Stand)]| {
            trees
                .iter()
                .map(|&(u, v, seed, _)| (u, v, seed))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            spots(&once),
            spots(&again),
            "the same tile grows the same forest"
        );
        let expected = 0.96 * (key.width_m() / FOREST.spacing_m).powi(2);
        assert!(
            (expected * 0.92..expected * 1.05).contains(&(once.len() as f64)),
            "{} trees for {expected:.0} cells of forest, minus the clearing",
            once.len()
        );
        assert!(
            once.iter().all(|&(u, v, ..)| {
                (0.0..1.0).contains(&u)
                    && (0.0..1.0).contains(&v)
                    && !((0.2..0.4).contains(&u) && (0.2..0.4).contains(&v))
            }),
            "every tree stands in this tile and none in the clearing"
        );
        let quarters: Vec<(f64, f64, u64)> = key
            .children()
            .iter()
            .flat_map(|&quarter| {
                let (_, du, dv, span) = quarter.ancestor(TREE_ZOOM);
                scatter(quarter, std::slice::from_ref(&wood))
                    .into_iter()
                    .map(move |(u, v, seed, _)| (du + u * span, dv + v * span, seed))
            })
            .collect();
        let sorted = |mut trees: Vec<(f64, f64, u64)>| {
            trees.sort_by_key(|&(.., seed)| seed);
            trees
        };
        let (near, far) = (sorted(quarters), sorted(spots(&once)));
        assert!(
            near.len() == far.len()
                && near.iter().zip(&far).all(|(a, b)| {
                    a.2 == b.2 && (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
                }),
            "the near quarters grow exactly the far tile's trees"
        );
        let next_door = TileKey {
            x: key.x + 1,
            ..key
        };
        assert!(
            scatter(next_door, &[wood]).is_empty(),
            "the wood ends at the tile edge"
        );
    }

    #[test]
    fn a_wood_grows_crowns_on_the_ground_coloured_like_the_canopy_and_none_on_a_road() {
        let raw = std::fs::read(format!(
            "{}/../test/Bridge/fixtures/openfreemap-14-8581-5738-zurich-lake-shore.pbf",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let json = crate::buildings::tile_json(&raw).unwrap();
        let parent = TileKey {
            z: BUILDING_ZOOM,
            x: 8581,
            y: 5738,
        };
        let (meshes, groves) = place_meshes(parent, &json, None, &|_| None);
        let layers: Vec<Layer> = meshes.iter().map(|mesh| mesh.layer).collect();
        assert_eq!(layers, vec![Layer::Building, Layer::Water]);
        assert!(
            meshes[1].normals.iter().all(|n| *n == [0.0, 0.0, 1.0]),
            "water lies flat"
        );
        assert!(!groves.is_empty(), "the shore has woods and parks");
        let children: Vec<TileKey> = parent
            .children()
            .into_iter()
            .flat_map(|child| child.children())
            .collect();
        let canopy = image::RgbaImage::from_pixel(8, 8, image::Rgba([40, 70, 30, 255]));
        let grown: Vec<Mesh> = children
            .iter()
            .flat_map(|&key| {
                std::iter::once(key).chain(key.children().into_iter().flat_map(TileKey::children))
            })
            .filter_map(|key| {
                tree_mesh(
                    key,
                    &groves,
                    &flat_patch(),
                    &canopy,
                    None,
                    Woods::FULL.far_keep,
                )
            })
            .collect();
        assert!(!grown.is_empty(), "some of the shore is wooded");
        assert!(grown.iter().all(|mesh| mesh.layer == Layer::Foliage));
        let (low, high) = grown
            .iter()
            .fold((f32::MAX, f32::MIN), |(low, high), mesh| {
                (low.min(mesh.low[2]), high.max(mesh.high[2]))
            });
        assert!(
            (603.9..604.1).contains(&low) && high < 604.0 + 30.0 * (1.0 + CROWN_LUMPS) as f32,
            "trees stand on the 604 m ground and grow no taller than 30 m: {low}..{high}"
        );
        let wooded = children
            .iter()
            .copied()
            .find(|&key| {
                tree_mesh(
                    key,
                    &groves,
                    &flat_patch(),
                    &canopy,
                    None,
                    Woods::FULL.far_keep,
                )
                .is_some()
            })
            .expect("a wooded tile");
        let home = (wooded.latitude(0.5), wooded.longitude(0.5));
        let cleared = tree_mesh(
            wooded,
            &groves,
            &flat_patch(),
            &canopy,
            Some(home),
            Woods::FULL.far_keep,
        );
        assert!(
            cleared.is_none_or(|mesh| { mesh.positions.iter().all(|p| p[0].hypot(p[1]) > 10.0) }),
            "no crown reaches over the spot the aircraft took off from"
        );
        let road = image::RgbaImage::from_pixel(8, 8, image::Rgba([170, 170, 165, 255]));
        assert!(
            children.iter().all(|&key| tree_mesh(
                key,
                &groves,
                &flat_patch(),
                &road,
                None,
                Woods::FULL.far_keep
            )
            .is_none()),
            "nothing grows where the picture shows a road"
        );
    }

    #[test]
    fn standing_inside_a_building_footprint_puts_the_ground_on_its_roof() {
        let mut scene = OpenScene::default();
        let now = Instant::now();
        let key = TileKey {
            z: BUILDING_ZOOM,
            x: 8580,
            y: 5733,
        };
        let flat = || {
            let mut png = std::io::Cursor::new(Vec::new());
            image::RgbImage::from_pixel(2, 2, image::Rgb([130, 92, 0]))
                .write_to(&mut png, image::ImageFormat::Png)
                .unwrap();
            Heights::from_terrarium(&png.into_inner()).unwrap()
        };
        scene.heights.put(key, Ok(flat()), now);
        let patch = Patch {
            heights: Arc::new(flat()),
            zoom: BUILDING_ZOOM,
            du: 0.0,
            dv: 0.0,
            span: 1.0,
        };
        let json = serde_json::json!({ "extent": 4096, "buildings": [{ "rings": [[1000, 1000, 3000, 1000, 3000, 3000, 1000, 3000]], "height": 9.0 }] });
        let house = building_mesh(key, &json, Some(&patch), &|_| None).expect("a building");
        scene
            .houses
            .insert(key, HouseEntry::loaded(vec![house], Vec::new(), now));
        let ground = |scene: &mut OpenScene, u: f64| {
            scene
                .ground(key.latitude(u), key.longitude(u), 8, now)
                .0
                .expect("a height")
        };
        let roof = ground(&mut scene, 0.5);
        assert!(
            (roof - 612.0).abs() < 0.1,
            "on the roof inside its 1 m parapet, 8 m above the 604 m street, got {roof}"
        );
        assert!(
            (ground(&mut scene, 0.1) - 604.0).abs() < 0.1,
            "beside the building, the street"
        );
    }

    #[test]
    fn a_building_takes_its_roof_from_the_picture_under_it_and_its_walls_from_a_facade_tone() {
        let key = TileKey {
            z: 14,
            x: 14980,
            y: 9903,
        };
        let json = serde_json::json!({ "extent": 4096, "buildings": [{ "rings": [[1000, 1000, 1100, 1000, 1100, 1100, 1000, 1100]], "height": 9.0 }] });
        let mesh = building_mesh(key, &json, None, &|under| {
            (under.z == ROOF_PICTURE_ZOOM).then(|| solid([120, 40, 40, 255]))
        })
        .expect("a building");
        let palette = mesh.take_texture().expect("palette")[0].clone();
        let roof = palette.get_pixel(0, 0).0;
        let wall = palette.get_pixel(1, 0).0;
        assert_eq!(roof, [180, 60, 60, 255]);
        let distance = |a: [f32; 3], b: [u8; 4]| {
            (0..3)
                .map(|c| (a[c] - f32::from(b[c])).powi(2))
                .sum::<f32>()
        };
        let roof_like = [180.0, 60.0, 60.0];
        assert!(
            FACADE_TONES
                .iter()
                .any(|tone| distance(*tone, wall) < distance(roof_like, wall)),
            "wall {wall:?} is a facade tone, not the roof {roof:?}"
        );
        let (walls, roofs): (Vec<usize>, Vec<usize>) =
            (0..mesh.normals.len()).partition(|&i| mesh.normals[i][2] > WALL_TAG - 0.5);
        assert!(
            walls
                .iter()
                .all(|&i| (mesh.normals[i][2] - WALL_TAG).floor() == 1.0),
            "every wall points at the wall colour"
        );
        assert!(
            roofs.iter().all(|&i| {
                let uv = mesh.uvs[i];
                [(uv[0] * PALETTE_WIDTH as f32) as u32, uv[1] as u32] == [0, 0]
            }),
            "every roof face points at the roof colour"
        );
    }

    #[test]
    fn a_square_block_gets_four_facades_a_parapet_roof_and_rooftop_units() {
        let json = serde_json::json!({ "extent": 4096, "buildings": [{ "rings": [[1000, 1000, 1100, 1000, 1100, 1100, 1000, 1100]], "height": 9.0, "base": 0.0 }] });
        let mesh = building_mesh(
            TileKey {
                z: 14,
                x: 14980,
                y: 9903,
            },
            &json,
            None,
            &|_| None,
        )
        .expect("a building");
        let units = (mesh.indices.len() - (4 * 6 + 2 * 3)) / 30;
        assert!((1..=MAX_ROOFTOPS).contains(&units));
        assert_eq!(mesh.indices.len(), 4 * 6 + 2 * 3 + units * 30);
        let top = mesh.positions[..16]
            .iter()
            .map(|p| p[2])
            .fold(f32::MIN, f32::max);
        assert!(
            (top - 9.0).abs() < 0.5,
            "the walls reach the building's height, got {top}"
        );
        assert!(
            mesh.positions[16..20]
                .iter()
                .all(|p| (p[2] - 8.0).abs() < 0.5),
            "the roof sits a parapet below the wall tops"
        );
        assert!(
            mesh.indices
                .iter()
                .all(|&i| (i as usize) < mesh.positions.len())
        );
        let facing: Vec<[f32; 3]> = mesh.normals.chunks(4).take(4).map(|wall| wall[0]).collect();
        let centre = |axis: usize| (mesh.low[axis] + mesh.high[axis]) / 2.0;
        let middles: Vec<[f32; 2]> = mesh
            .positions
            .chunks(4)
            .take(4)
            .map(|wall| {
                [
                    (wall[0][0] + wall[1][0]) / 2.0 - centre(0),
                    (wall[0][1] + wall[1][1]) / 2.0 - centre(1),
                ]
            })
            .collect();
        assert!(
            facing
                .iter()
                .zip(&middles)
                .all(|(n, m)| n[0] * m[0] + n[1] * m[1] > 0.0 && n[2] > WALL_TAG - 0.5),
            "each wall faces away from the middle of the building: {facing:?}"
        );
        assert!(
            mesh.normals[16..20].iter().all(|n| *n == [0.0, 0.0, 1.0]),
            "the roof faces up"
        );
    }

    #[test]
    fn a_dense_tile_joins_every_building_into_one_mesh_whose_roofs_point_at_their_own_corners() {
        let square = |n: usize| {
            let (x, y) = ((n % 40) * 100, (n / 40) * 100);
            serde_json::json!({ "rings": [[x, y, x + 50, y, x + 50, y + 50, x, y + 50]], "height": 9.0 })
        };
        let json = serde_json::json!({ "extent": 4096, "buildings": (0..1600).map(square).collect::<Vec<_>>() });
        let mesh = building_mesh(
            TileKey {
                z: 14,
                x: 14980,
                y: 9903,
            },
            &json,
            None,
            &|_| None,
        )
        .expect("buildings");
        assert_eq!(
            (mesh.positions.len(), mesh.indices.len()),
            (1600 * 20, 1600 * 30)
        );
        let last_roof = &mesh.indices[mesh.indices.len() - 6..];
        assert!(
            last_roof
                .iter()
                .all(|&i| (1599 * 20 + 16..1600 * 20).contains(&(i as usize))),
            "{last_roof:?}"
        );
    }
}
