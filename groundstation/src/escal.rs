use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::router::Backend;

pub const ESC_CAL_START: &str = "escCalibration.start";
pub const ESC_CAL_CLOSE: &str = "escCalibration.close";
const CAL_PREFIX: &str = "[cal] ";
const ESC_CAL_PARAM7: f64 = 1.0;
const FAILED: &str = "ESC Calibration failed. ";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Status {
    pub vehicle: u8,
    pub highlight: String,
    pub text: String,
    pub running: bool,
    pub warnings: Vec<String>,
}

static STATUS: Mutex<Option<Status>> = Mutex::new(None);

fn app() -> String {
    Some(crate::noticeboard::application_name()).filter(|n| !n.is_empty()).unwrap_or_else(|| "QGroundControl".to_string())
}

pub fn advance(status: &Status, message: &str) -> Option<Status> {
    let text = message.strip_prefix(CAL_PREFIX)?;
    let say = |highlight: &str, rest: String, running: bool| Some(Status { highlight: highlight.to_string(), text: rest, running, ..status.clone() });
    if let Some(started) = text.strip_prefix("calibration started: ") {
        if started.split(' ').count() != 2 {
            return say(FAILED, format!("{} cannot perform ESC Calibration with this version of firmware. You will need to upgrade to a newer firmware.", app()), false);
        }
    }
    match text {
        "Connect battery now" => say("WARNING: Props must be removed from vehicle prior to performing ESC calibration.", " Connect the battery now and calibration will begin.".to_string(), true),
        "Battery connected" => say("", "Performing calibration. This will take a few seconds..".to_string(), true),
        _ => match (text.strip_prefix("calibration failed: "), text.starts_with("calibration done:"), text.strip_prefix("config warning: ")) {
            (Some(reason), _, _) if reason.starts_with("Disconnect battery") => {
                say(FAILED, "You must disconnect the battery prior to performing ESC Calibration. Disconnect your battery and try again.".to_string(), false)
            }
            (Some(reason), _, _) => say(FAILED, reason.to_string(), false),
            (None, true, _) => say("", "Calibration complete. You can disconnect your battery now if you like.".to_string(), false),
            (None, false, Some(warning)) => Some(Status { warnings: [status.warnings.clone(), vec![warning.to_string()]].concat(), ..status.clone() }),
            _ => None,
        },
    }
}

pub fn on_text(vehicle: u8, message: &str) {
    let mut held = STATUS.lock().unwrap_or_else(PoisonError::into_inner);
    let next = held.as_ref().filter(|status| status.vehicle == vehicle && status.running).and_then(|status| advance(status, message));
    if next.is_some() {
        *held = next;
    }
}

pub fn esc_calibration_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    let held = STATUS.lock().unwrap_or_else(PoisonError::into_inner);
    json!({
        "kind": "object",
        "class": "EscCalibration",
        "open": held.is_some(),
        "running": held.as_ref().is_some_and(|s| s.running),
        "highlight": held.as_ref().map(|s| s.highlight.clone()),
        "text": held.as_ref().map(|s| s.text.clone()),
        "warnings": held.as_ref().map(|s| s.warnings.clone()).unwrap_or_default(),
    })
}

pub fn owns(path: &str) -> bool {
    path == ESC_CAL_START || path == ESC_CAL_CLOSE
}

pub fn run(backend: &dyn Backend, path: &str) -> Value {
    match path {
        ESC_CAL_START => {
            let Some(vehicle) = crate::guided::active_id(backend).and_then(|id| u8::try_from(id).ok()) else {
                return json!({ "ok": false, "reason": "No vehicle is connected." });
            };
            *STATUS.lock().unwrap_or_else(PoisonError::into_inner) = Some(Status { vehicle, highlight: String::new(), text: "Starting ESC calibration...".to_string(), running: true, warnings: Vec::new() });
            let params = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, ESC_CAL_PARAM7];
            crate::guided::dispatch(backend, Some(json!({ "action": "mavlinkCommand", "command": crate::sensorcal::CMD_PREFLIGHT_CALIBRATION, "params": params })), crate::guided::active_id(backend), "", "[]")
        }
        _ => {
            *STATUS.lock().unwrap_or_else(PoisonError::into_inner) = None;
            json!({ "ok": true })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn px4_cal_texts_walk_the_dialog_like_power_component_controller() {
        let start = Status { vehicle: 1, text: "Starting ESC calibration...".into(), running: true, ..Status::default() };
        let connect = advance(&start, "[cal] Connect battery now").unwrap();
        assert!(connect.highlight.starts_with("WARNING: Props must be removed") && connect.running);
        let connected = advance(&connect, "[cal] Battery connected").unwrap();
        assert_eq!(connected.text, "Performing calibration. This will take a few seconds..");
        let warned = advance(&connected, "[cal] config warning: ESC 3 not responding").unwrap();
        assert_eq!(warned.warnings, ["ESC 3 not responding"]);
        let done = advance(&warned, "[cal] calibration done: esc").unwrap();
        assert!(!done.running && done.text.starts_with("Calibration complete"));
        let battery = advance(&start, "[cal] calibration failed: Disconnect battery and try again").unwrap();
        assert_eq!((battery.highlight.as_str(), battery.running), (FAILED, false));
        assert!(battery.text.starts_with("You must disconnect the battery"));
        assert_eq!(advance(&start, "[cal] calibration failed: timeout").unwrap().text, "timeout");
        assert_eq!(advance(&start, "Some other text"), None);
        assert_eq!(advance(&start, "[cal] calibration started: 2 esc"), None, "a well-formed start changes nothing");
        assert!(!advance(&start, "[cal] calibration started: 2").unwrap().running, "a start without the firmware revision fails as incorrect reporting");
    }
}
