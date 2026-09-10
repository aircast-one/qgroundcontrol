use serde_json::{Value, json};

use crate::read::{flag, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.armed", "vehicle.flying", "vehicle.landing", "vehicle.flightMode", "vehicle.vehicleLinkManager.communicationLost"];

pub const STALE_NOTICE: &str = "No contact — these are the last values the vehicle sent.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    NotConnected,
    ContactLost,
    Flying,
    Landing,
    Armed,
    Disarmed,
}

pub const STATES: &[State] = &[State::NotConnected, State::ContactLost, State::Flying, State::Landing, State::Armed, State::Disarmed];

impl State {
    pub fn token(self) -> &'static str {
        match self {
            State::NotConnected => "notConnected",
            State::ContactLost => "contactLost",
            State::Flying => "flying",
            State::Landing => "landing",
            State::Armed => "armed",
            State::Disarmed => "disarmed",
        }
    }

    pub fn line(self) -> &'static str {
        match self {
            State::NotConnected => "Not connected",
            State::ContactLost => "Communication lost",
            State::Flying => "Flying",
            State::Landing => "Landing",
            State::Armed => "Armed",
            State::Disarmed => "Disarmed",
        }
    }
}

pub fn state_of(connected: bool, contact_lost: bool, armed: bool, flying: bool, landing: bool) -> State {
    match (connected, contact_lost, armed, flying, landing) {
        (false, ..) => State::NotConnected,
        (_, true, ..) => State::ContactLost,
        (_, _, false, ..) => State::Disarmed,
        (_, _, _, true, _) => State::Flying,
        (_, _, _, _, true) => State::Landing,
        _ => State::Armed,
    }
}

pub fn fly_state_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", "armed,flying,landing,flightMode"));
    let connected = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let links = object(&backend.get_fields("vehicle.vehicleLinkManager", "communicationLost"));
    let contact_lost = connected && flag(&links, "communicationLost");
    let state = state_of(connected, contact_lost, flag(&vehicle, "armed"), flag(&vehicle, "flying"), flag(&vehicle, "landing"));
    json!({
        "kind": "object",
        "class": "FlyState",
        "connected": connected,
        "armed": flag(&vehicle, "armed"),
        "flying": flag(&vehicle, "flying"),
        "landing": flag(&vehicle, "landing"),
        "contactLost": contact_lost,
        "state": state.token(),
        "stateText": state.line(),
        "staleNotice": if contact_lost { STALE_NOTICE } else { "" },
        "mode": text(&vehicle, "flightMode"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        vehicle: Value,
        lost: bool,
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            self.get_fields(path, "")
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "vehicle" => self.vehicle.to_string(),
                "vehicle.vehicleLinkManager" => json!({ "kind": "object", "communicationLost": self.lost }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _path: &str, _value: &str) -> String { String::new() }
        fn invoke(&self, _path: &str, _args: &str) -> String { String::new() }
        fn watch(&self, _paths: &[String]) {}
    }

    fn read(vehicle: Value, lost: bool) -> Value {
        fly_state_view(&Fake { vehicle, lost }, &[])
    }

    fn aloft(armed: bool, flying: bool, landing: bool) -> Value {
        json!({ "kind": "object", "armed": armed, "flying": flying, "landing": landing, "flightMode": "Hold" })
    }

    #[test]
    fn lost_contact_outranks_every_flight_state() {
        let view = read(aloft(true, true, false), true);
        assert_eq!((view["state"].as_str(), view["stateText"].as_str()), (Some("contactLost"), Some("Communication lost")), "a vehicle that stopped answering is not known to still be flying");
        assert_eq!(view["staleNotice"], STALE_NOTICE);
        assert_eq!((view["armed"].as_bool(), view["flying"].as_bool()), (Some(true), Some(true)), "the last known state is still served, it just no longer names the line");
    }

    #[test]
    fn a_disarmed_vehicle_never_reads_as_flying() {
        let view = read(aloft(false, true, false), false);
        assert_eq!(view["state"], "disarmed", "Vehicle::_flying is set from EXTENDED_SYS_STATE and never cleared on disarm, so armed is what decides the line");
        assert_eq!(view["flying"], true, "the last flying flag is still reported for a head that wants it");
    }

    #[test]
    fn an_armed_vehicle_reads_as_what_it_is_doing() {
        let line = |armed: bool, flying: bool, landing: bool| read(aloft(armed, flying, landing), false)["state"].as_str().unwrap().to_string();
        assert_eq!(line(true, true, false), "flying");
        assert_eq!(line(true, true, true), "flying", "a vehicle still under way reads as flying, as the Qt indicator does");
        assert_eq!(line(true, false, true), "landing");
        assert_eq!(line(true, false, false), "armed");
        assert_eq!(read(aloft(true, true, false), false)["staleNotice"], "", "a vehicle in contact carries no stale notice");
    }

    #[test]
    fn no_vehicle_is_not_a_lost_contact() {
        let view = read(json!({ "kind": "null" }), true);
        assert_eq!((view["state"].as_str(), view["stateText"].as_str()), (Some("notConnected"), Some("Not connected")), "never having heard a vehicle is not the same as losing one");
        assert_eq!((view["contactLost"].as_bool(), view["armed"].as_bool(), view["flying"].as_bool()), (Some(false), Some(false), Some(false)));
        assert_eq!(view["staleNotice"], "");
        assert_eq!(view["mode"], "");
    }

    #[test]
    fn the_listed_states_are_exactly_the_states_reachable() {
        let bit = |mask: u8, index: u8| mask & (1 << index) != 0;
        let reachable: std::collections::BTreeSet<State> = (0u8..32).map(|mask| state_of(bit(mask, 0), bit(mask, 1), bit(mask, 2), bit(mask, 3), bit(mask, 4))).collect();
        assert_eq!(reachable, STATES.iter().copied().collect(), "every state a head switches on is one this view can answer, and there are no others");
    }
}
