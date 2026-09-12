use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.sysidInControl",
    "vehicle.gcsControlStatusFlags_SystemManager",
    "vehicle.gcsControlStatusFlags_TakeoverAllowed",
    "vehicle.firstControlStatusReceived",
    "vehicle.sendControlRequestAllowed",
    "settings.mavlinkSettings.gcsMavlinkSystemID",
];

const FIELDS: &str = "sysidInControl,gcsControlStatusFlags_SystemManager,gcsControlStatusFlags_TakeoverAllowed,firstControlStatusReceived,sendControlRequestAllowed,operatorControlTakeoverTimeoutMsecs,requestOperatorControlRemainingMsecs";

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
    let holder = integer(&vehicle, "sysidInControl");
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
        "remainingMs": integer(&vehicle, "requestOperatorControlRemainingMsecs").filter(|left| *left >= 0),
        "reason": match (known, holder.zip(ours).map(|(h, o)| h == o)) {
            (false, _) => "This vehicle has not said who is flying it.",
            (true, Some(true)) => "",
            (true, _) => "Another ground station is flying this vehicle.",
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Station {
        known: bool,
        holder: i64,
        takeover: bool,
    }

    impl Backend for Station {
        fn get(&self, path: &str) -> String {
            match path {
                "settings.mavlinkSettings.gcsMavlinkSystemID.rawValue" => json!({ "kind": "value", "value": 250 }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "vehicle" => json!({
                    "kind": "object",
                    "sysidInControl": self.holder,
                    "gcsControlStatusFlags_SystemManager": true,
                    "gcsControlStatusFlags_TakeoverAllowed": self.takeover,
                    "firstControlStatusReceived": self.known,
                    "sendControlRequestAllowed": true,
                    "operatorControlTakeoverTimeoutMsecs": 10000,
                    "requestOperatorControlRemainingMsecs": 4200,
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
    fn a_vehicle_that_has_not_said_who_is_flying_it_is_not_a_vehicle_flown_by_someone_else() {
        let silent = operator_control_view(&Station { known: false, holder: 0, takeover: false }, &[]);
        assert_eq!(silent["inControl"], Value::Null, "before any CONTROL_STATUS arrives sysidInControl is 0 and both flags are false, which is byte-identical to another station holding it with takeover denied - firstControlStatusReceived is the only thing that tells them apart");
        assert_eq!(silent["takeoverAllowed"], Value::Null);
        assert_eq!(silent["holderSystemId"], Value::Null);
        assert!(silent["reason"].as_str().unwrap().contains("not said"));

        let ours = operator_control_view(&Station { known: true, holder: 250, takeover: false }, &[]);
        assert_eq!(ours["inControl"], true, "being in control is sysidInControl matching THIS ground station's own MAVLink system id, a GCS-side setting - comparing against the vehicle's id would be wrong on every non-default station");
        assert_eq!(ours["reason"], "");

        let theirs = operator_control_view(&Station { known: true, holder: 42, takeover: true }, &[]);
        assert_eq!(theirs["inControl"], false);
        assert_eq!(theirs["holderSystemId"], 42);
        assert_eq!(theirs["takeoverAllowed"], true);
        assert!(theirs["reason"].as_str().unwrap().contains("Another ground station"));
    }
}
