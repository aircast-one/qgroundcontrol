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
const ACTIONS: &[&str] = &[OPEN, SAVE, SEND, FETCH, STATUS, CORE_INSERT_WAYPOINT, CORE_REMOVE, CORE_INSERT_LAND, CORE_SET_COMMAND, CORE_SET_ALTITUDE, CORE_INSERT_TAKEOFF, CORE_SET_ALTITUDE_MODE, CORE_ITEMS, APPLY_DEFAULT_ALTITUDE, DISMISS_ALTITUDE_PROMPT, LOAD_VEHICLE_PLAN, KEEP_CURRENT_PLAN];
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
    dirty_for_save: bool,
    clean: Option<Document>,
    undo: Vec<Document>,
    redo: Vec<Document>,
    last_change_ms: u64,
    shown_vehicle: Option<u8>,
    vehicle_prompt: Option<bool>,
    breach_altitude: Option<f64>,
    wizard: Option<usize>,
}

pub const LOAD_VEHICLE_PLAN: &str = "core.plan.loadVehiclePlan";
pub const KEEP_CURRENT_PLAN: &str = "core.plan.keepCurrentPlan";
const NO_VEHICLE_SHOWN: u8 = u8::MAX;

pub fn vehicle_change_prompt_for(offline: Option<bool>, dirty: bool) -> Value {
    match offline {
        None => Value::Null,
        Some(offline) => json!({
            "title": if offline { "Plan View - Vehicle Disconnected" } else { "Plan View - Vehicle Changed" },
            "text": if offline {
                "The vehicle associated with the plan in the Plan View is no longer available. What would you like to do with that plan?"
            } else {
                "The plan being worked on in the Plan View is not from the current vehicle. What would you like to do with that plan?"
            },
            "loadText": match (dirty, offline) {
                (false, _) => "Load New Plan From Vehicle",
                (true, true) => "Discard Unsaved Changes",
                (true, false) => "Discard Unsaved Changes, Load New Plan From Vehicle",
            },
            "keepText": if offline { "Keep Current Plan" } else { "Keep Current Plan, Don't Update From Vehicle" },
        }),
    }
}

pub fn vehicle_change_prompt() -> Value {
    let state = held();
    vehicle_change_prompt_for(state.vehicle_prompt, state.dirty_for_save)
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

fn settle_uploaded(state: &mut Held) {
    state.dirty = false;
    state.clean = state.document.clone();
}

fn settle_clean(state: &mut Held) {
    settle_uploaded(state);
    state.dirty_for_save = false;
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
        LOAD_VEHICLE_PLAN => {
            {
                let mut state = held();
                state.shown_vehicle = state.vehicle_prompt.filter(|offline| *offline).map(|_| NO_VEHICLE_SHOWN);
                state.vehicle_prompt = None;
                state.dirty = false;
                state.dirty_for_save = false;
            }
            follow_vehicle();
            changed();
            json!({ "ok": true })
        }
        KEEP_CURRENT_PLAN => {
            held().vehicle_prompt = None;
            changed();
            json!({ "ok": true })
        }
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
                state.document = state.document.take().map(|d| Document { home: d.home.map(|h| [h[0], h[1], ground]), ..d });
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
        let gate = plandoc::firmware(current.firmware_type) == crate::cmdinfo::Firmware::Px4 && crate::settingsstore::raw_setting("settings.planViewSettings.useConditionGate").and_then(|v| v.as_bool()).unwrap_or(false);
        crate::surveyitems::CONDITION_GATE_SUPPORTED.store(gate, std::sync::atomic::Ordering::Relaxed);
        match change(current) {
            Ok(changed) => {
                let count = changed.items.len();
                shift_raw_edits(&current.items, &changed.items);
                let previous = state.document.replace(changed);
                remember(&mut state, previous, crate::hub::now_ms());
                state.dirty = true;
                state.dirty_for_save = true;
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

static MAP_CENTER_HINT: Mutex<Option<(f64, f64)>> = Mutex::new(None);

pub fn set_map_center_hint(latitude: f64, longitude: f64) {
    *MAP_CENTER_HINT.lock().unwrap_or_else(PoisonError::into_inner) = Some((latitude, longitude));
}

fn edit_defaults(backend: &dyn Backend) -> Option<plandoc::EditDefaults> {
    let map_center = *MAP_CENTER_HINT.lock().unwrap_or_else(PoisonError::into_inner);
    let vtol_transition_distance = crate::read::value_number(&backend.get("settings.planViewSettings.vtolTransitionDistance.rawValue")).unwrap_or(plandoc::VTOL_TRANSITION_DISTANCE_DEFAULT);
    crate::read::value_number(&backend.get(&format!("{DEFAULT_ALTITUDE}.rawValue"))).map(|mission_item_altitude| plandoc::EditDefaults { mission_item_altitude, map_center, vtol_transition_distance })
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

fn insert_landing(backend: &dyn Backend, args: &str) -> Value {
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
            relative: true,
            transition_distance: crate::read::value_number(&backend.get("settings.planViewSettings.vtolTransitionDistance.rawValue")),
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
        Ok(plandoc::with_survey_camera(plandoc::insert_complex(doc, "StructureScan", built, (latitude, longitude), index)))
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
        Ok(plandoc::with_survey_camera(plandoc::insert_complex(doc, kind, with_flight_speed(doc, built), (latitude, longitude), index)))
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
    match crate::missionitems::document_view(&document, selected, &crate::read::Unit::vertical(backend), &crate::read::Unit::horizontal(backend), &crate::read::Unit::speed(backend), crate::missionsummary::imperial(backend), rover) {
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
    let answer = edit(|doc| {
        let changed = match command {
            true => plandoc::set_command(doc, index, value as i64, &defaults),
            false => plandoc::set_altitude(doc, index, value),
        };
        changed.ok_or_else(|| format!("Item {index} cannot take that edit."))
    });
    if command && answer["ok"] == true {
        raw_edits().remove(&index);
        changed();
    }
    answer
}

pub fn offline_types_changed() {
    if !offline() {
        return;
    }
    let (firmware_type, vehicle_type) = (offline_type("offlineEditingFirmwareClass"), offline_type("offlineEditingVehicleClass"));
    let moved = {
        let mut state = held();
        let Some(document) = state.document.as_ref() else { return };
        let terrain = plandoc::firmware(firmware_type) != crate::cmdinfo::Firmware::Px4;
        let frame = match document.global_altitude_mode == crate::altitudemodes::TERRAIN_FRAME && !terrain {
            true => crate::altitudemodes::CALC_ABOVE_TERRAIN,
            false => document.global_altitude_mode,
        };
        let next = Document { firmware_type, vehicle_type, global_altitude_mode: frame, ..document.clone() };
        let differs = plandoc::save(&next) != plandoc::save(document);
        if differs {
            state.document = Some(next);
        }
        differs
    };
    if moved {
        changed();
    }
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
    let answer = edit(|doc| plandoc::remove(doc, index).map(|after| plandoc::after_scan_removed(doc, after)).ok_or_else(|| format!("This plan has no item {index} to remove.")));
    if answer["ok"] == true {
        let mut state = held();
        let last = state.document.as_ref().map_or(0, |d| d.items.len());
        state.selected = current_after_remove(index, last);
    }
    answer
}

fn current_after_remove(removed: usize, last_visual: usize) -> i64 {
    removed.min(last_visual) as i64
}

const WAYPOINTS_HEADER: &str = "QGC WPL";

fn plan_text(text: &str) -> Result<String, String> {
    match text.trim_start().starts_with(WAYPOINTS_HEADER) {
        true => {
            let (firmware_type, vehicle_type) = planned_types();
            crate::waypoints::parse(text).map(|file| crate::planfile::write(&crate::planfile::from_waypoints(&file, firmware_type, vehicle_type)))
        }
        false => Ok(text.to_string()),
    }
}

fn open(file: &str) -> Value {
    forget_raw_edits();
    let connected = !offline();
    let loaded = std::fs::read_to_string(file).map_err(|e| format!("Could not read {file}: {e}")).and_then(|text| load_plan(&plan_text(&text)?));
    match loaded {
        Ok(document) => {
            let count = document.items.len();
            {
                let mut state = held();
                let before = state.document.take();
                state.document = Some(document);
                state.wizard = None;
                remember(&mut state, before, crate::hub::now_ms());
                state.selected = 0;
                state.file = Some(file.to_string());
                settle_clean(&mut state);
                state.dirty = connected;
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
    let offline = offline();
    match std::fs::write(file, text) {
        Ok(()) => {
            {
                let mut state = held();
                state.file = Some(file.to_string());
                state.dirty_for_save = false;
                if offline {
                    settle_clean(&mut state);
                }
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

fn launch_at_takeoff(document: &Document, visual_index: usize) -> bool {
    let commands = crate::cmdinfo::tree(plandoc::firmware(document.firmware_type), plandoc::vehicle_class(document.vehicle_type));
    let forward_flight = matches!(plandoc::vehicle_class(document.vehicle_type), crate::cmdinfo::VehicleClass::FixedWing | crate::cmdinfo::VehicleClass::Vtol);
    let takeoff = visual_index.checked_sub(1).and_then(|at| document.items.get(at)).and_then(|item| match item {
        plandoc::Item::Simple(s) if commands.get(&s.command).is_some_and(|c| c.is_takeoff) => Some(s),
        _ => None,
    });
    takeoff.is_some_and(|s| {
        let at_home = match (s.params[4].zip(s.params[5]), document.home) {
            (Some((lat, lon)), Some(home)) => lat == home[0] && lon == home[1],
            _ => true,
        };
        !forward_flight && at_home
    })
}

fn contains_items(document: &Document) -> bool {
    let listed = |section: &Value, key: &str| section.get(key).and_then(Value::as_array).is_some_and(|items| !items.is_empty());
    !document.items.is_empty() || listed(&document.fence, "polygons") || listed(&document.fence, "circles") || listed(&document.rally, "points")
}

fn send_after_mission(id: u8, document: Document) {
    std::thread::spawn(move || {
        let (fence, rally) = crate::hub::lock().vehicle(id).map_or((false, false), crate::hub::Vehicle::plans_supported);
        let wanted: Vec<&str> = [("fence", fence), ("rally", rally)].into_iter().filter(|(_, supported)| *supported).map(|(k, _)| k).collect();
        let previous = std::iter::once("mission").chain(wanted.iter().copied()).collect::<Vec<_>>();
        wanted.iter().zip(previous).for_each(|(kind, before)| {
            if settle(id, before) {
                match crate::hub::lock().mission_request(Some(id), &send_shape(kind, &document), crate::hub::now_ms()) {
                    Ok(outbound) => deliver(outbound),
                    Err(error) => {
                        crate::noticeboard::post(crate::noticeboard::MESSAGE, "", &crate::hub::transfer_failed_text(kind, &error));
                    }
                }
            }
        });
        let _ = wanted.last().is_some_and(|last| settle(id, last));
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
        settle_uploaded(&mut held());
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
            (v.id, v.mission_snapshot(), v.sends_home(), fence, rally, (i64::from(v.autopilot), i64::from(v.vehicle_type)), v.home.map(|(lat, lon, alt)| [lat, lon, alt]))
        });
        (hub.active_id(), ready)
    };
    let (adopted, classes) = {
        let mut state = held();
        if state.fetching {
            return;
        }
        match (active, ready) {
            (None, _) if state.shown_vehicle.is_some() => {
                state.shown_vehicle = None;
                let has_items = state.document.as_ref().is_some_and(contains_items);
                if state.dirty_for_save && has_items {
                    state.vehicle_prompt = Some(true);
                    drop(state);
                    changed();
                    return;
                }
                let before = state.document.clone();
                state.document = state.document.clone().map(|d| Document { home: None, items: Vec::new(), settings_sections: Vec::new(), fence: empty_document().fence, rally: empty_document().rally, ..d });
                state.wizard = None;
                forget_raw_edits();
                remember(&mut state, before, crate::hub::now_ms());
                state.selected = 0;
                settle_clean(&mut state);
                state.file = None;
                (None, None)
            }
            (Some(id), Some(vehicle)) if state.shown_vehicle != Some(id) && vehicle.0 == id => {
                state.shown_vehicle = Some(id);
                let classes = offline_classes(vehicle.5 .0, vehicle.5 .1);
                let has_items = state.document.as_ref().is_some_and(contains_items);
                if state.dirty_for_save && has_items {
                    state.vehicle_prompt = Some(false);
                }
                ((!state.dirty_for_save || !has_items).then_some(vehicle), Some(classes))
            }
            _ => return,
        }
    };
    if let Some((firmware_class, vehicle_class)) = classes {
        crate::settingsstore::set_raw("settings.appSettings.offlineEditingFirmwareClass", &json!(firmware_class));
        crate::settingsstore::set_raw("settings.appSettings.offlineEditingVehicleClass", &json!(vehicle_class));
    }
    if let Some((_, snapshot, sends_home, fence, rally, types, vehicle_home)) = adopted {
        adopt(&snapshot, sends_home, fence, rally, types, vehicle_home);
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
    let adopted = std::fs::read_to_string(&file).map_err(|e| e.to_string()).and_then(|text| load_plan(&text));
    {
        let mut state = held();
        state.fetching = false;
        if let Ok(document) = adopted {
            forget_raw_edits();
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
    let Some(id) = crate::hub::lock().active_id() else {
        return refused("No vehicle is connected through the core.");
    };
    let started = crate::hub::lock().write_mission(Some(id), transfer, crate::hub::now_ms());
    match started {
        Ok(outbound) => {
            deliver(outbound);
            settle_uploaded(&mut held());
            send_after_mission(id, document);
            changed();
            json!({ "ok": true, "items": items.len() })
        }
        Err(reason) => refused(reason),
    }
}

fn transfer_outcome(active: Option<u8>, id: u8, in_progress: bool) -> Option<bool> {
    match (active == Some(id), in_progress) {
        (false, _) => Some(false),
        (true, true) => None,
        (true, false) => Some(true),
    }
}

fn settle(id: u8, kind: &str) -> bool {
    (0..600)
        .find_map(|_| {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let hub = crate::hub::lock();
            transfer_outcome(hub.active_id(), id, hub.vehicle(id).is_some_and(|v| v.mission_snapshot()[kind]["inProgress"].as_bool() == Some(true)))
        })
        .unwrap_or(false)
}

fn load(id: u8, kind: &str) -> bool {
    let started = crate::hub::lock().mission_request(Some(id), &json!({ "action": "load", "plan": kind }), crate::hub::now_ms());
    match started {
        Ok(outbound) => {
            deliver(outbound);
            settle(id, kind)
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

fn load_plan(text: &str) -> Result<Document, String> {
    plandoc::load(text, offline_type("offlineEditingVehicleClass")).map(|document| Document {
        cruise_speed: if document.cruise_speed.is_nan() { remembered_speed("offlineEditingCruiseSpeed", 15.0) } else { document.cruise_speed },
        hover_speed: if document.hover_speed.is_nan() { remembered_speed("offlineEditingHoverSpeed", 5.0) } else { document.hover_speed },
        ..document
    })
}

fn remembered_speed(name: &str, fallback: f64) -> f64 {
    crate::settingsstore::stored_text(name).and_then(|text| text.parse().ok()).unwrap_or(fallback)
}

fn adopt(snapshot: &Value, sends_home: bool, fence_read: bool, rally_read: bool, types: (i64, i64), vehicle_home: Option<[f64; 3]>) {
    forget_raw_edits();
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
    let downloaded_mission = plandoc::from_vehicle(&downloaded, sends_home, &template);
    let mission = Document { home: plandoc::planned_home(&downloaded_mission, vehicle_home), ..downloaded_mission };
    state.wizard = None;
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
    let Some((id, fence, rally, sends_home, types, vehicle_home)) = crate::hub::lock().active().map(|v| {
        let (fence, rally) = v.plans_supported();
        (v.id, fence, rally, v.sends_home(), (i64::from(v.autopilot), i64::from(v.vehicle_type)), v.home.map(|(lat, lon, alt)| [lat, lon, alt]))
    }) else {
        return refused("No vehicle is connected through the core.");
    };
    if crate::hub::lock().active().is_some_and(crate::hub::Vehicle::on_high_latency_link) {
        return refused(crate::hub::HIGH_LATENCY_DOWNLOAD);
    }
    held().fetching = true;
    std::thread::spawn(move || {
        let mission_read = load(id, "mission");
        let fence_read = fence && load(id, "fence");
        let rally_read = rally && load(id, "rally");
        let (still_active, snapshot) = {
            let hub = crate::hub::lock();
            (hub.active_id() == Some(id), hub.vehicle(id).map(crate::hub::Vehicle::mission_snapshot).unwrap_or(Value::Null))
        };
        match mission_read && still_active {
            true => {
                adopt(&snapshot, sends_home, fence_read, rally_read, types, vehicle_home);
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

struct FlownMission {
    items: Vec<Downloaded>,
    sends_home: bool,
    types: (i64, i64),
    current: i64,
    home: Option<[f64; 3]>,
}

fn flown_mission(v: &crate::hub::Vehicle) -> FlownMission {
    FlownMission {
        items: v.mission_items().iter().map(|i| Downloaded { frame: i64::from(i.frame), command: i64::from(i.command), params: i.params, auto_continue: i.auto_continue }).collect(),
        sends_home: v.sends_home(),
        types: (i64::from(v.autopilot), i64::from(v.vehicle_type)),
        current: i64::from(v.current_mission_index()),
        home: v.home.map(|(lat, lon, alt)| [lat, lon, alt]),
    }
}

fn flown_view(backend: &dyn Backend, mission: &FlownMission) -> Result<Value, String> {
    if mission.items.is_empty() {
        return Err("The vehicle holds no mission.".to_string());
    }
    let template = Document { firmware_type: mission.types.0, vehicle_type: mission.types.1, ..empty_document() };
    let downloaded = plandoc::from_vehicle(&mission.items, mission.sends_home, &template);
    let document = Document { home: downloaded.home.or(mission.home), ..downloaded };
    let rover = plandoc::vehicle_class(mission.types.1) == crate::cmdinfo::VehicleClass::Rover;
    let selected = visual_index_of_sequence(&document, mission.current).filter(|index| *index > 0).unwrap_or(-1);
    crate::missionitems::document_view(&document, selected, &crate::read::Unit::vertical(backend), &crate::read::Unit::horizontal(backend), &crate::read::Unit::speed(backend), crate::missionsummary::imperial(backend), rover)
}

pub fn fly_view(backend: &dyn Backend) -> Value {
    let (active, others) = {
        let hub = crate::hub::lock();
        let active_id = hub.active_id();
        let active = hub.active().map(flown_mission);
        let others: Vec<FlownMission> = hub.in_arrival_order().into_iter().filter(|v| Some(v.id) != active_id).map(flown_mission).collect();
        (active, others)
    };
    let other_views: Vec<Value> = others.iter().filter_map(|mission| flown_view(backend, mission).ok()).collect();
    let mut answer = active
        .ok_or_else(|| "The vehicle holds no mission.".to_string())
        .and_then(|mission| flown_view(backend, &mission))
        .unwrap_or_else(|reason| json!({ "kind": "object", "class": "MissionItems", "available": false, "items": [], "selected": -1, "reason": reason }));
    answer["others"] = Value::Array(other_views);
    answer
}

pub fn view(backend: &dyn Backend) -> Value {
    let (document, selected, clean) = {
        let state = held();
        (state.document.clone().unwrap_or_else(empty_document), state.selected, state.clean.clone())
    };
    let rover = crate::read::flag(&crate::read::object(&backend.get_fields("plan.controllerVehicle", "rover")), "rover");
    crate::missionitems::document_view(&document, selected, &crate::read::Unit::vertical(backend), &crate::read::Unit::horizontal(backend), &crate::read::Unit::speed(backend), crate::missionsummary::imperial(backend), rover)
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

fn camera_calc_index(path: &str) -> Option<usize> {
    path.strip_prefix("plan.missionController.visualItems.")?.strip_suffix(".cameraCalc")?.parse().ok()
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
            "homePosition": crate::hub::lock().active().and_then(|v| v.home).map(|(latitude, longitude, _)| json!({ "valid": true, "latitude": latitude, "longitude": longitude })),
        })),
        "plan.missionController.visualItems" => Some(json!({ "kind": "object", "count": visual_spans(&document).len() })),
        _ if camera_calc_index(path).is_some() => camera_calc_index(path)
            .and_then(|index| index.checked_sub(1))
            .and_then(|i| match document.items.get(i) {
                Some(plandoc::Item::Complex { json: survey, .. }) => crate::surveydoc::calc_of(survey).get("DistanceMode").cloned(),
                _ => None,
            })
            .map(|mode| json!({ "kind": "object", "distanceMode": mode })),
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
                "insertBeforeTakeoff": state.before_takeoff,
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

fn offline_classes(firmware: i64, vehicle: i64) -> (i64, i64) {
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
    (firmware_class, vehicle_class)
}

fn plan_for_offline_vehicle(backend: &dyn Backend) {
    if !offline() {
        return;
    }
    let Some((firmware, vehicle)) = held().document.as_ref().map(|d| (d.firmware_type, d.vehicle_type)) else { return };
    let (firmware_class, vehicle_class) = offline_classes(firmware, vehicle);
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
        forget_raw_edits();
        let current = state.document.replace(restored);
        match undoing {
            true => state.redo.extend(current),
            false => state.undo.extend(current),
        }
        state.last_change_ms = 0;
        state.dirty = state.document.as_ref().map(plandoc::save) != state.clean.as_ref().map(plandoc::save);
        state.dirty_for_save = state.dirty;
    }
    changed();
    json!({ "ok": true, "reason": null, "refusal": null })
}

pub fn plan_types() -> Option<(i64, i64)> {
    held().document.as_ref().map(|document| (document.firmware_type, document.vehicle_type))
}

pub fn history() -> Option<(bool, bool)> {
    enabled().then(|| {
        let state = held();
        (!state.undo.is_empty(), !state.redo.is_empty())
    })
}

fn remove_all_from_vehicle(backend: &dyn Backend) -> Value {
    let Some((id, (fence, rally))) = crate::hub::lock().active().map(|v| (v.id, v.plans_supported())) else {
        return refused("No vehicle is connected through the core.");
    };
    let kinds = ["mission"].into_iter().chain(fence.then_some("fence")).chain(rally.then_some("rally"));
    let (sent, refusals): (Vec<&'static str>, Vec<String>) = kinds.fold((Vec::new(), Vec::new()), |(sent, refusals), kind| {
        match crate::hub::lock().mission_request(Some(id), &json!({ "action": "removeAll", "plan": kind }), crate::hub::now_ms()) {
            Ok(outbound) => {
                deliver(outbound);
                ([sent, vec![kind]].concat(), refusals)
            }
            Err(reason) => (sent, [refusals, vec![reason]].concat()),
        }
    });
    let unopened = held().document.is_none();
    let fresh = unopened.then(|| fresh_document(backend));
    {
        let mut state = held();
        state.dirty = false;
        state.file = None;
    }
    changed();
    std::thread::spawn(move || {
        let removed: Vec<&str> = sent.into_iter().filter(|kind| removed_on(id, kind)).collect();
        if !removed.is_empty() {
            clear_kinds(fresh, &removed);
        }
    });
    match refusals.first() {
        None => json!({ "ok": true }),
        Some(reason) => refused(reason.clone()),
    }
}

fn removed_on(id: u8, kind: &str) -> bool {
    let settled = (0..600).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(100));
        crate::hub::lock().vehicle(id).is_none_or(|v| v.mission_snapshot()[kind]["inProgress"].as_bool() != Some(true))
    });
    settled && crate::hub::lock().vehicle(id).is_some_and(|v| v.mission_snapshot()[kind]["error"].is_null())
}

fn clear(backend: &dyn Backend) -> Value {
    let unopened = held().document.is_none();
    let fresh = unopened.then(|| fresh_document(backend));
    clear_kinds(fresh, &["mission", "fence", "rally"]);
    json!({ "ok": true })
}

fn clear_kinds(fresh: Option<Document>, kinds: &[&str]) {
    forget_raw_edits();
    {
        let mut state = held();
        let template = state.document.clone().or(fresh).unwrap_or_else(empty_document);
        let before = state.document.clone();
        let empty = empty_document();
        let mission = kinds.contains(&"mission");
        state.document = Some(Document {
            home: if mission { None } else { template.home },
            items: if mission { Vec::new() } else { template.items.clone() },
            settings_sections: if mission { Vec::new() } else { template.settings_sections.clone() },
            fence: if kinds.contains(&"fence") { empty.fence } else { template.fence.clone() },
            rally: if kinds.contains(&"rally") { empty.rally } else { template.rally.clone() },
            ..template
        });
        state.wizard = None;
        remember(&mut state, before, crate::hub::now_ms());
        state.selected = 0;
        settle_clean(&mut state);
        state.file = None;
    }
    changed();
}

fn takeoff_required_first() -> bool {
    let state = held();
    state.document.as_ref().is_some_and(|document| {
        let spans = visual_spans(document);
        let sequence = spans.get(usize::try_from(state.selected).unwrap_or(0)).map_or(0, |(first, _)| *first);
        let rules = crate::missionkinds::Rules { takeoff_not_required: planning_setting("takeoffItemNotRequired", false), multiple_landings: planning_setting("allowMultipleLandingPatterns", true) };
        crate::missionkinds::insert_state(document, &spans, sequence, &rules).only_takeoff
    })
}

fn shape_points(file: &str, polyline: bool) -> Result<Vec<(f64, f64)>, String> {
    let lower = file.to_lowercase();
    let (points, found_polyline) = match () {
        _ if lower.ends_with(".kml") => match crate::kml::parse_wanted(&std::fs::read_to_string(file).map_err(|e| format!("Unable to open file: {file} error: {e}"))?, polyline)? {
            crate::kml::Shape::Polygon(points) => (points, false),
            crate::kml::Shape::Polyline(points) => (points, true),
        },
        _ if lower.ends_with(".shp") => crate::shp::parse_wanted(file, Some(polyline)).map(|(kind, _, points)| (points, kind == "polyline"))?,
        _ => return Err("Unsupported file type. Only .kml and .shp are supported.".to_string()),
    };
    match found_polyline == polyline {
        true => Ok(points),
        false if polyline => Err("No polyline found in the file.".to_string()),
        false => Err("No polygon found in the file.".to_string()),
    }
}

fn insert_from_shape_file(backend: &dyn Backend, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let kind = match given.get(0).and_then(Value::as_str).unwrap_or("") {
        "Survey" | "survey" => "survey",
        "Corridor Scan" | "CorridorScan" => "corridor",
        "Structure Scan" | "StructureScan" => "structure",
        other => return refused(format!("{other} cannot be built from a file.")),
    };
    let file = given.get(1).and_then(Value::as_str).unwrap_or("");
    let points = match shape_points(file, kind == "corridor") {
        Ok(points) if !points.is_empty() => points,
        Ok(_) => return refused("The file holds no points."),
        Err(reason) => return refused(reason),
    };
    let count = points.len() as f64;
    let centre = (points.iter().map(|p| p.0).sum::<f64>() / count, points.iter().map(|p| p.1).sum::<f64>() / count);
    let inserted = insert_kind(backend, &json!([kind, centre.0, centre.1, given.get(2)]).to_string());
    let Some(placed) = inserted.get("index").and_then(Value::as_i64).filter(|_| inserted["ok"] == true) else { return inserted };
    let key = if kind == "corridor" { "polyline" } else { "polygon" };
    let vertices: Vec<Value> = points.iter().map(|(lat, lon)| json!([lat, lon])).collect();
    edit(|doc| {
        let at = usize::try_from(placed - 1).ok().filter(|i| *i < doc.items.len()).ok_or("The imported item is missing.")?;
        let plandoc::Item::Complex { kind, json, .. } = &doc.items[at] else { return Err("The imported item is missing.".to_string()) };
        let shaped = crate::surveydoc::regenerate_item(&with_polygon(json, key, vertices.clone()));
        let item = plandoc::Item::Complex { kind: kind.clone(), item_count: plandoc::complex_count(kind, &shaped).unwrap_or(0), json: shaped };
        Ok(Document { items: doc.items.iter().enumerate().map(|(k, it)| if k == at { item.clone() } else { it.clone() }).collect(), ..doc.clone() })
    })
}

fn insert_kind(backend: &dyn Backend, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let kind = given.get(0).and_then(Value::as_str).unwrap_or("");
    let rest = json!([given.get(1), given.get(2), given.get(3)]).to_string();
    let clicked = given.get(1).and_then(Value::as_f64).zip(given.get(2).and_then(Value::as_f64));
    let placed_home = clicked.is_some_and(|(latitude, longitude)| {
        let mut state = held();
        let homeless = state.document.as_ref().is_some_and(|d| d.home.is_none());
        state.document = state.document.take().map(|d| Document { home: d.home.or_else(|| plandoc::home_from_first_coordinate(&d, Some((latitude, longitude)))), ..d });
        homeless
    });
    if placed_home {
        settle_home_on_terrain(None);
    }
    let answered = match kind {
        "waypoint" if takeoff_required_first() => {
            let takeoff = insert_takeoff(backend, &json!([given.get(3)]).to_string());
            match takeoff.get("ok").and_then(Value::as_bool) {
                Some(true) => {
                    let after = given.get(3).and_then(Value::as_i64).filter(|i| *i != -1).map_or(-1, |i| i + 1);
                    insert_at(backend, &json!([given.get(1), given.get(2), after]).to_string(), false)
                }
                _ => takeoff,
            }
        }
        "waypoint" => insert_at(backend, &rest, false),
        "land" => {
            let class = held().document.as_ref().map(|d| plandoc::vehicle_class(d.vehicle_type));
            match class {
                Some(crate::cmdinfo::VehicleClass::FixedWing | crate::cmdinfo::VehicleClass::Vtol) => insert_landing(backend, &rest),
                _ => insert_at(backend, &rest, true),
            }
        }
        "takeoff" => {
            let answered = insert_takeoff(backend, &json!([given.get(3)]).to_string());
            let fixed_wing = held().document.as_ref().is_some_and(|d| plandoc::vehicle_class(d.vehicle_type) == crate::cmdinfo::VehicleClass::FixedWing);
            if answered.get("ok").and_then(Value::as_bool) == Some(true) && fixed_wing {
                let mut state = held();
                let count = state.document.as_ref().map_or(0, |d| d.items.len() as i64);
                let wanted = given.get(3).and_then(Value::as_i64).unwrap_or(-1);
                state.wizard = usize::try_from(if (1..=count).contains(&wanted) { wanted } else { count }).ok();
            }
            answered
        }
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
    if property == "coordinate" {
        if let Some(complex @ plandoc::Item::Complex { .. }) = index.checked_sub(1).and_then(|at| current.items.get(at)) {
            let target = crate::fenceedit::point(given.as_ref());
            let at = index - 1;
            return Some(match target.and_then(|(latitude, longitude)| crate::plantransform::move_complex_to(complex, latitude, longitude)) {
                Some(moved) => edit(|doc| Ok(Document { items: doc.items.iter().enumerate().map(|(k, it)| if k == at { moved.clone() } else { it.clone() }).collect(), ..doc.clone() })),
                None => refused(format!("Item {index} could not be moved there.")),
            });
        }
    }
    let landing_at = index.checked_sub(1).and_then(|at| current.items.get(at)).is_some_and(|item| matches!(item, plandoc::Item::Complex { kind, .. } if crate::landingpattern::is_landing(kind)));
    if property == "wizardMode" && !landing_at {
        return Some(match given.as_ref().and_then(Value::as_bool) {
            Some(false) => {
                held().wizard = None;
                changed();
                json!({ "ok": true, "result": true, "refusal": Value::Null, "reason": Value::Null })
            }
            _ => refused("The climb-out step is only ever left, with false."),
        });
    }
    if let Some(member) = property.strip_prefix("cameraSection.") {
        let (name, by_index) = match member.split_once('.') {
            Some((name, "enumIndex")) => (name, true),
            Some((name, _)) => (name, false),
            None => (member, false),
        };
        let wanted = match (name, by_index, number) {
            ("cameraAction", true, Some(i)) => plandoc::camera_action_value(i as usize).map(|v| json!(v)),
            (CAMERA_INTERVAL_DISTANCE, false, Some(shown)) if !member.ends_with(".rawValue") => Some(json!(crate::units::cooking("m").map_or(shown, |c| (c.base)(shown)))),
            _ => given.clone(),
        };
        return Some(match wanted.and_then(|value| plandoc::set_camera(&current, index, name, &value)) {
            Some(changed) => edit(|_| Ok(changed)),
            None => refused(format!("Item {index} has no camera field {name}.")),
        });
    }
    if index == 0 && property == LAUNCH_ALTITUDE {
        let metres = number.map(|shown| crate::read::Unit::vertical(backend).meters(shown));
        return Some(match (metres, current.home) {
            (Some(altitude), Some([latitude, longitude, _])) => edit(|doc| Ok(Document { home: Some([latitude, longitude, altitude]), ..doc.clone() })),
            (None, _) => refused("An altitude is a number."),
            (_, None) => refused("The plan has no launch position yet."),
        });
    }
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
    let commands = crate::cmdinfo::tree(plandoc::firmware(current.firmware_type), plandoc::command_class_at(&current, index.saturating_sub(1)));
    let simple = index.checked_sub(1).and_then(|i| current.items.get(i)).and_then(|item| match item {
        plandoc::Item::Simple(s) => Some(s),
        plandoc::Item::Complex { .. } => None,
    });
    let raw = simple.is_some_and(|s| raw_edit(s, commands.get(&s.command), index));
    if property == "rawEdit" {
        let on = given.as_ref().and_then(Value::as_bool).unwrap_or(false);
        let mut chosen = raw_edits();
        if on { chosen.insert(index) } else { chosen.remove(&index) };
        drop(chosen);
        changed();
        return Some(json!({ "ok": true }));
    }
    match (raw, property, number) {
        (true, "comboboxFacts.0", Some(n)) => return Some(item_edit(backend, &json!([index, n]).to_string(), true)),
        (true, "comboboxFacts.1", Some(n)) => return Some(answer(plandoc::set_frame(&current, index, n as i64).ok_or_else(|| format!("Item {index} has no frame.")))),
        _ => {}
    }
    let field = |group: &str| {
        let at: usize = property.strip_prefix(group)?.parse().ok()?;
        let s = simple?;
        match raw {
            true => (group == "textFieldFacts." && at < RAW_LABELS.len()).then_some(at + 1),
            false if group == "nanFacts." => nan_params(commands.get(&s.command)?).get(at).map(|(param, _)| usize::from(*param)),
            false => field_params(commands.get(&s.command)?, group == "comboboxFacts.").get(at).map(|(param, _)| usize::from(*param)),
        }
    };
    let unknown = || Err(format!("Item {index} has no such field."));
    let out_of_range = |param: usize, value: f64| -> Option<String> {
        let info = simple.and_then(|s| commands.get(&s.command)).and_then(|info| info.params.get(&u8::try_from(param).ok()?))?;
        let limit = |key: &str| info.get(key).and_then(Value::as_f64).filter(|v| v.is_finite());
        let (min, max) = (limit("min"), limit("max"));
        let shown = |raw_limit: f64| param_conversion(info).map_or(raw_limit, |c| (c.shown)(raw_limit));
        (!raw && value.is_finite() && (min.is_some_and(|m| value < m) || max.is_some_and(|m| value > m)))
            .then(|| format!("Value must be within {} and {}", qt_number(min.map_or(f64::MIN, shown)), qt_number(max.map_or(f64::MAX, shown))))
    };
    let raw_of = |param: usize, shown: f64| match raw {
        true => shown,
        false => simple
            .and_then(|s| commands.get(&s.command))
            .and_then(|info| info.params.get(&u8::try_from(param).ok()?))
            .and_then(param_conversion)
            .map_or(shown, |c| (c.base)(shown)),
    };
    Some(match property {
        "altitude" | "command" => return Some(match number {
            Some(n) => {
                let raw = if property == "altitude" { crate::missionitems::altitude_from_shown(n) } else { n };
                item_edit(backend, &json!([index, raw]).to_string(), property == "command")
            }
            None => refused("That field takes a number."),
        }),
        "altitudeMode" | "altitudeFrame" => answer(
            number
                .ok_or_else(|| "An altitude mode is a number.".to_string())
                .and_then(|mode| plandoc::set_altitude_mode(&current, index, mode as i64).ok_or_else(|| format!("Item {index} cannot take that altitude mode."))),
        ),
        "coordinate" | "launchCoordinate" => answer(
            crate::fenceedit::point(given.as_ref())
                .ok_or_else(|| "A position needs a latitude from -90 to 90 and a longitude from -180 to 180.".to_string())
                .and_then(|(latitude, longitude)| {
                    let moved_home = Document { home: Some([latitude, longitude, current.home.map_or(0.0, |h| h[2])]), ..current.clone() };
                    let moved_item = |doc: &Document| plandoc::set_param(doc, index, 5, latitude).and_then(|moved| plandoc::set_param(&moved, index, 6, longitude)).ok_or_else(|| format!("Item {index} has no position to move."));
                    let same_location = launch_at_takeoff(&current, index);
                    let transition_distance = crate::read::value_number(&backend.get("settings.planViewSettings.vtolTransitionDistance.rawValue")).unwrap_or(plandoc::VTOL_TRANSITION_DISTANCE_DEFAULT);
                    match (index, property) {
                        (0, _) => Ok(moved_home),
                        (_, "launchCoordinate") => plandoc::set_launch(&current, index, latitude, longitude, same_location, transition_distance).ok_or_else(|| format!("Item {index} is not a takeoff.")),
                        (_, _) if same_location => moved_item(&moved_home),
                        _ => moved_item(&current),
                    }
                }),
        ),
        "loiterRadius" => answer(number.ok_or_else(|| "A radius is a number.".to_string()).and_then(|v| plandoc::set_loiter_radius(&current, index, v).ok_or_else(|| format!("Item {index} is not a loiter.")))),
        "hold" => answer(
            number
                .filter(|v| v.is_finite() && *v >= 0.0)
                .ok_or_else(|| "A hold is a number of seconds, 0 or more.".to_string())
                .and_then(|v| plandoc::set_param(&current, index, usize::from(HOLD_PARAM), v).ok_or_else(|| format!("Item {index} has no hold."))),
        ),
        "speedSection.flightSpeed" => answer(
            number
                .map(|shown| crate::units::cooking("m/s").map_or(shown, |c| (c.base)(shown)))
                .ok_or_else(|| "A speed is a number.".to_string())
                .and_then(|v| plandoc::set_speed(&current, index, Some(v)).ok_or_else(|| format!("Item {index} carries no speed."))),
        ),
        "speedSection.specifyFlightSpeed" => {
            let on = given.as_ref().and_then(Value::as_bool).unwrap_or(false);
            let keep = plandoc::specified_speed(&current, index);
            let setting = |name: &str, default: f64| crate::read::value_number(&backend.get(&format!("settings.appSettings.{name}.rawValue"))).unwrap_or(default);
            let default = speed_in_force(&current, index.saturating_sub(1), setting("offlineEditingHoverSpeed", 5.0), setting("offlineEditingCruiseSpeed", 15.0));
            let speed = on.then(|| keep.unwrap_or(default));
            answer(plandoc::set_speed(&current, index, speed).ok_or_else(|| format!("Item {index} carries no speed.")))
        }
        _ if property.starts_with("nanFacts.") => match field("nanFacts.") {
            Some(param) => match number.and_then(|v| out_of_range(param, raw_of(param, v))) {
                Some(reason) => refused(reason),
                None => answer(plandoc::set_param(&current, index, param, number.map_or(f64::NAN, |v| raw_of(param, v))).ok_or_else(|| format!("Item {index} has no such field."))),
            },
            None => answer(unknown()),
        },
        _ => match (field("textFieldFacts.").or_else(|| field("comboboxFacts.")), number) {
            (Some(param), Some(v)) => match out_of_range(param, raw_of(param, v)) {
                Some(reason) => refused(reason),
                None => answer(plandoc::set_param(&current, index, param, raw_of(param, v)).ok_or_else(|| format!("Item {index} has no such field."))),
            },
            (Some(_), None) => refused("That field takes a number."),
            (None, _) => answer(unknown()),
        },
    })
}

fn qt_number(value: f64) -> String {
    let scientific = format!("{value:.5e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((scientific.as_str(), "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let trimmed = |text: &str| match text.contains('.') {
        true => text.trim_end_matches('0').trim_end_matches('.').to_string(),
        false => text.to_string(),
    };
    match (-4..6).contains(&exponent) {
        true => trimmed(&format!("{:.*}", (5 - exponent).max(0) as usize, value)),
        false => format!("{}e{}{:02}", trimmed(mantissa), if exponent < 0 { '-' } else { '+' }, exponent.abs()),
    }
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
                let default = held().breach_altitude.or_else(|| crate::read::value_number(&backend.get(&format!("{DEFAULT_ALTITUDE}.rawValue"))));
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
            let Some(metres) = given.as_f64().filter(|a| a.is_finite()).map(|shown| crate::read::Unit::vertical(backend).meters(shown)) else {
                return Some(refused("A breach return altitude is a number."));
            };
            held().breach_altitude = Some(metres);
            let placed = held().document.as_ref().is_some_and(|d| d.fence.get("breachReturn").and_then(Value::as_array).is_some_and(|b| b.len() >= 2));
            match placed {
                true => fence_edit(|f, r| Some((crate::fencedoc::set_breach_altitude(f, metres)?, r.clone())), ""),
                false => {
                    changed();
                    json!({ "ok": true })
                }
            }
        }
        _ => {
            if let Some((index, member)) = indexed(path, "plan.geoFenceController.polygons.") {
                return (member == "inclusion").then(|| fence_edit(|f, r| Some((crate::fencedoc::set_inclusion(f, "polygons", index, given.as_bool()?)?, r.clone())), "A polygon is an inclusion (true) or an exclusion (false)."));
            }
            if let Some((index, member)) = indexed(path, "plan.geoFenceController.circles.") {
                return Some(match member.as_str() {
                    "inclusion" => fence_edit(|f, r| Some((crate::fencedoc::set_inclusion(f, "circles", index, given.as_bool()?)?, r.clone())), "A circle is an inclusion (true) or an exclusion (false)."),
                    "center" => fence_edit(|f, r| Some((crate::fencedoc::set_circle(f, index, Some(point_of(Some(&given))?), None)?, r.clone())), "A circle's centre needs a latitude and a longitude."),
                    "radius" => {
                        let horizontal = crate::read::Unit::horizontal(backend);
                        let metres = given.as_f64().map(|shown| horizontal.meters(shown));
                        fence_edit(|f, r| Some((crate::fencedoc::set_circle(f, index, None, Some(metres.filter(|v| *v >= 0.1)?))?, r.clone())), "A circle's radius is at least 0.1 m.")
                    }
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
                        Some((f.clone(), crate::fencedoc::move_rally(r, index, at, given.as_f64().map(|shown| crate::read::Unit::vertical(backend).meters(shown)))?))
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
        "appendVertices" => {
            let added: Option<Vec<Value>> = given.get(0)?.as_array()?.iter().map(|point| point_of(Some(point)).map(spelled)).collect();
            Some(vertices.iter().cloned().chain(added?).collect())
        }
        "clear" => Some(Vec::new()),
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
            plandoc::Item::Complex { kind, json, .. } if kind == "CorridorScan" => return Some(crate::corridorscan::corridor_polygon(json).into_iter().map(|(latitude, longitude, _)| (latitude, longitude)).collect()),
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

fn live_wizard(state: &Held) -> Option<usize> {
    state.wizard.filter(|at| {
        let item = at.checked_sub(1).and_then(|i| state.document.as_ref()?.items.get(i));
        matches!(item, Some(plandoc::Item::Simple(s)) if [CMD_NAV_TAKEOFF, CMD_NAV_VTOL_TAKEOFF_ID].contains(&s.command))
    })
}

pub fn wizard_item() -> Option<usize> {
    enabled().then(|| live_wizard(&held())).flatten()
}

pub fn awaiting_terrain() -> bool {
    held().document.as_ref().is_some_and(waiting_on_terrain)
}

pub fn drawing() -> bool {
    let state = held();
    live_wizard(&state).is_some()
        || state.document.as_ref().is_some_and(|d| {
            d.items.iter().any(|item| {
                matches!(item, plandoc::Item::Complex { json, .. } if json.get("polygon").and_then(Value::as_array).is_some_and(|p| p.len() < 3) || json.get("polyline").and_then(Value::as_array).is_some_and(|p| p.len() < 2) || json.get(crate::landingpattern::WIZARD).and_then(Value::as_bool) == Some(true))
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
    if let Some(index) = path.strip_prefix("plan.missionController.visualItems.").and_then(|rest| rest.strip_suffix(".setLandingHeadingToTakeoffHeading")).and_then(|i| i.parse::<usize>().ok()) {
        return Some(edit(|doc| {
            let at = index.checked_sub(1).filter(|i| *i < doc.items.len()).ok_or("No such item.")?;
            let plandoc::Item::Complex { kind, json, item_count } = &doc.items[at] else { return Err("Only a landing pattern has a landing heading.".to_string()) };
            let Some(heading) = takeoff_heading(doc) else { return Ok(doc.clone()) };
            let turned = crate::landingpattern::edit(json, "landingHeading", &json!(heading)).ok_or("Only a landing pattern has a landing heading.")?;
            let item = plandoc::Item::Complex { kind: kind.clone(), item_count: plandoc::complex_count(kind, &turned).unwrap_or(*item_count), json: turned };
            Ok(Document { items: doc.items.iter().enumerate().map(|(k, it)| if k == at { item.clone() } else { it.clone() }).collect(), ..doc.clone() })
        }));
    }
    if let Some(index) = path.strip_prefix("plan.missionController.visualItems.").and_then(|rest| rest.strip_suffix(".rotateEntryPoint")).and_then(|i| i.parse::<usize>().ok()) {
        return Some(edit(|doc| {
            let at = index.checked_sub(1).filter(|i| *i < doc.items.len()).ok_or("No such item.")?;
            let plandoc::Item::Complex { kind, json, item_count } = &doc.items[at] else { return Err("Only a pattern has an entry point.".to_string()) };
            let rotated = crate::surveydoc::rotated_entry(kind, json).ok_or("This pattern has no entry point to rotate.")?;
            let item = plandoc::Item::Complex { kind: kind.clone(), item_count: plandoc::complex_count(kind, &rotated).unwrap_or(*item_count), json: rotated };
            Ok(Document { items: doc.items.iter().enumerate().map(|(k, it)| if k == at { item.clone() } else { it.clone() }).collect(), ..doc.clone() })
        }));
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
        "plan.sendToVehicle" if !carried() && crate::qthost::present() => send_through_host(backend),
        "plan.loadFromVehicle" if !carried() && crate::qthost::present() => fetch_through_host(backend),
        "plan.sendToVehicle" => send(),
        "plan.loadFromVehicle" => fetch(),
        "plan.removeAllFromVehicle" if !carried() && crate::qthost::present() => return None,
        "plan.removeAllFromVehicle" => remove_all_from_vehicle(backend),
        "plan.removeAll" => clear(backend),
        "mission.insert" => insert_kind(backend, args),
        "plan.missionController.insertComplexMissionItemFromKMLOrSHP" => insert_from_shape_file(backend, args),
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

    #[test]
    fn no_statement_holds_the_plan_lock_while_it_calls_into_the_backend() {
        let body = include_str!("coreplan.rs").split("#[cfg(test)]\nmod tests").next().unwrap_or("");
        let held_across: Vec<&str> = body.lines().filter(|line| line.contains("held()") && line.contains("(backend")).collect();
        assert!(held_across.is_empty(), "a held() temporary lives to the end of its statement, and a backend read can route back into controller_fields, which takes the same lock: {held_across:?}");
    }

    #[test]
    fn a_plan_sequence_is_dropped_when_the_active_vehicle_changes_like_plan_master_controller() {
        assert_eq!(transfer_outcome(Some(1), 1, true), None, "keeps waiting while its vehicle transfers");
        assert_eq!(transfer_outcome(Some(1), 1, false), Some(true), "moves on once the transfer settles");
        assert_eq!(transfer_outcome(Some(2), 1, true), Some(false), "_activeVehicleChanged sets the sequence Idle, so the old plan never reaches vehicle 2");
        assert_eq!(transfer_outcome(None, 1, false), Some(false), "nor continues with no vehicle");
    }

    #[test]
    fn the_photo_distance_is_shown_in_the_horizontal_units_like_qgcs_metre_fact() {
        let doc = plandoc::set_camera(&plandoc::set_camera(&empty_document(), 0, "cameraAction", &json!(2)).unwrap(), 0, CAMERA_INTERVAL_DISTANCE, &json!(30.48)).unwrap();
        let section = cooked_camera_distance(plandoc::camera_section(&doc.settings_sections), crate::units::cooking_with("m", |_| None, crate::units::IMPERIAL_US));
        let distance = section["facts"].as_array().unwrap().iter().find(|f| f["property"] == CAMERA_INTERVAL_DISTANCE).unwrap().clone();
        assert!((distance["value"].as_f64().unwrap() - 100.0).abs() < 1e-9, "{distance}");
        assert_eq!(distance["units"], "ft");
        assert_eq!(distance["valueString"], "100.0");
        let time = section["facts"].as_array().unwrap().iter().find(|f| f["property"] == "cameraPhotoIntervalTime").unwrap().clone();
        assert_eq!(time["units"], "secs", "only the distance is a metre fact");
    }

    #[test]
    fn the_speed_for_time_between_shots_follows_the_flight_status_calculator() {
        let simple = |command: i64, speed: f64| plandoc::Item::Simple(plandoc::Simple { command, frame: 2, params: [Some(1.0), Some(speed), Some(-1.0), Some(0.0), None, None, None], auto_continue: true, altitude: None, sections: vec![] });
        let doc = |vehicle_type: i64, items: Vec<plandoc::Item>| Document { vehicle_type, items, ..empty_document() };
        assert_eq!(speed_in_force(&doc(22, vec![]), 0, 5.0, 15.0), 5.0, "a VTOL starts at the hover speed");
        assert_eq!(speed_in_force(&doc(1, vec![]), 0, 5.0, 15.0), 15.0, "a plane at cruise");
        assert_eq!(speed_in_force(&doc(2, vec![simple(178, 10.0)]), 1, 5.0, 15.0), 10.0, "a standalone DO_CHANGE_SPEED counts");
        assert_eq!(speed_in_force(&doc(2, vec![simple(178, -1.0)]), 1, 5.0, 15.0), 5.0, "one leaving the speed alone does not");
        let later = doc(2, vec![simple(178, 10.0), simple(16, 0.0)]);
        assert_eq!(speed_section(&later, 2, &[], true, 5.0, 15.0)["value"], 10.0, "SimpleMissionItem::setMissionFlightStatus seeds an unset, available speed with the speed in force there");
        assert_eq!(speed_section(&later, 2, &[], true, 5.0, 15.0)["slider"], json!({ "from": 0.0, "to": 30.0, "decimals": 1 }), "SpeedSection.FactMetaData.json FlightSpeed userMin 0, userMax 30");
        assert_eq!(speed_section(&doc(22, vec![simple(16, 0.0)]), 1, &[], true, 5.0, 15.0)["available"], false, "SpeedSection::setAvailable only takes multirotors and fixed wings, so a VTOL never offers Flight Speed");
    }

    #[test]
    fn a_vtol_item_shows_the_fields_of_the_mode_it_flies_in() {
        let text = r#"{"fileType":"Plan","version":1,"groundStation":"QGroundControl","mission":{"version":2,"firmwareType":12,"vehicleType":22,"cruiseSpeed":15,"hoverSpeed":5,"plannedHomePosition":[47.0,8.0,500],"items":[{"type":"SimpleItem","autoContinue":true,"command":22,"doJumpId":1,"frame":3,"params":[15,0,0,null,47.001,8.0,50]}]},"geoFence":{"version":2,"polygons":[],"circles":[]},"rallyPoints":{"version":2,"points":[]}}"#;
        let doc = plandoc::load(text, 22).unwrap();
        assert_eq!(plandoc::command_class_at(&doc, 0), crate::cmdinfo::VehicleClass::FixedWing, "MissionFlightStatusCalculator starts a VTOL without a VTOL takeoff in fixed-wing mode");
        let metres = crate::read::Unit { name: "m".to_string(), factor: 1.0 };
        let facts = document_facts(&doc, 1, 5.0, 15.0, (&metres, &metres));
        let labels: Vec<String> = facts["fields"].as_array().unwrap().iter().filter_map(|f| f["label"].as_str().map(str::to_string)).collect();
        assert!(labels.iter().any(|l| l.contains("Pitch")), "so its NAV_TAKEOFF shows the fixed-wing Pitch field: {labels:?}");
        let simple = |command: i64| plandoc::Item::Simple(plandoc::Simple { command, frame: 3, params: [Some(0.0); 7], auto_continue: true, altitude: None, sections: vec![] });
        let mixed = Document { vehicle_type: 22, items: vec![simple(16), simple(84), simple(16), simple(22), simple(20), simple(16)], ..empty_document() };
        assert_eq!(plandoc::command_class_at(&mixed, 0), crate::cmdinfo::VehicleClass::FixedWing, "the start follows the LAST takeoff before the RTL, here a fixed-wing NAV_TAKEOFF");
        assert_eq!(plandoc::command_class_at(&mixed, 2), crate::cmdinfo::VehicleClass::FixedWing, "a VTOL takeoff leaves fixed-wing mode behind it");
        assert_eq!(plandoc::command_class_at(&mixed, 5), crate::cmdinfo::VehicleClass::Vtol, "items from the RTL on get no flight status, so they keep the VTOL tree");
    }

    #[test]
    fn a_multirotor_takeoff_at_home_moves_with_it_and_a_plane_does_not() {
        let takeoff = |lat: f64| plandoc::Item::Simple(plandoc::Simple { command: 22, frame: 3, params: [Some(0.0), Some(0.0), Some(0.0), None, Some(lat), Some(8.0), Some(30.0)], auto_continue: true, altitude: None, sections: vec![] });
        let copter = Document { vehicle_type: 2, firmware_type: 12, home: Some([47.0, 8.0, 0.0]), items: vec![takeoff(47.0)], ..empty_document() };
        assert!(launch_at_takeoff(&copter, 1), "TakeoffMissionItem keeps launch and takeoff together for a multirotor whose takeoff sits on home");
        assert!(!launch_at_takeoff(&Document { items: vec![takeoff(47.001)], ..copter.clone() }, 1), "unless the two were moved apart");
        assert!(!launch_at_takeoff(&Document { vehicle_type: 1, ..copter }, 1), "a fixed wing launches apart from its takeoff");
    }

    #[test]
    fn a_waypoint_offers_its_hold_time_which_qgc_keeps_among_the_advanced_fields() {
        let waypoint = |hold: Option<f64>| plandoc::Simple { command: 16, frame: 3, params: [hold, Some(0.0), Some(0.0), None, Some(47.0), Some(8.0), Some(30.0)], auto_continue: true, altitude: None, sections: vec![] };
        let commands = crate::cmdinfo::tree(crate::cmdinfo::Firmware::ArduPilot, crate::cmdinfo::VehicleClass::MultiRotor);
        let info = commands.get(&16);
        assert_eq!(hold_field(&waypoint(Some(5.0)), info, false, "plan.missionController.visualItems.2"), json!({ "value": 5.0, "units": "s", "path": "plan.missionController.visualItems.2.hold" }));
        assert_eq!(hold_field(&waypoint(None), info, false, "x")["value"], 0.0, "an unset hold reads as no hold");
        assert_eq!(hold_field(&waypoint(Some(5.0)), info, true, "x"), Value::Null, "raw edit already shows Param1");
        let takeoff = plandoc::Simple { command: 22, ..waypoint(Some(5.0)) };
        assert_eq!(hold_field(&takeoff, commands.get(&22), false, "x"), Value::Null);
    }

    #[test]
    fn a_plan_of_only_a_fence_or_rally_points_contains_items() {
        assert!(!contains_items(&empty_document()));
        assert!(contains_items(&Document { fence: json!({ "polygons": [{ "inclusion": true, "polygon": [] }], "circles": [] }), ..empty_document() }), "PlanMasterController::containsItems counts the geofence");
        assert!(contains_items(&Document { rally: json!({ "points": [[47.0, 8.0, 50.0]] }), ..empty_document() }), "and the rally points");
    }

    #[test]
    fn a_plan_without_speeds_keeps_the_offline_speeds_as_qgc_does() {
        let text = r#"{"fileType":"Plan","version":1,"groundStation":"QGroundControl","mission":{"version":2,"firmwareType":12,"vehicleType":2,"plannedHomePosition":[47.0,8.0,500],"items":[]},"geoFence":{"version":2,"polygons":[],"circles":[]},"rallyPoints":{"version":2,"points":[]}}"#;
        assert!(plandoc::load(text, 2).unwrap().cruise_speed.is_nan(), "absent is told apart from zero");
        let loaded = load_plan(text).unwrap();
        assert!(loaded.cruise_speed > 0.0 && loaded.hover_speed > 0.0, "MissionController::_loadJsonMissionFileV2 only overwrites the offline speeds when the file carries them");
    }

    #[test]
    fn a_mission_parameter_in_metres_is_shown_in_the_users_units_like_its_fact() {
        let feet = crate::units::cooking_with("m", |_| Some("0".to_string()), 0);
        let radius = json!({ "label": "Radius", "units": "m", "default": 30.0, "decimalPlaces": 1 });
        let shown = param_fact_with(&radius, 30.48, feet);
        assert!((shown["value"].as_f64().unwrap() - 100.0).abs() < 1e-9, "FactMetaData translates m to ft for a float parameter");
        assert_eq!((shown["units"].as_str(), shown["rawUnits"].as_str(), shown["rawValue"].as_f64()), (Some("ft"), Some("m"), Some(30.48)));
        let listed = json!({ "label": "Mode", "units": "m", "enumStrings": "A,B", "enumValues": "0,1" });
        assert_eq!(param_conversion(&listed).map(|c| c.name), None, "an enumerated fact is never translated");
        let pitch = json!({ "label": "Pitch", "units": "gimbal-degrees" });
        assert_eq!(param_conversion(&pitch).map(|c| ((c.shown)(10.0), c.name)), Some((-10.0, "deg")), "the built-in translator comes first, as setBuiltInTranslator does");
        let feet_back = crate::units::cooking_with("m", |_| Some("0".to_string()), 0).map(|c| (c.base)(100.0)).unwrap();
        assert!((feet_back - 30.48).abs() < 1e-9, "a value typed in feet is written back in metres");
    }

    #[test]
    fn the_climb_out_step_only_counts_while_its_takeoff_is_still_there() {
        let simple = |command: i64| plandoc::Item::Simple(plandoc::Simple { command, frame: 3, params: [Some(0.0); 7], auto_continue: true, altitude: None, sections: vec![] });
        let with = |items: Vec<plandoc::Item>| Held { wizard: Some(1), document: Some(Document { items, ..empty_document() }), ..Held::default() };
        assert_eq!(live_wizard(&with(vec![simple(CMD_NAV_TAKEOFF)])), Some(1));
        assert_eq!(live_wizard(&with(vec![simple(16)])), None, "an item inserted before it shifted the takeoff away");
        assert_eq!(live_wizard(&with(Vec::new())), None, "the takeoff was deleted, so save and upload are not held back by it");
    }

    #[test]
    fn a_connected_vehicle_sets_the_offline_planning_classes_like_plan_master_controller() {
        assert_eq!(offline_classes(3, 1), (3, 1), "an ArduPilot plane plans as ArduPilot fixed wing");
        assert_eq!(offline_classes(12, 2), (12, 2), "a PX4 quad as PX4 multirotor");
        assert_eq!(offline_classes(12, 22), (12, 20), "a VTOL type as VTOL");
    }

    fn by_value(value: Value) -> Value {
        match value {
            Value::Number(n) => json!(n.as_f64()),
            Value::Array(items) => Value::Array(items.into_iter().map(by_value).collect()),
            Value::Object(fields) => Value::Object(fields.into_iter().map(|(k, v)| (k, by_value(v))).collect()),
            other => other,
        }
    }

    #[test]
    fn a_boundary_file_gives_the_shape_the_pattern_needs() {
        let dir = std::env::temp_dir().join(format!("shape-points-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let kml = dir.join("area.kml");
        std::fs::write(&kml, "<kml><Placemark><Polygon><outerBoundaryIs><LinearRing><coordinates>8.0,47.0,0 8.01,47.0,0 8.01,47.01,0 8.0,47.0,0</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark></kml>").unwrap();
        let path = kml.to_string_lossy().to_string();
        assert_eq!(shape_points(&path, false).map(|p| p.len()), Ok(3), "the repeated closing vertex is dropped");
        assert_eq!(shape_points(&path, true), Err("KML file load failed. Unable to find LineString node in KML".to_string()), "a corridor needs a polyline");
        assert!(shape_points("area.gpx", false).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_removed_item_hands_current_to_the_one_now_at_its_index_or_the_last() {
        assert_eq!(current_after_remove(2, 4), 2, "MissionController::removeVisualItem keeps the index");
        assert_eq!(current_after_remove(5, 4), 4, "and steps back past the end");
        assert_eq!(current_after_remove(1, 0), 0, "with nothing left the settings item is current");
    }

    #[test]
    fn a_waypoints_file_opens_as_a_plan_like_load_text_file() {
        let text = "QGC WPL 110\n0\t1\t0\t16\t0\t0\t0\t0\t47.66\t-122.10\t5.2\t1\n1\t0\t3\t22\t0\t0\t0\t0\t47.661\t-122.103\t100\t1\n2\t0\t3\t16\t0\t0\t0\t0\t47.662\t-122.104\t100\t1\n";
        let doc = plandoc::load(&plan_text(text).unwrap(), 2).unwrap();
        assert_eq!(doc.items.len(), 2);
        assert_eq!(doc.home.map(|h| h[2]), Some(5.2), "the first row of a 110 file is the planned home");
        assert!(plan_text("QGC WPL 110\n0\t1\t0\n").is_err());
        assert_eq!(plan_text("{}").unwrap(), "{}", "a plan file passes through untouched");
    }

    #[test]
    fn a_new_default_altitude_lands_on_every_item_like_apply_new_altitude() {
        let doc = plandoc::load(include_str!("../tests/fixtures/survey-upload.plan"), 2).unwrap();
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
        let doc = plandoc::load(include_str!("../tests/fixtures/commands.plan"), 2).unwrap();
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
    fn a_dirty_plan_asks_what_to_do_when_its_vehicle_goes_or_changes() {
        assert_eq!(vehicle_change_prompt_for(None, true), Value::Null);
        let gone = vehicle_change_prompt_for(Some(true), true);
        assert_eq!((gone["title"].as_str(), gone["loadText"].as_str(), gone["keepText"].as_str()), (Some("Plan View - Vehicle Disconnected"), Some("Discard Unsaved Changes"), Some("Keep Current Plan")));
        let changed = vehicle_change_prompt_for(Some(false), true);
        assert_eq!(changed["loadText"], "Discard Unsaved Changes, Load New Plan From Vehicle");
        assert_eq!(vehicle_change_prompt_for(Some(false), false)["loadText"], "Load New Plan From Vehicle");
    }

    #[test]
    fn a_landing_lines_up_with_the_takeoff_run_from_home() {
        let doc = plandoc::load(include_str!("../tests/fixtures/commands.plan"), 2).unwrap();
        let takeoff = |lat: f64, lon: f64| plandoc::Item::Simple(plandoc::Simple { command: CMD_NAV_TAKEOFF, frame: 3, params: [Some(0.0), None, None, None, Some(lat), Some(lon), Some(30.0)], auto_continue: true, altitude: None, sections: vec![] });
        let home = doc.home.unwrap();
        let east = Document { items: vec![takeoff(home[0], home[1] + 0.01)], ..doc.clone() };
        assert!(takeoff_heading(&east).is_some_and(|h| (h - 90.0).abs() < 0.5));
        assert_eq!(takeoff_heading(&Document { items: Vec::new(), ..doc.clone() }), None, "no takeoff, no heading to copy");
    }

    #[test]
    fn a_pattern_shape_is_complete_once_it_can_be_flown() {
        assert!(!shape_complete("survey", &json!({ "polygon": [[1, 2], [3, 4]] })));
        assert!(shape_complete("survey", &json!({ "polygon": [[1, 2], [3, 4], [5, 6]] })));
        assert!(shape_complete("CorridorScan", &json!({ "polyline": [[1, 2], [3, 4]] })));
        assert!(!shape_complete("StructureScan", &json!({})));
    }

    #[test]
    fn range_limits_print_as_qstring_arg_does() {
        assert_eq!((qt_number(0.0), qt_number(3600.0), qt_number(f64::MAX), qt_number(-f64::MAX), qt_number(0.5), qt_number(123456.0)), ("0".into(), "3600".into(), "1.79769e+308".into(), "-1.79769e+308".into(), "0.5".into(), "123456".into()));
        assert_eq!(qt_number(1_234_567.0), "1.23457e+06");
        assert_eq!((qt_number(999_999.5), qt_number(9_999_999.0), qt_number(0.000_099_999_99)), ("1e+06".into(), "1e+07".into(), "0.0001".into()), "the exponent is read after rounding, as %g does");
    }

    #[test]
    fn show_all_values_moves_with_its_item() {
        let at = |lat: f64| plandoc::Item::Simple(plandoc::Simple { command: 16, frame: 3, params: [Some(0.0), Some(0.0), Some(0.0), None, Some(lat), Some(8.0), Some(30.0)], auto_continue: true, altitude: None, sections: vec![] });
        let (a, b, c) = (at(47.0), at(47.1), at(47.2));
        *raw_edits() = [3].into();
        shift_raw_edits(&[a.clone(), b.clone(), c.clone()], &[a.clone(), c.clone()]);
        assert_eq!(*raw_edits(), [2].into(), "deleting item 2 moves item 3's raw view to 2");
        shift_raw_edits(&[a.clone(), c.clone()], &[b.clone(), a.clone(), c.clone()]);
        assert_eq!(*raw_edits(), [3].into(), "an insert in front shifts it back");
        *raw_edits() = [2].into();
        shift_raw_edits(&[a.clone(), b.clone(), c.clone()], &[a, c]);
        assert!(raw_edits().is_empty(), "the deleted item's raw view goes with it");
    }

    #[test]
    fn an_item_qgc_cannot_show_friendly_is_edited_raw() {
        let doc = plandoc::load(include_str!("../tests/fixtures/commands.plan"), 2).unwrap();
        let metres = crate::read::Unit { name: "m".to_string(), factor: 1.0 };
        let friendly = document_facts(&doc, 1, 5.0, 15.0, (&metres, &metres));
        assert_eq!((friendly["rawEdit"].clone(), friendly["friendlyEditAllowed"].clone()), (json!(false), json!(true)));
        let stuck = Document {
            items: doc.items.iter().enumerate().map(|(i, item)| match (i, item) {
                (0, plandoc::Item::Simple(s)) => plandoc::Item::Simple(plandoc::Simple { auto_continue: false, ..s.clone() }),
                (_, other) => other.clone(),
            }).collect(),
            ..doc.clone()
        };
        let raw = document_facts(&stuck, 1, 5.0, 15.0, (&metres, &metres));
        assert_eq!((raw["rawEdit"].clone(), raw["friendlyEditAllowed"].clone()), (json!(true), json!(false)), "autoContinue off cannot be shown in simple mode");
        let fields = raw["fields"].as_array().unwrap();
        let names: Vec<&str> = fields.iter().map(|f| f["name"].as_str().unwrap_or_default()).collect();
        assert_eq!(names, ["Command", "Frame", "Param1", "Param2", "Param3", "Param4", "Lat/X", "Lon/Y", "Alt/Z"], "SimpleItemEditor lists comboboxFacts before textFieldFacts");
        assert_eq!(fields[1]["pathSuffix"], "comboboxFacts.1");
        assert_eq!((fields[0]["control"].clone(), fields[0]["display"].clone()), (json!("choice"), json!("MAV_CMD_NAV_WAYPOINT")));
        assert!(fields[1]["options"].as_array().unwrap().iter().any(|o| o["label"] == "MAV_FRAME_GLOBAL_TERRAIN_ALT" && o["raw"] == "10"));
    }

    #[test]
    fn optional_params_are_served_as_nan_facts_and_switch_off_to_nan() {
        let commands = crate::cmdinfo::tree(crate::cmdinfo::Firmware::Px4, crate::cmdinfo::VehicleClass::Vtol);
        let shown = |id: i64| nan_params(&commands[&id]).iter().map(|(i, _)| *i).collect::<Vec<_>>();
        assert_eq!(shown(85), [4], "SimpleItemEditor's nanFacts: VTOL land Yaw, while its advanced Approach Alt stays hidden");
        assert_eq!(shown(187), [1, 2, 3, 4], "each actuator of DO_SET_ACTUATOR is optional");
        assert!(shown(16).is_empty(), "a waypoint's Yaw is advanced and goes to nanFactsAdvanced, which the editor does not show");
        let doc = plandoc::load(include_str!("../tests/fixtures/commands.plan"), 2).unwrap();
        let at = 1;
        let unset = plandoc::set_param(&doc, at, 4, f64::NAN).unwrap();
        let Some(plandoc::Item::Simple(s)) = unset.items.get(at - 1) else { panic!() };
        assert!(s.params[3].is_some_and(f64::is_nan), "off writes NaN, which a vehicle reads as unchanged");
    }

    #[test]
    fn the_launch_altitude_is_an_editable_field_on_mission_settings() {
        let field = launch_altitude_field(123.5, "m", "plan.missionController.visualItems.0.plannedHomePositionAltitude");
        assert_eq!(field["path"], "plan.missionController.visualItems.0.plannedHomePositionAltitude");
        assert_eq!(field["label"], "Altitude", "MissionSettingsEditor's Launch Position altitude");
    }

    #[test]
    fn an_item_reads_edited_until_the_plan_it_is_in_is_saved() {
        let saved = plandoc::load(include_str!("../../test/MissionManager/SectionTest.plan"), 2).unwrap();
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
        assert_eq!(vertex_edit(&line, "clear", &json!([]), false), Some(Vec::new()), "QGCMapPolygon::clear empties the shape before a reset refills it");
        let refilled = vertex_edit(&[], "appendVertices", &json!([[{ "latitude": 1.0, "longitude": 2.0 }, { "latitude": 3.0, "longitude": 4.0 }]]), true).unwrap();
        assert_eq!(refilled, vec![json!([1.0, 2.0]), json!([3.0, 4.0])]);
        assert!(vertex_edit(&[], "appendVertices", &json!([[{ "latitude": 91.0, "longitude": 2.0 }]]), true).is_none(), "one bad coordinate refuses the whole list");
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
        let doc = plandoc::load(include_str!("../../test/MissionManager/SectionTest.plan"), 2).unwrap();
        let rows: Vec<Option<i64>> = (0..8).map(|seq| visual_index_of_sequence(&doc, seq)).collect();
        assert_eq!(rows, vec![Some(0), Some(1), Some(2), Some(3), Some(3), Some(4), None, None], "sequence 4 is the mount control folded into row 3, so selecting it selects row 3");
    }
}

pub struct PlanState {
    pub syncing: bool,
    pub progress: f64,
    pub dirty: bool,
    pub dirty_for_save: bool,
    pub contains_items: bool,
    pub has_mission_items: bool,
    pub file: String,
    pub global_mode: i64,
    pub item_count: i64,
}

fn has_entries(section: &Value, key: &str) -> bool {
    section.get(key).and_then(Value::as_array).is_some_and(|list| !list.is_empty())
}

pub fn plan_state() -> Option<PlanState> {
    if !enabled() {
        return None;
    }
    let mission = crate::hub::lock().active().map(|v| v.mission_snapshot()["mission"].clone()).unwrap_or_default();
    let syncing = mission["inProgress"].as_bool() == Some(true);
    let progress = mission["progress"].as_f64().unwrap_or(0.0);
    let state = held();
    let document = state.document.clone().unwrap_or_else(empty_document);
    let has_mission_items = !document.items.is_empty();
    Some(PlanState {
        syncing,
        progress,
        dirty: state.dirty,
        dirty_for_save: state.dirty_for_save,
        contains_items: has_mission_items || has_entries(&document.fence, "polygons") || has_entries(&document.fence, "circles") || has_entries(&document.rally, "points"),
        has_mission_items,
        file: state.file.clone().unwrap_or_default(),
        global_mode: document.global_altitude_mode,
        item_count: document.items.len() as i64,
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

fn param_conversion(param: &Value) -> Option<crate::units::Conversion> {
    let units = param.get("units").and_then(Value::as_str)?;
    list(param, "enumStrings").is_empty().then(|| crate::units::built_in(units).or_else(|| crate::units::cooking(units))).flatten()
}

fn param_fact(param: &Value, raw: f64) -> Value {
    param_fact_with(param, raw, param_conversion(param))
}

fn param_fact_with(param: &Value, raw: f64, conversion: Option<crate::units::Conversion>) -> Value {
    let show = |v: f64| conversion.map_or(v, |c| (c.shown)(v));
    let label = param.get("label").and_then(Value::as_str).unwrap_or("").to_string();
    let decimals = param.get("decimalPlaces").and_then(Value::as_i64).unwrap_or(DEFAULT_DECIMAL_PLACES);
    let raw_units = param.get("units").and_then(Value::as_str).unwrap_or("").to_string();
    let units = conversion.map_or_else(|| raw_units.clone(), |c| c.name.to_string());
    let value = show(raw);
    let default = param.get("default").and_then(Value::as_f64).map(show);
    let min = param.get("min").and_then(Value::as_f64).map(show);
    let max = param.get("max").and_then(Value::as_f64).map(show);
    let labels = list(param, "enumStrings");
    let values: Vec<f64> = list(param, "enumValues").iter().filter_map(|v| v.parse().ok()).collect();
    json!({
        "kind": "fact",
        "name": label,
        "shortDescription": label,
        "value": number_json(value),
        "rawValue": raw,
        "valueString": formatted(value, decimals),
        "units": units,
        "rawUnits": raw_units,
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

fn nan_params(info: &crate::cmdinfo::Command) -> Vec<(u8, Value)> {
    let flag = |p: &Value, key: &str| p.get(key).and_then(Value::as_bool).unwrap_or(false);
    (1..=7u8)
        .filter(|i| !info.hidden.contains(i))
        .filter_map(|i| info.params.get(&i).map(|p| (i, p.clone())))
        .filter(|(_, p)| flag(p, "nanUnchanged") && !flag(p, "advanced"))
        .collect()
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

static RAW_EDIT: std::sync::Mutex<std::collections::BTreeSet<usize>> = std::sync::Mutex::new(std::collections::BTreeSet::new());
const RAW_LABELS: [&str; 7] = ["Param1", "Param2", "Param3", "Param4", "Lat/X", "Lon/Y", "Alt/Z"];
const RAW_DECIMAL_PLACES: i64 = 7;
const MAV_FRAMES: [(&str, i64); 12] = [
    ("MAV_FRAME_GLOBAL", 0),
    ("MAV_FRAME_LOCAL_NED", 1),
    ("MAV_FRAME_MISSION", 2),
    ("MAV_FRAME_GLOBAL_RELATIVE_ALT", 3),
    ("MAV_FRAME_LOCAL_ENU", 4),
    ("MAV_FRAME_GLOBAL_INT", 5),
    ("MAV_FRAME_GLOBAL_RELATIVE_ALT_INT", 6),
    ("MAV_FRAME_LOCAL_OFFSET_NED", 7),
    ("MAV_FRAME_BODY_NED", 8),
    ("MAV_FRAME_BODY_OFFSET_NED", 9),
    ("MAV_FRAME_GLOBAL_TERRAIN_ALT", 10),
    ("MAV_FRAME_GLOBAL_TERRAIN_ALT_INT", 11),
];
const FRIENDLY_FRAMES: [i64; 3] = [0, 3, 10];

fn raw_edits() -> std::sync::MutexGuard<'static, std::collections::BTreeSet<usize>> {
    RAW_EDIT.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn forget_raw_edits() {
    raw_edits().clear();
}

fn shift_raw_edits(before: &[plandoc::Item], after: &[plandoc::Item]) {
    let shared = before.iter().zip(after).take_while(|(old, new)| old == new).count();
    let (gone, added) = (before.len().saturating_sub(after.len()), after.len().saturating_sub(before.len()));
    let mut chosen = raw_edits();
    let moved: std::collections::BTreeSet<usize> = chosen
        .iter()
        .filter(|index| **index <= shared || **index > shared + gone)
        .map(|index| if *index <= shared { *index } else { *index + added - gone })
        .collect();
    *chosen = moved;
}

fn raw_edit_chosen(index: usize) -> bool {
    raw_edits().contains(&index)
}

fn friendly_edit_allowed(simple: &plandoc::Simple, info: Option<&crate::cmdinfo::Command>) -> bool {
    info.is_some_and(|c| c.friendly_edit) && simple.auto_continue && (simple.altitude.is_none() || FRIENDLY_FRAMES.contains(&simple.frame))
}

fn raw_edit(simple: &plandoc::Simple, info: Option<&crate::cmdinfo::Command>, index: usize) -> bool {
    raw_edit_chosen(index) || !friendly_edit_allowed(simple, info)
}

fn choice(label: &str, choices: impl Iterator<Item = (String, i64)>) -> Value {
    let (names, values): (Vec<String>, Vec<String>) = choices.map(|(name, value)| (name, value.to_string())).unzip();
    json!({ "label": label, "enumStrings": names.join(","), "enumValues": values.join(",") })
}

fn raw_fields(simple: &plandoc::Simple, commands: &std::collections::BTreeMap<i64, crate::cmdinfo::Command>, item: &str) -> Vec<Value> {
    let control = |param: Value, value: f64, suffix: String| match crate::control::decode(&param_fact(&param, value), &format!("{item}.{suffix}")) {
        Value::Object(fields) => Value::Object(fields.into_iter().chain([("pathSuffix".to_string(), json!(suffix)), ("group".to_string(), json!("Settings"))]).collect()),
        other => other,
    };
    let text = RAW_LABELS.iter().enumerate().map(|(i, label)| control(json!({ "label": label, "decimalPlaces": RAW_DECIMAL_PLACES }), simple.params[i].unwrap_or(f64::NAN), format!("textFieldFacts.{i}")));
    let command = choice("Command", commands.values().map(|c| (c.raw_name.clone(), c.id)));
    let frame = choice("Frame", MAV_FRAMES.iter().map(|(name, value)| ((*name).to_string(), *value)));
    [control(command, simple.command as f64, "comboboxFacts.0".to_string()), control(frame, simple.frame as f64, "comboboxFacts.1".to_string())].into_iter().chain(text).collect()
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
    let nan = nan_params(info);
    let optional = nan.iter().map(|(i, p)| (*i, p)).enumerate().map(build("nanFacts")).map(|control| match control {
        Value::Object(map) => Value::Object(map.into_iter().chain([("optional".to_string(), json!(true))]).collect()),
        other => other,
    });
    combo_params.enumerate().map(build("comboboxFacts")).chain(text_params.enumerate().map(build("textFieldFacts"))).chain(optional).collect()
}

const HOLD_PARAM: u8 = 1;

fn hold_field(simple: &plandoc::Simple, info: Option<&crate::cmdinfo::Command>, raw: bool, item: &str) -> Value {
    let label = info.and_then(|c| c.params.get(&HOLD_PARAM)).and_then(|p| p.get("label")).and_then(Value::as_str);
    match (raw, simple.command, label) {
        (false, 16, Some("Hold")) => json!({ "value": simple.params[usize::from(HOLD_PARAM) - 1].filter(|v| v.is_finite()).unwrap_or(0.0), "units": "s", "path": format!("{item}.hold") }),
        _ => Value::Null,
    }
}

const FLIGHT_SPEED_USER_RANGE: (f64, f64) = (0.0, 30.0);

fn speed_section(document: &Document, index: usize, sections: &[plandoc::Simple], offered: bool, hover: f64, cruise: f64) -> Value {
    let available = offered && matches!(plandoc::vehicle_class(document.vehicle_type), crate::cmdinfo::VehicleClass::MultiRotor | crate::cmdinfo::VehicleClass::FixedWing);
    let item = format!("{ITEM_ROOT}.{index}.speedSection");
    let specified = sections.iter().find(|s| s.command == 178).and_then(|s| s.params[1]);
    let default = match (available, plandoc::vehicle_class(document.vehicle_type)) {
        (true, _) => speed_in_force(document, index.saturating_sub(1), hover, cruise),
        (false, crate::cmdinfo::VehicleClass::MultiRotor) => hover,
        (false, _) => cruise,
    };
    let speed = crate::units::cooking("m/s");
    json!({
        "available": available,
        "specified": specified.is_some(),
        "value": speed.map_or(specified.unwrap_or(default), |c| (c.shown)(specified.unwrap_or(default))),
        "units": speed.map_or("m/s", |c| c.name),
        "slider": crate::read::user_slider(&json!({ "userMin": speed.map_or(FLIGHT_SPEED_USER_RANGE.0, |c| (c.shown)(FLIGHT_SPEED_USER_RANGE.0)), "userMax": speed.map_or(FLIGHT_SPEED_USER_RANGE.1, |c| (c.shown)(FLIGHT_SPEED_USER_RANGE.1)), "decimalPlaces": 1 })),
        "path": format!("{item}.flightSpeed"),
        "specifyPath": format!("{item}.specifyFlightSpeed"),
    })
}

pub const LAUNCH_ALTITUDE: &str = "plannedHomePositionAltitude";

pub fn launch_altitude_field(shown: f64, units: &str, path: &str) -> Value {
    let fact = json!({ "kind": "fact", "name": LAUNCH_ALTITUDE, "shortDescription": "Altitude", "longDescription": "Actual position is set by the vehicle at flight time.", "type": "double", "value": shown, "rawValue": shown, "valueString": format!("{shown:.1}"), "units": units, "decimalPlaces": 1, "property": LAUNCH_ALTITUDE });
    crate::control::decode(&fact, path)
}

fn vehicle_has_home(backend: &dyn Backend) -> bool {
    crate::read::flag(&crate::read::object(&backend.get("vehicle.homePosition")), "valid")
}

pub fn mission_speed_section(backend: &dyn Backend) -> Value {
    let document = held().document.clone().unwrap_or_else(empty_document);
    let speed = |name: &str, default: f64| crate::read::value_number(&backend.get(&format!("settings.appSettings.{name}.rawValue"))).unwrap_or(default);
    speed_section(&document, 0, &document.settings_sections, true, speed("offlineEditingHoverSpeed", 5.0), speed("offlineEditingCruiseSpeed", 15.0))
}

pub fn item_facts(backend: &dyn Backend, index: usize) -> Value {
    let document = held().document.clone().unwrap_or_else(empty_document);
    let speed = |name: &str, default: f64| crate::read::value_number(&backend.get(&format!("settings.appSettings.{name}.rawValue"))).unwrap_or(default);
    let (vertical, horizontal) = (crate::read::Unit::vertical(backend), crate::read::Unit::horizontal(backend));
    let facts = document_facts(&document, index, speed("offlineEditingHoverSpeed", 5.0), speed("offlineEditingCruiseSpeed", 15.0), (&vertical, &horizontal));
    let launch = (index == 0 && !vehicle_has_home(backend)).then(|| document.home.map(|home| launch_altitude_field(vertical.show(home[2]), &vertical.name, &format!("{ITEM_ROOT}.0.{LAUNCH_ALTITUDE}")))).flatten();
    let read = crate::missionitems::document_reads(&document, -1).ok().and_then(|reads| reads.get(index).cloned()).unwrap_or_default();
    let wizard = held().wizard == Some(index) && read.get("isTakeoffItem").and_then(Value::as_bool) == Some(true);
    let wizard_rows = crate::itemfacts::wizard_info(wizard, plandoc::vehicle_class(document.vehicle_type) == crate::cmdinfo::VehicleClass::Vtol);
    match with_previous_coordinate(facts, plandoc::previous_coordinate(&document, index as i64)) {
        Value::Object(map) => crate::itemfacts::without_hidden_mission_speed(
            Value::Object(map.into_iter().chain([("launchAltitude".to_string(), launch.unwrap_or(Value::Null))]).chain(crate::itemfacts::command_info(&read)).chain(wizard_rows).collect()),
            index,
            plandoc::vehicle_class(document.vehicle_type) == crate::cmdinfo::VehicleClass::Vtol,
            plandoc::firmware(document.firmware_type) == crate::cmdinfo::Firmware::ArduPilot,
        ),
        other => other,
    }
}

pub fn with_previous_coordinate(facts: Value, previous: Option<(f64, f64)>) -> Value {
    match facts {
        Value::Object(fields) if fields.get("available") == Some(&json!(true)) => {
            Value::Object(fields.into_iter().chain([("previousCoordinate".to_string(), previous.map_or(Value::Null, |(latitude, longitude)| json!({ "latitude": latitude, "longitude": longitude })))]).collect())
        }
        other => other,
    }
}

const CMD_NAV_TAKEOFF: i64 = 22;
const CMD_NAV_VTOL_TAKEOFF_ID: i64 = 84;

fn takeoff_heading(doc: &Document) -> Option<f64> {
    let home = doc.home?;
    let plandoc::Item::Simple(takeoff) = doc.items.first()? else { return None };
    [CMD_NAV_TAKEOFF, CMD_NAV_VTOL_TAKEOFF_ID].contains(&takeoff.command).then_some(())?;
    let at = takeoff.params[4].zip(takeoff.params[5])?;
    Some(crate::surveygrid::azimuth_to((home[0], home[1]), at))
}

fn entry_row(kind: &str, pattern: &Value, item: &str) -> Value {
    let path = format!("{item}.rotateEntryPoint");
    match kind {
        "StructureScan" => json!({ "label": "Entry vertex", "value": (crate::structurescan::entry_vertex(pattern) + 1).to_string(), "path": path }),
        _ => crate::surveydoc::entry_point_name(kind, crate::surveydoc::entry_point(kind, pattern)).map_or(Value::Null, |name| json!({ "label": "Start from", "value": name, "path": path })),
    }
}

fn shape_complete(kind: &str, pattern: &Value) -> bool {
    let (key, least) = if kind == "CorridorScan" { ("polyline", 2) } else { ("polygon", 3) };
    pattern.get(key).and_then(Value::as_array).is_some_and(|vertices| vertices.len() >= least)
}

fn document_facts(document: &Document, index: usize, hover: f64, cruise: f64, units: (&crate::read::Unit, &crate::read::Unit)) -> Value {
    let document = document.clone();
    let commands = crate::cmdinfo::tree(plandoc::firmware(document.firmware_type), plandoc::command_class_at(&document, index.saturating_sub(1)));
    let item = format!("{ITEM_ROOT}.{index}");
    let base = |simple: bool, fields: Vec<Value>, section: Value, mode: Option<i64>| {
        json!({ "kind": "object", "class": "ItemFacts", "available": true, "index": index, "simple": simple, "fields": fields, "camera": Value::Null, "speedSection": section, "altitudeMode": mode })
    };
    match index.checked_sub(1).map(|i| document.items.get(i)) {
        None => base(false, Vec::new(), speed_section(&document, index, &document.settings_sections, true, hover, cruise), None),
        Some(Some(plandoc::Item::Simple(s))) => {
            let info = commands.get(&s.command);
            let raw = raw_edit(s, info, index);
            let facts = base(
                true,
                if raw { raw_fields(s, &commands, &item) } else { simple_fields(s, &commands, &item) },
                speed_section(&document, index, &s.sections, s.command == 16, hover, cruise),
                Some(s.altitude.as_ref().map_or(crate::altitudemodes::RELATIVE, |a| a.mode)),
            );
            match facts {
                Value::Object(fields) => {
                    let land = info.is_some_and(|c| c.is_land);
                    let hint = s.altitude.as_ref().and_then(|a| {
                        crate::itemfacts::altitude_hint(land, Some(a.mode), a.amsl_above_terrain.filter(|v| v.is_finite()).map(|metres| units.0.label(metres)))
                    });
                    Value::Object(fields.into_iter().chain([("rawEdit".to_string(), json!(raw)), ("friendlyEditAllowed".to_string(), json!(friendly_edit_allowed(s, info))), ("altitudeHint".to_string(), json!(hint)), ("hold".to_string(), hold_field(s, info, raw, &item))]).collect())
                }
                other => other,
            }
        }
        Some(Some(plandoc::Item::Complex { kind, json: survey, .. })) if kind == "survey" || kind == "CorridorScan" || kind == "StructureScan" => {
            let units = crate::surveydoc::Units { vertical: units.0, horizontal: units.1 };
            json!({ "kind": "object", "class": "ItemFacts", "available": true, "index": index, "simple": false, "fields": crate::surveydoc::fields(survey, &item, plandoc::vehicle_class(document.vehicle_type), &units), "camera": crate::surveydoc::camera(survey, &item, &units, plandoc::firmware(document.firmware_type) != crate::cmdinfo::Firmware::Px4), "speedSection": Value::Null, "altitudeMode": Value::Null, "presetKind": crate::presets::settings_group(kind).map(|_| kind.clone()), "areaHelp": crate::itemfacts::area_help(kind, shape_complete(kind, survey)), "entryPoint": entry_row(kind, survey, &item) })
        }
        Some(Some(plandoc::Item::Complex { kind, json: pattern, .. })) if crate::landingpattern::is_landing(kind) => {
            let units = crate::surveydoc::Units { vertical: units.0, horizontal: units.1 };
            json!({ "kind": "object", "class": "ItemFacts", "available": true, "index": index, "simple": false, "fields": crate::landingpattern::fields(pattern, &item, &units), "camera": Value::Null, "speedSection": Value::Null, "altitudeMode": Value::Null, "landing": true, "landingNotes": crate::landingpattern::notes(kind == crate::landingpattern::VTOL_PATTERN), "wizardMode": pattern.get(crate::landingpattern::WIZARD).and_then(Value::as_bool).unwrap_or(false), "wizardText": crate::landingpattern::wizard_text(pattern), "altitudesAreRelative": pattern.get("altitudesAreRelative").and_then(Value::as_bool).unwrap_or(true) })
        }
        Some(Some(plandoc::Item::Complex { kind, .. })) => json!({ "kind": "object", "class": "ItemFacts", "available": false, "index": index, "reason": format!("The core cannot edit a {kind} item yet.") }),
        Some(None) => json!({ "kind": "object", "class": "ItemFacts", "available": false, "index": index }),
    }
}

pub fn fly_fence_and_rally() -> Option<(Value, Value)> {
    if !enabled() {
        return None;
    }
    let snapshot = crate::hub::lock().active().map(|v| v.mission_snapshot());
    Some(match snapshot {
        Some(snapshot) => (fence_from(&snapshot["fence"]), json!({ "version": 2, "points": snapshot["rally"]["points"] })),
        None => (empty_document().fence, empty_document().rally),
    })
}

pub fn fence_and_rally() -> Option<(Value, Value)> {
    if !enabled() {
        return None;
    }
    let document = held().document.clone().unwrap_or_else(empty_document);
    Some((document.fence, document.rally))
}

fn specified_speed(simple: &plandoc::Simple) -> Option<f64> {
    (simple.command == 178).then_some(simple.params[1]).flatten().filter(|speed| *speed > 0.0)
}

fn speed_in_force(document: &Document, before: usize, hover: f64, cruise: f64) -> f64 {
    let hovering = matches!(plandoc::vehicle_class(document.vehicle_type), crate::cmdinfo::VehicleClass::MultiRotor | crate::cmdinfo::VehicleClass::Vtol);
    let start = if hovering { hover } else { cruise };
    let settings = document.settings_sections.iter().filter_map(specified_speed);
    let items = document.items.iter().take(before).flat_map(|item| match item {
        plandoc::Item::Simple(s) => std::iter::once(s).chain(s.sections.iter()).filter_map(specified_speed).collect::<Vec<_>>(),
        plandoc::Item::Complex { .. } => Vec::new(),
    });
    settings.chain(items).last().unwrap_or(start)
}

const BREACH_RETURN_META: &str = include_str!("../../src/MissionManager/BreachReturn.FactMetaData.json");
fn breach_altitude_now() -> Option<f64> {
    let (fence, pending) = {
        let state = held();
        (state.document.as_ref().map(|d| d.fence.clone()).unwrap_or(Value::Null), state.breach_altitude)
    };
    fence.get("breachReturn").and_then(Value::as_array).and_then(|b| b.get(2)).and_then(Value::as_f64)
        .or(pending)
        .or_else(|| crate::settingsstore::raw_setting(DEFAULT_ALTITUDE).and_then(|v| v.as_f64()))
}

pub fn breach_altitude_fact() -> Option<Value> {
    if !enabled() {
        return None;
    }
    let altitude = breach_altitude_now()?;
    let meta = crate::factmeta::from_file(BREACH_RETURN_META).ok()?.remove("Altitude")?;
    let mut fact = crate::settingsstore::fact_json(&meta, &json!(altitude), crate::units::cooking("vertical m"));
    ["defaultValueString", "userVisible", "visible"].iter().for_each(|key| fact[*key] = Value::Null);
    Some(fact)
}

const CAMERA_INTERVAL_DISTANCE: &str = "cameraPhotoIntervalDistance";

fn cooked_camera_distance(section: Value, cooking: Option<crate::units::Conversion>) -> Value {
    let Some(conversion) = cooking else { return section };
    let cook = |fact: &Value| match fact["property"] == CAMERA_INTERVAL_DISTANCE {
        true => {
            let shown = fact["value"].as_f64().map(conversion.shown);
            let text = shown.map(|v| format!("{v:.1}"));
            Value::Object(fact.as_object().cloned().unwrap_or_default().into_iter().chain([
                ("value".to_string(), json!(shown)),
                ("valueString".to_string(), json!(text)),
                ("enumOrValueString".to_string(), json!(text)),
                ("units".to_string(), json!(conversion.name)),
            ]).collect())
        }
        false => fact.clone(),
    };
    match section {
        Value::Object(map) => Value::Object(map.into_iter().map(|(key, value)| match (key.as_str(), value) {
            ("facts", Value::Array(facts)) => (key, Value::Array(facts.iter().map(cook).collect())),
            (_, value) => (key, value),
        }).collect()),
        other => other,
    }
}

pub fn camera_section(index: usize) -> Option<Value> {
    if !enabled() {
        return None;
    }
    let document = held().document.clone().unwrap_or_else(empty_document);
    let supports_mode = crate::cmdinfo::tree(plandoc::firmware(document.firmware_type), crate::cmdinfo::VehicleClass::Generic).contains_key(&530);
    let with_support = |section: Value| match section {
        Value::Object(map) => {
            let specified = map.get("specifyCameraMode").and_then(Value::as_bool).unwrap_or(false);
            Value::Object(map.into_iter().chain([("cameraModeSupported".to_string(), json!(specified || supports_mode))]).collect())
        }
        other => other,
    };
    let shown = |sections: &[plandoc::Simple]| with_support(cooked_camera_distance(plandoc::camera_section(sections), crate::units::cooking("m")));
    Some(match index.checked_sub(1).map(|at| document.items.get(at)) {
        None => shown(&document.settings_sections),
        Some(Some(plandoc::Item::Simple(simple))) if simple.command == plandoc::CMD_NAV_WAYPOINT => shown(&simple.sections),
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
            let (bottom, top) = crate::structurescan::flight_alts(&plan);
            let mut structure = stats(shots, per_second(plan.adjusted_side), 0.0, crate::structurescan::scan_distance(&flight, &plan));
            structure["layers"] = json!(plan.layers);
            structure["bottomFlightAlt"] = json!(bottom);
            structure["topFlightAlt"] = json!(top);
            (structure, calc_facts(&item["CameraCalc"]))
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
