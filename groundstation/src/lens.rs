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

const NO_REFERENCE: &str = "Nothing in view to measure against. Point the camera at something the detector recognises.";
const LOST_SIGHT: &str = "Lost sight of the reference while turning. Try again with something that stays in frame.";
const NO_ATTITUDE: &str = "The gimbal is not reporting its angle, so a measurement cannot be trusted.";

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
static STATUS: Mutex<Status> = Mutex::new(Status { measuring: false, progress: String::new(), error: None, generation: 0 });
static ANNOUNCED: Mutex<Value> = Mutex::new(Value::Null);

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
        .get_or_insert_with(|| file().and_then(|path| crate::vfs::read(path).ok()).and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default())
        .clone()
}

fn save(updated: Store) {
    if let Some(path) = file()
        && let Err(error) = crate::vfs::write(&path, serde_json::to_vec_pretty(&updated).unwrap_or_default())
    {
        log::warn!("lens measurements not saved to {path}: {error}");
    }
    *MEMORY.lock().unwrap_or_else(PoisonError::into_inner) = Some(updated);
}

pub fn merged(samples: &[Sample], sample: Sample) -> Vec<Sample> {
    samples.iter().copied().filter(|s| (s.zoom - sample.zoom).abs() > SAME_ZOOM).chain(std::iter::once(sample)).collect()
}

fn half_tan(degrees: f64) -> f64 {
    (degrees.to_radians() / 2.0).tan()
}

fn from_half_tan(t: f64) -> f64 {
    (2.0 * t.atan()).to_degrees()
}

pub fn fit(samples: &[Sample], zoom: f64, pick: fn(&Sample) -> f64) -> Option<f64> {
    let points: Vec<(f64, f64)> = samples.iter().filter(|s| pick(s) > 0.0).map(|s| (s.zoom, 1.0 / half_tan(pick(s)))).collect();
    let n = points.len() as f64;
    if points.is_empty() {
        return None;
    }
    let (mean_z, mean_y) = (points.iter().map(|p| p.0).sum::<f64>() / n, points.iter().map(|p| p.1).sum::<f64>() / n);
    let spread: f64 = points.iter().map(|p| (p.0 - mean_z).powi(2)).sum();
    let slope = match spread > 1e-9 {
        true => points.iter().map(|p| (p.0 - mean_z) * (p.1 - mean_y)).sum::<f64>() / spread,
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

pub fn resolve(samples: &[Sample], zoom: f64, reporting: bool, setting: (Option<f64>, Option<f64>)) -> (Option<f64>, Option<f64>, Source) {
    match (reporting, fit(samples, zoom, |s| s.h), fit(samples, zoom, |s| s.v)) {
        (true, _, _) => (setting.0, setting.1, Source::Camera),
        (false, Some(h), Some(v)) => (Some(h), Some(v), Source::Measured),
        _ => (setting.0, setting.1, Source::Setting),
    }
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|v| v.is_finite())
}

fn setting(backend: &dyn Backend, name: &str) -> Option<f64> {
    crate::read::value_number(&backend.value(&format!("settings.gimbalControllerSettings.{name}.rawValue")))
}

struct Lens {
    model: Option<String>,
    zoom: f64,
}

fn lens(backend: &dyn Backend) -> Lens {
    let camera = backend.value_fields(CAMERA, "modelName,zoomLevel");
    let model = camera.get("modelName").and_then(Value::as_str).map(str::trim).filter(|m| !m.is_empty()).map(str::to_uppercase);
    Lens { model, zoom: number(&camera, "zoomLevel").unwrap_or(0.0) }
}

pub fn field_of_view(backend: &dyn Backend) -> (Option<f64>, Option<f64>, Source) {
    let lens = lens(backend);
    let samples = lens.model.as_ref().and_then(|m| store().get(m).cloned()).unwrap_or_default();
    resolve(&samples, lens.zoom, camera_reports(crate::hub::now_ms()), (setting(backend, "cameraHFov"), setting(backend, "cameraVFov")))
}

pub fn lens_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let lens = lens(backend);
    let samples = lens.model.as_ref().and_then(|m| store().get(m).cloned()).unwrap_or_default();
    let (h, v, source) = field_of_view(backend);
    let current = status().clone();
    let detector = crate::detections::lock().tracker(crate::hub::now_ms()).is_some();
    let refusal = match (&lens.model, detector, current.measuring) {
        (_, _, true) => Some("A measurement is running."),
        (None, _, _) => Some("No camera has reported its model, so there is nothing to file a measurement under."),
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
    let now = json!([current.measuring, current.progress, current.error, current.generation, camera_reports(crate::hub::now_ms())]);
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
            label: value.get("label").and_then(Value::as_str).unwrap_or("").to_string(),
            confidence: number(value, "confidence").unwrap_or(0.0),
            target: value.get("target").and_then(Value::as_bool).unwrap_or(false),
        })
    }
}

pub fn reference(seen: &[Seen]) -> Option<Seen> {
    seen.iter().find(|s| s.target).or_else(|| seen.iter().max_by(|a, b| a.confidence.total_cmp(&b.confidence))).cloned()
}

pub fn matching(reference: &Seen, seen: &[Seen]) -> Option<Seen> {
    let difference = |s: &Seen| (s.w - reference.w).abs() + (s.h - reference.h).abs();
    seen.iter().filter(|s| s.label == reference.label).min_by(|a, b| difference(a).total_cmp(&difference(b))).cloned()
}

pub fn fov_from_totals(degrees: f64, shift: f64) -> Option<f64> {
    let shift = shift.abs();
    (shift >= MIN_SHIFT).then(|| degrees.abs() / shift).filter(|fov| (MIN_FOV..=MAX_FOV).contains(fov))
}

pub trait Rig {
    fn attitude(&self) -> Option<(f64, f64)>;
    fn aim(&self, pitch: f64, yaw: f64) -> Result<(), String>;
    fn seen(&self) -> Option<(Vec<Seen>, u64)>;
}

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    Finding { deadline: u64 },
    Turning,
    Settling { since: u64, still_since: u64, last: (f64, f64), before: (f64, f64) },
    Matching { deadline: u64, settled_at: u64, before: (f64, f64), after: (f64, f64) },
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Axis {
    Yaw(f64),
    Pitch(f64),
}

const SWEEPS: [Axis; 4] = [Axis::Yaw(STEP_YAW), Axis::Pitch(STEP_PITCH), Axis::Yaw(-STEP_YAW), Axis::Pitch(-STEP_PITCH)];

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
    degrees: f64,
    shift: f64,
    phase: Phase,
    hs: Vec<f64>,
    vs: Vec<f64>,
    last_error: Option<&'static str>,
}

impl Measurement {
    pub fn start(rig: &dyn Rig, now_ms: u64) -> Result<Measurement, &'static str> {
        let start = rig.attitude().ok_or(NO_ATTITUDE)?;
        Ok(Measurement { start, sweep: 0, steps: 0, reference: None, degrees: 0.0, shift: 0.0, phase: Phase::Finding { deadline: now_ms + WAIT_MS }, hs: Vec::new(), vs: Vec::new(), last_error: None })
    }

    fn progress(&self) -> String {
        format!("Measuring… {} of {}", (self.sweep + 1).min(SWEEPS.len()), SWEEPS.len())
    }

    fn finish_sweep(&mut self, now_ms: u64, error: Option<&'static str>) {
        match (error, fov_from_totals(self.degrees, self.shift)) {
            (None, Some(fov)) => match SWEEPS[self.sweep] {
                Axis::Yaw(_) => self.hs.push(fov),
                Axis::Pitch(_) => self.vs.push(fov),
            },
            (error, _) => self.last_error = Some(error.unwrap_or(LOST_SIGHT)),
        }
        self.sweep += 1;
        self.steps = 0;
        self.degrees = 0.0;
        self.shift = 0.0;
        self.reference = None;
        self.phase = match self.sweep < SWEEPS.len() {
            true => Phase::Finding { deadline: now_ms + WAIT_MS },
            false => Phase::Done,
        };
    }

    fn concluded(&self, rig: &dyn Rig) -> Outcome {
        if let Err(error) = rig.aim(self.start.0, self.start.1) {
            log::warn!("gimbal not returned after lens measurement: {error}");
        }
        match (self.hs.is_empty(), self.vs.is_empty()) {
            (false, false) => Outcome::Measured { h: self.hs.iter().sum::<f64>() / self.hs.len() as f64, v: self.vs.iter().sum::<f64>() / self.vs.len() as f64 },
            _ => Outcome::Failed(self.last_error.unwrap_or(LOST_SIGHT).to_string()),
        }
    }

    pub fn step(&mut self, rig: &dyn Rig, now_ms: u64) -> Outcome {
        match self.phase.clone() {
            Phase::Finding { deadline } => match rig.seen().and_then(|(seen, _)| reference(&seen)) {
                Some(found) => {
                    self.reference = Some(found);
                    self.phase = Phase::Turning;
                }
                None if now_ms >= deadline => self.finish_sweep(now_ms, Some(NO_REFERENCE)),
                None => {}
            },
            Phase::Turning => match rig.attitude() {
                None => self.finish_sweep(now_ms, Some(NO_ATTITUDE)),
                Some(before) => {
                    let (pitch, yaw) = match SWEEPS[self.sweep] {
                        Axis::Yaw(step) => (before.0, before.1 + step),
                        Axis::Pitch(step) => (before.0 + step, before.1),
                    };
                    match rig.aim(pitch, yaw) {
                        Ok(()) => self.phase = Phase::Settling { since: now_ms, still_since: now_ms, last: before, before },
                        Err(_) => self.finish_sweep(now_ms, Some(NO_ATTITUDE)),
                    }
                }
            },
            Phase::Settling { since, still_since, last, before } => match rig.attitude() {
                None => self.finish_sweep(now_ms, Some(NO_ATTITUDE)),
                Some(now) => {
                    let moving = (now.0 - last.0).abs() > STILL_DEGREES || (now.1 - last.1).abs() > STILL_DEGREES;
                    let still_since = if moving { now_ms } else { still_since };
                    let commanded = match SWEEPS[self.sweep] {
                        Axis::Yaw(step) | Axis::Pitch(step) => step.abs(),
                    };
                    let turned = (now.0 - before.0).abs() + (now.1 - before.1).abs() >= commanded / 2.0;
                    let settled = turned && now_ms.saturating_sub(still_since) >= STILL_MS || now_ms.saturating_sub(since) >= TURN_GIVE_UP_MS;
                    self.phase = match settled {
                        true => Phase::Matching { deadline: now_ms + WAIT_MS, settled_at: now_ms, before, after: now },
                        false => Phase::Settling { since, still_since, last: now, before },
                    };
                }
            },
            Phase::Matching { deadline, settled_at, before, after } => {
                let reference = self.reference.clone();
                let seen = rig.seen().filter(|(_, at)| *at >= settled_at).map(|(seen, _)| seen);
                match reference.as_ref().and_then(|r| seen.as_deref().and_then(|seen| matching(r, seen))) {
                    Some(now) => {
                        let ((x0, y0), (x1, y1)) = (reference.map_or((0.0, 0.0), |r| r.center()), now.center());
                        let (turned, slid) = match SWEEPS[self.sweep] {
                            Axis::Yaw(_) => (after.1 - before.1, x1 - x0),
                            Axis::Pitch(_) => (after.0 - before.0, y1 - y0),
                        };
                        self.degrees += turned;
                        self.shift += slid;
                        self.steps += 1;
                        self.reference = Some(now);
                        match self.shift.abs() >= TARGET_SHIFT || self.steps >= MAX_STEPS {
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
        let attitude = gimbals.active().and_then(|pair| gimbals.get(pair)).and_then(|g| g.attitude)?;
        Some((f64::from(attitude.pitch), f64::from(attitude.body_yaw)))
    }

    fn aim(&self, pitch: f64, yaw: f64) -> Result<(), String> {
        let action = json!({ "action": "gimbal", "op": "aim", "pitch": pitch, "bodyYaw": yaw, "vehicle": self.vehicle });
        GUIDE.get().and_then(|guide| guide(&action)).unwrap_or_else(|| Err("The vehicle is gone.".to_string()))
    }

    fn seen(&self) -> Option<(Vec<Seen>, u64)> {
        crate::detections::lock().fresh_boxes(crate::hub::now_ms()).map(|(boxes, at)| (boxes.iter().filter_map(Seen::from_json).collect(), at))
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
    let Some(vehicle) = crate::guided::active_id(backend).and_then(|id| u8::try_from(id).ok()) else {
        return refused("No vehicle is connected.");
    };
    let lens = lens(backend);
    let Some(model) = lens.model else { return refused("No camera has reported its model.") };
    let rig = Device { vehicle };
    let mut measurement = match Measurement::start(&rig, crate::hub::now_ms()) {
        Ok(measurement) => measurement,
        Err(reason) => return refused(reason),
    };
    set_status(true, &measurement.progress(), None);
    let zoom = lens.zoom;
    let started = crate::background::poll("groundstation-lens-measure", MEASURE_STEP, move || match measurement.step(&rig, crate::hub::now_ms()) {
        Outcome::Running(progress) => {
            if status().progress != progress {
                set_status(true, &progress, None);
            }
            false
        }
        Outcome::Measured { h, v } => {
            let mut all = store();
            let kept = merged(all.get(&model).map(Vec::as_slice).unwrap_or_default(), Sample { zoom, h, v });
            all.insert(model.clone(), kept);
            save(all);
            set_status(false, "", None);
            true
        }
        Outcome::Failed(reason) => {
            set_status(false, "", Some(&reason));
            true
        }
    });
    match started {
        Ok(()) => json!({ "ok": true, "result": Value::Null, "refusal": Value::Null, "reason": Value::Null }),
        Err(error) => {
            set_status(false, "", Some(&error.to_string()));
            refused("The measurement could not start.")
        }
    }
}

pub fn forget(backend: &dyn Backend) -> Value {
    let Some(model) = lens(backend).model else { return refused("No camera has reported its model.") };
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
        let wide = Sample { zoom: 0.0, h: 62.0, v: 37.35 };
        assert!((fit(&[wide], 50.0, |s| s.h).unwrap() - 62.0).abs() < 1e-9, "one sample cannot know the zoom curve");
        let tele = Sample { zoom: 100.0, h: from_half_tan(half_tan(62.0) / 10.0), v: from_half_tan(half_tan(37.35) / 10.0) };
        let middle = fit(&[wide, tele], 50.0, |s| s.h).unwrap();
        assert!((middle - from_half_tan(half_tan(62.0) / 5.5)).abs() < 1e-6, "halfway in zoom is 5.5x on a 1-10x lens, through the tangent: {middle}");
        assert!((fit(&[wide, tele], 0.0, |s| s.h).unwrap() - 62.0).abs() < 1e-6);
    }

    #[test]
    fn a_new_measurement_replaces_one_taken_at_the_same_zoom() {
        let kept = merged(&[Sample { zoom: 0.0, h: 60.0, v: 35.0 }, Sample { zoom: 50.0, h: 12.0, v: 7.0 }], Sample { zoom: 1.0, h: 62.0, v: 37.0 });
        assert_eq!(kept, vec![Sample { zoom: 50.0, h: 12.0, v: 7.0 }, Sample { zoom: 1.0, h: 62.0, v: 37.0 }]);
    }

    #[test]
    fn the_camera_wins_then_a_measurement_then_the_setting() {
        let measured = [Sample { zoom: 0.0, h: 62.0, v: 37.0 }];
        assert_eq!(resolve(&measured, 0.0, true, (Some(58.0), Some(33.0))), (Some(58.0), Some(33.0), Source::Camera));
        assert_eq!(resolve(&measured, 0.0, false, (Some(80.0), Some(80.0))).2, Source::Measured);
        assert_eq!(resolve(&[], 0.0, false, (Some(80.0), Some(80.0))), (Some(80.0), Some(80.0), Source::Setting));
    }

    #[test]
    fn a_shift_too_small_to_trust_is_no_measurement() {
        assert_eq!(fov_from_totals(10.0, 0.05), None);
        assert_eq!(fov_from_totals(-15.0, -0.25), Some(60.0));
        assert_eq!(fov_from_totals(0.1, 0.1), None, "a 1° view is outside any lens this flies");
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
    }

    fn bench(start: (f64, f64), rate: f64, blind: bool) -> Bench {
        Bench { hfov: 62.0, vfov: 37.0, attitude: RefCell::new(start), target: RefCell::new(start), object: start, aims: RefCell::new(Vec::new()), blind, rate }
    }

    fn toward(now: f64, target: f64, rate: f64) -> f64 {
        now + (target - now).clamp(-rate, rate)
    }

    impl Rig for Bench {
        fn attitude(&self) -> Option<(f64, f64)> {
            let (now, target) = (*self.attitude.borrow(), *self.target.borrow());
            *self.attitude.borrow_mut() = (toward(now.0, target.0, self.rate), toward(now.1, target.1, self.rate));
            Some(*self.attitude.borrow())
        }
        fn aim(&self, pitch: f64, yaw: f64) -> Result<(), String> {
            *self.target.borrow_mut() = (pitch, yaw);
            self.aims.borrow_mut().push((pitch, yaw));
            Ok(())
        }
        fn seen(&self) -> Option<(Vec<Seen>, u64)> {
            let (pitch, yaw) = *self.attitude.borrow();
            let x = 0.5 + (self.object.1 - yaw) / self.hfov;
            let y = 0.5 - (self.object.0 - pitch) / self.vfov;
            (!self.blind).then(|| (vec![Seen { x: x - 0.05, y: y - 0.05, w: 0.1, h: 0.1, label: "car".into(), confidence: 0.9, target: false }], u64::MAX))
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
    fn turning_the_gimbal_and_watching_the_scene_slide_measures_the_lens_and_puts_the_camera_back() {
        let bench = bench((-10.0, 5.0), 100.0, false);
        match run(&bench) {
            Outcome::Measured { h, v } => assert!((h - 62.0).abs() < 0.5 && (v - 37.0).abs() < 0.5, "{h} × {v}"),
            _ => panic!("the bench lens was not measured"),
        }
        assert_eq!(bench.aims.borrow().last(), Some(&(-10.0, 5.0)), "the camera goes back to what the pilot was watching");
    }

    #[test]
    fn a_gimbal_still_turning_is_waited_for_rather_than_read_mid_move() {
        let bench = bench((-10.0, 5.0), 0.3, false);
        match run(&bench) {
            Outcome::Measured { h, v } => assert!((h - 62.0).abs() < 0.5 && (v - 37.0).abs() < 0.5, "a slow gimbal read mid-turn mismeasures: {h} × {v}"),
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
        let car = Seen { x: 0.1, y: 0.1, w: 0.2, h: 0.1, label: "car".into(), confidence: 0.5, target: false };
        let truck = Seen { label: "truck".into(), confidence: 0.9, ..car.clone() };
        let tracked = Seen { target: true, ..car.clone() };
        assert_eq!(reference(&[car.clone(), truck.clone()]), Some(truck.clone()));
        assert_eq!(reference(&[truck.clone(), tracked.clone()]), Some(tracked));
        let moved = Seen { x: 0.3, ..car.clone() };
        assert_eq!(matching(&car, &[truck, moved.clone()]), Some(moved));
    }
}
