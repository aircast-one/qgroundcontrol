use serde_json::{Value, json};

use crate::read::{flag, integer, object};
use crate::router::Backend;

const KINDS: &[&str] = &["message", "vehicleError", "navigation"];

fn listed_ids(backend: &dyn Backend) -> Vec<i64> {
    object(&backend.get_fields("host", "notices")).get("notices").and_then(Value::as_array).map(|n| n.iter().filter_map(|notice| notice.get("id")?.as_i64()).collect()).unwrap_or_default()
}

fn invoked(backend: &dyn Backend, path: &str, args: Value) -> Value {
    let answered = object(&backend.invoke(path, &args.to_string()));
    answered.get("result").cloned().filter(|_| flag(&answered, "ok")).unwrap_or(Value::Null)
}

pub fn acknowledge(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let Some(id) = serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0)?.as_i64()) else {
        return json!({ "ok": false, "result": false, "refusal": "malformed", "reason": "A notice is acknowledged by its id." });
    };
    if !listed_ids(backend).contains(&id) {
        return json!({ "ok": false, "result": false, "refusal": "noSuchNotice", "reason": format!("There is no notice {id} waiting.") });
    }
    let removed = invoked(backend, path, json!([id])) == json!(true);
    json!({ "ok": removed, "result": removed, "refusal": Value::Null, "reason": match removed { true => Value::Null, false => json!("The notice is still waiting.") } })
}

pub fn acknowledge_through(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let Some(id) = serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0)?.as_i64()) else {
        return json!({ "ok": false, "result": 0, "refusal": "malformed", "reason": "Notices are acknowledged up to an id." });
    };
    let removed = invoked(backend, path, json!([id])).as_i64().unwrap_or(0);
    json!({ "ok": true, "result": removed, "refusal": Value::Null, "reason": Value::Null })
}

pub fn post(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let text = |i: usize| given.get(i).and_then(Value::as_str).unwrap_or("").to_string();
    let (kind, title, body) = (text(0), text(1), text(2));
    let refusal = match () {
        _ if !KINDS.contains(&kind.as_str()) => Some(("unknownKind", format!("A notice is a {}.", KINDS.join(", a ")))),
        _ if title.trim().is_empty() && body.trim().is_empty() => Some(("empty", "A notice needs a title or some text.".to_string())),
        _ => None,
    };
    if let Some((token, reason)) = refusal {
        return json!({ "ok": false, "result": false, "refusal": token, "reason": reason });
    }
    let posted = invoked(backend, path, json!([kind, title, body])) == json!(true);
    json!({ "ok": posted, "result": posted, "refusal": Value::Null, "reason": match posted { true => Value::Null, false => json!("The notice was not posted.") } })
}

pub fn clear_messages(backend: &dyn Backend, path: &str) -> Value {
    let before = object(&backend.get_fields("vehicle", "messageCount"));
    if before.get("kind").and_then(Value::as_str) != Some("object") {
        return json!({ "ok": false, "refusal": "noVehicle", "reason": "No vehicle is connected." });
    }
    let dispatched = flag(&object(&backend.invoke(path, "[]")), "ok");
    let after = integer(&object(&backend.get_fields("vehicle", "messageCount")), "messageCount");
    json!({
        "ok": dispatched && after == Some(0),
        "refusal": Value::Null,
        "cleared": integer(&before, "messageCount").zip(after).map(|(b, a)| (b - a).max(0)),
        "reason": match dispatched && after == Some(0) { true => Value::Null, false => json!("The vehicle's messages were not cleared.") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Host(RefCell<Vec<i64>>, RefCell<i64>);
    impl Backend for Host {
        fn get(&self, _p: &str) -> String { String::new() }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "host" => json!({ "kind": "object", "notices": self.0.borrow().iter().map(|id| json!({ "id": id })).collect::<Vec<_>>() }),
                _ => json!({ "kind": "object", "messageCount": *self.1.borrow() }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, p: &str, a: &str) -> String {
            let id = object(a)[0].as_i64().unwrap_or(0);
            let result = match p {
                "host.acknowledge" => {
                    let mut notices = self.0.borrow_mut();
                    let at = notices.iter().position(|n| *n == id);
                    json!(at.map(|at| notices.remove(at)).is_some())
                }
                "host.acknowledgeThrough" => {
                    let before = self.0.borrow().len();
                    self.0.borrow_mut().retain(|n| *n > id);
                    json!(before - self.0.borrow().len())
                }
                "vehicle.clearMessages" => {
                    *self.1.borrow_mut() = 0;
                    Value::Null
                }
                _ => json!(true),
            };
            json!({ "ok": true, "result": result }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_notice_call_that_would_answer_a_bare_false_says_why() {
        let host = Host(RefCell::new(vec![4, 5, 9]), RefCell::new(12));
        assert_eq!(acknowledge(&host, "host.acknowledge", "[7]")["refusal"], "noSuchNotice", "acknowledge answers false for an id it does not hold, which a head could not tell from a failure to remove one it does");
        assert_eq!(acknowledge(&host, "host.acknowledge", "[5]")["ok"], true);
        assert_eq!(acknowledge_through(&host, "host.acknowledgeThrough", "[8]")["result"], 1);
        assert_eq!(post(&host, "host.postNotice", r#"["banner", "t", "x"]"#)["refusal"], "unknownKind", "postNotice answers false for a kind it does not know");
        assert_eq!(post(&host, "host.postNotice", r#"["message", " ", ""]"#)["refusal"], "empty");
        assert_eq!(post(&host, "host.postNotice", r#"["message", "Battery", "Low"]"#)["result"], true, "the macOS head reads result, so the claimed path keeps it");
        let cleared = clear_messages(&host, "vehicle.clearMessages");
        assert_eq!((&cleared["ok"], &cleared["cleared"]), (&json!(true), &json!(12)));
    }
}
