use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["mavlinkConsole.lines", "vehicles.activeVehicleAvailable"];

pub fn console_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let connected = flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
    let read = object(&backend.get_fields("mavlinkConsole", "lines"));
    let lines: Vec<&str> = read
        .get("lines")
        .and_then(Value::as_array)
        .map(|lines| lines.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    json!({
        "kind": "object",
        "class": "MavlinkConsole",
        "connected": connected,
        "lines": lines,
        "count": lines.len(),
        "last": lines.last(),
        "emptyReason": match (lines.is_empty(), connected) {
            (false, _) => Value::Null,
            (true, false) => json!("Connect to a vehicle to open a shell on it."),
            (true, true) => json!("The vehicle has not printed anything yet."),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Console(Option<Vec<&'static str>>, bool);
    impl Backend for Console {
        fn get(&self, p: &str) -> String {
            self.get_fields(p, "")
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": self.1 }).to_string(),
                "mavlinkConsole" => match &self.0 {
                    Some(lines) => json!({ "kind": "object", "lines": lines }).to_string(),
                    None => json!({ "kind": "null" }).to_string(),
                },
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            String::new()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn an_empty_console_says_which_kind_of_empty_it_is() {
        let unplugged = console_view(&Console(Some(vec![]), false), &[]);
        assert_eq!(unplugged["count"], 0);
        assert!(unplugged["emptyReason"].as_str().unwrap().contains("Connect"), "no lines because there is no vehicle");

        let quiet = console_view(&Console(Some(vec![]), true), &[]);
        assert!(quiet["emptyReason"].as_str().unwrap().contains("not printed"), "no lines because the shell has said nothing, which is a different thing a head must not spell the same way");
        assert_ne!(quiet["emptyReason"], unplugged["emptyReason"]);

        let talking = console_view(&Console(Some(vec!["nsh> ", "ekf2 status"]), true), &[]);
        assert_eq!(talking["count"], 2);
        assert_eq!(talking["last"], "ekf2 status");
        assert_eq!(talking["emptyReason"], Value::Null, "a console with output has no empty to explain");

        let absent = console_view(&Console(None, true), &[]);
        assert_eq!(absent["count"], 0, "the controller answering nothing is not the controller answering an empty list, and neither is a crash");
    }
}
