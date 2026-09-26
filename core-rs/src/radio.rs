use serde_json::{Value, json};

use crate::read::{integer, object, text, truthy};
use crate::router::Backend;

pub const DEPS: &[&str] = &["radioCal", "vehicles.activeVehicleAvailable"];

const LOW_PWM: f64 = 1000.0;
const HIGH_PWM: f64 = 2000.0;
const STICKS: &[(&str, &str)] = &[("roll", "Roll"), ("pitch", "Pitch"), ("yaw", "Yaw"), ("throttle", "Throttle")];
const ABSENT: &str = "\u{2014}";

pub fn fraction(pwm: i64) -> f64 {
    match pwm > 0 {
        true => ((pwm as f64).clamp(LOW_PWM, HIGH_PWM) - LOW_PWM) / (HIGH_PWM - LOW_PWM),
        false => 0.0,
    }
}

pub fn summary(connected: bool, channel_count: i64, live: usize) -> String {
    match (connected, channel_count) {
        (false, _) => "No vehicle is connected.".to_string(),
        (true, 0) => "No transmitter is being heard.".to_string(),
        (true, n) => format!("{n} channel{} reported, {live} carrying a signal.", if n == 1 { "" } else { "s" }),
    }
}

pub fn shortfall(connected: bool, channel_count: i64, minimum: i64) -> String {
    match connected && channel_count > 0 && channel_count < minimum {
        true => format!("At least {minimum} channels are needed to fly; the transmitter reports {channel_count}."),
        false => String::new(),
    }
}

pub fn radio_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let cal = object(&backend.get("radioCal"));
    let connected = cal.get("kind").and_then(Value::as_str) == Some("object");
    let channel_count = integer(&cal, "channelCount").unwrap_or(0);
    let minimum = integer(&cal, "minChannelCount").unwrap_or(0);
    let channels: Vec<Value> = cal
        .get("rcValues")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .enumerate()
                .map(|(index, v)| {
                    let pwm = v.as_i64().unwrap_or(0);
                    json!({ "index": index, "label": (index + 1).to_string(), "value": pwm, "valueText": if pwm > 0 { pwm.to_string() } else { ABSENT.to_string() }, "fraction": fraction(pwm), "live": pwm > 0 })
                })
                .collect()
        })
        .unwrap_or_default();
    let live = channels.iter().filter(|c| c["live"] == true).count();
    let sticks: Vec<Value> = STICKS
        .iter()
        .map(|(key, title)| {
            let mapped = truthy(&cal, &format!("{key}ChannelMapped"));
            let pwm = integer(&cal, &format!("{key}ChannelRCValue")).unwrap_or(0);
            let value_text = match (mapped, pwm > 0) {
                (false, _) => "Not mapped".to_string(),
                (true, true) => pwm.to_string(),
                (true, false) => ABSENT.to_string(),
            };
            json!({ "key": key, "title": title, "mapped": mapped, "value": pwm, "valueText": value_text, "fraction": fraction(pwm), "reversed": truthy(&cal, &format!("{key}ChannelReversed")) })
        })
        .collect();
    let cancel_enabled = truthy(&cal, "cancelEnabled");
    json!({
        "kind": "object",
        "class": "Radio",
        "connected": connected,
        "channelCount": channel_count,
        "minimumChannels": minimum,
        "enoughChannels": channel_count >= minimum,
        "liveChannels": live,
        "summary": summary(connected, channel_count, live),
        "shortfall": shortfall(connected, channel_count, minimum),
        "calibrating": cancel_enabled,
        "statusText": text(&cal, "statusText"),
        "nextText": text(&cal, "nextText"),
        "nextEnabled": truthy(&cal, "nextEnabled"),
        "cancelEnabled": cancel_enabled,
        "skipEnabled": truthy(&cal, "skipEnabled"),
        "transmitterMode": integer(&cal, "transmitterMode").unwrap_or(2),
        "channels": channels,
        "sticks": sticks,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Next,
    Cancel,
    Skip,
}

#[derive(Clone, Copy, Debug, Default)]
struct Calibration {
    connected: bool,
    calibrating: bool,
    next_enabled: bool,
    cancel_enabled: bool,
    channels: i64,
    minimum: i64,
}

fn calibration(cal: &Value) -> Calibration {
    Calibration {
        connected: cal.get("kind").and_then(Value::as_str) == Some("object"),
        calibrating: truthy(cal, "calibrating") || truthy(cal, "cancelEnabled"),
        next_enabled: truthy(cal, "nextEnabled"),
        cancel_enabled: truthy(cal, "cancelEnabled"),
        channels: integer(cal, "channelCount").unwrap_or(0),
        minimum: integer(cal, "minChannelCount").unwrap_or(0),
    }
}

fn refusal(action: Action, state: Calibration) -> Option<(&'static str, String)> {
    match action {
        Action::Skip => Some(("unsupported", "This calibration has no step that can be skipped.".to_string())),
        _ if !state.connected => Some(("noVehicle", "No vehicle is connected.".to_string())),
        Action::Cancel if !state.cancel_enabled => Some(("idle", "No radio calibration is running.".to_string())),
        Action::Cancel => None,
        Action::Next if !state.calibrating && state.channels < state.minimum => Some(("tooFewChannels", format!("Detected {} channels. To operate the vehicle you need at least {}.", state.channels, state.minimum))),
        Action::Next if !state.next_enabled => Some(("waiting", "Follow the instruction on screen before continuing.".to_string())),
        Action::Next => None,
    }
}

pub fn act(backend: &dyn Backend, action: Action, path: &str) -> Value {
    if let Some((token, reason)) = refusal(action, calibration(&object(&backend.get("radioCal")))) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let dispatched = crate::read::flag(&object(&backend.invoke(path, "[]")), "ok");
    json!({
        "ok": dispatched,
        "refusal": Value::Null,
        "reason": match dispatched { true => Value::Null, false => json!("The radio calibration did not take the request.") },
    })
}

pub fn write_transmitter_mode(backend: &dyn Backend, path: &str, value: &str) -> Value {
    let asked = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value")?.as_f64());
    let Some(mode) = asked.filter(|m| m.fract() == 0.0 && (1.0..=4.0).contains(m)).map(|m| m as i64) else {
        return json!({ "ok": false, "result": false, "refusal": "outOfRange", "reason": "A transmitter mode is 1, 2, 3 or 4." });
    };
    if calibration(&object(&backend.get("radioCal"))).calibrating {
        return json!({ "ok": false, "result": false, "refusal": "calibrating", "reason": "Finish or cancel the calibration before changing the transmitter mode." });
    }
    let answered = crate::read::flag(&object(&backend.set(path, &json!({ "value": mode }).to_string())), "ok");
    json!({
        "ok": answered,
        "result": answered,
        "refusal": Value::Null,
        "mode": mode,
        "reason": match answered { true => Value::Null, false => json!("The radio calibration did not take the transmitter mode.") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(Value);
    impl Backend for Fake {
        fn get(&self, _p: &str) -> String { self.0.to_string() }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn pwm_maps_onto_the_thousand_to_two_thousand_band() {
        assert_eq!(fraction(0), 0.0);
        assert_eq!(fraction(1000), 0.0);
        assert_eq!(fraction(1500), 0.5);
        assert_eq!(fraction(2200), 1.0);
    }

    #[test]
    fn the_summary_and_shortfall_say_what_the_transmitter_reports() {
        assert_eq!(summary(false, 8, 8), "No vehicle is connected.");
        assert_eq!(summary(true, 0, 0), "No transmitter is being heard.");
        assert_eq!(summary(true, 1, 1), "1 channel reported, 1 carrying a signal.");
        assert_eq!(shortfall(true, 4, 5), "At least 5 channels are needed to fly; the transmitter reports 4.");
        assert_eq!(shortfall(true, 8, 5), "");
    }

    #[test]
    fn the_view_lists_channels_and_sticks() {
        let view = radio_view(&Fake(json!({
            "kind": "object", "channelCount": 3, "minChannelCount": 5, "rcValues": [1500, 0, 2000],
            "rollChannelMapped": true, "rollChannelRCValue": 1500, "rollChannelReversed": 1,
            "throttleChannelMapped": false, "cancelEnabled": true, "nextText": "Next",
        })), &[]);
        assert_eq!(view["connected"], true);
        assert_eq!(view["liveChannels"], 2);
        assert_eq!(view["channels"][1]["valueText"], "\u{2014}");
        assert_eq!(view["channels"][2]["fraction"], 1.0);
        assert_eq!(view["sticks"][0]["reversed"], true);
        assert_eq!(view["sticks"][3]["valueText"], "Not mapped");
        assert_eq!(view["calibrating"], true);
        assert_eq!(view["enoughChannels"], false);
        assert!(view["shortfall"].as_str().unwrap().starts_with("At least 5"));
        // The same view against the declaration upstream would have written if the typo in
        // RadioComponentController were corrected: bools where it currently sends numbers.
        let declared_bool = radio_view(&Fake(json!({
            "kind": "object", "channelCount": 3, "minChannelCount": 5, "rcValues": [1500, 0, 2000],
            "rollChannelMapped": true, "rollChannelRCValue": 1500, "rollChannelReversed": true,
            "throttleChannelMapped": false, "cancelEnabled": true, "nextText": "Next",
        })), &[]);
        assert_eq!(declared_bool["sticks"][0]["reversed"], true);
        assert_eq!(declared_bool["sticks"][3]["valueText"], "Not mapped");
        assert_eq!(declared_bool["calibrating"], true);

        // And the mapped flags read the same whichever way they are declared.
        let mapped_as_number = radio_view(&Fake(json!({
            "kind": "object", "channelCount": 3, "minChannelCount": 5, "rcValues": [1500, 0, 2000],
            "rollChannelMapped": 1, "rollChannelRCValue": 1500, "rollChannelReversed": 0,
            "cancelEnabled": 1, "nextText": "Next",
        })), &[]);
        assert_eq!(mapped_as_number["sticks"][0]["mapped"], true);
        assert_eq!(mapped_as_number["sticks"][0]["reversed"], false);
        assert_eq!(mapped_as_number["calibrating"], true);

        let none = radio_view(&Fake(json!({ "kind": "null" })), &[]);
        assert_eq!(none["connected"], false);
        assert_eq!(none["summary"], "No vehicle is connected.");
        assert_eq!(none["sticks"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn a_radio_button_that_would_do_nothing_says_why() {
        let ready = Calibration { connected: true, calibrating: false, next_enabled: true, cancel_enabled: false, channels: 8, minimum: 5 };
        let token = |action, state| refusal(action, state).map(|(t, _)| t);
        assert_eq!(token(Action::Next, ready), None);
        assert_eq!(token(Action::Next, Calibration { channels: 4, ..ready }), Some("tooFewChannels"), "nextButtonClicked reports this through showAppMessage, which no native head receives");
        assert!(refusal(Action::Next, Calibration { channels: 4, ..ready }).unwrap().1.contains("at least 5"));
        assert_eq!(token(Action::Next, Calibration { calibrating: true, channels: 4, next_enabled: true, ..ready }), None, "the channel count gates only the start, as _currentStep == -1 does");
        assert_eq!(token(Action::Next, Calibration { calibrating: true, next_enabled: false, ..ready }), Some("waiting"), "a step with no nextButtonFn ignores the click");
        assert_eq!(token(Action::Cancel, ready), Some("idle"));
        assert_eq!(token(Action::Cancel, Calibration { calibrating: true, cancel_enabled: true, ..ready }), None);
        assert_eq!(token(Action::Next, Calibration { connected: false, ..ready }), Some("noVehicle"));
        assert_eq!(token(Action::Skip, Calibration { calibrating: true, cancel_enabled: true, ..ready }), Some("unsupported"), "RemoteControlCalibrationController has no skipButtonClicked since the upstream merge, so a head invoking it reaches a method that does not exist");
    }

    #[test]
    fn a_transmitter_mode_outside_one_to_four_is_refused_rather_than_turned_into_two() {
        use std::cell::RefCell;
        struct Recording(Value, RefCell<Vec<String>>);
        impl Backend for Recording {
            fn get(&self, _p: &str) -> String { self.0.to_string() }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, v: &str) -> String {
                self.1.borrow_mut().push(v.to_string());
                json!({ "ok": true }).to_string()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true }).to_string() }
            fn watch(&self, _p: &[String]) {}
        }
        let idle = Recording(json!({ "kind": "object", "cancelEnabled": false }), RefCell::new(Vec::new()));
        [json!({ "value": 5 }), json!({ "value": 0 }), json!({ "value": 2.5 }), json!({ "value": "2" })].iter().for_each(|v| {
            assert_eq!(write_transmitter_mode(&idle, "radioCal.transmitterMode", &v.to_string())["refusal"], "outOfRange", "setTransmitterMode logs a warning and stores 2 for {v}, so the head's stick diagram changes to a mode nobody chose");
        });
        assert!(idle.1.borrow().is_empty());
        let taken = write_transmitter_mode(&idle, "radioCal.transmitterMode", r#"{"value":1}"#);
        assert_eq!((&taken["ok"], &taken["result"], &taken["mode"]), (&json!(true), &json!(true), &json!(1)));
        assert_eq!(idle.1.borrow().as_slice(), &[r#"{"value":1}"#.to_string()]);

        let running = Recording(json!({ "kind": "object", "cancelEnabled": true }), RefCell::new(Vec::new()));
        assert_eq!(write_transmitter_mode(&running, "radioCal.transmitterMode", r#"{"value":3}"#)["refusal"], "calibrating");
        assert!(running.1.borrow().is_empty());
        assert_eq!(act(&running, Action::Cancel, "radioCal.cancelButtonClicked")["ok"], true);
    }
}
