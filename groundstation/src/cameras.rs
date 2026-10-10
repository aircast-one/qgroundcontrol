use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::router::Backend;
use crate::videostate::{DEVICE_CAMERAS, SIGNAL_IDLE, SIGNAL_LIVE, SOURCE_SYNTHETIC, SOURCE_DISABLED, SOURCE_MPEGTS, SOURCE_RTSP, SOURCE_TCP, SOURCE_UDP_H264, SOURCE_UDP_H265, SOURCE_WEBRTC, URL_SOURCES, needs_url, source_usable};

pub const CAMERAS_FACT: &str = "cameras";
pub const ACTIVE_FACT: &str = "activeVideoSource";
pub const CAMERAS_PATH: &str = "settings.videoSettings.cameras";
pub const ACTIVE_PATH: &str = "settings.videoSettings.activeVideoSource";
pub const MULTI_VIEW_PATH: &str = "settings.videoSettings.multiViewEnabled";
pub const STREAM_ENABLED_PATH: &str = "settings.videoSettings.streamEnabled";
pub const DEPS: &[&str] = &[
    CAMERAS_PATH,
    ACTIVE_PATH,
    MULTI_VIEW_PATH,
    STREAM_ENABLED_PATH,
    "video.pipSlot",
    "video.cameraSignals",
    "video.cameraFromDrone",
    "video.activeVideoSource",
    "video.cameraNames",
    "video.cameraSources",
    "video.cameraUrls",
];

pub const CAMERAS_ADD: &str = "cameras.add";
pub const CAMERAS_UPDATE: &str = "cameras.update";
pub const CAMERAS_REMOVE: &str = "cameras.remove";
pub const CAMERAS_MOVE: &str = "cameras.move";
pub const CAMERAS_CLASSIFY: &str = "cameras.classify";
const CAMERAS_OWNED: [&str; 5] = [CAMERAS_ADD, CAMERAS_UPDATE, CAMERAS_REMOVE, CAMERAS_MOVE, CAMERAS_CLASSIFY];
const QT_STORE_CAMERAS: &str = "video.storeCameras";

const UNREADABLE: &str = "The camera list is not a readable list, so its cameras cannot be shown. Changing it now would replace it.";
const NO_SUCH_CAMERA: &str = "There is no camera at that position.";
const NEEDS_KIND: &str = "Pick the kind of stream this camera sends.";
const NEEDS_ADDRESS: &str = "This kind of stream needs an address.";
const RTSP_SCHEME: &str = "An RTSP address starts with rtsp://.";
const WHEP_SCHEME: &str = "A WebRTC address starts with http:// or https://.";
const DOUBLED_SCHEME: &str = "Leave the scheme off. The app adds it, and a doubled one fails to resolve.";
const UNKNOWN_ADDRESS: &str = "Start the address with rtsp://, http://, udp:// or tcp://, or type it as host:port.";
const NEEDS_LISTEN_PORT: &str = "Type the address as host:port, like 0.0.0.0:5600.";
const NEEDS_DIAL_PORT: &str = "Type the address as host:port, like 192.168.1.10:5600.";
const NEEDS_HOST: &str = "The address has no host. Add the camera's IP address or name after the scheme.";
const SCHEME_ADDED: [&str; 4] = [SOURCE_UDP_H264, SOURCE_UDP_H265, SOURCE_MPEGTS, SOURCE_TCP];

const LISTEN_DEFAULT: &str = "0.0.0.0:5600";
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
        _ if !kind_names().iter().any(|kind| kind == source) => Some(NEEDS_KIND),
        _ if needs_url(source) && url.is_empty() => Some(NEEDS_ADDRESS),
        _ if source == SOURCE_RTSP && scheme_of(source, url).is_none() => Some(RTSP_SCHEME),
        _ if source == SOURCE_WEBRTC && url.contains("://") && scheme_of(source, url).is_none() => Some(WHEP_SCHEME),
        _ if url.contains("://") && SCHEME_ADDED.contains(&source) => Some(DOUBLED_SCHEME),
        _ if SCHEME_ADDED.contains(&source) && crate::videohost::host_port(url).is_none() => Some(if source == SOURCE_TCP { NEEDS_DIAL_PORT } else { NEEDS_LISTEN_PORT }),
        _ if [SOURCE_RTSP, SOURCE_WEBRTC].contains(&source) && url_host(url).is_empty() => Some(NEEDS_HOST),
        _ => None,
    }
}

fn schemes(source: &str) -> &'static [&'static str] {
    match source {
        SOURCE_RTSP => &["rtsp://", "rtsps://"],
        SOURCE_WEBRTC => &["http://", "https://", "whep://", "wheps://"],
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

fn host_and_port(address: &str) -> bool {
    address.rsplit_once(':').is_some_and(|(host, port)| !host.is_empty() && !host.contains(['/', ' ']) && port.parse::<u16>().is_ok())
}

fn fitting(address: &str) -> Vec<&'static str> {
    match () {
        _ if address.contains("://") => URL_SOURCES.iter().copied().filter(|source| scheme_of(source, address).is_some()).collect(),
        _ if host_and_port(address) => SCHEME_ADDED.to_vec(),
        _ => Vec::new(),
    }
}

fn classified(address: &str) -> (String, Vec<&'static str>, Option<&'static str>) {
    let typed = address.trim();
    let fits = fitting(typed);
    let offered = kind_names();
    let choices: Vec<&'static str> = fits.iter().copied().filter(|source| offered.iter().any(|kind| kind == source)).collect();
    match choices.first() {
        Some(source) => {
            let normal = normalized(Camera::new("", source, typed)).url;
            let refusal = problem(source, &normal);
            (normal, choices, refusal)
        }
        None if typed.is_empty() => (String::new(), choices, Some(NEEDS_ADDRESS)),
        None => (typed.to_string(), choices, Some(UNKNOWN_ADDRESS)),
    }
}

pub fn classify(address: &str) -> Value {
    let (address, choices, refusal) = classified(address);
    json!({ "ok": true, "address": address, "kind": choices.first(), "choices": choices, "ambiguous": choices.len() > 1, "problem": refusal })
}

fn inferred(source: &str, url: &str) -> Result<String, &'static str> {
    match source.trim() {
        "" => {
            let (_, choices, refusal) = classified(url);
            choices.first().map(|source| source.to_string()).ok_or(refusal.unwrap_or(NEEDS_KIND))
        }
        named => Ok(named.to_string()),
    }
}

fn group(source: &str) -> &'static str {
    match () {
        _ if DEVICE_CAMERAS.contains(&source) || source == SOURCE_SYNTHETIC => GROUP_DEVICE,
        _ if needs_url(source) => GROUP_STREAMS,
        _ => GROUP_PRESETS,
    }
}

fn hint(source: &str) -> &'static str {
    match source {
        SOURCE_RTSP => "rtsp://192.168.1.10:8554/live",
        SOURCE_WEBRTC => "http://192.168.1.10:8889/cam/whep",
        SOURCE_TCP => "192.168.1.10:5600",
        SOURCE_UDP_H264 | SOURCE_UDP_H265 | SOURCE_MPEGTS => LISTEN_DEFAULT,
        _ => "",
    }
}

fn default_address(source: &str) -> &'static str {
    match source {
        SOURCE_UDP_H264 | SOURCE_UDP_H265 | SOURCE_MPEGTS => LISTEN_DEFAULT,
        _ => "",
    }
}

fn description(source: &str) -> String {
    match source {
        SOURCE_RTSP => "An rtsp:// address from an IP camera or video server".to_string(),
        SOURCE_UDP_H264 => "H.264 over RTP, sent to a port on this device".to_string(),
        SOURCE_UDP_H265 => "H.265 over RTP, sent to a port on this device".to_string(),
        SOURCE_TCP => "An MPEG-2 stream this device connects to".to_string(),
        SOURCE_MPEGTS => "An MPEG transport stream sent to a port on this device".to_string(),
        SOURCE_WEBRTC => "A WHEP address, like a MediaMTX server".to_string(),
        SOURCE_SYNTHETIC => "A 3D view drawn from the map and telemetry".to_string(),
        _ if DEVICE_CAMERAS.contains(&source) => format!("This device's {}", source.to_lowercase()),
        _ => crate::videostate::source_uri(source, ""),
    }
}

pub fn kinds() -> Vec<Value> {
    kind_names()
        .into_iter()
        .map(|source| json!({ "raw": source, "label": source, "group": group(&source), "needsUrl": needs_url(&source), "hint": hint(&source), "description": description(&source), "defaultAddress": default_address(&source) }))
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
    match path {
        CAMERAS_CLASSIFY => Some(classify(&text_arg(&given, 0))),
        _ => Some(edit(Some(backend), |cameras, active| changed(path, &given, cameras, active)).unwrap_or_else(|| refused(UNREADABLE))),
    }
}

type Change = (Option<(Vec<Camera>, i64)>, Value);

fn changed(path: &str, given: &Value, cameras: &[Camera], active: i64) -> Change {
    let refusal = |reason: &str| (None, refused(reason));
    let checked = |camera: Camera, write: &dyn Fn(Camera) -> Change| match problem(&camera.source, &camera.url) {
        Some(reason) => refusal(reason),
        None => write(camera),
    };
    match path {
        CAMERAS_ADD => match inferred(&text_arg(given, 1), &text_arg(given, 2)) {
            Err(reason) => refusal(reason),
            Ok(source) => checked(normalized(Camera::new(&text_arg(given, 0), &source, &text_arg(given, 2))), &|camera| {
                let next = cameras.iter().cloned().chain(std::iter::once(camera)).collect();
                (Some((next, active_after_add(active, cameras.len()))), json!({ "ok": true, "slot": cameras.len() }))
            }),
        },
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

const SHORT_LABEL_CHARS: usize = 10;

fn prefix(name: &str, slot: usize, words: usize) -> String {
    match name.split_whitespace().take(words).collect::<Vec<_>>().join(" ") {
        taken if taken.is_empty() => format!("Cam {}", slot + 1),
        taken => taken,
    }
}

fn whole_form(name: &str, slot: usize, words: usize) -> Option<String> {
    Some(prefix(name, slot, words)).filter(|taken| taken.chars().count() <= SHORT_LABEL_CHARS)
}

fn short_form(name: &str, slot: usize, words: usize) -> String {
    prefix(name, slot, words).chars().take(SHORT_LABEL_CHARS).collect::<String>().trim_end().to_string()
}

fn suffix_form(name: &str, words: usize) -> Option<String> {
    let all: Vec<&str> = name.split_whitespace().collect();
    let taken = all[all.len().saturating_sub(words)..].join(" ");
    (words < all.len() && taken.chars().any(char::is_alphabetic) && taken.chars().count() <= SHORT_LABEL_CHARS).then_some(taken)
}

fn shorts(cameras: &[(usize, &str)]) -> Vec<String> {
    let most = cameras.iter().map(|(_, name)| name.split_whitespace().count()).max().unwrap_or(0).max(1);
    let whole = (1..=most).map(|words| cameras.iter().map(|(slot, name)| whole_form(name, *slot, words)).collect::<Vec<_>>());
    let suffixes = (1..most).rev().map(|words| cameras.iter().map(|(_, name)| suffix_form(name, words)).collect::<Vec<_>>());
    let cut = (1..=most).map(|words| cameras.iter().map(|(slot, name)| Some(short_form(name, *slot, words))).collect::<Vec<_>>());
    let levels: Vec<Vec<Option<String>>> = whole.chain(suffixes).chain(cut).collect();
    let numbered: Vec<String> = cameras.iter().map(|(slot, name)| format!("{} {}", short_form(name, *slot, 1), slot + 1)).collect();
    let unique = |level: &Vec<Option<String>>, row: usize| {
        level[row].as_ref().filter(|label| level.iter().flatten().filter(|other| other == label).count() == 1 && !numbered.contains(label)).cloned()
    };
    let picked: Vec<String> = (0..cameras.len()).map(|row| levels.iter().find_map(|level| unique(level, row)).unwrap_or_else(|| numbered[row].clone())).collect();
    picked.iter().enumerate().map(|(row, label)| if picked.iter().filter(|other| *other == label).count() > 1 { numbered[row].clone() } else { label.clone() }).collect()
}

pub fn cameras_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let video = backend.value_fields("video", "activeVideoSource,cameraSignals,cameraFromDrone,cameraNames,cameraSources,cameraUrls,pipSlot");
    let switched = |path: &str, default: bool| backend.value(path).get("value").and_then(Value::as_bool).unwrap_or(default);
    let (cameras, active) = listed();
    view_of(&video, cameras, active, switched(MULTI_VIEW_PATH, false) && switched(STREAM_ENABLED_PATH, true))
}

pub fn view_of(video: &Value, stored: Option<Vec<Camera>>, stored_active: i64, pip_enabled: bool) -> Value {
    let strings = |key: &str| -> Vec<String> { video.get(key).and_then(Value::as_array).map(|a| a.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect()).unwrap_or_default() };
    let drones: Vec<bool> = video.get("cameraFromDrone").and_then(Value::as_array).map(|a| a.iter().map(|v| v.as_bool().unwrap_or(false)).collect()).unwrap_or_default();
    let active = crate::read::integer(video, "activeVideoSource").unwrap_or(stored_active);
    let (names, sources, urls, signals) = (strings("cameraNames"), strings("cameraSources"), strings("cameraUrls"), strings("cameraSignals"));
    let at = |list: &[String], slot: usize| list.get(slot).map_or("", String::as_str).to_string();
    let readable = stored.is_some();
    let listed: Vec<(usize, Camera, bool)> = stored
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(slot, camera)| (slot, camera, false))
        .chain(drones.iter().enumerate().filter(|(_, drone)| **drone).map(|(slot, _)| (slot, Camera::new(&at(&names, slot), &at(&sources, slot), &at(&urls, slot)), true)))
        .collect();
    let labels = shorts(&listed.iter().map(|(slot, camera, _)| (*slot, camera.name.as_str())).collect::<Vec<_>>());
    let rows: Vec<Value> = listed
        .iter()
        .zip(labels)
        .map(|((slot, camera, from_drone), short)| {
            json!({
                "slot": slot,
                "stored": (!from_drone).then_some(slot),
                "title": title(&camera.name, *slot),
                "short": short,
                "name": camera.name,
                "source": camera.source,
                "url": camera.url,
                "summary": summary(camera),
                "problem": (!from_drone).then(|| problem(&camera.source, &camera.url)).flatten(),
                "fromDrone": from_drone,
                "active": *slot as i64 == active,
                "status": if camera.source == SOURCE_SYNTHETIC { SIGNAL_LIVE } else { signals.get(*slot).map_or(SIGNAL_IDLE, String::as_str) },
            })
        })
        .collect();
    let pip = json!({ "enabled": pip_enabled, "slot": crate::read::integer(video, "pipSlot") });
    json!({ "kind": "object", "class": "Cameras", "readable": readable, "reason": (!readable).then_some(UNREADABLE), "cameras": rows, "active": active, "pip": pip, "kinds": kinds() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::videostate::{SOURCE_3DR_SOLO, SOURCE_BACK_CAMERA, SOURCE_HERELINK_HOTSPOT, SOURCE_PARROT_DISCOVERY, SOURCE_YUNEEC_MANTIS_G};

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
        assert_eq!(problem("No such kind", ""), Some(NEEDS_KIND));
    }

    #[test]
    fn every_kind_the_old_app_listed_is_offered_with_a_line_saying_what_it_is() {
        let listed = kinds();
        let raws: Vec<&str> = listed.iter().filter_map(|kind| kind["raw"].as_str()).collect();
        let old_app = [SOURCE_RTSP, SOURCE_UDP_H264, SOURCE_UDP_H265, SOURCE_TCP, SOURCE_MPEGTS, SOURCE_WEBRTC, SOURCE_3DR_SOLO, SOURCE_PARROT_DISCOVERY, SOURCE_YUNEEC_MANTIS_G, SOURCE_HERELINK_HOTSPOT];
        assert!(old_app.iter().all(|source| raws.contains(source)), "{raws:?}");
        let solo = listed.iter().find(|kind| kind["raw"] == SOURCE_3DR_SOLO).unwrap();
        assert_eq!((solo["group"].as_str(), solo["needsUrl"].as_bool(), solo["description"].as_str()), (Some(GROUP_PRESETS), Some(false), Some("udp://0.0.0.0:5600")), "a preset says the address it listens on");
        assert_eq!(problem(SOURCE_3DR_SOLO, ""), None);
        assert!(listed.iter().all(|kind| !kind["description"].as_str().unwrap_or_default().is_empty()));
        let udp = listed.iter().find(|kind| kind["raw"] == SOURCE_UDP_H265).unwrap();
        assert_eq!(udp["defaultAddress"], "0.0.0.0:5600", "a listening stream starts filled with the port most radios send to");
        assert_eq!(listed.iter().find(|kind| kind["raw"] == SOURCE_RTSP).unwrap()["defaultAddress"], "", "an address to dial has no sensible default");
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
        let shown = view_of(&video, Some(vec![cam("Front", SOURCE_RTSP, "rtsp://a/front")]), 0, false);
        let rows = shown["cameras"].as_array().unwrap();
        assert_eq!(rows.iter().map(|row| (row["title"].clone(), row["stored"].clone(), row["fromDrone"].clone(), row["active"].clone())).collect::<Vec<_>>(), vec![(json!("Front"), json!(0), json!(false), json!(false)), (json!("SIYI A8"), Value::Null, json!(true), json!(true))]);
        let unreadable = view_of(&video, None, 0, false);
        assert_eq!((unreadable["readable"].clone(), unreadable["reason"].clone(), unreadable["cameras"].as_array().map(Vec::len)), (json!(false), json!(UNREADABLE), Some(1)), "drone cameras still show while the operator's list cannot be read");
    }

    #[test]
    fn a_synthetic_view_is_a_camera_drawn_here_that_streams_nothing() {
        let video = json!({ "activeVideoSource": 0, "cameraSignals": ["noSignal"] });
        let row = view_of(&video, Some(vec![cam("Synthetic", SOURCE_SYNTHETIC, "")]), 0, false)["cameras"][0].clone();
        assert_eq!(row["status"].as_str(), Some(SIGNAL_LIVE), "it is drawn on this device, so it is always ready");
        assert_eq!(group(SOURCE_SYNTHETIC), GROUP_DEVICE);
        assert!(source_usable(SOURCE_SYNTHETIC, "") && !needs_url(SOURCE_SYNTHETIC));
        assert_eq!(crate::videostate::source_uri(SOURCE_SYNTHETIC, ""), "", "no pipeline starts for it, so the head draws it where the picture would be");
    }

    #[test]
    fn an_address_typed_with_the_scheme_the_app_adds_is_kept_without_it() {
        assert_eq!(normalized(cam("", SOURCE_UDP_H264, "UDP://0.0.0.0:5600")).url, "0.0.0.0:5600");
        assert_eq!(normalized(cam("", SOURCE_UDP_H265, "udp://0.0.0.0:5600")).url, "0.0.0.0:5600");
        assert_eq!(normalized(cam("", SOURCE_TCP, "tcp://10.0.0.5:5600")).url, "10.0.0.5:5600");
        assert_eq!(normalized(cam("", SOURCE_RTSP, "rtsp://10.0.0.5/live")).url, "rtsp://10.0.0.5/live", "RTSP keeps its scheme");
        assert_eq!(problem(SOURCE_UDP_H264, &normalized(cam("", SOURCE_UDP_H264, "rtsp://x")).url), Some(DOUBLED_SCHEME), "another kind's scheme is still refused");
    }

    fn labelled(names: &[&str]) -> Vec<String> {
        shorts(&names.iter().copied().enumerate().collect::<Vec<_>>())
    }

    #[test]
    fn a_compact_switch_reads_the_first_word_of_the_name_or_the_camera_number() {
        assert_eq!(labelled(&["SIYI A8 mini", "  Thermal  ", "Downward-looking", "Тепловізійна", ""]), vec!["SIYI", "Thermal", "Downward-l", "Тепловізій", "Cam 5"], "ten characters at most, counted as characters rather than bytes, and a whole word beats a cut one");
        assert_eq!(labelled(&["Gimbal Left", "Gimbal Right"]), vec!["Left", "Right"], "a name that fits only once cut reads by the word that ends it");
        assert_eq!(labelled(&["Downward-looking belly"]), vec!["belly"]);
        let video = json!({ "activeVideoSource": 0, "cameraFromDrone": [false, true], "cameraNames": ["", ""], "cameraSources": [SOURCE_RTSP, SOURCE_UDP_H264], "cameraUrls": ["rtsp://a/front", "0.0.0.0:5600"] });
        let rows = view_of(&video, Some(vec![cam("", SOURCE_RTSP, "rtsp://a/front")]), 0, false)["cameras"].clone();
        assert_eq!((rows[0]["short"].clone(), rows[1]["short"].clone()), (json!("Cam 1"), json!("Cam 2")));
    }

    #[test]
    fn two_cameras_never_share_a_short_label() {
        assert_eq!(labelled(&["SIYI A8", "SIYI ZR10", "Front"]), vec!["SIYI A8", "SIYI ZR10", "Front"], "two words tell the SIYIs apart and the camera nobody clashes with keeps its one word");
        assert_eq!(labelled(&["SIYI", "SIYI A8", "SIYI A8 mini"]), vec!["SIYI", "SIYI A8", "A8 mini"], "each takes the fewest words that set it apart, and a name too long for that reads by how it ends rather than cut short");
        assert_eq!(labelled(&["Gimbal Camera Left", "Gimbal Camera Right", "Gimbal"]), vec!["Left", "Right", "Gimbal"], "names that still match at ten characters are told apart by how they end");
        assert_eq!(labelled(&["MockCam 1 · Stream 1-1", "MockCam 1 · Stream 1-2"]), vec!["Stream 1-1", "Stream 1-2"], "streams of one drone camera read by stream, not by an invented camera number");
        assert_eq!(labelled(&["Gimbal Camera A long", "Gimbal Camera B long"]), vec!["A long", "B long"]);
        assert_eq!(labelled(&["Nose", "Nose"]), vec!["Nose 1", "Nose 2"]);
        assert_eq!(labelled(&["", "Cam 1", "Cam 2"]), vec!["Cam 1", "Cam 2", "Cam 3"], "a name that reads like another camera's numbered label gives way to the number");
        [vec!["Nose", "Nose", "Nose 1"], vec!["Nose 1", "Nose", "Nose"], vec!["", "", "Cam 2", "Cam 1 x", "Cam 1 y"], vec!["A B", "A B", "A 1", "A 2 x"]].iter().for_each(|names| {
            let labels = labelled(names);
            assert_eq!(labels.iter().collect::<std::collections::HashSet<_>>().len(), names.len(), "{names:?} came out as {labels:?}");
        });
        let video = json!({ "activeVideoSource": 0, "cameraFromDrone": [false, true], "cameraNames": ["SIYI A8", "SIYI ZR10"], "cameraSources": [SOURCE_RTSP, SOURCE_UDP_H264], "cameraUrls": ["rtsp://a/front", "0.0.0.0:5600"] });
        let rows = view_of(&video, Some(vec![cam("SIYI A8", SOURCE_RTSP, "rtsp://a/front")]), 0, false)["cameras"].clone();
        assert_eq!((rows[0]["short"].clone(), rows[1]["short"].clone()), (json!("SIYI A8"), json!("SIYI ZR10")), "a drone camera is counted among the operator's");
    }

    fn statuses(video: Value) -> Vec<Value> {
        let three = vec![cam("A", SOURCE_RTSP, "rtsp://a/0"), cam("B", SOURCE_RTSP, "rtsp://a/1"), cam("C", SOURCE_RTSP, "rtsp://a/2")];
        view_of(&video, Some(three), 0, true)["cameras"].as_array().unwrap().iter().map(|row| row["status"].clone()).collect()
    }

    #[test]
    fn each_row_reads_its_status_straight_from_the_hosts_camera_signals() {
        assert_eq!(statuses(json!({ "activeVideoSource": 0, "cameraSignals": ["live", "noSignal", "idle"] })), vec![json!("live"), json!("noSignal"), json!("idle")]);
        assert_eq!(statuses(json!({ "activeVideoSource": 1, "cameraSignals": ["idle", "connecting", "connecting"] })), vec![json!("idle"), json!("connecting"), json!("connecting")]);
        let drone = json!({ "activeVideoSource": 3, "cameraSignals": ["idle", "idle", "idle", "live"], "cameraFromDrone": [false, false, false, true], "cameraNames": ["A", "B", "C", "SIYI A8"], "cameraSources": [SOURCE_RTSP, SOURCE_RTSP, SOURCE_RTSP, SOURCE_UDP_H264], "cameraUrls": ["rtsp://a/0", "rtsp://a/1", "rtsp://a/2", "0.0.0.0:5600"] });
        assert_eq!(statuses(drone)[3], json!("live"), "a drone camera reads the signal at its own slot");
        assert_eq!(statuses(json!({ "activeVideoSource": 0 })), vec![json!("idle"); 3], "a host that serves no signals leaves every camera idle");
    }

    fn pip(video: Value, pip_enabled: bool) -> Value {
        let three = vec![cam("A", SOURCE_RTSP, "rtsp://a/0"), cam("B", SOURCE_RTSP, "rtsp://a/1"), cam("C", SOURCE_RTSP, "rtsp://a/2")];
        view_of(&video, Some(three), 0, pip_enabled)["pip"].clone()
    }

    #[test]
    fn the_picture_in_picture_names_the_camera_the_host_plays_there() {
        assert_eq!(pip(json!({ "activeVideoSource": 0, "pipSlot": 2 }), true), json!({ "enabled": true, "slot": 2 }), "the host picks the camera, so the view and the receiver cannot disagree");
        assert_eq!(pip(json!({ "activeVideoSource": 0, "pipSlot": 1 }), false), json!({ "enabled": false, "slot": 1 }), "with picture in picture or the stream off the camera it would show is still named");
        assert_eq!(pip(json!({ "activeVideoSource": 0, "pipSlot": null }), true), json!({ "enabled": true, "slot": null }), "no camera to put beside the main one, no slot");
        assert_eq!(pip(json!({ "activeVideoSource": 0 }), true)["slot"], Value::Null, "a host that serves no slot names none");
    }

    struct Switches {
        multi_view: bool,
        stream: Option<bool>,
    }

    impl Backend for Switches {
        fn get(&self, path: &str) -> String {
            match path {
                MULTI_VIEW_PATH => json!({ "kind": "value", "value": self.multi_view }).to_string(),
                STREAM_ENABLED_PATH => self.stream.map_or(String::new(), |on| json!({ "kind": "value", "value": on }).to_string()),
                _ => String::new(),
            }
        }
        fn get_fields(&self, _path: &str, _fields: &str) -> String {
            json!({ "activeVideoSource": 0, "pipSlot": 1 }).to_string()
        }
        fn set(&self, _path: &str, _value: &str) -> String {
            String::new()
        }
        fn invoke(&self, _path: &str, _args: &str) -> String {
            String::new()
        }
        fn watch(&self, _paths: &[String]) {}
    }

    #[test]
    fn the_picture_in_picture_is_offered_only_while_the_stream_is_on() {
        let enabled = |multi_view: bool, stream: Option<bool>| cameras_view(&Switches { multi_view, stream }, &[])["pip"].clone();
        assert_eq!(enabled(true, Some(true)), json!({ "enabled": true, "slot": 1 }));
        assert_eq!(enabled(true, Some(false))["enabled"], false, "with the stream off there is no picture to put a thumbnail beside");
        assert_eq!(enabled(false, Some(true))["enabled"], false);
        assert_eq!(enabled(true, None)["enabled"], true, "the stream is on unless a setting says otherwise");
        assert!(DEPS.contains(&"video.pipSlot") && DEPS.contains(&STREAM_ENABLED_PATH), "the view is recomputed when either changes");
    }

    #[test]
    fn an_address_the_player_cannot_open_is_refused_before_it_is_saved() {
        assert_eq!(classify("udp://0.0.0.0")["problem"], json!(NEEDS_LISTEN_PORT), "a listen address with no port has no socket to open");
        assert_eq!(changed(CAMERAS_ADD, &json!(["", "", "udp://0.0.0.0"]), &[], 0), (None, refused(NEEDS_LISTEN_PORT)), "and adding it says the same");
        assert_eq!(problem(SOURCE_MPEGTS, "0.0.0.0:99999"), Some(NEEDS_LISTEN_PORT));
        assert_eq!(problem(SOURCE_UDP_H265, ":5600"), None, "a bare port listens on every interface, as the pipeline reads it");
        assert_eq!(classify("tcp://cam")["problem"], json!(NEEDS_DIAL_PORT));
        assert_eq!(changed(CAMERAS_ADD, &json!(["", "", "tcp://cam"]), &[], 0), (None, refused(NEEDS_DIAL_PORT)));
        assert_eq!(problem(SOURCE_TCP, "cam:5600/feed"), None);
        assert_eq!(classify("rtsp://")["problem"], json!(NEEDS_HOST));
        assert_eq!(changed(CAMERAS_ADD, &json!(["", "", "rtsp://"]), &[], 0), (None, refused(NEEDS_HOST)));
        assert_eq!(problem(SOURCE_RTSP, "rtsp://admin:pw@:554/live"), Some(NEEDS_HOST), "credentials are not a host");
        assert_eq!(classify("http://")["problem"], json!(NEEDS_HOST));
        assert_eq!(changed(CAMERAS_ADD, &json!(["", "", "http://"]), &[], 0), (None, refused(NEEDS_HOST)));
        assert_eq!(problem(SOURCE_WEBRTC, "/cam/whep"), Some(NEEDS_HOST));
        let three = vec![cam("A", SOURCE_UDP_H264, "0.0.0.0:5600")];
        assert_eq!(changed(CAMERAS_UPDATE, &json!([0, "A", SOURCE_UDP_H264, "0.0.0.0"]), &three, 0), (None, refused(NEEDS_LISTEN_PORT)), "an edit is held to the same rule");
    }

    fn classified_as(address: &str) -> (Value, Value, Value, Value, Value) {
        let answer = classify(address);
        (answer["address"].clone(), answer["kind"].clone(), answer["choices"].clone(), answer["ambiguous"].clone(), answer["problem"].clone())
    }

    #[test]
    fn an_address_says_what_kind_of_stream_it_is() {
        let none = Value::Null;
        assert_eq!(classified_as("RTSP://10.0.0.5:8554/live"), (json!("rtsp://10.0.0.5:8554/live"), json!(SOURCE_RTSP), json!([SOURCE_RTSP]), json!(false), none.clone()));
        assert_eq!(classified_as("rtsps://cam/live").1, json!(SOURCE_RTSP));
        assert_eq!(classified_as(" https://sfu/cam/whep ").0, json!("https://sfu/cam/whep"));
        assert_eq!(classified_as("http://10.0.0.5:8889/cam/whep").1, json!(SOURCE_WEBRTC));
        assert_eq!(classified_as("whep://sfu/cam").1, json!(SOURCE_WEBRTC), "the player opens whep:// itself");
        assert_eq!(classified_as("wheps://sfu/cam").4, none);
        assert_eq!(classified_as("udp://0.0.0.0:5600"), (json!("0.0.0.0:5600"), json!(SOURCE_UDP_H264), json!([SOURCE_UDP_H264, SOURCE_UDP_H265, SOURCE_MPEGTS]), json!(true), none.clone()), "plain udp could carry any of three encodings");
        assert_eq!(classified_as("udp265://0.0.0.0:5600"), (json!("0.0.0.0:5600"), json!(SOURCE_UDP_H265), json!([SOURCE_UDP_H265]), json!(false), none.clone()));
        assert_eq!(classified_as("mpegts://0.0.0.0:5600").2, json!([SOURCE_MPEGTS]));
        assert_eq!(classified_as("tcp://10.0.0.5:5600"), (json!("10.0.0.5:5600"), json!(SOURCE_TCP), json!([SOURCE_TCP]), json!(false), none.clone()));
        assert_eq!(classified_as("0.0.0.0:5600"), (json!("0.0.0.0:5600"), json!(SOURCE_UDP_H264), json!([SOURCE_UDP_H264, SOURCE_UDP_H265, SOURCE_MPEGTS, SOURCE_TCP]), json!(true), none.clone()), "a bare host and port is a listen address unless the operator says otherwise");
        assert_eq!(classified_as("  "), (json!(""), none.clone(), json!([]), json!(false), json!(NEEDS_ADDRESS)));
        assert_eq!(classified_as("udp://"), (json!(""), json!(SOURCE_UDP_H264), json!([SOURCE_UDP_H264, SOURCE_UDP_H265, SOURCE_MPEGTS]), json!(true), json!(NEEDS_ADDRESS)), "the refusal is the one adding it would give");
        assert_eq!(classified_as("ftp://cam/live"), (json!("ftp://cam/live"), none.clone(), json!([]), json!(false), json!(UNKNOWN_ADDRESS)));
        assert_eq!(classified_as("rtsp:/10.0.0.5/live").4, json!(UNKNOWN_ADDRESS));
        assert_eq!(classified_as("10.0.0.5:8554/live").1, none, "a path with no scheme is not guessed at");
        assert_eq!(classified_as("cam.local").4, json!(UNKNOWN_ADDRESS));
        assert_eq!(problem(SOURCE_WEBRTC, "whep://sfu/cam"), None, "what classify accepts, adding accepts");
    }

    #[test]
    fn a_camera_added_without_a_kind_takes_the_kind_its_address_names() {
        let (write, answer) = changed(CAMERAS_ADD, &json!(["Nose", "", "RTSP://a/nose"]), &[], 0);
        assert_eq!((write, answer["slot"].as_u64()), (Some((vec![cam("Nose", SOURCE_RTSP, "rtsp://a/nose")], 0)), Some(0)));
        let (write, _) = changed(CAMERAS_ADD, &json!(["", "", "udp://0.0.0.0:5600"]), &[], 0);
        assert_eq!(write.map(|(next, _)| next), Some(vec![cam("", SOURCE_UDP_H264, "0.0.0.0:5600")]), "an ambiguous address takes the first kind on offer");
        let (write, _) = changed(CAMERAS_ADD, &json!(["", SOURCE_UDP_H265, "udp://0.0.0.0:5600"]), &[], 0);
        assert_eq!(write.map(|(next, _)| next), Some(vec![cam("", SOURCE_UDP_H265, "0.0.0.0:5600")]), "a kind the operator picked is kept");
        assert_eq!(changed(CAMERAS_ADD, &json!(["", "", "ftp://cam"]), &[], 0), (None, refused(UNKNOWN_ADDRESS)));
        assert_eq!(changed(CAMERAS_ADD, &json!(["", "", ""]), &[], 0), (None, refused(NEEDS_ADDRESS)));
        assert!(owns(CAMERAS_CLASSIFY), "the core answers classify itself and never asks Qt");
    }
}
