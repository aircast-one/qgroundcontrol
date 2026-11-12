use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

const ARMED_PWM: u16 = 1750;
const BAY_POSITIONS: f32 = 6.0;
const R2D2_SIX_POS_PWM: (f32, f32) = (1411.0, 1477.0);
pub const LAYOUT_SETTINGS: [&str; 11] = [
    "settings.autoConnectSettings.usbRadioPitchChannel",
    "settings.autoConnectSettings.usbRadioYawChannel",
    "settings.autoConnectSettings.usbRadioZoomChannel",
    "settings.autoConnectSettings.usbRadioPictureChannel",
    "settings.autoConnectSettings.usbRadioPaletteChannel",
    "settings.autoConnectSettings.usbRadioDropChannel",
    "settings.autoConnectSettings.usbRadioArmChannel",
    "settings.autoConnectSettings.usbRadioBayChannel",
    "settings.autoConnectSettings.usbRadioTiltChannel",
    "settings.autoConnectSettings.usbRadioLookChannel",
    "settings.autoConnectSettings.usbRadioStickChannel",
];
pub const LOOK_DOWN_PITCH: f32 = -90.0;
pub const LOOK_AHEAD_PITCH: f32 = 0.0;
pub const PICTURE_SETTING: &str = "IR_PIP";
pub const PALETTE_SETTING: &str = "IR_PALETTE";
pub const PICTURES: [u8; 3] = [0, 1, 3];
pub const STICK_PITCH_RANGE: (f32, f32) = (-90.0, 0.0);
pub const UNKNOWN_YAW_LIMITS: (f32, f32) = (-180.0, 180.0);
pub const TURN_SETTINGS: [&str; 2] = [
    "settings.autoConnectSettings.usbRadioYawSpeed",
    "settings.autoConnectSettings.usbRadioLookReversed",
];
pub const YAW_SPEED_DEFAULT: f32 = 60.0;
const YAW_EXPO: f32 = 0.7;
const YAW_DEADBAND_PWM: f32 = 40.0;
const STICK_CENTRE_PWM: f32 = 1500.0;
const STICK_HALF_SPAN_PWM: f32 = 512.0;
pub const STEADY_PWM: u16 = 3;
const PITCH_STEP_DEG: f32 = 0.5;
pub const RESEND_MS: u64 = 100;
const STICK_LOW_PWM: f32 = 988.0;
const STICK_SPAN_PWM: f32 = 1024.0;
const SWITCH_LOW_PWM: u16 = 1250;
const SWITCH_HIGH_PWM: u16 = 1750;
const PRESSED_PWM: u16 = 1500;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Move {
    Aim { pitch: f32, yaw: f32 },
    Turn(f32),
    Zoom(f32),
    Picture(u8),
    NextPalette,
    Drop { bay: i64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub pitch: usize,
    pub yaw: usize,
    pub zoom: usize,
    pub picture: usize,
    pub palette: usize,
    pub drop: usize,
    pub arm: usize,
    pub bay: usize,
    pub tilt: usize,
    pub look: usize,
    pub stick: usize,
    pub six_pos: Option<[u16; crate::sixpos::POSITIONS]>,
}

pub const R2D2: Layout = Layout {
    pitch: 18,
    yaw: 20,
    zoom: 21,
    picture: 22,
    palette: 24,
    drop: 11,
    arm: 12,
    bay: 5,
    tilt: 19,
    look: 23,
    stick: 17,
    six_pos: None,
};

const FUNCTION_NAMES: [&str; 11] = [
    "camera pitch",
    "camera yaw",
    "zoom",
    "picture",
    "palette",
    "drop",
    "arm",
    "bay",
    "tilt",
    "look",
    "left stick camera",
];

pub fn clash_notice(channel: usize, names: &[&str]) -> String {
    format!(
        "The drone reads flight modes on CH{channel}, so the radio's {} switch is off. Move flight modes or the switch to another channel.",
        names.join(" and ")
    )
}

impl Default for Layout {
    fn default() -> Self {
        R2D2
    }
}

impl Layout {
    pub fn without(self, channel: usize) -> Layout {
        let off = |at: usize| if channel > 0 && at == channel { 0 } else { at };
        Layout {
            pitch: off(self.pitch),
            yaw: off(self.yaw),
            zoom: off(self.zoom),
            picture: off(self.picture),
            palette: off(self.palette),
            drop: off(self.drop),
            arm: off(self.arm),
            bay: off(self.bay),
            tilt: off(self.tilt),
            look: off(self.look),
            stick: off(self.stick),
            six_pos: self.six_pos,
        }
    }

    pub fn named_on(&self, channel: usize) -> Vec<&'static str> {
        FUNCTION_NAMES
            .iter()
            .zip(self.channels())
            .filter(|(_, at)| channel > 0 && *at == channel)
            .map(|(name, _)| *name)
            .collect()
    }

    pub fn channels(&self) -> [usize; 11] {
        [
            self.pitch,
            self.yaw,
            self.zoom,
            self.picture,
            self.palette,
            self.drop,
            self.arm,
            self.bay,
            self.tilt,
            self.look,
            self.stick,
        ]
    }
}

pub fn layout() -> Layout {
    let [
        pitch,
        yaw,
        zoom,
        picture,
        palette,
        drop,
        arm,
        bay,
        tilt,
        look,
        stick,
    ] = crate::settingsstore::raw_settings(LAYOUT_SETTINGS).map(|value| {
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
            .map_or(0, |number| number as usize)
    });
    Layout {
        pitch,
        yaw,
        zoom,
        picture,
        palette,
        drop,
        arm,
        bay,
        tilt,
        look,
        stick,
        six_pos: crate::sixpos::learned(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ready {
    pub gimbal: bool,
    pub camera: bool,
    pub settings: bool,
}

pub const ALL_READY: Ready = Ready {
    gimbal: true,
    camera: true,
    settings: true,
};

impl Default for Ready {
    fn default() -> Self {
        ALL_READY
    }
}

#[derive(Debug)]
pub struct Sticks {
    acted: BTreeMap<usize, u16>,
    sent_ms: Option<u64>,
    drop_before: Option<u16>,
    yaw: Option<f32>,
    turn: Option<f32>,
    yaw_centred: bool,
    pub layout: Layout,
    pub bay: i64,
    pub ready: Ready,
    pub reported_yaw: Option<f32>,
    pub reported_pitch: Option<f32>,
    pub aimed_pitch: Option<f32>,
    pub yaw_speed: f32,
    pub look_reversed: bool,
    pub stick: Option<[usize; 2]>,
}

impl Default for Sticks {
    fn default() -> Self {
        Sticks::new()
    }
}

fn read(line: &[u16], channel: usize) -> Option<u16> {
    channel
        .checked_sub(1)
        .and_then(|at| line.get(at))
        .copied()
        .filter(|pwm| *pwm != u16::MAX)
}

fn between((low, high): (f32, f32), pwm: u16) -> f32 {
    let share = ((f32::from(pwm) - STICK_LOW_PWM) / STICK_SPAN_PWM).clamp(0.0, 1.0);
    low + share * (high - low)
}

fn turn_rate(pwm: u16, speed: f32) -> f32 {
    let off = f32::from(pwm) - STICK_CENTRE_PWM;
    let past = ((off.abs() - YAW_DEADBAND_PWM).max(0.0) / (STICK_HALF_SPAN_PWM - YAW_DEADBAND_PWM))
        .min(1.0);
    off.signum() * (past * (1.0 - YAW_EXPO) + past.powi(3) * YAW_EXPO) * speed
}

fn bay(pwm: u16, learned: Option<[u16; crate::sixpos::POSITIONS]>) -> Option<i64> {
    match learned {
        Some(table) => crate::sixpos::decode(pwm, table),
        None => known_bay(pwm),
    }
}

fn known_bay(pwm: u16) -> Option<i64> {
    [
        R2D2_SIX_POS_PWM,
        (STICK_LOW_PWM, STICK_LOW_PWM + STICK_SPAN_PWM),
    ]
    .into_iter()
    .find_map(|(low, high)| {
        let step = (high - low) / (BAY_POSITIONS - 1.0);
        let at = ((f32::from(pwm) - low) / step).round();
        ((0.0..BAY_POSITIONS).contains(&at)
            && (f32::from(pwm) - (low + at * step)).abs() <= step / 3.0)
            .then_some(at as i64 + 1)
    })
}

fn position(pwm: u16) -> usize {
    match pwm {
        ..SWITCH_LOW_PWM => 0,
        SWITCH_LOW_PWM..=SWITCH_HIGH_PWM => 1,
        _ => 2,
    }
}

impl Sticks {
    pub const fn new() -> Sticks {
        Sticks {
            acted: BTreeMap::new(),
            sent_ms: None,
            drop_before: None,
            yaw: None,
            turn: None,
            yaw_centred: false,
            layout: R2D2,
            bay: 1,
            ready: ALL_READY,
            reported_yaw: None,
            reported_pitch: None,
            aimed_pitch: None,
            yaw_speed: YAW_SPEED_DEFAULT,
            look_reversed: false,
            stick: None,
        }
    }

    fn steering(&self) -> Layout {
        self.stick.map_or(self.layout, |[pitch, yaw]| Layout {
            pitch,
            yaw,
            ..self.layout
        })
    }

    pub fn hand_back(&mut self) -> Option<Move> {
        let stop = self
            .turn
            .filter(|rate| *rate != 0.0)
            .map(|_| Move::Turn(0.0));
        self.turn = None;
        self.yaw_centred = false;
        stop
    }

    pub fn hand_stick(&mut self, stick: Option<[usize; 2]>) {
        if stick != self.stick {
            self.stick
                .into_iter()
                .chain(stick)
                .flatten()
                .for_each(|channel| {
                    self.acted.remove(&channel);
                });
            self.stick = stick;
        }
    }

    pub fn step(&mut self, line: &[u16], now_ms: u64) -> Vec<Move> {
        let layout = self.steering();
        let ready = self.ready;
        let due = self
            .sent_ms
            .is_none_or(|at| now_ms.saturating_sub(at) >= RESEND_MS);
        let moved = |channel: usize| {
            read(line, channel)
                .zip(self.acted.get(&channel))
                .is_some_and(|(now, was)| now.abs_diff(*was) >= STEADY_PWM)
        };
        let travelled = |channel: usize| {
            read(line, channel)
                .zip(self.acted.get(&channel))
                .map_or(0, |(now, was)| now.abs_diff(*was))
        };
        let pitch_source = if layout.pitch == 0 || travelled(layout.tilt) > travelled(layout.pitch)
        {
            layout.tilt
        } else {
            layout.pitch
        };
        let slider = read(line, layout.yaw).map(|pwm| turn_rate(pwm, self.yaw_speed));
        let rate = slider
            .filter(|_| ready.gimbal && self.yaw_centred)
            .unwrap_or(0.0);
        let turning = rate != 0.0;
        let yaw = self.yaw.or(self.reported_yaw).unwrap_or(0.0);
        let (lowest, highest) = STICK_PITCH_RANGE;
        let from = self.aimed_pitch.or(self.reported_pitch);
        let pointed = read(line, pitch_source).map(|pwm| {
            let at = between(STICK_PITCH_RANGE, pwm);
            let wanted = from
                .zip(self.acted.get(&pitch_source))
                .map_or(at, |(from, was)| {
                    at + (from - between(STICK_PITCH_RANGE, *was))
                });
            (wanted.clamp(lowest, highest), wanted)
        });
        let unchanged = |pitch: f32| from.is_some_and(|was| (was - pitch).abs() < PITCH_STEP_DEG);
        let aimed = pointed
            .map(|(pitch, _)| pitch)
            .filter(|pitch| ready.gimbal && due && moved(pitch_source) && !unchanged(*pitch))
            .map(|pitch| Move::Aim { pitch, yaw });
        let overrun = ready.gimbal
            && moved(pitch_source)
            && pointed.is_some_and(|(pitch, wanted)| pitch != wanted && unchanged(pitch));
        let ahead_at = if self.look_reversed { 0 } else { 2 };
        let looked = read(line, layout.look)
            .map(position)
            .filter(|at| {
                ready.gimbal
                    && *at != 1
                    && self
                        .acted
                        .get(&layout.look)
                        .is_some_and(|was| position(*was) != *at)
            })
            .map(|at| match at == ahead_at {
                true => Move::Aim {
                    pitch: LOOK_AHEAD_PITCH,
                    yaw: 0.0,
                },
                false => Move::Aim {
                    pitch: LOOK_DOWN_PITCH,
                    yaw,
                },
            });
        let changed = match self.turn {
            Some(sent) => sent != rate && (due || !turning),
            None => turning,
        };
        let aims = aimed.is_some() || looked.is_some();
        let turned = (ready.gimbal && (changed || (aims && turning))).then_some(Move::Turn(rate));
        self.turn = turned.map_or(self.turn, |_| Some(rate));
        self.aimed_pitch = match looked.or(aimed) {
            Some(Move::Aim { pitch, .. }) => Some(pitch),
            _ => self.aimed_pitch,
        };
        self.yaw_centred = self.yaw_centred || slider == Some(0.0);
        self.yaw = match (looked, turning) {
            (_, true) => None,
            (Some(Move::Aim { yaw, .. }), false) => Some(yaw),
            _ => self.yaw,
        };
        let zoomed = read(line, layout.zoom)
            .filter(|_| ready.camera && due && moved(layout.zoom))
            .map(|zoom| Move::Zoom(between((0.0, 100.0), zoom)));
        let switched = read(line, layout.picture)
            .map(position)
            .filter(|at| {
                ready.settings
                    && self
                        .acted
                        .get(&layout.picture)
                        .is_some_and(|was| position(*was) != *at)
            })
            .map(|at| Move::Picture(PICTURES[at]));
        let pressed = |channel: usize| {
            read(line, channel).is_some_and(|pwm| pwm > PRESSED_PWM)
                && self
                    .acted
                    .get(&channel)
                    .is_some_and(|was| *was <= PRESSED_PWM)
        };
        let palette = ready.settings && pressed(layout.palette);
        let high = |pwm: Option<u16>| pwm.is_some_and(|pwm| pwm > PRESSED_PWM);
        let dropped = (high(read(line, layout.drop))
            && high(self.acted.get(&layout.drop).copied())
            && self.drop_before.is_some_and(|pwm| pwm <= PRESSED_PWM))
        .then_some(Move::Drop { bay: self.bay });
        let refreshed: Vec<(usize, u16)> = [
            layout.drop,
            layout.arm,
            layout.bay,
            layout.pitch,
            layout.yaw,
            layout.zoom,
            layout.picture,
            layout.palette,
            layout.tilt,
            layout.look,
        ]
        .into_iter()
        .filter(|channel| {
            !self.acted.contains_key(channel)
                || match *channel {
                    c if c == layout.pitch || c == layout.yaw || c == layout.tilt => {
                        aimed.is_some() || overrun
                    }
                    c if c == layout.zoom => zoomed.is_some(),
                    c if c == layout.picture || c == layout.palette => ready.settings,
                    c if c == layout.look => ready.gimbal,
                    _ => true,
                }
        })
        .filter_map(|channel| read(line, channel).map(|pwm| (channel, pwm)))
        .collect();
        self.drop_before = read(line, layout.drop)
            .map_or(self.drop_before, |_| self.acted.get(&layout.drop).copied());
        self.acted.extend(refreshed);
        if aimed.is_some() || zoomed.is_some() || turned.is_some() {
            self.sent_ms = Some(now_ms);
        }
        aimed
            .into_iter()
            .chain(looked)
            .chain(turned)
            .chain(zoomed)
            .chain(switched)
            .chain(palette.then_some(Move::NextPalette))
            .chain(dropped)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AimState {
    acted: [Option<u16>; 3],
    turn: Option<f32>,
    aimed_pitch: Option<f32>,
}

impl Sticks {
    fn aim_channels(&self) -> [usize; 3] {
        let layout = self.steering();
        [layout.pitch, layout.yaw, layout.tilt]
    }

    pub fn aim_state(&self) -> AimState {
        AimState {
            acted: self
                .aim_channels()
                .map(|channel| self.acted.get(&channel).copied()),
            turn: self.turn,
            aimed_pitch: self.aimed_pitch,
        }
    }

    pub fn restore_aim(&mut self, state: AimState) {
        self.aim_channels()
            .into_iter()
            .zip(state.acted)
            .for_each(|(channel, was)| match was {
                Some(pwm) => {
                    self.acted.insert(channel, pwm);
                }
                None => {
                    self.acted.remove(&channel);
                }
            });
        self.turn = state.turn;
        self.aimed_pitch = state.aimed_pitch;
    }
}

static STICKS: Mutex<Sticks> = Mutex::new(Sticks::new());

fn ready() -> Ready {
    let (gimbal, camera) = crate::hub::lock()
        .active()
        .map_or((false, false), |vehicle| {
            (
                vehicle.gimbals.active().is_some(),
                vehicle.cameras.selected().is_some(),
            )
        });
    Ready {
        gimbal,
        camera,
        settings: crate::camsettings::settings_ready(),
    }
}

fn reported_gimbal() -> (Option<crate::gimbal::Attitude>, Option<f32>) {
    let hub = crate::hub::lock();
    hub.active()
        .and_then(|vehicle| {
            vehicle
                .gimbals
                .active()
                .and_then(|pair| vehicle.gimbals.get(pair))
        })
        .map_or((None, None), |gimbal| (gimbal.attitude, gimbal.aimed_pitch))
}

fn turning_settings() -> (f32, bool) {
    let [speed, reversed] = crate::settingsstore::raw_settings(TURN_SETTINGS);
    (
        speed
            .as_f64()
            .or_else(|| speed.as_str().and_then(|text| text.parse().ok()))
            .map_or(YAW_SPEED_DEFAULT, |speed| speed as f32),
        reversed.as_bool() == Some(true) || reversed.as_str() == Some("true"),
    )
}

pub fn slider_yaw_range((low, high): (f32, f32)) -> (f32, f32) {
    let (widest_low, widest_high) = UNKNOWN_YAW_LIMITS;
    (low.max(widest_low), high.min(widest_high))
}

fn guided(vehicle: u8, action: Value, told: Option<&str>) -> bool {
    let sent = crate::hub::lock().guided(Some(vehicle), &action, crate::hub::now_ms());
    match (sent, told) {
        (Ok(frames), _) => {
            frames.iter().for_each(|(link, bytes)| {
                crate::linkhost::write(&crate::linkhost::TRANSPORTS, *link, bytes);
            });
            true
        }
        (Err(reason), Some(kind)) => {
            crate::noticeboard::post_from_vehicle(kind, &reason);
            false
        }
        (Err(reason), None) => {
            log::debug!("the radio's camera channels could not {action}: {reason}");
            false
        }
    }
}

fn setting(answer: Value) {
    if answer["ok"] != json!(true) {
        log::debug!("the radio's camera channels could not change a camera setting: {answer}");
    }
}

pub fn steer(vehicle: u8, moves: &[Move]) -> bool {
    moves
        .iter()
        .map(|step| match *step {
            Move::Aim { pitch, yaw } => guided(
                vehicle,
                json!({ "action": "gimbal", "op": "aim", "pitch": pitch, "bodyYaw": yaw }),
                None,
            ),
            Move::Turn(yaw) => guided(
                vehicle,
                json!({ "action": "gimbal", "op": "rates", "pitch": 0.0, "yaw": yaw }),
                None,
            ),
            other => {
                steer_other(vehicle, other);
                true
            }
        })
        .fold(true, |aimed, sent| aimed && sent)
}

pub fn handed_back(vehicle: u8) {
    let stop = STICKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .hand_back();
    steer(vehicle, &Vec::from_iter(stop));
}

fn steer_other(vehicle: u8, step: Move) {
    match step {
        Move::Aim { .. } | Move::Turn(_) => {}
        Move::Zoom(percent) => {
            guided(
                vehicle,
                json!({ "action": "camera", "op": "level", "axis": "zoom", "percent": percent }),
                None,
            );
        }
        Move::Drop { bay } => {
            match crate::payloadrail::drop_decision(bay, crate::payloadrail::RADIO) {
                Ok(bay) => {
                    guided(
                        vehicle,
                        json!({ "action": "gripper", "gripAction": crate::guided::GRIPPER_RELEASE, "bay": bay }),
                        Some(crate::noticeboard::PAYLOAD),
                    );
                }
                Err(reason) => {
                    crate::noticeboard::post_from_vehicle(crate::noticeboard::PAYLOAD, &reason);
                }
            }
        }
        Move::Picture(value) => setting(crate::camsettings::set_setting(
            &json!([PICTURE_SETTING, value]).to_string(),
        )),
        Move::NextPalette => setting(crate::camsettings::step_setting(PALETTE_SETTING, 1)),
    }
}

pub fn switches(
    line: &[u16],
    now_ms: u64,
    layout: Layout,
) -> Option<crate::payloadrail::RadioSwitches> {
    read(line, layout.arm).map(|arm| crate::payloadrail::RadioSwitches {
        armed: arm >= ARMED_PWM,
        bay: read(line, layout.bay).and_then(|pwm| bay(pwm, layout.six_pos)),
        at_ms: now_ms,
    })
}

pub fn stick_switched(line: &[u16], layout: Layout) -> bool {
    read(line, layout.stick).is_some_and(|pwm| pwm > PRESSED_PWM)
}

pub fn heard(vehicle: u8, line: &[u16], layout: Layout, stick: Option<[usize; 2]>) {
    let (reported, aimed_pitch) = reported_gimbal();
    let (yaw_speed, look_reversed) = turning_settings();
    if let Some(switches) = switches(line, crate::hub::now_ms(), layout) {
        crate::payloadrail::radio_heard(switches);
    }
    let mut sticks = STICKS.lock().unwrap_or_else(PoisonError::into_inner);
    sticks.layout = layout;
    sticks.bay = crate::payloadrail::selected();
    sticks.ready = ready();
    sticks.reported_yaw = reported.map(|attitude| attitude.body_yaw);
    sticks.reported_pitch = reported.map(|attitude| attitude.pitch);
    sticks.aimed_pitch = aimed_pitch;
    sticks.yaw_speed = yaw_speed;
    sticks.look_reversed = look_reversed;
    sticks.hand_stick(stick);
    let before = sticks.aim_state();
    let moves = sticks.step(line, crate::hub::now_ms());
    if !steer(vehicle, &moves) {
        sticks.restore_aim(before);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PITCH_CHANNEL: usize = R2D2.pitch;
    const YAW_CHANNEL: usize = R2D2.yaw;
    const ZOOM_CHANNEL: usize = R2D2.zoom;
    const PICTURE_CHANNEL: usize = R2D2.picture;
    const PALETTE_CHANNEL: usize = R2D2.palette;
    const DROP_CHANNEL: usize = R2D2.drop;
    const ARM_CHANNEL: usize = R2D2.arm;
    const BAY_CHANNEL: usize = R2D2.bay;
    const TILT_CHANNEL: usize = R2D2.tilt;
    const LOOK_CHANNEL: usize = R2D2.look;

    fn line(pairs: &[(usize, u16)]) -> [u16; 24] {
        std::array::from_fn(|at| {
            pairs
                .iter()
                .find(|(channel, _)| *channel == at + 1)
                .map_or(1500, |(_, pwm)| *pwm)
        })
    }

    #[test]
    fn with_the_stick_switch_up_the_left_stick_steers_the_camera_like_the_sliders() {
        const THROTTLE: usize = 3;
        const RUDDER: usize = 4;
        assert!(stick_switched(&line(&[(R2D2.stick, 2012)]), R2D2));
        assert!(
            !stick_switched(&line(&[(R2D2.stick, 1500)]), R2D2),
            "only the switch's high end hands the stick over"
        );
        let mut sticks = Sticks::default();
        assert!(sticks.step(&line(&[(THROTTLE, 1100)]), 0).is_empty());
        sticks.hand_stick(Some([THROTTLE, RUDDER]));
        assert_eq!(
            sticks.step(&line(&[(THROTTLE, 1200)]), 1_000),
            vec![],
            "taking the stick leaves the camera where it is, wherever the throttle sits"
        );
        assert_eq!(
            sticks.step(&line(&[(THROTTLE, 988)]), 2_000),
            vec![Move::Aim {
                pitch: -90.0,
                yaw: 0.0
            }],
            "throttle down points the camera down, as the pitch slider does"
        );
        assert_eq!(
            sticks.step(&line(&[(THROTTLE, 988), (RUDDER, 2012)]), 3_000),
            vec![Move::Turn(YAW_SPEED_DEFAULT)],
            "yaw over turns the camera at the yaw slider's speed"
        );
        assert!(
            sticks
                .step(&line(&[(THROTTLE, 988), (RUDDER, 2012)]), 3_100)
                .is_empty(),
            "and keeps turning with nothing more sent"
        );
        assert_eq!(
            sticks.step(&line(&[(THROTTLE, 988)]), 3_200),
            vec![Move::Turn(0.0)],
            "and stops where it got to when the stick springs back"
        );
        sticks.hand_stick(None);
        assert!(
            sticks.step(&line(&[(THROTTLE, 2012)]), 4_000).is_empty(),
            "handed back, the left stick flies again and the camera stays put"
        );
        sticks.hand_stick(Some([THROTTLE, RUDDER]));
        assert!(
            sticks.step(&line(&[(THROTTLE, 1600)]), 5_000).is_empty(),
            "taking it again does not jump the camera to wherever the throttle was flown"
        );
    }
    #[test]
    fn an_aim_the_gimbal_refused_is_tried_again_from_the_next_line() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(PITCH_CHANNEL, 2012)]), 0);
        let before = sticks.aim_state();
        let down = line(&[(PITCH_CHANNEL, 988)]);
        assert_eq!(
            sticks.step(&down, 1_000),
            vec![Move::Aim {
                pitch: -90.0,
                yaw: 0.0
            }]
        );
        assert_eq!(
            sticks.step(&down, 2_000),
            vec![],
            "a slider that stays put is not sent again"
        );
        sticks.restore_aim(before);
        assert_eq!(
            sticks.step(&down, 3_000),
            vec![Move::Aim {
                pitch: -90.0,
                yaw: 0.0
            }],
            "once the refused aim is forgotten, the same slider position is sent again"
        );
    }

    #[test]
    fn the_pitch_slider_points_the_gimbal_once_moved_and_not_on_the_first_line() {
        let mut sticks = Sticks::default();
        assert_eq!(
            sticks.step(&line(&[(PITCH_CHANNEL, 1500)]), 0),
            vec![],
            "plugging the radio in leaves the camera where it is"
        );
        assert_eq!(
            sticks.step(&line(&[(PITCH_CHANNEL, 2012)]), 1_000),
            vec![Move::Aim {
                pitch: 0.0,
                yaw: 0.0
            }],
            "the right slider at the top looks ahead"
        );
        assert_eq!(
            sticks.step(&line(&[(PITCH_CHANNEL, 988)]), 2_000),
            vec![Move::Aim {
                pitch: -90.0,
                yaw: 0.0
            }],
            "and at the bottom straight down, without turning the camera"
        );
    }

    #[test]
    fn the_yaw_slider_turns_the_camera_at_a_rate_and_stops_it_at_the_centre() {
        let mut sticks = Sticks::default();
        assert!(sticks.step(&line(&[]), 0).is_empty());
        assert_eq!(
            sticks.step(&line(&[(YAW_CHANNEL, 2012)]), 1_000),
            vec![Move::Turn(YAW_SPEED_DEFAULT)],
            "all the way over it turns the camera at the set speed, as a rate the gimbal holds"
        );
        assert!(
            sticks.step(&line(&[(YAW_CHANNEL, 2010)]), 1_050).is_empty(),
            "a small move waits for the resend period"
        );
        assert!(
            sticks.step(&line(&[(YAW_CHANNEL, 2012)]), 1_100).is_empty(),
            "held where it was it sends nothing, so the gimbal turns smoothly instead of stepping between angles"
        );
        assert_eq!(
            sticks.step(&line(&[(YAW_CHANNEL, 1224)]), 1_200),
            vec![Move::Turn(turn_rate(1224, YAW_SPEED_DEFAULT))],
            "halfway to the other side turns the other way at half speed"
        );
        assert_eq!(
            sticks.step(&line(&[(YAW_CHANNEL, 1500 + 30)]), 1_250),
            vec![Move::Turn(0.0)],
            "back in the centre's dead band it stops at once, without waiting for the resend period"
        );
        assert!(
            sticks.step(&line(&[(YAW_CHANNEL, 1500)]), 1_400).is_empty(),
            "and holds there"
        );
    }

    #[test]
    fn a_radio_that_stops_mid_turn_stops_the_camera_and_a_slider_left_over_waits_for_the_centre() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[]), 0);
        let over = line(&[(YAW_CHANNEL, 2012)]);
        assert_eq!(
            sticks.step(&over, 1_000),
            vec![Move::Turn(YAW_SPEED_DEFAULT)]
        );
        assert_eq!(
            sticks.hand_back(),
            Some(Move::Turn(0.0)),
            "the app repeats a rate until told otherwise, so a radio gone quiet must stop the turn"
        );
        assert!(
            sticks.step(&over, 2_000).is_empty(),
            "back again with the slider still over, it waits for the centre before turning"
        );
        assert_eq!(sticks.hand_back(), None, "nothing turning, nothing to stop");
    }

    #[test]
    fn a_pitch_input_resting_at_its_end_or_jittering_does_not_keep_aiming_the_gimbal() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(TILT_CHANNEL, 1500)]), 0);
        assert_eq!(
            sticks.step(&line(&[(TILT_CHANNEL, 2012)]), 1_000),
            vec![Move::Aim {
                pitch: 0.0,
                yaw: 0.0
            }]
        );
        let resting: Vec<Move> = [2016, 2009, 2020, 2012]
            .into_iter()
            .zip((1..).map(|step| 1_000 + step * RESEND_MS))
            .flat_map(|(pwm, at)| sticks.step(&line(&[(TILT_CHANNEL, pwm)]), at))
            .collect();
        assert!(
            resting.is_empty(),
            "a knob at its end stop wanders a few microseconds, but the pitch it means stays 0: re-aiming ten times a second stops every turn"
        );
        let mut middle = Sticks::default();
        middle.step(&line(&[(PITCH_CHANNEL, 1400)]), 0);
        assert_eq!(middle.step(&line(&[(PITCH_CHANNEL, 1500)]), 1_000).len(), 1);
        assert!(
            middle
                .step(&line(&[(PITCH_CHANNEL, 1504)]), 1_100)
                .is_empty(),
            "4 microseconds of noise is a third of a degree"
        );
        assert_eq!(
            middle.step(&line(&[(PITCH_CHANNEL, 1510)]), 1_200),
            vec![Move::Aim {
                pitch: between(STICK_PITCH_RANGE, 1510),
                yaw: 0.0
            }],
            "a real move of a degree is sent"
        );
    }

    #[test]
    fn the_yaw_slider_nudges_near_the_centre_and_swings_fast_all_the_way_over() {
        assert_eq!(
            turn_rate(2012, 60.0),
            60.0,
            "all the way over is the full speed"
        );
        assert!(
            (turn_rate(1776, 60.0) - 14.25).abs() < 0.01,
            "halfway over is about a quarter of it"
        );
        assert!(
            turn_rate(1600, 60.0) < 3.0,
            "a fifth of the way over only nudges the camera"
        );
        assert_eq!(
            turn_rate(1500 + 40, 60.0),
            0.0,
            "the dead band turns nothing"
        );
        assert_eq!(turn_rate(988, 60.0), -60.0);
    }

    #[test]
    fn a_turn_the_gimbal_refused_is_sent_again_from_the_next_line() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[]), 0);
        let before = sticks.aim_state();
        let over = line(&[(YAW_CHANNEL, 2012)]);
        assert_eq!(
            sticks.step(&over, 1_000),
            vec![Move::Turn(YAW_SPEED_DEFAULT)]
        );
        sticks.restore_aim(before);
        assert_eq!(
            sticks.step(&over, 1_200),
            vec![Move::Turn(YAW_SPEED_DEFAULT)],
            "a turn the gimbal did not take is not taken as sent"
        );
    }
    #[test]
    fn a_yaw_slider_left_off_centre_turns_nothing_until_it_has_been_back_to_the_centre() {
        let mut sticks = Sticks::default();
        let over = line(&[(YAW_CHANNEL, 2012)]);
        assert!(sticks.step(&over, 0).is_empty());
        assert!(
            sticks.step(&over, 1_000).is_empty(),
            "a slider parked off centre when the radio connects does not spin the camera"
        );
        assert!(sticks.step(&line(&[(YAW_CHANNEL, 1500)]), 2_000).is_empty());
        assert_eq!(sticks.step(&over, 3_000).len(), 1);
    }
    #[test]
    fn a_slider_that_only_jitters_sends_nothing_and_a_moving_one_is_sent_ten_times_a_second() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(PITCH_CHANNEL, 1500)]), 0);
        assert!(
            sticks
                .step(&line(&[(PITCH_CHANNEL, 1500 + STEADY_PWM - 1)]), 1_000)
                .is_empty(),
            "slider noise does not move the camera"
        );
        assert_eq!(sticks.step(&line(&[(PITCH_CHANNEL, 1600)]), 2_000).len(), 1);
        assert!(
            sticks
                .step(&line(&[(PITCH_CHANNEL, 1700)]), 2_000 + RESEND_MS - 1)
                .is_empty()
        );
        assert_eq!(
            sticks.step(&line(&[(PITCH_CHANNEL, 1700)]), 2_000 + RESEND_MS),
            vec![Move::Aim {
                pitch: between(STICK_PITCH_RANGE, 1700),
                yaw: 0.0
            }],
            "a move held back by the resend period is sent on the next line after it"
        );
    }
    #[test]
    fn the_zoom_knob_sets_the_zoom_level_it_points_at() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(ZOOM_CHANNEL, 1422)]), 0);
        assert_eq!(
            sticks.step(&line(&[(ZOOM_CHANNEL, 2012)]), 1_000),
            vec![Move::Zoom(100.0)]
        );
        assert_eq!(
            sticks.step(&line(&[(ZOOM_CHANNEL, 988)]), 2_000),
            vec![Move::Zoom(0.0)]
        );
    }

    #[test]
    fn the_three_position_switch_picks_visible_the_cameras_default_or_thermal() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(PICTURE_CHANNEL, 2012)]), 0);
        assert_eq!(
            sticks.step(&line(&[(PICTURE_CHANNEL, 1500)]), 10),
            vec![Move::Picture(1)]
        );
        assert_eq!(
            sticks.step(&line(&[(PICTURE_CHANNEL, 988)]), 20),
            vec![Move::Picture(0)]
        );
        assert!(
            sticks.step(&line(&[(PICTURE_CHANNEL, 990)]), 30).is_empty(),
            "a switch left where it is changes nothing"
        );
        assert_eq!(
            sticks.step(&line(&[(PICTURE_CHANNEL, 2012)]), 40),
            vec![Move::Picture(3)]
        );
    }

    #[test]
    fn each_press_of_the_momentary_switch_steps_the_palette_once() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(PALETTE_CHANNEL, 988)]), 0);
        assert_eq!(
            sticks.step(&line(&[(PALETTE_CHANNEL, 2012)]), 10),
            vec![Move::NextPalette]
        );
        assert!(
            sticks
                .step(&line(&[(PALETTE_CHANNEL, 2012)]), 20)
                .is_empty(),
            "holding it down does not keep stepping"
        );
        assert!(sticks.step(&line(&[(PALETTE_CHANNEL, 988)]), 30).is_empty());
        assert_eq!(
            sticks.step(&line(&[(PALETTE_CHANNEL, 2012)]), 40),
            vec![Move::NextPalette]
        );
    }

    #[test]
    fn one_line_with_the_drop_switch_up_between_lines_with_it_down_drops_nothing() {
        let mut sticks = Sticks::default();
        let moves: Vec<Move> = [988, 2012, 988, 2012, 988]
            .into_iter()
            .zip((0..).step_by(10))
            .flat_map(|(drop, at)| sticks.step(&line(&[(DROP_CHANNEL, drop)]), at))
            .collect();
        assert!(moves.is_empty(), "a glitched line is not a flip: {moves:?}");
    }

    #[test]
    fn flipping_the_drop_switch_up_drops_the_selected_bay_once_and_flipping_it_down_drops_nothing()
    {
        let mut sticks = Sticks {
            bay: 1,
            ..Sticks::default()
        };
        sticks.step(&line(&[(DROP_CHANNEL, 988), (BAY_CHANNEL, 988)]), 0);
        assert!(
            sticks
                .step(&line(&[(DROP_CHANNEL, 2012), (BAY_CHANNEL, 988)]), 10)
                .is_empty(),
            "the first line with the switch up only arms the drop"
        );
        assert_eq!(
            sticks.step(&line(&[(DROP_CHANNEL, 2012), (BAY_CHANNEL, 988)]), 20),
            vec![Move::Drop { bay: 1 }],
            "the bay the payload rail has selected drops"
        );
        assert!(sticks.step(&line(&[(DROP_CHANNEL, 2012)]), 30).is_empty());
        assert!(sticks.step(&line(&[(DROP_CHANNEL, 988)]), 40).is_empty());
        sticks.bay = 4;
        sticks.step(&line(&[(DROP_CHANNEL, 2012), (BAY_CHANNEL, 1602)]), 50);
        assert_eq!(
            sticks.step(&line(&[(DROP_CHANNEL, 2012), (BAY_CHANNEL, 1602)]), 60),
            vec![Move::Drop { bay: 4 }],
            "a new selection drops next"
        );
        assert_eq!(
            [988, 1193, 1398, 1602, 1807, 2012].map(known_bay),
            [1, 2, 3, 4, 5, 6].map(Some),
            "EdgeTX spreads a 6POS switch evenly over the channel"
        );
        assert_eq!(
            [-179, -154, -127, -100, -73, -46].map(|raw| known_bay(crate::usbradio::pwm(raw))),
            [1, 2, 3, 4, 5, 6].map(Some),
            "R2D2's default model sends its 6POS in a narrow band below the centre, button 1 lowest"
        );
        assert_eq!(
            [1300, 1500, 1700].map(known_bay),
            [None; 3],
            "a channel resting between positions picks no bay, so the app's pick drops"
        );
        let learned = Some([1300, 1350, 1400, 1450, 1500, 1550]);
        assert_eq!(
            [1300, 1500, 1425].map(|pwm| bay(pwm, learned)),
            [Some(1), Some(5), None],
            "a learned 6POS reads only its own steps, not the scales it replaces"
        );
        let mut plugged_in_high = Sticks::default();
        assert!(
            plugged_in_high
                .step(&line(&[(DROP_CHANNEL, 2012)]), 0)
                .is_empty(),
            "a radio plugged in with the switch already up drops nothing"
        );
    }

    #[test]
    fn with_the_drop_switch_held_down_picking_another_bay_drops_nothing() {
        let mut sticks = Sticks {
            bay: 1,
            ..Sticks::default()
        };
        let at = |bay: u16, drop: u16| {
            line(&[
                (ARM_CHANNEL, 2012),
                (BAY_CHANNEL, bay),
                (DROP_CHANNEL, drop),
            ])
        };
        let moves: Vec<Move> = [
            (988, 988),
            (988, 988),
            (988, 2012),
            (988, 2012),
            (988, 2012),
            (1194, 2012),
            (1194, 2012),
            (1194, 2012),
            (1194, 2012),
        ]
        .into_iter()
        .zip((0..).step_by(10))
        .flat_map(|((bay, drop), now)| sticks.step(&at(bay, drop), now))
        .collect();
        assert_eq!(moves, vec![Move::Drop { bay: 1 }]);
    }

    #[test]
    fn the_6pos_only_picks_the_bay_and_the_drop_switch_drops_it() {
        let mut sticks = Sticks {
            bay: 3,
            ..Sticks::default()
        };
        let at = |bay: u16, drop: u16| {
            line(&[
                (ARM_CHANNEL, 2012),
                (BAY_CHANNEL, bay),
                (DROP_CHANNEL, drop),
            ])
        };
        let moves: Vec<Vec<Move>> = [
            (1398, 988),
            (1398, 988),
            (988, 988),
            (988, 988),
            (1602, 988),
            (1602, 988),
            (1602, 2012),
            (1602, 2012),
            (1602, 2012),
        ]
        .into_iter()
        .zip((0..).step_by(10))
        .map(|((bay, drop), now)| sticks.step(&at(bay, drop), now))
        .collect();
        assert_eq!(
            moves,
            vec![
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![Move::Drop { bay: 3 }],
                vec![],
            ],
            "with BOMARM armed a 6POS press drops nothing; the drop switch drops the picked bay, once"
        );
    }

    #[test]
    fn while_the_payload_is_safe_the_6pos_only_picks_the_bay() {
        let mut sticks = Sticks::default();
        let at = |arm: u16, bay: u16| {
            line(&[(ARM_CHANNEL, arm), (BAY_CHANNEL, bay), (DROP_CHANNEL, 988)])
        };
        let moves: Vec<Move> = [
            (988, 1398),
            (988, 1398),
            (988, 988),
            (988, 988),
            (2012, 988),
            (2012, 988),
            (2012, 988),
            (988, 988),
            (2012, 1602),
            (2012, 1602),
            (2012, 1602),
        ]
        .into_iter()
        .zip((0..).step_by(10))
        .flat_map(|((arm, bay), now)| sticks.step(&at(arm, bay), now))
        .collect();
        assert!(
            moves.is_empty(),
            "a position picked while safe, or together with arming, does not drop: {moves:?}"
        );
    }

    #[test]
    fn bomarm_up_arms_the_payload_and_the_6pos_picks_the_bay() {
        let up = switches(&line(&[(ARM_CHANNEL, 2012), (BAY_CHANNEL, 1398)]), 7, R2D2).unwrap();
        assert_eq!((up.armed, up.bay, up.at_ms), (true, Some(3), 7));
        assert!(
            !switches(&line(&[(ARM_CHANNEL, 1500)]), 0, R2D2)
                .unwrap()
                .armed,
            "the middle position is safe"
        );
        assert!(
            !switches(&line(&[(ARM_CHANNEL, 988)]), 0, R2D2)
                .unwrap()
                .armed
        );
        assert!(
            switches(&[1500; 11], 0, R2D2).is_none(),
            "a radio without CH12 arms nothing"
        );
    }

    #[test]
    fn channels_the_radio_did_not_send_steer_nothing() {
        let mut sticks = Sticks::default();
        let short: Vec<u16> = line(&[]).iter().take(17).copied().collect();
        assert!(sticks.step(&short, 0).is_empty());
        assert!(
            sticks
                .step(&line(&[(YAW_CHANNEL, 2012), (ZOOM_CHANNEL, 2012)]), 1_000)
                .is_empty(),
            "a channel first heard is where its control rests, not a move"
        );
        let unsent = [u16::MAX; 24];
        assert!(sticks.step(&unsent, 2_000).is_empty());
    }

    #[test]
    fn the_tilt_knob_and_the_pitch_slider_each_tilt_the_camera_on_from_where_the_other_left_it() {
        let mut sticks = Sticks::default();
        assert!(
            sticks.step(&line(&[(TILT_CHANNEL, 1500)]), 0).is_empty(),
            "the first line only records where the knob is"
        );
        assert_eq!(
            sticks.step(&line(&[(TILT_CHANNEL, 988)]), 1000),
            vec![Move::Aim {
                pitch: -90.0,
                yaw: 0.0
            }],
            "with no pitch known yet, the knob at its bottom points straight down"
        );
        assert_eq!(
            sticks.step(&line(&[(TILT_CHANNEL, 988), (PITCH_CHANNEL, 1756)]), 2000),
            vec![Move::Aim {
                pitch: -67.5,
                yaw: 0.0
            }],
            "the slider a quarter up tilts the camera a quarter up from there"
        );
        assert_eq!(
            sticks.step(&line(&[(TILT_CHANNEL, 1244), (PITCH_CHANNEL, 1756)]), 3000),
            vec![Move::Aim {
                pitch: -45.0,
                yaw: 0.0
            }],
            "and the knob a quarter up tilts it on from where the slider left it"
        );
    }

    #[test]
    fn after_look_the_knob_tilts_from_where_look_pointed_instead_of_jumping_back() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(TILT_CHANNEL, 1756)]), 0);
        assert_eq!(
            sticks.step(&line(&[(LOOK_CHANNEL, 988), (TILT_CHANNEL, 1756)]), 1_000),
            vec![Move::Aim {
                pitch: LOOK_DOWN_PITCH,
                yaw: 0.0
            }]
        );
        assert_eq!(
            sticks.step(&line(&[(LOOK_CHANNEL, 988), (TILT_CHANNEL, 2012)]), 2_000),
            vec![Move::Aim {
                pitch: -67.5,
                yaw: 0.0
            }],
            "a quarter turn up raises the camera a quarter from straight down, not to the knob's own 0"
        );
        assert_eq!(
            sticks.step(&line(&[(LOOK_CHANNEL, 2012), (TILT_CHANNEL, 2012)]), 3_000),
            vec![Move::Aim {
                pitch: LOOK_AHEAD_PITCH,
                yaw: 0.0
            }]
        );
        assert_eq!(
            sticks.step(&line(&[(LOOK_CHANNEL, 2012), (TILT_CHANNEL, 1756)]), 4_000),
            vec![Move::Aim {
                pitch: -22.5,
                yaw: 0.0
            }],
            "after Look ahead a quarter turn down lowers it a quarter from ahead"
        );
    }

    #[test]
    fn turning_the_knob_on_past_where_the_camera_stops_is_not_owed_back() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(TILT_CHANNEL, 1500)]), 0);
        sticks.step(&line(&[(LOOK_CHANNEL, 988)]), 1_000);
        assert!(
            sticks.step(&line(&[(TILT_CHANNEL, 988)]), 2_000).is_empty(),
            "the camera is already straight down, so turning the knob down moves nothing"
        );
        assert_eq!(
            sticks.step(&line(&[(TILT_CHANNEL, 1244)]), 3_000),
            vec![Move::Aim {
                pitch: -67.5,
                yaw: 0.0
            }],
            "and the first turn back up raises it at once"
        );
    }

    #[test]
    fn the_knob_tilts_from_the_last_aim_by_anyone_or_else_from_the_pitch_the_gimbal_reports() {
        let mut sticks = Sticks {
            reported_pitch: Some(-30.0),
            ..Sticks::default()
        };
        sticks.step(&line(&[(TILT_CHANNEL, 1500)]), 0);
        assert_eq!(
            sticks.step(&line(&[(TILT_CHANNEL, 1756)]), 1_000),
            vec![Move::Aim {
                pitch: -7.5,
                yaw: 0.0
            }]
        );
        sticks.aimed_pitch = Some(-60.0);
        assert_eq!(
            sticks.step(&line(&[(TILT_CHANNEL, 1500)]), 2_000),
            vec![Move::Aim {
                pitch: -82.5,
                yaw: 0.0
            }],
            "after an aim on screen the knob carries on from where the app pointed it"
        );
    }

    #[test]
    fn the_look_switch_points_ahead_when_high_and_down_when_low_and_rests_in_the_middle() {
        let mut sticks = Sticks::default();
        assert!(
            sticks.step(&line(&[(LOOK_CHANNEL, 1500)]), 0).is_empty(),
            "the first line only records the switch"
        );
        assert_eq!(
            sticks.step(&line(&[(LOOK_CHANNEL, 988)]), 1000),
            vec![Move::Aim {
                pitch: LOOK_DOWN_PITCH,
                yaw: 0.0
            }],
            "low points straight down"
        );
        assert!(
            sticks.step(&line(&[(LOOK_CHANNEL, 988)]), 2000).is_empty(),
            "holding the switch sends nothing more"
        );
        assert!(
            sticks.step(&line(&[(LOOK_CHANNEL, 1500)]), 3000).is_empty(),
            "the middle position leaves the gimbal where it is"
        );
        assert_eq!(
            sticks.step(&line(&[(LOOK_CHANNEL, 2012)]), 4000),
            vec![Move::Aim {
                pitch: LOOK_AHEAD_PITCH,
                yaw: 0.0
            }],
            "high points ahead, the way an R2D2 radio's switch up reads"
        );
        let mut reversed = Sticks {
            look_reversed: true,
            ..Sticks::default()
        };
        reversed.step(&line(&[(LOOK_CHANNEL, 1500)]), 0);
        assert_eq!(
            reversed.step(&line(&[(LOOK_CHANNEL, 2012)]), 1000),
            vec![Move::Aim {
                pitch: LOOK_DOWN_PITCH,
                yaw: 0.0
            }],
            "reversed, high points down"
        );
    }

    #[test]
    fn look_ahead_turns_the_camera_back_to_straight_ahead_and_look_down_keeps_its_heading() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[]), 0);
        assert_eq!(
            sticks.step(&line(&[(YAW_CHANNEL, 2012)]), 1_000),
            vec![Move::Turn(YAW_SPEED_DEFAULT)]
        );
        assert_eq!(sticks.step(&line(&[]), 1_200), vec![Move::Turn(0.0)]);
        sticks.reported_yaw = Some(4.0);
        assert_eq!(
            sticks.step(&line(&[(LOOK_CHANNEL, 988)]), 1_300),
            vec![Move::Aim {
                pitch: LOOK_DOWN_PITCH,
                yaw: 4.0
            }],
            "down keeps the heading the gimbal reports the slider turned it to"
        );
        assert_eq!(
            sticks.step(&line(&[(LOOK_CHANNEL, 2012)]), 2_000),
            vec![Move::Aim {
                pitch: LOOK_AHEAD_PITCH,
                yaw: 0.0
            }],
            "ahead undoes every turn"
        );
        assert_eq!(
            sticks.step(&line(&[(LOOK_CHANNEL, 2012), (YAW_CHANNEL, 988)]), 3_000),
            vec![Move::Turn(-YAW_SPEED_DEFAULT)],
            "and the next nudge turns from straight ahead"
        );
    }
    #[test]
    fn a_yaw_move_after_look_ahead_keeps_the_camera_level() {
        let mut sticks = Sticks::default();
        let rest = [
            (LOOK_CHANNEL, 1500),
            (PITCH_CHANNEL, 1500),
            (YAW_CHANNEL, 1500),
        ];
        assert!(sticks.step(&line(&rest), 0).is_empty());
        assert_eq!(
            sticks.step(&line(&[(LOOK_CHANNEL, 2012)]), 1_000),
            vec![Move::Aim {
                pitch: LOOK_AHEAD_PITCH,
                yaw: 0.0
            }]
        );
        assert_eq!(
            sticks.step(&line(&[(LOOK_CHANNEL, 2012), (YAW_CHANNEL, 2012)]), 2_000),
            vec![Move::Turn(YAW_SPEED_DEFAULT)],
            "turning the yaw slider sends only a rate, so the pitch Look chose stays, not the pitch slider's -45"
        );
        sticks.reported_yaw = Some(2.0);
        assert_eq!(
            sticks.step(
                &line(&[
                    (LOOK_CHANNEL, 2012),
                    (PITCH_CHANNEL, 988),
                    (YAW_CHANNEL, 2012)
                ]),
                3_000
            ),
            vec![
                Move::Aim {
                    pitch: -45.0,
                    yaw: 2.0
                },
                Move::Turn(YAW_SPEED_DEFAULT)
            ],
            "the pitch slider moving mid-turn tilts from where Look left it, at the yaw the gimbal is at, and the turn carries on"
        );
    }
    #[test]
    fn camera_and_payload_channels_are_settings_with_the_r2d2_layout_as_default() {
        let moved = Layout {
            pitch: 13,
            yaw: 14,
            zoom: 15,
            picture: 0,
            palette: 0,
            drop: 0,
            arm: 0,
            bay: 0,
            tilt: 0,
            look: 0,
            stick: 0,
            six_pos: None,
        };
        let mut sticks = Sticks {
            layout: moved,
            ..Sticks::default()
        };
        sticks.step(&line(&[(13, 1500), (14, 1500)]), 0);
        assert_eq!(
            sticks.step(&line(&[(13, 2012), (14, 1500)]), 1000).len(),
            1,
            "with pitch moved to CH13 that channel steers"
        );
        assert!(
            sticks
                .step(&line(&[(13, 2012), (14, 1500), (PITCH_CHANNEL, 988)]), 2000)
                .is_empty(),
            "and the R2D2 pitch channel steers nothing"
        );
        let mut off = Sticks {
            layout: moved,
            ..Sticks::default()
        };
        let drops: Vec<Move> = [988, 2012, 2012]
            .into_iter()
            .zip((0..).step_by(10))
            .flat_map(|(pwm, at)| off.step(&line(&[(DROP_CHANNEL, pwm)]), at))
            .collect();
        assert!(drops.is_empty(), "a function on channel 0 is off");
        assert!(switches(&line(&[(ARM_CHANNEL, 2012)]), 0, moved).is_none());
        assert_eq!(Sticks::default().layout, R2D2);
    }

    #[test]
    fn a_slider_moved_before_the_gimbal_is_found_aims_once_it_is() {
        let mut sticks = Sticks {
            ready: Ready {
                gimbal: false,
                camera: false,
                settings: false,
            },
            ..Sticks::default()
        };
        sticks.step(
            &line(&[
                (PITCH_CHANNEL, 1500),
                (YAW_CHANNEL, 1500),
                (ZOOM_CHANNEL, 1500),
            ]),
            0,
        );
        let moved = line(&[
            (PITCH_CHANNEL, 988),
            (YAW_CHANNEL, 1500),
            (ZOOM_CHANNEL, 2012),
        ]);
        assert!(
            sticks.step(&moved, 1000).is_empty(),
            "with no gimbal or camera found yet nothing is sent"
        );
        sticks.ready = ALL_READY;
        assert_eq!(
            sticks.step(&moved, 2000),
            vec![
                Move::Aim {
                    pitch: -90.0,
                    yaw: 0.0
                },
                Move::Zoom(100.0)
            ],
            "once they are found the slider's position is sent"
        );
    }

    #[test]
    fn the_yaw_slider_spans_at_most_half_a_turn_each_way() {
        assert_eq!(
            slider_yaw_range((-270.0, 270.0)),
            (-180.0, 180.0),
            "a ZR30 reports 270 degrees each way; an end of the slider past 180 would wrap to the other side"
        );
        assert_eq!(slider_yaw_range((-150.0, 150.0)), (-150.0, 150.0));
        assert_eq!(slider_yaw_range((-44.0, 90.0)), (-44.0, 90.0));
    }

    #[test]
    fn a_function_on_the_flight_mode_channel_is_off() {
        assert_eq!(
            R2D2.without(5),
            Layout { bay: 0, ..R2D2 },
            "flight modes on CH5 take the bay switch off it"
        );
        assert_eq!(R2D2.without(10), R2D2, "flight modes on CH10 leave it");
        assert_eq!(R2D2.without(0), R2D2, "no flight mode channel leaves it");
        assert_eq!(
            switches(&line(&[(ARM_CHANNEL, 2012), (5, 1500)]), 0, R2D2.without(5))
                .map(|radio| (radio.armed, radio.bay)),
            Some((true, None)),
            "the arm switch still arms, and the bay is left to the app"
        );
        assert_eq!(R2D2.named_on(5), vec!["bay"]);
        assert_eq!(R2D2.named_on(10), Vec::<&str>::new());
        assert_eq!(R2D2.named_on(0), Vec::<&str>::new());
        assert_eq!(
            clash_notice(5, &R2D2.named_on(5)),
            "The drone reads flight modes on CH5, so the radio's bay switch is off. Move flight modes or the switch to another channel."
        );
    }
}
