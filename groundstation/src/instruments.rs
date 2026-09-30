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

pub fn fact_path_of(selection: &str) -> String {
    let (group, name) = split_selection(selection);
    fact_path(&group, &name)
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
            let resolves = fact.get("found").and_then(Value::as_bool) != Some(false);
            let described = fact.get("shortDescription").and_then(Value::as_str).filter(|d| !d.is_empty());
            let fresh = converted(backend, &fact);
            // Qt prints "--.--" for a numeric fact whose cooked value is NaN - Fact::_variantToString
            // does it for valueTypeFloat and valueTypeDouble - and that is a value, not an absence.
            // JSON cannot carry NaN, so the bridge sends value: null while valueString still holds
            // the placeholder. Taking the string there put "--.-- ft" on the flight row with
            // missing: false, which tells every head it is a real reading. distanceToHome on a
            // vehicle that has sent no HOME_POSITION is the case Android hit.
            let unset = matches!(fact.get("value"), Some(Value::Null));
            let held = (!unset).then(|| crate::read::shown_text(&fact)).flatten();
            let value = fresh.as_ref().map(|(shown, _)| shown.clone()).or_else(|| held.map(crate::read::settled));
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
                "missingReason": match (value.is_none(), resolves) {
                    (false, _) => Value::Null,
                    (true, false) => json!("noSuchFact"),
                    (true, true) => json!("notReported"),
                },
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
    fn a_placeholder_is_an_absent_reading_and_not_a_value() {
        struct Homeless;
        impl Backend for Homeless {
            fn get(&self, path: &str) -> String {
                match path {
                    "vehicle.distanceToHome" => json!({ "kind": "fact", "name": "distanceToHome", "shortDescription": "Distance to Home", "value": null, "valueString": "--.--", "units": "ft", "rawUnits": "m" }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = instruments_view(&Homeless, &["vehicle/distanceToHome".to_string()]);
        let row = &view["items"][0];
        assert_eq!(row["missing"], true, "Qt writes --.-- when a numeric fact is NaN, and a vehicle that has sent no HOME_POSITION cannot compute a distance to it - serving the placeholder drew \"--.-- ft\" on the flight row as a reading");
        assert_eq!(row["missingReason"], "notReported", "the fact resolves and the vehicle will report it once home is set, so this is not a fact that does not exist");
        assert_eq!(row["value"], "\u{2014}", "the head gets the absent marker it already draws correctly");
        assert_eq!(row["units"], "", "and no unit, because a unit beside an absence is what made the placeholder look measured");
        assert_eq!(view["available"], false);
    }

    #[test]
    fn an_instrument_showing_an_enum_names_the_choice_and_not_its_raw_number() {
        struct Lettered;
        impl Backend for Lettered {
            fn get(&self, path: &str) -> String {
                match path {
                    "vehicle.flightMode" => json!({ "kind": "fact", "name": "flightMode", "shortDescription": "Flight Mode", "value": 6, "valueString": "6", "enumOrValueString": "Return", "enumStrings": ["Stabilize", "Return"], "enumValues": [0, 6], "enumIndex": 1, "units": "" }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = instruments_view(&Lettered, &["vehicle/flightMode".to_string()]);
        assert_eq!(view["items"][0]["value"], "Return", "valueString is the raw enum value and enumValues here are 0,6 rather than positional, so an instrument reading it shows 6 - a number that is not an index into anything the operator can see");
        assert_eq!(view["items"][0]["missing"], false);
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

    #[test]
    fn a_selection_stored_in_the_old_dotted_spelling_reaches_the_same_fact() {
        assert_eq!(fact_path_of("gps/lon"), "vehicle.gps.lon");
        assert_eq!(fact_path_of("gps.lon"), "vehicle.gps.lon", "a head that stored a fact as group.name before the slash form existed still resolves - the bare-name branch prefixes vehicle. and the dot the old format used is the dot the concatenation would have inserted. That is a coincidence of two independent decisions, not a design, and tightening this branch to reject a dotted name would silently invalidate every choice already stored on a device");
        assert_eq!(fact_path_of("altitudeRelative"), "vehicle.altitudeRelative");
        assert_eq!(fact_path_of("batteries.0/voltage"), "vehicle.batteries.0.voltage");
    }
}
