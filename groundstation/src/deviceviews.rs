use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use base64::Engine;
use serde_json::{Value, json};
use url::Url;

use crate::router::Backend;

pub const VIEWS_CHANGED: &str = "core.deviceViews@changed";
pub const DEPS: &[&str] = &[VIEWS_CHANGED];
pub const CONNECT: &str = "device.connect";
pub const DISCONNECT: &str = "device.disconnect";
pub const CALL: &str = "device.call";
pub const READ: &str = "device.read";
pub const UPLOAD: &str = "device.upload";
pub const QR: &str = "device.qr";
pub const REACH: &str = "device.reach";
const DEVICE_ACTIONS: &[&str] = &[CONNECT, DISCONNECT, CALL, READ, UPLOAD, QR, REACH];

const ORIGIN_KEY: &str = "AircastDevice/origin";
const RECONNECT: Duration = Duration::from_secs(2);
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const CALL_TIMEOUT: Duration = Duration::from_secs(20);
const LONGEST_CALL_S: u64 = 300;
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(600);
const REACH_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(target_family = "wasm")]
const MAX_VIEW_BYTES: usize = 4 << 20;
const NO_ANSWER: &str = "The device did not answer.";
const NOT_CONNECTED: &str = "No Aircast device is connected.";
const TYPE_AN_ADDRESS: &str = "Type an address like 192.168.1.100 or falcon.local.";
const ACCEPT_JSON: (&str, &str) = ("Accept", "application/json");
const HOME_VIEW: &str = "/api/views/aircraft";
const SECURE_ORIGIN: &str = "/api/tls";

#[derive(Default)]
struct Page {
    view: Value,
    following: bool,
}

#[derive(Default)]
struct Feed {
    origin: Option<String>,
    generation: u64,
    pages: BTreeMap<String, Page>,
    watched: BTreeSet<String>,
    dirty: bool,
}

static FEED: Mutex<Feed> = Mutex::new(Feed {
    origin: None,
    generation: 0,
    pages: BTreeMap::new(),
    watched: BTreeSet::new(),
    dirty: false,
});

fn lock() -> MutexGuard<'static, Feed> {
    FEED.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Feed {
    fn origin(&self) -> String {
        self.origin
            .clone()
            .or_else(|| crate::settingsstore::stored_text(ORIGIN_KEY))
            .unwrap_or_default()
    }

    fn retarget(&mut self, origin: &str) {
        self.origin = Some(origin.to_string());
        self.generation += 1;
        self.pages.clear();
        self.dirty = true;
    }

    fn wanted(&self, generation: u64, page: &str) -> bool {
        self.generation == generation && self.watched.contains(page)
    }

    fn receive(&mut self, generation: u64, page: &str, view: Value) {
        if let Some(held) = self
            .pages
            .get_mut(page)
            .filter(|held| self.generation == generation && held.view != view)
        {
            held.view = view;
            self.dirty = true;
        }
    }

    fn stopped(&mut self, generation: u64, page: &str) {
        if let Some(held) = self
            .pages
            .get_mut(page)
            .filter(|_| self.generation == generation)
        {
            held.following = false;
        }
    }
}

pub fn take_changed() -> bool {
    std::mem::take(&mut lock().dirty)
}

fn watched_page(path: &str) -> Option<String> {
    match crate::view::split(path) {
        ("view.device", args) => args.into_iter().next().filter(|page| page_name(page)),
        _ => None,
    }
}

pub fn follow_watched(paths: &BTreeSet<String>) {
    let mut feed = lock();
    feed.watched = paths.iter().filter_map(|path| watched_page(path)).collect();
    let origin = feed.origin();
    let generation = feed.generation;
    let starts: Vec<String> = feed
        .watched
        .iter()
        .filter(|page| {
            !origin.is_empty() && !feed.pages.get(*page).is_some_and(|held| held.following)
        })
        .cloned()
        .collect();
    starts
        .iter()
        .for_each(|page| feed.pages.entry(page.clone()).or_default().following = true);
    drop(feed);
    starts.into_iter().for_each(|page| {
        let followed = page.clone();
        let origin = origin.clone();
        if crate::background::spawn("groundstation-device-view", move || {
            follow(origin, generation, followed)
        })
        .is_err()
        {
            lock().stopped(generation, &page);
        }
    });
}

pub fn owns(path: &str) -> bool {
    DEVICE_ACTIONS.contains(&path)
}

fn has_scheme(text: &str) -> bool {
    text.split_once("://").is_some_and(|(scheme, _)| {
        scheme
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
    })
}

pub fn address(typed: &str) -> Option<String> {
    let text = typed.trim();
    let full = if has_scheme(text) {
        text.to_string()
    } else {
        format!("http://{text}")
    };
    Url::parse(&full)
        .ok()
        .filter(|url| {
            !text.is_empty()
                && matches!(url.scheme(), "http" | "https")
                && url.host_str().is_some_and(|host| !host.is_empty())
        })
        .map(|url| url.origin().ascii_serialization())
}

pub fn paired_host() -> String {
    host_of(&lock().origin())
}

pub fn host_of(origin: &str) -> String {
    Url::parse(origin)
        .ok()
        .and_then(|url| {
            url.host_str().map(|host| {
                url.port()
                    .map_or(host.to_string(), |port| format!("{host}:{port}"))
            })
        })
        .unwrap_or_default()
}

fn resolve(origin: &str, path: &str) -> Option<String> {
    let base = Url::parse(origin).ok()?;
    base.join(path)
        .ok()
        .filter(|url| path.starts_with('/') && url.origin() == base.origin())
        .map(String::from)
}

fn page_name(page: &str) -> bool {
    !page.is_empty()
        && page
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn event_view(line: &str) -> Option<Value> {
    line.strip_prefix("data:")
        .map(str::trim)
        .filter(|data| !data.is_empty())
        .and_then(|data| serde_json::from_str(data).ok())
        .filter(Value::is_object)
}

fn events_url(origin: &str, page: &str) -> String {
    format!("{origin}/api/views/{page}/events")
}

pub fn device_view(_backend: &dyn Backend, args: &[String]) -> Value {
    page_view(args.first().map(String::as_str).unwrap_or_default())
}

fn page_view(page: &str) -> Value {
    if !page_name(page) {
        return json!({ "kind": "refused", "reason": "view.device takes the name of an Aircast device page, as view.device(aircraft)" });
    }
    let feed = lock();
    let origin = feed.origin();
    let view = feed
        .pages
        .get(page)
        .map_or(Value::Null, |held| held.view.clone());
    json!({ "kind": "object", "class": "DeviceView", "page": page, "origin": origin, "host": host_of(&origin), "answered": !view.is_null(), "view": view })
}

fn take_views(lines: impl Iterator<Item = String>, generation: u64, page: &str) -> usize {
    lines
        .take_while(|_| lock().wanted(generation, page))
        .filter_map(|line| event_view(&line))
        .map(|view| lock().receive(generation, page, view))
        .count()
}

#[cfg(not(target_family = "wasm"))]
fn follow(origin: String, generation: u64, page: String) {
    use std::io::BufRead;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(PROBE_TIMEOUT))
        .timeout_recv_body(Some(Duration::from_secs(25)))
        .build()
        .into();
    let url = events_url(&origin, &page);
    let wanted = || lock().wanted(generation, &page);
    while wanted() {
        let heard = agent
            .get(&url)
            .header("Accept", "text/event-stream")
            .call()
            .map(|response| {
                take_views(
                    std::io::BufReader::new(response.into_body().into_reader())
                        .lines()
                        .map_while(Result::ok),
                    generation,
                    &page,
                )
            });
        if !matches!(heard, Ok(views) if views > 0) {
            std::thread::sleep(RECONNECT);
        }
    }
    lock().stopped(generation, &page);
}

#[cfg(target_family = "wasm")]
fn follow(origin: String, generation: u64, page: String) {
    wasm_bindgen_futures::spawn_local(async move {
        let url = events_url(&origin, &page);
        let wanted = || lock().wanted(generation, &page);
        while wanted() {
            let _ = crate::http::stream_lines(&url, MAX_VIEW_BYTES, |line| {
                if let Some(view) = event_view(line) {
                    lock().receive(generation, &page, view);
                }
                wanted()
            })
            .await;
            crate::background::pause(RECONNECT).await;
        }
        lock().stopped(generation, &page);
    });
}

fn adopt(origin: &str) {
    lock().retarget(origin);
    crate::settingsstore::written(ORIGIN_KEY, origin);
}

pub fn adopt_host(host: &str) {
    if let Some(origin) = address(host) {
        adopt(&origin);
    }
}

#[cfg(target_family = "wasm")]
const ALLOW_LOCAL_NETWORK: &str =
    " When the browser asks to let this site reach devices on your local network, allow it.";
#[cfg(not(target_family = "wasm"))]
const ALLOW_LOCAL_NETWORK: &str = "";

fn unreachable(origin: &str) -> String {
    format!(
        "{} did not answer as an Aircast device. Check it is on this network.{ALLOW_LOCAL_NETWORK}",
        host_of(origin)
    )
}

fn secure_origin(sent: Result<(u16, Vec<u8>), String>) -> Option<String> {
    sent.ok()
        .filter(|(status, _)| (200..300).contains(status))
        .and_then(|(_, bytes)| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|answer| {
            answer
                .get("origin")
                .and_then(Value::as_str)
                .and_then(|origin| Url::parse(origin).ok())
        })
        .filter(|url| {
            url.scheme() == "https" && url.host_str().is_some_and(|host| !host.is_empty())
        })
        .map(|url| url.origin().ascii_serialization())
}

fn upgraded(origin: &str) -> Option<String> {
    let asked = resolve(origin, SECURE_ORIGIN).filter(|_| origin.starts_with("http://"))?;
    let secure = secure_origin(crate::http::send(
        "GET",
        &asked,
        &[ACCEPT_JSON],
        None,
        PROBE_TIMEOUT,
    ))?;
    let home = resolve(&secure, HOME_VIEW)?;
    answers_as_device(&crate::http::send(
        "GET",
        &home,
        &[ACCEPT_JSON],
        None,
        PROBE_TIMEOUT,
    ))
    .then_some(secure)
}

fn answers_as_device(sent: &Result<(u16, Vec<u8>), String>) -> bool {
    sent.as_ref()
        .ok()
        .filter(|(status, _)| (200..300).contains(status))
        .and_then(|(_, bytes)| serde_json::from_slice::<Value>(bytes).ok())
        .is_some_and(|view| {
            view.get("headline")
                .and_then(Value::as_str)
                .is_some_and(|headline| !headline.is_empty())
        })
}

pub fn pair(args: &str) -> Value {
    let paired = crate::device::pair_command(args);
    if paired["ok"] == true {
        adopt_host(argument(&arguments(args), 0));
    }
    paired
}

fn refused(reason: &str) -> Value {
    json!({ "ok": false, "reason": reason, "field": "" })
}

pub fn answer(sent: Result<(u16, Vec<u8>), String>) -> Value {
    match sent {
        Err(_) => refused(NO_ANSWER),
        Ok((status, bytes)) => {
            let body = serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null);
            match (200..300).contains(&status) {
                true => json!({ "ok": true, "result": body }),
                false => json!({
                    "ok": false,
                    "reason": body.get("error").and_then(Value::as_str).filter(|error| !error.is_empty()).map_or_else(|| format!("The device refused ({status})."), str::to_string),
                    "field": body.get("field").and_then(Value::as_str).unwrap_or_default(),
                    "result": body,
                }),
            }
        }
    }
}

fn argument(args: &[Value], index: usize) -> &str {
    args.get(index).and_then(Value::as_str).unwrap_or_default()
}

fn method_of(args: &[Value]) -> Option<&'static str> {
    ["POST", "PUT", "DELETE"]
        .into_iter()
        .find(|method| *method == argument(args, 0))
}

fn target(path: &str) -> Result<String, Value> {
    let origin = lock().origin();
    match origin.is_empty() {
        true => Err(refused(NOT_CONNECTED)),
        false => resolve(&origin, path)
            .ok_or_else(|| refused(&format!("{path} is not a path on the device."))),
    }
}

pub enum Part {
    File {
        name: String,
        file_name: String,
        bytes: Vec<u8>,
    },
    Text {
        name: String,
        value: String,
    },
}

fn part_bytes(boundary: &str, part: &Part) -> Vec<u8> {
    match part {
        Part::File { name, file_name, bytes } => [
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"{file_name}\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes(),
            bytes,
            b"\r\n",
        ]
        .concat(),
        Part::Text { name, value } => format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").into_bytes(),
    }
}

pub fn multipart(boundary: &str, parts: &[Part]) -> Vec<u8> {
    parts
        .iter()
        .flat_map(|part| part_bytes(boundary, part))
        .chain(format!("--{boundary}--\r\n").into_bytes())
        .collect()
}

fn read_part(part: &Value) -> Result<Part, String> {
    let name = part
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .replace('"', "");
    match part.get("file").and_then(Value::as_str) {
        None => Ok(Part::Text {
            name,
            value: part
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        }),
        Some(file) => {
            let file_name = std::path::Path::new(file)
                .file_name()
                .and_then(|it| it.to_str())
                .unwrap_or("file")
                .replace('"', "");
            crate::vfs::read(file)
                .map(|bytes| Part::File {
                    name,
                    file_name: file_name.clone(),
                    bytes,
                })
                .map_err(|error| format!("Could not read {file_name}: {error}"))
        }
    }
}

enum Then {
    Answer,
    Adopt(String),
    Qr,
    Reach,
}

struct Request {
    method: &'static str,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
    timeout: Duration,
    then: Then,
}

enum Step {
    Answered(Value),
    Send(Request),
}

fn owned(headers: &[(&str, &str)]) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

fn get(url: String, accept: &str, timeout: Duration, then: Then) -> Step {
    Step::Send(Request {
        method: "GET",
        url,
        headers: owned(&[("Accept", accept)]),
        body: None,
        timeout,
        then,
    })
}

fn upload(method: &'static str, url: String, parts: &[Value]) -> Step {
    match parts
        .iter()
        .map(read_part)
        .collect::<Result<Vec<Part>, String>>()
    {
        Err(reason) => Step::Answered(refused(&reason)),
        Ok(parts) => {
            let boundary = format!("aircast{:x}", crate::hub::now_ms());
            Step::Send(Request {
                method,
                url,
                headers: owned(&[
                    ACCEPT_JSON,
                    (
                        "Content-Type",
                        &format!("multipart/form-data; boundary={boundary}"),
                    ),
                ]),
                body: Some(multipart(&boundary, &parts)),
                timeout: UPLOAD_TIMEOUT,
                then: Then::Answer,
            })
        }
    }
}

fn qr_path(text: &str) -> String {
    format!(
        "/qr?d={}",
        url::form_urlencoded::byte_serialize(text.as_bytes()).collect::<String>()
    )
}

fn qr_image(sent: Result<(u16, Vec<u8>), String>) -> Value {
    match sent {
        Ok((status, bytes)) if (200..300).contains(&status) => {
            json!({ "ok": true, "result": base64::engine::general_purpose::STANDARD.encode(bytes) })
        }
        Ok((status, _)) => refused(&format!("The device refused ({status}).")),
        Err(_) => refused(NO_ANSWER),
    }
}

fn step(path: &str, args: &[Value]) -> Step {
    match path {
        CONNECT => match address(argument(args, 0)) {
            None => Step::Answered(refused(TYPE_AN_ADDRESS)),
            Some(origin) => match resolve(&origin, HOME_VIEW) {
                None => Step::Answered(refused(&unreachable(&origin))),
                Some(url) => get(url, ACCEPT_JSON.1, PROBE_TIMEOUT, Then::Adopt(origin)),
            },
        },
        DISCONNECT => {
            lock().retarget("");
            crate::settingsstore::forgotten(ORIGIN_KEY);
            Step::Answered(json!({ "ok": true }))
        }
        CALL => match (method_of(args), target(argument(args, 1))) {
            (None, _) => Step::Answered(refused(
                "device.call takes POST, PUT or DELETE, a path on the device and an optional JSON body.",
            )),
            (_, Err(refusal)) => Step::Answered(refusal),
            (Some(method), Ok(url)) => {
                let body = args
                    .get(2)
                    .filter(|body| !body.is_null())
                    .map(Value::to_string);
                let headers: &[(&str, &str)] = if body.is_some() {
                    &[ACCEPT_JSON, ("Content-Type", "application/json")]
                } else {
                    &[ACCEPT_JSON]
                };
                Step::Send(Request {
                    method,
                    url,
                    headers: owned(headers),
                    body: body.map(String::into_bytes),
                    timeout: call_timeout(args.get(3)),
                    then: Then::Answer,
                })
            }
        },
        READ => match target(argument(args, 0)) {
            Err(refusal) => Step::Answered(refusal),
            Ok(url) => get(url, ACCEPT_JSON.1, CALL_TIMEOUT, Then::Answer),
        },
        UPLOAD => match (
            method_of(args),
            target(argument(args, 1)),
            args.get(2).and_then(Value::as_array),
        ) {
            (Some(method), Ok(url), Some(parts)) => upload(method, url, parts),
            (_, Err(refusal), _) => Step::Answered(refusal),
            _ => Step::Answered(refused(
                "device.upload takes POST or PUT, a path on the device and the form's parts, each a name with a file or a value.",
            )),
        },
        QR => match target(&qr_path(argument(args, 0))) {
            Err(refusal) => Step::Answered(refusal),
            Ok(url) => get(url, "image/png", CALL_TIMEOUT, Then::Qr),
        },
        REACH => match Url::parse(argument(args, 0))
            .ok()
            .filter(|url| matches!(url.scheme(), "http" | "https"))
        {
            None => Step::Answered(refused("device.reach takes an http or https address.")),
            Some(url) => Step::Send(Request {
                method: "HEAD",
                url: url.into(),
                headers: Vec::new(),
                body: None,
                timeout: REACH_TIMEOUT,
                then: Then::Reach,
            }),
        },
        _ => Step::Answered(refused(&format!("{path} is not a device action"))),
    }
}

fn call_timeout(wait: Option<&Value>) -> Duration {
    wait.and_then(Value::as_u64)
        .map_or(CALL_TIMEOUT, |seconds| {
            Duration::from_secs(seconds.clamp(1, LONGEST_CALL_S))
        })
}

fn finish(then: Then, sent: Result<(u16, Vec<u8>), String>) -> Value {
    match then {
        Then::Answer => answer(sent),
        Then::Adopt(origin) if answers_as_device(&sent) => {
            adopt(&upgraded(&origin).unwrap_or(origin));
            json!({ "ok": true })
        }
        Then::Adopt(origin) => refused(&unreachable(&origin)),
        Then::Qr => qr_image(sent),
        Then::Reach => json!({ "ok": true, "result": sent.is_ok() }),
    }
}

fn arguments(args: &str) -> Vec<Value> {
    serde_json::from_str::<Vec<Value>>(args).unwrap_or_default()
}

pub fn run(path: &str, args: &str) -> Value {
    match step(path, &arguments(args)) {
        Step::Answered(answer) => answer,
        Step::Send(request) => {
            let headers: Vec<(&str, &str)> = request
                .headers
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_str()))
                .collect();
            finish(
                request.then,
                crate::http::send(
                    request.method,
                    &request.url,
                    &headers,
                    request.body.as_deref(),
                    request.timeout,
                ),
            )
        }
    }
}

#[cfg(target_family = "wasm")]
pub async fn run_later(path: &str, args: &str) -> Value {
    match step(path, &arguments(args)) {
        Step::Answered(answer) => answer,
        Step::Send(request) => finish(
            request.then,
            crate::http::send_later(
                request.method,
                &request.url,
                &request.headers,
                request.body.as_deref(),
                request.timeout,
            )
            .await,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    fn serve(answers: Vec<(&'static str, String)>) -> (String, std::sync::mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let (heard, asked) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            answers.into_iter().for_each(|(content_type, body)| {
                let (mut stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let request: Vec<String> = (&mut reader).lines().map_while(Result::ok).take_while(|line| !line.is_empty()).collect();
                let length = request.iter().find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length:").map(|n| n.trim().parse::<usize>().unwrap())).unwrap_or(0);
                let mut sent = vec![0u8; length];
                std::io::Read::read_exact(&mut reader, &mut sent).unwrap();
                heard.send(format!("{}\n{}", request[0], String::from_utf8_lossy(&sent))).unwrap();
                let status = if body.contains("\"error\"") { "422 Unprocessable Entity" } else { "200 OK" };
                write!(stream, "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            });
        });
        (origin, asked)
    }

    #[test]
    fn a_typed_address_becomes_the_origin_the_device_answers_on() {
        assert_eq!(
            address("192.168.1.100").as_deref(),
            Some("http://192.168.1.100")
        );
        assert_eq!(
            address("  falcon.local:8080 ").as_deref(),
            Some("http://falcon.local:8080")
        );
        assert_eq!(
            address("https://Falcon.Local/fly/").as_deref(),
            Some("https://falcon.local")
        );
        assert_eq!(
            address("http://10.0.0.5:80/x").as_deref(),
            Some("http://10.0.0.5"),
            "the default port is not part of the origin"
        );
        assert_eq!(address(""), None);
        assert_eq!(address("ftp://falcon.local"), None);
        assert_eq!(host_of("http://10.0.0.5:8080"), "10.0.0.5:8080");
    }

    #[test]
    fn only_paths_on_the_device_itself_are_reached() {
        assert_eq!(
            resolve("http://10.0.0.5", "/api/streams/front/record").as_deref(),
            Some("http://10.0.0.5/api/streams/front/record")
        );
        assert_eq!(
            resolve("http://10.0.0.5", "//evil.example/x"),
            None,
            "a scheme-relative path would leave the device"
        );
        assert_eq!(resolve("http://10.0.0.5", "http://evil.example/x"), None);
        assert_eq!(resolve("http://10.0.0.5", "api/x"), None);
        assert!(page_name("aircraft") && !page_name("../x") && !page_name(""));
    }

    #[test]
    fn a_view_arrives_as_a_data_line_and_the_heartbeat_carries_none() {
        assert_eq!(
            event_view(r#"data: {"headline":"Ready to fly"}"#),
            Some(json!({ "headline": "Ready to fly" }))
        );
        assert_eq!(
            event_view("data: "),
            None,
            "aircastd's alive event has empty data"
        );
        assert_eq!(event_view("event: alive"), None);
        assert_eq!(event_view("data: [1,2]"), None);
    }

    #[test]
    fn a_refusal_names_the_field_the_device_refused_and_success_carries_its_answer() {
        assert_eq!(
            answer(Err("refused".into())),
            json!({ "ok": false, "reason": NO_ANSWER, "field": "" })
        );
        let refused = answer(Ok((
            422,
            br#"{"error":"APN is too long","field":"apn"}"#.to_vec(),
        )));
        assert_eq!(
            (
                refused["ok"].as_bool(),
                refused["reason"].as_str(),
                refused["field"].as_str()
            ),
            (Some(false), Some("APN is too long"), Some("apn"))
        );
        assert_eq!(
            answer(Ok((500, b"oops".to_vec())))["reason"],
            "The device refused (500)."
        );
        assert_eq!(
            answer(Ok((200, br#"{"ports":[]}"#.to_vec()))),
            json!({ "ok": true, "result": { "ports": [] } })
        );
    }

    #[test]
    fn a_form_carries_each_file_and_field_in_its_own_part() {
        let parts = [
            Part::File {
                name: "hef".into(),
                file_name: "yolo.hef".into(),
                bytes: vec![0, 1],
            },
            Part::Text {
                name: "use".into(),
                value: "true".into(),
            },
        ];
        assert_eq!(
            multipart("b0", &parts),
            b"--b0\r\nContent-Disposition: form-data; name=\"hef\"; filename=\"yolo.hef\"\r\nContent-Type: application/octet-stream\r\n\r\n\x00\x01\r\n--b0\r\nContent-Disposition: form-data; name=\"use\"\r\n\r\ntrue\r\n--b0--\r\n".to_vec()
        );
        assert!(
            read_part(&json!({ "name": "hef", "file": "/nonexistent/yolo.hef" }))
                .is_err_and(|reason| reason.starts_with("Could not read yolo.hef"))
        );
    }

    #[test]
    fn a_call_waits_as_long_as_it_asks_within_a_limit() {
        assert_eq!(call_timeout(None), CALL_TIMEOUT);
        assert_eq!(call_timeout(Some(&json!(60))), Duration::from_secs(60));
        assert_eq!(
            call_timeout(Some(&json!(86_400))),
            Duration::from_secs(LONGEST_CALL_S)
        );
        assert_eq!(call_timeout(Some(&json!("soon"))), CALL_TIMEOUT);
    }

    #[test]
    fn a_page_is_followed_only_while_a_head_watches_it() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let generation = {
            let mut feed = lock();
            feed.retarget("");
            feed.generation
        };
        let watched = |paths: &[&str]| {
            paths
                .iter()
                .map(|path| path.to_string())
                .collect::<BTreeSet<String>>()
        };
        follow_watched(&watched(&[
            "view.device(telemetry)",
            "view.device(../x)",
            "view.battery",
        ]));
        assert!(lock().wanted(generation, "telemetry"));
        assert!(!lock().wanted(generation, "../x") && !lock().wanted(generation, "battery"));
        assert!(
            !lock().pages.contains_key("telemetry"),
            "with no device connected nothing is followed"
        );
        follow_watched(&watched(&[]));
        assert!(
            !lock().wanted(generation, "telemetry"),
            "a page nobody watches stops being followed"
        );
    }

    #[test]
    fn connecting_probes_the_device_then_calls_and_reads_go_to_it_until_disconnected() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        assert_eq!(run(CONNECT, r#"["not a host at all/"]"#)["ok"], false);
        let (origin, asked) = serve(vec![
            (
                "application/json",
                r#"{"headline":"Ready to fly","level":0}"#.into(),
            ),
            ("application/json", r#"{"origin":""}"#.into()),
            ("application/json", r#"{"ok":true}"#.into()),
            (
                "application/json",
                r#"{"error":"Name it first","field":"name"}"#.into(),
            ),
            (
                "application/json",
                r#"{"networks":[{"ssid":"Field"}]}"#.into(),
            ),
            ("application/json", r#"{"switched":true}"#.into()),
        ]);
        assert_eq!(
            run(CONNECT, &json!([origin]).to_string()),
            json!({ "ok": true })
        );
        assert!(
            asked
                .recv()
                .unwrap()
                .starts_with("GET /api/views/aircraft ")
        );
        assert!(asked.recv().unwrap().starts_with("GET /api/tls "));
        assert_eq!(lock().origin(), origin);
        assert_eq!(
            run(CALL, r#"["PUT","/api/streams/front/record",{"on":true}]"#)["ok"],
            true
        );
        assert_eq!(
            asked.recv().unwrap(),
            "PUT /api/streams/front/record HTTP/1.1\n{\"on\":true}"
        );
        let refused = run(CALL, r#"["POST","/api/streams",null]"#);
        assert_eq!(
            (refused["reason"].as_str(), refused["field"].as_str()),
            (Some("Name it first"), Some("name"))
        );
        assert!(asked.recv().unwrap().starts_with("POST /api/streams "));
        assert_eq!(
            run(READ, r#"["/api/network/wifi/scan"]"#)["result"]["networks"][0]["ssid"],
            "Field"
        );
        assert!(
            asked
                .recv()
                .unwrap()
                .starts_with("GET /api/network/wifi/scan ")
        );
        let model = std::env::temp_dir().join("aircast-device-upload.hef");
        std::fs::write(&model, b"weights").unwrap();
        let parts = json!([{ "name": "hef", "file": model.to_string_lossy() }, { "name": "use", "value": "true" }]);
        assert_eq!(
            run(
                UPLOAD,
                &json!(["POST", "/api/detect/models", parts]).to_string()
            )["result"]["switched"],
            true
        );
        let sent = asked.recv().unwrap();
        assert!(
            sent.starts_with("POST /api/detect/models ")
                && sent.contains("filename=\"aircast-device-upload.hef\"")
                && sent.contains("weights")
                && sent.contains("name=\"use\"\r\n\r\ntrue"),
            "{sent}"
        );
        assert_eq!(run(CALL, r#"["PATCH","/api/x"]"#)["ok"], false);
        assert_eq!(run(REACH, r#"["ftp://falcon"]"#)["ok"], false);
        let silent = TcpListener::bind("127.0.0.1:0").unwrap();
        let closed = format!("http://{}", silent.local_addr().unwrap());
        drop(silent);
        assert_eq!(
            run(REACH, &json!([closed]).to_string()),
            json!({ "ok": true, "result": false })
        );
        assert_eq!(run(DISCONNECT, "[]"), json!({ "ok": true }));
        assert_eq!(run(READ, r#"["/api/x"]"#)["reason"], NOT_CONNECTED);
        let shown = page_view("aircraft");
        assert_eq!(
            (
                shown["origin"].as_str(),
                shown["answered"].as_bool(),
                shown["view"].is_null()
            ),
            (Some(""), Some(false), true)
        );
    }

    #[test]
    fn only_an_https_origin_the_device_names_is_taken_as_its_secure_origin() {
        let named = |body: &str| secure_origin(Ok((200, body.as_bytes().to_vec())));
        assert_eq!(
            named(r#"{"origin":"https://192-168-1-50.6f1c2a9e-3b7d-4e21-9a55-0c8d7e6f5a41.d.aircast.one"}"#).as_deref(),
            Some("https://192-168-1-50.6f1c2a9e-3b7d-4e21-9a55-0c8d7e6f5a41.d.aircast.one")
        );
        assert_eq!(
            named(r#"{"origin":"https://10-42-0-1.dev.d.aircast.one:8443/fly/"}"#).as_deref(),
            Some("https://10-42-0-1.dev.d.aircast.one:8443")
        );
        assert_eq!(named(r#"{"origin":""}"#), None);
        assert_eq!(named(r#"{"origin":"http://192.168.1.50"}"#), None);
        assert_eq!(named(r#"{"origin":"https://"}"#), None);
        assert_eq!(named("<html>router</html>"), None);
        assert_eq!(
            secure_origin(Ok((404, br#"{"origin":"https://a.b"}"#.to_vec()))),
            None
        );
        assert_eq!(secure_origin(Err("refused".into())), None);
    }

    #[test]
    fn a_secure_origin_that_does_not_answer_leaves_the_device_on_http() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        run(DISCONNECT, "[]");
        let silent = TcpListener::bind("127.0.0.1:0").unwrap();
        let unreachable_secure = format!("https://{}", silent.local_addr().unwrap());
        drop(silent);
        let (origin, asked) = serve(vec![
            (
                "application/json",
                r#"{"headline":"Ready to fly","level":0}"#.into(),
            ),
            (
                "application/json",
                json!({ "origin": unreachable_secure }).to_string(),
            ),
        ]);
        assert_eq!(
            run(CONNECT, &json!([origin]).to_string()),
            json!({ "ok": true })
        );
        assert!(
            asked
                .recv()
                .unwrap()
                .starts_with("GET /api/views/aircraft ")
        );
        assert!(asked.recv().unwrap().starts_with("GET /api/tls "));
        assert_eq!(lock().origin(), origin);
        run(DISCONNECT, "[]");
    }

    #[test]
    fn a_device_that_does_not_answer_as_aircast_is_not_adopted() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        run(DISCONNECT, "[]");
        let (origin, _asked) = serve(vec![("text/html", "<html>router</html>".into())]);
        assert_eq!(
            run(CONNECT, &json!([origin]).to_string())["reason"],
            unreachable(&origin)
        );
        assert_eq!(lock().origin(), "");
    }

    #[test]
    fn the_device_serving_the_page_is_the_device_whose_pages_are_followed() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let _pairing = crate::device::TEST_PAIRING
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        assert_eq!(
            pair(r#"["http://falcon.local:8080/fly/"]"#),
            json!({ "ok": true })
        );
        let shown = page_view("aircraft");
        assert_eq!(
            (shown["origin"].as_str(), shown["host"].as_str()),
            (Some("http://falcon.local:8080"), Some("falcon.local:8080"))
        );
        assert_eq!(pair(r#"["mailto:x@y"]"#)["ok"], false);
        assert_eq!(
            lock().origin(),
            "http://falcon.local:8080",
            "a page that cannot pair keeps the device it had"
        );
        crate::device::unpair();
        run(DISCONNECT, "[]");
    }

    #[test]
    fn a_followed_page_takes_each_view_the_device_streams() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let generation = {
            let mut feed = lock();
            feed.retarget("http://192.0.2.1");
            feed.pages.insert(
                "telemetry".into(),
                Page {
                    following: true,
                    ..Page::default()
                },
            );
            feed.watched.insert("telemetry".into());
            feed.generation
        };
        take_changed();
        let lines = [
            "event: alive",
            "data: ",
            "",
            r#"data: {"headline":"Waiting for GPS"}"#,
        ]
        .map(String::from);
        assert_eq!(
            take_views(lines.clone().into_iter(), generation, "telemetry"),
            1
        );
        assert_eq!(
            lock().pages["telemetry"].view,
            json!({ "headline": "Waiting for GPS" })
        );
        assert!(take_changed());
        assert_eq!(
            take_views(lines.into_iter(), generation - 1, "telemetry"),
            0,
            "a stream from before the device changed is dropped"
        );
        lock().retarget("");
    }
}
