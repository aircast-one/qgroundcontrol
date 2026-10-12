use std::sync::Arc;

use glam::{DMat4, DVec3, DVec4};
use serde_json::Value;
use url::Url;

use super::geo::Camera;
use super::glb::{self, Mesh};

pub const GOOGLE_ROOT: &str = "https://tile.googleapis.com/v1/3dtiles/root.json";
pub const ION_GOOGLE_ASSET: u64 = 2_275_207;

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

    pub fn outside(&self, plane: DVec4, eye: DVec3) -> bool {
        let normal = plane.truncate();
        let reach: f64 = self.half.iter().map(|axis| normal.dot(*axis).abs()).sum();
        normal.dot(self.center - eye) + plane.w < -reach
    }
}

pub fn frustum(culling: DMat4) -> [DVec4; 5] {
    let rows = [culling.row(0), culling.row(1), culling.row(2), culling.row(3)];
    [rows[3] + rows[0], rows[3] - rows[0], rows[3] + rows[1], rows[3] - rows[1], rows[2]].map(|plane| plane / plane.truncate().length())
}

enum Content {
    None,
    Pending(Url),
    Requested(Url),
    External,
    Meshes(Vec<Arc<Mesh>>),
    Failed,
}

pub struct Request {
    pub path: Vec<usize>,
    pub url: Url,
}

pub struct Tile {
    bounds: Obb,
    geometric_error: f64,
    content: Content,
    children: Vec<Tile>,
}

pub struct Tileset {
    root: Tile,
    session: Option<(String, String)>,
    keep: Vec<(String, String)>,
    pub maximum_screen_space_error: f64,
}

fn query(url: &Url) -> Vec<(String, String)> {
    url.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect()
}

fn tile(value: &Value, base: &Url) -> Option<Tile> {
    let bounds = Obb::from_box(&value.get("boundingVolume")?.get("box")?.as_array()?.iter().filter_map(Value::as_f64).collect::<Vec<_>>())?;
    let content = value
        .get("content")
        .and_then(|content| content.get("uri").or_else(|| content.get("url")))
        .and_then(Value::as_str)
        .and_then(|uri| base.join(uri).ok())
        .map_or(Content::None, Content::Pending);
    let children = value.get("children").and_then(Value::as_array).map(|children| children.iter().filter_map(|child| tile(child, base)).collect()).unwrap_or_default();
    Some(Tile { bounds, geometric_error: value.get("geometricError").and_then(Value::as_f64).unwrap_or(0.0), content, children })
}

pub fn root(json: &[u8], url: &Url) -> Option<Tile> {
    tile(serde_json::from_slice::<Value>(json).ok()?.get("root")?, url)
}

impl Tileset {
    pub fn new(root: Tile, url: &Url) -> Self {
        Self { root, session: None, keep: query(url), maximum_screen_space_error: 16.0 }
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

    pub fn select(&mut self, camera: &Camera, budget: usize) -> (Vec<Arc<Mesh>>, Vec<Request>) {
        let planes = frustum(camera.culling());
        let mut wanted = Vec::new();
        let mut root = std::mem::replace(&mut self.root, Tile { bounds: Obb { center: DVec3::ZERO, half: [DVec3::ZERO; 3] }, geometric_error: 0.0, content: Content::None, children: Vec::new() });
        let drawn = self.visit(&mut root, &mut Vec::new(), camera, &planes, budget, &mut wanted).unwrap_or_default();
        self.root = root;
        (drawn, wanted)
    }

    pub fn deliver(&mut self, path: &[usize], bytes: Option<Vec<u8>>) {
        let Some(tile) = path.iter().try_fold(&mut self.root, |tile, &child| tile.children.get_mut(child)) else { return };
        let Content::Requested(url) = &tile.content else { return };
        tile.content = match bytes {
            None => Content::Failed,
            Some(bytes) => match url.path().ends_with(".json") {
                true => root(&bytes, url).map_or(Content::Failed, |child| {
                    tile.children.push(child);
                    Content::External
                }),
                false => Content::Meshes(glb::meshes(&bytes).into_iter().map(Arc::new).collect()),
            },
        };
    }

    fn request(&mut self, tile: &mut Tile, path: &[usize], budget: usize, wanted: &mut Vec<Request>) {
        let Content::Pending(url) = &tile.content else { return };
        if wanted.len() >= budget {
            return;
        }
        let signed = self.signed(url);
        wanted.push(Request { path: path.to_vec(), url: signed.clone() });
        tile.content = Content::Requested(signed);
    }

    fn visit(&mut self, tile: &mut Tile, path: &mut Vec<usize>, camera: &Camera, planes: &[DVec4; 5], budget: usize, wanted: &mut Vec<Request>) -> Option<Vec<Arc<Mesh>>> {
        if planes.iter().any(|plane| tile.bounds.outside(*plane, camera.eye)) {
            return Some(Vec::new());
        }
        self.request(tile, path, budget, wanted);
        let refine = camera.screen_space_error(tile.geometric_error, tile.bounds.distance(camera.eye)) > self.maximum_screen_space_error;
        let external = matches!(tile.content, Content::External);
        let children = (refine || external) && !tile.children.is_empty();
        let below: Option<Vec<Arc<Mesh>>> = match children {
            true => tile
                .children
                .iter_mut()
                .enumerate()
                .map(|(index, child)| {
                    path.push(index);
                    let drawn = self.visit(child, path, camera, planes, budget, wanted);
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
            Content::None | Content::Failed if !children => Some(Vec::new()),
            _ => None,
        };
        below.or(own)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::photoreal::geo::Pose;

    fn camera() -> Camera {
        Camera::new(&Pose { latitude: 0.0, longitude: 0.0, height: 100.0, heading: 0.0, pitch: 0.0, roll: 0.0, fov: 60.0 }, 1280, 720)
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
        let ahead = Obb { center: camera.eye + camera.forward * 500.0, half: [DVec3::X * 10.0, DVec3::Y * 10.0, DVec3::Z * 10.0] };
        let behind = Obb { center: camera.eye - camera.forward * 500.0, ..ahead };
        assert!(!planes.iter().any(|plane| ahead.outside(*plane, camera.eye)));
        assert!(planes.iter().any(|plane| behind.outside(*plane, camera.eye)));
    }

    #[test]
    fn every_request_carries_the_key_and_the_session_the_root_handed_out() {
        let root_url = Url::parse("https://tile.googleapis.com/v1/3dtiles/root.json?key=K").unwrap();
        let mut tiles = Tileset::new(Tile { bounds: Obb { center: DVec3::ZERO, half: [DVec3::ZERO; 3] }, geometric_error: 0.0, content: Content::None, children: Vec::new() }, &root_url);
        let first = tiles.signed(&Url::parse("https://tile.googleapis.com/v1/3dtiles/datasets/a/files/b.json?session=S").unwrap());
        let later = tiles.signed(&Url::parse("https://tile.googleapis.com/v1/3dtiles/datasets/a/files/c.glb").unwrap());
        assert_eq!(first.query(), Some("session=S&key=K"));
        assert_eq!(later.query(), Some("key=K&session=S"));
    }

    #[test]
    fn a_parent_stays_drawn_until_every_visible_child_has_loaded() {
        let camera = camera();
        let at = |metres: f64| Obb { center: camera.eye + camera.forward * metres, half: [DVec3::X * 50.0, DVec3::Y * 50.0, DVec3::Z * 50.0] };
        let parent_mesh = Arc::new(Mesh { to_ecef: DMat4::IDENTITY, low: [0.0; 3], high: [0.0; 3], positions: Vec::new(), uvs: Vec::new(), indices: Vec::new(), texture: image::RgbaImage::new(1, 1) });
        let child = |path: &str| Tile { bounds: at(200.0), geometric_error: 1.0, content: Content::Pending(Url::parse(&format!("https://x/{path}.glb")).unwrap()), children: Vec::new() };
        let root = Tile { bounds: at(200.0), geometric_error: 1000.0, content: Content::Meshes(vec![parent_mesh.clone()]), children: vec![child("a"), child("b")] };
        let mut tiles = Tileset::new(root, &Url::parse("https://x/root.json").unwrap());
        let (drawn, wanted) = tiles.select(&camera, 8);
        assert!(drawn.len() == 1 && Arc::ptr_eq(&drawn[0], &parent_mesh), "with nothing loaded below, the coarse parent fills the view");
        assert_eq!(wanted.iter().map(|request| request.path.clone()).collect::<Vec<_>>(), vec![vec![0], vec![1]], "both children are asked for");
        tiles.deliver(&[0], None);
        let (drawn, wanted) = tiles.select(&camera, 8);
        assert!(drawn.len() == 1 && wanted.is_empty(), "a child that failed and one still in flight keep the parent up without asking twice");
    }
}
