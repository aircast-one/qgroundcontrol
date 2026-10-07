use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::router::Backend;
use crate::videostate::{
    DEVICE_CAMERAS, SOURCE_DISABLED, SOURCE_MPEGTS, SOURCE_RTSP, SOURCE_TCP, SOURCE_UDP_H264, SOURCE_UDP_H265, SOURCE_WEBRTC, needs_url, source_usable,
};

pub const CAMERAS_FACT: &str = "cameras";
pub const ACTIVE_FACT: &str = "activeVideoSource";
pub const CAMERAS_PATH: &str = "settings.videoSettings.cameras";
pub const ACTIVE_PATH: &str = "settings.videoSettings.activeVideoSource";
pub const DEPS: &[&str] = &[CAMERAS_PATH, ACTIVE_PATH, "video.cameraStatuses", "video.cameraFromDrone", "video.activeVideoSource", "video.cameraNames", "video.cameraSources", "video.cameraUrls"];

pub const CAMERAS_ADD: &str = "cameras.add";
pub const CAMERAS_UPDATE: &str = "cameras.update";
pub const CAMERAS_REMOVE: &str = "cameras.remove";
pub const CAMERAS_MOVE: &str = "cameras.move";
const CAMERAS_OWNED: [&str; 4] = [CAMERAS_ADD, CAMERAS_UPDATE, CAMERAS_REMOVE, CAMERAS_MOVE];
const QT_STORE_CAMERAS: &str = "video.storeCameras";

const UNREADABLE: &str = "The camera list is not a readable list, so its cameras cannot be shown. Changing it now would replace it.";
const NO_SUCH_CAMERA: &str = "There is no camera at that position.";
const NEEDS_KIND: &str = "Pick the kind of stream this camera sends.";
const CANNOT_PLAY: &str = "This kind of camera cannot show video in this app.";
const NEEDS_ADDRESS: &str = "This kind of stream needs an address.";
const RTSP_SCHEME: &str = "An RTSP address starts with rtsp://.";
const WHEP_SCHEME: &str = "A WebRTC address starts with http:// or https://.";
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
    let offered = kind_names().iter().any(|kind| kind == source);
    let known = source != SOURCE_DISABLED && crate::settingsstore::camera_sources().iter().any(|kind| kind == source);
    match () {
        _ if !offered && known => Some(CANNOT_PLAY),
        _ if !offered => Some(NEEDS_KIND),
        _ if needs_url(source) && url.is_empty() => Some(NEEDS_ADDRESS),
        _ if source == SOURCE_RTSP && scheme_of(source, url).is_none() => Some(RTSP_SCHEME),
        _ if source == SOURCE_WEBRTC && url.contains("://") && scheme_of(source, url).is_none() => Some(WHEP_SCHEME),
        _ if url.contains("://") && SCHEME_ADDED.contains(&source) => Some(DOUBLED_SCHEME),
        _ => None,
    }
}

fn schemes(source: &str) -> &'static [&'static str] {
    match source {
        SOURCE_RTSP => &["rtsp://", "rtsps://"],
        SOURCE_WEBRTC => &["http://", "https://"],
        SOURCE_UDP_H264 => &["udp://"],
        SOURCE_UDP_H265 => &["udp265://", "udp://"],
        SOURCE_MPEGTS => &["mpegts://", "udp://"],
        SOURCE_TCP => &["tcp://"],
        _ => &[],
    }
}

fn scheme_of(source: &str, url: &str) -> Option<&'static str> {
    schemes(source).iter().copied().find(|scheme| url.get(..scheme.len()).is_some_and(|head| head.eq_ignore_ascii_case(scheme)))
}

pub fn normalized(camera: Camera) -> Camera {
    let url = match scheme_of(&camera.source, &camera.url) {
        Some(scheme) if SCHEME_ADDED.contains(&camera.source.as_str()) => camera.url[scheme.len()..].to_string(),
        Some(scheme) => format!("{scheme}{}", &camera.url[scheme.len()..]),
        None => camera.url.clone(),
    };
    Camera { url, ..camera }
}

fn kind_names() -> Vec<String> {
    crate::settingsstore::camera_sources().into_iter().filter(|source| needs_url(source) || source_usable(source, "")).collect()
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
        .map(|source| json!({ "raw": source, "label": source, "group": group(&source), "more": group(&source) == GROUP_PRESETS, "needsUrl": needs_url(&source), "hint": hint(&source), "schemes": schemes(&source) }))
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
    let found = cameras.iter().position(|existing| match camera.name.is_empty() {
        true => existing.source == camera.source && existing.url == camera.url,
        false => existing.name == camera.name,
    });
    match found {
        Some(at) => (cameras.iter().enumerate().map(|(index, existing)| if index == at { camera.clone() } else { existing.clone() }).collect(), at),
        None => (cameras.iter().cloned().chain(std::iter::once(camera)).collect(), cameras.len()),
    }
}

fn url_host(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = rest.split(['/', '?']).next().unwrap_or(rest);
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
    host_port.rsplit_once(':').map_or(host_port, |(host, _)| host)
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

pub fn active_after_add(active: i64, stored: usize) -> i64 {
    match () {
        _ if stored == 0 => 0,
        _ if active >= stored as i64 => active + 1,
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

static EDITING: Mutex<()> = Mutex::new(());

fn listed() -> (Option<Vec<Camera>>, i64) {
    let [cameras, active] = crate::settingsstore::raw_settings([CAMERAS_PATH, ACTIVE_PATH]);
    (parse(cameras.as_str().unwrap_or_default()), active.as_i64().unwrap_or(0))
}

pub fn stored() -> Option<Vec<Camera>> {
    listed().0
}

pub fn edit<T>(backend: Option<&dyn Backend>, change: impl FnOnce(&[Camera], i64) -> (Option<(Vec<Camera>, i64)>, T)) -> Option<T> {
    let (answer, written) = {
        let _editing = EDITING.lock().unwrap_or_else(PoisonError::into_inner);
        let (cameras, active) = listed();
        let (write, answer) = change(&cameras?, active);
        (answer, write.map(|(next, next_active)| committed(&next, next_active)))
    };
    if let (Some(backend), Some((encoded, active))) = (backend.filter(|_| crate::qthost::present()), written) {
        backend.invoke(QT_STORE_CAMERAS, &json!([encoded, active]).to_string());
    }
    Some(answer)
}

fn committed(cameras: &[Camera], active: i64) -> (String, i64) {
    let encoded = encode(cameras);
    crate::settingsstore::set_raw_together(&[(CAMERAS_PATH, json!(encoded)), (ACTIVE_PATH, json!(active))]);
    (encoded, active)
}

pub fn adopt(camera: Camera) {
    edit(None, |cameras, _| {
        let (next, at) = with_named(cameras, camera);
        (Some((next, at as i64)), ())
    });
}

fn device_adopted(cameras: &[Camera], host: &str, device: Vec<Camera>) -> Option<(Vec<Camera>, i64)> {
    let count = device.len();
    let next = with_device(cameras, host, device);
    let at = (next.len() - count) as i64;
    (count > 0).then_some((next, at))
}

pub fn adopt_device(host: &str, device: Vec<Camera>) {
    edit(None, |cameras, _| (device_adopted(cameras, host, device), ()));
}

pub fn default_when_empty(camera: Camera) -> bool {
    edit(None, |cameras, _| (cameras.is_empty().then(|| (vec![camera], 0)), cameras.is_empty())).unwrap_or(false)
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
    Some(edit(Some(backend), |cameras, active| changed(path, &given, cameras, active)).unwrap_or_else(|| refused(UNREADABLE)))
}

type Change = (Option<(Vec<Camera>, i64)>, Value);

fn changed(path: &str, given: &Value, cameras: &[Camera], active: i64) -> Change {
    let refusal = |reason: &str| (None, refused(reason));
    let checked = |camera: Camera, write: &dyn Fn(Camera) -> Change| match problem(&camera.source, &camera.url) {
        Some(reason) => refusal(reason),
        None => write(camera),
    };
    match path {
        CAMERAS_ADD => checked(normalized(Camera::new(&text_arg(given, 0), &text_arg(given, 1), &text_arg(given, 2))), &|camera| {
            let next = cameras.iter().cloned().chain(std::iter::once(camera)).collect();
            (Some((next, active_after_add(active, cameras.len()))), json!({ "ok": true, "slot": cameras.len() }))
        }),
        CAMERAS_UPDATE => match index_arg(given, 0, cameras.len()) {
            None => refusal(NO_SUCH_CAMERA),
            Some(at) => checked(normalized(Camera::new(&text_arg(given, 1), &text_arg(given, 2), &text_arg(given, 3))), &|camera| {
                let next = cameras.iter().enumerate().map(|(index, existing)| if index == at { camera.clone() } else { existing.clone() }).collect();
                (Some((next, active)), json!({ "ok": true, "slot": at }))
            }),
        },
        CAMERAS_REMOVE => match index_arg(given, 0, cameras.len()) {
            None => refusal(NO_SUCH_CAMERA),
            Some(at) => {
                let next = cameras.iter().enumerate().filter(|(index, _)| *index != at).map(|(_, camera)| camera.clone()).collect();
                (Some((next, active_after_removal(active, at))), json!({ "ok": true }))
            }
        },
        CAMERAS_MOVE => match (index_arg(given, 0, cameras.len()), index_arg(given, 1, cameras.len())) {
            (Some(from), Some(to)) => {
                let moving = cameras[from].clone();
                let without: Vec<Camera> = cameras.iter().enumerate().filter(|(index, _)| *index != from).map(|(_, camera)| camera.clone()).collect();
                let next = without[..to].iter().cloned().chain(std::iter::once(moving)).chain(without[to..].iter().cloned()).collect();
                (Some((next, active_after_move(active, from, to))), json!({ "ok": true, "slot": to }))
            }
            _ => refusal(NO_SUCH_CAMERA),
        },
        _ => (None, Value::Null),
    }
}

pub fn cameras_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let video = crate::read::object(&backend.get_fields("video", "activeVideoSource,cameraStatuses,cameraFromDrone,cameraNames,cameraSources,cameraUrls"));
    let (cameras, active) = listed();
    view_of(&video, cameras, active)
}

fn view_of(video: &Value, stored: Option<Vec<Camera>>, stored_active: i64) -> Value {
    let strings = |key: &str| -> Vec<String> { video.get(key).and_then(Value::as_array).map(|a| a.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect()).unwrap_or_default() };
    let drone_flags: Vec<bool> = video.get("cameraFromDrone").and_then(Value::as_array).map(|a| a.iter().map(|v| v.as_bool().unwrap_or(false)).collect()).unwrap_or_default();
    let active = crate::read::integer(video, "activeVideoSource").unwrap_or(stored_active);
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
    match stored {
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
    use crate::videostate::{SOURCE_3DR_SOLO, SOURCE_BACK_CAMERA, SOURCE_HERELINK_HOTSPOT};

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
        assert_eq!(rtsp["schemes"], json!(["rtsp://", "rtsps://"]), "a typed rtsp:// address can pick its own kind");
        assert_eq!(rtsp["more"], json!(false));
        let preset = listed.iter().find(|kind| kind["group"] == GROUP_PRESETS).unwrap();
        assert_eq!(preset["more"], json!(true), "vehicle presets wait behind More types");
        assert!(listed.iter().all(|kind| kind["raw"] != SOURCE_3DR_SOLO), "a kind that can never start is not offered");
        assert_eq!(problem(SOURCE_3DR_SOLO, ""), Some(CANNOT_PLAY), "nor accepted when asked for directly, and the reason says the kind cannot play rather than that none was picked");
        assert_eq!(problem("No such kind", ""), Some(NEEDS_KIND));
    }

    #[test]
    fn adding_a_camera_leaves_the_one_on_screen_on_screen() {
        assert_eq!(active_after_add(0, 0), 0, "the first camera of an empty list is the one shown");
        assert_eq!(active_after_add(1, 3), 1, "a stored camera keeps its place");
        assert_eq!(active_after_add(3, 3), 4, "a remembered drone camera moves down one as the new camera goes in before it");
    }

    fn list(cameras: &[Camera]) -> Value {
        json!(cameras.iter().map(|camera| json!([camera.name, camera.source, camera.url])).collect::<Vec<_>>())
    }

    #[test]
    fn every_edit_keeps_the_camera_on_screen_and_refuses_without_writing() {
        let front = cam("Front", SOURCE_RTSP, "rtsp://a/front");
        let belly = cam("Belly", SOURCE_RTSP, "rtsp://a/belly");
        let three = vec![front.clone(), belly.clone(), cam("Tail", SOURCE_RTSP, "rtsp://a/tail")];
        let (write, answer) = changed(CAMERAS_ADD, &json!(["Nose", SOURCE_RTSP, "rtsp://a/nose"]), &three, 3);
        assert_eq!((write.map(|(next, active)| (next.len(), active)), answer["slot"].as_u64()), (Some((4, 4)), Some(3)), "a drone camera picked earlier stays picked after an add");
        let (write, answer) = changed(CAMERAS_ADD, &json!(["Nose", SOURCE_RTSP, ""]), &three, 0);
        assert_eq!((write, answer["reason"].as_str()), (None, Some(NEEDS_ADDRESS)), "a refusal writes nothing");
        let (write, _) = changed(CAMERAS_ADD, &json!(["", SOURCE_UDP_H264, "UDP://0.0.0.0:5600"]), &[], 2);
        assert_eq!(write, Some((vec![cam("", SOURCE_UDP_H264, "0.0.0.0:5600")], 0)), "the address is cleaned before it is checked and the first camera is shown");
        let (write, _) = changed(CAMERAS_UPDATE, &json!([1, "Belly", SOURCE_WEBRTC, "HTTPS://a/belly/whep"]), &three, 2);
        assert_eq!(write.map(|(next, active)| (next[1].url.clone(), active)), Some(("https://a/belly/whep".to_string(), 2)));
        assert_eq!(changed(CAMERAS_REMOVE, &json!([0]), &three, 2).0.map(|(next, active)| (next.len(), active)), Some((2, 1)));
        assert_eq!(changed(CAMERAS_MOVE, &json!([2, 0]), &three, 0).0.map(|(_, active)| active), Some(1));
        assert_eq!(changed(CAMERAS_REMOVE, &json!([5]), &three, 0), (None, refused(NO_SUCH_CAMERA)));
        assert_eq!(list(&changed(CAMERAS_MOVE, &json!([0, 1]), &three, 0).0.unwrap().0)[0][0], json!("Belly"));
    }

    #[test]
    fn a_stream_address_needs_the_scheme_its_kind_plays() {
        assert_eq!(problem(SOURCE_RTSP, "rtsp:/10.0.0.5/live"), Some(RTSP_SCHEME));
        assert_eq!(problem(SOURCE_RTSP, "RTSP://10.0.0.5/live"), None);
        assert_eq!(problem(SOURCE_WEBRTC, "rtsp://10.0.0.5/live"), Some(WHEP_SCHEME), "a WebRTC camera is not quietly played as RTSP");
        assert_eq!(problem(SOURCE_WEBRTC, "10.0.0.5:8889/cam/whep"), None, "a bare WHEP address gets http:// added");
        assert_eq!(normalized(cam("", SOURCE_WEBRTC, "HTTP://cam/whep")).url, "http://cam/whep", "the player matches lowercase schemes");
    }

    #[test]
    fn a_device_is_found_by_its_host_with_or_without_credentials_and_port() {
        assert_eq!(url_host("rtsp://admin:pw@10.0.0.5/live"), "10.0.0.5");
        assert_eq!(url_host("rtsp://admin:pw@10.0.0.5:554/live"), "10.0.0.5");
        assert_eq!(url_host("10.0.0.5:5600"), "10.0.0.5");
        let mine = vec![cam("Phone", SOURCE_BACK_CAMERA, ""), cam("old", SOURCE_RTSP, "rtsp://admin:pw@10.0.0.5/old")];
        assert_eq!(device_adopted(&mine, "10.0.0.5", vec![cam("front", SOURCE_RTSP, "rtsp://10.0.0.5/front")]), Some((vec![cam("Phone", SOURCE_BACK_CAMERA, ""), cam("front", SOURCE_RTSP, "rtsp://10.0.0.5/front")], 1)), "the device's first camera is shown");
        assert_eq!(device_adopted(&mine, "10.0.0.5", Vec::new()), None, "a device with no cameras changes nothing");
    }

    #[test]
    fn an_unnamed_camera_offered_twice_is_kept_once() {
        let list = vec![cam("", SOURCE_RTSP, "rtsp://a/live")];
        assert_eq!(with_named(&list, cam("", SOURCE_RTSP, "rtsp://a/live")), (list.clone(), 0));
        assert_eq!(with_named(&list, cam("", SOURCE_RTSP, "rtsp://a/other")).1, 1);
    }

    #[test]
    fn the_view_lists_drone_cameras_after_the_operators_and_says_when_the_list_is_unreadable() {
        let video = json!({ "activeVideoSource": 1, "cameraFromDrone": [false, true], "cameraNames": ["Front", "SIYI A8"], "cameraSources": [SOURCE_RTSP, SOURCE_UDP_H264], "cameraUrls": ["rtsp://a/front", "0.0.0.0:5600"] });
        let shown = view_of(&video, Some(vec![cam("Front", SOURCE_RTSP, "rtsp://a/front")]), 0);
        let rows = shown["cameras"].as_array().unwrap();
        assert_eq!(rows.iter().map(|row| (row["title"].clone(), row["stored"].clone(), row["fromDrone"].clone(), row["active"].clone())).collect::<Vec<_>>(), vec![(json!("Front"), json!(0), json!(false), json!(false)), (json!("SIYI A8"), Value::Null, json!(true), json!(true))]);
        let unreadable = view_of(&video, None, 0);
        assert_eq!((unreadable["readable"].clone(), unreadable["reason"].clone(), unreadable["cameras"].as_array().map(Vec::len)), (json!(false), json!(UNREADABLE), Some(1)), "drone cameras still show while the operator's list cannot be read");
    }

    #[test]
    fn an_address_typed_with_the_scheme_the_app_adds_is_kept_without_it() {
        assert_eq!(normalized(cam("", SOURCE_UDP_H264, "UDP://0.0.0.0:5600")).url, "0.0.0.0:5600");
        assert_eq!(normalized(cam("", SOURCE_UDP_H265, "udp://0.0.0.0:5600")).url, "0.0.0.0:5600");
        assert_eq!(normalized(cam("", SOURCE_TCP, "tcp://10.0.0.5:5600")).url, "10.0.0.5:5600");
        assert_eq!(normalized(cam("", SOURCE_RTSP, "rtsp://10.0.0.5/live")).url, "rtsp://10.0.0.5/live", "RTSP keeps its scheme");
        assert_eq!(problem(SOURCE_UDP_H264, &normalized(cam("", SOURCE_UDP_H264, "rtsp://x")).url), Some(DOUBLED_SCHEME), "another kind's scheme is still refused");
    }
}
