use super::host::{Draw, Feed};

const SYNTHETIC_TOKEN: &str = "settings.appSettings.photorealToken";

pub struct CoreFeed;

impl Feed for CoreFeed {
    fn pose(&self) -> Option<serde_json::Value> {
        serde_json::from_str(&crate::abi::core_get("view.syntheticView")).ok()
    }

    fn overlays(&self) -> Option<serde_json::Value> {
        serde_json::from_str(&crate::abi::core_get("view.syntheticOverlays")).ok()
    }

    fn token(&self) -> String {
        crate::settingsstore::raw_setting(SYNTHETIC_TOKEN)
            .and_then(|value| value.as_str().map(str::trim).map(str::to_string))
            .unwrap_or_default()
    }

    fn sticks(&self) -> Vec<crate::virtualjoystick::StickSample> {
        crate::virtualjoystick::sent_sticks()
    }

    fn report(&self, draw: Draw) {
        crate::synthview::set_draw(draw.name());
    }

    fn ground(&self, above_home: Option<f64>) {
        crate::synthview::set_ground(above_home);
    }
}
