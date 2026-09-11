use serde_json::{Value, json};

use crate::read::{flag, integer, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicles.vehicles.count", "vehicle.id"];

const FIELDS: &str = "id,vehicleTypeString,firmwareTypeString,armed,flying,flightMode,coordinate";

pub fn vehicles_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let count = integer(&object(&backend.get("vehicles.vehicles.count")), "value").unwrap_or(0).max(0);
    let active = integer(&object(&backend.get_fields("vehicle", "id")), "id");
    let listed: Vec<Value> = (0..count)
        .map(|index| {
            let read = object(&backend.get_fields(&format!("vehicles.vehicles.{index}"), FIELDS));
            let id = integer(&read, "id");
            json!({
                "id": id,
                "name": name_of(&read, id),
                "type": text(&read, "vehicleTypeString"),
                "firmware": text(&read, "firmwareTypeString"),
                "link": text(&object(&backend.get_fields(&format!("vehicles.vehicles.{index}.vehicleLinkManager"), "primaryLinkName")), "primaryLinkName"),
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
        // A head that draws one vehicle and never says which is safe with one aircraft and unsafe
        // with two, because an arm or a return reaches whichever is active and nothing says which.
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
                        "vehicleLinkManager" => json!({ "kind": "object", "primaryLinkName": vehicle["link"] }).to_string(),
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
        json!({ "kind": "object", "id": id, "vehicleTypeString": kind, "firmwareTypeString": "ArduPilot", "armed": false, "flying": false, "flightMode": "Loiter", "link": link })
    }

    #[test]
    fn one_vehicle_is_never_ambiguous() {
        let view = vehicles_view(&Fleet(vec![aircraft(1, "Multi-Rotor", "SITL")], Some(1)), &[]);
        assert_eq!(view["count"], 1);
        assert_eq!(view["ambiguous"], false);
        assert_eq!(view["vehicles"][0]["active"], true);
        assert_eq!(view["vehicles"][0]["name"], "Multi-Rotor 1");
        assert_eq!(view["vehicles"][0]["link"], "SITL");
    }

    #[test]
    fn two_vehicles_are_named_and_one_is_marked() {
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
        let view = vehicles_view(&Fleet(Vec::new(), None), &[]);
        assert_eq!(view["count"], 0);
        assert_eq!(view["ambiguous"], false);
        assert_eq!(view["activeId"], Value::Null);
        assert!(view["vehicles"].as_array().unwrap().is_empty());
    }

    #[test]
    fn a_vehicle_with_no_type_is_still_named_by_its_number() {
        let view = vehicles_view(&Fleet(vec![json!({ "kind": "object", "id": 7 })], Some(7)), &[]);
        assert_eq!(view["vehicles"][0]["name"], "Vehicle 7", "an aircraft that has not said what it is still has to be tellable from the other one");
        assert_eq!(view["vehicles"][0]["active"], true);
    }

    #[test]
    fn an_active_vehicle_the_list_does_not_hold_marks_nothing() {
        let view = vehicles_view(&Fleet(vec![aircraft(1, "Multi-Rotor", "Radio")], Some(9)), &[]);
        assert!(view["vehicles"].as_array().unwrap().iter().all(|v| v["active"] == false), "marking the wrong row active is worse than marking none");
        assert_eq!(view["activeId"], 9);
    }
}
