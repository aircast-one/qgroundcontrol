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
    dirty: bool,
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
                state.document = Some(changed);
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
                state.dirty = false;
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
        Ok(()) => {
            {
                let mut state = held();
                state.file = Some(file.to_string());
                state.dirty = false;
            }
            changed();
            json!({ "ok": true, "result": true })
        }
        Err(e) => refused(format!("Could not write {file}: {e}")),
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
            held().dirty = false;
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

fn adopt(snapshot: &Value, sends_home: bool, fence_read: bool, rally_read: bool) {
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
    let template = state.document.clone().unwrap_or_else(empty_document);
    let mission = plandoc::from_vehicle(&downloaded, sends_home, &template);
    state.document = Some(Document {
        fence: if fence_read { fence_from(&snapshot["fence"]) } else { mission.fence.clone() },
        rally: if rally_read { json!({ "version": 2, "points": snapshot["rally"]["points"] }) } else { mission.rally.clone() },
        ..mission
    });
    state.selected = 0;
    state.dirty = false;
    state.fetching = false;
}

fn fetch() -> Value {
    let Some((fence, rally, sends_home)) = crate::hub::lock().active().map(|v| {
        let (fence, rally) = v.plans_supported();
        (fence, rally, v.sends_home())
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
            true => adopt(&snapshot, sends_home, fence_read, rally_read),
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

fn plan_for_offline_vehicle(backend: &dyn Backend) {
    if !crate::read::flag(&crate::read::object(&backend.get_fields("plan", "offline")), "offline") {
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

fn clear() -> Value {
    {
        let mut state = held();
        let template = state.document.clone().unwrap_or_else(empty_document);
        state.document = Some(Document { home: None, items: Vec::new(), settings_sections: Vec::new(), ..template });
        state.selected = 0;
        state.dirty = false;
        state.file = None;
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
    let index: usize = index.parse().ok()?;
    let given = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").cloned());
    let number = given.as_ref().and_then(Value::as_f64);
    let answer = |edited: Result<Document, String>| match edited {
        Ok(changed) => edit(|_| Ok(changed)),
        Err(reason) => refused(reason),
    };
    let current = held().document.clone()?;
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
            None => refused("A breach return point needs a latitude and a longitude."),
        },
        "plan.geoFenceController.breachReturnAltitude" => fence_edit(|f, r| Some((crate::fencedoc::set_breach_altitude(f, given.as_f64()?)?, r.clone())), "Set a breach return point before its altitude."),
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
    Fence(usize),
}

fn shape_of(path: &str) -> Option<Shape> {
    if let Some(index) = path.strip_prefix("plan.missionController.visualItems.").and_then(|r| r.strip_suffix(".surveyAreaPolygon")) {
        return index.parse().ok().map(Shape::Survey);
    }
    path.strip_prefix("plan.geoFenceController.polygons.").and_then(|r| r.parse().ok()).map(Shape::Fence)
}

fn vertex_edit(vertices: &[Value], member: &str, given: &Value) -> Option<Vec<Value>> {
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
            (i < vertices.len() && vertices.len() > 3).then(|| vertices.iter().enumerate().filter(|(k, _)| *k != i).map(|(_, v)| v.clone()).collect())
        }
        "splitPolygonSegment" => {
            let i = index()?;
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
    let refusal = "That vertex edit does not fit this shape.";
    Some(match shape {
        Shape::Fence(index) => fence_edit(
            |f, r| {
                let polygons = f.get("polygons").and_then(Value::as_array)?;
                let polygon = polygons.get(index)?;
                let edited = with_polygon(polygon, "polygon", vertex_edit(polygon.get("polygon")?.as_array()?, member, &given)?);
                let replaced: Vec<Value> = polygons.iter().enumerate().map(|(k, p)| if k == index { edited.clone() } else { p.clone() }).collect();
                Some((with_polygon(f, "polygons", replaced), r.clone()))
            },
            refusal,
        ),
        Shape::Survey(index) => edit(|doc| {
            let at = index.checked_sub(1).filter(|i| *i < doc.items.len()).ok_or(refusal)?;
            let plandoc::Item::Complex { kind, json, .. } = &doc.items[at] else { return Err(refusal.to_string()) };
            let vertices = json.get("polygon").and_then(Value::as_array).cloned().unwrap_or_default();
            let edited = crate::surveydoc::regenerate(&with_polygon(json, "polygon", vertex_edit(&vertices, member, &given).ok_or(refusal)?));
            let item_count = edited["TransectStyleComplexItem"]["Items"].as_array().map_or(0, Vec::len);
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
        Shape::Fence(index) => document.fence.get("polygons")?.get(index)?.get("polygon")?.as_array()?.clone(),
    };
    Some(vertices.iter().filter_map(|v| Some((v.get(0)?.as_f64()?, v.get(1)?.as_f64()?))).collect())
}

pub fn drawing() -> bool {
    held().document.as_ref().is_some_and(|d| {
        d.items.iter().any(|item| matches!(item, plandoc::Item::Complex { json, .. } if json.get("polygon").and_then(Value::as_array).is_some_and(|p| p.len() < 3)))
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
            opened
        }),
        "plan.saveToFile" => first_text(args).map_or_else(|| refused("Save needs a path to write the plan to."), |file| save(&file)),
        "plan.saveToCurrent" => current().map_or_else(|| refused("This plan has not been saved to a file yet."), |file| save(&file)),
        "plan.sendToVehicle" => send(),
        "plan.loadFromVehicle" => fetch(),
        "plan.removeAll" => clear(),
        "mission.insert" => insert_kind(backend, args),
        "mission.remove" | "plan.missionController.removeVisualItem" => remove(args),
        "plan.missionController.setCurrentPlanViewSeqNum" => select(args),
        "plan.undo" | "plan.redo" => json!({ "ok": false, "refusal": "notTracking", "reason": "The core plan does not keep an undo history yet." }),
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
    fn the_editor_fields_of_each_command_are_the_ones_qt_builds() {
        let doc = plandoc::load(include_str!("../tests/fixtures/commands.plan")).unwrap();
        let qt: Vec<Value> = serde_json::from_str(include_str!("../tests/fixtures/itemfacts-commands-by-qt.json")).unwrap();
        qt.iter().enumerate().for_each(|(index, expected)| {
            let mine = by_value(document_facts(&doc, index, 5.0, 15.0));
            let expected = by_value(expected.clone());
            let differing: Vec<String> = expected.as_object().unwrap().iter().filter(|(k, v)| mine.get(k.as_str()) != Some(v)).map(|(k, v)| format!("{k}:\n  core {}\n  qt   {v}", mine.get(k.as_str()).unwrap_or(&Value::Null))).collect();
            assert!(differing.is_empty(), "item {index}: {}", differing.join("\n"));
        });
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
    document_facts(&document, index, speed("offlineEditingHoverSpeed", 5.0), speed("offlineEditingCruiseSpeed", 15.0))
}

fn document_facts(document: &Document, index: usize, hover: f64, cruise: f64) -> Value {
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
