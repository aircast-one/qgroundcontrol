use std::ffi::{CStr, CString, c_char};
use std::sync::{LazyLock, Mutex};

use crate::router::{Backend, Core};

#[global_allocator]
static ALLOCATOR: std::alloc::System = std::alloc::System;

type EventFn = Option<unsafe extern "C" fn(*const c_char, *const c_char)>;

unsafe extern "C" {
    fn qgc_qt_get(path: *const c_char) -> *mut c_char;
    fn qgc_qt_get_fields(path: *const c_char, fields_csv: *const c_char) -> *mut c_char;
    fn qgc_qt_set(path: *const c_char, value_json: *const c_char) -> *mut c_char;
    fn qgc_qt_invoke(path: *const c_char, args_json: *const c_char) -> *mut c_char;
    fn qgc_qt_watch(paths_csv: *const c_char);
    fn qgc_qt_set_event_handler(handler: EventFn);
    fn qgc_qt_free(text: *mut c_char);
}

struct QtBackend;

impl Backend for QtBackend {
    fn get(&self, path: &str) -> String {
        take(unsafe { qgc_qt_get(c(path).as_ptr()) })
    }
    fn get_fields(&self, path: &str, fields: &str) -> String {
        take(unsafe { qgc_qt_get_fields(c(path).as_ptr(), c(fields).as_ptr()) })
    }
    fn set(&self, path: &str, value: &str) -> String {
        take(unsafe { qgc_qt_set(c(path).as_ptr(), c(value).as_ptr()) })
    }
    fn invoke(&self, path: &str, args: &str) -> String {
        take(unsafe { qgc_qt_invoke(c(path).as_ptr(), c(args).as_ptr()) })
    }
    fn watch(&self, paths: &[String]) {
        unsafe { qgc_qt_watch(c(&paths.join(",")).as_ptr()) }
    }
}

static CORE: LazyLock<Core<QtBackend>> = LazyLock::new(|| Core::new(QtBackend));
static HEAD: Mutex<EventFn> = Mutex::new(None);

fn c(text: &str) -> CString {
    CString::new(text).unwrap_or_default()
}

fn text(ptr: *const c_char) -> String {
    match ptr.is_null() {
        true => String::new(),
        false => unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned(),
    }
}

fn take(ptr: *mut c_char) -> String {
    let owned = text(ptr);
    unsafe { qgc_qt_free(ptr) };
    owned
}

fn give(text: String) -> *mut c_char {
    c(&text).into_raw()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_get(path: *const c_char) -> *mut c_char {
    give(CORE.get(&text(path)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_get_fields(path: *const c_char, fields_csv: *const c_char) -> *mut c_char {
    give(CORE.get_fields(&text(path), &text(fields_csv)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_set(path: *const c_char, value_json: *const c_char) -> *mut c_char {
    give(CORE.set(&text(path), &text(value_json)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_invoke(path: *const c_char, args_json: *const c_char) -> *mut c_char {
    give(CORE.invoke(&text(path), &text(args_json)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_watch(paths_csv: *const c_char) {
    CORE.watch("", &split(paths_csv))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_watch_client(client: *const c_char, paths_csv: *const c_char) {
    CORE.watch(&text(client), &split(paths_csv))
}

fn split(paths_csv: *const c_char) -> Vec<String> {
    crate::view::split_paths(&text(paths_csv))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_set_event_handler(handler: EventFn) {
    *HEAD.lock().unwrap() = handler;
    *crate::detections::ON_CHANGE.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = handler.map(|_| std::sync::Arc::new(announce_detections) as std::sync::Arc<dyn Fn() + Send + Sync>);
    if handler.is_some() {
        start_pump();
    }
    unsafe { qgc_qt_set_event_handler(handler.map(|_| relay as unsafe extern "C" fn(*const c_char, *const c_char))) }
}

unsafe extern "C" fn relay(path: *const c_char, json: *const c_char) {
    let events = CORE.on_event(&text(path), &text(json));
    let Some(handler) = *HEAD.lock().unwrap() else { return };
    events.iter().for_each(|(p, j)| {
        let (p, j) = (c(p), c(j));
        unsafe { handler(p.as_ptr(), j.as_ptr()) }
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_free(text: *mut c_char) {
    if !text.is_null() {
        drop(unsafe { CString::from_raw(text) });
    }
}

type LinkWriterFn = Option<unsafe extern "C" fn(u32, *const u8, usize, *mut std::ffi::c_void)>;

struct WriterUser(*mut std::ffi::c_void);
unsafe impl Send for WriterUser {}
unsafe impl Sync for WriterUser {}

impl WriterUser {
    fn ptr(&self) -> *mut std::ffi::c_void {
        self.0
    }
}

fn outcome(result: Result<u32, String>) -> *mut c_char {
    give(match result {
        Ok(id) => serde_json::json!({ "ok": true, "id": id }).to_string(),
        Err(reason) => serde_json::json!({ "ok": false, "reason": reason }).to_string(),
    })
}

static HUB_SINK: std::sync::OnceLock<()> = std::sync::OnceLock::new();
static PUMP: std::sync::OnceLock<()> = std::sync::OnceLock::new();
const PUMP_PERIOD: std::time::Duration = std::time::Duration::from_millis(100);

fn deliver(outbound: Vec<(u32, Vec<u8>)>) {
    outbound.iter().for_each(|(link, bytes)| {
        crate::linkhost::write(&crate::linkhost::TRANSPORTS, *link, bytes);
    });
}

static GUIDED_ANNOUNCED: Mutex<String> = Mutex::new(String::new());

fn announce_guided() {
    if HEAD.lock().unwrap().is_none() {
        return;
    }
    let snapshot = crate::hub::lock().guided_snapshot(None).to_string();
    let changed = {
        let mut last = GUIDED_ANNOUNCED.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let changed = *last != snapshot;
        *last = snapshot.clone();
        changed
    };
    if changed {
        announce("view.coreGuided", &snapshot);
    }
}

fn start_pump() {
    PUMP.get_or_init(|| {
        std::thread::Builder::new()
            .name("qgc-core-pump".to_string())
            .spawn(|| loop {
                std::thread::sleep(PUMP_PERIOD);
                let open = crate::linkhost::TRANSPORTS.lock().unwrap().open_ids();
                let outbound = {
                    let mut hub = crate::hub::lock();
                    hub.retain_links(&open);
                    hub.expire(crate::hub::now_us());
                    hub.tick_with(crate::hub::now_ms(), crate::hub::now_us() / 1_000_000)
                };
                deliver(outbound);
                announce_guided();
                if crate::detections::lock().went_stale(crate::hub::now_ms()) {
                    announce_detections();
                }
            })
            .expect("pump thread");
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_parameter(request_json: *const c_char) -> *mut c_char {
    let request: serde_json::Value = serde_json::from_str(&text(request_json)).unwrap_or(serde_json::Value::Null);
    let vehicle = request.get("vehicle").and_then(serde_json::Value::as_u64).map(|id| id as u8);
    let sent = crate::hub::lock().parameter_request(vehicle, &request, crate::hub::now_ms());
    give(match sent {
        Ok(outbound) => {
            deliver(outbound);
            serde_json::json!({ "ok": true }).to_string()
        }
        Err(reason) => serde_json::json!({ "ok": false, "reason": reason }).to_string(),
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_remote_id(request_json: *const c_char) -> *mut c_char {
    let request: serde_json::Value = serde_json::from_str(&text(request_json)).unwrap_or(serde_json::Value::Null);
    let vehicle = request.get("vehicle").and_then(serde_json::Value::as_u64).map(|id| id as u8);
    give(match crate::hub::lock().remote_request(vehicle, &request) {
        Ok(()) => serde_json::json!({ "ok": true }).to_string(),
        Err(reason) => serde_json::json!({ "ok": false, "reason": reason }).to_string(),
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_mission(request_json: *const c_char) -> *mut c_char {
    let request: serde_json::Value = serde_json::from_str(&text(request_json)).unwrap_or(serde_json::Value::Null);
    let vehicle = request.get("vehicle").and_then(serde_json::Value::as_u64).map(|id| id as u8);
    let sent = crate::hub::lock().mission_request(vehicle, &request, crate::hub::now_ms());
    give(match sent {
        Ok(outbound) => {
            deliver(outbound);
            serde_json::json!({ "ok": true }).to_string()
        }
        Err(reason) => serde_json::json!({ "ok": false, "reason": reason }).to_string(),
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_guided(action_json: *const c_char) -> *mut c_char {
    let action: serde_json::Value = serde_json::from_str(&text(action_json)).unwrap_or(serde_json::Value::Null);
    let vehicle = action.get("vehicle").and_then(serde_json::Value::as_u64).map(|id| id as u8);
    let started = crate::hub::lock().guided(vehicle, &action, crate::hub::now_ms());
    give(match started {
        Ok(outbound) => {
            start_pump();
            deliver(outbound);
            announce_guided();
            serde_json::json!({ "ok": true }).to_string()
        }
        Err(reason) => serde_json::json!({ "ok": false, "reason": reason }).to_string(),
    })
}

fn install_hub_sink() {
    HUB_SINK.get_or_init(|| {
        let sink: crate::linkhost::FrameSink = std::sync::Arc::new(|frame: &crate::transport::Frame| {
            let outbound = crate::hub::lock().on_frame(crate::hub::Origin { link: frame.link, replay: frame.replay, v2: frame.v2 }, &frame.header, &frame.message, crate::hub::now_us(), crate::hub::now_ms());
            deliver(outbound);
        });
        crate::linkhost::TRANSPORTS.lock().unwrap().set_frame_sink(Some(sink));
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_link_open(config_json: *const c_char) -> *mut c_char {
    install_hub_sink();
    let reserved = crate::linkhost::qt_udp_ports(&QtBackend);
    outcome(crate::linkhost::open_json(&crate::linkhost::TRANSPORTS, &text(config_json), &reserved))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_link_close(id: u32, reason: *const c_char) -> bool {
    let closed = crate::linkhost::close(&crate::linkhost::TRANSPORTS, id, &text(reason));
    crate::hub::lock().link_closed(id);
    closed
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_link_write(id: u32, bytes: *const u8, len: usize) -> bool {
    if bytes.is_null() || len == 0 {
        return false;
    }
    let data = unsafe { std::slice::from_raw_parts(bytes, len) };
    crate::linkhost::write(&crate::linkhost::TRANSPORTS, id, data)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_host_link_open(kind: *const c_char, name: *const c_char) -> *mut c_char {
    install_hub_sink();
    outcome(Ok(crate::linkhost::host_open(&crate::linkhost::TRANSPORTS, &text(kind), &text(name))))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_host_link_bytes(id: u32, bytes: *const u8, len: usize) {
    if bytes.is_null() || len == 0 {
        return;
    }
    let copied = unsafe { std::slice::from_raw_parts(bytes, len) }.to_vec();
    crate::linkhost::host_bytes(&crate::linkhost::TRANSPORTS, id, &copied)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_host_link_closed(id: u32, reason: *const c_char) -> bool {
    let closed = crate::linkhost::host_closed(&crate::linkhost::TRANSPORTS, id, &text(reason));
    crate::hub::lock().link_closed(id);
    closed
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_set_link_writer(writer: LinkWriterFn, user: *mut std::ffi::c_void) {
    let user = WriterUser(user);
    let boxed: Option<crate::linkhost::Writer> = writer.map(|w| {
        let holder = user;
        std::sync::Arc::new(move |id: u32, bytes: &[u8]| unsafe { w(id, bytes.as_ptr(), bytes.len(), holder.ptr()) }) as crate::linkhost::Writer
    });
    crate::linkhost::TRANSPORTS.lock().unwrap().set_writer(boxed);
}

fn announce(path: &str, json: &str) {
    let handler = *HEAD.lock().unwrap();
    if let Some(handler) = handler {
        let (path, json) = (c(path), c(json));
        unsafe { handler(path.as_ptr(), json.as_ptr()) };
    }
}

fn announce_transports() {
    let snapshot = crate::linkhost::TRANSPORTS.lock().unwrap().snapshot().to_string();
    announce("view.transports", &snapshot);
}

fn announce_detections() {
    let snapshot = crate::detections::lock().snapshot(crate::hub::now_ms()).to_string();
    announce("view.detections", &snapshot);
}

type LinkBytesSinkFn = Option<unsafe extern "C" fn(u32, *const u8, usize, *mut std::ffi::c_void)>;
type LinkStateSinkFn = Option<unsafe extern "C" fn(u32, bool, *const c_char, *mut std::ffi::c_void)>;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_set_link_bytes_sink(sink: LinkBytesSinkFn, user: *mut std::ffi::c_void) {
    let holder = WriterUser(user);
    let boxed: Option<crate::linkhost::BytesSink> = sink.map(|f| std::sync::Arc::new(move |id: u32, bytes: &[u8]| unsafe { f(id, bytes.as_ptr(), bytes.len(), holder.ptr()) }) as crate::linkhost::BytesSink);
    crate::linkhost::TRANSPORTS.lock().unwrap().set_bytes_sink(boxed);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_set_link_state_sink(sink: LinkStateSinkFn, user: *mut std::ffi::c_void) {
    let holder = WriterUser(user);
    let boxed: Option<crate::linkhost::StateSink> = sink.map(|f| {
        std::sync::Arc::new(move |id: u32, open: bool, reason: &str| {
            let reason = c(reason);
            unsafe { f(id, open, reason.as_ptr(), holder.ptr()) }
        }) as crate::linkhost::StateSink
    });
    crate::linkhost::TRANSPORTS.lock().unwrap().set_state_sink(boxed);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_core_link_announce_on_state() {
    crate::linkhost::TRANSPORTS.lock().unwrap().set_state_hook(Some(std::sync::Arc::new(announce_transports)));
}
