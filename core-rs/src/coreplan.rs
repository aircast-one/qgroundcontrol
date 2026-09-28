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
const ACTIONS: &[&str] = &[OPEN, SAVE, SEND, FETCH, STATUS, CORE_INSERT_WAYPOINT, CORE_REMOVE, CORE_INSERT_LAND, CORE_SET_COMMAND, CORE_SET_ALTITUDE, CORE_INSERT_TAKEOFF, CORE_SET_ALTITUDE_MODE, CORE_ITEMS];
const DEFAULT_ALTITUDE: &str = "settings.appSettings.defaultMissionItemAltitude";

#[derive(Default)]
struct Held {
    document: Option<Document>,
    fetching: bool,
    selected: i64,
    file: Option<String>,
}

pub const CHANGED: &str = "core.plan@changed";

pub static ON_CHANGE: std::sync::Mutex<Option<std::sync::Arc<dyn Fn() + Send + Sync>>> = std::sync::Mutex::new(None);

static ENABLED: LazyLock<bool> = LazyLock::new(|| std::env::var("QGC_CORE_PLAN").is_ok_and(|v| v == "1"));

pub fn enabled() -> bool {
    *ENABLED
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
        match change(current) {
            Ok(changed) => {
                let count = changed.items.len();
                state.document = Some(changed);
                (json!({ "ok": true, "items": count }), before)
            }
            Err(reason) => return refused(reason),
        }
    };
    settle_home_on_terrain(before);
    changed();
    answer
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
                state.document = Some(document);
                state.selected = 0;
                state.file = Some(file.to_string());
            }
            settle_home_on_terrain(None);
            changed();
            json!({ "ok": true, "items": count, "result": true })
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
        state.selected = 0;
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
        settings_sections: Vec::new(),
        items: Vec::new(),
        fence: json!({ "circles": [], "polygons": [], "version": 2 }),
        rally: json!({ "points": [], "version": 2 }),
    }
}

pub fn view(backend: &dyn Backend) -> Value {
    let (document, selected) = {
        let state = held();
        (state.document.clone().unwrap_or_else(empty_document), state.selected)
    };
    let rover = crate::read::flag(&crate::read::object(&backend.get_fields("plan.controllerVehicle", "rover")), "rover");
    crate::missionitems::document_view(&document, selected, &crate::read::Unit::vertical(backend), &crate::read::Unit::speed(backend), crate::missionsummary::imperial(backend), rover)
        .unwrap_or_else(|reason| json!({ "kind": "object", "class": "MissionItems", "available": false, "items": [], "selected": -1, "reason": reason }))
}

fn visual_index_of_sequence(document: &Document, sequence: i64) -> Option<i64> {
    let starts = std::iter::once((0usize, document.settings_sections.len())).chain(document.items.iter().scan(document.settings_sections.len() + 1, |next, item| {
        let start = *next;
        let span = match item {
            plandoc::Item::Simple(s) => 1 + s.sections.len(),
            plandoc::Item::Complex { item_count, .. } => *item_count,
        };
        *next += span;
        Some((start, start + span - 1))
    }));
    starts.enumerate().find(|(_, (first, last))| (*first as i64..=*last as i64).contains(&sequence)).map(|(i, _)| i as i64)
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

fn clear() -> Value {
    {
        let mut state = held();
        let template = state.document.clone().unwrap_or_else(empty_document);
        state.document = Some(Document { home: None, items: Vec::new(), settings_sections: Vec::new(), ..template });
        state.selected = 0;
    }
    changed();
    json!({ "ok": true })
}

fn insert_kind(backend: &dyn Backend, args: &str) -> Value {
    let given: Value = serde_json::from_str(args).unwrap_or(Value::Null);
    let kind = given.get(0).and_then(Value::as_str).unwrap_or("");
    let rest = json!([given.get(1), given.get(2), given.get(3)]).to_string();
    let answered = match kind {
        "waypoint" => insert_at(backend, &rest, false),
        "land" => insert_at(backend, &rest, true),
        "takeoff" => insert_takeoff(backend, &json!([given.get(3)]).to_string()),
        other => return refused(format!("The core plan cannot insert a {other} yet.")),
    };
    match answered.get("ok").and_then(Value::as_bool) {
        Some(true) => json!({ "ok": true, "inserted": kind }),
        _ => answered,
    }
}

fn item_write(backend: &dyn Backend, path: &str, value: &str) -> Option<Value> {
    let rest = path.strip_prefix("plan.missionController.visualItems.")?;
    let (index, property) = rest.split_once('.')?;
    let index: u64 = index.parse().ok()?;
    let number = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").and_then(Value::as_f64));
    let command = match property {
        "altitude" => false,
        "command" => true,
        _ => return None,
    };
    Some(match number {
        Some(n) => item_edit(backend, &json!([index, n]).to_string(), command),
        None => refused("That field takes a number."),
    })
}

pub fn route_invoke(backend: &dyn Backend, path: &str, args: &str) -> Option<Value> {
    let current = || held().file.clone();
    Some(match path {
        "plan.loadFromFile" => first_text(args).map_or_else(|| refused("Open needs the path of a .plan file."), |file| open(&file)),
        "plan.saveToFile" => first_text(args).map_or_else(|| refused("Save needs a path to write the plan to."), |file| save(&file)),
        "plan.saveToCurrent" => current().map_or_else(|| refused("This plan has not been saved to a file yet."), |file| save(&file)),
        "plan.sendToVehicle" => send(),
        "plan.loadFromVehicle" => fetch(),
        "plan.removeAll" => clear(),
        "mission.insert" => insert_kind(backend, args),
        "mission.remove" | "plan.missionController.removeVisualItem" => remove(args),
        "plan.missionController.setCurrentPlanViewSeqNum" => select(args),
        _ => return None,
    })
}

pub fn route_set(backend: &dyn Backend, path: &str, value: &str) -> Option<Value> {
    item_write(backend, path, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sequence_number_selects_the_row_that_holds_it_as_qt_selects_it() {
        let doc = plandoc::load(include_str!("../../test/MissionManager/SectionTest.plan")).unwrap();
        let rows: Vec<Option<i64>> = (0..8).map(|seq| visual_index_of_sequence(&doc, seq)).collect();
        assert_eq!(rows, vec![Some(0), Some(1), Some(2), Some(3), Some(3), Some(4), None, None], "sequence 4 is the mount control folded into row 3, so selecting it selects row 3");
    }
}
