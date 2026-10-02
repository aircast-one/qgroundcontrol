use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};

use crate::linkconfig::{Kind, LinkConfig};
use crate::read::{flag, object};
use crate::replay::Replay;
use crate::router::Backend;
use crate::transport::LinkId;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", SHOW_BAR];
pub const START: &str = "logReplay.start";
pub const TOGGLE_PLAY: &str = "logReplay.togglePlay";
pub const SPEED: &str = "logReplay.speed";
pub const SEEK: &str = "logReplay.seek";
pub const CLOSE_REPLAY: &str = "logReplay.close";
const SHOW_BAR: &str = "settings.flyViewSettings.showLogReplayStatusBar";
const SPEEDS: [(&str, f64); 7] = [("0.1", 0.1), ("0.25", 0.25), ("0.5", 0.5), ("1x", 1.0), ("2x", 2.0), ("5x", 5.0), ("10x", 10.0)];
const DEFAULT_SPEED_INDEX: usize = 3;
const IDLE_POLL: Duration = Duration::from_millis(20);
const CLOSE_FIRST: &str = "You must close all connections prior to replaying a log.";

pub fn hms(seconds: u64) -> String {
    let (hours, minutes, secs) = (seconds / 3600, (seconds / 60) % 60, seconds % 60);
    match hours {
        0 => format!("{minutes:02}m:{secs:02}s"),
        h => format!("{h:02}h:{minutes:02}m:{secs:02}s"),
    }
}

pub fn short_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

struct Session {
    generation: u64,
    link: Option<LinkId>,
    replay: Replay,
    speed_index: usize,
    playhead_s: u64,
    percent: f64,
}

#[derive(Default)]
struct Controller {
    generation: u64,
    session: Option<Session>,
    error: String,
}

static CONTROLLER: LazyLock<Mutex<Controller>> = LazyLock::new(|| Mutex::new(Controller::default()));

pub fn playing() -> bool {
    lock().session.as_ref().is_some_and(|s| s.replay.is_playing())
}

fn lock() -> MutexGuard<'static, Controller> {
    CONTROLLER.lock().unwrap_or_else(PoisonError::into_inner)
}

pub struct ReplayLink {
    generation: u64,
}

impl ReplayLink {
    pub fn open(file: &str, deliver: impl Fn(&[u8]) + Send + 'static) -> Result<ReplayLink, String> {
        let bytes = std::fs::read(file).map_err(|e| format!("Unable to open log file: '{file}', error: {e}"))?;
        let replay = Replay::from_tlog(&bytes, crate::hub::now_us());
        if replay.duration_us() == 0 {
            return Err(format!("The log file '{file}' is corrupt or empty."));
        }
        let generation = {
            let mut controller = lock();
            controller.generation += 1;
            let mut session = Session { generation: controller.generation, link: None, replay, speed_index: DEFAULT_SPEED_INDEX, playhead_s: 0, percent: 0.0 };
            session.replay.play(crate::hub::now_ms());
            controller.session = Some(session);
            controller.error.clear();
            controller.generation
        };
        std::thread::Builder::new().name("groundstation-log-replay".to_string()).spawn(move || play(generation, deliver)).map_err(|e| e.to_string())?;
        Ok(ReplayLink { generation })
    }

    pub fn close(&self) {
        let mut controller = lock();
        if controller.session.as_ref().is_some_and(|s| s.generation == self.generation) {
            controller.session = None;
        }
    }
}

fn play(generation: u64, deliver: impl Fn(&[u8])) {
    loop {
        let batch = {
            let mut controller = lock();
            let Some(session) = controller.session.as_mut().filter(|s| s.generation == generation) else { return };
            match session.replay.is_playing() {
                false => None,
                true => {
                    let batch = session.replay.tick(crate::hub::now_ms());
                    session.playhead_s = batch.log_time_s;
                    session.percent = batch.percent;
                    Some(batch)
                }
            }
        };
        match batch {
            None => std::thread::sleep(IDLE_POLL),
            Some(batch) => {
                batch.frames.iter().for_each(|frame| deliver(frame));
                std::thread::sleep(Duration::from_millis(batch.wait_ms.max(1)));
            }
        }
    }
}

pub fn bind_link(link: LinkId) {
    if let Some(session) = lock().session.as_mut() {
        session.link = Some(link);
    }
}

fn args(text: &str) -> Vec<Value> {
    serde_json::from_str::<Vec<Value>>(text).unwrap_or_default()
}

fn start(backend: &dyn Backend, path: &str) -> Value {
    if flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable") {
        return json!({ "ok": false, "reason": CLOSE_FIRST });
    }
    let config = LinkConfig { name: short_name(path), auto_connect: false, high_latency: false, kind: Kind::LogReplay { file: path.to_string() } };
    match crate::linkhost::open(&crate::linkhost::TRANSPORTS, config, &[]) {
        Ok(link) => {
            bind_link(link);
            json!({ "ok": true })
        }
        Err(failure) => {
            let reason = format!("Link: {}, {}.", short_name(path), failure.reason);
            lock().error = reason.clone();
            json!({ "ok": false, "reason": reason })
        }
    }
}

fn close(backend: &dyn Backend) -> Value {
    let link = lock().session.as_ref().and_then(|s| s.link);
    if let Some(link) = link {
        crate::linkhost::close(&crate::linkhost::TRANSPORTS, link, "Log replay closed");
        crate::hub::lock().link_closed(link);
    }
    lock().session = None;
    crate::factwrite::write(backend, SHOW_BAR, &json!({ "value": false }).to_string())
}

pub fn run(backend: &dyn Backend, path: &str, text: &str) -> Value {
    let given = args(text);
    let number = given.first().and_then(Value::as_f64);
    let now_ms = crate::hub::now_ms();
    let with_session = |change: &dyn Fn(&mut Session)| {
        let mut controller = lock();
        match controller.session.as_mut() {
            Some(session) => {
                change(session);
                json!({ "ok": true })
            }
            None => json!({ "ok": false, "reason": "No telemetry log is loaded." }),
        }
    };
    match path {
        START => match given.first().and_then(Value::as_str) {
            Some(file) => start(backend, file),
            None => json!({ "ok": false, "reason": "logReplay.start takes the log file path" }),
        },
        TOGGLE_PLAY => with_session(&|s| match s.replay.is_playing() {
            true => s.replay.pause(),
            false => s.replay.play(now_ms),
        }),
        SPEED => match number.map(|i| i as usize).filter(|i| *i < SPEEDS.len()) {
            Some(index) => with_session(&|s| {
                s.speed_index = index;
                s.replay.set_speed(SPEEDS[index].1, now_ms);
            }),
            None => json!({ "ok": false, "reason": "logReplay.speed takes one of the speed options" }),
        },
        SEEK => match number {
            Some(percent) => with_session(&|s| {
                s.percent = s.replay.seek(percent);
                s.playhead_s = (s.percent / 100.0 * s.replay.duration_s() as f64) as u64;
            }),
            None => json!({ "ok": false, "reason": "logReplay.seek takes a percentage" }),
        },
        CLOSE_REPLAY => close(backend),
        _ => json!({ "ok": false, "reason": format!("{path} is not a log replay action") }),
    }
}

pub fn owns(path: &str) -> bool {
    [START, TOGGLE_PLAY, SPEED, SEEK, CLOSE_REPLAY].contains(&path)
}

pub fn log_replay_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let shown = flag(&object(&backend.get(SHOW_BAR)), "value");
    let vehicle = flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
    let controller = lock();
    let session = controller.session.as_ref();
    json!({
        "kind": "object",
        "class": "LogReplay",
        "available": crate::vehiclefacade::switched_on(),
        "shown": shown,
        "loaded": session.is_some(),
        "playing": session.is_some_and(|s| s.replay.is_playing()),
        "percent": session.map_or(0.0, |s| s.percent),
        "playheadTime": session.map(|s| hms(s.playhead_s)).unwrap_or_default(),
        "totalTime": session.map(|s| hms(s.replay.duration_s())).unwrap_or_default(),
        "speedIndex": session.map_or(DEFAULT_SPEED_INDEX, |s| s.speed_index),
        "speeds": SPEEDS.iter().map(|(label, _)| *label).collect::<Vec<_>>(),
        "canLoad": !vehicle,
        "loadRefusal": CLOSE_FIRST,
        "error": controller.error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_read_like_qgc_seconds_to_hms() {
        assert_eq!(hms(0), "00m:00s");
        assert_eq!(hms(75), "01m:15s");
        assert_eq!(hms(3725), "01h:02m:05s");
        assert_eq!(short_name("/data/logs/2026-10-01 flight.tlog"), "2026-10-01 flight.tlog");
    }

    #[test]
    fn a_replay_link_streams_the_log_until_it_is_closed() {
        let (sent, received) = std::sync::mpsc::channel::<usize>();
        let link = ReplayLink::open(&crate::samplelog::path(), move |bytes| {
            let _ = sent.send(bytes.len());
        })
        .unwrap();
        assert!(received.recv_timeout(Duration::from_secs(5)).is_ok_and(|n| n > 0), "frames flow as soon as the log opens, as QGC plays on connect");
        let view_state = lock().session.as_ref().map(|s| (s.replay.is_playing(), s.speed_index));
        assert_eq!(view_state, Some((true, DEFAULT_SPEED_INDEX)));
        link.close();
        assert!(lock().session.is_none());
        assert!(ReplayLink::open("/nonexistent/flight.tlog", |_| {}).is_err());
    }
}
