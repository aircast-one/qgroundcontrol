use serde_json::{Value, json};
use std::sync::{LazyLock, Mutex, PoisonError};

use crate::plandoc::{self, Document, Downloaded};
use crate::plantransfer::Item;
use crate::router::Backend;

pub const OPEN: &str = "core.plan.open";
pub const SAVE: &str = "core.plan.save";
pub const SEND: &str = "core.plan.send";
pub const FETCH: &str = "core.plan.fetch";
pub const STATUS: &str = "core.plan.status";
pub const CORE_INSERT_WAYPOINT: &str = "core.plan.insertWaypoint";
pub const CORE_REMOVE: &str = "core.plan.remove";
pub const CORE_INSERT_LAND: &str = "core.plan.insertLand";
pub const CORE_SET_COMMAND: &str = "core.plan.setCommand";
pub const CORE_SET_ALTITUDE: &str = "core.plan.setAltitude";
pub const CORE_INSERT_TAKEOFF: &str = "core.plan.insertTakeoff";
pub const CORE_SET_ALTITUDE_MODE: &str = "core.plan.setAltitudeMode";
pub const CORE_ITEMS: &str = "core.plan.items";
pub const APPLY_DEFAULT_ALTITUDE: &str = "core.plan.applyDefaultAltitude";
pub const DISMISS_ALTITUDE_PROMPT: &str = "core.plan.dismissAltitudePrompt";
const ACTIONS: &[&str] = &[OPEN, SAVE, SEND, FETCH, STATUS, CORE_INSERT_WAYPOINT, CORE_REMOVE, CORE_INSERT_LAND, CORE_SET_COMMAND, CORE_SET_ALTITUDE, CORE_INSERT_TAKEOFF, CORE_SET_ALTITUDE_MODE, CORE_ITEMS, APPLY_DEFAULT_ALTITUDE, DISMISS_ALTITUDE_PROMPT];
const CMD_NAV_LAND: i64 = 21;
const CMD_NAV_VTOL_LAND: i64 = 85;
const APPLY_ALTITUDE_TITLE: &str = "Apply new altitude";
const APPLY_ALTITUDE_TEXT: &str = "You have changed the default altitude for mission items. Would you like to apply that altitude to all the items in the current mission?";
static ASK_APPLY_ALTITUDE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
const DEFAULT_ALTITUDE: &str = "settings.appSettings.defaultMissionItemAltitude";

#[derive(Default)]
struct Held {
    document: Option<Document>,
    fetching: bool,
    selected: i64,
    file: Option<String>,
    dirty: bool,
    clean: Option<Document>,
    undo: Vec<Document>,
    redo: Vec<Document>,
    last_change_ms: u64,
    shown_vehicle: Option<u8>,
}

const UNDO_DEPTH: usize = 100;
const UNDO_COALESCE_MS: u64 = 500;

fn remember(state: &mut Held, before: Option<Document>, now: u64) {
    let burst = now.saturating_sub(state.last_change_ms) < UNDO_COALESCE_MS && !state.undo.is_empty();
    state.last_change_ms = now;
    if burst || before.is_none() || before == state.document {
        return;
    }
    state.undo = state.undo.iter().cloned().chain(before).rev().take(UNDO_DEPTH).collect::<Vec<_>>().into_iter().rev().collect();
    state.redo.clear();
}

fn settle_clean(state: &mut Held) {
    state.dirty = false;
    state.clean = state.document.clone();
}

pub const CHANGED: &str = "core.plan@changed";

pub static ON_CHANGE: std::sync::Mutex<Option<std::sync::Arc<dyn Fn() + Send + Sync>>> = std::sync::Mutex::new(None);

#[cfg(not(test))]
const ON_WITHOUT_SWITCH: bool = true;
#[cfg(test)]
const ON_WITHOUT_SWITCH: bool = false;

static ENABLED: LazyLock<bool> = LazyLock::new(|| std::env::var("QGC_CORE_PLAN").map_or(ON_WITHOUT_SWITCH, |v| v == "1"));

pub fn enabled() -> bool {
    *ENABLED
}

static UNDO_TRACKING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn undo_tracking() -> bool {
    UNDO_TRACKING.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn note_undo_tracking(on: bool) {
    UNDO_TRACKING.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn offline() -> bool {
    crate::hub::lock().active_id().is_none()
}

fn changed() {
    let notify = ON_CHANGE.lock().unwrap_or_else(PoisonError::into_inner).clone();
    if let Some(notify) = notify {
        notify();
    }
}

static HELD: LazyLock<Mutex<Held>> = LazyLock::new(|| Mutex::new(Held::default()));

pub fn owns(path: &str) -> bool {
    ACTIONS.contains(&path)
}

fn refused(reason: impl Into<String>) -> Value {
    json!({ "ok": false, "reason": reason.into() })
}

fn first_text(args: &str) -> Option<String> {
    serde_json::from_str::<Value>(args).ok()?.get(0)?.as_str().map(str::to_string)
}

pub fn act(backend: &dyn Backend, path: &str, args: &str) -> Value {
    match path {
        OPEN => first_text(args).map_or_else(|| refused("Open needs the path of a .plan file."), |file| open(&file)),
        SAVE => first_text(args).map_or_else(|| refused("Save needs a path to write the plan to."), |file| save(&file)),
        SEND => send(),
        FETCH => fetch(),
        STATUS => status(),
        CORE_INSERT_WAYPOINT => insert_at(backend, args, false),
        CORE_INSERT_LAND => insert_at(backend, args, true),
        CORE_SET_COMMAND => item_edit(backend, args, true),
        CORE_SET_ALTITUDE => item_edit(backend, args, false),
        CORE_INSERT_TAKEOFF => insert_takeoff(backend, args),
        CORE_SET_ALTITUDE_MODE => set_altitude_mode(args),
        CORE_ITEMS => items(backend, args),
        APPLY_DEFAULT_ALTITUDE => apply_default_altitude(backend),
        DISMISS_ALTITUDE_PROMPT => {
            ASK_APPLY_ALTITUDE.store(false, std::sync::atomic::Ordering::Relaxed);
            changed();
            json!({ "ok": true })
        }
        CORE_REMOVE => remove(args),
        _ => refused(format!("{path} is not a plan action the core performs")),
    }
}

fn held() -> std::sync::MutexGuard<'static, Held> {
    HELD.lock().unwrap_or_else(PoisonError::into_inner)
}

fn home_of(document: Option<&Document>) -> Option<(f64, f64)> {
    document.and_then(|d| d.home).map(|h| (h[0], h[1]))
}

fn settle_home_on_terrain(before: Option<(f64, f64)>) {
    let Some((latitude, longitude)) = home_of(held().document.as_ref()).filter(|now| Some(*now) != before) else {
        return;
    };
    std::thread::spawn(move || {
        let Ok(ground) = crate::terrainquery::elevation(latitude, longitude, None, &crate::terrainquery::fetch_over_http) else {
            return;
        };
        let settled = {
            let mut state = held();
            let same = home_of(state.document.as_ref()) == Some((latitude, longitude));
            if same {
                let moved = state.document.as_ref().and_then(|d| d.home).is_some_and(|h| h[2] != ground);
                state.document = state.document.take().map(|d| Document { home: d.home.map(|h| [h[0], h[1], ground]), ..d });
                state.dirty = state.dirty || moved;
            }
            same
        };
        if settled {
            changed();
        }
    });
}

fn edit(change: impl FnOnce(&Document) -> Result<Document, String>) -> Value {
    let (answer, before) = {
        let mut state = held();
        let Some(current) = state.document.as_ref() else {
            return refused("There is no plan to edit.");
        };
        let before = home_of(Some(current));
        match change(current) {
            Ok(changed) => {
                let count = changed.items.len();
                let previous = state.document.replace(changed);
                remember(&mut state, previous, crate::hub::now_ms());
                state.dirty = true;
                (json!({ "ok": true, "items": count }), before)
            }
            Err(reason) => return refused(reason),
        }
    };
    settle_home_on_terrain(before);
    changed();
    answer
}

pub fn apply(change: impl FnOnce(&Document) -> Result<Document, String>) -> Value {
    edit(change)
}

fn edit_defaults(backend: &dyn Backend) -> Option<plandoc::EditDefaults> {
    crate::read::value_number(&backend.get(&format!("{DEFAULT_ALTITUDE}.rawValue"))).map(|mission_item_altitude| plandoc::EditDefaults { mission_item_altitude })
}

fn insert_at(backend: &dyn Backend, args: &str, land: bool) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let number = |i: usize| given.get(i).and_then(Value::as_f64).filter(|v| v.is_finite());
    let (Some(latitude), Some(longitude)) = (number(0), number(1)) else {
        return refused("An item needs a latitude and a longitude.");
    };
    let index = given.get(2).and_then(Value::as_i64).unwrap_or(-1);
    let Some(defaults) = edit_defaults(backend) else {
        return refused("The default mission item altitude is not known.");
    };
    edit(|doc| match land {
        true => plandoc::insert_land(doc, latitude, longitude, index, &defaults),
        false => Ok(plandoc::insert_waypoint(doc, latitude, longitude, index, &defaults)),
    })
}

fn insert_takeoff(backend: &dyn Backend, args: &str) -> Value {
    let index = serde_json::from_str::<Value>(args).ok().and_then(|v| v.get(0).and_then(Value::as_i64)).unwrap_or(-1);
    let Some(defaults) = edit_defaults(backend) else {
        return refused("The default mission item altitude is not known.");
    };
    edit(|doc| plandoc::insert_takeoff(doc, index, &defaults))
}

fn remember_patterns<'a>(backend: &dyn Backend, vehicle_type: i64, items: impl Iterator<Item = &'a plandoc::Item>) {
    let multirotor = plandoc::vehicle_class(vehicle_type) == crate::cmdinfo::VehicleClass::MultiRotor;
    items
        .filter_map(|item| match item {
            plandoc::Item::Complex { json, .. } => Some(crate::surveydoc::changed_remembered(json, multirotor, &crate::settingsstore::stored_text)),
            plandoc::Item::Simple(_) => None,
        })
        .flatten()
        .for_each(|(key, value)| backend.remember_setting(&key, &value));
}

fn remember_item(backend: &dyn Backend, visual_index: i64) {
    let Some(document) = held().document.clone() else { return };
    let at = usize::try_from(visual_index - 1).ok();
    remember_patterns(backend, document.vehicle_type, at.and_then(|i| document.items.get(i)).into_iter());
}

fn insert_landing(_backend: &dyn Backend, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let number = |i: usize| given.get(i).and_then(Value::as_f64).filter(|v| v.is_finite());
    let (Some(latitude), Some(longitude)) = (number(0), number(1)) else {
        return refused("A landing pattern needs a latitude and a longitude.");
    };
    let index = given.get(2).and_then(Value::as_i64).unwrap_or(-1);
    edit(|doc| {
        let vtol = plandoc::vehicle_class(doc.vehicle_type) == crate::cmdinfo::VehicleClass::Vtol;
        let built = crate::landingpattern::fresh(&crate::landingpattern::Fresh {
            vtol,
            land: (latitude, longitude),
            ardupilot: plandoc::firmware(doc.firmware_type) == crate::cmdinfo::Firmware::ArduPilot,
            relative: doc.global_altitude_mode != crate::altitudemodes::ABSOLUTE,
        });
        let kind = if vtol { crate::landingpattern::VTOL_PATTERN } else { crate::landingpattern::FIXED_WING_PATTERN };
        Ok(plandoc::insert_complex(doc, kind, built, (latitude, longitude), index))
    })
}

fn insert_structure(backend: &dyn Backend, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let number = |i: usize| given.get(i).and_then(Value::as_f64).filter(|v| v.is_finite());
    let (Some(latitude), Some(longitude)) = (number(0), number(1)) else {
        return refused("A structure scan needs a latitude and a longitude.");
    };
    let index = given.get(2).and_then(Value::as_i64).unwrap_or(-1);
    let Some(defaults) = edit_defaults(backend) else {
        return refused("The default mission item altitude is not known.");
    };
    edit(|doc| {
        let distance_mode = match doc.global_altitude_mode {
            crate::altitudemodes::MIXED => crate::altitudemodes::RELATIVE,
            mode => mode,
        };
        let built = crate::surveydoc::fresh_structure(&crate::surveydoc::Fresh {
            center: (latitude, longitude),
            remembered: &crate::settingsstore::stored_text,
            multirotor: plandoc::vehicle_class(doc.vehicle_type) == crate::cmdinfo::VehicleClass::MultiRotor,
            alternates: false,
            default_altitude: defaults.mission_item_altitude,
            distance_mode,
            previous_mode: None,
        });
        Ok(plandoc::insert_complex(doc, "StructureScan", built, (latitude, longitude), index))
    })
}

fn insert_roi(backend: &dyn Backend, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let number = |i: usize| given.get(i).and_then(Value::as_f64).filter(|v| v.is_finite());
    let (Some(latitude), Some(longitude)) = (number(0), number(1)) else {
        return refused("A region of interest needs a latitude and a longitude.");
    };
    let index = given.get(2).and_then(Value::as_i64).unwrap_or(-1);
    let Some(defaults) = edit_defaults(backend) else {
        return refused("The default mission item altitude is not known.");
    };
    edit(|doc| Ok(plandoc::insert_roi(doc, latitude, longitude, index, &defaults)))
}

fn insert_scan(backend: &dyn Backend, args: &str, corridor: bool) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let number = |i: usize| given.get(i).and_then(Value::as_f64).filter(|v| v.is_finite());
    let (Some(latitude), Some(longitude)) = (number(0), number(1)) else {
        return refused("A scan needs a latitude and a longitude.");
    };
    let index = given.get(2).and_then(Value::as_i64).unwrap_or(-1);
    let Some(defaults) = edit_defaults(backend) else {
        return refused("The default mission item altitude is not known.");
    };
    edit(|doc| {
        let class = plandoc::vehicle_class(doc.vehicle_type);
        let distance_mode = match doc.global_altitude_mode {
            crate::altitudemodes::MIXED => crate::altitudemodes::RELATIVE,
            mode => mode,
        };
        let previous_mode = (doc.global_altitude_mode == crate::altitudemodes::MIXED && !corridor).then(|| plandoc::previous_altitude_mode(doc, index)).flatten();
        let fresh = crate::surveydoc::Fresh {
            center: (latitude, longitude),
            remembered: &crate::settingsstore::stored_text,
            multirotor: class == crate::cmdinfo::VehicleClass::MultiRotor,
            alternates: matches!(class, crate::cmdinfo::VehicleClass::FixedWing | crate::cmdinfo::VehicleClass::Vtol),
            default_altitude: defaults.mission_item_altitude,
            distance_mode,
            previous_mode,
        };
        let (kind, built) = match corridor {
            true => ("CorridorScan", crate::surveydoc::fresh_corridor(&fresh)),
            false => ("survey", crate::surveydoc::fresh(&fresh)),
        };
        Ok(plandoc::insert_complex(doc, kind, with_flight_speed(doc, built), (latitude, longitude), index))
    })
}

fn flight_speed(doc: &Document) -> f64 {
    use crate::cmdinfo::VehicleClass::{MultiRotor, Vtol};
    match plandoc::vehicle_class(doc.vehicle_type) {
        MultiRotor | Vtol => doc.hover_speed,
        _ => doc.cruise_speed,
    }
}

fn with_flight_speed(doc: &Document, item: Value) -> Value {
    let calc_mode = item.pointer("/TransectStyleComplexItem/CameraCalc/DistanceMode").and_then(Value::as_i64);
    if calc_mode != Some(crate::altitudemodes::CALC_ABOVE_TERRAIN) {
        return item;
    }
    let mut stamped = item;
    stamped["TransectStyleComplexItem"][crate::surveydoc::TERRAIN_FLIGHT_SPEED] = json!(flight_speed(doc));
    crate::surveydoc::regenerate_item(&stamped)
}

const WAITING_ON_TERRAIN: &str = "Plan is waiting on terrain data from server for correct altitude values.";

fn waiting_on_terrain(document: &Document) -> bool {
    document.items.iter().any(|item| matches!(item, plandoc::Item::Complex { json, .. } if crate::surveydoc::waiting_for_terrain(json)))
}

pub fn terrain_arrived() {
    let refreshed = {
        let mut state = held();
        let Some(current) = state.document.clone().filter(waiting_on_terrain) else { return };
        let items = current
            .items
            .iter()
            .map(|item| match item {
                plandoc::Item::Complex { kind, json, item_count } if crate::surveydoc::waiting_for_terrain(json) => {
                    let regenerated = crate::surveydoc::regenerate_item(json);
                    plandoc::Item::Complex { kind: kind.clone(), item_count: plandoc::complex_count(kind, &regenerated).unwrap_or(*item_count), json: regenerated }
                }
                other => other.clone(),
            })
            .collect();
        let refreshed = Document { items, ..current };
        let changed_any = state.document.as_ref() != Some(&refreshed);
        state.document = Some(refreshed);
        changed_any
    };
    if refreshed {
        changed();
    }
}

fn set_altitude_mode(args: &str) -> Value {
    let mode = serde_json::from_str::<Value>(args).ok().and_then(|v| v.get(0).and_then(Value::as_i64));
    match mode.filter(|m| (crate::altitudemodes::MIXED..=crate::altitudemodes::TERRAIN_FRAME).contains(m)) {
        Some(mode) => edit(|doc| Ok(plandoc::set_global_altitude_mode(doc, mode))),
        None => refused("An altitude mode is a number from 0 to 4."),
    }
}

fn items(backend: &dyn Backend, args: &str) -> Value {
    let selected = serde_json::from_str::<Value>(args).ok().and_then(|v| v.get(0).and_then(Value::as_i64)).unwrap_or(0);
    let Some(document) = held().document.clone() else {
        return refused("There is no plan.");
    };
    let rover = crate::read::flag(&crate::read::object(&backend.get_fields("plan.controllerVehicle", "rover")), "rover");
    match crate::missionitems::document_view(&document, selected, &crate::read::Unit::vertical(backend), &crate::read::Unit::speed(backend), crate::missionsummary::imperial(backend), rover) {
        Ok(view) => json!({ "ok": true, "view": view }),
        Err(reason) => refused(reason),
    }
}

fn item_edit(backend: &dyn Backend, args: &str, command: bool) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let (Some(index), Some(value)) = (given.get(0).and_then(Value::as_u64).and_then(|i| usize::try_from(i).ok()), given.get(1).and_then(Value::as_f64).filter(|v| v.is_finite())) else {
        return refused("An item edit needs the item's index and a number.");
    };
    let Some(defaults) = edit_defaults(backend) else {
        return refused("The default mission item altitude is not known.");
    };
    edit(|doc| {
        let changed = match command {
            true => plandoc::set_command(doc, index, value as i64, &defaults),
            false => plandoc::set_altitude(doc, index, value),
        };
        changed.ok_or_else(|| format!("Item {index} cannot take that edit."))
    })
}

pub fn default_altitude_changed() {
    let has_items = held().document.as_ref().is_some_and(|d| !d.items.is_empty());
    if has_items {
        ASK_APPLY_ALTITUDE.store(true, std::sync::atomic::Ordering::Relaxed);
        changed();
    }
}

pub fn altitude_prompt() -> Value {
    match ASK_APPLY_ALTITUDE.load(std::sync::atomic::Ordering::Relaxed) {
        true => json!({ "title": APPLY_ALTITUDE_TITLE, "text": APPLY_ALTITUDE_TEXT }),
        false => Value::Null,
    }
}

pub fn with_new_altitude(item: &plandoc::Item, altitude: f64) -> plandoc::Item {
    let metres = crate::read::Unit { name: "m".to_string(), factor: 1.0 };
    let units = crate::surveydoc::Units { vertical: &metres, horizontal: &metres };
    match item {
        plandoc::Item::Simple(simple) if simple.altitude.is_some() && ![CMD_NAV_LAND, CMD_NAV_VTOL_LAND].contains(&simple.command) => {
            let params = std::array::from_fn(|i| if i == 6 { Some(altitude) } else { simple.params[i] });
            plandoc::Item::Simple(plandoc::Simple { params, altitude: simple.altitude.clone().map(|held| plandoc::Altitude { altitude, ..held }), ..simple.clone() })
        }
        plandoc::Item::Complex { kind, json, item_count } => {
            let edited = match kind.as_str() {
                k if crate::landingpattern::is_landing(k) => crate::landingpattern::edit(json, "finalApproachAltitude", &json!(altitude)),
                "StructureScan" => crate::surveydoc::set(json, "entranceAlt", &json!(altitude), &units),
                _ => crate::surveydoc::set(json, "cameraCalc.valueSetIsDistance", &json!(true), &units).and_then(|distance| crate::surveydoc::set(&distance, "cameraCalc.distanceToSurface", &json!(altitude), &units)),
            };
            match edited {
                Some(edited) => plandoc::Item::Complex { kind: kind.clone(), item_count: plandoc::complex_count(kind, &edited).unwrap_or(*item_count), json: edited },
                None => item.clone(),
            }
        }
        other => other.clone(),
    }
}

fn apply_default_altitude(backend: &dyn Backend) -> Value {
    ASK_APPLY_ALTITUDE.store(false, std::sync::atomic::Ordering::Relaxed);
    let Some(defaults) = edit_defaults(backend) else {
        changed();
        return refused("The default mission item altitude is not known.");
    };
    edit(|doc| Ok(Document { items: doc.items.iter().map(|item| with_new_altitude(item, defaults.mission_item_altitude)).collect(), ..doc.clone() }))
}

fn remove(args: &str) -> Value {
    let index = serde_json::from_str::<Value>(args).ok().and_then(|v| v.get(0).and_then(Value::as_u64));
    let Some(index) = index.and_then(|i| usize::try_from(i).ok()) else {
        return refused("Remove needs the index of the item.");
    };
    edit(|doc| plandoc::remove(doc, index).ok_or_else(|| format!("This plan has no item {index} to remove.")))
}

fn open(file: &str) -> Value {
    let loaded = std::fs::read_to_string(file).map_err(|e| format!("Could not read {file}: {e}")).and_then(|text| plandoc::load(&text));
    match loaded {
        Ok(document) => {
            let count = document.items.len();
            {
                let mut state = held();
                let before = state.document.take();
                state.document = Some(document);
                remember(&mut state, before, crate::hub::now_ms());
                state.selected = 0;
                state.file = Some(file.to_string());
                settle_clean(&mut state);
            }
            settle_home_on_terrain(None);
            changed();
            json!({ "ok": true, "items": count, "result": true })
        }
        Err(reason) => refused(reason),
    }
}

fn save(file: &str) -> Value {
    if held().document.as_ref().is_some_and(waiting_on_terrain) {
        return refused(format!("Unable to Save. {WAITING_ON_TERRAIN}"));
    }
    let Some(text) = held().document.as_ref().map(|d| plandoc::save(d).to_string()) else {
        return refused("There is no plan to save.");
    };
    match std::fs::write(file, text) {
        Ok(()) => {
            {
                let mut state = held();
                state.file = Some(file.to_string());
                settle_clean(&mut state);
            }
            changed();
            json!({ "ok": true, "result": true })
        }
        Err(e) => refused(format!("Could not write {file}: {e}")),
    }
}

fn save_kml(file: &str) -> Value {
    let Some(plan) = held().document.as_ref().map(plandoc::save) else {
        return refused("There is no plan to export.");
    };
    let (firmware_type, vehicle_type) = planned_types();
    let planned = crate::plankml::Planned {
        firmware: plandoc::firmware(firmware_type),
        class: plandoc::vehicle_class(vehicle_type),
        application: crate::noticeboard::application_name(),
        vertical: crate::units::cooking("vertical m"),
    };
    let target = crate::plankml::with_extension(file);
    match crate::plankml::document(&plan, &planned).and_then(|kml| std::fs::write(&target, kml).map_err(|e| e.to_string())) {
        Ok(()) => json!({ "ok": true }),
        Err(e) => refused(format!("KML save error {file} : {e}")),
    }
}

fn deliver(outbound: Vec<(u32, Vec<u8>)>) {
    outbound.iter().for_each(|(link, bytes)| {
        crate::linkhost::write(&crate::linkhost::TRANSPORTS, *link, bytes);
    });
}

fn mission_idle(kind: &str) -> bool {
    crate::hub::lock().active().is_none_or(|v| v.mission_snapshot()[kind]["inProgress"].as_bool() != Some(true))
}

fn send_shape(kind: &str, document: &Document) -> Value {
    let pair = |v: &Value| json!([v.get(0), v.get(1)]);
    match kind {
        "fence" => json!({
            "action": "write",
            "plan": "fence",
            "polygons": document.fence["polygons"].as_array().map(|list| list.iter().map(|p| json!({
                "inclusion": p["inclusion"],
                "vertices": p["polygon"].as_array().map(|v| v.iter().map(pair).collect::<Vec<_>>()).unwrap_or_default(),
            })).collect::<Vec<_>>()).unwrap_or_default(),
            "circles": document.fence["circles"].as_array().map(|list| list.iter().map(|c| json!({
                "inclusion": c["inclusion"],
                "center": c["circle"]["center"],
                "radius": c["circle"]["radius"],
            })).collect::<Vec<_>>()).unwrap_or_default(),
            "breachReturn": document.fence.get("breachReturn").cloned().unwrap_or(Value::Null),
        }),
        _ => json!({ "action": "write", "plan": "rally", "points": document.rally["points"].as_array().cloned().unwrap_or_default() }),
    }
}

fn send_after_mission(document: Document) {
    std::thread::spawn(move || {
        let (fence, rally) = crate::hub::lock().active().map_or((false, false), crate::hub::Vehicle::plans_supported);
        let wanted: Vec<&str> = [("fence", fence), ("rally", rally)].into_iter().filter(|(_, supported)| *supported).map(|(k, _)| k).collect();
        let previous = std::iter::once("mission").chain(wanted.iter().copied()).collect::<Vec<_>>();
        wanted.iter().zip(previous).for_each(|(kind, before)| {
            if settle(before) {
                let started = crate::hub::lock().mission_request(None, &send_shape(kind, &document), crate::hub::now_ms());
                if let Ok(outbound) = started {
                    deliver(outbound);
                }
            }
        });
        let _ = wanted.last().is_some_and(|last| settle(last));
        changed();
    });
}

fn carried() -> bool {
    crate::hub::lock().active().is_some()
}

fn host_file() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("groundstation-plan-{}.plan", std::process::id()))
}

fn send_through_host(backend: &dyn Backend) -> Value {
    let Some(document) = held().document.clone() else {
        return refused("There is no plan to send.");
    };
    if waiting_on_terrain(&document) {
        return refused(format!("Unable to Upload. {WAITING_ON_TERRAIN}"));
    }
    let file = host_file();
    if let Err(e) = std::fs::write(&file, plandoc::save(&document).to_string()) {
        return refused(format!("The plan could not be handed to the vehicle link: {e}"));
    }
    let path = file.to_string_lossy().to_string();
    let loaded = crate::read::object(&backend.invoke("plan.loadFromFile", &json!([path]).to_string()));
    if !crate::read::flag(&loaded, "ok") {
        return refused("The vehicle link could not take the plan.");
    }
    let sent = crate::read::object(&backend.invoke("plan.sendToVehicle", "[]"));
    if crate::read::flag(&sent, "ok") {
        settle_clean(&mut held());
        changed();
    }
    sent
}

pub const HOST_SYNC: &str = "plan.syncInProgress";

static HOST_FETCH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn host_watches() -> Vec<String> {
    match HOST_FETCH.load(std::sync::atomic::Ordering::SeqCst) {
        true => vec![HOST_SYNC.to_string()],
        false => Vec::new(),
    }
}

fn host_syncing(backend: &dyn Backend) -> bool {
    crate::read::object(&backend.get(HOST_SYNC)).get("value").and_then(Value::as_bool) == Some(true)
}

fn follow_vehicle() {
    if !enabled() {
        return;
    }
    let shown = held().shown_vehicle;
    let (active, connected) = {
        let hub = crate::hub::lock();
        (hub.active_id(), hub.active().is_some_and(|v| v.connected))
    };
    if active == shown || (active.is_some() && !connected) {
        return;
    }
    let (active, ready) = {
        let hub = crate::hub::lock();
        let ready = hub.active().filter(|v| v.connected).map(|v| {
            let (fence, rally) = v.plans_supported();
            (v.id, v.mission_snapshot(), v.sends_home(), fence, rally, (i64::from(v.autopilot), i64::from(v.vehicle_type)))
        });
        (hub.active_id(), ready)
    };
    let adopted = {
        let mut state = held();
        if state.fetching {
            return;
        }
        match (active, ready) {
            (None, _) if state.shown_vehicle.is_some() => {
                state.shown_vehicle = None;
                let has_items = state.document.as_ref().is_some_and(|d| !d.items.is_empty());
                if state.dirty || !has_items {
                    return;
                }
                let before = state.document.clone();
                state.document = state.document.clone().map(|d| Document { home: None, items: Vec::new(), settings_sections: Vec::new(), ..d });
                remember(&mut state, before, crate::hub::now_ms());
                state.selected = 0;
                settle_clean(&mut state);
                state.file = None;
                None
            }
            (Some(id), Some(vehicle)) if state.shown_vehicle != Some(id) && vehicle.0 == id => {
                state.shown_vehicle = Some(id);
                (!state.dirty).then_some(vehicle)
            }
            _ => return,
        }
    };
    if let Some((_, snapshot, sends_home, fence, rally, types)) = adopted {
        adopt(&snapshot, sends_home, fence, rally, types);
        held().file = None;
    }
    changed();
}

pub fn poll_host(backend: &dyn Backend) {
    follow_vehicle();
    if HOST_FETCH.load(std::sync::atomic::Ordering::SeqCst) && !host_syncing(backend) {
        on_host_event(backend, HOST_SYNC, &json!({ "value": false }).to_string());
    }
}

fn fetch_through_host(backend: &dyn Backend) -> Value {
    let asked = crate::read::object(&backend.invoke("plan.loadFromVehicle", "[]"));
    if !host_syncing(backend) {
        return refused("The vehicle link did not start the download.");
    }
    if crate::read::flag(&asked, "ok") {
        HOST_FETCH.store(true, std::sync::atomic::Ordering::SeqCst);
        held().fetching = true;
        changed();
    }
    asked
}

pub fn on_host_event(backend: &dyn Backend, path: &str, value: &str) -> bool {
    let finished = path == HOST_SYNC && crate::read::object(value).get("value").and_then(Value::as_bool) == Some(false);
    if !finished || !HOST_FETCH.swap(false, std::sync::atomic::Ordering::SeqCst) {
        return false;
    }
    let file = host_file();
    let path = file.to_string_lossy().to_string();
    backend.invoke("plan.saveToFile", &json!([path]).to_string());
    let adopted = std::fs::read_to_string(&file).map_err(|e| e.to_string()).and_then(|text| plandoc::load(&text));
    {
        let mut state = held();
        state.fetching = false;
        if let Ok(document) = adopted {
            let before = state.document.replace(document);
            remember(&mut state, before, crate::hub::now_ms());
            state.selected = 0;
            state.file = None;
            settle_clean(&mut state);
        }
    }
    changed();
    true
}

fn send() -> Value {
    let Some(document) = held().document.clone() else {
        return refused("There is no plan to send.");
    };
    if waiting_on_terrain(&document) {
        return refused(format!("Unable to Upload. {WAITING_ON_TERRAIN}"));
    }
    let items = match crate::planitems::flatten(&plandoc::save(&document)) {
        Ok(items) => items,
        Err(reason) => return refused(reason),
    };
    let transfer: Option<Vec<Item>> = items
        .iter()
        .map(|i| Some(Item { seq: 0, frame: u8::try_from(i.frame).ok()?, command: u16::try_from(i.command).ok()?, current: false, auto_continue: i.auto_continue, params: i.params }))
        .collect();
    let Some(transfer) = transfer else {
        return refused("A mission item's frame or command does not fit MAVLink.");
    };
    let started = crate::hub::lock().write_mission(None, transfer, crate::hub::now_ms());
    match started {
        Ok(outbound) => {
            deliver(outbound);
            settle_clean(&mut held());
            send_after_mission(document);
            changed();
            json!({ "ok": true, "items": items.len() })
        }
        Err(reason) => refused(reason),
    }
}

fn settle(kind: &str) -> bool {
    (0..600).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(100));
        mission_idle(kind)
    })
}

fn load(kind: &str) -> bool {
    let started = crate::hub::lock().mission_request(None, &json!({ "action": "load", "plan": kind }), crate::hub::now_ms());
    match started {
        Ok(outbound) => {
            deliver(outbound);
            settle(kind)
        }
        Err(_) => false,
    }
}

fn number(value: &Value) -> f64 {
    value.as_f64().unwrap_or(f64::NAN)
}

fn fence_from(snapshot: &Value) -> Value {
    json!({
        "version": 2,
        "polygons": snapshot["polygons"].as_array().map(|list| list.iter().map(|p| json!({ "inclusion": p["inclusion"], "polygon": p["vertices"], "version": 1 })).collect::<Vec<_>>()).unwrap_or_default(),
        "circles": snapshot["circles"].as_array().map(|list| list.iter().map(|c| json!({ "inclusion": c["inclusion"], "circle": { "center": c["center"], "radius": c["radius"] }, "version": 1 })).collect::<Vec<_>>()).unwrap_or_default(),
    })
    .as_object()
    .cloned()
    .map(|mut fence| {
        if let Some(back) = snapshot.get("breachReturn").filter(|b| !b.is_null()) {
            fence.insert("breachReturn".to_string(), back.clone());
        }
        Value::Object(fence)
    })
    .unwrap_or(Value::Null)
}

fn remembered_speed(name: &str, fallback: f64) -> f64 {
    crate::settingsstore::stored_text(name).and_then(|text| text.parse().ok()).unwrap_or(fallback)
}

fn adopt(snapshot: &Value, sends_home: bool, fence_read: bool, rally_read: bool, types: (i64, i64)) {
    let downloaded: Vec<Downloaded> = snapshot["mission"]["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|i| Downloaded {
                    frame: i["frame"].as_i64().unwrap_or(0),
                    command: i["command"].as_i64().unwrap_or(0),
                    params: std::array::from_fn(|k| number(&i["params"][k])),
                    auto_continue: i["autoContinue"].as_bool().unwrap_or(true),
                })
                .collect()
        })
        .unwrap_or_default();
    let mut state = held();
    let held_document = state.document.clone().unwrap_or_else(|| Document {
        cruise_speed: remembered_speed("offlineEditingCruiseSpeed", 15.0),
        hover_speed: remembered_speed("offlineEditingHoverSpeed", 5.0),
        ..empty_document()
    });
    let template = Document { firmware_type: types.0, vehicle_type: types.1, ..held_document };
    let before = state.document.clone();
    let mission = plandoc::from_vehicle(&downloaded, sends_home, &template);
    state.document = Some(Document {
        fence: if fence_read { fence_from(&snapshot["fence"]) } else { mission.fence.clone() },
        rally: if rally_read { json!({ "version": 2, "points": snapshot["rally"]["points"] }) } else { mission.rally.clone() },
        ..mission
    });
    remember(&mut state, before, crate::hub::now_ms());
    state.selected = 0;
    settle_clean(&mut state);
    state.fetching = false;
}

fn fetch() -> Value {
    let Some((fence, rally, sends_home, types)) = crate::hub::lock().active().map(|v| {
        let (fence, rally) = v.plans_supported();
        (fence, rally, v.sends_home(), (i64::from(v.autopilot), i64::from(v.vehicle_type)))
    }) else {
        return refused("No vehicle is connected through the core.");
    };
    held().fetching = true;
    std::thread::spawn(move || {
        let mission_read = load("mission");
        let fence_read = fence && load("fence");
        let rally_read = rally && load("rally");
        let snapshot = crate::hub::lock().active().map(crate::hub::Vehicle::mission_snapshot).unwrap_or(Value::Null);
        match mission_read {
            true => {
                adopt(&snapshot, sends_home, fence_read, rally_read, types);
                settle_home_on_terrain(None);
            }
            false => held().fetching = false,
        }
        changed();
    });
    json!({ "ok": true })
}

fn status() -> Value {
    let mission = crate::hub::lock().active().map(|v| v.mission_snapshot()["mission"].clone()).unwrap_or(Value::Null);
    let state = held();
    json!({
        "ok": true,
        "transfer": mission,
        "fetching": state.fetching,
        "items": state.document.as_ref().map(|d| d.items.len()),
    })
}

fn empty_document() -> Document {
    Document {
        firmware_type: 0,
        vehicle_type: 0,
        cruise_speed: 0.0,
        hover_speed: 0.0,
        global_altitude_mode: crate::altitudemodes::RELATIVE,
        home: None,
        settings_sections: Vec::new(),
        items: Vec::new(),
        fence: json!({ "circles": [], "polygons": [], "version": 2 }),
        rally: json!({ "points": [], "version": 2 }),
    }
}

fn marked_edited(view: Value, document: &Document, clean: Option<&Document>) -> Value {
    let mut marked = view;
    if let Some(rows) = marked.get_mut("items").and_then(Value::as_array_mut) {
        rows.iter_mut().skip(1).zip(&document.items).for_each(|(row, item)| row["edited"] = json!(!clean.is_some_and(|c| c.items.contains(item))));
    }
    marked
}

pub fn view(backend: &dyn Backend) -> Value {
    let (document, selected, clean) = {
        let state = held();
        (state.document.clone().unwrap_or_else(empty_document), state.selected, state.clean.clone())
    };
    let rover = crate::read::flag(&crate::read::object(&backend.get_fields("plan.controllerVehicle", "rover")), "rover");
    crate::missionitems::document_view(&document, selected, &crate::read::Unit::vertical(backend), &crate::read::Unit::speed(backend), crate::missionsummary::imperial(backend), rover)
        .map(|view| marked_edited(view, &document, clean.as_ref()))
        .unwrap_or_else(|reason| json!({ "kind": "object", "class": "MissionItems", "available": false, "items": [], "selected": -1, "reason": reason }))
}

pub(crate) fn visual_spans(document: &Document) -> Vec<(i64, i64)> {
    std::iter::once((0usize, document.settings_sections.len()))
        .chain(document.items.iter().scan(document.settings_sections.len() + 1, |next, item| {
            let start = *next;
            let span = match item {
                plandoc::Item::Simple(s) => 1 + s.sections.len(),
                plandoc::Item::Complex { item_count, .. } => *item_count,
            };
            *next += span;
            Some((start, start + span - 1))
        }))
        .map(|(first, last)| (first as i64, last as i64))
        .collect()
}

fn visual_index_of_sequence(document: &Document, sequence: i64) -> Option<i64> {
    visual_spans(document).iter().position(|(first, last)| (*first..=*last).contains(&sequence)).map(|i| i as i64)
}

fn planning_setting(name: &str, unset: bool) -> bool {
    crate::settingsstore::raw_setting(&format!("settings.planViewSettings.{name}")).and_then(|v| v.as_bool()).unwrap_or(unset)
}

fn offline_type(name: &str) -> i64 {
    crate::settingsstore::raw_setting(&format!("settings.appSettings.{name}")).and_then(|v| v.as_i64()).unwrap_or(0)
}

fn planned_types() -> (i64, i64) {
    crate::hub::lock()
        .active()
        .map(|v| (i64::from(v.autopilot), i64::from(v.vehicle_type)))
        .unwrap_or_else(|| (offline_type("offlineEditingFirmwareClass"), offline_type("offlineEditingVehicleClass")))
}

pub fn controller_fields(path: &str) -> Option<Value> {
    if !enabled() {
        return None;
    }
    let (document, selected) = {
        let state = held();
        (state.document.clone().unwrap_or_else(empty_document), state.selected)
    };
    let (firmware_type, vehicle_type) = planned_types();
    let class = plandoc::vehicle_class(vehicle_type);
    let firmware = plandoc::firmware(firmware_type);
    match path {
        "plan.controllerVehicle" => Some(json!({
            "kind": "object",
            "multiRotor": class == crate::cmdinfo::VehicleClass::MultiRotor,
            "fixedWing": class == crate::cmdinfo::VehicleClass::FixedWing,
            "vtol": class == crate::cmdinfo::VehicleClass::Vtol,
            "rover": class == crate::cmdinfo::VehicleClass::Rover,
            "sub": class == crate::cmdinfo::VehicleClass::Sub,
            "apmFirmware": firmware == crate::cmdinfo::Firmware::ArduPilot,
            "px4Firmware": firmware == crate::cmdinfo::Firmware::Px4,
            "firmwareTypeString": match firmware {
                crate::cmdinfo::Firmware::Px4 => "PX4 Pro",
                crate::cmdinfo::Firmware::ArduPilot => "ArduPilot",
                crate::cmdinfo::Firmware::Generic => "Generic",
            },
            "vehicleTypeString": u8::try_from(vehicle_type).map_or("", crate::vehiclefacade::mav_type_text),
        })),
        "plan.missionController.visualItems" => Some(json!({ "kind": "object", "count": visual_spans(&document).len() })),
        "plan.missionController" => {
            let sequence = visual_spans(&document).get(usize::try_from(selected).unwrap_or(0)).map_or(0, |(first, _)| *first);
            let rules = crate::missionkinds::Rules { takeoff_not_required: planning_setting("takeoffItemNotRequired", false), multiple_landings: planning_setting("allowMultipleLandingPatterns", true) };
            let state = crate::missionkinds::insert_state(&document, &visual_spans(&document), sequence, &rules);
            Some(json!({
                "kind": "object",
                "containsItems": !document.items.is_empty(),
                "homePositionSet": state.home_set,
                "currentPlanViewSeqNum": sequence,
                "currentPlanViewVIIndex": selected,
                "onlyInsertTakeoffValid": state.only_takeoff,
                "isInsertTakeoffValid": state.takeoff,
                "isInsertLandValid": state.land,
                "hasLandItem": crate::missionkinds::has_land(&document),
                "isInsertROIValid": state.roi,
                "flyThroughCommandsAllowed": state.fly_through,
                "globalAltitudeFrame": document.global_altitude_mode,
            }))
        }
        _ => None,
    }
}

fn plan_speeds(backend: &dyn Backend, file: &str) {
    let Some(mission) = std::fs::read_to_string(file).ok().and_then(|text| serde_json::from_str::<Value>(&text).ok()).and_then(|plan| plan.get("mission").cloned()) else { return };
    [("cruiseSpeed", "offlineEditingCruiseSpeed"), ("hoverSpeed", "offlineEditingHoverSpeed")]
        .iter()
        .filter_map(|(key, setting)| mission.get(*key).and_then(Value::as_f64).map(|speed| (setting, speed)))
        .for_each(|(setting, speed)| {
            backend.set(&format!("settings.appSettings.{setting}"), &json!({ "value": speed }).to_string());
        });
}

fn plan_for_offline_vehicle(backend: &dyn Backend) {
    if !offline() {
        return;
    }
    let Some((firmware, vehicle)) = held().document.as_ref().map(|d| (d.firmware_type, d.vehicle_type)) else { return };
    let firmware_class = match plandoc::firmware(firmware) {
        crate::cmdinfo::Firmware::Px4 => 12,
        crate::cmdinfo::Firmware::ArduPilot => 3,
        crate::cmdinfo::Firmware::Generic => 0,
    };
    let vehicle_class = match plandoc::vehicle_class(vehicle) {
        crate::cmdinfo::VehicleClass::FixedWing => 1,
        crate::cmdinfo::VehicleClass::MultiRotor => 2,
        crate::cmdinfo::VehicleClass::Vtol => 20,
        crate::cmdinfo::VehicleClass::Sub => 12,
        crate::cmdinfo::VehicleClass::Rover => 10,
        crate::cmdinfo::VehicleClass::Generic => 0,
    };
    backend.set("settings.appSettings.offlineEditingFirmwareClass", &json!({ "value": firmware_class }).to_string());
    backend.set("settings.appSettings.offlineEditingVehicleClass", &json!({ "value": vehicle_class }).to_string());
}

fn select(args: &str) -> Value {
    let sequence = serde_json::from_str::<Value>(args).ok().and_then(|v| v.get(0).and_then(Value::as_i64));
    let picked = {
        let mut state = held();
        let found = state.document.as_ref().zip(sequence).and_then(|(d, seq)| visual_index_of_sequence(d, seq));
        if let Some(index) = found {
            state.selected = index;
        }
        found
    };
    match picked {
        Some(index) => {
            changed();
            json!({ "ok": true, "selected": index })
        }
        None => refused("There is no item at that sequence number."),
    }
}

fn setting_number(backend: &dyn Backend, name: &str) -> Option<f64> {
    crate::read::value_number(&backend.get(&format!("settings.appSettings.{name}.rawValue")))
}

fn fresh_document(backend: &dyn Backend) -> Document {
    let offline = empty_document();
    Document {
        firmware_type: setting_number(backend, "offlineEditingFirmwareClass").map_or(offline.firmware_type, |v| v as i64),
        vehicle_type: setting_number(backend, "offlineEditingVehicleClass").map_or(offline.vehicle_type, |v| v as i64),
        cruise_speed: setting_number(backend, "offlineEditingCruiseSpeed").unwrap_or(offline.cruise_speed),
        hover_speed: setting_number(backend, "offlineEditingHoverSpeed").unwrap_or(offline.hover_speed),
        ..offline
    }
}

fn step(undoing: bool) -> Value {
    let word = if undoing { "undo" } else { "redo" };
    if !undo_tracking() {
        return json!({ "ok": false, "reason": format!("This plan is not recording edits, so there is nothing to {word}."), "refusal": "notTracking" });
    }
    {
        let mut state = held();
        let taken = match undoing {
            true => state.undo.pop(),
            false => state.redo.pop(),
        };
        let Some(restored) = taken else {
            return json!({ "ok": false, "reason": format!("Nothing to {word}."), "refusal": "nothingTo" });
        };
        let current = state.document.replace(restored);
        match undoing {
            true => state.redo.extend(current),
            false => state.undo.extend(current),
        }
        state.last_change_ms = 0;
        state.dirty = state.document.as_ref().map(plandoc::save) != state.clean.as_ref().map(plandoc::save);
    }
    changed();
    json!({ "ok": true, "reason": null, "refusal": null })
}

pub fn history() -> Option<(bool, bool)> {
    enabled().then(|| {
        let state = held();
        (!state.undo.is_empty(), !state.redo.is_empty())
    })
}

fn clear(backend: &dyn Backend) -> Value {
    let fresh = held().document.is_none().then(|| fresh_document(backend));
    {
        let mut state = held();
        let template = state.document.clone().or(fresh).unwrap_or_else(empty_document);
        let before = state.document.clone();
        state.document = Some(Document { home: None, items: Vec::new(), settings_sections: Vec::new(), ..template });
        remember(&mut state, before, crate::hub::now_ms());
        state.selected = 0;
        settle_clean(&mut state);
        state.file = None;
    }
    changed();
    json!({ "ok": true })
}

fn insert_kind(backend: &dyn Backend, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let kind = given.get(0).and_then(Value::as_str).unwrap_or("");
    let rest = json!([given.get(1), given.get(2), given.get(3)]).to_string();
    let clicked = given.get(1).and_then(Value::as_f64).zip(given.get(2).and_then(Value::as_f64));
    let placed_home = clicked.is_some_and(|(latitude, longitude)| {
        let mut state = held();
        let homeless = state.document.as_ref().is_some_and(|d| d.home.is_none());
        state.document = state.document.take().map(|d| Document { home: d.home.or(Some([latitude, longitude, 0.0])), ..d });
        homeless
    });
    if placed_home {
        settle_home_on_terrain(None);
    }
    let answered = match kind {
        "waypoint" => insert_at(backend, &rest, false),
        "land" => {
            let class = held().document.as_ref().map(|d| plandoc::vehicle_class(d.vehicle_type));
            match class {
                Some(crate::cmdinfo::VehicleClass::FixedWing | crate::cmdinfo::VehicleClass::Vtol) => insert_landing(backend, &rest),
                _ => insert_at(backend, &rest, true),
            }
        }
        "takeoff" => insert_takeoff(backend, &json!([given.get(3)]).to_string()),
        "survey" => insert_scan(backend, &rest, false),
        "corridor" => insert_scan(backend, &rest, true),
        "structure" => insert_structure(backend, &rest),
        "roi" => insert_roi(backend, &rest),
        other => return refused(format!("The core plan cannot insert a {other} yet.")),
    };
    match answered.get("ok").and_then(Value::as_bool) {
        Some(true) => {
            let wanted = given.get(3).and_then(Value::as_i64).unwrap_or(-1);
            let placed = {
                let mut state = held();
                let count = state.document.as_ref().map_or(0, |d| d.items.len() as i64);
                let placed = if (1..=count).contains(&wanted) { wanted } else { count };
                state.selected = placed;
                placed
            };
            changed();
            remember_item(backend, placed);
            json!({ "ok": true, "inserted": kind, "index": placed })
        }
        _ => answered,
    }
}

fn item_write(backend: &dyn Backend, path: &str, value: &str) -> Option<Value> {
    let rest = path.strip_prefix("plan.missionController.visualItems.")?;
    let (index, property) = rest.split_once('.')?;
    let index: usize = index.parse().ok()?;
    let given = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").cloned());
    let number = given.as_ref().and_then(Value::as_f64);
    let answer = |edited: Result<Document, String>| match edited {
        Ok(changed) => edit(|_| Ok(changed)),
        Err(reason) => refused(reason),
    };
    let current = held().document.clone()?;
    if let Some(plandoc::Item::Complex { kind, json: pattern, item_count }) = index.checked_sub(1).and_then(|i| current.items.get(i)).filter(|item| matches!(item, plandoc::Item::Complex { kind, .. } if crate::landingpattern::is_landing(kind))) {
        let at = index - 1;
        let (vertical, horizontal) = (crate::read::Unit::vertical(backend), crate::read::Unit::horizontal(backend));
        let raw = given.as_ref().map(|value| crate::landingpattern::raw(pattern, property, value, &crate::surveydoc::Units { vertical: &vertical, horizontal: &horizontal }));
        let edited = raw.as_ref().and_then(|value| crate::landingpattern::moved(pattern, property, value).or_else(|| crate::landingpattern::edit(pattern, property, value)));
        let Some(edited) = edited else {
            return Some(refused(format!("The landing pattern has no field {property} the core edits.")));
        };
        let count = plandoc::complex_count(kind, &edited).unwrap_or(*item_count);
        let item = plandoc::Item::Complex { kind: kind.clone(), json: edited, item_count: count };
        return Some(edit(|doc| Ok(Document { items: doc.items.iter().enumerate().map(|(k, it)| if k == at { item.clone() } else { it.clone() }).collect(), ..doc.clone() })));
    }
    if let Some(plandoc::Item::Complex { kind, json: survey, .. }) = index.checked_sub(1).and_then(|i| current.items.get(i)) {
        if kind == "survey" || kind == "CorridorScan" || kind == "StructureScan" {
            let Some(value) = given.clone() else { return Some(refused("That field needs a value.")) };
            let at = index - 1;
            let (vertical, horizontal) = (crate::read::Unit::vertical(backend), crate::read::Unit::horizontal(backend));
            return Some(match crate::surveydoc::set(survey, property, &value, &crate::surveydoc::Units { vertical: &vertical, horizontal: &horizontal }) {
                Some(edited) => {
                    let item_count = plandoc::complex_count(kind, &edited).unwrap_or(0);
                    let item = plandoc::Item::Complex { kind: kind.clone(), json: edited, item_count };
                    let answered = edit(|doc| Ok(Document { items: doc.items.iter().enumerate().map(|(k, it)| if k == at { item.clone() } else { it.clone() }).collect(), ..doc.clone() }));
                    remember_item(backend, index as i64);
                    answered
                }
                None => refused(format!("The survey has no field {property}.")),
            });
        }
    }
    let field = |group: &str| {
        let at: usize = property.strip_prefix(group)?.parse().ok()?;
        let plandoc::Item::Simple(s) = current.items.get(index.checked_sub(1)?)? else { return None };
        let commands = crate::cmdinfo::tree(plandoc::firmware(current.firmware_type), plandoc::vehicle_class(current.vehicle_type));
        field_params(commands.get(&s.command)?, group == "comboboxFacts.").get(at).map(|(param, _)| usize::from(*param))
    };
    let unknown = || Err(format!("Item {index} has no such field."));
    Some(match property {
        "altitude" | "command" => return Some(match number {
            Some(n) => item_edit(backend, &json!([index, n]).to_string(), property == "command"),
            None => refused("That field takes a number."),
        }),
        "speedSection.flightSpeed" => answer(number.ok_or_else(|| "A speed is a number.".to_string()).and_then(|v| plandoc::set_speed(&current, index, Some(v)).ok_or_else(|| format!("Item {index} carries no speed.")))),
        "speedSection.specifyFlightSpeed" => {
            let on = given.as_ref().and_then(Value::as_bool).unwrap_or(false);
            let keep = plandoc::specified_speed(&current, index);
            let setting = |name: &str, default: f64| crate::read::value_number(&backend.get(&format!("settings.appSettings.{name}.rawValue"))).unwrap_or(default);
            let default = match plandoc::vehicle_class(current.vehicle_type) {
                crate::cmdinfo::VehicleClass::MultiRotor => setting("offlineEditingHoverSpeed", 5.0),
                _ => setting("offlineEditingCruiseSpeed", 15.0),
            };
            let speed = on.then(|| keep.unwrap_or(default));
            answer(plandoc::set_speed(&current, index, speed).ok_or_else(|| format!("Item {index} carries no speed.")))
        }
        _ => match (field("textFieldFacts.").or_else(|| field("comboboxFacts.")), number) {
            (Some(param), Some(v)) => answer(plandoc::set_param(&current, index, param, v).ok_or_else(|| format!("Item {index} has no such field."))),
            (Some(_), None) => refused("That field takes a number."),
            (None, _) => answer(unknown()),
        },
    })
}

fn point_of(value: Option<&Value>) -> Option<(f64, f64)> {
    let v = value?;
    let lat = v.get("latitude")?.as_f64().filter(|l| l.is_finite() && (-90.0..=90.0).contains(l))?;
    let lon = v.get("longitude")?.as_f64().filter(|l| l.is_finite() && (-180.0..=180.0).contains(l))?;
    Some((lat, lon))
}

fn fence_edit(change: impl FnOnce(&Value, &Value) -> Option<(Value, Value)>, refusal: &str) -> Value {
    let refusal = refusal.to_string();
    edit(move |doc| change(&doc.fence, &doc.rally).map(|(fence, rally)| Document { fence, rally, ..doc.clone() }).ok_or(refusal))
}

fn indexed(path: &str, prefix: &str) -> Option<(usize, String)> {
    let (index, member) = path.strip_prefix(prefix)?.split_once('.')?;
    Some((index.parse().ok()?, member.to_string()))
}

fn fence_invoke(backend: &dyn Backend, path: &str, args: &str) -> Option<Value> {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let first_index = || given.get(0).and_then(Value::as_u64).map(|i| i as usize);
    let window = || point_of(given.get(0)).zip(point_of(given.get(1))).filter(|((north, west), (south, east))| north > south && east != west);
    Some(match path {
        "plan.geoFenceController.addInclusionPolygon" => match window() {
            Some((tl, br)) => fence_edit(|f, r| Some((crate::fencedoc::add_polygon(f, tl, br), r.clone())), ""),
            None => refused("A new fence needs the map window's top-left and bottom-right corners."),
        },
        "plan.geoFenceController.addInclusionCircle" => match window() {
            Some((tl, br)) => fence_edit(|f, r| Some((crate::fencedoc::add_circle(f, tl, br), r.clone())), ""),
            None => refused("A new fence needs the map window's top-left and bottom-right corners."),
        },
        "plan.geoFenceController.deletePolygon" | "plan.geoFenceController.deleteCircle" => {
            let key = if path.ends_with("Polygon") { "polygons" } else { "circles" };
            fence_edit(|f, r| Some((crate::fencedoc::delete(f, key, first_index()?)?, r.clone())), "There is no fence shape at that position.")
        }
        "plan.rallyPointController.addPoint" => match point_of(given.get(0)) {
            Some(at) => {
                let fixed_wing = held().document.as_ref().is_some_and(|d| plandoc::vehicle_class(d.vehicle_type) == crate::cmdinfo::VehicleClass::FixedWing);
                let altitude = crate::read::value_number(&backend.get(&format!("{DEFAULT_ALTITUDE}.rawValue"))).unwrap_or(0.0);
                fence_edit(|f, r| Some((f.clone(), crate::fencedoc::add_rally(r, at, fixed_wing, altitude))), "")
            }
            None => refused("A rally point needs a latitude and a longitude."),
        },
        "plan.rallyPointController.removePoint" => {
            let index = given.get(0).and_then(Value::as_str).and_then(|r| r.strip_prefix("@plan.rallyPointController.points.")?.parse::<usize>().ok());
            fence_edit(|f, r| Some((f.clone(), crate::fencedoc::remove_rally(r, index?)?)), "There is no rally point at that position.")
        }
        _ => {
            let (polygon, member) = indexed(path, "plan.geoFenceController.polygons.")?;
            let vertex = first_index();
            match member.as_str() {
                "adjustVertex" => {
                    let at = point_of(given.get(1));
                    fence_edit(|f, r| Some((crate::fencedoc::move_vertex(f, polygon, vertex?, at?)?, r.clone())), "That vertex cannot be moved there.")
                }
                "removeVertex" => fence_edit(|f, r| Some((crate::fencedoc::remove_vertex(f, polygon, vertex?)?, r.clone())), "A fence polygon keeps at least three vertices."),
                _ => return None,
            }
        }
    })
}

fn fence_set(backend: &dyn Backend, path: &str, value: &str) -> Option<Value> {
    let given = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").cloned()).unwrap_or(Value::Null);
    let altitude = given.get("altitude").and_then(Value::as_f64).filter(|a| a.is_finite());
    Some(match path {
        "plan.geoFenceController.breachReturnPoint" => match point_of(Some(&given)) {
            Some(at) => {
                let default = crate::read::value_number(&backend.get(&format!("{DEFAULT_ALTITUDE}.rawValue")));
                fence_edit(
                    |f, r| {
                        let kept = f.get("breachReturn").and_then(|b| b.get(2)).and_then(Value::as_f64).or(default);
                        Some((crate::fencedoc::set_breach_return(f, at, kept), r.clone()))
                    },
                    "",
                )
            }
            None if given.is_null() => fence_edit(|f, r| Some((crate::fencedoc::clear_breach_return(f), r.clone())), ""),
            None => refused("A breach return point needs a latitude and a longitude."),
        },
        "plan.geoFenceController.breachReturnAltitude" => {
            let vertical = crate::read::Unit::vertical(backend);
            fence_edit(|f, r| Some((crate::fencedoc::set_breach_altitude(f, vertical.meters(given.as_f64()?))?, r.clone())), "Set a breach return point before its altitude.")
        }
        _ => {
            if let Some((index, member)) = indexed(path, "plan.geoFenceController.polygons.") {
                return (member == "inclusion").then(|| fence_edit(|f, r| Some((crate::fencedoc::set_inclusion(f, "polygons", index, given.as_bool()?)?, r.clone())), "A polygon is an inclusion (true) or an exclusion (false)."));
            }
            if let Some((index, member)) = indexed(path, "plan.geoFenceController.circles.") {
                return Some(match member.as_str() {
                    "inclusion" => fence_edit(|f, r| Some((crate::fencedoc::set_inclusion(f, "circles", index, given.as_bool()?)?, r.clone())), "A circle is an inclusion (true) or an exclusion (false)."),
                    "center" => fence_edit(|f, r| Some((crate::fencedoc::set_circle(f, index, Some(point_of(Some(&given))?), None)?, r.clone())), "A circle's centre needs a latitude and a longitude."),
                    "radius" => fence_edit(|f, r| Some((crate::fencedoc::set_circle(f, index, None, Some(given.as_f64().filter(|v| *v >= 0.1)?))?, r.clone())), "A circle's radius is at least 0.1 m."),
                    _ => return None,
                });
            }
            let (index, member) = indexed(path, "plan.rallyPointController.points.")?;
            match member.as_str() {
                "coordinate" => fence_edit(|f, r| Some((f.clone(), crate::fencedoc::move_rally(r, index, point_of(Some(&given))?, altitude)?)), "A rally point needs a latitude and a longitude."),
                "textFieldFacts.2" => fence_edit(
                    |f, r| {
                        let point = r["points"].get(index)?;
                        let at = (point.get(0)?.as_f64()?, point.get(1)?.as_f64()?);
                        Some((f.clone(), crate::fencedoc::move_rally(r, index, at, given.as_f64())?))
                    },
                    "A rally point's altitude is a number.",
                ),
                _ => return None,
            }
        }
    })
}

enum Shape {
    Survey(usize),
    Corridor(usize),
    Fence(usize),
}

fn shape_of(path: &str) -> Option<Shape> {
    if let Some(index) = path.strip_prefix("plan.missionController.visualItems.").and_then(|r| r.strip_suffix(".surveyAreaPolygon").or_else(|| r.strip_suffix(".structurePolygon"))) {
        return index.parse().ok().map(Shape::Survey);
    }
    if let Some(index) = path.strip_prefix("plan.missionController.visualItems.").and_then(|r| r.strip_suffix(".corridorPolyline")) {
        return index.parse().ok().map(Shape::Corridor);
    }
    path.strip_prefix("plan.geoFenceController.polygons.").and_then(|r| r.parse().ok()).map(Shape::Fence)
}

fn vertex_edit(vertices: &[Value], member: &str, given: &Value, ring: bool) -> Option<Vec<Value>> {
    let at = |i: usize| Some((vertices.get(i)?.get(0)?.as_f64()?, vertices.get(i)?.get(1)?.as_f64()?));
    let index = || given.get(0).and_then(Value::as_u64).map(|i| i as usize);
    let spelled = |(lat, lon): (f64, f64)| json!([lat, lon]);
    match member {
        "appendVertex" => Some(vertices.iter().cloned().chain(std::iter::once(spelled(point_of(given.get(0))?))).collect()),
        "adjustVertex" => {
            let (i, to) = (index()?, point_of(given.get(1))?);
            (i < vertices.len()).then(|| vertices.iter().enumerate().map(|(k, v)| if k == i { spelled(to) } else { v.clone() }).collect())
        }
        "removeVertex" => {
            let i = index()?;
            (i < vertices.len() && vertices.len() > if ring { 3 } else { 2 }).then(|| vertices.iter().enumerate().filter(|(k, _)| *k != i).map(|(_, v)| v.clone()).collect())
        }
        "splitPolygonSegment" | "splitSegment" => {
            let i = index()?;
            (ring || i + 1 < vertices.len()).then_some(())?;
            let next = if i + 1 >= vertices.len() { 0 } else { i + 1 };
            let (from, to) = (at(i)?, at(next)?);
            let middle = crate::surveygrid::at_distance_and_azimuth(from, crate::surveygrid::distance_between(from, to) / 2.0, crate::surveygrid::azimuth_to(from, to));
            let place = if next == 0 { vertices.len() } else { next };
            Some(vertices[..place].iter().cloned().chain(std::iter::once(spelled(middle))).chain(vertices[place..].iter().cloned()).collect())
        }
        _ => None,
    }
}

fn with_polygon(owner: &Value, key: &str, vertices: Vec<Value>) -> Value {
    let mut changed = owner.clone();
    changed[key] = Value::Array(vertices);
    changed
}

fn shape_invoke(path: &str, args: &str) -> Option<Value> {
    let (owner, member) = path.rsplit_once('.')?;
    let shape = shape_of(owner)?;
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    if let Some((_, ring)) = crate::itemshape::split_target(path) {
        let count = shape_vertices(owner).map_or(0, |v| v.len() as i64);
        if let Some((token, reason)) = crate::itemshape::split_refusal(count, ring, given.get(0).and_then(Value::as_i64)) {
            return Some(json!({ "ok": false, "refusal": token, "reason": reason }));
        }
    }
    let refusal = "That vertex edit does not fit this shape.";
    Some(match shape {
        Shape::Fence(index) => fence_edit(
            |f, r| {
                let polygons = f.get("polygons").and_then(Value::as_array)?;
                let polygon = polygons.get(index)?;
                let edited = with_polygon(polygon, "polygon", vertex_edit(polygon.get("polygon")?.as_array()?, member, &given, true)?);
                let replaced: Vec<Value> = polygons.iter().enumerate().map(|(k, p)| if k == index { edited.clone() } else { p.clone() }).collect();
                Some((with_polygon(f, "polygons", replaced), r.clone()))
            },
            refusal,
        ),
        Shape::Survey(index) | Shape::Corridor(index) => edit(|doc| {
            let (key, ring) = match shape_of(owner) {
                Some(Shape::Corridor(_)) => ("polyline", false),
                _ => ("polygon", true),
            };
            let at = index.checked_sub(1).filter(|i| *i < doc.items.len()).ok_or(refusal)?;
            let plandoc::Item::Complex { kind, json, .. } = &doc.items[at] else { return Err(refusal.to_string()) };
            let vertices = json.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
            let edited = crate::surveydoc::regenerate_item(&with_polygon(json, key, vertex_edit(&vertices, member, &given, ring).ok_or(refusal)?));
            let item_count = plandoc::complex_count(kind, &edited).unwrap_or(0);
            let item = plandoc::Item::Complex { kind: kind.clone(), json: edited, item_count };
            Ok(Document { items: doc.items.iter().enumerate().map(|(k, it)| if k == at { item.clone() } else { it.clone() }).collect(), ..doc.clone() })
        }),
    })
}

pub fn shape_vertices(path: &str) -> Option<Vec<(f64, f64)>> {
    if !enabled() {
        return None;
    }
    let document = held().document.clone()?;
    let vertices = match shape_of(path)? {
        Shape::Survey(index) => match document.items.get(index.checked_sub(1)?)? {
            plandoc::Item::Complex { json, .. } => json.get("polygon")?.as_array()?.clone(),
            plandoc::Item::Simple(_) => return None,
        },
        Shape::Corridor(index) => match document.items.get(index.checked_sub(1)?)? {
            plandoc::Item::Complex { json, .. } => json.get("polyline")?.as_array()?.clone(),
            plandoc::Item::Simple(_) => return None,
        },
        Shape::Fence(index) => document.fence.get("polygons")?.get(index)?.get("polygon")?.as_array()?.clone(),
    };
    Some(vertices.iter().filter_map(|v| Some((v.get(0)?.as_f64()?, v.get(1)?.as_f64()?))).collect())
}

pub fn drawing() -> bool {
    held().document.as_ref().is_some_and(|d| {
        d.items.iter().any(|item| {
            matches!(item, plandoc::Item::Complex { json, .. } if json.get("polygon").and_then(Value::as_array).is_some_and(|p| p.len() < 3) || json.get(crate::landingpattern::WIZARD).and_then(Value::as_bool) == Some(true))
        })
    })
}

pub fn route_invoke(backend: &dyn Backend, path: &str, args: &str) -> Option<Value> {
    if let Some(answer) = shape_invoke(path, args) {
        return Some(answer);
    }
    if let Some(answer) = fence_invoke(backend, path, args) {
        return Some(answer);
    }
    let current = || held().file.clone();
    Some(match path {
        "plan.loadFromFile" => first_text(args).map_or_else(|| refused("Open needs the path of a .plan file."), |file| {
            let opened = open(&file);
            plan_for_offline_vehicle(backend);
            plan_speeds(backend, &file);
            if let Some(document) = held().document.clone() {
                remember_patterns(backend, document.vehicle_type, document.items.iter());
            }
            opened
        }),
        "plan.saveToFile" => first_text(args).map_or_else(|| refused("Save needs a path to write the plan to."), |file| save(&file)),
        "plan.saveToKml" => first_text(args).map_or_else(|| refused("Export needs a path to write the KML to."), |file| save_kml(&file)),
        "plan.saveToCurrent" => current().map_or_else(|| refused("This plan has not been saved to a file yet."), |file| save(&file)),
        "plan.sendToVehicle" if !carried() => send_through_host(backend),
        "plan.loadFromVehicle" if !carried() => fetch_through_host(backend),
        "plan.sendToVehicle" => send(),
        "plan.loadFromVehicle" => fetch(),
        "plan.removeAll" => clear(backend),
        "mission.insert" => insert_kind(backend, args),
        "mission.remove" | "plan.missionController.removeVisualItem" => remove(args),
        "plan.missionController.setCurrentPlanViewSeqNum" => select(args),
        "plan.undo" => step(true),
        "plan.redo" => step(false),
        _ => return None,
    })
}

pub fn route_set(backend: &dyn Backend, path: &str, value: &str) -> Option<Value> {
    fence_set(backend, path, value).or_else(|| item_write(backend, path, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by_value(value: Value) -> Value {
        match value {
            Value::Number(n) => json!(n.as_f64()),
            Value::Array(items) => Value::Array(items.into_iter().map(by_value).collect()),
            Value::Object(fields) => Value::Object(fields.into_iter().map(|(k, v)| (k, by_value(v))).collect()),
            other => other,
        }
    }

    #[test]
    fn a_new_default_altitude_lands_on_every_item_like_apply_new_altitude() {
        let doc = plandoc::load(include_str!("../tests/fixtures/survey-upload.plan")).unwrap();
        let survey = doc.items.iter().find(|i| matches!(i, plandoc::Item::Complex { kind, .. } if kind == "survey")).unwrap();
        let plandoc::Item::Complex { json, .. } = with_new_altitude(survey, 77.0) else { panic!("a survey stays a survey") };
        assert_eq!(json["TransectStyleComplexItem"]["CameraCalc"]["DistanceToSurface"], 77.0);
        assert_eq!(json["TransectStyleComplexItem"]["CameraCalc"]["ValueSetIsDistance"], true, "TransectStyleComplexItem::applyNewAltitude switches the camera to distance first");
        assert!(json["TransectStyleComplexItem"]["Items"].as_array().unwrap().iter().filter(|i| i["command"] == 16).all(|i| i["params"][6] == 77.0), "the transects are rebuilt at the new height");
        let simple = |command: i64| plandoc::Item::Simple(plandoc::Simple { command, frame: 3, params: [Some(0.0); 7], auto_continue: true, altitude: Some(plandoc::Altitude { mode: 1, altitude: 30.0, amsl_above_terrain: None }), sections: Vec::new() });
        assert!(matches!(with_new_altitude(&simple(16), 77.0), plandoc::Item::Simple(s) if s.params[6] == Some(77.0) && s.altitude.as_ref().is_some_and(|a| a.altitude == 77.0)));
        assert!(matches!(with_new_altitude(&simple(CMD_NAV_LAND), 77.0), plandoc::Item::Simple(s) if s.params[6] == Some(0.0)), "a land item is left alone");
        let unplaced = plandoc::Item::Simple(plandoc::Simple { altitude: None, ..match simple(178) { plandoc::Item::Simple(s) => s, _ => unreachable!() } });
        assert_eq!(with_new_altitude(&unplaced, 77.0), unplaced, "a command without an altitude keeps its params");
    }

    #[test]
    fn the_editor_fields_of_each_command_are_the_ones_qt_builds() {
        let doc = plandoc::load(include_str!("../tests/fixtures/commands.plan")).unwrap();
        let qt: Vec<Value> = serde_json::from_str(include_str!("../tests/fixtures/itemfacts-commands-by-qt.json")).unwrap();
        qt.iter().enumerate().for_each(|(index, expected)| {
            let metres = crate::read::Unit { name: "m".to_string(), factor: 1.0 };
            let mine = by_value(document_facts(&doc, index, 5.0, 15.0, (&metres, &metres)));
            let expected = by_value(expected.clone());
            let differing: Vec<String> = expected.as_object().unwrap().iter().filter(|(k, v)| mine.get(k.as_str()) != Some(v)).map(|(k, v)| format!("{k}:\n  core {}\n  qt   {v}", mine.get(k.as_str()).unwrap_or(&Value::Null))).collect();
            assert!(differing.is_empty(), "item {index}: {}", differing.join("\n"));
        });
    }

    #[test]
    fn an_item_reads_edited_until_the_plan_it_is_in_is_saved() {
        let saved = plandoc::load(include_str!("../../test/MissionManager/SectionTest.plan")).unwrap();
        let moved = plandoc::set_altitude(&saved, 2, 33.0).unwrap();
        let rows = json!({ "items": std::iter::repeat_n(json!({}), moved.items.len() + 1).collect::<Vec<_>>() });
        let marked = marked_edited(rows.clone(), &moved, Some(&saved));
        let edited: Vec<bool> = marked["items"].as_array().unwrap().iter().skip(1).map(|r| r["edited"].as_bool().unwrap()).collect();
        assert_eq!(edited.iter().filter(|e| **e).count(), 1, "only the item whose altitude changed: {edited:?}");
        assert!(edited[1]);
        let never_saved = marked_edited(rows, &moved, None);
        assert!(never_saved["items"].as_array().unwrap().iter().skip(1).all(|r| r["edited"] == true), "a plan that was never saved or loaded is all new");
    }

    #[test]
    fn a_polyline_keeps_two_vertices_and_splits_only_its_real_segments() {
        let line = vec![json!([0.0, 0.0]), json!([0.0, 1.0]), json!([0.0, 2.0])];
        assert_eq!(vertex_edit(&line, "splitSegment", &json!([1]), false).map(|v| v.len()), Some(4));
        assert!(vertex_edit(&line, "splitSegment", &json!([2]), false).is_none(), "the last vertex starts no segment on a line");
        assert_eq!(vertex_edit(&line, "splitPolygonSegment", &json!([2]), true).map(|v| v.len()), Some(4), "a ring closes back to its first vertex");
        let two = vertex_edit(&line, "removeVertex", &json!([0]), false).unwrap();
        assert!(vertex_edit(&two, "removeVertex", &json!([0]), false).is_none());
    }

    #[test]
    fn edits_in_one_burst_undo_together_and_a_pause_starts_a_new_step() {
        let doc = |n: usize| Document { cruise_speed: n as f64, ..empty_document() };
        let mut state = Held { document: Some(doc(0)), ..Held::default() };
        let apply = |state: &mut Held, n: usize, at: u64| {
            let previous = state.document.replace(doc(n));
            remember(state, previous, at);
        };
        apply(&mut state, 1, 10_000);
        apply(&mut state, 2, 10_100);
        assert_eq!(state.undo, vec![doc(0)], "a drag's many writes are one step back to where it started");
        apply(&mut state, 3, 11_000);
        assert_eq!(state.undo, vec![doc(0), doc(2)]);
        (4..4 + UNDO_DEPTH + 5).for_each(|n| apply(&mut state, n, 20_000 + n as u64 * 1_000));
        assert_eq!(state.undo.len(), UNDO_DEPTH, "the oldest steps fall off as Qt's do");
    }

    #[test]
    fn a_sequence_number_selects_the_row_that_holds_it_as_qt_selects_it() {
        let doc = plandoc::load(include_str!("../../test/MissionManager/SectionTest.plan")).unwrap();
        let rows: Vec<Option<i64>> = (0..8).map(|seq| visual_index_of_sequence(&doc, seq)).collect();
        assert_eq!(rows, vec![Some(0), Some(1), Some(2), Some(3), Some(3), Some(4), None, None], "sequence 4 is the mount control folded into row 3, so selecting it selects row 3");
    }
}

pub struct PlanState {
    pub syncing: bool,
    pub dirty: bool,
    pub contains_items: bool,
    pub has_mission_items: bool,
    pub file: String,
    pub global_mode: i64,
}

fn has_entries(section: &Value, key: &str) -> bool {
    section.get(key).and_then(Value::as_array).is_some_and(|list| !list.is_empty())
}

pub fn plan_state() -> Option<PlanState> {
    if !enabled() {
        return None;
    }
    let syncing = crate::hub::lock().active().is_some_and(|v| v.mission_snapshot()["mission"]["inProgress"].as_bool() == Some(true));
    let state = held();
    let document = state.document.clone().unwrap_or_else(empty_document);
    let has_mission_items = !document.items.is_empty();
    Some(PlanState {
        syncing,
        dirty: state.dirty,
        contains_items: has_mission_items || has_entries(&document.fence, "polygons") || has_entries(&document.fence, "circles") || has_entries(&document.rally, "points"),
        has_mission_items,
        file: state.file.clone().unwrap_or_default(),
        global_mode: document.global_altitude_mode,
    })
}

pub fn current_document() -> Document {
    held().document.clone().unwrap_or_else(empty_document)
}

pub fn summary_fields(backend: &dyn Backend) -> Option<Value> {
    if !enabled() {
        return None;
    }
    let document = held().document.clone().unwrap_or_else(empty_document);
    let speed = |name: &str| crate::read::value_number(&backend.get(&format!("settings.appSettings.{name}.rawValue")));
    let speeds = crate::missionitems::Speeds {
        hover: speed("offlineEditingHoverSpeed").unwrap_or(5.0),
        cruise: speed("offlineEditingCruiseSpeed").unwrap_or(15.0),
        ascent: speed("offlineEditingAscentSpeed").unwrap_or(3.0),
        descent: speed("offlineEditingDescentSpeed").unwrap_or(1.0),
    };
    let status = crate::missionitems::flight_status(&document, &speeds);
    Some(json!({
        "kind": "object",
        "containsItems": !document.items.is_empty(),
        "missionTotalDistance": status.as_ref().map(|s| s.total_distance),
        "missionPlannedDistance": status.as_ref().map(|s| s.planned_distance),
        "missionTime": status.as_ref().map(|s| s.total_time),
        "missionHoverDistance": status.as_ref().map(|s| s.hover_distance),
        "missionCruiseDistance": status.as_ref().map(|s| s.cruise_distance),
        "missionMaxTelemetry": status.as_ref().map(|s| s.max_telemetry),
        "minAMSLAltitude": status.as_ref().map(|s| s.min_amsl).filter(|v| v.is_finite()),
        "maxAMSLAltitude": status.as_ref().map(|s| s.max_amsl).filter(|v| v.is_finite()),
    }))
}

const DEFAULT_DECIMAL_PLACES: i64 = 3;
const ITEM_ROOT: &str = "plan.missionController.visualItems";

fn formatted(value: f64, decimals: i64) -> String {
    let places = usize::try_from(decimals).unwrap_or(0);
    format!("{value:.places$}")
}

fn list(param: &Value, key: &str) -> Vec<String> {
    param.get(key).and_then(Value::as_str).map(|joined| joined.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default()
}

fn number_json(value: f64) -> Value {
    match value.fract() == 0.0 && value.abs() < 1e15 {
        true => json!(value as i64),
        false => json!(value),
    }
}

fn param_fact(param: &Value, value: f64) -> Value {
    let label = param.get("label").and_then(Value::as_str).unwrap_or("").to_string();
    let decimals = param.get("decimalPlaces").and_then(Value::as_i64).unwrap_or(DEFAULT_DECIMAL_PLACES);
    let units = param.get("units").and_then(Value::as_str).unwrap_or("").to_string();
    let default = param.get("default").and_then(Value::as_f64);
    let min = param.get("min").and_then(Value::as_f64);
    let max = param.get("max").and_then(Value::as_f64);
    let labels = list(param, "enumStrings");
    let values: Vec<f64> = list(param, "enumValues").iter().filter_map(|v| v.parse().ok()).collect();
    json!({
        "kind": "fact",
        "name": label,
        "shortDescription": label,
        "value": number_json(value),
        "rawValue": value,
        "valueString": formatted(value, decimals),
        "units": units,
        "rawUnits": units,
        "decimalPlaces": decimals,
        "min": min,
        "max": max,
        "minString": min.map(|m| formatted(m, decimals)),
        "maxString": max.map(|m| formatted(m, decimals)),
        "minIsDefaultForType": min.is_none(),
        "maxIsDefaultForType": max.is_none(),
        "defaultValueAvailable": default.is_some(),
        "defaultValue": default.map(number_json),
        "defaultValueString": default.map(|d| formatted(d, decimals)),
        "valueEqualsDefault": default == Some(value),
        "enumStrings": labels,
        "enumValues": values.iter().map(|v| number_json(*v)).collect::<Vec<_>>(),
        "enumIndex": values.iter().position(|v| *v == value).map_or(-1, |i| i as i64),
        "readOnly": false,
    })
}

fn field_params(info: &crate::cmdinfo::Command, combo: bool) -> Vec<(u8, Value)> {
    let flag = |p: &Value, key: &str| p.get(key).and_then(Value::as_bool).unwrap_or(false);
    (1..=7u8)
        .filter(|i| !info.hidden.contains(i))
        .filter_map(|i| info.params.get(&i).map(|p| (i, p.clone())))
        .filter(|(_, p)| match combo {
            true => !list(p, "enumStrings").is_empty(),
            false => list(p, "enumStrings").is_empty() && !flag(p, "nanUnchanged") && !flag(p, "advanced"),
        })
        .collect()
}

fn simple_fields(simple: &plandoc::Simple, commands: &std::collections::BTreeMap<i64, crate::cmdinfo::Command>, item: &str) -> Vec<Value> {
    let Some(info) = commands.get(&simple.command) else { return Vec::new() };
    let text = field_params(info, false);
    let combo = field_params(info, true);
    let text_params = text.iter().map(|(i, p)| (*i, p));
    let combo_params = combo.iter().map(|(i, p)| (*i, p));
    let build = |group: &'static str| {
        move |(at, (i, p)): (usize, (u8, &Value))| {
            let value = simple.params[usize::from(i) - 1].unwrap_or(f64::NAN);
            let suffix = format!("{group}.{at}");
            let mut control = crate::control::decode(&param_fact(p, value), &format!("{item}.{suffix}"));
            if let Value::Object(map) = &mut control {
                map.insert("pathSuffix".to_string(), json!(suffix));
                map.insert("group".to_string(), json!("Settings"));
            }
            control
        }
    };
    text_params.enumerate().map(build("textFieldFacts")).chain(combo_params.enumerate().map(build("comboboxFacts"))).collect()
}

fn speed_section(document: &Document, index: usize, sections: &[plandoc::Simple], available: bool, hover: f64, cruise: f64) -> Value {
    let item = format!("{ITEM_ROOT}.{index}.speedSection");
    let specified = sections.iter().find(|s| s.command == 178).and_then(|s| s.params[1]);
    let default = match plandoc::vehicle_class(document.vehicle_type) {
        crate::cmdinfo::VehicleClass::MultiRotor => hover,
        _ => cruise,
    };
    json!({
        "available": available,
        "specified": specified.is_some(),
        "value": specified.unwrap_or(default),
        "units": "m/s",
        "path": format!("{item}.flightSpeed"),
        "specifyPath": format!("{item}.specifyFlightSpeed"),
    })
}

pub fn item_facts(backend: &dyn Backend, index: usize) -> Value {
    let document = held().document.clone().unwrap_or_else(empty_document);
    let speed = |name: &str, default: f64| crate::read::value_number(&backend.get(&format!("settings.appSettings.{name}.rawValue"))).unwrap_or(default);
    let (vertical, horizontal) = (crate::read::Unit::vertical(backend), crate::read::Unit::horizontal(backend));
    document_facts(&document, index, speed("offlineEditingHoverSpeed", 5.0), speed("offlineEditingCruiseSpeed", 15.0), (&vertical, &horizontal))
}

fn document_facts(document: &Document, index: usize, hover: f64, cruise: f64, units: (&crate::read::Unit, &crate::read::Unit)) -> Value {
    let document = document.clone();
    let commands = crate::cmdinfo::tree(plandoc::firmware(document.firmware_type), plandoc::vehicle_class(document.vehicle_type));
    let item = format!("{ITEM_ROOT}.{index}");
    let base = |simple: bool, fields: Vec<Value>, section: Value, mode: Option<i64>| {
        json!({ "kind": "object", "class": "ItemFacts", "available": true, "index": index, "simple": simple, "fields": fields, "camera": Value::Null, "speedSection": section, "altitudeMode": mode })
    };
    match index.checked_sub(1).map(|i| document.items.get(i)) {
        None => base(false, Vec::new(), speed_section(&document, index, &document.settings_sections, true, hover, cruise), None),
        Some(Some(plandoc::Item::Simple(s))) => base(
            true,
            simple_fields(s, &commands, &item),
            speed_section(&document, index, &s.sections, s.command == 16, hover, cruise),
            Some(s.altitude.as_ref().map_or(crate::altitudemodes::RELATIVE, |a| a.mode)),
        ),
        Some(Some(plandoc::Item::Complex { kind, json: survey, .. })) if kind == "survey" || kind == "CorridorScan" || kind == "StructureScan" => {
            let multirotor = plandoc::vehicle_class(document.vehicle_type) == crate::cmdinfo::VehicleClass::MultiRotor;
            let units = crate::surveydoc::Units { vertical: units.0, horizontal: units.1 };
            json!({ "kind": "object", "class": "ItemFacts", "available": true, "index": index, "simple": false, "fields": crate::surveydoc::fields(survey, &item, multirotor, &units), "camera": crate::surveydoc::camera(survey, &item, &units), "speedSection": Value::Null, "altitudeMode": Value::Null })
        }
        Some(Some(plandoc::Item::Complex { kind, json: pattern, .. })) if crate::landingpattern::is_landing(kind) => {
            let units = crate::surveydoc::Units { vertical: units.0, horizontal: units.1 };
            json!({ "kind": "object", "class": "ItemFacts", "available": true, "index": index, "simple": false, "fields": crate::landingpattern::fields(pattern, &item, &units), "camera": Value::Null, "speedSection": Value::Null, "altitudeMode": Value::Null })
        }
        Some(Some(plandoc::Item::Complex { kind, .. })) => json!({ "kind": "object", "class": "ItemFacts", "available": false, "index": index, "reason": format!("The core cannot edit a {kind} item yet.") }),
        Some(None) => json!({ "kind": "object", "class": "ItemFacts", "available": false, "index": index }),
    }
}

pub fn fence_and_rally() -> Option<(Value, Value)> {
    if !enabled() {
        return None;
    }
    let document = held().document.clone().unwrap_or_else(empty_document);
    Some((document.fence, document.rally))
}

fn speed_in_force(document: &Document, before: usize, hover: f64, cruise: f64) -> f64 {
    let multirotor = plandoc::vehicle_class(document.vehicle_type) == crate::cmdinfo::VehicleClass::MultiRotor;
    let start = if multirotor { hover } else { cruise };
    let changes = std::iter::once(&document.settings_sections).chain(document.items.iter().take(before).filter_map(|item| match item {
        plandoc::Item::Simple(s) => Some(&s.sections),
        plandoc::Item::Complex { .. } => None,
    }));
    changes.fold(start, |speed, sections| sections.iter().find(|s| s.command == 178).and_then(|s| s.params[1]).unwrap_or(speed))
}

const BREACH_RETURN_META: &str = include_str!("../../src/MissionManager/BreachReturn.FactMetaData.json");
const BREACH_RETURN_DEFAULT_ALTITUDE: f64 = 75.0;

pub fn breach_altitude_fact() -> Option<Value> {
    if !enabled() {
        return None;
    }
    let fence = held().document.as_ref().map(|d| d.fence.clone()).unwrap_or(Value::Null);
    let altitude = fence.get("breachReturn").and_then(Value::as_array).and_then(|b| b.get(2)).and_then(Value::as_f64).unwrap_or(BREACH_RETURN_DEFAULT_ALTITUDE);
    let meta = crate::factmeta::from_file(BREACH_RETURN_META).ok()?.remove("Altitude")?;
    let mut fact = crate::settingsstore::fact_json(&meta, &json!(altitude), crate::units::cooking("vertical m"));
    ["defaultValueString", "userVisible", "visible"].iter().for_each(|key| fact[*key] = Value::Null);
    Some(fact)
}

pub fn camera_section(index: usize) -> Option<Value> {
    if !enabled() {
        return None;
    }
    let document = held().document.clone().unwrap_or_else(empty_document);
    Some(match index.checked_sub(1).map(|at| document.items.get(at)) {
        None => plandoc::camera_section(&document.settings_sections),
        Some(Some(plandoc::Item::Simple(simple))) => plandoc::camera_section(&simple.sections),
        _ => json!({ "kind": "null" }),
    })
}

pub fn landing_inputs(index: usize) -> Option<(Value, Value)> {
    if !enabled() {
        return None;
    }
    let item = held().document.clone().zip(index.checked_sub(1)).and_then(|(document, at)| document.items.get(at).cloned());
    Some(match item {
        Some(plandoc::Item::Complex { json: pattern, .. }) => crate::landingpattern::view_inputs(&pattern).unwrap_or((json!({ "kind": "object", "isSimpleItem": false }), json!({ "kind": "null" }))),
        Some(_) => (json!({ "kind": "object", "isSimpleItem": true }), json!({ "kind": "null" })),
        None => (json!({ "kind": "object", "isSimpleItem": false }), json!({ "kind": "null" })),
    })
}

pub fn survey_stats_inputs(backend: &dyn Backend, index: usize) -> Option<(Value, Value)> {
    if !enabled() {
        return None;
    }
    let not_survey = || Some((json!({ "kind": "object", "isSurveyItem": false }), json!({ "kind": "null" })));
    let Some((document, at)) = held().document.clone().zip(index.checked_sub(1)) else { return not_survey() };
    let Some(plandoc::Item::Complex { json: item, .. }) = document.items.get(at) else { return not_survey() };
    let kind = item.get("complexItemType").and_then(Value::as_str).unwrap_or("");
    let setting = |name: &str, default: f64| crate::read::value_number(&backend.get(&format!("settings.appSettings.{name}.rawValue"))).unwrap_or(default);
    let speed = speed_in_force(&document, at, setting("offlineEditingHoverSpeed", 5.0), setting("offlineEditingCruiseSpeed", 15.0));
    let number = |v: &Value, key: &str| v.get(key).and_then(Value::as_f64).unwrap_or(0.0);
    let per_second = |metres: f64| if speed == 0.0 { 0.0 } else { metres / speed };
    let horizontal = crate::read::Unit::horizontal(backend);
    let metres_fact = |property: &str, metres: f64| json!({ "property": property, "value": horizontal.show(metres), "rawValue": metres, "units": horizontal.name });
    let calc_facts = |calc: &Value| json!({ "kind": "object", "facts": [
        metres_fact("adjustedFootprintSide", number(calc, "AdjustedFootprintSide")),
        metres_fact("adjustedFootprintFrontal", number(calc, "AdjustedFootprintFrontal")),
        metres_fact("distanceToSurface", number(calc, "DistanceToSurface")),
        { "property": "minTriggerInterval", "value": number(calc, "MinTriggerInterval"), "rawValue": number(calc, "MinTriggerInterval") },
    ] });
    let stats = |shots: f64, seconds: f64, area: f64, distance: f64| json!({ "kind": "object", "isSurveyItem": kind == "survey", "cameraShots": shots, "timeBetweenShots": seconds, "coveredArea": area, "complexDistance": distance });
    let (stats, facts) = match kind {
        "survey" | "CorridorScan" => {
            let transect = &item["TransectStyleComplexItem"];
            let calc = &transect["CameraCalc"];
            let visual: Vec<(f64, f64)> = transect["VisualTransectPoints"].as_array().map(|p| p.iter().filter_map(|v| Some((v.get(0)?.as_f64()?, v.get(1)?.as_f64()?))).collect()).unwrap_or_default();
            let area = match kind {
                "survey" => crate::mappolygon::area(&crate::surveydoc::polygon(item)),
                _ => crate::mappolygon::area(&crate::corridorscan::corridor_polygon(item).iter().map(|(latitude, longitude, _)| (*latitude, *longitude)).collect::<Vec<_>>()),
            };
            let distance = visual.windows(2).map(|pair| crate::surveygrid::distance_between(pair[0], pair[1])).sum::<f64>();
            (stats(number(transect, "CameraShots"), per_second(number(calc, "AdjustedFootprintFrontal")), area, distance), calc_facts(calc))
        }
        "StructureScan" => {
            let flight = crate::structurescan::saved_flight(item).unwrap_or_default();
            let plan = crate::structurescan::saved_plan(item);
            let shots = crate::structurescan::camera_shots(&flight, plan.adjusted_side, plan.layers) as f64;
            (stats(shots, per_second(plan.adjusted_side), 0.0, crate::structurescan::scan_distance(&flight, &plan)), calc_facts(&item["CameraCalc"]))
        }
        _ => {
            let row = crate::landingpattern::row(item);
            let slope = crate::landingpattern::slope_start(item);
            let distance = row.zip(slope).map_or(0.0, |(row, slope)| crate::surveygrid::distance_between(row.approach, slope) + crate::surveygrid::distance_between(slope, row.land));
            (stats(0.0, 0.0, 0.0, distance), json!({ "kind": "null" }))
        }
    };
    Some((stats, facts))
}
