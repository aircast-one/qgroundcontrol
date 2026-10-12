use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use wgpu::rwh::{AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle};

use super::fetch::Network;
use super::overlay::Overlays;
use super::render::Renderer;
use super::session::{Frame, Session, Stats, Target};

const TICK: Duration = Duration::from_millis(33);
const TOKEN_EVERY: Duration = Duration::from_secs(1);
const OVERLAYS_EVERY: Duration = Duration::from_millis(500);
const SUMMARY_EVERY: Duration = Duration::from_secs(10);
const RELEASE_WAIT: Duration = Duration::from_secs(2);

#[link(name = "android")]
unsafe extern "C" {
    fn ANativeWindow_fromSurface(env: *mut jni::sys::JNIEnv, surface: jni::sys::jobject) -> *mut c_void;
    fn ANativeWindow_release(window: *mut c_void);
    fn ANativeWindow_getWidth(window: *mut c_void) -> i32;
    fn ANativeWindow_getHeight(window: *mut c_void) -> i32;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Draw {
    Loading,
    Drawn,
    Failed,
}

impl Draw {
    pub fn name(self) -> &'static str {
        match self {
            Draw::Loading => "loading",
            Draw::Drawn => "drawn",
            Draw::Failed => "failed",
        }
    }
}

pub trait Feed: Send + Sync {
    fn pose(&self) -> Option<serde_json::Value>;
    fn overlays(&self) -> Option<serde_json::Value>;
    fn token(&self) -> String;
    fn report(&self, draw: Draw);
}

struct Window(NonNull<c_void>);

unsafe impl Send for Window {}

impl Window {
    fn from_surface(env: *mut jni::sys::JNIEnv, surface: jni::sys::jobject) -> Option<Self> {
        (!surface.is_null()).then(|| NonNull::new(unsafe { ANativeWindow_fromSurface(env, surface) })).flatten().map(Window)
    }

    fn key(&self) -> usize {
        self.0.as_ptr() as usize
    }

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

enum Command {
    Attach(Window),
    Release(usize, SyncSender<()>),
}

struct Running {
    commands: Sender<Command>,
    thread: JoinHandle<()>,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

fn running() -> MutexGuard<'static, Option<Running>> {
    RUNNING.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn attach(env: *mut jni::sys::JNIEnv, surface: jni::sys::jobject, feed: Arc<dyn Feed>) -> bool {
    let Some(window) = Window::from_surface(env, surface) else {
        log::error!(target: "synthetic", "event=attach_failed reason=\"the surface has no native window\"");
        return false;
    };
    let mut running = running();
    if running.as_ref().is_none_or(|current| current.thread.is_finished()) {
        let (commands, received) = mpsc::channel();
        match std::thread::Builder::new().name("synthetic-view".to_string()).spawn(move || serve(&received, feed.as_ref())) {
            Ok(thread) => *running = Some(Running { commands, thread }),
            Err(error) => {
                log::error!(target: "synthetic", "event=attach_failed reason=\"no render thread: {error}\"");
                return false;
            }
        }
    }
    running.as_ref().is_some_and(|current| current.commands.send(Command::Attach(window)).is_ok())
}

pub fn release(env: *mut jni::sys::JNIEnv, surface: jni::sys::jobject) {
    let Some(window) = Window::from_surface(env, surface) else { return };
    let (done, finished) = mpsc::sync_channel(1);
    let sent = running().as_ref().is_some_and(|current| current.commands.send(Command::Release(window.key(), done)).is_ok());
    if sent && finished.recv_timeout(RELEASE_WAIT).is_err() {
        log::error!(target: "synthetic", "event=release_timeout window={:#x} waited_ms={}", window.key(), RELEASE_WAIT.as_millis());
    }
}

fn serve(commands: &Receiver<Command>, feed: &dyn Feed) {
    while let Ok(command) = commands.recv() {
        match command {
            Command::Attach(window) => show(window, commands, feed),
            Command::Release(_, done) => acknowledge(&done),
        }
    }
}

fn acknowledge(done: &SyncSender<()>) {
    if done.send(()).is_err() {
        log::warn!(target: "synthetic", "event=release_late reason=\"the surface owner stopped waiting\"");
    }
}

fn surface_for(instance: &wgpu::Instance, window: &Window) -> Result<wgpu::Surface<'static>, String> {
    let target = wgpu::SurfaceTargetUnsafe::RawHandle {
        raw_display_handle: Some(RawDisplayHandle::Android(AndroidDisplayHandle::new())),
        raw_window_handle: RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(window.0)),
    };
    unsafe { instance.create_surface_unsafe(target) }.map_err(|error| format!("no surface for the window: {error}"))
}

struct Gpu {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    format: wgpu::TextureFormat,
    renderer: Renderer,
}

impl Gpu {
    fn new(first: &Window) -> Result<(Self, wgpu::Surface<'static>), String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::VULKAN | wgpu::Backends::GL, ..wgpu::InstanceDescriptor::new_without_display_handle() });
        let surface = surface_for(&instance, first)?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions { compatible_surface: Some(&surface), ..Default::default() })).map_err(|error| format!("no GPU adapter for the window: {error}"))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor { required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()), ..Default::default() }))
            .map_err(|error| format!("the GPU refused a device: {error}"))?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities.formats.iter().copied().find(wgpu::TextureFormat::is_srgb).or_else(|| capabilities.formats.first().copied()).ok_or("the window offers no colour format")?;
        let renderer = Renderer::with_device(device.clone(), queue, format);
        Ok((Self { instance, adapter, device, format, renderer }, surface))
    }
}

struct Shown {
    surface: wgpu::Surface<'static>,
    size: (u32, u32),
    depth: Option<wgpu::TextureView>,
    drawn: bool,
    window: Window,
}

impl Shown {
    fn new(window: Window, surface: wgpu::Surface<'static>) -> Self {
        Self { surface, size: (0, 0), depth: None, drawn: false, window }
    }

    fn fit(&mut self, gpu: &Gpu) {
        let size = self.window.size();
        if size == self.size || size.0 == 0 || size.1 == 0 {
            return;
        }
        let Some(config) = self.surface.get_default_config(&gpu.adapter, size.0, size.1) else { return };
        self.surface.configure(&gpu.device, &wgpu::SurfaceConfiguration { format: gpu.format, present_mode: wgpu::PresentMode::Fifo, ..config });
        self.depth = Some(gpu.renderer.depth(size.0, size.1));
        self.size = size;
        self.drawn = false;
    }

    fn paint(&mut self, gpu: &mut Gpu, frame: Option<&Frame>, overlays: Option<&Overlays>) -> Result<bool, String> {
        if self.depth.is_none() {
            return Ok(false);
        }
        let texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return Ok(false),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.size = (0, 0);
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = surface_for(&gpu.instance, &self.window)?;
                self.size = (0, 0);
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Validation => return Err("the surface refused a frame".to_string()),
        };
        let Some(depth) = self.depth.as_ref() else { return Ok(false) };
        let view = texture.texture.create_view(&Default::default());
        let pictured = match frame {
            Some(frame) => {
                let (camera, meshes) = frame.view(overlays, self.size.0, self.size.1);
                gpu.renderer.draw(&view, depth, &camera, &meshes);
                !frame.ground.is_empty()
            }
            None => {
                gpu.renderer.clear(&view, depth);
                false
            }
        };
        gpu.renderer.present(texture);
        self.drawn = true;
        Ok(pictured)
    }
}

struct Throttled<T> {
    every: Duration,
    read: Option<Instant>,
    value: T,
}

impl<T: PartialEq> Throttled<T> {
    fn new(every: Duration, value: T) -> Self {
        Self { every, read: None, value }
    }

    fn refresh(&mut self, now: Instant, read: impl FnOnce() -> T) -> bool {
        if self.read.is_some_and(|at| now.duration_since(at) < self.every) {
            return false;
        }
        self.read = Some(now);
        let value = read();
        let changed = value != self.value;
        self.value = value;
        changed
    }
}

struct Summary {
    since: Instant,
    painted: u64,
    idle: u64,
    slowest: Duration,
}

impl Summary {
    fn new(since: Instant) -> Self {
        Self { since, painted: 0, idle: 0, slowest: Duration::ZERO }
    }

    fn report(&self, session: Option<&mut Session>, gpu: &Gpu, surfaces: usize, draw: Draw) {
        let (source, tiles, in_flight) = session.as_ref().map_or(("none", 0, 0), |session| (session.source(), session.loaded_tiles(), session.in_flight()));
        let stats = session.map(Session::take_stats).unwrap_or_default();
        let Stats { requested, loaded, failed, bytes, errors } = stats;
        log::info!(
            target: "synthetic",
            "event=summary seconds={} source={source} surfaces={surfaces} draw={} painted={} idle_ticks={} slowest_paint_ms={} requested={requested} loaded={loaded} failed={failed} bytes={bytes} tiles={tiles} in_flight={in_flight} resident={} uploads={} errors={errors:?}",
            self.since.elapsed().as_secs(),
            draw.name(),
            self.painted,
            self.idle,
            self.slowest.as_millis(),
            gpu.renderer.resident(),
            gpu.renderer.uploads,
        );
    }
}

fn start_session(token: &str) -> Option<Session> {
    Session::new(token, Arc::new(Network))
        .inspect_err(|error| log::error!(target: "synthetic", "event=session_failed source={} reason=\"{error}\"", if token.is_empty() { "open" } else { "google" }))
        .ok()
}

fn apply(command: Command, shown: &mut Vec<Shown>, gpu: &Gpu) {
    match command {
        Command::Attach(window) => match shown.iter_mut().find(|existing| existing.window.key() == window.key()) {
            Some(existing) => existing.drawn = false,
            None => match surface_for(&gpu.instance, &window) {
                Ok(surface) => shown.push(Shown::new(window, surface)),
                Err(reason) => log::error!(target: "synthetic", "event=attach_failed window={:#x} reason=\"{reason}\"", window.key()),
            },
        },
        Command::Release(key, done) => {
            shown.retain(|existing| existing.window.key() != key);
            acknowledge(&done);
        }
    }
}

fn show(first: Window, commands: &Receiver<Command>, feed: &dyn Feed) {
    let (mut gpu, surface) = match Gpu::new(&first) {
        Ok(ready) => ready,
        Err(reason) => {
            log::error!(target: "synthetic", "event=gpu_failed reason=\"{reason}\"");
            feed.report(Draw::Failed);
            return;
        }
    };
    let mut shown = vec![Shown::new(first, surface)];
    let mut token = Throttled::new(TOKEN_EVERY, None::<String>);
    let mut overlays = Throttled::new(OVERLAYS_EVERY, None::<Overlays>);
    let mut session = None;
    let mut summary = Summary::new(Instant::now());
    let mut framed = false;
    let mut draw = Draw::Loading;
    feed.report(draw);
    while !shown.is_empty() {
        let now = Instant::now();
        shown.iter_mut().for_each(|surface| surface.fit(&gpu));
        let rekeyed = token.refresh(now, || Some(feed.token()));
        if rekeyed {
            session = start_session(token.value.as_deref().unwrap_or_default());
        }
        let restyled = overlays.refresh(now, || feed.overlays().as_ref().and_then(Overlays::from_view));
        let target = feed.pose().as_ref().and_then(Target::from_view);
        let largest = shown.iter().map(|surface| surface.size).max_by_key(|(width, height)| u64::from(*width) * u64::from(*height)).unwrap_or_default();
        let frame = session.as_mut().filter(|_| largest.0 > 0 && largest.1 > 0).and_then(|session: &mut Session| session.frame(target.as_ref(), largest.0, largest.1, now));
        let redraw = rekeyed || restyled || frame.as_ref().is_some_and(|frame| frame.changed) || frame.is_some() != framed;
        framed = frame.is_some();
        let painted: Vec<Result<bool, String>> = shown.iter_mut().filter(|surface| redraw || !surface.drawn).map(|surface| surface.paint(&mut gpu, frame.as_ref(), overlays.value.as_ref())).collect();
        painted.iter().filter_map(|result| result.as_ref().err()).for_each(|reason| log::error!(target: "synthetic", "event=paint_failed reason=\"{reason}\""));
        let state = match (session.is_some(), painted.iter().any(|result| result == &Ok(true))) {
            (false, _) => Draw::Failed,
            (true, true) => Draw::Drawn,
            (true, false) if painted.is_empty() => draw,
            (true, false) => Draw::Loading,
        };
        if state != draw {
            draw = state;
            feed.report(draw);
        }
        match painted.is_empty() {
            true => summary.idle += 1,
            false => {
                summary.painted += 1;
                summary.slowest = summary.slowest.max(now.elapsed());
            }
        }
        if summary.since.elapsed() >= SUMMARY_EVERY {
            summary.report(session.as_mut(), &gpu, shown.len(), draw);
            summary = Summary::new(Instant::now());
        }
        match commands.recv_timeout(TICK.saturating_sub(now.elapsed())) {
            Ok(command) => apply(command, &mut shown, &gpu),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => shown.clear(),
        }
        commands.try_iter().for_each(|command| apply(command, &mut shown, &gpu));
    }
    feed.report(Draw::Loading);
}
