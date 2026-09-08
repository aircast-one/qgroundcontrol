use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use serde_json::json;

use crate::view;

pub trait Backend {
    fn get(&self, path: &str) -> String;
    fn get_fields(&self, path: &str, fields: &str) -> String;
    fn set(&self, path: &str, value: &str) -> String;
    fn invoke(&self, path: &str, args: &str) -> String;
    fn watch(&self, paths: &[String]);
}

#[derive(Default)]
struct Watching {
    clients: BTreeMap<String, BTreeSet<String>>,
    last: BTreeMap<String, String>,
}

impl Watching {
    fn asked(&self) -> BTreeSet<String> {
        self.clients.values().flatten().cloned().collect()
    }
}

pub struct Core<B> {
    backend: B,
    watching: Mutex<Watching>,
}

impl<B: Backend> Core<B> {
    pub fn new(backend: B) -> Self {
        Self { backend, watching: Mutex::new(Watching::default()) }
    }

    pub fn get(&self, path: &str) -> String {
        match (view::owns(path), view::lookup(path)) {
            (true, Some(v)) => v.render(&self.backend, path),
            (true, None) => null(),
            (false, _) => self.backend.get(path),
        }
    }

    pub fn get_fields(&self, path: &str, fields: &str) -> String {
        match (view::owns(path), view::lookup(path)) {
            (true, Some(v)) => v.render_fields(&self.backend, path, fields),
            (true, None) => null(),
            (false, _) => self.backend.get_fields(path, fields),
        }
    }

    pub fn set(&self, path: &str, value: &str) -> String {
        match view::owns(path) {
            true => refusal(path),
            false => self.backend.set(path, value),
        }
    }

    pub fn invoke(&self, path: &str, args: &str) -> String {
        match view::owns(path) {
            true => refusal(path),
            false => self.backend.invoke(path, args),
        }
    }

    pub fn watch(&self, client: &str, paths: &[String]) {
        let asked: BTreeSet<String> = {
            let mut watching = self.watching.lock().unwrap();
            match paths.is_empty() {
                true => watching.clients.remove(client),
                false => watching.clients.insert(client.to_string(), paths.iter().cloned().collect()),
            };
            let asked = watching.asked();
            watching.last.retain(|path, _| asked.contains(path));
            asked
        };
        let upstream: Vec<String> = asked
            .iter()
            .flat_map(|path| match view::lookup(path) {
                Some(v) => v.deps.iter().map(|d| d.to_string()).collect::<Vec<_>>(),
                None => vec![path.clone()],
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        self.backend.watch(&upstream);
    }

    pub fn on_event(&self, path: &str, json: &str) -> Vec<(String, String)> {
        let (direct, dependents): (bool, Vec<(String, &'static view::View)>) = {
            let asked = self.watching.lock().unwrap().asked();
            (
                asked.contains(path),
                asked
                    .iter()
                    .filter_map(|p| view::lookup(p).map(|v| (p.clone(), v)))
                    .filter(|(_, v)| v.deps.contains(&path))
                    .collect(),
            )
        };
        let recomputed: Vec<(String, String)> =
            dependents.iter().map(|(asked_path, v)| (asked_path.clone(), v.render(&self.backend, asked_path))).collect();
        let changed: Vec<(String, String)> = {
            let mut watching = self.watching.lock().unwrap();
            recomputed
                .into_iter()
                .filter(|(p, j)| watching.last.insert(p.clone(), j.clone()).as_deref() != Some(j.as_str()))
                .collect()
        };
        direct.then(|| (path.to_string(), json.to_string())).into_iter().chain(changed).collect()
    }
}

fn null() -> String {
    json!({ "kind": "null" }).to_string()
}

fn refusal(path: &str) -> String {
    json!({ "ok": false, "reason": format!("{path} is derived by the core and read-only") }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct Fake {
        messages: RefCell<String>,
        watched: RefCell<Vec<String>>,
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.formattedMessages" => json!({ "kind": "value", "value": *self.messages.borrow() }).to_string(),
                "vehicle.armed" => json!({ "kind": "value", "value": true }).to_string(),
                _ => null(),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            self.get(path)
        }
        fn set(&self, path: &str, _value: &str) -> String {
            json!({ "ok": true, "path": path }).to_string()
        }
        fn invoke(&self, path: &str, _args: &str) -> String {
            json!({ "ok": true, "path": path }).to_string()
        }
        fn watch(&self, paths: &[String]) {
            *self.watched.borrow_mut() = paths.to_vec();
        }
    }

    const ONE: &str = "<font style=\"c\">[1:2:3.4 ] Info: hello</font><br/>";

    fn parsed(text: &str) -> serde_json::Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn view_paths_are_served_by_the_core_and_everything_else_forwards() {
        let core = Core::new(Fake::default());
        *core.backend.messages.borrow_mut() = ONE.to_string();
        let messages = parsed(&core.get("view.messages"));
        assert_eq!(messages["class"], "VehicleMessages");
        assert_eq!(messages["count"], 1);
        assert_eq!(messages["items"][0]["text"], "hello");
        assert_eq!(parsed(&core.get("vehicle.armed"))["value"], true);
        assert_eq!(parsed(&core.get("view.nothing"))["kind"], "null");
    }

    #[test]
    fn view_paths_refuse_writes_the_way_the_qt_bridge_does() {
        let core = Core::new(Fake::default());
        assert_eq!(parsed(&core.set("view.messages", "{\"value\":1}"))["ok"], false);
        assert_eq!(parsed(&core.invoke("view.messages", "[]"))["ok"], false);
        assert_eq!(parsed(&core.set("vehicle.armed", "{\"value\":1}"))["ok"], true);
    }

    #[test]
    fn get_fields_projects_and_reports_unknown_names() {
        let core = Core::new(Fake::default());
        let projected = parsed(&core.get_fields("view.messages", "count,bogus"));
        assert_eq!(projected["count"], 0);
        assert!(projected.get("items").is_none());
        assert_eq!(projected["unknownFields"][0], "bogus");
    }

    #[test]
    fn watching_a_view_watches_its_dependencies_upstream() {
        let core = Core::new(Fake::default());
        core.watch("", &["view.messages".to_string(), "vehicle.armed".to_string()]);
        assert_eq!(*core.backend.watched.borrow(), vec!["vehicle.armed".to_string(), "vehicle.formattedMessages".to_string()]);
    }

    #[test]
    fn dependency_events_recompute_the_view_and_emit_only_on_change() {
        let core = Core::new(Fake::default());
        core.watch("", &["view.messages".to_string()]);
        *core.backend.messages.borrow_mut() = ONE.to_string();
        let first = core.on_event("vehicle.formattedMessages", "{}");
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].0, "view.messages");
        assert_eq!(parsed(&first[0].1)["count"], 1);
        assert!(core.on_event("vehicle.formattedMessages", "{}").is_empty());
        assert!(core.on_event("vehicle.armed", "{}").is_empty());
    }

    #[test]
    fn clients_keep_their_own_paths_and_the_union_goes_upstream() {
        let core = Core::new(Fake::default());
        core.watch("fly", &["vehicle.armed".to_string()]);
        core.watch("plan", &["view.messages".to_string()]);
        assert_eq!(*core.backend.watched.borrow(), vec!["vehicle.armed".to_string(), "vehicle.formattedMessages".to_string()]);
        core.watch("fly", &["vehicle.flying".to_string()]);
        assert_eq!(*core.backend.watched.borrow(), vec!["vehicle.flying".to_string(), "vehicle.formattedMessages".to_string()]);
        assert!(core.on_event("vehicle.armed", "{}").is_empty());
        core.watch("plan", &[]);
        assert_eq!(*core.backend.watched.borrow(), vec!["vehicle.flying".to_string()]);
        core.watch("fly", &[]);
        assert!(core.backend.watched.borrow().is_empty());
    }

    #[test]
    fn a_view_path_may_carry_arguments_in_parentheses() {
        assert_eq!(view::split("view.guidedAltitude(68.5)"), ("view.guidedAltitude", vec!["68.5".to_string()]));
        assert_eq!(view::split("view.plan"), ("view.plan", vec![]));
        assert_eq!(
            view::split("view.control(vehicle.parameterManager.getParameter(-1,RTL_ALT))"),
            ("view.control", vec!["vehicle.parameterManager.getParameter(-1,RTL_ALT)".to_string()])
        );
        assert_eq!(view::split("view.instruments(gps/count, vehicle/heading)").1.len(), 2);
        assert_eq!(view::split("view.linkForm(udp,,14550)").1, vec!["udp".to_string(), String::new(), "14550".to_string()]);
        assert!(view::split("view.plan()").1.is_empty());
        assert_eq!(
            view::split_paths("vehicle.armed,view.instruments(altitudeRelative,groundSpeed),view.plan"),
            vec!["vehicle.armed".to_string(), "view.instruments(altitudeRelative,groundSpeed)".to_string(), "view.plan".to_string()]
        );
        assert!(view::lookup("view.guidedAltitude(1, 2)").is_some());
        let core = Core::new(Fake::default());
        assert_eq!(parsed(&core.get("view.messages(anything)"))["class"], "VehicleMessages");
    }

    #[test]
    fn directly_watched_paths_pass_through_untouched() {
        let core = Core::new(Fake::default());
        core.watch("", &["vehicle.armed".to_string(), "view.messages".to_string()]);
        let events = core.on_event("vehicle.armed", "{\"kind\":\"value\",\"value\":true}");
        assert_eq!(events, vec![("vehicle.armed".to_string(), "{\"kind\":\"value\",\"value\":true}".to_string())]);
    }
}
