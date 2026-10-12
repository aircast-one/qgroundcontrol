use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use wgpu::rwh::{AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle};

use super::render::Renderer;
use super::session::{Session, Target};

const FRAME: Duration = Duration::from_millis(33);
pub const TOKEN_SETTING: &str = "settings.appSettings.photorealToken";

#[link(name = "android")]
unsafe extern "C" {
    fn ANativeWindow_fromSurface(env: *mut jni::sys::JNIEnv, surface: jni::sys::jobject) -> *mut c_void;
    fn ANativeWindow_release(window: *mut c_void);
    fn ANativeWindow_getWidth(window: *mut c_void) -> i32;
    fn ANativeWindow_getHeight(window: *mut c_void) -> i32;
}

struct Window(NonNull<c_void>);

unsafe impl Send for Window {}

impl Window {
    fn size(&self) -> (u32, u32) {
        let width = unsafe { ANativeWindow_getWidth(self.0.as_ptr()) };
        let height = unsafe { ANativeWindow_getHeight(self.0.as_ptr()) };
        (u32::try_from(width).unwrap_or(0), u32::try_from(height).unwrap_or(0))
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe { ANativeWindow_release(self.0.as_ptr()) }
    }
}

struct Running {
    window: usize,
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);
static SESSION: Mutex<Option<Session>> = Mutex::new(None);

pub fn token() -> String {
    crate::settingsstore::raw_setting(TOKEN_SETTING).and_then(|value| value.as_str().map(str::trim).map(str::to_string)).unwrap_or_default()
}

pub fn set_surface(env: *mut jni::sys::JNIEnv, surface: jni::sys::jobject) -> bool {
    let window = match surface.is_null() {
        true => None,
        false => NonNull::new(unsafe { ANativeWindow_fromSurface(env, surface) }).map(Window),
    };
    let mut running = RUNNING.lock().unwrap_or_else(PoisonError::into_inner);
    if let (Some(window), Some(current)) = (&window, running.as_ref())
        && current.window == window.0.as_ptr() as usize
    {
        return true;
    }
    if let Some(current) = running.take() {
        current.stop.store(true, Ordering::Relaxed);
        let _ = current.thread.join();
    }
    let Some(window) = window else { return surface.is_null() };
    let stop = Arc::new(AtomicBool::new(false));
    let key = window.0.as_ptr() as usize;
    let flag = stop.clone();
    let started = std::thread::Builder::new().name("photoreal".to_string()).spawn(move || render(window, &flag));
    *running = started.ok().map(|thread| Running { window: key, stop, thread });
    running.is_some()
}

fn render(window: Window, stop: &AtomicBool) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::VULKAN | wgpu::Backends::GL, ..wgpu::InstanceDescriptor::new_without_display_handle() });
    let target = wgpu::SurfaceTargetUnsafe::RawHandle {
        raw_display_handle: Some(RawDisplayHandle::Android(AndroidDisplayHandle::new())),
        raw_window_handle: RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(window.0)),
    };
    let Ok(surface) = (unsafe { instance.create_surface_unsafe(target) }) else {
        log::warn!("Photorealistic view: no surface for the window");
        return;
    };
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions { compatible_surface: Some(&surface), ..Default::default() })) else {
        log::warn!("Photorealistic view: no GPU adapter for the window");
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor { required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()), ..Default::default() })) else {
        log::warn!("Photorealistic view: the GPU refused a device");
        return;
    };
    let capabilities = surface.get_capabilities(&adapter);
    let Some(format) = capabilities.formats.iter().copied().find(wgpu::TextureFormat::is_srgb).or_else(|| capabilities.formats.first().copied()) else { return };
    let mut renderer = Renderer::with_device(device.clone(), queue, format);
    let mut configured = (0, 0);
    let mut depth = None;
    while !stop.load(Ordering::Relaxed) {
        let started = Instant::now();
        let (width, height) = window.size();
        if let Some(config) = surface.get_default_config(&adapter, width, height).filter(|_| (width, height) != configured && width > 0 && height > 0) {
            surface.configure(&device, &wgpu::SurfaceConfiguration { format, present_mode: wgpu::PresentMode::Fifo, ..config });
            depth = Some(renderer.depth(width, height));
            configured = (width, height);
        }
        let view: Option<serde_json::Value> = serde_json::from_str(&crate::abi::core_get("view.syntheticView")).ok();
        let target = view.as_ref().and_then(Target::from_view);
        let token = token();
        let drawn = {
            let mut session = SESSION.lock().unwrap_or_else(PoisonError::into_inner);
            if session.as_ref().is_none_or(|session| session.token() != token) {
                *session = Some(Session::new(&token));
            }
            session.as_mut().and_then(|session| session.frame(target.as_ref(), configured.0, configured.1))
        };
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => Some(frame),
            wgpu::CurrentSurfaceTexture::Outdated => {
                configured = (0, 0);
                None
            }
            _ => None,
        };
        if let (Some(frame), Some(depth)) = (frame, depth.as_ref()) {
            let view = frame.texture.create_view(&Default::default());
            match drawn {
                Some((camera, meshes)) => renderer.draw(&view, depth, &camera, &meshes),
                None => renderer.clear(&view, depth),
            }
            renderer.present(frame);
        }
        std::thread::sleep(FRAME.saturating_sub(started.elapsed()));
    }
}
