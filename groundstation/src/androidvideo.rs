use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::sync::OnceLock;
use std::time::Duration;

use jni::objects::{GlobalRef, JObject, JString};
use jni::{JNIEnv, JavaVM};

const LIBRARY: &CStr = c"libqgc_video.so";
const RTLD_NOW: c_int = 2;
const DRIVE_INTERVAL: Duration = Duration::from_millis(500);

unsafe extern "C" {
    fn dlopen(name: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

pub struct Video {
    init: unsafe extern "C" fn(*mut jni::sys::JavaVM, jni::sys::jobject, jni::sys::jobject, *const c_char, *const c_char) -> bool,
    pub set_surface: unsafe extern "C" fn(*mut jni::sys::JNIEnv, c_int, jni::sys::jobject) -> bool,
    force_decoder: unsafe extern "C" fn(c_int),
    start: unsafe extern "C" fn(c_int, *const c_char) -> bool,
    stop: unsafe extern "C" fn(c_int),
    running: unsafe extern "C" fn(c_int) -> bool,
    pub width: unsafe extern "C" fn(c_int) -> c_int,
    pub height: unsafe extern "C" fn(c_int) -> c_int,
    frames: unsafe extern "C" fn(c_int) -> i64,
    source_buffers: unsafe extern "C" fn(c_int) -> i64,
    last_error: unsafe extern "C" fn(c_int) -> *const c_char,
    stream_error: unsafe extern "C" fn(c_int) -> *const c_char,
    pub copy_frame: unsafe extern "C" fn(c_int, *mut c_void, c_int, *mut c_int, *mut c_int, *mut c_int) -> bool,
    start_recording: unsafe extern "C" fn(c_int, *const c_char, c_int) -> bool,
    stop_recording: unsafe extern "C" fn(c_int),
    recording: unsafe extern "C" fn(c_int) -> bool,
    mock_serve: Option<unsafe extern "C" fn(c_int, c_int) -> *mut c_void>,
    mock_uri: Option<unsafe extern "C" fn(*mut c_void) -> *const c_char>,
    mock_stop: Option<unsafe extern "C" fn(*mut c_void)>,
    mock_pattern: Option<unsafe extern "C" fn(*const c_char)>,
}

static VIDEO: OnceLock<Option<Video>> = OnceLock::new();
static APPLICATION: OnceLock<(GlobalRef, GlobalRef)> = OnceLock::new();

fn symbol<T>(handle: *mut c_void, name: &CStr) -> Option<T> {
    let found = unsafe { dlsym(handle, name.as_ptr()) };
    (!found.is_null()).then(|| unsafe { std::mem::transmute_copy(&found) })
}

fn load() -> Option<Video> {
    let handle = unsafe { dlopen(LIBRARY.as_ptr(), RTLD_NOW) };
    (!handle.is_null()).then_some(())?;
    let abi = symbol::<unsafe extern "C" fn() -> c_int>(handle, c"qgc_video_abi_version").map(|version| unsafe { version() });
    if abi != Some(crate::videohost::VIDEO_ABI_VERSION) {
        log::error!("libqgc_video.so speaks video ABI {abi:?} and this core speaks {}, so video stays off", crate::videohost::VIDEO_ABI_VERSION);
        return None;
    }
    Some(Video {
        init: symbol(handle, c"qgc_video_android_init")?,
        set_surface: symbol(handle, c"qgc_video_android_set_surface")?,
        force_decoder: symbol(handle, c"qgc_video_android_force_decoder")?,
        start: symbol(handle, c"qgc_video_start")?,
        stop: symbol(handle, c"qgc_video_stop")?,
        running: symbol(handle, c"qgc_video_running")?,
        width: symbol(handle, c"qgc_video_width")?,
        height: symbol(handle, c"qgc_video_height")?,
        frames: symbol(handle, c"qgc_video_frames")?,
        source_buffers: symbol(handle, c"qgc_video_source_buffers")?,
        last_error: symbol(handle, c"qgc_video_last_error")?,
        stream_error: symbol(handle, c"qgc_video_stream_error")?,
        copy_frame: symbol(handle, c"qgc_video_copy_frame")?,
        start_recording: symbol(handle, c"qgc_video_start_recording")?,
        stop_recording: symbol(handle, c"qgc_video_stop_recording")?,
        recording: symbol(handle, c"qgc_video_recording")?,
        mock_serve: symbol(handle, c"qgc_video_mock_serve"),
        mock_uri: symbol(handle, c"qgc_video_mock_uri"),
        mock_stop: symbol(handle, c"qgc_video_mock_stop"),
        mock_pattern: symbol(handle, c"qgc_video_mock_pattern"),
    })
}

pub fn video() -> Option<&'static Video> {
    VIDEO.get_or_init(load).as_ref()
}

pub struct MockStream {
    handle: usize,
    pub uri: String,
}

impl Drop for MockStream {
    fn drop(&mut self) {
        if let Some(stop) = video().and_then(|v| v.mock_stop) {
            unsafe { stop(self.handle as *mut c_void) };
        }
    }
}

pub fn mock_serve(kind: c_int, port: u16, pattern: &str) -> Option<MockStream> {
    let video = video()?;
    if let (Some(choose), Ok(name)) = (video.mock_pattern, CString::new(pattern)) {
        unsafe { choose(name.as_ptr()) };
    }
    let (serve, uri) = (video.mock_serve?, video.mock_uri?);
    let handle = unsafe { serve(kind, c_int::from(port)) };
    (!handle.is_null()).then(|| MockStream { handle: handle as usize, uri: unsafe { CStr::from_ptr(uri(handle)) }.to_string_lossy().into_owned() })
}

fn folder(env: &mut JNIEnv, application: &JObject, getter: &str) -> jni::errors::Result<CString> {
    let file = env.call_method(application, getter, "()Ljava/io/File;", &[])?.l()?;
    let path = JString::from(env.call_method(&file, "getAbsolutePath", "()Ljava/lang/String;", &[])?.l()?);
    let text: String = env.get_string(&path)?.into();
    Ok(CString::new(text).unwrap_or_default())
}

fn initialise(vm: &JavaVM, video: &Video) -> jni::errors::Result<bool> {
    let mut env = vm.attach_current_thread_as_daemon()?;
    let application = env.call_static_method("android/app/ActivityThread", "currentApplication", "()Landroid/app/Application;", &[])?.l()?;
    let loader = env.call_method(&application, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?.l()?;
    let (files, cache) = (folder(&mut env, &application, "getFilesDir")?, folder(&mut env, &application, "getCacheDir")?);
    let globals = (env.new_global_ref(application)?, env.new_global_ref(loader)?);
    let (application, loader) = APPLICATION.get_or_init(|| globals);
    Ok(unsafe { (video.init)(vm.get_java_vm_pointer(), application.as_obj().as_raw(), loader.as_obj().as_raw(), files.as_ptr(), cache.as_ptr()) })
}

#[derive(Default)]
struct Channel {
    driven: Option<String>,
    restarted: bool,
    error: String,
    streamed: String,
    decoding: bool,
}

impl Channel {
    fn stop_changed(&mut self, video: &Video, channel: c_int, wanted: &Option<String>) {
        let restart = crate::videohost::take_restart(channel as usize);
        if self.driven.is_some() && (restart || *wanted != self.driven) {
            unsafe { (video.stop)(channel) };
            log::info!("Video channel {channel} stopped");
            self.driven = None;
        }
    }

    fn start_wanted(&mut self, video: &Video, channel: c_int, wanted: Option<String>) {
        let Some(pipeline) = wanted.as_ref().filter(|_| wanted != self.driven) else { return };
        let text = CString::new(pipeline.as_str()).unwrap_or_default();
        let started = unsafe { (video.start)(channel, text.as_ptr()) };
        self.restarted = true;
        self.decoding = false;
        self.error = if started { String::new() } else { unsafe { CStr::from_ptr((video.last_error)(channel)) }.to_string_lossy().into_owned() };
        match started {
            true => log::info!("Video channel {channel} started"),
            false => log::warn!("Video pipeline on channel {channel} did not start: {}", self.error),
        }
        self.driven = wanted;
    }

    fn report(&mut self, video: &Video, channel: c_int) {
        if self.driven.is_none() {
            return;
        }
        let (running, frames, width, height) = unsafe { ((video.running)(channel), (video.frames)(channel), (video.width)(channel), (video.height)(channel)) };
        if frames > 0 && !std::mem::replace(&mut self.decoding, true) {
            log::info!("Video channel {channel} decoding {width}x{height}");
        }
        let source = unsafe { (video.source_buffers)(channel) };
        let streamed = unsafe { CStr::from_ptr((video.stream_error)(channel)) }.to_string_lossy().into_owned();
        if streamed != self.streamed && !streamed.is_empty() {
            log::warn!("Video stream error on channel {channel}: {streamed}");
        }
        self.streamed = streamed.clone();
        let error = if self.error.is_empty() { streamed } else { self.error.clone() };
        crate::videohost::invoke("video.reportNative", &serde_json::json!([running, frames, width, height, error, source, std::mem::take(&mut self.restarted), channel]).to_string());
    }
}

#[derive(Default)]
struct Driver {
    channels: [Channel; crate::videohost::VIDEO_CHANNELS],
    decoders_ranked: bool,
    recording: Option<serde_json::Value>,
    recording_reported: bool,
}

impl Driver {
    fn record(&mut self, video: &Video) {
        let main = crate::videohost::MAIN_CHANNEL as c_int;
        let wanted = crate::videohost::native_recording();
        if wanted != self.recording {
            if self.recording.is_some() {
                unsafe { (video.stop_recording)(main) };
            }
            if let Some((file, format)) = wanted.as_ref().and_then(|w| Some((CString::new(w.get("file")?.as_str()?).ok()?, w.get("format")?.as_i64()?))) {
                unsafe { (video.start_recording)(main, file.as_ptr(), format as c_int) };
            }
            self.recording = wanted;
        }
        let active = unsafe { (video.recording)(main) };
        if active != self.recording_reported {
            self.recording_reported = active;
            crate::videohost::invoke("video.reportRecording", &serde_json::json!([active]).to_string());
        }
    }

    fn step(&mut self, video: &Video) {
        let wanted: Vec<Option<String>> = (0..crate::videohost::VIDEO_CHANNELS).map(crate::videohost::channel_pipeline).collect();
        if wanted.iter().any(Option::is_some) && !std::mem::replace(&mut self.decoders_ranked, true) {
            let forced = crate::settingsstore::raw_setting("settings.videoSettings.forceVideoDecoder").and_then(|v| v.as_i64()).unwrap_or(0);
            unsafe { (video.force_decoder)(forced as c_int) };
        }
        self.channels.iter_mut().zip(&wanted).enumerate().for_each(|(channel, (played, wanted))| played.stop_changed(video, channel as c_int, wanted));
        self.channels.iter_mut().zip(wanted).enumerate().for_each(|(channel, (played, wanted))| played.start_wanted(video, channel as c_int, wanted));
        self.record(video);
        self.channels.iter_mut().enumerate().for_each(|(channel, played)| played.report(video, channel as c_int));
    }
}

pub fn start(vm: &'static JavaVM) {
    let _ = std::thread::Builder::new().name("qgc-video".to_string()).spawn(move || {
        let Some(video) = video() else { return };
        if !initialise(vm, video).unwrap_or(false) {
            return;
        }
        let mut driver = Driver::default();
        loop {
            driver.step(video);
            std::thread::sleep(DRIVE_INTERVAL);
        }
    });
}
