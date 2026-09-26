use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

const POINTS: &str = "plan.rallyPointController.points";

#[derive(Clone, Copy, Debug, Default)]
struct Rally {
    supported: Option<bool>,
    syncing: bool,
    count: usize,
}

fn rally(backend: &dyn Backend) -> Rally {
    Rally {
        supported: crate::plan::capability(backend, "rallyPointController"),
        syncing: flag(&object(&backend.get_fields("plan", "syncInProgress")), "syncInProgress"),
        count: count(backend),
    }
}

fn count(backend: &dyn Backend) -> usize {
    object(&backend.get(POINTS)).get("elements").and_then(Value::as_array).map_or(0, Vec::len)
}

fn coordinate(args: &Value) -> Option<(f64, f64)> {
    let point = args.get(0)?;
    let latitude = point.get("latitude")?.as_f64().filter(|v| v.is_finite() && (-90.0..=90.0).contains(v))?;
    let longitude = point.get("longitude")?.as_f64().filter(|v| v.is_finite() && (-180.0..=180.0).contains(v))?;
    Some((latitude, longitude))
}

fn add_refusal(state: Rally, at: Option<(f64, f64)>) -> Option<(&'static str, &'static str)> {
    match () {
        _ if at.is_none() => Some(("badCoordinate", "A rally point needs a latitude from -90 to 90 and a longitude from -180 to 180.")),
        _ if state.supported == Some(false) => Some(("unsupported", "This link does not accept rally points.")),
        _ if state.syncing => Some(("busy", "Wait for the sync to finish before adding a rally point.")),
        _ => None,
    }
}

fn remove_refusal(state: Rally, index: Option<usize>) -> Option<(&'static str, String)> {
    match index {
        None => Some(("malformed", format!("Name the rally point as @{POINTS}.<index>."))),
        Some(_) if state.syncing => Some(("busy", "Wait for the sync to finish before removing a rally point.".to_string())),
        Some(i) if i >= state.count => Some(("noSuchPoint", format!("There is no rally point at position {i}."))),
        Some(_) => None,
    }
}

pub fn add_point(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let at = coordinate(&given);
    let before = rally(backend);
    if let Some((token, reason)) = add_refusal(before, at) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let (latitude, longitude) = at.unwrap_or_default();
    let dispatched = flag(&object(&backend.invoke(path, &json!([{ "latitude": latitude, "longitude": longitude }]).to_string())), "ok");
    let added = dispatched && count(backend) == before.count + 1;
    json!({
        "ok": added,
        "refusal": Value::Null,
        "index": added.then_some(before.count),
        "reason": match added { true => Value::Null, false => json!("The rally point was not added.") },
    })
}

pub fn remove_point(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let index = given.get(0).and_then(Value::as_str).and_then(|r| r.strip_prefix('@')?.strip_prefix(POINTS)?.strip_prefix('.')?.parse::<usize>().ok());
    let before = rally(backend);
    if let Some((token, reason)) = remove_refusal(before, index) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let dispatched = flag(&object(&backend.invoke(path, &json!([format!("@{POINTS}.{}", index.unwrap_or(0))]).to_string())), "ok");
    let removed = dispatched && count(backend) + 1 == before.count;
    let marked = removed && flag(&object(&backend.set("plan.rallyPointController.dirty", &json!({ "value": true }).to_string())), "ok");
    json!({
        "ok": removed,
        "refusal": Value::Null,
        "markedUnsaved": marked,
        "reason": match removed { true => Value::Null, false => json!("The rally point is still there.") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn a_rally_point_is_added_only_somewhere_real_on_a_link_that_takes_them() {
        let open = Rally { supported: Some(true), syncing: false, count: 2 };
        let at = |lat: f64, lon: f64| coordinate(&json!([{ "latitude": lat, "longitude": lon }]));
        assert_eq!(add_refusal(open, at(47.4, 8.5)), None);
        assert_eq!(add_refusal(open, at(147.4, 8.5)).map(|r| r.0), Some("badCoordinate"), "addPoint appends whatever coordinate it is given");
        assert_eq!(add_refusal(open, None).map(|r| r.0), Some("badCoordinate"));
        assert_eq!(add_refusal(Rally { supported: Some(false), ..open }, at(47.4, 8.5)).map(|r| r.0), Some("unsupported"), "a point the vehicle will not take still dirties the plan and waits for an upload that drops it");
        assert_eq!(add_refusal(Rally { supported: None, ..open }, at(47.4, 8.5)), None, "a vehicle that has not said what it accepts is offered rally, as view.plan offers it");
        assert_eq!(add_refusal(Rally { syncing: true, ..open }, at(47.4, 8.5)).map(|r| r.0), Some("busy"));
        assert_eq!(remove_refusal(open, Some(1)), None);
        assert_eq!(remove_refusal(open, Some(2)).map(|r| r.0), Some("noSuchPoint"), "a stale position reaches removePoint as a null object, which matches nothing and says nothing");
        assert_eq!(remove_refusal(open, None).map(|r| r.0), Some("malformed"));
    }

    struct Controller {
        points: RefCell<usize>,
        obeys: bool,
        writes: RefCell<Vec<String>>,
    }

    impl Backend for Controller {
        fn get(&self, _p: &str) -> String { json!({ "kind": "object", "elements": vec![json!({}); *self.points.borrow()] }).to_string() }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "plan.managerVehicle" => json!({ "kind": "object", "capabilitiesKnown": true }),
                "plan.rallyPointController" => json!({ "kind": "object", "supported": true }),
                _ => json!({ "kind": "object", "syncInProgress": false }),
            }
            .to_string()
        }
        fn set(&self, p: &str, _v: &str) -> String {
            self.writes.borrow_mut().push(p.to_string());
            json!({ "ok": true }).to_string()
        }
        fn invoke(&self, p: &str, _a: &str) -> String {
            if self.obeys {
                let delta: isize = if p.ends_with("addPoint") { 1 } else { -1 };
                let now = *self.points.borrow() as isize + delta;
                *self.points.borrow_mut() = now as usize;
            }
            json!({ "ok": true }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_removed_rally_point_leaves_the_plan_marked_unsaved() {
        let controller = Controller { points: RefCell::new(2), obeys: true, writes: RefCell::new(Vec::new()) };
        let removed = remove_point(&controller, "plan.rallyPointController.removePoint", r#"["@plan.rallyPointController.points.0"]"#);
        assert_eq!((&removed["ok"], &removed["markedUnsaved"]), (&json!(true), &json!(true)));
        assert_eq!(
            controller.writes.borrow().as_slice(),
            &["plan.rallyPointController.dirty".to_string()],
            "addPoint calls setDirty(true) and removePoint does not, so deleting a rally point after a save left the plan reading saved and in sync with the vehicle"
        );
        let added = add_point(&controller, "plan.rallyPointController.addPoint", r#"[{"latitude":47.4,"longitude":8.5}]"#);
        assert_eq!((&added["ok"], &added["index"]), (&json!(true), &json!(1)));

        let deaf = Controller { points: RefCell::new(2), obeys: false, writes: RefCell::new(Vec::new()) };
        assert_eq!(remove_point(&deaf, "plan.rallyPointController.removePoint", r#"["@plan.rallyPointController.points.1"]"#)["ok"], false, "the count is read again, since a dispatched call is not a removed point");
        assert!(deaf.writes.borrow().is_empty(), "nothing is marked unsaved when nothing changed");
        assert_eq!(add_point(&deaf, "plan.rallyPointController.addPoint", r#"[{"latitude":47.4,"longitude":8.5}]"#)["ok"], false);
    }
}
