use serde_json::{Value, json};

use crate::joystick::{AxisCalibration, Function};

pub const MINIMUM_CHANNELS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Profile {
    pub center: i32,
    pub default_min: i32,
    pub default_max: i32,
    pub rough_center_delta: i32,
    pub move_delta: i32,
    pub settle_delta: i32,
    pub settle_ms: u64,
    pub valid_min: i32,
    pub valid_max: i32,
}

pub const JOYSTICK: Profile = {
    let (min, max) = (-32768, 32767);
    let range = max - min;
    Profile {
        center: 0,
        default_min: min,
        default_max: max,
        rough_center_delta: 700,
        move_delta: 32768 / 2,
        settle_delta: 1000,
        settle_ms: 300,
        valid_min: (min as f32 + range as f32 * 0.3) as i32,
        valid_max: (max as f32 - range as f32 * 0.3) as i32,
    }
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Input {
    CenterWaitBegin,
    StickDetect,
    StickMin,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Next {
    SaveTrims,
    Save,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Message {
    Neutral,
    ThrottleUp,
    ThrottleDown,
    YawRight,
    YawLeft,
    RollRight,
    RollLeft,
    PitchUp,
    PitchDown,
    ExtensionHigh,
    ExtensionLow,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stick {
    Left,
    Right,
}

fn stick_for(message: Message, mode: u8) -> Stick {
    let left_in = |modes: [u8; 2]| if modes.contains(&mode) { Stick::Left } else { Stick::Right };
    match message {
        Message::ThrottleUp | Message::ThrottleDown => left_in([2, 4]),
        Message::YawRight | Message::YawLeft => left_in([1, 2]),
        Message::RollRight | Message::RollLeft => left_in([3, 4]),
        _ => left_in([1, 3]),
    }
}

pub(crate) fn stick_positions(message: Message, mode: u8) -> [i32; 4] {
    let moved = match message {
        Message::ThrottleUp | Message::PitchUp => Some((0, 1)),
        Message::ThrottleDown | Message::PitchDown => Some((0, -1)),
        Message::YawRight | Message::RollRight => Some((1, 0)),
        Message::YawLeft | Message::RollLeft => Some((-1, 0)),
        Message::ExtensionHigh => return [1, 0, 0, 0],
        Message::ExtensionLow => return [-1, 0, 0, 0],
        Message::Neutral | Message::Complete => None,
    };
    moved.map_or([0; 4], |(x, y)| match stick_for(message, mode) {
        Stick::Left => [x, y, 0, 0],
        Stick::Right => [0, 0, x, y],
    })
}

#[derive(Debug, Clone, Copy)]
struct Step {
    function: Option<Function>,
    message: Message,
    input: Input,
    next: Next,
}

const fn step(function: Option<Function>, message: Message, input: Input, next: Next) -> Step {
    Step { function, message, input, next }
}

const fn axis(function: Function) -> [Step; 2] {
    [step(Some(function), Message::ExtensionHigh, Input::StickDetect, Next::None), step(Some(function), Message::ExtensionLow, Input::StickMin, Next::None)]
}

const STEPS: [Step; 26] = {
    let [p1, p2] = axis(Function::PitchExtension);
    let [r1, r2] = axis(Function::RollExtension);
    let [a1, a2] = axis(Function::Additional1);
    let [b1, b2] = axis(Function::Additional2);
    let [c1, c2] = axis(Function::Additional3);
    let [d1, d2] = axis(Function::Additional4);
    let [e1, e2] = axis(Function::Additional5);
    let [f1, f2] = axis(Function::Additional6);
    [
        step(None, Message::Neutral, Input::CenterWaitBegin, Next::SaveTrims),
        step(Some(Function::Throttle), Message::ThrottleUp, Input::StickDetect, Next::None),
        step(Some(Function::Throttle), Message::ThrottleDown, Input::StickMin, Next::None),
        step(Some(Function::Yaw), Message::YawRight, Input::StickDetect, Next::None),
        step(Some(Function::Yaw), Message::YawLeft, Input::StickMin, Next::None),
        step(Some(Function::Roll), Message::RollRight, Input::StickDetect, Next::None),
        step(Some(Function::Roll), Message::RollLeft, Input::StickMin, Next::None),
        step(Some(Function::Pitch), Message::PitchUp, Input::StickDetect, Next::None),
        step(Some(Function::Pitch), Message::PitchDown, Input::StickMin, Next::None),
        p1, p2, r1, r2, a1, a2, b1, b2, c1, c2, d1, d2, e1, e2, f1, f2,
        step(None, Message::Complete, Input::None, Next::Save),
    ]
};

fn extension_name(function: Function) -> &'static str {
    match function {
        Function::PitchExtension => "Pitch",
        Function::RollExtension => "Roll",
        Function::Additional1 => "Aux 1",
        Function::Additional2 => "Aux 2",
        Function::Additional3 => "Aux 3",
        Function::Additional4 => "Aux 4",
        Function::Additional5 => "Aux 5",
        Function::Additional6 => "Aux 6",
        _ => "Unknown",
    }
}

fn text(step: &Step) -> String {
    let name = step.function.map(extension_name).unwrap_or("Unknown");
    match step.message {
        Message::Neutral => "* Center all sticks as shown in diagram.\n* Make sure any additional axes are at a neutral position.\n* Please ensure all motor power is disconnected from the vehicle.\n* Click Next to continue".to_string(),
        Message::ThrottleUp => "Move the Throttle stick all the way up and hold it there...".to_string(),
        Message::ThrottleDown => "Move the Throttle stick all the way down and leave it there...".to_string(),
        Message::YawRight => "Move the Yaw stick all the way to the right and hold it there...".to_string(),
        Message::YawLeft => "Move the Yaw stick all the way to the left and hold it there...".to_string(),
        Message::RollRight => "Move the Roll stick all the way to the right and hold it there...".to_string(),
        Message::RollLeft => "Move the Roll stick all the way to the left and hold it there...".to_string(),
        Message::PitchUp => "Move the Pitch stick all the way up and hold it there...".to_string(),
        Message::PitchDown => "Move the Pitch stick all the way down and hold it there...".to_string(),
        Message::ExtensionHigh => format!("Move the {name} Extension stick to its high value position and hold it there..."),
        Message::ExtensionLow => format!("* Move the {name} Extension stick to its low value position and hold it there...\n* Select 'One-Sided' for controls like gamepad triggers."),
        Message::Complete => "All settings have been captured. Click Next to write the new parameters to your board.".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Channel {
    pub function: Option<Function>,
    pub reversed: bool,
    pub min: i32,
    pub max: i32,
    pub trim: i32,
    pub deadband: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    None,
    Refused(String),
    Save(Vec<Channel>),
}

#[derive(Debug, Clone)]
pub struct StickCal {
    profile: Profile,
    enabled: [bool; 12],
    pub channels: Vec<Channel>,
    raw: Vec<i32>,
    saved: Vec<i32>,
    step: Option<usize>,
    detect: Option<usize>,
    detect_value: i32,
    settle_since: Option<u64>,
    next_enabled: bool,
    status: String,
}

fn function_slot(function: Function) -> usize {
    function.index()
}

impl StickCal {
    pub fn new(profile: Profile, channel_count: usize, optional_enabled: [bool; 8]) -> StickCal {
        let enabled: [bool; 12] = std::array::from_fn(|i| i < 4 || optional_enabled[i - 4]);
        let center = Channel { function: None, reversed: false, min: profile.center, max: profile.center, trim: profile.center, deadband: 0 };
        StickCal { profile, enabled, channels: vec![center; channel_count], raw: vec![0; channel_count], saved: vec![0; channel_count], step: None, detect: None, detect_value: 0, settle_since: None, next_enabled: true, status: String::new() }
    }

    pub fn calibrating(&self) -> bool {
        self.step.is_some()
    }

    fn mapped(&self, function: Function) -> Option<usize> {
        self.channels.iter().position(|c| c.function == Some(function))
    }

    fn reset(&mut self) {
        let center = self.profile.center;
        self.channels.iter_mut().for_each(|c| *c = Channel { function: None, reversed: false, min: center, max: center, trim: center, deadband: 0 });
    }

    pub fn start(&mut self) -> Outcome {
        if self.channels.len() < MINIMUM_CHANNELS {
            return Outcome::Refused(format!("Detected {} channels. To operate vehicle, you need at least {MINIMUM_CHANNELS} channels.", self.channels.len()));
        }
        self.reset();
        self.step = Some(0);
        self.setup();
        Outcome::None
    }

    pub fn cancel(&mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        self.step = None;
        self.status.clear();
        self.next_enabled = true;
    }

    fn current(&self) -> Option<&'static Step> {
        self.step.and_then(|i| STEPS.get(i))
    }

    fn advance(&mut self) {
        self.step = self.step.map(|i| i + 1).filter(|i| *i < STEPS.len());
        match self.step {
            Some(_) => self.setup(),
            None => self.stop(),
        }
    }

    fn setup(&mut self) {
        let Some(step) = self.current() else { return };
        if step.function.is_some_and(|f| !self.enabled[function_slot(f)]) {
            return self.advance();
        }
        self.status = text(step);
        self.detect = None;
        self.settle_since = None;
        self.saved = self.raw.clone();
        self.next_enabled = step.next != Next::None;
    }

    pub fn one_sided_visible(&self) -> bool {
        self.current().is_some_and(|s| s.message == Message::ExtensionLow && s.function.is_some_and(|f| f.index() >= Function::Additional1.index()))
    }

    pub fn one_sided(&mut self) {
        if !self.one_sided_visible() {
            return;
        }
        let Some(channel) = self.current().and_then(|s| s.function).and_then(|f| self.mapped(f)) else { return };
        let info = &mut self.channels[channel];
        match info.reversed {
            true => info.max = info.trim,
            false => info.min = info.trim,
        }
        self.advance();
    }

    pub fn next(&mut self) -> Outcome {
        let Some(step) = self.current() else { return self.start() };
        match step.next {
            Next::SaveTrims => {
                self.channels.iter_mut().zip(&self.raw).for_each(|(c, raw)| c.trim = *raw);
                self.advance();
                Outcome::None
            }
            Next::Save => {
                self.validate();
                let saved = self.channels.clone();
                self.stop();
                Outcome::Save(saved)
            }
            Next::None => Outcome::None,
        }
    }

    fn settled(&mut self, value: i32, now_ms: u64) -> bool {
        if (self.detect_value - value).abs() > self.profile.settle_delta {
            self.detect_value = value;
            self.settle_since = None;
            return false;
        }
        match self.settle_since {
            Some(since) => now_ms.saturating_sub(since) > self.profile.settle_ms,
            None => {
                self.settle_since = Some(now_ms);
                false
            }
        }
    }

    pub fn channel_values(&mut self, values: &[i32], now_ms: u64) {
        let count = values.len().min(self.channels.len());
        self.raw[..count].copy_from_slice(&values[..count]);
        let Some(at) = self.step else { return };
        (0..count).for_each(|channel| self.input(at, channel, values[channel], now_ms));
    }

    fn input(&mut self, at: usize, channel: usize, value: i32, now_ms: u64) {
        let Some(step) = self.step.filter(|current| *current == at).and_then(|i| STEPS.get(i)) else { return };
        match step.input {
            Input::CenterWaitBegin => {
                let deadband = ((value.abs() as f64) * 1.1) as i32;
                if deadband > self.channels[channel].deadband {
                    self.channels[channel].deadband = deadband.min(self.profile.valid_max);
                }
                self.next_enabled = true;
            }
            Input::StickDetect => self.stick_detect(step, channel, value, now_ms),
            Input::StickMin => self.stick_min(step, channel, value, now_ms),
            Input::None => {}
        }
    }

    fn stick_detect(&mut self, step: &Step, channel: usize, value: i32, now_ms: u64) {
        let Some(function) = step.function else { return };
        if self.channels[channel].function.is_some() {
            return;
        }
        match self.detect {
            None => {
                if (self.saved[channel] - value).abs() > self.profile.move_delta {
                    self.detect = Some(channel);
                    self.detect_value = value;
                }
            }
            Some(detected) if detected == channel => {
                if self.settled(value, now_ms) {
                    let reversed = value < self.saved[channel];
                    let info = &mut self.channels[channel];
                    info.function = Some(function);
                    info.reversed = reversed;
                    match reversed {
                        true => info.min = value,
                        false => info.max = value,
                    }
                    self.advance();
                }
            }
            Some(_) => {}
        }
    }

    fn stick_min(&mut self, step: &Step, channel: usize, value: i32, now_ms: u64) {
        let Some(function) = step.function else { return };
        if self.mapped(function) != Some(channel) {
            return;
        }
        let center = self.profile.center;
        match self.detect {
            None => {
                let moved = match self.channels[channel].reversed {
                    true => value > center + self.profile.move_delta,
                    false => value < center - self.profile.move_delta,
                };
                if moved {
                    self.detect = Some(channel);
                    self.detect_value = value;
                }
            }
            Some(_) => {
                if self.settled(value, now_ms) {
                    let info = &mut self.channels[channel];
                    match info.reversed {
                        true => info.max = value,
                        false => info.min = value,
                    }
                    self.advance();
                }
            }
        }
    }

    fn validate(&mut self) {
        let p = self.profile;
        self.channels.iter_mut().for_each(|c| {
            let extension = c.function.is_some_and(|f| f.index() >= Function::PitchExtension.index());
            let one_sided = extension && ((c.trim == c.min && c.max >= p.valid_max) || (c.trim == c.max && c.min <= p.valid_min));
            if !one_sided && (c.min > p.valid_min || c.max < p.valid_max) {
                c.min = p.default_min;
                c.max = p.default_max;
                c.trim = c.min + (c.max - c.min) / 2;
            } else if !one_sided {
                c.trim = match extension {
                    false => c.trim.clamp(c.min, c.max),
                    true => c.min + (c.max - c.min) / 2,
                };
            }
        });
    }

    pub fn calibration(channel: &Channel) -> AxisCalibration {
        AxisCalibration { min: channel.min, max: channel.max, center: channel.trim, deadband: channel.deadband, reversed: channel.reversed }
    }

    pub fn stick_positions(&self, transmitter_mode: u8) -> [i32; 4] {
        self.current().map_or([0; 4], |step| stick_positions(step.message, transmitter_mode))
    }

    pub fn json(&self) -> Value {
        json!({
            "calibrating": self.calibrating(),
            "statusText": self.status,
            "nextText": if self.calibrating() { "Next" } else { "Calibrate" },
            "nextEnabled": self.next_enabled,
            "cancelEnabled": self.calibrating(),
            "oneSidedVisible": self.one_sided_visible(),
            "singleStickDisplay": self.current().is_some_and(|s| matches!(s.message, Message::ExtensionHigh | Message::ExtensionLow)),
            "channels": self.raw.iter().zip(&self.channels).enumerate().map(|(i, (raw, c))| json!({
                "index": i,
                "raw": raw,
                "function": c.function.map(Function::id),
                "deadband": c.deadband,
            })).collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_diagram_moves_the_stick_each_mode_puts_the_function_on() {
        assert_eq!(stick_positions(Message::ThrottleUp, 2), [0, 1, 0, 0], "mode 2 throttle is on the left");
        assert_eq!(stick_positions(Message::ThrottleUp, 1), [0, 0, 0, 1]);
        assert_eq!(stick_positions(Message::YawLeft, 3), [0, 0, -1, 0]);
        assert_eq!(stick_positions(Message::RollRight, 4), [1, 0, 0, 0]);
        assert_eq!(stick_positions(Message::PitchDown, 2), [0, 0, 0, -1]);
        assert_eq!(stick_positions(Message::Neutral, 2), [0; 4]);
        assert_eq!(stick_positions(Message::ExtensionLow, 4), [-1, 0, 0, 0]);
    }

    fn hold(cal: &mut StickCal, values: [i32; 4], from_ms: u64) -> u64 {
        (0..5).fold(from_ms, |at, i| {
            cal.channel_values(&values, at + i * 100);
            at + i * 100
        }) + 500
    }

    #[test]
    fn a_gamepad_is_calibrated_by_moving_each_stick_and_the_axes_are_mapped() {
        let mut cal = StickCal::new(JOYSTICK, 4, [false; 8]);
        assert_eq!(cal.next(), Outcome::None);
        assert!(cal.calibrating());
        cal.channel_values(&[300, -200, 0, 100], 0);
        assert_eq!(cal.channels[0].deadband, 330, "the neutral step records 10% over each axis's resting noise as its deadband");
        cal.next();
        assert!(cal.json()["statusText"].as_str().unwrap().starts_with("Move the Throttle stick all the way up"));
        let at = hold(&mut cal, [0, -32000, 0, 0], 1_000);
        let at = hold(&mut cal, [0, 32000, 0, 0], at);
        let at = hold(&mut cal, [32000, 0, 0, 0], at);
        let at = hold(&mut cal, [-32000, 0, 0, 0], at);
        let at = hold(&mut cal, [0, 0, 32000, 0], at);
        let at = hold(&mut cal, [0, 0, -32000, 0], at);
        let at = hold(&mut cal, [0, 0, 0, -32000], at);
        hold(&mut cal, [0, 0, 0, 32000], at);
        assert!(cal.json()["statusText"].as_str().unwrap().starts_with("All settings have been captured"));
        let Outcome::Save(channels) = cal.next() else { panic!("the last Next saves") };
        let functions: Vec<Option<Function>> = channels.iter().map(|c| c.function).collect();
        assert_eq!(functions, [Some(Function::Yaw), Some(Function::Throttle), Some(Function::Roll), Some(Function::Pitch)]);
        assert!(channels[1].reversed, "throttle up read negative, so that axis is reversed");
        assert!(!cal.calibrating());
    }

    #[test]
    fn too_few_axes_are_refused_and_disabled_extensions_are_skipped() {
        let mut short = StickCal::new(JOYSTICK, 2, [false; 8]);
        assert_eq!(short.start(), Outcome::Refused("Detected 2 channels. To operate vehicle, you need at least 4 channels.".into()));
        let mut aux = StickCal::new(JOYSTICK, 5, [false, false, true, false, false, false, false, false]);
        aux.start();
        aux.step = Some(13);
        aux.setup();
        assert!(aux.json()["statusText"].as_str().unwrap().contains("Aux 1"));
        aux.channels[4].function = Some(Function::Additional1);
        aux.step = Some(14);
        aux.setup();
        assert!(aux.one_sided_visible(), "a trigger can be calibrated one-sided");
        aux.one_sided();
        assert_eq!(aux.json()["statusText"].as_str().unwrap().starts_with("All settings"), true, "Aux 2-6 are off, so it skips to the end");
    }
}
