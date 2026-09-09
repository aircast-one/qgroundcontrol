use serde_json::{Value, json};

use crate::label::humanise;
use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable"];

pub const DEFAULTS: &[&str] = &["vehicle/altitudeRelative", "vehicle/groundSpeed", "vehicle/climbRate", "vehicle/distanceToHome", "vehicle/heading", "vehicle/altitudeAMSL"];

const ABSENT: &str = "\u{2014}";

pub fn display_units(units: &str) -> &str {
    match units {
        "v" => "V",
        other => other,
    }
}

pub fn fact_path(group: &str, name: &str) -> String {
    match group {
        "" | "vehicle" => format!("vehicle.{name}"),
        other => format!("vehicle.{other}.{name}"),
    }
}

fn split_selection(selection: &str) -> (String, String) {
    match selection.split_once('/') {
        Some((group, name)) => (group.to_string(), name.to_string()),
        None => ("vehicle".to_string(), selection.to_string()),
    }
}

fn selections(args: &[String]) -> Vec<(String, String)> {
    let chosen: Vec<(String, String)> = match args.is_empty() {
        true => DEFAULTS.iter().map(|s| split_selection(s)).collect(),
        false => args.iter().map(|s| split_selection(s)).collect(),
    };
    chosen.into_iter().filter(|(_, name)| !name.is_empty()).collect()
}

pub fn deps_for(args: &[String]) -> Vec<String> {
    DEPS.iter()
        .map(|d| d.to_string())
        .chain(selections(args).iter().map(|(group, name)| fact_path(group, name)))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn instruments_view(backend: &dyn Backend, args: &[String]) -> Value {
    let items: Vec<Value> = selections(args)
        .iter()
        .map(|(group, name)| {
            let fact = object(&backend.get(&fact_path(group, name)));
            let described = fact.get("shortDescription").and_then(Value::as_str).filter(|d| !d.is_empty());
            let value = fact.get("valueString").and_then(Value::as_str).filter(|v| !v.is_empty());
            let units = fact.get("units").and_then(Value::as_str).map(display_units).unwrap_or("");
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
                "vehicle.altitudeRelative" => json!({ "kind": "fact", "name": "altitudeRelative", "shortDescription": "Alt (Rel)", "valueString": "25.0", "units": "m" }),
                "vehicle.groundSpeed" => json!({ "kind": "fact", "name": "groundSpeed", "shortDescription": "", "valueString": "", "units": "m/s" }),
                "vehicle.batteries.0.voltage" => json!({ "kind": "fact", "name": "voltage", "shortDescription": "Voltage", "valueString": "15.80", "units": "v" }),
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

    #[test]
    fn dependencies_are_the_selected_facts_not_their_groups() {
        assert_eq!(deps_for(&["gps/count".to_string(), "vehicle/heading".to_string(), "batteries.0/voltage".to_string()]), vec!["vehicle.batteries.0.voltage", "vehicle.gps.count", "vehicle.heading", "vehicles.activeVehicleAvailable"]);
        assert_eq!(deps_for(&[]).len(), 7);
        assert!(deps_for(&[]).contains(&"vehicle.altitudeRelative".to_string()));
        assert_eq!(deps_for(&["/".to_string(), "".to_string(), "gps/".to_string()]), vec!["vehicles.activeVehicleAvailable"], "an empty name never turns into a read of the whole vehicle");
        assert!(instruments_view(&Fake, &["vehicle/".to_string()])["items"].as_array().unwrap().is_empty());
    }
}
