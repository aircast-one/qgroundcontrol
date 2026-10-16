use std::time::Duration;
use web_time::Instant;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lag {
    pub dead: Duration,
    pub rising: Duration,
    pub falling: Duration,
}

pub const LOITER_YAW: Lag = Lag { dead: Duration::from_millis(50), rising: Duration::from_millis(120), falling: Duration::from_millis(120) };
pub const ACRO_YAW: Lag = Lag { dead: Duration::from_millis(40), rising: Duration::from_millis(115), falling: Duration::from_millis(115) };
pub const ACRO_TILT: Lag = Lag { dead: Duration::from_millis(30), rising: Duration::from_millis(25), falling: Duration::from_millis(25) };
pub const LOITER_SPEED: Lag = Lag { dead: Duration::from_millis(150), rising: Duration::from_millis(1_500), falling: Duration::from_millis(500) };
pub const CLIMB: Lag = Lag { dead: Duration::from_millis(150), rising: Duration::from_millis(600), falling: Duration::from_millis(600) };

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub at: Instant,
    pub value: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Predicted {
    pub value: f64,
    pub travelled: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gain {
    pub full: f64,
    pub expo: f64,
    pub deadzone: f64,
}

impl Gain {
    pub fn commanded(&self, stick: f64) -> f64 {
        let past = (stick.abs() - self.deadzone).max(0.0) / (1.0 - self.deadzone).max(f64::EPSILON);
        let shaped = (1.0 - self.expo) * past + self.expo * past.powi(3);
        self.full * shaped.min(1.0) * stick.signum()
    }
}

fn settled(start: f64, command: Option<f64>, span: Duration, lag: Lag) -> Predicted {
    let seconds = span.as_secs_f64();
    match command {
        None => Predicted { value: start, travelled: start * seconds },
        Some(command) => {
            let constant = match command.abs() > start.abs() {
                true => lag.rising,
                false => lag.falling,
            }
            .as_secs_f64()
            .max(f64::EPSILON);
            let left = (-seconds / constant).exp();
            Predicted { value: command + (start - command) * left, travelled: command * seconds + (start - command) * constant * (1.0 - left) }
        }
    }
}

pub fn predicted(start: f64, from: Instant, until: Instant, steps: &[Step], delay: Duration, lag: Lag) -> Predicted {
    let effect = |step: &Step| step.at + delay;
    let commanded = |at: Instant| steps.iter().rev().find(|step| effect(step) <= at).map(|step| step.value);
    let edges: Vec<Instant> = std::iter::once(from).chain(steps.iter().map(effect).filter(|at| *at > from && *at < until)).chain(std::iter::once(until.max(from))).collect();
    edges.windows(2).fold(Predicted { value: start, travelled: 0.0 }, |sofar, edge| {
        let leg = settled(sofar.value, commanded(edge[0]), edge[1].saturating_duration_since(edge[0]), lag);
        Predicted { value: leg.value, travelled: sofar.travelled + leg.travelled }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAW: Lag = Lag { dead: Duration::ZERO, rising: Duration::from_millis(100), falling: Duration::from_millis(100) };

    #[test]
    fn without_a_command_the_reading_carries_on_at_its_rate() {
        let start = Instant::now();
        let ahead = predicted(30.0, start, start + Duration::from_millis(200), &[], Duration::from_millis(80), YAW);
        assert!((ahead.value - 30.0).abs() < 1e-9 && (ahead.travelled - 6.0).abs() < 1e-9, "{ahead:?}");
    }

    #[test]
    fn a_stick_pushed_now_turns_the_prediction_from_the_first_frame() {
        let start = Instant::now();
        let pushed = [Step { at: start, value: 0.0 }, Step { at: start + Duration::from_millis(100), value: 60.0 }];
        let delay = Duration::from_millis(80);
        let display = Duration::from_millis(25);
        let first_frame = predicted(0.0, start + Duration::from_millis(100), start + Duration::from_millis(100) + delay + display, &pushed, delay, YAW);
        assert!(first_frame.value > 10.0 && first_frame.travelled > 0.0, "the view starts the turn the drone is about to make on the very next frame: {first_frame:?}");
    }

    #[test]
    fn letting_go_does_not_swing_the_view_back() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let delay = Duration::from_millis(80);
        let held = [Step { at: at(0), value: 60.0 }];
        let released = [Step { at: at(0), value: 60.0 }, Step { at: at(500), value: 0.0 }];
        let before = predicted(60.0, at(450), at(500) + delay, &held, delay, YAW);
        let after = predicted(60.0, at(450), at(501) + delay, &released, delay, YAW);
        assert!(after.travelled >= before.travelled, "the drone keeps turning until the release reaches it, so the view never steps backwards: {before:?} {after:?}");
        assert!(after.value < before.value + 1e-6);
    }

    #[test]
    fn a_steady_command_is_drawn_without_lag_once_the_drone_matches_it() {
        let start = Instant::now();
        let steady = [Step { at: start, value: 45.0 }];
        let ahead = predicted(45.0, start + Duration::from_secs(1), start + Duration::from_millis(1_200), &steady, Duration::from_millis(80), YAW);
        assert!((ahead.value - 45.0).abs() < 1e-9 && (ahead.travelled - 9.0).abs() < 1e-9, "{ahead:?}");
    }

    #[test]
    fn a_gain_maps_the_stick_through_deadzone_and_expo() {
        let yaw = Gain { full: 150.0, expo: 0.0, deadzone: 0.04 };
        assert_eq!(yaw.commanded(0.03), 0.0, "inside the deadzone nothing is asked");
        assert!((yaw.commanded(1.0) - 150.0).abs() < 1e-9 && (yaw.commanded(-1.0) + 150.0).abs() < 1e-9);
        let acro = Gain { full: 360.0, expo: 0.3, deadzone: 0.0 };
        assert!((acro.commanded(0.5) - 360.0 * (0.7 * 0.5 + 0.3 * 0.125)).abs() < 1e-9, "ArduPilot's input_expo");
    }
}
