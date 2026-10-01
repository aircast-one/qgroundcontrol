use crate::actuators::{assignment_finish_writes, assignment_plan, assignment_start_writes};

pub const SPIN_VALUE: f32 = 0.15;
pub const SPIN_TIMEOUT: f32 = 0.5;
const SPIN_DELAY_MS: u64 = 1000;
const SPIN_DELAY_AFTER_ASSIGN_MS: u64 = 3000;
const ACK_TIMEOUT_MS: u64 = 1000;
const RESULT_ACCEPTED: u8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum State {
    #[default]
    Idle,
    Init,
    Running,
}

#[derive(Debug, Default)]
pub struct MotorAssignment {
    state: State,
    groups: Vec<Vec<String>>,
    selected_output: usize,
    first: i64,
    count: i64,
    assign_motors: bool,
    selected_motors: Vec<i64>,
    spin_due: Option<u64>,
    in_progress_until: Option<u64>,
    pub message: String,
}

pub type Writes = Vec<(String, i64)>;

impl MotorAssignment {
    pub fn active(&self) -> bool {
        self.state == State::Running
    }

    pub fn highlighted(&self) -> Vec<i64> {
        match self.active() {
            true => (0..self.count).filter(|m| !self.selected_motors.contains(m)).collect(),
            false => Vec::new(),
        }
    }

    pub fn init(&mut self, groups: Vec<Vec<String>>, labels: &[String], selected: usize, first: i64, count: i64, value_of: &dyn Fn(&str) -> Option<i64>) -> bool {
        match assignment_plan(&groups, labels, selected, first, count, value_of) {
            Ok(plan) => {
                *self = MotorAssignment { state: State::Init, groups, selected_output: selected, first, count, assign_motors: plan.assign_motors, message: plan.message, ..MotorAssignment::default() };
                true
            }
            Err(message) => {
                self.message = message;
                false
            }
        }
    }

    pub fn start(&mut self, now_ms: u64, value_of: &dyn Fn(&str) -> Option<i64>) -> Writes {
        if self.state != State::Init {
            return Vec::new();
        }
        self.state = State::Running;
        self.spin_due = Some(now_ms + if self.assign_motors { SPIN_DELAY_AFTER_ASSIGN_MS } else { SPIN_DELAY_MS });
        match self.assign_motors {
            true => assignment_start_writes(&self.groups, self.selected_output, self.first, self.count, value_of),
            false => Vec::new(),
        }
    }

    pub fn select(&mut self, motor: i64, now_ms: u64, value_of: &dyn Fn(&str) -> Option<i64>) -> Writes {
        if !self.active() || !self.highlighted().contains(&motor) {
            return Vec::new();
        }
        self.selected_motors.push(motor);
        if self.selected_motors.len() as i64 == self.count {
            self.state = State::Idle;
            self.spin_due = None;
            return assignment_finish_writes(&self.groups, self.first, &self.selected_motors, value_of);
        }
        self.spin_due = Some(now_ms + SPIN_DELAY_MS);
        Vec::new()
    }

    pub fn spin_again(&mut self, now_ms: u64) -> Option<i64> {
        if !self.active() || self.in_progress_until.is_some_and(|until| now_ms < until) {
            return None;
        }
        self.spin_due = None;
        self.in_progress_until = Some(now_ms + ACK_TIMEOUT_MS);
        Some(self.first + self.selected_motors.len() as i64)
    }

    pub fn tick(&mut self, now_ms: u64) -> Option<i64> {
        match self.spin_due {
            Some(due) if now_ms >= due => self.spin_again(now_ms),
            _ => None,
        }
    }

    pub fn awaiting_ack(&self) -> bool {
        self.in_progress_until.is_some()
    }

    pub fn on_ack(&mut self, result: u8) -> Option<&'static str> {
        self.in_progress_until = None;
        (result != RESULT_ACCEPTED).then(|| {
            self.abort();
            "Actuator test command failed"
        })
    }

    pub fn abort(&mut self) {
        self.state = State::Idle;
        self.spin_due = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups() -> Vec<Vec<String>> {
        vec![(1..=4).map(|i| format!("PWM_MAIN_FUNC{i}")).collect(), (1..=4).map(|i| format!("PWM_AUX_FUNC{i}")).collect()]
    }

    fn labels() -> Vec<String> {
        vec!["MAIN".into(), "AUX".into()]
    }

    #[test]
    fn unassigned_motors_are_offered_to_the_selected_output_and_partial_ones_refused() {
        let none = |_: &str| Some(0);
        let mut assignment = MotorAssignment::default();
        assert!(assignment.init(groups(), &labels(), 1, 101, 4, &none));
        assert!(assignment.message.contains("No motors are assigned yet") && assignment.message.contains("selected output (AUX)"));
        assert!(!assignment.active(), "confirmed only by start");
        let partial = |name: &str| Some(if name == "PWM_MAIN_FUNC1" { 101 } else { 0 });
        assert!(!assignment.init(groups(), &labels(), 0, 101, 4, &partial));
        assert_eq!(assignment.message, "Not all motors are assigned yet. Either clear all existing assignments or assign all motors to an output.");
    }

    #[test]
    fn the_motors_spin_one_by_one_and_the_taps_reorder_the_functions() {
        let on_main = |name: &str| name.strip_prefix("PWM_MAIN_FUNC").and_then(|i| i.parse::<i64>().ok()).map(|i| 100 + i).or(Some(0));
        let mut assignment = MotorAssignment::default();
        assert!(assignment.init(groups(), &labels(), 0, 101, 4, &on_main));
        assert!(!assignment.message.contains("<br />No motors"), "already assigned to the selected output, so nothing is reassigned");
        assert!(assignment.start(0, &on_main).is_empty());
        assert_eq!(assignment.tick(999), None);
        assert_eq!(assignment.tick(1000), Some(101), "the first motor spins after a second");
        assert_eq!(assignment.on_ack(0), None);
        [2, 0, 3].iter().enumerate().for_each(|(i, motor)| {
            assert!(assignment.select(*motor, 2000 + i as u64 * 2000, &on_main).is_empty());
            assert_eq!(assignment.tick(3000 + i as u64 * 2000), Some(102 + i as i64));
            assignment.on_ack(0);
        });
        assert_eq!(assignment.highlighted(), [1]);
        let writes = assignment.select(1, 9000, &on_main);
        assert_eq!(writes, [("PWM_MAIN_FUNC1".to_string(), 103), ("PWM_MAIN_FUNC2".to_string(), 101), ("PWM_MAIN_FUNC3".to_string(), 104), ("PWM_MAIN_FUNC4".to_string(), 102)], "the output that drove the nth spin gets the motor tapped nth");
        assert!(!assignment.active());
    }

    #[test]
    fn assigning_clears_other_outputs_first_and_a_rejection_aborts() {
        let on_aux = |name: &str| name.strip_prefix("PWM_AUX_FUNC").and_then(|i| i.parse::<i64>().ok()).map(|i| 100 + i).or(Some(0));
        let mut assignment = MotorAssignment::default();
        assert!(assignment.init(groups(), &labels(), 0, 101, 4, &on_aux));
        assert!(assignment.message.contains("currently assigned to a different output"));
        let writes = assignment.start(0, &on_aux);
        assert_eq!(writes.iter().filter(|(_, v)| *v == 0).count(), 4);
        assert_eq!(writes.iter().filter(|(n, _)| n.starts_with("PWM_MAIN")).map(|(_, v)| *v).collect::<Vec<_>>(), [101, 102, 103, 104]);
        assert_eq!(assignment.tick(2999), None, "ESCs get three seconds after a reassignment");
        assert_eq!(assignment.tick(3000), Some(101));
        assert_eq!(assignment.on_ack(4), Some("Actuator test command failed"));
        assert!(!assignment.active());
    }
}
