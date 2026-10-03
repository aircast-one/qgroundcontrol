use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};

use crate::cameradef::{CameraParameters, Out, Source};
use crate::mavout::Outbound;
use crate::router::Backend;

pub const DEPS: &[&str] = &[];
pub const SET_CAMERA_SETTING: &str = "cameraSettings.set";
const PARAM_ACK_ACCEPTED: u8 = 0;
const PARAM_ACK_FAILED: u8 = 2;
const PARAM_ACK_IN_PROGRESS: u8 = 3;
const DEFINITION_TIMEOUT: Duration = Duration::from_secs(15);
const MAVLINK_FTP_SCHEME: &str = "mftp://";
const DEFINITION_LOCALE: &str = "en_us";

struct Write {
    value: [u8; crate::cameradef::PARAM_VALUE_BYTES],
    param_type: u8,
    sent_ms: u64,
    retries: u32,
}

struct Ready {
    parameters: CameraParameters,
    uri: String,
    link: u32,
    writes: BTreeMap<String, Write>,
    updates_due: Option<u64>,
}

enum Entry {
    FtpWaiting(Wanted),
    FtpDownloading(Wanted),
    Fetching,
    Failed(String),
    Ready(Box<Ready>),
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

#[derive(Clone)]
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
        let over_ftp = camera.uri.to_ascii_lowercase().starts_with(MAVLINK_FTP_SCHEME) && cached(cache_path(&camera.vendor, &camera.model, camera.version).as_deref()).is_none();
        if over_ftp {
            store.insert(key, Entry::FtpWaiting(camera));
            return;
        }
        store.insert(key, Entry::Fetching);
    }
    std::thread::Builder::new().name("groundstation-camera-definition".to_string()).spawn(move || fetch(camera)).expect("camera definition thread");
}

fn start_ftp_downloads(now_ms: u64) {
    let waiting: Vec<Wanted> = STORE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .values()
        .filter_map(|entry| match entry {
            Entry::FtpWaiting(camera) => Some(camera.clone()),
            _ => None,
        })
        .collect();
    waiting.into_iter().for_each(|camera| {
        let action = json!({ "action": "ftp", "op": "download", "path": camera.uri, "cameraDefinition": camera.compid });
        let started = crate::hub::lock().guided(Some(camera.vehicle), &action, now_ms);
        let next = match started {
            Ok(frames) => {
                frames.iter().for_each(|(link, bytes)| {
                    crate::linkhost::write(&crate::linkhost::TRANSPORTS, *link, bytes);
                });
                Some(Entry::FtpDownloading(camera.clone()))
            }
            Err(reason) if reason == crate::hub::FILES_BUSY => None,
            Err(reason) => Some(Entry::Failed(reason)),
        };
        if let Some(next) = next {
            STORE.lock().unwrap_or_else(PoisonError::into_inner).insert((camera.vehicle, camera.compid), next);
        }
    });
}

pub fn ftp_finished(vehicle: u8, compid: u8, result: Result<crate::filejobs::Outcome, String>) {
    let Some(Entry::FtpDownloading(camera)) = STORE.lock().unwrap_or_else(PoisonError::into_inner).remove(&(vehicle, compid)) else { return };
    STORE.lock().unwrap_or_else(PoisonError::into_inner).insert((vehicle, compid), Entry::Fetching);
    std::thread::Builder::new()
        .name("groundstation-camera-definition".to_string())
        .spawn(move || {
            let bytes = match result {
                Ok(crate::filejobs::Outcome::Downloaded(bytes)) => crate::compmeta::inflate(&camera.uri, &bytes).map_err(|e| format!("Inflate of compressed xml failed: {e}")),
                Ok(_) => Err("The camera sent no definition file.".to_string()),
                Err(reason) => Err(reason),
            };
            if let (Ok(bytes), Some(path)) = (&bytes, cache_path(&camera.vendor, &camera.model, camera.version)) {
                let _ = std::fs::create_dir_all(path.parent().unwrap_or(&path)).and_then(|()| std::fs::write(&path, bytes));
            }
            settle(camera, bytes);
        })
        .expect("camera definition thread");
}

fn fetch(camera: Wanted) {
    let path = cache_path(&camera.vendor, &camera.model, camera.version);
    let bytes = match cached(path.as_deref()) {
        Some(bytes) => Ok(bytes),
        None => download(&camera.uri).inspect(|bytes| {
            if let Some(path) = &path {
                let _ = std::fs::create_dir_all(path.parent().unwrap_or(path)).and_then(|()| std::fs::write(path, bytes));
            }
        }),
    };
    settle(camera, bytes);
}

fn settle(camera: Wanted, bytes: Result<Vec<u8>, String>) {
    let entry = bytes.and_then(|b| crate::cameradef::from_bytes(&b, DEFINITION_LOCALE).map_err(|refusal| refusal.detail)).map(|definition| Entry::Ready(Box::new(Ready { parameters: CameraParameters::new(definition), uri: camera.uri.clone(), link: camera.link, writes: BTreeMap::new(), updates_due: None })));
    let ready = matches!(entry, Ok(Entry::Ready(ref ready)) if !ready.parameters.is_basic());
    STORE.lock().unwrap_or_else(PoisonError::into_inner).insert((camera.vehicle, camera.compid), entry.unwrap_or_else(Entry::Failed));
    if ready {
        request_all(camera.vehicle, camera.compid, camera.link);
    }
}

pub fn request_all(vehicle: u8, compid: u8, link: u32) {
    crate::hub::send_for(vehicle, link, &[Outbound::ParamExtRequestList { target: (vehicle, compid) }]);
}

pub fn parameter_name(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw).trim_end_matches('\0').to_string()
}

fn schedule(ready: &mut Ready, outs: &[Out], now_ms: u64) {
    if let Some(after_ms) = outs.iter().find_map(|out| match out {
        Out::ScheduleUpdates { after_ms } => Some(*after_ms),
        _ => None,
    }) {
        ready.updates_due = ready.updates_due.or(Some(now_ms + after_ms));
    }
}

fn with_ready<T>(key: (u8, u8), change: impl FnOnce(&mut Ready) -> T) -> Option<T> {
    match STORE.lock().unwrap_or_else(PoisonError::into_inner).get_mut(&key) {
        Some(Entry::Ready(ready)) => Some(change(ready)),
        _ => None,
    }
}

pub fn on_value(vehicle: u8, compid: u8, name: &str, param_type: u8, raw: &[u8], now_ms: u64) {
    let Some(value) = crate::cameradef::decode_param_value(param_type, raw) else { return };
    with_ready((vehicle, compid), |ready| {
        let outs = ready.parameters.on_value(name, value, Source::Camera, now_ms);
        schedule(ready, &outs, now_ms);
    });
}

pub enum AckStep {
    Settled,
    Wait,
    Abandon,
}

pub fn ack_step(result: u8, retries: u32) -> AckStep {
    match result {
        PARAM_ACK_ACCEPTED => AckStep::Settled,
        PARAM_ACK_IN_PROGRESS => AckStep::Wait,
        PARAM_ACK_FAILED if retries + 1 < crate::cameradef::FAILED_ACK_RETRIES => AckStep::Wait,
        PARAM_ACK_FAILED => AckStep::Abandon,
        _ => AckStep::Settled,
    }
}

pub fn on_ack(vehicle: u8, compid: u8, name: &str, param_type: u8, raw: &[u8], result: u8, now_ms: u64) {
    let reported = crate::cameradef::decode_param_value(param_type, raw);
    with_ready((vehicle, compid), |ready| {
        let retries = ready.writes.get(name).map_or(0, |w| w.retries);
        match ack_step(result, retries) {
            AckStep::Wait => {
                if let Some(write) = ready.writes.get_mut(name) {
                    write.sent_ms = now_ms;
                    write.retries += u32::from(result == PARAM_ACK_FAILED);
                }
            }
            AckStep::Abandon => {
                ready.writes.remove(name);
            }
            AckStep::Settled => {
                ready.writes.remove(name);
                if let Some(value) = reported {
                    let outs = ready.parameters.on_value(name, value, Source::Camera, now_ms);
                    schedule(ready, &outs, now_ms);
                }
            }
        }
    });
}

pub fn tick(now_ms: u64) {
    start_ftp_downloads(now_ms);
    let due: Vec<(u8, u32, Vec<Outbound>)> = STORE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter_mut()
        .filter_map(|((vehicle, compid), entry)| match entry {
            Entry::Ready(ready) => Some(((*vehicle, *compid), ready)),
            _ => None,
        })
        .map(|(target, ready)| {
            let timed_out: Vec<String> = ready.writes.iter().filter(|(_, w)| now_ms.saturating_sub(w.sent_ms) >= crate::cameradef::PARAM_WRITE_TIMEOUT_MS).map(|(name, _)| name.clone()).collect();
            let resends: Vec<Outbound> = timed_out
                .iter()
                .filter_map(|name| {
                    let write = ready.writes.get_mut(name)?;
                    write.retries += 1;
                    write.sent_ms = now_ms;
                    let outbound = (write.retries <= crate::cameradef::WRITE_TIMEOUT_RETRIES).then(|| Outbound::ParamExtSet { target, id: name.clone(), value: write.value, param_type: write.param_type });
                    if outbound.is_none() {
                        ready.writes.remove(name);
                    }
                    outbound
                })
                .collect();
            let updates_due = ready.updates_due.is_some_and(|at| now_ms >= at);
            let reads: Vec<Outbound> = match updates_due {
                true => {
                    ready.updates_due = None;
                    ready.parameters.take_pending_updates().into_iter().map(|id| Outbound::ParamExtRequestRead { target, id }).collect()
                }
                false => Vec::new(),
            };
            (target.0, ready.link, resends.into_iter().chain(reads).collect())
        })
        .collect();
    due.iter().for_each(|(vehicle, link, outbound)| crate::hub::send_for(*vehicle, *link, outbound));
}

fn refused(reason: &str) -> Value {
    json!({ "ok": false, "reason": reason })
}

pub fn set_setting(args: &str) -> Value {
    let given = serde_json::from_str::<Vec<Value>>(args).unwrap_or_default();
    let (Some(name), Some(wanted)) = (given.first().and_then(Value::as_str), given.get(1)) else { return refused("cameraSettings.set takes a parameter name and a value") };
    let Some(key) = selected_camera() else { return refused("No camera is selected.") };
    let outcome = with_ready(key, |ready| {
        let parameter = ready.parameters.definition.parameter(name).cloned().ok_or("This camera has no such setting.")?;
        if parameter.meta.read_only || ready.parameters.applicability(name) != crate::cameradef::Applicability::Applicable {
            return Err("This setting cannot be changed now.");
        }
        let value = crate::cameradef::coerce(parameter.meta.value_type, wanted.clone());
        let offered = ready.parameters.options(name).map(|o| o.entries.clone()).unwrap_or_default();
        if !offered.is_empty() && !offered.iter().any(|entry| crate::cameradef::same_typed(parameter.meta.value_type, &entry.value, &value)) {
            return Err("Pick one of the offered values.");
        }
        let param_type = crate::cameradef::mav_param_ext_type(parameter.meta.value_type);
        let bytes = crate::cameradef::encode_param_value(param_type, &value).ok_or("That value does not fit this setting.")?;
        ready.writes.insert(name.to_string(), Write { value: bytes, param_type, sent_ms: crate::hub::now_ms(), retries: 0 });
        let outs = ready.parameters.on_value(name, value, Source::Requested, crate::hub::now_ms());
        schedule(ready, &outs, crate::hub::now_ms());
        Ok((ready.link, Outbound::ParamExtSet { target: key, id: name.to_string(), value: bytes, param_type }))
    });
    match outcome {
        None => refused("The camera's settings are not loaded."),
        Some(Err(reason)) => refused(reason),
        Some(Ok((link, outbound))) => {
            crate::hub::send_for(key.0, link, &[outbound]);
            json!({ "ok": true })
        }
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
        Some(Entry::Fetching | Entry::FtpWaiting(_) | Entry::FtpDownloading(_)) => unavailable("fetching", ""),
        Some(Entry::Failed(error)) => unavailable("failed", error),
        Some(Entry::Ready(ready)) => {
            let mut shown = crate::cameradef::definition_json(&ready.parameters, &ready.uri, crate::hub::now_ms());
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

    #[test]
    fn acks_follow_qgc_camera_param_io() {
        assert!(matches!(ack_step(PARAM_ACK_ACCEPTED, 0), AckStep::Settled));
        assert!(matches!(ack_step(PARAM_ACK_IN_PROGRESS, 0), AckStep::Wait), "in progress waits longer without resending");
        assert!(matches!(ack_step(PARAM_ACK_FAILED, 0), AckStep::Wait), "a failure restarts the timer and the timeout resends");
        assert!(matches!(ack_step(PARAM_ACK_FAILED, 2), AckStep::Abandon));
        assert!(matches!(ack_step(1, 0), AckStep::Settled), "unsupported takes back the camera's own value");
    }

    #[test]
    fn values_encode_to_the_bytes_they_decode_from() {
        use crate::cameradef::{PARAM_EXT_TYPE_CUSTOM, PARAM_EXT_TYPE_INT32, PARAM_EXT_TYPE_REAL32, PARAM_EXT_TYPE_UINT8, decode_param_value, encode_param_value};
        [(PARAM_EXT_TYPE_UINT8, json!(3)), (PARAM_EXT_TYPE_INT32, json!(-7)), (PARAM_EXT_TYPE_REAL32, json!(0.5)), (PARAM_EXT_TYPE_CUSTOM, json!("4K"))].iter().for_each(|(kind, value)| {
            let bytes = encode_param_value(*kind, value).unwrap();
            assert_eq!(decode_param_value(*kind, &bytes).as_ref(), Some(value), "{kind}");
        });
        assert_eq!(encode_param_value(PARAM_EXT_TYPE_UINT8, &json!(true)).unwrap()[0], 1, "a bool rides as uint8 like QGC's union");
    }
}
