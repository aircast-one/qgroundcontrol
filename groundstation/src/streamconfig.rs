pub const MSG_ATTITUDE_QUATERNION: u32 = 31;
pub const MSG_ATTITUDE_TARGET: u32 = 83;
pub const MSG_LOCAL_POSITION_NED: u32 = 32;
pub const MSG_POSITION_TARGET_LOCAL_NED: u32 = 85;
pub const MSG_NAV_CONTROLLER_OUTPUT: u32 = 62;
pub const MSG_VFR_HUD: u32 = 74;
const HIGH_RATE_INTERVAL_US: i32 = 10_000;
const DEFAULT_RATE: i32 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Disabled,
    RateAndAttitude,
    VelocityAndPosition,
    AltitudeAndAirspeed,
}

impl Mode {
    pub fn from_index(index: i64) -> Option<Mode> {
        [Mode::Disabled, Mode::RateAndAttitude, Mode::VelocityAndPosition, Mode::AltitudeAndAirspeed].get(usize::try_from(index).ok()?).copied()
    }

    fn rates(self) -> Vec<(u32, i32)> {
        match self {
            Mode::Disabled => Vec::new(),
            Mode::RateAndAttitude => vec![(MSG_ATTITUDE_QUATERNION, HIGH_RATE_INTERVAL_US), (MSG_ATTITUDE_TARGET, HIGH_RATE_INTERVAL_US)],
            Mode::VelocityAndPosition => vec![(MSG_LOCAL_POSITION_NED, HIGH_RATE_INTERVAL_US), (MSG_POSITION_TARGET_LOCAL_NED, HIGH_RATE_INTERVAL_US)],
            Mode::AltitudeAndAirspeed => vec![(MSG_NAV_CONTROLLER_OUTPUT, HIGH_RATE_INTERVAL_US), (MSG_VFR_HUD, HIGH_RATE_INTERVAL_US)],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum State {
    #[default]
    Idle,
    Configuring,
    RestoringDefaults,
}

#[derive(Debug, Default)]
pub struct StreamConfig {
    state: State,
    next_state: State,
    desired: Vec<(u32, i32)>,
    next_desired: Vec<(u32, i32)>,
    changed: Vec<u32>,
}

pub type Interval = (u32, i32);

impl StreamConfig {
    pub fn set_mode(&mut self, mode: Mode) -> Option<Interval> {
        match mode {
            Mode::Disabled => self.set_next_state(State::RestoringDefaults),
            _ => {
                self.next_desired = mode.rates();
                self.set_next_state(State::Configuring)
            }
        }
    }

    pub fn got_ack(&mut self) -> Option<Interval> {
        match self.state {
            State::Configuring => self.next_desired_rate(),
            State::RestoringDefaults => self.restore_next_default(),
            State::Idle => None,
        }
    }

    fn set_next_state(&mut self, state: State) -> Option<Interval> {
        self.next_state = state;
        if self.state == State::Idle {
            self.state = State::RestoringDefaults;
            return self.restore_next_default();
        }
        None
    }

    fn next_desired_rate(&mut self) -> Option<Interval> {
        if self.next_state != State::Idle {
            self.desired.clear();
            self.state = State::RestoringDefaults;
            return self.restore_next_default();
        }
        let Some(rate) = self.desired.pop() else {
            self.state = State::Idle;
            return None;
        };
        self.changed.push(rate.0);
        Some(rate)
    }

    fn restore_next_default(&mut self) -> Option<Interval> {
        let Some(id) = self.changed.pop() else {
            match self.next_state {
                State::Configuring => {
                    self.state = State::Configuring;
                    self.desired = std::mem::take(&mut self.next_desired);
                    self.next_state = State::Idle;
                    return self.next_desired_rate();
                }
                State::RestoringDefaults | State::Idle => {
                    self.next_state = State::Idle;
                    self.state = State::Idle;
                    return None;
                }
            }
        };
        Some((id, DEFAULT_RATE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates_go_out_one_ack_at_a_time_and_are_restored_in_reverse() {
        let mut config = StreamConfig::default();
        assert_eq!(config.set_mode(Mode::RateAndAttitude), Some((MSG_ATTITUDE_TARGET, 10_000)), "the last desired rate goes first, as _desiredRates.last()");
        assert_eq!(config.got_ack(), Some((MSG_ATTITUDE_QUATERNION, 10_000)));
        assert_eq!(config.got_ack(), None, "configured, so idle");
        assert_eq!(config.set_mode(Mode::Disabled), Some((MSG_ATTITUDE_QUATERNION, 0)), "defaults come back last changed first");
        assert_eq!(config.got_ack(), Some((MSG_ATTITUDE_TARGET, 0)));
        assert_eq!(config.got_ack(), None);
        assert_eq!(config.got_ack(), None, "an ack while idle sends nothing");
    }

    #[test]
    fn a_new_mode_mid_configuration_restores_first_and_then_applies() {
        let mut config = StreamConfig::default();
        config.set_mode(Mode::RateAndAttitude);
        assert_eq!(config.set_mode(Mode::VelocityAndPosition), None, "a request while busy waits for the next ack");
        assert_eq!(config.got_ack(), Some((MSG_ATTITUDE_TARGET, 0)), "the half-done configuration is undone");
        assert_eq!(config.got_ack(), Some((MSG_POSITION_TARGET_LOCAL_NED, 10_000)));
        assert_eq!(config.got_ack(), Some((MSG_LOCAL_POSITION_NED, 10_000)));
        assert_eq!(config.got_ack(), None);
        assert_eq!(Mode::from_index(2), Some(Mode::VelocityAndPosition));
        assert_eq!(Mode::from_index(9), None);
    }
}
