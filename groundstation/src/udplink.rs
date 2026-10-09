use socket2::{Domain, Protocol, Socket, Type};
use std::collections::BTreeSet;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub fn local_addresses() -> BTreeSet<Ipv4Addr> {
    if_addrs::get_if_addrs().map(|list| list.into_iter().filter_map(|i| match i.ip() { std::net::IpAddr::V4(v4) => Some(v4), _ => None }).collect()).unwrap_or_default()
}

pub const MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 1);
const READ_TIMEOUT: Duration = Duration::from_millis(200);
const MAX_DATAGRAM: usize = 65535;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpConfig {
    pub local_port: u16,
    pub targets: Vec<(String, u16)>,
}

pub struct UdpLink {
    socket: UdpSocket,
    configured: Mutex<Vec<SocketAddr>>,
    session: Arc<Mutex<BTreeSet<SocketAddr>>>,
    stop: Arc<AtomicBool>,
    reader: Mutex<Option<JoinHandle<()>>>,
}

fn resolve(host: &str, port: u16) -> Option<SocketAddr> {
    (host, port).to_socket_addrs().ok()?.find(|a| a.is_ipv4())
}

fn resolved(targets: &[(String, u16)]) -> Vec<SocketAddr> {
    targets.iter().filter_map(|(h, p)| resolve(h, *p)).collect()
}

fn normalise(sender: SocketAddr, local: &BTreeSet<Ipv4Addr>) -> SocketAddr {
    match sender {
        SocketAddr::V4(v4) if v4.ip().is_loopback() || local.contains(v4.ip()) => SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, v4.port())),
        other => other,
    }
}

fn transient(error: &io::Error) -> bool {
    matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted | io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionRefused)
}

pub fn receive_until_lost<From>(running: impl Fn() -> bool, mut receive: impl FnMut(&mut [u8]) -> io::Result<(usize, From)>, mut deliver: impl FnMut(&[u8], From)) -> Option<io::Error> {
    let mut buffer = vec![0u8; MAX_DATAGRAM];
    std::iter::from_fn(|| running().then(|| receive(&mut buffer).map(|(len, from)| deliver(&buffer[..len], from)))).find_map(|received| received.err().filter(|error| !transient(error)))
}

fn bind_shared(port: u16) -> io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.bind(&SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)).into())?;
    let _ = socket.join_multicast_v4(&MULTICAST_GROUP, &Ipv4Addr::UNSPECIFIED);
    socket.set_read_timeout(Some(READ_TIMEOUT))?;
    Ok(socket.into())
}

impl UdpLink {
    pub fn open(config: &UdpConfig, local_addresses: BTreeSet<Ipv4Addr>, mut sink: impl FnMut(&[u8]) + Send + 'static, lost: impl FnOnce(String) + Send + 'static) -> io::Result<UdpLink> {
        let socket = bind_shared(config.local_port)?;
        let configured = Mutex::new(resolved(&config.targets));
        let session = Arc::new(Mutex::new(BTreeSet::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let reader = {
            let socket = socket.try_clone()?;
            let session = Arc::clone(&session);
            let stop = Arc::clone(&stop);
            std::thread::Builder::new().name("qgc-udp".into()).spawn(move || {
                let failed = receive_until_lost(
                    || !stop.load(Ordering::Relaxed),
                    |buffer| socket.recv_from(buffer),
                    |bytes, sender| {
                        if !bytes.is_empty() {
                            session.lock().unwrap().insert(normalise(sender, &local_addresses));
                            sink(bytes);
                        }
                    },
                );
                if let Some(error) = failed {
                    lost(error.to_string());
                }
            })?
        };
        Ok(UdpLink { socket, configured, session, stop, reader: Mutex::new(Some(reader)) })
    }

    pub fn local_port(&self) -> u16 {
        self.socket.local_addr().map(|a| a.port()).unwrap_or(0)
    }

    pub fn retarget(&self, targets: &[(String, u16)]) {
        let addresses = resolved(targets);
        *self.configured.lock().unwrap() = addresses;
    }

    pub fn targets(&self) -> Vec<SocketAddr> {
        let configured = self.configured.lock().unwrap().clone();
        let session = self.session.lock().unwrap();
        configured.iter().filter(|c| !session.contains(c)).copied().chain(session.iter().copied()).collect()
    }

    pub fn write(&self, bytes: &[u8]) -> usize {
        self.targets().iter().filter(|target| self.socket.send_to(bytes, target).is_ok()).count()
    }

    pub fn close(&self) {
        self.stop.store(true, Ordering::Relaxed);
        let handle = self.reader.lock().unwrap().take();
        if let Some(reader) = handle {
            let _ = reader.join();
        }
    }
}

impl Drop for UdpLink {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn heartbeat() -> Vec<u8> {
        let bytes = crate::samplelog::bytes();
        crate::tlog::entries(&bytes, u64::MAX).into_iter().map(|(_, f)| f).find(|f| f.len() == 21).unwrap()
    }

    #[test]
    fn a_peer_that_sends_first_becomes_a_session_target_and_gets_the_replies() {
        let (tx, rx) = mpsc::channel();
        let link = UdpLink::open(&UdpConfig { local_port: 0, targets: vec![] }, BTreeSet::new(), move |bytes| tx.send(bytes.to_vec()).unwrap(), |_| {}).unwrap();
        let port = link.local_port();
        assert!(port > 0);
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let frame = heartbeat();
        peer.send_to(&frame, ("127.0.0.1", port)).unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), frame);
        assert_eq!(link.targets(), vec![SocketAddr::from(([127, 0, 0, 1], peer.local_addr().unwrap().port()))]);
        assert_eq!(link.write(&frame), 1);
        let mut back = [0u8; 64];
        let (len, from) = peer.recv_from(&mut back).unwrap();
        assert_eq!((&back[..len], from.port()), (frame.as_slice(), port));
        link.close();
    }

    #[test]
    fn a_server_added_to_an_open_link_is_written_to_like_udplink_reading_its_configuration_live() {
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let peer_port = peer.local_addr().unwrap().port();
        let link = UdpLink::open(&UdpConfig { local_port: 0, targets: vec![] }, BTreeSet::new(), |_| {}, |_| {}).unwrap();
        assert_eq!(link.write(&heartbeat()), 0);
        link.retarget(&[("127.0.0.1".into(), peer_port)]);
        assert_eq!(link.write(&heartbeat()), 1);
        let mut back = [0u8; 64];
        assert!(peer.recv_from(&mut back).is_ok());
        link.retarget(&[]);
        assert_eq!(link.targets(), vec![], "a removed server is no longer written to");
        link.close();
    }

    #[test]
    fn configured_targets_are_written_to_once_even_after_they_answer() {
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let peer_port = peer.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        let link = UdpLink::open(&UdpConfig { local_port: 0, targets: vec![("localhost".into(), peer_port), ("nonexistent.invalid".into(), 1)] }, BTreeSet::new(), move |bytes| tx.send(bytes.len()).unwrap(), |_| {}).unwrap();
        assert_eq!(link.targets().len(), 1);
        let frame = heartbeat();
        assert_eq!(link.write(&frame), 1);
        let mut back = [0u8; 64];
        let (len, _) = peer.recv_from(&mut back).unwrap();
        assert_eq!(len, frame.len());
        peer.send_to(&frame, ("127.0.0.1", link.local_port())).unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), frame.len());
        assert_eq!(link.targets().len(), 1);
        assert_eq!(link.write(&frame), 1);
        link.close();
    }

    #[test]
    fn two_links_can_share_the_same_listen_port() {
        let first = UdpLink::open(&UdpConfig { local_port: 0, targets: vec![] }, BTreeSet::new(), |_| {}, |_| {}).unwrap();
        let port = first.local_port();
        let second = UdpLink::open(&UdpConfig { local_port: port, targets: vec![] }, BTreeSet::new(), |_| {}, |_| {});
        assert!(second.is_ok());
        assert_eq!(normalise(SocketAddr::from(([10, 0, 0, 5], 14550)), &BTreeSet::from([Ipv4Addr::new(10, 0, 0, 5)])), SocketAddr::from(([127, 0, 0, 1], 14550)));
    }

    #[test]
    fn a_socket_that_fails_for_good_ends_the_reader_while_timeouts_and_icmp_echoes_do_not() {
        let mut script = [Ok((2, ())), Err(io::ErrorKind::TimedOut), Err(io::ErrorKind::WouldBlock), Err(io::ErrorKind::Interrupted), Err(io::ErrorKind::ConnectionReset), Ok((1, ())), Err(io::ErrorKind::NotConnected), Ok((3, ()))]
            .map(|step| step.map_err(io::Error::from))
            .into_iter();
        let delivered = std::cell::RefCell::new(Vec::new());
        let lost = receive_until_lost(|| true, |_| script.next().unwrap(), |bytes, ()| delivered.borrow_mut().push(bytes.len()));
        assert_eq!(lost.map(|error| error.kind()), Some(io::ErrorKind::NotConnected), "a suspended app's defunct socket answers ENOTCONN at once, forever");
        assert_eq!(delivered.into_inner(), vec![2, 1]);
        assert_eq!(script.len(), 1, "nothing is read after the socket is gone, so the reader cannot spin");
        assert!(receive_until_lost(|| false, |_| -> io::Result<(usize, ())> { panic!("a stopped reader reads nothing") }, |_, ()| {}).is_none());
    }
}
