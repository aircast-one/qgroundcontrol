use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock, Mutex};

use crate::linkconfig::{self, Kind, LinkConfig};
use crate::tcplink::{self, TcpConfig, TcpLink};
use crate::transport::{Frame, LinkId, Owner, Registry};
use crate::udplink::{UdpConfig, UdpLink};

pub type Writer = Arc<dyn Fn(LinkId, &[u8]) + Send + Sync>;
pub type FrameSink = Arc<dyn Fn(&Frame) + Send + Sync>;

enum Owned {
    Udp(UdpLink),
    Tcp(TcpLink),
    #[cfg(not(target_os = "android"))]
    Serial(crate::seriallink::SerialLink),
}

#[derive(Default)]
pub struct Transports {
    registry: Arc<Mutex<Registry>>,
    owned: BTreeMap<LinkId, Owned>,
    configs: BTreeMap<LinkId, LinkConfig>,
    writer: Option<Writer>,
    sink: Option<FrameSink>,
}

pub static TRANSPORTS: LazyLock<Mutex<Transports>> = LazyLock::new(|| Mutex::new(Transports::default()));

fn deliver(registry: &Arc<Mutex<Registry>>, sink: &Option<FrameSink>, id: LinkId, bytes: &[u8]) {
    let frames = registry.lock().unwrap().bytes_in(id, bytes);
    if let Some(sink) = sink {
        frames.iter().for_each(|f| sink(f));
    }
}

impl Transports {
    pub fn set_writer(&mut self, writer: Option<Writer>) {
        self.writer = writer;
    }

    pub fn set_frame_sink(&mut self, sink: Option<FrameSink>) {
        self.sink = sink;
    }

    pub fn open(&mut self, config: LinkConfig) -> Result<LinkId, String> {
        self.open_with_reserved(config, &[])
    }

    pub fn open_with_reserved(&mut self, config: LinkConfig, reserved_udp_ports: &[u16]) -> Result<LinkId, String> {
        if let Kind::Udp { local_port, .. } = &config.kind {
            if *local_port != 0 && reserved_udp_ports.contains(local_port) {
                return Err(format!("port {local_port} is already served by a Qt link; opening a core link there would starve it"));
            }
        }
        let kind_name = match &config.kind {
            Kind::Udp { .. } => "udp",
            Kind::Tcp { .. } => "tcp",
            #[cfg(not(target_os = "android"))]
            Kind::Serial { .. } => "serial",
            other => return Err(format!("the core does not own {other:?} links on this platform")),
        };
        let id = self.registry.lock().unwrap().open(Owner::Core, kind_name, &config.name);
        let registry = Arc::clone(&self.registry);
        let sink = self.sink.clone();
        let owned = match &config.kind {
            Kind::Udp { local_port, hosts } => UdpLink::open(&UdpConfig { local_port: *local_port, targets: hosts.clone() }, Default::default(), move |bytes| deliver(&registry, &sink, id, bytes)).map(Owned::Udp),
            Kind::Tcp { host, port } => TcpLink::open(&TcpConfig { host: host.clone(), port: *port }, move |event| match event {
                tcplink::Event::Bytes(bytes) => deliver(&registry, &sink, id, &bytes),
                tcplink::Event::Disconnected(reason) => {
                    registry.lock().unwrap().close(id, &reason);
                }
            })
            .map(Owned::Tcp),
            #[cfg(not(target_os = "android"))]
            Kind::Serial { baud, data_bits, flow_control, stop_bits, parity, port_name, .. } => crate::seriallink::SerialLink::open(
                &crate::seriallink::SerialConfig { port_name: port_name.clone(), baud: *baud as u32, data_bits: *data_bits, parity: *parity, stop_bits: *stop_bits, flow_control: *flow_control, usb_direct: false },
                move |event| match event {
                    crate::seriallink::Event::Bytes(bytes) => deliver(&registry, &sink, id, &bytes),
                    crate::seriallink::Event::Disconnected(reason) => {
                        registry.lock().unwrap().close(id, &reason);
                    }
                },
            )
            .map(Owned::Serial),
            _ => unreachable!(),
        };
        match owned {
            Ok(link) => {
                self.owned.insert(id, link);
                self.configs.insert(id, config);
                Ok(id)
            }
            Err(e) => {
                self.registry.lock().unwrap().close(id, &e.to_string());
                Err(e.to_string())
            }
        }
    }

    pub fn open_json(&mut self, json: &str, reserved_udp_ports: &[u16]) -> Result<LinkId, String> {
        let value: Value = serde_json::from_str(json).map_err(|e| format!("not JSON: {e}"))?;
        self.open_with_reserved(linkconfig::from_json(&value)?, reserved_udp_ports)
    }

    pub fn host_open(&mut self, kind: &str, name: &str) -> LinkId {
        self.registry.lock().unwrap().open(Owner::Host, kind, name)
    }

    pub fn host_bytes(&self, id: LinkId, bytes: &[u8]) {
        deliver(&self.registry, &self.sink, id, bytes)
    }

    pub fn host_closed(&mut self, id: LinkId, reason: &str) -> bool {
        self.registry.lock().unwrap().close(id, reason)
    }

    pub fn write(&self, id: LinkId, bytes: &[u8]) -> bool {
        let sent = match self.owned.get(&id) {
            Some(Owned::Udp(link)) => link.write(bytes) > 0,
            Some(Owned::Tcp(link)) => link.write(bytes).is_ok(),
            #[cfg(not(target_os = "android"))]
            Some(Owned::Serial(link)) => link.write(bytes).is_ok(),
            None => match &self.writer {
                Some(writer) if self.registry.lock().unwrap().entry(id).is_some_and(|e| e.owner == Owner::Host) => {
                    writer(id, bytes);
                    true
                }
                _ => false,
            },
        };
        sent && self.registry.lock().unwrap().wrote(id, bytes.len())
    }

    pub fn close(&mut self, id: LinkId, reason: &str) -> bool {
        match self.owned.remove(&id) {
            Some(Owned::Udp(mut link)) => link.close(),
            Some(Owned::Tcp(mut link)) => link.close(),
            #[cfg(not(target_os = "android"))]
            Some(Owned::Serial(mut link)) => link.close(),
            None => {}
        }
        self.configs.remove(&id);
        self.registry.lock().unwrap().close(id, reason)
    }

    pub fn snapshot(&self) -> Value {
        let mut snapshot = self.registry.lock().unwrap().snapshot();
        if let Some(links) = snapshot.get_mut("links").and_then(Value::as_array_mut) {
            links.iter_mut().for_each(|link| {
                let id = link.get("id").and_then(Value::as_u64).unwrap_or(0) as LinkId;
                let owner = link.get("owner").and_then(Value::as_str).unwrap_or("").to_string();
                let state = link.get("state").and_then(Value::as_str).unwrap_or("").to_string();
                let frames = link.get("framesIn").and_then(Value::as_u64).unwrap_or(0);
                link["summary"] = json!(format!("{} link, {owner}-owned, {state}, {frames} frames", link.get("kind").and_then(Value::as_str).unwrap_or("")));
                link["config"] = self.configs.get(&id).map(linkconfig::to_json).unwrap_or(Value::Null);
            });
        }
        snapshot
    }
}

pub fn qt_links(backend: &dyn crate::router::Backend) -> Vec<Value> {
    let model = crate::read::object(&backend.get("links.linkConfigurations"));
    model
        .get("elements")
        .and_then(Value::as_array)
        .map(|elements| {
            elements
                .iter()
                .enumerate()
                .map(|(i, element)| {
                    let link = crate::links::link_json(i, element);
                    let connected = link["connected"].as_bool().unwrap_or(false);
                    let state = if connected { "open" } else { "closed" };
                    json!({
                        "id": Value::Null,
                        "kind": link["type"].clone(),
                        "name": link["name"].clone(),
                        "owner": "qt",
                        "state": state,
                        "reason": "",
                        "path": link["path"].clone(),
                        "port": link["port"].clone(),
                        "summary": format!("{} link, qt-owned, {state}", link["type"].as_str().unwrap_or("")),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn qt_udp_ports(backend: &dyn crate::router::Backend) -> Vec<u16> {
    qt_links(backend)
        .iter()
        .filter(|l| l["owner"] == "qt" && l["kind"] == "udp" && l["state"] == "open")
        .filter_map(|l| l["port"].as_u64().and_then(|p| u16::try_from(p).ok()))
        .filter(|p| *p != 0)
        .collect()
}

pub fn transports_view(backend: &dyn crate::router::Backend, _args: &[String]) -> Value {
    let mut snapshot = TRANSPORTS.lock().unwrap().snapshot();
    let qt = qt_links(backend);
    snapshot["qtOpenCount"] = json!(qt.iter().filter(|l| l["state"] == "open").count());
    if let Some(links) = snapshot.get_mut("links").and_then(Value::as_array_mut) {
        links.extend(qt);
    }
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    fn heartbeat() -> Vec<u8> {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../mav.tlog")).unwrap();
        crate::tlog::entries(&bytes, u64::MAX).into_iter().map(|(_, f)| f).find(|f| f.len() == 21).unwrap()
    }

    fn wait_for(check: impl Fn() -> bool) -> bool {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            if check() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn a_core_udp_link_frames_what_a_peer_sends_and_answers_it() {
        let mut transports = Transports::default();
        let seen = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&seen);
        transports.set_frame_sink(Some(Arc::new(move |_| {
            counter.fetch_add(1, Ordering::Relaxed);
        })));
        let id = transports.open_json(r#"{"kind":"udp","name":"Loop","port":0,"hosts":[]}"#, &[]).unwrap();
        let port = match transports.owned.get(&id) {
            Some(Owned::Udp(link)) => link.local_port(),
            _ => panic!("not udp"),
        };
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let frame = heartbeat();
        peer.send_to(&frame, ("127.0.0.1", port)).unwrap();
        assert!(wait_for(|| seen.load(Ordering::Relaxed) == 1));
        assert!(transports.write(id, &frame));
        let mut back = [0u8; 64];
        assert_eq!(peer.recv_from(&mut back).unwrap().0, frame.len());
        let snapshot = transports.snapshot();
        let link = &snapshot["links"][0];
        assert_eq!((link["owner"].as_str(), link["framesIn"].as_u64(), link["bytesOut"].as_u64()), (Some("core"), Some(1), Some(21)));
        assert_eq!(link["config"]["kind"], "udp");
        assert!(link["summary"].as_str().unwrap().contains("core-owned"));
        assert!(transports.close(id, "test done"));
        assert!(!transports.write(id, &frame));
    }

    #[test]
    fn a_host_link_frames_pushed_bytes_and_writes_through_the_writer() {
        let mut transports = Transports::default();
        let written = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&written);
        transports.set_writer(Some(Arc::new(move |id, bytes| sink.lock().unwrap().push((id, bytes.to_vec())))));
        let id = transports.host_open("usb", "Pixhawk");
        let frame = heartbeat();
        transports.host_bytes(id, &frame[..10]);
        transports.host_bytes(id, &frame[10..]);
        assert!(transports.write(id, b"cmd"));
        assert_eq!(*written.lock().unwrap(), vec![(id, b"cmd".to_vec())]);
        let snapshot = transports.snapshot();
        assert_eq!((snapshot["links"][0]["owner"].as_str(), snapshot["links"][0]["framesIn"].as_u64()), (Some("host"), Some(1)));
        assert!(transports.host_closed(id, "unplugged"));
        assert!(!transports.write(id, b"x"));
        assert!(transports.open_json(r#"{"kind":"serial","name":"S","portName":"/dev/none"}"#, &[]).is_err());
        assert!(transports.open_json(r#"{"kind":"udp","name":"Clash","port":14550,"hosts":[]}"#, &[14550]).unwrap_err().contains("starve"));
        assert!(transports.open_json(r#"{"kind":"tcp","name":"T","host":"127.0.0.1","port":1}"#, &[]).is_err());
        assert_eq!(transports.snapshot()["links"].as_array().unwrap().len(), 2);
    }
}
