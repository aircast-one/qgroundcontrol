use std::sync::Arc;

use glam::{DVec3, Vec3};

use super::geo::{east_north_up, ecef};
use super::mesh::{Layer, Mesh};

const PROBE_HEIGHT_M: f64 = 10_000.0;
const EARTHLY_M: std::ops::RangeInclusive<f64> = -1_000.0..=9_000.0;

fn hit_box(origin: Vec3, direction: Vec3, low: [f32; 3], high: [f32; 3]) -> bool {
    let (near, far) = (0..3).fold((f32::MIN, f32::MAX), |(near, far), axis| {
        let inverse = 1.0 / direction[axis];
        let (a, b) = (
            (low[axis] - origin[axis]) * inverse,
            (high[axis] - origin[axis]) * inverse,
        );
        (near.max(a.min(b)), far.min(a.max(b)))
    });
    near <= far && far >= 0.0
}

fn hit_triangle(origin: DVec3, direction: DVec3, a: DVec3, b: DVec3, c: DVec3) -> Option<f64> {
    let (edge1, edge2) = (b - a, c - a);
    let p = direction.cross(edge2);
    let determinant = edge1.dot(p);
    (determinant.abs() > 1e-12).then_some(())?;
    let inverse = 1.0 / determinant;
    let s = origin - a;
    let u = s.dot(p) * inverse;
    (0.0..=1.0).contains(&u).then_some(())?;
    let q = s.cross(edge1);
    let v = direction.dot(q) * inverse;
    (v >= 0.0 && u + v <= 1.0).then_some(())?;
    let t = edge2.dot(q) * inverse;
    (t > 0.0).then_some(t)
}

fn distance_along(mesh: &Mesh, origin: DVec3, direction: DVec3) -> Option<f64> {
    let to_local = mesh.to_ecef.inverse();
    let (local_origin, local_direction) = (
        to_local.transform_point3(origin),
        to_local.transform_vector3(direction),
    );
    hit_box(
        local_origin.as_vec3(),
        local_direction.as_vec3(),
        mesh.low,
        mesh.high,
    )
    .then_some(())?;
    let corner = |index: u32| {
        mesh.positions
            .get(index as usize)
            .map(|p| DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2])))
    };
    let scale = local_direction.length();
    mesh.indices
        .chunks_exact(3)
        .filter_map(|triangle| {
            hit_triangle(
                local_origin,
                local_direction,
                corner(triangle[0])?,
                corner(triangle[1])?,
                corner(triangle[2])?,
            )
        })
        .min_by(f64::total_cmp)
        .map(|t| t * scale)
}

pub fn height_at(meshes: &[Arc<Mesh>], latitude: f64, longitude: f64) -> Option<f64> {
    let origin = ecef(latitude, longitude, PROBE_HEIGHT_M);
    let down = -east_north_up(latitude, longitude).z_axis;
    meshes
        .iter()
        .filter(|mesh| mesh.layer != Layer::Foliage)
        .filter_map(|mesh| distance_along(mesh, origin, down))
        .min_by(f64::total_cmp)
        .map(|metres| PROBE_HEIGHT_M - metres)
        .filter(|height| EARTHLY_M.contains(height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{DMat3, DMat4};

    #[test]
    fn a_ray_from_above_finds_the_height_of_the_ground_under_it() {
        let (latitude, longitude, ground) = (-35.36, 149.16, 604.0);
        let frame = east_north_up(latitude, longitude);
        let to_ecef = DMat4::from_mat3_translation(
            DMat3::from_cols(frame.x_axis, frame.y_axis, frame.z_axis),
            ecef(latitude, longitude, ground),
        );
        let positions = vec![[-50.0, -50.0, 0.0], [50.0, -50.0, 0.0], [0.0, 50.0, 0.0]];
        let mesh = Arc::new(Mesh::new(
            to_ecef,
            positions,
            vec![[0.0; 2]; 3],
            vec![0, 1, 2],
            image::RgbaImage::new(1, 1),
            Layer::Opaque,
        ));
        let found = height_at(std::slice::from_ref(&mesh), latitude, longitude).expect("a hit");
        assert!((found - ground).abs() < 0.01, "found {found}");
        assert_eq!(
            height_at(&[mesh], latitude + 0.01, longitude),
            None,
            "a kilometre away there is no ground loaded"
        );
    }

    #[test]
    fn a_coarse_tile_far_below_the_real_surface_is_no_ground_to_stand_on() {
        let (latitude, longitude) = (47.397, 8.5455);
        let frame = east_north_up(latitude, longitude);
        let chord = |depth: f64| {
            let to_ecef = DMat4::from_mat3_translation(
                DMat3::from_cols(frame.x_axis, frame.y_axis, frame.z_axis),
                ecef(latitude, longitude, depth),
            );
            Arc::new(Mesh::new(
                to_ecef,
                vec![[-5e5, -5e5, 0.0], [5e5, -5e5, 0.0], [0.0, 5e5, 0.0]],
                vec![[0.0; 2]; 3],
                vec![0, 1, 2],
                image::RgbaImage::new(1, 1),
                Layer::Opaque,
            ))
        };
        assert_eq!(
            height_at(&[chord(-22_600.0)], latitude, longitude),
            None,
            "the flat face of a globe-sized tile sags 22 km under Zurich; standing on it puts the camera inside the earth"
        );
        assert_eq!(
            height_at(&[chord(-12_700_000.0)], latitude, longitude),
            None,
            "nor the far side of the planet"
        );
    }
}
