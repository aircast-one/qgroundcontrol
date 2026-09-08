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
