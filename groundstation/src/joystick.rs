use serde_json::{Value, json};

use crate::router::Backend;

pub const AXIS_MIN: i32 = -32768;
pub const AXIS_MAX: i32 = 32767;
pub const HAT_BUTTONS_PER_HAT: usize = 4;
pub const HAT_DIRECTIONS: [u8; HAT_BUTTONS_PER_HAT] = [0x01, 0x04, 0x08, 0x02];
pub const HAT_DIRECTION_IDS: [&str; HAT_BUTTONS_PER_HAT] = ["up", "down", "left", "right"];
pub const AXES_SCALING: f32 = 1000.0;
pub const MANUAL_CONTROL_EXTENSIONS: usize = 8;
pub const VEHICLE_BUTTON_BITS: usize = 32;
pub const RC_OVERRIDE_CHANNELS: usize = 6;
pub const RC_OVERRIDE_FIRST_CHANNEL: usize = 5;
pub const RC_OVERRIDE_RELEASE_COUNT: u8 = 3;
pub const RC_RELEASE: [u16; RC_OVERRIDE_CHANNELS] = [0, 0, 0, 0, u16::MAX - 1, u16::MAX - 1];
pub const RC_IGNORE: [u16; RC_OVERRIDE_CHANNELS] = [u16::MAX, u16::MAX, u16::MAX, u16::MAX, 0, 0];
pub const PWM_MIN: f32 = 800.0;
pub const PWM_CENTER: f32 = 1500.0;
pub const PWM_MAX: f32 = 2200.0;
pub const PWM_HALF_SPAN: f32 = 700.0;
pub const PWM_ONE_SIDED_SPAN: f32 = 1400.0;
pub const AXIS_FREQUENCY_DEFAULT_HZ: f64 = 25.0;
pub const AXIS_FREQUENCY_MIN_HZ: f64 = 0.25;
pub const AXIS_FREQUENCY_MAX_HZ: f64 = 200.0;
pub const BUTTON_FREQUENCY_DEFAULT_HZ: f64 = 5.0;
pub const BUTTON_FREQUENCY_MIN_HZ: f64 = 0.25;
pub const BUTTON_FREQUENCY_MAX_HZ: f64 = 50.0;
pub const EXPONENTIAL_PCT_DEFAULT: f64 = 0.0;
pub const EXPONENTIAL_PCT_MIN: f64 = 0.0;
pub const EXPONENTIAL_PCT_MAX: f64 = 50.0;
pub const TRANSMITTER_MODE_DEFAULT: u8 = 2;
pub const TRANSMITTER_MODE_STORED: u8 = 2;
pub const THROTTLE_SMOOTHING_STEP: f32 = 40.0 / 1000.0;
pub const CIRCLE_CORRECTION_LIMIT: f32 = std::f32::consts::FRAC_PI_4;
pub const ACTION_NONE: &str = "No Action";
pub const ACTION_NONE_ID: &str = "none";
pub const MANUAL_CONTROL: &str = "MANUAL_CONTROL";
pub const RC_CHANNELS_OVERRIDE: &str = "RC_CHANNELS_OVERRIDE";

pub const CALIBRATION_MIN_NOT_BELOW_MAX: &str = "minNotBelowMax";
pub const CALIBRATION_CENTRE_OUT_OF_RANGE: &str = "centreOutOfRange";
pub const CALIBRATION_DEADBAND_NEGATIVE: &str = "deadbandNegative";
pub const CALIBRATION_DEADBAND_EXCEEDS_TRAVEL: &str = "deadbandExceedsTravel";

pub const BLOCKED_NOT_POLLING: &str = "notPolling";
pub const BLOCKED_CONFIGURING: &str = "configuring";
pub const BLOCKED_NOT_CALIBRATED: &str = "notCalibrated";
pub const BLOCKED_MAPPING_INCOMPLETE: &str = "mappingIncomplete";
pub const BLOCKED_NO_SAMPLE: &str = "noSample";
pub const BLOCKED_MISSING_AXIS_DATA: &str = "missingAxisData";
pub const BLOCKED_INVALID_CALIBRATION: &str = "invalidCalibration";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Function {
    Roll,
    Pitch,
    Yaw,
    Throttle,
    PitchExtension,
    RollExtension,
    Additional1,
    Additional2,
    Additional3,
    Additional4,
    Additional5,
    Additional6,
}

pub const FUNCTIONS: [(Function, &str); 12] = [
    (Function::Roll, "roll"),
    (Function::Pitch, "pitch"),
    (Function::Yaw, "yaw"),
    (Function::Throttle, "throttle"),
    (Function::PitchExtension, "pitchExtension"),
    (Function::RollExtension, "rollExtension"),
    (Function::Additional1, "additionalAxis1"),
    (Function::Additional2, "additionalAxis2"),
    (Function::Additional3, "additionalAxis3"),
    (Function::Additional4, "additionalAxis4"),
    (Function::Additional5, "additionalAxis5"),
    (Function::Additional6, "additionalAxis6"),
];

pub const ATTITUDE: [Function; 4] = [Function::Roll, Function::Pitch, Function::Yaw, Function::Throttle];

pub const OPTIONAL: [Function; MANUAL_CONTROL_EXTENSIONS] = [
    Function::PitchExtension,
    Function::RollExtension,
    Function::Additional1,
    Function::Additional2,
    Function::Additional3,
    Function::Additional4,
    Function::Additional5,
    Function::Additional6,
];

pub const MANUAL_CONTROL_ONLY_EXTENSIONS: usize = 2;

const _: () = assert!(MANUAL_CONTROL_EXTENSIONS == 8);
const _: () = assert!(OPTIONAL.len() == MANUAL_CONTROL_ONLY_EXTENSIONS + RC_OVERRIDE_CHANNELS);

pub const TRANSMITTER_MODES: [[Function; 4]; 4] = [
    [Function::Yaw, Function::Pitch, Function::Roll, Function::Throttle],
    [Function::Yaw, Function::Throttle, Function::Roll, Function::Pitch],
    [Function::Roll, Function::Pitch, Function::Yaw, Function::Throttle],
    [Function::Roll, Function::Throttle, Function::Yaw, Function::Pitch],
];

impl Function {
    pub fn id(self) -> &'static str {
        FUNCTIONS.iter().find(|(f, _)| *f == self).map(|(_, id)| *id).unwrap_or("")
    }

    pub fn parse(id: &str) -> Option<Function> {
        FUNCTIONS.iter().find(|(_, name)| *name == id).map(|(f, _)| *f)
    }

    pub fn index(self) -> usize {
        FUNCTIONS.iter().position(|(f, _)| *f == self).unwrap_or(0)
    }

    pub fn extension_bit(self) -> Option<usize> {
        OPTIONAL.iter().position(|f| *f == self)
    }

    pub fn rc_channel(self) -> Option<usize> {
        self.extension_bit().filter(|bit| *bit >= MANUAL_CONTROL_ONLY_EXTENSIONS).map(|bit| RC_OVERRIDE_FIRST_CHANNEL + bit - MANUAL_CONTROL_ONLY_EXTENSIONS)
    }

    pub fn carried_by(self) -> &'static [&'static str] {
        match self.rc_channel() {
            Some(_) => &[MANUAL_CONTROL, RC_CHANNELS_OVERRIDE],
            None => &[MANUAL_CONTROL],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonEvent {
    #[default]
    None,
    Down,
    Repeat,
    Up,
}

impl ButtonEvent {
    pub fn id(self) -> &'static str {
        match self {
            ButtonEvent::None => "none",
            ButtonEvent::Down => "down",
            ButtonEvent::Repeat => "repeat",
            ButtonEvent::Up => "up",
        }
    }

    fn step(self, pressed: bool) -> ButtonEvent {
        match (pressed, self) {
            (true, ButtonEvent::None) => ButtonEvent::Down,
            (true, ButtonEvent::Down) => ButtonEvent::Repeat,
            (true, other) => other,
            (false, ButtonEvent::Down | ButtonEvent::Repeat) => ButtonEvent::Up,
            (false, ButtonEvent::Up) => ButtonEvent::None,
            (false, other) => other,
        }
    }

    fn held(self) -> bool {
        matches!(self, ButtonEvent::Down | ButtonEvent::Repeat)
    }
}

pub const ACTIONS: [(&str, &str, bool, bool); 31] = [
    ("arm", "Arm", false, false),
    ("disarm", "Disarm", false, false),
    ("toggleArm", "Toggle Arm", false, false),
    ("vtolFixedWing", "VTOL: Fixed Wing", false, false),
    ("vtolMultiRotor", "VTOL: Multi-Rotor", false, false),
    ("continuousZoomIn", "Continuous Zoom In", true, true),
    ("continuousZoomOut", "Continuous Zoom Out", true, true),
    ("stepZoomIn", "Step Zoom In", false, true),
    ("stepZoomOut", "Step Zoom Out", false, true),
    ("nextVideoStream", "Next Video Stream", false, false),
    ("previousVideoStream", "Previous Video Stream", false, false),
    ("nextCamera", "Next Camera", false, false),
    ("previousCamera", "Previous Camera", false, false),
    ("triggerCamera", "Trigger Camera", false, false),
    ("startRecordingVideo", "Start Recording Video", false, false),
    ("stopRecordingVideo", "Stop Recording Video", false, false),
    ("toggleRecordingVideo", "Toggle Recording Video", false, false),
    ("gimbalDown", "Gimbal Down", true, false),
    ("gimbalUp", "Gimbal Up", true, false),
    ("gimbalLeft", "Gimbal Left", true, false),
    ("gimbalRight", "Gimbal Right", true, false),
    ("gimbalCenter", "Gimbal Center", false, false),
    ("gimbalYawLock", "Gimbal Yaw Lock", false, false),
    ("gimbalYawFollow", "Gimbal Yaw Follow", false, false),
    ("emergencyStop", "Emergency Stop", false, false),
    ("gripperClose", "Gripper Close", false, false),
    ("gripperOpen", "Gripper Open", false, false),
    ("landingGearDeploy", "Landing gear deploy", false, false),
    ("landingGearRetract", "Landing gear retract", false, false),
    ("motorInterlockEnable", "Motor Interlock enable", false, false),
    ("motorInterlockDisable", "Motor Interlock disable", false, false),
];

pub fn action_id(action: &str) -> Option<&'static str> {
    match action {
        ACTION_NONE => Some(ACTION_NONE_ID),
        _ => ACTIONS.iter().find(|(_, name, _, _)| *name == action).map(|(id, _, _, _)| *id),
    }
}

pub fn can_repeat(action: &str) -> bool {
    ACTIONS.iter().any(|(_, name, _, repeat)| *name == action && *repeat)
}

pub fn accepts(action: &str, event: ButtonEvent) -> bool {
    if action.is_empty() || action == ACTION_NONE {
        return false;
    }
    match (ACTIONS.iter().find(|(_, name, _, _)| *name == action), event) {
        (_, ButtonEvent::None) => false,
        (_, ButtonEvent::Down) => true,
        (Some((_, _, up, _)), ButtonEvent::Up) => *up,
        (Some((_, _, _, repeat)), ButtonEvent::Repeat) => *repeat,
        (None, _) => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisCalibration {
    pub min: i32,
    pub max: i32,
    pub center: i32,
    pub deadband: i32,
    pub reversed: bool,
}

impl Default for AxisCalibration {
    fn default() -> Self {
        AxisCalibration { min: AXIS_MIN, max: AXIS_MAX, center: 0, deadband: 0, reversed: false }
    }
}

impl AxisCalibration {
    pub fn fault(&self) -> Option<&'static str> {
        let travel = [self.max.saturating_sub(self.center), self.center.saturating_sub(self.min)];
        [
            (self.min >= self.max, CALIBRATION_MIN_NOT_BELOW_MAX),
            (self.center < self.min || self.center > self.max, CALIBRATION_CENTRE_OUT_OF_RANGE),
            (self.deadband < 0, CALIBRATION_DEADBAND_NEGATIVE),
            (travel.iter().any(|span| *span > 0 && self.deadband >= *span), CALIBRATION_DEADBAND_EXCEEDS_TRAVEL),
        ]
        .into_iter()
        .find_map(|(broken, reason)| broken.then_some(reason))
    }

    pub fn valid(&self) -> bool {
        self.fault().is_none()
    }

    pub fn one_sided(&self) -> bool {
        self.center == self.min || self.center == self.max
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AdditionalAxes {
    #[default]
    ManualControl,
    RcChannelsOverride,
}

impl AdditionalAxes {
    pub fn id(self) -> &'static str {
        match self {
            AdditionalAxes::ManualControl => MANUAL_CONTROL,
            AdditionalAxes::RcChannelsOverride => RC_CHANNELS_OVERRIDE,
        }
    }

    pub fn stored(self) -> u8 {
        match self {
            AdditionalAxes::ManualControl => 0,
            AdditionalAxes::RcChannelsOverride => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThrottleMode {
    CenterZero,
    #[default]
    DownZero,
}

impl ThrottleMode {
    pub fn id(self) -> &'static str {
        match self {
            ThrottleMode::CenterZero => "CENTER_ZERO",
            ThrottleMode::DownZero => "DOWN_ZERO",
        }
    }

    pub fn stored(self) -> u8 {
        match self {
            ThrottleMode::CenterZero => 0,
            ThrottleMode::DownZero => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub calibrated: bool,
    pub circle_correction: bool,
    pub use_deadband: bool,
    pub negative_thrust: bool,
    pub throttle_smoothing: bool,
    pub throttle_mode: ThrottleMode,
    pub axis_frequency_hz: f64,
    pub button_frequency_hz: f64,
    pub exponential_pct: f64,
    pub additional_axes: AdditionalAxes,
    pub optional_enabled: [bool; 8],
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            calibrated: false,
            circle_correction: false,
            use_deadband: false,
            negative_thrust: false,
            throttle_smoothing: false,
            throttle_mode: ThrottleMode::DownZero,
            axis_frequency_hz: AXIS_FREQUENCY_DEFAULT_HZ,
            button_frequency_hz: BUTTON_FREQUENCY_DEFAULT_HZ,
            exponential_pct: EXPONENTIAL_PCT_DEFAULT,
            additional_axes: AdditionalAxes::ManualControl,
            optional_enabled: [false; 8],
        }
    }
}

fn interval_ms(hz: f64, low: f64, high: f64, fallback: f64) -> u64 {
    let rate = if hz.is_finite() { hz } else { fallback };
    (1000.0 / rate.clamp(low, high)) as u64
}

pub fn poll_interval_ms() -> u64 {
    interval_ms(AXIS_FREQUENCY_MAX_HZ, AXIS_FREQUENCY_MIN_HZ, AXIS_FREQUENCY_MAX_HZ, AXIS_FREQUENCY_DEFAULT_HZ)
        .min(interval_ms(BUTTON_FREQUENCY_MAX_HZ, BUTTON_FREQUENCY_MIN_HZ, BUTTON_FREQUENCY_MAX_HZ, BUTTON_FREQUENCY_DEFAULT_HZ))
        / 2
}

impl Settings {
    pub fn enabled(&self, function: Function) -> bool {
        function.extension_bit().is_none_or(|bit| self.optional_enabled[bit])
    }

    pub fn wanted(&self) -> Vec<Function> {
        ATTITUDE.into_iter().chain(OPTIONAL.into_iter().filter(|f| self.enabled(*f))).collect()
    }

    pub fn axis_interval_ms(&self) -> u64 {
        interval_ms(self.axis_frequency_hz, AXIS_FREQUENCY_MIN_HZ, AXIS_FREQUENCY_MAX_HZ, AXIS_FREQUENCY_DEFAULT_HZ)
    }

    pub fn button_interval_ms(&self) -> u64 {
        interval_ms(self.button_frequency_hz, BUTTON_FREQUENCY_MIN_HZ, BUTTON_FREQUENCY_MAX_HZ, BUTTON_FREQUENCY_DEFAULT_HZ)
    }

    pub fn effective_throttle_mode(&self, support: Support) -> ThrottleMode {
        match self.throttle_mode == ThrottleMode::CenterZero && support.throttle_mode_center_zero {
            true => ThrottleMode::CenterZero,
            false => ThrottleMode::DownZero,
        }
    }

    pub fn centre_zero(&self, support: Support) -> bool {
        self.effective_throttle_mode(support) == ThrottleMode::CenterZero
    }

    pub fn negative_thrust_active(&self, support: Support) -> bool {
        self.centre_zero(support) && self.negative_thrust && support.negative_thrust
    }

    pub fn smoothing_active(&self, support: Support) -> bool {
        self.throttle_smoothing && self.centre_zero(support)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Support {
    pub throttle_mode_center_zero: bool,
    pub negative_thrust: bool,
    pub default_transmitter_mode: u8,
}

impl Default for Support {
    fn default() -> Self {
        Support { throttle_mode_center_zero: true, negative_thrust: false, default_transmitter_mode: TRANSMITTER_MODE_DEFAULT }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Polling {
    pub vehicle: bool,
    pub configuration: bool,
}

impl Polling {
    fn idle(self) -> bool {
        !self.vehicle && !self.configuration
    }

    fn commands(self) -> bool {
        self.vehicle && !self.configuration
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Input<'a> {
    pub axes: &'a [i32],
    pub buttons: &'a [bool],
    pub hats: &'a [u8],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub action: String,
    pub repeat: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    Axes { roll: f32, pitch: f32, yaw: f32, throttle: f32 },
    ManualControl { x: i16, y: i16, z: i16, r: i16, buttons: u16, buttons2: u16, enabled_extensions: u8, extensions: [i16; MANUAL_CONTROL_EXTENSIONS] },
    RcChannelsOverride { channels: [u16; RC_OVERRIDE_CHANNELS] },
    Action { action: String, event: ButtonEvent },
    RawAxes { values: Vec<Option<i32>> },
    RawButton { index: usize, pressed: bool },
    MappingIncomplete { missing: Vec<Function> },
    MissingAxisData { axes: Vec<usize> },
    RequireCalibration { missing: Vec<Function> },
}

pub fn adjust_range(value: i32, calibration: &AxisCalibration, with_deadband: bool) -> f32 {
    let (basis, normalized, length) = if calibration.center == calibration.min {
        (1.0, 0.max(value - calibration.center) as f32, (calibration.max - calibration.center) as f32)
    } else if calibration.center == calibration.max {
        (-1.0, 0.max(calibration.center - value) as f32, (calibration.center - calibration.min) as f32)
    } else if value > calibration.center {
        (1.0, (value - calibration.center) as f32, (calibration.max - calibration.center) as f32)
    } else {
        (-1.0, (calibration.center - value) as f32, (calibration.center - calibration.min) as f32)
    };
    let deadband = calibration.deadband.max(0) as f32;
    if length <= 0.0 || length - deadband <= 0.0 {
        return 0.0;
    }
    let percent = match (with_deadband, normalized > deadband) {
        (true, true) => (normalized - deadband) / (length - deadband),
        (true, false) => 0.0,
        (false, _) => normalized / length,
    };
    let corrected = basis * percent * if calibration.reversed { -1.0 } else { 1.0 };
    corrected.clamp(-1.0, 1.0)
}

pub fn adjust_range_to_pwm(value: i32, calibration: &AxisCalibration, with_deadband: bool) -> Option<u16> {
    calibration.valid().then(|| {
        let normalized = adjust_range(value, calibration, with_deadband);
        let pwm = match calibration.one_sided() {
            true => PWM_MIN + normalized.abs().clamp(0.0, 1.0) * PWM_ONE_SIDED_SPAN,
            false => PWM_CENTER + normalized.clamp(-1.0, 1.0) * PWM_HALF_SPAN,
        };
        pwm.clamp(PWM_MIN, PWM_MAX).round() as u16
    })
}

pub fn circle_correct(value: f32) -> f32 {
    value.clamp(-CIRCLE_CORRECTION_LIMIT, CIRCLE_CORRECTION_LIMIT).asin().tan().clamp(-1.0, 1.0)
}

pub fn exponential(value: f32, percent: f64) -> f32 {
    if percent <= 0.0 {
        return value;
    }
    let factor = -(percent / 100.0) as f32;
    -factor * value.powi(3) + (1.0 + factor) * value
}

#[derive(Debug, Clone, PartialEq)]
struct Sample {
    axes: Vec<Option<i32>>,
    at_ms: u64,
}

#[derive(Debug)]
pub struct Joystick {
    axis_count: usize,
    button_count: usize,
    hat_count: usize,
    calibration: Vec<AxisCalibration>,
    axis_for: [Option<usize>; FUNCTIONS.len()],
    bindings: Vec<Option<Binding>>,
    events: Vec<ButtonEvent>,
    repeat_fired_ms: Vec<Option<u64>>,
    polling: Polling,
    sample: Option<Sample>,
    axis_sent_ms: Option<u64>,
    throttle_accumulator: f32,
    transmitter_mode: u8,
    rc_override_held: [bool; RC_OVERRIDE_CHANNELS],
    release_pending: u8,
    release_frame: [u16; RC_OVERRIDE_CHANNELS],
    shaped: Option<[f32; 4]>,
}

impl Joystick {
    pub fn new(axis_count: usize, button_count: usize, hat_count: usize) -> Joystick {
        let total = button_count + hat_count * HAT_BUTTONS_PER_HAT;
        Joystick {
            axis_count,
            button_count,
            hat_count,
            calibration: vec![AxisCalibration::default(); axis_count],
            axis_for: [None; FUNCTIONS.len()],
            bindings: vec![None; total],
            events: vec![ButtonEvent::None; total],
            repeat_fired_ms: vec![None; total],
            polling: Polling::default(),
            sample: None,
            axis_sent_ms: None,
            throttle_accumulator: 0.0,
            transmitter_mode: TRANSMITTER_MODE_STORED,
            rc_override_held: [false; RC_OVERRIDE_CHANNELS],
            release_pending: 0,
            release_frame: RC_RELEASE,
            shaped: None,
        }
    }

    pub fn axis_count(&self) -> usize {
        self.axis_count
    }

    pub fn total_button_count(&self) -> usize {
        self.bindings.len()
    }

    pub fn axis_for(&self, function: Function) -> Option<usize> {
        self.axis_for[function.index()]
    }

    pub fn function_for(&self, axis: usize) -> Option<Function> {
        FUNCTIONS.iter().map(|(f, _)| *f).find(|f| self.axis_for(*f) == Some(axis))
    }

    pub fn set_axis_function(&mut self, function: Function, axis: usize) -> bool {
        let taken = FUNCTIONS.iter().any(|(f, _)| *f != function && self.axis_for(*f) == Some(axis));
        if axis >= self.axis_count || taken {
            return false;
        }
        self.axis_for[function.index()] = Some(axis);
        true
    }

    pub fn clear_axis_function(&mut self, function: Function) {
        self.axis_for[function.index()] = None;
    }

    pub fn calibration(&self, axis: usize) -> Option<AxisCalibration> {
        self.calibration.get(axis).copied()
    }

    pub fn set_calibration(&mut self, axis: usize, calibration: AxisCalibration) -> bool {
        match (calibration.valid(), self.calibration.get_mut(axis)) {
            (true, Some(slot)) => {
                *slot = calibration;
                true
            }
            _ => false,
        }
    }

    pub fn reset_calibration(&mut self) {
        self.calibration = vec![AxisCalibration::default(); self.axis_count];
        self.axis_for = [None; FUNCTIONS.len()];
    }

    pub fn binding(&self, button: usize) -> Option<&Binding> {
        self.bindings.get(button).and_then(Option::as_ref)
    }

    pub fn set_button_action(&mut self, button: usize, action: Option<&str>) -> bool {
        if button >= self.bindings.len() {
            return false;
        }
        self.bindings[button] = action
            .map(str::trim)
            .filter(|name| !name.is_empty() && *name != ACTION_NONE)
            .map(|name| Binding { action: name.to_string(), repeat: false });
        self.repeat_fired_ms[button] = None;
        true
    }

    pub fn set_button_repeat(&mut self, button: usize, repeat: bool, capable: bool) -> bool {
        let Some(Some(binding)) = self.bindings.get_mut(button) else { return false };
        binding.repeat = repeat && (capable || can_repeat(&binding.action));
        self.repeat_fired_ms[button] = None;
        true
    }

    pub fn transmitter_mode(&self) -> u8 {
        self.transmitter_mode
    }

    pub fn set_transmitter_mode(&mut self, mode: u8) -> bool {
        if !(1..=4).contains(&mode) {
            return false;
        }
        let before = self.axis_for;
        self.axis_for = TRANSMITTER_MODES[self.transmitter_mode as usize - 1]
            .iter()
            .zip(TRANSMITTER_MODES[mode as usize - 1])
            .fold(before, |mut map, (from_function, to_function)| {
                map[to_function.index()] = before[from_function.index()];
                map
            });
        self.transmitter_mode = mode;
        true
    }

    pub fn missing_functions(&self, settings: &Settings) -> Vec<Function> {
        settings.wanted().into_iter().filter(|f| self.axis_for(*f).is_none()).collect()
    }

    pub fn invalid_axes(&self) -> Vec<usize> {
        (0..self.axis_count).filter(|axis| !self.calibration[*axis].valid()).collect()
    }

    pub fn validate(&mut self, settings: &Settings) -> Vec<Out> {
        if !settings.calibrated {
            self.reset_calibration();
            return Vec::new();
        }
        let missing = self.missing_functions(settings);
        if missing.is_empty() && self.invalid_axes().is_empty() {
            return Vec::new();
        }
        self.reset_calibration();
        vec![Out::RequireCalibration { missing }]
    }

    pub fn polling(&self) -> Polling {
        self.polling
    }

    pub fn rc_override_active(&self) -> bool {
        self.rc_override_held.iter().any(|channel| *channel)
    }

    fn begin_release(&mut self) {
        let held = std::mem::replace(&mut self.rc_override_held, [false; RC_OVERRIDE_CHANNELS]);
        if held.iter().any(|channel| *channel) {
            self.release_frame = std::array::from_fn(|channel| match held[channel] {
                true => RC_RELEASE[channel],
                false => RC_IGNORE[channel],
            });
            self.release_pending = RC_OVERRIDE_RELEASE_COUNT;
        }
    }

    fn drain_release(&mut self, frames: u8) -> Vec<Out> {
        let sending = self.release_pending.min(frames);
        self.release_pending -= sending;
        (0..sending).map(|_| Out::RcChannelsOverride { channels: self.release_frame }).collect()
    }

    fn release(&mut self, frames: u8) -> Vec<Out> {
        self.begin_release();
        self.drain_release(frames)
    }

    pub fn set_polling(&mut self, polling: Polling, now_ms: u64) -> Vec<Out> {
        if polling == self.polling {
            return Vec::new();
        }
        self.polling = polling;
        self.sample = None;
        self.shaped = None;
        self.throttle_accumulator = 0.0;
        self.events = vec![ButtonEvent::None; self.bindings.len()];
        self.repeat_fired_ms = vec![None; self.bindings.len()];
        self.axis_sent_ms = (!polling.idle()).then_some(now_ms);
        match polling.commands() {
            true => Vec::new(),
            false => self.release(RC_OVERRIDE_RELEASE_COUNT),
        }
    }

    pub fn stop(&mut self, now_ms: u64) -> Vec<Out> {
        self.set_polling(Polling::default(), now_ms)
    }

    fn pressed(&self, input: &Input) -> Vec<bool> {
        let buttons = (0..self.button_count).map(|i| input.buttons.get(i).copied().unwrap_or(false));
        let hats = (0..self.hat_count).flat_map(|hat| {
            let bits = input.hats.get(hat).copied().unwrap_or(0);
            HAT_DIRECTIONS.map(|direction| bits & direction != 0)
        });
        buttons.chain(hats).collect()
    }

    pub fn on_input(&mut self, input: Input<'_>, settings: &Settings, support: Support, now_ms: u64) -> Vec<Out> {
        if self.polling.idle() {
            return Vec::new();
        }
        self.sample = Some(Sample { axes: (0..self.axis_count).map(|axis| input.axes.get(axis).copied()).collect(), at_ms: now_ms });
        let pressed = self.pressed(&input);
        let before = std::mem::take(&mut self.events);
        self.events = before.iter().zip(pressed).map(|(event, down)| event.step(down)).collect();
        let buttons = self.button_out(settings, now_ms);
        let axes = match self.axis_count {
            0 => Vec::new(),
            _ => self.axis_out(settings, support, now_ms),
        };
        buttons.into_iter().chain(axes).collect()
    }

    fn button_out(&mut self, settings: &Settings, now_ms: u64) -> Vec<Out> {
        if self.polling.configuration {
            return self
                .events
                .iter()
                .enumerate()
                .filter_map(|(index, event)| match event {
                    ButtonEvent::Down => Some(Out::RawButton { index, pressed: true }),
                    ButtonEvent::Up => Some(Out::RawButton { index, pressed: false }),
                    _ => None,
                })
                .collect();
        }
        if !self.polling.vehicle || !settings.calibrated {
            return Vec::new();
        }
        let delay = settings.button_interval_ms();
        let fired = (0..self.bindings.len()).fold(Vec::new(), |fired: Vec<(usize, String, ButtonEvent)>, button| {
            let Some(binding) = self.bindings[button].clone() else { return fired };
            let event = self.events[button];
            let already = fired.iter().any(|(_, action, _)| *action == binding.action);
            let due = self.repeat_fired_ms[button].is_none_or(|last| now_ms.saturating_sub(last) > delay);
            match (event, binding.repeat) {
                (ButtonEvent::Down | ButtonEvent::Repeat, true) if due => fired.into_iter().chain([(button, binding.action, ButtonEvent::Repeat)]).collect(),
                (ButtonEvent::Down, false) if !already && self.all_sharing_held(button, &binding.action) => fired.into_iter().chain([(button, binding.action, ButtonEvent::Down)]).collect(),
                (ButtonEvent::Up, _) if !already => fired.into_iter().chain([(button, binding.action, ButtonEvent::Up)]).collect(),
                _ => fired,
            }
        });
        fired
            .into_iter()
            .filter_map(|(button, action, event)| {
                if event == ButtonEvent::Repeat {
                    self.repeat_fired_ms[button] = Some(now_ms);
                }
                accepts(&action, event).then_some(Out::Action { action, event })
            })
            .collect()
    }

    fn all_sharing_held(&self, button: usize, action: &str) -> bool {
        self.bindings
            .iter()
            .enumerate()
            .filter(|(index, binding)| *index != button && binding.as_ref().is_some_and(|b| b.action == action))
            .all(|(index, _)| self.events[index].held())
    }

    fn raw(&self, function: Function) -> Option<Option<(usize, i32)>> {
        let axis = self.axis_for(function)?;
        let value = self.sample.as_ref().and_then(|sample| sample.axes.get(axis).copied().flatten());
        Some(value.map(|value| (axis, value)))
    }

    fn shape(&self, function: Function, with_deadband: bool) -> Option<f32> {
        let (axis, value) = self.raw(function)??;
        Some(adjust_range(value, &self.calibration[axis], with_deadband))
    }

    fn blocking_reason(&self, settings: &Settings) -> Option<&'static str> {
        [
            (!self.polling.vehicle, BLOCKED_NOT_POLLING),
            (self.polling.configuration, BLOCKED_CONFIGURING),
            (!settings.calibrated, BLOCKED_NOT_CALIBRATED),
            (!self.missing_functions(settings).is_empty(), BLOCKED_MAPPING_INCOMPLETE),
            (!self.invalid_axes().is_empty(), BLOCKED_INVALID_CALIBRATION),
            (self.sample.is_none(), BLOCKED_NO_SAMPLE),
            (settings.wanted().iter().any(|f| self.raw(*f).flatten().is_none()), BLOCKED_MISSING_AXIS_DATA),
        ]
        .into_iter()
        .find_map(|(blocked, reason)| blocked.then_some(reason))
    }

    fn axis_out(&mut self, settings: &Settings, support: Support, now_ms: u64) -> Vec<Out> {
        let due = self.axis_sent_ms.is_none_or(|sent| now_ms.saturating_sub(sent) > settings.axis_interval_ms());
        if !due {
            return Vec::new();
        }
        self.axis_sent_ms = Some(now_ms);
        if self.polling.configuration {
            let values = self.sample.as_ref().map(|sample| sample.axes.clone()).unwrap_or_else(|| vec![None; self.axis_count]);
            return [Out::RawAxes { values }].into_iter().chain(self.release(1)).collect();
        }
        if !self.polling.vehicle || !settings.calibrated {
            return self.release(1);
        }
        let wanted = settings.wanted();
        let missing: Vec<Function> = wanted.iter().copied().filter(|f| self.axis_for(*f).is_none()).collect();
        if !missing.is_empty() {
            return [Out::MappingIncomplete { missing }].into_iter().chain(self.release(1)).collect();
        }
        let broken = self.invalid_axes();
        if !broken.is_empty() {
            return [Out::RequireCalibration { missing: broken.iter().filter_map(|axis| self.function_for(*axis)).collect() }].into_iter().chain(self.release(1)).collect();
        }
        let blind: Vec<usize> = wanted.iter().filter_map(|f| self.raw(*f).flatten().is_none().then(|| self.axis_for(*f)).flatten()).collect();
        if !blind.is_empty() {
            return [Out::MissingAxisData { axes: blind }].into_iter().chain(self.release(1)).collect();
        }
        let deadband = settings.use_deadband;
        let centre_zero = settings.centre_zero(support);
        let attitude = [
            self.shape(Function::Roll, deadband),
            self.shape(Function::Pitch, deadband),
            self.shape(Function::Yaw, deadband),
            self.shape(Function::Throttle, centre_zero && deadband),
        ];
        let [Some(roll), Some(pitch), Some(yaw), Some(raw_throttle)] = attitude else { return self.release(1) };
        let throttle = match settings.smoothing_active(support) {
            true => {
                self.throttle_accumulator = (self.throttle_accumulator + raw_throttle * THROTTLE_SMOOTHING_STEP).clamp(-1.0, 1.0);
                self.throttle_accumulator
            }
            false => {
                self.throttle_accumulator = 0.0;
                raw_throttle
            }
        };
        let circled = match settings.circle_correction {
            true => [circle_correct(roll), circle_correct(pitch), circle_correct(yaw), circle_correct(throttle)],
            false => [roll, pitch, yaw, throttle],
        };
        let [roll, pitch, yaw, throttle] = circled;
        let roll = exponential(roll, settings.exponential_pct);
        let pitch = exponential(pitch, settings.exponential_pct);
        let yaw = exponential(yaw, settings.exponential_pct);
        let throttle = match centre_zero {
            true if settings.negative_thrust_active(support) => throttle,
            true => throttle.max(0.0),
            false => (throttle + 1.0) / 2.0,
        };
        self.shaped = Some([roll, pitch, yaw, throttle]);

        let manual = settings.additional_axes == AdditionalAxes::ManualControl;
        let extensions: [Option<f32>; MANUAL_CONTROL_EXTENSIONS] = std::array::from_fn(|bit| {
            let function = OPTIONAL[bit];
            let sent_as_manual_control = bit < MANUAL_CONTROL_ONLY_EXTENSIONS || manual;
            (settings.enabled(function) && sent_as_manual_control).then(|| self.shape(function, deadband)).flatten()
        });
        let channels: [Option<u16>; RC_OVERRIDE_CHANNELS] = std::array::from_fn(|channel| {
            let function = OPTIONAL[channel + MANUAL_CONTROL_ONLY_EXTENSIONS];
            (!manual && settings.enabled(function))
                .then(|| self.raw(function).flatten().and_then(|(axis, value)| adjust_range_to_pwm(value, &self.calibration[axis], deadband)))
                .flatten()
        });

        let held = self.events.iter().enumerate().filter(|(index, event)| *index < VEHICLE_BUTTON_BITS && event.held()).fold(0u32, |bits, (index, _)| bits | (1u32 << index));
        let scaled = |value: f32| (value * AXES_SCALING) as i16;
        let control = Out::ManualControl {
            x: scaled(pitch),
            y: scaled(roll),
            z: scaled(throttle),
            r: scaled(yaw),
            buttons: (held & 0xFFFF) as u16,
            buttons2: (held >> 16) as u16,
            enabled_extensions: extensions.iter().enumerate().filter(|(_, value)| value.is_some()).fold(0u8, |bits, (bit, _)| bits | (1u8 << bit)),
            extensions: std::array::from_fn(|bit| extensions[bit].map(scaled).unwrap_or(0)),
        };
        let override_out = match channels.iter().any(Option::is_some) {
            true => {
                let frame = std::array::from_fn(|channel| {
                    channels[channel].unwrap_or(match self.rc_override_held[channel] {
                        true => RC_RELEASE[channel],
                        false => RC_IGNORE[channel],
                    })
                });
                self.rc_override_held = std::array::from_fn(|channel| channels[channel].is_some());
                self.release_pending = 0;
                vec![Out::RcChannelsOverride { channels: frame }]
            }
            false => self.release(1),
        };
        [Out::Axes { roll, pitch, yaw, throttle }, control].into_iter().chain(override_out).collect()
    }

    pub fn snapshot(&self, settings: &Settings, support: Support, now_ms: u64) -> Value {
        let age = self.sample.as_ref().map(|sample| now_ms.saturating_sub(sample.at_ms));
        let stale = age.map(|age| age > settings.axis_interval_ms());
        let deadband = settings.use_deadband;
        let reason = self.blocking_reason(settings);
        json!({
            "kind": "object",
            "class": "JoystickState",
            "axisCount": self.axis_count,
            "buttonCount": self.button_count,
            "hatCount": self.hat_count,
            "totalButtonCount": self.total_button_count(),
            "vehicleButtonBits": VEHICLE_BUTTON_BITS,
            "polling": { "vehicle": self.polling.vehicle, "configuration": self.polling.configuration },
            "calibrated": settings.calibrated,
            "commanding": reason.is_none(),
            "notCommandingReason": reason,
            "transmitterMode": self.transmitter_mode,
            "support": {
                "throttleModeCenterZero": support.throttle_mode_center_zero,
                "negativeThrust": support.negative_thrust,
                "defaultTransmitterMode": support.default_transmitter_mode,
            },
            "requestedThrottleMode": settings.throttle_mode.id(),
            "effectiveThrottleMode": settings.effective_throttle_mode(support).id(),
            "negativeThrustActive": settings.negative_thrust_active(support),
            "throttleSmoothingActive": settings.smoothing_active(support),
            "missingFunctions": self.missing_functions(settings).iter().map(|f| f.id()).collect::<Vec<_>>(),
            "invalidAxes": self.invalid_axes(),
            "sampleAgeMs": age,
            "stale": stale,
            "rcOverrideActive": self.rc_override_active(),
            "rcReleasePending": self.release_pending,
            "rates": {
                "axisHz": settings.axis_frequency_hz,
                "buttonHz": settings.button_frequency_hz,
                "axisIntervalMs": settings.axis_interval_ms(),
                "buttonIntervalMs": settings.button_interval_ms(),
                "pollIntervalMs": poll_interval_ms(),
            },
            "shaped": (stale == Some(false)).then_some(self.shaped).flatten().map(|[roll, pitch, yaw, throttle]| json!({ "roll": roll, "pitch": pitch, "yaw": yaw, "throttle": throttle })),
            "functions": FUNCTIONS.iter().map(|(function, id)| json!({ "id": id, "axis": self.axis_for(*function) })).collect::<Vec<_>>(),
            "axes": (0..self.axis_count).map(|axis| {
                let calibration = self.calibration[axis];
                let raw = self.sample.as_ref().and_then(|sample| sample.axes.get(axis).copied().flatten());
                json!({
                    "index": axis,
                    "raw": raw,
                    "function": self.function_for(axis).map(Function::id),
                    "shaped": raw.map(|value| adjust_range(value, &calibration, deadband)),
                    "valid": calibration.valid(),
                    "invalidReason": calibration.fault(),
                    "calibration": { "min": calibration.min, "max": calibration.max, "center": calibration.center, "deadband": calibration.deadband, "reversed": calibration.reversed },
                })
            }).collect::<Vec<_>>(),
            "buttons": (0..self.total_button_count()).map(|button| {
                let hat = button.checked_sub(self.button_count);
                json!({
                    "index": button,
                    "hatIndex": hat.map(|index| index / HAT_BUTTONS_PER_HAT),
                    "direction": hat.map(|index| HAT_DIRECTION_IDS[index % HAT_BUTTONS_PER_HAT]),
                    "reachesVehicle": button < VEHICLE_BUTTON_BITS,
                    "action": self.binding(button).map(|b| b.action.clone()),
                    "actionId": self.binding(button).and_then(|b| action_id(&b.action)),
                    "repeat": self.binding(button).is_some_and(|b| b.repeat),
                    "event": self.events[button].id(),
                })
            }).collect::<Vec<_>>(),
            "diagnostics": { "throttleAccumulator": self.throttle_accumulator },
        })
    }
}

fn setting_meta(name: &str, value_type: &str, default: Value, units: Option<&str>, bounds: Option<(f64, f64)>, enums: &[&str], default_from: Option<&str>) -> Value {
    json!({
        "name": name,
        "type": value_type,
        "default": default,
        "defaultFrom": default_from,
        "units": units,
        "min": bounds.map(|(low, _)| low),
        "max": bounds.map(|(_, high)| high),
        "enumValues": enums,
    })
}

fn settings_catalog() -> Vec<Value> {
    [
        setting_meta("calibrated", "bool", json!(false), None, None, &[], None),
        setting_meta("circleCorrection", "bool", json!(false), None, None, &[], None),
        setting_meta("useDeadband", "bool", json!(false), None, None, &[], None),
        setting_meta("negativeThrust", "bool", json!(false), None, None, &[], None),
        setting_meta("throttleSmoothing", "bool", json!(false), None, None, &[], None),
        setting_meta("throttleMode", "uint32", json!(ThrottleMode::default().stored()), None, None, &[ThrottleMode::CenterZero.id(), ThrottleMode::DownZero.id()], None),
        setting_meta("axisFrequencyHz", "double", json!(AXIS_FREQUENCY_DEFAULT_HZ), Some("Hz"), Some((AXIS_FREQUENCY_MIN_HZ, AXIS_FREQUENCY_MAX_HZ)), &[], None),
        setting_meta("buttonFrequencyHz", "double", json!(BUTTON_FREQUENCY_DEFAULT_HZ), Some("Hz"), Some((BUTTON_FREQUENCY_MIN_HZ, BUTTON_FREQUENCY_MAX_HZ)), &[], None),
        setting_meta("transmitterMode", "uint32", Value::Null, None, Some((1.0, 4.0)), &[], Some("support.defaultTransmitterMode")),
        setting_meta("exponentialPct", "double", json!(EXPONENTIAL_PCT_DEFAULT), Some("%"), Some((EXPONENTIAL_PCT_MIN, EXPONENTIAL_PCT_MAX)), &[], None),
        setting_meta("additionalAxesFunction", "uint32", json!(AdditionalAxes::default().stored()), None, None, &[AdditionalAxes::ManualControl.id(), AdditionalAxes::RcChannelsOverride.id()], None),
    ]
    .into_iter()
    .chain(OPTIONAL.iter().map(|function| setting_meta(&format!("enable_{}", function.id()), "bool", json!(false), None, None, &[], None)))
    .collect()
}

pub fn joystick_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    json!({
        "kind": "object",
        "class": "JoystickMapping",
        "axisRange": { "min": AXIS_MIN, "max": AXIS_MAX },
        "axesScaling": AXES_SCALING,
        "hatButtonsPerHat": HAT_BUTTONS_PER_HAT,
        "hatDirections": HAT_DIRECTION_IDS,
        "vehicleButtonBits": VEHICLE_BUTTON_BITS,
        "throttleSmoothingStep": THROTTLE_SMOOTHING_STEP,
        "rcOverride": {
            "channels": RC_OVERRIDE_CHANNELS,
            "firstChannel": RC_OVERRIDE_FIRST_CHANNEL,
            "pwmMin": PWM_MIN,
            "pwmCenter": PWM_CENTER,
            "pwmMax": PWM_MAX,
            "release": RC_RELEASE,
            "ignore": RC_IGNORE,
            "releaseFrames": RC_OVERRIDE_RELEASE_COUNT,
        },
        "functions": FUNCTIONS.iter().map(|(function, id)| json!({
            "id": id,
            "required": ATTITUDE.contains(function),
            "extensionBit": function.extension_bit(),
            "rcChannel": function.rc_channel(),
            "carriedBy": function.carried_by(),
        })).collect::<Vec<_>>(),
        "transmitterModes": TRANSMITTER_MODES.iter().enumerate().map(|(index, functions)| json!({
            "mode": index + 1,
            "storedAs": TRANSMITTER_MODE_STORED,
            "functions": functions.iter().map(|f| f.id()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "actions": std::iter::once(json!({ "id": ACTION_NONE_ID, "action": ACTION_NONE, "down": false, "up": false, "repeat": false }))
            .chain(ACTIONS.iter().map(|(id, action, up, repeat)| json!({ "id": id, "action": action, "down": true, "up": up, "repeat": repeat })))
            .collect::<Vec<_>>(),
        "settings": settings_catalog(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn centered() -> AxisCalibration {
        AxisCalibration { min: -32768, max: 32767, center: 0, deadband: 0, reversed: false }
    }

    fn flying() -> Settings {
        Settings { calibrated: true, use_deadband: false, ..Settings::default() }
    }

    fn centre_zero_flying() -> Settings {
        Settings { throttle_mode: ThrottleMode::CenterZero, ..flying() }
    }

    fn down_zero_firmware() -> Support {
        Support { throttle_mode_center_zero: false, ..Support::default() }
    }

    fn mapped() -> Joystick {
        let mut stick = Joystick::new(6, 4, 1);
        ATTITUDE.iter().enumerate().for_each(|(axis, function)| {
            stick.set_axis_function(*function, axis);
            stick.set_calibration(axis, centered());
        });
        stick
    }

    fn axes_of(out: &[Out]) -> Option<(f32, f32, f32, f32)> {
        out.iter().find_map(|o| match o {
            Out::Axes { roll, pitch, yaw, throttle } => Some((*roll, *pitch, *yaw, *throttle)),
            _ => None,
        })
    }

    fn control_of(out: &[Out]) -> Option<&Out> {
        out.iter().find(|o| matches!(o, Out::ManualControl { .. }))
    }

    fn overrides_of(out: &[Out]) -> Vec<[u16; RC_OVERRIDE_CHANNELS]> {
        out.iter()
            .filter_map(|o| match o {
                Out::RcChannelsOverride { channels } => Some(*channels),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_raw_axis_becomes_minus_one_to_one_around_the_calibrated_centre() {
        let calibration = AxisCalibration { min: -10000, max: 20000, center: 0, deadband: 2000, reversed: false };
        assert_eq!(adjust_range(20000, &calibration, false), 1.0);
        assert_eq!(adjust_range(-10000, &calibration, false), -1.0);
        assert_eq!(adjust_range(0, &calibration, false), 0.0);
        assert_eq!(adjust_range(10000, &calibration, false), 0.5, "above centre the span is centre..max, not min..max");
        assert_eq!(adjust_range(-5000, &calibration, false), -0.5, "below centre the span is centre..min, so an asymmetric axis stays asymmetric");
        assert_eq!(adjust_range(1999, &calibration, true), 0.0, "inside the deadband the answer is a real zero, not a small number");
        assert_eq!(adjust_range(11000, &calibration, true), 0.5, "outside the deadband the remaining travel is rescaled to the full range");
        assert_eq!(adjust_range(30000, &calibration, false), 1.0, "a value past max clamps instead of overshooting the protocol range");
        assert_eq!(adjust_range(20000, &AxisCalibration { reversed: true, ..calibration }, false), -1.0);
        let dead = AxisCalibration { min: 0, max: 0, center: 0, deadband: 0, reversed: false };
        assert_eq!(adjust_range(500, &dead, false), 0.0, "an axis with no travel answers zero rather than dividing by zero");
        let one_sided = AxisCalibration { min: 0, max: 32767, center: 0, deadband: 0, reversed: false };
        assert_eq!(adjust_range(16384, &one_sided, false), 0.50001526);
        assert_eq!(adjust_range(-500, &one_sided, false), 0.0, "a trigger pinned at its centre never reports negative travel");
    }

    #[test]
    fn a_deadband_wider_than_the_travel_reads_zero_instead_of_inverting_the_stick() {
        let swallowed = AxisCalibration { min: -10000, max: 10000, center: 0, deadband: 20000, reversed: false };
        assert_eq!(adjust_range(25000, &swallowed, true), 0.0, "a deadband past the end of travel must not divide by a negative span and hand back stick-left for stick-right");
        assert_eq!(adjust_range(25000, &swallowed, false), 0.0, "and it stays zero with the deadband switched off, because the stored numbers are still self-contradictory");
        let clipping = AxisCalibration { deadband: 12000, ..swallowed };
        assert_eq!(adjust_range(9000, &clipping, true), 0.0);
        assert_eq!(adjust_range(-9000, &clipping, true), 0.0);
        assert_eq!(adjust_range_to_pwm(9000, &clipping, true), None, "an axis nobody can trust drives no rc channel at all");
        assert_eq!(adjust_range_to_pwm(0, &AxisCalibration { min: 0, max: 0, center: 0, deadband: 0, reversed: false }, false), None, "a zero-travel axis releases its channel rather than commanding the bottom of the pwm range");
    }

    #[test]
    fn a_self_contradictory_calibration_is_refused_and_named_in_the_state() {
        let mut stick = Joystick::new(2, 1, 0);
        assert!(stick.set_calibration(0, centered()));
        assert!(!stick.set_calibration(0, AxisCalibration { min: 10, max: 10, center: 10, deadband: 0, reversed: false }), "min must sit below max");
        assert!(!stick.set_calibration(0, AxisCalibration { min: -100, max: 100, center: 500, deadband: 0, reversed: false }), "centre outside the travel would read a hard deflection at rest");
        assert!(!stick.set_calibration(0, AxisCalibration { min: -100, max: 100, center: 0, deadband: -5, reversed: false }));
        assert!(!stick.set_calibration(0, AxisCalibration { min: -100, max: 100, center: 0, deadband: 100, reversed: false }), "a deadband as wide as the travel leaves no travel");
        assert_eq!(stick.calibration(0), Some(centered()), "a refused calibration leaves the last good one in place");
        assert_eq!(AxisCalibration { min: 0, max: 100, center: 0, deadband: 50, reversed: false }.fault(), None, "a one-sided axis is judged on the side it actually travels");

        let settings = flying();
        let fresh = Joystick::new(1, 1, 0);
        assert_eq!(fresh.snapshot(&settings, Support::default(), 0)["axes"][0]["valid"], true);
        assert_eq!(fresh.snapshot(&settings, Support::default(), 0)["axes"][0]["invalidReason"], Value::Null);
        let mut broken = Joystick::new(1, 1, 0);
        broken.calibration[0] = AxisCalibration { min: -100, max: 100, center: 0, deadband: 200, reversed: false };
        let view = broken.snapshot(&settings, Support::default(), 0);
        assert_eq!(view["axes"][0]["valid"], false, "the calibration display has to be able to mark the axis instead of drawing the bad numbers as if they were fine");
        assert_eq!(view["axes"][0]["invalidReason"], CALIBRATION_DEADBAND_EXCEEDS_TRAVEL);
        assert_eq!(view["invalidAxes"], json!([0]));

        let mut flying_on_bad_numbers = mapped();
        flying_on_bad_numbers.calibration[3] = AxisCalibration { min: -100, max: 100, center: 0, deadband: 200, reversed: false };
        flying_on_bad_numbers.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let out = flying_on_bad_numbers.on_input(Input { axes: &[0, 0, 0, 0], buttons: &[false; 4], hats: &[0] }, &settings, Support::default(), 41);
        assert_eq!(out, vec![Out::RequireCalibration { missing: vec![Function::Throttle] }], "a range nobody can trust commands nothing and names the function it belongs to");
        assert_eq!(flying_on_bad_numbers.snapshot(&settings, Support::default(), 41)["notCommandingReason"], BLOCKED_INVALID_CALIBRATION);
    }

    #[test]
    fn an_rc_override_axis_maps_to_pwm_over_the_span_the_rest_of_the_app_allows() {
        let two_sided = centered();
        assert_eq!(adjust_range_to_pwm(0, &two_sided, false), Some(1500));
        assert_eq!(adjust_range_to_pwm(32767, &two_sided, false), Some(2200), "the vehicle bounds overrides to 800..2200, so a joystick that stops at 2000 silently clips a payload");
        assert_eq!(adjust_range_to_pwm(-32768, &two_sided, false), Some(800));
        let trigger = AxisCalibration { min: 0, max: 32767, center: 0, deadband: 0, reversed: false };
        assert_eq!(adjust_range_to_pwm(0, &trigger, false), Some(800), "a one-sided axis starts at the bottom of the pwm range, not the middle");
        assert_eq!(adjust_range_to_pwm(32767, &trigger, false), Some(2200));
        assert_eq!(adjust_range_to_pwm(16384, &trigger, false), Some(1500));
        assert_eq!(RC_RELEASE, [0, 0, 0, 0, u16::MAX - 1, u16::MAX - 1], "channels 5-8 release with 0 and the two extension channels with UINT16_MAX-1");
        assert_eq!(RC_IGNORE, [u16::MAX, u16::MAX, u16::MAX, u16::MAX, 0, 0], "leaving a channel alone is UINT16_MAX below channel 9 and 0 above it, which is the opposite encoding");
    }

    #[test]
    fn expo_and_circle_correction_shape_the_sticks_but_leave_the_ends_alone() {
        assert_eq!(exponential(0.5, 0.0), 0.5, "zero percent is the identity curve, not a no-op guard to forget");
        assert_eq!(exponential(1.0, 50.0), 1.0);
        assert_eq!(exponential(-1.0, 50.0), -1.0);
        assert_eq!(exponential(0.5, 50.0), 0.3125, "half expo softens the middle of the travel");
        assert!(exponential(0.5, 50.0) < 0.5 && exponential(0.9, 50.0) < 0.9);
        assert_eq!(circle_correct(0.0), 0.0);
        assert!(circle_correct(0.5) > 0.5, "the unit circle maps to a linear range, so mid travel grows");
        assert_eq!(circle_correct(1.0), 1.0, "past the quarter-turn limit the answer clamps instead of running to infinity");
        assert_eq!(circle_correct(-1.0), -1.0);
    }

    #[test]
    fn manual_control_carries_the_shaped_axes_scaled_by_a_thousand() {
        let mut stick = mapped();
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let settings = flying();
        let out = stick.on_input(Input { axes: &[16384, -16384, 32767, 0], buttons: &[false; 4], hats: &[0] }, &settings, Support::default(), 41);
        let (roll, pitch, yaw, throttle) = axes_of(&out).expect("a mapped calibrated stick reports its axes");
        assert!((roll - 0.5).abs() < 0.001 && (pitch + 0.5).abs() < 0.001 && (yaw - 1.0).abs() < 0.001);
        assert_eq!(throttle, 0.5, "with down-zero throttle the stick centre is half thrust");
        match control_of(&out).unwrap() {
            Out::ManualControl { x, y, z, r, buttons, buttons2, enabled_extensions, .. } => {
                assert_eq!((*x, *y, *z, *r), (-500, 500, 500, 1000), "x is pitch, y is roll, z is thrust, r is yaw");
                assert_eq!((*buttons, *buttons2, *enabled_extensions), (0, 0, 0), "no button held and no extension enabled is three zeroes, not an absent field");
            }
            other => panic!("expected a manual control message, got {other:?}"),
        }
    }

    #[test]
    fn the_axis_rate_comes_from_the_settings_and_nothing_is_sent_early() {
        let settings = Settings { axis_frequency_hz: 25.0, button_frequency_hz: 5.0, ..flying() };
        assert_eq!((settings.axis_interval_ms(), settings.button_interval_ms()), (40, 200));
        assert_eq!(poll_interval_ms(), 2, "the poll loop runs off the maximum rates, so a 25Hz axis setting still samples button edges every 2ms");
        let fast = Settings { axis_frequency_hz: 200.0, button_frequency_hz: 50.0, ..settings };
        assert_eq!(fast.axis_interval_ms(), 5);
        let nonsense = Settings { axis_frequency_hz: 0.0, button_frequency_hz: -1.0, ..settings };
        assert_eq!(nonsense.axis_interval_ms(), 4000, "a zero rate clamps to the documented minimum instead of dividing by zero");
        let broken = Settings { axis_frequency_hz: f64::NAN, ..settings };
        assert_eq!(broken.axis_interval_ms(), 40, "a NaN out of a stored setting falls back to the default rate rather than telling the head to spin");
        let mut stick = mapped();
        stick.set_polling(Polling { vehicle: true, configuration: false }, 1000);
        let input = Input { axes: &[0, 0, 0, 0], buttons: &[false; 4], hats: &[0] };
        assert!(control_of(&stick.on_input(input, &settings, Support::default(), 1040)).is_none(), "the period has to be past, not merely reached, exactly as the polled version did");
        assert!(control_of(&stick.on_input(input, &settings, Support::default(), 1041)).is_some());
        assert!(control_of(&stick.on_input(input, &settings, Support::default(), 1081)).is_none(), "the period restarts from the send, so the next one waits again");
        assert!(control_of(&stick.on_input(input, &settings, Support::default(), 1082)).is_some());
    }

    #[test]
    fn a_button_walks_down_repeat_up_none_and_only_fires_events_its_action_accepts() {
        let mut stick = mapped();
        stick.set_button_action(0, Some("Continuous Zoom In"));
        stick.set_button_action(1, Some("Arm"));
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let settings = flying();
        let down = stick.on_input(Input { axes: &[0, 0, 0, 0], buttons: &[true, true, false, false], hats: &[0] }, &settings, Support::default(), 0);
        let actions: Vec<&Out> = down.iter().filter(|o| matches!(o, Out::Action { .. })).collect();
        assert_eq!(actions.len(), 2, "both bound buttons fire once on the press");
        let held = stick.on_input(Input { axes: &[0, 0, 0, 0], buttons: &[true, true, false, false], hats: &[0] }, &settings, Support::default(), 1);
        assert!(!held.iter().any(|o| matches!(o, Out::Action { .. })), "holding a non-repeat action fires nothing more");
        let up = stick.on_input(Input { axes: &[0, 0, 0, 0], buttons: &[false, false, false, false], hats: &[0] }, &settings, Support::default(), 2);
        let released: Vec<&Out> = up.iter().filter(|o| matches!(o, Out::Action { .. })).collect();
        assert_eq!(released, vec![&Out::Action { action: "Continuous Zoom In".into(), event: ButtonEvent::Up }], "only an action with a release handler hears the release, so Arm never fires on let-go");
        assert!(!accepts("Arm", ButtonEvent::Up) && accepts("Gimbal Up", ButtonEvent::Up));
        assert!(accepts("Step Zoom In", ButtonEvent::Repeat) && !accepts("Arm", ButtonEvent::Repeat));
        assert!(accepts("Loiter", ButtonEvent::Down), "a flight mode the core does not know is still dispatched on the press");
        assert!(!accepts("Loiter", ButtonEvent::Up), "but an unknown action gets no release, because nothing would handle it");
        assert!(!accepts(ACTION_NONE, ButtonEvent::Down) && !accepts("", ButtonEvent::Down));
    }

    #[test]
    fn a_repeating_button_fires_on_the_button_rate_and_a_shared_action_fires_once() {
        let mut stick = mapped();
        stick.set_button_action(0, Some("Step Zoom In"));
        stick.set_button_repeat(0, true, false);
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let settings = Settings { button_frequency_hz: 5.0, ..flying() };
        let pressed = Input { axes: &[0, 0, 0, 0], buttons: &[true, false, false, false], hats: &[0] };
        let first = stick.on_input(pressed, &settings, Support::default(), 0);
        assert!(first.contains(&Out::Action { action: "Step Zoom In".into(), event: ButtonEvent::Repeat }), "a fresh binding has never fired, so the first press is not held back");
        assert!(!stick.on_input(pressed, &settings, Support::default(), 200).iter().any(|o| matches!(o, Out::Action { .. })), "200ms is not yet past a 200ms period");
        assert!(stick.on_input(pressed, &settings, Support::default(), 201).contains(&Out::Action { action: "Step Zoom In".into(), event: ButtonEvent::Repeat }));

        let mut shared = mapped();
        shared.set_button_action(0, Some("Emergency Stop"));
        shared.set_button_action(1, Some("Emergency Stop"));
        shared.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let one = shared.on_input(Input { axes: &[0, 0, 0, 0], buttons: &[true, false, false, false], hats: &[0] }, &settings, Support::default(), 0);
        assert!(!one.iter().any(|o| matches!(o, Out::Action { .. })), "an action bound to two buttons waits for both, which is the whole point of the two-button guard");
        let both = shared.on_input(Input { axes: &[0, 0, 0, 0], buttons: &[true, true, false, false], hats: &[0] }, &settings, Support::default(), 1);
        assert_eq!(both.iter().filter(|o| matches!(o, Out::Action { .. })).count(), 1, "and it fires once, not once per button");
        assert!(!shared.set_button_repeat(99, true, false), "an index past the last button is refused rather than panicking");
    }

    #[test]
    fn repeat_capability_is_the_heads_to_declare_because_the_core_holds_half_the_table() {
        assert!(can_repeat("Continuous Zoom In") && can_repeat("Continuous Zoom Out"), "the continuous zooms repeat upstream, so a stored repeat flag for them must survive a load");
        let mut stick = mapped();
        stick.set_button_action(0, Some("Continuous Zoom In"));
        assert!(stick.set_button_repeat(0, true, false));
        assert!(stick.binding(0).is_some_and(|b| b.repeat));
        stick.set_button_action(1, Some("Loiter"));
        assert!(stick.set_button_repeat(1, true, false));
        assert!(stick.binding(1).is_some_and(|b| !b.repeat), "an action the core does not know stays un-repeated while nobody vouches for it");
        assert!(stick.set_button_repeat(1, true, true));
        assert!(stick.binding(1).is_some_and(|b| b.repeat), "a plugin action the head declares repeat-capable can be made to repeat, which the core alone could never know");
        stick.set_button_action(2, Some("Emergency Stop"));
        assert!(stick.set_button_repeat(2, true, false));
        assert!(stick.binding(2).is_some_and(|b| !b.repeat), "and an action with no repeat handler stays un-repeated however hard the head asks");
    }

    #[test]
    fn buttons_held_ride_along_in_the_manual_control_bitmap_and_a_hat_is_four_buttons() {
        let mut stick = Joystick::new(4, 20, 1);
        ATTITUDE.iter().enumerate().for_each(|(axis, function)| {
            stick.set_axis_function(*function, axis);
            stick.set_calibration(axis, centered());
        });
        assert_eq!(stick.total_button_count(), 24, "each hat contributes four buttons after the real ones");
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let settings = flying();
        let mut buttons = [false; 20];
        buttons[0] = true;
        buttons[17] = true;
        let out = stick.on_input(Input { axes: &[0, 0, 0, 0], buttons: &buttons, hats: &[0x01] }, &settings, Support::default(), 41);
        match control_of(&out).unwrap() {
            Out::ManualControl { buttons, buttons2, .. } => {
                assert_eq!(*buttons, 1, "button 0 is the low word's first bit");
                assert_eq!(*buttons2, 1 << 1 | 1 << 4, "button 17 and hat-up (button 20) land in the high word");
            }
            other => panic!("expected a manual control message, got {other:?}"),
        }
        assert_eq!(HAT_DIRECTIONS, [0x01, 0x04, 0x08, 0x02], "the hat button order is up, down, left, right");
        let state = stick.snapshot(&settings, Support::default(), 41);
        assert_eq!(state["buttons"][19]["hatIndex"], Value::Null, "a real button belongs to no hat");
        assert_eq!(state["buttons"][20]["hatIndex"], 0);
        assert_eq!(state["buttons"][20]["direction"], "up", "the head must not have to guess which of the four hat buttons is which, or it will bind gimbal-left to hat-down");
        assert_eq!(state["buttons"][21]["direction"], "down");
        assert_eq!(state["buttons"][22]["direction"], "left");
        assert_eq!(state["buttons"][23]["direction"], "right");

        let mut wide = Joystick::new(4, 40, 0);
        ATTITUDE.iter().enumerate().for_each(|(axis, function)| {
            wide.set_axis_function(*function, axis);
            wide.set_calibration(axis, centered());
        });
        wide.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let mut far = [false; 40];
        far[35] = true;
        let out = wide.on_input(Input { axes: &[0, 0, 0, 0], buttons: &far, hats: &[] }, &settings, Support::default(), 41);
        match control_of(&out).unwrap() {
            Out::ManualControl { buttons, buttons2, .. } => assert_eq!((*buttons, *buttons2), (0, 0), "the message carries 32 button bits, so a 36th button is dropped rather than aliased onto another button's bit"),
            other => panic!("expected a manual control message, got {other:?}"),
        }
        let state = wide.snapshot(&settings, Support::default(), 41);
        assert_eq!(state["vehicleButtonBits"], VEHICLE_BUTTON_BITS);
        assert_eq!(state["buttons"][31]["reachesVehicle"], true);
        assert_eq!(state["buttons"][35]["reachesVehicle"], false, "a binding past the bitmap has to be visibly dead, not silently accepted");
    }

    #[test]
    fn throttle_follows_the_mode_the_firmware_supports_and_the_smoothing_accumulator_clears() {
        let mut stick = mapped();
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let bottom = Input { axes: &[0, 0, 0, -32768], buttons: &[false; 4], hats: &[0] };
        let base = flying();
        let out = stick.on_input(bottom, &base, Support::default(), 41);
        assert_eq!(axes_of(&out).unwrap().3, 0.0, "with down-zero throttle the stick bottom is zero thrust");

        let centre_zero = centre_zero_flying();
        let out = stick.on_input(bottom, &centre_zero, Support::default(), 82);
        assert_eq!(axes_of(&out).unwrap().3, 0.0, "centre-zero throttle without negative thrust floors at zero rather than reversing");
        let reversing = Settings { negative_thrust: true, ..centre_zero };
        let out = stick.on_input(bottom, &reversing, Support { negative_thrust: true, ..Support::default() }, 123);
        assert_eq!(axes_of(&out).unwrap().3, -1.0, "both the setting and the firmware have to allow reverse thrust");
        let centre = Input { axes: &[0, 0, 0, 0], buttons: &[false; 4], hats: &[0] };
        let out = stick.on_input(centre, &centre_zero, Support::default(), 164);
        assert_eq!(axes_of(&out).unwrap().3, 0.0, "with centre zero the stick centre means no thrust");
        let out = stick.on_input(centre, &centre_zero, down_zero_firmware(), 205);
        assert_eq!(axes_of(&out).unwrap().3, 0.5, "a firmware that does not support centre zero gets the 0:1 conversion regardless of the setting, so the same stick means half thrust");

        let smoothing = Settings { throttle_smoothing: true, ..centre_zero };
        let top = Input { axes: &[0, 0, 0, 32767], buttons: &[false; 4], hats: &[0] };
        let first = axes_of(&stick.on_input(top, &smoothing, Support::default(), 246)).unwrap().3;
        let second = axes_of(&stick.on_input(top, &smoothing, Support::default(), 287)).unwrap().3;
        assert!(first > 0.0 && second > first && second < 1.0, "smoothing ramps instead of jumping");
        stick.stop(320);
        stick.set_polling(Polling { vehicle: true, configuration: false }, 320);
        let after = axes_of(&stick.on_input(top, &smoothing, Support::default(), 362)).unwrap().3;
        assert_eq!(after, first, "the accumulator is cleared when polling stops, so a new session does not inherit the last flight's throttle");
    }

    #[test]
    fn an_unknown_firmware_capability_must_not_flip_the_throttle_mapping() {
        assert!(Support::default().throttle_mode_center_zero, "the base firmware plugin supports centre zero, so a forgotten capability field has to be the upstream answer, not the opposite one");
        assert!(!Support::default().negative_thrust);
        let mut stick = mapped();
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let centre = Input { axes: &[0, 0, 0, 0], buttons: &[false; 4], hats: &[0] };
        let settings = centre_zero_flying();
        let out = stick.on_input(centre, &settings, Support::default(), 41);
        assert_eq!(axes_of(&out).unwrap().3, 0.0, "an operator who ticked centre-zero and flies a default vehicle gets zero thrust at the neutral stick, not fifty percent");
        let state = stick.snapshot(&settings, down_zero_firmware(), 41);
        assert_eq!(state["support"]["throttleModeCenterZero"], false);
        assert_eq!(state["requestedThrottleMode"], "CENTER_ZERO");
        assert_eq!(state["effectiveThrottleMode"], "DOWN_ZERO", "the head can only grey the control and show what is actually flying if the capability reaches it");
        assert_eq!(state["negativeThrustActive"], false);
        let supported = stick.snapshot(&Settings { negative_thrust: true, ..settings }, Support { negative_thrust: true, ..Support::default() }, 41);
        assert_eq!(supported["effectiveThrottleMode"], "CENTER_ZERO");
        assert_eq!(supported["negativeThrustActive"], true);
    }

    #[test]
    fn smoothing_is_refused_in_down_zero_mode_so_releasing_the_throttle_still_reduces_thrust() {
        let smoothing = Settings { throttle_smoothing: true, ..flying() };
        assert!(!smoothing.smoothing_active(Support::default()), "accumulation is only ever legal with centre-zero throttle");
        assert!(Settings { throttle_smoothing: true, ..centre_zero_flying() }.smoothing_active(Support::default()));
        assert!(!Settings { throttle_smoothing: true, ..centre_zero_flying() }.smoothing_active(down_zero_firmware()), "a firmware that cannot do centre zero cannot accumulate either");

        let mut stick = mapped();
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let top = Input { axes: &[0, 0, 0, 32767], buttons: &[false; 4], hats: &[0] };
        let centre = Input { axes: &[0, 0, 0, 0], buttons: &[false; 4], hats: &[0] };
        let ramped = axes_of(&stick.on_input(top, &smoothing, Support::default(), 41)).unwrap().3;
        assert_eq!(ramped, 1.0, "in down-zero mode the throttle stick is followed directly");
        let released = axes_of(&stick.on_input(centre, &smoothing, Support::default(), 82)).unwrap().3;
        assert_eq!(released, 0.5, "returning the stick reduces thrust instead of freezing it where the integrator stopped");
        assert_eq!(stick.snapshot(&smoothing, Support::default(), 82)["throttleSmoothingActive"], false);
    }

    #[test]
    fn the_aux_axes_go_to_manual_control_extensions_or_to_rc_override_but_never_both() {
        let mut stick = mapped();
        stick.set_axis_function(Function::Additional1, 4);
        stick.set_calibration(4, centered());
        stick.set_axis_function(Function::PitchExtension, 5);
        stick.set_calibration(5, centered());
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let mut settings = flying();
        settings.optional_enabled[Function::Additional1.extension_bit().unwrap()] = true;
        settings.optional_enabled[Function::PitchExtension.extension_bit().unwrap()] = true;
        let input = Input { axes: &[0, 0, 0, 0, 32767, 16384], buttons: &[false; 4], hats: &[0] };
        let out = stick.on_input(input, &settings, Support::default(), 41);
        match control_of(&out).unwrap() {
            Out::ManualControl { enabled_extensions, extensions, .. } => {
                assert_eq!(*enabled_extensions, 0b0000_0101, "the pitch extension is bit 0 and additional axis 1 is bit 2");
                assert_eq!(extensions[0], 500);
                assert_eq!(extensions[2], 1000);
                assert_eq!(extensions[1], 0, "a disabled extension sends zero and, crucially, leaves its enable bit clear so the vehicle ignores it");
            }
            other => panic!("expected a manual control message, got {other:?}"),
        }
        assert!(overrides_of(&out).is_empty());

        let overriding = Settings { additional_axes: AdditionalAxes::RcChannelsOverride, ..settings };
        let out = stick.on_input(input, &overriding, Support::default(), 82);
        assert_eq!(
            overrides_of(&out),
            vec![[2200, u16::MAX, u16::MAX, u16::MAX, 0, 0]],
            "the enabled aux axis drives channel 5 and every channel this joystick does not hold is left alone, because the camera layer overrides the same block"
        );
        match control_of(&out).unwrap() {
            Out::ManualControl { enabled_extensions, .. } => assert_eq!(*enabled_extensions, 0b0000_0001, "in override mode only the roll and pitch extensions still ride in manual control"),
            other => panic!("expected a manual control message, got {other:?}"),
        }
        let back = stick.on_input(input, &settings, Support::default(), 123);
        assert_eq!(overrides_of(&back), vec![[0, u16::MAX, u16::MAX, u16::MAX, 0, 0]], "leaving override mode releases only the channel it held");
        let again = stick.on_input(input, &settings, Support::default(), 164);
        let third = stick.on_input(input, &settings, Support::default(), 205);
        assert_eq!(overrides_of(&again).len() + overrides_of(&third).len(), 2, "the release is repeated for a few ticks, because one dropped packet would leave the channel overridden until the vehicle's own timeout");
        let settled = stick.on_input(input, &settings, Support::default(), 246);
        assert!(overrides_of(&settled).is_empty(), "and then it stops, rather than releasing forever");
    }

    #[test]
    fn a_held_rc_channel_is_released_even_when_the_fault_that_stops_the_axes_arrives_first() {
        let mut stick = mapped();
        stick.set_axis_function(Function::Additional1, 4);
        stick.set_calibration(4, centered());
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let mut settings = flying();
        settings.additional_axes = AdditionalAxes::RcChannelsOverride;
        settings.optional_enabled[Function::Additional1.extension_bit().unwrap()] = true;
        let full = Input { axes: &[0, 0, 0, 0, 32767], buttons: &[false; 4], hats: &[0] };
        assert_eq!(overrides_of(&stick.on_input(full, &settings, Support::default(), 41)), vec![[2200, u16::MAX, u16::MAX, u16::MAX, 0, 0]]);
        assert!(stick.rc_override_active());

        let short = Input { axes: &[0, 0, 0, 0], buttons: &[false; 4], hats: &[0] };
        let dropped = stick.on_input(short, &settings, Support::default(), 82);
        assert!(dropped.iter().any(|o| matches!(o, Out::MissingAxisData { .. })), "the head dropping an axis for a frame is still a missing-data fault");
        assert_eq!(
            overrides_of(&dropped),
            vec![[0, u16::MAX, u16::MAX, u16::MAX, 0, 0]],
            "the release has to be produced on the very path the fault takes, or the channel stays at its last pwm with the flag that would clear it stuck on"
        );
        assert!(!stick.rc_override_active());
        let next = stick.on_input(short, &settings, Support::default(), 123);
        assert_eq!(overrides_of(&next).len(), 1, "and the repeat keeps draining while the fault persists");
    }

    #[test]
    fn opening_the_calibration_screen_on_a_live_stick_releases_the_channels_it_held() {
        let mut stick = mapped();
        stick.set_axis_function(Function::Additional1, 4);
        stick.set_calibration(4, centered());
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let mut settings = flying();
        settings.additional_axes = AdditionalAxes::RcChannelsOverride;
        settings.optional_enabled[Function::Additional1.extension_bit().unwrap()] = true;
        stick.on_input(Input { axes: &[0, 0, 0, 0, 32767], buttons: &[false; 4], hats: &[0] }, &settings, Support::default(), 41);
        assert!(stick.rc_override_active());
        let opened = stick.set_polling(Polling { vehicle: true, configuration: true }, 82);
        assert_eq!(
            overrides_of(&opened),
            vec![[0, u16::MAX, u16::MAX, u16::MAX, 0, 0]; RC_OVERRIDE_RELEASE_COUNT as usize],
            "configuration mode stops producing channels, so the held ones must be handed back and handed back more than once"
        );
        assert!(!stick.rc_override_active());
    }

    #[test]
    fn no_sample_stale_sample_and_a_centred_sample_are_three_different_answers() {
        let mut stick = mapped();
        let settings = flying();
        let idle = stick.snapshot(&settings, Support::default(), 0);
        assert_eq!(idle["sampleAgeMs"], Value::Null, "before the head pushes anything the age is absent, not zero");
        assert_eq!(idle["stale"], Value::Null);
        assert_eq!(idle["axes"][0]["raw"], Value::Null, "and an axis with no reading is null, not a centred stick");
        assert_eq!(idle["shaped"], Value::Null);

        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        stick.on_input(Input { axes: &[0, 0, 0, 0], buttons: &[false; 4], hats: &[0] }, &settings, Support::default(), 100);
        let fresh = stick.snapshot(&settings, Support::default(), 100);
        assert_eq!(fresh["sampleAgeMs"], 0);
        assert_eq!(fresh["stale"], false);
        assert_eq!(fresh["axes"][0]["raw"], 0, "a stick truly at rest reads zero, which is a reading");
        assert!(fresh["shaped"].is_object());
        let stale = stick.snapshot(&settings, Support::default(), 141);
        assert_eq!(stale["stale"], true, "a sample older than one axis period could not have fed the last message");
        assert_eq!(stale["shaped"], Value::Null, "and the last commanded attitude goes with it, instead of drawing frozen sticks that look live");
        stick.stop(200);
        assert_eq!(stick.snapshot(&settings, Support::default(), 200)["sampleAgeMs"], Value::Null, "stopping forgets the sample instead of leaving a stale one to look live");
    }

    #[test]
    fn the_state_says_why_nothing_is_reaching_the_vehicle() {
        let mut stick = mapped();
        let settings = flying();
        assert_eq!(stick.snapshot(&settings, Support::default(), 0)["notCommandingReason"], BLOCKED_NOT_POLLING);
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        assert_eq!(stick.snapshot(&Settings { calibrated: false, ..settings }, Support::default(), 0)["notCommandingReason"], BLOCKED_NOT_CALIBRATED);
        assert_eq!(stick.snapshot(&settings, Support::default(), 0)["notCommandingReason"], BLOCKED_NO_SAMPLE);
        stick.on_input(Input { axes: &[0, 0], buttons: &[false; 4], hats: &[0] }, &settings, Support::default(), 41);
        let blind = stick.snapshot(&settings, Support::default(), 41);
        assert_eq!(blind["commanding"], false);
        assert_eq!(blind["notCommandingReason"], BLOCKED_MISSING_AXIS_DATA, "a mapped axis the head stopped reporting must not render as a healthy mapping while nothing flies");
        assert_eq!(blind["stale"], false, "which it would, because the sample itself is fresh");
        stick.on_input(Input { axes: &[0, 0, 0, 0], buttons: &[false; 4], hats: &[0] }, &settings, Support::default(), 82);
        let healthy = stick.snapshot(&settings, Support::default(), 82);
        assert_eq!(healthy["commanding"], true);
        assert_eq!(healthy["notCommandingReason"], Value::Null);
        let mut configuring = mapped();
        configuring.set_polling(Polling { vehicle: true, configuration: true }, 0);
        assert_eq!(configuring.snapshot(&settings, Support::default(), 0)["notCommandingReason"], BLOCKED_CONFIGURING);
    }

    #[test]
    fn the_state_names_the_axis_for_every_function_and_refuses_to_share_one() {
        let mut stick = mapped();
        assert!(!stick.set_axis_function(Function::Yaw, 0), "roll already owns axis 0, and a coupled mapping rolls the vehicle when the operator yaws");
        assert_eq!(stick.axis_for(Function::Yaw), Some(2), "the refused assignment leaves the old one intact");
        stick.clear_axis_function(Function::Roll);
        assert!(stick.set_axis_function(Function::Yaw, 0), "once the axis is free it can be reassigned");
        let settings = flying();
        let state = stick.snapshot(&settings, Support::default(), 0);
        let functions = state["functions"].as_array().unwrap();
        assert_eq!(functions.len(), FUNCTIONS.len());
        assert_eq!(functions[0], json!({ "id": "roll", "axis": Value::Null }), "the mapping has to be renderable in the direction the operator thinks in");
        assert_eq!(functions[2], json!({ "id": "yaw", "axis": 0 }));
        assert_eq!(state["missingFunctions"], json!(["roll"]));
    }

    #[test]
    fn a_partly_mapped_stick_says_what_is_missing_and_sends_nothing() {
        let mut stick = Joystick::new(4, 2, 0);
        let settings = flying();
        stick.set_axis_function(Function::Roll, 0);
        stick.set_axis_function(Function::Pitch, 1);
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let out = stick.on_input(Input { axes: &[0, 0, 0, 0], buttons: &[false; 2], hats: &[] }, &settings, Support::default(), 41);
        assert_eq!(out, vec![Out::MappingIncomplete { missing: vec![Function::Yaw, Function::Throttle] }], "an unmapped attitude axis stops the whole message rather than sending a zero for it");
        ATTITUDE.iter().enumerate().for_each(|(axis, function)| {
            stick.set_axis_function(*function, axis);
        });
        let short = stick.on_input(Input { axes: &[0, 0], buttons: &[false; 2], hats: &[] }, &settings, Support::default(), 82);
        assert_eq!(short, vec![Out::MissingAxisData { axes: vec![2, 3] }], "a mapped axis the head did not report is missing data, which is a different fault from a missing mapping");
        let enabled_but_unmapped = Settings { optional_enabled: [true, false, false, false, false, false, false, false], ..settings };
        let out = stick.on_input(Input { axes: &[0, 0, 0, 0], buttons: &[false; 2], hats: &[] }, &enabled_but_unmapped, Support::default(), 123);
        assert_eq!(out, vec![Out::MappingIncomplete { missing: vec![Function::PitchExtension] }], "an enabled extension is as required as the attitude axes");
        assert_eq!(stick.missing_functions(&enabled_but_unmapped), vec![Function::PitchExtension]);
    }

    #[test]
    fn stored_settings_that_do_not_cover_the_enabled_axes_demand_a_new_calibration() {
        let mut stick = mapped();
        assert!(stick.validate(&flying()).is_empty(), "a complete mapping loads as it is");
        let with_extension = Settings { optional_enabled: [true, false, false, false, false, false, false, false], ..flying() };
        assert_eq!(stick.validate(&with_extension), vec![Out::RequireCalibration { missing: vec![Function::PitchExtension] }]);
        assert_eq!(stick.axis_for(Function::Roll), None, "the incomplete mapping is thrown away, so nothing half-loaded reaches a vehicle");
        let mut fresh = mapped();
        assert!(fresh.validate(&Settings { calibrated: false, ..flying() }).is_empty());
        assert_eq!(fresh.axis_for(Function::Roll), None, "an uncalibrated stick keeps no stored ranges at all");
        let mut bad = mapped();
        bad.calibration[0] = AxisCalibration { min: 100, max: -100, center: 0, deadband: 0, reversed: false };
        assert_eq!(bad.validate(&flying()), vec![Out::RequireCalibration { missing: Vec::new() }], "a stored range that cannot be believed is thrown away too, mapping or no mapping");
        assert_eq!(bad.axis_for(Function::Roll), None);
    }

    #[test]
    fn the_transmitter_mode_has_one_owner_and_a_repeated_apply_changes_nothing() {
        let mut stick = mapped();
        stick.set_axis_function(Function::Additional1, 4);
        assert_eq!(stick.transmitter_mode(), TRANSMITTER_MODE_STORED, "a stick loads in the mode its stored mapping was saved in");
        assert!(stick.set_transmitter_mode(1));
        assert_eq!(stick.axis_for(Function::Throttle), Some(1), "mode 1 swaps throttle and pitch against mode 2");
        assert_eq!(stick.axis_for(Function::Pitch), Some(3));
        assert_eq!(stick.axis_for(Function::Roll), Some(0));
        assert_eq!(stick.axis_for(Function::Yaw), Some(2));
        assert_eq!(stick.axis_for(Function::Additional1), Some(4), "an extension axis is not part of the transmitter mode table");
        assert_eq!(stick.transmitter_mode(), 1);
        stick.set_transmitter_mode(1);
        assert_eq!(stick.axis_for(Function::Throttle), Some(1), "applying the mode the stick is already in is a no-op, so a settings-changed signal cannot double-remap the sticks");
        assert!(stick.set_transmitter_mode(TRANSMITTER_MODE_STORED));
        assert_eq!(
            ATTITUDE.map(|f| stick.axis_for(f)),
            [Some(0), Some(1), Some(2), Some(3)],
            "the round trip through the stored mode is the identity, which is what makes saving in mode 2 safe"
        );
        assert!(!stick.set_transmitter_mode(9), "a mode outside 1..4 is refused");
        assert_eq!(stick.transmitter_mode(), TRANSMITTER_MODE_STORED);
        assert_eq!(stick.snapshot(&flying(), Support::default(), 0)["transmitterMode"], TRANSMITTER_MODE_STORED);
    }

    #[test]
    fn the_default_transmitter_mode_is_a_firmware_capability_not_a_flat_constant() {
        assert_eq!(Support::default().default_transmitter_mode, 2, "the base firmware plugin answers mode 2");
        let sub = Support { default_transmitter_mode: 3, ..Support::default() };
        let state = mapped().snapshot(&flying(), sub, 0);
        assert_eq!(state["support"]["defaultTransmitterMode"], 3, "a sub defaults to mode 3, and picking 2 for it swaps pitch and throttle on the physical sticks");
        let catalog = joystick_view(&Nothing, &[]);
        let entry = catalog["settings"].as_array().unwrap().iter().find(|s| s["name"] == "transmitterMode").unwrap().clone();
        assert_eq!(entry["default"], Value::Null, "a static catalog cannot know the vehicle, so it must not publish a default that contradicts one");
        assert_eq!(entry["defaultFrom"], "support.defaultTransmitterMode");
    }

    #[test]
    fn configuration_polling_reports_raw_travel_and_never_touches_the_vehicle() {
        let mut stick = mapped();
        stick.set_button_action(0, Some("Arm"));
        stick.set_polling(Polling { vehicle: false, configuration: true }, 0);
        let settings = flying();
        let out = stick.on_input(Input { axes: &[123, 0, 0, 0], buttons: &[true, false, false, false], hats: &[0] }, &settings, Support::default(), 41);
        assert!(out.contains(&Out::RawButton { index: 0, pressed: true }));
        assert!(out.contains(&Out::RawAxes { values: vec![Some(123), Some(0), Some(0), Some(0), None, None] }), "an axis the head did not report stays absent in the calibration display");
        assert!(!out.iter().any(|o| matches!(o, Out::ManualControl { .. } | Out::Action { .. })), "calibrating a stick must not fly the vehicle");
        let released = stick.on_input(Input { axes: &[123, 0, 0, 0], buttons: &[false; 4], hats: &[0] }, &settings, Support::default(), 82);
        assert!(released.contains(&Out::RawButton { index: 0, pressed: false }));
    }

    #[test]
    fn the_calibration_display_shapes_an_assigned_axis_the_same_way_as_an_unassigned_one() {
        let with_deadband = AxisCalibration { min: -32768, max: 32767, center: 0, deadband: 5000, reversed: false };
        let mut stick = mapped();
        [0, 4].iter().for_each(|axis| {
            stick.set_calibration(*axis, with_deadband);
        });
        stick.set_polling(Polling { vehicle: false, configuration: true }, 0);
        let settings = Settings { use_deadband: true, ..flying() };
        stick.on_input(Input { axes: &[1000, 1000, 1000, 1000, 1000, 1000], buttons: &[false; 4], hats: &[0] }, &settings, Support::default(), 41);
        let state = stick.snapshot(&settings, Support::default(), 41);
        assert_eq!(state["axes"][0]["function"], "roll");
        assert_eq!(state["axes"][4]["function"], Value::Null);
        assert_eq!(state["axes"][0]["shaped"], 0.0);
        assert_eq!(
            state["axes"][4]["shaped"],
            state["axes"][0]["shaped"],
            "an identical reading on an identical range renders identically whether or not a function is assigned, or the trace jumps the instant the operator picks one and reads as a broken axis"
        );
    }

    #[test]
    fn an_uncalibrated_or_unpolled_stick_sends_nothing_at_all() {
        let mut stick = mapped();
        let settings = flying();
        let input = Input { axes: &[0, 0, 0, 0], buttons: &[false; 4], hats: &[0] };
        assert!(stick.on_input(input, &settings, Support::default(), 41).is_empty(), "with no polling asked for, a pushed sample is dropped");
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        assert!(stick.on_input(input, &Settings { calibrated: false, ..settings }, Support::default(), 41).is_empty(), "an uncalibrated stick is never allowed to command a vehicle");
        stick.set_button_action(0, Some("Arm"));
        let pressed = Input { axes: &[0, 0, 0, 0], buttons: &[true, false, false, false], hats: &[0] };
        assert!(stick.on_input(pressed, &Settings { calibrated: false, ..settings }, Support::default(), 82).is_empty(), "and its buttons are dead too");
        [123, 164].iter().for_each(|now| {
            stick.on_input(input, &Settings { calibrated: false, ..settings }, Support::default(), *now);
        });
        assert!(
            stick.on_input(pressed, &settings, Support::default(), 205).contains(&Out::Action { action: "Arm".into(), event: ButtonEvent::Down }),
            "the press has to be a fresh down transition, because a button already held while the stick was uncalibrated must not arm the moment calibration lands"
        );
    }

    #[test]
    fn a_button_the_head_stops_reporting_releases_instead_of_latching_down() {
        let mut stick = mapped();
        stick.set_button_action(0, Some("Gimbal Up"));
        stick.set_polling(Polling { vehicle: true, configuration: false }, 0);
        let settings = flying();
        stick.on_input(Input { axes: &[0, 0, 0, 0], buttons: &[true, false, false, false], hats: &[0] }, &settings, Support::default(), 0);
        let short = stick.on_input(Input { axes: &[0, 0, 0, 0], buttons: &[], hats: &[] }, &settings, Support::default(), 1);
        assert!(short.contains(&Out::Action { action: "Gimbal Up".into(), event: ButtonEvent::Up }), "a gimbal left moving is worse than one that stops, so an unreported button counts as released");
        assert_eq!(ButtonEvent::None.step(true), ButtonEvent::Down);
        assert_eq!(ButtonEvent::Down.step(true), ButtonEvent::Repeat);
        assert_eq!(ButtonEvent::Repeat.step(true), ButtonEvent::Repeat);
        assert_eq!(ButtonEvent::Repeat.step(false), ButtonEvent::Up);
        assert_eq!(ButtonEvent::Up.step(false), ButtonEvent::None);
        assert_eq!(ButtonEvent::Up.step(true), ButtonEvent::Up, "a press seen in the same cycle as the release waits for the next one, exactly as the polled version did");
    }

    #[test]
    fn the_internal_smoothing_integrator_is_a_diagnostic_not_hardware_state() {
        let state = mapped().snapshot(&flying(), Support::default(), 0);
        assert_eq!(state["throttleAccumulator"], Value::Null, "a head has no correct way to draw the module's own integrator alongside stick positions");
        assert_eq!(state["diagnostics"]["throttleAccumulator"], 0.0);
    }

    struct Nothing;
    impl Backend for Nothing {
        fn get(&self, _path: &str) -> String {
            json!({ "kind": "null" }).to_string()
        }
        fn get_fields(&self, _path: &str, _fields: &str) -> String {
            self.get("")
        }
        fn set(&self, _path: &str, _value: &str) -> String {
            self.get("")
        }
        fn invoke(&self, _path: &str, _args: &str) -> String {
            self.get("")
        }
        fn watch(&self, _paths: &[String]) {}
    }

    #[test]
    fn the_catalog_names_every_rule_the_head_needs_and_formats_nothing() {
        let view = joystick_view(&Nothing, &[]);
        assert_eq!(view["class"], "JoystickMapping");
        assert_eq!(view["functions"].as_array().unwrap().len(), 12);
        assert_eq!(view["functions"][0], json!({ "id": "roll", "required": true, "extensionBit": Value::Null, "rcChannel": Value::Null, "carriedBy": ["MANUAL_CONTROL"] }));
        assert_eq!(view["functions"][4], json!({ "id": "pitchExtension", "required": false, "extensionBit": 0, "rcChannel": Value::Null, "carriedBy": ["MANUAL_CONTROL"] }));
        assert_eq!(
            view["functions"][6],
            json!({ "id": "additionalAxis1", "required": false, "extensionBit": 2, "rcChannel": 5, "carriedBy": ["MANUAL_CONTROL", "RC_CHANNELS_OVERRIDE"] }),
            "the operator wiring a payload has to be told which rc channel the axis drives, and that flipping the mode moves it onto a different message"
        );
        assert_eq!(view["functions"][11]["rcChannel"], 10, "the override block is channels 5 to 10, not a bare count of six");
        assert_eq!(view["rcOverride"]["firstChannel"], RC_OVERRIDE_FIRST_CHANNEL);
        assert_eq!(view["rcOverride"]["pwmMin"], 800.0);
        assert_eq!(view["rcOverride"]["pwmMax"], 2200.0);
        assert_eq!(view["rcOverride"]["releaseFrames"], RC_OVERRIDE_RELEASE_COUNT);
        assert_eq!(view["hatDirections"], json!(["up", "down", "left", "right"]), "four buttons per hat is useless without the order they come in");
        assert_eq!(view["vehicleButtonBits"], 32);
        assert_eq!(view["transmitterModes"][1]["functions"], json!(["yaw", "throttle", "roll", "pitch"]));
        let actions = view["actions"].as_array().unwrap();
        assert_eq!(actions[0]["action"], ACTION_NONE);
        assert_eq!(actions[0]["id"], ACTION_NONE_ID);
        assert!(
            actions.iter().all(|a| a["id"].as_str().is_some_and(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()))),
            "the operators read Ukrainian, so every binding label needs a stable id to localise from instead of untranslatable English prose"
        );
        assert!(actions.iter().any(|a| a["id"] == "motorInterlockDisable" && a["action"] == "Motor Interlock disable"));
        assert!(actions.iter().any(|a| a["action"] == "Step Zoom In" && a["repeat"] == true));
        assert!(actions.iter().any(|a| a["action"] == "Gimbal Up" && a["up"] == true && a["repeat"] == false));
        assert_eq!(actions.iter().filter(|a| a["repeat"] == true).count(), 4, "the two continuous zooms and the two step zooms repeat, and nothing else does");
        assert!(actions.iter().any(|a| a["action"] == "Continuous Zoom In" && a["repeat"] == true));
        let settings = view["settings"].as_array().unwrap();
        let rate = settings.iter().find(|s| s["name"] == "axisFrequencyHz").unwrap();
        assert_eq!(rate["units"], "Hz", "the head formats the number, the core only names the unit");
        assert_eq!((rate["min"].as_f64(), rate["max"].as_f64(), rate["default"].as_f64()), (Some(0.25), Some(200.0), Some(25.0)));
        assert_eq!(settings.iter().find(|s| s["name"] == "exponentialPct").unwrap()["units"], "%");
        assert_eq!(settings.iter().find(|s| s["name"] == "additionalAxesFunction").unwrap()["enumValues"], json!(["MANUAL_CONTROL", "RC_CHANNELS_OVERRIDE"]));
        assert_eq!(settings.iter().filter(|s| s["name"].as_str().is_some_and(|name| name.starts_with("enable_"))).count(), 8);
        let numeric_text: Vec<(&String, &Value)> = settings
            .iter()
            .flat_map(|entry| entry.as_object().unwrap())
            .filter(|(key, value)| ["default", "min", "max", "units"].contains(&key.as_str()) && value.as_str().is_some_and(|text| text.chars().any(char::is_numeric)))
            .collect();
        assert!(
            numeric_text.is_empty(),
            "a number that arrives as text has already chosen a decimal separator, and the operators read Ukrainian: {numeric_text:?}"
        );
    }

    #[test]
    fn the_action_names_are_the_wire_contract_and_no_token_is_invented() {
        let named = ["Gripper Close", "Gripper Open"];
        named.iter().for_each(|action| assert!(ACTIONS.iter().any(|(_, name, _, _)| name == action), "{action} is the name the executor dispatches on, so a renamed token is a silently dead button"));
        let invented = ["Gripper Grab", "Gripper Release", "Gripper Hold", "Continuous Focus In", "Continuous Focus Out", "Step Focus In", "Step Focus Out"];
        let catalog = joystick_view(&Nothing, &[])["actions"].as_array().unwrap().clone();
        invented.iter().for_each(|action| {
            assert!(!ACTIONS.iter().any(|(_, name, _, _)| name == action), "{action} reaches no executor, so advertising it as available hands the operator a button that does nothing");
            assert!(!catalog.iter().any(|entry| entry["action"] == *action));
            assert!(!can_repeat(action));
        });
        assert_eq!(ACTIONS.len(), 31);
        assert_eq!(action_id("Gripper Close"), Some("gripperClose"));
        assert_eq!(action_id("Loiter"), None, "a flight mode the core does not know has no id to localise, which the head has to see");
    }

    #[test]
    fn the_deadband_and_throttle_mode_defaults_match_the_settings_the_head_reads_back() {
        assert!(!Settings::default().use_deadband, "the stored default is off, and a head that trusts a catalog saying otherwise gets the opposite of every existing profile");
        assert_eq!(Settings::default().throttle_mode, ThrottleMode::DownZero);
        assert_eq!(ThrottleMode::CenterZero.stored(), 0);
        assert_eq!(ThrottleMode::DownZero.stored(), 1, "the old encoding is an int where centre-zero is 0, so a bool named for centre-zero inverts on migration");
        let settings = joystick_view(&Nothing, &[])["settings"].as_array().unwrap().clone();
        assert_eq!(settings.iter().find(|s| s["name"] == "useDeadband").unwrap()["default"], false);
        let mode = settings.iter().find(|s| s["name"] == "throttleMode").unwrap();
        assert_eq!(mode["default"], 1);
        assert_eq!(mode["enumValues"], json!(["CENTER_ZERO", "DOWN_ZERO"]));
        assert!(!settings.iter().any(|s| s["name"] == "throttleModeCenterZero"), "a bool whose polarity nobody can check has no place in the catalog");
    }
}