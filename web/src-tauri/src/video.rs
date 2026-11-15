use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use futures::StreamExt;
use percent_encoding::percent_decode_str;
use retina::client::{
    Credentials, PlayOptions, ReceiverReportPolicy, Session, SessionOptions, SetupOptions,
    Transport,
};
use retina::codec::{CodecItem, FrameFormat};
use tauri::async_runtime::JoinHandle;
use tauri::ipc::{Channel, InvokeResponseBody};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpStream, UdpSocket};
use url::Url;

use crate::annexb::{self, Codec, Unit};
use crate::mpegts::Demuxer;
use crate::record::{Recording, inside_folder};
use crate::rtp::Depacketizer;

const DATAGRAM: usize = 65_536;
const ANY_HOST: &str = "0.0.0.0";

const CONFIG: u8 = 0;
const FRAME: u8 = 1;
const ENDED: u8 = 2;
const RECORDING_STOPPED: u8 = 3;

const CLOSED: &str = "closed";

type Recorder = Arc<Mutex<Option<Recording>>>;
type Sink = Channel<InvokeResponseBody>;

struct Stream {
    task: JoinHandle<()>,
    url: Url,
    sink: Sink,
    recorder: Recorder,
}

#[derive(Default)]
pub struct Streams {
    next: AtomicU32,
    running: Mutex<HashMap<u32, Stream>>,
    closing: Mutex<Vec<JoinHandle<()>>>,
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

fn without_credentials(url: &Url) -> (Url, Option<Credentials>) {
    let decoded = |text: &str| percent_decode_str(text).decode_utf8_lossy().into_owned();
    let credentials = (!url.username().is_empty()).then(|| Credentials {
        username: decoded(url.username()),
        password: decoded(url.password().unwrap_or_default()),
    });
    let mut bare = url.clone();
    let _ = bare.set_username("");
    let _ = bare.set_password(None);
    (bare, credentials)
}

fn send(sink: &Sink, message: Vec<u8>) -> Result<(), String> {
    sink.send(InvokeResponseBody::Raw(message))
        .map_err(|error| format!("The page stopped listening: {error}"))
}

fn locked<T>(held: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    held.lock().unwrap_or_else(PoisonError::into_inner)
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
    fn new(id: u32, url: &Url, sink: Sink, recorder: Recorder) -> Self {
        Self {
            id,
            source: without_credentials(url).0.to_string(),
            sink,
            recorder,
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
        self.record(unit.codec, &nals, key)?;
        self.tally = self.tally.counted(unit, key);
        send(&self.sink, frame(key, now_ms(), unit))
    }

    fn record(&self, codec: Codec, nals: &[&[u8]], key: bool) -> Result<(), String> {
        let mut recording = locked(&self.recorder);
        let Some(failed) = recording
            .as_mut()
            .filter(|_| self.keyed)
            .and_then(|written| written.write(codec, nals, key).err())
        else {
            return Ok(());
        };
        *recording = None;
        drop(recording);
        log::warn!(target: "desktop.video", "event=recording_failed stream={} source={} reason=\"{failed}\"", self.id, self.source);
        send(&self.sink, recording_stopped(&failed.to_string()))
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        let reason = self.outcome.as_deref().unwrap_or(CLOSED);
        let level = match reason {
            CLOSED => log::Level::Info,
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
            locked(&self.recorder).is_some(),
        );
    }
}

async fn rtsp(url: &Url, live: &mut Live) -> Result<(), String> {
    let failed = |error: retina::Error| error.to_string();
    let (transport, url) = match url.scheme() {
        "rtspt" => (
            Transport::Tcp(Default::default()),
            Url::parse(&url.as_str().replacen("rtspt://", "rtsp://", 1))
                .map_err(|error| error.to_string())?,
        ),
        _ => (Transport::Udp(Default::default()), url.clone()),
    };
    let (bare, credentials) = without_credentials(&url);
    let options = SessionOptions::default()
        .creds(credentials)
        .user_agent("Aircast".to_string())
        .receiver_reports(ReceiverReportPolicy::Always);
    let mut session = Session::describe(bare, options).await.map_err(failed)?;
    let (video, codec) = session
        .streams()
        .iter()
        .enumerate()
        .find_map(
            |(index, stream)| match (stream.media(), stream.encoding_name()) {
                ("video", "h264") => Some((index, Codec::H264)),
                ("video", "h265") => Some((index, Codec::H265)),
                _ => None,
            },
        )
        .ok_or("The camera offers no H.264 or H.265 video.")?;
    session
        .setup(
            video,
            SetupOptions::default()
                .transport(transport)
                .frame_format(FrameFormat::SIMPLE),
        )
        .await
        .map_err(failed)?;
    let mut frames = session
        .play(PlayOptions::default())
        .await
        .map_err(failed)?
        .demuxed()
        .map_err(failed)?;
    while let Some(item) = frames.next().await {
        if let CodecItem::VideoFrame(shown) = item.map_err(failed)? {
            let lost = shown.loss();
            live.deliver(&Unit {
                codec,
                packets: 0,
                lost,
                data: shown.into_data(),
            })?;
        }
    }
    Err("The camera ended the stream.".to_string())
}

fn endpoint(url: &Url) -> Result<(String, u16), String> {
    let port = url.port().ok_or("The stream address has no port.")?;
    let host = url
        .host_str()
        .map(|host| host.trim_matches(['[', ']']))
        .filter(|host| !host.is_empty())
        .unwrap_or(ANY_HOST);
    Ok((host.to_string(), port))
}

async fn resolved(host: &str, port: u16) -> Result<SocketAddr, String> {
    tokio::net::lookup_host((host, port))
        .await
        .map_err(|error| format!("{host} cannot be found: {error}"))?
        .next()
        .ok_or_else(|| format!("{host} has no address."))
}

async fn listen(url: &Url) -> Result<UdpSocket, String> {
    let (host, port) = endpoint(url)?;
    let address = resolved(&host, port).await?;
    let bound = match address.ip() {
        IpAddr::V4(group) if group.is_multicast() => {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)
        }
        IpAddr::V6(group) if group.is_multicast() => {
            SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port)
        }
        _ => address,
    };
    let socket = UdpSocket::bind(bound)
        .await
        .map_err(|error| format!("Port {port} cannot be opened: {error}"))?;
    match address.ip() {
        IpAddr::V4(group) if group.is_multicast() => socket
            .join_multicast_v4(group, Ipv4Addr::UNSPECIFIED)
            .map_err(|error| error.to_string())?,
        IpAddr::V6(group) if group.is_multicast() => socket
            .join_multicast_v6(&group, 0)
            .map_err(|error| error.to_string())?,
        _ => {}
    }
    Ok(socket)
}

async fn connect(url: &Url) -> Result<TcpStream, String> {
    let (host, port) = endpoint(url)?;
    TcpStream::connect((host.as_str(), port))
        .await
        .map_err(|error| format!("{host}:{port} cannot be reached: {error}"))
}

async fn rtp(url: &Url, codec: Codec, live: &mut Live) -> Result<(), String> {
    let socket = listen(url).await?;
    let mut depacketizer = Depacketizer::new(codec);
    let mut datagram = vec![0; DATAGRAM];
    loop {
        let size = socket
            .recv(&mut datagram)
            .await
            .map_err(|error| error.to_string())?;
        depacketizer
            .push(&datagram[..size])
            .iter()
            .try_for_each(|unit| live.deliver(unit))?;
    }
}

async fn transport_stream(url: &Url, live: &mut Live) -> Result<(), String> {
    let mut demuxer = Demuxer::default();
    let mut received = vec![0; DATAGRAM];
    let mut deliver = |bytes: &[u8]| {
        demuxer
            .push(bytes)
            .iter()
            .try_for_each(|unit| live.deliver(unit))
    };
    match url.scheme() {
        "tcp" => {
            let mut stream = connect(url).await?;
            loop {
                match stream
                    .read(&mut received)
                    .await
                    .map_err(|error| error.to_string())?
                {
                    0 => return Err("The camera closed the connection.".to_string()),
                    size => deliver(&received[..size])?,
                }
            }
        }
        _ => {
            let socket = listen(url).await?;
            loop {
                let size = socket
                    .recv(&mut received)
                    .await
                    .map_err(|error| error.to_string())?;
                deliver(&received[..size])?;
            }
        }
    }
}

fn play(id: u32, url: Url, sink: Sink, recorder: Recorder) -> JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let mut live = Live::new(id, &url, sink, recorder);
        let played = match url.scheme() {
            "udp" => rtp(&url, Codec::H264, &mut live).await,
            "udp265" => rtp(&url, Codec::H265, &mut live).await,
            "mpegts" | "tcp" => transport_stream(&url, &mut live).await,
            _ => rtsp(&url, &mut live).await,
        };
        let reason = played.err().unwrap_or_default();
        live.outcome = Some(reason.clone());
        if let Err(error) = send(&live.sink, ended(&reason)) {
            log::debug!(target: "desktop.video", "event=end_not_delivered stream={id} reason=\"{error}\"");
        }
    })
}

async fn settled(task: JoinHandle<()>) {
    task.abort();
    let _ = task.await;
}

fn stop_recording(id: u32, recorder: &Recorder) -> Result<(), String> {
    let Some(recording) = locked(recorder).take() else {
        return Ok(());
    };
    log::info!(target: "desktop.video", "event=recording_stopped stream={id}");
    recording.finish().map_err(|error| error.to_string())
}

impl Streams {
    async fn open(&self, url: Url, sink: Sink) -> u32 {
        let closing = std::mem::take(&mut *locked(&self.closing));
        futures::future::join_all(closing.into_iter().map(settled)).await;
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let recorder = Recorder::default();
        let task = play(id, url.clone(), sink.clone(), recorder.clone());
        locked(&self.running).insert(
            id,
            Stream {
                task,
                url,
                sink,
                recorder,
            },
        );
        id
    }

    async fn close(&self, id: u32) -> Result<(), String> {
        let Some(stream) = locked(&self.running).remove(&id) else {
            return Ok(());
        };
        stream.task.abort();
        locked(&self.closing).push(stream.task);
        stop_recording(id, &stream.recorder)
    }

    async fn restart(&self, id: u32) -> Result<(), String> {
        let stream = locked(&self.running)
            .remove(&id)
            .ok_or("The stream is no longer playing.")?;
        settled(stream.task).await;
        let task = play(
            id,
            stream.url.clone(),
            stream.sink.clone(),
            stream.recorder.clone(),
        );
        locked(&self.running).insert(id, Stream { task, ..stream });
        Ok(())
    }

    fn record(&self, id: u32, file: Option<&Path>, size: (u32, u32)) -> Result<(), String> {
        let recorder = locked(&self.running)
            .get(&id)
            .map(|stream| stream.recorder.clone())
            .ok_or("The stream is no longer playing.")?;
        stop_recording(id, &recorder)?;
        let Some(file) = file else {
            return Ok(());
        };
        let folder = groundstation::settingsstore::video_save_path()
            .ok_or("The app has no video folder.")?;
        let target = inside_folder(Path::new(&folder), file)?;
        let recording = Recording::create(&target, size).map_err(|error| error.to_string())?;
        log::info!(target: "desktop.video", "event=recording_started stream={id} file=\"{}\" width={} height={}", target.display(), size.0, size.1);
        *locked(&recorder) = Some(recording);
        Ok(())
    }
}

#[tauri::command]
pub async fn video_open(
    url: String,
    frames: Sink,
    streams: tauri::State<'_, Streams>,
) -> Result<u32, String> {
    let url = Url::parse(&url).map_err(|error| error.to_string())?;
    Ok(streams.open(url, frames).await)
}

#[tauri::command]
pub async fn video_close(id: u32, streams: tauri::State<'_, Streams>) -> Result<(), String> {
    streams.close(id).await
}

#[tauri::command]
pub async fn video_restart(id: u32, streams: tauri::State<'_, Streams>) -> Result<(), String> {
    streams.restart(id).await
}

#[tauri::command]
pub fn video_record(
    id: u32,
    file: Option<String>,
    width: u32,
    height: u32,
    streams: tauri::State<'_, Streams>,
) -> Result<(), String> {
    streams.record(id, file.as_deref().map(Path::new), (width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet_sink() -> Sink {
        Channel::new(|_| Ok(()))
    }

    #[test]
    fn credentials_leave_the_url_for_digest_authentication() {
        let (bare, credentials) =
            without_credentials(&Url::parse("rtsp://admin:p%40ss@10.0.0.5:554/live").unwrap());
        assert_eq!(bare.as_str(), "rtsp://10.0.0.5:554/live");
        assert_eq!(
            credentials,
            Some(Credentials {
                username: "admin".to_string(),
                password: "p@ss".to_string()
            })
        );
        assert_eq!(
            without_credentials(&Url::parse("rtsp://cam/live").unwrap()).1,
            None
        );
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
                    lost: 2
                }
            ),
            [1, 1, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0, 0, 3, 0, 2, 9]
        );
        assert_eq!(ended("gone"), [2, b'g', b'o', b'n', b'e']);
        assert_eq!(recording_stopped("full"), [3, b'f', b'u', b'l', b'l']);
    }

    #[test]
    fn a_named_camera_is_looked_up_rather_than_replaced_by_this_computer() {
        assert_eq!(
            endpoint(&Url::parse("tcp://camera.local:5000").unwrap()).unwrap(),
            ("camera.local".to_string(), 5000)
        );
        assert_eq!(
            endpoint(&Url::parse("udp://0.0.0.0:5600").unwrap()).unwrap(),
            (ANY_HOST.to_string(), 5600)
        );
        assert_eq!(
            endpoint(&Url::parse("udp://[::1]:5600").unwrap()).unwrap(),
            ("::1".to_string(), 5600)
        );
        tauri::async_runtime::block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let camera = Url::parse(&format!("tcp://localhost:{port}")).unwrap();
            let reached = connect(&camera);
            let (connected, accepted) = futures::join!(reached, listener.accept());
            assert!(connected.is_ok() && accepted.is_ok());
        });
    }

    #[test]
    fn a_restart_reconnects_the_same_stream_and_keeps_its_recording() {
        tauri::async_runtime::block_on(async {
            let streams = Streams::default();
            let port = std::net::UdpSocket::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port();
            let id = streams
                .open(
                    Url::parse(&format!("udp://127.0.0.1:{port}")).unwrap(),
                    quiet_sink(),
                )
                .await;
            let recorder = locked(&streams.running)[&id].recorder.clone();
            streams.restart(id).await.unwrap();
            assert!(
                Arc::ptr_eq(&recorder, &locked(&streams.running)[&id].recorder),
                "the recording outlives the reconnect"
            );
            streams.close(id).await.unwrap();
            assert!(streams.restart(id).await.is_err());
        });
    }
}
