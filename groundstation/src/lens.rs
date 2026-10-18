use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use crate::router::Backend;

pub const LENS_CHANGED: &str = "core.lens@changed";
pub const MEASURE: &str = "camera.measureLens";
pub const FORGET: &str = "camera.forgetLens";
pub const DEPS: &[&str] = &[
    LENS_CHANGED,
    "vehicle.cameraManager.currentCameraInstance.modelName",
    "vehicle.cameraManager.currentCameraInstance.zoomLevel",
    "settings.gimbalControllerSettings.cameraHFov",
    "settings.gimbalControllerSettings.cameraVFov",
];

const FILE: &str = "LensMeasurements.json";
const CAMERA: &str = "vehicle.cameraManager.currentCameraInstance";
const REPORT_FRESH_MS: u64 = 10_000;
const SAME_ZOOM: f64 = 2.0;

const STEP_YAW: f64 = 2.5;
const STEP_PITCH: f64 = 2.0;
const MAX_STEPS: u32 = 8;
const TARGET_SHIFT: f64 = 0.25;
const MIN_SHIFT: f64 = 0.08;
const MIN_FOV: f64 = 2.0;
const MAX_FOV: f64 = 160.0;
const STILL_MS: u64 = 300;
const STILL_DEGREES: f64 = 0.05;
const TURN_GIVE_UP_MS: u64 = 4000;
const WAIT_MS: u64 = 3000;
pub const MEASURE_STEP: std::time::Duration = std::time::Duration::from_millis(100);

const NO_REFERENCE: &str =
    "Nothing in view to measure against. Point the camera at something the detector recognises.";
const LOST_SIGHT: &str =
    "Lost sight of the reference while turning. Try again with something that stays in frame.";
const DISAGREE: &str = "The views did not agree on one lens. The detector may be jumping between things; try again with one clear object in view.";
const MAX_MISFIT: f64 = 0.02;
const OUTLIER_TIMES: f64 = 4.0;
const OUTLIER_FLOOR: f64 = 0.02;
const FRAMES_AGREE: f64 = 0.01;
const NO_ATTITUDE: &str =
    "The gimbal is not reporting its angle, so a measurement cannot be trusted.";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    pub zoom: f64,
    pub h: f64,
    pub v: f64,
}

type Store = BTreeMap<String, Vec<Sample>>;

pub type Guide = fn(&Value) -> Option<Result<(), String>>;
pub static GUIDE: std::sync::OnceLock<Guide> = std::sync::OnceLock::new();

static REPORTED_AT: AtomicU64 = AtomicU64::new(0);
static MEMORY: Mutex<Option<Store>> = Mutex::new(None);
static STATUS: Mutex<Status> = Mutex::new(Status {
    measuring: false,
    progress: String::new(),
    error: None,
    generation: 0,
});
static ANNOUNCED: Mutex<Value> = Mutex::new(Value::Null);
static LAST_FIT: Mutex<Value> = Mutex::new(Value::Null);

#[derive(Clone, Debug)]
struct Status {
    measuring: bool,
    progress: String,
    error: Option<String>,
    generation: u64,
}

fn status() -> std::sync::MutexGuard<'static, Status> {
    STATUS.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn reported(now_ms: u64) {
    REPORTED_AT.store(now_ms.max(1), Ordering::Relaxed);
}

fn camera_reports(now_ms: u64) -> bool {
    let at = REPORTED_AT.load(Ordering::Relaxed);
    at != 0 && now_ms.saturating_sub(at) <= REPORT_FRESH_MS
}

fn file() -> Option<String> {
    crate::settingsstore::save_root().map(|root| format!("{}/{FILE}", root.trim_end_matches('/')))
}

fn store() -> Store {
    let mut memory = MEMORY.lock().unwrap_or_else(PoisonError::into_inner);
    memory
        .get_or_insert_with(|| {
            file()
                .and_then(|path| crate::vfs::read(path).ok())
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                .unwrap_or_default()
        })
        .clone()
}

fn save(updated: Store) {
    if let Some(path) = file()
        && let Err(error) = crate::vfs::write(
            &path,
            serde_json::to_vec_pretty(&updated).unwrap_or_default(),
        )
    {
        log::warn!("lens measurements not saved to {path}: {error}");
    }
    *MEMORY.lock().unwrap_or_else(PoisonError::into_inner) = Some(updated);
}

pub fn merged(samples: &[Sample], sample: Sample) -> Vec<Sample> {
    samples
        .iter()
        .copied()
        .filter(|s| (s.zoom - sample.zoom).abs() > SAME_ZOOM)
        .chain(std::iter::once(sample))
        .collect()
}

fn half_tan(degrees: f64) -> f64 {
    (degrees.to_radians() / 2.0).tan()
}

fn from_half_tan(t: f64) -> f64 {
    (2.0 * t.atan()).to_degrees()
}

pub fn fit(samples: &[Sample], zoom: f64, pick: fn(&Sample) -> f64) -> Option<f64> {
    let points: Vec<(f64, f64)> = samples
        .iter()
        .filter(|s| pick(s) > 0.0)
        .map(|s| (s.zoom, 1.0 / half_tan(pick(s))))
        .collect();
    let n = points.len() as f64;
    if points.is_empty() {
        return None;
    }
    let (mean_z, mean_y) = (
        points.iter().map(|p| p.0).sum::<f64>() / n,
        points.iter().map(|p| p.1).sum::<f64>() / n,
    );
    let spread: f64 = points.iter().map(|p| (p.0 - mean_z).powi(2)).sum();
    let slope = match spread > 1e-9 {
        true => {
            points
                .iter()
                .map(|p| (p.0 - mean_z) * (p.1 - mean_y))
                .sum::<f64>()
                / spread
        }
        false => 0.0,
    };
    let inverse = mean_y + slope * (zoom - mean_z);
    (inverse > 0.0).then(|| from_half_tan(1.0 / inverse))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Source {
    Camera,
    Measured,
    Setting,
}

impl Source {
    fn token(self) -> &'static str {
        match self {
            Source::Camera => "camera",
            Source::Measured => "measured",
            Source::Setting => "setting",
        }
    }
}

pub fn resolve(
    samples: &[Sample],
    zoom: f64,
    reporting: bool,
    setting: (Option<f64>, Option<f64>),
) -> (Option<f64>, Option<f64>, Source) {
    match (
        reporting,
        fit(samples, zoom, |s| s.h),
        fit(samples, zoom, |s| s.v),
    ) {
        (true, _, _) => (setting.0, setting.1, Source::Camera),
        (false, Some(h), Some(v)) => (Some(h), Some(v), Source::Measured),
        _ => (setting.0, setting.1, Source::Setting),
    }
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
}

fn setting(backend: &dyn Backend, name: &str) -> Option<f64> {
    crate::read::value_number(&backend.value(&format!(
        "settings.gimbalControllerSettings.{name}.rawValue"
    )))
}

struct Lens {
    model: Option<String>,
    zoom: f64,
}

fn lens(backend: &dyn Backend) -> Lens {
    let camera = backend.value_fields(CAMERA, "modelName,zoomLevel");
    let model = camera
        .get("modelName")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_uppercase);
    Lens {
        model,
        zoom: number(&camera, "zoomLevel").unwrap_or(0.0),
    }
}

pub fn field_of_view(backend: &dyn Backend) -> (Option<f64>, Option<f64>, Source) {
    let lens = lens(backend);
    let samples = lens
        .model
        .as_ref()
        .and_then(|m| store().get(m).cloned())
        .unwrap_or_default();
    resolve(
        &samples,
        lens.zoom,
        camera_reports(crate::hub::now_ms()),
        (
            setting(backend, "cameraHFov"),
            setting(backend, "cameraVFov"),
        ),
    )
}

pub fn lens_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let lens = lens(backend);
    let samples = lens
        .model
        .as_ref()
        .and_then(|m| store().get(m).cloned())
        .unwrap_or_default();
    let (h, v, source) = field_of_view(backend);
    let current = status().clone();
    let detector = crate::detections::lock()
        .tracker(crate::hub::now_ms())
        .is_some();
    let refusal = match (&lens.model, detector, current.measuring) {
        (_, _, true) => Some("A measurement is running."),
        (None, _, _) => Some(
            "No camera has reported its model, so there is nothing to file a measurement under.",
        ),
        (_, false, _) => Some("Measuring needs the device's detector running on this camera."),
        _ => None,
    };
    json!({
        "kind": "object",
        "class": "Lens",
        "model": lens.model,
        "zoom": lens.zoom,
        "hFov": h,
        "vFov": v,
        "source": source.token(),
        "approximate": source == Source::Setting,
        "summary": summary(h, v, source),
        "samples": samples.iter().map(|s| json!({ "zoom": s.zoom, "hFov": s.h, "vFov": s.v })).collect::<Vec<_>>(),
        "measuring": current.measuring,
        "progress": current.progress,
        "error": current.error,
        "canMeasure": refusal.is_none(),
        "lastFit": LAST_FIT.lock().unwrap_or_else(PoisonError::into_inner).clone(),
        "reason": refusal,
    })
}

fn summary(h: Option<f64>, v: Option<f64>, source: Source) -> String {
    let angles = match (h, v) {
        (Some(h), Some(v)) => format!("{h:.0}° × {v:.0}°"),
        _ => "Unknown".to_string(),
    };
    match source {
        Source::Camera => format!("{angles} · reported by the camera"),
        Source::Measured => format!("{angles} · measured"),
        Source::Setting => format!("{angles} · from settings, so taps aim approximately"),
    }
}

pub fn take_changed() -> bool {
    let current = status().clone();
    let now = json!([
        current.measuring,
        current.progress,
        current.error,
        current.generation,
        camera_reports(crate::hub::now_ms())
    ]);
    let mut announced = ANNOUNCED.lock().unwrap_or_else(PoisonError::into_inner);
    let changed = *announced != now;
    *announced = now;
    changed
}

fn set_status(measuring: bool, progress: &str, error: Option<&str>) {
    let mut current = status();
    current.measuring = measuring;
    current.progress = progress.to_string();
    current.error = error.map(str::to_string);
    current.generation += 1;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Seen {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub label: String,
    pub confidence: f64,
    pub target: bool,
}

impl Seen {
    fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    pub fn from_json(value: &Value) -> Option<Seen> {
        Some(Seen {
            x: number(value, "x")?,
            y: number(value, "y")?,
            w: number(value, "w")?,
            h: number(value, "h")?,
            label: value
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            confidence: number(value, "confidence").unwrap_or(0.0),
            target: value
                .get("target")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }
}

pub fn reference(seen: &[Seen]) -> Option<Seen> {
    seen.iter()
        .find(|s| s.target)
        .or_else(|| {
            seen.iter()
                .max_by(|a, b| a.confidence.total_cmp(&b.confidence))
        })
        .cloned()
}

pub fn matching(reference: &Seen, seen: &[Seen]) -> Option<Seen> {
    let difference = |s: &Seen| (s.w - reference.w).abs() + (s.h - reference.h).abs();
    seen.iter()
        .filter(|s| s.label == reference.label)
        .min_by(|a, b| difference(a).total_cmp(&difference(b)))
        .cloned()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observation {
    pub pitch: f64,
    pub yaw: f64,
    pub x: f64,
    pub y: f64,
}

fn half(fov: f64) -> f64 {
    (fov.to_radians() / 2.0).tan()
}

fn direction(pitch: f64, yaw: f64, right: f64, up: f64) -> [f64; 3] {
    let (sin, cos) = pitch.to_radians().sin_cos();
    let (forward, raised) = (cos - sin * up, sin + cos * up);
    let (ys, yc) = yaw.to_radians().sin_cos();
    [forward * yc - right * ys, forward * ys + right * yc, raised]
}

fn projected(world: [f64; 3], pitch: f64, yaw: f64) -> Option<(f64, f64)> {
    let (ys, yc) = yaw.to_radians().sin_cos();
    let (x1, y1, z1) = (
        world[0] * yc + world[1] * ys,
        -world[0] * ys + world[1] * yc,
        world[2],
    );
    let (sin, cos) = pitch.to_radians().sin_cos();
    let (forward, raised) = (cos * x1 + sin * z1, -sin * x1 + cos * z1);
    (forward > 1e-6).then(|| (y1 / forward, raised / forward))
}

pub fn seen_at(
    world: [f64; 3],
    pitch: f64,
    yaw: f64,
    h_fov: f64,
    v_fov: f64,
) -> Option<(f64, f64)> {
    projected(world, pitch, yaw).map(|(right, up)| {
        (
            0.5 + right / (2.0 * half(h_fov)),
            0.5 - up / (2.0 * half(v_fov)),
        )
    })
}

pub fn looking_at(observation: &Observation, h_fov: f64, v_fov: f64) -> [f64; 3] {
    direction(
        observation.pitch,
        observation.yaw,
        (observation.x - 0.5) * 2.0 * half(h_fov),
        (0.5 - observation.y) * 2.0 * half(v_fov),
    )
}

fn median(values: impl Iterator<Item = f64>) -> f64 {
    let mut sorted: Vec<f64> = values.collect();
    sorted.sort_by(f64::total_cmp);
    sorted.get(sorted.len() / 2).copied().unwrap_or(0.0)
}

fn object(sweep: &[Observation], h_fov: f64, v_fov: f64) -> [f64; 3] {
    let seen: Vec<[f64; 3]> = sweep
        .iter()
        .map(|o| looking_at(o, h_fov, v_fov))
        .map(|d| d.map(|c| c / d.iter().map(|c| c * c).sum::<f64>().sqrt()))
        .collect();
    std::array::from_fn(|axis| median(seen.iter().map(|d| d[axis])))
}

fn residuals(sweep: &[Observation], h_fov: f64, v_fov: f64) -> Vec<f64> {
    let world = object(sweep, h_fov, v_fov);
    sweep
        .iter()
        .map(|o| {
            seen_at(world, o.pitch, o.yaw, h_fov, v_fov)
                .map_or(1.0, |(x, y)| ((x - o.x).powi(2) + (y - o.y).powi(2)).sqrt())
        })
        .collect()
}

fn misfit(sweeps: &[Vec<Observation>], h_fov: f64, v_fov: f64) -> (f64, usize) {
    sweeps
        .iter()
        .flat_map(|sweep| residuals(sweep, h_fov, v_fov))
        .fold((0.0, 0), |(sum, n), e| (sum + e * e, n + 1))
}

fn best(
    sweeps: &[Vec<Observation>],
    hs: impl Iterator<Item = f64> + Clone,
    vs: impl Iterator<Item = f64> + Clone,
) -> Option<(f64, f64, f64)> {
    hs.flat_map(|h| vs.clone().map(move |v| (h, v)))
        .map(|(h, v)| {
            let (sum, n) = misfit(sweeps, h, v);
            (
                h,
                v,
                if n == 0 {
                    f64::INFINITY
                } else {
                    (sum / n as f64).sqrt()
                },
            )
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))
}

fn range(from: f64, to: f64, step: f64) -> impl Iterator<Item = f64> + Clone {
    (0..=((to - from) / step).round() as usize).map(move |i| from + i as f64 * step)
}

fn fit_once(sweeps: &[Vec<Observation>]) -> Option<(f64, f64, f64)> {
    let (h, v, _) = best(
        sweeps,
        range(MIN_FOV, MAX_FOV, 1.0),
        range(MIN_FOV, MAX_FOV, 1.0),
    )?;
    let (h, v, rms) = best(
        sweeps,
        range((h - 1.0).max(MIN_FOV), (h + 1.0).min(MAX_FOV), 0.02),
        range((v - 1.0).max(MIN_FOV), (v + 1.0).min(MAX_FOV), 0.02),
    )?;
    rms.is_finite().then_some((h, v, rms))
}

pub fn trimmed(sweeps: &[Vec<Observation>], h_fov: f64, v_fov: f64) -> Vec<Vec<Observation>> {
    let all: Vec<f64> = sweeps
        .iter()
        .flat_map(|sweep| residuals(sweep, h_fov, v_fov))
        .collect();
    let mut sorted = all.clone();
    sorted.sort_by(f64::total_cmp);
    let cut = (median(all.iter().copied()) * OUTLIER_TIMES).max(OUTLIER_FLOOR);
    let budget = all.len() / 4;
    let worst: Vec<f64> = sorted
        .iter()
        .rev()
        .copied()
        .filter(|r| *r > cut)
        .take(budget)
        .collect();
    let dropped = |r: f64| worst.last().is_some_and(|least| r >= *least);
    sweeps
        .iter()
        .map(|sweep| {
            let kept = residuals(sweep, h_fov, v_fov);
            sweep
                .iter()
                .zip(kept)
                .filter(|(_, r)| !dropped(*r))
                .map(|(o, _)| *o)
                .collect()
        })
        .collect()
}

pub fn fit_lens(sweeps: &[Vec<Observation>]) -> Option<(f64, f64, f64)> {
    let first = fit_once(sweeps)?;
    let kept = trimmed(sweeps, first.0, first.1);
    match kept.iter().map(Vec::len).sum::<usize>() < sweeps.iter().map(Vec::len).sum::<usize>() {
        true => fit_once(&kept),
        false => Some(first),
    }
}

pub trait Rig {
    fn attitude(&self) -> Option<(f64, f64)>;
    fn aim(&self, pitch: f64, yaw: f64) -> Result<(), String>;
    fn seen(&self) -> Option<(Vec<Seen>, u64)>;
}

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    Finding {
        deadline: u64,
    },
    Turning,
    Settling {
        since: u64,
        still_since: u64,
        last: (f64, f64),
        before: (f64, f64),
    },
    Matching {
        deadline: u64,
        settled_at: u64,
        after: (f64, f64),
        candidate: Option<(Seen, u64)>,
    },
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Axis {
    Yaw(f64),
    Pitch(f64),
}

const SWEEPS: [Axis; 4] = [
    Axis::Yaw(STEP_YAW),
    Axis::Pitch(STEP_PITCH),
    Axis::Yaw(-STEP_YAW),
    Axis::Pitch(-STEP_PITCH),
];

pub enum Outcome {
    Running(String),
    Measured { h: f64, v: f64 },
    Failed(String),
}

pub struct Measurement {
    start: (f64, f64),
    sweep: usize,
    steps: u32,
    reference: Option<Seen>,
    points: Vec<Observation>,
    phase: Phase,
    kept: Vec<(Axis, Vec<Observation>)>,
    last_error: Option<&'static str>,
}

impl Measurement {
    pub fn start(rig: &dyn Rig, now_ms: u64) -> Result<Measurement, &'static str> {
        let start = rig.attitude().ok_or(NO_ATTITUDE)?;
        Ok(Measurement {
            start,
            sweep: 0,
            steps: 0,
            reference: None,
            points: Vec::new(),
            phase: Phase::Finding {
                deadline: now_ms + WAIT_MS,
            },
            kept: Vec::new(),
            last_error: None,
        })
    }

    fn progress(&self) -> String {
        format!(
            "Measuring… {} of {}",
            (self.sweep + 1).min(SWEEPS.len()),
            SWEEPS.len()
        )
    }

    fn travelled(&self) -> f64 {
        match (self.points.first(), self.points.last(), SWEEPS[self.sweep]) {
            (Some(first), Some(last), Axis::Yaw(_)) => (last.x - first.x).abs(),
            (Some(first), Some(last), Axis::Pitch(_)) => (last.y - first.y).abs(),
            _ => 0.0,
        }
    }

    fn finish_sweep(&mut self, now_ms: u64, error: Option<&'static str>) {
        match (error, self.travelled() >= MIN_SHIFT) {
            (None, true) => self
                .kept
                .push((SWEEPS[self.sweep], std::mem::take(&mut self.points))),
            (error, _) => self.last_error = Some(error.unwrap_or(LOST_SIGHT)),
        }
        self.points.clear();
        self.sweep += 1;
        self.steps = 0;
        self.reference = None;
        self.phase = match self.sweep < SWEEPS.len() {
            true => Phase::Finding {
                deadline: now_ms + WAIT_MS,
            },
            false => Phase::Done,
        };
    }

    fn concluded(&self, rig: &dyn Rig) -> Outcome {
        if let Err(error) = rig.aim(self.start.0, self.start.1) {
            log::warn!("gimbal not returned after lens measurement: {error}");
        }
        let swept = |yaw: bool| {
            self.kept
                .iter()
                .any(|(axis, _)| matches!(axis, Axis::Yaw(_)) == yaw)
        };
        if !(swept(true) && swept(false)) {
            return Outcome::Failed(self.last_error.unwrap_or(LOST_SIGHT).to_string());
        }
        let sweeps: Vec<Vec<Observation>> =
            self.kept.iter().map(|(_, points)| points.clone()).collect();
        let fitted = fit_lens(&sweeps);
        *LAST_FIT.lock().unwrap_or_else(PoisonError::into_inner) = json!({
            "hFov": fitted.map(|f| f.0),
            "vFov": fitted.map(|f| f.1),
            "misfit": fitted.map(|f| f.2),
            "observations": sweeps.iter().map(|sweep| sweep.iter().map(|o| json!([o.pitch, o.yaw, o.x, o.y])).collect::<Vec<_>>()).collect::<Vec<_>>(),
        });
        match fitted {
            Some((h, v, misfit)) if misfit <= MAX_MISFIT => Outcome::Measured { h, v },
            Some(_) => Outcome::Failed(DISAGREE.to_string()),
            None => Outcome::Failed(LOST_SIGHT.to_string()),
        }
    }

    pub fn step(&mut self, rig: &dyn Rig, now_ms: u64) -> Outcome {
        match self.phase.clone() {
            Phase::Finding { deadline } => {
                match rig.seen().and_then(|(seen, _)| reference(&seen)) {
                    Some(found) => {
                        self.reference = Some(found);
                        self.phase = Phase::Turning;
                    }
                    None if now_ms >= deadline => self.finish_sweep(now_ms, Some(NO_REFERENCE)),
                    None => {}
                }
            }
            Phase::Turning => match rig.attitude() {
                None => self.finish_sweep(now_ms, Some(NO_ATTITUDE)),
                Some(before) => {
                    if self.points.is_empty()
                        && let Some(first) = &self.reference
                    {
                        let (x, y) = first.center();
                        self.points.push(Observation {
                            pitch: before.0,
                            yaw: before.1,
                            x,
                            y,
                        });
                    }
                    let (pitch, yaw) = match SWEEPS[self.sweep] {
                        Axis::Yaw(step) => (before.0, before.1 + step),
                        Axis::Pitch(step) => (before.0 + step, before.1),
                    };
                    match rig.aim(pitch, yaw) {
                        Ok(()) => {
                            self.phase = Phase::Settling {
                                since: now_ms,
                                still_since: now_ms,
                                last: before,
                                before,
                            }
                        }
                        Err(_) => self.finish_sweep(now_ms, Some(NO_ATTITUDE)),
                    }
                }
            },
            Phase::Settling {
                since,
                still_since,
                last,
                before,
            } => match rig.attitude() {
                None => self.finish_sweep(now_ms, Some(NO_ATTITUDE)),
                Some(now) => {
                    let moving = (now.0 - last.0).abs() > STILL_DEGREES
                        || (now.1 - last.1).abs() > STILL_DEGREES;
                    let still_since = if moving { now_ms } else { still_since };
                    let commanded = match SWEEPS[self.sweep] {
                        Axis::Yaw(step) | Axis::Pitch(step) => step.abs(),
                    };
                    let turned =
                        (now.0 - before.0).abs() + (now.1 - before.1).abs() >= commanded / 2.0;
                    let settled = turned && now_ms.saturating_sub(still_since) >= STILL_MS
                        || now_ms.saturating_sub(since) >= TURN_GIVE_UP_MS;
                    self.phase = match settled {
                        true => Phase::Matching {
                            deadline: now_ms + WAIT_MS,
                            settled_at: now_ms,
                            after: now,
                            candidate: None,
                        },
                        false => Phase::Settling {
                            since,
                            still_since,
                            last: now,
                            before,
                        },
                    };
                }
            },
            Phase::Matching {
                deadline,
                settled_at,
                after,
                candidate,
            } => {
                let reference = self.reference.clone();
                let frame = rig.seen().filter(|(_, at)| *at >= settled_at);
                let found = reference.as_ref().and_then(|r| {
                    frame
                        .as_ref()
                        .and_then(|(seen, at)| matching(r, seen).map(|m| (m, *at)))
                });
                let agreed = match (&candidate, &found) {
                    (Some((earlier, earlier_at)), Some((now, at))) if at != earlier_at => {
                        let ((x0, y0), (x1, y1)) = (earlier.center(), now.center());
                        ((x1 - x0).abs() <= FRAMES_AGREE && (y1 - y0).abs() <= FRAMES_AGREE)
                            .then(|| now.clone())
                    }
                    _ => None,
                };
                if agreed.is_none()
                    && let Some((now, at)) = found.clone()
                    && candidate
                        .as_ref()
                        .is_none_or(|(_, earlier_at)| *earlier_at != at)
                {
                    self.phase = Phase::Matching {
                        deadline,
                        settled_at,
                        after,
                        candidate: Some((now, at)),
                    };
                }
                match agreed {
                    Some(now) => {
                        let (x, y) = now.center();
                        self.points.push(Observation {
                            pitch: after.0,
                            yaw: after.1,
                            x,
                            y,
                        });
                        self.steps += 1;
                        self.reference = Some(now);
                        match self.travelled() >= TARGET_SHIFT || self.steps >= MAX_STEPS {
                            true => self.finish_sweep(now_ms, None),
                            false => self.phase = Phase::Turning,
                        }
                    }
                    None if now_ms >= deadline => self.finish_sweep(now_ms, None),
                    None => {}
                }
            }
            Phase::Done => {}
        }
        match self.phase {
            Phase::Done => self.concluded(rig),
            _ => Outcome::Running(self.progress()),
        }
    }
}

struct Device {
    vehicle: u8,
}

impl Rig for Device {
    fn attitude(&self) -> Option<(f64, f64)> {
        let hub = crate::hub::lock();
        let gimbals = &hub.vehicle(self.vehicle)?.gimbals;
        let attitude = gimbals
            .active()
            .and_then(|pair| gimbals.get(pair))
            .and_then(|g| g.attitude)?;
        Some((f64::from(attitude.pitch), f64::from(attitude.body_yaw)))
    }

    fn aim(&self, pitch: f64, yaw: f64) -> Result<(), String> {
        let action = json!({ "action": "gimbal", "op": "aim", "pitch": pitch, "bodyYaw": yaw, "vehicle": self.vehicle });
        GUIDE
            .get()
            .and_then(|guide| guide(&action))
            .unwrap_or_else(|| Err("The vehicle is gone.".to_string()))
    }

    fn seen(&self) -> Option<(Vec<Seen>, u64)> {
        crate::detections::lock()
            .fresh_boxes(crate::hub::now_ms())
            .map(|(boxes, at)| (boxes.iter().filter_map(Seen::from_json).collect(), at))
    }
}

fn refused(reason: &str) -> Value {
    json!({ "ok": false, "result": Value::Null, "refusal": "unavailable", "reason": reason })
}

pub fn measure(backend: &dyn Backend) -> Value {
    let view = lens_view(backend, &[]);
    if let Some(reason) = view.get("reason").and_then(Value::as_str) {
        return refused(reason);
    }
    let Some(vehicle) = crate::guided::active_id(backend).and_then(|id| u8::try_from(id).ok())
    else {
        return refused("No vehicle is connected.");
    };
    let lens = lens(backend);
    let Some(model) = lens.model else {
        return refused("No camera has reported its model.");
    };
    let rig = Device { vehicle };
    let mut measurement = match Measurement::start(&rig, crate::hub::now_ms()) {
        Ok(measurement) => measurement,
        Err(reason) => return refused(reason),
    };
    set_status(true, &measurement.progress(), None);
    let zoom = lens.zoom;
    let started = crate::background::poll("groundstation-lens-measure", MEASURE_STEP, move || {
        match measurement.step(&rig, crate::hub::now_ms()) {
            Outcome::Running(progress) => {
                if status().progress != progress {
                    set_status(true, &progress, None);
                }
                false
            }
            Outcome::Measured { h, v } => {
                let mut all = store();
                let kept = merged(
                    all.get(&model).map(Vec::as_slice).unwrap_or_default(),
                    Sample { zoom, h, v },
                );
                all.insert(model.clone(), kept);
                save(all);
                set_status(false, "", None);
                true
            }
            Outcome::Failed(reason) => {
                set_status(false, "", Some(&reason));
                true
            }
        }
    });
    match started {
        Ok(()) => {
            json!({ "ok": true, "result": Value::Null, "refusal": Value::Null, "reason": Value::Null })
        }
        Err(error) => {
            set_status(false, "", Some(&error.to_string()));
            refused("The measurement could not start.")
        }
    }
}

pub fn forget(backend: &dyn Backend) -> Value {
    let Some(model) = lens(backend).model else {
        return refused("No camera has reported its model.");
    };
    let mut all = store();
    all.remove(&model);
    save(all);
    set_status(false, "", None);
    json!({ "ok": true, "result": Value::Null, "refusal": Value::Null, "reason": Value::Null })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn one_sample_holds_at_every_zoom_and_two_follow_the_optics_between_and_beyond() {
        let wide = Sample {
            zoom: 0.0,
            h: 62.0,
            v: 37.35,
        };
        assert!(
            (fit(&[wide], 50.0, |s| s.h).unwrap() - 62.0).abs() < 1e-9,
            "one sample cannot know the zoom curve"
        );
        let tele = Sample {
            zoom: 100.0,
            h: from_half_tan(half_tan(62.0) / 10.0),
            v: from_half_tan(half_tan(37.35) / 10.0),
        };
        let middle = fit(&[wide, tele], 50.0, |s| s.h).unwrap();
        assert!(
            (middle - from_half_tan(half_tan(62.0) / 5.5)).abs() < 1e-6,
            "halfway in zoom is 5.5x on a 1-10x lens, through the tangent: {middle}"
        );
        assert!((fit(&[wide, tele], 0.0, |s| s.h).unwrap() - 62.0).abs() < 1e-6);
    }

    #[test]
    fn a_new_measurement_replaces_one_taken_at_the_same_zoom() {
        let kept = merged(
            &[
                Sample {
                    zoom: 0.0,
                    h: 60.0,
                    v: 35.0,
                },
                Sample {
                    zoom: 50.0,
                    h: 12.0,
                    v: 7.0,
                },
            ],
            Sample {
                zoom: 1.0,
                h: 62.0,
                v: 37.0,
            },
        );
        assert_eq!(
            kept,
            vec![
                Sample {
                    zoom: 50.0,
                    h: 12.0,
                    v: 7.0
                },
                Sample {
                    zoom: 1.0,
                    h: 62.0,
                    v: 37.0
                }
            ]
        );
    }

    #[test]
    fn the_camera_wins_then_a_measurement_then_the_setting() {
        let measured = [Sample {
            zoom: 0.0,
            h: 62.0,
            v: 37.0,
        }];
        assert_eq!(
            resolve(&measured, 0.0, true, (Some(58.0), Some(33.0))),
            (Some(58.0), Some(33.0), Source::Camera)
        );
        assert_eq!(
            resolve(&measured, 0.0, false, (Some(80.0), Some(80.0))).2,
            Source::Measured
        );
        assert_eq!(
            resolve(&[], 0.0, false, (Some(80.0), Some(80.0))),
            (Some(80.0), Some(80.0), Source::Setting)
        );
    }

    #[test]
    fn glitched_detections_are_dropped_instead_of_bending_the_lens_even_the_first_of_a_sweep() {
        let world = direction(-20.0, 0.0, 0.0, 0.0);
        let sweep = |axis: fn(f64) -> (f64, f64)| -> Vec<Observation> {
            (0..8)
                .map(|step| {
                    let (pitch, yaw) = axis(f64::from(step) * 2.0);
                    let (x, y) = seen_at(world, pitch, yaw, 62.0, 37.0).unwrap();
                    Observation { pitch, yaw, x, y }
                })
                .collect()
        };
        let mut yawing = sweep(|d| (-20.0, d));
        yawing[4].y += 0.17;
        yawing[0].x -= 0.12;
        let sweeps = vec![yawing, sweep(|d| (-20.0 + d, 0.0))];
        let (h, v, misfit) = fit_lens(&sweeps).unwrap();
        assert!(
            (h - 62.0).abs() < 0.05 && (v - 37.0).abs() < 0.05 && misfit < 0.001,
            "{h} × {v} misfit {misfit}"
        );
    }

    #[test]
    fn a_direction_seen_through_the_lens_projects_back_to_the_same_place() {
        let seen = Observation {
            pitch: -35.0,
            yaw: 12.0,
            x: 0.8,
            y: 0.3,
        };
        let world = looking_at(&seen, 62.0, 37.35);
        let (x, y) = seen_at(world, -35.0, 12.0, 62.0, 37.35).unwrap();
        assert!((x - 0.8).abs() < 1e-9 && (y - 0.3).abs() < 1e-9, "{x} {y}");
        let (yaw, tilt) = crate::gimbal::aim_through_lens(0.6, 0.4, 62.0, 37.35, -35.0);
        let (x, y) = seen_at(
            direction(-35.0, 0.0, 0.6 * half(62.0), 0.4 * half(37.35)),
            f64::from(tilt),
            f64::from(yaw),
            62.0,
            37.35,
        )
        .unwrap();
        assert!(
            (x - 0.5).abs() < 1e-5 && (y - 0.5).abs() < 1e-5,
            "a tap aims the gimbal so the tapped thing lands in the middle: {x} {y}"
        );
    }

    struct Bench {
        hfov: f64,
        vfov: f64,
        attitude: RefCell<(f64, f64)>,
        target: RefCell<(f64, f64)>,
        object: (f64, f64),
        aims: RefCell<Vec<(f64, f64)>>,
        blind: bool,
        rate: f64,
        frames: std::cell::Cell<u64>,
    }

    fn bench(start: (f64, f64), rate: f64, blind: bool) -> Bench {
        Bench {
            hfov: 62.0,
            vfov: 37.0,
            attitude: RefCell::new(start),
            target: RefCell::new(start),
            object: start,
            aims: RefCell::new(Vec::new()),
            blind,
            rate,
            frames: std::cell::Cell::new(0),
        }
    }

    fn toward(now: f64, target: f64, rate: f64) -> f64 {
        now + (target - now).clamp(-rate, rate)
    }

    impl Rig for Bench {
        fn attitude(&self) -> Option<(f64, f64)> {
            let (now, target) = (*self.attitude.borrow(), *self.target.borrow());
            *self.attitude.borrow_mut() = (
                toward(now.0, target.0, self.rate),
                toward(now.1, target.1, self.rate),
            );
            Some(*self.attitude.borrow())
        }
        fn aim(&self, pitch: f64, yaw: f64) -> Result<(), String> {
            *self.target.borrow_mut() = (pitch, yaw);
            self.aims.borrow_mut().push((pitch, yaw));
            Ok(())
        }
        fn seen(&self) -> Option<(Vec<Seen>, u64)> {
            let (pitch, yaw) = *self.attitude.borrow();
            let world = direction(self.object.0, self.object.1, 0.0, 0.0);
            let (x, y) = seen_at(world, pitch, yaw, self.hfov, self.vfov)?;
            self.frames.set(self.frames.get() + 1);
            (!self.blind).then(|| {
                (
                    vec![Seen {
                        x: x - 0.05,
                        y: y - 0.05,
                        w: 0.1,
                        h: 0.1,
                        label: "car".into(),
                        confidence: 0.9,
                        target: false,
                    }],
                    u64::MAX - 1_000_000 + self.frames.get(),
                )
            })
        }
    }

    fn run(bench: &Bench) -> Outcome {
        let mut measurement = Measurement::start(bench, 0).unwrap();
        (0..2000u64)
            .map(|tick| measurement.step(bench, tick * 100))
            .find(|outcome| !matches!(outcome, Outcome::Running(_)))
            .expect("the measurement ends")
    }

    #[test]
    fn turning_the_gimbal_and_watching_the_scene_slide_measures_the_lens_and_puts_the_camera_back()
    {
        let bench = bench((-40.0, 5.0), 100.0, false);
        match run(&bench) {
            Outcome::Measured { h, v } => assert!(
                (h - 62.0).abs() < 0.05 && (v - 37.0).abs() < 0.05,
                "a wide lens through the tangent, tilted down: {h} × {v}"
            ),
            _ => panic!("the bench lens was not measured"),
        }
        assert_eq!(
            bench.aims.borrow().last(),
            Some(&(-40.0, 5.0)),
            "the camera goes back to what the pilot was watching"
        );
    }

    #[test]
    fn a_gimbal_still_turning_is_waited_for_rather_than_read_mid_move() {
        let bench = bench((-10.0, 5.0), 0.3, false);
        match run(&bench) {
            Outcome::Measured { h, v } => assert!(
                (h - 62.0).abs() < 0.05 && (v - 37.0).abs() < 0.05,
                "a slow gimbal read mid-turn mismeasures: {h} × {v}"
            ),
            _ => panic!("the slow bench lens was not measured"),
        }
    }

    #[test]
    fn nothing_in_view_says_so_and_still_puts_the_camera_back() {
        let bench = bench((0.0, 0.0), 100.0, true);
        match run(&bench) {
            Outcome::Failed(reason) => assert_eq!(reason, NO_REFERENCE),
            _ => panic!("a blind detector measured something"),
        }
        assert_eq!(bench.aims.borrow().last(), Some(&(0.0, 0.0)));
    }

    #[test]
    fn the_tracked_box_is_the_reference_and_the_same_label_and_size_is_it_again() {
        let car = Seen {
            x: 0.1,
            y: 0.1,
            w: 0.2,
            h: 0.1,
            label: "car".into(),
            confidence: 0.5,
            target: false,
        };
        let truck = Seen {
            label: "truck".into(),
            confidence: 0.9,
            ..car.clone()
        };
        let tracked = Seen {
            target: true,
            ..car.clone()
        };
        assert_eq!(
            reference(&[car.clone(), truck.clone()]),
            Some(truck.clone())
        );
        assert_eq!(reference(&[truck.clone(), tracked.clone()]), Some(tracked));
        let moved = Seen {
            x: 0.3,
            ..car.clone()
        };
        assert_eq!(matching(&car, &[truck, moved.clone()]), Some(moved));
    }
}
