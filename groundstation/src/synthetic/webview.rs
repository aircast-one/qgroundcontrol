use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_time::Instant;

use super::fetch::Network;
use super::open::Detail;
use super::overlay::Overlays;
use super::render::{Renderer, Targets};
use super::session::{Frame, Session, Target};

const TOKEN_EVERY: Duration = Duration::from_secs(1);
const OVERLAYS_EVERY: Duration = Duration::from_millis(500);
const WARM_PER_TICK: usize = 6;
const SYNTHETIC_TOKEN: &str = "settings.appSettings.photorealToken";
const LOADING: &str = "loading";
const DRAWN: &str = "drawn";
const FAILED: &str = "failed";

struct View {
    key: u32,
    canvas: web_sys::OffscreenCanvas,
    wanted: (u32, u32),
    surface: Option<wgpu::Surface<'static>>,
    refused: bool,
    size: (u32, u32),
    targets: Option<Targets>,
    drawn: bool,
}

struct Gpu {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    canvas_format: wgpu::TextureFormat,
    format: wgpu::TextureFormat,
    renderer: Renderer,
}

struct Throttled<T> {
    every: Duration,
    read: Option<Instant>,
    value: T,
}

struct World {
    gpu: Gpu,
    session: Option<Session>,
    token: Throttled<Option<String>>,
    overlays: Throttled<Option<Overlays>>,
    framed: bool,
    draw: &'static str,
}

enum Start {
    Idle,
    Starting(u32),
    Ready(Box<World>),
    Failed,
}

struct Web {
    views: Vec<View>,
    start: Start,
    keys: u32,
    starts: u32,
    polling: bool,
}

thread_local! {
    static WEB: RefCell<Web> = const { RefCell::new(Web { views: Vec::new(), start: Start::Idle, keys: 0, starts: 0, polling: false }) };
}

pub fn attach(canvas: web_sys::OffscreenCanvas, width: u32, height: u32) -> u32 {
    WEB.with(|web| {
        let mut web = web.borrow_mut();
        web.keys += 1;
        let key = web.keys;
        web.views.push(View {
            key,
            canvas,
            wanted: (width, height),
            surface: None,
            refused: false,
            size: (0, 0),
            targets: None,
            drawn: false,
        });
        if matches!(web.start, Start::Failed) {
            web.start = Start::Idle;
        }
        if !web.polling {
            web.polling = true;
            next_frame();
        }
        key
    })
}

pub fn resize(key: u32, width: u32, height: u32) {
    WEB.with(|web| {
        web.borrow_mut()
            .views
            .iter_mut()
            .filter(|view| view.key == key)
            .for_each(|view| view.wanted = (width, height))
    });
}

pub fn release(key: u32) {
    WEB.with(|web| web.borrow_mut().views.retain(|view| view.key != key));
}

fn tick() -> bool {
    WEB.with(|web| web.borrow_mut().tick())
}

pub fn draw_now() {
    WEB.with(|web| {
        let web = &mut *web.borrow_mut();
        if let Start::Ready(world) = &mut web.start {
            world.show(&mut web.views);
        }
    });
}

fn next_frame() {
    let frame = Closure::once_into_js(|| {
        if !tick() {
            next_frame();
        }
    });
    let scope: web_sys::DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
    if let Err(error) = scope.request_animation_frame(frame.unchecked_ref()) {
        log::error!(target: "synthetic", "event=attach_failed reason=\"no render loop: {error:?}\"");
    }
}

fn read(path: &str) -> Option<serde_json::Value> {
    serde_json::from_str(&crate::abi::core_get(path)).ok()
}

fn token() -> String {
    crate::settingsstore::raw_setting(SYNTHETIC_TOKEN)
        .and_then(|value| value.as_str().map(str::trim).map(str::to_string))
        .unwrap_or_default()
}

fn start_session(token: &str, detail: Detail) -> Option<Session> {
    Session::new(token, Arc::new(Network), detail)
        .inspect_err(|error| log::error!(target: "synthetic", "event=session_failed source={} reason=\"{error}\"", if token.is_empty() { "open" } else { "google" }))
        .ok()
}

impl Web {
    fn tick(&mut self) -> bool {
        if self.views.is_empty() {
            self.start = Start::Idle;
            self.polling = false;
            crate::synthview::set_draw(LOADING);
            return true;
        }
        if let Start::Ready(world) = &mut self.start
            && !world.adopt(&mut self.views)
        {
            self.views.iter_mut().for_each(View::forget);
            self.start = Start::Idle;
        }
        if matches!(self.start, Start::Idle) {
            self.begin();
        }
        if let Start::Ready(world) = &mut self.start {
            world.show(&mut self.views);
        }
        false
    }

    fn begin(&mut self) {
        let Some(first) = self.views.first() else {
            return;
        };
        let (key, canvas) = (first.key, first.canvas.clone());
        self.starts += 1;
        let start = self.starts;
        self.start = Start::Starting(start);
        crate::synthview::set_draw(LOADING);
        wasm_bindgen_futures::spawn_local(async move {
            let built = Gpu::new(canvas).await;
            WEB.with(|web| web.borrow_mut().started(start, key, built));
        });
    }

    fn started(
        &mut self,
        start: u32,
        key: u32,
        built: Result<(Gpu, wgpu::Surface<'static>), String>,
    ) {
        if !matches!(self.start, Start::Starting(current) if current == start) {
            return;
        }
        match built {
            Err(reason) => {
                log::error!(target: "synthetic", "event=gpu_failed reason=\"{reason}\"");
                crate::synthview::set_draw(FAILED);
                self.start = Start::Failed;
            }
            Ok((gpu, surface)) => {
                let info = gpu.adapter.get_info();
                log::info!(target: "synthetic", "event=gpu_ready backend={:?} adapter=\"{}\" format={:?}", info.backend, info.name, gpu.format);
                if let Some(view) = self.views.iter_mut().find(|view| view.key == key) {
                    view.surface = Some(surface);
                }
                self.start = Start::Ready(Box::new(World::new(gpu)));
            }
        }
    }
}

impl Gpu {
    async fn new(
        canvas: web_sys::OffscreenCanvas,
    ) -> Result<(Self, wgpu::Surface<'static>), String> {
        let instance = wgpu::util::new_instance_with_webgpu_detection(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        })
        .await;
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::OffscreenCanvas(canvas))
            .map_err(|error| format!("no surface for the canvas: {error}"))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(|error| format!("no GPU adapter for the canvas: {error}"))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .map_err(|error| format!("the GPU refused a device: {error}"))?;
        device.on_uncaptured_error(Arc::new(
            |error| log::error!(target: "synthetic", "event=gpu_error reason=\"{error}\""),
        ));
        let formats = surface.get_capabilities(&adapter).formats;
        let canvas_format = formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| formats.first().copied())
            .ok_or("the canvas offers no colour format")?;
        let format = canvas_format.add_srgb_suffix();
        let renderer = Renderer::with_device(device.clone(), queue, format);
        Ok((
            Self {
                instance,
                adapter,
                device,
                canvas_format,
                format,
                renderer,
            },
            surface,
        ))
    }

    fn surface_for(
        &self,
        canvas: &web_sys::OffscreenCanvas,
    ) -> Result<wgpu::Surface<'static>, String> {
        let surface = self
            .instance
            .create_surface(wgpu::SurfaceTarget::OffscreenCanvas(canvas.clone()))
            .map_err(|error| format!("no surface for the canvas: {error}"))?;
        match surface
            .get_capabilities(&self.adapter)
            .formats
            .contains(&self.canvas_format)
        {
            true => Ok(surface),
            false => Err(format!(
                "the canvas does not offer the kept format {:?}",
                self.canvas_format
            )),
        }
    }
}

impl View {
    fn forget(&mut self) {
        self.surface = None;
        self.refused = false;
        self.targets = None;
        self.size = (0, 0);
    }

    fn fit(&mut self, gpu: &Gpu) {
        let (width, height) = self.wanted;
        if self.wanted == self.size || width == 0 || height == 0 {
            return;
        }
        let Some(surface) = self.surface.as_ref() else {
            return;
        };
        let Some(config) = surface.get_default_config(&gpu.adapter, width, height) else {
            return;
        };
        let view_formats = if gpu.format == gpu.canvas_format {
            Vec::new()
        } else {
            vec![gpu.format]
        };
        surface.configure(
            &gpu.device,
            &wgpu::SurfaceConfiguration {
                format: gpu.canvas_format,
                view_formats,
                present_mode: wgpu::PresentMode::Fifo,
                ..config
            },
        );
        self.targets = Some(gpu.renderer.targets(width, height));
        self.size = self.wanted;
        self.drawn = false;
    }

    fn paint(
        &mut self,
        gpu: &mut Gpu,
        frame: Option<&Frame>,
        overlays: Option<&Overlays>,
    ) -> Result<bool, String> {
        let (Some(surface), Some(targets)) = (self.surface.as_ref(), self.targets.as_ref()) else {
            return Ok(false);
        };
        let texture = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.size = (0, 0);
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("the surface refused a frame".to_string());
            }
        };
        let view = texture.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(gpu.format),
            ..Default::default()
        });
        let pictured = match frame {
            Some(frame) => {
                let (camera, meshes) = frame.view(overlays, self.size.0, self.size.1);
                gpu.renderer
                    .draw(&view, targets, &camera, &frame.backfill, &meshes);
                !frame.ground.is_empty()
            }
            None => {
                gpu.renderer.clear(&view, targets);
                false
            }
        };
        gpu.renderer.present(texture);
        self.drawn = true;
        Ok(pictured)
    }
}

impl<T: PartialEq> Throttled<T> {
    fn new(every: Duration, value: T) -> Self {
        Self {
            every,
            read: None,
            value,
        }
    }

    fn refresh(&mut self, now: Instant, read: impl FnOnce() -> T) -> bool {
        if self
            .read
            .is_some_and(|at| now.duration_since(at) < self.every)
        {
            return false;
        }
        self.read = Some(now);
        let value = read();
        let changed = value != self.value;
        self.value = value;
        changed
    }
}

impl World {
    fn new(gpu: Gpu) -> Self {
        Self {
            gpu,
            session: None,
            token: Throttled::new(TOKEN_EVERY, None),
            overlays: Throttled::new(OVERLAYS_EVERY, None),
            framed: false,
            draw: LOADING,
        }
    }

    fn adopt(&self, views: &mut [View]) -> bool {
        let refused: Vec<(u32, String)> = views
            .iter_mut()
            .filter(|view| view.surface.is_none() && !view.refused)
            .filter_map(|view| match self.gpu.surface_for(&view.canvas) {
                Ok(surface) => {
                    view.surface = Some(surface);
                    None
                }
                Err(reason) => {
                    view.refused = true;
                    Some((view.key, reason))
                }
            })
            .collect();
        let shown = views.iter().any(|view| view.surface.is_some());
        refused.iter().filter(|_| shown).for_each(|(key, reason)| log::error!(target: "synthetic", "event=attach_failed window={key} reason=\"{reason}\""));
        shown
    }

    fn show(&mut self, views: &mut [View]) {
        let now = Instant::now();
        views.iter_mut().for_each(|view| view.fit(&self.gpu));
        let rekeyed = self.token.refresh(now, || Some(token()));
        if rekeyed {
            self.session = start_session(
                self.token.value.as_deref().unwrap_or_default(),
                self.gpu.renderer.detail(),
            );
        }
        let restyled = self.overlays.refresh(now, || {
            read("view.syntheticOverlays")
                .as_ref()
                .and_then(Overlays::from_view)
        });
        let target = read("view.syntheticView")
            .as_ref()
            .and_then(Target::from_view);
        let sticks = crate::virtualjoystick::sent_sticks();
        let largest = views
            .iter()
            .map(|view| view.size)
            .max_by_key(|(width, height)| u64::from(*width) * u64::from(*height))
            .unwrap_or_default();
        let mut frame = self
            .session
            .as_mut()
            .filter(|_| largest.0 > 0 && largest.1 > 0)
            .and_then(|session| session.frame(target.as_ref(), &sticks, largest.0, largest.1, now));
        crate::synthview::set_ground(frame.as_ref().and_then(Frame::ground_above_home));
        if let Some((frame, session)) = frame.as_mut().zip(self.session.as_mut()) {
            session.warmed(
                self.gpu
                    .renderer
                    .warm(std::mem::take(&mut frame.warm), WARM_PER_TICK),
            );
        }
        let redraw = rekeyed
            || restyled
            || frame.as_ref().is_some_and(|frame| frame.changed)
            || frame.is_some() != self.framed;
        self.framed = frame.is_some();
        let painted: Vec<Result<bool, String>> = views
            .iter_mut()
            .filter(|view| redraw || !view.drawn)
            .map(|view| view.paint(&mut self.gpu, frame.as_ref(), self.overlays.value.as_ref()))
            .collect();
        painted
            .iter()
            .filter_map(|result| result.as_ref().err())
            .for_each(
                |reason| log::error!(target: "synthetic", "event=paint_failed reason=\"{reason}\""),
            );
        let draw = match (
            self.session.is_some(),
            painted.iter().any(|result| result == &Ok(true)),
        ) {
            (false, _) => FAILED,
            (true, true) => DRAWN,
            (true, false) if painted.is_empty() => self.draw,
            (true, false) => LOADING,
        };
        if draw != self.draw {
            self.draw = draw;
            crate::synthview::set_draw(draw);
        }
    }
}
