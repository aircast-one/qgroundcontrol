use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::guided;
use crate::messages;
use crate::plan;
use crate::router::Backend;

pub struct View {
    pub path: &'static str,
    pub deps: &'static [&'static str],
    compute: fn(&dyn Backend) -> Value,
}

pub const VIEWS: &[View] = &[
    View { path: "view.messages", deps: &["vehicle.formattedMessages"], compute: messages_view },
    View { path: "view.plan", deps: plan::DEPS, compute: plan::plan_view },
    View { path: "view.guidedActions", deps: guided::DEPS, compute: guided::guided_view },
];

pub fn owns(path: &str) -> bool {
    path.split(['.', '[']).next() == Some("view")
}

pub fn lookup(path: &str) -> Option<&'static View> {
    VIEWS.iter().find(|view| view.path == path)
}

impl View {
    pub fn render(&self, backend: &dyn Backend) -> String {
        (self.compute)(backend).to_string()
    }

    pub fn render_fields(&self, backend: &dyn Backend, fields: &str) -> String {
        let value = (self.compute)(backend);
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

fn messages_view(backend: &dyn Backend) -> Value {
    let items = messages::parse(&value_string(&backend.get("vehicle.formattedMessages")));
    json!({ "kind": "object", "class": "VehicleMessages", "count": items.len(), "items": items })
}

fn value_string(json: &str) -> String {
    serde_json::from_str::<Value>(json)
        .ok()
        .and_then(|value| value.get("value")?.as_str().map(str::to_owned))
        .unwrap_or_default()
}
