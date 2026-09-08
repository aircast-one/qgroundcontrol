use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "mavlinkInspector.systems.0.messages"];
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
    let model = object(&backend.get_fields("mavlinkInspector.systems.0.messages", FIELDS));
    let messages: Vec<Value> = model
        .get("elements")
        .and_then(Value::as_array)
        .map(|elements| {
            elements
                .iter()
                .enumerate()
                .filter_map(|(index, m)| {
                    let name = m.get("name").and_then(Value::as_str).filter(|n| !n.is_empty())?;
                    let rate = m.get("actualRateHz").and_then(Value::as_f64).unwrap_or(0.0);
                    let target = m.get("targetRateHz").and_then(Value::as_i64).unwrap_or(RATE_DEFAULT);
                    Some(json!({
                        "index": index,
                        "path": format!("mavlinkInspector.systems.0.messages.{index}"),
                        "id": m.get("id").and_then(Value::as_i64).unwrap_or(0),
                        "compId": m.get("compId").and_then(Value::as_i64).unwrap_or(0),
                        "name": name,
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
        "messages": messages,
        "rateChoices": RATE_CHOICES.iter().map(|r| json!({ "rate": r, "title": rate_title(*r) })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
                ] }).to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = inspector_view(&Fake, &[]);
        assert_eq!(view["available"], true);
        assert_eq!(view["messages"].as_array().unwrap().len(), 1);
        assert_eq!(view["messages"][0]["rateText"], "1.0 Hz");
        assert_eq!(view["messages"][0]["path"], "mavlinkInspector.systems.0.messages.0");
        assert_eq!(view["rateChoices"].as_array().unwrap().len(), 15);
    }
}
