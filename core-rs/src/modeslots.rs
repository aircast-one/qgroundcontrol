use serde_json::{Value, json};

use crate::read::{object, result_flag, value_number};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.apmFirmware", "radioCal.rcValues"];

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

fn names(backend: &dyn Backend) -> (&'static str, &'static str) {
    match exists(backend, "MODE_CH") {
        true => ("MODE_CH", "MODE"),
        false => ("FLTMODE_CH", "FLTMODE"),
    }
}

fn exists(backend: &dyn Backend, name: &str) -> bool {
    result_flag(&backend.invoke("vehicle.parameterManager.parameterExists", &json!([-1, name]).to_string()))
}

fn parameter(backend: &dyn Backend, name: &str) -> Option<f64> {
    exists(backend, name).then(|| value_number(&backend.get(&format!("vehicle.parameterManager.getParameter(-1,{name}).rawValue")))).flatten()
}

fn parameter_text(backend: &dyn Backend, name: &str) -> Option<String> {
    let fact = object(&backend.get(&format!("vehicle.parameterManager.getParameter(-1,{name})")));
    match fact.get("kind").and_then(Value::as_str) == Some("fact") {
        true => fact.get("enumOrValueString").and_then(Value::as_str).filter(|mode| !mode.is_empty()).map(str::to_string),
        false => None,
    }
}

pub fn slots_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let vehicle = object(&backend.get_fields("vehicle", "apmFirmware"));
    if vehicle.get("kind").and_then(Value::as_str) != Some("object") {
        return json!({ "kind": "object", "class": "ModeSlots", "available": false, "slots": [], "liveSlot": 0, "channel": 0, "reason": "No vehicle is connected." });
    }
    let (channel_name, slot_prefix) = names(backend);
    if !exists(backend, &format!("{slot_prefix}1")) {
        return json!({ "kind": "object", "class": "ModeSlots", "available": false, "slots": [], "liveSlot": 0, "channel": 0, "reason": "This vehicle does not choose its flight modes from a transmitter channel." });
    }

    let channel_index = parameter(backend, channel_name).map(|value| value as i64 - 1).unwrap_or(DEFAULT_CHANNEL_INDEX);
    let radio = object(&backend.get_fields("radioCal", "rcValues,channelCount"));
    let pwm: Vec<i64> = radio.get("rcValues").and_then(Value::as_array).map(|values| values.iter().filter_map(Value::as_i64).collect()).unwrap_or_default();
    let reachable = channel_index >= 0 && (channel_index as usize) < pwm.len();
    let live = match reachable {
        true => slot_for(pwm.get(channel_index as usize).copied()),
        false => 0,
    };

    let slots: Vec<Value> = (0..SLOTS)
        .map(|index| {
            json!({
                "slot": index + 1,
                "mode": parameter_text(backend, &format!("{slot_prefix}{}", index + 1)),
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
        "channelPwm": pwm.get(channel_index as usize).copied(),
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
        parameters: Vec<(String, f64, String)>,
        pwm: Vec<i64>,
    }

    impl Fake {
        fn named(&self, wanted: &str) -> Option<&(String, f64, String)> {
            self.parameters.iter().find(|(name, _, _)| name == wanted)
        }
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            let Some(rest) = path.strip_prefix("vehicle.parameterManager.getParameter(-1,") else {
                return json!({ "kind": "null" }).to_string();
            };
            let (name, tail) = rest.split_once(')').unwrap_or((rest, ""));
            match self.named(name) {
                Some((_, value, spelled)) if tail == ".rawValue" => json!({ "kind": "value", "value": value }).to_string(),
                Some((name, value, spelled)) => json!({ "kind": "fact", "name": name, "value": value, "enumOrValueString": spelled }).to_string(),
                None => json!({ "kind": "value", "value": Value::Null, "found": false }).to_string(),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "vehicle" => json!({ "kind": "object", "apmFirmware": true }).to_string(),
                "radioCal" => json!({ "kind": "object", "rcValues": self.pwm, "channelCount": self.pwm.len() }).to_string(),
                _ => json!({ "kind": "null" }).to_string(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, path: &str, args: &str) -> String {
            if path != "vehicle.parameterManager.parameterExists" {
                return String::new();
            }
            let asked: Value = serde_json::from_str(args).unwrap_or(Value::Null);
            let name = asked.get(1).and_then(Value::as_str).unwrap_or_default();
            json!({ "ok": true, "result": self.named(name).is_some() }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    fn channels(mode_pwm: i64, channel: usize) -> Vec<i64> {
        (0..8).map(|index| if index == channel { mode_pwm } else { 1500 }).collect()
    }

    fn copter(channel: f64, pwm: Vec<i64>) -> Fake {
        let slots = (1..=SLOTS).map(|slot| (format!("FLTMODE{slot}"), slot as f64, format!("Mode {slot}")));
        Fake { parameters: std::iter::once(("FLTMODE_CH".to_string(), channel, String::new())).chain(slots).collect(), pwm }
    }

    #[test]
    fn the_live_slot_is_marked_on_the_slot_the_transmitter_selects() {
        let view = slots_view(&copter(5.0, channels(1500, 4)), &[]);
        assert_eq!(view["channel"], 5);
        assert_eq!(view["liveSlot"], 4, "fifteen hundred sits in the fourth band");
        let slots = view["slots"].as_array().unwrap();
        assert_eq!(slots.len(), SLOTS);
        assert_eq!(slots.iter().filter(|slot| slot["live"] == true).count(), 1, "exactly one slot is live, which is the whole question the screen exists to answer");
        assert_eq!(slots[3]["live"], true);
        assert_eq!(slots[3]["mode"], "Mode 4");
    }

    #[test]
    fn the_channel_the_vehicle_names_is_the_channel_that_is_read() {
        let view = slots_view(&copter(7.0, channels(1200, 6)), &[]);
        assert_eq!(view["channel"], 7);
        assert_eq!(view["liveSlot"], 1, "twelve hundred on channel seven is the first slot");
        assert_eq!(view["channelPwm"], 1200);
    }

    #[test]
    fn a_vehicle_with_no_mode_channel_parameter_falls_back_to_channel_five() {
        let mut without = copter(5.0, channels(1200, 4));
        without.parameters.retain(|(name, _, _)| name != "FLTMODE_CH");
        let view = slots_view(&without, &[]);
        assert_eq!(view["channel"], 5, "the Qt controller defaults to the fifth channel when the parameter is absent, and a head must not guess differently");
        assert_eq!(view["liveSlot"], 1);
    }

    #[test]
    fn a_rover_reads_its_own_parameter_names() {
        let slots = (1..=SLOTS).map(|slot| (format!("MODE{slot}"), slot as f64, format!("Rover mode {slot}")));
        let rover = Fake {
            parameters: std::iter::once(("MODE_CH".to_string(), 5.0, String::new())).chain(slots).collect(),
            pwm: channels(1500, 4),
        };
        let view = slots_view(&rover, &[]);
        assert_eq!(view["slots"][0]["mode"], "Rover mode 1", "a rover names its slots MODE1 to MODE6, and which names to read is decided by which the vehicle has rather than by its type");
        assert_eq!(view["liveSlot"], 4);
    }

    #[test]
    fn a_vehicle_that_chooses_no_modes_from_a_channel_says_so() {
        let bare = Fake { parameters: Vec::new(), pwm: channels(1500, 4) };
        let view = slots_view(&bare, &[]);
        assert_eq!(view["available"], false, "a vehicle with no slot parameters at all has no slots to show, and six empty rows would be a lie");
        assert!(view["reason"].as_str().unwrap().contains("transmitter channel"));
    }

    #[test]
    fn a_mode_channel_past_the_last_one_the_transmitter_sends_selects_nothing() {
        let view = slots_view(&copter(12.0, channels(1500, 4)), &[]);
        assert_eq!(view["liveSlot"], 0);
        assert_eq!(view["channelPwm"], Value::Null, "there is no reading for a channel that is not being received");
        assert!(view["slots"].as_array().unwrap().iter().all(|slot| slot["live"] == false), "no slot may be marked live when the channel carrying the selection is not being received");
        assert!(view["reason"].as_str().unwrap().contains("not sending"));

        // A channel parameter of zero puts the index at -1. The guard that used to wrap this read
        // was redundant - the cast wraps to a value no channel list reaches, so the lookup misses
        // either way - and removing it must not change the answer.
        let below = slots_view(&copter(0.0, channels(1500, 8)), &[]);
        assert_eq!(below["channelPwm"], Value::Null, "a mode channel of zero names no channel, so there is no reading rather than the last one");
        assert_eq!(below["liveSlot"], 0);
        assert!(below["reason"].as_str().unwrap().contains("not sending"));
    }

    #[test]
    fn no_slot_is_ever_marked_live_unless_the_view_is_answering() {
        let states = [
            slots_view(&copter(5.0, Vec::new()), &[]),
            slots_view(&copter(5.0, channels(-1, 4)), &[]),
            slots_view(&copter(12.0, channels(1500, 4)), &[]),
            slots_view(&copter(0.0, channels(1500, 4)), &[]),
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
    fn a_channel_option_is_reported_for_every_channel_the_screen_offers() {
        let mut pwm = channels(1500, 4);
        pwm.resize(16, 1000);
        pwm[6] = 1900;
        let view = slots_view(&copter(5.0, pwm), &[]);
        let options = view["channelOptions"].as_array().unwrap();
        assert_eq!(options.len(), CHANNEL_OPTIONS);
        assert_eq!(options[0]["channel"], 6);
        assert_eq!(options[1]["channel"], 7);
        assert_eq!(options[1]["enabled"], true, "channel seven is the second option and it is the one held high");
        assert_eq!(options[0]["enabled"], false);
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
