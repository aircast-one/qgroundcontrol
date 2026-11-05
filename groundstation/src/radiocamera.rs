use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

pub const BAY: usize = 5;
pub const DROP: usize = 11;
const BAY_POSITIONS: f32 = 6.0;
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
    Drop { bay: i64 },
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
        let pressed = |channel: usize| {
            read(line, channel).is_some_and(|pwm| pwm > PRESSED_PWM)
                && self
                    .acted
                    .get(&channel)
                    .is_some_and(|was| *was <= PRESSED_PWM)
        };
        let palette = pressed(PALETTE);
        let dropped = pressed(DROP)
            .then(|| read(line, BAY).map_or(1, bay))
            .map(|bay| Move::Drop { bay });
        let refreshed: Vec<(usize, u16)> = [DROP, PITCH, YAW, ZOOM, PICTURE, PALETTE]
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
            .chain(palette.then_some(Move::NextPalette))
            .chain(dropped)
            .collect()
    }
}

static STICKS: Mutex<Sticks> = Mutex::new(Sticks {
    acted: BTreeMap::new(),
    sent_ms: None,
});

fn bays_through_the_app(vehicle: &crate::hub::Vehicle) -> Option<i64> {
    let servo_bays = vehicle.payload_servos().len() as i64;
    (vehicle.payload_setup().enabled || servo_bays > 0).then_some(servo_bays)
}

pub fn named(bay: i64, servo_bays: i64) -> i64 {
    match (1..=servo_bays).contains(&bay) {
        true => bay,
        false => 0,
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
        .unwrap_or(UNKNOWN_YAW_LIMITS)
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
        Move::Drop { bay } => guided(
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

pub fn heard(vehicle: u8, line: &[u16]) {
    let limits = yaw_limits();
    let servo_bays = crate::hub::lock().active().and_then(bays_through_the_app);
    let moves: Vec<Move> = STICKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .step(line, limits, crate::hub::now_ms())
        .into_iter()
        .filter_map(|step| match step {
            Move::Drop { bay } => servo_bays.map(|bays| Move::Drop {
                bay: named(bay, bays),
            }),
            other => Some(other),
        })
        .collect();
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
    fn flipping_the_drop_switch_up_drops_once_and_flipping_it_down_drops_nothing() {
        let mut sticks = Sticks::default();
        sticks.step(&line(&[(DROP, 988), (BAY, 988)]), LIMITS, 0);
        assert_eq!(
            sticks.step(&line(&[(DROP, 2012), (BAY, 988)]), LIMITS, 10),
            vec![Move::Drop { bay: 1 }],
            "the six-position switch at its first position picks bay 1"
        );
        assert!(sticks.step(&line(&[(DROP, 2012)]), LIMITS, 20).is_empty());
        assert!(sticks.step(&line(&[(DROP, 988)]), LIMITS, 30).is_empty());
        assert_eq!(
            sticks.step(&line(&[(DROP, 2012), (BAY, 1602)]), LIMITS, 40),
            vec![Move::Drop { bay: 4 }],
            "the fourth position picks bay 4"
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
        let mut no_six_pos = Sticks::default();
        no_six_pos.step(&[988; 11], LIMITS, 0);
        assert_eq!(
            no_six_pos.step(&[2012; 11], LIMITS, 10),
            vec![Move::Drop { bay: 6 }],
            "a model without a 6POS switch leaves CH5 wherever it rests"
        );
        let mut plugged_in_high = Sticks::default();
        assert!(
            plugged_in_high
                .step(&line(&[(DROP, 2012)]), LIMITS, 0)
                .is_empty(),
            "a radio plugged in with the switch already up drops nothing"
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
