use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub const CONNECT_TIMEOUT: Duration = Duration::from_millis(1000);
const READ_TIMEOUT: Duration = Duration::from_millis(200);
const WRITE_TIMEOUT: Duration = Duration::from_millis(2000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Bytes(Vec<u8>),
    Disconnected(String),
}

pub struct TcpLink {
    stream: TcpStream,
    stop: Arc<AtomicBool>,
    reader: Mutex<Option<JoinHandle<()>>>,
    peer: SocketAddr,
}

impl TcpLink {
    pub fn open(config: &TcpConfig, mut sink: impl FnMut(Event) + Send + 'static) -> io::Result<TcpLink> {
        let addresses: Vec<SocketAddr> = (config.host.as_str(), config.port).to_socket_addrs()?.collect();
        let stream = addresses
            .iter()
            .map(|a| TcpStream::connect_timeout(a, CONNECT_TIMEOUT))
            .find_map(Result::ok)
            .ok_or_else(|| io::Error::new(io::ErrorKind::ConnectionRefused, format!("Connection to {}:{} failed", config.host, config.port)))?;
        stream.set_read_timeout(Some(READ_TIMEOUT))?;
        stream.set_write_timeout(Some(WRITE_TIMEOUT))?;
        stream.set_nodelay(true)?;
        let peer = stream.peer_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let reader = {
            let mut socket = stream.try_clone()?;
            let stop = Arc::clone(&stop);
            std::thread::Builder::new().name("qgc-tcp".into()).spawn(move || {
                let mut buffer = vec![0u8; 16 * 1024];
                while !stop.load(Ordering::Relaxed) {
                    match socket.read(&mut buffer) {
                        Ok(0) => {
                            sink(Event::Disconnected("peer closed the connection".into()));
                            return;
                        }
                        Ok(len) => sink(Event::Bytes(buffer[..len].to_vec())),
                        Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted) => {}
                        Err(e) => {
                            sink(Event::Disconnected(e.to_string()));
                            return;
                        }
                    }
                }
            })?
        };
        Ok(TcpLink { stream, stop, reader: Mutex::new(Some(reader)), peer })
    }

    pub fn peer(&self) -> SocketAddr {
        self.peer
    }

    pub fn write(&self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Data to Send is Empty"));
        }
        (&self.stream).write_all(bytes).map(|_| bytes.len())
    }

    pub fn close(&self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.stream.shutdown(Shutdown::Both);
        let handle = self.reader.lock().unwrap().take();
        if let Some(reader) = handle {
            let _ = reader.join();
        }
    }
}

impl Drop for TcpLink {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::mpsc;

    #[test]
    fn bytes_flow_both_ways_and_a_closed_peer_is_reported() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut first = [0u8; 5];
            socket.read_exact(&mut first).unwrap();
            socket.write_all(b"pong").unwrap();
            first
        });
        let (tx, rx) = mpsc::channel();
        let link = TcpLink::open(&TcpConfig { host: "localhost".into(), port }, move |e| tx.send(e).unwrap()).unwrap();
        assert_eq!(link.peer().port(), port);
        assert_eq!(link.write(b"hello").unwrap(), 5);
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), Event::Bytes(b"pong".to_vec()));
        assert_eq!(server.join().unwrap(), *b"hello");
        assert!(matches!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), Event::Disconnected(_)));
        assert!(link.write(b"").is_err());
        link.close();
    }

    #[test]
    fn a_refused_port_fails_within_the_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let started = std::time::Instant::now();
        assert!(TcpLink::open(&TcpConfig { host: "127.0.0.1".into(), port }, |_| {}).is_err());
        assert!(started.elapsed() < Duration::from_secs(3));
        assert!(TcpLink::open(&TcpConfig { host: "nonexistent.invalid".into(), port: 1 }, |_| {}).is_err());
    }
}
