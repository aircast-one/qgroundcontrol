use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use glam::DMat4;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Backfill,
    Opaque,
    Foliage,
    Water,
    Translucent,
    OnTop,
}

pub struct Mesh {
    pub id: u64,
    pub to_ecef: DMat4,
    pub low: [f32; 3],
    pub high: [f32; 3],
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub layer: Layer,
    texture: Mutex<Option<Vec<image::RgbaImage>>>,
}

pub fn bounds(positions: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    positions
        .iter()
        .fold(([f32::MAX; 3], [f32::MIN; 3]), |(low, high), p| {
            (
                std::array::from_fn(|axis| low[axis].min(p[axis])),
                std::array::from_fn(|axis| high[axis].max(p[axis])),
            )
        })
}

pub fn mip_chain(base: image::RgbaImage) -> Vec<image::RgbaImage> {
    std::iter::successors(Some(base), |level| {
        let (width, height) = level.dimensions();
        (width > 1 || height > 1).then(|| {
            image::imageops::resize(
                level,
                (width / 2).max(1),
                (height / 2).max(1),
                image::imageops::FilterType::Triangle,
            )
        })
    })
    .collect()
}

pub fn solid(colour: [u8; 4]) -> image::RgbaImage {
    image::RgbaImage::from_pixel(1, 1, image::Rgba(colour))
}

impl Mesh {
    pub fn new(
        to_ecef: DMat4,
        positions: Vec<[f32; 3]>,
        uvs: Vec<[f32; 2]>,
        indices: Vec<u32>,
        texture: image::RgbaImage,
        layer: Layer,
    ) -> Self {
        let (low, high) = bounds(&positions);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            to_ecef,
            low,
            high,
            positions,
            uvs,
            normals: Vec::new(),
            indices,
            layer,
            texture: Mutex::new(Some(mip_chain(texture))),
        }
    }

    pub fn with_normals(self, normals: Vec<[f32; 3]>) -> Self {
        Self { normals, ..self }
    }

    pub fn take_texture(&self) -> Option<Vec<image::RgbaImage>> {
        self.texture
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh() -> Mesh {
        Mesh::new(
            DMat4::IDENTITY,
            vec![[0.0; 3]],
            vec![[0.0; 2]],
            vec![0],
            image::RgbaImage::new(8, 4),
            Layer::Opaque,
        )
    }

    #[test]
    fn every_mesh_gets_its_own_id_even_when_memory_is_reused() {
        let first = mesh().id;
        let second = mesh().id;
        assert_ne!(first, second);
    }

    #[test]
    fn a_texture_hands_over_its_mip_chain_once() {
        let mesh = mesh();
        let chain = mesh
            .take_texture()
            .expect("the first upload gets the texture");
        assert_eq!(
            chain
                .iter()
                .map(image::RgbaImage::dimensions)
                .collect::<Vec<_>>(),
            vec![(8, 4), (4, 2), (2, 1), (1, 1)]
        );
        assert!(
            mesh.take_texture().is_none(),
            "the CPU copy is gone after upload"
        );
    }
}
