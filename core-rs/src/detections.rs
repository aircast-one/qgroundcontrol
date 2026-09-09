use serde_json::{Value, json};
use std::io::{BufRead, BufReader};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use crate::read::value_string;
use crate::router::Backend;

pub const DEPS: &[&str] = &["settings.videoSettings.rtspUrl.rawValue"];
pub const STALE_MS: u64 = 1000;
const RETRY: Duration = Duration::from_secs(1);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const SILENCE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub host: String,
    pub camera: String,
}

impl Source {
    pub fn stream_url(&self) -> String {
        format!("http://{}/api/streams/{}/detections/stream", self.host, self.camera)
    }
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub boxes: Vec<Value>,
    pub track: Value,
    pub received_ms: u64,
}

#[derive(Debug, Default)]
pub struct Feed {
    source: Option<Source>,
    generation: u64,
    frame: Option<Frame>,
    error: Option<String>,
    stale_announced: bool,
}

pub static FEED: LazyLock<Mutex<Feed>> = LazyLock::new(|| Mutex::new(Feed::default()));
pub static ON_CHANGE: Mutex<Option<Arc<dyn Fn() + Send + Sync>>> = Mutex::new(None);

pub fn lock() -> MutexGuard<'static, Feed> {
    FEED.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn parse(url: &str) -> Option<Source> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_string();
    let segments: Vec<&str> = parsed.path_segments()?.filter(|s| !s.is_empty()).collect();
    let camera = match segments.as_slice() {
        [] => return None,
        [.., before, last] if matches!(*last, "whep" | "whip") => before,
        [.., last] => last,
    };
    Some(Source { host, camera: camera.to_string() })
}

pub fn frame_from(payload: &Value, received_ms: u64) -> Frame {
    let boxes = payload
        .get("boxes")
        .and_then(Value::as_array)
        .map(|boxes| {
            boxes
                .iter()
                .map(|b| {
                    json!({
                        "x": b.get("x").and_then(Value::as_f64).unwrap_or(0.0),
                        "y": b.get("y").and_then(Value::as_f64).unwrap_or(0.0),
                        "w": b.get("w").and_then(Value::as_f64).unwrap_or(0.0),
                        "h": b.get("h").and_then(Value::as_f64).unwrap_or(0.0),
                        "label": b.get("label").and_then(Value::as_str).unwrap_or(""),
                        "confidence": b.get("conf").and_then(Value::as_f64).unwrap_or(0.0),
                        "target": b.get("target").and_then(Value::as_bool).unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Frame { boxes, track: payload.get("track").cloned().unwrap_or(Value::Null), received_ms }
}

pub fn event_payload(line: &str) -> Option<Value> {
    line.strip_prefix("data:").and_then(|data| serde_json::from_str(data.trim()).ok())
}

impl Feed {
    pub fn retarget(&mut self, source: Option<Source>) -> bool {
        if self.source == source {
            return false;
        }
        self.source = source;
        self.generation += 1;
        self.frame = None;
        self.error = None;
        self.stale_announced = false;
        true
    }

    pub fn receive(&mut self, generation: u64, frame: Frame) -> bool {
        if generation != self.generation {
            return false;
        }
        self.frame = Some(frame);
        self.error = None;
        self.stale_announced = false;
        true
    }

    pub fn went_stale(&mut self, now_ms: u64) -> bool {
        let stale = self.frame.as_ref().is_some_and(|f| now_ms.saturating_sub(f.received_ms) > STALE_MS);
        let first = stale && !self.stale_announced;
        self.stale_announced = self.stale_announced || first;
        first
    }

    pub fn fail(&mut self, generation: u64, error: String) -> bool {
        if generation != self.generation {
            return false;
        }
        self.error = Some(error);
        true
    }

    pub fn snapshot(&self, now_ms: u64) -> Value {
        let age_ms = self.frame.as_ref().map(|f| now_ms.saturating_sub(f.received_ms));
        let stale = age_ms.is_none_or(|age| age > STALE_MS);
        json!({
            "kind": "object",
            "class": "Detections",
            "available": self.source.is_some(),
            "host": self.source.as_ref().map(|s| s.host.clone()),
            "camera": self.source.as_ref().map(|s| s.camera.clone()),
            "stale": stale,
            "ageMs": age_ms,
            "boxes": if stale { Vec::new() } else { self.frame.as_ref().map(|f| f.boxes.clone()).unwrap_or_default() },
            "track": if stale { Value::Null } else { self.frame.as_ref().map(|f| f.track.clone()).unwrap_or(Value::Null) },
            "error": self.error,
        })
    }
}

fn changed() {
    let hook = ON_CHANGE.lock().unwrap_or_else(PoisonError::into_inner).clone();
    if let Some(hook) = hook {
        hook();
    }
}

fn follow(source: Source, generation: u64) {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_connect(Some(CONNECT_TIMEOUT)).timeout_recv_body(Some(SILENCE_TIMEOUT)).build().into();
    while lock().generation == generation {
        let outcome: Result<usize, String> = agent.get(source.stream_url()).call().map_err(|e| e.to_string()).map(|response| {
            BufReader::new(response.into_body().into_reader())
                .lines()
                .map_while(Result::ok)
                .take_while(|_| lock().generation == generation)
                .filter_map(|line| event_payload(&line))
                .map(|payload| {
                    let frame = frame_from(&payload, crate::hub::now_ms());
                    if lock().receive(generation, frame) {
                        changed();
                    }
                })
                .count()
        });
        match outcome {
            Ok(frames) if frames > 0 => {}
            Ok(_) => std::thread::sleep(RETRY),
            Err(error) => {
                if lock().fail(generation, error) {
                    changed();
                }
                std::thread::sleep(RETRY);
            }
        }
    }
}

pub fn detections_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let source = parse(&value_string(&backend.get(DEPS[0])));
    let mut feed = lock();
    if feed.retarget(source.clone())
        && let Some(source) = source
    {
        let generation = feed.generation;
        std::thread::Builder::new().name("qgc-core-detections".to_string()).spawn(move || follow(source, generation)).expect("detections thread");
    }
    feed.snapshot(crate::hub::now_ms())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rtsp_url_names_the_host_and_the_camera_as_the_qml_overlay_did() {
        assert_eq!(parse("rtsp://10.0.0.5:8554/front"), Some(Source { host: "10.0.0.5".into(), camera: "front".into() }));
        assert_eq!(parse("rtsp://user:pw@drone.local:8554/cams/rear/whep?x=1"), Some(Source { host: "drone.local".into(), camera: "rear".into() }));
        assert_eq!(parse("rtsp://drone.local/whep"), Some(Source { host: "drone.local".into(), camera: "whep".into() }));
        assert_eq!(parse("rtsp://cam:pw@cam/whep"), Some(Source { host: "cam".into(), camera: "whep".into() }), "the host text inside the credentials does not confuse the path");
        assert_eq!(parse("rtsp://[::1]:8554/front"), Some(Source { host: "[::1]".into(), camera: "front".into() }));
        assert_eq!(parse("rtsp://drone.local:8554/"), None);
        assert_eq!(parse("not a url"), None);
        assert!(parse("").is_none());
        assert_eq!(parse("rtsp://10.0.0.5:8554/front").unwrap().stream_url(), "http://10.0.0.5/api/streams/front/detections/stream");
    }

    #[test]
    fn frames_arrive_as_sse_lines_and_go_stale_after_a_second() {
        let mut feed = Feed::default();
        assert_eq!(feed.snapshot(0)["available"], false);
        assert!(feed.retarget(parse("rtsp://h:8554/cam")));
        assert!(!feed.retarget(parse("rtsp://h:8554/cam")), "the same source does not restart the stream");
        let generation = feed.generation;
        let payload = event_payload(r#"data: {"boxes":[{"label":"car","conf":0.91,"x":0.1,"y":0.2,"w":0.3,"h":0.4,"target":true}],"ageMs":0}"#).unwrap();
        assert!(event_payload(": keepalive").is_none());
        assert!(feed.receive(generation, frame_from(&payload, 5_000)));
        let fresh = feed.snapshot(5_500);
        assert_eq!(fresh["stale"], false);
        assert_eq!(fresh["ageMs"], 500);
        assert_eq!(fresh["boxes"][0], json!({ "x": 0.1, "y": 0.2, "w": 0.3, "h": 0.4, "label": "car", "confidence": 0.91, "target": true }));
        assert!(!feed.went_stale(5_500));
        let old = feed.snapshot(6_001);
        assert_eq!((old["stale"].clone(), old["boxes"].as_array().unwrap().len()), (json!(true), 0));
        assert!(feed.went_stale(6_001), "the first tick past a second announces the blank");
        assert!(!feed.went_stale(6_100), "and only the first");
        assert!(!feed.receive(generation - 1, frame_from(&payload, 7_000)), "a frame from a superseded stream is dropped");
        assert!(feed.fail(generation, "connection refused".into()));
        assert_eq!(feed.snapshot(7_000)["error"], "connection refused");
        assert!(feed.retarget(None));
        assert_eq!(feed.snapshot(7_000), json!({ "kind": "object", "class": "Detections", "available": false, "host": null, "camera": null, "stale": true, "ageMs": null, "boxes": [], "track": null, "error": null }));
    }
}
