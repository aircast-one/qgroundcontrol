use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

const LINKS: &str = "links.linkConfigurations";
const SUPPORT_HOST: &str = "settings.mavlinkSettings.forwardMavlinkAPMSupportHostName.rawValue";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Forwarding {
    Start,
    End,
}

fn configurations(backend: &dyn Backend) -> Option<Vec<Value>> {
    object(&backend.get(LINKS)).get("elements")?.as_array().cloned()
}

fn connected(element: &Value) -> bool {
    element.get("children").and_then(Value::as_array).is_some_and(|c| c.iter().any(|v| v.as_str() == Some("link")))
}

fn connect_refusal(index: Option<usize>, links: Option<&[Value]>) -> Option<(&'static str, String)> {
    let Some(index) = index else {
        return Some(("malformed", format!("Name the link as @{LINKS}.<index>.")));
    };
    let Some(links) = links else {
        return Some(("unavailable", "The link list is not available.".to_string()));
    };
    match links.get(index) {
        None => Some(("noSuchLink", format!("There is no link at position {index}."))),
        Some(_) => None,
    }
}

pub fn connect(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let index = given.get(0).and_then(Value::as_str).and_then(|r| r.strip_prefix('@')?.strip_prefix(LINKS)?.strip_prefix('.')?.parse::<usize>().ok());
    let links = configurations(backend);
    if let Some((token, reason)) = connect_refusal(index, links.as_deref()) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let index = index.unwrap_or(0);
    let element = links.as_deref().and_then(|l| l.get(index)).cloned().unwrap_or(Value::Null);
    let name = element.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    if connected(&element) {
        return json!({ "ok": true, "refusal": Value::Null, "name": name, "connected": true, "alreadyConnected": true, "reason": Value::Null });
    }
    let dispatched = flag(&object(&backend.invoke(path, &json!([format!("@{LINKS}.{index}")]).to_string())), "ok");
    let now = configurations(backend).and_then(|l| l.iter().find(|e| e.get("name").and_then(Value::as_str) == Some(name.as_str())).cloned());
    json!({
        "ok": dispatched,
        "refusal": Value::Null,
        "name": name,
        "connected": now.as_ref().is_some_and(connected),
        "alreadyConnected": false,
        "lastError": now.as_ref().and_then(|e| e.get("lastError")).and_then(Value::as_str).filter(|e| !e.is_empty()),
        "reason": match dispatched { true => Value::Null, false => json!("The link manager did not take the request.") },
    })
}

fn forwarding_refusal(action: Forwarding, forwarding: bool, host: &str) -> Option<(&'static str, &'static str)> {
    match action {
        Forwarding::Start if forwarding => Some(("alreadyForwarding", "Telemetry is already being forwarded to support.")),
        Forwarding::Start if host.trim().is_empty() => Some(("noHost", "Set the support server's address first.")),
        Forwarding::Start => None,
        Forwarding::End if !forwarding => Some(("idle", "Telemetry is not being forwarded to support.")),
        Forwarding::End => None,
    }
}

pub fn support_forwarding(backend: &dyn Backend, action: Forwarding, path: &str) -> Value {
    let forwarding = flag(&object(&backend.get_fields("links", "mavlinkSupportForwardingEnabled")), "mavlinkSupportForwardingEnabled");
    let host = object(&backend.get(SUPPORT_HOST)).get("value").and_then(Value::as_str).unwrap_or("").to_string();
    if let Some((token, reason)) = forwarding_refusal(action, forwarding, &host) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let dispatched = flag(&object(&backend.invoke(path, "[]")), "ok");
    json!({
        "ok": dispatched,
        "refusal": Value::Null,
        "host": (action == Forwarding::Start).then_some(host),
        "reason": match dispatched { true => Value::Null, false => json!("The link manager did not take the request.") },
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Create {
    AndConnect,
    Serial,
}

fn names(links: Option<&[Value]>) -> Vec<String> {
    links.unwrap_or_default().iter().filter_map(|e| e.get("name")?.as_str().map(str::to_string)).collect()
}

fn create_refusal(name: &str, taken: &[String], form: Option<(&'static str, &'static str)>) -> Option<(&'static str, String)> {
    match () {
        _ if name.trim().is_empty() => Some(("name", "A link needs a name.".to_string())),
        _ if taken.iter().any(|t| t == name) => Some(("name", format!("A link called {name} already exists."))),
        _ => form.map(|(field, sentence)| (field, sentence.to_string())),
    }
}

pub fn create(backend: &dyn Backend, how: Create, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let text = |i: usize| given.get(i).and_then(Value::as_str).unwrap_or("").to_string();
    let whole = |i: usize| given.get(i).and_then(Value::as_f64).filter(|v| v.fract() == 0.0).map(|v| v as i64);
    let (kind, name, host, port, forwarded) = match how {
        Create::AndConnect => (text(0).to_lowercase(), text(1), text(2), whole(3), json!([text(0), text(1), text(2), whole(3)])),
        Create::Serial => ("serial".to_string(), text(0), text(1), whole(2), json!([text(0), text(1), whole(2)])),
    };
    let rates = crate::links::serial_baud_rates(backend);
    let form = crate::links::form_error(&kind, &host, crate::links::port_ok(&kind, port, &rates), &crate::links::link_type_ids(backend));
    if let Some((field, reason)) = create_refusal(&name, &names(configurations(backend).as_deref()), form) {
        return json!({ "ok": false, "result": false, "refusal": "invalid", "errorField": field, "reason": reason });
    }
    let answered = object(&backend.invoke(path, &forwarded.to_string()));
    let made = flag(&answered, "ok") && flag(&answered, "result");
    json!({
        "ok": made,
        "result": made,
        "refusal": Value::Null,
        "errorField": Value::Null,
        "name": name,
        "reason": match made { true => Value::Null, false => json!("The link manager did not create the link.") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_forwarding_starts_once_to_somewhere_and_ends_only_what_runs() {
        assert_eq!(forwarding_refusal(Forwarding::Start, false, "support.ardupilot.org:30001"), None);
        assert_eq!(
            forwarding_refusal(Forwarding::Start, true, "support.ardupilot.org:30001").map(|r| r.0),
            Some("alreadyForwarding"),
            "createMavlinkForwardingSupportLink adds a dynamic configuration every time it is called, so a second start makes a second link under the same name"
        );
        assert_eq!(forwarding_refusal(Forwarding::Start, false, "  ").map(|r| r.0), Some("noHost"), "an empty host makes a forwarding link that sends to nobody and reads as forwarding");
        assert_eq!(forwarding_refusal(Forwarding::End, false, "").map(|r| r.0), Some("idle"));
        assert_eq!(forwarding_refusal(Forwarding::End, true, ""), None);
    }

    #[test]
    fn a_link_is_connected_at_the_position_named_and_an_open_one_is_left_alone() {
        use std::cell::RefCell;
        struct Manager(RefCell<Vec<Value>>, RefCell<usize>);
        impl Backend for Manager {
            fn get(&self, _p: &str) -> String { json!({ "kind": "object", "elements": self.0.borrow().clone() }).to_string() }
            fn get_fields(&self, _p: &str, _f: &str) -> String { String::new() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String {
                *self.1.borrow_mut() += 1;
                self.0.borrow_mut()[1]["children"] = json!(["link"]);
                json!({ "ok": true }).to_string()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let manager = Manager(RefCell::new(vec![json!({ "name": "SITL", "children": ["link"] }), json!({ "name": "Radio", "children": [] })]), RefCell::new(0));
        let path = "links.createConnectedLink";
        let open = connect(&manager, path, r#"["@links.linkConfigurations.0"]"#);
        assert_eq!((&open["ok"], &open["alreadyConnected"]), (&json!(true), &json!(true)), "an open link is answered without a call, which createConnectedLink would have logged and returned from anyway");
        let radio = connect(&manager, path, r#"["@links.linkConfigurations.1"]"#);
        assert_eq!((&radio["name"], &radio["connected"]), (&json!("Radio"), &json!(true)));
        assert_eq!(*manager.1.borrow(), 1);
        assert_eq!(connect(&manager, path, r#"["@links.linkConfigurations.7"]"#)["refusal"], "noSuchLink", "a stale position reaches Qt as a null config");
        assert_eq!(connect(&manager, path, "[]")["refusal"], "malformed");
    }

    #[test]
    fn a_new_link_is_refused_with_the_field_to_fix_before_the_manager_says_false() {
        let taken = vec!["SITL".to_string()];
        assert_eq!(create_refusal("Radio", &taken, None), None);
        assert_eq!(create_refusal("SITL", &taken, None).map(|r| r.0), Some("name"), "createAndConnectLink and createSerialConfiguration answer a name in use with false and a log line, so the form could not say which field to change");
        assert_eq!(create_refusal("  ", &taken, None).map(|r| r.0), Some("name"));
        assert_eq!(create_refusal("Radio", &taken, Some(("port", "Port must be a number between 1 and 65535."))).map(|r| r.0), Some("port"));

        use std::cell::RefCell;
        struct Manager(RefCell<Vec<String>>);
        impl Backend for Manager {
            fn get(&self, _p: &str) -> String { json!({ "kind": "object", "elements": [{ "name": "SITL" }] }).to_string() }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "object", "linkTypeIds": ["udp", "tcp", "serial"], "serialBaudRates": ["57600", "115200"] }).to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, a: &str) -> String {
                self.0.borrow_mut().push(a.to_string());
                json!({ "ok": true, "result": true }).to_string()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let manager = Manager(RefCell::new(Vec::new()));
        let tcp = create(&manager, Create::AndConnect, "links.createAndConnectLink", r#"["TCP", "Bench", "", 5760]"#);
        assert_eq!((&tcp["refusal"], &tcp["errorField"]), (&json!("invalid"), &json!("host")), "a TCP link with no address is the form's own rule, answered before the manager is asked");
        assert_eq!(create(&manager, Create::AndConnect, "links.createAndConnectLink", r#"["UDP", "Bench", "", 70000]"#)["errorField"], "port");
        assert_eq!(create(&manager, Create::Serial, "links.createSerialConfiguration", r#"["Radio", "ttyUSB0", 9600]"#)["errorField"], "port", "a rate the radio does not offer");
        assert!(manager.0.borrow().is_empty());
        let made = create(&manager, Create::Serial, "links.createSerialConfiguration", r#"["Radio", "ttyUSB0", 57600]"#);
        assert_eq!((&made["ok"], &made["result"]), (&json!(true), &json!(true)), "the head reads result, so the claimed path keeps it");
        assert_eq!(manager.0.borrow().as_slice(), &[r#"["Radio","ttyUSB0",57600]"#.to_string()]);
    }
}
