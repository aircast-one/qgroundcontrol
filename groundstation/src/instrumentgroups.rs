use serde_json::{Value, json};

use crate::label::humanise;
use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.id", "vehicle.batteries.count"];

fn facts_of(group: &Value, prefix: &str) -> Vec<Value> {
    group
        .get("facts")
        .and_then(Value::as_array)
        .map(|facts| {
            facts
                .iter()
                .filter_map(|fact| {
                    let property = fact.get("property").and_then(Value::as_str)?;
                    let named = fact.get("name").and_then(Value::as_str).filter(|n| !n.is_empty()).unwrap_or(property);
                    let described = fact.get("shortDescription").and_then(Value::as_str).filter(|d| !d.is_empty());
                    Some(json!({
                        "name": property,
                        "selection": format!("{prefix}{property}"),
                        "label": described.map(str::to_string).unwrap_or_else(|| humanise(named)),
                    }))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn group_facts(backend: &dyn Backend, group: &str) -> Vec<Value> {
    facts_of(&object(&backend.get(&format!("vehicle.{group}"))), &format!("{group}/"))
}

fn catalogued() -> Option<(Vec<Value>, Value)> {
    let sub = crate::vehiclefacade::switched_on().then(|| crate::hub::lock().active().map(crate::hub::Vehicle::is_ardusub)).flatten()?;
    let (groups, vehicle) = crate::vehiclefact::instrument_catalogue();
    let listed = groups
        .into_iter()
        .filter(|(group, _)| sub || *group != crate::vehiclefact::SUB_INFO_GROUP)
        .filter_map(|(group, listing)| {
            let facts = facts_of(&listing, &format!("{group}/"));
            (!facts.is_empty()).then(|| json!({ "group": group, "title": humanise(group), "facts": facts }))
        })
        .collect();
    Some((listed, vehicle))
}

pub fn instrument_groups_view(backend: &dyn Backend, _args: &[String]) -> Value {
    if let Some((groups, vehicle)) = catalogued() {
        return instrument_groups(backend, true, groups, &vehicle);
    }
    let vehicle = object(&backend.get("vehicle"));
    let available = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let groups: Vec<Value> = vehicle
        .get("children")
        .and_then(Value::as_array)
        .map(|children| {
            children
                .iter()
                .filter_map(Value::as_str)
                .filter(|group| *group != "vehicle")
                .filter_map(|group| {
                    let facts = group_facts(backend, group);
                    (!facts.is_empty()).then(|| json!({ "group": group, "title": humanise(group), "facts": facts }))
                })
                .collect()
        })
        .unwrap_or_default();
    instrument_groups(backend, available, groups, &vehicle)
}

fn instrument_groups(backend: &dyn Backend, available: bool, groups: Vec<Value>, vehicle: &Value) -> Value {
    let packs = crate::read::value_number(&backend.get("vehicle.batteries.count")).unwrap_or(0.0).max(0.0) as i64;
    // One group per battery pack, in the same shape as groups so a head decodes both one way. The
    // macOS head named these itself and read each pack raw; the ids stay batteries.<n>, which
    // view.instruments resolves to vehicle.batteries.<n>.<fact>, and the title numbers from 1.
    let pack_groups: Vec<Value> = (0..packs)
        .filter_map(|pack| {
            let group = format!("batteries.{pack}");
            let facts = facts_of(&object(&backend.get(&format!("vehicle.{group}"))), &format!("{group}/"));
            (!facts.is_empty()).then(|| json!({ "group": group, "title": format!("Battery {}", pack + 1), "facts": facts }))
        })
        .collect();
    json!({
        "kind": "object",
        "class": "InstrumentGroups",
        "available": available,
        "groups": groups,
        "packs": packs,
        "packGroups": pack_groups,
        // The vehicle's own readings stay out of groups, where the macOS head would meet them twice.
        // The Android head read the whole vehicle object raw to list them; they are served here
        // under a bare selection, which view.instruments resolves against the vehicle by default.
        "vehicleFacts": match available {
            true => facts_of(vehicle, ""),
            false => Vec::new(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fleet;
    impl Backend for Fleet {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicle.batteries.count" => json!({ "kind": "value", "value": 2 }).to_string(),
                "vehicle.batteries.0" => json!({ "kind": "object", "facts": [{ "property": "voltage", "name": "voltage", "shortDescription": "Voltage" }] }).to_string(),
                "vehicle.gps" => json!({ "kind": "object", "facts": [
                    { "property": "lock", "name": "lock", "shortDescription": "GPS Lock" },
                    { "property": "count", "name": "count", "shortDescription": "" },
                ] }).to_string(),
                "vehicle.escStatus" => json!({ "kind": "object", "facts": [
                    { "property": "rpmFirst", "name": "rpm1", "shortDescription": "" },
                ] }).to_string(),
                "vehicle.parameterManager" => json!({ "kind": "object", "facts": [] }).to_string(),
                "vehicle" => json!({ "kind": "object", "id": 1, "children": ["vehicle", "gps", "escStatus", "parameterManager"], "facts": [
                    { "property": "altitudeRelative", "name": "altitudeRelative", "shortDescription": "Rel. Alt." },
                    { "property": "groundSpeed", "name": "groundSpeed", "shortDescription": "" },
                ] }).to_string(),
                "vehicle.vehicle" => json!({ "kind": "object", "facts": [{ "property": "altitude", "name": "altitude", "shortDescription": "" }] }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "vehicle" => json!({ "kind": "object", "id": 1, "children": ["vehicle", "gps", "escStatus", "parameterManager"] }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_picker_gets_the_key_that_round_trips_and_the_label_that_reads() {
        let view = instrument_groups_view(&Fleet, &[]);
        let groups = view["groups"].as_array().unwrap();
        let named: Vec<&str> = groups.iter().filter_map(|g| g["group"].as_str()).collect();

        assert_eq!(named, ["gps", "escStatus"], "parameterManager is one of vehicle's children and carries no facts, and the group literally named 'vehicle' is one the macOS head already builds itself under the id \"\" - serving it too would put two Vehicle groups in that picker over the same readings, and every default it has stored is in the other id's form");
        let served = groups[0]["facts"][0]["selection"].as_str().unwrap();
        assert_eq!(
            crate::instruments::fact_path_of(served),
            "vehicle.gps.lock",
            "the selection is spelled here and parsed in instruments, and nothing but this stops the two drifting - a separator changed on one side would leave every picked reading resolving to a path the bridge does not have"
        );
        assert_eq!(groups[0]["facts"][0]["selection"], "gps/lock", "a head handing back a bare name gets it split against the vehicle group by default, so gps/lon spelled as lon resolves to vehicle.lon and answers noSuchFact - serving the whole selection removes the guess rather than documenting it");

        let esc = &groups[1]["facts"][0];
        assert_eq!(esc["name"], "rpmFirst", "the head hands this back as view.instruments(escStatus/<name>), which resolves it as a PATH, so it has to be the Q_PROPERTY and not the fact's own name");
        assert_eq!(esc["label"], "Rpm 1", "and the label humanises the FACT name, because an operator numbers motors 1 to 4 rather than First to Fourth");

        assert_eq!(groups[0]["facts"][0]["label"], "GPS Lock", "shortDescription wins whenever the fact has one");
        assert_eq!(groups[0]["facts"][1]["label"], "Count", "and humanise fills in when it does not");
        assert_eq!(groups[0]["title"], "GPS", "the acronym table earns its keep on the title too, so the picker heading is not Gps");
        assert_eq!(view["packs"], json!(2), "the head names its own battery groups from this count, so the core does not invent their ids");

        let packs = view["packGroups"].as_array().unwrap();
        assert_eq!(packs.len(), 1, "a pack that answers no facts is left out, as an empty child group is");
        assert_eq!((&packs[0]["group"], &packs[0]["title"]), (&json!("batteries.0"), &json!("Battery 1")));
        let voltage = packs[0]["facts"][0]["selection"].as_str().unwrap();
        assert_eq!(crate::instruments::fact_path_of(voltage), "vehicle.batteries.0.voltage", "the selection resolves to the pack's own fact");

        let own = view["vehicleFacts"].as_array().unwrap();
        assert_eq!((&own[0]["name"], &own[0]["label"], &own[0]["selection"]), (&json!("altitudeRelative"), &json!("Rel. Alt."), &json!("altitudeRelative")));
        assert_eq!(own[1]["label"], "Ground Speed", "a fact with no description is humanised like every other group's");
        assert_eq!(crate::instruments::fact_path_of("altitudeRelative"), "vehicle.altitudeRelative", "a bare selection is the vehicle's own reading");
    }
}
