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
const HOLD_MS: u64 = 600;
const PLATEAU_PWM: u16 = 2;
const DISTINCT_PWM: u16 = 5;

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

impl Learn {
    pub fn heard(self, pwm: u16, now_ms: u64) -> Learn {
        let first_ms = self.first_ms.unwrap_or(now_ms);
        let (at, since) = match self.plateau {
            Some((at, since)) if at.abs_diff(pwm) <= PLATEAU_PWM => (at, since),
            _ => (pwm, now_ms),
        };
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

    pub fn table(&self) -> Option<[u16; POSITIONS]> {
        let pressed: Vec<u16> = self
            .positions
            .iter()
            .filter(|position| position.pressed)
            .map(|position| position.pwm)
            .collect();
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
        ordered.try_into().ok()
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

pub fn parse(text: &str) -> Option<[u16; POSITIONS]> {
    text.split(',')
        .map(|value| value.trim().parse::<u16>().ok())
        .collect::<Option<Vec<u16>>>()
        .and_then(|values| values.try_into().ok())
}

fn text(table: [u16; POSITIONS]) -> String {
    table.map(|pwm| pwm.to_string()).join(",")
}

pub fn learned() -> Option<[u16; POSITIONS]> {
    crate::settingsstore::raw_setting(SIX_POS_SETTING)
        .and_then(|value| value.as_str().and_then(parse))
}

static LEARNING: Mutex<Option<Learn>> = Mutex::new(None);
static CHANGED_FLAG: AtomicBool = AtomicBool::new(false);

pub fn take_changed() -> bool {
    CHANGED_FLAG.swap(false, Ordering::Relaxed)
}

pub fn heard(line: &[u16], bay_channel: usize, now_ms: u64) {
    let Some(pwm) = bay_channel
        .checked_sub(1)
        .and_then(|at| line.get(at))
        .copied()
        .filter(|pwm| *pwm != u16::MAX)
    else {
        return;
    };
    let mut learning = LEARNING.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(learn) = learning.take() else {
        return;
    };
    let before = learn.heard_count();
    let next = learn.heard(pwm, now_ms);
    if next.heard_count() != before {
        CHANGED_FLAG.store(true, Ordering::Relaxed);
    }
    match next.table() {
        Some(table) => {
            crate::settingsstore::set_raw(SIX_POS_SETTING, &json!(text(table)));
            log::info!(target: "radio", "learned the 6POS: {}", text(table));
        }
        None => *learning = Some(next),
    }
}

pub fn owns(path: &str) -> bool {
    [LEARN_SIX_POS, CANCEL_SIX_POS, FORGET_SIX_POS].contains(&path)
}

pub fn run(_backend: &dyn Backend, path: &str, _args: &str) -> Value {
    *LEARNING.lock().unwrap_or_else(PoisonError::into_inner) =
        (path == LEARN_SIX_POS).then(Learn::default);
    if path == FORGET_SIX_POS {
        crate::settingsstore::set_raw(SIX_POS_SETTING, &json!(""));
    }
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

    fn learn(presses: &[(u16, u64)]) -> Learn {
        presses
            .iter()
            .fold(Learn::default(), |learn, (pwm, at)| learn.heard(*pwm, *at))
    }

    fn held(pwm: u16, from: u64) -> [(u16, u64); 2] {
        [(pwm, from), (pwm, from + HOLD_MS)]
    }

    #[test]
    fn pressing_each_button_once_learns_the_table_in_button_order() {
        let r2d2 = [1411u16, 1423, 1437, 1450, 1464, 1477];
        let from_rest_on_six: Vec<(u16, u64)> = std::iter::once(held(1477, 0))
            .chain(
                r2d2[..5]
                    .iter()
                    .enumerate()
                    .map(|(at, pwm)| held(*pwm, 1_000 * (at as u64 + 1))),
            )
            .flatten()
            .collect();
        assert_eq!(
            learn(&from_rest_on_six).table(),
            Some(r2d2),
            "the switch resting on 6 counts once, and buttons 1 to 5 pressed in order rise"
        );
        let reversed: Vec<(u16, u64)> = r2d2
            .iter()
            .rev()
            .enumerate()
            .flat_map(|(at, pwm)| held(*pwm, 1_000 * (at as u64 + 1)))
            .collect();
        assert_eq!(
            learn(&reversed).table(),
            Some([1477, 1464, 1450, 1437, 1423, 1411]),
            "a mix that falls from button 1 to 6 keeps button 1 first"
        );
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
        let table = [1411u16, 1423, 1437, 1450, 1464, 1477];
        assert_eq!(
            [1411, 1424, 1477].map(|pwm| decode(pwm, table)),
            [Some(1), Some(2), Some(6)]
        );
        assert_eq!(decode(1500, table), None, "off the table picks nothing");
        assert_eq!(parse("1411, 1423,1437,1450,1464,1477"), Some(table));
        assert_eq!(parse("1411,1423"), None);
        assert_eq!(parse(""), None);
    }
}
