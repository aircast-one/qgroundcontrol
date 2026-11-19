use serde_json::{Value, json};

use crate::label::humanise;
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.id",
    "vehicle.batteries.count",
    crate::gimbal::GIMBAL_CHANGED,
];

fn facts_of(group: &Value, prefix: &str) -> Vec<Value> {
    let listed: Vec<&Value> = group
        .get("facts")
        .and_then(Value::as_array)
        .map(|facts| facts.iter().collect())
        .unwrap_or_default();
    let has_text_twin = |name: &str| {
        listed
            .iter()
            .any(|other| property_of(other) == Some(&format!("{name}{TEXT_TWIN}")))
    };
    listed
        .iter()
        .filter_map(|fact| {
            let property = property_of(fact)?;
            let named = fact
                .get("name")
                .and_then(Value::as_str)
                .filter(|n| !n.is_empty())
                .unwrap_or(property);
            let described = fact
                .get("shortDescription")
                .and_then(Value::as_str)
                .filter(|d| !d.is_empty() && *d != named);
            (!has_text_twin(property)).then(|| {
                json!({
                    "name": property,
                    "selection": format!("{prefix}{property}"),
                    "label": described.map_or_else(|| undescribed_label(property, named), crate::label::capitalised),
                })
            })
        })
        .collect()
}

const TEXT_TWIN: &str = "Str";

const UNDESCRIBED_LABELS: &[(&str, &str)] = &[
    ("airSpeedSetpoint", "Airspeed target"),
    ("altitudeTuning", "Altitude change since connecting"),
    ("altitudeTuningSetpoint", "Altitude target since connecting"),
    ("xTrackError", "Distance off the track"),
    ("rangeFinderDist", "Rangefinder distance"),
    ("isStale", "GPS data out of date"),
    ("managerCompid", "Gimbal manager ID"),
];

fn undescribed_label(property: &str, named: &str) -> String {
    UNDESCRIBED_LABELS
        .iter()
        .find(|(name, _)| *name == property)
        .map_or_else(|| sentence(named), |(_, label)| label.to_string())
}

fn property_of(fact: &Value) -> Option<&str> {
    fact.get("property").and_then(Value::as_str)
}

const GROUP_TITLES: &[(&str, &str)] = &[
    ("aircastLink", "Aircast link"),
    ("distanceSensors", "Distance sensors"),
    ("efi", "Engine"),
    ("gps", "GPS"),
    ("gps2", "GPS 2"),
    ("gpsAggregate", "GPS integrity"),
    ("radioStatus", "Telemetry radio"),
    ("rpm", "RPM"),
];

fn sentence(identifier: &str) -> String {
    humanise(identifier)
        .split(' ')
        .enumerate()
        .map(
            |(at, word)| match at == 0 || word.chars().all(|c| !c.is_lowercase()) {
                true => word.to_string(),
                false => word.to_lowercase(),
            },
        )
        .collect::<Vec<_>>()
        .join(" ")
}

fn title(group: &str) -> String {
    GROUP_TITLES
        .iter()
        .find(|(name, _)| *name == group)
        .map_or_else(|| sentence(group), |(_, title)| title.to_string())
}

fn gimbal_title(at: usize, count: usize) -> String {
    match count {
        1 => "Gimbal".to_string(),
        _ => format!("Gimbal {}", at + 1),
    }
}

fn group_facts(backend: &dyn Backend, group: &str) -> Vec<Value> {
    facts_of(
        &backend.value(&format!("vehicle.{group}")),
        &format!("{group}/"),
    )
}

fn catalogued() -> Option<(Vec<Value>, Value)> {
    let (sub, fact_groups) = crate::vehiclefacade::switched_on()
        .then(|| {
            crate::hub::lock()
                .active()
                .map(|v| (v.is_ardusub(), v.gimbals.fact_groups()))
        })
        .flatten()?;
    let (groups, vehicle) = crate::vehiclefact::instrument_catalogue(sub);
    let count = fact_groups.len();
    let gimbals: Vec<(String, String, Value)> = fact_groups
        .into_iter()
        .enumerate()
        .map(|(at, (pair, _))| {
            (
                crate::vehiclefact::gimbal_group_name(pair),
                gimbal_title(at, count),
                crate::vehiclefact::gimbal_listing(),
            )
        })
        .collect();
    let listed = groups
        .into_iter()
        .map(|(group, listing)| (group.to_string(), title(group), listing))
        .chain(gimbals)
        .filter(|(group, _, _)| sub || group != crate::vehiclefact::SUB_INFO_GROUP)
        .filter(|(group, _, _)| group != "orbitMapCircle")
        .filter_map(|(group, title, listing)| {
            let facts = facts_of(&listing, &format!("{group}/"));
            (!facts.is_empty()).then(|| json!({ "group": group, "title": title, "facts": facts }))
        })
        .collect();
    Some((listed, vehicle))
}

pub fn instrument_groups_view(backend: &dyn Backend, _args: &[String]) -> Value {
    if let Some((groups, vehicle)) = catalogued() {
        return instrument_groups(backend, true, groups, &vehicle);
    }
    let vehicle = backend.value("vehicle");
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
                    (!facts.is_empty())
                        .then(|| json!({ "group": group, "title": title(group), "facts": facts }))
                })
                .collect()
        })
        .unwrap_or_default();
    instrument_groups(backend, available, groups, &vehicle)
}

fn instrument_groups(
    backend: &dyn Backend,
    available: bool,
    groups: Vec<Value>,
    vehicle: &Value,
) -> Value {
    let packs = crate::read::value_number(&backend.value("vehicle.batteries.count"))
        .unwrap_or(0.0)
        .max(0.0) as i64;
    let pack_groups: Vec<Value> = (0..packs)
        .filter_map(|pack| {
            let group = format!("batteries.{pack}");
            let facts = facts_of(
                &backend.value(&format!("vehicle.{group}")),
                &format!("{group}/"),
            );
            (!facts.is_empty())
                .then(|| json!({ "group": group, "title": format!("Battery {}", pack + 1), "facts": facts }))
        })
        .collect();
    json!({
        "kind": "object",
        "class": "InstrumentGroups",
        "available": available,
        "groups": groups.into_iter().map(|group| (group["title"].as_str().unwrap_or_default().to_string(), group)).collect::<std::collections::BTreeMap<_, _>>().into_values().collect::<Vec<_>>(),
        "packs": packs,
        "packGroups": pack_groups,
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
                "vehicle.batteries.0" => json!({ "kind": "object", "facts": [
                    { "property": "voltage", "name": "voltage", "shortDescription": "Voltage" },
                    { "property": "timeRemaining", "name": "timeRemaining", "shortDescription": "Time Remaining" },
                    { "property": "timeRemainingStr", "name": "timeRemainingStr", "shortDescription": "Time Remaining" },
                ] }).to_string(),
                "vehicle.batteries.0.id" => json!({ "kind": "fact", "value": 3 }).to_string(),
                "vehicle.gps" => json!({ "kind": "object", "facts": [
                    { "property": "lock", "name": "lock", "shortDescription": "GPS Lock" },
                    { "property": "count", "name": "count", "shortDescription": "" },
                    { "property": "runtime", "name": "runtime", "shortDescription": "runtime" },
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
        fn set(&self, _p: &str, _v: &str) -> String {
            String::new()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            String::new()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_picker_gets_the_key_that_round_trips_and_the_label_that_reads() {
        let view = instrument_groups_view(&Fleet, &[]);
        let groups = view["groups"].as_array().unwrap();
        let named: Vec<&str> = groups.iter().filter_map(|g| g["group"].as_str()).collect();
        let (esc_group, gps_group) = (&groups[0], &groups[1]);

        assert_eq!(
            named,
            ["escStatus", "gps"],
            "groups are listed in FactGroup::factGroupNames (QMap) order; parameterManager is one of vehicle's children and carries no facts, and the group literally named 'vehicle' is one the macOS head already builds itself under the id \"\" - serving it too would put two Vehicle groups in that picker over the same readings, and every default it has stored is in the other id's form"
        );
        let served = gps_group["facts"][0]["selection"].as_str().unwrap();
        assert_eq!(
            crate::instruments::fact_path_of(served),
            "vehicle.gps.lock",
            "the selection is spelled here and parsed in instruments, and nothing but this stops the two drifting - a separator changed on one side would leave every picked reading resolving to a path the bridge does not have"
        );
        assert_eq!(
            gps_group["facts"][0]["selection"], "gps/lock",
            "a head handing back a bare name gets it split against the vehicle group by default, so gps/lon spelled as lon resolves to vehicle.lon and answers noSuchFact - serving the whole selection removes the guess rather than documenting it"
        );

        let esc = &esc_group["facts"][0];
        assert_eq!(
            esc["name"], "rpmFirst",
            "the head hands this back as view.instruments(escStatus/<name>), which resolves it as a PATH, so it has to be the Q_PROPERTY and not the fact's own name"
        );
        assert_eq!(
            esc["label"], "Rpm 1",
            "a fact with no description is listed by its FACT name, spelt out as words rather than the identifier"
        );

        assert_eq!(
            gps_group["facts"][0]["label"], "GPS Lock",
            "shortDescription wins whenever the fact has one"
        );
        assert_eq!(
            gps_group["facts"][1]["label"], "Count",
            "and the fact name fills in when it does not"
        );
        assert_eq!(
            gps_group["facts"][2]["label"], "Runtime",
            "a description that only repeats the identifier reads like one with none"
        );
        assert_eq!(
            gps_group["title"], "GPS",
            "the picker heads each group with words a pilot reads, not the FactGroup name"
        );
        assert_eq!(
            view["packs"],
            json!(2),
            "the head names its own battery groups from this count, so the core does not invent their ids"
        );

        let packs = view["packGroups"].as_array().unwrap();
        assert_eq!(
            packs.len(),
            1,
            "a pack that answers no facts is left out, as an empty child group is"
        );
        assert_eq!(
            (&packs[0]["group"], &packs[0]["title"]),
            (&json!("batteries.0"), &json!("Battery 1")),
            "the pack is named as the battery sheet numbers it, from one"
        );
        assert_eq!(
            packs[0]["facts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|fact| fact["name"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["voltage", "timeRemainingStr"],
            "a reading with a formatted Str twin is offered once, as the twin, instead of two Time remaining rows"
        );
        let voltage = packs[0]["facts"][0]["selection"].as_str().unwrap();
        assert_eq!(
            crate::instruments::fact_path_of(voltage),
            "vehicle.batteries.0.voltage",
            "the selection resolves to the pack's own fact"
        );

        let own = view["vehicleFacts"].as_array().unwrap();
        assert_eq!(
            (&own[0]["name"], &own[0]["label"], &own[0]["selection"]),
            (
                &json!("altitudeRelative"),
                &json!("Rel. Alt."),
                &json!("altitudeRelative")
            )
        );
        assert_eq!(
            own[1]["label"], "Ground speed",
            "a fact with no description reads its name like every other group's"
        );
        assert_eq!(
            crate::instruments::fact_path_of("altitudeRelative"),
            "vehicle.altitudeRelative",
            "a bare selection is the vehicle's own reading"
        );
    }

    #[test]
    fn every_reading_in_the_chooser_reads_as_words() {
        let (groups, vehicle) = crate::vehiclefact::instrument_catalogue(false);
        let labels: Vec<String> = groups
            .iter()
            .map(|(group, facts)| facts_of(facts, &format!("{group}/")))
            .chain(std::iter::once(facts_of(&vehicle, "")))
            .flatten()
            .filter_map(|fact| fact["label"].as_str().map(str::to_string))
            .collect();
        let jargon = [
            "Alt (",
            "Esimate",
            "mainfold",
            "BarometricPressure",
            "Vibe ",
            "Spd",
            "Next WP",
            "Sat Count",
            "Throttle %",
            "Imu ",
            "Hobbs",
            "X track",
            "Range finder dist",
            "Is stale",
            "Manager compid",
            "Altitude tuning",
            "Air Speed",
        ];
        let found: Vec<&String> = labels
            .iter()
            .filter(|label| jargon.iter().any(|word| label.contains(word)))
            .collect();
        assert!(
            found.is_empty(),
            "jargon left in Choose readings: {found:?}"
        );
        assert!(labels.contains(&"Altitude Above Home".to_string()));
        assert!(labels.contains(&"Distance off the track".to_string()));
    }

    #[test]
    fn group_titles_read_as_words_and_a_lone_gimbal_is_just_gimbal() {
        assert_eq!(
            [
                "estimatorStatus",
                "localPositionSetpoint",
                "gpsAggregate",
                "efi",
                "wind"
            ]
            .map(title),
            [
                "Estimator status",
                "Local position setpoint",
                "GPS integrity",
                "Engine",
                "Wind"
            ]
        );
        assert_eq!(
            (gimbal_title(0, 1), gimbal_title(1, 2)),
            ("Gimbal".to_string(), "Gimbal 2".to_string()),
            "gimbal1154 is a manager and device id glued together, not a name"
        );
    }
}
