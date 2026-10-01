use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};

use crate::cameradef::{CameraParameters, Source};
use crate::router::Backend;

pub const DEPS: &[&str] = &[];
const DEFINITION_TIMEOUT: Duration = Duration::from_secs(15);
const MAVLINK_FTP_SCHEME: &str = "mftp://";
const DEFINITION_LOCALE: &str = "en_us";

enum Entry {
    Fetching,
    Failed(String),
    Ready { parameters: Box<CameraParameters>, uri: String },
}

static STORE: Mutex<BTreeMap<(u8, u8), Entry>> = Mutex::new(BTreeMap::new());

pub fn cache_file_name(vendor: &str, model: &str, version: u16) -> String {
    format!("{vendor}_{model}_{version:03}.xml")
}

fn cache_path(vendor: &str, model: &str, version: u16) -> Option<std::path::PathBuf> {
    let folder = crate::settingsstore::parameter_save_path()?;
    Some(std::path::Path::new(&folder).join(cache_file_name(vendor, model, version)))
}

fn cached(path: Option<&std::path::Path>) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path?).ok()?;
    crate::cameradef::from_bytes(&bytes, DEFINITION_LOCALE).is_ok().then_some(bytes)
}

fn download(uri: &str) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(DEFINITION_TIMEOUT)).build().into();
    let mut answer = agent.get(uri).call().map_err(|e| format!("Camera definition ({uri}) download error: {e}"))?;
    answer.body_mut().read_to_vec().map_err(|e| format!("Camera definition ({uri}) download error: {e}"))
}

pub struct Wanted {
    pub vehicle: u8,
    pub compid: u8,
    pub link: u32,
    pub vendor: String,
    pub model: String,
    pub version: u16,
    pub uri: String,
}

pub fn wanted(camera: Wanted) {
    let key = (camera.vehicle, camera.compid);
    {
        let mut store = STORE.lock().unwrap_or_else(PoisonError::into_inner);
        if store.contains_key(&key) || camera.uri.is_empty() || !crate::vehiclefacade::switched_on() {
            return;
        }
        store.insert(key, Entry::Fetching);
    }
    std::thread::Builder::new().name("groundstation-camera-definition".to_string()).spawn(move || fetch(camera)).expect("camera definition thread");
}

fn fetch(camera: Wanted) {
    let path = cache_path(&camera.vendor, &camera.model, camera.version);
    let bytes = match cached(path.as_deref()) {
        Some(bytes) => Ok(bytes),
        None if camera.uri.to_ascii_lowercase().starts_with(MAVLINK_FTP_SCHEME) => Err("Camera definitions over MAVLink FTP are not loaded yet.".to_string()),
        None => download(&camera.uri).inspect(|bytes| {
            if let Some(path) = &path {
                let _ = std::fs::create_dir_all(path.parent().unwrap_or(path)).and_then(|()| std::fs::write(path, bytes));
            }
        }),
    };
    let entry = bytes.and_then(|b| crate::cameradef::from_bytes(&b, DEFINITION_LOCALE).map_err(|refusal| refusal.detail)).map(|definition| Entry::Ready { parameters: Box::new(CameraParameters::new(definition)), uri: camera.uri.clone() });
    let ready = matches!(entry, Ok(Entry::Ready { ref parameters, .. }) if !parameters.is_basic());
    STORE.lock().unwrap_or_else(PoisonError::into_inner).insert((camera.vehicle, camera.compid), entry.unwrap_or_else(Entry::Failed));
    if ready {
        request_all(camera.vehicle, camera.compid, camera.link);
    }
}

pub fn request_all(vehicle: u8, compid: u8, link: u32) {
    if let Some(bytes) = crate::mavout::encode_next(&crate::mavout::Outbound::ParamExtRequestList { target: (vehicle, compid) }) {
        crate::linkhost::write(&crate::linkhost::TRANSPORTS, link, &bytes);
    }
}

pub fn parameter_name(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw).trim_end_matches('\0').to_string()
}

pub fn on_value(vehicle: u8, compid: u8, name: &str, param_type: u8, raw: &[u8], now_ms: u64) {
    let Some(value) = crate::cameradef::decode_param_value(param_type, raw) else { return };
    if let Some(Entry::Ready { parameters, .. }) = STORE.lock().unwrap_or_else(PoisonError::into_inner).get_mut(&(vehicle, compid)) {
        parameters.on_value(name, value, Source::Camera, now_ms);
    }
}

fn selected_camera() -> Option<(u8, u8)> {
    let hub = crate::hub::lock();
    let vehicle = hub.active()?;
    let index = vehicle.cameras.selected_index()?;
    Some((vehicle.id, vehicle.cameras.compid_at(index)?))
}

pub fn camera_settings_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    let unavailable = |state: &str, error: &str| json!({ "kind": "object", "class": "CameraSettings", "state": state, "error": error });
    let Some(key) = selected_camera() else { return unavailable("noCamera", "") };
    let store = STORE.lock().unwrap_or_else(PoisonError::into_inner);
    match store.get(&key) {
        None => unavailable("noDefinition", ""),
        Some(Entry::Fetching) => unavailable("fetching", ""),
        Some(Entry::Failed(error)) => unavailable("failed", error),
        Some(Entry::Ready { parameters, uri }) => {
            let mut shown = crate::cameradef::definition_json(parameters, uri, crate::hub::now_ms());
            shown["class"] = json!("CameraSettings");
            shown["state"] = json!("ready");
            shown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cache_file_is_named_like_qgcs() {
        assert_eq!(cache_file_name("Yuneec", "E90", 7), "Yuneec_E90_007.xml");
    }

    #[test]
    fn a_parameter_id_stops_at_its_terminator() {
        assert_eq!(parameter_name(b"CAM_EV\0\0\0\0\0\0\0\0\0\0"), "CAM_EV");
        assert_eq!(parameter_name(b"SIXTEEN_CHARS_ID"), "SIXTEEN_CHARS_ID", "a full 16-byte id has no terminator");
    }
}
