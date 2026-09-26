use serde_json::{Value, json};

use crate::read::{flag, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.armed", "vehicle.flying", "vehicle.landing", "vehicle.flightMode", "vehicle.vehicleLinkManager.communicationLost", "vehicle.vehicleLinkManager.communicationLostEnabled", "planFly.missionController.currentMissionIndex", "vehicle.rcRSSI", "vehicle.rcChannelOverrideActive", "vehicle.radioStatus.lrssi", "vehicle.radioStatus.rrssi", "vehicle.radioStatus.lNoise", "vehicle.radioStatus.rNoise", "vehicle.radioStatus.rxErrors"];

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
        (_, _, _, true, true) => State::Landing,
        (_, _, _, true, false) => State::Flying,
        _ => State::Armed,
    }
}

// QGC reports 255 when the vehicle has not said what the signal strength is, and 0..=100
// otherwise. The range test excludes the sentinel on its own, so there is no separate
// constant for it - one would be a branch nothing can reach. Zero stays a READING and the
// worst one: QGC's own indicator hides at zero, so a total RC loss looks the same there as
// an aircraft with no transmitter fitted.
fn rc_signal(vehicle: &Value) -> Option<i64> {
    vehicle.get("rcRSSI").and_then(Value::as_i64).filter(|rssi| (0..=100).contains(rssi))
}

fn flying_to(backend: &dyn Backend) -> Option<i64> {
    object(&backend.get_fields("planFly.missionController", "currentMissionIndex"))
        .get("currentMissionIndex")
        .and_then(Value::as_i64)
        .filter(|sequence| *sequence >= 0)
}

// TelemetryRSSIIndicator.qml:30 hides the whole indicator on telemetryLRSSI == 0, and every SiK
// field is zero before the first RADIO_STATUS arrives - so zero here is "no radio has spoken",
// not a reading of zero dBm. Serving the numbers ungated would let a head draw -0 dBm and a
// healthy-looking link for a radio that has never reported.
fn telemetry(radio: &Value) -> Value {
    let reading = |name: &str| radio.get(name).and_then(|fact| fact.get("value").or(Some(fact))).and_then(Value::as_i64);
    match reading("lrssi").filter(|local| *local != 0) {
        None => Value::Null,
        Some(local) => json!({
            "localRssiDbm": local,
            "remoteRssiDbm": reading("rrssi"),
            "localNoise": reading("lNoise"),
            "remoteNoise": reading("rNoise"),
            "receiveErrors": reading("rxErrors"),
        }),
    }
}

pub fn fly_state_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", "armed,flying,landing,flightMode,rcRSSI,supportsRadio,rcChannelOverrideActive"));
    let radio = object(&backend.get_fields("vehicle.radioStatus", "lrssi,rrssi,lNoise,rNoise,rxErrors"));
    let connected = vehicle.get("kind").and_then(Value::as_str) == Some("object");
    // _commLostCheck returns early when the watch is disabled, so communicationLost never updates
    // and false means "nobody is looking" rather than "every link is fine". view.frame and
    // view.vehicleLinks both gate on the enabled flag; this one did not, and served a plain bool
    // that called an unmonitored link healthy. Unknown is not evidence of a loss, so the state and
    // the notice stay as they are - only the field admits it does not know.
    let links = object(&backend.get_fields("vehicle.vehicleLinkManager", "communicationLost,communicationLostEnabled"));
    let watching = flag(&links, "communicationLostEnabled");
    let reported = connected.then(|| watching.then(|| flag(&links, "communicationLost"))).flatten();
    let contact_lost = reported.unwrap_or(false);
    let state = state_of(connected, contact_lost, flag(&vehicle, "armed"), flag(&vehicle, "flying"), flag(&vehicle, "landing"));
    json!({
        "kind": "object",
        "class": "FlyState",
        "connected": connected,
        "armed": flag(&vehicle, "armed"),
        "flying": flag(&vehicle, "flying"),
        "landing": flag(&vehicle, "landing"),
        "contactLost": reported,
        "state": state.token(),
        "stateText": state.line(),
        "staleNotice": if contact_lost { STALE_NOTICE } else { "" },
        "mode": text(&vehicle, "flightMode"),
        "flyingToSequence": flying_to(backend),
        "rcSupported": flag(&vehicle, "supportsRadio"),
        "rcSignal": rc_signal(&vehicle),
        "rcSignalText": rc_signal(&vehicle).map(|percent| match percent {
            0 => "No signal".to_string(),
            percent => format!("{percent}%"),
        }),
        // rcChannelOverrideActive is !_rcChannelOverrides.isEmpty(), so with no vehicle it is not
        // false but unknown - the same distinction contactLost makes, and the one that decides
        // whether a head may draw "manual control is not being overridden" or must draw nothing.
        "rcOverride": connected.then(|| flag(&vehicle, "rcChannelOverrideActive")),
        "telemetry": telemetry(&radio),
    })
}

fn reboot_refusal(state: &str) -> Option<(&'static str, &'static str)> {
    match state {
        "disarmed" => None,
        "notConnected" => Some(("noVehicle", "No vehicle is connected.")),
        "contactLost" => Some(("contactLost", "The vehicle has stopped answering, so it is not known to be on the ground. Restore the link before rebooting.")),
        "flying" | "landing" => Some(("flying", "The vehicle is in the air. Land and disarm before rebooting.")),
        _ => Some(("armed", "The vehicle is armed. Disarm it before rebooting.")),
    }
}

pub fn reboot(backend: &dyn Backend, path: &str) -> Value {
    let view = fly_state_view(backend, &[]);
    if let Some((token, reason)) = reboot_refusal(view["state"].as_str().unwrap_or("notConnected")) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let dispatched = flag(&object(&backend.invoke(path, "[]")), "ok");
    json!({
        "ok": dispatched,
        "refusal": Value::Null,
        "reason": match dispatched { true => Value::Null, false => json!("The vehicle was not asked to reboot.") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        vehicle: Value,
        lost: bool,
        flying_to: i64,
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            self.get_fields(path, "")
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "vehicle" => self.vehicle.to_string(),
                "vehicle.radioStatus" => self.vehicle.get("radioStatus").cloned().unwrap_or_else(|| json!({ "kind": "null" })).to_string(),
                "planFly.missionController" => json!({ "kind": "object", "currentMissionIndex": self.flying_to }).to_string(),
                "vehicle.vehicleLinkManager" => json!({ "kind": "object", "communicationLost": self.lost, "communicationLostEnabled": true }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _path: &str, _value: &str) -> String { String::new() }
        fn invoke(&self, _path: &str, _args: &str) -> String { String::new() }
        fn watch(&self, _paths: &[String]) {}
    }

    fn read(vehicle: Value, lost: bool) -> Value {
        fly_state_view(&Fake { vehicle, lost, flying_to: -1 }, &[])
    }

    #[test]
    fn a_transmitter_reporting_nothing_is_not_a_transmitter_reporting_no_signal() {
        let at = |rssi: i64| {
            let mut vehicle = aloft(true, true, false);
            vehicle["rcRSSI"] = json!(rssi);
            vehicle["supportsRadio"] = json!(true);
            fly_state_view(&Fake { vehicle, lost: false, flying_to: -1 }, &[])
        };
        assert_eq!(at(72)["rcSignal"], 72);
        assert_eq!(at(72)["rcSignalText"], "72%");
        assert_eq!(at(0)["rcSignalText"], "No signal", "a transmitter that is switched off is not a transmitter at zero per cent - an operator reading 0% concludes the link is alive and terrible rather than absent");
        assert_eq!(at(0)["rcSignal"], 0, "zero is a READING and the worst one - the transmitter is gone. QGC's own indicator hides at zero, so a total RC loss looks exactly like an aircraft with no transmitter fitted");
        assert_eq!(at(255)["rcSignal"], Value::Null, "255 is QGC's sentinel for a vehicle that has not reported a strength at all, which is the one case there is nothing to draw");
        assert_eq!(at(255)["rcSignalText"], Value::Null);
    }

    #[test]
    fn a_radio_that_has_never_reported_is_not_a_radio_at_zero_dbm() {
        let radio = |local: i64| {
            let mut vehicle = aloft(true, true, false);
            vehicle["radioStatus"] = json!({ "kind": "object", "lrssi": { "value": local }, "rrssi": { "value": -42 }, "lNoise": { "value": 12 }, "rNoise": { "value": 14 }, "rxErrors": { "value": 3 } });
            fly_state_view(&Fake { vehicle, lost: false, flying_to: -1 }, &[])["telemetry"].clone()
        };

        assert_eq!(
            radio(0),
            Value::Null,
            "every SiK field is zero until the first RADIO_STATUS arrives, and TelemetryRSSIIndicator.qml:30 hides the whole indicator on that - serving the numbers anyway lets a head draw a healthy -0 dBm link for a radio that has never spoken"
        );

        let reporting = radio(-91);
        assert_eq!(
            (reporting["localRssiDbm"].clone(), reporting["remoteRssiDbm"].clone(), reporting["receiveErrors"].clone()),
            (json!(-91), json!(-42), json!(3))
        );
        assert_eq!((reporting["localNoise"].clone(), reporting["remoteNoise"].clone()), (json!(12), json!(14)));
    }

    #[test]
    fn an_rc_override_is_unknown_without_a_vehicle_rather_than_absent() {
        let mut overridden = aloft(true, true, false);
        overridden["rcChannelOverrideActive"] = json!(true);
        assert_eq!(read(overridden, false)["rcOverride"], json!(true));

        assert_eq!(read(aloft(true, true, false), false)["rcOverride"], json!(false), "a connected vehicle with an empty override list is genuinely not overridden");
        assert_eq!(
            read(json!({ "kind": "null" }), false)["rcOverride"],
            Value::Null,
            "rcChannelOverrideActive is !_rcChannelOverrides.isEmpty(), so with no vehicle there is no list to be empty - false would tell an operator manual control is definitely not being overridden, which is a safety claim nothing has made"
        );
    }

    #[test]
    fn the_item_the_aircraft_is_flying_to_is_a_sequence_and_is_withheld_on_the_ground() {
        let airborne = |flying_to: i64| fly_state_view(&Fake { vehicle: aloft(true, true, false), lost: false, flying_to }, &[])["flyingToSequence"].clone();
        assert_eq!(airborne(3), json!(3), "MissionController::currentMissionIndex is a sequence number, already stepped past home for firmwares that do not send it, so a head matches it against an item's sequence rather than its index - the two differ as soon as a complex item is in the plan");
        assert_eq!(airborne(-1), Value::Null, "the fly controller answers -1 in the plan view and before the vehicle names an item, and a head drawing -1 would highlight nothing or the row before the first");
        assert_eq!(airborne(0), json!(0), "sequence 0 is the launch row and a real answer");
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
        assert_eq!(line(true, true, true), "landing", "landing is only ever set alongside flying, so it has to outrank it or it can never name the line");
        assert_eq!(line(true, false, true), "armed", "Vehicle::_setLanding is guarded by armed(), so landing latches through an auto-disarm on touchdown; requiring flying makes that latch unreachable");
        assert_eq!(line(true, false, false), "armed");
        assert_eq!(read(aloft(true, true, false), false)["staleNotice"], "", "a vehicle in contact carries no stale notice");
    }

    #[test]
    fn no_vehicle_is_not_a_lost_contact() {
        let view = read(json!({ "kind": "null" }), true);
        assert_eq!((view["state"].as_str(), view["stateText"].as_str()), (Some("notConnected"), Some("Not connected")), "never having heard a vehicle is not the same as losing one");
        assert_eq!(view["contactLost"], Value::Null, "with no vehicle there is no link to have lost, and false would be a claim about one");
        assert_eq!((view["armed"].as_bool(), view["flying"].as_bool()), (Some(false), Some(false)));
        assert_eq!(view["staleNotice"], "");
        assert_eq!(view["mode"], "");
    }

    #[test]
    fn the_listed_states_are_exactly_the_states_reachable() {
        let bit = |mask: u8, index: u8| mask & (1 << index) != 0;
        let reachable: std::collections::BTreeSet<State> = (0u8..32).map(|mask| state_of(bit(mask, 0), bit(mask, 1), bit(mask, 2), bit(mask, 3), bit(mask, 4))).collect();
        assert_eq!(reachable, STATES.iter().copied().collect(), "every state a head switches on is one this view can answer, and there are no others");
    }

    #[test]
    fn a_link_nobody_is_watching_is_not_reported_as_healthy() {
        struct Unwatched;
        impl Backend for Unwatched {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "vehicle" => json!({ "kind": "object", "armed": false, "flying": false, "landing": false, "flightMode": "Hold" }).to_string(),
                    "vehicle.vehicleLinkManager" => json!({ "kind": "object", "communicationLost": false, "communicationLostEnabled": false }).to_string(),
                    _ => json!({ "kind": "null" }).to_string(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        let unwatched = fly_state_view(&Unwatched, &[]);
        assert_eq!(unwatched["contactLost"], Value::Null, "_commLostCheck returns early when the watch is disabled, so communicationLost never updates and false means nobody is looking - this view served it raw and called an unmonitored link healthy, in five places on one head");
        assert_eq!(unwatched["state"].as_str(), Some("disarmed"), "and unknown is not evidence of a loss, so the state it drives is the one the vehicle actually reports");
        assert_eq!(unwatched["staleNotice"], "", "nor does an unknown link earn a stale notice");
    }

    #[test]
    fn a_reboot_goes_only_to_a_vehicle_known_to_be_disarmed_on_the_ground() {
        assert_eq!(reboot_refusal("disarmed"), None);
        STATES.iter().filter(|s| **s != State::Disarmed).for_each(|state| {
            assert!(reboot_refusal(state.token()).is_some(), "rebootVehicle sends MAV_CMD_PREFLIGHT_REBOOT_SHUTDOWN with no check, and {} is a state where that drops an aircraft or reaches nobody", state.token());
        });
        assert_eq!(reboot_refusal("contactLost").map(|(t, _)| t), Some("contactLost"), "a vehicle that stopped answering is not known to be on the ground");
        assert_eq!(reboot_refusal("landing").map(|(t, _)| t), Some("flying"));
        assert_eq!(reboot_refusal("somethingNew").map(|(t, _)| t), Some("armed"), "a state this list has not met is refused, not waved through");
    }
}
