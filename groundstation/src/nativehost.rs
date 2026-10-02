use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::io::{BufRead, BufReader, Write};
use std::sync::{Condvar, Mutex, OnceLock, PoisonError};

use serde_json::{Value, json};

use crate::debugapi::{DebugApi, Host};
use crate::nativeargs::{deep_link_device, deep_link_writes, options};

static QUIT: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());
static DEBUG_SERVER: OnceLock<u16> = OnceLock::new();

fn given(text: &str) -> *mut c_char {
    CString::new(text).unwrap_or_default().into_raw()
}

fn text(value: &str) -> CString {
    CString::new(value).unwrap_or_default()
}

fn read(raw: *const c_char) -> String {
    if raw.is_null() { String::new() } else { unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned() }
}

fn taken(raw: *mut c_char) -> String {
    let owned = read(raw);
    if !raw.is_null() {
        unsafe { crate::abi::qgc_core_free(raw) };
    }
    owned
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_bridge_get(path: *const c_char) -> *mut c_char {
    unsafe { crate::abi::qgc_core_get(path) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_bridge_get_fields(path: *const c_char, fields_csv: *const c_char) -> *mut c_char {
    unsafe { crate::abi::qgc_core_get_fields(path, fields_csv) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_bridge_set(path: *const c_char, value_json: *const c_char) -> *mut c_char {
    unsafe { crate::abi::qgc_core_set(path, value_json) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_bridge_invoke(path: *const c_char, args_json: *const c_char) -> *mut c_char {
    unsafe { crate::abi::qgc_core_invoke(path, args_json) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_bridge_watch(paths_csv: *const c_char) {
    unsafe { crate::abi::qgc_core_watch(paths_csv) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_bridge_watch_client(client: *const c_char, paths_csv: *const c_char) {
    unsafe { crate::abi::qgc_core_watch_client(client, paths_csv) }
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_bridge_watch_status() -> *mut c_char {
    given("{}")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_bridge_set_event_handler(handler: Option<unsafe extern "C" fn(*const c_char, *const c_char)>) {
    unsafe { crate::abi::qgc_core_set_event_handler(handler) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_bridge_free(text: *mut c_char) {
    unsafe { crate::abi::qgc_core_free(text) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_links_create(kind: c_int, name: *const c_char, host: *const c_char, port: c_int) -> c_int {
    let name = read(name).trim().to_string();
    let host = read(host);
    let table = crate::corelinks::table();
    let config = match table.kind(i64::from(kind)) {
        Some(crate::linkconfig::LinkKind::Tcp) => crate::corelinks::created("tcp", &name, &host, i64::from(port)),
        Some(crate::linkconfig::LinkKind::Udp) => crate::corelinks::created("udp", &name, "", i64::from(port)),
        Some(crate::linkconfig::LinkKind::Serial) => Some(crate::corelinks::serial_entry(&name, &host, u32::try_from(port).unwrap_or(0)).config),
        _ => None,
    };
    c_int::from(config.is_some_and(|config| crate::corelinks::add(config)))
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_core_headless() -> c_int {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_runs_event_loop() -> c_int {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_set_host_provides_ui(_provides: c_int) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_start(argc: c_int, argv: *const *const c_char) -> c_int {
    let arguments: Vec<String> = (0..usize::try_from(argc).unwrap_or(0)).map(|i| read(if argv.is_null() { std::ptr::null() } else { unsafe { *argv.add(i) } })).collect();
    let chosen = options(&arguments);
    crate::units::set_measurement_system(chosen.measurement_system);
    crate::applog::install();
    unsafe {
        crate::abi::qgc_core_settings_open(text(&chosen.settings.to_string_lossy()).as_ptr());
        crate::abi::qgc_core_set_application_name(text(&chosen.application).as_ptr());
        crate::abi::qgc_core_gcs_position_source(text(&chosen.position_source).as_ptr());
    }
    crate::settingsstore::establish_save_path(chosen.save_path.as_deref(), chosen.removable_save_path.as_deref(), &chosen.application);
    crate::telemetrylog::recover_lost();
    crate::mavlinklog::configure_hub();
    if let Some(cache) = &chosen.map_cache {
        unsafe { crate::abi::qgc_core_set_map_cache_path(text(cache).as_ptr()) };
    }
    if chosen.autoconnect {
        crate::abi::qgc_core_links_start();
    }
    crate::abi::start_pump();
    if let Some(port) = chosen.debug_port {
        start_debug_server(port);
    }
    if let Some(link) = arguments.iter().skip(1).find(|a| a.starts_with("aircast-qgc://")) {
        unsafe { qgc_handle_deep_link(text(link).as_ptr()) };
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_run() -> c_int {
    let (quit, wake) = &QUIT;
    let asked = quit.lock().unwrap_or_else(PoisonError::into_inner);
    drop(wake.wait_while(asked, |asked| !*asked).unwrap_or_else(PoisonError::into_inner));
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_request_quit() {
    let (quit, wake) = &QUIT;
    *quit.lock().unwrap_or_else(PoisonError::into_inner) = true;
    wake.notify_all();
}

#[unsafe(no_mangle)]
pub extern "C" fn qgc_shutdown() {
    crate::settingsstore::persist();
}

fn write_setting(path: &str, value: &str) {
    let payload = json!({ "value": value }).to_string();
    taken(unsafe { crate::abi::qgc_core_set(text(path).as_ptr(), text(&payload).as_ptr()) });
}

fn setup_from_device(host: &str) {
    if let Some(config) = crate::devicesetup::fetch(host, crate::devicesetup::STREAM_CONFIG) {
        let via = crate::devicesetup::fetch(host, crate::devicesetup::WATCH_VIA).unwrap_or(Value::Null);
        crate::devicesetup::camera_writes(host, &config, &via).iter().for_each(|(path, value)| write_setting(path, value));
    }
    let Some(telemetry) = crate::devicesetup::fetch(host, crate::devicesetup::TELEMETRY_CONFIG) else { return };
    if let Some((api_base, link)) = crate::devicesetup::cloud_link(host, &telemetry) {
        crate::account::set_api_base(&api_base);
        crate::corelinks::replace_and_connect(link);
    }
    if let Some(link) = crate::devicesetup::telemetry_link(host, &telemetry) {
        crate::corelinks::replace_and_connect(link);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_handle_deep_link(link: *const c_char) {
    let link = read(link);
    if let Some(host) = deep_link_device(&link) {
        std::thread::spawn(move || setup_from_device(&host));
    }
    let Some((debug, writes)) = deep_link_writes(&link) else { return };
    if let Some(port) = debug {
        start_debug_server(port);
    }
    writes.iter().for_each(|(path, value)| write_setting(path, value));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_map_current_type() -> *mut c_char {
    let value = |path: &str| serde_json::from_str::<Value>(&taken(unsafe { crate::abi::qgc_core_get(text(path).as_ptr()) })).ok().and_then(|v| v.get("value").and_then(Value::as_str).map(str::to_string)).unwrap_or_default();
    given(&format!("{} {}", value("settings.flightMapSettings.mapProvider.rawValue"), value("settings.flightMapSettings.mapType.rawValue")))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_map_tile_fetch(map_type: *const c_char, x: c_int, y: c_int, zoom: c_int, handler: Option<unsafe extern "C" fn(*const u8, c_int, *mut c_void)>, context: *mut c_void) {
    let Some(handler) = handler else { return };
    let provider = read(map_type);
    let context = context as usize;
    std::thread::spawn(move || {
        let cache = crate::terrainservice::cache_path().and_then(|path| crate::tilecache::Cache::open(&path).ok());
        let persist = !crate::settingsstore::raw_setting("settings.appSettings.disableAllPersistence").and_then(|v| v.as_bool()).unwrap_or(false);
        let image = crate::maptiles::fetch(&provider, x, y, zoom, &crate::mapurls::keys_from_settings(), cache.as_ref(), persist, &crate::maptiles::fetch_over_http);
        match image {
            Some(image) => unsafe { handler(image.as_ptr(), c_int::try_from(image.len()).unwrap_or(0), context as *mut c_void) },
            None => unsafe { handler(std::ptr::null(), 0, context as *mut c_void) },
        }
    });
}

struct CoreOnly;

impl Host for CoreOnly {
    fn bridge_get(&self, path: &str) -> String {
        taken(unsafe { crate::abi::qgc_core_get(text(path).as_ptr()) })
    }
    fn bridge_set(&self, path: &str, payload_json: &str) -> String {
        taken(unsafe { crate::abi::qgc_core_set(text(path).as_ptr(), text(payload_json).as_ptr()) })
    }
    fn bridge_invoke(&self, path: &str, args_json: &str) -> String {
        taken(unsafe { crate::abi::qgc_core_invoke(text(path).as_ptr(), text(args_json).as_ptr()) })
    }
    fn link_configurations(&self) -> Value {
        serde_json::from_str::<Value>(&self.bridge_get(crate::corelinks::MODEL)).unwrap_or(Value::Null)
    }
    fn link_open(&self, config_json: &str) -> Result<u32, String> {
        let answer: Value = serde_json::from_str(&taken(unsafe { crate::abi::qgc_core_link_open(text(config_json).as_ptr()) })).unwrap_or(Value::Null);
        answer["id"].as_u64().and_then(|id| u32::try_from(id).ok()).ok_or_else(|| answer["reason"].as_str().unwrap_or("the link did not open").to_string())
    }
    fn link_close(&self, _name: Option<&str>) -> Result<usize, String> {
        Err("closing by name needs the host's link list".to_string())
    }
    fn mission(&self, request_json: &str) -> Result<(), String> {
        let answer: Value = serde_json::from_str(&taken(unsafe { crate::abi::qgc_core_mission(text(request_json).as_ptr()) })).unwrap_or(Value::Null);
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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeDebugHooks {
    windows: Option<unsafe extern "C" fn() -> *mut c_char>,
    click: Option<unsafe extern "C" fn(*const c_char, f64, f64) -> *mut c_char>,
    type_text: Option<unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_char>,
    probe: Option<unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char>,
    menu: Option<unsafe extern "C" fn() -> *mut c_char>,
    menu_invoke: Option<unsafe extern "C" fn(*const c_char) -> *mut c_char>,
    bridge_stats: Option<unsafe extern "C" fn() -> *mut c_char>,
}

static NATIVE_HOOKS: Mutex<Option<NativeDebugHooks>> = Mutex::new(None);

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgc_native_debug_install(hooks: *const NativeDebugHooks) {
    *NATIVE_HOOKS.lock().unwrap_or_else(PoisonError::into_inner) = if hooks.is_null() { None } else { Some(unsafe { *hooks }) };
}

fn hook_answer(raw: *mut c_char) -> String {
    let answer = if raw.is_null() { "{}".to_string() } else { read(raw) };
    if !raw.is_null() {
        unsafe { free(raw.cast()) };
    }
    answer
}

fn native_route(path: &str, query: &str) -> (u16, String) {
    let refuse = |message: &str| (400, json!({ "error": message }).to_string());
    let Some(hooks) = *NATIVE_HOOKS.lock().unwrap_or_else(PoisonError::into_inner) else {
        return refuse("this build has no native UI installed");
    };
    let pairs = crate::debugapi::query_pairs(query);
    let value = |key: &str| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
    let called = match path {
        "/native/windows" => hooks.windows.map(|f| unsafe { f() }),
        "/native/menu" => hooks.menu.map(|f| unsafe { f() }),
        "/native/bridge" => hooks.bridge_stats.map(|f| unsafe { f() }),
        "/native/menu/invoke" => match value("path").filter(|p| !p.is_empty()) {
            Some(item) => hooks.menu_invoke.map(|f| unsafe { f(text(&item).as_ptr()) }),
            None => return refuse("path is required, e.g. path=Window/Native Telemetry"),
        },
        "/native/probe" => {
            let args: serde_json::Map<String, Value> = pairs.iter().filter(|(k, _)| k != "id" && k != "action").map(|(k, v)| (k.clone(), json!(v))).collect();
            let (id, action, args) = (text(&value("id").unwrap_or_default()), text(&value("action").unwrap_or_default()), text(&Value::Object(args).to_string()));
            hooks.probe.map(|f| unsafe { f(id.as_ptr(), action.as_ptr(), args.as_ptr()) })
        }
        "/native/type" => match value("window").filter(|w| !w.is_empty()) {
            Some(window) => hooks.type_text.map(|f| unsafe { f(text(&window).as_ptr(), text(&value("text").unwrap_or_default()).as_ptr()) }),
            None => return refuse("window is required"),
        },
        "/native/click" => match (value("window").filter(|w| !w.is_empty()), value("x").and_then(|x| x.parse::<f64>().ok()), value("y").and_then(|y| y.parse::<f64>().ok())) {
            (Some(window), Some(x), Some(y)) => hooks.click.map(|f| unsafe { f(text(&window).as_ptr(), x, y) }),
            _ => return refuse("window, x and y are required"),
        },
        _ => return refuse(&format!("unknown native route: {path}")),
    };
    (200, called.map_or_else(|| "{}".to_string(), hook_answer))
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
    let (status, body) = match path.starts_with("/native/") {
        true => native_route(path, query),
        false => {
            let response = api.dispatch(&CoreOnly, method, path, query, "");
            (response.status, response.body_text())
        }
    };
    let mut stream = stream;
    write!(stream, "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

fn start_debug_server(port: u16) {
    if DEBUG_SERVER.set(port).is_err() {
        return;
    }
    let Ok(listener) = std::net::TcpListener::bind(("127.0.0.1", port)) else { return };
    std::thread::Builder::new()
        .name("qgc-debug-api".to_string())
        .spawn(move || {
            let api = DebugApi::new();
            listener.incoming().flatten().for_each(|stream| {
                let _ = serve(stream, &api);
            });
        })
        .ok();
}
