use std::cell::{Cell, RefCell};
use std::ffi::{CStr, CString, c_char, c_void};

use wasm_bindgen::prelude::*;

const HUB_PERIOD_MS: u64 = 100;

thread_local! {
    static EVENTS: RefCell<Option<js_sys::Function>> = const { RefCell::new(None) };
    static WRITER: RefCell<Option<js_sys::Function>> = const { RefCell::new(None) };
    static LAST_HUB_MS: Cell<u64> = const { Cell::new(0) };
    static SERIAL_HOST: RefCell<Option<js_sys::Object>> = const { RefCell::new(None) };
    static SERIAL_PORTS: RefCell<Vec<crate::boards::PortInfo>> = const { RefCell::new(Vec::new()) };
    static TILES: Option<crate::tilecache::Cache> = crate::tilecache::Cache::open_in_memory().ok();
}

fn c(text: &str) -> CString {
    CString::new(text).unwrap_or_default()
}

fn read(raw: *const c_char) -> String {
    unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned()
}

fn taken(raw: *mut c_char) -> String {
    let owned = read(raw);
    unsafe { crate::abi::qgc_core_free(raw) };
    owned
}

#[wasm_bindgen(start)]
fn start() {
    std::panic::set_hook(Box::new(|info| web_sys::console::error_1(&info.to_string().into())));
    crate::applog::install();
}

#[wasm_bindgen]
pub fn get(path: &str) -> String {
    taken(unsafe { crate::abi::qgc_core_get(c(path).as_ptr()) })
}

#[wasm_bindgen(js_name = getFields)]
pub fn get_fields(path: &str, fields: &str) -> String {
    taken(unsafe { crate::abi::qgc_core_get_fields(c(path).as_ptr(), c(fields).as_ptr()) })
}

#[wasm_bindgen]
pub fn set(path: &str, value: &str) -> String {
    taken(unsafe { crate::abi::qgc_core_set(c(path).as_ptr(), c(value).as_ptr()) })
}

#[wasm_bindgen]
pub fn invoke(path: &str, args: &str) -> String {
    taken(unsafe { crate::abi::qgc_core_invoke(c(path).as_ptr(), c(args).as_ptr()) })
}

#[wasm_bindgen]
pub fn guided(action: &str) -> String {
    taken(unsafe { crate::abi::qgc_core_guided(c(action).as_ptr()) })
}

#[wasm_bindgen]
pub fn watch(paths: &str) {
    unsafe { crate::abi::qgc_core_watch(c(paths).as_ptr()) }
}

#[wasm_bindgen(js_name = watchClient)]
pub fn watch_client(client: &str, paths: &str) {
    unsafe { crate::abi::qgc_core_watch_client(c(client).as_ptr(), c(paths).as_ptr()) }
}

#[wasm_bindgen(js_name = onEvent)]
pub fn on_event(handler: js_sys::Function) {
    EVENTS.with(|events| *events.borrow_mut() = Some(handler));
    unsafe { crate::abi::qgc_core_set_event_handler(Some(event)) }
}

unsafe extern "C" fn event(path: *const c_char, json: *const c_char) {
    let (path, json) = (JsValue::from(read(path)), JsValue::from(read(json)));
    EVENTS.with(|events| events.borrow().as_ref().map(|handler| handler.call2(&JsValue::NULL, &path, &json)));
}

#[wasm_bindgen(js_name = settingsOpen)]
pub fn settings_open(text: &str) {
    crate::settingsstore::open_text(text);
    crate::applog::load_categories();
}

#[wasm_bindgen(js_name = settingsText)]
pub fn settings_text() -> String {
    crate::settingsstore::text().unwrap_or_default()
}

#[wasm_bindgen(js_name = settingsRevision)]
pub fn settings_revision() -> u64 {
    crate::settingsstore::revision()
}

#[wasm_bindgen(js_name = setMockLinksAvailable)]
pub fn set_mock_links_available(available: bool) {
    crate::mocklink::set_available(available);
}

#[wasm_bindgen(js_name = linkOpen)]
pub fn link_open(config: &str) -> String {
    taken(unsafe { crate::abi::qgc_core_link_open(c(config).as_ptr()) })
}

#[wasm_bindgen(js_name = linkClose)]
pub fn link_close(id: u32, reason: &str) -> bool {
    unsafe { crate::abi::qgc_core_link_close(id, c(reason).as_ptr()) }
}

#[wasm_bindgen(js_name = hostLinkOpen)]
pub fn host_link_open(kind: &str, name: &str) -> String {
    taken(unsafe { crate::abi::qgc_core_host_link_open(c(kind).as_ptr(), c(name).as_ptr()) })
}

#[wasm_bindgen(js_name = hostLinkBytes)]
pub fn host_link_bytes(id: u32, bytes: &[u8]) {
    unsafe { crate::abi::qgc_core_host_link_bytes(id, bytes.as_ptr(), bytes.len()) }
}

#[wasm_bindgen(js_name = hostLinkClosed)]
pub fn host_link_closed(id: u32, reason: &str) -> bool {
    unsafe { crate::abi::qgc_core_host_link_closed(id, c(reason).as_ptr()) }
}

#[wasm_bindgen(js_name = onLinkWrite)]
pub fn on_link_write(writer: js_sys::Function) {
    WRITER.with(|held| *held.borrow_mut() = Some(writer));
    unsafe { crate::abi::qgc_core_set_link_writer(Some(write), std::ptr::null_mut()) }
}

unsafe extern "C" fn write(id: u32, bytes: *const u8, len: usize, _user: *mut c_void) {
    let bytes = js_sys::Uint8Array::from(unsafe { std::slice::from_raw_parts(bytes, len) });
    WRITER.with(|held| held.borrow().as_ref().map(|writer| writer.call2(&JsValue::NULL, &id.into(), &bytes)));
}

fn serial_call(name: &str, args: js_sys::Array) -> JsValue {
    SERIAL_HOST.with(|host| {
        host.borrow()
            .as_ref()
            .and_then(|host| js_sys::Reflect::get(host, &name.into()).ok())
            .and_then(|method| method.dyn_into::<js_sys::Function>().ok())
            .and_then(|method| method.apply(&JsValue::NULL, &args).ok())
            .unwrap_or(JsValue::UNDEFINED)
    })
}

fn serial_open(id: u32, name: &str, baud: u32, data_bits: i64, stop_bits: i64, parity: i64) -> bool {
    let args: js_sys::Array = [JsValue::from(id), name.into(), baud.into(), (data_bits as f64).into(), (stop_bits as f64).into(), (parity as f64).into()].into_iter().collect();
    serial_call("open", args).as_bool().unwrap_or(false)
}

fn serial_write(id: u32, bytes: &[u8]) -> bool {
    let args: js_sys::Array = [JsValue::from(id), js_sys::Uint8Array::from(bytes).into()].into_iter().collect();
    serial_call("write", args).as_bool().unwrap_or(false)
}

fn serial_close(id: u32) {
    serial_call("close", std::iter::once(JsValue::from(id)).collect());
}

fn serial_ports() -> Vec<crate::boards::PortInfo> {
    SERIAL_PORTS.with(|ports| ports.borrow().clone())
}

#[wasm_bindgen(js_name = setSerialHost)]
pub fn set_serial_host(host: js_sys::Object) {
    SERIAL_HOST.with(|held| *held.borrow_mut() = Some(host));
    crate::platformserial::install(crate::platformserial::Hooks { open: serial_open, write: serial_write, close: serial_close, ports: serial_ports });
}

#[wasm_bindgen(js_name = setSerialPorts)]
pub fn set_serial_ports(lines: Vec<String>) {
    let ports = lines.iter().filter_map(|line| crate::platformserial::port_from_info(line)).collect();
    SERIAL_PORTS.with(|held| *held.borrow_mut() = ports);
}

#[wasm_bindgen(js_name = serialReceived)]
pub fn serial_received(id: u32, bytes: &[u8]) {
    crate::platformserial::received(id, bytes.to_vec());
}

#[wasm_bindgen(js_name = serialClosed)]
pub fn serial_closed(id: u32, reason: &str) {
    crate::platformserial::closed(id, reason);
}

#[wasm_bindgen(js_name = mapTile)]
pub fn map_tile(map_type: &str, x: i32, y: i32, zoom: i32) -> Option<Vec<u8>> {
    TILES.with(|cache| crate::maptiles::fetch_remembered(map_type, x, y, zoom, cache.as_ref(), true))
}

#[cfg(feature = "synthetic")]
#[wasm_bindgen(js_name = syntheticAttach)]
pub fn synthetic_attach(canvas: web_sys::OffscreenCanvas, width: u32, height: u32) -> u32 {
    crate::synthetic::webview::attach(canvas, width, height)
}

#[cfg(feature = "synthetic")]
#[wasm_bindgen(js_name = syntheticResize)]
pub fn synthetic_resize(key: u32, width: u32, height: u32) {
    crate::synthetic::webview::resize(key, width, height);
}

#[cfg(feature = "synthetic")]
#[wasm_bindgen(js_name = syntheticRelease)]
pub fn synthetic_release(key: u32) {
    crate::synthetic::webview::release(key);
}

#[wasm_bindgen(js_name = vfsWrite)]
pub fn vfs_write(path: &str, bytes: &[u8]) -> bool {
    crate::vfs::write(path, bytes).is_ok()
}

#[wasm_bindgen(js_name = vfsRead)]
pub fn vfs_read(path: &str) -> Option<Vec<u8>> {
    crate::vfs::read(path).ok()
}

#[wasm_bindgen(js_name = vfsRemove)]
pub fn vfs_remove(path: &str) -> bool {
    crate::vfs::remove(path).is_ok()
}

#[wasm_bindgen(js_name = vfsRemovePrefixed)]
pub fn vfs_remove_prefixed(prefix: &str) {
    crate::vfs::remove_prefixed(prefix);
}

#[wasm_bindgen(js_name = gcsPositionSource)]
pub fn gcs_position_source(token: &str) {
    unsafe { crate::abi::qgc_core_gcs_position_source(c(token).as_ptr()) }
}

fn reading(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

#[wasm_bindgen(js_name = gcsPositionUpdate)]
#[allow(clippy::too_many_arguments)]
pub fn gcs_position_update(latitude: f64, longitude: f64, altitude: f64, horizontal_accuracy_m: f64, vertical_accuracy_m: f64, direction_deg: f64, direction_accuracy_deg: f64, ground_speed_m_s: f64) {
    crate::gcsposition::report(crate::gcsposition::Update {
        latitude: reading(latitude),
        longitude: reading(longitude),
        altitude: reading(altitude),
        horizontal_accuracy_m: reading(horizontal_accuracy_m),
        vertical_accuracy_m: reading(vertical_accuracy_m),
        direction_deg: reading(direction_deg),
        direction_accuracy_deg: reading(direction_accuracy_deg),
        ground_speed_m_s: reading(ground_speed_m_s),
    });
}

#[wasm_bindgen(js_name = gcsPositionError)]
pub fn gcs_position_error(code: i32) {
    crate::gcsposition::report_error(i64::from(code));
}

#[wasm_bindgen(js_name = notifyDeepLink)]
pub fn notify_deep_link(link: &str) {
    if let Some(host) = crate::nativeargs::deep_link_device(link) {
        crate::deeplinksetup::offer(&host);
    }
    if let Some((_, Some(camera))) = crate::nativeargs::deep_link_writes(link, crate::mocklink::available()) {
        crate::cameras::adopt(camera);
    }
}

#[wasm_bindgen]
pub fn pump() {
    crate::background::run_queued();
    crate::mocklink::pump();
    let now = crate::hub::now_ms();
    if now.saturating_sub(LAST_HUB_MS.get()) >= HUB_PERIOD_MS {
        LAST_HUB_MS.set(now);
        crate::abi::pump_once();
    }
}
