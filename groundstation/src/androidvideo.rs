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
    pub set_surface: unsafe extern "C" fn(*mut jni::sys::JNIEnv, jni::sys::jobject) -> bool,
    force_decoder: unsafe extern "C" fn(c_int),
    start: unsafe extern "C" fn(*const c_char) -> bool,
    stop: unsafe extern "C" fn(),
    running: unsafe extern "C" fn() -> bool,
    pub width: unsafe extern "C" fn() -> c_int,
    pub height: unsafe extern "C" fn() -> c_int,
    pub frames: unsafe extern "C" fn() -> i64,
    source_buffers: unsafe extern "C" fn() -> i64,
    last_error: unsafe extern "C" fn() -> *const c_char,
    pub copy_frame: unsafe extern "C" fn(*mut c_void, c_int, *mut c_int, *mut c_int, *mut c_int) -> bool,
    start_recording: unsafe extern "C" fn(*const c_char, c_int) -> bool,
    stop_recording: unsafe extern "C" fn(),
    recording: unsafe extern "C" fn() -> bool,
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
        copy_frame: symbol(handle, c"qgc_video_copy_frame")?,
        start_recording: symbol(handle, c"qgc_video_start_recording")?,
        stop_recording: symbol(handle, c"qgc_video_stop_recording")?,
        recording: symbol(handle, c"qgc_video_recording")?,
    })
}

pub fn video() -> Option<&'static Video> {
    VIDEO.get_or_init(load).as_ref()
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
struct Driver {
    driven: Option<String>,
    restarted: bool,
    decoders_ranked: bool,
    error: String,
    recording: Option<serde_json::Value>,
    recording_reported: bool,
}

impl Driver {
    fn record(&mut self, video: &Video) {
        let wanted = crate::videohost::native_recording();
        if wanted != self.recording {
            if self.recording.is_some() {
                unsafe { (video.stop_recording)() };
            }
            if let Some((file, format)) = wanted.as_ref().and_then(|w| Some((CString::new(w.get("file")?.as_str()?).ok()?, w.get("format")?.as_i64()?))) {
                unsafe { (video.start_recording)(file.as_ptr(), format as c_int) };
            }
            self.recording = wanted;
        }
        let active = unsafe { (video.recording)() };
        if active != self.recording_reported {
            self.recording_reported = active;
            crate::videohost::invoke("video.reportRecording", &serde_json::json!([active]).to_string());
        }
    }

    fn step(&mut self, video: &Video) {
        if crate::videohost::RESTART.swap(false, std::sync::atomic::Ordering::Relaxed) && self.driven.take().is_some() {
            unsafe { (video.stop)() };
        }
        let wanted = crate::videohost::native_pipeline();
        if wanted != self.driven {
            match &wanted {
                Some(pipeline) => {
                    if !std::mem::replace(&mut self.decoders_ranked, true) {
                        let forced = crate::settingsstore::raw_setting("settings.videoSettings.forceVideoDecoder").and_then(|v| v.as_i64()).unwrap_or(0);
                        unsafe { (video.force_decoder)(forced as c_int) };
                    }
                    let text = CString::new(pipeline.as_str()).unwrap_or_default();
                    let started = unsafe { (video.start)(text.as_ptr()) };
                    self.restarted = true;
                    self.error = if started { String::new() } else { unsafe { CStr::from_ptr((video.last_error)()) }.to_string_lossy().into_owned() };
                }
                None => unsafe { (video.stop)() },
            }
            self.driven = wanted;
        }
        self.record(video);
        if self.driven.is_none() {
            return;
        }
        let (running, frames, width, height) = unsafe { ((video.running)(), (video.frames)(), (video.width)(), (video.height)()) };
        crate::videostats::sample(running, frames, i64::from(height), crate::hub::now_ms());
        let source = unsafe { (video.source_buffers)() };
        crate::videohost::invoke("video.reportNative", &serde_json::json!([running, frames, width, height, self.error, source, std::mem::take(&mut self.restarted)]).to_string());
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
