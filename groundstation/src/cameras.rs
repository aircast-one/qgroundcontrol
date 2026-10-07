use serde_json::{Value, json};

use crate::router::Backend;
use crate::videostate::{
    DEVICE_CAMERAS, SOURCE_DISABLED, SOURCE_MPEGTS, SOURCE_RTSP, SOURCE_TCP, SOURCE_UDP_H264, SOURCE_UDP_H265, SOURCE_WEBRTC, needs_url,
};

pub const CAMERAS_FACT: &str = "cameras";
pub const CAMERAS_PATH: &str = "settings.videoSettings.cameras";
pub const ACTIVE_PATH: &str = "settings.videoSettings.activeVideoSource";
pub const DEPS: &[&str] = &[CAMERAS_PATH, ACTIVE_PATH, "video.cameraStatuses", "video.cameraFromDrone", "video.activeVideoSource", "video.cameraNames", "video.cameraSources", "video.cameraUrls"];

pub const CAMERAS_ADD: &str = "cameras.add";
pub const CAMERAS_UPDATE: &str = "cameras.update";
pub const CAMERAS_REMOVE: &str = "cameras.remove";
pub const CAMERAS_MOVE: &str = "cameras.move";
const CAMERAS_OWNED: [&str; 4] = [CAMERAS_ADD, CAMERAS_UPDATE, CAMERAS_REMOVE, CAMERAS_MOVE];

const UNREADABLE: &str = "The camera list is not a readable list, so its cameras cannot be shown. Changing it now would replace it.";
const NO_SUCH_CAMERA: &str = "There is no camera at that position.";
const NEEDS_KIND: &str = "Pick the kind of stream this camera sends.";
const NEEDS_ADDRESS: &str = "This kind of stream needs an address.";
const RTSP_SCHEME: &str = "An RTSP address starts with rtsp://.";
const DOUBLED_SCHEME: &str = "Leave the scheme off. The app adds it, and a doubled one fails to resolve.";
const SCHEME_ADDED: [&str; 4] = [SOURCE_UDP_H264, SOURCE_UDP_H265, SOURCE_MPEGTS, SOURCE_TCP];

const GROUP_STREAMS: &str = "Video streams";
const GROUP_DEVICE: &str = "This device";
const GROUP_PRESETS: &str = "Vehicle and radio presets";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Camera {
    pub name: String,
    pub source: String,
    pub url: String,
}

impl Camera {
    pub fn new(name: &str, source: &str, url: &str) -> Camera {
        Camera { name: name.trim().to_string(), source: source.trim().to_string(), url: url.trim().to_string() }
    }
}

pub fn owns(path: &str) -> bool {
    CAMERAS_OWNED.contains(&path)
}

fn field(entry: &Value, key: &str) -> String {
    entry.get(key).and_then(Value::as_str).unwrap_or_default().trim().to_string()
}

pub fn parse(text: &str) -> Option<Vec<Camera>> {
    match text.trim() {
        "" => Some(Vec::new()),
        trimmed => serde_json::from_str::<Value>(trimmed)
            .ok()?
            .as_array()
            .map(|entries| entries.iter().map(|entry| Camera { name: field(entry, "name"), source: field(entry, "source"), url: field(entry, "url") }).collect()),
    }
}

pub fn encode(cameras: &[Camera]) -> String {
    Value::Array(cameras.iter().map(|camera| json!({ "name": camera.name, "source": camera.source, "url": camera.url })).collect()).to_string()
}

pub fn problem(source: &str, url: &str) -> Option<&'static str> {
    match () {
        _ if source.is_empty() || source == SOURCE_DISABLED || !kind_names().iter().any(|kind| kind == source) => Some(NEEDS_KIND),
        _ if needs_url(source) && url.is_empty() => Some(NEEDS_ADDRESS),
        _ if source == SOURCE_RTSP && !url.to_lowercase().starts_with("rtsp") => Some(RTSP_SCHEME),
        _ if url.contains("://") && SCHEME_ADDED.contains(&source) => Some(DOUBLED_SCHEME),
        _ => None,
    }
}

fn kind_names() -> Vec<String> {
    crate::settingsstore::camera_sources()
}

fn group(source: &str) -> &'static str {
    match () {
        _ if DEVICE_CAMERAS.contains(&source) => GROUP_DEVICE,
        _ if needs_url(source) => GROUP_STREAMS,
        _ => GROUP_PRESETS,
    }
}

fn hint(source: &str) -> &'static str {
    match source {
        SOURCE_RTSP => "rtsp://192.168.1.10:8554/live",
        SOURCE_WEBRTC => "http://192.168.1.10:8889/cam/whep",
        SOURCE_TCP => "192.168.1.10:5600",
        SOURCE_UDP_H264 | SOURCE_UDP_H265 | SOURCE_MPEGTS => "0.0.0.0:5600",
        _ => "",
    }
}

pub fn kinds() -> Vec<Value> {
    kind_names()
        .into_iter()
        .map(|source| json!({ "raw": source, "label": source, "group": group(&source), "needsUrl": needs_url(&source), "hint": hint(&source) }))
        .collect()
}

pub fn summary(camera: &Camera) -> String {
    match () {
        _ if camera.source.is_empty() => "Not set up".to_string(),
        _ if !needs_url(&camera.source) => camera.source.clone(),
        _ if camera.url.is_empty() => format!("{} · no address", camera.source),
        _ => format!("{} · {}", camera.source, camera.url),
    }
}

pub fn title(name: &str, slot: usize) -> String {
    match name.trim() {
        "" => format!("Camera {}", slot + 1),
        named => named.to_string(),
    }
}

pub fn with_named(cameras: &[Camera], camera: Camera) -> (Vec<Camera>, usize) {
    let found = cameras.iter().position(|existing| !camera.name.is_empty() && existing.name == camera.name);
    match found {
        Some(at) => (cameras.iter().enumerate().map(|(index, existing)| if index == at { camera.clone() } else { existing.clone() }).collect(), at),
        None => (cameras.iter().cloned().chain(std::iter::once(camera)).collect(), cameras.len()),
    }
}

fn url_host(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = rest.split(['/', '?']).next().unwrap_or(rest);
    authority.rsplit_once('@').map_or(authority, |(_, host)| host).rsplit_once(':').map_or(authority, |(host, _)| host)
}

pub fn with_device(cameras: &[Camera], host: &str, device: Vec<Camera>) -> Vec<Camera> {
    cameras.iter().filter(|camera| url_host(&camera.url) != host).cloned().chain(device).collect()
}

pub fn active_after_removal(active: i64, removed: usize) -> i64 {
    let removed = removed as i64;
    match () {
        _ if active == removed => 0,
        _ if active > removed => active - 1,
        _ => active,
    }
}

pub fn active_after_move(active: i64, from: usize, to: usize) -> i64 {
    let (from, to) = (from as i64, to as i64);
    match () {
        _ if active == from => to,
        _ if from < to && active > from && active <= to => active - 1,
        _ if to < from && active >= to && active < from => active + 1,
        _ => active,
    }
}

pub fn stored() -> Option<Vec<Camera>> {
    let text = crate::settingsstore::raw_setting(CAMERAS_PATH).and_then(|value| value.as_str().map(str::to_string)).unwrap_or_default();
    parse(&text)
}

fn active() -> i64 {
    crate::settingsstore::raw_setting(ACTIVE_PATH).and_then(|value| value.as_i64()).unwrap_or(0)
}

pub fn store(cameras: &[Camera], active: i64) {
    crate::settingsstore::set_raw(CAMERAS_PATH, &json!(encode(cameras)));
    crate::settingsstore::set_raw(ACTIVE_PATH, &json!(active));
}

fn store_through(backend: &dyn Backend, cameras: &[Camera], active: i64) {
    crate::settingsstore::set(backend, CAMERAS_PATH, &json!({ "value": encode(cameras) }).to_string());
    crate::settingsstore::set(backend, ACTIVE_PATH, &json!({ "value": active }).to_string());
}

pub fn adopt(camera: Camera) {
    if let Some(cameras) = stored() {
        let (next, at) = with_named(&cameras, camera);
        store(&next, at as i64);
    }
}

pub fn adopt_device(host: &str, device: Vec<Camera>) {
    if let Some(cameras) = stored().filter(|_| !device.is_empty()) {
        let count = device.len();
        let next = with_device(&cameras, host, device);
        store(&next, (next.len() - count) as i64);
    }
}

fn refused(reason: &str) -> Value {
    json!({ "ok": false, "reason": reason })
}

fn text_arg(args: &Value, at: usize) -> String {
    args.get(at).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn index_arg(args: &Value, at: usize, count: usize) -> Option<usize> {
    args.get(at).and_then(Value::as_u64).and_then(|index| usize::try_from(index).ok()).filter(|index| *index < count)
}

pub fn invoke(backend: &dyn Backend, path: &str, args: &str) -> Option<Value> {
    owns(path).then_some(())?;
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let Some(cameras) = stored() else { return Some(refused(UNREADABLE)) };
    let current = active();
    Some(match path {
        CAMERAS_ADD => {
            let camera = Camera::new(&text_arg(&given, 0), &text_arg(&given, 1), &text_arg(&given, 2));
            match problem(&camera.source, &camera.url) {
                Some(reason) => refused(reason),
                None => {
                    let next: Vec<Camera> = cameras.iter().cloned().chain(std::iter::once(camera)).collect();
                    store_through(backend, &next, if cameras.is_empty() { 0 } else { current });
                    json!({ "ok": true, "slot": cameras.len() })
                }
            }
        }
        CAMERAS_UPDATE => match index_arg(&given, 0, cameras.len()) {
            None => refused(NO_SUCH_CAMERA),
            Some(at) => {
                let camera = Camera::new(&text_arg(&given, 1), &text_arg(&given, 2), &text_arg(&given, 3));
                match problem(&camera.source, &camera.url) {
                    Some(reason) => refused(reason),
                    None => {
                        let next: Vec<Camera> = cameras.iter().enumerate().map(|(index, existing)| if index == at { camera.clone() } else { existing.clone() }).collect();
                        store_through(backend, &next, current);
                        json!({ "ok": true, "slot": at })
                    }
                }
            }
        },
        CAMERAS_REMOVE => match index_arg(&given, 0, cameras.len()) {
            None => refused(NO_SUCH_CAMERA),
            Some(at) => {
                let next: Vec<Camera> = cameras.iter().enumerate().filter(|(index, _)| *index != at).map(|(_, camera)| camera.clone()).collect();
                store_through(backend, &next, active_after_removal(current, at));
                json!({ "ok": true })
            }
        },
        CAMERAS_MOVE => match (index_arg(&given, 0, cameras.len()), index_arg(&given, 1, cameras.len())) {
            (Some(from), Some(to)) => {
                let moving = cameras[from].clone();
                let without: Vec<Camera> = cameras.iter().enumerate().filter(|(index, _)| *index != from).map(|(_, camera)| camera.clone()).collect();
                let next: Vec<Camera> = without[..to].iter().cloned().chain(std::iter::once(moving)).chain(without[to..].iter().cloned()).collect();
                store_through(backend, &next, active_after_move(current, from, to));
                json!({ "ok": true, "slot": to })
            }
            _ => refused(NO_SUCH_CAMERA),
        },
        _ => return None,
    })
}

pub fn cameras_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let video = crate::read::object(&backend.get_fields("video", "activeVideoSource,cameraStatuses,cameraFromDrone,cameraNames,cameraSources,cameraUrls"));
    let strings = |key: &str| -> Vec<String> { video.get(key).and_then(Value::as_array).map(|a| a.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect()).unwrap_or_default() };
    let drone_flags: Vec<bool> = video.get("cameraFromDrone").and_then(Value::as_array).map(|a| a.iter().map(|v| v.as_bool().unwrap_or(false)).collect()).unwrap_or_default();
    let active = crate::read::integer(&video, "activeVideoSource").unwrap_or_else(active);
    let (names, sources, urls) = (strings("cameraNames"), strings("cameraSources"), strings("cameraUrls"));
    let drone: Vec<Value> = drone_flags
        .iter()
        .enumerate()
        .filter(|(_, drone)| **drone)
        .map(|(slot, _)| {
            let camera = Camera::new(names.get(slot).map_or("", String::as_str), sources.get(slot).map_or("", String::as_str), urls.get(slot).map_or("", String::as_str));
            json!({ "slot": slot, "stored": Value::Null, "title": title(&camera.name, slot), "name": camera.name, "source": camera.source, "url": camera.url, "summary": summary(&camera), "problem": Value::Null, "fromDrone": true, "active": slot as i64 == active })
        })
        .collect();
    match stored() {
        None => json!({ "kind": "object", "class": "Cameras", "readable": false, "reason": UNREADABLE, "cameras": drone, "active": active, "kinds": kinds() }),
        Some(cameras) => {
            let mine = cameras.iter().enumerate().map(|(slot, camera)| {
                json!({ "slot": slot, "stored": slot, "title": title(&camera.name, slot), "name": camera.name, "source": camera.source, "url": camera.url, "summary": summary(camera), "problem": problem(&camera.source, &camera.url), "fromDrone": false, "active": slot as i64 == active })
            });
            json!({ "kind": "object", "class": "Cameras", "readable": true, "reason": Value::Null, "cameras": mine.chain(drone).collect::<Vec<_>>(), "active": active, "kinds": kinds() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::videostate::{SOURCE_BACK_CAMERA, SOURCE_HERELINK_HOTSPOT};

    fn cam(name: &str, source: &str, url: &str) -> Camera {
        Camera::new(name, source, url)
    }

    #[test]
    fn the_list_round_trips_and_an_unreadable_one_is_not_an_empty_one() {
        let list = vec![cam("Front", SOURCE_RTSP, "rtsp://10.0.0.5:8554/front"), cam("", SOURCE_BACK_CAMERA, "")];
        assert_eq!(parse(&encode(&list)), Some(list));
        assert_eq!(parse(""), Some(Vec::new()), "nothing stored is no cameras");
        assert_eq!(parse("{not a list"), None, "a list that cannot be read must not be mistaken for an empty one and overwritten");
    }

    #[test]
    fn a_camera_is_refused_with_the_fix_an_operator_can_make() {
        assert_eq!(problem("", ""), Some(NEEDS_KIND));
        assert_eq!(problem(SOURCE_DISABLED, ""), Some(NEEDS_KIND));
        assert_eq!(problem(SOURCE_RTSP, ""), Some(NEEDS_ADDRESS));
        assert_eq!(problem(SOURCE_RTSP, "10.0.0.5:8554/live"), Some(RTSP_SCHEME));
        assert_eq!(problem(SOURCE_UDP_H264, "udp://0.0.0.0:5600"), Some(DOUBLED_SCHEME));
        assert_eq!(problem(SOURCE_UDP_H264, "0.0.0.0:5600"), None);
        assert_eq!(problem(SOURCE_WEBRTC, "https://cam.local/whep"), None);
        assert_eq!(problem(SOURCE_HERELINK_HOTSPOT, ""), None, "a preset needs no address");
    }

    #[test]
    fn a_named_camera_replaces_its_namesake_and_a_new_one_goes_last() {
        let list = vec![cam("Front", SOURCE_RTSP, "rtsp://a/front"), cam("Belly", SOURCE_RTSP, "rtsp://a/belly")];
        let (replaced, at) = with_named(&list, cam("Belly", SOURCE_WEBRTC, "http://a/belly/whep"));
        assert_eq!((replaced.len(), at, replaced[1].source.as_str()), (2, 1, SOURCE_WEBRTC));
        let (added, at) = with_named(&list, cam("Tail", SOURCE_RTSP, "rtsp://a/tail"));
        assert_eq!((added.len(), at), (3, 2));
    }

    #[test]
    fn setting_up_a_device_swaps_only_that_devices_cameras() {
        let list = vec![cam("Old front", SOURCE_RTSP, "rtsp://10.0.0.5:8554/front"), cam("Other drone", SOURCE_RTSP, "rtsp://10.0.0.9:8554/cam"), cam("Phone", SOURCE_BACK_CAMERA, "")];
        let next = with_device(&list, "10.0.0.5", vec![cam("front (10.0.0.5)", SOURCE_RTSP, "rtsp://10.0.0.5:8554/front"), cam("belly (10.0.0.5)", SOURCE_RTSP, "rtsp://10.0.0.5:8554/belly")]);
        assert_eq!(next.iter().map(|camera| camera.name.as_str()).collect::<Vec<_>>(), vec!["Other drone", "Phone", "front (10.0.0.5)", "belly (10.0.0.5)"]);
    }

    #[test]
    fn the_active_camera_follows_its_camera_through_removal_and_reordering() {
        assert_eq!(active_after_removal(2, 2), 0, "removing the camera on screen falls back to the first");
        assert_eq!(active_after_removal(2, 0), 1);
        assert_eq!(active_after_removal(0, 2), 0);
        assert_eq!(active_after_move(0, 0, 2), 2, "the moved camera keeps being the one on screen");
        assert_eq!(active_after_move(2, 0, 2), 1);
        assert_eq!(active_after_move(1, 2, 0), 2);
        assert_eq!(active_after_move(3, 0, 2), 3);
    }

    #[test]
    fn every_kind_says_where_it_belongs_and_whether_it_needs_an_address() {
        let listed = kinds();
        let rtsp = listed.iter().find(|kind| kind["raw"] == SOURCE_RTSP).unwrap();
        assert_eq!((rtsp["group"].as_str(), rtsp["needsUrl"].as_bool(), rtsp["hint"].as_str()), (Some(GROUP_STREAMS), Some(true), Some("rtsp://192.168.1.10:8554/live")));
        assert!(listed.iter().all(|kind| kind["raw"] != SOURCE_DISABLED), "no camera is of kind disabled");
    }
}
