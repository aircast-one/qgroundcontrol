use std::ffi::{CStr, CString, c_char};
use std::io::{BufRead, BufReader, Write};

use qgc_core::debugapi::{DebugApi, Host};
use serde_json::{Value, json};

unsafe extern "C" {
    fn qgc_core_get(path: *const c_char) -> *mut c_char;
    fn qgc_core_set(path: *const c_char, value_json: *const c_char) -> *mut c_char;
    fn qgc_core_invoke(path: *const c_char, args_json: *const c_char) -> *mut c_char;
    fn qgc_core_link_open(config_json: *const c_char) -> *mut c_char;
    fn qgc_core_mission(request_json: *const c_char) -> *mut c_char;
    fn qgc_core_settings_open(path: *const c_char);
    fn qgc_core_set_map_cache_path(path: *const c_char);
    fn qgc_core_free(text: *mut c_char);
}

fn given(text: &str) -> *mut c_char {
    CString::new(text).unwrap_or_default().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_qt_get(_path: *const c_char) -> *mut c_char {
    given(r#"{"kind":"null"}"#)
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_qt_get_fields(_path: *const c_char, _fields: *const c_char) -> *mut c_char {
    given(r#"{"kind":"null"}"#)
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_qt_set(_path: *const c_char, _value: *const c_char) -> *mut c_char {
    given(r#"{"ok":false,"reason":"no host"}"#)
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_qt_invoke(_path: *const c_char, _args: *const c_char) -> *mut c_char {
    given(r#"{"ok":false,"reason":"no host"}"#)
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_qt_watch(_paths: *const c_char) {}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_qt_remember_setting(_key: *const c_char, _value: *const c_char) {}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_qt_set_event_handler(_handler: Option<unsafe extern "C" fn(*const c_char, *const c_char)>) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_qt_free(text: *mut c_char) {
    if !text.is_null() {
        drop(unsafe { CString::from_raw(text) });
    }
}

fn text(value: &str) -> CString {
    CString::new(value).unwrap_or_default()
}

fn taken(raw: *mut c_char) -> String {
    if raw.is_null() {
        return String::new();
    }
    let owned = unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned();
    unsafe { qgc_core_free(raw) };
    owned
}

struct CoreOnly;

impl Host for CoreOnly {
    fn bridge_get(&self, path: &str) -> String {
        taken(unsafe { qgc_core_get(text(path).as_ptr()) })
    }
    fn bridge_set(&self, path: &str, payload_json: &str) -> String {
        taken(unsafe { qgc_core_set(text(path).as_ptr(), text(payload_json).as_ptr()) })
    }
    fn bridge_invoke(&self, path: &str, args_json: &str) -> String {
        taken(unsafe { qgc_core_invoke(text(path).as_ptr(), text(args_json).as_ptr()) })
    }
    fn link_configurations(&self) -> Value {
        json!({ "elements": [] })
    }
    fn link_open(&self, config_json: &str) -> Result<u32, String> {
        let answer: Value = serde_json::from_str(&taken(unsafe { qgc_core_link_open(text(config_json).as_ptr()) })).unwrap_or(Value::Null);
        answer["id"].as_u64().and_then(|id| u32::try_from(id).ok()).ok_or_else(|| answer["reason"].as_str().unwrap_or("the link did not open").to_string())
    }
    fn link_close(&self, _name: Option<&str>) -> Result<usize, String> {
        Err("closing by name needs the host's link list".to_string())
    }
    fn mission(&self, request_json: &str) -> Result<(), String> {
        let answer: Value = serde_json::from_str(&taken(unsafe { qgc_core_mission(text(request_json).as_ptr()) })).unwrap_or(Value::Null);
        match answer["ok"].as_bool() {
            Some(true) => Ok(()),
            _ => Err(answer["reason"].as_str().unwrap_or("the mission request failed").to_string()),
        }
    }
    fn set_logging_rules(&self, _rules: &str) {}
    fn bridge_writes_allowed(&self) -> bool {
        std::env::var("QGC_DEBUG_API_ALLOW_ACTUATORS").is_ok_and(|v| v == "1")
    }
    fn mock_links_available(&self) -> bool {
        false
    }
    fn mock_link_present(&self) -> bool {
        false
    }
    fn vehicle_connected(&self) -> bool {
        serde_json::from_str::<Value>(&self.bridge_get("vehicles.activeVehicleAvailable")).ok().and_then(|v| v["value"].as_bool()).unwrap_or(false)
    }
}

fn serve(stream: std::net::TcpStream, api: &DebugApi) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    let mut header = String::new();
    while reader.read_line(&mut header)? > 2 {
        header.clear();
    }
    let mut parts = request.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or("/"));
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let response = api.dispatch(&CoreOnly, method, path, query, "");
    let body = response.body_text();
    let mut stream = stream;
    write!(stream, "HTTP/1.1 {} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", response.status, body.len())
}

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    let option = |name: &str| arguments.iter().position(|a| a == name).and_then(|i| arguments.get(i + 1)).cloned();
    if let Some(settings) = option("--settings") {
        unsafe { qgc_core_settings_open(text(&settings).as_ptr()) };
    }
    if let Some(cache) = option("--map-cache") {
        unsafe { qgc_core_set_map_cache_path(text(&cache).as_ptr()) };
    }
    let port = option("--port").and_then(|p| p.parse::<u16>().ok()).unwrap_or(8777);
    let listener = std::net::TcpListener::bind(("127.0.0.1", port)).expect("the debug API port is free");
    let api = DebugApi::new();
    listener.incoming().flatten().for_each(|stream| {
        let _ = serve(stream, &api);
    });
}
