use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

const CONTROLS: &str = "settings.flyViewSettings.rcControls.rawValue";
const CHANNELS: std::ops::RangeInclusive<i64> = 1..=18;
const PWM: std::ops::RangeInclusive<i64> = 800..=2200;

const SWITCH_POSITIONS: [i64; 3] = [1000, 1500, 2000];
const SWITCH_ON_ABOVE: i64 = 1500;

pub const DEPS: &[&str] = &["settings.flyViewSettings.rcControls.rawValue", "vehicles.activeVehicleAvailable", "vehicle.id", "vehicle.rcChannelValues", "vehicle.rcChannelOverrides"];

const CAMERA_CHANNELS: [&str; 5] = ["gimbalTiltChannel", "gimbalPanChannel", "cameraZoomChannel", "cameraLightChannel", "cameraRecordChannel"];

fn configured_channels(backend: &dyn Backend) -> Vec<i64> {
    let text = backend.value(CONTROLS).get("value").and_then(Value::as_str).unwrap_or("[]").to_string();
    let controls: Vec<i64> = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .map(|controls| controls.iter().filter_map(|c| c.get("channel")?.as_i64()).collect())
        .unwrap_or_default();
    let camera = CAMERA_CHANNELS.iter().filter_map(|name| crate::read::value_number(&backend.value(&format!("settings.flyViewSettings.{name}.rawValue"))).map(|c| c as i64).filter(|c| *c > 0));
    controls.into_iter().chain(camera).collect()
}

fn connected(backend: &dyn Backend) -> bool {
    flag(&backend.value_fields("vehicles", "activeVehicleAvailable"), "activeVehicleAvailable")
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

fn switch_position(pwm: i64) -> usize {
    (0..SWITCH_POSITIONS.len()).min_by_key(|at| (pwm - SWITCH_POSITIONS[*at]).abs()).unwrap_or(1)
}

fn control_state(control: &Value, live: &[i64], held: &serde_json::Map<String, Value>) -> Option<Value> {
    let channel = control.get("channel").and_then(Value::as_i64).filter(|channel| CHANNELS.contains(channel))?;
    let kind = control.get("type").and_then(Value::as_str).filter(|kind| ["slider", "button", "switch3", "momentary"].contains(kind))?;
    let label = control.get("label").and_then(Value::as_str).filter(|label| !label.trim().is_empty()).map_or_else(|| format!("CH{channel}"), str::to_string);
    let ours = held.get(&channel.to_string()).and_then(Value::as_i64);
    let pwm = ours.or_else(|| live.get(channel as usize - 1).copied().filter(|pwm| PWM.contains(pwm)));
    Some(json!({
        "label": label,
        "channel": channel,
        "type": kind,
        "pwm": pwm,
        "held": ours.is_some(),
        "position": pwm.map(switch_position),
        "on": pwm.map(|pwm| pwm > SWITCH_ON_ABOVE),
    }))
}

pub fn controls_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let text = backend.value(CONTROLS).get("value").and_then(Value::as_str).unwrap_or("[]").to_string();
    let controls: Vec<Value> = serde_json::from_str::<Value>(&text).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
    let vehicle = backend.value_fields("vehicle", "rcChannelValues,rcChannelOverrides");
    let live: Vec<i64> = match connected(backend) {
        true => vehicle.get("rcChannelValues").and_then(Value::as_array).map(|values| values.iter().filter_map(Value::as_i64).collect()).unwrap_or_default(),
        false => Vec::new(),
    };
    let held = vehicle.get("rcChannelOverrides").and_then(Value::as_object).cloned().unwrap_or_default();
    json!({
        "kind": "object",
        "class": "RcControls",
        "controls": controls.iter().filter_map(|control| control_state(control, &live, &held)).collect::<Vec<_>>(),
        "holding": !held.is_empty(),
    })
}

pub(crate) fn on_core(backend: &dyn Backend, mut action: Value) -> Option<bool> {
    crate::vehiclefacade::switched_on().then_some(())?;
    action["vehicle"] = backend.value("vehicle.id").get("value").cloned().filter(Value::is_u64)?;
    backend.core_guided(&action).map(|started| started.is_ok())
}

pub fn set(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let whole = |i: usize| given.get(i).and_then(Value::as_f64).filter(|v| v.fract() == 0.0).map(|v| v as i64);
    let (channel, pwm) = (whole(0), whole(1));
    if let Some((token, reason)) = override_refusal(channel, pwm, connected(backend), &configured_channels(backend)) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let action = json!({ "action": "rcOverride", "channel": channel, "pwm": pwm });
    let dispatched = on_core(backend, action).unwrap_or_else(|| flag(&object(&backend.invoke(path, &json!([channel, pwm]).to_string())), "ok"));
    json!({ "ok": dispatched, "refusal": Value::Null, "reason": match dispatched { true => Value::Null, false => json!("The vehicle was not sent the override.") } })
}

pub fn clear(backend: &dyn Backend, path: &str) -> Value {
    if !connected(backend) {
        return json!({ "ok": false, "refusal": "noVehicle", "reason": "No vehicle is connected." });
    }
    let dispatched = on_core(backend, json!({ "action": "rcRelease" })).unwrap_or_else(|| flag(&object(&backend.invoke(path, "[]")), "ok"));
    json!({ "ok": dispatched, "refusal": Value::Null, "reason": match dispatched { true => Value::Null, false => json!("The vehicle was not sent the release.") } })
}

pub fn release(backend: &dyn Backend, path: &str, args: &str) -> Value {
    if !connected(backend) {
        return json!({ "ok": false, "refusal": "noVehicle", "reason": "No vehicle is connected." });
    }
    let channel = serde_json::from_str::<Value>(args).ok().and_then(|given| given.get(0).and_then(Value::as_i64));
    let Some(channel) = channel.filter(|c| (1..=18).contains(c)) else {
        return json!({ "ok": false, "refusal": "noSuchChannel", "reason": "RC channels run from 1 to 18." });
    };
    let dispatched = on_core(backend, json!({ "action": "rcReleaseChannel", "channel": channel })).unwrap_or_else(|| flag(&object(&backend.invoke(path, &json!([channel]).to_string())), "ok"));
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
        assert_eq!(release(&rig, "vehicle.releaseRcChannelOverride", "[6]")["ok"], true);
        assert_eq!(release(&rig, "vehicle.releaseRcChannelOverride", "[19]")["refusal"], "noSuchChannel");
        assert_eq!(set(&Vehicle("not json", true), "vehicle.setRcChannelOverride", "[6, 1600]")["refusal"], "notConfigured", "an unreadable control list configures nothing");
    }

    struct Rig {
        controls: &'static str,
        connected: bool,
        live: Value,
        held: Value,
    }

    impl Backend for Rig {
        fn get(&self, _p: &str) -> String { json!({ "kind": "value", "value": self.controls }).to_string() }
        fn get_fields(&self, path: &str, _f: &str) -> String {
            match path {
                "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": self.connected }),
                _ => json!({ "kind": "object", "rcChannelValues": self.live, "rcChannelOverrides": self.held }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    const PANEL: &str = r#"[{"label":"Mode","channel":5,"type":"switch3"},{"label":"","channel":7,"type":"button"},{"label":"Tilt","channel":8,"type":"slider"},{"label":"Odd","channel":9,"type":"dial"}]"#;

    #[test]
    fn the_on_screen_controls_start_where_the_radio_left_each_channel() {
        let radio = Rig { controls: PANEL, connected: true, live: json!([1500, 1500, 1000, 1500, 1985, 1000, 2012, 1320]), held: json!({}) };
        let view = controls_view(&radio, &[]);
        let controls = view["controls"].as_array().unwrap();
        assert_eq!(controls.len(), 3, "a control of a type no head draws is left out, as the heads' own parser does");
        assert_eq!((&controls[0]["position"], &controls[0]["held"]), (&json!(2), &json!(false)), "the mode switch the radio left high shows high when the pilot moves to the 4G link, not the middle every widget used to start in");
        assert_eq!((&controls[1]["label"], &controls[1]["on"]), (&json!("CH7"), &json!(true)), "a radio switch at 2012 µs is on");
        assert_eq!(controls[2]["pwm"], 1320);
        assert_eq!(view["holding"], false);
    }

    #[test]
    fn a_channel_this_tablet_holds_shows_its_own_value_and_an_unknown_one_shows_nothing() {
        let holding = Rig { controls: PANEL, connected: true, live: json!([1500, 1500, 1000, 1500, 1985]), held: json!({ "5": 1000 }) };
        let view = controls_view(&holding, &[]);
        assert_eq!((&view["controls"][0]["position"], &view["controls"][0]["held"]), (&json!(0), &json!(true)), "the telemetry lags the tap by a frame or more, so the held value wins");
        assert_eq!(view["controls"][1]["pwm"], Value::Null, "channel 7 was never reported, so the widget must not pretend to know it");
        assert_eq!(view["holding"], true);
        let no_receiver = Rig { controls: PANEL, connected: true, live: json!([0, 0, 0, 0, 0, 0, 0, 0]), held: json!({}) };
        assert_eq!(controls_view(&no_receiver, &[])["controls"][0]["position"], Value::Null, "ArduPilot reports 0 for a channel no receiver ever drove");
        let gone = Rig { controls: PANEL, connected: false, live: json!([1500, 1500, 1000, 1500, 1985]), held: json!({}) };
        assert_eq!(controls_view(&gone, &[])["controls"][0]["pwm"], Value::Null);
    }
}
