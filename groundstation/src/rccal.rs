use serde_json::{Value, json};

pub const MAX_CHANNELS: usize = 18;
pub const MINIMUM_CHANNELS: usize = 4;
pub const DEFAULT_MIN: i32 = 1000;
pub const DEFAULT_MAX: i32 = 2000;
pub const CENTER: i32 = 1500;
const ROUGH_CENTER_DELTA: i32 = 50;
const MOVE_DELTA: i32 = 300;
const SETTLE_DELTA: i32 = 20;
const SETTLE_MS: u64 = 1000;
const VALID_MIN: i32 = 1300;
const VALID_MAX: i32 = 1700;
const ABSENT_TRIM: i32 = 1500;
const ABSENT_MIN: i32 = 1100;
const ABSENT_MAX: i32 = 1900;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Function {
    Roll,
    Pitch,
    Yaw,
    Throttle,
}

pub const FUNCTIONS: [Function; 4] = [Function::Roll, Function::Pitch, Function::Yaw, Function::Throttle];

impl Function {
    fn index(self) -> usize {
        FUNCTIONS.iter().position(|f| *f == self).unwrap_or(0)
    }

    pub fn map_param(self, px4: bool) -> &'static str {
        match (self, px4) {
            (Function::Roll, true) => "RC_MAP_ROLL",
            (Function::Pitch, true) => "RC_MAP_PITCH",
            (Function::Yaw, true) => "RC_MAP_YAW",
            (Function::Throttle, true) => "RC_MAP_THROTTLE",
            (Function::Roll, false) => "RCMAP_ROLL",
            (Function::Pitch, false) => "RCMAP_PITCH",
            (Function::Yaw, false) => "RCMAP_YAW",
            (Function::Throttle, false) => "RCMAP_THROTTLE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    StickNeutral,
    Detect(Function),
    Min(Function),
    SwitchMinMax,
    Complete,
}

const STEPS: [Step; 11] = [
    Step::StickNeutral,
    Step::Detect(Function::Throttle),
    Step::Min(Function::Throttle),
    Step::Detect(Function::Yaw),
    Step::Min(Function::Yaw),
    Step::Detect(Function::Roll),
    Step::Min(Function::Roll),
    Step::Detect(Function::Pitch),
    Step::Min(Function::Pitch),
    Step::SwitchMinMax,
    Step::Complete,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Channel {
    pub function: Option<Function>,
    pub reversed: bool,
    pub min: i32,
    pub max: i32,
    pub trim: i32,
}

impl Default for Channel {
    fn default() -> Self {
        Channel { function: None, reversed: false, min: CENTER, max: CENTER, trim: CENTER }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vehicle {
    pub px4: bool,
    pub multi_rotor: bool,
    pub helicopter: bool,
    pub rover: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    None,
    StartCalibration,
    StopCalibration,
    Write(Vec<(String, f64)>),
    ThrottleReversed,
}

#[derive(Debug, Clone)]
pub struct RcCal {
    pub channels: [Channel; MAX_CHANNELS],
    mapping: [Option<usize>; 4],
    raw: [i32; MAX_CHANNELS],
    saved: [i32; MAX_CHANNELS],
    pub count: usize,
    step: Option<usize>,
    detect: Option<usize>,
    detect_value: i32,
    settle_since: Option<u64>,
    pub next_enabled: bool,
    pub transmitter_mode: i32,
    centered_throttle: bool,
    pub status_text: String,
    throttle_reversed_failure: bool,
}

impl Default for RcCal {
    fn default() -> Self {
        RcCal {
            channels: [Channel::default(); MAX_CHANNELS],
            mapping: [None; 4],
            raw: [0; MAX_CHANNELS],
            saved: [0; MAX_CHANNELS],
            count: 0,
            step: None,
            detect: None,
            detect_value: 0,
            settle_since: None,
            next_enabled: true,
            transmitter_mode: 2,
            centered_throttle: false,
            status_text: String::new(),
            throttle_reversed_failure: false,
        }
    }
}

fn diagram(step: Step, centered_throttle: bool) -> Option<crate::stickcal::Message> {
    use crate::stickcal::Message;
    match step {
        Step::StickNeutral if centered_throttle => None,
        Step::StickNeutral | Step::Min(Function::Throttle) => Some(Message::ThrottleDown),
        Step::Detect(Function::Throttle) => Some(Message::ThrottleUp),
        Step::Detect(Function::Yaw) => Some(Message::YawRight),
        Step::Min(Function::Yaw) => Some(Message::YawLeft),
        Step::Detect(Function::Roll) => Some(Message::RollRight),
        Step::Min(Function::Roll) => Some(Message::RollLeft),
        Step::Detect(Function::Pitch) => Some(Message::PitchUp),
        Step::Min(Function::Pitch) => Some(Message::PitchDown),
        Step::SwitchMinMax | Step::Complete => None,
    }
}

fn message(step: Step, centered_throttle: bool) -> &'static str {
    match step {
        Step::StickNeutral if centered_throttle => "* Center all sticks as shown in diagram.\n* Make sure any additional axes are at a neutral position.\n* Please ensure all motor power is disconnected from the vehicle.\n* Click Next to continue",
        Step::StickNeutral => "* Lower the Throttle stick all the way down as shown in diagram\n* Please ensure all motor power is disconnected AND all props are removed from the vehicle.\n* Click Next to continue",
        Step::Detect(Function::Throttle) => "Move the Throttle stick all the way up and hold it there...",
        Step::Min(Function::Throttle) => "Move the Throttle stick all the way down and leave it there...",
        Step::Detect(Function::Yaw) => "Move the Yaw stick all the way to the right and hold it there...",
        Step::Min(Function::Yaw) => "Move the Yaw stick all the way to the left and hold it there...",
        Step::Detect(Function::Roll) => "Move the Roll stick all the way to the right and hold it there...",
        Step::Min(Function::Roll) => "Move the Roll stick all the way to the left and hold it there...",
        Step::Detect(Function::Pitch) => "Move the Pitch stick all the way up and hold it there...",
        Step::Min(Function::Pitch) => "Move the Pitch stick all the way down and hold it there...",
        Step::SwitchMinMax => "Move all the transmitter switches and/or dials back and forth to their extreme positions.",
        Step::Complete => "All settings have been captured. Click Next to write the new parameters to your board.",
    }
}

impl RcCal {
    pub fn for_vehicle(vehicle: &Vehicle, transmitter_mode: i32) -> RcCal {
        RcCal { centered_throttle: vehicle.rover, transmitter_mode: if (1..=4).contains(&transmitter_mode) { transmitter_mode } else { 2 }, ..RcCal::default() }
    }

    pub fn set_centered_throttle(&mut self, centered: bool) {
        self.centered_throttle = centered;
    }

    pub fn calibrating(&self) -> bool {
        self.step.is_some()
    }

    pub fn mapped(&self, function: Function) -> Option<usize> {
        self.mapping[function.index()]
    }

    pub fn adjusted(&self, function: Function) -> i32 {
        self.mapped(function).map_or(CENTER, |c| {
            let info = &self.channels[c];
            match info.reversed {
                false => self.raw[c],
                true => (info.min + info.max - self.raw[c]).clamp(info.min, info.max),
            }
        })
    }

    pub fn reversed(&self, function: Function) -> bool {
        self.mapped(function).is_some_and(|c| self.channels[c].reversed)
    }

    pub fn rc_values(&self) -> Vec<i32> {
        self.raw[..self.count].to_vec()
    }

    pub fn read_stored(&mut self, vehicle: &Vehicle, parameter: &dyn Fn(&str) -> Option<f64>) {
        let reversed_param = |n: usize| match parameter("RC1_REVERSED") {
            Some(_) => parameter(&format!("RC{n}_REVERSED")).is_some_and(|v| v != 0.0),
            None => parameter(&format!("RC{n}_REV")).is_some_and(|v| v == -1.0),
        };
        self.channels = std::array::from_fn(|i| {
            let n = i + 1;
            match parameter(&format!("RC{n}_MIN")) {
                None => Channel { function: None, reversed: false, min: ABSENT_MIN, max: ABSENT_MAX, trim: ABSENT_TRIM },
                Some(min) => Channel {
                    function: None,
                    reversed: reversed_param(n),
                    min: min as i32,
                    max: parameter(&format!("RC{n}_MAX")).map_or(ABSENT_MAX, |v| v as i32),
                    trim: parameter(&format!("RC{n}_TRIM")).map_or(ABSENT_TRIM, |v| v as i32),
                },
            }
        });
        self.mapping = FUNCTIONS.map(|f| parameter(f.map_param(vehicle.px4)).map(|v| v as i64).filter(|c| (1..=MAX_CHANNELS as i64).contains(c)).map(|c| c as usize - 1));
        FUNCTIONS.iter().for_each(|f| {
            if let Some(c) = self.mapped(*f) {
                self.channels[c].function = Some(*f);
            }
        });
    }

    fn reset_internal(&mut self) {
        self.channels = [Channel::default(); MAX_CHANNELS];
        self.mapping = [None; 4];
    }

    fn setup_current(&mut self) {
        let Some(step) = self.step.and_then(|s| STEPS.get(s)).copied() else { return };
        self.status_text = message(step, self.centered_throttle).to_string();
        self.detect = None;
        self.settle_since = None;
        self.saved = self.raw;
        self.next_enabled = matches!(step, Step::StickNeutral | Step::SwitchMinMax | Step::Complete);
    }

    fn advance(&mut self) -> Outcome {
        let next = self.step.map_or(0, |s| s + 1);
        match next < STEPS.len() {
            true => {
                self.step = Some(next);
                self.setup_current();
                Outcome::None
            }
            false => Outcome::StopCalibration,
        }
    }

    pub fn forget_failure(&mut self) {
        self.throttle_reversed_failure = false;
    }

    pub fn stop(&mut self, vehicle: &Vehicle, parameter: &dyn Fn(&str) -> Option<f64>) -> Outcome {
        let was = self.step.take().is_some();
        self.read_stored(vehicle, parameter);
        self.status_text.clear();
        self.next_enabled = true;
        match was {
            true => Outcome::StopCalibration,
            false => Outcome::None,
        }
    }

    pub fn next(&mut self, vehicle: &Vehicle, parameter: &dyn Fn(&str) -> Option<f64>) -> Vec<Outcome> {
        match self.step.and_then(|s| STEPS.get(s)).copied() {
            None if self.count < MINIMUM_CHANNELS => Vec::new(),
            None => {
                self.reset_internal();
                self.throttle_reversed_failure = false;
                self.step = Some(0);
                self.setup_current();
                vec![Outcome::StartCalibration]
            }
            Some(Step::StickNeutral) => {
                (0..self.count).for_each(|i| self.channels[i].trim = self.raw[i]);
                vec![self.advance()]
            }
            Some(Step::SwitchMinMax) => vec![self.advance()],
            Some(Step::Complete) => {
                let saved = self.save(vehicle, parameter);
                let stopped = self.stop(vehicle, parameter);
                vec![saved, stopped]
            }
            Some(_) => Vec::new(),
        }
    }

    fn settled(&mut self, value: i32, now_ms: u64) -> bool {
        if (self.detect_value - value).abs() > SETTLE_DELTA {
            self.detect_value = value;
            self.settle_since = None;
            return false;
        }
        match self.settle_since {
            Some(since) => now_ms.saturating_sub(since) > SETTLE_MS,
            None => {
                self.settle_since = Some(now_ms);
                false
            }
        }
    }

    pub fn channel_values(&mut self, values: &[i32], now_ms: u64) -> Outcome {
        let count = values.len().min(MAX_CHANNELS);
        values.iter().take(count).enumerate().fold(Outcome::None, |outcome, (channel, value)| {
            self.raw[channel] = *value;
            let step = self.step.and_then(|s| STEPS.get(s)).copied();
            let this = match step {
                None => {
                    self.count = count;
                    Outcome::None
                }
                Some(Step::StickNeutral) => {
                    self.next_enabled = true;
                    Outcome::None
                }
                Some(Step::Detect(function)) => self.stick_detect(function, channel, *value, now_ms),
                Some(Step::Min(function)) => self.stick_min(function, channel, *value, now_ms),
                Some(Step::SwitchMinMax) => {
                    self.switch_min_max(channel, *value);
                    Outcome::None
                }
                Some(Step::Complete) => Outcome::None,
            };
            if outcome == Outcome::None { this } else { outcome }
        })
    }

    fn stick_detect(&mut self, function: Function, channel: usize, value: i32, now_ms: u64) -> Outcome {
        if self.channels[channel].function.is_some() {
            return Outcome::None;
        }
        let detect = self.detect;
        match detect {
            None => {
                if (self.saved[channel] - value).abs() > MOVE_DELTA {
                    self.detect = Some(channel);
                    self.detect_value = value;
                }
                Outcome::None
            }
            Some(detected) if detected == channel && self.settled(value, now_ms) => {
                self.mapping[function.index()] = Some(channel);
                let reversed = value < self.saved[channel];
                let info = &mut self.channels[channel];
                info.function = Some(function);
                info.reversed = reversed;
                if reversed { info.min = value } else { info.max = value }
                self.advance()
            }
            Some(_) => Outcome::None,
        }
    }

    fn stick_min(&mut self, function: Function, channel: usize, value: i32, now_ms: u64) -> Outcome {
        if self.mapped(function) != Some(channel) {
            return Outcome::None;
        }
        let detect = self.detect;
        match detect {
            None => {
                let reversed = self.channels[channel].reversed;
                if (reversed && value > CENTER + MOVE_DELTA) || (!reversed && value < CENTER - MOVE_DELTA) {
                    self.detect = Some(channel);
                    self.detect_value = value;
                }
                Outcome::None
            }
            Some(_) if self.settled(value, now_ms) => {
                let info = &mut self.channels[channel];
                if info.reversed { info.max = value } else { info.min = value }
                if function == Function::Throttle {
                    info.trim = value;
                }
                self.advance()
            }
            Some(_) => Outcome::None,
        }
    }

    fn switch_min_max(&mut self, channel: usize, value: i32) {
        let info = &mut self.channels[channel];
        if info.function.is_some() || (CENTER - value).abs() <= MOVE_DELTA {
            return;
        }
        if value < CENTER { info.min = info.min.min(value) } else { info.max = info.max.max(value) }
    }

    fn validate(&mut self) {
        let count = self.count;
        self.channels.iter_mut().enumerate().for_each(|(i, info)| {
            let defaulted = || Channel { function: info.function, reversed: info.reversed, min: DEFAULT_MIN, max: DEFAULT_MAX, trim: DEFAULT_MIN + (DEFAULT_MAX - DEFAULT_MIN) / 2 };
            *info = match (i < count, info.min > VALID_MIN || info.max < VALID_MAX) {
                (false, _) => Channel { function: None, reversed: false, ..defaulted() },
                (true, true) => defaulted(),
                (true, false) if info.function.is_some() => Channel { trim: info.trim.clamp(info.min, info.max), ..*info },
                (true, false) => Channel { trim: info.min + (info.max - info.min) / 2, ..*info },
            };
        });
    }

    fn save(&mut self, vehicle: &Vehicle, parameter: &dyn Fn(&str) -> Option<f64>) -> Outcome {
        let throttle_reversed = self.mapped(Function::Throttle).is_some_and(|c| self.channels[c].reversed);
        if !vehicle.px4 && (vehicle.multi_rotor || vehicle.helicopter) && throttle_reversed {
            self.read_stored(vehicle, parameter);
            self.throttle_reversed_failure = true;
            return Outcome::ThrottleReversed;
        }
        self.validate();
        let bool_reversal = parameter("RC1_REVERSED").is_some();
        let channels: Vec<(String, f64)> = self
            .channels
            .iter()
            .enumerate()
            .filter(|(i, _)| parameter(&format!("RC{}_MIN", i + 1)).is_some())
            .flat_map(|(i, info)| {
                let n = i + 1;
                let limits = [(format!("RC{n}_TRIM"), f64::from(info.trim)), (format!("RC{n}_MIN"), f64::from(info.min)), (format!("RC{n}_MAX"), f64::from(info.max))];
                let reversed = match vehicle.px4 || info.function != Some(Function::Pitch) { true => info.reversed, false => !info.reversed };
                let reversal = (vehicle.px4 || vehicle.multi_rotor).then(|| match bool_reversal {
                    true => (format!("RC{n}_REVERSED"), if reversed { 1.0 } else { 0.0 }),
                    false => (format!("RC{n}_REV"), if reversed { -1.0 } else { 1.0 }),
                });
                limits.into_iter().chain(reversal)
            })
            .collect();
        let maps: Vec<(String, f64)> = FUNCTIONS
            .iter()
            .filter_map(|f| {
                let channel = self.mapped(*f).map_or(0.0, |c| (c + 1) as f64);
                let name = f.map_param(vehicle.px4);
                parameter(name).filter(|held| *held != channel).map(|_| (name.to_string(), channel))
            })
            .collect();
        let count = (vehicle.px4 && parameter("RC_CHAN_CNT").is_some()).then(|| ("RC_CHAN_CNT".to_string(), self.count as f64));
        let writes = channels.into_iter().chain(maps).chain(count).collect();
        Outcome::Write(writes)
    }

    pub fn stick_positions(&self) -> [i32; 4] {
        let mode = u8::try_from(self.transmitter_mode).unwrap_or(2);
        self.step
            .and_then(|s| STEPS.get(s))
            .and_then(|step| diagram(*step, self.centered_throttle))
            .map_or([0; 4], |message| crate::stickcal::stick_positions(message, mode))
    }

    pub fn json(&self) -> Value {
        let fields = |f: Function| (f, self.mapped(f).is_some(), self.adjusted(f), self.reversed(f));
        let object: serde_json::Map<String, Value> = FUNCTIONS
            .iter()
            .flat_map(|f| {
                let (function, mapped, adjusted, reversed) = fields(*f);
                let key = match function { Function::Roll => "roll", Function::Pitch => "pitch", Function::Yaw => "yaw", Function::Throttle => "throttle" };
                let capital = match function { Function::Roll => "Roll", Function::Pitch => "Pitch", Function::Yaw => "Yaw", Function::Throttle => "Throttle" };
                [(format!("{key}ChannelMapped"), json!(mapped)), (format!("adjusted{capital}ChannelValue"), json!(adjusted)), (format!("{key}ChannelReversed"), json!(i32::from(reversed)))]
            })
            .collect();
        let mut answer = json!({
            "kind": "object",
            "class": "RadioComponentController",
            "statusText": self.status_text,
            "nextText": if self.calibrating() { "Next" } else { "Calibrate" },
            "nextEnabled": self.next_enabled,
            "cancelEnabled": self.calibrating(),
            "calibrating": self.calibrating(),
            "rcValues": self.rc_values(),
            "minChannelCount": MINIMUM_CHANNELS,
            "channelCount": self.count,
            "channelValueMin": DEFAULT_MIN,
            "channelValueMax": DEFAULT_MAX,
            "transmitterMode": self.transmitter_mode,
            "centeredThrottle": self.centered_throttle,
            "joystickMode": false,
            "throttleReversed": self.throttle_reversed_failure,
            "stickDisplayPositions": self.stick_positions(),
        });
        answer.as_object_mut().into_iter().for_each(|a| a.extend(object.clone()));
        answer
    }
}

pub fn clamped(values: &[u16]) -> Vec<i32> {
    values.iter().map(|v| i32::from(*v).clamp(DEFAULT_MIN, DEFAULT_MAX)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn copter() -> Vehicle {
        Vehicle { px4: false, multi_rotor: true, helicopter: false, rover: false }
    }

    fn apm_params() -> BTreeMap<String, f64> {
        (1..=16)
            .flat_map(|n| [(format!("RC{n}_MIN"), 1100.0), (format!("RC{n}_MAX"), 1900.0), (format!("RC{n}_TRIM"), 1500.0), (format!("RC{n}_REVERSED"), 0.0)])
            .chain([("RCMAP_ROLL".to_string(), 1.0), ("RCMAP_PITCH".to_string(), 2.0), ("RCMAP_THROTTLE".to_string(), 3.0), ("RCMAP_YAW".to_string(), 4.0)])
            .collect()
    }

    fn hold(cal: &mut RcCal, values: &[i32], from_ms: u64) -> (u64, Vec<Outcome>) {
        (0..14).fold((from_ms, Vec::new()), |(now, mut outcomes), _| {
            let outcome = cal.channel_values(values, now);
            if outcome != Outcome::None {
                outcomes.push(outcome);
            }
            (now + 100, outcomes)
        })
    }

    #[test]
    fn stored_parameters_map_the_sticks_and_an_absent_channel_takes_the_placeholders() {
        let params = apm_params();
        let lookup = |name: &str| params.get(name).copied();
        let mut cal = RcCal::for_vehicle(&copter(), 2);
        cal.read_stored(&copter(), &lookup);
        assert_eq!((cal.mapped(Function::Throttle), cal.mapped(Function::Yaw)), (Some(2), Some(3)));
        assert_eq!(cal.channels[16], Channel { function: None, reversed: false, min: 1100, max: 1900, trim: 1500 });
    }

    #[test]
    fn a_full_calibration_maps_each_stick_from_movement_and_writes_the_limits() {
        let params = apm_params();
        let lookup = |name: &str| params.get(name).copied();
        let mut cal = RcCal::for_vehicle(&copter(), 2);
        let center = [1500, 1500, 1000, 1500, 1500, 1500, 1500, 1500];
        cal.channel_values(&center, 0);
        assert_eq!(cal.next(&copter(), &lookup), vec![Outcome::StartCalibration]);
        cal.channel_values(&center, 100);
        assert_eq!(cal.next(&copter(), &lookup), vec![Outcome::None], "neutral saves every trim and moves on");
        let with = |channel: usize, value: i32| { let mut v = center; v[channel] = value; v };
        let (t, _) = hold(&mut cal, &with(2, 1950), 200);
        let (t, _) = hold(&mut cal, &with(2, 1050), t);
        let (t, _) = hold(&mut cal, &with(3, 1940), t);
        let (t, _) = hold(&mut cal, &with(3, 1060), t);
        let (t, _) = hold(&mut cal, &with(0, 1930), t);
        let (t, _) = hold(&mut cal, &with(0, 1070), t);
        let (t, _) = hold(&mut cal, &with(1, 1080), t);
        let (_, _) = hold(&mut cal, &with(1, 1920), t);
        assert_eq!((cal.mapped(Function::Throttle), cal.mapped(Function::Yaw), cal.mapped(Function::Roll), cal.mapped(Function::Pitch)), (Some(2), Some(3), Some(0), Some(1)));
        assert!(cal.reversed(Function::Pitch), "pitch pushed up read lower than centre");
        assert_eq!(cal.status_text, message(Step::SwitchMinMax, false));
        cal.channel_values(&with(5, 1010), 9_000);
        cal.channel_values(&with(5, 1990), 9_100);
        assert_eq!(cal.next(&copter(), &lookup), vec![Outcome::None]);
        let outcomes = cal.next(&copter(), &lookup);
        let Outcome::Write(writes) = &outcomes[0] else { panic!("{outcomes:?}") };
        let get = |name: &str| writes.iter().find(|(n, _)| n == name).map(|(_, v)| *v);
        assert_eq!((get("RC3_MIN"), get("RC3_MAX"), get("RC3_TRIM")), (Some(1050.0), Some(1950.0), Some(1050.0)), "the throttle trim is its low end");
        assert_eq!((get("RC6_MIN"), get("RC6_MAX"), get("RC6_TRIM")), (Some(1010.0), Some(1990.0), Some(1500.0)), "a switch takes the extremes it was moved to and a centred trim");
        assert_eq!(get("RC2_REVERSED"), Some(0.0), "ArduPilot copters store pitch reversal inverted");
        assert_eq!(get("RC1_REVERSED"), Some(0.0));
        assert_eq!((get("RCMAP_ROLL"), get("RCMAP_THROTTLE")), (None, None), "a mapping that did not change is not rewritten");
        assert_eq!(get("RC8_MIN"), Some(1000.0), "a channel never moved keeps the default range");
        assert_eq!(outcomes[1], Outcome::StopCalibration);
        assert!(!cal.calibrating());
        assert_eq!(cal.mapped(Function::Pitch), Some(1), "stopping reads the stored calibration back");
    }

    #[test]
    fn a_reversed_throttle_on_a_copter_refuses_to_save() {
        let params = apm_params();
        let lookup = |name: &str| params.get(name).copied();
        let mut cal = RcCal::for_vehicle(&copter(), 2);
        cal.channel_values(&[1500, 1500, 1500, 1500], 0);
        cal.next(&copter(), &lookup);
        cal.next(&copter(), &lookup);
        let (_, _) = hold(&mut cal, &[1500, 1500, 1100, 1500], 100);
        assert!(cal.reversed(Function::Throttle));
        cal.step = Some(STEPS.len() - 1);
        assert_eq!(cal.next(&copter(), &lookup)[0], Outcome::ThrottleReversed);
        assert_eq!(cal.json()["throttleReversed"], true, "the head opens QGC's Throttle channel reversed dialog from this");
    }

    #[test]
    fn too_few_channels_cannot_start_and_values_are_clamped() {
        let params = apm_params();
        let lookup = |name: &str| params.get(name).copied();
        let mut cal = RcCal::for_vehicle(&copter(), 9);
        assert_eq!(cal.transmitter_mode, 2, "an invalid mode falls back to 2");
        cal.channel_values(&[1500, 1500, 1500], 0);
        assert!(cal.next(&copter(), &lookup).is_empty());
        assert_eq!(clamped(&[900, 2100, 1500]), vec![1000, 2000, 1500]);
    }

    #[test]
    fn the_diagram_shows_each_step_like_remote_control_calibration_controller() {
        let mut cal = RcCal::for_vehicle(&copter(), 2);
        cal.step = Some(0);
        assert_eq!(cal.stick_positions(), [0, -1, 0, 0], "throttle-down neutral in mode 2 pulls the left stick down");
        cal.set_centered_throttle(true);
        assert_eq!(cal.stick_positions(), [0; 4], "centred throttle centres both sticks");
        cal.step = Some(STEPS.iter().position(|s| *s == Step::Detect(Function::Yaw)).unwrap());
        assert_eq!(cal.stick_positions(), [1, 0, 0, 0], "mode 2 yaw is the left stick");
        cal.transmitter_mode = 1;
        cal.step = Some(STEPS.iter().position(|s| *s == Step::Min(Function::Pitch)).unwrap());
        assert_eq!(cal.stick_positions(), [0, -1, 0, 0], "mode 1 pitch is the left stick");
        cal.step = None;
        assert_eq!(cal.json()["stickDisplayPositions"], json!([0, 0, 0, 0]));
    }
}
