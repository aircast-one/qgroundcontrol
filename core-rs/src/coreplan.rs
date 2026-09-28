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
const ACTIONS: &[&str] = &[OPEN, SAVE, SEND, FETCH, STATUS];

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

pub fn act(_backend: &dyn Backend, path: &str, args: &str) -> Value {
    match path {
        OPEN => first_text(args).map_or_else(|| refused("Open needs the path of a .plan file."), |file| open(&file)),
        SAVE => first_text(args).map_or_else(|| refused("Save needs a path to write the plan to."), |file| save(&file)),
        SEND => send(),
        FETCH => fetch(),
        STATUS => status(),
        _ => refused(format!("{path} is not a plan action the core performs")),
    }
}

fn held() -> std::sync::MutexGuard<'static, Held> {
    HELD.lock().unwrap_or_else(PoisonError::into_inner)
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
        home: [0.0, 0.0, 0.0],
        items: Vec::new(),
        fence: json!({ "circles": [], "polygons": [], "version": 2 }),
        rally: json!({ "points": [], "version": 2 }),
    }
}
