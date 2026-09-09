use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock, Mutex};

use crate::linkconfig::{self, Kind, LinkConfig};
use crate::tcplink::{self, TcpConfig, TcpLink};
use crate::transport::{Frame, LinkId, Owner, Registry, State};
use crate::udplink::{UdpConfig, UdpLink};

pub type Writer = Arc<dyn Fn(LinkId, &[u8]) + Send + Sync>;
pub type FrameSink = Arc<dyn Fn(&Frame) + Send + Sync>;
pub type StateHook = Arc<dyn Fn() + Send + Sync>;
const KEEP_CLOSED: usize = 16;

pub enum Owned {
    Udp(UdpLink),
    Tcp(TcpLink),
    #[cfg(not(target_os = "android"))]
    Serial(crate::seriallink::SerialLink),
}

impl Owned {
    fn write(&self, bytes: &[u8]) -> bool {
        match self {
            Owned::Udp(link) => link.write(bytes) > 0,
            Owned::Tcp(link) => link.write(bytes).is_ok(),
            #[cfg(not(target_os = "android"))]
            Owned::Serial(link) => link.write(bytes).is_ok(),
        }
    }

    fn close(&self) {
        match self {
            Owned::Udp(link) => link.close(),
            Owned::Tcp(link) => link.close(),
            #[cfg(not(target_os = "android"))]
            Owned::Serial(link) => link.close(),
        }
    }
}

#[derive(Clone, Default)]
pub struct Shared {
    registry: Arc<Mutex<Registry>>,
    sink: Arc<Mutex<Option<FrameSink>>>,
    state_hook: Arc<Mutex<Option<StateHook>>>,
}

impl Shared {
    fn deliver(&self, id: LinkId, bytes: &[u8]) {
        let frames = self.registry.lock().unwrap().bytes_in(id, bytes);
        let sink = self.sink.lock().unwrap().clone();
        if let Some(sink) = sink {
            frames.iter().for_each(|f| sink(f));
        }
    }

    fn closed_by_reader(&self, id: LinkId, reason: &str) {
        self.registry.lock().unwrap().close(id, reason);
        let hook = self.state_hook.lock().unwrap().clone();
        if let Some(hook) = hook {
            hook();
        }
    }
}

#[derive(Default)]
pub struct Transports {
    shared: Shared,
    owned: BTreeMap<LinkId, Arc<Owned>>,
    configs: BTreeMap<LinkId, LinkConfig>,
    writer: Option<Writer>,
}

pub static TRANSPORTS: LazyLock<Mutex<Transports>> = LazyLock::new(|| Mutex::new(Transports::default()));

impl Transports {
    pub fn set_writer(&mut self, writer: Option<Writer>) {
        self.writer = writer;
    }

    pub fn set_frame_sink(&mut self, sink: Option<FrameSink>) {
        *self.shared.sink.lock().unwrap() = sink;
    }

    pub fn set_state_hook(&mut self, hook: Option<StateHook>) {
        *self.shared.state_hook.lock().unwrap() = hook;
    }

    fn reap(&mut self) {
        let dead: Vec<LinkId> = {
            let registry = self.shared.registry.lock().unwrap();
            self.owned.keys().copied().filter(|id| registry.entry(*id).is_none_or(|e| e.state == State::Closed)).collect()
        };
        let links: Vec<Arc<Owned>> = dead.iter().filter_map(|id| self.owned.remove(id)).collect();
        dead.iter().for_each(|id| {
            self.configs.remove(id);
        });
        links.iter().for_each(|l| l.close());
        self.shared.registry.lock().unwrap().prune_closed(KEEP_CLOSED);
    }

    pub fn host_open(&mut self, kind: &str, name: &str) -> LinkId {
        self.reap();
        self.shared.registry.lock().unwrap().open(Owner::Host, kind, name)
    }

    pub fn host_closed(&mut self, id: LinkId, reason: &str) -> bool {
        let closed = self.shared.registry.lock().unwrap().close(id, reason);
        self.reap();
        closed
    }

    pub fn snapshot(&self) -> Value {
        let mut snapshot = self.shared.registry.lock().unwrap().snapshot();
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

fn build(shared: &Shared, id: LinkId, config: &LinkConfig) -> Result<Owned, String> {
    match &config.kind {
        Kind::Udp { local_port, hosts } => {
            let shared = shared.clone();
            UdpLink::open(&UdpConfig { local_port: *local_port, targets: hosts.clone() }, crate::udplink::local_addresses(), move |bytes| shared.deliver(id, bytes)).map(Owned::Udp).map_err(|e| e.to_string())
        }
        Kind::Tcp { host, port } => {
            let shared = shared.clone();
            TcpLink::open(&TcpConfig { host: host.clone(), port: *port }, move |event| match event {
                tcplink::Event::Bytes(bytes) => shared.deliver(id, &bytes),
                tcplink::Event::Disconnected(reason) => shared.closed_by_reader(id, &reason),
            })
            .map(Owned::Tcp)
            .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        Kind::Serial { baud, data_bits, flow_control, stop_bits, parity, port_name, .. } => {
            let shared = shared.clone();
            crate::seriallink::SerialLink::open(
                &crate::seriallink::SerialConfig { port_name: port_name.clone(), baud: *baud as u32, data_bits: *data_bits, parity: *parity, stop_bits: *stop_bits, flow_control: *flow_control, usb_direct: false },
                move |event| match event {
                    crate::seriallink::Event::Bytes(bytes) => shared.deliver(id, &bytes),
                    crate::seriallink::Event::Disconnected(reason) => shared.closed_by_reader(id, &reason),
                },
            )
            .map(Owned::Serial)
            .map_err(|e| e.to_string())
        }
        other => Err(format!("the core does not own {other:?} links on this platform")),
    }
}

fn kind_name(kind: &Kind) -> &'static str {
    match kind {
        Kind::Udp { .. } => "udp",
        Kind::Tcp { .. } => "tcp",
        Kind::Serial { .. } => "serial",
        Kind::Bluetooth { .. } => "bluetooth",
        Kind::Mock { .. } => "mock",
        Kind::AirLink => "airlink",
        Kind::LogReplay { .. } => "logReplay",
    }
}

pub fn open(transports: &Mutex<Transports>, config: LinkConfig, reserved_udp_ports: &[u16]) -> Result<LinkId, String> {
    if let Kind::Udp { local_port, .. } = &config.kind {
        if *local_port != 0 && reserved_udp_ports.contains(local_port) {
            return Err(format!("port {local_port} is already served by a Qt link; opening a core link there would starve it"));
        }
    }
    let (shared, id) = {
        let mut guard = transports.lock().unwrap();
        guard.reap();
        let id = guard.shared.registry.lock().unwrap().open(Owner::Core, kind_name(&config.kind), &config.name);
        (guard.shared.clone(), id)
    };
    match build(&shared, id, &config) {
        Ok(link) => {
            let mut guard = transports.lock().unwrap();
            guard.owned.insert(id, Arc::new(link));
            guard.configs.insert(id, config);
            Ok(id)
        }
        Err(reason) => {
            shared.registry.lock().unwrap().remove(id);
            Err(reason)
        }
    }
}

pub fn open_json(transports: &Mutex<Transports>, json: &str, reserved_udp_ports: &[u16]) -> Result<LinkId, String> {
    let value: Value = serde_json::from_str(json).map_err(|e| format!("not JSON: {e}"))?;
    open(transports, linkconfig::from_json(&value)?, reserved_udp_ports)
}

pub fn host_bytes(transports: &Mutex<Transports>, id: LinkId, bytes: &[u8]) {
    let shared = transports.lock().unwrap().shared.clone();
    shared.deliver(id, bytes)
}

pub fn write(transports: &Mutex<Transports>, id: LinkId, bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let (owned, writer, shared) = {
        let guard = transports.lock().unwrap();
        (guard.owned.get(&id).cloned(), guard.writer.clone(), guard.shared.clone())
    };
    let open = shared.registry.lock().unwrap().entry(id).is_some_and(|e| e.state == State::Open);
    if !open {
        return false;
    }
    let sent = match (owned, writer) {
        (Some(link), _) => link.write(bytes),
        (None, Some(writer)) => {
            let host_owned = shared.registry.lock().unwrap().entry(id).is_some_and(|e| e.owner == Owner::Host);
            host_owned && {
                writer(id, bytes);
                true
            }
        }
        (None, None) => false,
    };
    sent && shared.registry.lock().unwrap().wrote(id, bytes.len())
}

pub fn close(transports: &Mutex<Transports>, id: LinkId, reason: &str) -> bool {
    let (owned, shared) = {
        let mut guard = transports.lock().unwrap();
        guard.configs.remove(&id);
        (guard.owned.remove(&id), guard.shared.clone())
    };
    if let Some(link) = owned {
        link.close();
    }
    let closed = shared.registry.lock().unwrap().close(id, reason);
    transports.lock().unwrap().reap();
    closed
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

    fn local_port(transports: &Mutex<Transports>, id: LinkId) -> u16 {
        match transports.lock().unwrap().owned.get(&id).map(Arc::clone) {
            Some(link) => match &*link {
                Owned::Udp(udp) => udp.local_port(),
                _ => panic!("not udp"),
            },
            None => panic!("no link"),
        }
    }

    #[test]
    fn a_core_udp_link_frames_what_a_peer_sends_and_answers_it() {
        let transports = Mutex::new(Transports::default());
        let seen = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&seen);
        let id = open_json(&transports, r#"{"kind":"udp","name":"Loop","port":0,"hosts":[]}"#, &[]).unwrap();
        transports.lock().unwrap().set_frame_sink(Some(Arc::new(move |_| {
            counter.fetch_add(1, Ordering::Relaxed);
        })));
        let port = local_port(&transports, id);
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let frame = heartbeat();
        peer.send_to(&frame, ("127.0.0.1", port)).unwrap();
        assert!(wait_for(|| seen.load(Ordering::Relaxed) == 1), "a sink set after open still receives frames");
        assert!(write(&transports, id, &frame));
        let mut back = [0u8; 64];
        assert_eq!(peer.recv_from(&mut back).unwrap().0, frame.len());
        let snapshot = transports.lock().unwrap().snapshot();
        let link = &snapshot["links"][0];
        assert_eq!((link["owner"].as_str(), link["framesIn"].as_u64(), link["bytesOut"].as_u64()), (Some("core"), Some(1), Some(21)));
        assert_eq!(link["config"]["kind"], "udp");
        assert!(link["summary"].as_str().unwrap().contains("core-owned"));
        assert!(close(&transports, id, "test done"));
        assert!(!write(&transports, id, &frame));
        assert!(!write(&transports, id, &[]));
    }

    #[test]
    fn a_host_link_frames_pushed_bytes_and_writes_through_the_writer() {
        let transports = Mutex::new(Transports::default());
        let written = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&written);
        transports.lock().unwrap().set_writer(Some(Arc::new(move |id, bytes| sink.lock().unwrap().push((id, bytes.to_vec())))));
        let id = transports.lock().unwrap().host_open("usb", "Pixhawk");
        let frame = heartbeat();
        host_bytes(&transports, id, &frame[..10]);
        host_bytes(&transports, id, &frame[10..]);
        assert!(write(&transports, id, b"cmd"));
        assert_eq!(*written.lock().unwrap(), vec![(id, b"cmd".to_vec())]);
        let snapshot = transports.lock().unwrap().snapshot();
        assert_eq!((snapshot["links"][0]["owner"].as_str(), snapshot["links"][0]["framesIn"].as_u64()), (Some("host"), Some(1)));
        assert!(transports.lock().unwrap().host_closed(id, "unplugged"));
        assert!(!write(&transports, id, b"x"));
        assert_eq!(written.lock().unwrap().len(), 1);
        assert!(open_json(&transports, r#"{"kind":"serial","name":"S","portName":"/dev/none"}"#, &[]).is_err());
        assert!(open_json(&transports, r#"{"kind":"udp","name":"Clash","port":14550,"hosts":[]}"#, &[14550]).unwrap_err().contains("starve"));
        assert!(open_json(&transports, r#"{"kind":"tcp","name":"T","host":"127.0.0.1","port":1}"#, &[]).is_err());
        assert_eq!(transports.lock().unwrap().snapshot()["links"].as_array().unwrap().len(), 1, "failed opens leave no entry");
    }

    #[test]
    fn a_peer_closing_a_tcp_link_reaps_it_and_fires_the_hook() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let transports = Mutex::new(Transports::default());
        let fired = Arc::new(AtomicUsize::new(0));
        let hook = Arc::clone(&fired);
        transports.lock().unwrap().set_state_hook(Some(Arc::new(move || {
            hook.fetch_add(1, Ordering::Relaxed);
        })));
        let id = open_json(&transports, &format!(r#"{{"kind":"tcp","name":"SITL","host":"127.0.0.1","port":{port}}}"#), &[]).unwrap();
        let (socket, _) = listener.accept().unwrap();
        drop(socket);
        assert!(wait_for(|| fired.load(Ordering::Relaxed) == 1));
        assert!(!write(&transports, id, b"x"));
        let snapshot = transports.lock().unwrap().snapshot();
        assert_eq!(snapshot["links"][0]["state"], "closed");
        transports.lock().unwrap().host_open("usb", "next");
        assert!(transports.lock().unwrap().owned.is_empty(), "the dead link was reaped");
    }
}
