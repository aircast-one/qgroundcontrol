use std::collections::BTreeMap;

pub const CMD_ACTUATOR_TEST: u16 = 310;
pub const FUNCTION_OFFSET: i64 = 1000;
const WATCHDOG_MS: u64 = 100;
const ACK_TIMEOUT_MS: u64 = 1000;
const RESULT_ACCEPTED: u8 = 0;
const RESULT_TEMPORARILY_REJECTED: u8 = 1;
const RESULT_DENIED: u8 = 2;
const RESULT_UNSUPPORTED: u8 = 3;
const RESULT_IN_PROGRESS: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    NotActive,
    Active,
    StopRequest,
    Stopping,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Channel {
    state: State,
    value: f32,
    updated_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Request {
    pub function: i64,
    pub value: f32,
    pub timeout: f32,
}

#[derive(Debug, Default)]
pub struct ActuatorTest {
    active: bool,
    channels: BTreeMap<i64, Channel>,
    current: Option<i64>,
    in_progress_until: Option<u64>,
    pub had_failure: bool,
}

impl ActuatorTest {
    pub fn set_active(&mut self, active: bool, now_ms: u64) -> Option<Request> {
        let next = match active {
            true => None,
            false => self.stop(None, now_ms),
        };
        self.active = active;
        next
    }

    pub fn set(&mut self, function: i64, value: f32, now_ms: u64) -> Option<Request> {
        if !self.active {
            return None;
        }
        self.channels.insert(function, Channel { state: State::Active, value, updated_ms: now_ms });
        self.send_next(now_ms)
    }

    pub fn stop(&mut self, function: Option<i64>, now_ms: u64) -> Option<Request> {
        self.channels
            .iter_mut()
            .filter(|(f, c)| function.is_none_or(|wanted| **f == wanted) && c.state == State::Active)
            .for_each(|(_, c)| c.state = State::StopRequest);
        self.send_next(now_ms)
    }

    pub fn tick(&mut self, now_ms: u64) -> Option<Request> {
        self.channels
            .values_mut()
            .filter(|c| c.state == State::Active && now_ms.saturating_sub(c.updated_ms) > WATCHDOG_MS)
            .for_each(|c| c.state = State::StopRequest);
        if self.in_progress_until.is_some_and(|until| now_ms >= until) {
            self.in_progress_until = None;
        }
        self.send_next(now_ms)
    }

    pub fn on_ack(&mut self, result: u8, now_ms: u64) -> (Option<Request>, Option<&'static str>) {
        if result == RESULT_IN_PROGRESS {
            self.in_progress_until = Some(now_ms + ACK_TIMEOUT_MS);
            return (None, None);
        }
        self.in_progress_until = None;
        let current = self.current.and_then(|f| self.channels.get_mut(&f));
        let message = match result {
            RESULT_ACCEPTED => {
                if let Some(channel) = current.filter(|c| c.state == State::Stopping) {
                    channel.state = State::NotActive;
                }
                None
            }
            failure => {
                if let Some(channel) = current {
                    channel.state = State::NotActive;
                }
                let first = !self.had_failure;
                self.had_failure = true;
                first.then_some(match failure {
                    RESULT_TEMPORARILY_REJECTED => "Actuator test command temporarily rejected",
                    RESULT_DENIED => "Actuator test command denied",
                    RESULT_UNSUPPORTED => "Actuator test command not supported",
                    _ => "Actuator test command failed",
                })
            }
        };
        (self.send_next(now_ms), message)
    }

    fn send_next(&mut self, now_ms: u64) -> Option<Request> {
        if self.in_progress_until.is_some() || self.channels.is_empty() {
            return None;
        }
        let order: Vec<i64> = self.channels.keys().copied().collect();
        let start = self.current.and_then(|f| order.iter().position(|o| *o == f)).map_or(0, |i| i + 1);
        let next = (0..order.len()).map(|step| order[(start + step) % order.len()]).find(|f| matches!(self.channels[f].state, State::Active | State::StopRequest))?;
        self.current = Some(next);
        let channel = self.channels.get_mut(&next)?;
        let request = match channel.state {
            State::Active => Request { function: next, value: channel.value, timeout: 1.0 },
            _ => {
                channel.state = State::Stopping;
                Request { function: next, value: f32::NAN, timeout: 0.0 }
            }
        };
        self.in_progress_until = Some(now_ms + ACK_TIMEOUT_MS);
        Some(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_moves_until_the_sliders_are_enabled_and_one_command_flies_at_a_time() {
        let mut test = ActuatorTest::default();
        assert_eq!(test.set(101, 0.2, 0), None, "the safety switch is off");
        test.set_active(true, 0);
        assert_eq!(test.set(101, 0.2, 0), Some(Request { function: 101, value: 0.2, timeout: 1.0 }));
        assert_eq!(test.set(102, 0.3, 10), None, "the first command is still unacknowledged");
        assert_eq!(test.on_ack(RESULT_ACCEPTED, 20).0, Some(Request { function: 102, value: 0.3, timeout: 1.0 }), "acks cycle through the active actuators");
        assert_eq!(test.on_ack(RESULT_ACCEPTED, 30).0, Some(Request { function: 101, value: 0.2, timeout: 1.0 }));
    }

    #[test]
    fn a_slider_not_refreshed_within_100_ms_is_stopped_with_nan() {
        let mut test = ActuatorTest::default();
        test.set_active(true, 0);
        test.set(101, 0.5, 0);
        assert_eq!(test.on_ack(RESULT_ACCEPTED, 10).0.map(|r| r.function), Some(101), "an active actuator is sent again as soon as it is acknowledged");
        assert_eq!(test.tick(150), None, "the watchdog marks it, and the stop waits for the command in flight");
        let stopped = test.on_ack(RESULT_ACCEPTED, 160).0.expect("a stop request");
        assert_eq!((stopped.function, stopped.value.is_nan(), stopped.timeout), (101, true, 0.0));
        assert_eq!(test.on_ack(RESULT_ACCEPTED, 170).0, None, "stopped and acknowledged, so nothing more is sent");
    }

    #[test]
    fn a_rejection_is_reported_once_and_switching_off_stops_everything() {
        let mut test = ActuatorTest::default();
        test.set_active(true, 0);
        test.set(101, 0.5, 0);
        assert_eq!(test.on_ack(RESULT_IN_PROGRESS, 5), (None, None), "IN_PROGRESS keeps waiting for the final answer, as MavCommandQueue does");
        assert!(!test.had_failure);
        assert_eq!(test.on_ack(RESULT_DENIED, 10).1, Some("Actuator test command denied"));
        test.set(102, 0.5, 20);
        assert_eq!(test.on_ack(RESULT_UNSUPPORTED, 30).1, None, "only the first failure is announced");
        assert!(test.had_failure);
        let mut on = ActuatorTest::default();
        on.set_active(true, 0);
        on.set(201, 0.1, 0);
        on.on_ack(RESULT_ACCEPTED, 5);
        assert_eq!(on.set_active(false, 10), None);
        let off = on.on_ack(RESULT_ACCEPTED, 15).0.expect("a stop");
        assert!(off.value.is_nan());
        assert_eq!(on.tick(2000), None, "unacknowledged stop times out but nothing is active");
    }
}
