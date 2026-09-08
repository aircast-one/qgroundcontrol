use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::altitude;
use crate::guided;
use crate::messages;
use crate::plan;
use crate::read::value_string;
use crate::speed;
use crate::takeoff;
use crate::router::Backend;

pub struct View {
    pub path: &'static str,
    pub deps: &'static [&'static str],
    compute: fn(&dyn Backend, &[String]) -> Value,
}

pub const VIEWS: &[View] = &[
    View { path: "view.messages", deps: &["vehicle.formattedMessages"], compute: messages_view },
    View { path: "view.plan", deps: plan::DEPS, compute: plan::plan_view },
    View { path: "view.guidedActions", deps: guided::DEPS, compute: guided::guided_view },
    View { path: "view.guidedAltitude", deps: altitude::DEPS, compute: altitude::altitude_view },
    View { path: "view.guidedTakeoff", deps: takeoff::DEPS, compute: takeoff::takeoff_view },
    View { path: "view.guidedSpeed", deps: speed::DEPS, compute: speed::speed_view },
];

pub fn owns(path: &str) -> bool {
    path.split(['.', '[']).next() == Some("view")
}

pub fn lookup(path: &str) -> Option<&'static View> {
    let (base, _) = split(path);
    VIEWS.iter().find(|view| view.path == base)
}

pub fn split(path: &str) -> (&str, Vec<String>) {
    match path.split_once('(') {
        Some((base, rest)) => (
            base,
            rest.trim_end_matches(')').split(',').map(str::trim).filter(|a| !a.is_empty()).map(str::to_string).collect(),
        ),
        None => (path, Vec::new()),
    }
}

impl View {
    pub fn render(&self, backend: &dyn Backend, path: &str) -> String {
        (self.compute)(backend, &split(path).1).to_string()
    }

    pub fn render_fields(&self, backend: &dyn Backend, path: &str, fields: &str) -> String {
        let value = (self.compute)(backend, &split(path).1);
        let Value::Object(map) = value else { return value.to_string() };
        if fields.trim() == "*" {
            return Value::Object(map).to_string();
        }
        let wanted: BTreeSet<&str> = fields.split(',').map(str::trim).filter(|f| !f.is_empty()).collect();
        let unknown: Vec<&str> = wanted.iter().copied().filter(|f| !map.contains_key(*f)).collect();
        let kept = map
            .into_iter()
            .filter(|(key, _)| key == "kind" || key == "class" || wanted.contains(key.as_str()))
            .chain((!unknown.is_empty()).then(|| ("unknownFields".to_string(), json!(unknown))))
            .collect();
        Value::Object(kept).to_string()
    }
}

fn messages_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let items = messages::parse(&value_string(&backend.get("vehicle.formattedMessages")));
    json!({ "kind": "object", "class": "VehicleMessages", "count": items.len(), "items": items })
}

