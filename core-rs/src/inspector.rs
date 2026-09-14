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
    json!({
        "kind": "object",
        "class": "MavlinkInspector",
        "available": model.get("kind").and_then(Value::as_str) == Some("object"),
        "systemId": system,
        "messages": messages,
        "rateChoices": RATE_CHOICES.iter().map(|r| json!({ "rate": r, "title": rate_title(*r) })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
