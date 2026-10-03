use serde_json::{Value, json};

use crate::read::{object, result_flag, value_number};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.apmFirmware",
    "radioCal.rcValues",
    "vehicle.parameterManager.getParameter(-1,FLTMODE_CH).rawValue",
    "vehicle.parameterManager.getParameter(-1,FLTMODE1).rawValue",
    "vehicle.parameterManager.getParameter(-1,FLTMODE2).rawValue",
    "vehicle.parameterManager.getParameter(-1,FLTMODE3).rawValue",
    "vehicle.parameterManager.getParameter(-1,FLTMODE4).rawValue",
    "vehicle.parameterManager.getParameter(-1,FLTMODE5).rawValue",
    "vehicle.parameterManager.getParameter(-1,FLTMODE6).rawValue",
    "vehicle.parameterManager.getParameter(-1,MODE_CH).rawValue",
    "vehicle.parameterManager.getParameter(-1,MODE1).rawValue",
    "vehicle.parameterManager.getParameter(-1,MODE2).rawValue",
    "vehicle.parameterManager.getParameter(-1,MODE3).rawValue",
    "vehicle.parameterManager.getParameter(-1,MODE4).rawValue",
    "vehicle.parameterManager.getParameter(-1,MODE5).rawValue",
    "vehicle.parameterManager.getParameter(-1,MODE6).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_MAP_FLTMODE).rawValue",
    "vehicle.parameterManager.getParameter(-1,COM_FLTMODE1).rawValue",
    "vehicle.parameterManager.getParameter(-1,COM_FLTMODE2).rawValue",
    "vehicle.parameterManager.getParameter(-1,COM_FLTMODE3).rawValue",
    "vehicle.parameterManager.getParameter(-1,COM_FLTMODE4).rawValue",
    "vehicle.parameterManager.getParameter(-1,COM_FLTMODE5).rawValue",
    "vehicle.parameterManager.getParameter(-1,COM_FLTMODE6).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_MAP_ARM_SW).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_ARMSWITCH_TH).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_MAP_GEAR_SW).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_GEAR_TH).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_MAP_KILL_SW).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_KILLSWITCH_TH).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_MAP_LOITER_SW).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_LOITER_TH).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_MAP_OFFB_SW).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_OFFB_TH).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_MAP_RETURN_SW).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_RETURN_TH).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_MAP_TRANS_SW).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_TRANS_TH).rawValue",
    "vehicle.parameterManager.getParameter(-1,RC_MAP_FLAPS).rawValue",
];

pub const SLOTS: usize = 6;
const PX4_SWITCHES: [(&str, &str); 8] = [("RC_MAP_ARM_SW", "RC_ARMSWITCH_TH"), ("RC_MAP_GEAR_SW", "RC_GEAR_TH"), ("RC_MAP_KILL_SW", "RC_KILLSWITCH_TH"), ("RC_MAP_LOITER_SW", "RC_LOITER_TH"), ("RC_MAP_OFFB_SW", "RC_OFFB_TH"), ("RC_MAP_RETURN_SW", "RC_RETURN_TH"), ("RC_MAP_TRANS_SW", "RC_TRANS_TH"), ("RC_MAP_FLAPS", "")];
const DEFAULT_SWITCH_THRESHOLD: f64 = 0.5;

pub fn switch_active(pwm: Option<i64>, threshold: f64) -> bool {
    let at = 1000.0 + 1000.0 * threshold;
    pwm.is_some_and(|value| match threshold >= 0.0 {
        true => value as f64 > at,
        false => value as f64 <= at,
    })
}
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

#[derive(Clone, Copy)]
pub struct Calibration {
    pub min: f64,
    pub max: f64,
    pub trim: f64,
    pub reversed: bool,
}

pub fn px4_slot(pwm: Option<i64>, cal: Calibration) -> usize {
    let Some(value) = pwm.filter(|v| *v != NO_RC) else { return 0 };
    let (value, trim) = (value as f32, cal.trim as f32);
    let slots = SLOTS as f32;
    let slot_width_half = 2.0 / slots / 2.0;
    let (slot_min, slot_max) = (-1.0f32 - 0.05, 1.0f32 + 0.05);
    let calibrated = match value {
        v if v > trim => (v - trim) / (cal.max as f32 - trim),
        v if v < trim => (v - trim) / (trim - cal.min as f32),
        _ => 0.0,
    } * if cal.reversed { -1.0 } else { 1.0 };
    let index = ((((calibrated - slot_min) * slots) + slot_width_half) / (slot_max - slot_min) + (1.0 / slots)) as i64;
    index.clamp(0, SLOTS as i64 - 1) as usize + 1
}

fn names(backend: &dyn Backend) -> (&'static str, &'static str) {
    match (exists(backend, "RC_MAP_FLTMODE"), exists(backend, "MODE_CH")) {
        (true, _) => ("RC_MAP_FLTMODE", "COM_FLTMODE"),
        (false, true) => ("MODE_CH", "MODE"),
        (false, false) => ("FLTMODE_CH", "FLTMODE"),
    }
}

fn px4_calibration(backend: &dyn Backend, channel: i64) -> Option<Calibration> {
    let read = |what: &str| parameter(backend, &format!("RC{channel}_{what}"));
    Some(Calibration { min: read("MIN")?, max: read("MAX")?, trim: read("TRIM")?, reversed: read("REV")? < 0.0 })
}

fn exists(backend: &dyn Backend, name: &str) -> bool {
    result_flag(&backend.invoke("vehicle.parameterManager.parameterExists", &json!([-1, name]).to_string()))
}

fn parameter(backend: &dyn Backend, name: &str) -> Option<f64> {
    exists(backend, name).then(|| value_number(&backend.get(&format!("vehicle.parameterManager.getParameter(-1,{name}).rawValue")))).flatten()
}

fn switch_label(backend: &dyn Backend, name: &str) -> String {
    object(&backend.get(&format!("vehicle.parameterManager.getParameter(-1,{name})")))
        .get("shortDescription")
        .and_then(Value::as_str)
        .filter(|label| !label.is_empty())
        .map_or_else(|| name.to_string(), str::to_string)
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
    let radio = object(&backend.get_fields("radioCal", "rcValues"));
    let pwm: Vec<i64> = radio.get("rcValues").and_then(Value::as_array).map(|values| values.iter().filter_map(Value::as_i64).collect()).unwrap_or_default();
    let reachable = channel_index >= 0 && (channel_index as usize) < pwm.len();
    let live = match (reachable, channel_name) {
        (false, _) => 0,
        (true, "RC_MAP_FLTMODE") => px4_calibration(backend, channel_index + 1).map_or(0, |cal| px4_slot(pwm.get(channel_index as usize).copied(), cal)),
        (true, _) => slot_for(pwm.get(channel_index as usize).copied()),
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

    let options: Vec<Value> = match channel_name {
        "RC_MAP_FLTMODE" => Vec::new(),
        _ => (0..CHANNEL_OPTIONS).map(|index| json!({ "channel": index + 6, "enabled": option_enabled(pwm.get(index + 5).copied()) })).collect(),
    };

    let raised: Vec<&str> = match channel_name {
        "RC_MAP_FLTMODE" => PX4_SWITCHES
            .iter()
            .filter(|(switch, _)| exists(backend, switch))
            .filter(|(switch, threshold)| {
                let th = (!threshold.is_empty()).then(|| parameter(backend, threshold)).flatten().unwrap_or(DEFAULT_SWITCH_THRESHOLD);
                let pwm_at = parameter(backend, switch).and_then(|ch| usize::try_from(ch as i64 - 1).ok()).and_then(|i| pwm.get(i).copied());
                switch_active(pwm_at, th)
            })
            .map(|(switch, _)| *switch)
            .collect(),
        _ => Vec::new(),
    };
    let active_switches: Vec<String> = raised.iter().map(|switch| switch_label(backend, switch)).collect();
    let switch_params: Vec<String> = match channel_name {
        "RC_MAP_FLTMODE" => raised.iter().map(|switch| switch.to_string()).collect(),
        _ => (0..CHANNEL_OPTIONS).filter(|index| option_enabled(pwm.get(index + 5).copied())).map(|index| format!("RC{}_OPTION", index + 6)).collect(),
    };
    let active_params: Vec<String> = (live > 0).then(|| format!("{slot_prefix}{live}")).into_iter().chain(switch_params).collect();
    json!({
        "kind": "object",
        "class": "ModeSlots",
        "available": true,
        "activeParams": active_params,
        "activeSwitches": active_switches,
        "channelMonitor": channel_name == "RC_MAP_FLTMODE",
        "channel": (channel_index >= 0).then_some(channel_index + 1),
        "channelPwm": pwm.get(channel_index as usize).copied(),
        "liveSlot": live,
        "slots": slots,
        "channelOptions": options,
        "reason": match (reachable, live) {
            _ if channel_index < 0 => "No mode channel is assigned.",
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
    fn a_px4_switch_is_on_past_its_threshold_like_px4_flight_modes() {
        assert!(switch_active(Some(1600), 0.5));
        assert!(!switch_active(Some(1500), 0.5), "exactly at the threshold is off");
        assert!(switch_active(Some(700), -0.25), "a negative threshold inverts, with the point taken as QGC computes it: 1000 + 1000 * -0.25 = 750");
        assert!(!switch_active(Some(1200), -0.25));
        assert!(!switch_active(None, 0.5), "no reading, no highlight");
    }

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
    fn px4_normalises_around_trim_into_six_slots_as_px4_simple_flight_modes_controller_does() {
        let cal = Calibration { min: 1000.0, max: 2000.0, trim: 1500.0, reversed: false };
        let slots: Vec<usize> = [1000, 1150, 1300, 1450, 1500, 1600, 1750, 1900, 2000].iter().map(|pwm| px4_slot(Some(*pwm), cal)).collect();
        assert_eq!(slots, [1, 2, 3, 3, 4, 4, 5, 6, 6], "1150 normalises to -0.7, which the slot formula floors into slot 2");
        assert_eq!(px4_slot(Some(1000), Calibration { reversed: true, ..cal }), 6, "RCn_REV flips the channel");
        assert_eq!(px4_slot(Some(-1), cal), 0);
        assert_eq!(px4_slot(None, cal), 0);
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
    fn the_rows_to_highlight_are_the_live_slot_and_the_raised_option_channels() {
        let mut pwm = channels(1500, 4);
        pwm[6] = 1900;
        let view = slots_view(&copter(5.0, pwm), &[]);
        assert_eq!(view["activeParams"], json!(["FLTMODE4", "RC7_OPTION"]), "APMFlightModesComponent paints the active slot and each enabled channel option yellow");
        let off = slots_view(&copter(12.0, channels(1500, 4)), &[]);
        assert_eq!(off["activeParams"], json!([]));
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

        let below = slots_view(&copter(0.0, channels(1500, 8)), &[]);
        assert_eq!(below["channelPwm"], Value::Null, "a mode channel of zero names no channel, so there is no reading rather than the last one");
        assert_eq!(below["liveSlot"], 0);
        assert_eq!((below["reason"].as_str(), below["channel"].clone()), (Some("No mode channel is assigned."), Value::Null), "an unassigned channel is said so, never named channel 0");
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
