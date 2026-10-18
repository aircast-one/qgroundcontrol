use glam::dcamera::rh::{proj::directx, view};
use glam::{DMat3, DMat4, DVec3};

const WGS84_A: f64 = 6_378_137.0;
const WGS84_E2: f64 = 6.694_379_990_14e-3;
const SUN_EAST_NORTH_UP: DVec3 = DVec3::new(-0.6, 0.8, 1.0);

pub fn ecef(latitude: f64, longitude: f64, height: f64) -> DVec3 {
    let (lat, lng) = (latitude.to_radians(), longitude.to_radians());
    let n = WGS84_A / (1.0 - WGS84_E2 * lat.sin().powi(2)).sqrt();
    DVec3::new(
        (n + height) * lat.cos() * lng.cos(),
        (n + height) * lat.cos() * lng.sin(),
        (n * (1.0 - WGS84_E2) + height) * lat.sin(),
    )
}

pub fn east_north_up(latitude: f64, longitude: f64) -> DMat3 {
    let (lat, lng) = (latitude.to_radians(), longitude.to_radians());
    DMat3::from_cols(
        DVec3::new(-lng.sin(), lng.cos(), 0.0),
        DVec3::new(-lat.sin() * lng.cos(), -lat.sin() * lng.sin(), lat.cos()),
        DVec3::new(lat.cos() * lng.cos(), lat.cos() * lng.sin(), lat.sin()),
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub latitude: f64,
    pub longitude: f64,
    pub height: f64,
    pub heading: f64,
    pub pitch: f64,
    pub roll: f64,
    pub fov: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub eye: DVec3,
    pub forward: DVec3,
    pub up: DVec3,
    pub zenith: DVec3,
    pub sun: DVec3,
    pub fovy: f64,
    pub aspect: f64,
    pub height_px: f64,
}

impl Camera {
    pub fn new(pose: &Pose, width_px: u32, height_px: u32) -> Self {
        let frame = east_north_up(pose.latitude, pose.longitude);
        let (heading, pitch, roll) = (
            pose.heading.to_radians(),
            pose.pitch.to_radians(),
            pose.roll.to_radians(),
        );
        let forward = DVec3::new(
            heading.sin() * pitch.cos(),
            heading.cos() * pitch.cos(),
            pitch.sin(),
        );
        let level_right = DVec3::new(heading.cos(), -heading.sin(), 0.0);
        let level_up = level_right.cross(forward);
        let up = level_up * roll.cos() + level_right * roll.sin();
        let aspect = f64::from(width_px) / f64::from(height_px.max(1));
        Self {
            eye: ecef(pose.latitude, pose.longitude, pose.height),
            forward: frame * forward,
            up: frame * up,
            zenith: frame * DVec3::Z,
            sun: (frame * SUN_EAST_NORTH_UP).normalize(),
            fovy: 2.0 * ((pose.fov.to_radians() / 2.0).tan() / aspect).atan(),
            aspect,
            height_px: f64::from(height_px),
        }
    }

    pub fn view(&self) -> DMat4 {
        view::look_to_mat4(DVec3::ZERO, self.forward, self.up)
    }

    pub fn unproject(&self) -> DMat4 {
        (self.projection() * self.view()).inverse()
    }

    pub fn projection(&self) -> DMat4 {
        directx::perspective_infinite_reverse(self.fovy, self.aspect, 0.5)
    }

    pub fn culling(&self) -> DMat4 {
        directx::perspective(self.fovy, self.aspect, 0.5, 1.0e8) * self.view()
    }

    pub fn screen_space_error(&self, geometric_error: f64, distance: f64) -> f64 {
        geometric_error * self.height_px / (distance.max(1.0e-3) * 2.0 * (self.fovy / 2.0).tan())
    }

    pub fn relative(&self, to_ecef: DMat4) -> DMat4 {
        DMat4::from_translation(-self.eye) * to_ecef
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmac_lands_where_google_tiles_put_it() {
        let at = ecef(-35.363262, 149.165237, 604.0);
        assert!(
            (at - DVec3::new(-4_471_570.0, 2_669_235.0, -3_671_130.0)).length() < 2_000.0,
            "{at}"
        );
    }

    #[test]
    fn a_level_camera_looks_along_its_heading_with_up_away_from_the_earth() {
        let camera = Camera::new(
            &Pose {
                latitude: 0.0,
                longitude: 0.0,
                height: 0.0,
                heading: 90.0,
                pitch: 0.0,
                roll: 0.0,
                fov: 60.0,
            },
            1280,
            720,
        );
        assert!(
            (camera.forward - DVec3::Y).length() < 1e-9,
            "east at the origin is +Y in ECEF, got {}",
            camera.forward
        );
        assert!(
            (camera.up - DVec3::X).length() < 1e-9,
            "up at the origin is +X in ECEF, got {}",
            camera.up
        );
    }

    #[test]
    fn the_sun_shines_from_high_in_the_north_west_wherever_the_camera_is() {
        let camera = Camera::new(
            &Pose {
                latitude: 0.0,
                longitude: 0.0,
                height: 0.0,
                heading: 0.0,
                pitch: 0.0,
                roll: 0.0,
                fov: 60.0,
            },
            1280,
            720,
        );
        assert!(
            (camera.zenith - DVec3::X).length() < 1e-9,
            "straight up at the origin is +X in ECEF, got {}",
            camera.zenith
        );
        assert!(
            (camera.sun.dot(camera.zenith) - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9,
            "45 degrees above the horizon"
        );
        assert!(
            camera.sun.y < 0.0 && camera.sun.z > 0.0,
            "west (-Y) and north (+Z) at the origin, got {}",
            camera.sun
        );
    }

    #[test]
    fn looking_back_through_the_screen_finds_the_way_the_camera_faces() {
        let camera = Camera::new(
            &Pose {
                latitude: 0.0,
                longitude: 0.0,
                height: 0.0,
                heading: 90.0,
                pitch: 0.0,
                roll: 0.0,
                fov: 60.0,
            },
            1280,
            720,
        );
        let through = camera.unproject() * glam::DVec4::new(0.0, 0.0, 0.5, 1.0);
        assert!(
            ((through.truncate() / through.w).normalize() - camera.forward).length() < 1e-9,
            "the middle of the screen looks along the camera's forward"
        );
    }

    #[test]
    fn banked_right_the_top_of_the_frame_leans_right_so_the_horizon_rises_on_the_right() {
        let camera = Camera::new(
            &Pose {
                latitude: 0.0,
                longitude: 0.0,
                height: 0.0,
                heading: 0.0,
                pitch: 0.0,
                roll: 90.0,
                fov: 60.0,
            },
            1280,
            720,
        );
        assert!(
            (camera.up - DVec3::Y).length() < 1e-9,
            "facing north with the right wing straight down, the top of the frame points east (+Y in ECEF), got {}",
            camera.up
        );
    }

    #[test]
    fn the_screen_space_error_halves_when_the_distance_doubles() {
        let camera = Camera::new(
            &Pose {
                latitude: 0.0,
                longitude: 0.0,
                height: 0.0,
                heading: 0.0,
                pitch: 0.0,
                roll: 0.0,
                fov: 60.0,
            },
            1280,
            720,
        );
        assert!(
            (camera.screen_space_error(10.0, 100.0) / camera.screen_space_error(10.0, 200.0) - 2.0)
                .abs()
                < 1e-12
        );
    }
}
