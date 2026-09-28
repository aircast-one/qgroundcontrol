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
const ACTIONS: &[&str] = &[OPEN, SAVE, SEND, FETCH, STATUS, CORE_INSERT_WAYPOINT, CORE_REMOVE, CORE_INSERT_LAND, CORE_SET_COMMAND, CORE_SET_ALTITUDE, CORE_INSERT_TAKEOFF, CORE_SET_ALTITUDE_MODE];
const DEFAULT_ALTITUDE: &str = "settings.appSettings.defaultMissionItemAltitude";

#[derive(Default)]
struct Held {
    document: Option<Document>,
    fetching: bool,
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
        CORE_REMOVE => remove(args),
        _ => refused(format!("{path} is not a plan action the core performs")),
    }
}

fn held() -> std::sync::MutexGuard<'static, Held> {
    HELD.lock().unwrap_or_else(PoisonError::into_inner)
}

fn edit(change: impl FnOnce(&Document) -> Result<Document, String>) -> Value {
    let mut state = held();
    let Some(current) = state.document.as_ref() else {
        return refused("There is no plan to edit.");
    };
    match change(current) {
        Ok(changed) => {
            let count = changed.items.len();
            state.document = Some(changed);
            json!({ "ok": true, "items": count })
        }
        Err(reason) => refused(reason),
    }
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

fn set_altitude_mode(args: &str) -> Value {
    let mode = serde_json::from_str::<Value>(args).ok().and_then(|v| v.get(0).and_then(Value::as_i64));
    match mode.filter(|m| (crate::altitudemodes::MIXED..=crate::altitudemodes::TERRAIN_FRAME).contains(m)) {
        Some(mode) => edit(|doc| Ok(plandoc::set_global_altitude_mode(doc, mode))),
        None => refused("An altitude mode is a number from 0 to 4."),
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
            held().document = Some(document);
            json!({ "ok": true, "items": count })
        }
        Err(reason) => refused(reason),
    }
}

fn save(file: &str) -> Value {
    let Some(text) = held().document.as_ref().map(|d| plandoc::save(d).to_string()) else {
        return refused("There is no plan to save.");
    };
    match std::fs::write(file, text) {
        Ok(()) => json!({ "ok": true }),
        Err(e) => refused(format!("Could not write {file}: {e}")),
    }
}

fn deliver(outbound: Vec<(u32, Vec<u8>)>) {
    outbound.iter().for_each(|(link, bytes)| {
        crate::linkhost::write(&crate::linkhost::TRANSPORTS, *link, bytes);
    });
}

fn send() -> Value {
    let Some(document) = held().document.clone() else {
        return refused("There is no plan to send.");
    };
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
            json!({ "ok": true, "items": items.len() })
        }
        Err(reason) => refused(reason),
    }
}

fn fetch() -> Value {
    let started = crate::hub::lock().mission_request(None, &json!({ "action": "load" }), crate::hub::now_ms());
    match started {
        Ok(outbound) => {
            held().fetching = true;
            deliver(outbound);
            json!({ "ok": true })
        }
        Err(reason) => refused(reason),
    }
}

fn number(value: &Value) -> f64 {
    value.as_f64().unwrap_or(f64::NAN)
}

fn status() -> Value {
    let (snapshot, sends_home) = {
        let hub = crate::hub::lock();
        (hub.active().map(|v| v.mission_snapshot()["mission"].clone()), hub.active().map(crate::hub::Vehicle::sends_home))
    };
    let mission = snapshot.unwrap_or(Value::Null);
    let idle = mission.get("inProgress").and_then(Value::as_bool) == Some(false);
    let mut state = held();
    if state.fetching && idle {
        state.fetching = false;
        let downloaded: Vec<Downloaded> = mission["items"]
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
        let template = state.document.clone().unwrap_or_else(empty_document);
        state.document = Some(plandoc::from_vehicle(&downloaded, sends_home.unwrap_or(false), &template));
    }
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
        items: Vec::new(),
        fence: json!({ "circles": [], "polygons": [], "version": 2 }),
        rally: json!({ "points": [], "version": 2 }),
    }
}
