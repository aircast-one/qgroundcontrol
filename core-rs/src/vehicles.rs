use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::read::{flag, integer, nested_coordinate, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicles.vehicles.count", "vehicle.id"];

const FIELDS: &str = "id,vehicleTypeString,firmwareTypeString,armed,flying,flightMode,coordinate";
const WATCHED_PER_VEHICLE: [&str; 4] = ["armed", "flying", "flightMode", "coordinate"];
static VEHICLES_SEEN: AtomicUsize = AtomicUsize::new(0);

pub fn deps() -> Vec<String> {
    DEPS.iter().map(|d| d.to_string()).chain(per_vehicle_paths()).collect()
}

fn per_vehicle_paths() -> Vec<String> {
    (0..VEHICLES_SEEN.load(Ordering::Relaxed))
        .flat_map(|index| {
            WATCHED_PER_VEHICLE
                .iter()
                .map(move |name| format!("vehicles.vehicles.{index}.{name}"))
                .chain(std::iter::once(format!("vehicles.vehicles.{index}.vehicleLinkManager.communicationLost")))
        })
        .collect()
}

pub fn vehicles_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let count = integer(&object(&backend.get("vehicles.vehicles.count")), "value").unwrap_or(0).max(0);
    VEHICLES_SEEN.store(usize::try_from(count).unwrap_or(0), Ordering::Relaxed);
    let active = integer(&object(&backend.get_fields("vehicle", "id")), "id");
    let listed: Vec<Value> = (0..count)
        .map(|index| {
            let read = object(&backend.get_fields(&format!("vehicles.vehicles.{index}"), FIELDS));
            let link = object(&backend.get_fields(&format!("vehicles.vehicles.{index}.vehicleLinkManager"), "primaryLinkName,communicationLost,communicationLostEnabled"));
            let id = integer(&read, "id");
            json!({
                "id": id,
                "name": name_of(&read, id),
                "type": text(&read, "vehicleTypeString"),
                "firmware": text(&read, "firmwareTypeString"),
                "link": text(&link, "primaryLinkName"),
                "contactLost": flag(&link, "communicationLostEnabled").then(|| flag(&link, "communicationLost")),
                "coordinate": nested_coordinate(&read).map(|(latitude, longitude)| json!({ "latitude": latitude, "longitude": longitude })),
                "active": id.is_some() && id == active,
                "armed": flag(&read, "armed"),
                "flying": flag(&read, "flying"),
                "flightMode": text(&read, "flightMode"),
            })
        })
        .collect();
    json!({
        "kind": "object",
        "class": "Vehicles",
        "count": listed.len(),
        "activeId": active,
        "ambiguous": listed.len() > 1,
        "vehicles": listed,
    })
}

fn name_of(read: &Value, id: Option<i64>) -> String {
    let kind = text(read, "vehicleTypeString");
    match (id, kind.is_empty()) {
        (Some(id), false) => format!("{kind} {id}"),
        (Some(id), true) => format!("Vehicle {id}"),
        (None, _) => "Vehicle".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static FLEET_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    fn fleet_guard() -> std::sync::MutexGuard<'static, ()> {
        FLEET_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    struct Fleet(Vec<Value>, Option<i64>);

    impl Backend for Fleet {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicles.vehicles.count" => json!({ "kind": "value", "value": self.0.len() }).to_string(),
                _ => String::new(),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            if path == "vehicle" {
                return match self.1 {
                    Some(id) => json!({ "kind": "object", "id": id }).to_string(),
                    None => json!({ "kind": "null" }).to_string(),
                };
            }
            if let Some(rest) = path.strip_prefix("vehicles.vehicles.") {
                let (index, tail) = rest.split_once('.').unwrap_or((rest, ""));
                if let Some(vehicle) = index.parse::<usize>().ok().and_then(|index| self.0.get(index)) {
                    return match tail {
                        "vehicleLinkManager" => json!({ "kind": "object", "primaryLinkName": vehicle["link"], "communicationLost": vehicle["quiet"], "communicationLostEnabled": vehicle.get("watching").cloned().unwrap_or(json!(true)) }).to_string(),
                        _ => vehicle.to_string(),
                    };
                }
            }
            String::new()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn aircraft(id: i64, kind: &str, link: &str) -> Value {
        json!({ "kind": "object", "id": id, "vehicleTypeString": kind, "firmwareTypeString": "ArduPilot", "armed": false, "flying": false, "flightMode": "Loiter", "link": link, "quiet": false,
                "coordinate": { "valid": true, "latitude": 47.397, "longitude": 8.546, "altitude": 12.0 } })
    }

    fn grounded(id: i64) -> Value {
        let mut vehicle = aircraft(id, "Fixed Wing", "Radio");
        vehicle["coordinate"] = Value::Null;
        vehicle["quiet"] = json!(true);
        vehicle
    }

    #[test]
    fn every_vehicle_carries_where_it_is_so_a_map_can_draw_more_than_the_active_one() {
        let _fleet = fleet_guard();
        let view = vehicles_view(&Fleet(vec![aircraft(1, "Multi-Rotor", "SITL"), grounded(2)], Some(1)), &[]);
        assert_eq!(view["vehicles"][0]["coordinate"]["latitude"], 47.397, "the inactive vehicle is the one a head could not draw before, so the position has to travel per vehicle rather than for the active one alone");
        assert_eq!(view["vehicles"][1]["coordinate"], Value::Null, "an invalid coordinate arrives from a nested read as a null rather than as valid:false, so keying on the flag alone would draw a marker at nowhere");
        assert_eq!(view["vehicles"][1]["contactLost"], true, "a position latches for the best part of a minute after the vehicle stops talking, so a head needs to know the marker is a memory");
        assert_eq!(view["vehicles"][0]["contactLost"], false);

        let mut unwatched = grounded(3);
        unwatched["watching"] = json!(false);
        let view = vehicles_view(&Fleet(vec![aircraft(1, "Multi-Rotor", "SITL"), unwatched], Some(1)), &[]);
        assert_eq!(view["vehicles"][1]["contactLost"], Value::Null, "with the watch off the flag stays false however long the vehicle has been silent, so serving it raw would call a stale marker live - view.vehicleLinks answers this with a null and a head reads that as unwatched");
    }

    #[test]
    fn one_vehicle_is_never_ambiguous() {
        let _fleet = fleet_guard();
        let view = vehicles_view(&Fleet(vec![aircraft(1, "Multi-Rotor", "SITL")], Some(1)), &[]);
        assert_eq!(view["count"], 1);
        assert_eq!(view["ambiguous"], false);
        assert_eq!(view["vehicles"][0]["active"], true);
        assert_eq!(view["vehicles"][0]["name"], "Multi-Rotor 1");
        assert_eq!(view["vehicles"][0]["link"], "SITL");
    }

    #[test]
    fn two_vehicles_are_named_and_one_is_marked() {
        let _fleet = fleet_guard();
        let fleet = Fleet(vec![aircraft(1, "Multi-Rotor", "Radio"), aircraft(2, "Fixed Wing", "UDP")], Some(2));
        let view = vehicles_view(&fleet, &[]);
        assert_eq!(view["count"], 2);
        assert_eq!(view["ambiguous"], true, "an operator with two aircraft up cannot tell which one an arm reaches unless the head says so");
        assert_eq!(view["activeId"], 2);
        assert_eq!(view["vehicles"][0]["active"], false);
        assert_eq!(view["vehicles"][1]["active"], true);
        assert_eq!(view["vehicles"][1]["name"], "Fixed Wing 2");
        assert_eq!(view["vehicles"][0]["link"], "Radio", "which link it arrived on is how an operator tells two identical airframes apart");
    }

    #[test]
    fn no_vehicle_is_an_empty_fleet_rather_than_an_absent_answer() {
        let _fleet = fleet_guard();
        let view = vehicles_view(&Fleet(Vec::new(), None), &[]);
        assert_eq!(view["count"], 0);
        assert_eq!(view["ambiguous"], false);
        assert_eq!(view["activeId"], Value::Null);
        assert!(view["vehicles"].as_array().unwrap().is_empty());
    }

    #[test]
    fn a_vehicle_with_no_type_is_still_named_by_its_number() {
        let _fleet = fleet_guard();
        let view = vehicles_view(&Fleet(vec![json!({ "kind": "object", "id": 7 })], Some(7)), &[]);
        assert_eq!(view["vehicles"][0]["name"], "Vehicle 7", "an aircraft that has not said what it is still has to be tellable from the other one");
        assert_eq!(view["vehicles"][0]["active"], true);
    }

    #[test]
    fn an_active_vehicle_the_list_does_not_hold_marks_nothing() {
        let _fleet = fleet_guard();
        let view = vehicles_view(&Fleet(vec![aircraft(1, "Multi-Rotor", "Radio")], Some(9)), &[]);
        assert!(view["vehicles"].as_array().unwrap().iter().all(|v| v["active"] == false), "marking the wrong row active is worse than marking none");
        assert_eq!(view["activeId"], 9);
    }

    #[test]
    fn a_second_vehicle_is_watched_field_by_field_and_not_only_counted() {
        let _fleet = fleet_guard();
        vehicles_view(&Fleet(vec![], None), &[]);
        assert!(deps().iter().all(|d| !d.starts_with("vehicles.vehicles.0.")), "with no vehicle there is nothing to watch, and a path that resolves to nothing binds to no signal and is silently downgraded to the idle poll");

        vehicles_view(&Fleet(vec![aircraft(1, "Multi-Rotor", "SITL"), grounded(2)], Some(1)), &[]);
        let after = deps();
        ["armed", "flying", "flightMode", "coordinate"].iter().for_each(|name| {
            assert!(after.contains(&format!("vehicles.vehicles.1.{name}")), "{name} on the second vehicle is served by this view, so nothing recomputes it unless it is watched");
        });
        assert!(after.contains(&"vehicles.vehicles.1.vehicleLinkManager.communicationLost".to_string()), "contactLost is the field a head draws a stale marker from, and it is the one a count-only watch misses for the longest");
        assert!(after.contains(&"vehicles.vehicles.count".to_string()), "the count stays watched, because it is what makes the list re-derive when the fleet changes");
    }

    #[test]
    fn a_seventeenth_vehicle_is_watched_like_the_rest() {
        let _fleet = fleet_guard();
        let fleet: Vec<Value> = (1..=17).map(|id| aircraft(id, "Multi-Rotor", "SITL")).collect();
        vehicles_view(&Fleet(fleet, Some(1)), &[]);
        assert!(deps().contains(&"vehicles.vehicles.16.armed".to_string()), "a cap on how many vehicles are watched stops at a number nothing reports, so the vehicles past it are stale with no tell - the count QGC gives is already bounded by what has connected");
    }
}
