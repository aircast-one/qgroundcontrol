use crate::guidedcmd::{self, Step};
use serde_json::{Value, json};

pub const MODE_TRIES: u32 = 3;
pub const MODE_WAIT_MS: u64 = 1300;
pub const ARM_WAIT_MS: u64 = 1500;

#[derive(Debug, Clone, PartialEq)]
pub enum Emit {
    Command { command: u16, params: [f64; 7], command_int: bool, frame: u8, show_error: bool },
    SetMode { base_mode: u8, custom_mode: u32 },
    PositionTargetLocalNed { frame: u8, type_mask: u16, x: f64, y: f64, z: f64 },
    GuidedMissionItem { latitude: f64, longitude: f64, altitude_relative: f64 },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Observed {
    pub flight_mode: String,
    pub armed: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
enum Wait {
    #[default]
    None,
    Mode { since_ms: u64, tries: u32 },
    Armed { since_ms: u64 },
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
pub struct Executor {
    steps: Vec<Step>,
    at: usize,
    wait: Wait,
    state: State,
    label: String,
}

fn label(step: &Step) -> String {
    match step {
        Step::Command { command, .. } => format!("Sending command {command}"),
        Step::SetMode { mode, .. } | Step::WaitForMode(mode) => format!("Changing mode to {mode}"),
        Step::Arm | Step::WaitArmed => "Arming".to_string(),
        Step::PositionTargetLocalNed { .. } => "Sending position target".to_string(),
        Step::GuidedMissionItem { .. } => "Sending guided waypoint".to_string(),
        Step::SkipIfNoDelta => String::new(),
    }
}

fn mode_emit(step: &Step) -> Option<Emit> {
    match step {
        Step::SetMode { base_mode, custom_mode, via_command, .. } => Some(if *via_command {
            Emit::Command { command: guidedcmd::CMD_DO_SET_MODE, params: [f64::from(crate::modes::FLAG_CUSTOM), *custom_mode as f64, 0.0, 0.0, 0.0, 0.0, 0.0], command_int: false, frame: guidedcmd::FRAME_GLOBAL, show_error: true }
        } else {
            Emit::SetMode { base_mode: *base_mode, custom_mode: *custom_mode }
        }),
        _ => None,
    }
}

impl Executor {
    pub fn running(&self) -> bool {
        self.state == State::Running
    }

    pub fn start(&mut self, steps: Vec<Step>, observed: &Observed, now_ms: u64) -> Vec<Emit> {
        *self = Executor { steps, state: State::Running, ..Executor::default() };
        self.advance(observed, now_ms)
    }

    fn fail(&mut self, reason: String) {
        self.state = State::Failed(reason);
        self.wait = Wait::None;
    }

    pub fn advance(&mut self, observed: &Observed, now_ms: u64) -> Vec<Emit> {
        let mut out = Vec::new();
        while self.running() {
            match self.wait.clone() {
                Wait::Mode { since_ms, tries } => {
                    let wanted = matches!(self.steps.get(self.at), Some(Step::SetMode { mode, .. } | Step::WaitForMode(mode)) if *mode == observed.flight_mode);
                    if wanted {
                        self.wait = Wait::None;
                        self.at += 1;
                        continue;
                    }
                    if now_ms.saturating_sub(since_ms) < MODE_WAIT_MS {
                        return out;
                    }
                    let resend = self.steps.get(self.at).and_then(mode_emit);
                    match (tries < MODE_TRIES, resend) {
                        (true, Some(emit)) => {
                            out.push(emit);
                            self.wait = Wait::Mode { since_ms: now_ms, tries: tries + 1 };
                        }
                        _ => {
                            let mode = match self.steps.get(self.at) {
                                Some(Step::SetMode { mode, .. } | Step::WaitForMode(mode)) => mode.clone(),
                                _ => String::new(),
                            };
                            self.fail(format!("Unable to change to {mode} mode."));
                        }
                    }
                    return out;
                }
                Wait::Armed { since_ms } => {
                    if observed.armed {
                        self.wait = Wait::None;
                        self.at += 1;
                        continue;
                    }
                    if now_ms.saturating_sub(since_ms) >= ARM_WAIT_MS {
                        self.fail("Unable to arm vehicle.".to_string());
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
                Step::SetMode { mode, .. } | Step::WaitForMode(mode) if mode == observed.flight_mode => self.at += 1,
                Step::SetMode { .. } => {
                    out.extend(mode_emit(&step));
                    self.wait = Wait::Mode { since_ms: now_ms, tries: 1 };
                }
                Step::WaitForMode(_) => self.wait = Wait::Mode { since_ms: now_ms, tries: MODE_TRIES },
                Step::Arm | Step::WaitArmed if observed.armed => self.at += 1,
                Step::Arm => {
                    out.push(match guidedcmd::arm(true, false) {
                        Step::Command { command, params, command_int, frame, .. } => Emit::Command { command, params, command_int, frame, show_error: false },
                        _ => unreachable!(),
                    });
                    self.at += 1;
                    self.wait = Wait::Armed { since_ms: now_ms };
                }
                Step::WaitArmed => self.wait = Wait::Armed { since_ms: now_ms },
                Step::Command { command, params, command_int, frame, show_error } => {
                    out.push(Emit::Command { command, params, command_int, frame, show_error });
                    self.at += 1;
                }
                Step::PositionTargetLocalNed { frame, type_mask, x, y, z } => {
                    out.push(Emit::PositionTargetLocalNed { frame, type_mask, x, y, z });
                    self.at += 1;
                }
                Step::GuidedMissionItem { latitude, longitude, altitude_relative } => {
                    out.push(Emit::GuidedMissionItem { latitude, longitude, altitude_relative });
                    self.at += 1;
                }
                Step::SkipIfNoDelta => self.at += 1,
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
        VehicleState { autopilot: AUTOPILOT_ARDUPILOT, vehicle_type: 2, base_mode: 0x81, flight_mode: "Loiter".into(), armed: false, altitude_amsl: Some(500.0), altitude_relative: Some(20.0), home_altitude: Some(480.0), capabilities: 0, reposition_supported: None, minimum_takeoff_altitude: 2.5, current_heading: None }
    }

    fn steps(plan: Plan) -> Vec<Step> {
        match plan {
            Plan::Steps(steps) => steps,
            Plan::Refused(reason) => panic!("{reason}"),
        }
    }

    fn observed(mode: &str, armed: bool) -> Observed {
        Observed { flight_mode: mode.into(), armed }
    }

    fn commands(emits: &[Emit]) -> Vec<u16> {
        emits.iter().filter_map(|e| match e { Emit::Command { command, .. } => Some(*command), _ => None }).collect()
    }

    #[test]
    fn a_copter_takeoff_changes_mode_arms_and_takes_off_as_the_vehicle_reports_each_step() {
        let mut executor = Executor::default();
        let first = executor.start(steps(guidedcmd::takeoff(&copter(), 10.0)), &observed("Loiter", false), 0);
        assert_eq!(commands(&first), vec![guidedcmd::CMD_DO_SET_MODE]);
        assert_eq!(executor.snapshot()["label"], "Changing mode to Guided");
        assert!(executor.advance(&observed("Loiter", false), 500).is_empty(), "nothing is resent before the wait elapses");
        let armed = executor.advance(&observed("Guided", false), 900);
        assert_eq!(commands(&armed), vec![CMD_COMPONENT_ARM_DISARM]);
        assert_eq!(executor.snapshot()["label"], "Arming");
        assert!(executor.advance(&observed("Guided", false), 1000).is_empty());
        let takeoff = executor.advance(&observed("Guided", true), 1100);
        assert_eq!(commands(&takeoff), vec![CMD_NAV_TAKEOFF]);
        assert_eq!(executor.snapshot()["state"], "done");
        assert!(!executor.running());
    }

    #[test]
    fn the_mode_change_is_retried_three_times_then_fails_as_the_qt_head_does() {
        let mut executor = Executor::default();
        let plan = steps(guidedcmd::rtl(&copter(), false));
        assert_eq!(executor.start(plan, &observed("Loiter", true), 0).len(), 1);
        assert!(executor.advance(&observed("Loiter", true), 1299).is_empty());
        assert_eq!(executor.advance(&observed("Loiter", true), 1300).len(), 1, "second try");
        assert_eq!(executor.advance(&observed("Loiter", true), 2600).len(), 1, "third try");
        assert!(executor.advance(&observed("Loiter", true), 3899).is_empty());
        assert!(executor.advance(&observed("Loiter", true), 3900).is_empty());
        assert_eq!(executor.snapshot()["state"], "failed");
        assert_eq!(executor.snapshot()["reason"], "Unable to change to RTL mode.");
    }

    #[test]
    fn arming_is_sent_once_and_gives_up_after_the_heartbeat_window() {
        let mut executor = Executor::default();
        let plan = steps(guidedcmd::takeoff(&VehicleState { flight_mode: "Guided".into(), ..copter() }, 10.0));
        let first = executor.start(plan, &observed("Guided", false), 0);
        assert_eq!(commands(&first), vec![CMD_COMPONENT_ARM_DISARM], "the mode already matches, so arming is the first thing sent");
        assert!(executor.advance(&observed("Guided", false), 1499).is_empty());
        assert!(executor.advance(&observed("Guided", false), 1500).is_empty());
        assert_eq!(executor.snapshot(), json!({ "state": "failed", "reason": "Unable to arm vehicle.", "step": 3, "steps": 5, "label": "Arming" }));
    }

    #[test]
    fn set_mode_messages_and_fire_and_forget_plans_finish_at_once() {
        let state = VehicleState { autopilot: AUTOPILOT_PX4, flight_mode: "Position".into(), ..copter() };
        let mut executor = Executor::default();
        let emits = executor.start(steps(guidedcmd::rtl(&state, false)), &observed("Position", true), 0);
        assert!(matches!(emits[0], Emit::SetMode { base_mode: 0x81, .. }));
        assert_eq!(executor.advance(&observed("Return", true), 10), Vec::<Emit>::new());
        assert_eq!(executor.snapshot()["state"], "done");
        let mut goto = Executor::default();
        let emits = goto.start(steps(guidedcmd::goto(&copter(), 47.4, 8.5, 0.0)), &observed("Guided", true), 0);
        assert!(matches!(emits.as_slice(), [Emit::Command { command: 192, .. }, Emit::GuidedMissionItem { .. }]));
        assert_eq!(goto.snapshot()["state"], "done");
    }
}
