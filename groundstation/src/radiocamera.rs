use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

pub const PITCH: usize = 18;
pub const YAW: usize = 20;
pub const ZOOM: usize = 21;
pub const PICTURE: usize = 22;
pub const PALETTE: usize = 24;
pub const PICTURE_SETTING: &str = "IR_PIP";
pub const PALETTE_SETTING: &str = "IR_PALETTE";
pub const PICTURES: [u8; 3] = [0, 1, 3];
pub const PITCH_RANGE: (f32, f32) = (-90.0, 0.0);
pub const UNKNOWN_YAW_LIMITS: (f32, f32) = (-180.0, 180.0);
pub const STEADY_PWM: u16 = 3;
pub const RESEND_MS: u64 = 100;
const LOW_PWM: f32 = 988.0;
const SPAN_PWM: f32 = 1024.0;
const SWITCH_LOW_PWM: u16 = 1250;
const SWITCH_HIGH_PWM: u16 = 1750;
const PRESSED_PWM: u16 = 1500;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Move {
    Aim { pitch: f32, yaw: f32 },
    Zoom(f32),
    Picture(u8),
    NextPalette,
}

#[derive(Debug, Default)]
pub struct Sticks {
    acted: BTreeMap<usize, u16>,
    sent_ms: Option<u64>,
}

fn read(line: &[u16], channel: usize) -> Option<u16> {
    line.get(channel - 1)
        .copied()
        .filter(|pwm| *pwm != u16::MAX)
}

fn between((low, high): (f32, f32), pwm: u16) -> f32 {
    let share = ((f32::from(pwm) - LOW_PWM) / SPAN_PWM).clamp(0.0, 1.0);
    low + share * (high - low)
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
        let due = self
            .sent_ms
            .is_none_or(|at| now_ms.saturating_sub(at) >= RESEND_MS);
        let moved = |channel: usize| {
            read(line, channel)
                .zip(self.acted.get(&channel))
                .is_some_and(|(now, was)| now.abs_diff(*was) >= STEADY_PWM)
        };
        let aimed = read(line, PITCH)
            .zip(read(line, YAW))
            .filter(|_| due && (moved(PITCH) || moved(YAW)))
            .map(|(pitch, yaw)| Move::Aim {
                pitch: between(PITCH_RANGE, pitch),
                yaw: between(yaw_limits, yaw),
            });
        let zoomed = read(line, ZOOM)
            .filter(|_| due && moved(ZOOM))
            .map(|zoom| Move::Zoom(between((0.0, 100.0), zoom)));
        let switched = read(line, PICTURE)
            .map(position)
            .filter(|at| {
                self.acted
                    .get(&PICTURE)
                    .is_some_and(|was| position(*was) != *at)
            })
            .map(|at| Move::Picture(PICTURES[at]));
        let pressed = read(line, PALETTE).is_some_and(|pwm| pwm > PRESSED_PWM)
            && self
                .acted
                .get(&PALETTE)
                .is_some_and(|was| *was <= PRESSED_PWM);
        let refreshed: Vec<(usize, u16)> = [PITCH, YAW, ZOOM, PICTURE, PALETTE]
            .into_iter()
            .filter(|channel| {
                !self.acted.contains_key(channel)
                    || match *channel {
                        PITCH | YAW => aimed.is_some(),
                        ZOOM => zoomed.is_some(),
                        _ => true,
                    }
            })
            .filter_map(|channel| read(line, channel).map(|pwm| (channel, pwm)))
            .collect();
        self.acted.extend(refreshed);
        if aimed.is_some() || zoomed.is_some() {
            self.sent_ms = Some(now_ms);
        }
        aimed
            .into_iter()
            .chain(zoomed)
            .chain(switched)
            .chain(pressed.then_some(Move::NextPalette))
            .collect()
    }
}

static STICKS: Mutex<Sticks> = Mutex::new(Sticks {
    acted: BTreeMap::new(),
    sent_ms: None,
});

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
        .unwrap_or(UNKNOWN_YAW_LIMITS)
}

fn guided(vehicle: u8, action: Value) {
    let sent = crate::hub::lock().guided(Some(vehicle), &action, crate::hub::now_ms());
    match sent {
        Ok(frames) => frames.iter().for_each(|(link, bytes)| {
            crate::linkhost::write(&crate::linkhost::TRANSPORTS, *link, bytes);
        }),
        Err(reason) => log::debug!("the radio's camera channels could not {action}: {reason}"),
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
        ),
        Move::Zoom(percent) => guided(
            vehicle,
            json!({ "action": "camera", "op": "level", "axis": "zoom", "percent": percent }),
        ),
        Move::Picture(value) => setting(crate::camsettings::set_setting(
            &json!([PICTURE_SETTING, value]).to_string(),
        )),
        Move::NextPalette => setting(crate::camsettings::step_setting(PALETTE_SETTING, 1)),
    })
}

pub fn heard(vehicle: u8, line: &[u16]) {
    let limits = yaw_limits();
    let moves = STICKS.lock().unwrap_or_else(PoisonError::into_inner).step(
        line,
        limits,
        crate::hub::now_ms(),
    );
    steer(vehicle, &moves);
}

#[cfg(test)]
mod tests {
    use super::*;

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
            sticks.step(&line(&[(PITCH, 2012), (YAW, 1471)]), LIMITS, 0),
            vec![],
            "plugging the radio in leaves the camera where it is"
        );
        assert_eq!(
            sticks.step(&line(&[(PITCH, 2012), (YAW, 2012)]), LIMITS, 1_000),
            vec![Move::Aim {
                pitch: 0.0,
                yaw: 150.0
            }],
            "the right slider at the top looks ahead and the left one at the top turns to the gimbal's right limit"
        );
        assert_eq!(
            sticks.step(&line(&[(PITCH, 988), (YAW, 988)]), LIMITS, 2_000),
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
        sticks.step(&line(&[(YAW, 1500)]), LIMITS, 0);
        assert!(
            sticks
                .step(&line(&[(YAW, 1500 + STEADY_PWM - 1)]), LIMITS, 1_000)
                .is_empty(),
            "slider noise does not move the camera"
        );
        assert_eq!(sticks.step(&line(&[(YAW, 1600)]), LIMITS, 2_000).len(), 1);
        assert!(
            sticks
                .step(&line(&[(YAW, 1700)]), LIMITS, 2_000 + RESEND_MS - 1)
                .is_empty()
        );
        assert_eq!(
            sticks.step(&line(&[(YAW, 1700)]), LIMITS, 2_000 + RESEND_MS),
            vec![Move::Aim {
                pitch: between(PITCH_RANGE, 1500),
                yaw: between(LIMITS, 1700)
            }],
            "a move held back by the resend period is sent on the next line after it"
        );
    }

    #[test]
    fn the_zoom_knob_sets_the_zoom_level_it_points_at() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(ZOOM, 1422)]), LIMITS, 0);
        assert_eq!(
            sticks.step(&line(&[(ZOOM, 2012)]), LIMITS, 1_000),
            vec![Move::Zoom(100.0)]
        );
        assert_eq!(
            sticks.step(&line(&[(ZOOM, 988)]), LIMITS, 2_000),
            vec![Move::Zoom(0.0)]
        );
    }

    #[test]
    fn the_three_position_switch_picks_visible_the_cameras_default_or_thermal() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(PICTURE, 2012)]), LIMITS, 0);
        assert_eq!(
            sticks.step(&line(&[(PICTURE, 1500)]), LIMITS, 10),
            vec![Move::Picture(1)]
        );
        assert_eq!(
            sticks.step(&line(&[(PICTURE, 988)]), LIMITS, 20),
            vec![Move::Picture(0)]
        );
        assert!(
            sticks.step(&line(&[(PICTURE, 990)]), LIMITS, 30).is_empty(),
            "a switch left where it is changes nothing"
        );
        assert_eq!(
            sticks.step(&line(&[(PICTURE, 2012)]), LIMITS, 40),
            vec![Move::Picture(3)]
        );
    }

    #[test]
    fn each_press_of_the_momentary_switch_steps_the_palette_once() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(PALETTE, 988)]), LIMITS, 0);
        assert_eq!(
            sticks.step(&line(&[(PALETTE, 2012)]), LIMITS, 10),
            vec![Move::NextPalette]
        );
        assert!(
            sticks
                .step(&line(&[(PALETTE, 2012)]), LIMITS, 20)
                .is_empty(),
            "holding it down does not keep stepping"
        );
        assert!(sticks.step(&line(&[(PALETTE, 988)]), LIMITS, 30).is_empty());
        assert_eq!(
            sticks.step(&line(&[(PALETTE, 2012)]), LIMITS, 40),
            vec![Move::NextPalette]
        );
    }

    #[test]
    fn channels_the_radio_did_not_send_steer_nothing() {
        let mut sticks = Sticks::default();
        let short: Vec<u16> = line(&[]).iter().take(17).copied().collect();
        assert!(sticks.step(&short, LIMITS, 0).is_empty());
        assert!(
            sticks
                .step(&line(&[(YAW, 2012), (ZOOM, 2012)]), LIMITS, 1_000)
                .is_empty(),
            "a channel first heard is where its control rests, not a move"
        );
        let unsent = [u16::MAX; 24];
        assert!(sticks.step(&unsent, LIMITS, 2_000).is_empty());
    }
}
