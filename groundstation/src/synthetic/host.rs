use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::fetch::Network;
use super::overlay::Overlays;
use super::render::{Renderer, Targets};
use super::session::{Frame, Session, Stats, Target};
use super::window::Window;

const TICK: Duration = Duration::from_millis(16);
const TOKEN_EVERY: Duration = Duration::from_secs(1);
const OVERLAYS_EVERY: Duration = Duration::from_millis(500);
const SUMMARY_EVERY: Duration = Duration::from_secs(10);
const RELEASE_WAIT: Duration = Duration::from_secs(2);
const KEEP_UNSEEN: Duration = Duration::from_secs(60);
const WARM_PER_TICK: usize = 6;

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
    fn sticks(&self) -> Vec<crate::virtualjoystick::StickSample>;
    fn report(&self, draw: Draw);
}

enum Command {
    Attach(Window),
    Release(usize, SyncSender<()>),
    Wake,
}

struct Running {
    commands: Sender<Command>,
    thread: JoinHandle<()>,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

fn running() -> MutexGuard<'static, Option<Running>> {
    RUNNING.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn attach(window: Window, feed: Arc<dyn Feed>) -> bool {
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

pub fn wake() {
    if let Some(current) = running().as_ref() {
        current.commands.send(Command::Wake).ok();
    }
}

pub fn release(key: usize) {
    let (done, finished) = mpsc::sync_channel(1);
    let sent = running().as_ref().is_some_and(|current| current.commands.send(Command::Release(key, done)).is_ok());
    if sent && finished.recv_timeout(RELEASE_WAIT).is_err() {
        log::error!(target: "synthetic", "event=release_timeout window={key:#x} waited_ms={}", RELEASE_WAIT.as_millis());
    }
}

struct Kept {
    gpu: Gpu,
    session: Option<Session>,
    token: Throttled<Option<String>>,
    unseen_since: Instant,
}

impl Kept {
    fn surface_for(&self, window: &Window) -> Result<wgpu::Surface<'static>, String> {
        let surface = surface_for(&self.gpu.instance, window)?;
        match surface.get_capabilities(&self.gpu.adapter).formats.contains(&self.gpu.format) {
            true => Ok(surface),
            false => Err(format!("the window does not offer the kept format {:?}", self.gpu.format)),
        }
    }

    fn drop_world(self, reason: &str, detail: &str) {
        let (source, tiles, in_flight) = world(self.session.as_ref());
        log::info!(
            target: "synthetic",
            "event=world_dropped reason={reason} unseen_ms={} source={source} tiles={tiles} in_flight={in_flight} resident={} uploads={} detail=\"{detail}\"",
            self.unseen_since.elapsed().as_millis(),
            self.gpu.renderer.resident(),
            self.gpu.renderer.uploads,
        );
    }
}

fn world(session: Option<&Session>) -> (&'static str, usize, usize) {
    session.map_or(("none", 0, 0), |session| (session.source(), session.loaded_tiles(), session.in_flight()))
}

fn handing_over(session: Option<&Session>) -> (&'static str, String) {
    session.map_or(("none", "none".to_string()), |session| (session.showing(), session.handover().map_or_else(|| "none".to_string(), |after| after.as_millis().to_string())))
}

fn serve(commands: &Receiver<Command>, feed: &dyn Feed) {
    let mut kept: Option<Kept> = None;
    loop {
        let next = if kept.is_some() { commands.recv_timeout(KEEP_UNSEEN) } else { commands.recv().map_err(|_| RecvTimeoutError::Disconnected) };
        match next {
            Ok(Command::Attach(window)) => kept = show(window, kept.take(), commands, feed),
            Ok(Command::Release(_, done)) => acknowledge(&done),
            Ok(Command::Wake) => {}
            Err(RecvTimeoutError::Timeout) => {
                if let Some(expired) = kept.take() {
                    expired.drop_world("unseen", "");
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                if let Some(stopped) = kept.take() {
                    stopped.drop_world("shutdown", "");
                }
                return;
            }
        }
    }
}

struct Acquired {
    gpu: Gpu,
    surface: wgpu::Surface<'static>,
    session: Option<Session>,
    token: Throttled<Option<String>>,
    unseen: Option<Duration>,
}

fn acquire(first: &Window, kept: Option<Kept>) -> Result<Acquired, String> {
    let reused = kept.and_then(|kept| match kept.surface_for(first) {
        Ok(surface) => Some((kept, surface)),
        Err(reason) => {
            kept.drop_world("reuse_failed", &reason);
            None
        }
    });
    match reused {
        Some((kept, surface)) => Ok(Acquired { unseen: Some(kept.unseen_since.elapsed()), gpu: kept.gpu, surface, session: kept.session, token: kept.token }),
        None => Gpu::new(first).map(|(gpu, surface)| Acquired { gpu, surface, session: None, token: Throttled::new(TOKEN_EVERY, None), unseen: None }),
    }
}

struct Visit {
    opened: Instant,
    unseen: Option<Duration>,
    gpu_setup: Duration,
    first_picture: Option<Duration>,
    attaches: u32,
    releases: u32,
    surfaces_peak: usize,
    painted: u64,
    slowest: Duration,
    paint_failures: usize,
    ended: &'static str,
}

impl Visit {
    fn new(opened: Instant, unseen: Option<Duration>) -> Self {
        Self { opened, unseen, gpu_setup: opened.elapsed(), first_picture: None, attaches: 1, releases: 0, surfaces_peak: 1, painted: 0, slowest: Duration::ZERO, paint_failures: 0, ended: "released" }
    }

    fn note(&mut self, command: &Command) {
        match command {
            Command::Attach(_) => self.attaches += 1,
            Command::Release(..) => self.releases += 1,
            Command::Wake => {}
        }
    }

    fn report(&self, session: Option<&Session>, gpu: &Gpu, draw: Draw) {
        let (source, tiles, in_flight) = world(session);
        let (showing, handover) = handing_over(session);
        log::info!(
            target: "synthetic",
            "event=view_shown source={source} showing={showing} handover_ms={handover} reused={} unseen_ms={} gpu_setup_ms={} first_picture_ms={} visible_ms={} attaches={} releases={} surfaces_peak={} painted={} slowest_paint_ms={} paint_failures={} tiles={tiles} in_flight={in_flight} resident={} uploads={} draw={} ended={}",
            self.unseen.is_some(),
            self.unseen.map_or(0, |unseen| unseen.as_millis()),
            self.gpu_setup.as_millis(),
            self.first_picture.map_or_else(|| "none".to_string(), |at| at.as_millis().to_string()),
            self.opened.elapsed().as_millis(),
            self.attaches,
            self.releases,
            self.surfaces_peak,
            self.painted,
            self.slowest.as_millis(),
            self.paint_failures,
            gpu.renderer.resident(),
            gpu.renderer.uploads,
            draw.name(),
            self.ended,
        );
    }
}

fn acknowledge(done: &SyncSender<()>) {
    if done.send(()).is_err() {
        log::warn!(target: "synthetic", "event=release_late reason=\"the surface owner stopped waiting\"");
    }
}

fn surface_for(instance: &wgpu::Instance, window: &Window) -> Result<wgpu::Surface<'static>, String> {
    unsafe { instance.create_surface_unsafe(window.target()) }.map_err(|error| format!("no surface for the window: {error}"))
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
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::PRIMARY | wgpu::Backends::GL, ..wgpu::InstanceDescriptor::new_without_display_handle() });
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
    targets: Option<Targets>,
    drawn: bool,
    window: Window,
    acquire_slowest: Duration,
    draw_slowest: Duration,
    present: wgpu::PresentMode,
}

impl Shown {
    fn new(window: Window, surface: wgpu::Surface<'static>) -> Self {
        Self { surface, size: (0, 0), targets: None, drawn: false, window, acquire_slowest: Duration::ZERO, draw_slowest: Duration::ZERO, present: wgpu::PresentMode::Fifo }
    }

    fn fit(&mut self, gpu: &Gpu) {
        let size = self.window.size();
        if size == self.size || size.0 == 0 || size.1 == 0 {
            return;
        }
        let Some(config) = self.surface.get_default_config(&gpu.adapter, size.0, size.1) else { return };
        self.present = match self.surface.get_capabilities(&gpu.adapter).present_modes.contains(&wgpu::PresentMode::Mailbox) {
            true => wgpu::PresentMode::Mailbox,
            false => wgpu::PresentMode::Fifo,
        };
        let desired_maximum_frame_latency = match self.present {
            wgpu::PresentMode::Mailbox => 1,
            _ => config.desired_maximum_frame_latency,
        };
        self.surface.configure(&gpu.device, &wgpu::SurfaceConfiguration { format: gpu.format, present_mode: self.present, desired_maximum_frame_latency, ..config });
        self.targets = Some(gpu.renderer.targets(size.0, size.1));
        self.size = size;
        self.drawn = false;
    }

    fn paint(&mut self, gpu: &mut Gpu, frame: Option<&Frame>, overlays: Option<&Overlays>) -> Result<bool, String> {
        if self.targets.is_none() {
            return Ok(false);
        }
        let acquiring = Instant::now();
        let acquired = self.surface.get_current_texture();
        self.acquire_slowest = self.acquire_slowest.max(acquiring.elapsed());
        let texture = match acquired {
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
        let Some(targets) = self.targets.as_ref() else { return Ok(false) };
        let drawing = Instant::now();
        let view = texture.texture.create_view(&Default::default());
        let pictured = match frame {
            Some(frame) => {
                let (camera, meshes) = frame.view(overlays, self.size.0, self.size.1);
                gpu.renderer.draw(&view, targets, &camera, &frame.backfill, &meshes);
                !frame.ground.is_empty()
            }
            None => {
                gpu.renderer.clear(&view, targets);
                false
            }
        };
        gpu.renderer.present(texture);
        self.draw_slowest = self.draw_slowest.max(drawing.elapsed());
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
    reading_ages_ms: Vec<u64>,
    stick_to_frame_ms: Vec<u64>,
}

impl Summary {
    fn new(since: Instant) -> Self {
        Self { since, painted: 0, idle: 0, slowest: Duration::ZERO, reading_ages_ms: Vec::new(), stick_to_frame_ms: Vec::new() }
    }

    fn report(&self, session: Option<&mut Session>, gpu: &Gpu, shown: &mut [Shown], draw: Draw) {
        let surfaces = shown.len();
        let acquire_slowest = shown.iter().map(|surface| surface.acquire_slowest).max().unwrap_or_default();
        let draw_slowest = shown.iter().map(|surface| surface.draw_slowest).max().unwrap_or_default();
        let present = shown.first().map_or_else(|| "none".to_string(), |surface| format!("{:?}", surface.present).to_lowercase());
        shown.iter_mut().for_each(|surface| {
            surface.acquire_slowest = Duration::ZERO;
            surface.draw_slowest = Duration::ZERO;
        });
        let (source, tiles, in_flight) = world(session.as_deref());
        let (showing, handover) = handing_over(session.as_deref());
        let warming = session.as_deref().and_then(Session::warming).map_or_else(|| "none".to_string(), |left| left.to_string());
        let stats = session.map(Session::take_stats).unwrap_or_default();
        let Stats { requested, loaded, failed, bytes, errors } = stats;
        log::info!(
            target: "synthetic",
            "event=summary seconds={} source={source} showing={showing} handover_ms={handover} warming={warming} surfaces={surfaces} present={present} draw={} painted={} idle_ticks={} slowest_paint_ms={} acquire_slowest_ms={} draw_slowest_ms={} reading_age_p50_ms={} reading_age_max_ms={} stick_to_frame_p50_ms={} stick_to_frame_max_ms={} stick_frames={} requested={requested} loaded={loaded} failed={failed} bytes={bytes} tiles={tiles} in_flight={in_flight} resident={} uploads={} errors={errors:?}",
            self.since.elapsed().as_secs(),
            draw.name(),
            self.painted,
            self.idle,
            self.slowest.as_millis(),
            acquire_slowest.as_millis(),
            draw_slowest.as_millis(),
            crate::stats::shown(crate::stats::median(&self.reading_ages_ms)),
            crate::stats::shown(self.reading_ages_ms.iter().copied().max()),
            crate::stats::shown(crate::stats::median(&self.stick_to_frame_ms)),
            crate::stats::shown(self.stick_to_frame_ms.iter().copied().max()),
            self.stick_to_frame_ms.len(),
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
            Some(existing) => {
                existing.window = window;
                existing.drawn = false;
            }
            None => match surface_for(&gpu.instance, &window) {
                Ok(surface) => shown.push(Shown::new(window, surface)),
                Err(reason) => log::error!(target: "synthetic", "event=attach_failed window={:#x} reason=\"{reason}\"", window.key()),
            },
        },
        Command::Release(key, done) => {
            shown.retain(|existing| existing.window.key() != key);
            acknowledge(&done);
        }
        Command::Wake => {}
    }
}

fn show(first: Window, kept: Option<Kept>, commands: &Receiver<Command>, feed: &dyn Feed) -> Option<Kept> {
    let opened = Instant::now();
    let Acquired { mut gpu, surface, mut session, mut token, unseen } = match acquire(&first, kept) {
        Ok(acquired) => acquired,
        Err(reason) => {
            log::error!(target: "synthetic", "event=gpu_failed reason=\"{reason}\"");
            feed.report(Draw::Failed);
            return None;
        }
    };
    let mut visit = Visit::new(opened, unseen);
    let mut shown = vec![Shown::new(first, surface)];
    let mut overlays = Throttled::new(OVERLAYS_EVERY, None::<Overlays>);
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
        let sticks = feed.sticks();
        let largest = shown.iter().map(|surface| surface.size).max_by_key(|(width, height)| u64::from(*width) * u64::from(*height)).unwrap_or_default();
        let mut frame = session.as_mut().filter(|_| largest.0 > 0 && largest.1 > 0).and_then(|session: &mut Session| session.frame(target.as_ref(), &sticks, largest.0, largest.1, now));
        if let Some((frame, session)) = frame.as_mut().zip(session.as_mut()) {
            session.warmed(gpu.renderer.warm(std::mem::take(&mut frame.warm), WARM_PER_TICK));
        }
        let redraw = rekeyed || restyled || frame.as_ref().is_some_and(|frame| frame.changed) || frame.is_some() != framed;
        framed = frame.is_some();
        let painted: Vec<Result<bool, String>> = shown.iter_mut().filter(|surface| redraw || !surface.drawn).map(|surface| surface.paint(&mut gpu, frame.as_ref(), overlays.value.as_ref())).collect();
        painted.iter().filter_map(|result| result.as_ref().err()).for_each(|reason| log::error!(target: "synthetic", "event=paint_failed reason=\"{reason}\""));
        visit.paint_failures += painted.iter().filter(|result| result.is_err()).count();
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
        if draw == Draw::Drawn && visit.first_picture.is_none() {
            visit.first_picture = Some(opened.elapsed());
        }
        match painted.is_empty() {
            true => summary.idle += 1,
            false => {
                summary.painted += 1;
                summary.slowest = summary.slowest.max(now.elapsed());
                summary.reading_ages_ms.extend(frame.as_ref().map(|frame| u64::try_from(frame.reading_age.as_millis()).unwrap_or(u64::MAX)));
                summary.stick_to_frame_ms.extend(frame.as_ref().and_then(|frame| frame.stick_to_frame).map(|touched| u64::try_from((touched + now.elapsed()).as_millis()).unwrap_or(u64::MAX)));
                visit.painted += 1;
                visit.slowest = visit.slowest.max(now.elapsed());
            }
        }
        if summary.since.elapsed() >= SUMMARY_EVERY {
            summary.report(session.as_mut(), &gpu, &mut shown, draw);
            summary = Summary::new(Instant::now());
        }
        match commands.recv_timeout(TICK.saturating_sub(now.elapsed())) {
            Ok(command) => {
                visit.note(&command);
                apply(command, &mut shown, &gpu);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                visit.ended = "disconnected";
                shown.clear();
            }
        }
        commands.try_iter().for_each(|command| {
            visit.note(&command);
            apply(command, &mut shown, &gpu);
        });
        visit.surfaces_peak = visit.surfaces_peak.max(shown.len());
    }
    visit.report(session.as_ref(), &gpu, draw);
    feed.report(Draw::Loading);
    Some(Kept { gpu, session, token, unseen_since: Instant::now() })
}
