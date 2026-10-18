use std::sync::Arc;

use glam::{DMat3, DMat4, DVec3};
use serde_json::Value;

use super::geo::{Camera, east_north_up, ecef};
use super::mesh::{Layer, Mesh, solid};

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
pub struct Overlays {
    pub home: Option<(f64, f64)>,
    pub route: Vec<Spot>,
    pub waypoints: Vec<(Spot, String)>,
    pub fences: Vec<Fence>,
    pub fence_top: f64,
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
        })
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
    let Some((origin_lat, origin_lng)) = overlays
        .home
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
    ]
    .into_iter()
    .flatten()
    .collect()
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
}
