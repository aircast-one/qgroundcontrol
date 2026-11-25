use std::sync::{Arc, LazyLock};
use std::time::Duration;
use web_time::Instant;

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
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
const LABEL_PX: f64 = 22.0;
const WALL_FOOT_M: f64 = 100.0;
const CURTAIN_FOOT_M: f64 = 1_000.0;
const STEM_PX: f64 = 2.0;
const CIRCLE_STEPS: usize = 64;
const EARTH_RADIUS_M: f64 = 6_371_000.0;
const ROUTE: [u8; 4] = [0xFF, 0xE9, 0xA6, 255];
const CURTAIN: [u8; 4] = [0xFF, 0xE9, 0xA6, 48];
const HOME: [u8; 4] = [0x4C, 0xAF, 0x50, 255];
const WAYPOINT: [u8; 4] = [255, 255, 255, 255];
const INCLUSION: [u8; 4] = [0xFF, 0x8A, 0x65, 64];
const EXCLUSION: [u8; 4] = [0xE5, 0x39, 0x35, 77];
const MARK_TEXELS: u32 = 32;
const TRACKED: [u8; 4] = [0x00, 0xE5, 0xFF, 255];
const TRACKED_PX: f64 = 28.0;
const TRACKED_M: f64 = 6.0;
const TRACKED_GLIDE_LIMIT: Duration = Duration::from_secs(2);
const AIM: [u8; 4] = [0xFF, 0x3B, 0x30, 255];
const IMPACT: [u8; 4] = [0xFF, 0xD6, 0x0A, 255];
const IMPACT_PX: f64 = 32.0;
const LEAD_PX: f64 = 22.0;
const LEAD_LINE_PX: f64 = 2.0;
const AIRCRAFT: [u8; 4] = [0xFF, 0x3D, 0x00, 255];
const UNSURE: [u8; 4] = [0x9E, 0x9E, 0x9E, 255];
const AIRCRAFT_PX: f64 = 22.0;
const AIRCRAFT_LABEL_PX: f64 = 40.0;
const NOSE_PX: f64 = 34.0;
const NOSE_LINE_PX: f64 = 4.0;
const DROP_LINE_PX: f64 = 2.0;
const CAR_PAINTS: [[u8; 4]; 4] = [
    [0xE6, 0xE7, 0xE8, 255],
    [0xA4, 0xA8, 0xAE, 255],
    [0x24, 0x3B, 0x5C, 255],
    [0x9C, 0x1F, 0x1F, 255],
];
const CAR_GLASS: [u8; 4] = [0x1A, 0x20, 0x27, 255];
const CAR_TYRE: [u8; 4] = [0x12, 0x12, 0x12, 255];
const CAR_SHADOW: [u8; 4] = [0, 0, 0, 120];
const GLASS_ROW: usize = CAR_PAINTS.len();
const TYRE_ROW: usize = CAR_PAINTS.len() + 1;
const BRACKET_TEXELS: u32 = 10;
const BRACKET_THICKNESS: u32 = 3;
const LABEL_FONT: &[u8] = include_bytes!("../../../resources/fonts/Geist-SemiBold.ttf");
const GLYPHS: &str = "0123456789Hm Pabeilnorstu";
const UNSURE_LABEL: &str = "Position unreliable";
const GLYPH_PX: f32 = 72.0;
const HALO_PX: i32 = 8;
const HALO_ALPHA: f32 = 0.75;
const HALO_SOLID: f32 = 0.4;

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
    pub aim: Option<Spot>,
    pub impact: Option<Spot>,
    pub lead: Option<Spot>,
    pub cars: Vec<Tracked>,
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
            aim: view.get("aim").and_then(spot),
            impact: view.get("impact").and_then(spot),
            lead: view.get("lead").and_then(spot),
            cars: listed(view, "cars")
                .iter()
                .filter_map(|car| Tracked::from_view(car, Instant::now()))
                .collect(),
        })
    }

    pub fn animating(&self) -> bool {
        self.tracked.as_ref().is_some_and(Tracked::moving) || self.cars.iter().any(Tracked::moving)
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

fn ribbon(camera: &Camera, a: DVec3, b: DVec3, width_px: f64) -> [DVec3; 4] {
    let side = |p: DVec3| {
        (b - a).cross(p - camera.eye).normalize_or_zero() * width_px / 2.0
            * metres_per_px(camera, p)
    };
    [a - side(a), b - side(b), b + side(b), a + side(a)]
}

fn hoop(colour: [u8; 4]) -> image::RgbaImage {
    let middle = MARK_TEXELS as f32 / 2.0;
    image::RgbaImage::from_fn(MARK_TEXELS, MARK_TEXELS, |x, y| {
        let distance =
            ((x as f32 + 0.5 - middle).powi(2) + (y as f32 + 0.5 - middle).powi(2)).sqrt() / middle;
        image::Rgba(match distance {
            d if (0.65..0.85).contains(&d) || d < 0.12 => colour,
            d if (0.5..1.0).contains(&d) || d < 0.22 => [0, 0, 0, 255],
            _ => [0, 0, 0, 0],
        })
    })
}

type Face = ([DVec3; 4], usize);

fn rect(half_width: f64, rear: f64, front: f64) -> [(f64, f64); 4] {
    [
        (-half_width, rear),
        (half_width, rear),
        (half_width, front),
        (-half_width, front),
    ]
}

fn span(left: f64, right: f64, rear: f64, front: f64) -> [(f64, f64); 4] {
    [(left, rear), (right, rear), (right, front), (left, front)]
}

fn prism(
    bottom: [(f64, f64); 4],
    top: [(f64, f64); 4],
    (low, high): (f64, f64),
    lid: usize,
    walls: usize,
) -> Vec<Face> {
    let at = |(x, y): (f64, f64), z: f64| DVec3::new(x, y, z);
    std::iter::once((top.map(|c| at(c, high)), lid))
        .chain((0..4).map(|i| {
            let j = (i + 1) % 4;
            (
                [
                    at(bottom[i], low),
                    at(bottom[j], low),
                    at(top[j], high),
                    at(top[i], high),
                ],
                walls,
            )
        }))
        .collect()
}

fn car_faces(paint: usize) -> Vec<Face> {
    let wheels = [(-0.98, -0.76), (0.76, 0.98)]
        .into_iter()
        .flat_map(|(left, right)| {
            [-1.38, 1.42].into_iter().flat_map(move |y| {
                let tyre = span(left, right, y - 0.33, y + 0.33);
                prism(tyre, tyre, (0.0, 0.62), TYRE_ROW, TYRE_ROW)
            })
        });
    prism(
        rect(0.9, -2.25, 2.25),
        rect(0.88, -2.2, 2.2),
        (0.3, 0.86),
        paint,
        paint,
    )
    .into_iter()
    .chain(prism(
        rect(0.84, -1.3, 0.85),
        rect(0.7, -0.88, 0.32),
        (0.86, 1.44),
        paint,
        GLASS_ROW,
    ))
    .chain(wheels)
    .collect()
}

fn car_colours() -> image::RgbaImage {
    let rows: Vec<[u8; 4]> = CAR_PAINTS
        .iter()
        .copied()
        .chain([CAR_GLASS, CAR_TYRE])
        .collect();
    image::RgbaImage::from_fn(1, rows.len() as u32, |_, y| image::Rgba(rows[y as usize]))
}

fn car_turn(car: &Tracked, now: Instant) -> (f64, f64, DMat3) {
    let (lat, lng) = car.at(now);
    (
        lat,
        lng,
        east_north_up(lat, lng) * DMat3::from_rotation_z(-car.east_speed.atan2(car.north_speed)),
    )
}

pub fn cars(cars: &[Tracked], home_ground: f64, now: Instant) -> Vec<Arc<Mesh>> {
    let Some(first) = cars.first() else {
        return Vec::new();
    };
    let (origin_lat, origin_lng) = first.at(now);
    let frame = &Frame::at(origin_lat, origin_lng);
    let rows = (CAR_PAINTS.len() + 2) as f32;
    let placed: Vec<([f32; 3], [f32; 3], [f32; 2])> = cars
        .iter()
        .enumerate()
        .flat_map(|(index, car)| {
            let (lat, lng, turn) = car_turn(car, now);
            let base = ecef(lat, lng, home_ground + car.spot.above_home);
            let to_local = frame.to_local * turn;
            car_faces(index % CAR_PAINTS.len())
                .into_iter()
                .flat_map(move |(corners, row)| {
                    let normal = (corners[1] - corners[0])
                        .cross(corners[3] - corners[0])
                        .normalize_or_zero();
                    let lit = (to_local * normal).as_vec3().to_array();
                    corners.map(|c| {
                        (
                            frame.local(base + turn * c),
                            lit,
                            [0.5, (row as f32 + 0.5) / rows],
                        )
                    })
                })
        })
        .collect();
    let count = (placed.len() / 4) as u32;
    let body = Mesh::new(
        frame.to_ecef,
        placed.iter().map(|vertex| vertex.0).collect(),
        placed.iter().map(|vertex| vertex.2).collect(),
        (0..count)
            .flat_map(|q| [0, 1, 2, 0, 2, 3].map(|i| q * 4 + i))
            .collect(),
        car_colours(),
        Layer::Opaque,
    )
    .with_normals(placed.iter().map(|vertex| vertex.1).collect());
    let shadows = cars.iter().map(|car| {
        let (lat, lng, turn) = car_turn(car, now);
        let base = ecef(lat, lng, home_ground + car.spot.above_home + 0.03);
        (
            rect(1.1, -2.5, 2.5).map(|(x, y)| base + turn * DVec3::new(x, y, 0.0)),
            FULL_UV,
        )
    });
    std::iter::once(Arc::new(body))
        .chain(quads(frame, shadows, solid(CAR_SHADOW), Layer::Translucent))
        .collect()
}

struct Glyph {
    character: char,
    left: u32,
    width: u32,
    advance: f32,
}

struct Glyphs {
    atlas: image::RgbaImage,
    glyphs: Vec<Glyph>,
    text_px: f32,
}

static LABEL_GLYPHS: LazyLock<Glyphs> = LazyLock::new(rasterised);

fn coverage(font: &FontRef, character: char, width: u32, height: u32, baseline: f32) -> Vec<f32> {
    let id = font
        .glyph_id(character)
        .with_scale_and_position(PxScale::from(GLYPH_PX), point(HALO_PX as f32, baseline));
    let mut cells = vec![0.0; (width * height) as usize];
    if let Some(outline) = font.outline_glyph(id) {
        let bounds = outline.px_bounds();
        outline.draw(|x, y, value| {
            let (x, y) = (
                x as i32 + bounds.min.x as i32,
                y as i32 + bounds.min.y as i32,
            );
            if (0..width as i32).contains(&x) && (0..height as i32).contains(&y) {
                cells[(y as u32 * width + x as u32) as usize] = value;
            }
        });
    }
    cells
}

fn halo(cells: &[f32], width: u32, height: u32, x: i32, y: i32) -> f32 {
    let reach = HALO_PX as f32;
    (-HALO_PX..=HALO_PX)
        .flat_map(|dy| (-HALO_PX..=HALO_PX).map(move |dx| (dx, dy)))
        .filter(|&(dx, dy)| {
            (0..width as i32).contains(&(x + dx)) && (0..height as i32).contains(&(y + dy))
        })
        .map(|(dx, dy)| {
            let fade = (1.0 - ((dx * dx + dy * dy) as f32).sqrt() / reach) / (1.0 - HALO_SOLID);
            cells[((y + dy) as u32 * width + (x + dx) as u32) as usize] * fade.clamp(0.0, 1.0)
        })
        .fold(0.0, f32::max)
}

fn lettered(cells: &[f32], width: u32, height: u32, x: u32, y: u32) -> image::Rgba<u8> {
    let ink = cells[(y * width + x) as usize];
    let shadow = halo(cells, width, height, x as i32, y as i32) * HALO_ALPHA;
    let alpha = ink + shadow * (1.0 - ink);
    let white = if alpha > 0.0 { ink / alpha } else { 0.0 };
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    image::Rgba([byte(white), byte(white), byte(white), byte(alpha)])
}

fn rasterised() -> Glyphs {
    let font =
        FontRef::try_from_slice(LABEL_FONT).expect("the label font is a valid TrueType font");
    let scaled = font.as_scaled(PxScale::from(GLYPH_PX));
    let text_px = scaled.ascent() - scaled.descent();
    let height = text_px.ceil() as u32 + 2 * HALO_PX as u32;
    let baseline = HALO_PX as f32 + scaled.ascent();
    let drawn: Vec<(char, f32, u32, Vec<f32>)> = GLYPHS
        .chars()
        .map(|character| {
            let advance = scaled.h_advance(font.glyph_id(character));
            let width = advance.ceil() as u32 + 2 * HALO_PX as u32;
            (
                character,
                advance,
                width,
                coverage(&font, character, width, height, baseline),
            )
        })
        .collect();
    let lefts = drawn.iter().scan(0, |left, (_, _, width, _)| {
        let at = *left;
        *left += width;
        Some(at)
    });
    let glyphs: Vec<Glyph> = drawn
        .iter()
        .zip(lefts)
        .map(|((character, advance, width, _), left)| Glyph {
            character: *character,
            left,
            width: *width,
            advance: *advance,
        })
        .collect();
    let total = glyphs.iter().map(|glyph| glyph.width).sum::<u32>();
    let atlas = image::RgbaImage::from_fn(total, height, |x, y| {
        glyphs
            .iter()
            .zip(&drawn)
            .find(|(glyph, _)| (glyph.left..glyph.left + glyph.width).contains(&x))
            .map(|(glyph, (_, _, width, cells))| lettered(cells, *width, height, x - glyph.left, y))
            .unwrap_or(image::Rgba([0, 0, 0, 0]))
    });
    Glyphs {
        atlas,
        glyphs,
        text_px,
    }
}

pub fn atlas() -> image::RgbaImage {
    LABEL_GLYPHS.atlas.clone()
}

fn label(
    camera: &Camera,
    at: DVec3,
    text: &str,
    height_px: f64,
) -> Vec<([DVec3; 4], [[f32; 2]; 4])> {
    let set = &*LABEL_GLYPHS;
    let scale = height_px / f64::from(set.text_px);
    let (atlas_w, atlas_h) = set.atlas.dimensions();
    let glyphs: Vec<&Glyph> = text
        .chars()
        .filter_map(|character| set.glyphs.iter().find(|glyph| glyph.character == character))
        .collect();
    let span = glyphs
        .iter()
        .map(|glyph| f64::from(glyph.advance))
        .sum::<f64>()
        * scale;
    let right = camera.forward.cross(camera.up).normalize() * metres_per_px(camera, at);
    glyphs
        .iter()
        .scan(-span / 2.0, |pen, glyph| {
            let advance = f64::from(glyph.advance) * scale;
            let centre = *pen + advance / 2.0;
            *pen += advance;
            Some((glyph, centre))
        })
        .map(|(glyph, centre)| {
            let (left, right_uv) = (
                glyph.left as f32 / atlas_w as f32,
                (glyph.left + glyph.width) as f32 / atlas_w as f32,
            );
            let quad = billboard(
                camera,
                at + right * centre,
                height_px,
                f64::from(glyph.width) * scale,
                f64::from(atlas_h) * scale,
            );
            (
                quad,
                [[left, 1.0], [right_uv, 1.0], [right_uv, 0.0], [left, 0.0]],
            )
        })
        .collect()
}

const FULL_UV: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

pub fn meshes(overlays: &Overlays, home_ground: f64, camera: &Camera) -> Vec<Arc<Mesh>> {
    let now = Instant::now();
    let Some((origin_lat, origin_lng)) = overlays
        .home
        .or_else(|| overlays.tracked.as_ref().map(|tracked| tracked.at(now)))
        .or_else(|| overlays.cars.first().map(|car| car.at(now)))
        .or_else(|| {
            overlays
                .aim
                .iter()
                .chain(&overlays.impact)
                .next()
                .map(|s| (s.latitude, s.longitude))
        })
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
        (
            ribbon(camera, lifted(&pair[0]), lifted(&pair[1]), ROUTE_PX),
            FULL_UV,
        )
    });
    let foot = |s: &Spot| ecef(s.latitude, s.longitude, home_ground - CURTAIN_FOOT_M);
    let curtain = overlays.route.windows(2).map(|pair| {
        (
            [
                lifted(&pair[0]),
                lifted(&pair[1]),
                foot(&pair[1]),
                foot(&pair[0]),
            ],
            FULL_UV,
        )
    });
    let stems = overlays
        .waypoints
        .iter()
        .map(|(s, _)| (ribbon(camera, lifted(s), foot(s), STEM_PX), FULL_UV));
    let marks = overlays
        .waypoints
        .iter()
        .map(|(s, _)| (billboard(camera, lifted(s), 0.0, MARK_PX, MARK_PX), FULL_UV));
    let home = overlays.home.map(|(lat, lng)| ecef(lat, lng, home_ground));
    let home_mark = home.map(|at| (billboard(camera, at, 0.0, MARK_PX, MARK_PX), FULL_UV));
    let labels = overlays
        .waypoints
        .iter()
        .flat_map(|(s, text)| label(camera, lifted(s), text, LABEL_PX))
        .chain(
            home.into_iter()
                .flat_map(|at| label(camera, at, "H", LABEL_PX)),
        );
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
        quads(&frame, stems, solid(ROUTE), Layer::Opaque),
        quads(&frame, curtain, solid(CURTAIN), Layer::Translucent),
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
        quads(
            &frame,
            overlays.aim.iter().map(|s| {
                let at = lifted(s);
                let size = TRACKED_PX.max(TRACKED_M / metres_per_px(camera, at));
                (billboard(camera, at, 0.0, size, size), FULL_UV)
            }),
            brackets(AIM),
            Layer::OnTop,
        ),
        quads(
            &frame,
            overlays.impact.iter().map(|s| {
                (
                    billboard(camera, lifted(s), 0.0, IMPACT_PX, IMPACT_PX),
                    FULL_UV,
                )
            }),
            hoop(IMPACT),
            Layer::OnTop,
        ),
        quads(
            &frame,
            overlays.aim.iter().zip(&overlays.lead).map(|(aim, lead)| {
                (
                    ribbon(camera, lifted(aim), lifted(lead), LEAD_LINE_PX),
                    FULL_UV,
                )
            }),
            solid(AIM),
            Layer::OnTop,
        ),
        quads(
            &frame,
            overlays
                .lead
                .iter()
                .map(|s| (billboard(camera, lifted(s), 0.0, LEAD_PX, LEAD_PX), FULL_UV)),
            hoop(AIM),
            Layer::OnTop,
        ),
    ]
    .into_iter()
    .flatten()
    .chain(cars(&overlays.cars, home_ground, now))
    .collect()
}

pub fn aircraft(
    at: (f64, f64, f64),
    heading: f64,
    ground: Option<f64>,
    camera: &Camera,
    unsure: bool,
) -> Vec<Arc<Mesh>> {
    let colour = if unsure { UNSURE } else { AIRCRAFT };
    let frame = Frame::at(at.0, at.1);
    let body = ecef(at.0, at.1, at.2);
    let unit = metres_per_px(camera, body);
    let bearing = heading.to_radians();
    let nose = body
        + east_north_up(at.0, at.1)
            * DVec3::new(bearing.sin(), bearing.cos(), 0.0)
            * NOSE_PX
            * unit;
    let foot = ground.map(|height| ecef(at.0, at.1, height));
    let above = match unsure {
        true => Some(UNSURE_LABEL.to_string()),
        false => ground.map(|height| format!("{:.0} m", (at.2 - height).max(0.0))),
    };
    let over_marker = body + camera.up * AIRCRAFT_PX / 2.0 * unit;
    [
        quads(
            &frame,
            foot.map(|foot| (ribbon(camera, body, foot, DROP_LINE_PX), FULL_UV))
                .into_iter(),
            solid(colour),
            Layer::Opaque,
        ),
        quads(
            &frame,
            std::iter::once((ribbon(camera, body, nose, NOSE_LINE_PX), FULL_UV)),
            solid(colour),
            Layer::OnTop,
        ),
        quads(
            &frame,
            std::iter::once((
                billboard(camera, body, 0.0, AIRCRAFT_PX, AIRCRAFT_PX),
                FULL_UV,
            )),
            disc(colour),
            Layer::OnTop,
        ),
        quads(
            &frame,
            above
                .iter()
                .flat_map(|text| label(camera, over_marker, text, AIRCRAFT_LABEL_PX)),
            atlas(),
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
                (Layer::Opaque, 1),
                (Layer::Translucent, 2),
                (Layer::Translucent, CIRCLE_STEPS),
                (Layer::Translucent, 3),
                (Layer::OnTop, 1),
                (Layer::OnTop, 1),
                (Layer::OnTop, 3)
            ],
            "two route legs, the waypoint's stem, the curtain under both legs, the inclusion circle, the exclusion triangle, one waypoint, home, and '12' plus 'H'"
        );
        assert!(meshes.iter().all(|mesh| {
            mesh.indices
                .iter()
                .all(|&i| (i as usize) < mesh.positions.len())
        }));
    }

    #[test]
    fn the_drop_target_and_its_impact_point_draw_on_top_with_no_home_or_plan() {
        let overlays = Overlays::from_view(&json!({
            "fenceTop": 50.0,
            "aim": { "latitude": -35.362, "longitude": 149.165, "aboveHome": 0.0 },
            "impact": { "latitude": -35.3625, "longitude": 149.165, "aboveHome": 0.0 },
        }))
        .expect("overlays");
        let layers: Vec<(Layer, usize)> = meshes(&overlays, 600.0, &camera())
            .iter()
            .map(|mesh| (mesh.layer, mesh.indices.len() / 6))
            .collect();
        assert_eq!(layers, vec![(Layer::OnTop, 1), (Layer::OnTop, 1)]);
    }

    #[test]
    fn the_lead_point_draws_as_a_ring_with_a_line_back_to_the_target() {
        let overlays = Overlays::from_view(&json!({
            "fenceTop": 50.0,
            "aim": { "latitude": -35.362, "longitude": 149.165, "aboveHome": 0.0 },
            "lead": { "latitude": -35.3618, "longitude": 149.165, "aboveHome": 0.0 },
        }))
        .expect("overlays");
        let layers: Vec<(Layer, usize)> = meshes(&overlays, 600.0, &camera())
            .iter()
            .map(|mesh| (mesh.layer, mesh.indices.len() / 6))
            .collect();
        assert_eq!(
            layers,
            vec![(Layer::OnTop, 1), (Layer::OnTop, 1), (Layer::OnTop, 1)],
            "the target brackets, the line to the lead, and the lead ring"
        );
    }

    #[test]
    fn another_ground_vehicle_draws_as_a_car_that_drives_on_with_its_speed() {
        let overlays = Overlays::from_view(&json!({
            "fenceTop": 50.0,
            "cars": [{ "latitude": -35.362, "longitude": 149.165, "aboveHome": 0.0, "northSpeed": 5.0, "eastSpeed": 0.0, "ageMs": 0.0 }],
        }))
        .expect("overlays");
        assert!(
            overlays.animating(),
            "a moving car keeps the view repainting"
        );
        let drawn = meshes(&overlays, 600.0, &camera());
        let layers: Vec<(Layer, usize)> = drawn
            .iter()
            .map(|mesh| (mesh.layer, mesh.indices.len() / 6))
            .collect();
        assert_eq!(
            layers,
            vec![(Layer::Opaque, 30), (Layer::Translucent, 1)],
            "a body, a glass cabin and four wheels, five lit faces each, over a contact shadow"
        );
        let body = &drawn[0];
        assert_eq!(body.normals.len(), body.positions.len(), "the car is lit");
        let size: Vec<f32> = (0..3).map(|i| body.high[i] - body.low[i]).collect();
        assert!(
            [1.44, 1.96, 4.5]
                .iter()
                .all(|want| size.iter().any(|got| (got - want).abs() < 0.05)),
            "a 4.5 x 1.96 x 1.44 m car, wheels included: {size:?}"
        );
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
    fn an_aircraft_whose_position_is_unreliable_is_drawn_grey_and_says_so_instead_of_a_height() {
        let camera = camera();
        let at = (-35.364, 149.164, 680.0);
        let sure = aircraft(at, 90.0, Some(600.0), &camera, false);
        let unsure = aircraft(at, 90.0, Some(600.0), &camera, true);
        let colours = |meshes: &[Arc<Mesh>]| {
            meshes[..3]
                .iter()
                .map(|mesh| {
                    mesh.take_texture().and_then(|mips| {
                        mips.first()
                            .map(|t| t.get_pixel(t.width() / 2, t.height() / 2).0)
                    })
                })
                .collect::<Vec<_>>()
        };
        assert!(
            colours(&sure).iter().all(|c| *c == Some(AIRCRAFT)),
            "the marker, nose and drop line are orange"
        );
        assert!(
            colours(&unsure).iter().all(|c| *c == Some(UNSURE)),
            "and grey when the position is unreliable"
        );
        assert_eq!(
            unsure[3].indices.len(),
            UNSURE_LABEL.len() * 6,
            "the label reads the warning, a letter per quad"
        );
    }

    #[test]
    fn the_route_hangs_a_curtain_and_each_waypoint_a_stem_down_past_the_ground_so_the_terrain_cuts_them_where_they_meet_it()
     {
        let overlays = Overlays::from_view(&view()).unwrap();
        let drawn = meshes(&overlays, GROUND, &camera());
        let lowest = |layer: Layer| {
            drawn
                .iter()
                .filter(|mesh| mesh.layer == layer)
                .flat_map(|mesh| {
                    mesh.positions.iter().map(|p| {
                        let at = mesh.to_ecef.transform_point3(DVec3::new(
                            f64::from(p[0]),
                            f64::from(p[1]),
                            f64::from(p[2]),
                        ));
                        at.length() - ecef(SPOT.0, SPOT.1, 0.0).length()
                    })
                })
                .fold(f64::MAX, f64::min)
        };
        assert!(
            lowest(Layer::Translucent) < GROUND - CURTAIN_FOOT_M / 2.0,
            "the curtain reaches well below the home ground"
        );
        assert!(
            lowest(Layer::Opaque) < GROUND - CURTAIN_FOOT_M / 2.0,
            "and so do the stems"
        );
    }

    #[test]
    fn label_glyphs_are_white_letters_on_a_soft_dark_halo_that_fades_out() {
        let set = &*LABEL_GLYPHS;
        let one = set
            .glyphs
            .iter()
            .find(|glyph| glyph.character == '1')
            .expect("a '1'");
        let column = |x: u32| {
            (0..set.atlas.height())
                .map(|y| set.atlas.get_pixel(one.left + x, y).0)
                .collect::<Vec<_>>()
        };
        let pixels: Vec<[u8; 4]> = (0..one.width).flat_map(column).collect();
        assert!(
            pixels.iter().any(|p| *p == [255, 255, 255, 255]),
            "solid white stroke"
        );
        assert!(
            pixels
                .iter()
                .any(|p| p[0] < 40 && (60..200).contains(&p[3])),
            "a dark, partly see-through halo around it"
        );
        assert_eq!(
            set.atlas.get_pixel(one.left, 0).0[3],
            0,
            "clear at the cell's corner"
        );
    }

    #[test]
    fn a_label_spaces_its_letters_by_their_own_widths_and_centres_on_its_point() {
        let camera = camera();
        let at = camera.eye + camera.forward * 100.0;
        let right = camera.forward.cross(camera.up).normalize();
        let along = |quad: &[DVec3; 4]| (quad.iter().sum::<DVec3>() / 4.0 - at).dot(right);
        let quads = label(&camera, at, "1 m", LABEL_PX);
        assert_eq!(quads.len(), 3, "a quad per letter, the space included");
        let width = |quad: &[DVec3; 4]| (quad[1] - quad[0]).length();
        assert!(
            width(&quads[2].0) > width(&quads[0].0),
            "an 'm' is wider than a '1'"
        );
        let (first, last) = (along(&quads[0].0), along(&quads[2].0));
        assert!(
            first < 0.0 && last > 0.0,
            "the text runs left to right across its point"
        );
        assert!(
            (first + last).abs() < width(&quads[2].0),
            "and sits around it"
        );
    }

    const SPOT: (f64, f64) = (-35.363, 149.165);
    const GROUND: f64 = 600.0;

    #[test]
    fn the_aircraft_shows_its_nose_a_drop_line_and_its_height_above_the_ground_in_metres() {
        let camera = Camera::new(
            &Pose {
                latitude: SPOT.0 - 0.01,
                longitude: SPOT.1,
                height: GROUND + 400.0,
                heading: 0.0,
                pitch: -20.0,
                roll: 0.0,
                fov: 60.0,
            },
            1280,
            720,
        );
        let at = (SPOT.0, SPOT.1, GROUND + 249.0);
        let marked = aircraft(at, 90.0, Some(GROUND), &camera, false);
        assert_eq!(marked.len(), 4, "drop line, nose, marker and label");
        assert_eq!(
            marked[3].indices.len(),
            "249 m".len() * 6,
            "one quad per character of the height, spaced from its unit"
        );
        assert_eq!(
            aircraft(at, 90.0, None, &camera, false).len(),
            2,
            "with the ground unknown there is no drop line and no height to give"
        );
    }

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
