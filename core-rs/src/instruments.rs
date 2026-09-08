use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::label::humanise;
use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.vehicle", "vehicle.gps", "vehicle.batteries", "vehicle.wind"];

pub const DEFAULTS: &[&str] = &["vehicle/altitudeRelative", "vehicle/groundSpeed", "vehicle/climbRate", "vehicle/distanceToHome", "vehicle/heading", "vehicle/altitudeAMSL"];

const ABSENT: &str = "\u{2014}";

pub fn display_units(units: &str) -> &str {
    match units {
        "v" => "V",
        other => other,
    }
}

pub fn group_path(group: &str) -> String {
    match group {
        "" | "vehicle" => "vehicle.vehicle".to_string(),
        other => format!("vehicle.{other}"),
    }
}

fn split_selection(selection: &str) -> (String, String) {
    match selection.split_once('/') {
        Some((group, name)) => (group.to_string(), name.to_string()),
        None => ("vehicle".to_string(), selection.to_string()),
    }
}

pub fn instruments_view(backend: &dyn Backend, args: &[String]) -> Value {
    let selections: Vec<(String, String)> = match args.is_empty() {
        true => DEFAULTS.iter().map(|s| split_selection(s)).collect(),
        false => args.iter().map(|s| split_selection(s)).collect(),
    };
    let groups: BTreeMap<String, Value> = selections
        .iter()
        .map(|(group, _)| group.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|group| {
            let read = object(&backend.get(&group_path(&group)));
            (group, read)
        })
        .collect();
    let items: Vec<Value> = selections
        .iter()
        .map(|(group, name)| {
            let fact = groups.get(group).and_then(|g| g.get("facts")).and_then(Value::as_array).and_then(|facts| facts.iter().find(|f| f.get("name").and_then(Value::as_str) == Some(name)));
            let described = fact.and_then(|f| f.get("shortDescription")).and_then(Value::as_str).filter(|d| !d.is_empty());
            let value = fact.and_then(|f| f.get("valueString")).and_then(Value::as_str).filter(|v| !v.is_empty());
            let units = fact.and_then(|f| f.get("units")).and_then(Value::as_str).map(display_units).unwrap_or("");
            json!({
                "id": format!("{group}/{name}"),
                "group": group,
                "name": name,
                "label": described.map(str::to_string).unwrap_or_else(|| humanise(name)),
                "value": value.unwrap_or(ABSENT),
                "units": if value.is_some() { units } else { "" },
                "missing": value.is_none(),
            })
        })
        .collect();
    json!({
        "kind": "object",
        "class": "Instruments",
        "available": items.iter().any(|i| i["missing"] == false),
        "items": items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;
    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.vehicle" => json!({ "kind": "object", "facts": [
                    { "name": "altitudeRelative", "shortDescription": "Alt (Rel)", "valueString": "25.0", "units": "m" },
                    { "name": "groundSpeed", "shortDescription": "", "valueString": "", "units": "m/s" },
                ] }),
                "vehicle.batteries.0" => json!({ "kind": "object", "facts": [ { "name": "voltage", "shortDescription": "Voltage", "valueString": "15.80", "units": "v" } ] }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn defaults_are_the_six_vehicle_facts() {
        let view = instruments_view(&Fake, &[]);
        let items = view["items"].as_array().unwrap();
        assert_eq!(items.len(), 6);
        assert_eq!(items[0]["label"], "Alt (Rel)");
        assert_eq!(items[0]["value"], "25.0");
        assert_eq!(items[0]["units"], "m");
        assert_eq!(items[1]["label"], "Ground Speed");
        assert_eq!(items[1]["value"], "\u{2014}");
        assert_eq!(items[1]["missing"], true);
        assert_eq!(items[2]["label"], "Climb Rate");
        assert_eq!(view["available"], true);
    }

    #[test]
    fn a_selection_may_reach_into_another_group_and_units_are_displayed() {
        let view = instruments_view(&Fake, &["batteries.0/voltage".to_string(), "altitudeRelative".to_string()]);
        let items = view["items"].as_array().unwrap();
        assert_eq!(items[0]["id"], "batteries.0/voltage");
        assert_eq!(items[0]["units"], "V");
        assert_eq!(items[1]["group"], "vehicle");
        assert_eq!(items[1]["value"], "25.0");
    }
}
