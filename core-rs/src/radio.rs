use serde_json::{Value, json};

use crate::read::{flag, integer, object, text};
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
            let mapped = flag(&cal, &format!("{key}ChannelMapped"));
            let pwm = integer(&cal, &format!("{key}ChannelRCValue")).unwrap_or(0);
            let value_text = match (mapped, pwm > 0) {
                (false, _) => "Not mapped".to_string(),
                (true, true) => pwm.to_string(),
                (true, false) => ABSENT.to_string(),
            };
            json!({ "key": key, "title": title, "mapped": mapped, "value": pwm, "valueText": value_text, "fraction": fraction(pwm), "reversed": integer(&cal, &format!("{key}ChannelReversed")).unwrap_or(0) != 0 })
        })
        .collect();
    let cancel_enabled = flag(&cal, "cancelEnabled");
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
        "nextEnabled": flag(&cal, "nextEnabled"),
        "cancelEnabled": cancel_enabled,
        "skipEnabled": flag(&cal, "skipEnabled"),
        "transmitterMode": integer(&cal, "transmitterMode").unwrap_or(2),
        "channels": channels,
        "sticks": sticks,
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
        let none = radio_view(&Fake(json!({ "kind": "null" })), &[]);
        assert_eq!(none["connected"], false);
        assert_eq!(none["summary"], "No vehicle is connected.");
        assert_eq!(none["sticks"].as_array().unwrap().len(), 4);
    }
}
