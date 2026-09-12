use serde_json::{Value, json};

use crate::label::humanise;
use crate::read::{Unit, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "settings.unitsSettings.horizontalDistanceUnits",
    "settings.unitsSettings.verticalDistanceUnits",
    "settings.unitsSettings.speedUnits",
    "settings.unitsSettings.areaUnits",
];

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

fn dimensioned(backend: &dyn Backend, raw_units: &str) -> Option<Unit> {
    match raw_units {
        "m" | "meter" | "meters" => Some(Unit::horizontal(backend)),
        "vertical m" => Some(Unit::vertical(backend)),
        "m/s" => Some(Unit::speed(backend)),
        "m^2" => Some(Unit::area(backend)),
        _ => None,
    }
}

fn converted(backend: &dyn Backend, fact: &Value) -> Option<(String, String)> {
    let raw_units = fact.get("rawUnits").and_then(Value::as_str)?;
    let unit = dimensioned(backend, raw_units)?;
    let raw = fact.get("rawValue").and_then(Value::as_f64).filter(|value| value.is_finite())?;
    let places = fact.get("decimalPlaces").and_then(Value::as_i64).unwrap_or(1).clamp(0, 6) as usize;
    Some((crate::read::settled(format!("{:.places$}", unit.show(raw))), unit.name.clone()))
}

pub fn instruments_view(backend: &dyn Backend, args: &[String]) -> Value {
    let items: Vec<Value> = selections(args)
        .iter()
        .map(|(group, name)| {
            let fact = object(&backend.get(&fact_path(group, name)));
            let described = fact.get("shortDescription").and_then(Value::as_str).filter(|d| !d.is_empty());
            let fresh = converted(backend, &fact);
            let held = fact.get("valueString").and_then(Value::as_str).filter(|v| !v.is_empty());
            let value = fresh.as_ref().map(|(shown, _)| shown.clone()).or_else(|| held.map(|v| crate::read::settled(v.to_string())));
            let units = match &fresh {
                Some((_, name)) => display_units(name.as_str()),
                None => fact.get("units").and_then(Value::as_str).map(display_units).unwrap_or(""),
            };
            json!({
                "id": format!("{group}/{name}"),
                "group": group,
                "name": name,
                "label": described.map(str::to_string).unwrap_or_else(|| humanise(name)),
                "value": value.clone().unwrap_or_else(|| ABSENT.to_string()),
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
    fn a_grounded_altimeter_does_not_read_below_the_launch_point() {
        struct Resting;
        impl Backend for Resting {
            fn get(&self, path: &str) -> String {
                match path {
                    "vehicle.altitudeRelative" => json!({ "kind": "fact", "name": "altitudeRelative", "shortDescription": "Alt (Rel)", "valueString": "-0.0", "units": "m" }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = instruments_view(&Resting, &["vehicle/altitudeRelative".to_string()]);
        assert_eq!(view["items"][0]["value"], "0.0", "Qt spells the fact and a vehicle sitting on the ground reports a hair under zero, so valueString arrives as -0.0 - and neither of us formats that string, so nothing stripped the sign before it reached an altimeter reading as below the launch point");
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
    fn a_reading_is_converted_now_rather_than_when_the_metadata_was_built() {
        struct Feet;
        impl Backend for Feet {
            fn get(&self, path: &str) -> String {
                match path {
                    "vehicle.altitudeRelative" => json!({ "kind": "fact", "name": "altitudeRelative", "shortDescription": "Alt (Rel)", "valueString": "25.0", "units": "m", "rawUnits": "vertical m", "rawValue": 25.0, "decimalPlaces": 1 }),
                    "vehicle.groundSpeed" => json!({ "kind": "fact", "name": "groundSpeed", "shortDescription": "Speed", "valueString": "8.0", "units": "m/s", "rawUnits": "m/s", "rawValue": 8.0, "decimalPlaces": 1 }),
                    "vehicle.heading" => json!({ "kind": "fact", "name": "heading", "shortDescription": "Heading", "valueString": "270", "units": "deg", "rawUnits": "deg", "rawValue": 270.0, "decimalPlaces": 0 }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "units" => json!({ "kind": "object", "appSettingsVerticalDistanceUnitsString": "ft", "appSettingsSpeedUnitsString": "m/s" }).to_string(),
                    _ => self.get(path),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, path: &str, args: &str) -> String {
                let metres = || serde_json::from_str::<Vec<f64>>(args).unwrap()[0];
                match path {
                    "units.metersToAppSettingsVerticalDistanceUnits" => json!({ "ok": true, "result": metres() * 3.2808399 }).to_string(),
                    _ => String::new(),
                }
            }
            fn watch(&self, _p: &[String]) {}
        }
        let read = |name: &str| {
            let view = instruments_view(&Feet, &["vehicle/altitudeRelative".to_string(), "vehicle/groundSpeed".to_string(), "vehicle/heading".to_string()]);
            let items = view["items"].as_array().unwrap().clone();
            items.into_iter().find(|i| i["name"] == name).unwrap()
        };
        let altitude = read("altitudeRelative");
        assert_eq!(altitude["value"], "82.0", "Fact::units is CONSTANT and its translator is bound once when the metadata is built, so valueString still said 25.0 metres after the operator chose feet");
        assert_eq!(altitude["units"], "ft");

        let speed = read("groundSpeed");
        assert_eq!(speed["value"], "8.0", "a speed is converted by the speed preference, which this operator left metric - applying the distance choice to every reading would have turned it into feet per second");
        assert_eq!(speed["units"], "m/s");

        let heading = read("heading");
        assert_eq!(heading["value"], "270", "a bearing has no unit preference at all, so it keeps the fact's own string");
        assert_eq!(heading["units"], "deg");
    }

    #[test]
    fn dependencies_are_the_selected_facts_not_their_groups() {
        assert_eq!(deps_for(&["gps/count".to_string(), "vehicle/heading".to_string(), "batteries.0/voltage".to_string()]), vec!["settings.unitsSettings.areaUnits", "settings.unitsSettings.horizontalDistanceUnits", "settings.unitsSettings.speedUnits", "settings.unitsSettings.verticalDistanceUnits", "vehicle.batteries.0.voltage", "vehicle.gps.count", "vehicle.heading", "vehicles.activeVehicleAvailable"]);
        assert_eq!(deps_for(&[]).len(), 11);
        assert!(deps_for(&[]).contains(&"vehicle.altitudeRelative".to_string()));
        assert_eq!(deps_for(&["/".to_string(), "".to_string(), "gps/".to_string()]), vec!["settings.unitsSettings.areaUnits", "settings.unitsSettings.horizontalDistanceUnits", "settings.unitsSettings.speedUnits", "settings.unitsSettings.verticalDistanceUnits", "vehicles.activeVehicleAvailable"], "an empty name never turns into a read of the whole vehicle");
        assert!(instruments_view(&Fake, &["vehicle/".to_string()])["items"].as_array().unwrap().is_empty());
    }
}
