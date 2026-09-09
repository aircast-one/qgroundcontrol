use std::collections::BTreeMap;

pub const COMP_ID_ALL: u8 = 0;
pub const MAX_RETRY: u32 = 3;
pub const ACK_TIMEOUT_MS: u64 = 3000;
pub const ACK_TIMEOUT_HIGH_LATENCY_MS: u64 = 120_000;
pub const MESSAGE_WAIT_MS: u64 = 1000;
pub const RESULT_ACCEPTED: u8 = 0;
pub const RESULT_FAILED: u8 = 4;
pub const RESULT_IN_PROGRESS: u8 = 5;
pub const CMD_DO_MOTOR_TEST: u16 = 209;
pub const CMD_DO_AUTOTUNE_ENABLE: u16 = 212;
pub const CMD_PREFLIGHT_STORAGE: u16 = 245;
pub const CMD_RUN_PREARM_CHECKS: u16 = 401;
pub const CMD_START_RX_PAIR: u16 = 500;
pub const CMD_SET_MESSAGE_INTERVAL: u16 = 511;
pub const CMD_REQUEST_MESSAGE: u16 = 512;
pub const CMD_REQUEST_PROTOCOL_VERSION: u16 = 519;
pub const CMD_REQUEST_AUTOPILOT_CAPABILITIES: u16 = 520;
pub const FRAME_MISSION: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    ResultOnly,
    NoResponse,
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestFailure {
    None,
    CommandError,
    CommandNotAcked,
    MessageNotReceived,
    DuplicateCommand,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub component: u8,
    pub command: u16,
    pub command_int: bool,
    pub frame: u8,
    pub params: [f64; 7],
    pub show_error: bool,
    pub tag: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    Send { component: u8, command: u16, command_int: bool, frame: u8, params: [f64; 7], x: i32, y: i32 },
    Result { tag: u64, component: u8, command: u16, result: u8, failure: Failure },
    Progress { tag: u64, component: u8, command: u16 },
    ShowError(String),
    RequestResult { tag: u64, component: u8, message_id: u32, result: u8, failure: RequestFailure },
}

#[derive(Debug, Clone)]
struct Entry {
    command: Command,
    tries: u32,
    max_tries: u32,
    ack_timeout_ms: u64,
    sent_at_ms: u64,
}

#[derive(Debug, Clone)]
struct Request {
    tag: u64,
    ack_received: bool,
    message_received: bool,
    wait_started_ms: Option<u64>,
}

#[derive(Debug, Default)]
pub struct Commands {
    entries: Vec<Entry>,
    requests: BTreeMap<(u8, u32), Request>,
    pub px4: bool,
    pub high_latency: bool,
}

fn retries(command: u16) -> bool {
    matches!(command, CMD_REQUEST_AUTOPILOT_CAPABILITIES | CMD_REQUEST_PROTOCOL_VERSION | CMD_REQUEST_MESSAGE | CMD_PREFLIGHT_STORAGE | CMD_RUN_PREARM_CHECKS)
}

fn can_duplicate(command: u16) -> bool {
    matches!(command, CMD_DO_MOTOR_TEST | CMD_SET_MESSAGE_INTERVAL)
}

fn result_text(command: u16, result: u8) -> Option<String> {
    let verb = match result {
        1 => "temporarily rejected",
        2 => "denied",
        3 => "not supported",
        4 => "failed",
        _ => return None,
    };
    Some(format!("MAV_CMD {command} command {verb}"))
}

impl Commands {
    pub fn pending(&self, component: u8, command: u16) -> bool {
        self.entries.iter().any(|e| e.command.component == component && e.command.command == command)
    }

    fn index(&self, component: u8, command: u16) -> Option<usize> {
        self.entries.iter().position(|e| e.command.component == component && e.command.command == command)
    }

    pub fn send(&mut self, command: Command, now_ms: u64) -> Vec<Out> {
        let all = command.component == COMP_ID_ALL;
        if all || (self.pending(command.component, command.command) && !can_duplicate(command.command)) {
            let failure = if all { Failure::ResultOnly } else { Failure::Duplicate };
            let mut out = vec![Out::Result { tag: command.tag, component: command.component, command: command.command, result: RESULT_FAILED, failure }];
            if command.show_error {
                out.push(Out::ShowError(format!("Unable to send command: {}.", if all { "Internal error - MAV_COMP_ID_ALL not supported" } else { "Waiting on previous response to same command." })));
            }
            return out;
        }
        let max_tries = if retries(command.command) { MAX_RETRY } else { 1 };
        let ack_timeout_ms = if self.high_latency { ACK_TIMEOUT_HIGH_LATENCY_MS } else { ACK_TIMEOUT_MS };
        self.entries.push(Entry { command, tries: 0, max_tries, ack_timeout_ms, sent_at_ms: now_ms });
        let index = self.entries.len() - 1;
        self.transmit(index, now_ms)
    }

    fn transmit(&mut self, index: usize, now_ms: u64) -> Vec<Out> {
        let entry = &mut self.entries[index];
        entry.tries += 1;
        entry.sent_at_ms = now_ms;
        if entry.tries > entry.max_tries {
            let entry = self.entries.remove(index);
            let c = entry.command;
            let mut out = vec![Out::Result { tag: c.tag, component: c.component, command: c.command, result: RESULT_FAILED, failure: Failure::NoResponse }];
            if c.show_error {
                out.push(Out::ShowError(format!("Vehicle did not respond to command: MAV_CMD {}", c.command)));
            }
            return out;
        }
        if entry.tries > 1 && !self.px4 && entry.command.command == CMD_START_RX_PAIR {
            return Vec::new();
        }
        let c = &entry.command;
        let scale = |v: f64| if c.frame == FRAME_MISSION { v as i32 } else { (v * 1e7) as i32 };
        vec![Out::Send { component: c.component, command: c.command, command_int: c.command_int, frame: c.frame, params: c.params, x: scale(c.params[4]), y: scale(c.params[5]) }]
    }

    pub fn tick(&mut self, now_ms: u64) -> Vec<Out> {
        let due: Vec<usize> = (0..self.entries.len()).rev().filter(|i| now_ms.saturating_sub(self.entries[*i].sent_at_ms) > self.entries[*i].ack_timeout_ms).collect();
        let mut out: Vec<Out> = due.into_iter().flat_map(|i| self.transmit(i, now_ms)).collect();
        let expired = self.requests.iter().find(|(_, r)| r.wait_started_ms.is_some_and(|started| now_ms.saturating_sub(started) > MESSAGE_WAIT_MS)).map(|(k, r)| (*k, r.tag));
        if let Some(((component, message_id), tag)) = expired {
            self.requests.remove(&(component, message_id));
            out.push(Out::RequestResult { tag, component, message_id, result: RESULT_FAILED, failure: RequestFailure::MessageNotReceived });
        }
        out
    }

    pub fn on_ack(&mut self, component: u8, command: u16, result: u8, now_ms: u64) -> Vec<Out> {
        let Some(index) = self.index(component, command) else { return Vec::new() };
        if result == RESULT_IN_PROGRESS {
            let entry = if self.px4 && command == CMD_DO_AUTOTUNE_ENABLE {
                self.entries.remove(index)
            } else {
                let entry = &mut self.entries[index];
                entry.max_tries = 1;
                entry.sent_at_ms = now_ms;
                entry.clone()
            };
            return vec![Out::Progress { tag: entry.command.tag, component, command }];
        }
        let entry = self.entries.remove(index);
        let c = entry.command;
        let mut out = vec![Out::Result { tag: c.tag, component, command, result, failure: Failure::ResultOnly }];
        if c.show_error {
            if let Some(text) = result_text(command, result) {
                out.push(Out::ShowError(text));
            }
        }
        if command == CMD_REQUEST_MESSAGE {
            out.extend(self.request_acked(component, c.params[0] as u32, result, now_ms, c.tag));
        }
        out
    }

    pub fn request_message(&mut self, tag: u64, component: u8, message_id: u32, params: [f64; 5], now_ms: u64) -> Vec<Out> {
        self.requests.insert((component, message_id), Request { tag, ack_received: false, message_received: false, wait_started_ms: None });
        let command = Command { component, command: CMD_REQUEST_MESSAGE, command_int: false, frame: 0, params: [message_id as f64, params[0], params[1], params[2], params[3], params[4], 0.0], show_error: false, tag };
        let out = self.send(command, now_ms);
        out.into_iter()
            .flat_map(|o| match o {
                Out::Result { failure, result, .. } => {
                    self.requests.remove(&(component, message_id));
                    let failure = if failure == Failure::Duplicate { RequestFailure::DuplicateCommand } else { RequestFailure::CommandError };
                    vec![Out::RequestResult { tag, component, message_id, result, failure }]
                }
                other => vec![other],
            })
            .collect()
    }

    fn request_acked(&mut self, component: u8, message_id: u32, result: u8, now_ms: u64, tag: u64) -> Vec<Out> {
        let Some(request) = self.requests.get_mut(&(component, message_id)) else { return Vec::new() };
        request.ack_received = true;
        if result != RESULT_ACCEPTED {
            self.requests.remove(&(component, message_id));
            return vec![Out::RequestResult { tag, component, message_id, result, failure: RequestFailure::CommandError }];
        }
        if request.message_received {
            self.requests.remove(&(component, message_id));
            return vec![Out::RequestResult { tag, component, message_id, result: RESULT_ACCEPTED, failure: RequestFailure::None }];
        }
        request.wait_started_ms = Some(now_ms);
        Vec::new()
    }

    pub fn on_message(&mut self, component: u8, message_id: u32) -> Vec<Out> {
        let Some(request) = self.requests.get(&(component, message_id)).cloned() else { return Vec::new() };
        if !request.ack_received {
            if let Some(index) = self.index(component, CMD_REQUEST_MESSAGE) {
                self.entries.remove(index);
            }
        }
        self.requests.remove(&(component, message_id));
        vec![Out::RequestResult { tag: request.tag, component, message_id, result: RESULT_ACCEPTED, failure: RequestFailure::None }]
    }

    fn no_response(&mut self, component: u8, message_id: u32) -> Vec<Out> {
        self.requests.remove(&(component, message_id)).map(|r| vec![Out::RequestResult { tag: r.tag, component, message_id, result: RESULT_FAILED, failure: RequestFailure::CommandNotAcked }]).unwrap_or_default()
    }

    pub fn resolve(&mut self, out: Vec<Out>) -> Vec<Out> {
        out.into_iter()
            .flat_map(|o| match o {
                Out::Result { component, command: CMD_REQUEST_MESSAGE, failure: Failure::NoResponse, tag, .. } => {
                    let message_id = self.requests.iter().find(|(k, r)| k.0 == component && r.tag == tag).map(|(k, _)| k.1);
                    let mut all = vec![Out::Result { tag, component, command: CMD_REQUEST_MESSAGE, result: RESULT_FAILED, failure: Failure::NoResponse }];
                    if let Some(message_id) = message_id {
                        all.extend(self.no_response(component, message_id));
                    }
                    all
                }
                other => vec![other],
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arm(tag: u64) -> Command {
        Command { component: 1, command: 400, command_int: false, frame: 0, params: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], show_error: true, tag }
    }

    #[test]
    fn a_plain_command_is_sent_once_and_its_ack_reports_the_result() {
        let mut commands = Commands::default();
        let sent = commands.send(arm(7), 0);
        assert!(matches!(sent.as_slice(), [Out::Send { component: 1, command: 400, command_int: false, .. }]));
        assert!(commands.pending(1, 400));
        assert_eq!(commands.on_ack(1, 400, 2, 10), vec![Out::Result { tag: 7, component: 1, command: 400, result: 2, failure: Failure::ResultOnly }, Out::ShowError("MAV_CMD 400 command denied".into())]);
        assert!(!commands.pending(1, 400));
        assert!(commands.on_ack(1, 400, 0, 10).is_empty());
        let unretried = commands.send(arm(8), 100);
        assert_eq!(unretried.len(), 1);
        assert_eq!(commands.tick(100 + ACK_TIMEOUT_MS + 1), vec![Out::Result { tag: 8, component: 1, command: 400, result: RESULT_FAILED, failure: Failure::NoResponse }, Out::ShowError("Vehicle did not respond to command: MAV_CMD 400".into())]);
    }

    #[test]
    fn duplicates_and_component_all_are_refused_unless_the_command_allows_it() {
        let mut commands = Commands::default();
        commands.send(arm(1), 0);
        let duplicate = commands.send(arm(2), 1);
        assert_eq!(duplicate[0], Out::Result { tag: 2, component: 1, command: 400, result: RESULT_FAILED, failure: Failure::Duplicate });
        let all = commands.send(Command { component: COMP_ID_ALL, ..arm(3) }, 1);
        assert_eq!(all[0], Out::Result { tag: 3, component: 0, command: 400, result: RESULT_FAILED, failure: Failure::ResultOnly });
        let motor = Command { command: CMD_DO_MOTOR_TEST, ..arm(4) };
        commands.send(motor.clone(), 2);
        assert!(matches!(commands.send(motor, 3).as_slice(), [Out::Send { .. }]));
    }

    #[test]
    fn a_retried_command_goes_out_three_times_then_fails_and_in_progress_stops_the_retries() {
        let mut commands = Commands::default();
        let check = Command { command: CMD_RUN_PREARM_CHECKS, show_error: false, ..arm(5) };
        assert_eq!(commands.send(check, 0).len(), 1);
        assert_eq!(commands.tick(ACK_TIMEOUT_MS + 1).len(), 1);
        assert_eq!(commands.tick(2 * ACK_TIMEOUT_MS + 2).len(), 1);
        assert_eq!(commands.tick(3 * ACK_TIMEOUT_MS + 3), vec![Out::Result { tag: 5, component: 1, command: CMD_RUN_PREARM_CHECKS, result: RESULT_FAILED, failure: Failure::NoResponse }]);
        let mut slow = Commands { high_latency: true, ..Default::default() };
        slow.send(Command { command: CMD_RUN_PREARM_CHECKS, ..arm(6) }, 0);
        assert!(slow.tick(ACK_TIMEOUT_MS + 1).is_empty());
        let mut busy = Commands::default();
        busy.send(Command { command: CMD_RUN_PREARM_CHECKS, show_error: false, ..arm(9) }, 0);
        assert_eq!(busy.on_ack(1, CMD_RUN_PREARM_CHECKS, RESULT_IN_PROGRESS, 100), vec![Out::Progress { tag: 9, component: 1, command: CMD_RUN_PREARM_CHECKS }]);
        assert!(busy.tick(100 + ACK_TIMEOUT_MS - 1).is_empty());
        assert!(matches!(busy.tick(100 + ACK_TIMEOUT_MS + 1).as_slice(), [Out::Result { failure: Failure::NoResponse, .. }]));
        let mut apm = Commands::default();
        apm.send(Command { command: CMD_START_RX_PAIR, show_error: false, ..arm(10) }, 0);
        assert!(matches!(apm.tick(ACK_TIMEOUT_MS + 1).as_slice(), [Out::Result { failure: Failure::NoResponse, .. }]));
    }

    #[test]
    fn a_requested_message_succeeds_after_the_ack_or_before_it_and_times_out_without_it() {
        let mut commands = Commands::default();
        let sent = commands.request_message(11, 1, 148, [0.0; 5], 0);
        assert!(matches!(sent.as_slice(), [Out::Send { command: CMD_REQUEST_MESSAGE, params, .. }] if params[0] == 148.0));
        assert!(commands.on_ack(1, CMD_REQUEST_MESSAGE, RESULT_ACCEPTED, 5).iter().all(|o| !matches!(o, Out::RequestResult { .. })));
        assert_eq!(commands.on_message(1, 148), vec![Out::RequestResult { tag: 11, component: 1, message_id: 148, result: RESULT_ACCEPTED, failure: RequestFailure::None }]);
        commands.request_message(12, 1, 300, [0.0; 5], 100);
        assert_eq!(commands.on_message(1, 300), vec![Out::RequestResult { tag: 12, component: 1, message_id: 300, result: RESULT_ACCEPTED, failure: RequestFailure::None }]);
        assert!(!commands.pending(1, CMD_REQUEST_MESSAGE));
        commands.request_message(13, 1, 148, [0.0; 5], 200);
        commands.on_ack(1, CMD_REQUEST_MESSAGE, RESULT_ACCEPTED, 205);
        assert!(commands.tick(205 + MESSAGE_WAIT_MS).is_empty());
        assert_eq!(commands.tick(205 + MESSAGE_WAIT_MS + 1), vec![Out::RequestResult { tag: 13, component: 1, message_id: 148, result: RESULT_FAILED, failure: RequestFailure::MessageNotReceived }]);
        commands.request_message(14, 1, 148, [0.0; 5], 300);
        let denied = commands.on_ack(1, CMD_REQUEST_MESSAGE, 2, 305);
        assert!(denied.contains(&Out::RequestResult { tag: 14, component: 1, message_id: 148, result: 2, failure: RequestFailure::CommandError }));
        commands.request_message(15, 1, 148, [0.0; 5], 400);
        let duplicate = commands.request_message(16, 1, 148, [0.0; 5], 401);
        assert!(duplicate.contains(&Out::RequestResult { tag: 16, component: 1, message_id: 148, result: RESULT_FAILED, failure: RequestFailure::DuplicateCommand }));
        let mut unanswered = Commands::default();
        unanswered.request_message(17, 1, 148, [0.0; 5], 0);
        let gave_up = (1..=3).flat_map(|n| unanswered.tick(n * (ACK_TIMEOUT_MS + 1))).collect::<Vec<_>>();
        let resolved = unanswered.resolve(gave_up);
        assert!(resolved.contains(&Out::RequestResult { tag: 17, component: 1, message_id: 148, result: RESULT_FAILED, failure: RequestFailure::CommandNotAcked }));
    }
}
