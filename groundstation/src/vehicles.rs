use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::read::{flag, integer, nested_coordinate, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicles.vehicles.count", "vehicles.selectedVehicles.count", "vehicle.id"];

const FIELDS: &str = "id,vehicleTypeString,firmwareTypeString,armed,flying,flightMode,missionFlightMode,coordinate,homePosition,fixedWing,vtol,airship,flightModes,flightModeSetAvailable";
const WATCHED_PER_VEHICLE: [&str; 9] = ["armed", "flying", "flightMode", "coordinate", "homePosition", "heading", "altitudeRelative", "groundSpeed", "airSpeed"];
static VEHICLES_SEEN: AtomicUsize = AtomicUsize::new(0);

fn proximity(backend: &dyn Backend, index: i64) -> Value {
    let group = object(&backend.get(&format!("vehicles.vehicles.{index}.distanceSensors")));
    match flag(&group, "telemetryAvailable") {
        true => json!({ "shown": true, "maxMeters": crate::proximity::max_meters(&group), "sectors": crate::proximity::sectors(&group) }),
        false => Value::Null,
    }
}

fn shown_fact(backend: &dyn Backend, index: i64, name: &str) -> Option<String> {
    let fact = object(&backend.get(&format!("vehicles.vehicles.{index}.{name}")));
    let spelled = fact.get("valueString").and_then(Value::as_str).filter(|s| !s.trim().is_empty() && !s.starts_with("--"))?;
    let units = fact.get("units").and_then(Value::as_str).unwrap_or_default();
    Some([spelled, crate::instruments::display_units(units)].iter().filter(|part| !part.is_empty()).copied().collect::<Vec<_>>().join(" "))
}

pub fn telemetry(read: &Value, value_of: &dyn Fn(&str) -> Option<String>) -> Vec<Value> {
    let forward_flight = ["fixedWing", "vtol", "airship"].iter().any(|class| flag(read, class));
    let speed = if forward_flight { ("AirSpd", "airSpeed") } else { ("Ground Speed", "groundSpeed") };
    [("Alt (Rel)", "altitudeRelative"), speed]
        .iter()
        .filter_map(|(label, name)| value_of(name).map(|value| json!({ "label": label, "value": value })))
        .collect()
}

fn heading(backend: &dyn Backend, index: i64) -> Option<f64> {
    let fact = object(&backend.get(&format!("vehicles.vehicles.{index}.heading")));
    fact.get("rawValue").or_else(|| fact.get("value")).and_then(Value::as_f64).filter(|h| h.is_finite()).map(|h| h.rem_euclid(360.0))
}

pub fn deps() -> Vec<String> {
    DEPS.iter().map(|d| d.to_string()).chain(per_vehicle_paths()).collect()
}

fn per_vehicle_paths() -> Vec<String> {
    (0..VEHICLES_SEEN.load(Ordering::Relaxed))
        .flat_map(|index| {
            WATCHED_PER_VEHICLE
                .iter()
                .map(move |name| format!("vehicles.vehicles.{index}.{name}"))
                .chain(["communicationLost", "communicationLostEnabled"].map(move |flag| format!("vehicles.vehicles.{index}.vehicleLinkManager.{flag}")))
                .chain(crate::proximity::DEPS.iter().filter_map(move |dep| dep.strip_prefix("vehicle.").map(|fact| format!("vehicles.vehicles.{index}.{fact}"))))
        })
        .collect()
}

const MV_ACTIONS: [(&str, &str, &str, &str); 4] = [
    ("mvArm", "Arm", "Arm (MV)", "Arm selected vehicles."),
    ("mvDisarm", "Disarm", "Disarm (MV)", "Disarm selected vehicles."),
    ("mvStartMission", "Start", "Start Mission (MV)", "Takeoff from ground and start the current mission for selected vehicles."),
    ("mvPause", "Pause", "Pause (MV)", "Pause selected vehicles at their current position."),
];

fn selected_ids(backend: &dyn Backend) -> Vec<i64> {
    let count = integer(&object(&backend.get("vehicles.selectedVehicles.count")), "value").unwrap_or(0).max(0);
    (0..count)
        .filter_map(|index| integer(&object(&backend.get_fields(&format!("vehicles.selectedVehicles.{index}"), "id")), "id"))
        .collect()
}

fn mv_refusal(id: &str, chosen: &[&Value]) -> Option<&'static str> {
    let armed: Vec<&&Value> = chosen.iter().filter(|v| v["armed"] == json!(true)).collect();
    match (chosen.is_empty(), id) {
        (true, _) => Some("No vehicles are selected."),
        (false, "mvArm") => chosen.iter().all(|v| v["armed"] == json!(true)).then_some("Every selected vehicle is already armed."),
        (false, "mvDisarm" | "mvStartMission" | "mvPause") if armed.is_empty() => Some("No selected vehicle is armed."),
        (false, "mvDisarm") => None,
        (false, "mvStartMission") => armed
            .iter()
            .all(|v| v["flightMode"] == v["missionFlightMode"])
            .then_some("Every armed selection is already flying its mission."),
        (false, "mvPause") => armed.iter().all(|v| v["pauseSupported"] != json!(true)).then_some("No selected vehicle supports being paused."),
        (false, _) => Some("The core does not know this action."),
    }
}

fn mv_actions(listed: &[Value]) -> Value {
    let chosen: Vec<&Value> = listed.iter().filter(|v| v["selected"] == json!(true)).collect();
    MV_ACTIONS
        .iter()
        .map(|(id, title, confirm_title, prompt)| {
            let refusal = mv_refusal(id, &chosen);
            json!({
                "id": id,
                "title": title,
                "confirmTitle": confirm_title,
                "prompt": prompt,
                "offer": match refusal { Some(_) => "blocked", None => "ready" },
                "reason": refusal.unwrap_or(""),
            })
        })
        .collect::<Vec<Value>>()
        .into()
}

pub fn vehicles_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let count = integer(&object(&backend.get("vehicles.vehicles.count")), "value").unwrap_or(0).max(0);
    VEHICLES_SEEN.store(usize::try_from(count).unwrap_or(0), Ordering::Relaxed);
    let active = integer(&object(&backend.get_fields("vehicle", "id")), "id");
    let chosen_ids = selected_ids(backend);
    let listed: Vec<Value> = (0..count)
        .map(|index| {
            let read = object(&backend.get_fields(&format!("vehicles.vehicles.{index}"), FIELDS));
            let link = object(&backend.get_fields(&format!("vehicles.vehicles.{index}.vehicleLinkManager"), "primaryLinkName,communicationLost,communicationLostEnabled"));
            let supports = object(&backend.get_fields(&format!("vehicles.vehicles.{index}.supports"), "pauseVehicle"));
            let id = integer(&read, "id");
            json!({
                "id": id,
                "name": name_of(&read, id),
                "type": text(&read, "vehicleTypeString"),
                "firmware": text(&read, "firmwareTypeString"),
                "link": text(&link, "primaryLinkName"),
                "contactLost": flag(&link, "communicationLostEnabled").then(|| flag(&link, "communicationLost")),
                "coordinate": nested_coordinate(&read).map(|(latitude, longitude)| json!({ "latitude": latitude, "longitude": longitude })),
                "heading": heading(backend, index),
                "home": crate::read::nested_coordinate_at(&read, "homePosition").map(|(latitude, longitude)| json!({ "latitude": latitude, "longitude": longitude })),
                "active": id.is_some() && id == active,
                "armed": flag(&read, "armed"),
                "flying": flag(&read, "flying"),
                "flightMode": text(&read, "flightMode"),
                "missionFlightMode": text(&read, "missionFlightMode"),
                "pauseSupported": flag(&supports, "pauseVehicle"),
                "selected": id.is_some_and(|id| chosen_ids.contains(&id)),
                "proximity": proximity(backend, index),
                "telemetry": telemetry(&read, &|name| shown_fact(backend, index, name)),
                "index": index,
                "flightModes": flag(&read, "flightModeSetAvailable").then(|| read.get("flightModes").cloned()).flatten().filter(Value::is_array).unwrap_or(json!([])),
            })
        })
        .collect();
    json!({
        "kind": "object",
        "class": "Vehicles",
        "count": listed.len(),
        "activeId": active,
        "ambiguous": listed.len() > 1,
        "selectedCount": chosen_ids.len(),
        "canSelectAll": chosen_ids.len() != listed.len(),
        "canDeselectAll": !chosen_ids.is_empty(),
        "actions": mv_actions(&listed),
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

pub fn fleet_mode_target(path: &str) -> Option<usize> {
    path.strip_prefix("vehicles.vehicles.")?.strip_suffix(".flightMode")?.parse().ok()
}

pub fn fleet_mode_refusal(listed: &Value, wanted_vehicle: Option<i64>, asked: Option<&str>) -> Option<(&'static str, &'static str)> {
    let modes: Vec<&str> = listed.get("flightModes").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    match (listed.get("id").and_then(Value::as_i64), asked) {
        (id, _) if id.is_none() || id != wanted_vehicle => Some(("vehicleMoved", "That vehicle is no longer in that row. Pick it again.")),
        (_, None) => Some(("notAMode", "Choose a flight mode.")),
        _ if !flag(listed, "flightModeSetAvailable") => Some(("cannotSet", "This vehicle cannot change flight mode.")),
        (_, Some(mode)) if !modes.contains(&mode) => Some(("notAMode", "That is not one of this vehicle's flight modes.")),
        _ => None,
    }
}

pub fn write_fleet_mode(backend: &dyn Backend, path: &str, value: &str) -> Value {
    let given = serde_json::from_str::<Value>(value).unwrap_or(Value::Null);
    let asked = given.get("value").and_then(Value::as_str).map(str::to_string);
    let index = fleet_mode_target(path).unwrap_or_default();
    let listed = object(&backend.get_fields(&format!("vehicles.vehicles.{index}"), "id,flightMode,flightModes,flightModeSetAvailable"));
    if let Some((token, reason)) = fleet_mode_refusal(&listed, given.get("vehicle").and_then(Value::as_i64), asked.as_deref()) {
        return json!({ "ok": false, "result": false, "refusal": token, "reason": reason });
    }
    let asked = asked.unwrap_or_default();
    if text(&listed, "flightMode") == asked {
        return json!({ "ok": true, "result": true, "refusal": Value::Null, "unchanged": true, "reason": Value::Null });
    }
    let answer = object(&backend.set(path, &json!({ "value": asked, "vehicle": listed.get("id").cloned().unwrap_or(Value::Null) }).to_string()));
    let answered = flag(&answer, "ok");
    json!({
        "ok": answered,
        "result": answered,
        "refusal": Value::Null,
        "unchanged": false,
        "reason": match answered { true => Value::Null, false => answer.get("reason").cloned().filter(|r| r.is_string()).unwrap_or(json!("The vehicle was not asked to change mode.")) },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fleet_mode_write_goes_only_to_the_vehicle_the_row_showed() {
        let row = json!({ "id": 2, "flightMode": "Loiter", "flightModes": ["Loiter", "RTL"], "flightModeSetAvailable": true });
        assert_eq!(fleet_mode_refusal(&row, Some(3), Some("RTL")).map(|r| r.0), Some("vehicleMoved"), "a row that now holds another vehicle is refused, unlike the index alone");
        assert_eq!(fleet_mode_refusal(&row, Some(2), Some("RTL")), None);
        assert_eq!(fleet_mode_refusal(&row, Some(2), Some("Acro")).map(|r| r.0), Some("notAMode"));
        assert_eq!(fleet_mode_refusal(&json!({ "id": 2, "flightModeSetAvailable": false }), Some(2), Some("RTL")).map(|r| r.0), Some("cannotSet"));
        assert_eq!(fleet_mode_target("vehicles.vehicles.4.flightMode"), Some(4));
        assert_eq!(fleet_mode_target("vehicle.flightMode"), None);
    }


    #[test]
    fn each_row_carries_altitude_and_ground_or_air_speed_like_the_multi_vehicle_telemetry_bar() {
        let values = |name: &str| match name { "altitudeRelative" => Some("12.3 m".to_string()), "groundSpeed" => Some("5.4 m/s".to_string()), "airSpeed" => Some("18.0 m/s".to_string()), _ => None };
        assert_eq!(telemetry(&json!({ "fixedWing": false }), &values), vec![json!({ "label": "Alt (Rel)", "value": "12.3 m" }), json!({ "label": "Ground Speed", "value": "5.4 m/s" })]);
        assert_eq!(telemetry(&json!({ "vtol": true }), &values)[1], json!({ "label": "AirSpd", "value": "18.0 m/s" }), "fixed wing, VTOL and airships show airspeed");
        assert!(telemetry(&json!({}), &|_| None).is_empty());
    }
    static FLEET_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    fn fleet_guard() -> std::sync::MutexGuard<'static, ()> {
        FLEET_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    struct Fleet(Vec<Value>, Option<i64>);

    impl Fleet {
        fn chosen(&self) -> Vec<Value> {
            self.0.iter().filter(|v| v["chosen"] == json!(true)).cloned().collect()
        }
    }

    impl Backend for Fleet {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicles.vehicles.count" => json!({ "kind": "value", "value": self.0.len() }).to_string(),
                "vehicles.selectedVehicles.count" => json!({ "kind": "value", "value": self.chosen().len() }).to_string(),
                _ => match path.strip_prefix("vehicles.vehicles.").and_then(|rest| rest.split_once('.')).and_then(|(i, tail)| Some((i.parse::<usize>().ok()?, tail))) {
                    Some((index, "heading")) => self.0.get(index).and_then(|v| v.get("headingFact")).map(Value::to_string).unwrap_or_default(),
                    Some((index, "distanceSensors")) => self.0.get(index).and_then(|v| v.get("distanceSensors")).map(Value::to_string).unwrap_or_default(),
                    _ => String::new(),
                },
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            if let Some(index) = path.strip_prefix("vehicles.selectedVehicles.") {
                return match index.parse::<usize>().ok().and_then(|index| self.chosen().get(index).cloned()) {
                    Some(vehicle) => json!({ "kind": "object", "id": vehicle["id"] }).to_string(),
                    None => String::new(),
                };
            }
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
                        "supports" => json!({ "kind": "object", "pauseVehicle": vehicle["pauseVehicleSupported"] }).to_string(),
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
        json!({ "kind": "object", "id": id, "vehicleTypeString": kind, "firmwareTypeString": "ArduPilot", "armed": false, "flying": false, "flightMode": "Loiter", "missionFlightMode": "Auto", "pauseVehicleSupported": true, "link": link, "quiet": false,
                "coordinate": { "valid": true, "latitude": 47.397, "longitude": 8.546, "altitude": 12.0 } })
    }

    fn offer_of(view: &Value, id: &str) -> (String, String) {
        let action = view["actions"].as_array().unwrap().iter().find(|a| a["id"] == json!(id)).unwrap();
        (action["offer"].as_str().unwrap().to_string(), action["reason"].as_str().unwrap().to_string())
    }

    #[test]
    fn nothing_is_offered_to_a_fleet_with_no_selection() {
        let _guard = fleet_guard();
        let view = vehicles_view(&Fleet(vec![aircraft(1, "Quadrotor", "Radio"), aircraft(2, "Quadrotor", "Radio")], Some(1)), &[]);

        assert_eq!(view["selectedCount"], json!(0));
        assert_eq!((view["canSelectAll"].clone(), view["canDeselectAll"].clone()), (json!(true), json!(false)));
        assert!(view["vehicles"].as_array().unwrap().iter().all(|v| v["selected"] == json!(false)));
        MV_ACTIONS.iter().for_each(|(id, _, _, _)| {
            assert_eq!(
                offer_of(&view, id),
                ("blocked".to_string(), "No vehicles are selected.".to_string()),
                "QGC has no arm-all: every multi-vehicle action reads QGroundControl.multiVehicleManager.selectedVehicles and acts on nothing when it is empty, so an empty selection has to refuse rather than reach the whole fleet"
            );
        });
    }

    #[test]
    fn each_multi_vehicle_action_follows_its_own_availability_rule() {
        let _guard = fleet_guard();
        let chosen = |armed: bool, mode: &str, pausable: bool| {
            let mut vehicle = aircraft(1, "Quadrotor", "Radio");
            vehicle["chosen"] = json!(true);
            vehicle["armed"] = json!(armed);
            vehicle["flightMode"] = json!(mode);
            vehicle["pauseVehicleSupported"] = json!(pausable);
            vehicles_view(&Fleet(vec![vehicle], Some(1)), &[])
        };

        let disarmed = chosen(false, "Loiter", true);
        assert_eq!(disarmed["selectedCount"], json!(1));
        assert_eq!(disarmed["canSelectAll"], json!(false), "MultiVehicleList.qml enables Select All on a count mismatch, so a fully selected fleet has nothing left to select");
        assert_eq!(offer_of(&disarmed, "mvArm").0, "ready");
        assert_eq!(offer_of(&disarmed, "mvDisarm"), ("blocked".to_string(), "No selected vehicle is armed.".to_string()));
        assert_eq!(offer_of(&disarmed, "mvStartMission").1, "No selected vehicle is armed.", "startAvailable requires armed === true before it looks at the flight mode, so a disarmed selection is refused for being disarmed rather than for its mode");
        assert_eq!(offer_of(&disarmed, "mvPause").1, "No selected vehicle is armed.");

        let armed = chosen(true, "Loiter", true);
        assert_eq!(offer_of(&armed, "mvArm"), ("blocked".to_string(), "Every selected vehicle is already armed.".to_string()));
        assert_eq!(offer_of(&armed, "mvDisarm").0, "ready");
        assert_eq!(offer_of(&armed, "mvStartMission").0, "ready");
        assert_eq!(offer_of(&armed, "mvPause").0, "ready");

        assert_eq!(
            offer_of(&chosen(true, "Auto", true), "mvStartMission"),
            ("blocked".to_string(), "Every armed selection is already flying its mission.".to_string()),
            "startAvailable compares flightMode against the vehicle's own missionFlightMode rather than a literal, because the name of the mission mode differs between firmwares"
        );
        assert_eq!(
            offer_of(&chosen(true, "Loiter", false), "mvPause"),
            ("blocked".to_string(), "No selected vehicle supports being paused.".to_string())
        );
        assert_eq!(offer_of(&chosen(true, "Loiter", false), "mvDisarm").0, "ready", "pauseVehicleSupported gates only Pause");

        assert_eq!(
            mv_refusal("mvLandEverything", &[&aircraft(1, "Quadrotor", "Radio")]),
            Some("The core does not know this action."),
            "an id that reaches no rule has to refuse: the fallthrough used to be None, so a typo in MV_ACTIONS would have offered an unrecognised fleet-wide command as ready"
        );
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
        assert!(after.contains(&"vehicles.vehicles.1.vehicleLinkManager.communicationLostEnabled".to_string()), "and the flag that decides whether contactLost means anything - turning link monitoring off changes a served true or false into a null, and a view that does not watch it keeps reporting a verdict nobody is checking any more");
        assert!(after.contains(&"vehicles.vehicles.count".to_string()), "the count stays watched, because it is what makes the list re-derive when the fleet changes");
    }

    #[test]
    fn a_seventeenth_vehicle_is_watched_like_the_rest() {
        let _fleet = fleet_guard();
        let fleet: Vec<Value> = (1..=17).map(|id| aircraft(id, "Multi-Rotor", "SITL")).collect();
        vehicles_view(&Fleet(fleet, Some(1)), &[]);
        assert!(deps().contains(&"vehicles.vehicles.16.armed".to_string()), "a cap on how many vehicles are watched stops at a number nothing reports, so the vehicles past it are stale with no tell - the count QGC gives is already bounded by what has connected");
    }

    #[test]
    fn every_vehicle_carries_its_own_proximity_radar() {
        let _guard = fleet_guard();
        let mut lead = aircraft(1, "Quadrotor", "Radio");
        lead["distanceSensors"] = json!({ "kind": "object", "telemetryAvailable": true, "facts": [{ "property": "rotationNone", "rawValue": 2.5, "valueString": "2.50" }, { "name": "maxDistance", "rawValue": 40.0 }] });
        let wing = aircraft(2, "Quadrotor", "Radio");
        let view = vehicles_view(&Fleet(vec![lead, wing], Some(2)), &[]);
        assert_eq!(view["vehicles"][0]["proximity"]["sectors"][0]["meters"], 2.5, "FlyViewMap draws ProximityRadarMapView for every vehicle, not only the active one");
        assert_eq!(view["vehicles"][0]["proximity"]["maxMeters"], 40.0);
        assert_eq!(view["vehicles"][1]["proximity"], Value::Null, "a vehicle with no distance sensors draws no radar");
        assert!(deps().contains(&"vehicles.vehicles.0.distanceSensors.rotationNone".to_string()), "the group itself has no change signal; its facts do");
    }

    #[test]
    fn every_vehicle_carries_its_own_heading_and_home() {
        let _guard = fleet_guard();
        let mut lead = aircraft(1, "Quadrotor", "Radio");
        lead["headingFact"] = json!({ "kind": "fact", "name": "heading", "value": 1200.0, "rawValue": -90.0 });
        lead["homePosition"] = json!({ "latitude": 47.397, "longitude": 8.545, "altitude": 488.0, "valid": true });
        let mut wing = aircraft(2, "Quadrotor", "Radio");
        wing["headingFact"] = json!({ "kind": "fact", "name": "heading", "value": null, "rawValue": null });
        wing["homePosition"] = json!({ "latitude": 0.0, "longitude": 0.0, "valid": false });
        let view = vehicles_view(&Fleet(vec![lead, wing], Some(1)), &[]);
        let listed = view["vehicles"].as_array().unwrap();
        assert_eq!(listed[0]["heading"], json!(270.0), "the raw degrees are served and wrapped into 0..360, whatever the cooked value says");
        assert_eq!(listed[0]["home"], json!({ "latitude": 47.397, "longitude": 8.545 }));
        assert_eq!((&listed[1]["heading"], &listed[1]["home"]), (&Value::Null, &Value::Null), "no attitude yet is no heading rather than north, and an invalid home is no home");
        assert!(deps().iter().any(|d| d == "vehicles.vehicles.1.heading") && deps().iter().any(|d| d == "vehicles.vehicles.1.homePosition"));
    }
}
