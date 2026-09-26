use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

const CONFIGURATIONS: &str = "links.linkConfigurations";

fn configured_names(backend: &dyn Backend) -> Option<Vec<String>> {
    let model = object(&backend.get(CONFIGURATIONS));
    let elements = model.get("elements")?.as_array()?;
    Some(elements.iter().map(|e| e.get("name").and_then(Value::as_str).unwrap_or("").to_string()).collect())
}

pub(crate) fn armed_vehicle_links(backend: &dyn Backend) -> Vec<String> {
    let vehicle = object(&backend.get_fields("vehicle", "armed"));
    match crate::read::flag(&vehicle, "armed") {
        true => object(&backend.get_fields("vehicle.vehicleLinkManager", "linkNames")).get("linkNames").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default(),
        false => Vec::new(),
    }
}

fn removal_refusal(index: Option<usize>, expected: Option<&str>, names: Option<&[String]>, armed_on: &[String]) -> Option<(&'static str, String)> {
    let Some(index) = index else {
        return Some(("malformed", format!("Name the link as @{CONFIGURATIONS}.<index>.")));
    };
    let Some(names) = names else {
        return Some(("unavailable", "The link list is not available.".to_string()));
    };
    let Some(name) = names.get(index) else {
        return Some(("noSuchLink", format!("There is no link at position {index}.")));
    };
    match () {
        _ if expected.is_some_and(|e| e != name) => Some(("moved", format!("The list changed: position {index} is now {name}."))),
        _ if armed_on.iter().any(|n| n == name) => Some(("carriesArmedVehicle", format!("{name} is carrying an armed vehicle. Disarm before removing it."))),
        _ => None,
    }
}

pub fn remove_configuration(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let parsed = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let index = parsed.get(0).and_then(Value::as_str).and_then(|r| r.strip_prefix('@')?.strip_prefix(CONFIGURATIONS)?.strip_prefix('.')?.parse::<usize>().ok());
    let expected = parsed.get(1).and_then(Value::as_str);
    let names = configured_names(backend);
    let armed_on = armed_vehicle_links(backend);
    if let Some((token, reason)) = removal_refusal(index, expected, names.as_deref(), &armed_on) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let name = names.as_deref().and_then(|n| n.get(index?)).cloned().unwrap_or_default();
    let dispatched = crate::read::flag(&object(&backend.invoke(path, &json!([format!("@{CONFIGURATIONS}.{}", index.unwrap_or(0))]).to_string())), "ok");
    let gone = configured_names(backend).is_some_and(|after| !after.contains(&name));
    json!({
        "ok": dispatched && gone,
        "refusal": Value::Null,
        "removed": name,
        "reason": match dispatched && gone { true => Value::Null, false => json!("The link is still listed.") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_is_removed_only_at_the_position_named_and_never_from_under_an_armed_vehicle() {
        let names = vec!["SITL".to_string(), "Radio".to_string()];
        let token = |index, expected, armed: &[String]| removal_refusal(index, expected, Some(&names), armed).map(|(t, _)| t);
        assert_eq!(token(Some(1), None, &[]), None);
        assert_eq!(token(Some(1), Some("Radio"), &[]), None);
        assert_eq!(token(Some(1), Some("SITL"), &[]), Some("moved"), "the reference is a list position, and a link added or removed between the read and the tap moves a different configuration under it");
        assert_eq!(token(Some(2), None, &[]), Some("noSuchLink"), "removeConfiguration receives a null config for a stale index and says 'Internal error' to the log only");
        assert_eq!(token(None, None, &[]), Some("malformed"));
        assert_eq!(token(Some(0), None, &["SITL".to_string()]), Some("carriesArmedVehicle"), "removing a configuration disconnects its link first, so an armed vehicle would lose its ground station in flight");
        assert_eq!(removal_refusal(Some(0), None, None, &[]).map(|(t, _)| t), Some("unavailable"));

        use std::cell::RefCell;
        struct Manager(RefCell<Vec<&'static str>>, bool, RefCell<Vec<String>>);
        impl Backend for Manager {
            fn get(&self, _p: &str) -> String { json!({ "kind": "object", "elements": self.0.borrow().iter().map(|n| json!({ "name": n })).collect::<Vec<_>>() }).to_string() }
            fn get_fields(&self, p: &str, _f: &str) -> String {
                match p {
                    "vehicle" => json!({ "kind": "object", "armed": false }),
                    _ => json!({ "kind": "object", "linkNames": [] }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, a: &str) -> String {
                self.2.borrow_mut().push(a.to_string());
                if self.1 {
                    self.0.borrow_mut().remove(1);
                }
                json!({ "ok": true }).to_string()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let obeying = Manager(RefCell::new(vec!["SITL", "Radio"]), true, RefCell::new(Vec::new()));
        let removed = remove_configuration(&obeying, "links.removeConfiguration", r#"["@links.linkConfigurations.1", "Radio"]"#);
        assert_eq!((&removed["ok"], &removed["removed"]), (&json!(true), &json!("Radio")));
        assert_eq!(obeying.2.borrow().as_slice(), &[r#"["@links.linkConfigurations.1"]"#.to_string()], "the expected name is the core's check and is not passed on to a Qt method that takes one argument");
        let deaf = Manager(RefCell::new(vec!["SITL", "Radio"]), false, RefCell::new(Vec::new()));
        assert_eq!(remove_configuration(&deaf, "links.removeConfiguration", r#"["@links.linkConfigurations.1"]"#)["ok"], false, "a dispatched call is not a removed link; the list is read again");
    }
}
