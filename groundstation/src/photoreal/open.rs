use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::Arc;

use glam::{DMat3, DMat4, DVec3, DVec4};

use super::geo::{Camera, east_north_up, ecef};
use super::glb::{Mesh, bounds};
use super::tileset::frustum;

pub const TERRAIN: &str = "Terrarium";
pub const BUILDINGS: &str = "Buildings";
const TERRAIN_ZOOM: u32 = 15;
const MAX_ZOOM: u32 = 18;
const GRID: usize = 16;
const TEXELS: f64 = 256.0;
const MAX_TEXEL_PX: f64 = 1.5;
const RELIEF_M: f64 = 2_000.0;
const REACH_M: f64 = 60_000.0;
const BUILDING_ZOOM: u32 = 14;
const BUILDING_REACH_M: f64 = 1_500.0;
const KEPT: usize = 600;
const FALLBACK: [u8; 4] = [118, 124, 104, 255];
const ROOF: [u8; 4] = [217, 211, 199, 255];
const WALLS: [[u8; 4]; 3] = [[200, 194, 182, 255], [170, 164, 152, 255], [140, 135, 125, 255]];

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
        let latitude = self.latitude(0.5).to_radians();
        2.0 * PI * 6_378_137.0 * latitude.cos() / f64::from(1u32 << self.z)
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
    pub fn from_terrarium(png: &[u8]) -> Option<Self> {
        let image = image::load_from_memory(png).ok()?.to_rgb8();
        let size = image.width() as usize;
        (image.height() as usize == size && size > 1).then_some(())?;
        let metres = image.pixels().map(|p| f32::from(p[0]) * 256.0 + f32::from(p[1]) + f32::from(p[2]) / 256.0 - 32_768.0).collect();
        Some(Self { size, metres })
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
}

pub struct Want {
    pub provider: String,
    pub key: TileKey,
}

enum Slot<T> {
    Asked,
    Have(Option<Arc<T>>),
}

struct Store<T> {
    slots: HashMap<TileKey, (Slot<T>, u64)>,
}

impl<T> Store<T> {
    fn new() -> Self {
        Self { slots: HashMap::new() }
    }

    fn get(&mut self, key: TileKey, frame: u64) -> Option<&Slot<T>> {
        self.slots.get_mut(&key).map(|(slot, used)| {
            *used = frame;
            &*slot
        })
    }

    fn ready(&mut self, key: TileKey, frame: u64) -> Option<Option<Arc<T>>> {
        match self.get(key, frame) {
            Some(Slot::Have(value)) => Some(value.clone()),
            _ => None,
        }
    }

    fn ask(&mut self, key: TileKey, frame: u64) -> bool {
        let fresh = !self.slots.contains_key(&key);
        fresh.then(|| self.slots.insert(key, (Slot::Asked, frame)));
        fresh
    }

    fn put(&mut self, key: TileKey, value: Option<T>, frame: u64) {
        self.slots.insert(key, (Slot::Have(value.map(Arc::new)), frame));
    }

    fn trim(&mut self, frame: u64) {
        if self.slots.len() > KEPT {
            let mut ages: Vec<u64> = self.slots.values().map(|(_, used)| *used).collect();
            ages.sort_unstable();
            let cutoff = ages[ages.len() - KEPT].min(frame);
            self.slots.retain(|_, (slot, used)| *used >= cutoff || matches!(slot, Slot::Asked));
        }
    }
}

pub struct OpenScene {
    imagery: String,
    frame: u64,
    heights: Store<Heights>,
    pictures: Store<image::RgbaImage>,
    buildings: Store<Vec<u8>>,
    terrain: HashMap<TileKey, (Arc<Mesh>, u64)>,
    houses: HashMap<TileKey, (Option<Arc<Mesh>>, u64)>,
}

fn frame_at(latitude: f64, longitude: f64, height: f64) -> (DMat4, DMat3) {
    let enu = east_north_up(latitude, longitude);
    (DMat4::from_mat3_translation(enu, ecef(latitude, longitude, height)), enu.transpose())
}

fn solid(colour: [u8; 4]) -> image::RgbaImage {
    image::RgbaImage::from_pixel(1, 1, image::Rgba(colour))
}

fn terrain_mesh(key: TileKey, heights: Option<(&Heights, f64, f64, f64)>, picture: Option<&image::RgbaImage>) -> Mesh {
    let height_at = |u: f64, v: f64| heights.map_or(0.0, |(h, du, dv, span)| h.at(du + u * span, dv + v * span));
    let (center_lat, center_lng) = (key.latitude(0.5), key.longitude(0.5));
    let (to_ecef, to_local) = frame_at(center_lat, center_lng, 0.0);
    let origin = ecef(center_lat, center_lng, 0.0);
    let local = |latitude: f64, longitude: f64, height: f64| -> [f32; 3] { (to_local * (ecef(latitude, longitude, height) - origin)).as_vec3().to_array() };
    let skirt = key.width_m() / GRID as f64;
    let steps = GRID + 1;
    let grid: Vec<([f32; 3], [f32; 2])> = (0..steps)
        .flat_map(|row| (0..steps).map(move |column| (column as f64 / GRID as f64, row as f64 / GRID as f64)))
        .map(|(u, v)| (local(key.latitude(v), key.longitude(u), height_at(u, v)), [u as f32, v as f32]))
        .collect();
    let edge: Vec<usize> = (0..GRID).map(|i| i).chain((0..GRID).map(|i| i * steps + GRID)).chain((0..GRID).map(|i| GRID * steps + GRID - i)).chain((0..GRID).map(|i| (GRID - i) * steps)).collect();
    let lowered: Vec<([f32; 3], [f32; 2])> = edge
        .iter()
        .map(|&index| {
            let (u, v) = (index % steps, index / steps);
            let (u, v) = (u as f64 / GRID as f64, v as f64 / GRID as f64);
            (local(key.latitude(v), key.longitude(u), height_at(u, v) - skirt), grid[index].1)
        })
        .collect();
    let cells = (0..GRID).flat_map(|row| (0..GRID).map(move |column| row * steps + column)).flat_map(|i| [i, i + 1, i + steps, i + 1, i + steps + 1, i + steps]);
    let ring = edge.len();
    let skirts = (0..ring).flat_map(|i| {
        let (top_a, top_b) = (edge[i], edge[(i + 1) % ring]);
        let (low_a, low_b) = (grid.len() + i, grid.len() + (i + 1) % ring);
        [top_a, top_b, low_a, top_b, low_b, low_a]
    });
    let indices: Vec<u32> = cells.chain(skirts).map(|i| i as u32).collect();
    let (positions, uvs): (Vec<[f32; 3]>, Vec<[f32; 2]>) = grid.into_iter().chain(lowered).unzip();
    let (low, high) = bounds(&positions);
    Mesh { to_ecef, low, high, positions, uvs, indices, texture: picture.cloned().unwrap_or_else(|| solid(FALLBACK)) }
}

fn palette() -> image::RgbaImage {
    image::RgbaImage::from_fn(4, 1, |x, _| image::Rgba(if x == 0 { ROOF } else { WALLS[(x - 1) as usize] }))
}

fn shade_u(east: f64, north: f64) -> f32 {
    let length = (east * east + north * north).sqrt().max(1e-9);
    let facing = (-east * 0.6 + north * 0.8) / length;
    let band = if facing > 0.33 { 0 } else if facing > -0.33 { 1 } else { 2 };
    (band as f32 + 1.5) / 4.0
}

fn building_mesh(key: TileKey, json: &[u8], heights: Option<(&Heights, f64, f64, f64)>) -> Option<Mesh> {
    let tile: serde_json::Value = serde_json::from_slice(json).ok()?;
    let extent = tile.get("extent").and_then(serde_json::Value::as_f64).unwrap_or(4096.0);
    let ground = |u: f64, v: f64| heights.map_or(0.0, |(h, du, dv, span)| h.at(du + u * span, dv + v * span));
    let (center_lat, center_lng) = (key.latitude(0.5), key.longitude(0.5));
    let (to_ecef, to_local) = frame_at(center_lat, center_lng, 0.0);
    let origin = ecef(center_lat, center_lng, 0.0);
    let local = |u: f64, v: f64, height: f64| -> DVec3 { to_local * (ecef(key.latitude(v), key.longitude(u), height) - origin) };
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut earcut = earcut::Earcut::<f64>::new();
    tile.get("buildings").and_then(serde_json::Value::as_array).into_iter().flatten().for_each(|building| {
        let rings: Vec<Vec<(f64, f64)>> = building
            .get("rings")
            .and_then(serde_json::Value::as_array)
            .map(|rings| rings.iter().filter_map(serde_json::Value::as_array).map(|flat| flat.chunks_exact(2).filter_map(|p| Some((p[0].as_f64()? / extent, p[1].as_f64()? / extent))).collect()).collect())
            .unwrap_or_default();
        let Some(outline) = rings.first().filter(|ring| ring.len() >= 3) else { return };
        let top = building.get("height").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
        let base = building.get("base").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
        let floor = ground(outline[0].0, outline[0].1);
        rings.iter().for_each(|ring| {
            (0..ring.len()).for_each(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                let (low_a, low_b, high_a, high_b) = (local(a.0, a.1, floor + base), local(b.0, b.1, floor + base), local(a.0, a.1, floor + top), local(b.0, b.1, floor + top));
                let along = high_b - high_a;
                let shade = shade_u(along.y, -along.x);
                let first = positions.len() as u32;
                positions.extend([low_a, low_b, high_b, high_a].map(|p| p.as_vec3().to_array()));
                uvs.extend([[shade, 0.5]; 4]);
                indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
            });
        });
        let flat: Vec<[f64; 2]> = rings.iter().flatten().map(|&(u, v)| { let p = local(u, v, floor + top); [p.x, p.y] }).collect();
        let holes: Vec<u32> = rings.iter().scan(0u32, |at, ring| { let start = *at; *at += ring.len() as u32; Some(start) }).skip(1).collect();
        let mut triangles: Vec<u32> = Vec::new();
        earcut.earcut(flat.iter().copied(), &holes, &mut triangles);
        let first = positions.len() as u32;
        positions.extend(rings.iter().flatten().map(|&(u, v)| local(u, v, floor + top).as_vec3().to_array()));
        uvs.extend(std::iter::repeat_n([0.125, 0.5], flat.len()));
        indices.extend(triangles.into_iter().map(|i| first + i));
    });
    (!indices.is_empty()).then_some(())?;
    let (low, high) = bounds(&positions);
    Some(Mesh { to_ecef, low, high, positions, uvs, indices, texture: palette() })
}

impl OpenScene {
    pub fn new() -> Self {
        Self { imagery: String::new(), frame: 0, heights: Store::new(), pictures: Store::new(), buildings: Store::new(), terrain: HashMap::new(), houses: HashMap::new() }
    }

    pub fn deliver(&mut self, provider: &str, key: TileKey, bytes: Option<Vec<u8>>) {
        let frame = self.frame;
        match provider {
            TERRAIN => self.heights.put(key, bytes.as_deref().and_then(Heights::from_terrarium), frame),
            BUILDINGS => self.buildings.put(key, bytes, frame),
            imagery if imagery == self.imagery => self.pictures.put(key, bytes.and_then(|b| image::load_from_memory(&b).ok()).map(|i| i.to_rgba8()), frame),
            _ => {}
        }
    }

    fn heights_for(&mut self, key: TileKey, wanted: &mut Vec<Want>, budget: usize) -> Option<Option<(Arc<Heights>, f64, f64, f64)>> {
        let (source, du, dv, span) = key.ancestor(TERRAIN_ZOOM);
        match self.heights.ready(source, self.frame) {
            Some(found) => Some(found.map(|h| (h, du, dv, span))),
            None => {
                if wanted.len() < budget && self.heights.ask(source, self.frame) {
                    wanted.push(Want { provider: TERRAIN.to_string(), key: source });
                }
                None
            }
        }
    }

    pub fn ground(&mut self, latitude: f64, longitude: f64, wanted: &mut Vec<Want>, budget: usize) -> Option<f64> {
        let (key, u, v) = TileKey::containing(latitude, longitude, TERRAIN_ZOOM);
        self.heights_for(key, wanted, budget)?.map(|(heights, du, dv, span)| heights.at(du + u * span, dv + v * span))
    }

    fn terrain(&mut self, key: TileKey, wanted: &mut Vec<Want>, budget: usize) -> Option<Arc<Mesh>> {
        if let Some((mesh, used)) = self.terrain.get_mut(&key) {
            *used = self.frame;
            return Some(mesh.clone());
        }
        let heights = self.heights_for(key, wanted, budget);
        let picture = match self.pictures.ready(key, self.frame) {
            Some(found) => Some(found),
            None => {
                if wanted.len() < budget && self.pictures.ask(key, self.frame) {
                    wanted.push(Want { provider: self.imagery.clone(), key });
                }
                None
            }
        };
        let (heights, picture) = (heights?, picture?);
        let mesh = Arc::new(terrain_mesh(key, heights.as_ref().map(|(h, du, dv, span)| (h.as_ref(), *du, *dv, *span)), picture.as_deref()));
        self.terrain.insert(key, (mesh.clone(), self.frame));
        Some(mesh)
    }

    fn visible(key: TileKey, camera: &Camera, planes: &[DVec4; 5], ground: f64) -> bool {
        let center = ecef(key.latitude(0.5), key.longitude(0.5), ground);
        let radius = key.width_m() * std::f64::consts::FRAC_1_SQRT_2 + RELIEF_M;
        let distance = (center - camera.eye).length();
        distance - radius < REACH_M && !planes.iter().any(|plane| plane.truncate().dot(center - camera.eye) + plane.w < -radius)
    }

    fn visit(&mut self, key: TileKey, camera: &Camera, planes: &[DVec4; 5], ground: f64, wanted: &mut Vec<Want>, budget: usize) -> Option<Vec<Arc<Mesh>>> {
        if !Self::visible(key, camera, planes, ground) {
            return Some(Vec::new());
        }
        let center = ecef(key.latitude(0.5), key.longitude(0.5), ground);
        let distance = ((center - camera.eye).length() - key.width_m() * std::f64::consts::FRAC_1_SQRT_2).max(1.0);
        let refine = key.z < MAX_ZOOM && camera.screen_space_error(key.width_m() / TEXELS, distance) > MAX_TEXEL_PX;
        let own = self.terrain(key, wanted, budget);
        let below = refine.then(|| {
            key.children().iter().map(|child| self.visit(*child, camera, planes, ground, wanted, budget)).collect::<Vec<_>>().into_iter().collect::<Option<Vec<_>>>().map(|lists| lists.into_iter().flatten().collect::<Vec<_>>())
        });
        below.flatten().or_else(|| own.map(|mesh| vec![mesh]))
    }

    fn houses(&mut self, camera_latitude: f64, camera_longitude: f64, wanted: &mut Vec<Want>, budget: usize) -> Vec<Arc<Mesh>> {
        let (center, _, _) = TileKey::containing(camera_latitude, camera_longitude, BUILDING_ZOOM);
        let span = (BUILDING_REACH_M / center.width_m()).ceil() as i64;
        let around: Vec<TileKey> = (-span..=span)
            .flat_map(|dy| (-span..=span).map(move |dx| (dx, dy)))
            .filter_map(|(dx, dy)| Some(TileKey { z: BUILDING_ZOOM, x: u32::try_from(i64::from(center.x) + dx).ok()?, y: u32::try_from(i64::from(center.y) + dy).ok()? }))
            .collect();
        around
            .into_iter()
            .filter_map(|key| {
                if let Some((mesh, used)) = self.houses.get_mut(&key) {
                    *used = self.frame;
                    return mesh.clone();
                }
                let json = match self.buildings.ready(key, self.frame) {
                    Some(json) => json,
                    None => {
                        if wanted.len() < budget && self.buildings.ask(key, self.frame) {
                            wanted.push(Want { provider: BUILDINGS.to_string(), key });
                        }
                        return None;
                    }
                };
                let heights = self.heights_for(key, wanted, budget)?;
                let mesh = json.and_then(|json| building_mesh(key, &json, heights.as_ref().map(|(h, du, dv, span)| (h.as_ref(), *du, *dv, *span)))).map(Arc::new);
                self.houses.insert(key, (mesh.clone(), self.frame));
                mesh
            })
            .collect()
    }

    pub fn select(&mut self, camera: &Camera, imagery: &str, ground: f64, latitude: f64, longitude: f64, budget: usize) -> (Vec<Arc<Mesh>>, Vec<Want>) {
        self.frame += 1;
        if imagery != self.imagery {
            self.imagery = imagery.to_string();
            self.pictures = Store::new();
            self.terrain.clear();
        }
        let planes = frustum(camera.culling());
        let mut wanted = Vec::new();
        let ground_meshes = self.visit(TileKey { z: 0, x: 0, y: 0 }, camera, &planes, ground, &mut wanted, budget).unwrap_or_default();
        let houses = self.houses(latitude, longitude, &mut wanted, budget);
        let frame = self.frame;
        if self.terrain.len() > KEPT {
            self.terrain.retain(|_, (_, used)| frame.saturating_sub(*used) < 120);
        }
        self.houses.retain(|_, (_, used)| frame.saturating_sub(*used) < 600);
        self.heights.trim(frame);
        self.pictures.trim(frame);
        self.buildings.trim(frame);
        (ground_meshes.into_iter().chain(houses).collect(), wanted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn terrarium_pixels_decode_to_metres() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(2, 2, image::Rgb([130, 92, 128])).write_to(&mut png, image::ImageFormat::Png).unwrap();
        let heights = Heights::from_terrarium(&png.into_inner()).unwrap();
        assert_eq!(heights.at(0.5, 0.5), 130.0 * 256.0 + 92.0 + 0.5 - 32_768.0);
    }

    #[test]
    fn a_terrain_tile_without_imagery_still_draws_with_its_skirt() {
        let mesh = terrain_mesh(TileKey { z: 15, x: 29961, y: 19806 }, None, None);
        assert_eq!(mesh.positions.len(), (GRID + 1) * (GRID + 1) + 4 * GRID);
        assert_eq!(mesh.indices.len(), GRID * GRID * 6 + 4 * GRID * 6);
        assert!(mesh.indices.iter().all(|&i| (i as usize) < mesh.positions.len()));
    }

    #[test]
    fn a_square_building_gets_four_walls_and_a_two_triangle_roof() {
        let json = serde_json::json!({ "extent": 4096, "buildings": [{ "rings": [[1000, 1000, 1100, 1000, 1100, 1100, 1000, 1100]], "height": 9.0, "base": 0.0 }] });
        let mesh = building_mesh(TileKey { z: 14, x: 14980, y: 9903 }, &serde_json::to_vec(&json).unwrap(), None).expect("a building");
        assert_eq!(mesh.indices.len(), 4 * 6 + 2 * 3);
        assert!((mesh.high[2] - 9.0).abs() < 0.5, "the roof sits at the building's height, got {}", mesh.high[2]);
    }
}
