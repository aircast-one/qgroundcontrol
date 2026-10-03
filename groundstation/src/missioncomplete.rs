use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::read::{flag, integer, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.armed",
    "vehicle.flightMode",
    "vehicle.cameraTriggerPoints.count",
    "planFly.missionController.containsItems",
    "planFly.geoFenceController.containsItems",
    "planFly.rallyPointController.containsItems",
    "planFly.missionController.resumeMissionIndex",
    crate::coreplan::CHANGED,
];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Latch {
    armed: Option<bool>,
    was_armed: bool,
    was_in_mission: bool,
    shown: u64,
    open: bool,
}

pub struct Reading {
    pub connected: bool,
    pub armed: bool,
    pub in_mission: bool,
    pub has_plan: bool,
}

pub fn step(latch: Latch, now: &Reading) -> Latch {
    let armed = !now.connected || now.armed;
    let in_mission = now.connected && now.in_mission;
    let base = Latch { armed: now.connected.then_some(now.armed), open: latch.open && now.connected, ..latch };
    match (latch.armed.unwrap_or(true) != armed, armed) {
        (true, true) => Latch { was_armed: true, was_in_mission: in_mission, ..base },
        (true, false) => {
            let show = latch.was_armed && latch.was_in_mission && now.has_plan;
            Latch { was_armed: false, was_in_mission: false, shown: latch.shown + u64::from(show), open: latch.open || show, ..base }
        }
        (false, true) => Latch { was_in_mission: latch.was_in_mission || in_mission, ..base },
        (false, false) => base,
    }
}

static LATCH: Mutex<Latch> = Mutex::new(Latch { armed: None, was_armed: false, was_in_mission: false, shown: 0, open: false });

fn contains(backend: &dyn Backend, controller: &str) -> bool {
    flag(&object(&backend.get_fields(&format!("planFly.{controller}"), "containsItems")), "containsItems")
}

fn reading(backend: &dyn Backend) -> (Reading, i64) {
    let vehicle = object(&backend.get_fields("vehicle", "armed,flightMode,missionFlightMode"));
    let connected = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let in_mission = vehicle.get("flightMode").is_some() && vehicle.get("flightMode") == vehicle.get("missionFlightMode");
    let images = integer(&object(&backend.get_fields("vehicle.cameraTriggerPoints", "count")), "count").unwrap_or(0);
    let mission = match crate::coreplan::plan_state() {
        Some(plan) => plan.has_mission_items,
        None => contains(backend, "missionController"),
    };
    let has_plan = mission || contains(backend, "geoFenceController") || contains(backend, "rallyPointController") || images != 0;
    (Reading { connected, armed: flag(&vehicle, "armed"), in_mission, has_plan }, images)
}

pub fn mission_complete_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let (now, images) = reading(backend);
    let latch = {
        let mut held = LATCH.lock().unwrap_or_else(PoisonError::into_inner);
        *held = step(*held, &now);
        *held
    };
    let resume = latch.open.then(|| crate::guided::resume_offer(backend)).flatten();
    json!({
        "kind": "object",
        "class": "MissionComplete",
        "open": latch.open,
        "id": latch.shown,
        "imagesTaken": images,
        "resumeFromWaypoint": resume,
        "batteryWarning": resume.is_some(),
    })
}

pub fn dismiss(id: u64) -> Value {
    let mut held = LATCH.lock().unwrap_or_else(PoisonError::into_inner);
    match held.open && held.shown == id {
        true => {
            held.open = false;
            json!({ "ok": true })
        }
        false => json!({ "ok": false, "reason": "That mission complete notice is no longer open." }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(armed: bool, in_mission: bool) -> Reading {
        Reading { connected: true, armed, in_mission, has_plan: true }
    }

    fn run(readings: &[Reading]) -> Latch {
        readings.iter().fold(Latch::default(), step)
    }

    #[test]
    fn opens_on_disarm_after_an_armed_mission() {
        assert!(run(&[at(false, false), at(true, false), at(true, true), at(false, false)]).open, "armed, then Auto, then disarmed is a flown plan");
        assert!(run(&[at(false, false), at(true, true), at(false, true)]).open);
        assert!(!run(&[at(false, false), at(true, false), at(false, false)]).open, "a flight that never entered the mission mode finished nothing");
        assert!(!run(&[at(false, false), at(true, true), at(true, true), at(true, true)]).open, "still armed is still flying");
        let no_plan = [at(false, false), at(true, true), Reading { has_plan: false, ..at(false, false) }];
        assert!(!run(&no_plan).open, "with no plan, fence, rally or photos there is nothing to report");
        assert!(!run(&[at(false, false), at(true, true), Reading { connected: false, ..at(false, false) }]).open, "losing the vehicle closes it, as the dialog does when activeVehicle goes");
        assert!(!run(&[at(true, true), at(false, true)]).open, "a vehicle first seen already armed never armed under QGC's eyes, since no vehicle reads as armed");
        let gone = Reading { connected: false, ..at(false, false) };
        assert!(run(&[at(false, false), at(true, true), gone, at(false, false)]).open, "a link lost in flight that returns disarmed is the finished mission QGC reports");
    }

    #[test]
    fn repeated_readings_do_not_retrigger() {
        let opened = run(&[at(false, false), at(true, true), at(false, false), at(false, false), at(false, false)]);
        assert_eq!(opened.shown, 1);
    }

    #[test]
    fn dismissing_needs_the_open_id() {
        *LATCH.lock().unwrap() = Latch { open: true, shown: 4, ..Latch::default() };
        assert_eq!(dismiss(3)["ok"], false);
        assert_eq!(dismiss(4)["ok"], true);
        assert!(!LATCH.lock().unwrap().open);
    }
}
