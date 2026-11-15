use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use futures::StreamExt;
use percent_encoding::percent_decode_str;
use retina::client::{
    Credentials, PlayOptions, ReceiverReportPolicy, Session, SessionOptions, SetupOptions,
    Transport, UdpTransportOptions,
};
use retina::codec::{CodecItem, FrameFormat};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpStream, UdpSocket};
use url::Url;

use crate::annexb::{Codec, Unit};
use crate::mpegts::Demuxer;
use crate::rtp::{Depacketizer, REORDER_WINDOW};

const DATAGRAM: usize = 65_536;
const REORDER_DELAY: std::time::Duration = std::time::Duration::from_millis(20);
const ANY_HOST: &str = "0.0.0.0";

pub fn without_credentials(url: &Url) -> (Url, Option<Credentials>) {
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

pub async fn play(
    url: &Url,
    deliver: &mut impl FnMut(&Unit) -> Result<(), String>,
) -> Result<(), String> {
    match url.scheme() {
        "udp" => rtp(url, Codec::H264, deliver).await,
        "udp265" => rtp(url, Codec::H265, deliver).await,
        "mpegts" | "tcp" => transport_stream(url, deliver).await,
        _ => rtsp(url, deliver).await,
    }
}

async fn rtsp(
    url: &Url,
    deliver: &mut impl FnMut(&Unit) -> Result<(), String>,
) -> Result<(), String> {
    let failed = |error: retina::Error| error.to_string();
    let (transport, url) = match url.scheme() {
        "rtspt" => (
            Transport::Tcp(Default::default()),
            Url::parse(&url.as_str().replacen("rtspt://", "rtsp://", 1))
                .map_err(|error| error.to_string())?,
        ),
        _ => (
            Transport::Udp(UdpTransportOptions::default().reorder(REORDER_WINDOW, REORDER_DELAY)),
            url.clone(),
        ),
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
            let (lost, timestamp) = (shown.loss(), shown.timestamp().timestamp());
            deliver(&Unit {
                codec,
                packets: 0,
                lost,
                timestamp: u64::try_from(timestamp).unwrap_or_default(),
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

async fn rtp(
    url: &Url,
    codec: Codec,
    deliver: &mut impl FnMut(&Unit) -> Result<(), String>,
) -> Result<(), String> {
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
            .try_for_each(&mut *deliver)?;
    }
}

async fn transport_stream(
    url: &Url,
    deliver: &mut impl FnMut(&Unit) -> Result<(), String>,
) -> Result<(), String> {
    let mut demuxer = Demuxer::default();
    let mut received = vec![0; DATAGRAM];
    let mut demuxed = |bytes: &[u8]| demuxer.push(bytes).iter().try_for_each(&mut *deliver);
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
                    size => demuxed(&received[..size])?,
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
                demuxed(&received[..size])?;
            }
        }
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
}
