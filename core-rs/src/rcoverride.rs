use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

const CONTROLS: &str = "settings.flyViewSettings.rcControls.rawValue";
const CHANNELS: std::ops::RangeInclusive<i64> = 1..=18;
const PWM: std::ops::RangeInclusive<i64> = 800..=2200;

fn configured_channels(backend: &dyn Backend) -> Vec<i64> {
    let text = object(&backend.get(CONTROLS)).get("value").and_then(Value::as_str).unwrap_or("[]").to_string();
    serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .map(|controls| controls.iter().filter_map(|c| c.get("channel")?.as_i64()).collect())
        .unwrap_or_default()
}

fn connected(backend: &dyn Backend) -> bool {
    flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable")
}

fn override_refusal(channel: Option<i64>, pwm: Option<i64>, connected: bool, configured: &[i64]) -> Option<(&'static str, String)> {
    let (Some(channel), Some(pwm)) = (channel, pwm) else {
        return Some(("malformed", "An RC override takes a channel number and a PWM value.".to_string()));
    };
    match () {
        _ if !connected => Some(("noVehicle", "No vehicle is connected.".to_string())),
        _ if !CHANNELS.contains(&channel) => Some(("noSuchChannel", format!("RC channels run from 1 to {}.", CHANNELS.end()))),
        _ if !configured.contains(&channel) => Some(("notConfigured", format!("No on-screen control is set up for channel {channel}."))),
        _ if !PWM.contains(&pwm) => Some(("pwmOutOfRange", format!("A PWM value runs from {} to {}.", PWM.start(), PWM.end()))),
        _ => None,
    }
}

pub fn set(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let whole = |i: usize| given.get(i).and_then(Value::as_f64).filter(|v| v.fract() == 0.0).map(|v| v as i64);
    let (channel, pwm) = (whole(0), whole(1));
    if let Some((token, reason)) = override_refusal(channel, pwm, connected(backend), &configured_channels(backend)) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let dispatched = flag(&object(&backend.invoke(path, &json!([channel, pwm]).to_string())), "ok");
    json!({ "ok": dispatched, "refusal": Value::Null, "reason": match dispatched { true => Value::Null, false => json!("The vehicle was not sent the override.") } })
}

pub fn clear(backend: &dyn Backend, path: &str) -> Value {
    if !connected(backend) {
        return json!({ "ok": false, "refusal": "noVehicle", "reason": "No vehicle is connected." });
    }
    let dispatched = flag(&object(&backend.invoke(path, "[]")), "ok");
    json!({ "ok": dispatched, "refusal": Value::Null, "reason": match dispatched { true => Value::Null, false => json!("The vehicle was not sent the release.") } })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_override_goes_only_to_a_channel_the_operator_put_a_control_on() {
        let configured = [6, 7];
        let token = |channel, pwm, up| override_refusal(Some(channel), Some(pwm), up, &configured).map(|r| r.0);
        assert_eq!(token(6, 1500, true), None);
        assert_eq!(token(3, 1500, true), Some("notConfigured"), "only the on-screen controls send overrides, so a request for a channel none of them is on - throttle on most airframes is 3 - is a caller's bug, and Vehicle::setRcChannelOverride would hold it every 200 ms until released");
        assert_eq!(token(19, 1500, true), Some("noSuchChannel"), "setRcChannelOverride answers a channel past 18 with a log warning only");
        assert_eq!(token(6, 2500, true), Some("pwmOutOfRange"), "and clamps a PWM outside 800..2200 without a word");
        assert_eq!(token(6, 1500, false), Some("noVehicle"));
        assert_eq!(override_refusal(None, Some(1500), true, &configured).map(|r| r.0), Some("malformed"));

        struct Vehicle(&'static str, bool);
        impl Backend for Vehicle {
            fn get(&self, _p: &str) -> String { json!({ "kind": "value", "value": self.0 }).to_string() }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "object", "activeVehicleAvailable": self.1 }).to_string() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true }).to_string() }
            fn watch(&self, _p: &[String]) {}
        }
        let rig = Vehicle(r#"[{"label":"Gimbal","channel":6,"type":"slider"}]"#, true);
        assert_eq!(set(&rig, "vehicle.setRcChannelOverride", "[6, 1600]")["ok"], true);
        assert_eq!(set(&rig, "vehicle.setRcChannelOverride", "[7, 1600]")["refusal"], "notConfigured");
        assert_eq!(clear(&rig, "vehicle.clearRcChannelOverrides")["ok"], true, "a release is never refused while a vehicle is there");
        assert_eq!(set(&Vehicle("not json", true), "vehicle.setRcChannelOverride", "[6, 1600]")["refusal"], "notConfigured", "an unreadable control list configures nothing");
    }
}
