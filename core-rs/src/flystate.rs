use serde_json::{Value, json};

use crate::read::{flag, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.armed", "vehicle.flying", "vehicle.flightMode", "vehicle.vehicleLinkManager.communicationLost"];

pub const CONTACT_LOST: &str = "Communication lost";
pub const STALE_NOTICE: &str = "No contact — these are the last values the vehicle sent.";
const NOT_CONNECTED: &str = "Not connected";
const FLYING: &str = "Flying";
const ARMED: &str = "Armed";
const DISARMED: &str = "Disarmed";

pub fn state_text(connected: bool, contact_lost: bool, armed: bool, flying: bool) -> &'static str {
    match (connected, contact_lost, flying, armed) {
        (false, ..) => NOT_CONNECTED,
        (_, true, ..) => CONTACT_LOST,
        (_, _, true, _) => FLYING,
        (_, _, _, true) => ARMED,
        _ => DISARMED,
    }
}

pub fn fly_state_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get("vehicle"));
    let connected = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    let links = object(&backend.get("vehicle.vehicleLinkManager"));
    let contact_lost = connected && flag(&links, "communicationLost");
    let armed = connected && flag(&vehicle, "armed");
    let flying = connected && flag(&vehicle, "flying");
    json!({
        "kind": "object",
        "class": "FlyState",
        "connected": connected,
        "armed": armed,
        "flying": flying,
        "contactLost": contact_lost,
        "state": state_text(connected, contact_lost, armed, flying),
        "staleNotice": if contact_lost { STALE_NOTICE } else { "" },
        "mode": if connected { text(&vehicle, "flightMode") } else { String::new() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::tests::MapBackend;

    fn backend(vehicle: Value, lost: bool) -> MapBackend {
        MapBackend::new(vec![
            ("vehicle".to_string(), vehicle.to_string()),
            ("vehicle.vehicleLinkManager".to_string(), json!({ "kind": "object", "communicationLost": lost }).to_string()),
        ])
    }

    fn connected_vehicle(armed: bool, flying: bool) -> Value {
        json!({ "kind": "object", "armed": armed, "flying": flying, "flightMode": "Hold" })
    }

    #[test]
    fn lost_contact_outranks_armed_and_flying() {
        let flying = fly_state_view(&backend(connected_vehicle(true, true), true), &[]);
        assert_eq!(flying["state"], CONTACT_LOST, "a vehicle that stopped answering is not known to still be flying");
        assert_eq!(flying["contactLost"], true);
        assert_eq!(flying["staleNotice"], STALE_NOTICE);
        assert_eq!((flying["armed"].as_bool(), flying["flying"].as_bool()), (Some(true), Some(true)), "the last known state is still served, it just no longer names the line");
    }

    #[test]
    fn a_talking_vehicle_reads_as_its_flight_state() {
        let cases = [(false, false, DISARMED), (true, false, ARMED), (true, true, FLYING)];
        let read = |armed: bool, flying: bool| fly_state_view(&backend(connected_vehicle(armed, flying), false), &[])["state"].as_str().unwrap().to_string();
        assert!(cases.iter().all(|(armed, flying, expected)| read(*armed, *flying) == *expected));
        let steady = fly_state_view(&backend(connected_vehicle(true, true), false), &[]);
        assert_eq!(steady["staleNotice"], "", "a vehicle in contact carries no stale notice");
        assert_eq!(steady["mode"], "Hold");
    }

    #[test]
    fn no_vehicle_is_not_a_lost_contact() {
        let view = fly_state_view(&backend(json!({ "kind": "null" }), true), &[]);
        assert_eq!(view["state"], NOT_CONNECTED, "never having heard a vehicle is not the same as losing one");
        assert_eq!((view["contactLost"].as_bool(), view["armed"].as_bool(), view["flying"].as_bool()), (Some(false), Some(false), Some(false)));
        assert_eq!(view["staleNotice"], "");
        assert_eq!(view["mode"], "");
    }
}
