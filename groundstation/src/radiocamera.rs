use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

const ARMED_PWM: u16 = 1750;
const BAY_POSITIONS: f32 = 6.0;
pub const LAYOUT_SETTINGS: [&str; 10] = [
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
];
pub const LOOK_DOWN_PITCH: f32 = -90.0;
pub const LOOK_AHEAD_PITCH: f32 = 0.0;
pub const PICTURE_SETTING: &str = "IR_PIP";
pub const PALETTE_SETTING: &str = "IR_PALETTE";
pub const PICTURES: [u8; 3] = [0, 1, 3];
pub const STICK_PITCH_RANGE: (f32, f32) = (-90.0, 0.0);
pub const UNKNOWN_YAW_LIMITS: (f32, f32) = (-180.0, 180.0);
pub const STEADY_PWM: u16 = 3;
pub const RESEND_MS: u64 = 100;
const STICK_LOW_PWM: f32 = 988.0;
const STICK_SPAN_PWM: f32 = 1024.0;
const SWITCH_LOW_PWM: u16 = 1250;
const SWITCH_HIGH_PWM: u16 = 1750;
const PRESSED_PWM: u16 = 1500;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Move {
    Aim { pitch: f32, yaw: f32 },
    Zoom(f32),
    Picture(u8),
    NextPalette,
    Drop { bay: i64 },
    DropBay(i64),
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
};

const FUNCTION_NAMES: [&str; 10] = [
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

    pub fn channels(&self) -> [usize; 10] {
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

#[derive(Debug, Default)]
pub struct Sticks {
    acted: BTreeMap<usize, u16>,
    sent_ms: Option<u64>,
    drop_before: Option<u16>,
    bay_before: Option<u16>,
    arm_before: Option<u16>,
    pub layout: Layout,
    pub bay: i64,
    pub ready: Ready,
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

fn bay(pwm: u16) -> i64 {
    (between((0.0, BAY_POSITIONS - 1.0), pwm).round() as i64) + 1
}

fn position(pwm: u16) -> usize {
    match pwm {
        ..SWITCH_LOW_PWM => 0,
        SWITCH_LOW_PWM..=SWITCH_HIGH_PWM => 1,
        _ => 2,
    }
}

impl Sticks {
    pub fn step(&mut self, line: &[u16], yaw_limits: (f32, f32), now_ms: u64) -> Vec<Move> {
        let layout = self.layout;
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
        let aimed = read(line, pitch_source)
            .zip(read(line, layout.yaw))
            .filter(|_| ready.gimbal && due && (moved(pitch_source) || moved(layout.yaw)))
            .map(|(pitch, yaw)| Move::Aim {
                pitch: between(STICK_PITCH_RANGE, pitch),
                yaw: between(yaw_limits, yaw),
            });
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
            .map(|at| Move::Aim {
                pitch: if at == 2 {
                    LOOK_DOWN_PITCH
                } else {
                    LOOK_AHEAD_PITCH
                },
                yaw: read(line, layout.yaw).map_or(0.0, |yaw| between(yaw_limits, yaw)),
            });
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
        let armed = |pwm: Option<u16>| pwm.is_some_and(|pwm| pwm >= ARMED_PWM);
        let pressed_bay = read(line, layout.bay)
            .map(bay)
            .filter(|now| {
                armed(read(line, layout.arm))
                    && armed(self.acted.get(&layout.arm).copied())
                    && armed(self.arm_before)
                    && self.acted.get(&layout.bay).map(|pwm| bay(*pwm)) == Some(*now)
                    && self.bay_before.is_some_and(|pwm| bay(pwm) != *now)
            })
            .map(Move::DropBay);
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
                        aimed.is_some()
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
        self.bay_before = read(line, layout.bay)
            .map_or(self.bay_before, |_| self.acted.get(&layout.bay).copied());
        self.arm_before = read(line, layout.arm)
            .map_or(self.arm_before, |_| self.acted.get(&layout.arm).copied());
        self.acted.extend(refreshed);
        if aimed.is_some() || zoomed.is_some() {
            self.sent_ms = Some(now_ms);
        }
        aimed
            .into_iter()
            .chain(looked)
            .chain(zoomed)
            .chain(switched)
            .chain(palette.then_some(Move::NextPalette))
            .chain(dropped.filter(|_| pressed_bay.is_none()))
            .chain(pressed_bay)
            .collect()
    }
}

static STICKS: Mutex<Sticks> = Mutex::new(Sticks {
    acted: BTreeMap::new(),
    sent_ms: None,
    drop_before: None,
    bay_before: None,
    arm_before: None,
    layout: R2D2,
    bay: 1,
    ready: ALL_READY,
});

fn bays_through_the_app(vehicle: &crate::hub::Vehicle) -> Option<i64> {
    let servo_bays = vehicle.payload_servos().len() as i64;
    (vehicle.payload_setup().enabled || servo_bays > 0).then_some(servo_bays)
}

fn through_the_app(step: Move, servo_bays: Option<i64>) -> Option<Move> {
    match step {
        Move::Drop { bay } => servo_bays.map(|bays| Move::Drop {
            bay: named(bay, bays),
        }),
        Move::DropBay(bay) => servo_bays
            .filter(|bays| (1..=*bays).contains(&bay))
            .map(|_| Move::Drop { bay }),
        other => Some(other),
    }
}

pub fn named(bay: i64, servo_bays: i64) -> i64 {
    match (1..=servo_bays).contains(&bay) {
        true => bay,
        false => 0,
    }
}

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

fn yaw_limits() -> (f32, f32) {
    let hub = crate::hub::lock();
    hub.active()
        .and_then(|vehicle| {
            vehicle
                .gimbals
                .active()
                .and_then(|pair| vehicle.gimbals.get(pair))
        })
        .and_then(|gimbal| gimbal.limits[4].zip(gimbal.limits[5]))
        .map_or(UNKNOWN_YAW_LIMITS, slider_yaw_range)
}

pub fn slider_yaw_range((low, high): (f32, f32)) -> (f32, f32) {
    let (widest_low, widest_high) = UNKNOWN_YAW_LIMITS;
    (low.max(widest_low), high.min(widest_high))
}

fn guided(vehicle: u8, action: Value, told: Option<&str>) {
    let sent = crate::hub::lock().guided(Some(vehicle), &action, crate::hub::now_ms());
    match (sent, told) {
        (Ok(frames), _) => frames.iter().for_each(|(link, bytes)| {
            crate::linkhost::write(&crate::linkhost::TRANSPORTS, *link, bytes);
        }),
        (Err(reason), Some(kind)) => {
            crate::noticeboard::post_from_vehicle(kind, &reason);
        }
        (Err(reason), None) => {
            log::debug!("the radio's camera channels could not {action}: {reason}")
        }
    }
}

fn setting(answer: Value) {
    if answer["ok"] != json!(true) {
        log::debug!("the radio's camera channels could not change a camera setting: {answer}");
    }
}

pub fn steer(vehicle: u8, moves: &[Move]) {
    moves.iter().for_each(|step| match *step {
        Move::Aim { pitch, yaw } => guided(
            vehicle,
            json!({ "action": "gimbal", "op": "aim", "pitch": pitch, "bodyYaw": yaw }),
            None,
        ),
        Move::Zoom(percent) => guided(
            vehicle,
            json!({ "action": "camera", "op": "level", "axis": "zoom", "percent": percent }),
            None,
        ),
        Move::Drop { .. } | Move::DropBay(_) if !crate::payloadrail::armed() => {
            crate::noticeboard::post_from_vehicle(
                crate::noticeboard::PAYLOAD,
                crate::payloadrail::SAFE,
            );
        }
        Move::Drop { bay } | Move::DropBay(bay) => guided(
            vehicle,
            json!({ "action": "gripper", "gripAction": crate::guided::GRIPPER_RELEASE, "bay": bay }),
            Some(crate::noticeboard::PAYLOAD),
        ),
        Move::Picture(value) => setting(crate::camsettings::set_setting(
            &json!([PICTURE_SETTING, value]).to_string(),
        )),
        Move::NextPalette => setting(crate::camsettings::step_setting(PALETTE_SETTING, 1)),
    })
}

pub fn switches(
    line: &[u16],
    now_ms: u64,
    layout: Layout,
) -> Option<crate::payloadrail::RadioSwitches> {
    read(line, layout.arm).map(|arm| crate::payloadrail::RadioSwitches {
        armed: arm >= ARMED_PWM,
        bay: read(line, layout.bay).map(bay),
        at_ms: now_ms,
    })
}

pub fn heard(vehicle: u8, line: &[u16], layout: Layout) {
    let limits = yaw_limits();
    if let Some(switches) = switches(line, crate::hub::now_ms(), layout) {
        crate::payloadrail::radio_heard(switches);
    }
    let servo_bays = crate::hub::lock().active().and_then(bays_through_the_app);
    let mut sticks = STICKS.lock().unwrap_or_else(PoisonError::into_inner);
    sticks.layout = layout;
    sticks.bay = crate::payloadrail::selected();
    sticks.ready = ready();
    let moves: Vec<Move> = sticks
        .step(line, limits, crate::hub::now_ms())
        .into_iter()
        .filter_map(|step| through_the_app(step, servo_bays))
        .collect();
    steer(vehicle, &moves);
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

    const LIMITS: (f32, f32) = (-150.0, 150.0);

    fn line(pairs: &[(usize, u16)]) -> [u16; 24] {
        std::array::from_fn(|at| {
            pairs
                .iter()
                .find(|(channel, _)| *channel == at + 1)
                .map_or(1500, |(_, pwm)| *pwm)
        })
    }

    #[test]
    fn the_sliders_point_the_gimbal_once_moved_and_not_on_the_first_line() {
        let mut sticks = Sticks::default();
        assert_eq!(
            sticks.step(
                &line(&[(PITCH_CHANNEL, 2012), (YAW_CHANNEL, 1471)]),
                LIMITS,
                0
            ),
            vec![],
            "plugging the radio in leaves the camera where it is"
        );
        assert_eq!(
            sticks.step(
                &line(&[(PITCH_CHANNEL, 2012), (YAW_CHANNEL, 2012)]),
                LIMITS,
                1_000
            ),
            vec![Move::Aim {
                pitch: 0.0,
                yaw: 150.0
            }],
            "the right slider at the top looks ahead and the left one at the top turns to the gimbal's right limit"
        );
        assert_eq!(
            sticks.step(
                &line(&[(PITCH_CHANNEL, 988), (YAW_CHANNEL, 988)]),
                LIMITS,
                2_000
            ),
            vec![Move::Aim {
                pitch: -90.0,
                yaw: -150.0
            }],
            "both sliders at the bottom look straight down and as far left as the gimbal goes"
        );
    }

    #[test]
    fn a_slider_that_only_jitters_sends_nothing_and_a_moving_one_is_sent_ten_times_a_second() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(YAW_CHANNEL, 1500)]), LIMITS, 0);
        assert!(
            sticks
                .step(
                    &line(&[(YAW_CHANNEL, 1500 + STEADY_PWM - 1)]),
                    LIMITS,
                    1_000
                )
                .is_empty(),
            "slider noise does not move the camera"
        );
        assert_eq!(
            sticks
                .step(&line(&[(YAW_CHANNEL, 1600)]), LIMITS, 2_000)
                .len(),
            1
        );
        assert!(
            sticks
                .step(&line(&[(YAW_CHANNEL, 1700)]), LIMITS, 2_000 + RESEND_MS - 1)
                .is_empty()
        );
        assert_eq!(
            sticks.step(&line(&[(YAW_CHANNEL, 1700)]), LIMITS, 2_000 + RESEND_MS),
            vec![Move::Aim {
                pitch: between(STICK_PITCH_RANGE, 1500),
                yaw: between(LIMITS, 1700)
            }],
            "a move held back by the resend period is sent on the next line after it"
        );
    }

    #[test]
    fn the_zoom_knob_sets_the_zoom_level_it_points_at() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(ZOOM_CHANNEL, 1422)]), LIMITS, 0);
        assert_eq!(
            sticks.step(&line(&[(ZOOM_CHANNEL, 2012)]), LIMITS, 1_000),
            vec![Move::Zoom(100.0)]
        );
        assert_eq!(
            sticks.step(&line(&[(ZOOM_CHANNEL, 988)]), LIMITS, 2_000),
            vec![Move::Zoom(0.0)]
        );
    }

    #[test]
    fn the_three_position_switch_picks_visible_the_cameras_default_or_thermal() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(PICTURE_CHANNEL, 2012)]), LIMITS, 0);
        assert_eq!(
            sticks.step(&line(&[(PICTURE_CHANNEL, 1500)]), LIMITS, 10),
            vec![Move::Picture(1)]
        );
        assert_eq!(
            sticks.step(&line(&[(PICTURE_CHANNEL, 988)]), LIMITS, 20),
            vec![Move::Picture(0)]
        );
        assert!(
            sticks
                .step(&line(&[(PICTURE_CHANNEL, 990)]), LIMITS, 30)
                .is_empty(),
            "a switch left where it is changes nothing"
        );
        assert_eq!(
            sticks.step(&line(&[(PICTURE_CHANNEL, 2012)]), LIMITS, 40),
            vec![Move::Picture(3)]
        );
    }

    #[test]
    fn each_press_of_the_momentary_switch_steps_the_palette_once() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(PALETTE_CHANNEL, 988)]), LIMITS, 0);
        assert_eq!(
            sticks.step(&line(&[(PALETTE_CHANNEL, 2012)]), LIMITS, 10),
            vec![Move::NextPalette]
        );
        assert!(
            sticks
                .step(&line(&[(PALETTE_CHANNEL, 2012)]), LIMITS, 20)
                .is_empty(),
            "holding it down does not keep stepping"
        );
        assert!(
            sticks
                .step(&line(&[(PALETTE_CHANNEL, 988)]), LIMITS, 30)
                .is_empty()
        );
        assert_eq!(
            sticks.step(&line(&[(PALETTE_CHANNEL, 2012)]), LIMITS, 40),
            vec![Move::NextPalette]
        );
    }

    #[test]
    fn one_line_with_the_drop_switch_up_between_lines_with_it_down_drops_nothing() {
        let mut sticks = Sticks::default();
        let moves: Vec<Move> = [988, 2012, 988, 2012, 988]
            .into_iter()
            .zip((0..).step_by(10))
            .flat_map(|(drop, at)| sticks.step(&line(&[(DROP_CHANNEL, drop)]), LIMITS, at))
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
        sticks.step(&line(&[(DROP_CHANNEL, 988), (BAY_CHANNEL, 988)]), LIMITS, 0);
        assert!(
            sticks
                .step(
                    &line(&[(DROP_CHANNEL, 2012), (BAY_CHANNEL, 988)]),
                    LIMITS,
                    10
                )
                .is_empty(),
            "the first line with the switch up only arms the drop"
        );
        assert_eq!(
            sticks.step(
                &line(&[(DROP_CHANNEL, 2012), (BAY_CHANNEL, 988)]),
                LIMITS,
                20
            ),
            vec![Move::Drop { bay: 1 }],
            "the bay the payload rail has selected drops"
        );
        assert!(
            sticks
                .step(&line(&[(DROP_CHANNEL, 2012)]), LIMITS, 30)
                .is_empty()
        );
        assert!(
            sticks
                .step(&line(&[(DROP_CHANNEL, 988)]), LIMITS, 40)
                .is_empty()
        );
        sticks.bay = 4;
        sticks.step(
            &line(&[(DROP_CHANNEL, 2012), (BAY_CHANNEL, 1602)]),
            LIMITS,
            50,
        );
        assert_eq!(
            sticks.step(
                &line(&[(DROP_CHANNEL, 2012), (BAY_CHANNEL, 1602)]),
                LIMITS,
                60
            ),
            vec![Move::Drop { bay: 4 }],
            "a new selection drops next"
        );
        assert_eq!(
            [988, 1193, 1398, 1602, 1807, 2012].map(bay),
            [1, 2, 3, 4, 5, 6],
            "EdgeTX spreads a 6POS switch evenly over the channel"
        );
        assert_eq!(
            (named(3, 4), named(4, 0), named(5, 4)),
            (3, 0, 0),
            "a bay the drone does not have, or a gripper that is not per bay, drops the next bay instead"
        );
        let mut plugged_in_high = Sticks::default();
        assert!(
            plugged_in_high
                .step(&line(&[(DROP_CHANNEL, 2012)]), LIMITS, 0)
                .is_empty(),
            "a radio plugged in with the switch already up drops nothing"
        );
    }

    #[test]
    fn with_bomarm_armed_pressing_a_6pos_position_drops_that_bay_once() {
        let mut sticks = Sticks::default();
        let armed =
            |bay: u16| line(&[(ARM_CHANNEL, 2012), (BAY_CHANNEL, bay), (DROP_CHANNEL, 988)]);
        let moves: Vec<Vec<Move>> = [1398, 1398, 1398, 988, 988, 988, 1602, 1602]
            .into_iter()
            .zip((0..).step_by(10))
            .map(|(bay, at)| sticks.step(&armed(bay), LIMITS, at))
            .collect();
        assert_eq!(
            moves,
            vec![
                vec![],
                vec![],
                vec![],
                vec![],
                vec![Move::DropBay(1)],
                vec![],
                vec![],
                vec![Move::DropBay(4)],
            ],
            "arming on a set position drops nothing; a press drops its bay on the second line, once"
        );
    }

    #[test]
    fn a_6pos_press_and_the_drop_switch_together_drop_the_bay_once() {
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
        let moves: Vec<Move> = [
            (988, 988),
            (988, 988),
            (1398, 2012),
            (1398, 2012),
            (1398, 2012),
        ]
        .into_iter()
        .zip((0..).step_by(10))
        .flat_map(|((bay, drop), now)| sticks.step(&at(bay, drop), LIMITS, now))
        .collect();
        assert_eq!(moves, vec![Move::DropBay(3)]);
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
        .flat_map(|((arm, bay), now)| sticks.step(&at(arm, bay), LIMITS, now))
        .collect();
        assert!(
            moves.is_empty(),
            "a position picked while safe, or together with arming, does not drop: {moves:?}"
        );
    }

    #[test]
    fn a_pressed_bay_drops_only_where_the_drone_has_that_bay_on_a_servo() {
        assert_eq!(
            through_the_app(Move::DropBay(3), Some(4)),
            Some(Move::Drop { bay: 3 })
        );
        assert_eq!(
            through_the_app(Move::DropBay(5), Some(4)),
            None,
            "the 6POS has more positions than the drone has bays"
        );
        assert_eq!(
            through_the_app(Move::DropBay(1), Some(0)),
            None,
            "one gripper still drops from the drop switch"
        );
        assert_eq!(
            through_the_app(Move::DropBay(1), None),
            None,
            "a drone with no payload leaves the radio to the flight controller"
        );
        assert_eq!(
            through_the_app(Move::Drop { bay: 5 }, Some(4)),
            Some(Move::Drop { bay: 0 })
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
        assert!(sticks.step(&short, LIMITS, 0).is_empty());
        assert!(
            sticks
                .step(
                    &line(&[(YAW_CHANNEL, 2012), (ZOOM_CHANNEL, 2012)]),
                    LIMITS,
                    1_000
                )
                .is_empty(),
            "a channel first heard is where its control rests, not a move"
        );
        let unsent = [u16::MAX; 24];
        assert!(sticks.step(&unsent, LIMITS, 2_000).is_empty());
    }

    #[test]
    fn the_tilt_knob_points_the_gimbal_like_the_pitch_slider_and_the_last_one_moved_wins() {
        let mut sticks = Sticks::default();
        assert!(
            sticks
                .step(&line(&[(TILT_CHANNEL, 1500)]), LIMITS, 0)
                .is_empty(),
            "the first line only records where the knob is"
        );
        assert_eq!(
            sticks.step(&line(&[(TILT_CHANNEL, 988)]), LIMITS, 1000),
            vec![Move::Aim {
                pitch: -90.0,
                yaw: 0.0
            }],
            "the knob at its bottom points straight down"
        );
        assert_eq!(
            sticks.step(
                &line(&[(TILT_CHANNEL, 988), (PITCH_CHANNEL, 2012)]),
                LIMITS,
                2000
            ),
            vec![Move::Aim {
                pitch: 0.0,
                yaw: 0.0
            }],
            "the slider moving afterwards takes the gimbal with it"
        );
        assert_eq!(
            sticks.step(
                &line(&[(TILT_CHANNEL, 1500), (PITCH_CHANNEL, 2012)]),
                LIMITS,
                3000
            ),
            vec![Move::Aim {
                pitch: -45.0,
                yaw: 0.0
            }],
            "and the knob moving again takes it back"
        );
    }

    #[test]
    fn the_look_switch_points_down_when_up_and_ahead_when_down_and_rests_in_the_middle() {
        let mut sticks = Sticks::default();
        assert!(
            sticks
                .step(
                    &line(&[(LOOK_CHANNEL, 1500), (YAW_CHANNEL, 2012)]),
                    LIMITS,
                    0
                )
                .is_empty(),
            "the first line only records the switch"
        );
        assert_eq!(
            sticks.step(
                &line(&[(LOOK_CHANNEL, 2012), (YAW_CHANNEL, 2012)]),
                LIMITS,
                1000
            ),
            vec![Move::Aim {
                pitch: LOOK_DOWN_PITCH,
                yaw: 150.0
            }],
            "up points straight down, at the yaw slider's heading"
        );
        assert!(
            sticks
                .step(
                    &line(&[(LOOK_CHANNEL, 2012), (YAW_CHANNEL, 2012)]),
                    LIMITS,
                    2000
                )
                .is_empty(),
            "holding the switch sends nothing more"
        );
        assert!(
            sticks
                .step(
                    &line(&[(LOOK_CHANNEL, 1500), (YAW_CHANNEL, 2012)]),
                    LIMITS,
                    3000
                )
                .is_empty(),
            "the middle position leaves the gimbal where it is"
        );
        assert_eq!(
            sticks.step(
                &line(&[(LOOK_CHANNEL, 988), (YAW_CHANNEL, 2012)]),
                LIMITS,
                4000
            ),
            vec![Move::Aim {
                pitch: LOOK_AHEAD_PITCH,
                yaw: 150.0
            }],
            "down points ahead"
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
        };
        let mut sticks = Sticks {
            layout: moved,
            ..Sticks::default()
        };
        sticks.step(&line(&[(13, 1500), (14, 1500)]), LIMITS, 0);
        assert_eq!(
            sticks
                .step(&line(&[(13, 2012), (14, 1500)]), LIMITS, 1000)
                .len(),
            1,
            "with pitch moved to CH13 that channel steers"
        );
        assert!(
            sticks
                .step(
                    &line(&[(13, 2012), (14, 1500), (PITCH_CHANNEL, 988)]),
                    LIMITS,
                    2000
                )
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
            .flat_map(|(pwm, at)| off.step(&line(&[(DROP_CHANNEL, pwm)]), LIMITS, at))
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
            LIMITS,
            0,
        );
        let moved = line(&[
            (PITCH_CHANNEL, 988),
            (YAW_CHANNEL, 1500),
            (ZOOM_CHANNEL, 2012),
        ]);
        assert!(
            sticks.step(&moved, LIMITS, 1000).is_empty(),
            "with no gimbal or camera found yet nothing is sent"
        );
        sticks.ready = ALL_READY;
        assert_eq!(
            sticks.step(&moved, LIMITS, 2000),
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
