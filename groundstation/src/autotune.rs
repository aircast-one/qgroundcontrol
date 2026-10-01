use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.autotune.autotuneInProgress", "vehicle.autotune.autotuneStatus", "vehicle.autotune.autotuneProgress", "vehicle.flying", "vehicle.landing", "vehicles.activeVehicleAvailable"];

pub const REQUEST: &str = "vehicle.autotune.autotuneRequest";
pub const CMD_DO_AUTOTUNE_ENABLE: u16 = 212;
pub const POLL_MS: u64 = 1000;
const RESULT_ACCEPTED: u8 = 0;
const RESULT_FAILED: u8 = 4;
const RESULT_IN_PROGRESS: u8 = 5;
const WAIT_FOR_DISARM: u8 = 95;
const DONE: u8 = 100;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Autotune {
    pub in_progress: bool,
    pub progress: f64,
    pub status: String,
    disarm_message_displayed: bool,
}

impl Autotune {
    pub fn request(&mut self) {
        self.in_progress = true;
        self.status = "Autotune: In progress".to_string();
    }

    pub fn on_ack(&mut self, result: u8, progress: u8) -> Option<&'static str> {
        if !self.in_progress {
            return None;
        }
        match result {
            RESULT_IN_PROGRESS | RESULT_ACCEPTED => self.on_progress(progress),
            RESULT_FAILED => {
                self.stop();
                self.status = "Autotune: Failed".to_string();
                None
            }
            error => {
                self.stop();
                self.status = format!("Autotune: Ack error {error}");
                None
            }
        }
    }

    fn stop(&mut self) {
        self.in_progress = false;
        self.disarm_message_displayed = false;
    }

    fn on_progress(&mut self, progress: u8) -> Option<&'static str> {
        self.progress = f64::from(progress) / 100.0;
        let (status, message): (&str, Option<&'static str>) = match progress {
            p if p < 20 => ("Autotune: initializing", None),
            p if p < 40 => ("Autotune: roll", None),
            p if p < 60 => ("Autotune: pitch", None),
            p if p < 80 => ("Autotune: yaw", None),
            WAIT_FOR_DISARM => {
                let first = !self.disarm_message_displayed;
                self.disarm_message_displayed = true;
                ("Wait for disarm", first.then_some("Land and disarm the vehicle in order to apply the parameters."))
            }
            p if p < DONE => ("Autotune: in progress", None),
            p => {
                self.in_progress = false;
                match p == DONE {
                    true => ("Autotune: Success", Some("Autotune successful.")),
                    false => ("Autotune: Unknown error", None),
                }
            }
        };
        self.status = status.to_string();
        message
    }
}

pub fn blocked_reason(in_progress: bool, landing: bool, flying: bool) -> &'static str {
    match (in_progress, landing, flying) {
        (true, _, _) => "",
        (false, true, _) => "Not while landing",
        (false, false, false) => "Take off first - auto-tuning runs in flight",
        _ => "",
    }
}

pub fn autotune_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let connected = flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
    let vehicle = object(&backend.get_fields("vehicle", "flying,landing"));
    let tune = object(&backend.get_fields("vehicle.autotune", "autotuneInProgress,autotuneStatus,autotuneProgress"));
    let (flying, landing, in_progress) = (flag(&vehicle, "flying"), flag(&vehicle, "landing"), flag(&tune, "autotuneInProgress"));
    let blocked = blocked_reason(in_progress, landing, flying);
    json!({
        "kind": "object",
        "class": "Autotune",
        "available": connected,
        "inProgress": in_progress,
        "progress": tune.get("autotuneProgress").and_then(Value::as_f64).unwrap_or(0.0),
        "status": if blocked.is_empty() { tune.get("autotuneStatus").and_then(Value::as_str).unwrap_or("").to_string() } else { blocked.to_string() },
        "canStart": flying && !landing && !in_progress,
        "warning": "WARNING!\n\nThe auto-tuning procedure should be executed with caution and requires the vehicle to fly stable enough before attempting the procedure! \n\nBefore starting the auto-tuning process, make sure that: \n1. You have read the auto-tuning guide and have followed the preliminary steps \n2. The current control gains are good enough to stabilize the drone in presence of medium disturbances \n3. You are ready to abort the auto-tuning sequence by moving the RC sticks, if anything unexpected happens. \n\nClick Ok to start the auto-tuning process.\n",
    })
}

pub fn request(backend: &dyn Backend) -> Value {
    crate::guided::dispatch(backend, Some(json!({ "action": "autotune" })), crate::guided::active_id(backend), REQUEST, "[]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_names_the_axis_being_tuned_and_success_ends_it() {
        let mut tune = Autotune::default();
        assert_eq!(tune.on_ack(RESULT_IN_PROGRESS, 30), None, "nothing is followed until a request");
        tune.request();
        assert_eq!(tune.status, "Autotune: In progress");
        [(10, "Autotune: initializing"), (30, "Autotune: roll"), (50, "Autotune: pitch"), (70, "Autotune: yaw"), (85, "Autotune: in progress")]
            .iter()
            .for_each(|(p, s)| {
                tune.on_ack(RESULT_IN_PROGRESS, *p);
                assert_eq!(tune.status, *s);
            });
        assert_eq!(tune.on_ack(RESULT_IN_PROGRESS, 95), Some("Land and disarm the vehicle in order to apply the parameters."));
        assert_eq!(tune.on_ack(RESULT_IN_PROGRESS, 95), None, "the disarm message is shown once");
        assert_eq!(tune.status, "Wait for disarm");
        assert_eq!(tune.on_ack(RESULT_ACCEPTED, 100), Some("Autotune successful."));
        assert!(!tune.in_progress);
        assert_eq!(tune.progress, 1.0);
    }

    #[test]
    fn a_failure_or_an_unexpected_result_stops_it() {
        let mut failed = Autotune::default();
        failed.request();
        failed.on_ack(RESULT_FAILED, 0);
        assert_eq!((failed.in_progress, failed.status.as_str()), (false, "Autotune: Failed"));
        let mut denied = Autotune::default();
        denied.request();
        denied.on_ack(2, 0);
        assert_eq!(denied.status, "Autotune: Ack error 2");
        assert_eq!(blocked_reason(false, false, false), "Take off first - auto-tuning runs in flight");
        assert_eq!(blocked_reason(false, true, true), "Not while landing");
        assert_eq!(blocked_reason(true, true, false), "");
    }
}
