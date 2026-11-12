use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::router::Backend;

pub const SIX_POS_VIEW: &str = "view.usbRadioSixPos";
pub const LEARN_SIX_POS: &str = "usbRadio.learnSixPos";
pub const CANCEL_SIX_POS: &str = "usbRadio.cancelSixPos";
pub const FORGET_SIX_POS: &str = "usbRadio.forgetSixPos";
pub const SIX_POS_CHANGED: &str = "core.usbRadioSixPos@changed";
pub const SIX_POS_SETTING: &str = "settings.autoConnectSettings.usbRadioSixPos";
pub const DEPS: &[&str] = &[SIX_POS_CHANGED, SIX_POS_SETTING];
pub const POSITIONS: usize = 6;
pub const OUT_OF_ORDER: &str = "The 6POS buttons were pressed out of order. Start again from button 1 and press each one in turn.";
pub const MISMATCH: &str = "This radio's 6POS does not match the one the app learned. Learn it again in Settings, Control, Radio on USB.";
const HOLD_MS: u64 = 600;
const MISMATCH_MS: u64 = 1_500;
const PLATEAU_PWM: u16 = 2;
const DISTINCT_PWM: u16 = 5;
const PORT_SEPARATOR: char = '|';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Position {
    pwm: u16,
    pressed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Learn {
    positions: Vec<Position>,
    plateau: Option<(u16, u64)>,
    first_ms: Option<u64>,
}

fn plateau(was: Option<(u16, u64)>, pwm: u16, now_ms: u64) -> (u16, u64) {
    match was {
        Some((at, since)) if at.abs_diff(pwm) <= PLATEAU_PWM => (at, since),
        _ => (pwm, now_ms),
    }
}

impl Learn {
    pub fn heard(self, pwm: u16, now_ms: u64) -> Learn {
        let first_ms = self.first_ms.unwrap_or(now_ms);
        let (at, since) = plateau(self.plateau, pwm, now_ms);
        let held = now_ms.saturating_sub(since) >= HOLD_MS;
        let new = !self
            .positions
            .iter()
            .any(|position| position.pwm.abs_diff(at) <= DISTINCT_PWM);
        let positions = match held && new && self.positions.len() < POSITIONS {
            true => [
                self.positions,
                vec![Position {
                    pwm: at,
                    pressed: since > first_ms,
                }],
            ]
            .concat(),
            false => self.positions,
        };
        Learn {
            positions,
            plateau: Some((at, since)),
            first_ms: Some(first_ms),
        }
    }

    pub fn heard_count(&self) -> usize {
        self.positions.len()
    }

    fn pressed(&self) -> Vec<u16> {
        self.positions
            .iter()
            .filter(|position| position.pressed)
            .map(|position| position.pwm)
            .collect()
    }

    pub fn out_of_order(&self) -> bool {
        let pressed = self.pressed();
        let rises: Vec<bool> = pressed.windows(2).map(|pair| pair[1] > pair[0]).collect();
        !(rises.iter().all(|rise| *rise) || rises.iter().all(|rise| !rise))
    }

    pub fn table(&self) -> Option<[u16; POSITIONS]> {
        let pressed = self.pressed();
        let rising = pressed
            .first()
            .zip(pressed.last())
            .is_none_or(|(first, last)| last >= first);
        let ascending: Vec<u16> = self
            .positions
            .iter()
            .map(|position| position.pwm)
            .collect::<BTreeSet<u16>>()
            .into_iter()
            .collect();
        let ordered: Vec<u16> = match rising {
            true => ascending,
            false => ascending.into_iter().rev().collect(),
        };
        (!self.out_of_order())
            .then(|| ordered.try_into().ok())
            .flatten()
    }
}

pub fn decode(pwm: u16, table: [u16; POSITIONS]) -> Option<i64> {
    let gap = table
        .windows(2)
        .map(|pair| pair[0].abs_diff(pair[1]))
        .min()
        .unwrap_or(0);
    table
        .iter()
        .position(|step| step.abs_diff(pwm) * 3 <= gap)
        .map(|at| at as i64 + 1)
}

pub fn off_table(
    was: Option<(u16, u64)>,
    pwm: u16,
    table: [u16; POSITIONS],
    now_ms: u64,
) -> (Option<(u16, u64)>, bool) {
    match decode(pwm, table) {
        Some(_) => (None, false),
        None => {
            let (at, since) = plateau(was, pwm, now_ms);
            (
                Some((at, since)),
                now_ms.saturating_sub(since) >= MISMATCH_MS,
            )
        }
    }
}

pub fn blocked(streaming: bool, set_bay: usize, bound_bay: usize) -> Option<String> {
    match (streaming, set_bay, bound_bay) {
        (false, ..) => Some("Connect the radio on USB to learn its 6POS.".to_string()),
        (true, 0, _) => {
            Some("The payload bay channel is off, so there is no 6POS to learn.".to_string())
        }
        (true, set, 0) => Some(format!(
            "The drone uses CH{set}, so the radio's bay switch is off and its 6POS cannot be learned."
        )),
        _ => None,
    }
}

pub fn parse(text: &str) -> Option<(String, [u16; POSITIONS])> {
    let (port, values) = text.split_once(PORT_SEPARATOR)?;
    values
        .split(',')
        .map(|value| value.trim().parse::<u16>().ok())
        .collect::<Option<Vec<u16>>>()
        .and_then(|values| values.try_into().ok())
        .map(|table| (port.to_string(), table))
}

fn text(table: [u16; POSITIONS]) -> String {
    table.map(|pwm| pwm.to_string()).join(",")
}

fn port() -> String {
    crate::usbradio::wanted().unwrap_or_default()
}

pub fn learned() -> Option<[u16; POSITIONS]> {
    crate::settingsstore::raw_setting(SIX_POS_SETTING)
        .and_then(|value| value.as_str().and_then(parse))
        .filter(|(learned_on, _)| *learned_on == port())
        .map(|(_, table)| table)
}

static LEARNING: Mutex<Option<Learn>> = Mutex::new(None);
static OFF_TABLE: Mutex<Option<(u16, u64)>> = Mutex::new(None);
static WARNED: AtomicBool = AtomicBool::new(false);
static CHANGED_FLAG: AtomicBool = AtomicBool::new(false);

pub fn take_changed() -> bool {
    CHANGED_FLAG.swap(false, Ordering::Relaxed)
}

fn bay_pwm(line: &[u16], bay_channel: usize) -> Option<u16> {
    bay_channel
        .checked_sub(1)
        .and_then(|at| line.get(at))
        .copied()
        .filter(|pwm| *pwm != u16::MAX)
}

pub fn heard(line: &[u16], bay_channel: usize, table: Option<[u16; POSITIONS]>, now_ms: u64) {
    let Some(pwm) = bay_pwm(line, bay_channel) else {
        return;
    };
    let mut learning = LEARNING.lock().unwrap_or_else(PoisonError::into_inner);
    match learning.take() {
        Some(learn) => {
            let before = learn.heard_count();
            let next = learn.heard(pwm, now_ms);
            if next.heard_count() != before {
                CHANGED_FLAG.store(true, Ordering::Relaxed);
            }
            *learning = match (next.out_of_order(), next.table()) {
                (true, _) => {
                    crate::noticeboard::post_from_vehicle(crate::noticeboard::BRIEF, OUT_OF_ORDER);
                    Some(Learn::default())
                }
                (false, Some(table)) => {
                    crate::settingsstore::set_raw(
                        SIX_POS_SETTING,
                        &json!(format!("{}{PORT_SEPARATOR}{}", port(), text(table))),
                    );
                    WARNED.store(false, Ordering::Relaxed);
                    log::info!(target: "radio", "event=six_pos_learned port={:?} table={}", port(), text(table));
                    None
                }
                (false, None) => Some(next),
            };
        }
        None => {
            let Some(table) = table else {
                return;
            };
            let mut was = OFF_TABLE.lock().unwrap_or_else(PoisonError::into_inner);
            let (next, fires) = off_table(*was, pwm, table, now_ms);
            *was = next;
            if fires && !WARNED.swap(true, Ordering::Relaxed) {
                crate::noticeboard::post_from_vehicle(crate::noticeboard::BRIEF, MISMATCH);
            }
        }
    }
}

pub fn owns(path: &str) -> bool {
    [LEARN_SIX_POS, CANCEL_SIX_POS, FORGET_SIX_POS].contains(&path)
}

fn blocked_now() -> Option<String> {
    let (set_bay, bound_bay) = crate::usbradio::bay_channels();
    blocked(crate::usbradio::streaming(), set_bay, bound_bay)
}

pub fn run(_backend: &dyn Backend, path: &str, _args: &str) -> Value {
    let refusal = (path == LEARN_SIX_POS).then(blocked_now).flatten();
    if let Some(reason) = refusal {
        return json!({ "ok": false, "reason": reason });
    }
    *LEARNING.lock().unwrap_or_else(PoisonError::into_inner) =
        (path == LEARN_SIX_POS).then(Learn::default);
    if path == FORGET_SIX_POS {
        crate::settingsstore::set_raw(SIX_POS_SETTING, &json!(""));
    }
    WARNED.store(false, Ordering::Relaxed);
    CHANGED_FLAG.store(true, Ordering::Relaxed);
    json!({ "ok": true })
}

pub fn view(_backend: &dyn Backend, _args: &[String]) -> Value {
    let learning = LEARNING
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map(Learn::heard_count);
    let table = learned();
    json!({
        "kind": "object",
        "learning": learning.is_some(),
        "heard": learning.unwrap_or(0),
        "learned": table.map(|table| table.to_vec()),
        "blocked": blocked_now(),
        "text": match (learning, table) {
            (Some(heard), _) => format!("Press each 6POS button from 1 to 6 and hold it a second. {heard} of 6 positions heard."),
            (None, Some(table)) => format!("The radio's 6POS is learned: {}.", text(table)),
            (None, None) => "The 6POS is read as EdgeTX or R2D2 send it. Learn it if the strip does not follow your buttons.".to_string(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const R2D2: [u16; POSITIONS] = [1411, 1423, 1437, 1450, 1464, 1477];

    fn learn(presses: &[(u16, u64)]) -> Learn {
        presses
            .iter()
            .fold(Learn::default(), |learn, (pwm, at)| learn.heard(*pwm, *at))
    }

    fn buttons(order: &[usize]) -> Vec<(u16, u64)> {
        order
            .iter()
            .enumerate()
            .flat_map(|(at, button)| {
                let from = 1_000 * at as u64;
                [(R2D2[button - 1], from), (R2D2[button - 1], from + HOLD_MS)]
            })
            .collect()
    }

    #[test]
    fn pressing_each_button_once_learns_the_table_in_button_order() {
        assert_eq!(
            learn(&buttons(&[6, 1, 2, 3, 4, 5])).table(),
            Some(R2D2),
            "the switch resting on 6 counts once, and buttons 1 to 5 pressed in order rise"
        );
        assert_eq!(
            learn(&buttons(&[3, 1, 2, 4, 5, 6])).table(),
            Some(R2D2),
            "resting on 3 and skipping it while pressing the rest in order"
        );
        let falling: [u16; POSITIONS] = std::array::from_fn(|at| R2D2[POSITIONS - 1 - at]);
        let reversed: Vec<(u16, u64)> = falling
            .iter()
            .enumerate()
            .flat_map(|(at, pwm)| {
                [
                    (*pwm, 1_000 * at as u64),
                    (*pwm, 1_000 * at as u64 + HOLD_MS),
                ]
            })
            .collect();
        assert_eq!(
            learn(&reversed).table(),
            Some(falling),
            "a mix that falls from button 1 to 6 keeps button 1 first"
        );
    }

    #[test]
    fn buttons_pressed_out_of_order_learn_nothing() {
        let jumbled = learn(&buttons(&[1, 6, 2, 3, 4, 5]));
        assert!(
            jumbled.out_of_order(),
            "pressing 6 before 2 to 5 would reverse the table and pick the wrong bays"
        );
        assert_eq!(jumbled.table(), None);
        assert!(
            learn(&buttons(&[1, 2, 4, 3])).out_of_order(),
            "caught as soon as the order turns, not only at the sixth button"
        );
        assert!(!learn(&buttons(&[2, 3, 4])).out_of_order());
    }

    #[test]
    fn only_a_held_new_position_counts() {
        let passing = learn(&[(1411, 0), (1450, 100), (1477, 200)]);
        assert_eq!(
            passing.heard_count(),
            0,
            "positions passed through count for nothing"
        );
        let jitter = learn(&[(1411, 0), (1412, 300), (1411, 600), (1413, 1_300)]);
        assert_eq!(
            jitter.heard_count(),
            1,
            "a couple of PWM of jitter is the same position"
        );
        assert_eq!(jitter.table(), None, "five more to go");
    }

    #[test]
    fn a_learned_table_decodes_only_its_own_steps() {
        assert_eq!(
            [1411, 1424, 1477].map(|pwm| decode(pwm, R2D2)),
            [Some(1), Some(2), Some(6)]
        );
        assert_eq!(decode(1500, R2D2), None, "off the table picks nothing");
        assert_eq!(
            parse("webserial-0|1411, 1423,1437,1450,1464,1477"),
            Some(("webserial-0".to_string(), R2D2))
        );
        assert_eq!(
            parse("1411,1423,1437,1450,1464,1477"),
            None,
            "a table names its radio"
        );
        assert_eq!(parse("webserial-0|1411,1423"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn a_radio_whose_6pos_sits_off_the_learned_table_is_called_out() {
        let (was, fires) = off_table(None, 1500, R2D2, 0);
        assert!(!fires, "one line off the table is a switch on its way");
        let (was, fires) = off_table(was, 1501, R2D2, MISMATCH_MS);
        assert!(fires, "held off every learned step, it is another radio");
        assert_eq!(off_table(was, 1411, R2D2, MISMATCH_MS + 10), (None, false));
    }

    #[test]
    fn learning_says_why_it_cannot_start() {
        assert_eq!(
            blocked(false, 5, 5).as_deref(),
            Some("Connect the radio on USB to learn its 6POS.")
        );
        assert_eq!(
            blocked(true, 0, 0).as_deref(),
            Some("The payload bay channel is off, so there is no 6POS to learn.")
        );
        assert_eq!(
            blocked(true, 5, 0).as_deref(),
            Some(
                "The drone uses CH5, so the radio's bay switch is off and its 6POS cannot be learned."
            )
        );
        assert_eq!(blocked(true, 5, 5), None);
    }
}
