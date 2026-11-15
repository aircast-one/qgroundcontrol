use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tauri::async_runtime::{JoinHandle, spawn_blocking};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::watch;
use url::Url;

use crate::annexb::{self, Codec, Unit};
use crate::ingest::{self, without_credentials};
use crate::record::{Recording, Written, inside_folder};

const CONFIG: u8 = 0;
const FRAME: u8 = 1;
const ENDED: u8 = 2;
const RECORDING_STOPPED: u8 = 3;

const CLOSED: &str = "closed";
const NOT_PLAYING: &str = "The stream is no longer playing.";
const PAGE_LEFT: &str = "The page reloaded before the stream opened.";
const RESTARTED: &str = "restarted";
const DROPPED: &str = "dropped";

#[derive(Default)]
struct Slot {
    recording: Option<Recording>,
    closed: bool,
}

type Recorder = Arc<Mutex<Slot>>;
type Sink = Channel<InvokeResponseBody>;
type StopReason = Arc<OnceLock<&'static str>>;
type Finished = watch::Receiver<()>;

struct Stream {
    task: JoinHandle<()>,
    sessions: Vec<Finished>,
    stopping: StopReason,
    url: Url,
    sink: Sink,
    recorder: Recorder,
}

#[derive(Default)]
pub struct Streams {
    next: AtomicU32,
    page: AtomicU64,
    running: Mutex<HashMap<u32, Stream>>,
    closing: Mutex<Vec<Finished>>,
}

#[derive(Default)]
struct Tally {
    frames: u64,
    keyframes: u64,
    packets: u64,
    lost: u64,
    bytes: u64,
}

struct Live {
    id: u32,
    source: String,
    sink: Sink,
    recorder: Recorder,
    stopping: StopReason,
    announced: Option<String>,
    keyed: bool,
    started: Instant,
    tally: Tally,
    outcome: Option<String>,
}

fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |since| since.as_secs_f64() * 1000.0)
}

fn config(codec: &str) -> Vec<u8> {
    [&[CONFIG][..], codec.as_bytes()].concat()
}

fn frame(key: bool, sent_ms: f64, unit: &Unit) -> Vec<u8> {
    [
        &[FRAME, u8::from(key)][..],
        &sent_ms.to_be_bytes(),
        &unit.packets.to_be_bytes(),
        &unit.lost.to_be_bytes(),
        &unit.data,
    ]
    .concat()
}

fn ended(reason: &str) -> Vec<u8> {
    [&[ENDED][..], reason.as_bytes()].concat()
}

fn recording_stopped(reason: &str) -> Vec<u8> {
    [&[RECORDING_STOPPED][..], reason.as_bytes()].concat()
}

fn send(sink: &Sink, message: Vec<u8>) -> Result<(), String> {
    sink.send(InvokeResponseBody::Raw(message))
        .map_err(|error| format!("The page stopped listening: {error}"))
}

fn locked<T>(held: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    held.lock().unwrap_or_else(PoisonError::into_inner)
}

fn log_recording(id: u32, event: &str, written: &Written, reason: &str) {
    log::info!(
        target: "desktop.video",
        "event={event} stream={id} file=\"{}\" frames={} bytes={} seconds={:.1} reason=\"{reason}\"",
        written.file.display(),
        written.frames,
        written.bytes,
        written.seconds,
    );
}

impl Tally {
    fn counted(&self, unit: &Unit, key: bool) -> Self {
        Self {
            frames: self.frames + 1,
            keyframes: self.keyframes + u64::from(key),
            packets: self.packets + u64::from(unit.packets),
            lost: self.lost + u64::from(unit.lost),
            bytes: self.bytes + unit.data.len() as u64,
        }
    }
}

impl Live {
    fn new(id: u32, url: &Url, sink: Sink, recorder: Recorder, stopping: StopReason) -> Self {
        Self {
            id,
            source: without_credentials(url).0.to_string(),
            sink,
            recorder,
            stopping,
            announced: None,
            keyed: false,
            started: Instant::now(),
            tally: Tally::default(),
            outcome: None,
        }
    }

    fn deliver(&mut self, unit: &Unit) -> Result<(), String> {
        let nals = annexb::nals(&unit.data);
        if let Some(found) = annexb::codec_of(unit.codec, &nals)
            .filter(|found| self.announced.as_ref() != Some(found))
        {
            send(&self.sink, config(&found))?;
            self.announced = Some(found);
        }
        let key = annexb::keyed(unit.codec, &nals);
        self.keyed |= key;
        self.tally = self.tally.counted(unit, key);
        send(&self.sink, frame(key, now_ms(), unit))?;
        tokio::task::block_in_place(|| self.record(unit.codec, &nals, key, unit.timestamp))
    }

    fn record(
        &self,
        codec: Codec,
        nals: &[&[u8]],
        key: bool,
        timestamp: u64,
    ) -> Result<(), String> {
        let mut slot = locked(&self.recorder);
        let Some(failed) = slot
            .recording
            .as_mut()
            .filter(|_| self.keyed)
            .and_then(|written| written.write(codec, nals, key, timestamp).err())
        else {
            return Ok(());
        };
        let stopped = slot.recording.take();
        drop(slot);
        if let Some(stopped) = stopped {
            finished(self.id, stopped, "recording_failed", &failed.to_string());
        }
        send(&self.sink, recording_stopped(&failed.to_string()))
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        let reason = self
            .outcome
            .as_deref()
            .or(self.stopping.get().copied())
            .unwrap_or(DROPPED);
        let level = match reason {
            CLOSED | RESTARTED => log::Level::Info,
            _ => log::Level::Warn,
        };
        log::log!(
            target: "desktop.video",
            level,
            "event=stream_ended stream={} source={} codec={} frames={} keyframes={} packets={} lost={} bytes={} seconds={:.1} recording={} reason=\"{reason}\"",
            self.id,
            self.source,
            self.announced.as_deref().unwrap_or("none"),
            self.tally.frames,
            self.tally.keyframes,
            self.tally.packets,
            self.tally.lost,
            self.tally.bytes,
            self.started.elapsed().as_secs_f64(),
            locked(&self.recorder).recording.is_some(),
        );
    }
}

async fn settled(mut finished: Finished) {
    while finished.changed().await.is_ok() {}
}

fn still_running(sessions: Vec<Finished>) -> Vec<Finished> {
    sessions
        .into_iter()
        .filter(|finished| finished.has_changed().is_ok())
        .collect()
}

fn play(
    id: u32,
    stream: (&Url, &Sink, &Recorder, &StopReason),
    previous: Vec<Finished>,
) -> (JoinHandle<()>, Finished) {
    let (url, sink, recorder, stopping) = (
        stream.0.clone(),
        stream.1.clone(),
        stream.2.clone(),
        stream.3.clone(),
    );
    let (finishing, finished) = watch::channel(());
    let task = tauri::async_runtime::spawn(async move {
        let _finishing = finishing;
        futures::future::join_all(previous.into_iter().map(settled)).await;
        let mut live = Live::new(id, &url, sink, recorder, stopping);
        let played = ingest::play(&url, &mut |unit: &Unit| live.deliver(unit)).await;
        let reason = played.err().unwrap_or_default();
        live.outcome = Some(reason.clone());
        if let Err(error) = send(&live.sink, ended(&reason)) {
            log::debug!(target: "desktop.video", "event=end_not_delivered stream={id} reason=\"{error}\"");
        }
    });
    (task, finished)
}

fn started(id: u32, file: &Path, size: (u32, u32)) -> Result<Recording, String> {
    let folder =
        groundstation::settingsstore::video_save_path().ok_or("The app has no video folder.")?;
    let target = inside_folder(Path::new(&folder), file)?;
    let recording = Recording::create(&target, size).map_err(|error| error.to_string())?;
    log::info!(target: "desktop.video", "event=recording_started stream={id} file=\"{}\" width={} height={}", target.display(), size.0, size.1);
    Ok(recording)
}

fn finished(id: u32, recording: Recording, event: &str, reason: &str) {
    match recording.finish() {
        Ok(written) => log_recording(id, event, &written, reason),
        Err(error) => {
            log::warn!(target: "desktop.video", "event=recording_not_finished stream={id} reason=\"{error}\"");
        }
    }
}

fn close_recording(id: u32, recorder: &Recorder) {
    let mut slot = locked(recorder);
    slot.closed = true;
    let stopped = slot.recording.take();
    drop(slot);
    if let Some(stopped) = stopped {
        finished(id, stopped, "recording_stopped", "");
    }
}

impl Streams {
    async fn open(&self, url: Url, sink: Sink) -> Result<u32, String> {
        let page = self.page.load(Ordering::Relaxed);
        let closing = locked(&self.closing).clone();
        futures::future::join_all(closing.into_iter().map(settled)).await;
        let mut running = locked(&self.running);
        if self.page.load(Ordering::Relaxed) != page {
            return Err(PAGE_LEFT.to_string());
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (recorder, stopping) = (Recorder::default(), StopReason::default());
        let (task, finished) = play(id, (&url, &sink, &recorder, &stopping), Vec::new());
        running.insert(
            id,
            Stream {
                task,
                sessions: vec![finished],
                stopping,
                url,
                sink,
                recorder,
            },
        );
        Ok(id)
    }

    fn close(&self, id: u32) -> Result<(), String> {
        let Some(stream) = locked(&self.running).remove(&id) else {
            return Ok(());
        };
        stream.stopping.get_or_init(|| CLOSED);
        stream.task.abort();
        let mut closing = locked(&self.closing);
        let pending = still_running([std::mem::take(&mut *closing), stream.sessions].concat());
        *closing = pending;
        drop(closing);
        close_recording(id, &stream.recorder);
        Ok(())
    }

    fn restart(&self, id: u32) -> Result<(), String> {
        let mut running = locked(&self.running);
        let stream = running.remove(&id).ok_or(NOT_PLAYING)?;
        stream.stopping.get_or_init(|| RESTARTED);
        stream.task.abort();
        let stopping = StopReason::default();
        let earlier = still_running(stream.sessions);
        let (task, finished) = play(
            id,
            (&stream.url, &stream.sink, &stream.recorder, &stopping),
            earlier.clone(),
        );
        running.insert(
            id,
            Stream {
                task,
                sessions: [earlier, vec![finished]].concat(),
                stopping,
                url: stream.url,
                sink: stream.sink,
                recorder: stream.recorder,
            },
        );
        Ok(())
    }

    pub fn page_left(&self) -> Vec<u32> {
        let running = locked(&self.running);
        self.page.fetch_add(1, Ordering::Relaxed);
        running.keys().copied().collect()
    }

    pub fn close_each(&self, ids: Vec<u32>) {
        ids.into_iter()
            .filter_map(|id| self.close(id).err().map(|error| (id, error)))
            .for_each(|(id, error)| {
                log::warn!(target: "desktop.video", "event=close_failed stream={id} reason=\"{error}\"");
            });
    }

    fn record(&self, id: u32, file: Option<&Path>, size: (u32, u32)) -> Result<(), String> {
        let recorder = locked(&self.running)
            .get(&id)
            .map(|stream| stream.recorder.clone())
            .ok_or(NOT_PLAYING)?;
        let mut slot = locked(&recorder);
        if slot.closed {
            return Err(NOT_PLAYING.to_string());
        }
        let previous = slot.recording.take();
        let (recording, refused) = match file.map(|file| started(id, file, size)).transpose() {
            Ok(recording) => (recording, None),
            Err(reason) => (None, Some(reason)),
        };
        slot.recording = recording;
        drop(slot);
        if let Some(previous) = previous {
            finished(id, previous, "recording_stopped", "");
        }
        refused.map_or(Ok(()), Err)
    }
}

#[tauri::command]
pub async fn video_open(
    url: String,
    frames: Sink,
    streams: tauri::State<'_, Streams>,
) -> Result<u32, String> {
    let url = Url::parse(&url).map_err(|error| error.to_string())?;
    streams.open(url, frames).await
}

async fn off_the_window<R: Runtime, T: Send + 'static>(
    app: AppHandle<R>,
    work: impl FnOnce(&Streams) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    spawn_blocking(move || work(&app.state::<Streams>()))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn video_close<R: Runtime>(app: AppHandle<R>, id: u32) -> Result<(), String> {
    off_the_window(app, move |streams| streams.close(id)).await
}

#[tauri::command]
pub async fn video_restart<R: Runtime>(app: AppHandle<R>, id: u32) -> Result<(), String> {
    off_the_window(app, move |streams| streams.restart(id)).await
}

#[tauri::command]
pub async fn video_record<R: Runtime>(
    app: AppHandle<R>,
    id: u32,
    file: Option<String>,
    width: u32,
    height: u32,
) -> Result<(), String> {
    off_the_window(app, move |streams| {
        streams.record(id, file.as_deref().map(Path::new), (width, height))
    })
    .await
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;

    const ATTEMPTS: usize = 400;
    const WAIT: Duration = Duration::from_millis(25);

    fn listening_sink() -> (Sink, mpsc::Receiver<Vec<u8>>) {
        let (heard, hearing) = mpsc::channel();
        let sink = Channel::new(move |body| {
            if let InvokeResponseBody::Raw(bytes) = body {
                let _ = heard.send(bytes);
            }
            Ok(())
        });
        (sink, hearing)
    }

    fn rtp(sequence: u16, marker: bool, payload: &[u8]) -> Vec<u8> {
        [
            &[0x80, if marker { 0xe0 } else { 0x60 }][..],
            &sequence.to_be_bytes(),
            &90u32.to_be_bytes(),
            &[0, 0, 0, 1],
            payload,
        ]
        .concat()
    }

    fn announced_after_sending(port: u16, hearing: &mpsc::Receiver<Vec<u8>>) -> Result<(), String> {
        sent_until(port, hearing, CONFIG, || true)
    }

    fn described(message: &[u8]) -> String {
        let text = String::from_utf8_lossy(message.get(1..).unwrap_or_default());
        match message.first() {
            Some(&CONFIG) => format!("config {text}"),
            Some(&FRAME) => "frame".to_string(),
            Some(&ENDED) => format!("ended: {text}"),
            Some(&RECORDING_STOPPED) => format!("recording stopped: {text}"),
            _ => "nothing".to_string(),
        }
    }

    fn sent_until(
        port: u16,
        hearing: &mpsc::Receiver<Vec<u8>>,
        kind: u8,
        done: impl Fn() -> bool,
    ) -> Result<(), String> {
        let camera = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let outcome = (0..ATTEMPTS).try_fold(Vec::new(), |heard: Vec<String>, attempt| {
            let sequence = u16::try_from(attempt * 3).unwrap();
            [
                rtp(sequence, false, &[0x67, 0x42, 0xc0, 0x1f, 0xda]),
                rtp(sequence + 1, false, &[0x68, 0xce, 0x3c, 0x80]),
                rtp(sequence + 2, true, &[0x65, 0x88, 0x84]),
            ]
            .iter()
            .for_each(|packet| {
                camera.send_to(packet, ("127.0.0.1", port)).unwrap();
            });
            match hearing.recv_timeout(WAIT) {
                Ok(message) if message[0] == kind && done() => std::ops::ControlFlow::Break(()),
                Ok(message) if heard.len() < 8 => {
                    std::ops::ControlFlow::Continue([heard, vec![described(&message)]].concat())
                }
                _ => std::ops::ControlFlow::Continue(heard),
            }
        });
        match outcome {
            std::ops::ControlFlow::Break(()) => Ok(()),
            std::ops::ControlFlow::Continue(heard) => Err(format!(
                "after {ATTEMPTS} sends the stream answered with {heard:?}"
            )),
        }
    }

    fn free_port() -> u16 {
        std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[test]
    fn messages_carry_the_layout_the_web_player_reads() {
        assert_eq!(config("avc1.42c01f"), [&[0][..], b"avc1.42c01f"].concat());
        assert_eq!(
            frame(
                true,
                1.5,
                &Unit {
                    codec: Codec::H264,
                    data: vec![9],
                    packets: 3,
                    lost: 2,
                    timestamp: 0
                }
            ),
            [1, 1, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0, 0, 3, 0, 2, 9]
        );
        assert_eq!(ended("gone"), [2, b'g', b'o', b'n', b'e']);
        assert_eq!(recording_stopped("full"), [3, b'f', b'u', b'l', b'l']);
    }

    #[test]
    fn a_restart_reconnects_the_same_stream_and_keeps_its_recording() {
        tauri::async_runtime::block_on(async {
            let streams = Streams::default();
            let port = free_port();
            let (sink, hearing) = listening_sink();
            let id = streams
                .open(
                    Url::parse(&format!("udp://127.0.0.1:{port}")).unwrap(),
                    sink,
                )
                .await
                .unwrap();
            announced_after_sending(port, &hearing).unwrap();
            let recorder = locked(&streams.running)[&id].recorder.clone();
            let file =
                std::env::temp_dir().join(format!("aircast-restart-{}.mp4", std::process::id()));
            locked(&recorder).recording = Some(Recording::create(&file, (1280, 720)).unwrap());
            let frames = || {
                locked(&recorder)
                    .recording
                    .as_ref()
                    .map_or(0, |it| it.written().frames)
            };
            sent_until(port, &hearing, FRAME, || frames() > 0).unwrap();
            let before = frames();
            streams.restart(id).unwrap();
            hearing.try_iter().for_each(drop);
            announced_after_sending(port, &hearing).unwrap_or_else(|heard| {
                panic!("a fresh session announces the codec again, so the camera was reconnected: {heard}")
            });
            sent_until(port, &hearing, FRAME, || frames() > before).unwrap_or_else(|heard| {
                panic!("frames from the reconnected camera go on into the same recording: {heard}")
            });
            streams.close(id).unwrap();
            assert!(std::fs::metadata(&file).unwrap().len() > 0);
            std::fs::remove_file(&file).unwrap();
        });
    }

    #[test]
    fn a_stream_still_opening_when_the_page_reloads_is_not_left_playing() {
        tauri::async_runtime::block_on(async {
            let streams = Streams::default();
            let (closed, closing) = watch::channel(());
            locked(&streams.closing).push(closing);
            let url = Url::parse(&format!("udp://127.0.0.1:{}", free_port())).unwrap();
            let opening = streams.open(url, listening_sink().0);
            futures::pin_mut!(opening);
            assert!(futures::poll!(&mut opening).is_pending());
            assert!(streams.page_left().is_empty());
            drop(closed);
            assert_eq!(opening.await, Err(PAGE_LEFT.to_string()));
            assert!(locked(&streams.running).is_empty());
        });
    }

    #[test]
    fn a_close_that_lands_during_a_restart_leaves_nothing_playing() {
        tauri::async_runtime::block_on(async {
            let streams = Streams::default();
            let port = free_port();
            let url = Url::parse(&format!("udp://127.0.0.1:{port}")).unwrap();
            let id = streams.open(url.clone(), listening_sink().0).await.unwrap();
            let recorder = locked(&streams.running)[&id].recorder.clone();
            streams.restart(id).unwrap();
            streams.close(id).unwrap();
            assert!(locked(&streams.running).is_empty());
            assert!(
                locked(&recorder).closed,
                "a recording that starts while the stream closes finds the slot closed instead of being left running"
            );
            assert!(streams.restart(id).is_err());
            let (sink, hearing) = listening_sink();
            let reopened = streams.open(url, sink).await.unwrap();
            announced_after_sending(port, &hearing).unwrap_or_else(|heard| {
                panic!("the port is free for the next stream, so nothing from the restart kept it: {heard}")
            });
            streams.close(reopened).unwrap();
        });
    }
}
