use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["mavlinkConsole.lines", "vehicles.activeVehicleAvailable"];

const MAX_HISTORY: usize = 100;

#[derive(Debug, Default)]
pub struct CommandHistory {
    lines: Vec<String>,
    index: usize,
}

impl CommandHistory {
    pub fn sent(&mut self, command: &str) {
        command.split('\n').filter(|line| !line.is_empty()).for_each(|line| self.append(line));
    }

    fn append(&mut self, command: &str) {
        if self.lines.last().map(String::as_str) != Some(command) {
            if self.lines.len() >= MAX_HISTORY {
                self.lines.remove(0);
            }
            self.lines.push(command.to_string());
        }
        self.index = self.lines.len();
    }

    pub fn up(&mut self, current: &str) -> String {
        if self.index == 0 {
            return current.to_string();
        }
        self.index -= 1;
        self.lines.get(self.index).cloned().unwrap_or_default()
    }

    pub fn down(&mut self, current: &str) -> String {
        if self.index >= self.lines.len() {
            return current.to_string();
        }
        self.index += 1;
        self.lines.get(self.index).cloned().unwrap_or_default()
    }
}

pub static HISTORY: std::sync::Mutex<CommandHistory> = std::sync::Mutex::new(CommandHistory { lines: Vec::new(), index: 0 });

pub fn console_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let connected = flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
    let read = object(&backend.get_fields("mavlinkConsole", "lines"));
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_history_recalls_like_the_console_controller() {
        let mut history = CommandHistory::default();
        history.sent("ver all\nver all\n\nparam show");
        assert_eq!(history.up("typed"), "param show", "each line of a multi-line send is its own entry, and a repeat is kept once");
        assert_eq!(history.up(""), "ver all");
        assert_eq!(history.up("x"), "x", "at the oldest entry up keeps what is typed");
        assert_eq!(history.down(""), "param show");
        assert_eq!(history.down(""), "", "past the newest entry the line is blank");
        assert_eq!(history.down("draft"), "draft");
    }

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
    fn the_console_serves_only_what_the_shell_printed() {
        let unplugged = console_view(&Console(Some(vec![]), false), &[]);
        assert_eq!((unplugged["count"].clone(), unplugged["connected"].clone()), (json!(0), json!(false)));
        assert!(unplugged.get("emptyReason").is_none(), "MAVLinkConsolePage has no empty-state sentence; an empty console is an empty text area");

        let talking = console_view(&Console(Some(vec!["nsh> ", "ekf2 status"]), true), &[]);
        assert_eq!(talking["count"], 2);
        assert_eq!(talking["last"], "ekf2 status");

        let mid_line = console_view(&Console(Some(vec!["nsh> ", "ekf2 status", ""]), true), &[]);
        assert_eq!(mid_line["count"], 2, "the controller adds the row for the line being assembled the moment a newline lands");
        assert_eq!(mid_line["last"], "ekf2 status");

        let blank_inside = console_view(&Console(Some(vec!["nsh> ver all", "", "HW arch: PX4_FMU_V5", ""]), true), &[]);
        assert_eq!(blank_inside["count"], 3, "only the row being assembled goes; a blank line the vehicle printed stays");
        assert_eq!(blank_inside["lines"][1], "");

        let only_partial = console_view(&Console(Some(vec![""]), true), &[]);
        assert_eq!(only_partial["count"], 0);

        let absent = console_view(&Console(None, true), &[]);
        assert_eq!(absent["count"], 0);
    }
}
