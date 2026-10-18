use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

pub struct Hooks {
    pub open: fn(u32, &str) -> bool,
    pub send: fn(u32, &[u8]) -> bool,
    pub close: fn(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Opened,
    Bytes(Vec<u8>),
    Closed(String),
}

type Receiver = Arc<dyn Fn(Event) + Send + Sync>;

static HOOKS: OnceLock<Hooks> = OnceLock::new();
static RECEIVERS: Mutex<BTreeMap<u32, Receiver>> = Mutex::new(BTreeMap::new());
static NEXT: AtomicU32 = AtomicU32::new(1);

pub fn install(hooks: Hooks) {
    let _ = HOOKS.set(hooks);
}

fn receiver(id: u32) -> Option<Receiver> {
    RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).get(&id).cloned()
}

pub fn opened(id: u32) {
    if let Some(deliver) = receiver(id) {
        deliver(Event::Opened);
    }
}

pub fn received(id: u32, bytes: Vec<u8>) {
    if let Some(deliver) = receiver(id) {
        deliver(Event::Bytes(bytes));
    }
}

pub fn closed(id: u32, reason: &str) {
    let taken = RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).remove(&id);
    if let Some(deliver) = taken {
        deliver(Event::Closed(reason.to_string()));
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct HostSocket {
    id: u32,
}

impl HostSocket {
    pub fn open(url: &str, on_event: impl Fn(Event) + Send + Sync + 'static) -> Result<HostSocket, String> {
        let hooks = HOOKS.get().ok_or_else(|| crate::device::NEEDS_DEVICE.to_string())?;
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).insert(id, Arc::new(on_event));
        match (hooks.open)(id, url) {
            true => Ok(HostSocket { id }),
            false => {
                RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).remove(&id);
                Err(format!("Could not open {url}"))
            }
        }
    }

    pub fn send(&self, bytes: &[u8]) -> bool {
        HOOKS.get().is_some_and(|hooks| (hooks.send)(self.id, bytes))
    }
}

impl Drop for HostSocket {
    fn drop(&mut self) {
        RECEIVERS.lock().unwrap_or_else(PoisonError::into_inner).remove(&self.id);
        if let Some(hooks) = HOOKS.get() {
            (hooks.close)(self.id);
        }
    }
}

const FOLLOW_STEP: std::time::Duration = std::time::Duration::from_millis(200);

pub struct Follower {
    retry: std::time::Duration,
    url: Box<dyn Fn() -> Option<String> + Send>,
    current: Box<dyn Fn() -> bool + Send>,
    on_event: Receiver,
    down: Arc<std::sync::atomic::AtomicBool>,
    socket: Option<HostSocket>,
    retry_at: Option<web_time::Instant>,
}

impl Follower {
    pub fn new(
        retry: std::time::Duration,
        url: impl Fn() -> Option<String> + Send + 'static,
        current: impl Fn() -> bool + Send + 'static,
        on_event: impl Fn(Event) + Send + Sync + 'static,
    ) -> Follower {
        Follower { retry, url: Box::new(url), current: Box::new(current), on_event: Arc::new(on_event), down: Arc::new(std::sync::atomic::AtomicBool::new(true)), socket: None, retry_at: None }
    }

    pub fn step(&mut self, now: web_time::Instant) -> bool {
        if !(self.current)() {
            self.socket = None;
            return true;
        }
        if self.down.load(Ordering::Relaxed) && self.retry_at.is_none_or(|at| now >= at) {
            self.retry_at = Some(now + self.retry);
            self.socket = None;
            match self.open() {
                Ok(socket) => self.socket = Some(socket),
                Err(reason) => (self.on_event)(Event::Closed(reason)),
            }
        }
        false
    }

    pub fn send(&self, bytes: &[u8]) -> bool {
        self.socket.as_ref().is_some_and(|socket| socket.send(bytes))
    }

    fn open(&self) -> Result<HostSocket, String> {
        let url = (self.url)().ok_or_else(|| crate::device::NEEDS_DEVICE.to_string())?;
        self.down.store(false, Ordering::Relaxed);
        let (down, on_event) = (self.down.clone(), self.on_event.clone());
        HostSocket::open(&url, move |event| {
            if matches!(event, Event::Closed(_)) {
                down.store(true, Ordering::Relaxed);
            }
            on_event(event);
        })
        .inspect_err(|_| self.down.store(true, Ordering::Relaxed))
    }
}

pub fn follow(name: &str, mut follower: Follower) -> std::io::Result<()> {
    crate::background::poll(name, FOLLOW_STEP, move || follower.step(web_time::Instant::now()))
}

pub fn lines(pending: &mut Vec<u8>, bytes: &[u8], limit: usize) -> Vec<String> {
    pending.extend_from_slice(bytes);
    let complete = pending.iter().rposition(|byte| *byte == b'\n').map(|end| pending.drain(..=end).collect::<Vec<u8>>()).unwrap_or_default();
    if pending.len() > limit {
        pending.clear();
    }
    String::from_utf8_lossy(&complete).lines().map(str::to_string).collect()
}

#[cfg(test)]
pub(crate) mod fake {
    use super::*;

    pub static OPENED: Mutex<Vec<(u32, String)>> = Mutex::new(Vec::new());
    pub static SENT: Mutex<Vec<(u32, Vec<u8>)>> = Mutex::new(Vec::new());
    pub static CLOSED: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    pub static SERIAL: Mutex<()> = Mutex::new(());

    pub fn install() -> std::sync::MutexGuard<'static, ()> {
        let guard = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
        super::install(Hooks {
            open: |id, url| {
                OPENED.lock().unwrap_or_else(PoisonError::into_inner).push((id, url.to_string()));
                true
            },
            send: |id, bytes| {
                SENT.lock().unwrap_or_else(PoisonError::into_inner).push((id, bytes.to_vec()));
                true
            },
            close: |id| CLOSED.lock().unwrap_or_else(PoisonError::into_inner).push(id),
        });
        OPENED.lock().unwrap_or_else(PoisonError::into_inner).clear();
        SENT.lock().unwrap_or_else(PoisonError::into_inner).clear();
        CLOSED.lock().unwrap_or_else(PoisonError::into_inner).clear();
        guard
    }

    pub fn last_opened() -> Option<(u32, String)> {
        OPENED.lock().unwrap_or_else(PoisonError::into_inner).last().cloned()
    }

    pub fn sent_to(id: u32) -> Vec<u8> {
        SENT.lock().unwrap_or_else(PoisonError::into_inner).iter().filter(|(to, _)| *to == id).flat_map(|(_, bytes)| bytes.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_socket_hears_its_host_until_it_is_dropped() {
        let _host = fake::install();
        let heard = Arc::new(Mutex::new(Vec::new()));
        let log = heard.clone();
        let socket = HostSocket::open("ws://falcon.local/fly/socket?tcp=h%3A1", move |event| log.lock().unwrap().push(event)).unwrap();
        let (id, url) = fake::last_opened().unwrap();
        assert_eq!(url, "ws://falcon.local/fly/socket?tcp=h%3A1");
        opened(id);
        received(id, b"$GPGGA".to_vec());
        assert!(socket.send(b"GET /"));
        assert_eq!(fake::sent_to(id), b"GET /");
        drop(socket);
        received(id, b"late".to_vec());
        assert_eq!(*heard.lock().unwrap(), vec![Event::Opened, Event::Bytes(b"$GPGGA".to_vec())], "nothing reaches a dropped socket");
        assert!(fake::CLOSED.lock().unwrap().contains(&id));
    }

    #[test]
    fn the_host_closing_a_socket_tells_its_owner_why_once() {
        let _host = fake::install();
        let heard = Arc::new(Mutex::new(Vec::new()));
        let log = heard.clone();
        let _socket = HostSocket::open("ws://x/fly/socket?udp=14550", move |event| log.lock().unwrap().push(event)).unwrap();
        let (id, _) = fake::last_opened().unwrap();
        closed(id, "listen udp :14550: address already in use");
        closed(id, "again");
        assert_eq!(*heard.lock().unwrap(), vec![Event::Closed("listen udp :14550: address already in use".into())]);
    }

    #[test]
    fn lines_come_out_whole_however_the_bytes_were_split() {
        let mut pending = Vec::new();
        assert!(lines(&mut pending, b"MSG,3,1,1,4CA2", 64).is_empty());
        assert_eq!(lines(&mut pending, b"D5,1\r\nMSG,4\nMSG", 64), vec!["MSG,3,1,1,4CA2D5,1".to_string(), "MSG,4".to_string()]);
        assert_eq!(pending, b"MSG");
        assert!(lines(&mut pending, &[b'x'; 80], 64).is_empty() && pending.is_empty(), "a line longer than the limit is dropped");
    }

    #[test]
    fn a_follower_reopens_after_its_host_drops_it_and_stops_once_it_is_no_longer_wanted() {
        let _host = fake::install();
        let wanted = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let still = wanted.clone();
        let heard = Arc::new(Mutex::new(Vec::new()));
        let log = heard.clone();
        let mut follower = Follower::new(std::time::Duration::from_secs(5), || Some("ws://d/fly/socket?tcp=h%3A1".into()), move || still.load(Ordering::Relaxed), move |event| log.lock().unwrap().push(event));
        let start = web_time::Instant::now();
        assert!(!follower.step(start));
        let (first, _) = fake::last_opened().unwrap();
        closed(first, "connection refused");
        assert!(!follower.step(start + std::time::Duration::from_secs(1)));
        assert_eq!(fake::last_opened().unwrap().0, first, "it waits out the retry before dialling again");
        assert!(!follower.step(start + std::time::Duration::from_secs(5)));
        let (second, _) = fake::last_opened().unwrap();
        assert_ne!(second, first);
        assert!(follower.send(b"x") && fake::sent_to(second) == b"x");
        wanted.store(false, Ordering::Relaxed);
        assert!(follower.step(start + std::time::Duration::from_secs(6)), "a follower nobody wants is done");
        assert!(fake::CLOSED.lock().unwrap().contains(&second));
        assert_eq!(*heard.lock().unwrap(), vec![Event::Closed("connection refused".into())]);
    }

    #[test]
    fn a_follower_without_a_device_says_so_and_keeps_waiting_for_one() {
        let _host = fake::install();
        let paired = Arc::new(Mutex::new(None::<String>));
        let url = paired.clone();
        let heard = Arc::new(Mutex::new(Vec::new()));
        let log = heard.clone();
        let mut follower = Follower::new(std::time::Duration::from_secs(5), move || url.lock().unwrap().clone(), || true, move |event| log.lock().unwrap().push(event));
        let start = web_time::Instant::now();
        follower.step(start);
        assert_eq!(*heard.lock().unwrap(), vec![Event::Closed(crate::device::NEEDS_DEVICE.into())]);
        *paired.lock().unwrap() = Some("ws://d/fly/socket?udp=14550".into());
        follower.step(start + std::time::Duration::from_secs(5));
        assert_eq!(fake::last_opened().unwrap().1, "ws://d/fly/socket?udp=14550", "pairing later is picked up on the next retry");
    }
}
