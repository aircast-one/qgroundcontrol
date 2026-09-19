use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.gcsMain",
    "vehicle.gcsControlStatusFlags_SystemManager",
    "vehicle.gcsControlStatusFlags_TakeoverAllowed",
    "vehicle.firstControlStatusReceived",
    "vehicle.sendControlRequestAllowed",
    "settings.mavlinkSettings.gcsMavlinkSystemID",
];

const FIELDS: &str = "gcsMain,gcsControlStatusFlags_SystemManager,gcsControlStatusFlags_TakeoverAllowed,firstControlStatusReceived,sendControlRequestAllowed,operatorControlTakeoverTimeoutMsecs";

fn integer(read: &Value, key: &str) -> Option<i64> {
    read.get(key).and_then(Value::as_i64)
}

pub fn operator_control_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", FIELDS));
    if vehicle.get("kind").and_then(Value::as_str) != Some("object") {
        return json!({
            "kind": "object",
            "class": "OperatorControl",
            "available": false,
            "known": false,
            "inControl": Value::Null,
            "holderSystemId": Value::Null,
            "takeoverAllowed": Value::Null,
            "systemManager": Value::Null,
            "reason": "No vehicle is connected.",
        });
    }
    let known = flag(&vehicle, "firstControlStatusReceived");
    let holder = integer(&vehicle, "gcsMain");
    let ours = crate::read::value_number(&backend.get("settings.mavlinkSettings.gcsMavlinkSystemID.rawValue")).map(|id| id as i64);
    let answered = |yes: bool| known.then_some(yes);
    json!({
        "kind": "object",
        "class": "OperatorControl",
        "available": true,
        "known": known,
        "inControl": known.then(|| holder.zip(ours).map(|(holder, ours)| holder == ours)).flatten(),
        "holderSystemId": known.then_some(holder).flatten(),
        "takeoverAllowed": answered(flag(&vehicle, "gcsControlStatusFlags_TakeoverAllowed")),
        "systemManager": answered(flag(&vehicle, "gcsControlStatusFlags_SystemManager")),
        "requestAllowed": flag(&vehicle, "sendControlRequestAllowed"),
        "takeoverTimeoutMs": integer(&vehicle, "operatorControlTakeoverTimeoutMsecs"),
        "reason": match (known, holder, ours) {
            (false, _, _) => "This vehicle has not said who is flying it.",
            (true, Some(holder), Some(ours)) if holder == ours => "",
            (true, Some(_), Some(_)) => "Another ground station is flying this vehicle.",
            (true, None, _) => "This vehicle reported its control status without saying which station holds it.",
            (true, Some(_), None) => "This ground station could not read its own MAVLink system id, so it cannot tell whether the station flying this vehicle is itself.",
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Station {
        known: bool,
        holder: Option<i64>,
        takeover: bool,
        own_id: Option<i64>,
    }

    impl Backend for Station {
        fn get(&self, path: &str) -> String {
            match path {
                "settings.mavlinkSettings.gcsMavlinkSystemID.rawValue" => match self.own_id {
                    Some(id) => json!({ "kind": "value", "value": id }),
                    None => json!({ "kind": "null" }),
                },
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "vehicle" => json!({
                    "kind": "object",
                    "gcsMain": self.holder,
                    "gcsControlStatusFlags_SystemManager": true,
                    "gcsControlStatusFlags_TakeoverAllowed": self.takeover,
                    "firstControlStatusReceived": self.known,
                    "sendControlRequestAllowed": true,
                    "operatorControlTakeoverTimeoutMsecs": 10000,
                }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_comparison_the_core_could_not_make_is_not_a_verdict_that_someone_else_is_flying() {
        let unreadable = operator_control_view(&Station { known: true, holder: Some(250), takeover: false, own_id: None }, &[]);
        assert_eq!(unreadable["inControl"], Value::Null);
        assert_eq!(
            unreadable["reason"],
            "This ground station could not read its own MAVLink system id, so it cannot tell whether the station flying this vehicle is itself.",
            "the arm used to be a catch-all that swallowed None into \"Another ground station is flying this vehicle\", so a station that could not read its own id was told someone else held control - a verdict nothing had established, and one a head greys out its controls on"
        );

        let anonymous = operator_control_view(&Station { known: true, holder: None, takeover: false, own_id: Some(255) }, &[]);
        assert_eq!(
            anonymous["reason"],
            "This vehicle reported its control status without saying which station holds it.",
            "a missing holder and an unreadable own id are two different silences and neither is the other station"
        );

        let theirs = operator_control_view(&Station { known: true, holder: Some(42), takeover: false, own_id: Some(255) }, &[]);
        assert_eq!(theirs["reason"], "Another ground station is flying this vehicle.", "the real case still reads as it did");
    }

    #[test]
    fn the_countdown_is_not_served_because_no_watching_head_could_receive_it() {
        let view = operator_control_view(&Station { known: true, holder: Some(250), takeover: false, own_id: Some(250) }, &[]);

        assert!(
            !view.as_object().unwrap().contains_key("remainingMs"),
            "Vehicle::requestOperatorControlStartTimer sets _sendControlRequestAllowed and emits BEFORE starting _timerRequestOperatorControl, and that emit is the only dep that fires because requestOperatorControlRemainingMsecs is CONSTANT. So the view renders at the one instant remainingTime() returns -1 for an inactive timer, and nothing re-fires for the next ten seconds. A poll through /bridge/get counts down cleanly, which is why this looked like it worked: it serves a value no watching head can ever receive, and a field that is null exactly when a head would use it reads as available. A countdown belongs to the head, ticking off takeoverTimeoutMs"
        );
        assert!(view["takeoverTimeoutMs"].is_i64(), "the head needs the duration to tick off, so removing the countdown must not remove what a countdown is built from");
    }

    #[test]
    fn a_vehicle_that_has_not_said_who_is_flying_it_is_not_a_vehicle_flown_by_someone_else() {
        let silent = operator_control_view(&Station { known: false, holder: Some(0), takeover: false, own_id: Some(250) }, &[]);
        assert_eq!(silent["inControl"], Value::Null, "before any CONTROL_STATUS arrives gcsMain is 0 and both flags are false, which is byte-identical to another station holding it with takeover denied - firstControlStatusReceived is the only thing that tells them apart");
        assert_eq!(silent["takeoverAllowed"], Value::Null);
        assert_eq!(silent["holderSystemId"], Value::Null);
        assert!(silent["reason"].as_str().unwrap().contains("not said"));

        let ours = operator_control_view(&Station { known: true, holder: Some(250), takeover: false, own_id: Some(250) }, &[]);
        assert_eq!(ours["inControl"], true, "being in control is gcsMain matching THIS ground station's own MAVLink system id, a GCS-side setting - comparing against the vehicle's id would be wrong on every non-default station");
        assert_eq!(ours["reason"], "");

        let theirs = operator_control_view(&Station { known: true, holder: Some(42), takeover: true, own_id: Some(250) }, &[]);
        assert_eq!(theirs["inControl"], false);
        assert_eq!(theirs["holderSystemId"], 42);
        assert_eq!(theirs["takeoverAllowed"], true);
        assert!(theirs["reason"].as_str().unwrap().contains("Another ground station"));
    }
}
