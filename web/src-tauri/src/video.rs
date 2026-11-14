use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

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
use crate::record::Recording;
use crate::rtp::Depacketizer;

const DATAGRAM: usize = 65_536;
const RTP_PAYLOAD: usize = 1_400;

const CONFIG: u8 = 0;
const FRAME: u8 = 1;
const ENDED: u8 = 2;

type Recorder = Arc<Mutex<Option<Recording>>>;

#[derive(Default)]
pub struct Streams {
    next: AtomicU32,
    running: Mutex<HashMap<u32, (JoinHandle<()>, Recorder)>>,
}

struct Live {
    sink: Channel<InvokeResponseBody>,
    recorder: Recorder,
    announced: Option<String>,
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

fn send(sink: &Channel<InvokeResponseBody>, message: Vec<u8>) -> Result<(), String> {
    sink.send(InvokeResponseBody::Raw(message))
        .map_err(|error| error.to_string())
}

fn locked<T>(held: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    held.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Live {
    fn deliver(&mut self, unit: &Unit) -> Result<(), String> {
        if let Some(found) = annexb::codec_of(unit.codec, &unit.data)
            .filter(|found| self.announced.as_ref() != Some(found))
        {
            send(&self.sink, config(&found))?;
            self.announced = Some(found);
        }
        let key = annexb::keyed(unit.codec, &unit.data);
        let mut recording = locked(&self.recorder);
        if let Some(failed) = recording
            .as_mut()
            .and_then(|written| written.write(unit.codec, &unit.data, key).err())
        {
            *recording = None;
            send(&self.sink, ended(&format!("Recording stopped: {failed}")))?;
        }
        drop(recording);
        send(&self.sink, frame(key, now_ms(), unit))
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
            let data = shown.into_data();
            live.deliver(&Unit {
                codec,
                packets: u16::try_from(data.len().div_ceil(RTP_PAYLOAD)).unwrap_or(u16::MAX),
                lost,
                data,
            })?;
        }
    }
    Err("The camera ended the stream.".to_string())
}

fn endpoint(url: &Url) -> Result<(IpAddr, u16), String> {
    let port = url.port().ok_or("The stream address has no port.")?;
    let host = url.host_str().unwrap_or_default().trim_matches(['[', ']']);
    Ok((
        host.parse().unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        port,
    ))
}

async fn listen(url: &Url) -> Result<UdpSocket, String> {
    let (address, port) = endpoint(url)?;
    let bound = if address.is_multicast() {
        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    } else {
        address
    };
    let socket = UdpSocket::bind((bound, port))
        .await
        .map_err(|error| format!("Port {port} cannot be opened: {error}"))?;
    if let IpAddr::V4(group) = address.is_multicast().then_some(address).unwrap_or(bound) {
        if group.is_multicast() {
            socket
                .join_multicast_v4(group, Ipv4Addr::UNSPECIFIED)
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(socket)
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
            let mut stream = TcpStream::connect(endpoint(url)?)
                .await
                .map_err(|error| error.to_string())?;
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

#[tauri::command]
pub fn video_open(
    url: String,
    frames: Channel<InvokeResponseBody>,
    streams: tauri::State<'_, Streams>,
) -> Result<u32, String> {
    let url = Url::parse(&url).map_err(|error| error.to_string())?;
    let id = streams.next.fetch_add(1, Ordering::Relaxed);
    let recorder = Recorder::default();
    let mut live = Live {
        sink: frames,
        recorder: recorder.clone(),
        announced: None,
    };
    let task = tauri::async_runtime::spawn(async move {
        let played = match url.scheme() {
            "udp" => rtp(&url, Codec::H264, &mut live).await,
            "udp265" => rtp(&url, Codec::H265, &mut live).await,
            "mpegts" | "tcp" => transport_stream(&url, &mut live).await,
            _ => rtsp(&url, &mut live).await,
        };
        let reason = played.err().unwrap_or_default();
        let _ = send(&live.sink, ended(&reason));
    });
    locked(&streams.running).insert(id, (task, recorder));
    Ok(id)
}

fn stop_recording(recorder: &Recorder) -> Result<(), String> {
    locked(recorder)
        .take()
        .map_or(Ok(()), Recording::finish)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn video_close(id: u32, streams: tauri::State<'_, Streams>) -> Result<(), String> {
    let Some((task, recorder)) = locked(&streams.running).remove(&id) else {
        return Ok(());
    };
    task.abort();
    stop_recording(&recorder)
}

#[tauri::command]
pub fn video_record(
    id: u32,
    file: Option<String>,
    width: u32,
    height: u32,
    streams: tauri::State<'_, Streams>,
) -> Result<(), String> {
    let recorder = locked(&streams.running)
        .get(&id)
        .map(|(_, recorder)| recorder.clone())
        .ok_or("The stream is no longer playing.")?;
    stop_recording(&recorder)?;
    *locked(&recorder) = file
        .map(|path| Recording::create(&path, (width, height)))
        .transpose()
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
