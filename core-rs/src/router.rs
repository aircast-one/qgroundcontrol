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
    upstream: Vec<String>,
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
            (true, None) => view::unknown(path).to_string(),
            (false, _) => self.backend.get(path),
        }
    }

    pub fn get_fields(&self, path: &str, fields: &str) -> String {
        match (view::owns(path), view::lookup(path)) {
            (true, Some(v)) => v.render_fields(&self.backend, path, fields),
            (true, None) => view::unknown(path).to_string(),
            (false, _) => self.backend.get_fields(path, fields),
        }
    }

    pub fn set(&self, path: &str, value: &str) -> String {
        if crate::actions::owns_write(path) {
            return crate::actions::write(&self.backend, path, value).to_string();
        }
        match view::owns(path) {
            true => refusal(path),
            false => self.backend.set(path, value),
        }
    }

    pub fn invoke(&self, path: &str, args: &str) -> String {
        if crate::actions::owns(path) {
            return crate::actions::run(&self.backend, path, args).to_string();
        }
        match view::owns(path) {
            true => refusal(path),
            false => self.backend.invoke(path, args),
        }
    }

    pub fn watch(&self, client: &str, paths: &[String]) {
        // Only paths this client did not already hold count as fresh. A head that re-subscribes on
        // every recomposition asks for the same set repeatedly, and forcing a re-emit on each of
        // those would turn one screen's lifecycle into a stream of identical deliveries.
        let (asked, fresh): (BTreeSet<String>, Vec<String>) = {
            let mut watching = self.watching.lock().unwrap();
            let held = watching.clients.get(client).cloned().unwrap_or_default();
            match paths.is_empty() {
                true => watching.clients.remove(client),
                false => watching.clients.insert(client.to_string(), paths.iter().cloned().collect()),
            };
            let asked = watching.asked();
            watching.last.retain(|path, _| asked.contains(path));
            let fresh = paths.iter().filter(|path| !held.contains(*path) && view::lookup(path).is_some()).cloned().collect();
            (asked, fresh)
        };
        self.rewatch(&asked, !fresh.is_empty());
        // A watch used to register paths and nothing else. Two views spawn the thread that
        // produces their data inside their own compute - view.adsbTraffic and view.detections -
        // so a head that only watches never ran the compute, never started the feed, and received
        // nothing at all, for as long as it was open. Dropping `last` for the paths this client
        // asked for is what makes the next upstream emission deliver them rather than dedupe
        // against a value the client never saw; the render itself still happens on Qt's thread,
        // through the ordinary event route, because the bridge queues its first poll.
        let mut watching = self.watching.lock().unwrap();
        fresh.iter().for_each(|path| {
            watching.last.remove(path);
        });
    }

    fn rewatch(&self, asked: &BTreeSet<String>, force: bool) {
        let upstream: Vec<String> = asked
            .iter()
            .flat_map(|path| match view::lookup(path) {
                Some(v) => v.deps_for(&view::split(path).1),
                None => vec![path.clone()],
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let changed = {
            let mut watching = self.watching.lock().unwrap();
            let changed = watching.upstream != upstream;
            watching.upstream = upstream.clone();
            changed
        };
        // Re-asking for a set the backend already holds is what makes a subscribe deliver: the
        // Qt watcher clears what it last saw and re-emits every path, and only then does a view
        // whose dependencies were already watched by some other view get recomputed. Without it a
        // second subscriber to an already-watched set is registered and never served.
        if changed || force {
            self.backend.watch(&upstream);
        }
    }

    pub fn on_event(&self, path: &str, json: &str) -> Vec<(String, String)> {
        let (direct, dependents): (bool, Vec<(String, &'static view::View)>) = {
            let asked = self.watching.lock().unwrap().asked();
            (
                asked.contains(path),
                asked
                    .iter()
                    .filter_map(|p| view::lookup(p).map(|v| (p.clone(), v)))
                    .filter(|(p, v)| v.deps_for(&view::split(p).1).iter().any(|d| d == path))
                    .collect(),
            )
        };
        let recomputed: Vec<(String, String)> =
            dependents.iter().map(|(asked_path, v)| (asked_path.clone(), v.render(&self.backend, asked_path))).collect();
        let (changed, asked): (Vec<(String, String)>, BTreeSet<String>) = {
            let mut watching = self.watching.lock().unwrap();
            let changed = recomputed
                .into_iter()
                .filter(|(p, j)| watching.last.insert(p.clone(), j.clone()).as_deref() != Some(j.as_str()))
                .collect();
            (changed, watching.asked())
        };
        if !dependents.is_empty() {
            self.rewatch(&asked, false);
        }
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
        pack_count: RefCell<usize>,
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.formattedMessages" => json!({ "kind": "value", "value": *self.messages.borrow() }).to_string(),
                "vehicle.armed" => json!({ "kind": "value", "value": true }).to_string(),
                "vehicle.batteries.count" => json!({ "kind": "value", "value": *self.pack_count.borrow() }).to_string(),
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
    fn a_view_path_that_does_not_exist_says_so_instead_of_answering_null() {
        let core = Core::new(Fake::default());
        let missing = parsed(&core.get("view.obstacleDistance"));
        assert_eq!(missing["kind"], "null", "kind stays null so nothing switching on it changes, and the reason rides beside it");
        assert_eq!(
            missing["reason"],
            "no such view: view.obstacleDistance - did you mean view.obstacle?",
            "a refusal for a path that does not exist and a refusal for a path with nothing to say were the same bytes, and the core is the only thing that knows the difference - a head reading a mistyped name got kind:null twice in two turns and took it as data both times"
        );

        assert_eq!(parsed(&core.get("view.notAnythingAtAll"))["reason"], "no such view: view.notAnythingAtAll", "with no near name there is nothing to suggest, and suggesting something unrelated would be worse than suggesting nothing");
        assert_eq!(parsed(&core.get_fields("view.obstacleDistance", "enabled"))["reason"], "no such view: view.obstacleDistance - did you mean view.obstacle?", "the field-list read refuses the same way, or a head that asks for fields learns nothing");
        assert!(parsed(&core.get("view.obstacle")).get("reason").is_none(), "a view that exists and has nothing to say must not pick up a refusal, which would make every quiet answer look like a typo");
    }

    #[test]
    fn a_claimed_write_reaches_the_core_and_an_unclaimed_one_still_forwards() {
        // Calling actions::write directly proves the function works and says nothing about whether
        // anything CALLS it. router.set is the only door, and an unclaimed path goes straight to
        // the backend with every check skipped and the suite still green - the qgc_core_guided
        // shape, which no test of the function itself can see.
        let core = Core::new(Fake::default());
        let claimed = parsed(&core.set("vehicle.cameraManager.currentCameraInstance.zoomLevel", &json!({ "value": 40.0 }).to_string()));
        assert_eq!(claimed["ok"], false, "the fake serves no camera, so the core answers rather than writing");
        assert!(claimed["reason"].as_str().unwrap().contains("No camera"), "and a bare passthrough would carry a path instead of a reason: {claimed}");

        let forwarded = parsed(&core.set("vehicle.somethingElse", &json!({ "value": 1 }).to_string()));
        assert_eq!(forwarded["path"], "vehicle.somethingElse", "an unclaimed write still forwards untouched");
        assert_eq!(parsed(&core.set("view.messages", "{}"))["ok"], false, "a view is still not writable");
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
    fn a_view_with_arguments_watches_the_facts_those_arguments_name() {
        let core = Core::new(Fake::default());
        core.watch("fly", &["view.instruments(gps/count, vehicle/heading)".to_string()]);
        assert_eq!(*core.backend.watched.borrow(), vec!["settings.unitsSettings.areaUnits".to_string(), "settings.unitsSettings.horizontalDistanceUnits".to_string(), "settings.unitsSettings.speedUnits".to_string(), "settings.unitsSettings.verticalDistanceUnits".to_string(), "vehicle.gps.count".to_string(), "vehicle.heading".to_string(), "vehicles.activeVehicleAvailable".to_string()], "the strip converts every reading itself, so a change of unit has to wake it");
        assert_eq!(core.on_event("vehicle.heading", "{}").len(), 1);
        assert!(core.on_event("vehicle.vehicle", "{}").is_empty(), "the whole vehicle object is no longer a dependency");
    }

    #[test]
    fn a_view_whose_dependencies_grow_after_a_read_is_re_watched() {
        let core = Core::new(Fake::default());
        core.watch("fly", &["view.battery".to_string()]);
        assert!(core.backend.watched.borrow().contains(&"vehicle.batteries.count".to_string()));
        assert!(!core.backend.watched.borrow().contains(&"vehicle.batteries.2.voltage".to_string()), "a third pack is not watched until a vehicle reports one");
        *core.backend.pack_count.borrow_mut() = 3;
        core.on_event("vehicle.batteries.count", "{\"kind\":\"value\",\"value\":3}");
        let after = core.backend.watched.borrow().clone();
        assert!(after.contains(&"vehicle.batteries.2.voltage".to_string()), "the third pack's facts joined the watch after the count moved");
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

    #[test]
    fn a_file_path_carrying_a_comma_survives_routing_and_reaches_the_file() {
        let dir = std::env::temp_dir().join("qgc-core-router-comma");
        std::fs::create_dir_all(&dir).unwrap();
        let awkward = dir.join("Flights, 2026 (2).tlog");
        std::fs::write(&awkward, b"not a real tlog, but a real file").unwrap();

        let core = Core::new(Fake::default());
        let read = parsed(&core.get(&format!("view.tlog({})", awkward.display())));
        std::fs::remove_file(&awkward).ok();

        assert_eq!(read["path"], awkward.display().to_string(), "split() returning one argument is not the same as the file being read, so this drives the whole door - owns, lookup, split, compute, std::fs - and the path echoed back has to be the whole name");
        assert_eq!(read["readable"], true, "and it opened: a truncated path reads as a missing file, which is exactly how this failed before");
    }

    #[test]
    fn subscribing_to_a_view_whose_dependencies_are_already_watched_still_asks_the_backend() {
        let core = Core::new(Fake::default());
        core.watch("fly", &["view.messages".to_string()]);
        let first = core.backend.watched.borrow().clone();

        core.backend.watched.borrow_mut().clear();
        core.watch("plan", &["view.messages".to_string()]);
        assert_eq!(*core.backend.watched.borrow(), first, "a second client asking for a view the first already watches changes no upstream path, so the backend was never re-asked - and the Qt watcher's re-emit is the only thing that runs the compute. view.adsbTraffic and view.detections spawn their feed inside that compute, so a head that only watches received nothing at all");
    }

    #[test]
    fn a_watched_view_is_delivered_again_rather_than_deduped_against_a_value_this_client_never_saw() {
        let core = Core::new(Fake::default());
        core.watch("fly", &["view.messages".to_string()]);
        assert!(!core.on_event("vehicle.formattedMessages", "{}").is_empty(), "the first event after a subscribe has to reach the subscriber");

        assert!(core.on_event("vehicle.formattedMessages", "{}").is_empty(), "and an unchanged value after that is still deduped, or every dep tick becomes a delivery");

        core.watch("plan", &["view.messages".to_string()]);
        assert!(!core.on_event("vehicle.formattedMessages", "{}").is_empty(), "a new subscriber drops the remembered value, because last is what the LAST client was sent and this one has been sent nothing");
    }

    #[test]
    fn re_asking_for_a_set_this_client_already_holds_delivers_nothing_new() {
        let core = Core::new(Fake::default());
        core.watch("fly", &["view.messages".to_string()]);
        core.on_event("vehicle.formattedMessages", "{}");

        core.backend.watched.borrow_mut().clear();
        core.watch("fly", &["view.messages".to_string()]);
        assert!(core.backend.watched.borrow().is_empty(), "a head that re-subscribes on every recomposition asks for the same set repeatedly, and forcing a re-emit on each would turn one screen's lifecycle into a stream of identical deliveries");
        assert!(core.on_event("vehicle.formattedMessages", "{}").is_empty(), "and the value it already has is still deduped");

        core.watch("fly", &["view.messages".to_string(), "view.warnings".to_string()]);
        assert!(!core.backend.watched.borrow().is_empty(), "adding a path to a set is a fresh subscription for that path, and it has to reach the backend");
    }
}
