use std::sync::Arc;
use std::time::{Duration, Instant};

use glam::{DMat3, DMat4, DVec3};
use serde_json::Value;

use super::geo::{Camera, east_north_up, ecef};
use super::mesh::{Layer, Mesh, solid};

const BOTTLE_SIDES: usize = 12;
const BOTTLE_LENGTH_M: f64 = 0.32;
const BOTTLE_RADIUS_M: f64 = 0.045;
const BOTTLE_PX: f64 = 32.0;
const BODY: usize = 0;
const LABEL: usize = 1;
const CAP: usize = 2;
const BOTTLE_COLOURS: [[u8; 4]; 3] = [
    [0xA8, 0xD8, 0xF0, 255],
    [0xF5, 0xF5, 0xF5, 255],
    [0x15, 0x65, 0xC0, 255],
];
const BOTTLE_PROFILE: [(f64, f64, usize); 12] = [
    (0.0, 0.0, BODY),
    (0.040, 0.0, BODY),
    (BOTTLE_RADIUS_M, 0.012, BODY),
    (BOTTLE_RADIUS_M, 0.09, LABEL),
    (BOTTLE_RADIUS_M, 0.17, BODY),
    (BOTTLE_RADIUS_M, 0.215, BODY),
    (0.030, 0.255, BODY),
    (0.015, 0.28, BODY),
    (0.015, 0.292, CAP),
    (0.017, 0.292, CAP),
    (0.017, BOTTLE_LENGTH_M, CAP),
    (0.0, BOTTLE_LENGTH_M, CAP),
];

#[derive(Clone, Debug, PartialEq)]
pub struct Bottle {
    pub latitude: f64,
    pub longitude: f64,
    pub above_home: f64,
    pub heading: f64,
    pub tilt: f64,
    pub landed: bool,
}

impl Bottle {
    pub fn from_view(value: &Value) -> Option<Self> {
        let number = |key: &str| value.get(key)?.as_f64().filter(|v| v.is_finite());
        Some(Self {
            latitude: number("latitude")?,
            longitude: number("longitude")?,
            above_home: number("aboveHome")?,
            heading: number("heading").unwrap_or(0.0),
            tilt: number("tilt").unwrap_or(0.0),
            landed: value
                .get("landed")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }

    pub fn listed(view: &Value) -> Vec<Self> {
        listed(view, "bottles")
            .iter()
            .filter_map(Self::from_view)
            .collect()
    }
}

const ROUTE_PX: f64 = 3.0;
const MARK_PX: f64 = 14.0;
const LABEL_PX: f64 = 18.0;
const LABEL_RAISE_PX: f64 = 18.0;
const WALL_FOOT_M: f64 = 100.0;
const CIRCLE_STEPS: usize = 64;
const EARTH_RADIUS_M: f64 = 6_371_000.0;
const ROUTE: [u8; 4] = [0xFF, 0xE9, 0xA6, 255];
const HOME: [u8; 4] = [0x4C, 0xAF, 0x50, 255];
const WAYPOINT: [u8; 4] = [255, 255, 255, 255];
const INCLUSION: [u8; 4] = [0xFF, 0x8A, 0x65, 64];
const EXCLUSION: [u8; 4] = [0xE5, 0x39, 0x35, 77];
const MARK_TEXELS: u32 = 32;
const TRACKED: [u8; 4] = [0x00, 0xE5, 0xFF, 255];
const TRACKED_PX: f64 = 28.0;
const TRACKED_M: f64 = 6.0;
const TRACKED_GLIDE_LIMIT: Duration = Duration::from_secs(2);
const BRACKET_TEXELS: u32 = 10;
const BRACKET_THICKNESS: u32 = 3;
const GLYPHS: &str = "0123456789H";
const GLYPH_SCALE: u32 = 4;
const GLYPH_CELL: (u32, u32) = (5, 7);
const FONT: [[u8; 5]; 11] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b001, 0b001, 0b001],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b101, 0b101],
];

#[derive(Clone, Debug, PartialEq)]
pub struct Spot {
    pub latitude: f64,
    pub longitude: f64,
    pub above_home: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fence {
    pub inclusion: bool,
    pub points: Vec<(f64, f64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tracked {
    pub spot: Spot,
    pub north_speed: f64,
    pub east_speed: f64,
    pub seen: Instant,
}

impl Tracked {
    fn from_view(value: &Value, now: Instant) -> Option<Self> {
        let number = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite())
        };
        let age = Duration::from_millis(number("ageMs").unwrap_or(0.0).max(0.0) as u64);
        Some(Self {
            spot: spot(value)?,
            north_speed: number("northSpeed").unwrap_or(0.0),
            east_speed: number("eastSpeed").unwrap_or(0.0),
            seen: now.checked_sub(age).unwrap_or(now),
        })
    }

    pub fn moving(&self) -> bool {
        self.north_speed != 0.0 || self.east_speed != 0.0
    }

    pub fn at(&self, now: Instant) -> (f64, f64) {
        let gliding = now
            .saturating_duration_since(self.seen)
            .min(TRACKED_GLIDE_LIMIT)
            .as_secs_f64();
        let north = self.north_speed * gliding / EARTH_RADIUS_M;
        let east =
            self.east_speed * gliding / (EARTH_RADIUS_M * self.spot.latitude.to_radians().cos());
        (
            self.spot.latitude + north.to_degrees(),
            self.spot.longitude + east.to_degrees(),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Overlays {
    pub home: Option<(f64, f64)>,
    pub route: Vec<Spot>,
    pub waypoints: Vec<(Spot, String)>,
    pub fences: Vec<Fence>,
    pub fence_top: f64,
    pub tracked: Option<Tracked>,
}

fn spot(value: &Value) -> Option<Spot> {
    Some(Spot {
        latitude: value.get("latitude")?.as_f64()?,
        longitude: value.get("longitude")?.as_f64()?,
        above_home: value
            .get("aboveHome")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
    })
}

fn listed(value: &Value, key: &str) -> Vec<Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn ring(latitude: f64, longitude: f64, radius: f64) -> Vec<(f64, f64)> {
    (0..CIRCLE_STEPS)
        .map(|step| {
            let turn = step as f64 / CIRCLE_STEPS as f64 * std::f64::consts::TAU;
            let lat = latitude + (radius / EARTH_RADIUS_M * turn.cos()).to_degrees();
            let lng = longitude
                + (radius / (EARTH_RADIUS_M * latitude.to_radians().cos()) * turn.sin())
                    .to_degrees();
            (lat, lng)
        })
        .collect()
}

impl Overlays {
    pub fn from_view(view: &Value) -> Option<Self> {
        let fence_top = view.get("fenceTop")?.as_f64()?;
        let polygons = listed(view, "polygons").into_iter().filter_map(|polygon| {
            let points: Vec<(f64, f64)> = listed(&polygon, "points")
                .iter()
                .filter_map(spot)
                .map(|s| (s.latitude, s.longitude))
                .collect();
            (points.len() > 2).then(|| Fence {
                inclusion: polygon
                    .get("inclusion")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                points,
            })
        });
        let circles = listed(view, "circles").into_iter().filter_map(|circle| {
            let centre = spot(&circle)?;
            let radius = circle.get("radius")?.as_f64().filter(|r| *r > 0.0)?;
            Some(Fence {
                inclusion: circle
                    .get("inclusion")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                points: ring(centre.latitude, centre.longitude, radius),
            })
        });
        Some(Self {
            home: view
                .get("home")
                .and_then(spot)
                .map(|s| (s.latitude, s.longitude)),
            route: listed(view, "route").iter().filter_map(spot).collect(),
            waypoints: listed(view, "waypoints")
                .iter()
                .filter_map(|w| Some((spot(w)?, w.get("label")?.as_str()?.to_string())))
                .collect(),
            fences: polygons.chain(circles).collect(),
            fence_top,
            tracked: view
                .get("target")
                .and_then(|target| Tracked::from_view(target, Instant::now())),
        })
    }

    pub fn animating(&self) -> bool {
        self.tracked.as_ref().is_some_and(Tracked::moving)
    }
}

struct Frame {
    origin: DVec3,
    to_ecef: DMat4,
    to_local: DMat3,
}

impl Frame {
    fn at(latitude: f64, longitude: f64) -> Self {
        let enu = east_north_up(latitude, longitude);
        let origin = ecef(latitude, longitude, 0.0);
        Self {
            origin,
            to_ecef: DMat4::from_mat3_translation(enu, origin),
            to_local: enu.transpose(),
        }
    }

    fn local(&self, at: DVec3) -> [f32; 3] {
        (self.to_local * (at - self.origin)).as_vec3().to_array()
    }
}

fn metres_per_px(camera: &Camera, at: DVec3) -> f64 {
    (at - camera.eye).length() * 2.0 * (camera.fovy / 2.0).tan() / camera.height_px
}

fn quads(
    frame: &Frame,
    corners: impl Iterator<Item = ([DVec3; 4], [[f32; 2]; 4])>,
    texture: image::RgbaImage,
    layer: Layer,
) -> Option<Arc<Mesh>> {
    let (positions, uvs): (Vec<[f32; 3]>, Vec<[f32; 2]>) = corners
        .flat_map(|(points, uv)| points.map(|p| frame.local(p)).into_iter().zip(uv))
        .unzip();
    let count = (positions.len() / 4) as u32;
    (count > 0).then(|| {
        Arc::new(Mesh::new(
            frame.to_ecef,
            positions,
            uvs,
            (0..count)
                .flat_map(|q| [0, 1, 2, 0, 2, 3].map(|i| q * 4 + i))
                .collect(),
            texture,
            layer,
        ))
    })
}

fn billboard(
    camera: &Camera,
    at: DVec3,
    raise_px: f64,
    width_px: f64,
    height_px: f64,
) -> [DVec3; 4] {
    let right = camera.forward.cross(camera.up).normalize();
    let unit = metres_per_px(camera, at);
    let centre = at + camera.up * raise_px * unit;
    let (w, h) = (
        right * width_px / 2.0 * unit,
        camera.up * height_px / 2.0 * unit,
    );
    [
        centre - w - h,
        centre + w - h,
        centre + w + h,
        centre - w + h,
    ]
}

fn brackets(colour: [u8; 4]) -> image::RgbaImage {
    let last = MARK_TEXELS - 1;
    image::RgbaImage::from_fn(MARK_TEXELS, MARK_TEXELS, |x, y| {
        let edge = |v: u32| v < BRACKET_THICKNESS || v > last - BRACKET_THICKNESS;
        let corner = |v: u32| v < BRACKET_TEXELS || v > last - BRACKET_TEXELS;
        image::Rgba(if (edge(x) && corner(y)) || (edge(y) && corner(x)) {
            colour
        } else {
            [0, 0, 0, 0]
        })
    })
}

fn disc(colour: [u8; 4]) -> image::RgbaImage {
    let middle = MARK_TEXELS as f32 / 2.0;
    image::RgbaImage::from_fn(MARK_TEXELS, MARK_TEXELS, |x, y| {
        let distance =
            ((x as f32 + 0.5 - middle).powi(2) + (y as f32 + 0.5 - middle).powi(2)).sqrt() / middle;
        image::Rgba(if distance < 0.7 {
            colour
        } else if distance < 1.0 {
            [0, 0, 0, 255]
        } else {
            [0, 0, 0, 0]
        })
    })
}

fn glyph_lit(glyph: usize, column: i32, row: i32) -> bool {
    (0..3).contains(&column)
        && (0..5).contains(&row)
        && FONT[glyph][row as usize] & (0b100 >> column) != 0
}

pub fn atlas() -> image::RgbaImage {
    let (cell_w, cell_h) = GLYPH_CELL;
    image::RgbaImage::from_fn(
        cell_w * GLYPHS.len() as u32 * GLYPH_SCALE,
        cell_h * GLYPH_SCALE,
        |x, y| {
            let (glyph, column, row) = (
                (x / GLYPH_SCALE / cell_w) as usize,
                (x / GLYPH_SCALE % cell_w) as i32 - 1,
                (y / GLYPH_SCALE) as i32 - 1,
            );
            let lit = glyph_lit(glyph, column, row);
            let edge = (-1..=1)
                .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                .any(|(dx, dy)| glyph_lit(glyph, column + dx, row + dy));
            image::Rgba(if lit {
                [255, 255, 255, 255]
            } else if edge {
                [0, 0, 0, 255]
            } else {
                [0, 0, 0, 0]
            })
        },
    )
}

fn glyph_uv(character: char) -> Option<[[f32; 2]; 4]> {
    let index = GLYPHS.find(character)? as f32;
    let width = 1.0 / GLYPHS.len() as f32;
    let (left, right) = (index * width, (index + 1.0) * width);
    Some([[left, 1.0], [right, 1.0], [right, 0.0], [left, 0.0]])
}

fn label(camera: &Camera, at: DVec3, text: &str) -> Vec<([DVec3; 4], [[f32; 2]; 4])> {
    let (cell_w, cell_h) = GLYPH_CELL;
    let char_w = LABEL_PX * f64::from(cell_w) / f64::from(cell_h);
    let right = camera.forward.cross(camera.up).normalize() * metres_per_px(camera, at);
    let start = -(text.chars().count() as f64 - 1.0) / 2.0 * char_w;
    text.chars()
        .enumerate()
        .filter_map(|(index, character)| {
            let uv = glyph_uv(character)?;
            let shifted = at + right * (start + index as f64 * char_w);
            Some((
                billboard(camera, shifted, LABEL_RAISE_PX, char_w, LABEL_PX),
                uv,
            ))
        })
        .collect()
}

const FULL_UV: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

pub fn meshes(overlays: &Overlays, home_ground: f64, camera: &Camera) -> Vec<Arc<Mesh>> {
    let now = Instant::now();
    let Some((origin_lat, origin_lng)) = overlays
        .home
        .or_else(|| overlays.tracked.as_ref().map(|tracked| tracked.at(now)))
        .or_else(|| overlays.route.first().map(|s| (s.latitude, s.longitude)))
        .or_else(|| {
            overlays
                .fences
                .first()
                .and_then(|f| f.points.first().copied())
        })
    else {
        return Vec::new();
    };
    let frame = Frame::at(origin_lat, origin_lng);
    let lifted = |s: &Spot| ecef(s.latitude, s.longitude, home_ground + s.above_home);
    let route = overlays.route.windows(2).map(|pair| {
        let (a, b) = (lifted(&pair[0]), lifted(&pair[1]));
        let side = |p: DVec3| {
            (b - a).cross(p - camera.eye).normalize_or_zero() * ROUTE_PX / 2.0
                * metres_per_px(camera, p)
        };
        (
            [a - side(a), b - side(b), b + side(b), a + side(a)],
            FULL_UV,
        )
    });
    let marks = overlays
        .waypoints
        .iter()
        .map(|(s, _)| (billboard(camera, lifted(s), 0.0, MARK_PX, MARK_PX), FULL_UV));
    let home = overlays.home.map(|(lat, lng)| ecef(lat, lng, home_ground));
    let home_mark = home.map(|at| (billboard(camera, at, 0.0, MARK_PX, MARK_PX), FULL_UV));
    let labels = overlays
        .waypoints
        .iter()
        .flat_map(|(s, text)| label(camera, lifted(s), text))
        .chain(home.into_iter().flat_map(|at| label(camera, at, "H")));
    let walls = |inclusion: bool| {
        let (low, high) = (home_ground - WALL_FOOT_M, home_ground + overlays.fence_top);
        overlays
            .fences
            .iter()
            .filter(move |fence| fence.inclusion == inclusion)
            .flat_map(move |fence| {
                let points = &fence.points;
                (0..points.len()).map(move |i| {
                    let ((a_lat, a_lng), (b_lat, b_lng)) =
                        (points[i], points[(i + 1) % points.len()]);
                    (
                        [
                            ecef(a_lat, a_lng, low),
                            ecef(b_lat, b_lng, low),
                            ecef(b_lat, b_lng, high),
                            ecef(a_lat, a_lng, high),
                        ],
                        FULL_UV,
                    )
                })
            })
    };
    [
        quads(&frame, route, solid(ROUTE), Layer::Opaque),
        quads(&frame, walls(true), solid(INCLUSION), Layer::Translucent),
        quads(&frame, walls(false), solid(EXCLUSION), Layer::Translucent),
        quads(&frame, marks, disc(WAYPOINT), Layer::OnTop),
        quads(&frame, home_mark.into_iter(), disc(HOME), Layer::OnTop),
        quads(&frame, labels, atlas(), Layer::OnTop),
        quads(
            &frame,
            overlays.tracked.iter().map(|tracked| {
                let (lat, lng) = tracked.at(now);
                let at = ecef(lat, lng, home_ground + tracked.spot.above_home);
                let size = TRACKED_PX.max(TRACKED_M / metres_per_px(camera, at));
                (billboard(camera, at, 0.0, size, size), FULL_UV)
            }),
            brackets(TRACKED),
            Layer::OnTop,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

type Quad = ([DVec3; 4], [DVec3; 4], usize);

fn bottle_quads() -> Vec<Quad> {
    let middle = BOTTLE_LENGTH_M / 2.0;
    let turn = |step: usize| (step as f64 / BOTTLE_SIDES as f64 * std::f64::consts::TAU).sin_cos();
    BOTTLE_PROFILE
        .windows(2)
        .flat_map(|pair| {
            let ((ar, az, colour), (br, bz, _)) = (pair[0], pair[1]);
            let length = (bz - az).hypot(ar - br);
            let (nr, nz) = ((bz - az) / length, (ar - br) / length);
            let at = move |r: f64, z: f64, (sin, cos): (f64, f64)| {
                DVec3::new(r * cos, r * sin, z - middle)
            };
            let facing = move |(sin, cos): (f64, f64)| DVec3::new(nr * cos, nr * sin, nz);
            (0..BOTTLE_SIDES).map(move |side| {
                let (from, to) = (turn(side), turn(side + 1));
                (
                    [
                        at(ar, az, from),
                        at(ar, az, to),
                        at(br, bz, to),
                        at(br, bz, from),
                    ],
                    [facing(from), facing(to), facing(to), facing(from)],
                    colour,
                )
            })
        })
        .collect()
}

pub fn bottles(bottles: &[Bottle], home_ground: f64, camera: &Camera) -> Option<Arc<Mesh>> {
    let first = bottles.first()?;
    let frame = &Frame::at(first.latitude, first.longitude);
    let shape = &bottle_quads();
    let rows = BOTTLE_COLOURS.len() as f32;
    let placed: Vec<([f32; 3], [f32; 3], [f32; 2])> = bottles
        .iter()
        .flat_map(|bottle| {
            let tilt = bottle.tilt.to_radians();
            let turn = east_north_up(bottle.latitude, bottle.longitude)
                * DMat3::from_rotation_z(-bottle.heading.to_radians())
                * DMat3::from_rotation_x(-tilt);
            let lowest = home_ground + bottle.above_home;
            let scale = (BOTTLE_PX
                * metres_per_px(camera, ecef(bottle.latitude, bottle.longitude, lowest))
                / BOTTLE_LENGTH_M)
                .max(1.0);
            let lift =
                BOTTLE_LENGTH_M / 2.0 * tilt.cos().abs() + BOTTLE_RADIUS_M * tilt.sin().abs();
            let centre = ecef(bottle.latitude, bottle.longitude, lowest + lift * scale);
            let to_local = frame.to_local * turn;
            shape.iter().flat_map(move |(corners, normals, colour)| {
                corners.iter().zip(normals).map(move |(corner, normal)| {
                    (
                        frame.local(centre + turn * (*corner * scale)),
                        (to_local * *normal).as_vec3().to_array(),
                        [0.5, (*colour as f32 + 0.5) / rows],
                    )
                })
            })
        })
        .collect();
    let count = (placed.len() / 4) as u32;
    Some(Arc::new(
        Mesh::new(
            frame.to_ecef,
            placed.iter().map(|vertex| vertex.0).collect(),
            placed.iter().map(|vertex| vertex.2).collect(),
            (0..count)
                .flat_map(|q| [0, 1, 2, 0, 2, 3].map(|i| q * 4 + i))
                .collect(),
            image::RgbaImage::from_fn(1, BOTTLE_COLOURS.len() as u32, |_, y| {
                image::Rgba(BOTTLE_COLOURS[y as usize])
            }),
            Layer::Opaque,
        )
        .with_normals(placed.iter().map(|vertex| vertex.1).collect()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::geo::Pose;
    use serde_json::json;

    fn view() -> Value {
        json!({
            "home": { "latitude": -35.363, "longitude": 149.165 },
            "route": [{ "latitude": -35.363, "longitude": 149.165, "aboveHome": 0.0 }, { "latitude": -35.362, "longitude": 149.166, "aboveHome": 30.0 }, { "latitude": -35.361, "longitude": 149.165, "aboveHome": 30.0 }],
            "waypoints": [{ "latitude": -35.362, "longitude": 149.166, "aboveHome": 30.0, "label": "12" }],
            "polygons": [{ "inclusion": false, "points": [{ "latitude": -35.36, "longitude": 149.16 }, { "latitude": -35.36, "longitude": 149.17 }, { "latitude": -35.37, "longitude": 149.17 }] }],
            "circles": [{ "inclusion": true, "latitude": -35.363, "longitude": 149.165, "radius": 300.0 }],
            "fenceTop": 50.0
        })
    }

    fn camera() -> Camera {
        Camera::new(
            &Pose {
                latitude: -35.364,
                longitude: 149.164,
                height: 650.0,
                heading: 45.0,
                pitch: -15.0,
                roll: 0.0,
                fov: 70.0,
            },
            1280,
            720,
        )
    }

    #[test]
    fn the_plan_and_fences_read_from_the_core_view() {
        let overlays = Overlays::from_view(&view()).expect("overlays");
        assert_eq!(
            (
                overlays.route.len(),
                overlays.waypoints.len(),
                overlays.fences.len()
            ),
            (3, 1, 2)
        );
        assert_eq!(overlays.fences[1].points.len(), CIRCLE_STEPS);
        assert_eq!(
            Overlays::from_view(&json!({})),
            None,
            "a view without fenceTop is not an overlay view"
        );
    }

    #[test]
    fn route_walls_marks_and_labels_each_land_in_the_layer_that_draws_them_right() {
        let meshes = meshes(&Overlays::from_view(&view()).unwrap(), 600.0, &camera());
        let layers: Vec<(Layer, usize)> = meshes
            .iter()
            .map(|mesh| (mesh.layer, mesh.indices.len() / 6))
            .collect();
        assert_eq!(
            layers,
            vec![
                (Layer::Opaque, 2),
                (Layer::Translucent, CIRCLE_STEPS),
                (Layer::Translucent, 3),
                (Layer::OnTop, 1),
                (Layer::OnTop, 1),
                (Layer::OnTop, 3)
            ],
            "two route legs, the inclusion circle, the exclusion triangle, one waypoint, home, and '12' plus 'H'"
        );
        assert!(meshes.iter().all(|mesh| {
            mesh.indices
                .iter()
                .all(|&i| (i as usize) < mesh.positions.len())
        }));
    }

    #[test]
    fn a_tracked_target_draws_on_top_and_glides_on_with_its_speed() {
        let tracked_view = |age_ms: f64| json!({ "fenceTop": 50.0, "target": { "latitude": -35.362, "longitude": 149.165, "aboveHome": 0.0, "northSpeed": 10.0, "eastSpeed": 0.0, "ageMs": age_ms } });
        let overlays = Overlays::from_view(&tracked_view(0.0)).expect("overlays");
        assert!(
            overlays.animating(),
            "a moving target keeps the view repainting"
        );
        let layers: Vec<(Layer, usize)> = meshes(&overlays, 600.0, &camera())
            .iter()
            .map(|mesh| (mesh.layer, mesh.indices.len() / 6))
            .collect();
        assert_eq!(
            layers,
            vec![(Layer::OnTop, 1)],
            "one reticle, even with no home or plan"
        );
        let tracked = overlays.tracked.expect("tracked");
        let (later, _) = tracked.at(tracked.seen + Duration::from_millis(500));
        assert!(
            ((later - tracked.spot.latitude).to_radians() * EARTH_RADIUS_M - 5.0).abs() < 0.01,
            "10 m/s north for half a second is 5 m"
        );
        let (capped, _) = tracked.at(tracked.seen + Duration::from_secs(60));
        assert!(
            ((capped - tracked.spot.latitude).to_radians() * EARTH_RADIUS_M - 20.0).abs() < 0.01,
            "a target that stops reporting glides no further than two seconds"
        );
        let stale = Overlays::from_view(&tracked_view(1500.0))
            .unwrap()
            .tracked
            .unwrap();
        assert!(
            Instant::now().duration_since(stale.seen) >= Duration::from_millis(1500),
            "a report's age counts as glide already done"
        );
    }

    #[test]
    fn a_mark_stays_the_same_size_on_screen_at_any_distance() {
        let camera = camera();
        let near = billboard(
            &camera,
            camera.eye + camera.forward * 100.0,
            0.0,
            MARK_PX,
            MARK_PX,
        );
        let far = billboard(
            &camera,
            camera.eye + camera.forward * 1000.0,
            0.0,
            MARK_PX,
            MARK_PX,
        );
        assert!(((far[1] - far[0]).length() / (near[1] - near[0]).length() - 10.0).abs() < 1e-6);
    }

    #[test]
    fn the_glyph_atlas_draws_digits_white_inside_a_black_outline() {
        let atlas = atlas();
        assert_eq!(atlas.dimensions(), (5 * 11 * GLYPH_SCALE, 7 * GLYPH_SCALE));
        let pixel = |glyph: u32, column: u32, row: u32| {
            atlas
                .get_pixel((glyph * 5 + column) * GLYPH_SCALE, row * GLYPH_SCALE)
                .0
        };
        assert_eq!(
            pixel(1, 2, 1),
            [255, 255, 255, 255],
            "the top of the '1' stroke"
        );
        assert_eq!(pixel(1, 1, 1), [0, 0, 0, 255], "its outline");
        assert_eq!(pixel(1, 4, 0), [0, 0, 0, 0], "clear around it");
    }

    const SPOT: (f64, f64) = (-35.363, 149.165);
    const GROUND: f64 = 600.0;

    fn looking_down_from(metres: f64) -> Camera {
        Camera::new(
            &Pose {
                latitude: SPOT.0,
                longitude: SPOT.1,
                height: GROUND + metres,
                heading: 0.0,
                pitch: -90.0,
                roll: 0.0,
                fov: 70.0,
            },
            1280,
            720,
        )
    }

    fn bottle(tilt: f64) -> Bottle {
        Bottle {
            latitude: SPOT.0,
            longitude: SPOT.1,
            above_home: 0.0,
            heading: 90.0,
            tilt,
            landed: tilt == 90.0,
        }
    }

    fn spans(mesh: &Mesh) -> [(f64, f64); 3] {
        let axes = east_north_up(SPOT.0, SPOT.1);
        let ground = ecef(SPOT.0, SPOT.1, GROUND);
        let points: Vec<DVec3> = mesh
            .positions
            .iter()
            .map(|p| mesh.to_ecef.transform_point3(DVec3::from(p.map(f64::from))) - ground)
            .collect();
        [axes.x_axis, axes.y_axis, axes.z_axis].map(|axis| {
            points.iter().fold((f64::MAX, f64::MIN), |(low, high), p| {
                (low.min(p.dot(axis)), high.max(p.dot(axis)))
            })
        })
    }

    #[test]
    fn the_bottles_are_read_from_the_core_view() {
        let view = json!({ "bottles": [{ "bay": 1, "latitude": -35.36, "longitude": 149.16, "aboveHome": 4.5, "heading": 30.0, "tilt": 45.0, "landed": false }, { "latitude": "?" }] });
        assert_eq!(
            Bottle::listed(&view),
            [Bottle {
                latitude: -35.36,
                longitude: 149.16,
                above_home: 4.5,
                heading: 30.0,
                tilt: 45.0,
                landed: false
            }]
        );
        assert!(Bottle::listed(&json!({})).is_empty());
    }

    #[test]
    fn a_landed_bottle_lies_on_the_ground_along_its_heading_cap_first() {
        let mesh = bottles(&[bottle(90.0)], GROUND, &looking_down_from(2.0)).unwrap();
        let [east, north, up] = spans(&mesh);
        assert!(
            (up.0).abs() < 1e-3,
            "it rests on the ground, not in it: {up:?}"
        );
        assert!((up.1 - 2.0 * BOTTLE_RADIUS_M).abs() < 1e-3);
        assert!((east.1 - east.0 - BOTTLE_LENGTH_M).abs() < 1e-3, "{east:?}");
        assert!((north.1 - north.0 - 2.0 * BOTTLE_RADIUS_M).abs() < 1e-3);
        assert_eq!(mesh.layer, Layer::Opaque);
        assert_eq!(
            mesh.normals.len(),
            mesh.positions.len(),
            "it is lit by the sun"
        );
        assert_eq!(
            mesh.indices.len(),
            (BOTTLE_PROFILE.len() - 1) * BOTTLE_SIDES * 6
        );
    }

    #[test]
    fn a_falling_bottle_hangs_cap_up_from_where_it_is() {
        let mesh = bottles(&[bottle(0.0)], GROUND, &looking_down_from(2.0)).unwrap();
        let [east, _, up] = spans(&mesh);
        assert!((up.0).abs() < 1e-3 && (up.1 - BOTTLE_LENGTH_M).abs() < 1e-3);
        assert!((east.1 - east.0 - 2.0 * BOTTLE_RADIUS_M).abs() < 1e-3);
    }

    #[test]
    fn a_far_bottle_is_drawn_big_enough_to_find_and_each_bottle_is_drawn() {
        let camera = looking_down_from(300.0);
        let mesh = bottles(&[bottle(90.0), bottle(0.0)], GROUND, &camera).unwrap();
        assert_eq!(
            mesh.positions.len(),
            2 * (BOTTLE_PROFILE.len() - 1) * BOTTLE_SIDES * 4
        );
        let alone = bottles(&[bottle(90.0)], GROUND, &camera).unwrap();
        let [east, _, _] = spans(&alone);
        let wanted = BOTTLE_PX * metres_per_px(&camera, ecef(SPOT.0, SPOT.1, GROUND));
        assert!(
            (east.1 - east.0 - wanted).abs() < 1e-3 * wanted,
            "{east:?} vs {wanted}"
        );
        assert!(bottles(&[], GROUND, &camera).is_none());
    }
}
