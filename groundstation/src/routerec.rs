use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use crate::read::{Unit, flag, format_measure};
use crate::router::Backend;

pub const ROUTE_CHANGED: &str = "core.routeRecorder@changed";
pub const DEPS: &[&str] = &[
    ROUTE_CHANGED,
    "vehicles.activeVehicleAvailable",
    "settings.unitsSettings.horizontalDistanceUnits",
];
pub const ROUTE_START: &str = "routeRecorder.start";
pub const ROUTE_STOP: &str = "routeRecorder.stop";
pub const ROUTE_RENAME: &str = "routeRecorder.rename";
pub const ROUTE_DISMISS: &str = "routeRecorder.dismiss";
pub const ROUTE_ACTIONS: [&str; 4] = [ROUTE_START, ROUTE_STOP, ROUTE_RENAME, ROUTE_DISMISS];

const MIN_SPACING_M: f64 = 2.0;
const SIMPLIFY_TOLERANCE_M: f64 = 3.0;
const TRIM_RADIUS_M: f64 = 10.0;
const SAVE_INTERVAL_MS: u64 = 10_000;
const ROUTE_SAVED: &str = "Route saved";
const TOO_SHORT: &str = "Route not saved: the vehicle didn't fly far enough while recording.";
const NO_FOLDER: &str = "Route not recorded: set a save location in Application Settings.";
const NAME_USED: &str = "Name already used";

type Position = (f64, f64, f64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub position: Position,
    pub gimbal: Option<(f64, f64)>,
    pub look_at: Option<Position>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Observed {
    pub flying: bool,
    pub sample: Option<Sample>,
}

#[derive(Debug, Clone, PartialEq)]
struct Session {
    vehicle: u8,
    name: String,
    file: String,
    samples: Vec<Sample>,
    distance: f64,
    saved_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct Outcome {
    file: Option<String>,
    message: String,
}

#[derive(Debug, Default)]
struct Recorder {
    session: Option<Session>,
    flying: bool,
    outcome: Option<Outcome>,
}

static RECORDER: Mutex<Recorder> = Mutex::new(Recorder {
    session: None,
    flying: false,
    outcome: None,
});
static CHANGED: AtomicBool = AtomicBool::new(false);

fn lock() -> std::sync::MutexGuard<'static, Recorder> {
    RECORDER.lock().unwrap_or_else(PoisonError::into_inner)
}

fn changed() {
    CHANGED.store(true, Ordering::Relaxed);
}

pub fn take_changed() -> bool {
    CHANGED.swap(false, Ordering::Relaxed)
}

fn horizontal(from: Position, to: Position) -> f64 {
    crate::surveygrid::distance_between((from.0, from.1), (to.0, to.1))
}

pub fn route_indices(path: &[Position]) -> Vec<usize> {
    let (Some(&start), Some(&end)) = (path.first(), path.last()) else {
        return Vec::new();
    };
    let first_away = path
        .iter()
        .position(|p| horizontal(start, *p) >= TRIM_RADIUS_M);
    let last_away = path
        .iter()
        .rposition(|p| horizontal(end, *p) >= TRIM_RADIUS_M);
    match (first_away, last_away) {
        (Some(first_away), Some(last_away)) => {
            let (first, last) = (first_away.saturating_sub(1), last_away + 1);
            match first < last {
                true => {
                    crate::geo::simplify_path_indices(&path[first..=last], SIMPLIFY_TOLERANCE_M)
                        .into_iter()
                        .map(|i| first + i)
                        .collect()
                }
                false => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}

fn route_of(samples: &[Sample]) -> Vec<Sample> {
    let path: Vec<Position> = samples.iter().map(|s| s.position).collect();
    route_indices(&path)
        .into_iter()
        .map(|i| samples[i])
        .collect()
}

pub fn kml(name: &str, route: &[Sample]) -> String {
    crate::plankml::route(
        name,
        &route
            .iter()
            .map(|s| crate::plankml::RouteSample {
                position: s.position,
                gimbal: s.gimbal,
                look_at: s.look_at,
            })
            .collect::<Vec<_>>(),
    )
}

impl Session {
    fn add(self, sample: Sample) -> Session {
        let spaced = self.samples.last().is_none_or(|last| {
            horizontal(last.position, sample.position) >= MIN_SPACING_M
                || (last.position.2 - sample.position.2).abs() >= MIN_SPACING_M
        });
        match spaced {
            false => self,
            true => Session {
                distance: self.distance
                    + self
                        .samples
                        .last()
                        .map_or(0.0, |last| horizontal(last.position, sample.position)),
                samples: self
                    .samples
                    .into_iter()
                    .chain(std::iter::once(sample))
                    .collect(),
                ..self
            },
        }
    }

    fn saved(&self) -> Option<(String, String)> {
        let route = route_of(&self.samples);
        (route.len() >= 2).then(|| (self.file.clone(), kml(&self.name, &route)))
    }
}

impl Recorder {
    fn observe(&mut self, observed: Observed, now_ms: u64) -> (bool, Option<(String, String)>) {
        let Some(session) = self.session.take() else {
            return (false, None);
        };
        let before = (session.samples.len(), self.flying);
        self.flying = observed.flying;
        let grown = match observed.sample.filter(|_| observed.flying) {
            Some(sample) => session.add(sample),
            None => session,
        };
        let due = now_ms.saturating_sub(grown.saved_ms) >= SAVE_INTERVAL_MS;
        let save = due.then(|| grown.saved()).flatten();
        let moved = (grown.samples.len(), self.flying) != before;
        self.session = Some(Session {
            saved_ms: if due { now_ms } else { grown.saved_ms },
            ..grown
        });
        (moved, save)
    }
}

fn write(file: &str, text: &str) -> Result<(), String> {
    crate::vfs::replace(file, text).map_err(|e| e.to_string())
}

pub fn tick(now_ms: u64) {
    let Some(vehicle) = lock().session.as_ref().map(|s| s.vehicle) else {
        return;
    };
    let observed = crate::hub::lock()
        .vehicle(vehicle)
        .map(crate::hub::Vehicle::route_observation)
        .unwrap_or(Observed {
            flying: false,
            sample: None,
        });
    let moved = {
        let mut recorder = lock();
        let (moved, save) = recorder.observe(observed, now_ms);
        if let Some((file, text)) = save
            && let Err(error) = write(&file, &text)
        {
            log::warn!(target: "routerec", "could not save {file}: {error}");
        }
        moved
    };
    if moved {
        changed();
    }
}

fn refused(token: &str, reason: &str) -> Value {
    json!({ "ok": false, "refusal": token, "reason": reason })
}

fn route_name(now: chrono::DateTime<chrono::Local>) -> String {
    format!("Route {}", now.format("%Y-%m-%d %H-%M-%S"))
}

fn start(folder: Option<String>, vehicle: Option<u8>, now_ms: u64) -> Value {
    let Some(vehicle) = vehicle else {
        return refused("noVehicle", "No vehicle is connected.");
    };
    let Some(folder) = folder.filter(|f| !f.is_empty()) else {
        return refused("noFolder", NO_FOLDER);
    };
    if lock().session.is_some() {
        return refused("recording", "A route is already being recorded.");
    }
    let _ = crate::vfs::create_dir_all(&folder);
    let name = route_name(chrono::Local::now());
    let file = std::path::Path::new(&folder)
        .join(format!("{name}.kml"))
        .to_string_lossy()
        .into_owned();
    {
        let mut recorder = lock();
        recorder.session = Some(Session {
            vehicle,
            name,
            file: file.clone(),
            samples: Vec::new(),
            distance: 0.0,
            saved_ms: now_ms,
        });
        recorder.flying = false;
        recorder.outcome = None;
    }
    changed();
    json!({ "ok": true, "refusal": Value::Null, "reason": Value::Null, "file": file })
}

fn stop() -> Value {
    let Some(session) = lock().session.take() else {
        return refused("notRecording", "No route is being recorded.");
    };
    let outcome = match session.saved() {
        None => {
            let _ = crate::vfs::remove(&session.file);
            Outcome {
                file: None,
                message: TOO_SHORT.to_string(),
            }
        }
        Some((file, text)) => match write(&file, &text) {
            Ok(()) => Outcome {
                file: Some(file),
                message: ROUTE_SAVED.to_string(),
            },
            Err(_) => Outcome {
                file: None,
                message: format!("Route save error {file}"),
            },
        },
    };
    let answer = json!({ "ok": true, "refusal": Value::Null, "reason": Value::Null, "savedFile": outcome.file, "message": outcome.message });
    lock().outcome = Some(outcome);
    changed();
    answer
}

pub fn renamed_path(file: &str, new_name: &str) -> Result<String, &'static str> {
    let cleaned = new_name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            other => other,
        })
        .collect::<String>()
        .trim()
        .to_string();
    let path = std::path::Path::new(file);
    if cleaned.is_empty() {
        return Err("A route needs a name.");
    }
    if !crate::vfs::exists(path) {
        return Err("The route file is no longer there.");
    }
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let target = path
        .with_file_name(format!("{cleaned}.{extension}"))
        .to_string_lossy()
        .into_owned();
    match target == file || crate::vfs::exists(&target) {
        true => Err(NAME_USED),
        false => Ok(target),
    }
}

fn rename(file: &str, new_name: &str) -> Value {
    let moved = renamed_path(file, new_name).and_then(|target| {
        crate::vfs::read(file)
            .and_then(|bytes| crate::vfs::write(&target, bytes))
            .and_then(|()| crate::vfs::remove(file))
            .map(|()| target)
            .map_err(|_| "The route file could not be renamed.")
    });
    match moved {
        Ok(target) => {
            {
                let mut recorder = lock();
                if let Some(outcome) = recorder
                    .outcome
                    .as_mut()
                    .filter(|o| o.file.as_deref() == Some(file))
                {
                    outcome.file = Some(target.clone());
                    outcome.message = ROUTE_SAVED.to_string();
                }
            }
            changed();
            json!({ "ok": true, "refusal": Value::Null, "reason": Value::Null, "path": target })
        }
        Err(reason) => {
            json!({ "ok": false, "refusal": if reason == NAME_USED { "nameUsed" } else { "notRenamed" }, "reason": reason, "path": file })
        }
    }
}

fn dismiss() -> Value {
    lock().outcome = None;
    changed();
    json!({ "ok": true, "refusal": Value::Null, "reason": Value::Null })
}

pub fn run(path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let text = |i: usize| given.get(i).and_then(Value::as_str).unwrap_or("");
    match path {
        ROUTE_START => {
            let vehicle = crate::hub::lock().active_id();
            start(
                crate::settingsstore::mission_save_path(),
                vehicle,
                crate::hub::now_ms(),
            )
        }
        ROUTE_STOP => stop(),
        ROUTE_RENAME => rename(text(0), text(1)),
        ROUTE_DISMISS => dismiss(),
        _ => refused("unknown", "Not a route recorder action."),
    }
}

fn stem(file: &str) -> Option<String> {
    std::path::Path::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(str::to_string)
}

pub fn route_recorder_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let unit = Unit::horizontal(backend);
    let available = flag(
        &backend.value_fields("vehicles", "activeVehicleAvailable"),
        "activeVehicleAvailable",
    );
    let (session, flying, outcome) = {
        let recorder = lock();
        (
            recorder.session.clone(),
            recorder.flying,
            recorder.outcome.clone(),
        )
    };
    let distance = session.as_ref().map_or(0.0, |s| s.distance);
    let distance_text = format_measure(unit.show(distance), &unit.name);
    let recording = session.is_some();
    let waiting = recording && !flying && distance <= 0.0;
    json!({
        "kind": "object",
        "class": "RouteRecorder",
        "available": available || recording,
        "recording": recording,
        "waitingForFlight": waiting,
        "distanceMetres": distance,
        "distanceText": distance_text,
        "text": recording.then(|| match waiting {
            true => "Recording route · starts when the vehicle is flying".to_string(),
            false => format!("Recording route · {distance_text}"),
        }),
        "file": session.as_ref().map(|s| s.file.clone()),
        "path": session.map(|s| s.samples.iter().map(|sample| json!({ "latitude": sample.position.0, "longitude": sample.position.1 })).collect::<Vec<_>>()).unwrap_or_default(),
        "savedFile": outcome.as_ref().and_then(|o| o.file.clone()),
        "savedName": outcome.as_ref().and_then(|o| o.file.as_deref().and_then(stem)),
        "message": outcome.map(|o| o.message),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GROUND: Position = (41.7100, 44.7900, 600.0);

    fn vertical(at: Position, from: f64, to: f64) -> Vec<Position> {
        let step = if to > from { 3.0 } else { -3.0 };
        (0..)
            .map(|i| from + i as f64 * step)
            .take_while(|altitude| match step > 0.0 {
                true => *altitude <= to,
                false => *altitude >= to,
            })
            .map(|altitude| (at.0, at.1, altitude))
            .collect()
    }

    fn away(from: Position, distance: f64, azimuth: f64, altitude: f64) -> Position {
        let (lat, lon) =
            crate::surveygrid::at_distance_and_azimuth((from.0, from.1), distance, azimuth);
        (lat, lon, altitude)
    }

    #[test]
    fn climb_and_descent_at_the_ends_are_trimmed_like_route_recorder_test() {
        let top = (GROUND.0, GROUND.1, 660.0);
        let out: Vec<Position> = (1..=50)
            .map(|i| away(top, i as f64 * 2.0, 0.0, 660.0))
            .collect();
        let last = *out.last().unwrap();
        let path: Vec<Position> = [
            vertical(GROUND, 600.0, 660.0),
            out,
            vertical(last, 660.0, 600.0),
        ]
        .concat();
        let route: Vec<Position> = route_indices(&path).into_iter().map(|i| path[i]).collect();
        assert_eq!(route.len(), 2);
        assert_eq!((route[0].2, route[1].2), (660.0, 660.0));
        assert!(horizontal(route[0], route[1]) > 75.0);
    }

    #[test]
    fn hovering_records_no_route() {
        let path: Vec<Position> = vertical(GROUND, 600.0, 660.0)
            .into_iter()
            .chain(std::iter::once(away(GROUND, 4.0, 45.0, 660.0)))
            .collect();
        assert!(route_indices(&path).is_empty());
        assert!(route_indices(&[]).is_empty());
    }

    fn sample(position: Position) -> Sample {
        Sample {
            position,
            gimbal: Some((-30.0, 10.0)),
            look_at: None,
        }
    }

    #[test]
    fn samples_are_taken_only_while_flying_and_two_metres_apart_and_saved_every_ten_seconds() {
        let mut recorder = Recorder {
            session: Some(Session {
                vehicle: 1,
                name: "Route".into(),
                file: "/tmp/route.kml".into(),
                samples: Vec::new(),
                distance: 0.0,
                saved_ms: 0,
            }),
            ..Recorder::default()
        };
        let at = |position: Position, flying: bool| Observed {
            flying,
            sample: Some(sample(position)),
        };
        assert_eq!(recorder.observe(at(GROUND, false), 100), (false, None));
        assert_eq!(recorder.observe(at(GROUND, true), 200).0, true);
        assert_eq!(
            recorder.observe(at(away(GROUND, 1.0, 0.0, 601.0), true), 300),
            (false, None),
            "closer than 2 m both ways is the same sample"
        );
        recorder.observe(at(away(GROUND, 1.0, 0.0, 603.0), true), 400);
        recorder.observe(at(away(GROUND, 50.0, 0.0, 603.0), true), 500);
        let session = recorder.session.as_ref().unwrap();
        assert_eq!(session.samples.len(), 3, "a climb of 2 m is a sample");
        assert!(
            (session.distance - 50.0).abs() < 0.5,
            "{}",
            session.distance
        );
        let (_, save) = recorder.observe(at(away(GROUND, 100.0, 0.0, 603.0), true), 10_000);
        let (file, text) = save.expect("ten seconds after the start the file is written");
        assert_eq!(file, "/tmp/route.kml");
        assert!(text.contains("<name>Route</name>"));
        assert_eq!(
            recorder
                .observe(at(away(GROUND, 150.0, 0.0, 603.0), true), 12_000)
                .1,
            None
        );
    }

    #[test]
    fn a_saved_route_loads_back_with_absolute_altitudes_and_its_camera() {
        let tower = away(GROUND, 200.0, 60.0, 605.0);
        let saved = [
            Sample {
                position: away(GROUND, 0.0, 0.0, 660.0),
                gimbal: Some((-30.0, 10.0)),
                look_at: None,
            },
            Sample {
                position: away(GROUND, 150.0, 45.0, 670.0),
                gimbal: Some((-45.0, 0.0)),
                look_at: Some(tower),
            },
            Sample {
                position: away(GROUND, 300.0, 90.0, 680.0),
                gimbal: None,
                look_at: None,
            },
        ];
        let text = kml("Route", &saved);
        assert!(
            text.contains("<Data name=\"gimbal\">")
                && text.contains("<altitudeMode>absolute</altitudeMode>")
        );
        let loaded = crate::kml::route(&text).unwrap();
        assert_eq!(loaded.altitude, crate::kml::RouteAltitude::Absolute);
        assert_eq!(loaded.path.len(), 3);
        loaded.path.iter().zip(&saved).for_each(|(back, sample)| {
            assert!(horizontal(*back, sample.position) < 0.01);
            assert_eq!(back.2, sample.position.2);
        });
        assert_eq!(loaded.gimbal[0], (-30.0, 10.0));
        assert!(loaded.gimbal[2].0.is_nan());
        assert_eq!(loaded.look_at[0], None);
        let looked = loaded.look_at[1].unwrap();
        assert!(horizontal(looked, tower) < 0.05);
        assert_eq!(looked.2, tower.2);
    }

    #[test]
    fn rename_keeps_the_extension_and_refuses_to_overwrite() {
        let folder = std::env::temp_dir().join(format!("qgc-route-rename-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let original = folder.join("Route 1.kml").to_string_lossy().into_owned();
        let other = folder.join("Tower line.kml").to_string_lossy().into_owned();
        [&original, &other]
            .iter()
            .for_each(|path| std::fs::write(path, "kml").unwrap());
        assert_eq!(renamed_path(&original, "Tower line"), Err(NAME_USED));
        assert_eq!(renamed_path(&original, "Route 1"), Err(NAME_USED));
        assert!(
            renamed_path(&original, " / ").is_ok(),
            "a slash alone is still a name"
        );
        assert_eq!(renamed_path(&original, "   "), Err("A route needs a name."));
        let answer = rename(&original, " Tower 7/North ");
        let target = folder
            .join("Tower 7-North.kml")
            .to_string_lossy()
            .into_owned();
        assert_eq!(answer["path"], json!(target));
        assert!(std::path::Path::new(&target).exists());
        assert!(!std::path::Path::new(&original).exists());
        assert_eq!(rename(&other, "Tower 7-North")["refusal"], "nameUsed");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_route_file_is_named_for_when_recording_started() {
        use chrono::TimeZone;
        let when = chrono::Local
            .with_ymd_and_hms(2026, 10, 24, 9, 5, 7)
            .unwrap();
        assert_eq!(route_name(when), "Route 2026-10-24 09-05-07");
    }

    #[test]
    fn starting_needs_a_vehicle_and_a_missions_folder() {
        assert_eq!(start(Some("/tmp".into()), None, 0)["refusal"], "noVehicle");
        assert_eq!(start(None, Some(1), 0)["reason"], NO_FOLDER);
    }
}
