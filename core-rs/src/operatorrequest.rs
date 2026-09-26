use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

const TIMEOUT: &str = "settings.flyViewSettings.requestControlTimeout";
const ALLOW_TAKEOVER: &str = "settings.flyViewSettings.requestControlAllowTakeover.rawValue";
const TIMEOUT_BOUNDS: (i64, i64) = (3, 60);

fn timeout_setting(backend: &dyn Backend) -> (i64, (i64, i64)) {
    let fact = crate::control::decode(&object(&backend.get(TIMEOUT)), TIMEOUT);
    let bound = |key: &str, fallback: i64| fact.get(key).and_then(Value::as_f64).map_or(fallback, |v| v as i64);
    let bounds = (bound("min", TIMEOUT_BOUNDS.0), bound("max", TIMEOUT_BOUNDS.1));
    (fact.get("value").and_then(Value::as_f64).map_or(10, |v| v as i64), bounds)
}

fn request_refusal(view: &Value, timeout: i64, bounds: (i64, i64)) -> Option<(&'static str, String)> {
    match () {
        _ if view["available"] != true => Some(("noVehicle", "No vehicle is connected.".to_string())),
        _ if view["inControl"] == true => Some(("alreadyInControl", "This ground station is already flying this vehicle.".to_string())),
        _ if view["requestAllowed"] == false => Some(("pending", "The last request is still waiting for an answer.".to_string())),
        _ if timeout != 0 && !(bounds.0..=bounds.1).contains(&timeout) => Some(("timeoutOutOfRange", format!("A request waits 0, or {} to {} seconds.", bounds.0, bounds.1))),
        _ => None,
    }
}

pub fn request(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let view = crate::operatorcontrol::operator_control_view(backend, &[]);
    let (setting, bounds) = timeout_setting(backend);
    let allow_takeover = given.get(0).and_then(Value::as_bool).unwrap_or_else(|| flag(&object(&backend.get(ALLOW_TAKEOVER)), "value"));
    let timeout = given.get(1).and_then(Value::as_f64).filter(|v| v.fract() == 0.0).map(|v| v as i64).unwrap_or(match view["takeoverAllowed"] == true {
        true => 0,
        false => setting,
    });
    if let Some((token, reason)) = request_refusal(&view, timeout, bounds) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let dispatched = flag(&object(&backend.invoke(path, &json!([allow_takeover, timeout]).to_string())), "ok");
    json!({
        "ok": dispatched,
        "refusal": Value::Null,
        "allowTakeover": allow_takeover,
        "timeoutSeconds": timeout,
        "reason": match dispatched { true => Value::Null, false => json!("The vehicle was not sent the request.") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_control_request_is_not_sent_twice_or_to_a_vehicle_this_station_already_flies() {
        let waiting = json!({ "available": true, "inControl": false, "requestAllowed": true });
        let token = |view: &Value, timeout| request_refusal(view, timeout, TIMEOUT_BOUNDS).map(|r| r.0);
        assert_eq!(token(&waiting, 10), None);
        assert_eq!(token(&waiting, 0), None, "zero is the no-wait request a head sends when the holder allows takeover");
        assert_eq!(token(&json!({ "available": true, "inControl": false, "requestAllowed": false }), 10), Some("pending"), "requestOperatorControl never reads sendControlRequestAllowed, so a second tap sent a second request while the first was still being answered");
        assert_eq!(token(&json!({ "available": true, "inControl": true, "requestAllowed": true }), 10), Some("alreadyInControl"));
        assert_eq!(token(&waiting, 90), Some("timeoutOutOfRange"), "a timeout outside the setting's bounds is replaced by the default without a word, so the countdown a head draws is not the one sent");
        assert_eq!(token(&json!({ "available": false }), 10), Some("noVehicle"));

        struct Vehicle(bool);
        impl Backend for Vehicle {
            fn get(&self, p: &str) -> String {
                match p {
                    TIMEOUT => json!({ "kind": "fact", "value": 15, "rawValue": 15, "min": 3, "max": 60 }),
                    _ => json!({ "kind": "value", "value": false }),
                }
                .to_string()
            }
            fn get_fields(&self, _p: &str, _f: &str) -> String {
                json!({ "kind": "object", "gcsMain": 7, "firstControlStatusReceived": true, "sendControlRequestAllowed": true, "gcsControlStatusFlags_TakeoverAllowed": self.0, "gcsMavlinkSystemID": 255, "activeVehicleAvailable": true }).to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true }).to_string() }
            fn watch(&self, _p: &[String]) {}
        }
        assert_eq!(request(&Vehicle(false), "vehicle.requestOperatorControl", "[]")["timeoutSeconds"], 15, "with no timeout given the core applies the rule the Android head carried: the setting, unless the holder allows takeover");
        assert_eq!(request(&Vehicle(true), "vehicle.requestOperatorControl", "[]")["timeoutSeconds"], 0);
    }
}
