use std::cell::{Cell, RefCell};
use std::ffi::{CStr, CString, c_char, c_void};

use wasm_bindgen::prelude::*;

const HUB_PERIOD_MS: u64 = 100;

thread_local! {
    static EVENTS: RefCell<Option<js_sys::Function>> = const { RefCell::new(None) };
    static WRITER: RefCell<Option<js_sys::Function>> = const { RefCell::new(None) };
    static LAST_HUB_MS: Cell<u64> = const { Cell::new(0) };
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

#[wasm_bindgen(js_name = onEvent)]
pub fn on_event(handler: js_sys::Function) {
    EVENTS.with(|events| *events.borrow_mut() = Some(handler));
    unsafe { crate::abi::qgc_core_set_event_handler(Some(event)) }
}

unsafe extern "C" fn event(path: *const c_char, json: *const c_char) {
    let (path, json) = (JsValue::from(read(path)), JsValue::from(read(json)));
    EVENTS.with(|events| events.borrow().as_ref().map(|handler| handler.call2(&JsValue::NULL, &path, &json)));
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
