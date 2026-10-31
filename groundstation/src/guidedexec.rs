use crate::guidedcmd::{self, Step};
use serde_json::{Value, json};

pub const MODE_TRIES: u32 = 3;
pub const MODE_WAIT_MS: u64 = 1300;
const ARM_ANSWER_WAIT_MS: u64 = 3000;
const ARMED_AFTER_ACK_WAIT_MS: u64 = 2500;
const LATE_ARM_WATCH_MS: u64 = 10_000;
pub const ACCEPT_WAIT_MS: u64 = 10_000;
const LIVE_STEP_MS: u64 = 250;
pub const LATE_ARM_DISARMED: &str = "The aircraft armed only after the app had stopped waiting for it, so the app disarmed it. Check the link and try again.";
pub const LATE_ARM_FLYING: &str =
    "The aircraft armed only after the app had stopped waiting for it, and it is flying.";

#[derive(Debug, Clone, PartialEq)]
pub enum Emit {
    Command {
        command: u16,
        params: [f64; 7],
        command_int: bool,
        frame: u8,
        show_error: bool,
    },
    SetMode {
        base_mode: u8,
        custom_mode: u32,
    },
    PositionTargetLocalNed {
        frame: u8,
        type_mask: u16,
        x: f64,
        y: f64,
        z: f64,
    },
    GuidedMissionItem {
        latitude: f64,
        longitude: f64,
        altitude_relative: f64,
    },
    Tell(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Observed {
    pub flight_mode: String,
    pub armed: bool,
    pub flying: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
enum Wait {
    #[default]
    None,
    Mode {
        since_ms: u64,
        tries: u32,
        answered: bool,
    },
    Armed {
        since_ms: u64,
        accepted: bool,
    },
    Accepted {
        command: u16,
        since_ms: u64,
        failure: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum State {
    #[default]
    Idle,
    Running,
    Done,
    Failed(String),
}

#[derive(Debug, Default)]
struct Clock {
    last_ms: Option<u64>,
    live_ms: u64,
}

impl Clock {
    fn read(&mut self, now_ms: u64) -> u64 {
        let seen = self.live_ms;
        self.live_ms += self
            .last_ms
            .map_or(0, |last| now_ms.saturating_sub(last).min(LIVE_STEP_MS));
        self.last_ms = Some(now_ms);
        seen
    }
}

#[derive(Debug, Default)]
pub struct Executor {
    steps: Vec<Step>,
    at: usize,
    wait: Wait,
    state: State,
    label: String,
    acked: Option<(u16, bool)>,
    failures: Option<(String, String)>,
    clock: Clock,
    late_arm_until: Option<u64>,
}

fn label(step: &Step) -> String {
    match step {
        Step::Command { command, .. } => format!("Sending command {command}"),
        Step::SetMode { mode, .. } | Step::WaitForMode(mode) => format!("Changing mode to {mode}"),
        Step::Arm | Step::WaitArmed => "Arming".to_string(),
        Step::PositionTargetLocalNed { .. } => "Sending position target".to_string(),
        Step::GuidedMissionItem { .. } => "Sending guided waypoint".to_string(),
        Step::SkipIfNoDelta => String::new(),
        Step::AwaitAccepted { command, .. } => format!("Waiting for command {command}"),
        Step::FailWith { .. } => String::new(),
    }
}

fn arm_emit(arm: bool, show_error: bool) -> Emit {
    match guidedcmd::arm(arm, false) {
        Step::Command {
            command,
            params,
            command_int,
            frame,
            ..
        } => Emit::Command {
            command,
            params,
            command_int,
            frame,
            show_error,
        },
        _ => unreachable!(),
    }
}

fn mode_emit(step: &Step) -> Option<Emit> {
    match step {
        Step::SetMode {
            base_mode,
            custom_mode,
            via_command,
            ..
        } => Some(if *via_command {
            Emit::Command {
                command: guidedcmd::CMD_DO_SET_MODE,
                params: [
                    f64::from(crate::modes::FLAG_CUSTOM),
                    *custom_mode as f64,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                ],
                command_int: false,
                frame: guidedcmd::FRAME_GLOBAL,
                show_error: true,
            }
        } else {
            Emit::SetMode {
                base_mode: *base_mode,
                custom_mode: *custom_mode,
            }
        }),
        _ => None,
    }
}

impl Executor {
    pub fn running(&self) -> bool {
        self.state == State::Running
    }

    pub fn start(&mut self, steps: Vec<Step>, observed: &Observed, now_ms: u64) -> Vec<Emit> {
        *self = Executor {
            steps,
            state: State::Running,
            ..Executor::default()
        };
        self.advance(observed, now_ms)
    }

    pub fn on_command_result(&mut self, command: u16, accepted: bool) {
        let arming = command == guidedcmd::CMD_COMPONENT_ARM_DISARM
            && (matches!(self.wait, Wait::Armed { .. }) || self.late_arm_until.is_some());
        if arming
            || matches!(self.wait, Wait::Accepted { command: waited, .. } if waited == command)
        {
            self.acked = Some((command, accepted));
        }
        if let Wait::Mode {
            since_ms,
            tries,
            answered: false,
        } = self.wait
            && command == guidedcmd::CMD_DO_SET_MODE
        {
            match accepted {
                true => {
                    self.wait = Wait::Mode {
                        since_ms,
                        tries,
                        answered: true,
                    }
                }
                false => {
                    let mode = match self.steps.get(self.at) {
                        Some(Step::SetMode { mode, .. } | Step::WaitForMode(mode)) => mode.clone(),
                        _ => String::new(),
                    };
                    self.fail(format!("Unable to change to {mode} mode."));
                }
            }
        }
    }

    fn finish(&mut self) {
        self.state = State::Done;
        self.wait = Wait::None;
        self.label = String::new();
    }

    fn fail(&mut self, reason: String) {
        self.state = State::Failed(reason);
        self.wait = Wait::None;
    }

    fn arm_failure(&self) -> String {
        self.failures.as_ref().map_or_else(
            || "Unable to arm vehicle.".to_string(),
            |(_, text)| text.clone(),
        )
    }

    fn late_arm(&mut self, observed: &Observed, live_ms: u64) -> Vec<Emit> {
        let Some(until) = self.late_arm_until else {
            return Vec::new();
        };
        let acked = matches!(
            self.acked.take(),
            Some((guidedcmd::CMD_COMPONENT_ARM_DISARM, true))
        );
        if live_ms > until {
            self.late_arm_until = None;
            return Vec::new();
        }
        if !observed.armed && !acked {
            return Vec::new();
        }
        self.late_arm_until = None;
        let told = if observed.flying {
            LATE_ARM_FLYING
        } else {
            LATE_ARM_DISARMED
        };
        self.state = State::Failed(told.to_string());
        (!observed.flying)
            .then(|| arm_emit(false, true))
            .into_iter()
            .chain([Emit::Tell(told.to_string())])
            .collect()
    }

    pub fn advance(&mut self, observed: &Observed, now_ms: u64) -> Vec<Emit> {
        let live_ms = self.clock.read(now_ms);
        let mut out = self.late_arm(observed, live_ms);
        while self.running() {
            match self.wait.clone() {
                Wait::Mode {
                    since_ms, tries, ..
                } => {
                    let wanted = matches!(self.steps.get(self.at), Some(Step::SetMode { mode, .. } | Step::WaitForMode(mode)) if *mode == observed.flight_mode);
                    if wanted {
                        self.wait = Wait::None;
                        self.at += 1;
                        continue;
                    }
                    if live_ms.saturating_sub(since_ms) < MODE_WAIT_MS {
                        return out;
                    }
                    let resend = self.steps.get(self.at).and_then(mode_emit);
                    match (tries < MODE_TRIES, resend) {
                        (true, Some(emit)) => {
                            out.push(emit);
                            self.wait = Wait::Mode {
                                since_ms: live_ms,
                                tries: tries + 1,
                                answered: false,
                            };
                        }
                        _ => {
                            let mode = match self.steps.get(self.at) {
                                Some(Step::SetMode { mode, .. } | Step::WaitForMode(mode)) => {
                                    mode.clone()
                                }
                                _ => String::new(),
                            };
                            self.fail(self.failures.as_ref().map_or_else(
                                || format!("Unable to change to {mode} mode."),
                                |(text, _)| text.clone(),
                            ));
                        }
                    }
                    return out;
                }
                Wait::Accepted {
                    command,
                    since_ms,
                    ref failure,
                } => {
                    let failure = failure.clone();
                    let refused = |executor: &mut Self| match failure {
                        Some(reason) => executor.fail(reason),
                        None => executor.finish(),
                    };
                    match self.acked.take() {
                        Some((acked, true)) if acked == command => {
                            self.wait = Wait::None;
                            self.at += 1;
                            continue;
                        }
                        Some((acked, false)) if acked == command => refused(self),
                        _ if live_ms.saturating_sub(since_ms) >= ACCEPT_WAIT_MS => refused(self),
                        _ => {}
                    }
                    return out;
                }
                Wait::Armed { since_ms, accepted } => {
                    if observed.armed {
                        self.wait = Wait::None;
                        self.at += 1;
                        continue;
                    }
                    let window = if accepted {
                        ARMED_AFTER_ACK_WAIT_MS
                    } else {
                        ARM_ANSWER_WAIT_MS
                    };
                    match self.acked.take() {
                        Some((guidedcmd::CMD_COMPONENT_ARM_DISARM, false)) => {
                            self.fail(self.arm_failure())
                        }
                        Some((guidedcmd::CMD_COMPONENT_ARM_DISARM, true)) if !accepted => {
                            self.wait = Wait::Armed {
                                since_ms: live_ms,
                                accepted: true,
                            }
                        }
                        _ if live_ms.saturating_sub(since_ms) >= window => {
                            self.fail(self.arm_failure());
                            self.late_arm_until = Some(live_ms + LATE_ARM_WATCH_MS);
                        }
                        _ => {}
                    }
                    return out;
                }
                Wait::None => {}
            }
            let Some(step) = self.steps.get(self.at).cloned() else {
                self.state = State::Done;
                self.label = String::new();
                return out;
            };
            self.label = label(&step);
            match step {
                Step::SetMode { mode, .. } | Step::WaitForMode(mode)
                    if mode == observed.flight_mode =>
                {
                    self.at += 1
                }
                Step::SetMode { .. } => {
                    out.extend(mode_emit(&step));
                    self.wait = Wait::Mode {
                        since_ms: live_ms,
                        tries: 1,
                        answered: false,
                    };
                }
                Step::WaitForMode(_) => {
                    self.wait = Wait::Mode {
                        since_ms: live_ms,
                        tries: MODE_TRIES,
                        answered: true,
                    }
                }
                Step::Arm | Step::WaitArmed if observed.armed => self.at += 1,
                Step::Arm => {
                    out.push(arm_emit(true, false));
                    self.at += 1;
                    self.wait = Wait::Armed {
                        since_ms: live_ms,
                        accepted: false,
                    };
                }
                Step::WaitArmed => {
                    self.wait = Wait::Armed {
                        since_ms: live_ms,
                        accepted: false,
                    }
                }
                Step::Command {
                    command,
                    params,
                    command_int,
                    frame,
                    show_error,
                } => {
                    out.push(Emit::Command {
                        command,
                        params,
                        command_int,
                        frame,
                        show_error,
                    });
                    self.at += 1;
                }
                Step::PositionTargetLocalNed {
                    frame,
                    type_mask,
                    x,
                    y,
                    z,
                } => {
                    out.push(Emit::PositionTargetLocalNed {
                        frame,
                        type_mask,
                        x,
                        y,
                        z,
                    });
                    self.at += 1;
                }
                Step::GuidedMissionItem {
                    latitude,
                    longitude,
                    altitude_relative,
                } => {
                    out.push(Emit::GuidedMissionItem {
                        latitude,
                        longitude,
                        altitude_relative,
                    });
                    self.at += 1;
                }
                Step::SkipIfNoDelta => self.at += 1,
                Step::AwaitAccepted { command, failure } => {
                    self.wait = Wait::Accepted {
                        command,
                        since_ms: live_ms,
                        failure,
                    }
                }
                Step::FailWith { mode, arm } => {
                    self.failures = Some((mode, arm));
                    self.at += 1;
                }
            }
        }
        out
    }

    pub fn snapshot(&self) -> Value {
        let (state, reason) = match &self.state {
            State::Idle => ("idle", Value::Null),
            State::Running => ("running", Value::Null),
            State::Done => ("done", Value::Null),
            State::Failed(reason) => ("failed", Value::from(reason.as_str())),
        };
        json!({ "state": state, "reason": reason, "step": self.at.min(self.steps.len()), "steps": self.steps.len(), "label": self.label })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guidedcmd::{CMD_COMPONENT_ARM_DISARM, CMD_NAV_TAKEOFF, Plan, VehicleState};
    use crate::modes::{AUTOPILOT_ARDUPILOT, AUTOPILOT_PX4};

    fn copter() -> VehicleState {
        VehicleState {
            autopilot: AUTOPILOT_ARDUPILOT,
            vehicle_type: 2,
            base_mode: 0x81,
            flight_mode: "Loiter".into(),
            armed: false,
            altitude_amsl: Some(500.0),
            altitude_relative: Some(20.0),
            home_altitude: Some(480.0),
            capabilities: 0,
            reposition_supported: None,
            minimum_takeoff_altitude: 2.5,
            current_heading: None,
            announced_modes: Vec::new(),
        }
    }

    fn steps(plan: Plan) -> Vec<Step> {
        match plan {
            Plan::Steps(steps) => steps,
            Plan::Refused(reason) => panic!("{reason}"),
        }
    }

    fn observed(mode: &str, armed: bool) -> Observed {
        Observed {
            flight_mode: mode.into(),
            armed,
            flying: false,
        }
    }

    fn ticks(executor: &mut Executor, seen: &Observed, from_ms: u64, to_ms: u64) -> Vec<Emit> {
        (from_ms..=to_ms)
            .step_by(100)
            .flat_map(|at| executor.advance(seen, at))
            .collect()
    }

    fn arming_in_guided() -> Executor {
        let mut executor = Executor::default();
        let plan = steps(guidedcmd::takeoff(
            &VehicleState {
                flight_mode: "Guided".into(),
                ..copter()
            },
            10.0,
        ));
        assert_eq!(
            commands(&executor.start(plan, &observed("Guided", false), 0)),
            vec![CMD_COMPONENT_ARM_DISARM]
        );
        executor
    }

    fn disarm() -> Emit {
        Emit::Command {
            command: CMD_COMPONENT_ARM_DISARM,
            params: [0.0; 7],
            command_int: false,
            frame: guidedcmd::FRAME_GLOBAL,
            show_error: true,
        }
    }

    fn commands(emits: &[Emit]) -> Vec<u16> {
        emits
            .iter()
            .filter_map(|e| match e {
                Emit::Command { command, .. } => Some(*command),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_copter_takeoff_changes_mode_arms_and_takes_off_as_the_vehicle_reports_each_step() {
        let mut executor = Executor::default();
        let first = executor.start(
            steps(guidedcmd::takeoff(&copter(), 10.0)),
            &observed("Loiter", false),
            0,
        );
        assert_eq!(commands(&first), vec![guidedcmd::CMD_DO_SET_MODE]);
        assert_eq!(executor.snapshot()["label"], "Changing mode to Guided");
        assert!(
            executor.advance(&observed("Loiter", false), 500).is_empty(),
            "nothing is resent before the wait elapses"
        );
        let armed = executor.advance(&observed("Guided", false), 900);
        assert_eq!(commands(&armed), vec![CMD_COMPONENT_ARM_DISARM]);
        assert_eq!(executor.snapshot()["label"], "Arming");
        assert!(
            executor
                .advance(&observed("Guided", false), 1000)
                .is_empty()
        );
        let takeoff = executor.advance(&observed("Guided", true), 1100);
        assert_eq!(commands(&takeoff), vec![CMD_NAV_TAKEOFF]);
        assert_eq!(executor.snapshot()["state"], "done");
        assert!(!executor.running());
    }

    #[test]
    fn the_mode_change_is_retried_three_times_then_fails_as_the_qt_head_does() {
        let mut executor = Executor::default();
        let plan = steps(guidedcmd::rtl(&copter(), false));
        let loiter = observed("Loiter", true);
        assert_eq!(executor.start(plan, &loiter, 0).len(), 1);
        assert!(ticks(&mut executor, &loiter, 100, 1_300).is_empty());
        assert_eq!(
            ticks(&mut executor, &loiter, 1_400, 1_400).len(),
            1,
            "second try"
        );
        assert!(ticks(&mut executor, &loiter, 1_500, 2_600).is_empty());
        assert_eq!(
            ticks(&mut executor, &loiter, 2_700, 2_700).len(),
            1,
            "third try"
        );
        assert!(ticks(&mut executor, &loiter, 2_800, 3_900).is_empty());
        assert_eq!(executor.snapshot()["state"], "running");
        assert!(ticks(&mut executor, &loiter, 4_000, 4_000).is_empty());
        assert_eq!(executor.snapshot()["state"], "failed");
        assert_eq!(
            executor.snapshot()["reason"],
            "Unable to change to RTL mode."
        );
    }

    #[test]
    fn a_mode_the_vehicle_refuses_is_not_asked_for_again() {
        let mut executor = Executor::default();
        let plan = steps(guidedcmd::rtl(&copter(), false));
        assert_eq!(executor.start(plan, &observed("Loiter", true), 0).len(), 1);
        executor.on_command_result(guidedcmd::CMD_DO_SET_MODE, false);
        assert!(
            executor.advance(&observed("Loiter", true), 1300).is_empty(),
            "a refusal is an answer; asking twice more only repeats it"
        );
        assert_eq!(
            executor.snapshot()["reason"],
            "Unable to change to RTL mode."
        );
        let mut accepted = Executor::default();
        accepted.start(
            steps(guidedcmd::rtl(&copter(), false)),
            &observed("Loiter", true),
            0,
        );
        accepted.on_command_result(guidedcmd::CMD_DO_SET_MODE, true);
        accepted.on_command_result(guidedcmd::CMD_DO_SET_MODE, false);
        assert_eq!(
            accepted.snapshot()["state"],
            "running",
            "once the vehicle took the change, a later refusal answers some other request"
        );
    }

    #[test]
    fn an_arm_nobody_answers_is_sent_once_and_gives_up_after_the_answer_window() {
        let mut executor = arming_in_guided();
        let guided = observed("Guided", false);
        assert!(ticks(&mut executor, &guided, 100, 3_000).is_empty());
        assert_eq!(executor.snapshot()["state"], "running");
        assert!(ticks(&mut executor, &guided, 3_100, 3_100).is_empty());
        assert_eq!(
            executor.snapshot(),
            json!({ "state": "failed", "reason": "Unable to takeoff: Vehicle failed to arm.", "step": 4, "steps": 6, "label": "Arming" })
        );
    }

    #[test]
    fn a_stalled_core_does_not_spend_the_arm_window_and_reads_what_arrived_before_deciding() {
        let mut executor = arming_in_guided();
        let guided = observed("Guided", false);
        assert!(ticks(&mut executor, &guided, 100, 1_000).is_empty());
        assert!(
            executor.advance(&guided, 3_500).is_empty(),
            "two and a half seconds of a blocked worker count as one tick"
        );
        assert_eq!(executor.snapshot()["state"], "running");
        assert_eq!(
            commands(&executor.advance(&observed("Guided", true), 3_600)),
            vec![CMD_NAV_TAKEOFF],
            "the heartbeat that arrived during the stall is read before the window is judged"
        );

        let mut late = arming_in_guided();
        assert!(ticks(&mut late, &guided, 100, 2_900).is_empty());
        assert!(
            late.advance(&guided, 6_000).is_empty(),
            "a stall at the very end of the window still leaves one tick to read the link"
        );
        assert_eq!(late.snapshot()["state"], "running");
        assert_eq!(
            commands(&late.advance(&observed("Guided", true), 6_100)),
            vec![CMD_NAV_TAKEOFF]
        );
    }

    #[test]
    fn an_accepted_arm_waits_for_its_heartbeat_over_a_slow_link() {
        let mut executor = arming_in_guided();
        let guided = observed("Guided", false);
        assert!(ticks(&mut executor, &guided, 100, 1_200).is_empty());
        executor.on_command_result(CMD_COMPONENT_ARM_DISARM, true);
        assert!(ticks(&mut executor, &guided, 1_300, 3_600).is_empty());
        assert_eq!(
            executor.snapshot()["state"],
            "running",
            "past the unanswered window, but the vehicle said it armed"
        );
        assert_eq!(
            commands(&executor.advance(&observed("Guided", true), 3_700)),
            vec![CMD_NAV_TAKEOFF]
        );

        let mut silent = arming_in_guided();
        assert!(ticks(&mut silent, &guided, 100, 1_200).is_empty());
        silent.on_command_result(CMD_COMPONENT_ARM_DISARM, true);
        assert!(ticks(&mut silent, &guided, 1_300, 3_700).is_empty());
        assert_eq!(silent.snapshot()["state"], "running");
        assert!(ticks(&mut silent, &guided, 3_800, 3_800).is_empty());
        assert_eq!(
            silent.snapshot()["reason"],
            "Unable to takeoff: Vehicle failed to arm.",
            "two and a half seconds after the ack with no armed heartbeat is a failure"
        );
    }

    #[test]
    fn a_refused_arm_fails_at_once_and_a_later_arm_is_left_alone() {
        let mut executor = arming_in_guided();
        let guided = observed("Guided", false);
        assert!(ticks(&mut executor, &guided, 100, 300).is_empty());
        executor.on_command_result(CMD_COMPONENT_ARM_DISARM, false);
        assert!(ticks(&mut executor, &guided, 400, 400).is_empty());
        assert_eq!(
            executor.snapshot()["reason"],
            "Unable to takeoff: Vehicle failed to arm."
        );
        assert!(
            ticks(&mut executor, &observed("Guided", true), 500, 5_000).is_empty(),
            "the vehicle refused this arm, so arming afterwards came from somewhere else"
        );
    }

    #[test]
    fn an_arm_that_lands_after_the_failure_is_disarmed_on_the_ground_once() {
        let mut executor = arming_in_guided();
        let guided = observed("Guided", false);
        assert!(ticks(&mut executor, &guided, 100, 3_100).is_empty());
        assert_eq!(executor.snapshot()["state"], "failed");
        assert_eq!(
            ticks(&mut executor, &observed("Guided", true), 3_200, 6_000),
            vec![disarm(), Emit::Tell(LATE_ARM_DISARMED.into())],
            "a normal disarm, never the forced one, and the pilot hears why"
        );
        assert_eq!(executor.snapshot()["reason"], LATE_ARM_DISARMED);

        let mut acked = arming_in_guided();
        assert!(ticks(&mut acked, &guided, 100, 3_100).is_empty());
        acked.on_command_result(CMD_COMPONENT_ARM_DISARM, true);
        assert_eq!(
            ticks(&mut acked, &guided, 3_200, 3_200),
            vec![disarm(), Emit::Tell(LATE_ARM_DISARMED.into())],
            "the late ack says this arm went through before any heartbeat shows it"
        );
    }

    #[test]
    fn an_arm_that_lands_after_the_failure_in_the_air_is_reported_and_never_disarmed() {
        let mut executor = arming_in_guided();
        assert!(ticks(&mut executor, &observed("Guided", false), 100, 3_100).is_empty());
        let flying = Observed {
            flying: true,
            ..observed("Guided", true)
        };
        assert_eq!(
            ticks(&mut executor, &flying, 3_200, 6_000),
            vec![Emit::Tell(LATE_ARM_FLYING.into())]
        );
    }

    #[test]
    fn an_arm_long_after_the_failure_or_after_a_new_command_is_not_taken_for_this_one() {
        let mut executor = arming_in_guided();
        let guided = observed("Guided", false);
        assert!(ticks(&mut executor, &guided, 100, 3_100).is_empty());
        assert!(ticks(&mut executor, &guided, 3_200, 13_200).is_empty());
        assert!(
            ticks(&mut executor, &observed("Guided", true), 13_300, 14_000).is_empty(),
            "ten seconds on, an arm is the pilot's own"
        );

        let mut replaced = arming_in_guided();
        assert!(ticks(&mut replaced, &guided, 100, 3_100).is_empty());
        replaced.start(
            steps(guidedcmd::rtl(&copter(), false)),
            &observed("Guided", false),
            3_200,
        );
        assert!(
            commands(&ticks(
                &mut replaced,
                &observed("Guided", true),
                3_300,
                4_000
            ))
            .iter()
            .all(|command| *command != CMD_COMPONENT_ARM_DISARM)
        );
    }

    #[test]
    fn a_mission_start_fails_in_the_firmware_plugins_words() {
        let mut executor = Executor::default();
        executor.start(
            steps(guidedcmd::start_mission(&copter(), false)),
            &observed("Loiter", false),
            0,
        );
        ticks(&mut executor, &observed("Loiter", false), 100, 4_000);
        assert_eq!(
            executor.snapshot()["reason"],
            "Unable to start mission: Vehicle failed to change to Guided mode."
        );
        let mut arming = Executor::default();
        arming.start(
            steps(guidedcmd::start_mission(&copter(), false)),
            &observed("Guided", false),
            0,
        );
        ticks(&mut arming, &observed("Guided", false), 100, 3_100);
        assert_eq!(
            arming.snapshot()["reason"],
            "Unable to start mission: Vehicle failed to arm."
        );
    }

    #[test]
    fn set_mode_messages_and_fire_and_forget_plans_finish_at_once() {
        let state = VehicleState {
            autopilot: AUTOPILOT_PX4,
            flight_mode: "Position".into(),
            ..copter()
        };
        let mut executor = Executor::default();
        let emits = executor.start(
            steps(guidedcmd::rtl(&state, false)),
            &observed("Position", true),
            0,
        );
        assert!(matches!(
            emits[0],
            Emit::SetMode {
                base_mode: 0x81,
                ..
            }
        ));
        assert_eq!(
            executor.advance(&observed("Return", true), 10),
            Vec::<Emit>::new()
        );
        assert_eq!(executor.snapshot()["state"], "done");
        let mut goto = Executor::default();
        let emits = goto.start(
            steps(guidedcmd::goto(&copter(), 47.4, 8.5, 0.0)),
            &observed("Guided", true),
            0,
        );
        assert!(matches!(
            emits.as_slice(),
            [
                Emit::Command { command: 192, .. },
                Emit::GuidedMissionItem { .. }
            ]
        ));
        assert_eq!(goto.snapshot()["state"], "done");
    }

    #[test]
    fn px4_takeoff_arms_only_after_the_vehicle_accepts_it() {
        let mut px4 = copter();
        px4.autopilot = crate::modes::AUTOPILOT_PX4;
        px4.altitude_amsl = Some(500.0);
        let mut exec = Executor::default();
        let sent = exec.start(
            steps(guidedcmd::takeoff(&px4, 10.0)),
            &observed("Hold", false),
            0,
        );
        assert_eq!(commands(&sent), vec![guidedcmd::CMD_NAV_TAKEOFF]);
        assert!(
            exec.advance(&observed("Hold", false), 100).is_empty(),
            "nothing more until the takeoff is answered"
        );
        exec.on_command_result(guidedcmd::CMD_NAV_TAKEOFF, true);
        assert_eq!(
            commands(&exec.advance(&observed("Takeoff", false), 200)),
            vec![guidedcmd::CMD_COMPONENT_ARM_DISARM]
        );

        let mut refused = Executor::default();
        refused.start(
            steps(guidedcmd::takeoff(&px4, 10.0)),
            &observed("Hold", false),
            0,
        );
        refused.on_command_result(guidedcmd::CMD_NAV_TAKEOFF, false);
        assert!(
            refused.advance(&observed("Hold", false), 100).is_empty(),
            "a refused takeoff never arms"
        );
        assert!(!refused.running());
    }

    #[test]
    fn a_px4_pause_the_vehicle_refuses_never_changes_altitude() {
        let mut px4 = copter();
        px4.autopilot = crate::modes::AUTOPILOT_PX4;
        px4.home_altitude = Some(500.0);
        let mut exec = Executor::default();
        let sent = exec.start(
            steps(guidedcmd::change_altitude(&px4, 5.0, true)),
            &observed("Mission", false),
            0,
        );
        assert_eq!(commands(&sent), vec![guidedcmd::CMD_DO_REPOSITION]);
        exec.on_command_result(guidedcmd::CMD_DO_REPOSITION, false);
        assert!(exec.advance(&observed("Mission", false), 100).is_empty());
        assert_eq!(
            exec.snapshot()["reason"],
            "Unable to pause vehicle.",
            "PX4FirmwarePlugin::_pauseVehicleThenChangeAltResultHandler"
        );
    }
}
