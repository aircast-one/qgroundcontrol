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
pub const CONVERT_TO_VEHICLE: &str = "core.plan.convertToVehicle";
pub const MOVE_ITEM: &str = "core.plan.moveItem";
pub const PLACE_DROP: &str = "core.plan.placeDrop";
pub const REMOVE_DROP: &str = "core.plan.removeDrop";
pub const SET_DROP_RELEASE: &str = "core.plan.setDropRelease";
pub const MOVE_ACTION: &str = "core.plan.moveAction";
pub const SET_WAIT: &str = "core.plan.setWait";
pub const DRAFT_DEPS: &[&str] = &[CHANGED];
pub const RESTORE_DRAFT: &str = "core.plan.restoreDraft";
const ACTIONS: &[&str] = &[
    RESTORE_DRAFT,
    CONVERT_TO_VEHICLE,
    MOVE_ITEM,
    PLACE_DROP,
    REMOVE_DROP,
    SET_DROP_RELEASE,
    MOVE_ACTION,
    SET_WAIT,
    OPEN,
    SAVE,
    SEND,
    FETCH,
    STATUS,
    CORE_INSERT_WAYPOINT,
    CORE_REMOVE,
    CORE_INSERT_LAND,
    CORE_SET_COMMAND,
    CORE_SET_ALTITUDE,
    CORE_INSERT_TAKEOFF,
    CORE_SET_ALTITUDE_MODE,
    CORE_ITEMS,
    APPLY_DEFAULT_ALTITUDE,
    DISMISS_ALTITUDE_PROMPT,
    LOAD_VEHICLE_PLAN,
    KEEP_CURRENT_PLAN,
];
const CMD_NAV_LAND: i64 = 21;
const CMD_NAV_VTOL_LAND: i64 = 85;
const APPLY_ALTITUDE_TITLE: &str = "Change every waypoint to the new route altitude?";
const APPLY_ALTITUDE_TEXT: &str = "New waypoints use the new altitude. The waypoints already placed keep their own altitudes unless you change them all.";
static ASK_APPLY_ALTITUDE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
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
    note: Option<String>,
    note_id: u64,
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
            "connected": !offline,
            "dirty": dirty,
        }),
    }
}

pub fn vehicle_change_prompt() -> Value {
    let (prompt, dirty) = {
        let state = held();
        (state.vehicle_prompt, state.dirty_for_save)
    };
    let stored = match prompt {
        Some(false) => stored_rows(),
        _ => None,
    };
    match vehicle_change_prompt_for(prompt, dirty) {
        Value::Object(mut fields) => {
            fields.insert("aircraftItems".to_string(), json!(stored));
            Value::Object(fields)
        }
        other => other,
    }
}

const UNDO_DEPTH: usize = 100;
const UNDO_COALESCE_MS: u64 = 500;

fn remember(state: &mut Held, before: Option<Document>, now: u64) {
    let burst =
        now.saturating_sub(state.last_change_ms) < UNDO_COALESCE_MS && !state.undo.is_empty();
    remember_as(state, before, now, burst);
}

fn remember_step(state: &mut Held, before: Option<Document>) {
    remember_as(state, before, 0, false);
}

fn remember_as(state: &mut Held, before: Option<Document>, now: u64, burst: bool) {
    state.last_change_ms = now;
    if burst || before.is_none() || before == state.document {
        return;
    }
    state.undo = state
        .undo
        .iter()
        .cloned()
        .chain(before)
        .rev()
        .take(UNDO_DEPTH)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    state.redo.clear();
}

fn settle_uploaded(state: &mut Held) {
    state.dirty = false;
    state.clean = state.document.clone();
}

fn keep_for_this_vehicle(state: &mut Held) {
    state.vehicle_prompt = None;
    state.dirty = true;
    state.clean = None;
}

fn settle_clean(state: &mut Held) {
    settle_uploaded(state);
    state.dirty_for_save = false;
}

pub const CHANGED: &str = "core.plan@changed";

pub static ON_CHANGE: std::sync::Mutex<Option<std::sync::Arc<dyn Fn() + Send + Sync>>> =
    std::sync::Mutex::new(None);

#[cfg(not(test))]
const ON_WITHOUT_SWITCH: bool = true;
#[cfg(test)]
const ON_WITHOUT_SWITCH: bool = false;

static ENABLED: LazyLock<bool> =
    LazyLock::new(|| std::env::var("QGC_CORE_PLAN").map_or(ON_WITHOUT_SWITCH, |v| v == "1"));

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

static REVISION: Mutex<(u64, u64)> = Mutex::new((0, 0));

fn revision_of(document: &Document) -> u64 {
    let fingerprint = {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&plandoc::save(document).to_string(), &mut hasher);
        std::hash::Hasher::finish(&hasher)
    };
    let mut held = REVISION.lock().unwrap_or_else(PoisonError::into_inner);
    if held.0 != fingerprint {
        *held = (fingerprint, held.1 + 1);
    }
    held.1
}

pub fn revision() -> u64 {
    let document = held().document.clone().unwrap_or_else(empty_document);
    revision_of(&document)
}

fn changed() {
    let notify = ON_CHANGE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
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
    serde_json::from_str::<Value>(args)
        .ok()?
        .get(0)?
        .as_str()
        .map(str::to_string)
}

pub fn act(backend: &dyn Backend, path: &str, args: &str) -> Value {
    match path {
        OPEN => first_text(args).map_or_else(
            || refused("Open needs the path of a .plan file."),
            |file| open(backend, &file),
        ),
        SAVE => first_text(args).map_or_else(
            || refused("Save needs a path to write the plan to."),
            |file| save(&file),
        ),
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
                state.shown_vehicle = state
                    .vehicle_prompt
                    .filter(|offline| *offline)
                    .map(|_| NO_VEHICLE_SHOWN);
                state.vehicle_prompt = None;
                state.dirty = false;
                state.dirty_for_save = false;
            }
            follow_vehicle();
            changed();
            json!({ "ok": true })
        }
        KEEP_CURRENT_PLAN => {
            keep_for_this_vehicle(&mut held());
            changed();
            json!({ "ok": true })
        }
        DISMISS_ALTITUDE_PROMPT => {
            ASK_APPLY_ALTITUDE.store(false, std::sync::atomic::Ordering::Relaxed);
            changed();
            json!({ "ok": true })
        }
        CORE_REMOVE => remove(args),
        MOVE_ITEM => move_item(args),
        PLACE_DROP => place_drop(backend, args),
        REMOVE_DROP => remove_drop(args),
        SET_DROP_RELEASE => set_drop_release(backend, args),
        MOVE_ACTION => move_action(args),
        SET_WAIT => set_wait(args),
        RESTORE_DRAFT => first_text(args).map_or_else(
            || refused("Restoring needs the plan that was kept."),
            |text| restore_draft(&text),
        ),
        CONVERT_TO_VEHICLE => convert_to_vehicle(),
        _ => refused(format!("{path} is not a plan action the core performs")),
    }
}

fn held() -> std::sync::MutexGuard<'static, Held> {
    HELD.lock().unwrap_or_else(PoisonError::into_inner)
}

fn home_of(document: Option<&Document>) -> Option<(f64, f64)> {
    document.and_then(|d| d.home).map(|h| (h[0], h[1]))
}

static HOME_GROUND_PENDING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn home_ground_pending() -> bool {
    HOME_GROUND_PENDING.load(std::sync::atomic::Ordering::Relaxed) > 0
}

fn settle_home_on_terrain(before: Option<(f64, f64)>) {
    let Some((latitude, longitude)) =
        home_of(held().document.as_ref()).filter(|now| Some(*now) != before)
    else {
        return;
    };
    HOME_GROUND_PENDING.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    crate::terrainservice::height_then(latitude, longitude, move |ground| {
        if let Some(ground) = ground {
            let mut state = held();
            if home_of(state.document.as_ref()) == Some((latitude, longitude)) {
                state.document = state.document.take().map(|d| Document {
                    home: d.home.map(|h| [h[0], h[1], ground]),
                    ..d
                });
            }
        }
        HOME_GROUND_PENDING.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        changed();
    });
}

fn edit(change: impl FnOnce(&Document) -> Result<Document, String>) -> Value {
    edit_selecting(change, None)
}

fn edit_selecting(
    change: impl FnOnce(&Document) -> Result<Document, String>,
    selected: Option<i64>,
) -> Value {
    edit_with(|doc| {
        change(doc).map(|document| Edited {
            document,
            selected,
            note: None,
            plain: None,
            own_step: false,
        })
    })
}

struct Edited {
    document: Document,
    selected: Option<i64>,
    note: Option<String>,
    plain: Option<usize>,
    own_step: bool,
}

fn edit_with(change: impl FnOnce(&Document) -> Result<Edited, String>) -> Value {
    let types = offline_types();
    let (answer, before) = {
        let mut state = held();
        let Some(current) = state
            .document
            .clone()
            .map(|document| tracked(document, types))
        else {
            return refused("There is no plan to edit.");
        };
        let before = home_of(Some(&current));
        let gate = plandoc::firmware(current.firmware_type) == crate::cmdinfo::Firmware::Px4
            && crate::settingsstore::raw_setting("settings.planViewSettings.useConditionGate")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        crate::surveyitems::CONDITION_GATE_SUPPORTED
            .store(gate, std::sync::atomic::Ordering::Relaxed);
        match change(&current) {
            Ok(edited) => {
                let changed = plandoc::above_terrain(&edited.document, &terrain_height);
                let count = changed.items.len();
                shift_raw_edits(&current.items, &changed.items);
                if let Some(index) = edited.plain {
                    raw_edits().remove(&index);
                }
                let previous = state.document.replace(tracked(changed, types));
                match edited.own_step {
                    true => remember_step(&mut state, previous),
                    false => remember(&mut state, previous, crate::hub::now_ms()),
                }
                state.dirty = true;
                state.dirty_for_save = true;
                state.selected = edited.selected.unwrap_or(state.selected);
                state.note_id += u64::from(edited.note.is_some());
                state.note = edited.note;
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
    *MAP_CENTER_HINT
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some((latitude, longitude));
}

fn edit_defaults(backend: &dyn Backend) -> Option<plandoc::EditDefaults> {
    let map_center = *MAP_CENTER_HINT
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let vtol_transition_distance = crate::read::value_number(
        &backend.value("settings.planViewSettings.vtolTransitionDistance.rawValue"),
    )
    .unwrap_or(plandoc::VTOL_TRANSITION_DISTANCE_DEFAULT);
    crate::read::value_number(&backend.value(&format!("{DEFAULT_ALTITUDE}.rawValue"))).map(
        |mission_item_altitude| plandoc::EditDefaults {
            mission_item_altitude,
            map_center,
            vtol_transition_distance,
        },
    )
}

fn insert_at(backend: &dyn Backend, args: &str, land: bool) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let number = |i: usize| {
        given
            .get(i)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
    };
    let (Some(latitude), Some(longitude)) = (number(0), number(1)) else {
        return refused("An item needs a latitude and a longitude.");
    };
    let index = given.get(2).and_then(Value::as_i64).unwrap_or(-1);
    let Some(defaults) = edit_defaults(backend) else {
        return refused("The default mission item altitude is not known.");
    };
    edit(|doc| match land {
        true => plandoc::insert_land(doc, latitude, longitude, index, &defaults),
        false => Ok(plandoc::insert_waypoint(
            doc, latitude, longitude, index, &defaults,
        )),
    })
}

fn insert_takeoff(backend: &dyn Backend, args: &str) -> Value {
    let index = serde_json::from_str::<Value>(args)
        .ok()
        .and_then(|v| v.get(0).and_then(Value::as_i64))
        .unwrap_or(-1);
    let Some(defaults) = edit_defaults(backend) else {
        return refused("The default mission item altitude is not known.");
    };
    edit(|doc| plandoc::insert_takeoff(doc, index, &defaults))
}

fn remember_patterns<'a>(
    backend: &dyn Backend,
    vehicle_type: i64,
    items: impl Iterator<Item = &'a plandoc::Item>,
) {
    let multirotor =
        plandoc::vehicle_class(vehicle_type) == crate::cmdinfo::VehicleClass::MultiRotor;
    items
        .filter_map(|item| match item {
            plandoc::Item::Complex { json, .. } => Some(crate::surveydoc::changed_remembered(
                json,
                multirotor,
                &crate::settingsstore::stored_text,
            )),
            plandoc::Item::Simple(_) => None,
        })
        .flatten()
        .for_each(|(key, value)| backend.remember_setting(&key, &value));
}

fn remember_item(backend: &dyn Backend, visual_index: i64) {
    let Some(document) = held().document.clone() else {
        return;
    };
    let at = usize::try_from(visual_index - 1).ok();
    remember_patterns(
        backend,
        document.vehicle_type,
        at.and_then(|i| document.items.get(i)).into_iter(),
    );
}

fn insert_landing(backend: &dyn Backend, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let number = |i: usize| {
        given
            .get(i)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
    };
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
            transition_distance: crate::read::value_number(
                &backend.value("settings.planViewSettings.vtolTransitionDistance.rawValue"),
            ),
        });
        let kind = if vtol {
            crate::landingpattern::VTOL_PATTERN
        } else {
            crate::landingpattern::FIXED_WING_PATTERN
        };
        Ok(plandoc::insert_complex(
            doc,
            kind,
            built,
            (latitude, longitude),
            index,
        ))
    })
}

fn insert_structure(backend: &dyn Backend, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let number = |i: usize| {
        given
            .get(i)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
    };
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
            multirotor: plandoc::vehicle_class(doc.vehicle_type)
                == crate::cmdinfo::VehicleClass::MultiRotor,
            alternates: false,
            default_altitude: defaults.mission_item_altitude,
            distance_mode,
            previous_mode: None,
        });
        Ok(plandoc::with_survey_camera(plandoc::insert_complex(
            doc,
            "StructureScan",
            built,
            (latitude, longitude),
            index,
        )))
    })
}

fn insert_roi(backend: &dyn Backend, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let number = |i: usize| {
        given
            .get(i)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
    };
    let (Some(latitude), Some(longitude)) = (number(0), number(1)) else {
        return refused("A region of interest needs a latitude and a longitude.");
    };
    let index = given.get(2).and_then(Value::as_i64).unwrap_or(-1);
    let Some(defaults) = edit_defaults(backend) else {
        return refused("The default mission item altitude is not known.");
    };
    edit(|doc| {
        Ok(plandoc::insert_roi(
            doc, latitude, longitude, index, &defaults,
        ))
    })
}

fn insert_drop(backend: &dyn Backend, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let index = given.get(2).and_then(Value::as_i64).unwrap_or(-1);
    let Some(defaults) = edit_defaults(backend) else {
        return refused("The default mission item altitude is not known.");
    };
    edit(|doc| {
        plandoc::insert_drop(
            doc,
            index,
            &defaults,
            &crate::payloadcheck::servo_bays(
                plandoc::firmware(doc.firmware_type) == crate::cmdinfo::Firmware::ArduPilot,
            ),
        )
    })
}

fn plan_servos(doc: &Document) -> Vec<crate::payloadcheck::Servo> {
    crate::payloadcheck::servo_bays(
        plandoc::firmware(doc.firmware_type) == crate::cmdinfo::Firmware::ArduPilot,
    )
}

fn waypoint_given(given: &Value) -> Option<usize> {
    given
        .get(0)
        .and_then(Value::as_u64)
        .and_then(|v| usize::try_from(v).ok())
}

fn place_drop(backend: &dyn Backend, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let Some(waypoint) = waypoint_given(&given) else {
        return refused(plandoc::NOT_A_WAYPOINT);
    };
    let wait = match given.get(1) {
        None | Some(Value::Null) => Ok(None),
        Some(wait) => wait
            .as_f64()
            .filter(|seconds| seconds.is_finite())
            .map(Some)
            .ok_or(plandoc::WAIT_OUT_OF_RANGE),
    };
    let wait = match wait {
        Ok(wait) => wait,
        Err(reason) => return refused(reason),
    };
    let Some(defaults) = edit_defaults(backend) else {
        return refused("The default mission item altitude is not known.");
    };
    edit(|doc| plandoc::place_drop(doc, waypoint, wait, &defaults, &plan_servos(doc)))
}

fn remove_drop(args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    match waypoint_given(&given) {
        Some(waypoint) => edit(|doc| plandoc::remove_drop(doc, waypoint, &plan_servos(doc))),
        None => refused(plandoc::NOT_A_WAYPOINT),
    }
}

fn set_drop_release(backend: &dyn Backend, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let Some(waypoint) = waypoint_given(&given) else {
        return refused(plandoc::NOT_A_WAYPOINT);
    };
    let wanted = given.get(1).cloned().unwrap_or(Value::Null);
    let number = |key: &str| wanted.get(key).and_then(Value::as_f64);
    let (servo, output, pwm) = (
        wanted.get("servo").and_then(Value::as_bool),
        number("output"),
        number("pwm"),
    );
    let Some(defaults) = edit_defaults(backend) else {
        return refused("The default mission item altitude is not known.");
    };
    let kept = std::cell::RefCell::new(Vec::new());
    let answer = edit(|doc| {
        let was = plan_servos(doc);
        let retyped = match servo.filter(|servo| *servo == was.is_empty()) {
            Some(servo) => {
                let now = crate::payloadcheck::configured_bays(servo);
                plandoc::replace_drop(doc, waypoint, &defaults, &was, &now).map(|doc| (doc, now))
            }
            None => Ok((doc.clone(), was)),
        }?;
        let (document, servos) = retyped;
        let edited = match output.is_some() || pwm.is_some() {
            true => plandoc::set_drop_servo(&document, waypoint, output, pwm, &servos),
            false => Ok(document.clone()),
        }?;
        let release_at = |doc: &Document, servos: &[crate::payloadcheck::Servo]| {
            plandoc::drop_after(doc, waypoint, servos)
                .flatten()
                .and_then(|drop| plandoc::drop_release(doc, drop, servos))
        };
        let bay = release_at(&document, &servos)
            .and_then(|release| release.bay)
            .or_else(|| (servos.len() == 1).then_some(1));
        *kept.borrow_mut() = crate::payloadcheck::remembered(
            servo,
            release_at(&edited, &servos)
                .zip(bay)
                .map(|(release, bay)| (bay, release.output, release.pwm)),
        );
        Ok(edited)
    });
    if answer["ok"] == true {
        crate::settingsstore::set_raw_together(
            &kept
                .borrow()
                .iter()
                .map(|(path, value)| (path.as_str(), value.clone()))
                .collect::<Vec<_>>(),
        );
        changed();
    }
    answer
}

fn insert_scan(backend: &dyn Backend, args: &str, corridor: bool) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let number = |i: usize| {
        given
            .get(i)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
    };
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
        let previous_mode = (doc.global_altitude_mode == crate::altitudemodes::MIXED && !corridor)
            .then(|| plandoc::previous_altitude_mode(doc, index))
            .flatten();
        let fresh = crate::surveydoc::Fresh {
            center: (latitude, longitude),
            remembered: &crate::settingsstore::stored_text,
            multirotor: class == crate::cmdinfo::VehicleClass::MultiRotor,
            alternates: matches!(
                class,
                crate::cmdinfo::VehicleClass::FixedWing | crate::cmdinfo::VehicleClass::Vtol
            ),
            default_altitude: defaults.mission_item_altitude,
            distance_mode,
            previous_mode,
        };
        let (kind, built) = match corridor {
            true => ("CorridorScan", crate::surveydoc::fresh_corridor(&fresh)),
            false => ("survey", crate::surveydoc::fresh(&fresh)),
        };
        Ok(plandoc::with_survey_camera(plandoc::insert_complex(
            doc,
            kind,
            with_flight_speed(doc, built),
            (latitude, longitude),
            index,
        )))
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
    let calc_mode = item
        .pointer("/TransectStyleComplexItem/CameraCalc/DistanceMode")
        .and_then(Value::as_i64);
    if calc_mode != Some(crate::altitudemodes::CALC_ABOVE_TERRAIN) {
        return item;
    }
    let mut stamped = item;
    stamped["TransectStyleComplexItem"][crate::surveydoc::TERRAIN_FLIGHT_SPEED] =
        json!(flight_speed(doc));
    crate::surveydoc::regenerate_item(&stamped)
}

const WAITING_ON_TERRAIN: &str =
    "Plan is waiting on terrain data from server for correct altitude values.";

fn terrain_height(latitude: f64, longitude: f64) -> Option<f64> {
    crate::terrainservice::cached_height(latitude, longitude).flatten()
}

fn waiting_on_terrain(document: &Document) -> bool {
    plandoc::awaiting_terrain(document) || document.items.iter().any(|item| matches!(item, plandoc::Item::Complex { json, .. } if crate::surveydoc::waiting_for_terrain(json)))
}

pub fn terrain_arrived() {
    let refreshed = {
        let mut state = held();
        let Some(current) = state.document.clone().filter(waiting_on_terrain) else {
            return;
        };
        let items = current
            .items
            .iter()
            .map(|item| match item {
                plandoc::Item::Complex {
                    kind,
                    json,
                    item_count,
                } if crate::surveydoc::waiting_for_terrain(json) => {
                    let regenerated = crate::surveydoc::regenerate_item(json);
                    plandoc::Item::Complex {
                        kind: kind.clone(),
                        item_count: plandoc::complex_count(kind, &regenerated)
                            .unwrap_or(*item_count),
                        json: regenerated,
                    }
                }
                other => other.clone(),
            })
            .collect();
        let refreshed = plandoc::above_terrain(&Document { items, ..current }, &terrain_height);
        let changed_any = state.document.as_ref() != Some(&refreshed);
        state.document = Some(refreshed);
        changed_any
    };
    if refreshed {
        changed();
    }
}

fn set_altitude_mode(args: &str) -> Value {
    let mode = serde_json::from_str::<Value>(args)
        .ok()
        .and_then(|v| v.get(0).and_then(Value::as_i64));
    match mode
        .filter(|m| (crate::altitudemodes::MIXED..=crate::altitudemodes::TERRAIN_FRAME).contains(m))
    {
        Some(mode) => edit(|doc| Ok(plandoc::set_global_altitude_mode(doc, mode))),
        None => refused("An altitude mode is a number from 0 to 4."),
    }
}

fn items(backend: &dyn Backend, args: &str) -> Value {
    let selected = serde_json::from_str::<Value>(args)
        .ok()
        .and_then(|v| v.get(0).and_then(Value::as_i64))
        .unwrap_or(0);
    let Some(document) = held().document.clone() else {
        return refused("There is no plan.");
    };
    let rover = crate::read::flag(
        &backend.value_fields("plan.controllerVehicle", "rover"),
        "rover",
    );
    match crate::missionitems::document_view(
        &document,
        selected,
        &crate::read::Unit::vertical(backend),
        &crate::read::Unit::horizontal(backend),
        &crate::read::Unit::speed(backend),
        rover,
    ) {
        Ok(view) => json!({ "ok": true, "view": view }),
        Err(reason) => refused(reason),
    }
}

fn item_edit(backend: &dyn Backend, args: &str, command: bool) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let (Some(index), Some(value)) = (
        given
            .get(0)
            .and_then(Value::as_u64)
            .and_then(|i| usize::try_from(i).ok()),
        given
            .get(1)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
    ) else {
        return refused("An item edit needs the item's index and a number.");
    };
    let Some(defaults) = edit_defaults(backend) else {
        return refused("The default mission item altitude is not known.");
    };
    edit_with(|doc| {
        let (document, added) = match command {
            true => plandoc::command_at(doc, index, value as i64, &defaults),
            false => plandoc::set_altitude(doc, index, value).map(|changed| (changed, None)),
        }
        .ok_or_else(|| format!("Item {index} cannot take that edit."))?;
        Ok(Edited {
            note: added.and_then(|at| added_note(&document, index, at)),
            selected: added.map(|at| at as i64),
            plain: Some(index).filter(|_| command && added.is_none()),
            own_step: false,
            document,
        })
    })
}

fn added_note(doc: &Document, waypoint: usize, added: usize) -> Option<String> {
    let reads = crate::missionitems::document_reads(doc, -1).ok()?;
    let stops = crate::missionitems::stops(&reads);
    let name = |index: usize| {
        reads
            .get(index)?
            .get("commandName")?
            .as_str()
            .map(crate::label::sentence)
    };
    let at = match stops.get(waypoint).copied().flatten() {
        Some(number) => format!("{} {number}", name(waypoint)?),
        None => name(waypoint)?,
    };
    Some(format!("{} added at {at}", name(added)?))
}

fn offline_types() -> (i64, i64) {
    (
        offline_type("offlineEditingFirmwareClass"),
        offline_type("offlineEditingVehicleClass"),
    )
}

fn tracked(document: Document, (firmware_type, vehicle_type): (i64, i64)) -> Document {
    match document.items.is_empty() {
        true => plandoc::retyped(document, firmware_type, vehicle_type),
        false => document,
    }
}

pub fn offline_types_changed() {
    let types = offline_types();
    let moved = {
        let mut state = held();
        let Some(document) = state.document.as_ref() else {
            return;
        };
        let next = tracked(document.clone(), types);
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

const SAME_ALTITUDE_METRES: f64 = 0.05;

fn follows_one_altitude(document: &Document) -> bool {
    let heights: Vec<f64> = document
        .items
        .iter()
        .filter_map(|item| match item {
            plandoc::Item::Simple(simple)
                if ![CMD_NAV_LAND, CMD_NAV_VTOL_LAND].contains(&simple.command) =>
            {
                simple.altitude.as_ref().map(|held| held.altitude)
            }
            _ => None,
        })
        .collect();
    heights
        .windows(2)
        .all(|pair| (pair[0] - pair[1]).abs() < SAME_ALTITUDE_METRES)
}

pub fn default_altitude_changed() {
    let (has_items, uniform) = {
        let state = held();
        state.document.as_ref().map_or((false, false), |d| {
            (!d.items.is_empty(), follows_one_altitude(d))
        })
    };
    let route_altitude =
        crate::settingsstore::raw_setting(DEFAULT_ALTITUDE).and_then(|v| v.as_f64());
    match (has_items, uniform, route_altitude) {
        (false, _, _) => {}
        (true, true, Some(metres)) => {
            edit(|doc| {
                Ok(Document {
                    items: doc
                        .items
                        .iter()
                        .map(|item| with_new_altitude(item, metres))
                        .collect(),
                    ..doc.clone()
                })
            });
        }
        (true, _, _) => {
            ASK_APPLY_ALTITUDE.store(true, std::sync::atomic::Ordering::Relaxed);
            changed();
        }
    }
}

pub fn altitude_prompt() -> Value {
    match ASK_APPLY_ALTITUDE.load(std::sync::atomic::Ordering::Relaxed) {
        true => json!({ "title": APPLY_ALTITUDE_TITLE, "text": APPLY_ALTITUDE_TEXT }),
        false => Value::Null,
    }
}

pub fn with_new_altitude(item: &plandoc::Item, altitude: f64) -> plandoc::Item {
    let metres = crate::read::Unit {
        name: "m".to_string(),
        factor: 1.0,
    };
    let units = crate::surveydoc::Units {
        vertical: &metres,
        horizontal: &metres,
    };
    match item {
        plandoc::Item::Simple(simple)
            if simple.altitude.is_some()
                && ![CMD_NAV_LAND, CMD_NAV_VTOL_LAND].contains(&simple.command) =>
        {
            let params = std::array::from_fn(|i| {
                if i == 6 {
                    Some(altitude)
                } else {
                    simple.params[i]
                }
            });
            plandoc::Item::Simple(plandoc::Simple {
                params,
                altitude: simple
                    .altitude
                    .clone()
                    .map(|held| plandoc::Altitude { altitude, ..held }),
                ..simple.clone()
            })
        }
        plandoc::Item::Complex {
            kind,
            json,
            item_count,
        } => {
            let edited = match kind.as_str() {
                k if crate::landingpattern::is_landing(k) => {
                    crate::landingpattern::edit(json, "finalApproachAltitude", &json!(altitude))
                }
                "StructureScan" => {
                    crate::surveydoc::set(json, "entranceAlt", &json!(altitude), &units)
                }
                _ => crate::surveydoc::set(
                    json,
                    "cameraCalc.valueSetIsDistance",
                    &json!(true),
                    &units,
                )
                .and_then(|distance| {
                    crate::surveydoc::set(
                        &distance,
                        "cameraCalc.distanceToSurface",
                        &json!(altitude),
                        &units,
                    )
                }),
            };
            match edited {
                Some(edited) => plandoc::Item::Complex {
                    kind: kind.clone(),
                    item_count: plandoc::complex_count(kind, &edited).unwrap_or(*item_count),
                    json: edited,
                },
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
    edit(|doc| {
        Ok(Document {
            items: doc
                .items
                .iter()
                .map(|item| with_new_altitude(item, defaults.mission_item_altitude))
                .collect(),
            ..doc.clone()
        })
    })
}

fn remove(args: &str) -> Value {
    let index = serde_json::from_str::<Value>(args)
        .ok()
        .and_then(|v| v.get(0).and_then(Value::as_u64));
    let Some(index) = index.and_then(|i| usize::try_from(i).ok()) else {
        return refused("Remove needs the index of the item.");
    };
    let answer = edit(|doc| {
        plandoc::remove_with_riders(doc, index).map(|after| plandoc::after_scan_removed(doc, after))
    });
    if answer["ok"] == true {
        let mut state = held();
        let last = state.document.as_ref().map_or(0, |d| d.items.len());
        state.selected = current_after_remove(index, last);
    }
    answer
}

fn convert_to_vehicle() -> Value {
    let active = crate::hub::lock()
        .active()
        .map(|v| (i64::from(v.autopilot), i64::from(v.vehicle_type)));
    match active {
        Some((firmware, vehicle)) => edit(|doc| plandoc::for_vehicle(doc, firmware, vehicle)),
        None => refused("No vehicle is connected to convert the plan for."),
    }
}

fn move_item(args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let (Some(index), Some(step)) = (
        given
            .get(0)
            .and_then(Value::as_u64)
            .and_then(|i| usize::try_from(i).ok()),
        given
            .get(1)
            .and_then(Value::as_i64)
            .filter(|step| step.abs() == 1),
    ) else {
        return refused("Moving an item needs its index and a step of 1 or -1.");
    };
    let landed = std::cell::Cell::new(index);
    match edit_with(|doc| {
        plandoc::move_item(doc, index, step > 0).map(|(document, moved)| {
            landed.set(moved);
            Edited {
                document,
                selected: Some(moved as i64),
                note: None,
                plain: None,
                own_step: false,
            }
        })
    }) {
        answer if answer["ok"] == true => json!({ "ok": true, "index": landed.get() }),
        answer => answer,
    }
}

fn current_after_remove(removed: usize, last_visual: usize) -> i64 {
    removed.min(last_visual) as i64
}

const TEXT_MISSION_SUFFIXES: &[&str] = &["waypoints", "txt"];
const PLAN_SUFFIX: &str = "plan";

fn is_text_mission(file: &str) -> bool {
    std::path::Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|suffix| TEXT_MISSION_SUFFIXES.contains(&suffix))
}

fn load_text_mission(text: &str) -> Result<Document, String> {
    let (firmware_type, vehicle_type) = planned_types();
    let file = crate::waypoints::parse(text).map_err(|e| format!("Mission: {e}"))?;
    let plan = crate::planfile::write(&crate::planfile::from_waypoints(
        &file,
        firmware_type,
        vehicle_type,
    ));
    let document = with_offline_speeds(
        plandoc::load_text_mission(&plan, offline_type("offlineEditingVehicleClass"))
            .map_err(|e| format!("Mission: {e}"))?,
    );
    Ok(placed_text_mission(
        document,
        file.home.is_some(),
        active_vehicle_home(),
    ))
}

fn launch_ground(
    document: &Document,
    vehicle_home: Option<[f64; 3]>,
    terrain: &dyn Fn(f64, f64) -> Option<f64>,
) -> Option<f64> {
    let home = document.home?;
    terrain(home[0], home[1])
        .or_else(|| {
            vehicle_home
                .filter(|vehicle| (vehicle[0], vehicle[1]) == (home[0], home[1]))
                .map(|vehicle| vehicle[2])
        })
        .filter(|ground| ground.is_finite())
}

fn active_vehicle_home() -> Option<[f64; 3]> {
    crate::hub::lock()
        .active()
        .and_then(|v| v.home)
        .map(|(latitude, longitude, altitude)| [latitude, longitude, altitude])
}

fn placed_text_mission(
    document: Document,
    home_in_file: bool,
    vehicle_home: Option<[f64; 3]>,
) -> Document {
    let commands = crate::cmdinfo::tree(
        plandoc::firmware(document.firmware_type),
        plandoc::vehicle_class(document.vehicle_type),
    );
    let takeoff = |s: &plandoc::Simple| commands.get(&s.command).is_some_and(|c| c.is_takeoff);
    let has_takeoff = document
        .items
        .iter()
        .any(|item| matches!(item, plandoc::Item::Simple(s) if takeoff(s)));
    let home_while_loading = match home_in_file {
        true => document.home,
        false => vehicle_home.filter(|_| has_takeoff),
    };
    let items = document
        .items
        .iter()
        .map(|item| match (item, home_while_loading) {
            (plandoc::Item::Simple(s), Some(home))
                if takeoff(s)
                    && !commands
                        .get(&s.command)
                        .is_some_and(|c| c.specifies_coordinate) =>
            {
                plandoc::Item::Simple(plandoc::Simple {
                    params: std::array::from_fn(|i| match i {
                        4 => Some(home[0]),
                        5 => Some(home[1]),
                        _ => s.params[i],
                    }),
                    ..s.clone()
                })
            }
            _ => item.clone(),
        })
        .collect();
    let placed = Document { items, ..document };
    Document {
        home: home_while_loading.or_else(|| plandoc::home_from_first_coordinate(&placed, None)),
        ..placed
    }
}

fn plan_file_after_load(file: &str) -> String {
    std::path::Path::new(file)
        .with_extension(PLAN_SUFFIX)
        .to_string_lossy()
        .into_owned()
}

fn load_failed_text(file: &str, reason: &str) -> String {
    format!("Error loading Plan file ({file}). {reason}")
}

struct Imported {
    document: Document,
    wizard: Option<usize>,
    summary: Option<String>,
}

fn takes_off(document: &Document) -> bool {
    use crate::cmdinfo::{Firmware, VehicleClass};
    matches!(
        plandoc::firmware(document.firmware_type),
        Firmware::Px4 | Firmware::ArduPilot
    ) && matches!(
        plandoc::vehicle_class(document.vehicle_type),
        VehicleClass::FixedWing | VehicleClass::MultiRotor | VehicleClass::Vtol
    )
}

struct KmlContext {
    types: (i64, i64),
    offline_vehicle_type: i64,
    vehicle_home: Option<[f64; 3]>,
    takeoff_not_required: bool,
}

fn kml_document(
    route: &crate::kml::Route,
    context: &KmlContext,
    defaults: &plandoc::EditDefaults,
) -> Result<Imported, String> {
    let imported = crate::kmlmission::mission(
        route,
        context.types.0,
        context.types.1,
        defaults.mission_item_altitude,
    );
    let loaded = with_offline_speeds(
        plandoc::load_text_mission(
            &crate::planfile::write(&imported.mission),
            context.offline_vehicle_type,
        )
        .map_err(|e| format!("Mission: {e}"))?,
    );
    let homed = Document {
        home: context
            .vehicle_home
            .or_else(|| plandoc::home_from_first_coordinate(&loaded, None)),
        ..loaded
    };
    let takeoff = takes_off(&homed) && !context.takeoff_not_required;
    let placing = takeoff
        && plandoc::vehicle_class(homed.vehicle_type) == crate::cmdinfo::VehicleClass::FixedWing;
    let document = match takeoff {
        true => plandoc::insert_takeoff(&homed, 1, defaults)?,
        false => homed,
    };
    Ok(Imported {
        document,
        wizard: placing.then_some(1),
        summary: Some(crate::kmlmission::summary(
            route,
            &imported,
            defaults.mission_item_altitude,
            &crate::kmlmission::Added {
                takeoff,
                takeoff_needs_placing: placing,
            },
        )),
    })
}

fn load_kml(backend: &dyn Backend, file: &str) -> Result<Imported, String> {
    let route = crate::kml::route(&crate::kml::read(file)?)?;
    let defaults =
        edit_defaults(backend).ok_or("The default mission item altitude is not known.")?;
    let context = KmlContext {
        types: planned_types(),
        offline_vehicle_type: offline_type("offlineEditingVehicleClass"),
        vehicle_home: active_vehicle_home(),
        takeoff_not_required: planning_setting("takeoffItemNotRequired", false),
    };
    kml_document(&route, &context, &defaults).map(|imported| Imported {
        document: plandoc::above_terrain(&imported.document, &terrain_height),
        ..imported
    })
}

fn load_file(backend: &dyn Backend, file: &str) -> Result<Imported, String> {
    let plain = |document| Imported {
        document,
        wizard: None,
        summary: None,
    };
    match crate::kml::is_kml(file) {
        true => load_kml(backend, file),
        false => crate::vfs::read(file)
            .map_err(|e| format!("{e} {file}"))
            .and_then(|bytes| {
                let text = String::from_utf8_lossy(&bytes);
                match is_text_mission(file) {
                    true => load_text_mission(&text),
                    false => load_plan(&text),
                }
            })
            .map(plain),
    }
}

const KML_IMPORTED: &str = "KML imported";

fn open(backend: &dyn Backend, file: &str) -> Value {
    forget_raw_edits();
    let connected = !offline();
    match load_file(backend, file) {
        Ok(Imported {
            document,
            wizard,
            summary,
        }) => {
            let count = document.items.len();
            {
                let mut state = held();
                let before = state.document.take();
                state.document = Some(document);
                state.wizard = wizard;
                remember(&mut state, before, crate::hub::now_ms());
                state.selected = 0;
                state.file = Some(plan_file_after_load(file));
                settle_clean(&mut state);
                state.dirty = connected;
            }
            settle_home_on_terrain(None);
            changed();
            if let Some(summary) = &summary {
                crate::noticeboard::post(crate::noticeboard::MESSAGE, KML_IMPORTED, summary);
            }
            json!({ "ok": true, "items": count, "result": true, "summary": summary })
        }
        Err(reason) => {
            held().file = None;
            changed();
            refused(reason)
        }
    }
}

fn save(file: &str) -> Value {
    if held().document.as_ref().is_some_and(waiting_on_terrain) {
        return refused(format!("Unable to Save. {WAITING_ON_TERRAIN}"));
    }
    let Some(text) = held()
        .document
        .as_ref()
        .map(|d| plandoc::save(d).to_string())
    else {
        return refused("There is no plan to save.");
    };
    let offline = offline();
    match crate::vfs::write(file, text) {
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
    let Some(plan) = held().document.as_ref().map(plandoc::mission) else {
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
    match crate::plankml::document(&plan, &planned)
        .and_then(|kml| crate::vfs::write(&target, kml).map_err(|e| e.to_string()))
    {
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
        _ => {
            json!({ "action": "write", "plan": "rally", "points": document.rally["points"].as_array().cloned().unwrap_or_default() })
        }
    }
}

fn launch_at_takeoff(document: &Document, visual_index: usize) -> bool {
    let commands = crate::cmdinfo::tree(
        plandoc::firmware(document.firmware_type),
        plandoc::vehicle_class(document.vehicle_type),
    );
    let forward_flight = matches!(
        plandoc::vehicle_class(document.vehicle_type),
        crate::cmdinfo::VehicleClass::FixedWing | crate::cmdinfo::VehicleClass::Vtol
    );
    let takeoff = visual_index
        .checked_sub(1)
        .and_then(|at| document.items.get(at))
        .and_then(|item| match item {
            plandoc::Item::Simple(s) if commands.get(&s.command).is_some_and(|c| c.is_takeoff) => {
                Some(s)
            }
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

pub fn draft_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    json!({ "kind": "object", "class": "PlanDraft", "text": draft(&held()) })
}

fn draft(state: &Held) -> Option<String> {
    state
        .document
        .as_ref()
        .filter(|document| contains_items(document))
        .map(|document| plandoc::save(document).to_string())
}

fn restore_draft(text: &str) -> Value {
    let Ok(document) = load_plan(text) else {
        return refused("The plan kept from the last session could not be read.");
    };
    let restored = restored_into(&mut held(), document);
    if restored {
        settle_home_on_terrain(None);
        changed();
    }
    json!({ "ok": true, "result": restored })
}

fn restored_into(state: &mut Held, document: Document) -> bool {
    if state.document.as_ref().is_some_and(contains_items) || !contains_items(&document) {
        return false;
    }
    state.document = Some(document);
    state.selected = 0;
    state.file = None;
    state.clean = None;
    state.dirty = true;
    state.dirty_for_save = true;
    true
}

fn contains_items(document: &Document) -> bool {
    let listed = |section: &Value, key: &str| {
        section
            .get(key)
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty())
    };
    !document.items.is_empty()
        || listed(&document.fence, "polygons")
        || listed(&document.fence, "circles")
        || listed(&document.rally, "points")
}

fn send_after_mission(id: u8, document: Document) {
    let _ = crate::background::spawn("groundstation-plan-send", move || {
        let (fence, rally) = crate::hub::lock()
            .vehicle(id)
            .map_or((false, false), crate::hub::Vehicle::plans_supported);
        let wanted: Vec<&'static str> = [("fence", fence), ("rally", rally)]
            .into_iter()
            .filter(|(_, supported)| *supported)
            .map(|(k, _)| k)
            .collect();
        send_rest(id, "mission", wanted, document);
    });
}

fn send_rest(id: u8, before: &'static str, rest: Vec<&'static str>, document: Document) {
    match rest.split_first() {
        None if before == "mission" => changed(),
        None => settle_then(id, before, |_| changed()),
        Some((&kind, later)) => {
            let later = later.to_vec();
            settle_then(id, before, move |settled| {
                settle_then(id, kind, move |free| {
                    if settled && free {
                        let requested = crate::hub::lock().mission_request(
                            Some(id),
                            &send_shape(kind, &document),
                            crate::hub::now_ms(),
                        );
                        match requested {
                            Ok(outbound) => deliver(outbound),
                            Err(error) => {
                                crate::noticeboard::post(
                                    crate::noticeboard::MESSAGE,
                                    "",
                                    &crate::hub::transfer_failed_text(kind, &error),
                                );
                            }
                        }
                    }
                    send_rest(id, kind, later, document);
                });
            });
        }
    }
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
        return refused(format!(
            "The plan could not be handed to the vehicle link: {e}"
        ));
    }
    let path = file.to_string_lossy().to_string();
    let loaded =
        crate::read::object(&backend.invoke("plan.loadFromFile", &json!([path]).to_string()));
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
    backend
        .value(HOST_SYNC)
        .get("value")
        .and_then(Value::as_bool)
        == Some(true)
}

fn follow_vehicle() {
    if !enabled() {
        return;
    }
    plan_for_active_vehicle();
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
            (
                v.id,
                v.mission_snapshot(),
                v.sends_home(),
                fence,
                rally,
                (i64::from(v.autopilot), i64::from(v.vehicle_type)),
                v.home.map(|(lat, lon, alt)| [lat, lon, alt]),
            )
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
                let has_items = state.document.as_ref().is_some_and(contains_items);
                if state.dirty_for_save && has_items {
                    state.vehicle_prompt = Some(true);
                    drop(state);
                    changed();
                    return;
                }
                let before = state.document.clone();
                state.document = state.document.clone().map(|d| Document {
                    home: None,
                    items: Vec::new(),
                    settings_sections: Vec::new(),
                    fence: empty_document().fence,
                    rally: empty_document().rally,
                    ..d
                });
                state.wizard = None;
                forget_raw_edits();
                remember(&mut state, before, crate::hub::now_ms());
                state.selected = 0;
                settle_clean(&mut state);
                state.file = None;
                None
            }
            (Some(id), Some(vehicle)) if state.shown_vehicle != Some(id) && vehicle.0 == id => {
                state.shown_vehicle = Some(id);
                let has_items = state.document.as_ref().is_some_and(contains_items);
                if state.dirty_for_save && has_items {
                    state.vehicle_prompt = Some(false);
                }
                (!state.dirty_for_save || !has_items).then_some(vehicle)
            }
            _ => return,
        }
    };
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
    let finished = path == HOST_SYNC
        && crate::read::object(value)
            .get("value")
            .and_then(Value::as_bool)
            == Some(false);
    if !finished || !HOST_FETCH.swap(false, std::sync::atomic::Ordering::SeqCst) {
        return false;
    }
    let file = host_file();
    let path = file.to_string_lossy().to_string();
    backend.invoke("plan.saveToFile", &json!([path]).to_string());
    let adopted = std::fs::read_to_string(&file)
        .map_err(|e| e.to_string())
        .and_then(|text| load_plan(&text));
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
    let items = match crate::planitems::flatten(&plandoc::mission(&plandoc::as_flown(&document))) {
        Ok(items) => items,
        Err(reason) => return refused(reason),
    };
    let transfer: Option<Vec<Item>> = items
        .iter()
        .map(|i| {
            Some(Item {
                seq: 0,
                frame: u8::try_from(i.frame).ok()?,
                command: u16::try_from(i.command).ok()?,
                current: false,
                auto_continue: i.auto_continue,
                params: i.params,
            })
        })
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

const TRANSFER_SETTLE_MS: u64 = 60_000;
const TRANSFER_POLL: std::time::Duration = std::time::Duration::from_millis(100);

fn once_settled(
    name: &str,
    mut check: impl FnMut() -> Option<bool> + Send + 'static,
    then: impl FnOnce(bool) + Send + 'static,
) {
    let deadline = crate::hub::now_ms() + TRANSFER_SETTLE_MS;
    let mut then = Some(then);
    let _ = crate::background::poll(name, TRANSFER_POLL, move || {
        match check().or_else(|| (crate::hub::now_ms() >= deadline).then_some(false)) {
            None => false,
            Some(settled) => {
                if let Some(then) = then.take() {
                    then(settled);
                }
                true
            }
        }
    });
}

fn settle_then(id: u8, kind: &'static str, then: impl FnOnce(bool) + Send + 'static) {
    once_settled(
        "groundstation-plan-transfer",
        move || {
            let hub = crate::hub::lock();
            transfer_outcome(
                hub.active_id(),
                id,
                hub.vehicle(id).is_some_and(|v| {
                    v.mission_snapshot()[kind]["inProgress"].as_bool() == Some(true)
                }),
            )
        },
        then,
    );
}

fn read_succeeded(snapshot: &Value, kind: &str) -> bool {
    snapshot[kind]["error"].is_null()
}

fn load_then(id: u8, kind: &'static str, then: impl FnOnce(bool) + Send + 'static) {
    let started = crate::hub::lock().mission_request(
        Some(id),
        &json!({ "action": "load", "plan": kind }),
        crate::hub::now_ms(),
    );
    match started {
        Ok(outbound) => {
            deliver(outbound);
            settle_then(id, kind, move |settled| {
                let read = settled
                    && crate::hub::lock()
                        .vehicle(id)
                        .is_some_and(|v| read_succeeded(&v.mission_snapshot(), kind));
                then(read);
            });
        }
        Err(_) => then(false),
    }
}

fn load_if(wanted: bool, id: u8, kind: &'static str, then: impl FnOnce(bool) + Send + 'static) {
    match wanted {
        true => load_then(id, kind, then),
        false => then(false),
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
    plandoc::load(text, offline_type("offlineEditingVehicleClass")).map(with_offline_speeds)
}

fn with_offline_speeds(document: Document) -> Document {
    Document {
        cruise_speed: if document.cruise_speed.is_nan() {
            remembered_speed("offlineEditingCruiseSpeed", 15.0)
        } else {
            document.cruise_speed
        },
        hover_speed: if document.hover_speed.is_nan() {
            remembered_speed("offlineEditingHoverSpeed", 5.0)
        } else {
            document.hover_speed
        },
        ..document
    }
}

fn remembered_speed(name: &str, fallback: f64) -> f64 {
    crate::settingsstore::stored_text(name)
        .and_then(|text| text.parse().ok())
        .unwrap_or(fallback)
}

fn adopt(
    snapshot: &Value,
    sends_home: bool,
    fence_read: bool,
    rally_read: bool,
    types: (i64, i64),
    vehicle_home: Option<[f64; 3]>,
) {
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
    let template = Document {
        firmware_type: types.0,
        vehicle_type: types.1,
        ..held_document
    };
    let before = state.document.clone();
    let downloaded_mission = plandoc::from_vehicle(&downloaded, sends_home, &template);
    let mission = Document {
        home: plandoc::planned_home(&downloaded_mission, vehicle_home),
        ..downloaded_mission
    };
    state.wizard = None;
    state.document = Some(Document {
        fence: if fence_read {
            fence_from(&snapshot["fence"])
        } else {
            mission.fence.clone()
        },
        rally: if rally_read {
            json!({ "version": 2, "points": snapshot["rally"]["points"] })
        } else {
            mission.rally.clone()
        },
        ..mission
    });
    remember(&mut state, before, crate::hub::now_ms());
    state.selected = 0;
    settle_clean(&mut state);
    state.fetching = false;
}

fn fetch() -> Value {
    let Some((id, fence, rally, sends_home, types, vehicle_home)) =
        crate::hub::lock().active().map(|v| {
            let (fence, rally) = v.plans_supported();
            (
                v.id,
                fence,
                rally,
                v.sends_home(),
                (i64::from(v.autopilot), i64::from(v.vehicle_type)),
                v.home.map(|(lat, lon, alt)| [lat, lon, alt]),
            )
        })
    else {
        return refused("No vehicle is connected through the core.");
    };
    if crate::hub::lock()
        .active()
        .is_some_and(crate::hub::Vehicle::on_high_latency_link)
    {
        return refused(crate::hub::HIGH_LATENCY_DOWNLOAD);
    }
    held().fetching = true;
    let _ = crate::background::spawn("groundstation-plan-fetch", move || {
        load_then(id, "mission", move |mission_read| {
            load_if(fence, id, "fence", move |fence_read| {
                load_if(rally, id, "rally", move |rally_read| {
                    let (still_active, snapshot) = {
                        let hub = crate::hub::lock();
                        (
                            hub.active_id() == Some(id),
                            hub.vehicle(id)
                                .map(crate::hub::Vehicle::mission_snapshot)
                                .unwrap_or(Value::Null),
                        )
                    };
                    match mission_read && still_active {
                        true => {
                            adopt(
                                &snapshot,
                                sends_home,
                                fence_read,
                                rally_read,
                                types,
                                vehicle_home,
                            );
                            settle_home_on_terrain(None);
                        }
                        false => held().fetching = false,
                    }
                    changed();
                });
            });
        });
    });
    json!({ "ok": true })
}

fn status() -> Value {
    let mission = crate::hub::lock()
        .active()
        .map(|v| v.mission_snapshot()["mission"].clone())
        .unwrap_or(Value::Null);
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
        rows.iter_mut()
            .skip(1)
            .zip(&document.items)
            .for_each(|(row, item)| {
                row["edited"] = json!(!clean.is_some_and(|c| c.items.contains(item)))
            });
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
        items: v
            .mission_items()
            .iter()
            .map(|i| Downloaded {
                frame: i64::from(i.frame),
                command: i64::from(i.command),
                params: i.params,
                auto_continue: i.auto_continue,
            })
            .collect(),
        sends_home: v.sends_home(),
        types: (i64::from(v.autopilot), i64::from(v.vehicle_type)),
        current: i64::from(v.current_mission_index()),
        home: v.home.map(|(lat, lon, alt)| [lat, lon, alt]),
    }
}

fn flown_document(mission: &FlownMission) -> Document {
    let template = Document {
        firmware_type: mission.types.0,
        vehicle_type: mission.types.1,
        ..empty_document()
    };
    let downloaded = plandoc::from_vehicle(&mission.items, mission.sends_home, &template);
    Document {
        home: downloaded.home.or(mission.home),
        ..downloaded
    }
}

fn sequence_count(document: &Document) -> i64 {
    visual_spans(document)
        .last()
        .map_or(0, |(_, last)| last + 1)
}

pub struct FlownRow {
    pub place: String,
    pub arrived: bool,
    pub within: Option<(usize, usize)>,
}

fn pattern_commands(kind: &str, json: &Value) -> Vec<i64> {
    match kind {
        k if plandoc::TRANSECT_STYLE.contains(&k) => json
            .pointer("/TransectStyleComplexItem/Items")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("command").and_then(Value::as_i64))
                    .collect()
            })
            .unwrap_or_default(),
        "StructureScan" => crate::structurescan::saved_items(json)
            .map(|items| items.iter().map(|item| i64::from(item.command)).collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn waypoint_within(document: &Document, at: i64, sequence: i64) -> Option<(usize, usize)> {
    let Some(plandoc::Item::Complex { kind, json, .. }) =
        document.items.get(usize::try_from(at - 1).ok()?)
    else {
        return None;
    };
    let (first, _) = *visual_spans(document).get(usize::try_from(at).ok()?)?;
    let commands = pattern_commands(kind, json);
    let waypoints = |list: &[i64]| {
        list.iter()
            .filter(|command| **command == plandoc::CMD_NAV_WAYPOINT)
            .count()
    };
    let reached = waypoints(commands.get(..=usize::try_from(sequence - first).ok()?)?);
    Some((reached.max(1), waypoints(&commands))).filter(|(_, total)| *total > 1)
}

fn row_of_sequence(document: &Document, sequence: i64) -> Option<FlownRow> {
    let at = visual_index_of_sequence(document, sequence).filter(|at| *at > 0)?;
    let row = usize::try_from(at).ok()?;
    let reads = crate::missionitems::document_reads(document, -1).ok()?;
    let stops = crate::missionitems::stops(&reads);
    let total = stops.iter().flatten().count();
    let name = |index: usize| {
        reads
            .get(index)
            .and_then(|read| read.get("commandName"))
            .and_then(Value::as_str)
            .map(str::to_lowercase)
    };
    let stop_place = |index: usize| {
        Some(format!(
            "{} {} of {total}",
            name(index)?,
            stops.get(index).copied().flatten()?
        ))
    };
    let own = name(row)?;
    let has_a_place = |index: usize| {
        stops[index].is_some()
            || crate::read::flag(&reads[index], "specifiesCoordinate")
            || crate::missionitems::letter(&reads[index]).is_some()
    };
    let attached = (1..row).rev().find(|index| has_a_place(*index));
    let (place, arrived) = match (stops[row], attached) {
        (Some(_), _) => (stop_place(row)?, false),
        (None, Some(index)) if !has_a_place(row) => (
            format!("{} · {own}", stop_place(index).or_else(|| name(index))?),
            true,
        ),
        _ => (own, false),
    };
    Some(FlownRow {
        place,
        arrived,
        within: waypoint_within(document, at, sequence),
    })
}

fn flown_rows_document(planned: Option<&Document>, mission: &FlownMission) -> Document {
    let flown = flown_document(mission);
    planned
        .map(plandoc::as_flown)
        .filter(|plan| sequence_count(plan) == sequence_count(&flown))
        .unwrap_or(flown)
}

pub fn flown_row(sequence: i64) -> Option<FlownRow> {
    let mission = crate::hub::lock().active().map(flown_mission)?;
    let planned = {
        let state = held();
        state.document.clone().filter(|_| !state.dirty)
    };
    row_of_sequence(&flown_rows_document(planned.as_ref(), &mission), sequence)
}

fn rows_of(mission: &FlownMission) -> Option<usize> {
    Some(flown_document(mission).items.len()).filter(|rows| *rows > 0)
}

pub fn stored_rows() -> Option<usize> {
    let mission = crate::hub::lock().active().map(flown_mission)?;
    rows_of(&mission)
}

fn flown_view(backend: &dyn Backend, mission: &FlownMission) -> Result<Value, String> {
    if mission.items.is_empty() {
        return Err("The vehicle holds no mission.".to_string());
    }
    let document = flown_document(mission);
    let rover = plandoc::vehicle_class(mission.types.1) == crate::cmdinfo::VehicleClass::Rover;
    let selected = visual_index_of_sequence(&document, mission.current)
        .filter(|index| *index > 0)
        .unwrap_or(-1);
    crate::missionitems::document_view(
        &document,
        selected,
        &crate::read::Unit::vertical(backend),
        &crate::read::Unit::horizontal(backend),
        &crate::read::Unit::speed(backend),
        rover,
    )
}

pub fn fly_view(backend: &dyn Backend) -> Value {
    let (active, others) = {
        let hub = crate::hub::lock();
        let active_id = hub.active_id();
        let active = hub.active().map(flown_mission);
        let others: Vec<FlownMission> = hub
            .in_arrival_order()
            .into_iter()
            .filter(|v| Some(v.id) != active_id)
            .map(flown_mission)
            .collect();
        (active, others)
    };
    let other_views: Vec<Value> = others
        .iter()
        .filter_map(|mission| flown_view(backend, mission).ok())
        .collect();
    let mut answer = active
        .ok_or_else(|| "The vehicle holds no mission.".to_string())
        .and_then(|mission| flown_view(backend, &mission))
        .unwrap_or_else(|reason| json!({ "kind": "object", "class": "MissionItems", "available": false, "items": [], "selected": -1, "reason": reason }));
    answer["others"] = Value::Array(other_views);
    answer
}

pub fn view(backend: &dyn Backend) -> Value {
    let (document, selected, clean, note) = {
        let state = held();
        (
            state.document.clone().unwrap_or_else(empty_document),
            state.selected,
            state.clean.clone(),
            json!({ "note": state.note, "noteId": state.note_id }),
        )
    };
    let revision = revision_of(&document);
    let rover = crate::read::flag(
        &backend.value_fields("plan.controllerVehicle", "rover"),
        "rover",
    );
    crate::missionitems::document_view(&document, selected, &crate::read::Unit::vertical(backend), &crate::read::Unit::horizontal(backend), &crate::read::Unit::speed(backend), rover)
        .map(|view| with_note(marked_edited(view, &document, clean.as_ref()), note, revision))
        .unwrap_or_else(|reason| json!({ "kind": "object", "class": "MissionItems", "available": false, "items": [], "selected": -1, "reason": reason }))
}

fn with_note(mut view: Value, note: Value, revision: u64) -> Value {
    view["note"] = note["note"].clone();
    view["noteId"] = note["noteId"].clone();
    view["revision"] = json!(revision);
    view
}

pub(crate) fn visual_spans(document: &Document) -> Vec<(i64, i64)> {
    std::iter::once((0usize, document.settings_sections.len()))
        .chain(
            plandoc::item_spans(document)
                .into_iter()
                .map(|(start, span)| (start, start + span - 1)),
        )
        .map(|(first, last)| (first as i64, last as i64))
        .collect()
}

fn visual_index_of_sequence(document: &Document, sequence: i64) -> Option<i64> {
    visual_spans(document)
        .iter()
        .position(|(first, last)| (*first..=*last).contains(&sequence))
        .map(|i| i as i64)
}

fn planning_setting(name: &str, unset: bool) -> bool {
    crate::settingsstore::raw_setting(&format!("settings.planViewSettings.{name}"))
        .and_then(|v| v.as_bool())
        .unwrap_or(unset)
}

fn offline_type(name: &str) -> i64 {
    crate::settingsstore::raw_setting(&format!("settings.appSettings.{name}"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

fn planned_types() -> (i64, i64) {
    crate::hub::lock()
        .active()
        .map(|v| (i64::from(v.autopilot), i64::from(v.vehicle_type)))
        .unwrap_or_else(|| {
            (
                offline_type("offlineEditingFirmwareClass"),
                offline_type("offlineEditingVehicleClass"),
            )
        })
}

fn planned_type_text(
    connected: bool,
    vehicle_type: i64,
    class: crate::cmdinfo::VehicleClass,
) -> &'static str {
    match (connected, class) {
        (true, _) => u8::try_from(vehicle_type).map_or("", crate::vehiclefacade::mav_type_text),
        (false, crate::cmdinfo::VehicleClass::FixedWing) => "Fixed Wing",
        (false, crate::cmdinfo::VehicleClass::MultiRotor) => "Multi-Rotor",
        (false, crate::cmdinfo::VehicleClass::Vtol) => "VTOL",
        (false, crate::cmdinfo::VehicleClass::Rover) => "Rover",
        (false, crate::cmdinfo::VehicleClass::Sub) => "Sub",
        (false, _) => "Unknown",
    }
}

fn camera_calc_index(path: &str) -> Option<usize> {
    path.strip_prefix("plan.missionController.visualItems.")?
        .strip_suffix(".cameraCalc")?
        .parse()
        .ok()
}

pub fn controller_fields(path: &str) -> Option<Value> {
    if !enabled() || !path.starts_with("plan.") {
        return None;
    }
    let (document, selected) = {
        let state = held();
        (
            state.document.clone().unwrap_or_else(empty_document),
            state.selected,
        )
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
            "airship": class == crate::cmdinfo::VehicleClass::Airship,
            "apmFirmware": firmware == crate::cmdinfo::Firmware::ArduPilot,
            "px4Firmware": firmware == crate::cmdinfo::Firmware::Px4,
            "firmwareTypeString": match firmware {
                crate::cmdinfo::Firmware::Px4 => "PX4 Pro",
                crate::cmdinfo::Firmware::ArduPilot => "ArduPilot",
                crate::cmdinfo::Firmware::Generic => "Generic",
            },
            "vehicleTypeString": planned_type_text(crate::hub::lock().active().is_some(), vehicle_type, class),
            "homePosition": crate::hub::lock().active().and_then(|v| v.home).map(|(latitude, longitude, _)| json!({ "valid": true, "latitude": latitude, "longitude": longitude })),
        })),
        "plan.missionController.visualItems" => {
            Some(json!({ "kind": "object", "count": visual_spans(&document).len() }))
        }
        _ if camera_calc_index(path).is_some() => camera_calc_index(path)
            .and_then(|index| index.checked_sub(1))
            .and_then(|i| match document.items.get(i) {
                Some(plandoc::Item::Complex { json: survey, .. }) => {
                    crate::surveydoc::calc_of(survey)
                        .get("DistanceMode")
                        .cloned()
                }
                _ => None,
            })
            .map(|mode| json!({ "kind": "object", "distanceMode": mode })),
        "plan.missionController" => {
            let sequence = visual_spans(&document)
                .get(usize::try_from(selected).unwrap_or(0))
                .map_or(0, |(first, _)| *first);
            let rules = crate::missionkinds::Rules {
                takeoff_not_required: planning_setting("takeoffItemNotRequired", false),
                multiple_landings: planning_setting("allowMultipleLandingPatterns", true),
            };
            let state = crate::missionkinds::insert_state(
                &document,
                &visual_spans(&document),
                sequence,
                &rules,
            );
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
    let Some(mission) = crate::vfs::read_to_string(file)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|plan| plan.get("mission").cloned())
    else {
        return;
    };
    [
        ("cruiseSpeed", "offlineEditingCruiseSpeed"),
        ("hoverSpeed", "offlineEditingHoverSpeed"),
    ]
    .iter()
    .filter_map(|(key, setting)| {
        mission
            .get(*key)
            .and_then(Value::as_f64)
            .map(|speed| (setting, speed))
    })
    .for_each(|(setting, speed)| {
        backend.set(
            &format!("settings.appSettings.{setting}"),
            &json!({ "value": speed }).to_string(),
        );
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
        crate::cmdinfo::VehicleClass::Generic | crate::cmdinfo::VehicleClass::Airship => 0,
    };
    (firmware_class, vehicle_class)
}

fn plan_for_active_vehicle() {
    let active = crate::hub::lock()
        .active()
        .map(|v| offline_classes(i64::from(v.autopilot), i64::from(v.vehicle_type)));
    let Some((firmware_class, vehicle_class)) =
        active.filter(|classes| *classes != offline_types())
    else {
        return;
    };
    crate::settingsstore::set_raw_together(&[
        (
            "settings.appSettings.offlineEditingFirmwareClass",
            json!(firmware_class),
        ),
        (
            "settings.appSettings.offlineEditingVehicleClass",
            json!(vehicle_class),
        ),
    ]);
}

fn plan_for_offline_vehicle(backend: &dyn Backend) {
    if !offline() {
        return;
    }
    let Some((firmware, vehicle)) = held()
        .document
        .as_ref()
        .map(|d| (d.firmware_type, d.vehicle_type))
    else {
        return;
    };
    let (firmware_class, vehicle_class) = offline_classes(firmware, vehicle);
    backend.set(
        "settings.appSettings.offlineEditingFirmwareClass",
        &json!({ "value": firmware_class }).to_string(),
    );
    backend.set(
        "settings.appSettings.offlineEditingVehicleClass",
        &json!({ "value": vehicle_class }).to_string(),
    );
}

fn select(args: &str) -> Value {
    let sequence = serde_json::from_str::<Value>(args)
        .ok()
        .and_then(|v| v.get(0).and_then(Value::as_i64));
    let picked = {
        let mut state = held();
        let found = state
            .document
            .as_ref()
            .zip(sequence)
            .and_then(|(d, seq)| visual_index_of_sequence(d, seq));
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
    crate::read::value_number(&backend.value(&format!("settings.appSettings.{name}.rawValue")))
}

fn fresh_document(backend: &dyn Backend) -> Document {
    let offline = empty_document();
    Document {
        firmware_type: setting_number(backend, "offlineEditingFirmwareClass")
            .map_or(offline.firmware_type, |v| v as i64),
        vehicle_type: setting_number(backend, "offlineEditingVehicleClass")
            .map_or(offline.vehicle_type, |v| v as i64),
        cruise_speed: setting_number(backend, "offlineEditingCruiseSpeed")
            .unwrap_or(offline.cruise_speed),
        hover_speed: setting_number(backend, "offlineEditingHoverSpeed")
            .unwrap_or(offline.hover_speed),
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
        let current = state
            .document
            .replace(plandoc::above_terrain(&restored, &terrain_height));
        match undoing {
            true => state.redo.extend(current),
            false => state.undo.extend(current),
        }
        state.last_change_ms = 0;
        state.dirty =
            state.document.as_ref().map(plandoc::save) != state.clean.as_ref().map(plandoc::save);
        state.dirty_for_save = state.dirty;
    }
    changed();
    json!({ "ok": true, "reason": null, "refusal": null })
}

pub fn plan_types() -> Option<(i64, i64)> {
    held()
        .document
        .as_ref()
        .map(|document| (document.firmware_type, document.vehicle_type))
}

pub fn history() -> Option<(bool, bool)> {
    enabled().then(|| {
        let state = held();
        (!state.undo.is_empty(), !state.redo.is_empty())
    })
}

fn remove_all_from_vehicle(backend: &dyn Backend) -> Value {
    let Some((id, (fence, rally))) = crate::hub::lock()
        .active()
        .map(|v| (v.id, v.plans_supported()))
    else {
        return refused("No vehicle is connected through the core.");
    };
    let kinds = ["mission"]
        .into_iter()
        .chain(fence.then_some("fence"))
        .chain(rally.then_some("rally"));
    let (sent, refusals): (Vec<&'static str>, Vec<String>) = kinds.fold(
        (Vec::new(), Vec::new()),
        |(sent, refusals), kind| match crate::hub::lock().mission_request(
            Some(id),
            &json!({ "action": "removeAll", "plan": kind }),
            crate::hub::now_ms(),
        ) {
            Ok(outbound) => {
                deliver(outbound);
                ([sent, vec![kind]].concat(), refusals)
            }
            Err(reason) => (sent, [refusals, vec![reason]].concat()),
        },
    );
    let unopened = held().document.is_none();
    let fresh = unopened.then(|| fresh_document(backend));
    {
        let mut state = held();
        state.dirty = false;
        state.file = None;
    }
    changed();
    let _ = crate::background::spawn("groundstation-plan-remove", move || {
        removed_then(id, sent, Vec::new(), fresh)
    });
    match refusals.first() {
        None => json!({ "ok": true }),
        Some(reason) => refused(reason.clone()),
    }
}

fn removed_then(
    id: u8,
    kinds: Vec<&'static str>,
    removed: Vec<&'static str>,
    fresh: Option<Document>,
) {
    match kinds.split_first() {
        None if removed.is_empty() => {}
        None => clear_kinds(fresh, &removed),
        Some((&kind, later)) => {
            let later = later.to_vec();
            once_settled(
                "groundstation-plan-remove",
                move || {
                    let hub = crate::hub::lock();
                    let busy = hub.vehicle(id).is_some_and(|v| {
                        v.mission_snapshot()[kind]["inProgress"].as_bool() == Some(true)
                    });
                    (!busy).then(|| {
                        hub.vehicle(id)
                            .is_some_and(|v| v.mission_snapshot()[kind]["error"].is_null())
                    })
                },
                move |gone| {
                    removed_then(
                        id,
                        later,
                        if gone {
                            [removed, vec![kind]].concat()
                        } else {
                            removed
                        },
                        fresh,
                    )
                },
            );
        }
    }
}

fn clear(backend: &dyn Backend) -> Value {
    let unopened = held().document.is_none();
    let fresh = unopened.then(|| fresh_document(backend));
    clear_kinds(fresh, &["mission", "fence", "rally"]);
    json!({ "ok": true })
}

fn clear_kinds(fresh: Option<Document>, kinds: &[&str]) {
    forget_raw_edits();
    let types = offline_types();
    {
        let mut state = held();
        let template = state
            .document
            .clone()
            .or(fresh)
            .unwrap_or_else(empty_document);
        let before = state.document.clone();
        let empty = empty_document();
        let mission = kinds.contains(&"mission");
        let cleared = Document {
            home: if mission { None } else { template.home },
            items: if mission {
                Vec::new()
            } else {
                template.items.clone()
            },
            settings_sections: if mission {
                Vec::new()
            } else {
                template.settings_sections.clone()
            },
            fence: if kinds.contains(&"fence") {
                empty.fence
            } else {
                template.fence.clone()
            },
            rally: if kinds.contains(&"rally") {
                empty.rally
            } else {
                template.rally.clone()
            },
            ..template
        };
        state.document = Some(tracked(cleared, types));
        state.wizard = None;
        remember(&mut state, before, crate::hub::now_ms());
        state.selected = 0;
        settle_clean(&mut state);
        state.file = None;
    }
    changed();
}

pub fn launches_from_vehicle() -> bool {
    takeoff_required_first() && active_vehicle_home().is_some()
}

fn takeoff_required_first() -> bool {
    let state = held();
    state.document.as_ref().is_some_and(|document| {
        let spans = visual_spans(document);
        let sequence = spans
            .get(usize::try_from(state.selected).unwrap_or(0))
            .map_or(0, |(first, _)| *first);
        let rules = crate::missionkinds::Rules {
            takeoff_not_required: planning_setting("takeoffItemNotRequired", false),
            multiple_landings: planning_setting("allowMultipleLandingPatterns", true),
        };
        crate::missionkinds::insert_state(document, &spans, sequence, &rules).only_takeoff
    })
}

fn shape_points(file: &str, polyline: bool) -> Result<Vec<(f64, f64)>, String> {
    let lower = file.to_lowercase();
    let (points, found_polyline) = match () {
        _ if lower.ends_with(".kml") => match crate::kml::parse_wanted(
            &crate::vfs::read_to_string(file)
                .map_err(|e| format!("Unable to open file: {file} error: {e}"))?,
            polyline,
        )? {
            crate::kml::Shape::Polygon(points) => (points, false),
            crate::kml::Shape::Polyline(points) => (points, true),
        },
        _ if lower.ends_with(".shp") => crate::shp::parse_wanted(file, Some(polyline))
            .map(|(kind, _, points)| (points, kind == "polyline"))?,
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
    let centre = (
        points.iter().map(|p| p.0).sum::<f64>() / count,
        points.iter().map(|p| p.1).sum::<f64>() / count,
    );
    let inserted = insert_kind(
        backend,
        &json!([kind, centre.0, centre.1, given.get(2)]).to_string(),
    );
    let Some(placed) = inserted
        .get("index")
        .and_then(Value::as_i64)
        .filter(|_| inserted["ok"] == true)
    else {
        return inserted;
    };
    let key = if kind == "corridor" {
        "polyline"
    } else {
        "polygon"
    };
    let vertices: Vec<Value> = points.iter().map(|(lat, lon)| json!([lat, lon])).collect();
    edit(|doc| {
        let at = usize::try_from(placed - 1)
            .ok()
            .filter(|i| *i < doc.items.len())
            .ok_or("The imported item is missing.")?;
        let plandoc::Item::Complex { kind, json, .. } = &doc.items[at] else {
            return Err("The imported item is missing.".to_string());
        };
        let shaped = crate::surveydoc::regenerate_item(&with_polygon(json, key, vertices.clone()));
        let item = plandoc::Item::Complex {
            kind: kind.clone(),
            item_count: plandoc::complex_count(kind, &shaped).unwrap_or(0),
            json: shaped,
        };
        Ok(Document {
            items: doc
                .items
                .iter()
                .enumerate()
                .map(|(k, it)| if k == at { item.clone() } else { it.clone() })
                .collect(),
            ..doc.clone()
        })
    })
}

fn opened(backend: &dyn Backend) {
    let unopened = held().document.is_none();
    if unopened {
        clear(backend);
    }
}

fn insert_kind(backend: &dyn Backend, args: &str) -> Value {
    opened(backend);
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let kind = given.get(0).and_then(Value::as_str).unwrap_or("");
    let asked = given.get(3).and_then(Value::as_i64).unwrap_or(-1);
    let index = held().document.as_ref().map_or(asked, |document| {
        plandoc::past_takeoff(document, kind, asked)
    });
    let rest = json!([given.get(1), given.get(2), index]).to_string();
    let clicked = given
        .get(1)
        .and_then(Value::as_f64)
        .zip(given.get(2).and_then(Value::as_f64));
    let launches = kind == "takeoff" || (kind == "waypoint" && takeoff_required_first());
    let vehicle_home = launches.then(active_vehicle_home).flatten();
    let placed_home = (clicked.is_some() || vehicle_home.is_some()) && {
        let mut state = held();
        let homeless = state.document.as_ref().is_some_and(|d| d.home.is_none());
        state.document = state.document.take().map(|d| Document {
            home: d
                .home
                .or(vehicle_home)
                .or_else(|| plandoc::home_from_first_coordinate(&d, clicked)),
            ..d
        });
        homeless
    };
    if placed_home {
        settle_home_on_terrain(None);
    }
    let answered = match kind {
        "waypoint" if takeoff_required_first() => {
            let takeoff = insert_takeoff(backend, &json!([given.get(3)]).to_string());
            match takeoff.get("ok").and_then(Value::as_bool) {
                Some(true) => {
                    let after = given
                        .get(3)
                        .and_then(Value::as_i64)
                        .filter(|i| *i != -1)
                        .map_or(-1, |i| i + 1);
                    insert_at(
                        backend,
                        &json!([given.get(1), given.get(2), after]).to_string(),
                        false,
                    )
                }
                _ => takeoff,
            }
        }
        "waypoint" => insert_at(backend, &rest, false),
        "land" => {
            let class = held()
                .document
                .as_ref()
                .map(|d| plandoc::vehicle_class(d.vehicle_type));
            match class {
                Some(
                    crate::cmdinfo::VehicleClass::FixedWing | crate::cmdinfo::VehicleClass::Vtol,
                ) => insert_landing(backend, &rest),
                _ => insert_at(backend, &rest, true),
            }
        }
        "takeoff" => {
            let answered = insert_takeoff(backend, &json!([given.get(3)]).to_string());
            let fixed_wing = held().document.as_ref().is_some_and(|d| {
                plandoc::vehicle_class(d.vehicle_type) == crate::cmdinfo::VehicleClass::FixedWing
            });
            if answered.get("ok").and_then(Value::as_bool) == Some(true) && fixed_wing {
                let mut state = held();
                let count = state.document.as_ref().map_or(0, |d| d.items.len() as i64);
                let wanted = given.get(3).and_then(Value::as_i64).unwrap_or(-1);
                state.wizard = usize::try_from(if (1..=count).contains(&wanted) {
                    wanted
                } else {
                    count
                })
                .ok();
            }
            answered
        }
        "survey" => insert_scan(backend, &rest, false),
        "corridor" => insert_scan(backend, &rest, true),
        "structure" => insert_structure(backend, &rest),
        "roi" => insert_roi(backend, &rest),
        "drop" => insert_drop(backend, &rest),
        other => return refused(format!("The core plan cannot insert a {other} yet.")),
    };
    match answered.get("ok").and_then(Value::as_bool) {
        Some(true) => {
            let wanted = index;
            let placed = {
                let mut state = held();
                let count = state.document.as_ref().map_or(0, |d| d.items.len() as i64);
                let placed = if (1..=count).contains(&wanted) {
                    wanted
                } else {
                    count
                };
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
    let given = serde_json::from_str::<Value>(value)
        .ok()
        .and_then(|v| v.get("value").cloned());
    let number = given.as_ref().and_then(crate::factwrite::number);
    let answer = |edited: Result<Document, String>| match edited {
        Ok(changed) => edit(|_| Ok(changed)),
        Err(reason) => refused(reason),
    };
    let current = held().document.clone()?;
    if property == "coordinate" {
        if let Some(complex @ plandoc::Item::Complex { .. }) =
            index.checked_sub(1).and_then(|at| current.items.get(at))
        {
            let target = crate::fenceedit::point(given.as_ref());
            let at = index - 1;
            return Some(
                match target.and_then(|(latitude, longitude)| {
                    crate::plantransform::move_complex_to(complex, latitude, longitude)
                }) {
                    Some(moved) => edit(|doc| {
                        Ok(Document {
                            items: doc
                                .items
                                .iter()
                                .enumerate()
                                .map(|(k, it)| if k == at { moved.clone() } else { it.clone() })
                                .collect(),
                            ..doc.clone()
                        })
                    }),
                    None => refused(format!("Item {index} could not be moved there.")),
                },
            );
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
            ("cameraAction", true, Some(i)) => {
                plandoc::camera_action_value(i as usize).map(|v| json!(v))
            }
            (CAMERA_INTERVAL_DISTANCE, false, Some(shown)) if !member.ends_with(".rawValue") => {
                Some(json!(
                    crate::units::cooking("m").map_or(shown, |c| (c.base)(shown))
                ))
            }
            _ => given.clone(),
        };
        return Some(
            match wanted.and_then(|value| plandoc::set_camera(&current, index, name, &value)) {
                Some(changed) => edit(|_| Ok(changed)),
                None => refused(format!("Item {index} has no camera field {name}.")),
            },
        );
    }
    if index == 0 && property == LAUNCH_ALTITUDE {
        let metres = number.map(|shown| crate::read::Unit::vertical(backend).meters(shown));
        return Some(match (metres, current.home) {
            (Some(altitude), Some([latitude, longitude, _])) => edit(|doc| {
                Ok(Document {
                    home: Some([latitude, longitude, altitude]),
                    ..doc.clone()
                })
            }),
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
    if let Some(plandoc::Item::Complex {
        kind, json: survey, ..
    }) = index.checked_sub(1).and_then(|i| current.items.get(i))
    {
        if kind == "survey" || kind == "CorridorScan" || kind == "StructureScan" {
            let Some(value) = given.clone() else {
                return Some(refused("That field needs a value."));
            };
            let at = index - 1;
            let (vertical, horizontal) = (
                crate::read::Unit::vertical(backend),
                crate::read::Unit::horizontal(backend),
            );
            return Some(
                match crate::surveydoc::set(
                    survey,
                    property,
                    &value,
                    &crate::surveydoc::Units {
                        vertical: &vertical,
                        horizontal: &horizontal,
                    },
                ) {
                    Some(edited) => {
                        let item_count = plandoc::complex_count(kind, &edited).unwrap_or(0);
                        let item = plandoc::Item::Complex {
                            kind: kind.clone(),
                            json: edited,
                            item_count,
                        };
                        let answered = edit(|doc| {
                            Ok(Document {
                                items: doc
                                    .items
                                    .iter()
                                    .enumerate()
                                    .map(|(k, it)| if k == at { item.clone() } else { it.clone() })
                                    .collect(),
                                ..doc.clone()
                            })
                        });
                        remember_item(backend, index as i64);
                        answered
                    }
                    None if !crate::surveydoc::knows(property) => {
                        refused(format!("The survey has no field {property}."))
                    }
                    None if crate::factwrite::number(&value).is_none() => {
                        refused(crate::factwrite::INVALID_NUMBER)
                    }
                    None => refused(format!("{property} cannot take that value.")),
                },
            );
        }
    }
    let commands = crate::cmdinfo::tree(
        plandoc::firmware(current.firmware_type),
        plandoc::command_class_at(&current, index.saturating_sub(1)),
    );
    let simple = index
        .checked_sub(1)
        .and_then(|i| current.items.get(i))
        .and_then(|item| match item {
            plandoc::Item::Simple(s) => Some(s),
            plandoc::Item::Complex { .. } => None,
        });
    let raw = simple.is_some_and(|s| raw_edit(s, commands.get(&s.command), index));
    if property == "rawEdit" {
        let on = given.as_ref().and_then(Value::as_bool).unwrap_or(false);
        let mut chosen = raw_edits();
        if on {
            chosen.insert(index)
        } else {
            chosen.remove(&index)
        };
        drop(chosen);
        changed();
        return Some(json!({ "ok": true }));
    }
    let (property, number) = match property
        .strip_suffix(".enumIndex")
        .filter(|combo| combo.starts_with("comboboxFacts."))
    {
        Some(combo) => (
            combo,
            number.and_then(|at| {
                let slot = combo.strip_prefix("comboboxFacts.")?.parse().ok()?;
                let choices = match raw {
                    true => Choices::Raw,
                    false => Choices::Friendly(simple?),
                };
                combo_value(choices, &commands, slot, usize::try_from(at as i64).ok()?)
            }),
        ),
        None => (property, number),
    };
    match (raw, property, number) {
        (true, "comboboxFacts.0", Some(n)) => {
            return Some(item_edit(backend, &json!([index, n]).to_string(), true));
        }
        (true, "comboboxFacts.1", Some(n)) => {
            return Some(answer(plandoc::set_frame(&current, index, n as i64)));
        }
        _ => {}
    }
    let field = |group: &str| {
        let at: usize = property.strip_prefix(group)?.parse().ok()?;
        let s = simple?;
        match raw {
            true => (group == "textFieldFacts." && at < RAW_LABELS.len()).then_some(at + 1),
            false if group == "nanFacts." => nan_params(commands.get(&s.command)?)
                .get(at)
                .map(|(param, _)| usize::from(*param)),
            false => field_params(commands.get(&s.command)?, group == "comboboxFacts.")
                .get(at)
                .map(|(param, _)| usize::from(*param)),
        }
    };
    let unknown = || Err(format!("Item {index} has no such field."));
    let out_of_range = |param: usize, value: f64| -> Option<String> {
        let info = simple
            .and_then(|s| commands.get(&s.command))
            .and_then(|info| info.params.get(&u8::try_from(param).ok()?))?;
        let limit = |key: &str| {
            info.get(key)
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite())
        };
        let (min, max) = (limit("min"), limit("max"));
        let shown =
            |raw_limit: f64| param_conversion(info).map_or(raw_limit, |c| (c.shown)(raw_limit));
        (!raw
            && value.is_finite()
            && (min.is_some_and(|m| value < m) || max.is_some_and(|m| value > m)))
        .then(|| {
            format!(
                "Value must be within {} and {}",
                qt_number(min.map_or(f64::MIN, shown)),
                qt_number(max.map_or(f64::MAX, shown))
            )
        })
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
        "altitude" | "command" => {
            return Some(match number {
                Some(n) => {
                    let raw = if property == "altitude" {
                        crate::missionitems::altitude_from_shown(n)
                    } else {
                        n
                    };
                    item_edit(
                        backend,
                        &json!([index, raw]).to_string(),
                        property == "command",
                    )
                }
                None => refused("That field takes a number."),
            });
        }
        "altitudeMode" | "altitudeFrame" => answer(
            number
                .ok_or_else(|| "An altitude mode is a number.".to_string())
                .and_then(|mode| {
                    plandoc::set_altitude_mode_keeping_height(
                        &current,
                        index,
                        mode as i64,
                        launch_ground(&current, active_vehicle_home(), &terrain_height),
                        &terrain_height,
                    )
                }),
        ),
        "coordinate" | "launchCoordinate" => answer(
            crate::fenceedit::point(given.as_ref())
                .ok_or_else(|| {
                    "A position needs a latitude from -90 to 90 and a longitude from -180 to 180."
                        .to_string()
                })
                .and_then(|(latitude, longitude)| {
                    let moved_home = Document {
                        home: Some([latitude, longitude, current.home.map_or(0.0, |h| h[2])]),
                        ..current.clone()
                    };
                    let moved_item = |doc: &Document| {
                        plandoc::set_param(doc, index, 5, latitude)
                            .and_then(|moved| plandoc::set_param(&moved, index, 6, longitude))
                            .ok_or_else(|| format!("Item {index} has no position to move."))
                    };
                    let same_location = launch_at_takeoff(&current, index);
                    let transition_distance = crate::read::value_number(
                        &backend.value("settings.planViewSettings.vtolTransitionDistance.rawValue"),
                    )
                    .unwrap_or(plandoc::VTOL_TRANSITION_DISTANCE_DEFAULT);
                    match (index, property) {
                        (0, _) => Ok(moved_home),
                        (_, "launchCoordinate") => plandoc::set_launch(
                            &current,
                            index,
                            latitude,
                            longitude,
                            same_location,
                            transition_distance,
                        )
                        .ok_or_else(|| format!("Item {index} is not a takeoff.")),
                        (_, _) if same_location => moved_item(&moved_home),
                        _ => moved_item(&current),
                    }
                }),
        ),
        "loiterRadius" => answer(
            number
                .ok_or_else(|| "A radius is a number.".to_string())
                .and_then(|v| {
                    plandoc::set_loiter_radius(&current, index, v)
                        .ok_or_else(|| format!("Item {index} is not a loiter."))
                }),
        ),
        "yaw" => answer(
            plandoc::set_param(
                &current,
                index,
                usize::from(YAW_PARAM),
                number.filter(|v| v.is_finite()).unwrap_or(f64::NAN),
            )
            .ok_or_else(|| format!("Item {index} has no heading.")),
        ),
        "hold" => answer(
            number
                .filter(|v| v.is_finite() && *v >= 0.0)
                .ok_or_else(|| "A hold is a number of seconds, 0 or more.".to_string())
                .and_then(|v| {
                    plandoc::set_param(&current, index, usize::from(HOLD_PARAM), v)
                        .ok_or_else(|| format!("Item {index} has no hold."))
                }),
        ),
        "speedSection.flightSpeed" => answer(
            number
                .map(|shown| crate::units::cooking("m/s").map_or(shown, |c| (c.base)(shown)))
                .ok_or_else(|| "A speed is a number.".to_string())
                .and_then(|v| {
                    plandoc::set_speed(&current, index, Some(v))
                        .ok_or_else(|| format!("Item {index} carries no speed."))
                }),
        ),
        "speedSection.specifyFlightSpeed" => {
            let on = given.as_ref().and_then(Value::as_bool).unwrap_or(false);
            let keep = plandoc::specified_speed(&current, index);
            let speeds = planning_speeds(backend);
            let default = speed_in_force(
                &current,
                index.saturating_sub(1),
                speeds.hover,
                speeds.cruise,
            );
            let speed = on.then(|| keep.unwrap_or(default));
            answer(
                plandoc::set_speed(&current, index, speed)
                    .ok_or_else(|| format!("Item {index} carries no speed.")),
            )
        }
        _ if property.starts_with("nanFacts.") => match field("nanFacts.") {
            Some(param) => match number.and_then(|v| out_of_range(param, raw_of(param, v))) {
                Some(reason) => refused(reason),
                None => answer(
                    plandoc::set_param(
                        &current,
                        index,
                        param,
                        number.map_or(f64::NAN, |v| raw_of(param, v)),
                    )
                    .ok_or_else(|| format!("Item {index} has no such field.")),
                ),
            },
            None => answer(unknown()),
        },
        _ => match (
            field("textFieldFacts.").or_else(|| field("comboboxFacts.")),
            number,
        ) {
            (Some(param), Some(v))
                if !raw
                    && param == plandoc::JUMP_TARGET_PARAM
                    && simple.is_some_and(|s| s.command == plandoc::JUMP_COMMAND) =>
            {
                answer(
                    jump_sequence(&current, v)
                        .ok_or_else(|| NO_SUCH_WAYPOINT.to_string())
                        .and_then(|sequence| {
                            plandoc::set_param(&current, index, param, sequence)
                                .ok_or_else(|| format!("Item {index} has no such field."))
                        }),
                )
            }
            (Some(param), Some(v)) => match out_of_range(param, raw_of(param, v)) {
                Some(reason) => refused(reason),
                None => answer(
                    plandoc::set_param(&current, index, param, raw_of(param, v))
                        .ok_or_else(|| format!("Item {index} has no such field.")),
                ),
            },
            (Some(param), None)
                if raw
                    && param <= OPTIONAL_RAW_PARAMS
                    && given.as_ref().is_none_or(|entry| {
                        entry.is_null() || entry.as_str().is_some_and(|text| text.trim().is_empty())
                    }) =>
            {
                answer(
                    plandoc::clear_param(&current, index, param)
                        .ok_or_else(|| format!("Item {index} has no such field.")),
                )
            }
            (Some(_), None) => refused("That field takes a number."),
            (None, _) => answer(unknown()),
        },
    })
}

const NO_SUCH_WAYPOINT: &str =
    "This plan has no waypoint with that number. A jump goes to a numbered waypoint.";
const JUMP_TARGET_LABEL: &str = "Waypoint";

fn plan_stops(doc: &Document) -> Vec<Option<i64>> {
    crate::missionitems::document_reads(doc, -1)
        .map(|reads| crate::missionitems::stops(&reads))
        .unwrap_or_default()
}

fn jump_sequence(doc: &Document, stop: f64) -> Option<f64> {
    let row = plan_stops(doc)
        .iter()
        .position(|numbered| numbered.is_some_and(|number| number as f64 == stop))?;
    plandoc::first_sequence(doc, row).map(|sequence| sequence as f64)
}

fn jump_stop(doc: &Document, sequence: f64) -> Option<f64> {
    let row = plandoc::row_of_sequence(doc, sequence)?;
    plan_stops(doc)
        .get(row)
        .copied()
        .flatten()
        .map(|number| number as f64)
}

pub(crate) fn qt_number(value: f64) -> String {
    let scientific = format!("{value:.5e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .unwrap_or((scientific.as_str(), "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let trimmed = |text: &str| match text.contains('.') {
        true => text.trim_end_matches('0').trim_end_matches('.').to_string(),
        false => text.to_string(),
    };
    match (-4..6).contains(&exponent) {
        true => trimmed(&format!("{:.*}", (5 - exponent).max(0) as usize, value)),
        false => format!(
            "{}e{}{:02}",
            trimmed(mantissa),
            if exponent < 0 { '-' } else { '+' },
            exponent.abs()
        ),
    }
}

fn point_of(value: Option<&Value>) -> Option<(f64, f64)> {
    let v = value?;
    let lat = v
        .get("latitude")?
        .as_f64()
        .filter(|l| l.is_finite() && (-90.0..=90.0).contains(l))?;
    let lon = v
        .get("longitude")?
        .as_f64()
        .filter(|l| l.is_finite() && (-180.0..=180.0).contains(l))?;
    Some((lat, lon))
}

fn fence_edit(
    change: impl FnOnce(&Value, &Value) -> Option<(Value, Value)>,
    refusal: &str,
) -> Value {
    let refusal = refusal.to_string();
    edit(move |doc| {
        change(&doc.fence, &doc.rally)
            .map(|(fence, rally)| Document {
                fence,
                rally,
                ..doc.clone()
            })
            .ok_or(refusal)
    })
}

fn indexed(path: &str, prefix: &str) -> Option<(usize, String)> {
    let (index, member) = path.strip_prefix(prefix)?.split_once('.')?;
    Some((index.parse().ok()?, member.to_string()))
}

fn fence_invoke(backend: &dyn Backend, path: &str, args: &str) -> Option<Value> {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let first_index = || given.get(0).and_then(Value::as_u64).map(|i| i as usize);
    let window = || {
        point_of(given.get(0))
            .zip(point_of(given.get(1)))
            .filter(|((north, west), (south, east))| north > south && east != west)
    };
    Some(match path {
        "plan.geoFenceController.addInclusionPolygon" => match window() {
            Some((tl, br)) => {
                opened(backend);
                fence_edit(
                    |f, r| Some((crate::fencedoc::add_polygon(f, tl, br), r.clone())),
                    "",
                )
            }
            None => {
                refused("A new fence needs the map window's top-left and bottom-right corners.")
            }
        },
        "plan.geoFenceController.addInclusionCircle" => match window() {
            Some((tl, br)) => {
                opened(backend);
                fence_edit(
                    |f, r| Some((crate::fencedoc::add_circle(f, tl, br), r.clone())),
                    "",
                )
            }
            None => {
                refused("A new fence needs the map window's top-left and bottom-right corners.")
            }
        },
        "plan.geoFenceController.deletePolygon" | "plan.geoFenceController.deleteCircle" => {
            let key = if path.ends_with("Polygon") {
                "polygons"
            } else {
                "circles"
            };
            fence_edit(
                |f, r| Some((crate::fencedoc::delete(f, key, first_index()?)?, r.clone())),
                "There is no fence shape at that position.",
            )
        }
        "plan.rallyPointController.addPoint" => match point_of(given.get(0)) {
            Some(at) => {
                opened(backend);
                let altitude = crate::battery::return_altitude(backend).unwrap_or_else(|| {
                    crate::read::value_number(
                        &backend.value(&format!("{DEFAULT_ALTITUDE}.rawValue")),
                    )
                    .unwrap_or(0.0)
                });
                fence_edit(
                    |f, r| Some((f.clone(), crate::fencedoc::add_rally(r, at, altitude))),
                    "",
                )
            }
            None => refused("A rally point needs a latitude and a longitude."),
        },
        "plan.rallyPointController.removePoint" => {
            let index = given.get(0).and_then(Value::as_str).and_then(|r| {
                r.strip_prefix("@plan.rallyPointController.points.")?
                    .parse::<usize>()
                    .ok()
            });
            fence_edit(
                |f, r| Some((f.clone(), crate::fencedoc::remove_rally(r, index?)?)),
                "There is no rally point at that position.",
            )
        }
        _ => {
            let (polygon, member) = indexed(path, "plan.geoFenceController.polygons.")?;
            let vertex = first_index();
            match member.as_str() {
                "adjustVertex" => {
                    let at = point_of(given.get(1));
                    fence_edit(
                        |f, r| {
                            Some((
                                crate::fencedoc::move_vertex(f, polygon, vertex?, at?)?,
                                r.clone(),
                            ))
                        },
                        "That vertex cannot be moved there.",
                    )
                }
                "removeVertex" => fence_edit(
                    |f, r| {
                        Some((
                            crate::fencedoc::remove_vertex(f, polygon, vertex?)?,
                            r.clone(),
                        ))
                    },
                    "A fence polygon keeps at least three vertices.",
                ),
                _ => return None,
            }
        }
    })
}

fn fence_set(backend: &dyn Backend, path: &str, value: &str) -> Option<Value> {
    let given = serde_json::from_str::<Value>(value)
        .ok()
        .and_then(|v| v.get("value").cloned())
        .unwrap_or(Value::Null);
    let altitude = given
        .get("altitude")
        .and_then(Value::as_f64)
        .filter(|a| a.is_finite());
    Some(match path {
        "plan.geoFenceController.breachReturnPoint" => match point_of(Some(&given)) {
            Some(at) => {
                let default = held().breach_altitude.or_else(|| {
                    crate::read::value_number(
                        &backend.value(&format!("{DEFAULT_ALTITUDE}.rawValue")),
                    )
                });
                fence_edit(
                    |f, r| {
                        let kept = f
                            .get("breachReturn")
                            .and_then(|b| b.get(2))
                            .and_then(Value::as_f64)
                            .or(default);
                        Some((crate::fencedoc::set_breach_return(f, at, kept), r.clone()))
                    },
                    "",
                )
            }
            None if given.is_null() => fence_edit(
                |f, r| Some((crate::fencedoc::clear_breach_return(f), r.clone())),
                "",
            ),
            None => refused("A breach return point needs a latitude and a longitude."),
        },
        "plan.geoFenceController.breachReturnAltitude" => {
            let Some(metres) = given
                .as_f64()
                .filter(|a| a.is_finite())
                .map(|shown| crate::read::Unit::vertical(backend).meters(shown))
            else {
                return Some(refused("A breach return altitude is a number."));
            };
            held().breach_altitude = Some(metres);
            let placed = held().document.as_ref().is_some_and(|d| {
                d.fence
                    .get("breachReturn")
                    .and_then(Value::as_array)
                    .is_some_and(|b| b.len() >= 2)
            });
            match placed {
                true => fence_edit(
                    |f, r| Some((crate::fencedoc::set_breach_altitude(f, metres)?, r.clone())),
                    "",
                ),
                false => {
                    changed();
                    json!({ "ok": true })
                }
            }
        }
        _ => {
            if let Some((index, member)) = indexed(path, "plan.geoFenceController.polygons.") {
                return (member == "inclusion").then(|| {
                    fence_edit(
                        |f, r| {
                            Some((
                                crate::fencedoc::set_inclusion(
                                    f,
                                    "polygons",
                                    index,
                                    given.as_bool()?,
                                )?,
                                r.clone(),
                            ))
                        },
                        "A polygon is an inclusion (true) or an exclusion (false).",
                    )
                });
            }
            if let Some((index, member)) = indexed(path, "plan.geoFenceController.circles.") {
                return Some(match member.as_str() {
                    "inclusion" => fence_edit(
                        |f, r| {
                            Some((
                                crate::fencedoc::set_inclusion(
                                    f,
                                    "circles",
                                    index,
                                    given.as_bool()?,
                                )?,
                                r.clone(),
                            ))
                        },
                        "A circle is an inclusion (true) or an exclusion (false).",
                    ),
                    "center" => fence_edit(
                        |f, r| {
                            Some((
                                crate::fencedoc::set_circle(
                                    f,
                                    index,
                                    Some(point_of(Some(&given))?),
                                    None,
                                )?,
                                r.clone(),
                            ))
                        },
                        "A circle's center needs a latitude and a longitude.",
                    ),
                    "radius" => {
                        let horizontal = crate::read::Unit::horizontal(backend);
                        let metres = given.as_f64().map(|shown| horizontal.meters(shown));
                        fence_edit(
                            |f, r| {
                                Some((
                                    crate::fencedoc::set_circle(
                                        f,
                                        index,
                                        None,
                                        Some(metres.filter(|v| *v >= 0.1)?),
                                    )?,
                                    r.clone(),
                                ))
                            },
                            "A circle's radius is at least 0.1 m.",
                        )
                    }
                    _ => return None,
                });
            }
            let (index, member) = indexed(path, "plan.rallyPointController.points.")?;
            match member.as_str() {
                "coordinate" => fence_edit(
                    |f, r| {
                        Some((
                            f.clone(),
                            crate::fencedoc::move_rally(
                                r,
                                index,
                                point_of(Some(&given))?,
                                altitude,
                            )?,
                        ))
                    },
                    "A rally point needs a latitude and a longitude.",
                ),
                "textFieldFacts.2" => fence_edit(
                    |f, r| {
                        let point = r["points"].get(index)?;
                        let at = (point.get(0)?.as_f64()?, point.get(1)?.as_f64()?);
                        Some((
                            f.clone(),
                            crate::fencedoc::move_rally(
                                r,
                                index,
                                at,
                                given.as_f64().map(|shown| {
                                    crate::read::Unit::vertical(backend).meters(shown)
                                }),
                            )?,
                        ))
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
    if let Some(index) = path
        .strip_prefix("plan.missionController.visualItems.")
        .and_then(|r| {
            r.strip_suffix(".surveyAreaPolygon")
                .or_else(|| r.strip_suffix(".structurePolygon"))
        })
    {
        return index.parse().ok().map(Shape::Survey);
    }
    if let Some(index) = path
        .strip_prefix("plan.missionController.visualItems.")
        .and_then(|r| r.strip_suffix(".corridorPolyline"))
    {
        return index.parse().ok().map(Shape::Corridor);
    }
    path.strip_prefix("plan.geoFenceController.polygons.")
        .and_then(|r| r.parse().ok())
        .map(Shape::Fence)
}

fn vertex_edit(vertices: &[Value], member: &str, given: &Value, ring: bool) -> Option<Vec<Value>> {
    let at = |i: usize| {
        Some((
            vertices.get(i)?.get(0)?.as_f64()?,
            vertices.get(i)?.get(1)?.as_f64()?,
        ))
    };
    let index = || given.get(0).and_then(Value::as_u64).map(|i| i as usize);
    let spelled = |(lat, lon): (f64, f64)| json!([lat, lon]);
    match member {
        "appendVertex" => Some(
            vertices
                .iter()
                .cloned()
                .chain(std::iter::once(spelled(point_of(given.get(0))?)))
                .collect(),
        ),
        "appendVertices" => {
            let added: Option<Vec<Value>> = given
                .get(0)?
                .as_array()?
                .iter()
                .map(|point| point_of(Some(point)).map(spelled))
                .collect();
            Some(vertices.iter().cloned().chain(added?).collect())
        }
        "clear" => Some(Vec::new()),
        "adjustVertex" => {
            let (i, to) = (index()?, point_of(given.get(1))?);
            (i < vertices.len()).then(|| {
                vertices
                    .iter()
                    .enumerate()
                    .map(|(k, v)| if k == i { spelled(to) } else { v.clone() })
                    .collect()
            })
        }
        "removeVertex" => {
            let i = index()?;
            (i < vertices.len() && vertices.len() > if ring { 3 } else { 2 }).then(|| {
                vertices
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| *k != i)
                    .map(|(_, v)| v.clone())
                    .collect()
            })
        }
        "splitPolygonSegment" | "splitSegment" => {
            let i = index()?;
            (ring || i + 1 < vertices.len()).then_some(())?;
            let next = if i + 1 >= vertices.len() { 0 } else { i + 1 };
            let (from, to) = (at(i)?, at(next)?);
            let middle = crate::surveygrid::at_distance_and_azimuth(
                from,
                crate::surveygrid::distance_between(from, to) / 2.0,
                crate::surveygrid::azimuth_to(from, to),
            );
            let place = if next == 0 { vertices.len() } else { next };
            Some(
                vertices[..place]
                    .iter()
                    .cloned()
                    .chain(std::iter::once(spelled(middle)))
                    .chain(vertices[place..].iter().cloned())
                    .collect(),
            )
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
        if let Some((token, reason)) =
            crate::itemshape::split_refusal(count, ring, given.get(0).and_then(Value::as_i64))
        {
            return Some(json!({ "ok": false, "refusal": token, "reason": reason }));
        }
    }
    let refusal = "That vertex edit does not fit this shape.";
    Some(match shape {
        Shape::Fence(index) => fence_edit(
            |f, r| {
                let polygons = f.get("polygons").and_then(Value::as_array)?;
                let polygon = polygons.get(index)?;
                let edited = with_polygon(
                    polygon,
                    "polygon",
                    vertex_edit(polygon.get("polygon")?.as_array()?, member, &given, true)?,
                );
                let replaced: Vec<Value> = polygons
                    .iter()
                    .enumerate()
                    .map(|(k, p)| {
                        if k == index {
                            edited.clone()
                        } else {
                            p.clone()
                        }
                    })
                    .collect();
                Some((with_polygon(f, "polygons", replaced), r.clone()))
            },
            refusal,
        ),
        Shape::Survey(index) | Shape::Corridor(index) => edit(|doc| {
            let (key, ring) = match shape_of(owner) {
                Some(Shape::Corridor(_)) => ("polyline", false),
                _ => ("polygon", true),
            };
            let at = index
                .checked_sub(1)
                .filter(|i| *i < doc.items.len())
                .ok_or(refusal)?;
            let plandoc::Item::Complex { kind, json, .. } = &doc.items[at] else {
                return Err(refusal.to_string());
            };
            let vertices = json
                .get(key)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let edited = crate::surveydoc::regenerate_item(&with_polygon(
                json,
                key,
                vertex_edit(&vertices, member, &given, ring).ok_or(refusal)?,
            ));
            let item_count = plandoc::complex_count(kind, &edited).unwrap_or(0);
            let item = plandoc::Item::Complex {
                kind: kind.clone(),
                json: edited,
                item_count,
            };
            Ok(Document {
                items: doc
                    .items
                    .iter()
                    .enumerate()
                    .map(|(k, it)| if k == at { item.clone() } else { it.clone() })
                    .collect(),
                ..doc.clone()
            })
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
            plandoc::Item::Complex { kind, json, .. } if kind == "CorridorScan" => {
                return Some(
                    crate::corridorscan::corridor_polygon(json)
                        .into_iter()
                        .map(|(latitude, longitude, _)| (latitude, longitude))
                        .collect(),
                );
            }
            plandoc::Item::Complex { json, .. } => json.get("polygon")?.as_array()?.clone(),
            plandoc::Item::Simple(_) => return None,
        },
        Shape::Corridor(index) => match document.items.get(index.checked_sub(1)?)? {
            plandoc::Item::Complex { json, .. } => json.get("polyline")?.as_array()?.clone(),
            plandoc::Item::Simple(_) => return None,
        },
        Shape::Fence(index) => document
            .fence
            .get("polygons")?
            .get(index)?
            .get("polygon")?
            .as_array()?
            .clone(),
    };
    Some(
        vertices
            .iter()
            .filter_map(|v| Some((v.get(0)?.as_f64()?, v.get(1)?.as_f64()?)))
            .collect(),
    )
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
    if let Some(index) = path
        .strip_prefix("plan.missionController.visualItems.")
        .and_then(|rest| rest.strip_suffix(".setLandingHeadingToTakeoffHeading"))
        .and_then(|i| i.parse::<usize>().ok())
    {
        return Some(edit(|doc| {
            let at = index
                .checked_sub(1)
                .filter(|i| *i < doc.items.len())
                .ok_or("No such item.")?;
            let plandoc::Item::Complex {
                kind,
                json,
                item_count,
            } = &doc.items[at]
            else {
                return Err("Only a landing pattern has a landing heading.".to_string());
            };
            let Some(heading) = takeoff_heading(doc) else {
                return Ok(doc.clone());
            };
            let turned = crate::landingpattern::edit(json, "landingHeading", &json!(heading))
                .ok_or("Only a landing pattern has a landing heading.")?;
            let item = plandoc::Item::Complex {
                kind: kind.clone(),
                item_count: plandoc::complex_count(kind, &turned).unwrap_or(*item_count),
                json: turned,
            };
            Ok(Document {
                items: doc
                    .items
                    .iter()
                    .enumerate()
                    .map(|(k, it)| if k == at { item.clone() } else { it.clone() })
                    .collect(),
                ..doc.clone()
            })
        }));
    }
    if let Some(index) = path
        .strip_prefix("plan.missionController.visualItems.")
        .and_then(|rest| rest.strip_suffix(".rotateEntryPoint"))
        .and_then(|i| i.parse::<usize>().ok())
    {
        return Some(edit(|doc| {
            let at = index
                .checked_sub(1)
                .filter(|i| *i < doc.items.len())
                .ok_or("No such item.")?;
            let plandoc::Item::Complex {
                kind,
                json,
                item_count,
            } = &doc.items[at]
            else {
                return Err("Only a pattern has an entry point.".to_string());
            };
            let rotated = crate::surveydoc::rotated_entry(kind, json)
                .ok_or("This pattern has no entry point to rotate.")?;
            let item = plandoc::Item::Complex {
                kind: kind.clone(),
                item_count: plandoc::complex_count(kind, &rotated).unwrap_or(*item_count),
                json: rotated,
            };
            Ok(Document {
                items: doc
                    .items
                    .iter()
                    .enumerate()
                    .map(|(k, it)| if k == at { item.clone() } else { it.clone() })
                    .collect(),
                ..doc.clone()
            })
        }));
    }
    let current = || held().file.clone();
    Some(match path {
        "plan.loadFromFile" => first_text(args).map_or_else(
            || refused("Open needs the path of a .plan file."),
            |file| {
                let opened = open(backend, &file);
                if let Some(reason) = opened["reason"].as_str().filter(|_| opened["ok"] != true) {
                    crate::noticeboard::post(
                        crate::noticeboard::MESSAGE,
                        "",
                        &load_failed_text(&file, reason),
                    );
                }
                plan_for_offline_vehicle(backend);
                plan_speeds(backend, &file);
                if let Some(document) = held().document.clone() {
                    remember_patterns(backend, document.vehicle_type, document.items.iter());
                }
                opened
            },
        ),
        "plan.saveToFile" => first_text(args).map_or_else(
            || refused("Save needs a path to write the plan to."),
            |file| save(&file),
        ),
        "plan.saveToKml" => first_text(args).map_or_else(
            || refused("Export needs a path to write the KML to."),
            |file| save_kml(&file),
        ),
        "plan.saveToCurrent" => current().map_or_else(
            || refused("This plan has not been saved to a file yet."),
            |file| save(&file),
        ),
        "plan.sendToVehicle" if !carried() && crate::qthost::present() => {
            send_through_host(backend)
        }
        "plan.loadFromVehicle" if !carried() && crate::qthost::present() => {
            fetch_through_host(backend)
        }
        "plan.sendToVehicle" => send(),
        "plan.loadFromVehicle" => fetch(),
        "plan.removeAllFromVehicle" if !carried() && crate::qthost::present() => return None,
        "plan.removeAllFromVehicle" => remove_all_from_vehicle(backend),
        "plan.removeAll" => clear(backend),
        "mission.insert" => insert_kind(backend, args),
        "plan.missionController.insertComplexMissionItemFromKMLOrSHP" => {
            insert_from_shape_file(backend, args)
        }
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

    struct Flown(Vec<(&'static str, f64, &'static str)>);

    impl Backend for Flown {
        fn get(&self, path: &str) -> String {
            let name = path
                .trim_start_matches("vehicle.parameterManager.getParameter(-1,")
                .trim_start_matches("settings.appSettings.")
                .trim_end_matches(".rawValue")
                .trim_end_matches(')');
            self.0
                .iter()
                .find(|(n, ..)| *n == name)
                .map_or(json!({ "kind": "null" }), |(n, v, units)| {
                    json!({ "kind": "fact", "name": n, "value": v, "rawValue": v, "rawUnits": units })
                })
                .to_string()
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            self.get(path)
        }
        fn set(&self, _path: &str, _value: &str) -> String {
            String::new()
        }
        fn invoke(&self, _path: &str, _args: &str) -> String {
            String::new()
        }
        fn watch(&self, _paths: &[String]) {}
    }

    #[test]
    fn an_offline_plan_names_its_vehicle_by_the_class_the_vehicle_setting_offers() {
        assert_eq!(
            planned_type_text(false, 2, crate::cmdinfo::VehicleClass::MultiRotor),
            "Multi-Rotor",
            "offline, type 2 is the Multi-Rotor choice, not a Quadrotor beside a dropdown saying Multi-rotor"
        );
        assert_eq!(
            planned_type_text(true, 2, crate::cmdinfo::VehicleClass::MultiRotor),
            "Quadrotor",
            "a connected vehicle is named by what it reports"
        );
    }

    #[test]
    fn a_connected_vehicle_plans_at_the_speeds_it_flies() {
        let offline = Flown(vec![
            ("offlineEditingHoverSpeed", 5.0, "m/s"),
            ("offlineEditingCruiseSpeed", 15.0, "m/s"),
        ]);
        let copter = Flown(
            offline
                .0
                .iter()
                .copied()
                .chain([
                    ("WPNAV_SPEED", 1000.0, "cm/s"),
                    ("WPNAV_SPEED_UP", 250.0, "cm/s"),
                    ("WPNAV_SPEED_DN", 150.0, "cm/s"),
                ])
                .collect(),
        );
        let px4 = Flown(vec![
            ("MPC_XY_CRUISE", 12.0, "m/s"),
            ("FW_AIRSPD_TRIM", 18.0, "m/s"),
        ]);
        let pick = |b: &Flown| {
            let s = planning_speeds(b);
            (s.hover, s.cruise, s.ascent, s.descent)
        };
        assert_eq!(
            pick(&offline),
            (5.0, 15.0, 3.0, 1.0),
            "with no vehicle the offline editing speeds plan the mission"
        );
        assert_eq!(
            pick(&copter),
            (10.0, 15.0, 2.5, 1.5),
            "ArduCopter flies its waypoints at WPNAV_SPEED, in centimetres per second"
        );
        assert_eq!(pick(&px4), (12.0, 18.0, 3.0, 1.0));
    }

    #[test]
    fn no_statement_holds_the_plan_lock_while_it_calls_into_the_backend() {
        let body = include_str!("coreplan.rs")
            .split("#[cfg(test)]\nmod tests")
            .next()
            .unwrap_or("");
        let held_across: Vec<&str> = body
            .lines()
            .filter(|line| line.contains("held()") && line.contains("(backend"))
            .collect();
        assert!(
            held_across.is_empty(),
            "a held() temporary lives to the end of its statement, and a backend read can route back into controller_fields, which takes the same lock: {held_across:?}"
        );
    }

    #[test]
    fn a_download_that_failed_is_never_adopted_over_the_plan() {
        assert!(read_succeeded(
            &json!({ "mission": { "inProgress": false, "error": null, "items": [] } }),
            "mission"
        ));
        assert!(
            !read_succeeded(
                &json!({ "mission": { "inProgress": false, "error": "Link lost", "items": [] } }),
                "mission"
            ),
            "a dropped link leaves no items, which must not replace the plan"
        );
    }

    #[test]
    fn a_plan_sequence_is_dropped_when_the_active_vehicle_changes_like_plan_master_controller() {
        assert_eq!(
            transfer_outcome(Some(1), 1, true),
            None,
            "keeps waiting while its vehicle transfers"
        );
        assert_eq!(
            transfer_outcome(Some(1), 1, false),
            Some(true),
            "moves on once the transfer settles"
        );
        assert_eq!(
            transfer_outcome(Some(2), 1, true),
            Some(false),
            "_activeVehicleChanged sets the sequence Idle, so the old plan never reaches vehicle 2"
        );
        assert_eq!(
            transfer_outcome(None, 1, false),
            Some(false),
            "nor continues with no vehicle"
        );
    }

    #[test]
    fn the_photo_distance_is_shown_in_the_horizontal_units_like_qgcs_metre_fact() {
        let doc = plandoc::set_camera(
            &plandoc::set_camera(&empty_document(), 0, "cameraAction", &json!(2)).unwrap(),
            0,
            CAMERA_INTERVAL_DISTANCE,
            &json!(30.48),
        )
        .unwrap();
        let section = cooked_camera_distance(
            plandoc::camera_section(&doc.settings_sections),
            crate::units::cooking_with("m", |_| None, crate::units::IMPERIAL_US),
        );
        let distance = section["facts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["property"] == CAMERA_INTERVAL_DISTANCE)
            .unwrap()
            .clone();
        assert!(
            (distance["value"].as_f64().unwrap() - 100.0).abs() < 1e-9,
            "{distance}"
        );
        assert_eq!(distance["units"], "ft");
        assert_eq!(distance["valueString"], "100.0");
        let time = section["facts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["property"] == "cameraPhotoIntervalTime")
            .unwrap()
            .clone();
        assert_eq!(time["units"], "secs", "only the distance is a metre fact");
    }

    #[test]
    fn the_speed_for_time_between_shots_follows_the_flight_status_calculator() {
        let simple = |command: i64, speed: f64| {
            plandoc::Item::Simple(plandoc::Simple {
                command,
                frame: 2,
                params: [
                    Some(1.0),
                    Some(speed),
                    Some(-1.0),
                    Some(0.0),
                    None,
                    None,
                    None,
                ],
                auto_continue: true,
                altitude: None,
                sections: vec![],
            })
        };
        let doc = |vehicle_type: i64, items: Vec<plandoc::Item>| Document {
            vehicle_type,
            items,
            ..empty_document()
        };
        assert_eq!(
            speed_in_force(&doc(22, vec![]), 0, 5.0, 15.0),
            5.0,
            "a VTOL starts at the hover speed"
        );
        assert_eq!(
            speed_in_force(&doc(1, vec![]), 0, 5.0, 15.0),
            15.0,
            "a plane at cruise"
        );
        assert_eq!(
            speed_in_force(&doc(2, vec![simple(178, 10.0)]), 1, 5.0, 15.0),
            10.0,
            "a standalone DO_CHANGE_SPEED counts"
        );
        assert_eq!(
            speed_in_force(&doc(2, vec![simple(178, -1.0)]), 1, 5.0, 15.0),
            5.0,
            "one leaving the speed alone does not"
        );
        let later = doc(2, vec![simple(178, 10.0), simple(16, 0.0)]);
        assert_eq!(
            speed_section(&later, 2, &[], true, 5.0, 15.0)["value"],
            10.0,
            "SimpleMissionItem::setMissionFlightStatus seeds an unset, available speed with the speed in force there"
        );
        assert_eq!(
            speed_section(&later, 2, &[], true, 5.0, 15.0)["slider"],
            json!({ "from": 0.0, "to": 30.0, "decimals": 1 }),
            "SpeedSection.FactMetaData.json FlightSpeed userMin 0, userMax 30"
        );
        assert_eq!(
            speed_section(&doc(22, vec![simple(16, 0.0)]), 1, &[], true, 5.0, 15.0)["available"],
            false,
            "SpeedSection::setAvailable only takes multirotors and fixed wings, so a VTOL never offers Flight Speed"
        );
    }

    #[test]
    fn a_vtol_item_shows_the_fields_of_the_mode_it_flies_in() {
        let text = r#"{"fileType":"Plan","version":1,"groundStation":"QGroundControl","mission":{"version":2,"firmwareType":12,"vehicleType":22,"cruiseSpeed":15,"hoverSpeed":5,"plannedHomePosition":[47.0,8.0,500],"items":[{"type":"SimpleItem","autoContinue":true,"command":22,"doJumpId":1,"frame":3,"params":[15,0,0,null,47.001,8.0,50]}]},"geoFence":{"version":2,"polygons":[],"circles":[]},"rallyPoints":{"version":2,"points":[]}}"#;
        let doc = plandoc::load(text, 22).unwrap();
        assert_eq!(
            plandoc::command_class_at(&doc, 0),
            crate::cmdinfo::VehicleClass::FixedWing,
            "MissionFlightStatusCalculator starts a VTOL without a VTOL takeoff in fixed-wing mode"
        );
        let metres = crate::read::Unit {
            name: "m".to_string(),
            factor: 1.0,
        };
        let facts = document_facts(&doc, 1, 5.0, 15.0, (&metres, &metres));
        let labels: Vec<String> = facts["fields"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|f| f["label"].as_str().map(str::to_string))
            .collect();
        assert!(
            labels.iter().any(|l| l.contains("Pitch")),
            "so its NAV_TAKEOFF shows the fixed-wing Pitch field: {labels:?}"
        );
        let simple = |command: i64| {
            plandoc::Item::Simple(plandoc::Simple {
                command,
                frame: 3,
                params: [Some(0.0); 7],
                auto_continue: true,
                altitude: None,
                sections: vec![],
            })
        };
        let mixed = Document {
            vehicle_type: 22,
            items: vec![
                simple(16),
                simple(84),
                simple(16),
                simple(22),
                simple(20),
                simple(16),
            ],
            ..empty_document()
        };
        assert_eq!(
            plandoc::command_class_at(&mixed, 0),
            crate::cmdinfo::VehicleClass::FixedWing,
            "the start follows the LAST takeoff before the RTL, here a fixed-wing NAV_TAKEOFF"
        );
        assert_eq!(
            plandoc::command_class_at(&mixed, 2),
            crate::cmdinfo::VehicleClass::FixedWing,
            "a VTOL takeoff leaves fixed-wing mode behind it"
        );
        assert_eq!(
            plandoc::command_class_at(&mixed, 5),
            crate::cmdinfo::VehicleClass::Vtol,
            "items from the RTL on get no flight status, so they keep the VTOL tree"
        );
    }

    #[test]
    fn launch_ground_is_the_vehicle_home_or_terrain_not_the_planned_altitude() {
        let moved = Document {
            home: Some([47.1, 8.1, 481.0]),
            ..empty_document()
        };
        let terrain = |_: f64, _: f64| Some(300.0);
        let unknown = |_: f64, _: f64| None;
        assert_eq!(
            launch_ground(&moved, Some([47.1, 8.1, 485.0]), &terrain),
            Some(300.0),
            "the terrain under home is what the plan shows as the launch once it arrives"
        );
        assert_eq!(
            launch_ground(&moved, Some([47.1, 8.1, 485.0]), &unknown),
            Some(485.0),
            "without terrain, a plan launching from the vehicle's home uses the altitude the vehicle reported"
        );
        assert_eq!(
            launch_ground(&moved, Some([47.0, 8.0, 485.0]), &unknown),
            None,
            "a moved home keeps the old site's 481 m until its own terrain arrives, so it is not known yet"
        );
        assert_eq!(launch_ground(&empty_document(), None, &terrain), None);
    }

    #[test]
    fn a_multirotor_takeoff_at_home_moves_with_it_and_a_plane_does_not() {
        let takeoff = |lat: f64| {
            plandoc::Item::Simple(plandoc::Simple {
                command: 22,
                frame: 3,
                params: [
                    Some(0.0),
                    Some(0.0),
                    Some(0.0),
                    None,
                    Some(lat),
                    Some(8.0),
                    Some(30.0),
                ],
                auto_continue: true,
                altitude: None,
                sections: vec![],
            })
        };
        let copter = Document {
            vehicle_type: 2,
            firmware_type: 12,
            home: Some([47.0, 8.0, 0.0]),
            items: vec![takeoff(47.0)],
            ..empty_document()
        };
        assert!(
            launch_at_takeoff(&copter, 1),
            "TakeoffMissionItem keeps launch and takeoff together for a multirotor whose takeoff sits on home"
        );
        assert!(
            !launch_at_takeoff(
                &Document {
                    items: vec![takeoff(47.001)],
                    ..copter.clone()
                },
                1
            ),
            "unless the two were moved apart"
        );
        assert!(
            !launch_at_takeoff(
                &Document {
                    vehicle_type: 1,
                    ..copter
                },
                1
            ),
            "a fixed wing launches apart from its takeoff"
        );
    }

    #[test]
    fn a_new_fixed_wing_landing_offers_its_own_setup_step_rather_than_the_takeoff_one() {
        let plane = Document {
            vehicle_type: 1,
            firmware_type: 3,
            ..empty_document()
        };
        let landing = crate::landingpattern::fresh(&crate::landingpattern::Fresh {
            vtol: false,
            land: (47.3977, 8.5456),
            ardupilot: true,
            relative: true,
            transition_distance: None,
        });
        let planned = plandoc::insert_complex(
            &plane,
            crate::landingpattern::FIXED_WING_PATTERN,
            landing,
            (47.3977, 8.5456),
            -1,
        );
        let facts = facts_in(&Flown(Vec::new()), &planned, None, 1);
        assert_eq!(
            facts["wizardMode"],
            json!(true),
            "the plan holds the landing unfinished, so its editor must offer the step that finishes it"
        );
        assert_ne!(facts["wizardText"], json!([]));
        let takeoff = plandoc::insert_simple(
            &plane,
            22,
            47.3977,
            8.5456,
            -1,
            &plandoc::EditDefaults {
                mission_item_altitude: 50.0,
                map_center: None,
                vtol_transition_distance: plandoc::VTOL_TRANSITION_DISTANCE_DEFAULT,
            },
        );
        assert_eq!(
            facts_in(&Flown(Vec::new()), &takeoff, Some(1), 1)["wizardMode"],
            json!(true),
            "the takeoff's climb-out step still shows"
        );
    }

    #[test]
    fn a_waypoint_offers_its_hold_time_which_qgc_keeps_among_the_advanced_fields() {
        let waypoint = |hold: Option<f64>| plandoc::Simple {
            command: 16,
            frame: 3,
            params: [
                hold,
                Some(0.0),
                Some(0.0),
                None,
                Some(47.0),
                Some(8.0),
                Some(30.0),
            ],
            auto_continue: true,
            altitude: None,
            sections: vec![],
        };
        let commands = crate::cmdinfo::tree(
            crate::cmdinfo::Firmware::ArduPilot,
            crate::cmdinfo::VehicleClass::MultiRotor,
        );
        let info = commands.get(&16);
        assert_eq!(
            hold_field(
                &waypoint(Some(5.0)),
                info,
                false,
                "plan.missionController.visualItems.2"
            ),
            json!({ "value": 5.0, "units": "s", "path": "plan.missionController.visualItems.2.hold" })
        );
        assert_eq!(
            hold_field(&waypoint(None), info, false, "x")["value"],
            0.0,
            "an unset hold reads as no hold"
        );
        assert_eq!(
            hold_field(&waypoint(Some(5.0)), info, true, "x"),
            Value::Null,
            "raw edit already shows Param1"
        );
        let takeoff = plandoc::Simple {
            command: 22,
            ..waypoint(Some(5.0))
        };
        assert_eq!(
            hold_field(&takeoff, commands.get(&22), false, "x"),
            Value::Null
        );
        let plane = crate::cmdinfo::tree(
            crate::cmdinfo::Firmware::ArduPilot,
            crate::cmdinfo::VehicleClass::FixedWing,
        );
        assert_eq!(
            hold_field(&waypoint(Some(5.0)), plane.get(&16), false, "x"),
            Value::Null,
            "a plane flies through a waypoint, so it is not offered a hover it ignores"
        );
    }

    #[test]
    fn a_route_flown_at_one_altitude_follows_the_route_altitude_without_asking() {
        let at = |command: i64, metres: f64| {
            plandoc::Item::Simple(plandoc::Simple {
                command,
                frame: 3,
                params: [
                    Some(0.0),
                    Some(0.0),
                    Some(0.0),
                    None,
                    Some(47.0),
                    Some(8.0),
                    Some(metres),
                ],
                auto_continue: true,
                altitude: Some(plandoc::Altitude {
                    mode: crate::altitudemodes::RELATIVE,
                    altitude: metres,
                    amsl_above_terrain: None,
                }),
                sections: vec![],
            })
        };
        let plan = |items: Vec<plandoc::Item>| Document {
            items,
            ..empty_document()
        };
        assert!(
            follows_one_altitude(&plan(vec![at(22, 50.0), at(16, 50.0), at(16, 50.0)])),
            "takeoff and waypoints at one height are one route altitude"
        );
        assert!(
            follows_one_altitude(&plan(vec![at(16, 50.0), at(21, 0.0)])),
            "a landing comes down by design and does not count as a custom height"
        );
        assert!(
            !follows_one_altitude(&plan(vec![at(16, 50.0), at(16, 80.0)])),
            "a waypoint the operator raised is theirs, so changing the route asks first"
        );
    }

    #[test]
    fn a_waypoint_offers_its_heading_unset_until_the_operator_turns_the_aircraft() {
        let waypoint = |yaw: Option<f64>| plandoc::Simple {
            command: 16,
            frame: 3,
            params: [
                Some(0.0),
                Some(0.0),
                Some(0.0),
                yaw,
                Some(47.0),
                Some(8.0),
                Some(30.0),
            ],
            auto_continue: true,
            altitude: None,
            sections: vec![],
        };
        let commands = crate::cmdinfo::tree(
            crate::cmdinfo::Firmware::ArduPilot,
            crate::cmdinfo::VehicleClass::MultiRotor,
        );
        let info = commands.get(&16);
        assert_eq!(
            yaw_field(
                &waypoint(Some(90.0)),
                info,
                false,
                "plan.missionController.visualItems.2"
            ),
            json!({ "value": 90.0, "units": "deg", "path": "plan.missionController.visualItems.2.yaw" })
        );
        assert_eq!(
            yaw_field(&waypoint(Some(f64::NAN)), info, false, "x")["value"],
            Value::Null,
            "NaN is MAV_CMD_NAV_WAYPOINT's keep-the-heading, so the action reads as off"
        );
        assert_eq!(
            yaw_field(&waypoint(Some(90.0)), info, true, "x"),
            Value::Null,
            "raw edit already shows Param4"
        );
        assert_eq!(
            yaw_field(
                &plandoc::Simple {
                    command: 22,
                    ..waypoint(Some(90.0))
                },
                commands.get(&22),
                false,
                "x"
            ),
            Value::Null
        );
    }

    #[test]
    fn a_plan_of_only_a_fence_or_rally_points_contains_items() {
        assert!(!contains_items(&empty_document()));
        assert!(
            contains_items(&Document {
                fence: json!({ "polygons": [{ "inclusion": true, "polygon": [] }], "circles": [] }),
                ..empty_document()
            }),
            "PlanMasterController::containsItems counts the geofence"
        );
        assert!(
            contains_items(&Document {
                rally: json!({ "points": [[47.0, 8.0, 50.0]] }),
                ..empty_document()
            }),
            "and the rally points"
        );
    }

    #[test]
    fn a_plan_without_speeds_keeps_the_offline_speeds_as_qgc_does() {
        let text = r#"{"fileType":"Plan","version":1,"groundStation":"QGroundControl","mission":{"version":2,"firmwareType":12,"vehicleType":2,"plannedHomePosition":[47.0,8.0,500],"items":[]},"geoFence":{"version":2,"polygons":[],"circles":[]},"rallyPoints":{"version":2,"points":[]}}"#;
        assert!(
            plandoc::load(text, 2).unwrap().cruise_speed.is_nan(),
            "absent is told apart from zero"
        );
        let loaded = load_plan(text).unwrap();
        assert!(
            loaded.cruise_speed > 0.0 && loaded.hover_speed > 0.0,
            "MissionController::_loadJsonMissionFileV2 only overwrites the offline speeds when the file carries them"
        );
    }

    #[test]
    fn a_mission_parameter_in_metres_is_shown_in_the_users_units_like_its_fact() {
        let feet = crate::units::cooking_with("m", |_| Some("0".to_string()), 0);
        let radius =
            json!({ "label": "Radius", "units": "m", "default": 30.0, "decimalPlaces": 1 });
        let shown = param_fact_with(&radius, 30.48, feet);
        assert!(
            (shown["value"].as_f64().unwrap() - 100.0).abs() < 1e-9,
            "FactMetaData translates m to ft for a float parameter"
        );
        assert_eq!(
            (
                shown["units"].as_str(),
                shown["rawUnits"].as_str(),
                shown["rawValue"].as_f64()
            ),
            (Some("ft"), Some("m"), Some(30.48))
        );
        let listed =
            json!({ "label": "Mode", "units": "m", "enumStrings": "A,B", "enumValues": "0,1" });
        assert_eq!(
            param_conversion(&listed).map(|c| c.name),
            None,
            "an enumerated fact is never translated"
        );
        let pitch = json!({ "label": "Pitch", "units": "gimbal-degrees" });
        assert_eq!(
            param_conversion(&pitch).map(|c| ((c.shown)(10.0), c.name)),
            Some((-10.0, "deg")),
            "the built-in translator comes first, as setBuiltInTranslator does"
        );
        let feet_back = crate::units::cooking_with("m", |_| Some("0".to_string()), 0)
            .map(|c| (c.base)(100.0))
            .unwrap();
        assert!(
            (feet_back - 30.48).abs() < 1e-9,
            "a value typed in feet is written back in metres"
        );
    }

    #[test]
    fn the_climb_out_step_only_counts_while_its_takeoff_is_still_there() {
        let simple = |command: i64| {
            plandoc::Item::Simple(plandoc::Simple {
                command,
                frame: 3,
                params: [Some(0.0); 7],
                auto_continue: true,
                altitude: None,
                sections: vec![],
            })
        };
        let with = |items: Vec<plandoc::Item>| Held {
            wizard: Some(1),
            document: Some(Document {
                items,
                ..empty_document()
            }),
            ..Held::default()
        };
        assert_eq!(live_wizard(&with(vec![simple(CMD_NAV_TAKEOFF)])), Some(1));
        assert_eq!(
            live_wizard(&with(vec![simple(16)])),
            None,
            "an item inserted before it shifted the takeoff away"
        );
        assert_eq!(
            live_wizard(&with(Vec::new())),
            None,
            "the takeoff was deleted, so save and upload are not held back by it"
        );
    }

    #[test]
    fn a_connected_vehicle_sets_the_offline_planning_classes_like_plan_master_controller() {
        assert_eq!(
            offline_classes(3, 1),
            (3, 1),
            "an ArduPilot plane plans as ArduPilot fixed wing"
        );
        assert_eq!(
            offline_classes(12, 2),
            (12, 2),
            "a PX4 quad as PX4 multirotor"
        );
        assert_eq!(offline_classes(12, 22), (12, 20), "a VTOL type as VTOL");
    }

    fn by_value(value: Value) -> Value {
        match value {
            Value::Number(n) => json!(n.as_f64()),
            Value::Array(items) => Value::Array(items.into_iter().map(by_value).collect()),
            Value::Object(fields) => {
                Value::Object(fields.into_iter().map(|(k, v)| (k, by_value(v))).collect())
            }
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
        assert_eq!(
            shape_points(&path, false).map(|p| p.len()),
            Ok(3),
            "the repeated closing vertex is dropped"
        );
        assert_eq!(
            shape_points(&path, true),
            Err("KML file load failed. Unable to find LineString node in KML".to_string()),
            "a corridor needs a polyline"
        );
        assert!(shape_points("area.gpx", false).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_removed_item_hands_current_to_the_one_now_at_its_index_or_the_last() {
        assert_eq!(
            current_after_remove(2, 4),
            2,
            "MissionController::removeVisualItem keeps the index"
        );
        assert_eq!(current_after_remove(5, 4), 4, "and steps back past the end");
        assert_eq!(
            current_after_remove(1, 0),
            0,
            "with nothing left the settings item is current"
        );
    }

    fn kml_plan(text: &str, types: (i64, i64)) -> Imported {
        kml_document(
            &crate::kml::route(text).unwrap(),
            &KmlContext {
                types,
                offline_vehicle_type: types.1,
                vehicle_home: None,
                takeoff_not_required: false,
            },
            &plandoc::EditDefaults {
                mission_item_altitude: 50.0,
                map_center: None,
                vtol_transition_distance: 300.0,
            },
        )
        .unwrap()
    }

    fn kml_fixture(name: &str) -> String {
        crate::kml::read(&format!(
            "{}/../test/MissionManager/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    fn simple_at(document: &Document, index: usize) -> &plandoc::Simple {
        match &document.items[index] {
            plandoc::Item::Simple(simple) => simple,
            plandoc::Item::Complex { .. } => panic!("item {index} is not simple"),
        }
    }

    fn commands_of(document: &Document) -> Vec<i64> {
        (0..document.items.len())
            .map(|i| simple_at(document, i).command)
            .collect()
    }

    const PX4_COPTER: (i64, i64) = (12, 2);
    const APM_COPTER: (i64, i64) = (3, 2);
    const TOWERS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2"><Document>
  <Placemark><LineString><coordinates>44.7900,41.7100,0 44.7950,41.7120,0 44.8010,41.7150,0</coordinates></LineString></Placemark>
  <Placemark><name>Tower A</name><Point><coordinates>44.7951,41.7122,0</coordinates></Point></Placemark>
  <Placemark><name>Tower B</name><Point><coordinates>44.8012,41.7151,0</coordinates></Point></Placemark>
</Document></kml>"#;

    #[test]
    fn a_ground_path_opens_above_the_terrain_with_a_takeoff_and_a_return_like_plan_master_controller_test()
     {
        let imported = kml_plan(&kml_fixture("RouteGroundPath.kml"), PX4_COPTER);
        let document = &imported.document;
        assert_eq!(commands_of(document), vec![22, 16, 16, 16, 20]);
        assert_eq!(
            document.global_altitude_mode,
            crate::altitudemodes::CALC_ABOVE_TERRAIN
        );
        (1..4).for_each(|i| {
            let held = simple_at(document, i).altitude.clone().unwrap();
            assert_eq!(
                (held.mode, held.altitude),
                (crate::altitudemodes::CALC_ABOVE_TERRAIN, 50.0)
            );
        });
        assert_eq!(simple_at(document, 3).params[5], Some(44.8010));
        let summary = imported.summary.unwrap();
        assert!(
            regex::Regex::new(r"(?s)Imported 3 waypoints\..*no flight altitudes.*above the terrain.*Offline.*takeoff was added.*return to launch")
                .unwrap()
                .is_match(&summary),
            "{summary}"
        );
        assert_eq!(imported.wizard, None);
    }

    #[test]
    fn an_absolute_kmz_keeps_its_altitudes_above_sea_level() {
        let imported = kml_plan(&kml_fixture("RouteAbsolutePath.kmz"), PX4_COPTER);
        let document = &imported.document;
        assert_eq!(
            document.global_altitude_mode,
            crate::altitudemodes::ABSOLUTE
        );
        let first = simple_at(document, 1).altitude.clone().unwrap();
        assert_eq!(
            (first.mode, first.altitude),
            (crate::altitudemodes::ABSOLUTE, 560.5)
        );
        assert_eq!(
            simple_at(document, 3).altitude.clone().unwrap().altitude,
            590.25
        );
        assert!(
            imported
                .summary
                .unwrap()
                .contains("above sea level, as in the file")
        );
        let points = kml_plan(&kml_fixture("RoutePlacemarks.kml"), PX4_COPTER).document;
        assert_eq!(commands_of(&points), vec![22, 16, 16, 20]);
        assert_eq!(
            (
                simple_at(&points, 1).params[4],
                simple_at(&points, 2).params[4]
            ),
            (Some(41.7100), Some(41.7150))
        );
    }

    #[test]
    fn towers_beside_a_path_become_look_at_points_before_their_nearest_vertex() {
        let imported = kml_plan(TOWERS, PX4_COPTER);
        assert_eq!(
            commands_of(&imported.document),
            vec![22, 16, 195, 16, 195, 16, 197, 20]
        );
        assert_eq!(simple_at(&imported.document, 2).params[5], Some(44.7951));
        assert!(
            imported
                .summary
                .unwrap()
                .contains("2 placemarks became look-at points")
        );
    }

    #[test]
    fn towers_on_ardupilot_stop_looking_with_a_zero_location_that_uploads() {
        let document = kml_plan(TOWERS, APM_COPTER).document;
        assert_eq!(
            commands_of(&document),
            vec![22, 16, 201, 16, 201, 16, 201, 20]
        );
        assert_eq!(simple_at(&document, 2).params[0], Some(3.0));
        let stop = simple_at(&document, 6);
        assert_eq!(stop.params[0], Some(0.0));
        assert_eq!(
            (stop.params[4], stop.params[5], stop.params[6]),
            (Some(0.0), Some(0.0), Some(0.0))
        );
        assert_eq!(stop.frame, 3);
        assert!(
            !plandoc::awaiting_terrain(&Document {
                items: vec![document.items[6].clone()],
                ..document.clone()
            }),
            "the stop-looking item never waits for terrain"
        );
    }

    #[test]
    fn a_fixed_wing_takeoff_asks_to_be_placed() {
        let imported = kml_plan(TOWERS, (12, 1));
        assert_eq!(imported.wizard, Some(1));
        assert!(imported.summary.unwrap().contains("Drag its T on the map"));
        let vtol = kml_plan(TOWERS, (12, 20)).document;
        assert_eq!(
            simple_at(&vtol, 0).command,
            84,
            "a VTOL gets a VTOL takeoff"
        );
        let rover = kml_plan(TOWERS, (12, 10));
        assert_eq!(
            commands_of(&rover.document)[0],
            16,
            "a rover has no takeoff"
        );
        assert!(!rover.summary.unwrap().contains("takeoff"));
    }

    #[test]
    fn a_recorded_route_replays_its_camera_like_plan_master_controller_test() {
        let start = (41.7100, 44.7900);
        let moved = |distance: f64, azimuth: f64| {
            crate::surveygrid::at_distance_and_azimuth(start, distance, azimuth)
        };
        let (second, third, fourth) = (moved(100.0, 0.0), moved(150.0, 45.0), moved(300.0, 90.0));
        let text = format!(
            r#"<kml><Document><Placemark><ExtendedData><Data name="gimbal"><value>-30.0,10.0 -30.2,10.0 -45.0,0.0 -60.0,0.0</value></Data><Data name="lookAt"><value>- - 44.7930000,41.7110000,600.0 -</value></Data></ExtendedData><LineString><altitudeMode>absolute</altitudeMode><coordinates>44.79,41.71,650 {},{},650 {},{},660 {},{},670</coordinates></LineString></Placemark></Document></kml>"#,
            second.1, second.0, third.1, third.0, fourth.1, fourth.0
        );
        let imported = kml_plan(&text, PX4_COPTER);
        let document = &imported.document;
        assert_eq!(
            commands_of(document),
            vec![22, 16, 16, 195, 16, 197, 205, 16, 20]
        );
        let initial = &document.settings_sections;
        assert_eq!(
            initial
                .iter()
                .map(|s| (s.command, s.params[0], s.params[2]))
                .collect::<Vec<_>>(),
            vec![(205, Some(-30.0), Some(10.0))],
            "the first angle is the plan's initial camera setting"
        );
        let roi = simple_at(document, 3).altitude.clone().unwrap();
        assert_eq!(
            (roi.mode, roi.altitude),
            (crate::altitudemodes::ABSOLUTE, 600.0)
        );
        let gimbal = simple_at(document, 6);
        assert_eq!(
            (gimbal.params[0], gimbal.params[6]),
            (Some(-60.0), Some(2.0))
        );
        assert!(
            imported
                .summary
                .unwrap()
                .contains("camera angles and look-at points from the recording")
        );
    }

    #[test]
    fn a_waypoints_file_opens_as_a_plan_like_load_text_file() {
        let doc = load_text_mission("QGC WPL 110\n0\t1\t0\t16\t0\t0\t0\t0\t47.66\t-122.10\t5.2\t1\n1\t0\t3\t22\t0\t0\t0\t0\t47.661\t-122.103\t100\t1\n2\t0\t3\t16\t0\t0\t0\t0\t47.662\t-122.104\t100\t1\n").unwrap();
        assert_eq!(doc.items.len(), 2);
        assert_eq!(
            doc.home.map(|h| h[2]),
            Some(5.2),
            "the first row of a 110 file is the planned home"
        );
        assert_eq!(
            doc.global_altitude_mode,
            crate::planfile::ALTITUDE_MODE_MIXED,
            "loadTextFile sets the Mixed global frame"
        );
        assert!(
            !doc.cruise_speed.is_nan() && !doc.hover_speed.is_nan(),
            "the offline speeds fill in"
        );
        assert_eq!(
            load_text_mission("QGC WPL 110\n0\t1\t0\n").unwrap_err(),
            "Mission: The mission file is corrupted."
        );
        assert_eq!(
            load_text_mission("{}").unwrap_err(),
            format!("Mission: {}", crate::waypoints::not_compatible())
        );
    }

    #[test]
    fn a_120_file_takes_its_home_from_the_first_coordinate_and_keeps_jumps_unchecked() {
        let doc = load_text_mission("QGC WPL 120\n0\t0\t2\t178\t1\t5\t0\t0\t0\t0\t0\t1\n1\t0\t3\t16\t0\t0\t0\t0\t47.0\t8.0\t50\t1\n2\t0\t2\t177\t40\t1\t0\t0\t0\t0\t0\t1\n").unwrap();
        let home = doc
            .home
            .expect("_setPlannedHomePositionFromFirstCoordinate places it");
        assert!(
            (home[0] - 47.0).abs() > 1e-5 && (home[1] - 8.0).abs() < 1e-9 && home[2] == 0.0,
            "30 m north of the first coordinate item, at altitude 0: {home:?}"
        );
        let plandoc::Item::Simple(jump) = &doc.items[2] else {
            panic!("the jump stays a simple item")
        };
        assert_eq!(
            jump.params[0],
            Some(41.0),
            "_loadTextMissionFile never checks a jump target"
        );
        assert_eq!(
            load_text_mission("QGC WPL 120\n").unwrap().home,
            None,
            "with nothing to measure from the home stays unset"
        );
    }

    #[test]
    fn a_text_loaded_takeoff_without_a_coordinate_sits_on_launch_like_takeoff_mission_item() {
        let text = "QGC WPL 120\n0\t0\t3\t22\t0\t0\t0\t0\t0\t0\t30\t1\n1\t0\t3\t16\t0\t0\t0\t0\t47.0\t8.0\t50\t1\n";
        let plan = crate::planfile::write(&crate::planfile::from_waypoints(
            &crate::waypoints::parse(text).unwrap(),
            3,
            2,
        ));
        let document = plandoc::load_text_mission(&plan, 2).unwrap();
        let takeoff_at = |doc: &Document| match &doc.items[0] {
            plandoc::Item::Simple(s) => (s.params[4], s.params[5]),
            _ => panic!("takeoff stays simple"),
        };
        let vehicle = placed_text_mission(document.clone(), false, Some([46.5, 7.5, 400.0]));
        assert_eq!(
            vehicle.home,
            Some([46.5, 7.5, 400.0]),
            "a 120 file with a takeoff adopts the active vehicle's home"
        );
        assert_eq!(
            takeoff_at(&vehicle),
            (Some(46.5), Some(7.5)),
            "an ArduPilot takeoff has no coordinate, so it is put on launch"
        );
        let offline = placed_text_mission(document.clone(), false, None);
        assert_eq!(
            takeoff_at(&offline),
            (Some(0.0), Some(0.0)),
            "with no launch yet nothing moves"
        );
        assert!(
            offline.home.is_some_and(|h| h[2] == 0.0),
            "and home comes from the first coordinate afterwards"
        );
        let px4 = placed_text_mission(
            Document {
                firmware_type: 12,
                ..document
            },
            false,
            Some([46.5, 7.5, 400.0]),
        );
        assert_eq!(
            takeoff_at(&px4),
            (Some(0.0), Some(0.0)),
            "a PX4 takeoff specifies its own coordinate"
        );
    }

    #[test]
    fn the_suffix_picks_the_loader_and_a_loaded_file_is_saved_as_a_plan_beside_it() {
        assert!(is_text_mission("/a/m.waypoints") && is_text_mission("/a/m.txt"));
        assert!(
            !is_text_mission("/a/m.plan") && !is_text_mission("/a/m.WAYPOINTS"),
            "QFileInfo::suffix compares case-sensitively"
        );
        assert_eq!(
            plan_file_after_load("/a/b.c.waypoints"),
            "/a/b.c.plan",
            "completeBaseName keeps every dot but the last"
        );
        assert_eq!(
            load_failed_text("/a/m.plan", "Mission: x"),
            "Error loading Plan file (/a/m.plan). Mission: x"
        );
    }

    #[test]
    fn a_new_default_altitude_lands_on_every_item_like_apply_new_altitude() {
        let doc = plandoc::load(include_str!("../tests/fixtures/survey-upload.plan"), 2).unwrap();
        let survey = doc
            .items
            .iter()
            .find(|i| matches!(i, plandoc::Item::Complex { kind, .. } if kind == "survey"))
            .unwrap();
        let plandoc::Item::Complex { json, .. } = with_new_altitude(survey, 77.0) else {
            panic!("a survey stays a survey")
        };
        assert_eq!(
            json["TransectStyleComplexItem"]["CameraCalc"]["DistanceToSurface"],
            77.0
        );
        assert_eq!(
            json["TransectStyleComplexItem"]["CameraCalc"]["ValueSetIsDistance"], true,
            "TransectStyleComplexItem::applyNewAltitude switches the camera to distance first"
        );
        assert!(
            json["TransectStyleComplexItem"]["Items"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|i| i["command"] == 16)
                .all(|i| i["params"][6] == 77.0),
            "the transects are rebuilt at the new height"
        );
        let simple = |command: i64| {
            plandoc::Item::Simple(plandoc::Simple {
                command,
                frame: 3,
                params: [Some(0.0); 7],
                auto_continue: true,
                altitude: Some(plandoc::Altitude {
                    mode: 1,
                    altitude: 30.0,
                    amsl_above_terrain: None,
                }),
                sections: Vec::new(),
            })
        };
        assert!(
            matches!(with_new_altitude(&simple(16), 77.0), plandoc::Item::Simple(s) if s.params[6] == Some(77.0) && s.altitude.as_ref().is_some_and(|a| a.altitude == 77.0))
        );
        assert!(
            matches!(with_new_altitude(&simple(CMD_NAV_LAND), 77.0), plandoc::Item::Simple(s) if s.params[6] == Some(0.0)),
            "a land item is left alone"
        );
        let unplaced = plandoc::Item::Simple(plandoc::Simple {
            altitude: None,
            ..match simple(178) {
                plandoc::Item::Simple(s) => s,
                _ => unreachable!(),
            }
        });
        assert_eq!(
            with_new_altitude(&unplaced, 77.0),
            unplaced,
            "a command without an altitude keeps its params"
        );
    }

    #[test]
    fn the_editor_fields_of_each_command_are_the_ones_qt_builds() {
        let doc = plandoc::load(include_str!("../tests/fixtures/commands.plan"), 2).unwrap();
        let qt: Vec<Value> = serde_json::from_str(include_str!(
            "../tests/fixtures/itemfacts-commands-by-qt.json"
        ))
        .unwrap();
        qt.iter().enumerate().for_each(|(index, expected)| {
            let metres = crate::read::Unit {
                name: "m".to_string(),
                factor: 1.0,
            };
            let mine = by_value(document_facts(&doc, index, 5.0, 15.0, (&metres, &metres)));
            let expected = by_value(expected.clone());
            let differing: Vec<String> = expected
                .as_object()
                .unwrap()
                .iter()
                .filter(|(k, v)| mine.get(k.as_str()) != Some(v))
                .map(|(k, v)| {
                    format!(
                        "{k}:\n  core {}\n  qt   {v}",
                        mine.get(k.as_str()).unwrap_or(&Value::Null)
                    )
                })
                .collect();
            assert!(
                differing.is_empty(),
                "item {index}: {}",
                differing.join("\n")
            );
        });
    }

    #[test]
    fn a_dirty_plan_asks_what_to_do_when_its_vehicle_goes_or_changes() {
        assert_eq!(vehicle_change_prompt_for(None, true), Value::Null);
        let gone = vehicle_change_prompt_for(Some(true), true);
        assert_eq!(
            (
                gone["title"].as_str(),
                gone["loadText"].as_str(),
                gone["keepText"].as_str()
            ),
            (
                Some("Plan View - Vehicle Disconnected"),
                Some("Discard Unsaved Changes"),
                Some("Keep Current Plan")
            )
        );
        let changed = vehicle_change_prompt_for(Some(false), true);
        assert_eq!(
            changed["loadText"],
            "Discard Unsaved Changes, Load New Plan From Vehicle"
        );
        assert_eq!(
            vehicle_change_prompt_for(Some(false), false)["loadText"],
            "Load New Plan From Vehicle"
        );
        assert_eq!(
            (
                changed["connected"].as_bool(),
                gone["connected"].as_bool(),
                changed["dirty"].as_bool()
            ),
            (Some(true), Some(false), Some(true)),
            "a head can word the choice for the case it is in"
        );
    }

    const SPEED_SECTION: i64 = 178;

    fn flown(commands: &[i64], sends_home: bool, current: i64) -> FlownMission {
        FlownMission {
            items: commands
                .iter()
                .enumerate()
                .map(|(at, command)| Downloaded {
                    frame: if at == 0 && sends_home { 0 } else { 3 },
                    command: *command,
                    params: match *command {
                        SPEED_SECTION => [1.0, 5.0, -1.0, 0.0, 0.0, 0.0, 0.0],
                        _ => [0.0, 0.0, 0.0, 0.0, 47.0 + at as f64 * 0.001, 8.0, 50.0],
                    },
                    auto_continue: true,
                })
                .collect(),
            sends_home,
            types: (if sends_home { 3 } else { 12 }, 2),
            current,
            home: Some([47.0, 8.0, 400.0]),
        }
    }

    #[test]
    fn the_aircraft_route_is_counted_in_the_rows_the_plan_list_shows() {
        assert_eq!(
            rows_of(&flown(&[16, 22, 16, SPEED_SECTION, 16, 20], true, 0)),
            Some(4),
            "ArduPilot sends its home as row 0, and the speed change is the waypoint's own speed setting, so the pilot is offered four rows, not five commands"
        );
        assert_eq!(
            rows_of(&flown(&[22, 16, 16, 20], false, 0)),
            Some(4),
            "PX4 sends only the mission"
        );
        assert_eq!(
            rows_of(&flown(&[16], true, 0)),
            None,
            "a home alone is no route"
        );
        assert_eq!(rows_of(&flown(&[], false, 0)), None);
    }

    #[test]
    fn an_action_is_reported_at_the_place_the_aircraft_stands_numbered_or_not() {
        let vehicle = flown_document(&flown(&[16, 22, 16, 183, 19, 183, 16], true, 0));
        let at_waypoint = row_of_sequence(&vehicle, 3).unwrap();
        assert_eq!(
            (at_waypoint.place.as_str(), at_waypoint.arrived),
            ("waypoint 1 of 2 · set servo", true)
        );
        let at_loiter = row_of_sequence(&vehicle, 5).unwrap();
        assert!(
            at_loiter.arrived
                && at_loiter.place.starts_with("loiter")
                && at_loiter.place.ends_with(" · set servo"),
            "the servo after the loiter fires at the loiter, not back at waypoint 1: {}",
            at_loiter.place
        );
        let flying = row_of_sequence(&vehicle, 6).unwrap();
        assert_eq!(
            [Some(&flying), None].map(|row| crate::missionprogress::set_waypoint_line(row, 6)),
            [
                "Continue the mission from waypoint 2 of 2",
                "Continue the mission from mission item 6"
            ],
            "the marker tapped reads 2, so the sheet says 2, not the mission sequence 6"
        );
        let progress = crate::missionprogress::Progress {
            current: 3,
            last: 6,
        };
        assert_eq!(
            [Some(&at_waypoint), Some(&flying), None]
                .map(|row| crate::missionprogress::progress_line(row, &progress)),
            [
                "At waypoint 1 of 2 · set servo",
                "To waypoint 2 of 2",
                "Mission item 3 of 6"
            ]
        );
    }

    #[test]
    fn a_raw_list_cannot_be_shifted_by_a_comma_in_a_name_and_a_raw_label_names_plain_units() {
        let listed = choice(
            "Command",
            [("Wait, then go".to_string(), 5), ("Land".to_string(), 21)].into_iter(),
        );
        assert_eq!(
            (
                list(&listed, "enumStrings").len(),
                list(&listed, "enumValues")
            ),
            (2, vec!["5".to_string(), "21".to_string()]),
            "the names and the values stay one to one"
        );
        assert_eq!(
            ["vertical m", "horizontal m", "m/s", "deg"].map(plain_units),
            ["m", "m", "m/s", "deg"]
        );
    }

    #[test]
    fn a_flown_sequence_is_numbered_by_the_plan_row_that_holds_it() {
        let commands = [16, 22, 16, SPEED_SECTION, 16, 20];
        let mission = flown(&commands, true, 0);
        let vehicle = flown_document(&mission);
        let takeoff = row_of_sequence(&vehicle, 1).unwrap();
        assert_eq!(
            (takeoff.place.as_str(), takeoff.arrived),
            ("takeoff", false),
            "a takeoff is named, not numbered"
        );
        let after_mount = row_of_sequence(&vehicle, 4).unwrap();
        assert_eq!(
            after_mount.place, "waypoint 2 of 2",
            "sequence 4 follows the waypoint whose speed setting is sequence 3, so it is the second of two waypoints, not waypoint 4 of 5"
        );
        assert_eq!(
            row_of_sequence(&vehicle, 5).unwrap().place,
            "return to launch",
            "the return is named"
        );
        assert!(row_of_sequence(&vehicle, 0).is_none(), "home is not a row");
        let survey =
            plandoc::load(include_str!("../tests/fixtures/survey-upload.plan"), 2).unwrap();
        let midway = visual_spans(&survey)
            .iter()
            .find(|(first, last)| last > first && *first > 0)
            .map(|(first, last)| (first + last) / 2)
            .unwrap();
        let surveyed = row_of_sequence(&survey, midway).unwrap();
        assert!(
            surveyed.place.starts_with("survey "),
            "a survey is one numbered stop, whichever of its commands is flying: {}",
            surveyed.place
        );
        let surveyed_at = visual_index_of_sequence(&survey, midway).unwrap();
        let (first, last) = visual_spans(&survey)[usize::try_from(surveyed_at).unwrap()];
        let Some(plandoc::Item::Complex { kind, json, .. }) =
            survey.items.get(usize::try_from(surveyed_at - 1).unwrap())
        else {
            panic!("the survey row is a pattern")
        };
        let commands = pattern_commands(kind, json);
        let total = commands
            .iter()
            .filter(|command| **command == plandoc::CMD_NAV_WAYPOINT)
            .count();
        let third = commands
            .iter()
            .enumerate()
            .filter(|(_, command)| **command == plandoc::CMD_NAV_WAYPOINT)
            .nth(2)
            .map(|(offset, _)| first + offset as i64)
            .unwrap();
        assert_eq!(
            row_of_sequence(&survey, third).unwrap().within,
            Some((3, total)),
            "the core knows which of the survey's own waypoints a sequence is, so a resume can say where inside the survey it starts"
        );
        assert_eq!(
            row_of_sequence(&survey, last)
                .unwrap()
                .within
                .map(|(_, of)| of),
            Some(total)
        );
        assert_eq!(takeoff.within, None, "a simple row has nothing inside it");
        let planned = Some(&survey);
        assert_eq!(
            sequence_count(&flown_rows_document(planned, &mission)),
            sequence_count(&vehicle),
            "a plan that is not the one on the aircraft is not used to number its rows"
        );
    }

    #[test]
    fn a_landing_lines_up_with_the_takeoff_run_from_home() {
        let doc = plandoc::load(include_str!("../tests/fixtures/commands.plan"), 2).unwrap();
        let takeoff = |lat: f64, lon: f64| {
            plandoc::Item::Simple(plandoc::Simple {
                command: CMD_NAV_TAKEOFF,
                frame: 3,
                params: [
                    Some(0.0),
                    None,
                    None,
                    None,
                    Some(lat),
                    Some(lon),
                    Some(30.0),
                ],
                auto_continue: true,
                altitude: None,
                sections: vec![],
            })
        };
        let home = doc.home.unwrap();
        let east = Document {
            items: vec![takeoff(home[0], home[1] + 0.01)],
            ..doc.clone()
        };
        assert!(takeoff_heading(&east).is_some_and(|h| (h - 90.0).abs() < 0.5));
        assert_eq!(
            takeoff_heading(&Document {
                items: Vec::new(),
                ..doc.clone()
            }),
            None,
            "no takeoff, no heading to copy"
        );
    }

    #[test]
    fn a_pattern_shape_is_complete_once_it_can_be_flown() {
        assert!(!shape_complete(
            "survey",
            &json!({ "polygon": [[1, 2], [3, 4]] })
        ));
        assert!(shape_complete(
            "survey",
            &json!({ "polygon": [[1, 2], [3, 4], [5, 6]] })
        ));
        assert!(shape_complete(
            "CorridorScan",
            &json!({ "polyline": [[1, 2], [3, 4]] })
        ));
        assert!(!shape_complete("StructureScan", &json!({})));
    }

    #[test]
    fn range_limits_print_as_qstring_arg_does() {
        assert_eq!(
            (
                qt_number(0.0),
                qt_number(3600.0),
                qt_number(f64::MAX),
                qt_number(-f64::MAX),
                qt_number(0.5),
                qt_number(123456.0)
            ),
            (
                "0".into(),
                "3600".into(),
                "1.79769e+308".into(),
                "-1.79769e+308".into(),
                "0.5".into(),
                "123456".into()
            )
        );
        assert_eq!(qt_number(1_234_567.0), "1.23457e+06");
        assert_eq!(
            (
                qt_number(999_999.5),
                qt_number(9_999_999.0),
                qt_number(0.000_099_999_99)
            ),
            ("1e+06".into(), "1e+07".into(), "0.0001".into()),
            "the exponent is read after rounding, as %g does"
        );
    }

    #[test]
    fn show_all_values_moves_with_its_item() {
        let at = |lat: f64| {
            plandoc::Item::Simple(plandoc::Simple {
                command: 16,
                frame: 3,
                params: [
                    Some(0.0),
                    Some(0.0),
                    Some(0.0),
                    None,
                    Some(lat),
                    Some(8.0),
                    Some(30.0),
                ],
                auto_continue: true,
                altitude: None,
                sections: vec![],
            })
        };
        let (a, b, c) = (at(47.0), at(47.1), at(47.2));
        *raw_edits() = [3].into();
        shift_raw_edits(&[a.clone(), b.clone(), c.clone()], &[a.clone(), c.clone()]);
        assert_eq!(
            *raw_edits(),
            [2].into(),
            "deleting item 2 moves item 3's raw view to 2"
        );
        shift_raw_edits(&[a.clone(), c.clone()], &[b.clone(), a.clone(), c.clone()]);
        assert_eq!(
            *raw_edits(),
            [3].into(),
            "an insert in front shifts it back"
        );
        *raw_edits() = [2].into();
        shift_raw_edits(&[a.clone(), b.clone(), c.clone()], &[a, c]);
        assert!(
            raw_edits().is_empty(),
            "the deleted item's raw view goes with it"
        );
    }

    #[test]
    fn a_choice_picked_by_its_place_in_the_list_writes_the_value_at_that_place() {
        let commands = crate::cmdinfo::tree(
            crate::cmdinfo::Firmware::ArduPilot,
            crate::cmdinfo::VehicleClass::MultiRotor,
        );
        let delay = commands.keys().position(|id| *id == 93).unwrap();
        assert_eq!(
            combo_value(Choices::Raw, &commands, 0, delay),
            Some(93.0),
            "the raw Command list writes its index, which is the command at that place, not a refused enumIndex"
        );
        let (id, param) = commands
            .iter()
            .find_map(|(id, info)| {
                field_params(info, true)
                    .first()
                    .map(|(_, p)| (*id, p.clone()))
            })
            .unwrap();
        let simple = plandoc::Simple {
            command: id,
            frame: 3,
            params: [None; 7],
            auto_continue: true,
            altitude: None,
            sections: Vec::new(),
        };
        assert_eq!(
            combo_value(Choices::Friendly(&simple), &commands, 0, 0),
            list(&param, "enumValues")[0].parse().ok()
        );
        assert_eq!(combo_value(Choices::Raw, &commands, 0, 9999), None);
    }

    #[test]
    fn an_item_qgc_cannot_show_friendly_is_edited_raw() {
        let doc = plandoc::load(include_str!("../tests/fixtures/commands.plan"), 2).unwrap();
        let metres = crate::read::Unit {
            name: "m".to_string(),
            factor: 1.0,
        };
        let friendly = document_facts(&doc, 1, 5.0, 15.0, (&metres, &metres));
        assert_eq!(
            (
                friendly["rawEdit"].clone(),
                friendly["friendlyEditAllowed"].clone()
            ),
            (json!(false), json!(true))
        );
        let stuck = Document {
            items: doc
                .items
                .iter()
                .enumerate()
                .map(|(i, item)| match (i, item) {
                    (0, plandoc::Item::Simple(s)) => plandoc::Item::Simple(plandoc::Simple {
                        auto_continue: false,
                        ..s.clone()
                    }),
                    (_, other) => other.clone(),
                })
                .collect(),
            ..doc.clone()
        };
        let raw = document_facts(&stuck, 1, 5.0, 15.0, (&metres, &metres));
        assert_eq!(
            (raw["rawEdit"].clone(), raw["friendlyEditAllowed"].clone()),
            (json!(true), json!(false)),
            "autoContinue off cannot be shown in simple mode"
        );
        let fields = raw["fields"].as_array().unwrap();
        let names: Vec<&str> = fields
            .iter()
            .map(|f| f["name"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(
            names,
            [
                "Command",
                "Frame",
                "Hold (secs)",
                "Acceptance (m)",
                "Pass Radius (m)",
                "Yaw (deg)",
                "Latitude",
                "Longitude",
                "Altitude"
            ],
            "the raw view lists the choices first, then each value by what it means for this command rather than Param1 or Lat/X"
        );
        assert_eq!(fields[1]["pathSuffix"], "comboboxFacts.1");
        assert_eq!(
            (fields[0]["control"].clone(), fields[0]["display"].clone()),
            (json!("choice"), json!("Waypoint")),
            "a command reads by the name the command picker gives it, not MAV_CMD_NAV_WAYPOINT"
        );
        assert!(
            fields[1]["options"]
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o["label"] == "Above terrain" && o["raw"] == "10")
        );
    }

    #[test]
    fn optional_params_are_served_as_nan_facts_and_switch_off_to_nan() {
        let commands = crate::cmdinfo::tree(
            crate::cmdinfo::Firmware::Px4,
            crate::cmdinfo::VehicleClass::Vtol,
        );
        let shown = |id: i64| {
            nan_params(&commands[&id])
                .iter()
                .map(|(i, _)| *i)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            shown(85),
            [4],
            "SimpleItemEditor's nanFacts: VTOL land Yaw, while its advanced Approach Alt stays hidden"
        );
        assert_eq!(
            shown(187),
            [1, 2, 3, 4],
            "each actuator of DO_SET_ACTUATOR is optional"
        );
        assert!(
            shown(16).is_empty(),
            "a waypoint's Yaw is advanced and goes to nanFactsAdvanced, which the editor does not show"
        );
        let doc = plandoc::load(include_str!("../tests/fixtures/commands.plan"), 2).unwrap();
        let at = 1;
        let unset = plandoc::set_param(&doc, at, 4, f64::NAN).unwrap();
        let Some(plandoc::Item::Simple(s)) = unset.items.get(at - 1) else {
            panic!()
        };
        assert!(
            s.params[3].is_some_and(f64::is_nan),
            "off writes NaN, which a vehicle reads as unchanged"
        );
    }

    #[test]
    fn the_launch_altitude_is_an_editable_field_on_mission_settings() {
        let field = launch_altitude_field(
            123.5,
            "m",
            "plan.missionController.visualItems.0.plannedHomePositionAltitude",
        );
        assert_eq!(
            field["path"],
            "plan.missionController.visualItems.0.plannedHomePositionAltitude"
        );
        assert_eq!(
            field["label"], "Altitude",
            "MissionSettingsEditor's Launch Position altitude"
        );
    }

    #[test]
    fn an_item_reads_edited_until_the_plan_it_is_in_is_saved() {
        let saved = plandoc::load(
            include_str!("../../test/MissionManager/SectionTest.plan"),
            2,
        )
        .unwrap();
        let moved = plandoc::set_altitude(&saved, 2, 33.0).unwrap();
        let rows = json!({ "items": std::iter::repeat_n(json!({}), moved.items.len() + 1).collect::<Vec<_>>() });
        let marked = marked_edited(rows.clone(), &moved, Some(&saved));
        let edited: Vec<bool> = marked["items"]
            .as_array()
            .unwrap()
            .iter()
            .skip(1)
            .map(|r| r["edited"].as_bool().unwrap())
            .collect();
        assert_eq!(
            edited.iter().filter(|e| **e).count(),
            1,
            "only the item whose altitude changed: {edited:?}"
        );
        assert!(edited[1]);
        let never_saved = marked_edited(rows, &moved, None);
        assert!(
            never_saved["items"]
                .as_array()
                .unwrap()
                .iter()
                .skip(1)
                .all(|r| r["edited"] == true),
            "a plan that was never saved or loaded is all new"
        );
    }

    #[test]
    fn a_polyline_keeps_two_vertices_and_splits_only_its_real_segments() {
        let line = vec![json!([0.0, 0.0]), json!([0.0, 1.0]), json!([0.0, 2.0])];
        assert_eq!(
            vertex_edit(&line, "splitSegment", &json!([1]), false).map(|v| v.len()),
            Some(4)
        );
        assert!(
            vertex_edit(&line, "splitSegment", &json!([2]), false).is_none(),
            "the last vertex starts no segment on a line"
        );
        assert_eq!(
            vertex_edit(&line, "splitPolygonSegment", &json!([2]), true).map(|v| v.len()),
            Some(4),
            "a ring closes back to its first vertex"
        );
        assert_eq!(
            vertex_edit(&line, "clear", &json!([]), false),
            Some(Vec::new()),
            "QGCMapPolygon::clear empties the shape before a reset refills it"
        );
        let refilled = vertex_edit(&[], "appendVertices", &json!([[{ "latitude": 1.0, "longitude": 2.0 }, { "latitude": 3.0, "longitude": 4.0 }]]), true).unwrap();
        assert_eq!(refilled, vec![json!([1.0, 2.0]), json!([3.0, 4.0])]);
        assert!(
            vertex_edit(
                &[],
                "appendVertices",
                &json!([[{ "latitude": 91.0, "longitude": 2.0 }]]),
                true
            )
            .is_none(),
            "one bad coordinate refuses the whole list"
        );
        let two = vertex_edit(&line, "removeVertex", &json!([0]), false).unwrap();
        assert!(vertex_edit(&two, "removeVertex", &json!([0]), false).is_none());
    }

    #[test]
    fn a_plan_kept_across_a_reload_comes_back_untitled_and_unsent_without_replacing_one_in_progress()
     {
        let drawn = Document {
            items: vec![plandoc::Item::Simple(plandoc::Simple {
                command: 16,
                frame: 3,
                params: [
                    Some(0.0),
                    Some(0.0),
                    Some(0.0),
                    Some(0.0),
                    Some(47.0),
                    Some(8.0),
                    Some(50.0),
                ],
                auto_continue: true,
                altitude: None,
                sections: Vec::new(),
            })],
            ..empty_document()
        };
        let mut fresh = Held::default();
        assert_eq!(draft(&fresh), None, "nothing drawn, nothing kept");
        assert!(restored_into(&mut fresh, drawn.clone()));
        assert_eq!(
            (fresh.file.clone(), fresh.dirty, fresh.dirty_for_save),
            (None, true, true),
            "the reloaded plan is the one being worked on, not on the aircraft and not saved"
        );
        assert!(draft(&fresh).is_some());
        let mut working = Held {
            document: Some(Document {
                cruise_speed: 9.0,
                ..drawn.clone()
            }),
            ..Held::default()
        };
        assert!(
            !restored_into(&mut working, drawn),
            "a plan already in progress is never replaced by the one kept from before"
        );
        assert_eq!(working.document.map(|d| d.cruise_speed), Some(9.0));
    }

    #[test]
    fn a_plan_kept_when_another_aircraft_connects_still_has_to_go_up_to_it() {
        let uploaded = Document {
            cruise_speed: 12.0,
            ..empty_document()
        };
        let mut state = Held {
            document: Some(uploaded.clone()),
            clean: Some(uploaded),
            vehicle_prompt: Some(false),
            ..Held::default()
        };
        keep_for_this_vehicle(&mut state);
        assert_eq!(
            (state.vehicle_prompt, state.dirty, state.clean),
            (None, true, None),
            "the plan went up to the aircraft that left, so the new one holds none of it and Plan offers Upload, not Fly"
        );
    }

    #[test]
    fn edits_in_one_burst_undo_together_and_a_pause_starts_a_new_step() {
        let doc = |n: usize| Document {
            cruise_speed: n as f64,
            ..empty_document()
        };
        let mut state = Held {
            document: Some(doc(0)),
            ..Held::default()
        };
        let apply = |state: &mut Held, n: usize, at: u64| {
            let previous = state.document.replace(doc(n));
            remember(state, previous, at);
        };
        apply(&mut state, 1, 10_000);
        apply(&mut state, 2, 10_100);
        assert_eq!(
            state.undo,
            vec![doc(0)],
            "a drag's many writes are one step back to where it started"
        );
        apply(&mut state, 3, 11_000);
        assert_eq!(state.undo, vec![doc(0), doc(2)]);
        (4..4 + UNDO_DEPTH + 5).for_each(|n| apply(&mut state, n, 20_000 + n as u64 * 1_000));
        assert_eq!(
            state.undo.len(),
            UNDO_DEPTH,
            "the oldest steps fall off as Qt's do"
        );
    }

    #[test]
    fn a_sequence_number_selects_the_row_that_holds_it_as_qt_selects_it() {
        let doc = plandoc::load(
            include_str!("../../test/MissionManager/SectionTest.plan"),
            2,
        )
        .unwrap();
        let rows: Vec<Option<i64>> = (0..8)
            .map(|seq| visual_index_of_sequence(&doc, seq))
            .collect();
        assert_eq!(
            rows,
            vec![
                Some(0),
                Some(1),
                Some(2),
                Some(3),
                Some(3),
                Some(4),
                None,
                None
            ],
            "sequence 4 is the mount control folded into row 3, so selecting it selects row 3"
        );
    }
}

pub struct PlanState {
    pub syncing: bool,
    pub downloading: bool,
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
    section
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|list| !list.is_empty())
}

pub fn plan_state() -> Option<PlanState> {
    if !enabled() {
        return None;
    }
    let mission = crate::hub::lock()
        .active()
        .map(|v| v.mission_snapshot()["mission"].clone())
        .unwrap_or_default();
    let syncing = mission["inProgress"].as_bool() == Some(true);
    let reading = mission["transaction"].as_str() == Some("Read");
    let progress = mission["progress"].as_f64().unwrap_or(0.0);
    let state = held();
    let document = state.document.clone().unwrap_or_else(empty_document);
    let has_mission_items = !document.items.is_empty();
    Some(PlanState {
        syncing,
        downloading: syncing && (reading || state.fetching),
        progress,
        dirty: state.dirty,
        dirty_for_save: state.dirty_for_save,
        contains_items: has_mission_items
            || has_entries(&document.fence, "polygons")
            || has_entries(&document.fence, "circles")
            || has_entries(&document.rally, "points"),
        has_mission_items,
        file: state.file.clone().unwrap_or_default(),
        global_mode: document.global_altitude_mode,
        item_count: document.items.len() as i64,
    })
}

pub fn current_document() -> Document {
    held().document.clone().unwrap_or_else(empty_document)
}

pub fn planning_speeds(backend: &dyn Backend) -> crate::missionitems::Speeds {
    let flown = |names: &[&str]| crate::battery::parameter_si(backend, names).filter(|s| *s > 0.0);
    let setting = |name: &str, default: f64| {
        crate::read::value_number(&backend.value(&format!("settings.appSettings.{name}.rawValue")))
            .unwrap_or(default)
    };
    crate::missionitems::Speeds {
        hover: flown(&["MPC_XY_CRUISE", "WP_SPD", "WPNAV_SPEED", "Q_WP_SPEED"])
            .unwrap_or_else(|| setting("offlineEditingHoverSpeed", 5.0)),
        cruise: flown(&[
            "FW_AIRSPD_TRIM",
            "AIRSPEED_CRUISE",
            "TRIM_ARSPD_CM",
            "WP_SPEED",
        ])
        .unwrap_or_else(|| setting("offlineEditingCruiseSpeed", 15.0)),
        ascent: flown(&["MPC_Z_V_AUTO_UP", "WP_SPD_UP", "WPNAV_SPEED_UP"])
            .unwrap_or_else(|| setting("offlineEditingAscentSpeed", 3.0)),
        descent: flown(&["MPC_Z_V_AUTO_DN", "WP_SPD_DN", "WPNAV_SPEED_DN"])
            .unwrap_or_else(|| setting("offlineEditingDescentSpeed", 1.0)),
    }
}

pub fn summary_fields(backend: &dyn Backend) -> Option<Value> {
    if !enabled() {
        return None;
    }
    let document = held().document.clone().unwrap_or_else(empty_document);
    let status = crate::missionitems::flight_status(&document, &planning_speeds(backend));
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

const OPTIONAL_RAW_PARAMS: usize = 4;

fn formatted(value: f64, decimals: i64) -> String {
    let places = usize::try_from(decimals).unwrap_or(0);
    match value.is_nan() {
        true => String::new(),
        false => format!("{value:.places$}"),
    }
}

fn list(param: &Value, key: &str) -> Vec<String> {
    param
        .get(key)
        .and_then(Value::as_str)
        .map(|joined| joined.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default()
}

fn number_json(value: f64) -> Value {
    match value.fract() == 0.0 && value.abs() < 1e15 {
        true => json!(value as i64),
        false => json!(value),
    }
}

fn param_conversion(param: &Value) -> Option<crate::units::Conversion> {
    let units = param.get("units").and_then(Value::as_str)?;
    list(param, "enumStrings")
        .is_empty()
        .then(|| crate::units::built_in(units).or_else(|| crate::units::cooking(units)))
        .flatten()
}

fn param_fact(param: &Value, raw: f64) -> Value {
    param_fact_with(param, raw, param_conversion(param))
}

fn param_fact_with(param: &Value, raw: f64, conversion: Option<crate::units::Conversion>) -> Value {
    let show = |v: f64| conversion.map_or(v, |c| (c.shown)(v));
    let label = param
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let decimals = param
        .get("decimalPlaces")
        .and_then(Value::as_i64)
        .unwrap_or(DEFAULT_DECIMAL_PLACES);
    let raw_units = param
        .get("units")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let units = conversion.map_or_else(|| raw_units.clone(), |c| c.name.to_string());
    let value = show(raw);
    let default = param.get("default").and_then(Value::as_f64).map(show);
    let min = param.get("min").and_then(Value::as_f64).map(show);
    let max = param.get("max").and_then(Value::as_f64).map(show);
    let labels = list(param, "enumStrings");
    let values: Vec<f64> = list(param, "enumValues")
        .iter()
        .filter_map(|v| v.parse().ok())
        .collect();
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
            false => {
                list(p, "enumStrings").is_empty()
                    && !flag(p, "nanUnchanged")
                    && !flag(p, "advanced")
            }
        })
        .collect()
}

static RAW_EDIT: std::sync::Mutex<std::collections::BTreeSet<usize>> =
    std::sync::Mutex::new(std::collections::BTreeSet::new());
const RAW_LABELS: [&str; 7] = [
    "Param 1", "Param 2", "Param 3", "Param 4", "Param 5", "Param 6", "Param 7",
];
const POSITION_LABELS: [&str; 3] = ["Latitude", "Longitude", "Altitude"];
const RAW_DECIMAL_PLACES: i64 = 7;
const MAV_FRAMES: [(&str, i64); 12] = [
    ("Above sea level", 0),
    ("Local north-east-down", 1),
    ("Mission", 2),
    ("Relative to launch", 3),
    ("Local east-north-up", 4),
    ("Above sea level (finer position)", 5),
    ("Relative to launch (finer position)", 6),
    ("Offset from the vehicle", 7),
    ("Body frame", 8),
    ("Offset from the body", 9),
    ("Above terrain", 10),
    ("Above terrain (finer position)", 11),
];
const FRIENDLY_FRAMES: [i64; 3] = [0, 3, 10];

fn raw_edits() -> std::sync::MutexGuard<'static, std::collections::BTreeSet<usize>> {
    RAW_EDIT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn forget_raw_edits() {
    raw_edits().clear();
}

fn shift_raw_edits(before: &[plandoc::Item], after: &[plandoc::Item]) {
    let shared = before
        .iter()
        .zip(after)
        .take_while(|(old, new)| old == new)
        .count();
    let (gone, added) = (
        before.len().saturating_sub(after.len()),
        after.len().saturating_sub(before.len()),
    );
    let mut chosen = raw_edits();
    let moved: std::collections::BTreeSet<usize> = chosen
        .iter()
        .filter(|index| **index <= shared || **index > shared + gone)
        .map(|index| {
            if *index <= shared {
                *index
            } else {
                *index + added - gone
            }
        })
        .collect();
    *chosen = moved;
}

fn raw_edit_chosen(index: usize) -> bool {
    raw_edits().contains(&index)
}

fn friendly_edit_allowed(simple: &plandoc::Simple, info: Option<&crate::cmdinfo::Command>) -> bool {
    info.is_some_and(|c| c.friendly_edit)
        && simple.auto_continue
        && (simple.altitude.is_none() || FRIENDLY_FRAMES.contains(&simple.frame))
}

fn raw_edit(
    simple: &plandoc::Simple,
    info: Option<&crate::cmdinfo::Command>,
    index: usize,
) -> bool {
    raw_edit_chosen(index) || !friendly_edit_allowed(simple, info)
}

fn choice(label: &str, choices: impl Iterator<Item = (String, i64)>) -> Value {
    let (names, values): (Vec<String>, Vec<String>) = choices
        .map(|(name, value)| (name.replace(',', ""), value.to_string()))
        .unzip();
    json!({ "label": label, "enumStrings": names.join(","), "enumValues": values.join(",") })
}

fn plain_units(units: &str) -> &str {
    units
        .strip_prefix("vertical ")
        .or_else(|| units.strip_prefix("horizontal "))
        .unwrap_or(units)
}

fn raw_fields(
    simple: &plandoc::Simple,
    commands: &std::collections::BTreeMap<i64, crate::cmdinfo::Command>,
    item: &str,
) -> Vec<Value> {
    let control = |param: Value, value: f64, suffix: String| match crate::control::decode(
        &param_fact(&param, value),
        &format!("{item}.{suffix}"),
    ) {
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .chain([
                    ("pathSuffix".to_string(), json!(suffix)),
                    ("group".to_string(), json!("Settings")),
                ])
                .collect(),
        ),
        other => other,
    };
    let info = commands.get(&simple.command);
    let text = RAW_LABELS.iter().enumerate().map(|(i, fallback)| {
        let param = info.and_then(|c| c.params.get(&u8::try_from(i + 1).ok()?));
        let text = |key: &str| {
            param
                .and_then(|p| p.get(key)?.as_str())
                .filter(|t| !t.trim().is_empty())
        };
        let position = info
            .filter(|c| c.specifies_coordinate)
            .and_then(|_| POSITION_LABELS.get(i.checked_sub(4)?));
        let label = text("label").or(position.copied()).unwrap_or(fallback);
        control(
            json!({
                "label": text("units").map_or_else(|| label.to_string(), |units| format!("{label} ({})", plain_units(units))),
                "decimalPlaces": RAW_DECIMAL_PLACES
            }),
            simple.params[i].unwrap_or(f64::NAN),
            format!("textFieldFacts.{i}"),
        )
    });
    let [command, frame] = raw_choices(commands);
    [
        control(
            command,
            simple.command as f64,
            "comboboxFacts.0".to_string(),
        ),
        control(frame, simple.frame as f64, "comboboxFacts.1".to_string()),
    ]
    .into_iter()
    .chain(text)
    .collect()
}

fn raw_choices(commands: &std::collections::BTreeMap<i64, crate::cmdinfo::Command>) -> [Value; 2] {
    [
        choice(
            "Command",
            commands.values().map(|c| {
                let shown = match c.friendly_name.trim().is_empty() {
                    true => &c.raw_name,
                    false => &c.friendly_name,
                };
                (shown.clone(), c.id)
            }),
        ),
        choice(
            "Frame",
            MAV_FRAMES
                .iter()
                .map(|(name, value)| ((*name).to_string(), *value)),
        ),
    ]
}

enum Choices<'a> {
    Raw,
    Friendly(&'a plandoc::Simple),
}

fn combo_value(
    choices: Choices,
    commands: &std::collections::BTreeMap<i64, crate::cmdinfo::Command>,
    slot: usize,
    at: usize,
) -> Option<f64> {
    let choices = match choices {
        Choices::Raw => raw_choices(commands).into_iter().nth(slot)?,
        Choices::Friendly(simple) => {
            field_params(commands.get(&simple.command)?, true)
                .into_iter()
                .nth(slot)?
                .1
        }
    };
    list(&choices, "enumValues").get(at)?.parse().ok()
}

fn simple_fields(
    simple: &plandoc::Simple,
    commands: &std::collections::BTreeMap<i64, crate::cmdinfo::Command>,
    item: &str,
    jump_stop: Option<f64>,
) -> Vec<Value> {
    let Some(info) = commands.get(&simple.command) else {
        return Vec::new();
    };
    let text = field_params(info, false);
    let combo = field_params(info, true);
    let text_params = text.iter().map(|(i, p)| (*i, p));
    let combo_params = combo.iter().map(|(i, p)| (*i, p));
    let build = |group: &'static str| {
        move |(at, (i, p)): (usize, (u8, &Value))| {
            let jumps_here = simple.command == plandoc::JUMP_COMMAND
                && usize::from(i) == plandoc::JUMP_TARGET_PARAM;
            let (value, p) = match jumps_here {
                true => (
                    jump_stop.unwrap_or(f64::NAN),
                    Value::Object(
                        p.as_object()
                            .cloned()
                            .unwrap_or_default()
                            .into_iter()
                            .chain([("label".to_string(), json!(JUMP_TARGET_LABEL))])
                            .collect(),
                    ),
                ),
                false => (
                    simple.params[usize::from(i) - 1].unwrap_or(f64::NAN),
                    p.clone(),
                ),
            };
            let suffix = format!("{group}.{at}");
            let mut control =
                crate::control::decode(&param_fact(&p, value), &format!("{item}.{suffix}"));
            if let Value::Object(map) = &mut control {
                map.insert("pathSuffix".to_string(), json!(suffix));
                map.insert("group".to_string(), json!("Settings"));
            }
            control
        }
    };
    let nan = nan_params(info);
    let optional = nan
        .iter()
        .map(|(i, p)| (*i, p))
        .enumerate()
        .map(build("nanFacts"))
        .map(|control| match control {
            Value::Object(map) => Value::Object(
                map.into_iter()
                    .chain([("optional".to_string(), json!(true))])
                    .collect(),
            ),
            other => other,
        });
    combo_params
        .enumerate()
        .map(build("comboboxFacts"))
        .chain(text_params.enumerate().map(build("textFieldFacts")))
        .chain(optional)
        .collect()
}

const HOLD_PARAM: u8 = 1;

const YAW_PARAM: u8 = 4;

fn yaw_field(
    simple: &plandoc::Simple,
    info: Option<&crate::cmdinfo::Command>,
    raw: bool,
    item: &str,
) -> Value {
    let label = info
        .and_then(|c| c.params.get(&YAW_PARAM))
        .and_then(|p| p.get("label"))
        .and_then(Value::as_str);
    match (raw, simple.command, label) {
        (false, 16, Some("Yaw")) => {
            json!({ "value": simple.params[usize::from(YAW_PARAM) - 1].filter(|v| v.is_finite()), "units": "deg", "path": format!("{item}.yaw") })
        }
        _ => Value::Null,
    }
}

fn hold_field(
    simple: &plandoc::Simple,
    info: Option<&crate::cmdinfo::Command>,
    raw: bool,
    item: &str,
) -> Value {
    let label = info
        .filter(|c| !c.hidden.contains(&HOLD_PARAM))
        .and_then(|c| c.params.get(&HOLD_PARAM))
        .and_then(|p| p.get("label"))
        .and_then(Value::as_str);
    match (raw, simple.command, label) {
        (false, 16, Some("Hold")) => {
            json!({ "value": simple.params[usize::from(HOLD_PARAM) - 1].filter(|v| v.is_finite()).unwrap_or(0.0), "units": "s", "path": format!("{item}.hold") })
        }
        _ => Value::Null,
    }
}

const FLIGHT_SPEED_USER_RANGE: (f64, f64) = (0.0, 30.0);

fn speed_section(
    document: &Document,
    index: usize,
    sections: &[plandoc::Simple],
    offered: bool,
    hover: f64,
    cruise: f64,
) -> Value {
    let available = offered
        && matches!(
            plandoc::vehicle_class(document.vehicle_type),
            crate::cmdinfo::VehicleClass::MultiRotor | crate::cmdinfo::VehicleClass::FixedWing
        );
    let item = format!("{ITEM_ROOT}.{index}.speedSection");
    let specified = sections
        .iter()
        .find(|s| s.command == 178)
        .and_then(|s| s.params[1]);
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
    crate::read::flag(&backend.value("vehicle.homePosition"), "valid")
}

pub fn mission_speed_section(backend: &dyn Backend) -> Value {
    let document = held().document.clone().unwrap_or_else(empty_document);
    let speeds = planning_speeds(backend);
    speed_section(
        &document,
        0,
        &document.settings_sections,
        true,
        speeds.hover,
        speeds.cruise,
    )
}

pub fn item_facts(backend: &dyn Backend, index: usize) -> Value {
    let (document, wizard_at) = {
        let state = held();
        (
            state.document.clone().unwrap_or_else(empty_document),
            state.wizard,
        )
    };
    facts_in(backend, &document, wizard_at, index)
}

fn facts_in(
    backend: &dyn Backend,
    document: &Document,
    wizard_at: Option<usize>,
    index: usize,
) -> Value {
    let speeds = planning_speeds(backend);
    let (vertical, horizontal) = (
        crate::read::Unit::vertical(backend),
        crate::read::Unit::horizontal(backend),
    );
    let facts = document_facts(
        document,
        index,
        speeds.hover,
        speeds.cruise,
        (&vertical, &horizontal),
    );
    let launch = (index == 0 && !vehicle_has_home(backend))
        .then(|| {
            document.home.map(|home| {
                launch_altitude_field(
                    vertical.show(home[2]),
                    &vertical.name,
                    &format!("{ITEM_ROOT}.0.{LAUNCH_ALTITUDE}"),
                )
            })
        })
        .flatten();
    let read = crate::missionitems::document_reads(document, -1)
        .ok()
        .and_then(|reads| reads.get(index).cloned())
        .unwrap_or_default();
    let wizard = wizard_at == Some(index)
        && read.get("isTakeoffItem").and_then(Value::as_bool) == Some(true);
    let wizard_rows = crate::itemfacts::wizard_info(
        wizard,
        plandoc::vehicle_class(document.vehicle_type) == crate::cmdinfo::VehicleClass::Vtol,
    );
    match with_previous_coordinate(facts, plandoc::previous_coordinate(document, index as i64)) {
        Value::Object(map) => {
            let takeoff_wizard: Vec<(String, Value)> = wizard_rows
                .into_iter()
                .filter(|(key, _)| !map.contains_key(key))
                .collect();
            crate::itemfacts::without_hidden_mission_speed(
                Value::Object(
                    map.into_iter()
                        .chain([("launchAltitude".to_string(), launch.unwrap_or(Value::Null))])
                        .chain([("drop".to_string(), drop_fact(document, index))])
                        .chain([("actions".to_string(), actions_fact(document, index))])
                        .chain(crate::itemfacts::command_info(&read))
                        .chain(takeoff_wizard)
                        .collect(),
                ),
                index,
                plandoc::vehicle_class(document.vehicle_type) == crate::cmdinfo::VehicleClass::Vtol,
                plandoc::firmware(document.firmware_type) == crate::cmdinfo::Firmware::ArduPilot,
            )
        }
        other => other,
    }
}

fn step_name(kind: plandoc::StepKind) -> &'static str {
    match kind {
        plandoc::StepKind::Hover => "Hover",
        plandoc::StepKind::Wait => "Wait",
        plandoc::StepKind::Gimbal => "Gimbal",
        plandoc::StepKind::Camera => "Camera",
        plandoc::StepKind::Mode => "Mode",
        plandoc::StepKind::Drop => "Drop",
        plandoc::StepKind::Command => "Command",
    }
}

fn actions_fact(document: &Document, waypoint: usize) -> Value {
    let commands = plandoc::commands_at(document, waypoint.saturating_sub(1));
    let servos = plan_servos(document);
    plandoc::steps(document, waypoint, &servos)
        .zip(plandoc::action_places(document, waypoint, &servos))
        .map_or(Value::Null, |(steps, places)| {
            json!(
                steps
                    .iter()
                    .zip(places)
                    .map(|(step, (to, refused))| {
                        let title = match step.kind {
                            plandoc::StepKind::Wait => "Wait".to_string(),
                            plandoc::StepKind::Command => step
                                .commands
                                .first()
                                .and_then(|command| commands.get(&command.command))
                                .map_or_else(String::new, |info| {
                                    crate::label::sentence(&info.friendly_name)
                                }),
                            kind => step_name(kind).to_string(),
                        };
                        json!({
                            "kind": step_name(step.kind),
                            "title": title,
                            "seconds": step.seconds,
                            "item": step.item,
                            "places": to,
                            "refused": refused
                                .iter()
                                .map(|it| json!({ "place": it.place, "reason": it.reason, "about": it.about }))
                                .collect::<Vec<_>>(),
                        })
                    })
                    .collect::<Vec<_>>()
            )
        })
}

fn move_action(args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let place = |i: usize| {
        given
            .get(i)
            .and_then(Value::as_u64)
            .and_then(|v| usize::try_from(v).ok())
    };
    match (place(0), place(1), place(2)) {
        (Some(waypoint), Some(from), Some(to)) => edit_with(|doc| {
            plandoc::move_action(doc, waypoint, from, to, &plan_servos(doc)).map(|document| {
                Edited {
                    document,
                    selected: None,
                    note: None,
                    plain: None,
                    own_step: true,
                }
            })
        }),
        _ => refused("Moving an action needs the waypoint, the action's place and its new place."),
    }
}

fn set_wait(args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let place = |i: usize| {
        given
            .get(i)
            .and_then(Value::as_u64)
            .and_then(|v| usize::try_from(v).ok())
    };
    match (
        place(0),
        place(1),
        given
            .get(2)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
    ) {
        (Some(waypoint), Some(at), Some(seconds)) => {
            edit(|doc| plandoc::set_wait(doc, waypoint, at, seconds, &plan_servos(doc)))
        }
        _ => refused(plandoc::WAIT_SECONDS_RANGE),
    }
}

fn drop_fact(document: &Document, waypoint: usize) -> Value {
    let servos = plan_servos(document);
    plandoc::drop_after(document, waypoint, &servos).map_or(Value::Null, |drop| {
        let release = drop.and_then(|at| plandoc::drop_release(document, at, &servos));
        json!({
            "index": drop,
            "wait": drop.map_or(0.0, |at| plandoc::drop_wait(document, waypoint, at)),
            "waitOffered": plandoc::wait_offered(document),
            "servo": match drop {
                Some(_) => release.is_some(),
                None => !servos.is_empty(),
            },
            "output": release.map(|release| release.output),
            "pwm": release.map(|release| release.pwm),
            "bay": release.and_then(|release| release.bay).filter(|_| servos.len() > 1),
        })
    })
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
    let plandoc::Item::Simple(takeoff) = doc.items.first()? else {
        return None;
    };
    [CMD_NAV_TAKEOFF, CMD_NAV_VTOL_TAKEOFF_ID]
        .contains(&takeoff.command)
        .then_some(())?;
    let at = takeoff.params[4].zip(takeoff.params[5])?;
    Some(crate::surveygrid::azimuth_to((home[0], home[1]), at))
}

fn entry_row(kind: &str, pattern: &Value, item: &str) -> Value {
    let path = format!("{item}.rotateEntryPoint");
    match kind {
        "StructureScan" => {
            json!({ "label": "Entry vertex", "value": (crate::structurescan::entry_vertex(pattern) + 1).to_string(), "path": path })
        }
        _ => crate::surveydoc::entry_point_name(kind, crate::surveydoc::entry_point(kind, pattern))
            .map_or(
                Value::Null,
                |name| json!({ "label": "Start from", "value": name, "path": path }),
            ),
    }
}

fn shape_complete(kind: &str, pattern: &Value) -> bool {
    let (key, least) = if kind == "CorridorScan" {
        ("polyline", 2)
    } else {
        ("polygon", 3)
    };
    pattern
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|vertices| vertices.len() >= least)
}

fn document_facts(
    document: &Document,
    index: usize,
    hover: f64,
    cruise: f64,
    units: (&crate::read::Unit, &crate::read::Unit),
) -> Value {
    let document = document.clone();
    let commands = crate::cmdinfo::tree(
        plandoc::firmware(document.firmware_type),
        plandoc::command_class_at(&document, index.saturating_sub(1)),
    );
    let item = format!("{ITEM_ROOT}.{index}");
    let base = |simple: bool, fields: Vec<Value>, section: Value, mode: Option<i64>| json!({ "kind": "object", "class": "ItemFacts", "available": true, "index": index, "simple": simple, "fields": fields, "camera": Value::Null, "speedSection": section, "altitudeMode": mode });
    match index.checked_sub(1).map(|i| document.items.get(i)) {
        None => base(
            false,
            Vec::new(),
            speed_section(
                &document,
                index,
                &document.settings_sections,
                true,
                hover,
                cruise,
            ),
            None,
        ),
        Some(Some(plandoc::Item::Simple(s))) => {
            let info = commands.get(&s.command);
            let raw = raw_edit(s, info, index);
            let facts = base(
                true,
                if raw {
                    raw_fields(s, &commands, &item)
                } else {
                    simple_fields(
                        s,
                        &commands,
                        &item,
                        s.params[plandoc::JUMP_TARGET_PARAM - 1]
                            .and_then(|sequence| jump_stop(&document, sequence)),
                    )
                },
                speed_section(
                    &document,
                    index,
                    &s.sections,
                    s.command == 16,
                    hover,
                    cruise,
                ),
                Some(
                    s.altitude
                        .as_ref()
                        .map_or(crate::altitudemodes::RELATIVE, |a| a.mode),
                ),
            );
            match facts {
                Value::Object(fields) => {
                    let land = info.is_some_and(|c| c.is_land);
                    let hint = s.altitude.as_ref().and_then(|a| {
                        crate::itemfacts::altitude_hint(
                            land,
                            Some(a.mode),
                            a.amsl_above_terrain
                                .filter(|v| v.is_finite())
                                .map(|metres| units.0.label(metres)),
                        )
                    });
                    Value::Object(
                        fields
                            .into_iter()
                            .chain([
                                ("rawEdit".to_string(), json!(raw)),
                                (
                                    "friendlyEditAllowed".to_string(),
                                    json!(friendly_edit_allowed(s, info)),
                                ),
                                ("altitudeHint".to_string(), json!(hint)),
                                ("hold".to_string(), hold_field(s, info, raw, &item)),
                                ("yaw".to_string(), yaw_field(s, info, raw, &item)),
                            ])
                            .collect(),
                    )
                }
                other => other,
            }
        }
        Some(Some(plandoc::Item::Complex {
            kind, json: survey, ..
        })) if kind == "survey" || kind == "CorridorScan" || kind == "StructureScan" => {
            let units = crate::surveydoc::Units {
                vertical: units.0,
                horizontal: units.1,
            };
            json!({ "kind": "object", "class": "ItemFacts", "available": true, "index": index, "simple": false, "fields": crate::surveydoc::fields(survey, &item, plandoc::vehicle_class(document.vehicle_type), &units), "camera": crate::surveydoc::camera(survey, &item, &units, plandoc::firmware(document.firmware_type) != crate::cmdinfo::Firmware::Px4), "speedSection": Value::Null, "altitudeMode": Value::Null, "presetKind": crate::presets::settings_group(kind).map(|_| kind.clone()), "areaHelp": crate::itemfacts::area_help(kind, shape_complete(kind, survey)), "gridNote": crate::itemfacts::grid_note(kind), "entryPoint": entry_row(kind, survey, &item) })
        }
        Some(Some(plandoc::Item::Complex {
            kind,
            json: pattern,
            ..
        })) if crate::landingpattern::is_landing(kind) => {
            let units = crate::surveydoc::Units {
                vertical: units.0,
                horizontal: units.1,
            };
            json!({ "kind": "object", "class": "ItemFacts", "available": true, "index": index, "simple": false, "fields": crate::landingpattern::fields(pattern, &item, &units), "camera": Value::Null, "speedSection": Value::Null, "altitudeMode": Value::Null, "landing": true, "landingNotes": crate::landingpattern::notes(kind == crate::landingpattern::VTOL_PATTERN), "wizardMode": pattern.get(crate::landingpattern::WIZARD).and_then(Value::as_bool).unwrap_or(false), "wizardText": crate::landingpattern::wizard_text(pattern), "altitudesAreRelative": pattern.get("altitudesAreRelative").and_then(Value::as_bool).unwrap_or(true) })
        }
        Some(Some(plandoc::Item::Complex { kind, .. })) => {
            json!({ "kind": "object", "class": "ItemFacts", "available": false, "index": index, "reason": format!("The core cannot edit a {kind} item yet.") })
        }
        Some(None) => {
            json!({ "kind": "object", "class": "ItemFacts", "available": false, "index": index })
        }
    }
}

pub fn fly_fence_and_rally() -> Option<(Value, Value)> {
    if !enabled() {
        return None;
    }
    let snapshot = crate::hub::lock().active().map(|v| v.mission_snapshot());
    Some(match snapshot {
        Some(snapshot) => (
            fence_from(&snapshot["fence"]),
            json!({ "version": 2, "points": snapshot["rally"]["points"] }),
        ),
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
    (simple.command == 178)
        .then_some(simple.params[1])
        .flatten()
        .filter(|speed| *speed > 0.0)
}

fn speed_in_force(document: &Document, before: usize, hover: f64, cruise: f64) -> f64 {
    let hovering = matches!(
        plandoc::vehicle_class(document.vehicle_type),
        crate::cmdinfo::VehicleClass::MultiRotor | crate::cmdinfo::VehicleClass::Vtol
    );
    let start = if hovering { hover } else { cruise };
    let settings = document
        .settings_sections
        .iter()
        .filter_map(specified_speed);
    let items = document
        .items
        .iter()
        .take(before)
        .flat_map(|item| match item {
            plandoc::Item::Simple(s) => std::iter::once(s)
                .chain(s.sections.iter())
                .filter_map(specified_speed)
                .collect::<Vec<_>>(),
            plandoc::Item::Complex { .. } => Vec::new(),
        });
    settings.chain(items).last().unwrap_or(start)
}

const BREACH_RETURN_META: &str =
    include_str!("../../src/MissionManager/BreachReturn.FactMetaData.json");
fn breach_altitude_now() -> Option<f64> {
    let (fence, pending) = {
        let state = held();
        (
            state
                .document
                .as_ref()
                .map(|d| d.fence.clone())
                .unwrap_or(Value::Null),
            state.breach_altitude,
        )
    };
    fence
        .get("breachReturn")
        .and_then(Value::as_array)
        .and_then(|b| b.get(2))
        .and_then(Value::as_f64)
        .or(pending)
        .or_else(|| crate::settingsstore::raw_setting(DEFAULT_ALTITUDE).and_then(|v| v.as_f64()))
}

pub fn breach_altitude_fact() -> Option<Value> {
    if !enabled() {
        return None;
    }
    let altitude = breach_altitude_now()?;
    let meta = crate::factmeta::fact(BREACH_RETURN_META, "Altitude")?;
    let mut fact = crate::settingsstore::fact_json(
        &meta,
        &json!(altitude),
        crate::units::cooking("vertical m"),
    );
    ["defaultValueString", "userVisible", "visible"]
        .iter()
        .for_each(|key| fact[*key] = Value::Null);
    Some(fact)
}

const CAMERA_INTERVAL_DISTANCE: &str = "cameraPhotoIntervalDistance";

fn cooked_camera_distance(section: Value, cooking: Option<crate::units::Conversion>) -> Value {
    let Some(conversion) = cooking else {
        return section;
    };
    let cook = |fact: &Value| match fact["property"] == CAMERA_INTERVAL_DISTANCE {
        true => {
            let shown = fact["value"].as_f64().map(conversion.shown);
            let text = shown.map(|v| format!("{v:.1}"));
            Value::Object(
                fact.as_object()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .chain([
                        ("value".to_string(), json!(shown)),
                        ("valueString".to_string(), json!(text)),
                        ("enumOrValueString".to_string(), json!(text)),
                        ("units".to_string(), json!(conversion.name)),
                    ])
                    .collect(),
            )
        }
        false => fact.clone(),
    };
    match section {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| match (key.as_str(), value) {
                    ("facts", Value::Array(facts)) => {
                        (key, Value::Array(facts.iter().map(cook).collect()))
                    }
                    (_, value) => (key, value),
                })
                .collect(),
        ),
        other => other,
    }
}

pub fn camera_section(index: usize) -> Option<Value> {
    if !enabled() {
        return None;
    }
    let document = held().document.clone().unwrap_or_else(empty_document);
    let supports_mode = crate::cmdinfo::tree(
        plandoc::firmware(document.firmware_type),
        crate::cmdinfo::VehicleClass::Generic,
    )
    .contains_key(&530);
    let with_support = |section: Value| match section {
        Value::Object(map) => {
            let specified = map
                .get("specifyCameraMode")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            Value::Object(
                map.into_iter()
                    .chain([(
                        "cameraModeSupported".to_string(),
                        json!(specified || supports_mode),
                    )])
                    .collect(),
            )
        }
        other => other,
    };
    let shown = |sections: &[plandoc::Simple]| {
        with_support(cooked_camera_distance(
            plandoc::camera_section(sections),
            crate::units::cooking("m"),
        ))
    };
    Some(
        match index.checked_sub(1).map(|at| document.items.get(at)) {
            None => shown(&document.settings_sections),
            Some(Some(plandoc::Item::Simple(simple)))
                if simple.command == plandoc::CMD_NAV_WAYPOINT =>
            {
                shown(&plandoc::run_commands(&document, index - 1))
            }
            _ => json!({ "kind": "null" }),
        },
    )
}

pub fn landing_inputs(index: usize) -> Option<(Value, Value)> {
    if !enabled() {
        return None;
    }
    let item = held()
        .document
        .clone()
        .zip(index.checked_sub(1))
        .and_then(|(document, at)| document.items.get(at).cloned());
    Some(match item {
        Some(plandoc::Item::Complex { json: pattern, .. }) => {
            crate::landingpattern::view_inputs(&pattern).unwrap_or((
                json!({ "kind": "object", "isSimpleItem": false }),
                json!({ "kind": "null" }),
            ))
        }
        Some(_) => (
            json!({ "kind": "object", "isSimpleItem": true }),
            json!({ "kind": "null" }),
        ),
        None => (
            json!({ "kind": "object", "isSimpleItem": false }),
            json!({ "kind": "null" }),
        ),
    })
}

pub fn survey_stats_inputs(backend: &dyn Backend, index: usize) -> Option<(Value, Value)> {
    if !enabled() {
        return None;
    }
    let not_survey = || {
        Some((
            json!({ "kind": "object", "isSurveyItem": false }),
            json!({ "kind": "null" }),
        ))
    };
    let Some((document, at)) = held().document.clone().zip(index.checked_sub(1)) else {
        return not_survey();
    };
    let Some(plandoc::Item::Complex { json: item, .. }) = document.items.get(at) else {
        return not_survey();
    };
    let kind = item
        .get("complexItemType")
        .and_then(Value::as_str)
        .unwrap_or("");
    let speeds = planning_speeds(backend);
    let speed = speed_in_force(&document, at, speeds.hover, speeds.cruise);
    let number = |v: &Value, key: &str| v.get(key).and_then(Value::as_f64).unwrap_or(0.0);
    let per_second = |metres: f64| if speed == 0.0 { 0.0 } else { metres / speed };
    let horizontal = crate::read::Unit::horizontal(backend);
    let vertical = crate::read::Unit::vertical(backend);
    let metres_fact = |property: &str, metres: f64| json!({ "property": property, "value": horizontal.show(metres), "rawValue": metres, "units": horizontal.name });
    let height_fact = |property: &str, metres: f64| json!({ "property": property, "value": vertical.show(metres), "rawValue": metres, "units": vertical.name });
    let calc_facts = |calc: &Value| {
        json!({ "kind": "object", "facts": [
        metres_fact("adjustedFootprintSide", number(calc, "AdjustedFootprintSide")),
        metres_fact("adjustedFootprintFrontal", number(calc, "AdjustedFootprintFrontal")),
        height_fact("distanceToSurface", number(calc, "DistanceToSurface")),
        { "property": "minTriggerInterval", "value": number(calc, "MinTriggerInterval"), "rawValue": number(calc, "MinTriggerInterval") },
    ] })
    };
    let stats = |shots: f64, seconds: f64, area: f64, distance: f64| json!({ "kind": "object", "isSurveyItem": kind == "survey", "cameraShots": shots, "timeBetweenShots": seconds, "coveredArea": area, "complexDistance": distance });
    let (stats, facts) = match kind {
        "survey" | "CorridorScan" => {
            let transect = &item["TransectStyleComplexItem"];
            let calc = &transect["CameraCalc"];
            let visual: Vec<(f64, f64)> = transect["VisualTransectPoints"]
                .as_array()
                .map(|p| {
                    p.iter()
                        .filter_map(|v| Some((v.get(0)?.as_f64()?, v.get(1)?.as_f64()?)))
                        .collect()
                })
                .unwrap_or_default();
            let area = match kind {
                "survey" => crate::mappolygon::area(&crate::surveydoc::polygon(item)),
                _ => crate::mappolygon::area(
                    &crate::corridorscan::corridor_polygon(item)
                        .iter()
                        .map(|(latitude, longitude, _)| (*latitude, *longitude))
                        .collect::<Vec<_>>(),
                ),
            };
            let distance = visual
                .windows(2)
                .map(|pair| crate::surveygrid::distance_between(pair[0], pair[1]))
                .sum::<f64>();
            (
                stats(
                    number(transect, "CameraShots"),
                    per_second(number(calc, "AdjustedFootprintFrontal")),
                    area,
                    distance,
                ),
                calc_facts(calc),
            )
        }
        "StructureScan" => {
            let flight = crate::structurescan::saved_flight(item).unwrap_or_default();
            let plan = crate::structurescan::saved_plan(item);
            let shots =
                crate::structurescan::camera_shots(&flight, plan.adjusted_side, plan.layers) as f64;
            let (bottom, top) = crate::structurescan::flight_alts(&plan);
            let mut structure = stats(
                shots,
                per_second(plan.adjusted_side),
                0.0,
                crate::structurescan::scan_distance(&flight, &plan),
            );
            structure["layers"] = json!(plan.layers);
            structure["bottomFlightAlt"] = json!(bottom);
            structure["topFlightAlt"] = json!(top);
            (structure, calc_facts(&item["CameraCalc"]))
        }
        _ => {
            let row = crate::landingpattern::row(item);
            let slope = crate::landingpattern::slope_start(item);
            let distance = row.zip(slope).map_or(0.0, |(row, slope)| {
                crate::surveygrid::distance_between(row.approach, slope)
                    + crate::surveygrid::distance_between(slope, row.land)
            });
            (stats(0.0, 0.0, 0.0, distance), json!({ "kind": "null" }))
        }
    };
    Some((stats, facts))
}
