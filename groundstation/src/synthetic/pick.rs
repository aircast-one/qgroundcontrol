use std::sync::{Arc, Mutex, PoisonError};

use glam::DVec4;
use serde_json::{Value, json};

use super::geo::{Camera, Pose};
use super::ground;
use super::mesh::Mesh;

type Seen = (Pose, Vec<Arc<Mesh>>);

static SEEN: Mutex<Option<Seen>> = Mutex::new(None);

pub fn seen(eye: Pose, ground: &[Arc<Mesh>]) {
    *SEEN.lock().unwrap_or_else(PoisonError::into_inner) = Some((eye, ground.to_vec()));
}

pub fn ground_at(
    eye: &Pose,
    meshes: &[Arc<Mesh>],
    [x, y, width, height]: [f64; 4],
) -> Option<(f64, f64)> {
    let camera = Camera::new(eye, width as u32, height as u32);
    let through =
        camera.unproject() * DVec4::new(2.0 * x / width - 1.0, 1.0 - 2.0 * y / height, 0.5, 1.0);
    let direction = (through.truncate() / through.w).normalize();
    ground::nearest_hit(meshes, camera.eye, direction).map(|metres| {
        let at = camera.eye + direction * metres;
        let (latitude, longitude, _) = crate::geo::geodetic(at.x, at.y, at.z);
        (latitude, longitude)
    })
}

pub fn run(given: &[f64]) -> Value {
    let seen = SEEN.lock().unwrap_or_else(PoisonError::into_inner).clone();
    match (given, seen) {
        ([x, y, width, height], Some((eye, meshes))) if *width >= 1.0 && *height >= 1.0 => {
            ground_at(&eye, &meshes, [*x, *y, *width, *height]).map_or_else(
                || json!({ "ok": false, "reason": "There is no ground under that point." }),
                |(latitude, longitude)| {
                    json!({ "ok": true, "latitude": latitude, "longitude": longitude })
                },
            )
        }
        ([_, _, _, _], None) => json!({ "ok": false, "reason": "The 3D map is not showing." }),
        _ => json!({
            "ok": false,
            "reason": "map3d.pick needs the x and y of the press and the view width and height, in pixels"
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::geo::{east_north_up, ecef};
    use crate::synthetic::mesh::Layer;
    use glam::{DMat3, DMat4};

    fn flat_ground(latitude: f64, longitude: f64, height: f64) -> Arc<Mesh> {
        let frame = east_north_up(latitude, longitude);
        Arc::new(Mesh::new(
            DMat4::from_mat3_translation(
                DMat3::from_cols(frame.x_axis, frame.y_axis, frame.z_axis),
                ecef(latitude, longitude, height),
            ),
            vec![
                [-5000.0, -5000.0, 0.0],
                [5000.0, -5000.0, 0.0],
                [5000.0, 5000.0, 0.0],
                [-5000.0, 5000.0, 0.0],
            ],
            vec![[0.0; 2]; 4],
            vec![0, 1, 2, 0, 2, 3],
            image::RgbaImage::new(1, 1),
            Layer::Opaque,
        ))
    }

    #[test]
    fn a_press_finds_the_ground_spot_the_3d_camera_shows_under_it() {
        let (latitude, longitude, ground) = (-35.363, 149.165, 584.0);
        let meshes = [flat_ground(latitude, longitude, ground)];
        let looking_north = Pose {
            latitude,
            longitude,
            height: ground + 100.0,
            heading: 0.0,
            pitch: -45.0,
            roll: 0.0,
            fov: crate::map3d::FOV_DEG,
        };
        let (centre_lat, centre_lon) =
            ground_at(&looking_north, &meshes, [640.0, 360.0, 1280.0, 720.0])
                .expect("ground ahead");
        let ahead = crate::track::distance_m((latitude, longitude), (centre_lat, centre_lon));
        assert!(
            (ahead - 100.0).abs() < 1.0 && centre_lat > latitude,
            "45 degrees down from 100 m up, the middle of the view is 100 m north; got {ahead}"
        );
        let (lower_lat, _) = ground_at(&looking_north, &meshes, [640.0, 700.0, 1280.0, 720.0])
            .expect("ground below");
        assert!(
            lower_lat < centre_lat,
            "lower on the screen is nearer the camera"
        );
        let (_, right_lon) = ground_at(&looking_north, &meshes, [1200.0, 360.0, 1280.0, 720.0])
            .expect("ground right");
        assert!(
            right_lon > centre_lon,
            "looking north, the right of the screen is east"
        );
        let nearly_level = Pose {
            pitch: -5.0,
            ..looking_north
        };
        assert_eq!(
            ground_at(&nearly_level, &meshes, [640.0, 0.0, 1280.0, 720.0]),
            None,
            "looking nearly level, the top edge is sky"
        );
    }
}
