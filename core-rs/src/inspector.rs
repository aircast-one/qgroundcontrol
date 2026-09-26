use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

// activeSystem.id is CONSTANT on QGCMAVLinkSystem, so declaring the path would only have polled a
// value that never changes. What changes is WHICH system, and activeSystemChanged is a zero-argument
// notify on the controller, so the @ form binds where the path could not. Without it two systems that
// have both received no messages serialise to the same JSON, the poll sees no change, nothing emits,
// and the view keeps serving the previous system's id beside the new system's empty message list.
pub const DEPS: &[&str] =
    &["vehicles.activeVehicleAvailable", "mavlinkInspector.activeSystem.messages", "mavlinkInspector@activeSystemChanged"];
const FIELDS: &str = "id,compId,name,count,actualRateHz,targetRateHz,selected";
const RATE_DISABLED: i64 = -1;
const RATE_DEFAULT: i64 = 0;
const RATE_CHOICES: &[i64] = &[-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 25, 50, 100];

pub fn rate_text(hz: f64) -> String {
    match hz {
        h if h <= 0.0 => "\u{2014}".to_string(),
        h if h < 0.05 => "<0.1 Hz".to_string(),
        h => format!("{h:.1} Hz"),
    }
}

pub fn rate_title(rate: i64) -> String {
    match rate {
        RATE_DISABLED => "Off".to_string(),
        RATE_DEFAULT => "Default".to_string(),
        r => format!("{r} Hz"),
    }
}

pub fn shown_rate(rate: i64) -> i64 {
    match RATE_CHOICES.contains(&rate) {
        true => rate,
        false => RATE_DEFAULT,
    }
}

pub fn empty_text(available: bool, any: bool) -> &'static str {
    match (available, any) {
        (false, _) => "Connect a vehicle to inspect its MAVLink traffic.",
        (true, false) => "Waiting for this vehicle's first message\u{2026}",
        (true, true) => "",
    }
}

pub fn inspector_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let model = object(&backend.get_fields("mavlinkInspector.activeSystem.messages", FIELDS));
    let system = crate::read::integer(&object(&backend.get("mavlinkInspector.activeSystem.id")), "value");
    let messages: Vec<Value> = model
        .get("elements")
        .and_then(Value::as_array)
        .map(|elements| {
            let name_of = |m: &Value| m.get("name").and_then(Value::as_str).filter(|n| !n.is_empty()).map(str::to_string);
            let repeated = |name: &str| elements.iter().filter(|m| name_of(m).as_deref() == Some(name)).count() > 1;
            elements
                .iter()
                .enumerate()
                .filter_map(|(index, m)| {
                    let name = name_of(m)?;
                    let comp_id = m.get("compId").and_then(Value::as_i64).unwrap_or(0);
                    let title = if repeated(&name) { format!("{name} (comp {comp_id})") } else { name.clone() };
                    let rate = m.get("actualRateHz").and_then(Value::as_f64).unwrap_or(0.0);
                    let target = m.get("targetRateHz").and_then(Value::as_i64).unwrap_or(RATE_DEFAULT);
                    Some(json!({
                        "index": index,
                        "path": format!("mavlinkInspector.activeSystem.messages.{index}"),
                        "id": m.get("id").and_then(Value::as_i64).unwrap_or(0),
                        "compId": comp_id,
                        "name": name,
                        "title": title,
                        "count": m.get("count").and_then(Value::as_i64).unwrap_or(0),
                        "rateHz": rate,
                        "rateText": rate_text(rate),
                        "targetRateHz": target,
                        "targetRateTitle": rate_title(shown_rate(target)),
                        "selected": m.get("selected").and_then(Value::as_bool).unwrap_or(false),
                    }))
                })
                .collect()
        })
        .unwrap_or_default();
    let available = model.get("kind").and_then(Value::as_str) == Some("object");
    let fields = messages.iter().find(|m| m["selected"] == true).map_or_else(Vec::new, |m| selected_fields(backend, m["index"].as_u64().unwrap_or(0)));
    json!({
        "kind": "object",
        "class": "MavlinkInspector",
        "available": available,
        "emptyText": empty_text(available, !messages.is_empty()),
        "systemId": system,
        "messages": messages,
        "fields": fields,
        "rateChoices": RATE_CHOICES.iter().map(|r| json!({ "rate": r, "title": rate_title(*r) })).collect::<Vec<_>>(),
    })
}

// The macOS head read the selected message's field list as a second raw path, built from an index
// it took out of this view a line earlier. Serving the list here keeps the two from ever describing
// different messages, and drops elements without a name as the head did.
fn selected_fields(backend: &dyn Backend, index: u64) -> Vec<Value> {
    object(&backend.get_fields(&format!("mavlinkInspector.activeSystem.messages.{index}.fields"), "name,type,value"))
        .get("elements")
        .and_then(Value::as_array)
        .map(|elements| {
            elements
                .iter()
                .filter_map(|f| {
                    let name = f.get("name").and_then(Value::as_str).filter(|n| !n.is_empty())?;
                    let spelled = |key: &str| f.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
                    Some(json!({ "name": name, "type": spelled("type"), "value": spelled("value") }))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn selected_message(model: &Value) -> Option<&Value> {
    model.get("elements")?.as_array()?.iter().find(|m| m.get("selected").and_then(Value::as_bool) == Some(true))
}

fn rate_refusal(rate: Option<i64>, available: bool, selected: Option<&Value>) -> Option<(&'static str, &'static str)> {
    let component = selected.and_then(|m| m.get("compId")?.as_i64()).unwrap_or(0);
    match () {
        _ if rate.is_none_or(|r| !RATE_CHOICES.contains(&r)) => Some(("unknownRate", "Choose one of the offered rates.")),
        _ if !available => Some(("noVehicle", "No vehicle is being inspected.")),
        _ if selected.is_none() => Some(("noSelection", "Select a message first; the rate applies to the selected one.")),
        _ if component == 0 => Some(("noComponent", "The selected message has no component to ask.")),
        _ => None,
    }
}

pub fn set_message_interval(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let rate = serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0)?.as_i64());
    let model = object(&backend.get_fields("mavlinkInspector.activeSystem.messages", FIELDS));
    let available = model.get("kind").and_then(Value::as_str) == Some("object");
    let selected = selected_message(&model);
    if let Some((token, reason)) = rate_refusal(rate, available, selected) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let dispatched = crate::read::flag(&object(&backend.invoke(path, &json!([rate]).to_string())), "ok");
    json!({
        "ok": dispatched,
        "refusal": Value::Null,
        "rate": rate,
        "rateTitle": rate.map(rate_title),
        "message": selected.and_then(|m| m.get("name")).cloned().unwrap_or(Value::Null),
        "messageId": selected.and_then(|m| m.get("id")).cloned().unwrap_or(Value::Null),
        "reason": match dispatched { true => Value::Null, false => json!("The inspector did not take the rate.") },
    })
}

pub const SELECTED: &str = "mavlinkInspector.activeSystem.selected";

pub fn write_selected(backend: &dyn Backend, value: &str) -> Value {
    let refused = |token: &str, reason: String| json!({ "ok": false, "result": false, "refusal": token, "reason": reason });
    let Some(index) = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value")?.as_i64()) else {
        return refused("malformed", "A message is selected by its position in the list.".to_string());
    };
    let model = object(&backend.get_fields("mavlinkInspector.activeSystem.messages", FIELDS));
    let count = model.get("elements").and_then(Value::as_array).map_or(0, Vec::len) as i64;
    if model.get("kind").and_then(Value::as_str) != Some("object") {
        return refused("noVehicle", "No vehicle is being inspected.".to_string());
    }
    if !(0..count).contains(&index) {
        return refused("noSuchMessage", format!("There is no message at position {index}."));
    }
    let answered = crate::read::flag(&object(&backend.set(SELECTED, &json!({ "value": index }).to_string())), "ok");
    let held = crate::read::integer(&object(&backend.get_fields("mavlinkInspector.activeSystem", "selected")), "selected");
    let took = answered && held == Some(index);
    json!({ "ok": took, "result": took, "refusal": Value::Null, "reason": match took { true => Value::Null, false => json!("The inspector did not select that message.") } })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vehicle_that_has_not_spoken_yet_is_not_a_vehicle_that_is_not_there() {
        assert_eq!(empty_text(false, false), "Connect a vehicle to inspect its MAVLink traffic.");
        assert_eq!(
            empty_text(true, false),
            "Waiting for this vehicle's first message\u{2026}",
            "MAVLinkInspectorController connects messageReceived in its constructor, so activeSystem is an object before any frame has been recorded - the head drew that transient as an empty card under a note promising every message the vehicle is sending. I had argued this state was unreachable because a system exists only because it heartbeat; the macOS session measured it and I was wrong"
        );
        assert_eq!(empty_text(true, true), "", "a table with rows needs no sentence, and one left behind would sit under a list that contradicts it");

        struct Silent;
        impl Backend for Silent {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "mavlinkInspector.activeSystem.messages" => json!({ "kind": "object", "elements": [] }).to_string(),
                    "mavlinkInspector.activeSystem.id" => json!({ "kind": "value", "value": 1 }).to_string(),
                    _ => json!({ "kind": "null" }).to_string(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let listening = inspector_view(&Silent, &[]);
        assert_eq!((listening["available"].clone(), listening["messages"].as_array().map(Vec::len)), (json!(true), Some(0)), "the state this is about: a system is there and the table is empty");
        assert_eq!(listening["emptyText"], "Waiting for this vehicle's first message\u{2026}");
    }

    #[test]
    fn a_dropped_message_does_not_shift_the_indices_after_it() {
        struct System(Value);
        impl Backend for System {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "mavlinkInspector.activeSystem.messages" => self.0.to_string(),
                    _ => json!({ "kind": "null" }).to_string(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        let view = inspector_view(&System(json!({ "kind": "object", "elements": [
            { "name": "HEARTBEAT", "id": 0, "compId": 1, "count": 10, "actualRateHz": 1.0 },
            { "name": "", "id": 1, "compId": 1, "count": 0 },
            { "name": "ATTITUDE", "id": 30, "compId": 1, "count": 99, "actualRateHz": 50.0 },
        ] })), &[]);

        let listed = view["messages"].as_array().unwrap();
        assert_eq!(listed.len(), 2, "the unnamed entry is not shown");
        assert_eq!(listed[0]["name"], "HEARTBEAT");
        assert_eq!(listed[1]["name"], "ATTITUDE");
        assert_eq!(listed[1]["index"], 2, "MavlinkInspector.swift feeds this index back into the raw Qt path mavlinkInspector.activeSystem.messages.N.fields, so it has to be the position in Qt's list: ATTITUDE sits at Qt index 2 even though it is the second row drawn, and enumerate() before filter_map() is the only reason that holds");
        assert_eq!(listed[1]["path"], "mavlinkInspector.activeSystem.messages.2");
    }

    #[test]
    fn rates_read_as_hertz_with_a_floor_and_a_dash() {
        assert_eq!(rate_text(0.0), "\u{2014}");
        assert_eq!(rate_text(0.02), "<0.1 Hz");
        assert_eq!(rate_text(4.04), "4.0 Hz");
        assert_eq!(rate_title(-1), "Off");
        assert_eq!(rate_title(0), "Default");
        assert_eq!(rate_title(25), "25 Hz");
        assert_eq!(shown_rate(7), 7);
        assert_eq!(shown_rate(13), 0);
    }

    #[test]
    fn the_view_lists_messages_with_their_paths() {
        struct Fake;
        impl Backend for Fake {
            fn get(&self, _p: &str) -> String { String::new() }
            fn get_fields(&self, _p: &str, _f: &str) -> String {
                json!({ "kind": "object", "elements": [
                    { "id": 0, "compId": 1, "name": "HEARTBEAT", "count": 12, "actualRateHz": 1.0, "targetRateHz": 0, "selected": true },
                    { "id": 30, "compId": 1, "name": "", "count": 1 },
                    { "id": 262, "compId": 100, "name": "CAMERA_CAPTURE_STATUS", "count": 3 },
                    { "id": 262, "compId": 101, "name": "CAMERA_CAPTURE_STATUS", "count": 2 },
                ] }).to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = inspector_view(&Fake, &[]);
        assert_eq!(view["available"], true);
        assert_eq!(view["messages"].as_array().unwrap().len(), 3);
        assert_eq!(view["messages"][0]["title"], "HEARTBEAT");
        assert_eq!(view["messages"][1]["title"], "CAMERA_CAPTURE_STATUS (comp 100)");
        assert_eq!(view["messages"][2]["path"], "mavlinkInspector.activeSystem.messages.3");
        assert_eq!(view["messages"][0]["rateText"], "1.0 Hz");
        assert_eq!(view["messages"][0]["path"], "mavlinkInspector.activeSystem.messages.0");
        assert_eq!(view["rateChoices"].as_array().unwrap().len(), 15);
    }

    #[test]
    fn the_view_describes_the_system_the_write_will_act_on() {
        assert!(DEPS.iter().any(|dep| dep == &"mavlinkInspector@activeSystemChanged"), "the id is CONSTANT on the system and the system object is what swaps, so the only thing that can fire on a swap is the controller's own signal");
        assert!(DEPS.iter().all(|dep| !dep.contains("activeSystem.id")), "declaring the id path would bind nothing and poll a value that cannot change");
        assert!(DEPS.iter().all(|dep| !dep.contains("systems.0")), "systems.0 is whichever vehicle connected first; setMessageInterval acts on activeSystem, and with two vehicles those are different aircraft");
        assert!(DEPS.iter().any(|dep| dep.contains("activeSystem")));
        assert!(DEPS.iter().any(|dep| dep.contains("messages")), "the messages the view lists come from that same system");
    }

    #[test]
    fn a_rate_names_the_message_it_went_to_and_refuses_when_there_is_none() {
        use std::cell::RefCell;
        struct Inspector(Value, RefCell<Vec<String>>);
        impl Backend for Inspector {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, _p: &str, _f: &str) -> String { self.0.to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, a: &str) -> String {
                self.1.borrow_mut().push(a.to_string());
                json!({ "ok": true }).to_string()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let listing = |selected: bool, comp: i64| json!({ "kind": "object", "elements": [
            { "name": "HEARTBEAT", "id": 0, "compId": 1, "selected": false },
            { "name": "ATTITUDE", "id": 30, "compId": comp, "selected": selected },
        ] });
        let path = "mavlinkInspector.setMessageInterval";

        let chosen = Inspector(listing(true, 1), RefCell::new(Vec::new()));
        let taken = set_message_interval(&chosen, path, "[25]");
        assert_eq!((&taken["ok"], &taken["message"], &taken["rateTitle"]), (&json!(true), &json!("ATTITUDE"), &json!("25 Hz")), "the rate goes to whichever message is selected, which the call itself never names");
        assert_eq!(set_message_interval(&chosen, path, "[13]")["refusal"], "unknownRate", "setMessageRate passes any int to the vehicle, and 13 is not a rate either head offers");
        assert_eq!(set_message_interval(&chosen, path, "[]")["refusal"], "unknownRate");
        assert_eq!(chosen.1.borrow().as_slice(), &["[25]".to_string()]);

        let none = Inspector(listing(false, 1), RefCell::new(Vec::new()));
        assert_eq!(set_message_interval(&none, path, "[5]")["refusal"], "noSelection", "setMessageInterval returns with no selected message and nothing says so");
        assert_eq!(set_message_interval(&Inspector(listing(true, 0), RefCell::new(Vec::new())), path, "[5]")["refusal"], "noComponent");
        assert_eq!(set_message_interval(&Inspector(json!({ "kind": "null" }), RefCell::new(Vec::new())), path, "[5]")["refusal"], "noVehicle");
        assert!(none.1.borrow().is_empty());
    }

    #[test]
    fn a_message_is_selected_only_by_a_position_the_list_has() {
        use std::cell::Cell;
        struct System(Cell<i64>);
        impl Backend for System {
            fn get(&self, _p: &str) -> String { String::new() }
            fn get_fields(&self, p: &str, _f: &str) -> String {
                match p {
                    "mavlinkInspector.activeSystem" => json!({ "kind": "object", "selected": self.0.get() }),
                    _ => json!({ "kind": "object", "elements": [{ "name": "HEARTBEAT" }, { "name": "ATTITUDE" }] }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, v: &str) -> String {
                self.0.set(object(v)["value"].as_i64().unwrap());
                json!({ "ok": true }).to_string()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let system = System(Cell::new(0));
        assert_eq!(write_selected(&system, r#"{"value":1}"#)["result"], true);
        assert_eq!(write_selected(&system, r#"{"value":5}"#)["refusal"], "noSuchMessage", "QGCMAVLinkSystem::setSelected returns in silence for a position past the list");
        assert_eq!(system.0.get(), 1);
    }

    #[test]
    fn the_selected_messages_fields_come_with_the_list_that_selected_it() {
        struct Inspector;
        impl Backend for Inspector {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, p: &str, _f: &str) -> String {
                match p {
                    "mavlinkInspector.activeSystem.messages" => json!({ "kind": "object", "elements": [
                        { "id": 0, "compId": 1, "name": "HEARTBEAT", "selected": false },
                        { "id": 24, "compId": 1, "name": "GPS_RAW_INT", "selected": true },
                    ] }),
                    "mavlinkInspector.activeSystem.messages.1.fields" => json!({ "kind": "object", "elements": [
                        { "name": "fix_type", "type": "uint8_t", "value": "3" },
                        { "name": "", "type": "uint8_t", "value": "9" },
                        { "name": "satellites_visible", "type": "uint8_t" },
                    ] }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = inspector_view(&Inspector, &[]);
        assert_eq!(
            view["fields"],
            json!([{ "name": "fix_type", "type": "uint8_t", "value": "3" }, { "name": "satellites_visible", "type": "uint8_t", "value": "" }]),
            "the fields are those of the message this same answer marks selected, and a nameless element is dropped as the head dropped it"
        );
    }
}
