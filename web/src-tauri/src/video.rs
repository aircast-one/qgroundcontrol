use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use futures::StreamExt;
use percent_encoding::percent_decode_str;
use retina::client::{
    Credentials, PlayOptions, ReceiverReportPolicy, Session, SessionOptions, SetupOptions,
    Transport,
};
use retina::codec::{CodecItem, ParametersRef, VideoParameters};
use tauri::async_runtime::JoinHandle;
use tauri::ipc::{Channel, InvokeResponseBody};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpStream, UdpSocket};
use url::Url;

use crate::annexb::{self, Codec};
use crate::mpegts::Demuxer;
use crate::rtp::Depacketizer;

const DATAGRAM: usize = 65_536;

const CONFIG: u8 = 0;
const FRAME: u8 = 1;
const ENDED: u8 = 2;

#[derive(Default)]
pub struct Streams {
    next: AtomicU32,
    running: Mutex<HashMap<u32, JoinHandle<()>>>,
}

fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |since| since.as_secs_f64() * 1000.0)
}

fn config(codec: &str, (width, height): (u32, u32), description: &[u8]) -> Vec<u8> {
    [
        &[CONFIG][..],
        &u16::try_from(codec.len()).unwrap_or(0).to_be_bytes(),
        codec.as_bytes(),
        &width.to_be_bytes(),
        &height.to_be_bytes(),
        description,
    ]
    .concat()
}

fn frame(key: bool, sent_ms: f64, data: &[u8]) -> Vec<u8> {
    [&[FRAME, u8::from(key)][..], &sent_ms.to_be_bytes(), data].concat()
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

fn send_config(
    sink: &Channel<InvokeResponseBody>,
    parameters: &VideoParameters,
) -> Result<(), String> {
    send(
        sink,
        config(
            parameters.rfc6381_codec(),
            parameters.pixel_dimensions(),
            parameters.extra_data(),
        ),
    )
}

async fn rtsp(url: &Url, sink: &Channel<InvokeResponseBody>) -> Result<(), String> {
    let failed = |error: retina::Error| error.to_string();
    let (bare, credentials) = without_credentials(url);
    let options = SessionOptions::default()
        .creds(credentials)
        .user_agent("Aircast".to_string())
        .receiver_reports(ReceiverReportPolicy::Always);
    let mut session = Session::describe(bare, options).await.map_err(failed)?;
    let video = session
        .streams()
        .iter()
        .position(|stream| {
            stream.media() == "video" && matches!(stream.encoding_name(), "h264" | "h265")
        })
        .ok_or("The camera offers no H.264 or H.265 video.")?;
    session
        .setup(
            video,
            SetupOptions::default().transport(Transport::Udp(Default::default())),
        )
        .await
        .map_err(failed)?;
    let mut frames = session
        .play(PlayOptions::default())
        .await
        .map_err(failed)?
        .demuxed()
        .map_err(failed)?;
    if let Some(ParametersRef::Video(parameters)) = frames.streams()[video].parameters() {
        send_config(sink, parameters)?;
    }
    while let Some(item) = frames.next().await {
        let CodecItem::VideoFrame(shown) = item.map_err(failed)? else {
            continue;
        };
        if shown.has_new_parameters() {
            if let Some(ParametersRef::Video(parameters)) = frames.streams()[video].parameters() {
                send_config(sink, parameters)?;
            }
        }
        send(
            sink,
            frame(shown.is_random_access_point(), now_ms(), shown.data()),
        )?;
    }
    Err("The camera ended the stream.".to_string())
}

fn forward(
    sink: &Channel<InvokeResponseBody>,
    codec: Codec,
    unit: &[u8],
    announced: &mut Option<String>,
) -> Result<(), String> {
    if let Some(found) =
        annexb::codec_of(codec, unit).filter(|found| announced.as_ref() != Some(found))
    {
        send(sink, config(&found, (0, 0), &[]))?;
        *announced = Some(found);
    }
    send(sink, frame(annexb::keyed(codec, unit), now_ms(), unit))
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

async fn rtp(url: &Url, codec: Codec, sink: &Channel<InvokeResponseBody>) -> Result<(), String> {
    let socket = listen(url).await?;
    let mut depacketizer = Depacketizer::new(codec);
    let mut announced = None;
    let mut datagram = vec![0; DATAGRAM];
    loop {
        let size = socket
            .recv(&mut datagram)
            .await
            .map_err(|error| error.to_string())?;
        depacketizer
            .push(&datagram[..size])
            .iter()
            .try_for_each(|unit| forward(sink, codec, unit, &mut announced))?;
    }
}

async fn transport_stream(url: &Url, sink: &Channel<InvokeResponseBody>) -> Result<(), String> {
    let mut demuxer = Demuxer::default();
    let mut announced = None;
    let mut received = vec![0; DATAGRAM];
    let mut deliver = |bytes: &[u8]| {
        demuxer
            .push(bytes)
            .iter()
            .try_for_each(|(codec, unit)| forward(sink, *codec, unit, &mut announced))
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
    let task = tauri::async_runtime::spawn(async move {
        let played = match url.scheme() {
            "udp" => rtp(&url, Codec::H264, &frames).await,
            "udp265" => rtp(&url, Codec::H265, &frames).await,
            "mpegts" | "tcp" => transport_stream(&url, &frames).await,
            _ => rtsp(&url, &frames).await,
        };
        let reason = played.err().unwrap_or_default();
        let _ = send(&frames, ended(&reason));
    });
    streams
        .running
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(id, task);
    Ok(id)
}

#[tauri::command]
pub fn video_close(id: u32, streams: tauri::State<'_, Streams>) {
    if let Some(task) = streams
        .running
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&id)
    {
        task.abort();
    }
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
        assert_eq!(
            config("avc1.42c01f", (1280, 720), &[1, 2]),
            [
                0, 0, 11, b'a', b'v', b'c', b'1', b'.', b'4', b'2', b'c', b'0', b'1', b'f', 0, 0,
                5, 0, 0, 0, 2, 208, 1, 2
            ]
        );
        assert_eq!(
            frame(true, 1.5, &[9]),
            [1, 1, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0, 9]
        );
        assert_eq!(ended("gone"), [2, b'g', b'o', b'n', b'e']);
    }
}
