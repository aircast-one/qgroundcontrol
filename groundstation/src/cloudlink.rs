use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};
use std::time::{Duration, Instant};

use tungstenite::Message;
use tungstenite::client::IntoClientRequest;
use tungstenite::stream::MaybeTlsStream;

pub const ACCOUNT_GROUP: &str = "AircastAccount";
const CONTROL_PREFIX: u8 = 0x00;
const INITIAL_BACKOFF_MS: u64 = 2000;
const MAX_BACKOFF_MS: u64 = 60_000;
const POLL: Duration = Duration::from_millis(20);
const OUTBOX_FRAMES: usize = 64;
const RELAY_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const RELAY_WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const PING_EVERY: Duration = Duration::from_secs(5);
const RELAY_SILENT: Duration = Duration::from_secs(15);
pub const RELAY_LOST: &str = "The Aircast relay connection was lost. Reconnecting.";

pub fn relay_url(api_base: &str, device_id: &str) -> Option<String> {
    let parsed = url::Url::parse(api_base.trim()).ok().filter(|u| u.host_str().is_some_and(|h| !h.is_empty()))?;
    let device = device_id.trim();
    if device.is_empty() {
        return None;
    }
    let scheme = if parsed.scheme() == "http" { "ws" } else { "wss" };
    let host = parsed.host_str()?;
    let port = parsed.port().map(|p| format!(":{p}")).unwrap_or_default();
    let path = parsed.path().trim_end_matches('/');
    Some(format!("{scheme}://{host}{port}{path}/v1/mavlink/web/{device}/ws"))
}

pub fn token_key(api_base: &str) -> Option<String> {
    let host = url::Url::parse(api_base.trim()).ok()?.host_str()?.to_string();
    Some(format!("{ACCOUNT_GROUP}/tokens/{host}"))
}

pub fn backoff_ms(failures: u32) -> u64 {
    (INITIAL_BACKOFF_MS << failures.min(5)).min(MAX_BACKOFF_MS)
}

pub struct CloudLink {
    outbox: SyncSender<Vec<u8>>,
    wanted: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
}

type Socket = tungstenite::WebSocket<MaybeTlsStream<TcpStream>>;

fn reached(host: &str, port: u16) -> Result<TcpStream, String> {
    let addresses: Vec<std::net::SocketAddr> = (host.trim_start_matches('[').trim_end_matches(']'), port).to_socket_addrs().map_err(|e| e.to_string())?.collect();
    let stream = addresses.iter().find_map(|address| TcpStream::connect_timeout(address, RELAY_CONNECT_TIMEOUT).ok()).ok_or_else(|| format!("No answer from {host}:{port}"))?;
    stream.set_read_timeout(Some(RELAY_CONNECT_TIMEOUT)).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(RELAY_WRITE_TIMEOUT)).map_err(|e| e.to_string())?;
    Ok(stream)
}

fn connect(url: &str, token: &str) -> Result<Socket, String> {
    let mut request = url.into_client_request().map_err(|e| e.to_string())?;
    request.headers_mut().insert("Authorization", format!("Bearer {token}").parse().map_err(|_| "the token is not a header value".to_string())?);
    let host = request.uri().host().ok_or("the relay address has no host")?.to_string();
    let port = request.uri().port_u16().unwrap_or(if request.uri().scheme_str() == Some("wss") { 443 } else { 80 });
    let (socket, _) = tungstenite::client_tls(request, reached(&host, port)?).map_err(|e| e.to_string())?;
    let stream = match socket.get_ref() {
        MaybeTlsStream::Plain(tcp) => Some(tcp),
        MaybeTlsStream::Rustls(tls) => Some(tls.get_ref()),
        _ => None,
    };
    if let Some(tcp) = stream {
        tcp.set_read_timeout(Some(POLL)).map_err(|e| e.to_string())?;
    }
    Ok(socket)
}

fn pump(socket: &mut Socket, outbox: &Receiver<Vec<u8>>, wanted: &AtomicBool, deliver: &mut dyn FnMut(&[u8])) -> bool {
    let (mut heard, mut pinged) = (Instant::now(), Instant::now());
    while wanted.load(Ordering::SeqCst) {
        loop {
            match outbox.try_recv() {
                Ok(bytes) => {
                    if socket.send(Message::binary(bytes)).is_err() {
                        return true;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return false,
            }
        }
        if heard.elapsed() >= RELAY_SILENT {
            return true;
        }
        if pinged.elapsed() >= PING_EVERY {
            if socket.send(Message::Ping(tungstenite::Bytes::new())).is_err() {
                return true;
            }
            pinged = Instant::now();
        }
        match socket.read() {
            Ok(Message::Binary(message)) => {
                heard = Instant::now();
                if message.first().is_some_and(|b| *b != CONTROL_PREFIX) {
                    deliver(&message);
                }
            }
            Ok(Message::Close(_)) => return true,
            Ok(_) => heard = Instant::now(),
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(_) => return true,
        }
    }
    let _ = socket.close(None);
    false
}

impl CloudLink {
    pub fn open(url: String, token: String, mut deliver: impl FnMut(&[u8]) + Send + 'static, relay: impl Fn(bool, &str) + Send + 'static) -> CloudLink {
        let (sender, outbox) = std::sync::mpsc::sync_channel::<Vec<u8>>(OUTBOX_FRAMES);
        let wanted = Arc::new(AtomicBool::new(true));
        let connected = Arc::new(AtomicBool::new(false));
        let (running, up) = (wanted.clone(), connected.clone());
        std::thread::Builder::new()
            .name("qgc-cloud".into())
            .spawn(move || {
                let mut failures = 0u32;
                while running.load(Ordering::SeqCst) {
                    let retry = match connect(&url, &token) {
                        Ok(mut socket) => {
                            failures = 0;
                            outbox.try_iter().for_each(drop);
                            up.store(true, Ordering::SeqCst);
                            relay(true, "");
                            let retry = pump(&mut socket, &outbox, &running, &mut deliver);
                            up.store(false, Ordering::SeqCst);
                            if retry {
                                relay(false, RELAY_LOST);
                            }
                            retry
                        }
                        Err(error) => {
                            relay(false, &error);
                            true
                        }
                    };
                    if !retry {
                        return;
                    }
                    let wait = backoff_ms(failures);
                    failures += 1;
                    let steps = wait / POLL.as_millis() as u64;
                    (0..steps).take_while(|_| running.load(Ordering::SeqCst)).for_each(|_| std::thread::sleep(POLL));
                }
            })
            .ok();
        CloudLink { outbox: sender, wanted, connected }
    }

    pub fn write(&self, bytes: &[u8]) -> bool {
        self.wanted.load(Ordering::SeqCst) && self.connected.load(Ordering::SeqCst) && self.outbox.try_send(bytes.to_vec()).is_ok()
    }

    pub fn close(&self) {
        self.wanted.store(false, Ordering::SeqCst);
    }
}

impl Drop for CloudLink {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_relay_url_follows_aircast_cloud_configuration() {
        assert_eq!(relay_url("https://api.aircast.one", "d-1").as_deref(), Some("wss://api.aircast.one/v1/mavlink/web/d-1/ws"));
        assert_eq!(relay_url("http://10.0.0.5:8080/base/", " d-2 ").as_deref(), Some("ws://10.0.0.5:8080/base/v1/mavlink/web/d-2/ws"));
        assert_eq!(relay_url("https://api.aircast.one", ""), None);
        assert_eq!(relay_url("not a url", "d"), None);
        assert_eq!(token_key("https://api.aircast.one/").as_deref(), Some("AircastAccount/tokens/api.aircast.one"));
        assert_eq!((backoff_ms(0), backoff_ms(4), backoff_ms(9)), (2000, 32_000, 60_000));
    }

    #[test]
    fn frames_cross_the_relay_both_ways_and_status_frames_are_not_mavlink() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let authorised = Arc::new(std::sync::Mutex::new(String::new()));
        let seen = authorised.clone();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let callback = |request: &tungstenite::handshake::server::Request, response| {
                *seen.lock().unwrap() = request.headers().get("Authorization").and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
                Ok(response)
            };
            let mut socket = tungstenite::accept_hdr(stream, callback).unwrap();
            socket.send(Message::binary(vec![CONTROL_PREFIX, 1, b'o', b'k'])).unwrap();
            socket.send(Message::binary(vec![0xFD, 1, 2, 3])).unwrap();
            loop {
                if let Ok(Message::Binary(echo)) = socket.read() {
                    return echo.to_vec();
                }
            }
        });
        let (tx, rx) = std::sync::mpsc::channel();
        let link = CloudLink::open(
            format!("ws://127.0.0.1:{port}/v1/mavlink/web/d/ws"),
            "secret".into(),
            move |bytes| {
                let _ = tx.send(bytes.to_vec());
            },
            |_, _| {},
        );
        let received = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(received, vec![0xFD, 1, 2, 3], "the control frame is the relay's status, never handed on as MAVLink");
        assert!(link.write(&[0xFD, 9]));
        assert_eq!(server.join().unwrap(), vec![0xFD, 9]);
        assert_eq!(*authorised.lock().unwrap(), "Bearer secret");
        link.close();
    }

    #[test]
    fn nothing_waits_for_the_vehicle_while_the_relay_is_down_and_the_outbox_is_bounded() {
        let (outbox, waiting) = std::sync::mpsc::sync_channel(OUTBOX_FRAMES);
        let link = CloudLink { outbox, wanted: Arc::new(AtomicBool::new(true)), connected: Arc::new(AtomicBool::new(false)) };
        assert!(!link.write(&[0xFD, 1]), "AircastCloudLink::_writeBytes drops what it cannot send, so an RTL the UI already showed as failed is never replayed on reconnect");
        assert_eq!(waiting.try_iter().count(), 0);
        link.connected.store(true, Ordering::SeqCst);
        assert_eq!((0..OUTBOX_FRAMES + 10).filter(|_| link.write(&[0xFD, 2])).count(), OUTBOX_FRAMES, "a relay that stops draining holds a few frames, never the whole outage");
    }

    #[test]
    fn a_relay_that_drops_the_connection_is_reported_down_and_takes_no_writes() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            drop(tungstenite::accept(stream).unwrap());
        });
        let (tx, states) = std::sync::mpsc::channel();
        let link = CloudLink::open(format!("ws://127.0.0.1:{port}/v1/mavlink/web/d/ws"), "secret".into(), |_| {}, move |up, reason| {
            let _ = tx.send((up, reason.to_string()));
        });
        assert_eq!(states.recv_timeout(Duration::from_secs(5)).unwrap(), (true, String::new()));
        server.join().unwrap();
        assert_eq!(states.recv_timeout(Duration::from_secs(5)).unwrap(), (false, RELAY_LOST.to_string()), "the registry and hub hear of the outage instead of writing into it");
        assert!(!link.write(&[0xFD, 3]));
        link.close();
    }
}
