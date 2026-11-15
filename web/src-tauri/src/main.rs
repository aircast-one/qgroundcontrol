#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod annexb;
mod files;
mod ingest;
mod matroska;
mod mpegts;
mod record;
mod rtp;
mod synthetic;
mod video;
#[cfg(windows)]
mod webview2;
mod window;

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use groundstation::nativehost::{
    qgc_bridge_free, qgc_bridge_get, qgc_bridge_get_fields, qgc_bridge_invoke, qgc_bridge_set,
    qgc_bridge_set_event_handler, qgc_bridge_watch_client, qgc_handle_deep_link,
    qgc_map_tile_fetch, qgc_shutdown, qgc_start,
};
use serde_json::Value;
use tauri::async_runtime::spawn_blocking;
use tauri::utils::config::BackgroundThrottlingPolicy;
use tauri::webview::{DownloadEvent, NewWindowResponse, PageLoadEvent};
use tauri::{
    AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent,
    ipc::Response,
};
use tokio::sync::oneshot;

unsafe extern "C" {
    fn qgc_core_guided(action_json: *const c_char) -> *mut c_char;
    fn qgc_core_link_open(config_json: *const c_char) -> *mut c_char;
    fn qgc_core_link_close(id: u32, reason: *const c_char) -> bool;
    fn qgc_core_links_start();
    fn qgc_core_gcs_position_source(token: *const c_char);
}

const EVENT_FLUSH: Duration = Duration::from_millis(16);
const LINK_SCHEMES: [&str; 2] = ["http", "https"];
const APP_HOST: &str = "tauri.localhost";

static EVENTS: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
static EVENTS_ARRIVED: Condvar = Condvar::new();

fn events() -> MutexGuard<'static, Vec<(String, String)>> {
    EVENTS.lock().unwrap_or_else(PoisonError::into_inner)
}

fn text(value: &str) -> Result<CString, String> {
    CString::new(value)
        .map_err(|_| format!("{value:?} holds a NUL byte, which the core cannot take."))
}

fn read(raw: *const c_char) -> String {
    if raw.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(raw) }
            .to_string_lossy()
            .into_owned()
    }
}

fn taken(raw: *mut c_char) -> String {
    let owned = read(raw);
    if !raw.is_null() {
        unsafe { qgc_bridge_free(raw) };
    }
    owned
}

unsafe extern "C" fn event(path: *const c_char, json: *const c_char) {
    events().push((read(path), read(json)));
    EVENTS_ARRIVED.notify_one();
}

fn forward_events(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            drop(
                EVENTS_ARRIVED
                    .wait_while(events(), |waiting| waiting.is_empty())
                    .unwrap_or_else(PoisonError::into_inner),
            );
            std::thread::sleep(EVENT_FLUSH);
            let batch = std::mem::take(&mut *events());
            if let Err(error) = app.emit("core-events", batch) {
                log::warn!(target: "desktop.core", "event=core_events_not_delivered reason=\"{error}\"");
            }
        }
    });
}

fn string_at(args: &[Value], index: usize) -> String {
    match args.get(index) {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

fn number_at(args: &[Value], index: usize) -> f64 {
    args.get(index).and_then(Value::as_f64).unwrap_or(f64::NAN)
}

fn reading(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

#[tauri::command]
async fn core(name: String, args: Vec<Value>) -> Result<Value, String> {
    spawn_blocking(move || core_call(&name, &args))
        .await
        .map_err(|error| error.to_string())?
}

fn core_call(name: &str, args: &[Value]) -> Result<Value, String> {
    let at = |index| text(&string_at(args, index));
    Ok(match name {
        "get" => taken(unsafe { qgc_bridge_get(at(0)?.as_ptr()) }).into(),
        "getFields" => {
            taken(unsafe { qgc_bridge_get_fields(at(0)?.as_ptr(), at(1)?.as_ptr()) }).into()
        }
        "getMany" => args
            .first()
            .and_then(Value::as_array)
            .map_or(Ok(Vec::new()), |paths| {
                paths
                    .iter()
                    .map(|path| {
                        let path = text(path.as_str().unwrap_or_default())?;
                        Ok(Value::from(taken(unsafe { qgc_bridge_get(path.as_ptr()) })))
                    })
                    .collect::<Result<Vec<_>, String>>()
            })?
            .into(),
        "set" => taken(unsafe { qgc_bridge_set(at(0)?.as_ptr(), at(1)?.as_ptr()) }).into(),
        "invoke" => taken(unsafe { qgc_bridge_invoke(at(0)?.as_ptr(), at(1)?.as_ptr()) }).into(),
        "guided" => taken(unsafe { qgc_core_guided(at(0)?.as_ptr()) }).into(),
        "deviceInvoke" => groundstation::deviceviews::run(&string_at(args, 0), &string_at(args, 1))
            .to_string()
            .into(),
        "watchClient" => {
            unsafe { qgc_bridge_watch_client(at(0)?.as_ptr(), at(1)?.as_ptr()) };
            Value::Null
        }
        "linkOpen" => taken(unsafe { qgc_core_link_open(at(0)?.as_ptr()) }).into(),
        "linkClose" => unsafe {
            qgc_core_link_close(
                args.first().and_then(Value::as_u64).unwrap_or(0) as u32,
                at(1)?.as_ptr(),
            )
        }
        .into(),
        "linksStart" => {
            unsafe { qgc_core_links_start() };
            Value::Null
        }
        "setMockLinksAvailable" => {
            groundstation::mocklink::set_available(
                args.first().and_then(Value::as_bool).unwrap_or(false),
            );
            Value::Null
        }
        "gcsPositionSource" => {
            unsafe { qgc_core_gcs_position_source(at(0)?.as_ptr()) };
            Value::Null
        }
        "gcsPositionUpdate" => {
            groundstation::gcsposition::report(groundstation::gcsposition::Update {
                latitude: reading(number_at(args, 0)),
                longitude: reading(number_at(args, 1)),
                altitude: reading(number_at(args, 2)),
                horizontal_accuracy_m: reading(number_at(args, 3)),
                vertical_accuracy_m: reading(number_at(args, 4)),
                direction_deg: reading(number_at(args, 5)),
                direction_accuracy_deg: reading(number_at(args, 6)),
                ground_speed_m_s: reading(number_at(args, 7)),
            });
            Value::Null
        }
        "gcsPositionError" => {
            groundstation::gcsposition::report_error(
                args.first().and_then(Value::as_i64).unwrap_or(0),
            );
            Value::Null
        }
        "notifyDeepLink" => {
            unsafe { qgc_handle_deep_link(at(0)?.as_ptr()) };
            Value::Null
        }
        unknown => return Err(format!("The core has no call named {unknown}.")),
    })
}

unsafe extern "C" fn tile_arrived(bytes: *const u8, length: c_int, context: *mut c_void) {
    let sender = unsafe { Box::from_raw(context.cast::<oneshot::Sender<Vec<u8>>>()) };
    let tile = match (bytes.is_null(), usize::try_from(length)) {
        (false, Ok(length)) => unsafe { std::slice::from_raw_parts(bytes, length) }.to_vec(),
        _ => Vec::new(),
    };
    if sender.send(tile).is_err() {
        log::debug!(target: "desktop.core", "event=tile_unwanted");
    }
}

#[tauri::command]
async fn map_tile(map_type: String, x: i32, y: i32, zoom: i32) -> Result<Response, String> {
    let map_type = text(&map_type)?;
    let (sender, arrived) = oneshot::channel::<Vec<u8>>();
    let context = Box::into_raw(Box::new(sender)).cast::<c_void>();
    unsafe { qgc_map_tile_fetch(map_type.as_ptr(), x, y, zoom, Some(tile_arrived), context) };
    Ok(Response::new(arrived.await.unwrap_or_default()))
}

fn open_link(url: &url::Url) {
    if !LINK_SCHEMES.contains(&url.scheme()) {
        log::warn!(target: "desktop.window", "event=link_refused scheme={}", url.scheme());
        return;
    }
    if let Err(error) = tauri_plugin_opener::open_url(url.as_str(), None::<&str>) {
        log::warn!(target: "desktop.window", "event=link_not_opened host={} reason=\"{error}\"", url.host_str().unwrap_or_default());
    }
}

fn inside_the_app(url: &url::Url, dev: Option<&url::Url>) -> bool {
    url.scheme() == "tauri"
        || url.host_str() == Some(APP_HOST)
        || dev.is_some_and(|dev| dev.origin() == url.origin())
}

fn start_core(version: &str) {
    let arguments: Vec<CString> = std::env::args()
        .chain(
            [
                "--no-autoconnect",
                "--no-app-update",
                "--app-version",
                version,
            ]
            .map(str::to_string),
        )
        .filter_map(|argument| text(&argument).ok())
        .collect();
    let pointers: Vec<*const c_char> = arguments.iter().map(|a| a.as_ptr()).collect();
    unsafe {
        qgc_bridge_set_event_handler(Some(event));
        qgc_start(
            c_int::try_from(pointers.len()).unwrap_or(0),
            pointers.as_ptr(),
        );
    }
}

fn main() {
    let context = tauri::generate_context!();
    start_core(&context.package_info().version.to_string());
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(video::Streams::default())
        .manage(synthetic::Views::default())
        .setup(|app| {
            forward_events(app.handle().clone());
            let downloads = app.path().download_dir().ok();
            let dev = tauri::is_dev()
                .then(|| app.config().build.dev_url.clone())
                .flatten();
            let _window = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Aircast")
                .inner_size(1440.0, 900.0)
                .min_inner_size(900.0, 600.0)
                .background_throttling(BackgroundThrottlingPolicy::Disabled)
                .on_download(move |_, event| {
                    if let (DownloadEvent::Requested { destination, .. }, Some(folder)) =
                        (event, downloads.as_ref())
                    {
                        *destination = folder.join(destination.file_name().unwrap_or_default());
                    }
                    true
                })
                .on_new_window(|url, _| {
                    open_link(&url);
                    NewWindowResponse::Deny
                })
                .on_navigation(move |url| {
                    let inside = inside_the_app(url, dev.as_ref());
                    if !inside {
                        open_link(url);
                    }
                    inside
                })
                .on_page_load(|shown, loading| {
                    if loading.event() == PageLoadEvent::Started {
                        let app = shown.app_handle().clone();
                        let streams = app.state::<video::Streams>().page_left();
                        let views = app.state::<synthetic::Views>().forget_all();
                        spawn_blocking(move || {
                            app.state::<video::Streams>().close_each(streams);
                            synthetic::Views::release_each(views);
                        });
                    }
                })
                .build()?;
            #[cfg(windows)]
            webview2::allow_location(&_window)?;
            #[cfg(target_os = "macos")]
            window::guard_terminate(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core,
            map_tile,
            video::video_open,
            video::video_close,
            video::video_restart,
            video::video_record,
            files::staged_root,
            files::staged_prepare,
            files::staged_write,
            files::staged_read,
            files::staged_remove,
            files::staged_remove_prefixed,
            files::staged_list,
            files::saved_read,
            files::saved_list,
            synthetic::synthetic_open,
            synthetic::synthetic_resize,
            synthetic::synthetic_close,
            window::close_anyway,
            window::close_prompt_shown,
            window::close_prompt_dismissed,
            window::full_screen,
            window::toggle_full_screen
        ])
        .on_window_event(|shown, event| match event {
            WindowEvent::CloseRequested { api, .. } if window::holds_close(shown.app_handle()) => {
                api.prevent_close();
            }
            WindowEvent::Resized(_) => {
                if let Err(error) = shown
                    .is_fullscreen()
                    .and_then(|full| shown.emit("full-screen", full))
                {
                    log::warn!(target: "desktop.window", "event=full_screen_unknown reason=\"{error}\"");
                }
            }
            _ => {}
        })
        .build(context)
        .expect("the Aircast window failed to open")
        .run(|app, event| match event {
            RunEvent::ExitRequested { api, .. } if window::holds_close(app) => api.prevent_exit(),
            RunEvent::Exit => {
                let streams = app.state::<video::Streams>();
                streams.close_each(streams.page_left());
                qgc_shutdown();
            }
            _ => {}
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_stays_on_the_app_and_other_pages_open_outside() {
        let url = |text: &str| url::Url::parse(text).unwrap();
        let dev = url("http://localhost:8800");
        assert!(inside_the_app(&url("tauri://localhost/plan"), None));
        assert!(inside_the_app(&url("http://tauri.localhost/fly"), None));
        assert!(inside_the_app(
            &url("http://localhost:8800/fly"),
            Some(&dev)
        ));
        assert!(!inside_the_app(&url("http://localhost:8800/fly"), None));
        assert!(!inside_the_app(
            &url("https://docs.qgroundcontrol.com/"),
            Some(&dev)
        ));
        assert!(!inside_the_app(&url("http://localhost:8801/"), Some(&dev)));
    }
}
