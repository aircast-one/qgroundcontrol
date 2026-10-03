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
}

static STATUS: Mutex<Option<Status>> = Mutex::new(None);

pub fn advance(status: &Status, message: &str) -> Option<Status> {
    let text = message.strip_prefix(CAL_PREFIX)?;
    let say = |highlight: &str, rest: String, running: bool| Some(Status { highlight: highlight.to_string(), text: rest, running, ..status.clone() });
    match text {
        "Connect battery now" => say("WARNING: Props must be removed from vehicle prior to performing ESC calibration.", " Connect the battery now and calibration will begin.".to_string(), true),
        "Battery connected" => say("", "Performing calibration. This will take a few seconds..".to_string(), true),
        _ => match (text.strip_prefix("calibration failed: "), text.starts_with("calibration done:")) {
            (Some(reason), _) if reason.starts_with("Disconnect battery") => {
                say(FAILED, "You must disconnect the battery prior to performing ESC Calibration. Disconnect your battery and try again.".to_string(), false)
            }
            (Some(reason), _) => say(FAILED, reason.to_string(), false),
            (None, true) => say("", "Calibration complete. You can disconnect your battery now if you like.".to_string(), false),
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

const NO_VEHICLE: &str = "No vehicle is connected.";

pub fn esc_calibration_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    let gone = { STATUS.lock().unwrap_or_else(PoisonError::into_inner).as_ref().filter(|s| s.running).map(|s| s.vehicle) }.filter(|vehicle| crate::hub::lock().vehicle(*vehicle).is_none());
    if gone.is_some() {
        let mut held = STATUS.lock().unwrap_or_else(PoisonError::into_inner);
        *held = held.take().map(|s| Status { highlight: FAILED.to_string(), text: "The vehicle disconnected.".to_string(), running: false, ..s });
    }
    let held = STATUS.lock().unwrap_or_else(PoisonError::into_inner);
    json!({
        "kind": "object",
        "class": "EscCalibration",
        "open": held.is_some(),
        "running": held.as_ref().is_some_and(|s| s.running),
        "highlight": held.as_ref().map(|s| s.highlight.clone()),
        "text": held.as_ref().map(|s| s.text.clone()),
    })
}

pub fn owns(path: &str) -> bool {
    path == ESC_CAL_START || path == ESC_CAL_CLOSE
}

pub fn run(backend: &dyn Backend, path: &str) -> Value {
    match path {
        ESC_CAL_START => {
            let failed = |vehicle: u8, reason: &str| Some(Status { vehicle, highlight: FAILED.to_string(), text: reason.to_string(), running: false });
            let Some(vehicle) = crate::guided::active_id(backend).and_then(|id| u8::try_from(id).ok()) else {
                *STATUS.lock().unwrap_or_else(PoisonError::into_inner) = failed(0, NO_VEHICLE);
                return json!({ "ok": false, "reason": NO_VEHICLE });
            };
            *STATUS.lock().unwrap_or_else(PoisonError::into_inner) = Some(Status { vehicle, highlight: String::new(), text: "Starting ESC calibration...".to_string(), running: true });
            let params = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, ESC_CAL_PARAM7];
            let sent = crate::guided::dispatch(backend, Some(json!({ "action": "mavlinkCommand", "command": crate::sensorcal::CMD_PREFLIGHT_CALIBRATION, "params": params })), crate::guided::active_id(backend), "", "[]");
            if sent.get("ok").and_then(Value::as_bool) != Some(true) {
                *STATUS.lock().unwrap_or_else(PoisonError::into_inner) = failed(vehicle, sent.get("reason").and_then(Value::as_str).unwrap_or("The vehicle was not sent the calibration command."));
            }
            sent
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
    fn a_calibration_that_cannot_start_ends_as_a_failure_the_dialog_can_close() {
        struct Nothing;
        impl Backend for Nothing {
            fn get(&self, _: &str) -> String { json!({ "kind": "null" }).to_string() }
            fn get_fields(&self, _: &str, _: &str) -> String { json!({ "kind": "null" }).to_string() }
            fn set(&self, _: &str, _: &str) -> String { String::new() }
            fn invoke(&self, _: &str, _: &str) -> String { String::new() }
            fn watch(&self, _: &[String]) {}
        }
        assert_eq!(run(&Nothing, ESC_CAL_START)["ok"], false);
        let view = esc_calibration_view(&Nothing, &[]);
        assert_eq!((&view["open"], &view["running"], &view["highlight"]), (&json!(true), &json!(false), &json!(FAILED)), "OK is enabled only on an outcome, so a start that never happened has to be one");
        run(&Nothing, ESC_CAL_CLOSE);
        assert_eq!(esc_calibration_view(&Nothing, &[])["open"], false);
    }

    #[test]
    fn px4_cal_texts_walk_the_dialog_like_power_component_controller() {
        let start = Status { vehicle: 1, text: "Starting ESC calibration...".into(), running: true, ..Status::default() };
        let connect = advance(&start, "[cal] Connect battery now").unwrap();
        assert!(connect.highlight.starts_with("WARNING: Props must be removed") && connect.running);
        let connected = advance(&connect, "[cal] Battery connected").unwrap();
        assert_eq!(connected.text, "Performing calibration. This will take a few seconds..");
        assert!(advance(&connected, "[cal] config warning: ESC 3 not responding").is_none(), "ESCCalibrationDialog shows no config warnings");
        let done = advance(&connected, "[cal] calibration done: esc").unwrap();
        assert!(!done.running && done.text.starts_with("Calibration complete"));
        let battery = advance(&start, "[cal] calibration failed: Disconnect battery and try again").unwrap();
        assert_eq!((battery.highlight.as_str(), battery.running), (FAILED, false));
        assert!(battery.text.starts_with("You must disconnect the battery"));
        assert_eq!(advance(&start, "[cal] calibration failed: timeout").unwrap().text, "timeout");
        assert_eq!(advance(&start, "Some other text"), None);
        assert_eq!(advance(&start, "[cal] calibration started: 2 esc"), None, "a well-formed start changes nothing");
        assert_eq!(advance(&start, "[cal] calibration started: 2"), None, "PowerComponentController only emits incorrectFirmwareRevReporting, which no QML connects, so the calibration keeps listening");
    }
}
