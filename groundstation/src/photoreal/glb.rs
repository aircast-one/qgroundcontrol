use glam::{DMat4, DQuat, DVec3};
use serde_json::Value;

const GLB_MAGIC: u32 = 0x4654_6C67;
const JSON_CHUNK: u32 = 0x4E4F_534A;
const BIN_CHUNK: u32 = 0x004E_4942;
const FLOAT: u64 = 5126;
const UNSIGNED_BYTE: u64 = 5121;
const UNSIGNED_SHORT: u64 = 5123;
const UNSIGNED_INT: u64 = 5125;

const Y_UP_TO_Z_UP: DMat4 = DMat4::from_cols_array(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]);

pub struct Mesh {
    pub to_ecef: DMat4,
    pub low: [f32; 3],
    pub high: [f32; 3],
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub texture: image::RgbaImage,
}

struct Glb<'a> {
    json: Value,
    bin: &'a [u8],
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn split(bytes: &[u8]) -> Option<Glb<'_>> {
    (u32_at(bytes, 0)? == GLB_MAGIC).then_some(())?;
    let json_len = u32_at(bytes, 12)? as usize;
    (u32_at(bytes, 16)? == JSON_CHUNK).then_some(())?;
    let json = serde_json::from_slice(bytes.get(20..20 + json_len)?).ok()?;
    let bin_at = 20 + json_len;
    let bin = match u32_at(bytes, bin_at + 4) {
        Some(BIN_CHUNK) => bytes.get(bin_at + 8..bin_at + 8 + u32_at(bytes, bin_at)? as usize)?,
        _ => &[],
    };
    Some(Glb { json, bin })
}

fn number(value: &Value, key: &str) -> Option<u64> {
    value.get(key)?.as_u64()
}

fn view_bytes<'a>(glb: &'a Glb, view: u64) -> Option<(&'a [u8], usize)> {
    let view = glb.json.get("bufferViews")?.get(view as usize)?;
    let start = number(view, "byteOffset").unwrap_or(0) as usize;
    let length = number(view, "byteLength")? as usize;
    Some((glb.bin.get(start..start + length)?, number(view, "byteStride").unwrap_or(0) as usize))
}

fn component(bytes: &[u8], kind: u64, normalized: bool) -> Option<f32> {
    match kind {
        FLOAT => Some(f32::from_le_bytes(bytes.get(..4)?.try_into().ok()?)),
        UNSIGNED_SHORT => Some(f32::from(u16::from_le_bytes(bytes.get(..2)?.try_into().ok()?)) / if normalized { 65535.0 } else { 1.0 }),
        UNSIGNED_BYTE => Some(f32::from(*bytes.first()?) / if normalized { 255.0 } else { 1.0 }),
        _ => None,
    }
}

fn component_size(kind: u64) -> usize {
    match kind {
        UNSIGNED_BYTE => 1,
        UNSIGNED_SHORT => 2,
        _ => 4,
    }
}

fn floats<const N: usize>(glb: &Glb, accessor: u64) -> Option<Vec<[f32; N]>> {
    let accessor = glb.json.get("accessors")?.get(accessor as usize)?;
    let kind = number(accessor, "componentType")?;
    let normalized = accessor.get("normalized").and_then(Value::as_bool).unwrap_or(false);
    let (bytes, stride) = view_bytes(glb, number(accessor, "bufferView")?)?;
    let size = component_size(kind);
    let stride = if stride == 0 { size * N } else { stride };
    let offset = number(accessor, "byteOffset").unwrap_or(0) as usize;
    (0..number(accessor, "count")? as usize)
        .map(|item| {
            let at = offset + item * stride;
            let values: Option<Vec<f32>> = (0..N).map(|lane| component(bytes.get(at + lane * size..)?, kind, normalized)).collect();
            values?.try_into().ok()
        })
        .collect()
}

fn indices(glb: &Glb, accessor: u64) -> Option<Vec<u32>> {
    let accessor = glb.json.get("accessors")?.get(accessor as usize)?;
    let kind = number(accessor, "componentType")?;
    let (bytes, _) = view_bytes(glb, number(accessor, "bufferView")?)?;
    let offset = number(accessor, "byteOffset").unwrap_or(0) as usize;
    let size = component_size(kind);
    (0..number(accessor, "count")? as usize)
        .map(|item| {
            let at = offset + item * size;
            match kind {
                UNSIGNED_INT => Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?)),
                UNSIGNED_SHORT => Some(u32::from(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))),
                UNSIGNED_BYTE => Some(u32::from(*bytes.get(at)?)),
                _ => None,
            }
        })
        .collect()
}

fn texture(glb: &Glb, material: Option<u64>) -> Option<image::RgbaImage> {
    let material = glb.json.get("materials")?.get(material? as usize)?;
    let texture = number(material.get("pbrMetallicRoughness")?.get("baseColorTexture")?, "index")?;
    let source = number(glb.json.get("textures")?.get(texture as usize)?, "source")?;
    let image = glb.json.get("images")?.get(source as usize)?;
    let (bytes, _) = view_bytes(glb, number(image, "bufferView")?)?;
    Some(image::load_from_memory(bytes).ok()?.to_rgba8())
}

fn floats_of(node: &Value, key: &str) -> Option<Vec<f64>> {
    node.get(key)?.as_array()?.iter().map(Value::as_f64).collect()
}

fn local(node: &Value) -> DMat4 {
    match floats_of(node, "matrix").and_then(|m| <[f64; 16]>::try_from(m).ok()) {
        Some(matrix) => DMat4::from_cols_array(&matrix),
        None => {
            let translation = floats_of(node, "translation").and_then(|t| <[f64; 3]>::try_from(t).ok()).map_or(DVec3::ZERO, DVec3::from_array);
            let rotation = floats_of(node, "rotation").and_then(|r| <[f64; 4]>::try_from(r).ok()).map_or(DQuat::IDENTITY, DQuat::from_array);
            let scale = floats_of(node, "scale").and_then(|s| <[f64; 3]>::try_from(s).ok()).map_or(DVec3::ONE, DVec3::from_array);
            DMat4::from_scale_rotation_translation(scale, rotation, translation)
        }
    }
}

fn primitives(glb: &Glb, mesh: u64, to_ecef: DMat4) -> Vec<Mesh> {
    let listed = glb.json.get("meshes").and_then(|meshes| meshes.get(mesh as usize)).and_then(|mesh| mesh.get("primitives")).and_then(Value::as_array).cloned().unwrap_or_default();
    listed
        .iter()
        .filter_map(|primitive| {
            let attributes = primitive.get("attributes")?;
            let positions = floats::<3>(glb, number(attributes, "POSITION")?)?;
            let uvs = floats::<2>(glb, number(attributes, "TEXCOORD_0")?)?;
            let indices = match number(primitive, "indices") {
                Some(accessor) => indices(glb, accessor)?,
                None => (0..positions.len() as u32).collect(),
            };
            let (low, high) = bounds(&positions);
            Some(Mesh { to_ecef, low, high, positions, uvs, indices, texture: texture(glb, number(primitive, "material"))? })
        })
        .collect()
}

pub fn bounds(positions: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    positions.iter().fold(([f32::MAX; 3], [f32::MIN; 3]), |(low, high), p| (std::array::from_fn(|axis| low[axis].min(p[axis])), std::array::from_fn(|axis| high[axis].max(p[axis]))))
}

fn walk(glb: &Glb, node: u64, parent: DMat4) -> Vec<Mesh> {
    let Some(value) = glb.json.get("nodes").and_then(|nodes| nodes.get(node as usize)) else { return Vec::new() };
    let world = parent * local(value);
    let own = number(value, "mesh").map(|mesh| primitives(glb, mesh, world)).unwrap_or_default();
    let children = value.get("children").and_then(Value::as_array).cloned().unwrap_or_default();
    own.into_iter().chain(children.iter().filter_map(Value::as_u64).flat_map(|child| walk(glb, child, world))).collect()
}

pub fn meshes(bytes: &[u8]) -> Vec<Mesh> {
    let Some(glb) = split(bytes) else { return Vec::new() };
    let scene = number(&glb.json, "scene").unwrap_or(0);
    let roots: Vec<u64> = glb.json.get("scenes").and_then(|scenes| scenes.get(scene as usize)).and_then(|scene| scene.get("nodes")).and_then(Value::as_array).map(|nodes| nodes.iter().filter_map(Value::as_u64).collect()).unwrap_or_default();
    roots.into_iter().flat_map(|root| walk(&glb, root, Y_UP_TO_Z_UP)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glb(json: &Value, bin: &[u8]) -> Vec<u8> {
        let text = {
            let raw = serde_json::to_vec(json).unwrap();
            let padding = (4 - raw.len() % 4) % 4;
            raw.into_iter().chain(std::iter::repeat_n(b' ', padding)).collect::<Vec<u8>>()
        };
        let body: Vec<u8> = bin.iter().copied().chain(std::iter::repeat_n(0, (4 - bin.len() % 4) % 4)).collect();
        let total = 12 + 8 + text.len() + 8 + body.len();
        [GLB_MAGIC.to_le_bytes(), 2u32.to_le_bytes(), (total as u32).to_le_bytes(), (text.len() as u32).to_le_bytes(), JSON_CHUNK.to_le_bytes()]
            .concat()
            .into_iter()
            .chain(text)
            .chain([(body.len() as u32).to_le_bytes(), BIN_CHUNK.to_le_bytes()].concat())
            .chain(body)
            .collect()
    }

    fn png() -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255])).write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn a_google_style_tile_lands_in_ecef_with_its_texture() {
        let positions: Vec<u8> = [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]].iter().flatten().flat_map(|v| v.to_le_bytes()).collect();
        let uvs: Vec<u8> = [[0u16, 0], [65535, 0], [0, 65535]].iter().flatten().flat_map(|v| v.to_le_bytes()).collect();
        let picture = png();
        let bin: Vec<u8> = positions.iter().chain(&uvs).chain(&picture).copied().collect();
        let json = serde_json::json!({
            "scene": 0,
            "scenes": [{ "nodes": [0] }],
            "nodes": [{ "mesh": 0, "matrix": [1, 0, 0, 0, 0, 0, -1, 0, 0, 1, 0, 0, -4_471_593.497_109_432, -3_671_137.465_224_134, -2_669_273.854_156_598, 1] }],
            "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0, "TEXCOORD_0": 1 }, "material": 0 }] }],
            "materials": [{ "pbrMetallicRoughness": { "baseColorTexture": { "index": 0 } } }],
            "textures": [{ "source": 0 }],
            "images": [{ "bufferView": 2, "mimeType": "image/png" }],
            "accessors": [
                { "bufferView": 0, "componentType": FLOAT, "count": 3, "type": "VEC3" },
                { "bufferView": 1, "componentType": UNSIGNED_SHORT, "normalized": true, "count": 3, "type": "VEC2" }
            ],
            "bufferViews": [
                { "byteOffset": 0, "byteLength": positions.len() },
                { "byteOffset": positions.len(), "byteLength": uvs.len() },
                { "byteOffset": positions.len() + uvs.len(), "byteLength": picture.len() }
            ]
        });
        let meshes = meshes(&glb(&json, &bin));
        assert_eq!(meshes.len(), 1);
        let mesh = &meshes[0];
        let origin = mesh.to_ecef.transform_point3(DVec3::ZERO);
        assert!((origin - DVec3::new(-4_471_593.497_109_432, 2_669_273.854_156_598, -3_671_137.465_224_134)).length() < 1e-6, "the tile's origin sits at its ECEF spot to the millimetre, got {origin}");
        assert_eq!(mesh.indices, vec![0, 1, 2]);
        assert_eq!(mesh.uvs[1], [1.0, 0.0]);
        assert_eq!(mesh.texture.get_pixel(1, 1).0, [10, 20, 30, 255]);
    }

    #[test]
    fn something_that_is_not_a_glb_yields_nothing() {
        assert!(meshes(b"not a glb at all").is_empty());
    }
}
