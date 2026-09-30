use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["mavlinkConsole.lines", "vehicles.activeVehicleAvailable"];

pub fn console_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let connected = flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
    let read = object(&backend.get_fields("mavlinkConsole", "lines"));
    // MAVLinkConsoleController.cc:139 grows its model on every newline to "ensure line exists", so
    // the list always ends with the row being assembled - empty between a newline and the next
    // character. Counting it makes the total one too high and makes `last` the empty string while
    // a line is arriving.
    let mut lines: Vec<&str> = read
        .get("lines")
        .and_then(Value::as_array)
        .map(|lines| lines.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    json!({
        "kind": "object",
        "class": "MavlinkConsole",
        "connected": connected,
        "lines": lines,
        "count": lines.len(),
        "last": lines.last(),
        "emptyReason": match (lines.is_empty(), connected) {
            (false, _) => Value::Null,
            (true, false) => json!("Connect a vehicle to open a shell on it."),
            (true, true) => json!("The vehicle has printed nothing."),
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
        assert!(quiet["emptyReason"].as_str().unwrap().contains("printed nothing"), "no lines because the shell has said nothing, which is a different thing a head must not spell the same way");
        assert_ne!(quiet["emptyReason"], unplugged["emptyReason"]);

        let talking = console_view(&Console(Some(vec!["nsh> ", "ekf2 status"]), true), &[]);
        assert_eq!(talking["count"], 2);
        assert_eq!(talking["last"], "ekf2 status");

        let mid_line = console_view(&Console(Some(vec!["nsh> ", "ekf2 status", ""]), true), &[]);
        assert_eq!(mid_line["count"], 2, "the controller adds the row for the line being assembled the moment a newline lands, so counting it reports a line the vehicle has not sent");
        assert_eq!(mid_line["last"], "ekf2 status", "and taking it as `last` hands a head an empty string between a newline and the next character");
        assert_eq!(mid_line["emptyReason"], Value::Null);

        let blank_inside = console_view(&Console(Some(vec!["nsh> ver all", "", "HW arch: PX4_FMU_V5", ""]), true), &[]);
        assert_eq!(blank_inside["count"], 3, "only the row being assembled goes; a blank line the vehicle actually printed is output and spacing an operator can see");
        assert_eq!(blank_inside["lines"][1], "", "so the blank in the middle survives");
        assert_eq!(blank_inside["last"], "HW arch: PX4_FMU_V5");

        let only_partial = console_view(&Console(Some(vec![""]), true), &[]);
        assert_eq!(only_partial["count"], 0, "a console holding nothing but the row it is about to fill has printed nothing");
        assert!(only_partial["emptyReason"].as_str().unwrap().contains("printed nothing"));
        assert_eq!(talking["emptyReason"], Value::Null, "a console with output has no empty to explain");

        let absent = console_view(&Console(None, true), &[]);
        assert_eq!(absent["count"], 0, "the controller answering nothing is not the controller answering an empty list, and neither is a crash");
    }
}
