use serde_json::{Value, json};

use crate::read::{integer, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.apmFirmware", "radioCal"];

pub const SLOTS: usize = 6;
pub const CHANNEL_OPTIONS: usize = 11;
const THRESHOLDS: [i64; 5] = [1230, 1360, 1490, 1620, 1749];
const OPTION_ON_ABOVE: i64 = 1800;
const DEFAULT_CHANNEL_INDEX: i64 = 4;
const NO_RC: i64 = -1;

pub fn slot_for(pwm: Option<i64>) -> usize {
    match pwm {
        None => 0,
        Some(NO_RC) => 0,
        Some(value) => THRESHOLDS.iter().position(|threshold| value <= *threshold).map(|index| index + 1).unwrap_or(SLOTS),
    }
}

pub fn option_enabled(pwm: Option<i64>) -> bool {
    pwm.is_some_and(|value| value > OPTION_ON_ABOVE)
}

fn names(rover: bool) -> (String, String) {
    match rover {
        true => ("MODE_CH".to_string(), "MODE".to_string()),
        false => ("FLTMODE_CH".to_string(), "FLTMODE".to_string()),
    }
}

fn parameter(backend: &dyn Backend, name: &str) -> Option<Value> {
    let fact = object(&backend.get(&format!("vehicle.parameterManager.getParameter(-1,{name})")));
    (fact.get("kind").and_then(Value::as_str) == Some("fact")).then_some(fact)
}

pub fn slots_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", "apmFirmware,vehicleType"));
    if vehicle.get("kind").and_then(Value::as_str) != Some("object") {
        return json!({ "kind": "object", "class": "ModeSlots", "available": false, "slots": [], "liveSlot": 0, "channel": 0, "reason": "No vehicle is connected." });
    }
    let rover = text(&vehicle, "vehicleType") == "Rover";
    let (channel_name, slot_prefix) = names(rover);

    let channel_index = parameter(backend, &channel_name).and_then(|fact| integer(&fact, "rawValue")).map(|value| value - 1).unwrap_or(DEFAULT_CHANNEL_INDEX);
    let radio = object(&backend.get_fields("radioCal", "rcValues,channelCount"));
    let pwm: Vec<i64> = radio.get("rcValues").and_then(Value::as_array).map(|values| values.iter().filter_map(Value::as_i64).collect()).unwrap_or_default();
    let reachable = channel_index >= 0 && (channel_index as usize) < pwm.len();
    let live = match reachable {
        true => slot_for(pwm.get(channel_index as usize).copied()),
        false => 0,
    };

    let slots: Vec<Value> = (0..SLOTS)
        .map(|index| {
            let named = parameter(backend, &format!("{slot_prefix}{}", index + 1));
            json!({
                "slot": index + 1,
                "mode": named.as_ref().map(|fact| text(fact, "enumOrValueString")).filter(|mode| !mode.is_empty()),
                "live": live == index + 1,
            })
        })
        .collect();

    let options: Vec<Value> = (0..CHANNEL_OPTIONS)
        .map(|index| json!({ "channel": index + 6, "enabled": option_enabled(pwm.get(index + 5).copied()) }))
        .collect();

    json!({
        "kind": "object",
        "class": "ModeSlots",
        "available": true,
        "channel": channel_index + 1,
        "channelPwm": pwm.get(channel_index.max(0) as usize).copied(),
        "liveSlot": live,
        "slots": slots,
        "channelOptions": options,
        "reason": match (reachable, live) {
            (false, _) => "The transmitter is not sending on the mode channel.",
            (_, 0) => "No mode is selected on the transmitter.",
            _ => "",
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pwm_falls_into_the_slot_the_vehicle_would_pick() {
        assert_eq!(slot_for(Some(1000)), 1, "anything at or below the first threshold is the first slot");
        assert_eq!(slot_for(Some(1230)), 1, "the threshold belongs to the slot below it");
        assert_eq!(slot_for(Some(1231)), 2);
        assert_eq!(slot_for(Some(1360)), 2);
        assert_eq!(slot_for(Some(1490)), 3);
        assert_eq!(slot_for(Some(1620)), 4);
        assert_eq!(slot_for(Some(1749)), 5);
        assert_eq!(slot_for(Some(1750)), 6, "above every threshold is the last slot rather than none");
        assert_eq!(slot_for(Some(2000)), 6);
    }

    #[test]
    fn a_channel_with_no_transmitter_selects_nothing() {
        assert_eq!(slot_for(Some(-1)), 0, "minus one is the value a channel carries when nothing is transmitting on it");
        assert_eq!(slot_for(None), 0);
        assert_eq!(slot_for(Some(0)), 1, "zero is a real reading and falls in the first band, which is not the same as no reading");
    }

    #[test]
    fn a_channel_option_turns_on_above_its_threshold() {
        assert!(!option_enabled(Some(1800)), "the threshold itself is not above it");
        assert!(option_enabled(Some(1801)));
        assert!(!option_enabled(Some(-1)));
        assert!(!option_enabled(None));
    }

    struct Fake {
        rover: bool,
        pwm: Vec<i64>,
        channel: Option<i64>,
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            match path.strip_prefix("vehicle.parameterManager.getParameter(-1,").and_then(|rest| rest.strip_suffix(')')) {
                Some(name) if name == "FLTMODE_CH" || name == "MODE_CH" => match self.channel {
                    Some(channel) => json!({ "kind": "fact", "rawValue": channel }).to_string(),
                    None => json!({ "kind": "null" }).to_string(),
                },
                Some(name) if name.starts_with("FLTMODE") || name.starts_with("MODE") => {
                    json!({ "kind": "fact", "rawValue": 0, "enumOrValueString": format!("Mode {}", name.chars().last().unwrap()) }).to_string()
                }
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "vehicle" => json!({ "kind": "object", "apmFirmware": true, "vehicleType": if self.rover { "Rover" } else { "Multi-Rotor" } }).to_string(),
                "radioCal" => json!({ "kind": "object", "rcValues": self.pwm, "channelCount": self.pwm.len() }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn channels(mode_pwm: i64) -> Vec<i64> {
        (0..8).map(|index| if index == 4 { mode_pwm } else { 1500 }).collect()
    }

    #[test]
    fn the_live_slot_is_marked_on_the_slot_the_transmitter_selects() {
        let view = slots_view(&Fake { rover: false, pwm: channels(1500), channel: Some(5) }, &[]);
        assert_eq!(view["liveSlot"], 4, "fifteen hundred sits in the fourth band");
        let slots = view["slots"].as_array().unwrap();
        assert_eq!(slots.len(), SLOTS);
        assert_eq!(slots.iter().filter(|slot| slot["live"] == true).count(), 1, "exactly one slot is live, which is the whole question the screen exists to answer");
        assert_eq!(slots[3]["live"], true);
        assert_eq!(slots[3]["mode"], "Mode 4");
        assert_eq!(view["channel"], 5);
    }

    #[test]
    fn a_vehicle_with_no_mode_channel_parameter_falls_back_to_channel_five() {
        let view = slots_view(&Fake { rover: false, pwm: channels(1200), channel: None }, &[]);
        assert_eq!(view["channel"], 5, "the Qt controller defaults to the fifth channel when the parameter is absent, and a head must not guess differently");
        assert_eq!(view["liveSlot"], 1);
    }

    #[test]
    fn a_mode_channel_past_the_last_one_the_transmitter_sends_selects_nothing() {
        let view = slots_view(&Fake { rover: false, pwm: channels(1500), channel: Some(12) }, &[]);
        assert_eq!(view["liveSlot"], 0);
        assert!(view["slots"].as_array().unwrap().iter().all(|slot| slot["live"] == false), "no slot may be marked live when the channel carrying the selection is not being received");
        assert!(view["reason"].as_str().unwrap().contains("not sending"));
    }

    #[test]
    fn a_rover_reads_its_own_parameter_names() {
        let rover = slots_view(&Fake { rover: true, pwm: channels(1500), channel: Some(5) }, &[]);
        assert_eq!(rover["slots"][0]["mode"], "Mode 1", "a rover names its slots MODE1 to MODE6 and its channel MODE_CH, and reading FLTMODE on one would find nothing");
        assert_eq!(rover["liveSlot"], 4);
    }

    #[test]
    fn a_channel_option_is_reported_for_every_channel_the_screen_offers() {
        let mut pwm = channels(1500);
        pwm.resize(16, 1000);
        pwm[6] = 1900;
        let view = slots_view(&Fake { rover: false, pwm, channel: Some(5) }, &[]);
        let options = view["channelOptions"].as_array().unwrap();
        assert_eq!(options.len(), CHANNEL_OPTIONS);
        assert_eq!(options[0]["channel"], 6);
        assert_eq!(options[1]["channel"], 7);
        assert_eq!(options[1]["enabled"], true, "channel seven is the second option and it is the one held high");
        assert_eq!(options[0]["enabled"], false);
    }

    #[test]
    fn no_slot_is_ever_marked_live_unless_the_view_is_answering() {
        let states = [
            slots_view(&Fake { rover: false, pwm: Vec::new(), channel: Some(5) }, &[]),
            slots_view(&Fake { rover: false, pwm: channels(-1), channel: Some(5) }, &[]),
            slots_view(&Fake { rover: false, pwm: channels(1500), channel: Some(12) }, &[]),
            slots_view(&Fake { rover: false, pwm: channels(1500), channel: Some(-4) }, &[]),
        ];
        states.iter().for_each(|view| {
            assert_eq!(view["liveSlot"], 0);
            assert!(
                view["slots"].as_array().unwrap().iter().all(|slot| slot["live"] == false),
                "a head reading a slot's own flag must never need to check availability as well; if the two can disagree, a head that trusts one paints from a partial read"
            );
        });
    }

    #[test]
    fn no_vehicle_is_an_answer_rather_than_six_empty_slots() {
        struct Nothing;
        impl Backend for Nothing {
            fn get(&self, _p: &str) -> String { String::new() }
            fn get_fields(&self, _p: &str, _f: &str) -> String { String::new() }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let view = slots_view(&Nothing, &[]);
        assert_eq!(view["available"], false);
        assert_eq!(view["liveSlot"], 0);
        assert!(view["slots"].as_array().unwrap().is_empty());
    }
}
