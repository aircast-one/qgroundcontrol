use std::sync::{Mutex, PoisonError};

use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["settings.appSettings.logSavePath", "settings.appSettings.savePath", "vehicles.activeVehicleAvailable", "logDownload.requestingList", "logDownload.downloadingLogs", "logDownload.selectedCount", "logDownload.model", "vehicle.id"];

pub fn human_size(bytes: i64) -> String {
    const UNITS: &[&str] = &["bytes", "KB", "MB", "GB"];
    let step = match bytes <= 0 {
        true => 0,
        false => ((bytes as f64).ln() / 1024f64.ln()).floor().min((UNITS.len() - 1) as f64) as usize,
    };
    match step {
        0 => format!("{bytes} bytes"),
        s => format!("{:.1} {}", bytes as f64 / 1024f64.powi(s as i32), UNITS[s]),
    }
}

const CLOCK_SET_YEAR: i64 = 2010;

pub fn time_state(received: bool, raw: &str) -> &'static str {
    let year = raw.get(..4).and_then(|y| y.parse::<i64>().ok());
    match (received, year) {
        (false, _) => "unreceived",
        (true, Some(y)) if y >= CLOCK_SET_YEAR => "known",
        _ => "unknown",
    }
}

pub fn erase_warning(count: usize) -> String {
    match count {
        1 => "The one log on the vehicle will be deleted. If you have not downloaded it, it is gone for good.".to_string(),
        n => format!("All {n} logs will be deleted from the vehicle. Anything you have not downloaded is gone for good."),
    }
}

pub fn empty_text(connected: bool, requesting: bool, answered: bool) -> &'static str {
    match (requesting, connected, answered) {
        (true, ..) => "Asking the vehicle for its logs\u{2026}",
        (false, false, _) => "Connect a vehicle to list its logs.",
        (false, true, true) => "This vehicle has no logs.",
        (false, true, false) => "No logs listed yet. Refresh to ask the vehicle.",
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Idle,
    Asking,
    Answered,
}

static ASKED: Mutex<Option<(i64, Phase)>> = Mutex::new(None);

fn advance(previous: Phase, requesting: bool) -> Phase {
    match (requesting, previous) {
        (true, _) => Phase::Asking,
        (false, Phase::Idle) => Phase::Idle,
        (false, _) => Phase::Answered,
    }
}

fn answered(vehicle: Option<i64>, requesting: bool) -> bool {
    let Some(id) = vehicle else {
        return false;
    };
    let mut asked = ASKED.lock().unwrap_or_else(PoisonError::into_inner);
    let previous = asked.filter(|(known, _)| *known == id).map_or(Phase::Idle, |(_, phase)| phase);
    let phase = advance(previous, requesting);
    *asked = Some((id, phase));
    phase == Phase::Answered
}

pub fn logs_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let connected = flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
    let root = object(&backend.get_fields("logDownload", "requestingList,downloadingLogs"));
    let saving = object(&backend.get_fields("settings.appSettings", "logSavePath,savePath"));
    let save_path = saving.get("logSavePath").and_then(Value::as_str).unwrap_or("").to_string();
    let chosen = crate::read::text(saving.get("savePath").unwrap_or(&Value::Null), "valueString");
    let requesting = flag(&root, "requestingList");
    let vehicle = object(&backend.get("vehicle.id")).get("value").and_then(Value::as_i64);
    let downloading = flag(&root, "downloadingLogs");
    let entries: Vec<Value> = object(&backend.get("logDownload.model"))
        .get("elements")
        .and_then(Value::as_array)
        .map(|elements| {
            elements
                .iter()
                .enumerate()
                .filter_map(|(index, e)| {
                    let id = e.get("id").and_then(Value::as_i64)?;
                    let size = e.get("size").and_then(Value::as_i64).unwrap_or(0);
                    let time = e.get("time").and_then(Value::as_str).unwrap_or("");
                    Some(json!({
                        "index": index,
                        "id": id,
                        "sizeBytes": size,
                        "sizeText": human_size(size),
                        "status": e.get("status").and_then(Value::as_str).unwrap_or(""),
                        "statusId": e.get("statusId").and_then(Value::as_str).unwrap_or(""),
                        "received": flag(e, "received"),
                        "selected": flag(e, "selected"),
                        "time": time,
                        "timeState": time_state(flag(e, "received"), time),
                    }))
                })
                .collect()
        })
        .unwrap_or_default();
    let busy = requesting || downloading;
    json!({
        "kind": "object",
        "class": "Logs",
        "connected": connected,
        "savePath": save_path,
        "savePathReason": match (save_path.is_empty(), chosen.trim().is_empty()) {
            (false, _) => Value::Null,
            (true, true) => json!("notChosen"),
            (true, false) => json!("missing"),
        },
        "requestingList": requesting,
        "downloading": downloading,
        "busy": busy,
        // These three all send MAVLink and LogDownloadPage.qml gates only the first on a vehicle,
        // which is where the core took them from: eraseAll returns on a null vehicle with nothing
        // but a log line, so an operator confirms a destructive action and is told nothing happened.
        // A selection is deliberately NOT a term here. download() with none selected is a no-op,
        // but AnalyzeWindow.swift offers Download per row and selects inside the action, so gating
        // on a selection that only exists after the click disables every button permanently.
        // That precondition belongs to the call, not to the view.
        "canRefresh": connected && !busy,
        "canDownload": connected && !busy,
        "canCancel": busy,
        "canErase": connected && !entries.is_empty() && !busy,
        "anyDownloaded": entries.iter().any(|e| e["statusId"] == "downloaded"),
        "emptyText": empty_text(connected, requesting, answered(vehicle, requesting)),
        "eraseWarning": erase_warning(entries.len()),
        "entries": entries,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Refresh,
    Download,
    Cancel,
    EraseAll,
}

#[derive(Clone, Copy, Debug, Default)]
struct Controller {
    connected: bool,
    busy: bool,
    entries: usize,
    selected: usize,
    save_path: bool,
}

fn refusal(action: Action, state: Controller) -> Option<(&'static str, &'static str)> {
    match action {
        Action::Cancel if !state.busy => Some(("idle", "Nothing is being listed or downloaded.")),
        Action::Cancel => None,
        _ if !state.connected => Some(("noVehicle", "No vehicle is connected.")),
        _ if state.busy => Some(("busy", "Wait for the current list or download to finish, or cancel it.")),
        Action::Download if state.selected == 0 => Some(("nothingSelected", "Select at least one log to download.")),
        Action::Download if !state.save_path => Some(("noSavePath", "Choose a folder to save logs in.")),
        Action::EraseAll if state.entries == 0 => Some(("noLogs", "The vehicle has no logs listed to erase.")),
        _ => None,
    }
}

pub fn act(backend: &dyn Backend, action: Action, path: &str, args: &str) -> Value {
    let chosen = serde_json::from_str::<Value>(args).ok().and_then(|a| a.as_array()?.first()?.as_str().map(str::to_string)).filter(|p| !p.trim().is_empty());
    let root = object(&backend.get_fields("logDownload", "requestingList,downloadingLogs,selectedCount"));
    let saving = object(&backend.get_fields("settings.appSettings", "logSavePath"));
    let state = Controller {
        connected: flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable"),
        busy: flag(&root, "requestingList") || flag(&root, "downloadingLogs"),
        entries: object(&backend.get("logDownload.model")).get("elements").and_then(Value::as_array).map_or(0, Vec::len),
        selected: root.get("selectedCount").and_then(Value::as_u64).unwrap_or(0) as usize,
        save_path: chosen.is_some() || saving.get("logSavePath").and_then(Value::as_str).is_some_and(|p| !p.is_empty()),
    };
    if let Some((token, reason)) = refusal(action, state) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    let args = match (action, chosen) {
        (Action::Download, Some(folder)) => json!([folder]).to_string(),
        _ => "[]".to_string(),
    };
    let dispatched = flag(&object(&backend.invoke(path, &args)), "ok");
    json!({
        "ok": dispatched,
        "refusal": Value::Null,
        "reason": match dispatched { true => Value::Null, false => json!("The log controller did not take the request.") },
    })
}

pub fn selection_index(path: &str) -> Option<usize> {
    path.strip_prefix("logDownload.model.")?.strip_suffix(".selected")?.parse().ok()
}

pub fn write_selected(backend: &dyn Backend, path: &str, value: &str) -> Value {
    let refused = |token: &str, reason: String| json!({ "ok": false, "result": false, "refusal": token, "reason": reason });
    let (Some(index), Some(on)) = (selection_index(path), serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value")?.as_bool())) else {
        return refused("malformed", "A log is selected with true and cleared with false.".to_string());
    };
    let root = object(&backend.get_fields("logDownload", "requestingList,downloadingLogs"));
    if flag(&root, "downloadingLogs") || flag(&root, "requestingList") {
        return refused("busy", "Wait for the current list or download to finish before changing the selection.".to_string());
    }
    let count = |model: &Value| model.get("elements").and_then(Value::as_array).map_or(0, Vec::len);
    let model = object(&backend.get("logDownload.model"));
    if index >= count(&model) {
        return refused("noSuchLog", format!("There is no log at position {index}."));
    }
    let answered = flag(&object(&backend.set(path, &json!({ "value": on }).to_string())), "ok");
    let held = object(&backend.get("logDownload.model")).get("elements").and_then(|e| e.get(index)).map(|e| flag(e, "selected"));
    let took = answered && held == Some(on);
    json!({ "ok": took, "result": took, "refusal": Value::Null, "reason": match took { true => Value::Null, false => json!("The log list did not keep that selection.") } })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_read_like_a_person_wrote_them_and_times_name_their_branch() {
        assert_eq!(human_size(0), "0 bytes");
        assert_eq!(human_size(900), "900 bytes");
        assert_eq!(human_size(4096), "4.0 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(time_state(true, "2026-09-08T14:42:51.000"), "known");
        assert_eq!(time_state(true, "1970-01-01T00:00:12.000"), "unknown", "a clock that was never set reads as the QGC page's Date Unknown");
        assert_eq!(time_state(false, "2026-09-08T14:42:51.000"), "unreceived", "an entry not yet received shows no time at all");
        assert_eq!(time_state(true, "garbage"), "unknown");
        assert_eq!(time_state(true, ""), "unknown");
        assert!(erase_warning(1).starts_with("The one log"));
        assert!(erase_warning(3).starts_with("All 3 logs"));
    }

    #[test]
    fn a_vehicle_that_answered_with_no_logs_is_not_a_vehicle_nobody_asked() {
        assert_eq!(empty_text(false, false, false), "Connect a vehicle to list its logs.");
        assert_eq!(empty_text(true, true, false), "Asking the vehicle for its logs\u{2026}");
        assert_eq!(empty_text(true, false, false), "No logs listed yet. Refresh to ask the vehicle.");
        assert_eq!(empty_text(true, false, true), "This vehicle has no logs.", "one line stood for both, so an operator whose vehicle had answered was told to redo the request that had already produced the true answer");

        assert_eq!(advance(Phase::Idle, false), Phase::Idle, "a vehicle nobody has asked stays unasked however long it sits there - this is the branch that keeps the latch from claiming an answer it never heard");
        assert_eq!(advance(Phase::Idle, true), Phase::Asking);
        assert_eq!(advance(Phase::Asking, false), Phase::Answered, "requestingList going true then false IS the answer arriving; LogDownloadController clears it whether the vehicle listed ten logs or none");
        assert_eq!(advance(Phase::Answered, false), Phase::Answered);
        assert_eq!(advance(Phase::Answered, true), Phase::Asking, "a refresh puts it back to asking, so a second request that returns nothing does not read as still holding the first answer");
    }

    #[test]
    fn the_answered_latch_belongs_to_one_vehicle() {
        struct Named(i64, bool);
        impl Backend for Named {
            fn get(&self, path: &str) -> String {
                match path {
                    "vehicle.id" => json!({ "kind": "value", "value": self.0 }),
                    _ => json!({ "kind": "object", "elements": [] }),
                }
                .to_string()
            }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": true }),
                    "settings.appSettings" => json!({ "kind": "object", "logSavePath": "/Users/p/Logs" }),
                    _ => json!({ "kind": "object", "requestingList": self.1, "downloadingLogs": false }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        assert_eq!(logs_view(&Named(9001, false), &[])["emptyText"], "No logs listed yet. Refresh to ask the vehicle.", "a vehicle that has just connected has answered nothing");
        assert_eq!(logs_view(&Named(9001, true), &[])["emptyText"], "Asking the vehicle for its logs\u{2026}");
        assert_eq!(logs_view(&Named(9001, false), &[])["emptyText"], "This vehicle has no logs.");
        assert_eq!(
            logs_view(&Named(9002, false), &[])["emptyText"],
            "No logs listed yet. Refresh to ask the vehicle.",
            "the latch is a fact about one vehicle, and a second one connecting inherits nothing - without the id it would be told it has no logs on the strength of a request sent to a different aircraft"
        );
        assert_eq!(logs_view(&Named(9001, false), &[])["emptyText"], "No logs listed yet. Refresh to ask the vehicle.", "and coming back to the first vehicle does not resurrect the old answer either: the latch holds one vehicle, so a reconnect asks again rather than reporting what the last session heard");
    }

    #[test]
    fn the_buttons_follow_the_controller_state() {
        struct Fake { connected: bool, requesting: bool, entries: Value }
        impl Backend for Fake {
            fn get(&self, _p: &str) -> String { json!({ "kind": "object", "elements": self.entries }).to_string() }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": self.connected }),
                    "settings.appSettings" => json!({ "kind": "object", "logSavePath": "/Users/p/Logs" }),
                    _ => json!({ "kind": "object", "requestingList": self.requesting, "downloadingLogs": false }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let idle = logs_view(&Fake { connected: true, requesting: false, entries: json!([{ "id": 1, "size": 4096, "status": "Downloaded", "statusId": "downloaded", "received": true, "selected": false, "time": "2026-09-08T14:42:51.000" }]) }, &[]);
        assert_eq!(idle["canRefresh"], true);
        assert_eq!(idle["canDownload"], true, "the head selects the row inside download(), so requiring a selection the click has not made yet would disable every Download button there is");
        assert_eq!(idle["canErase"], true);
        assert_eq!(idle["anyDownloaded"], true);

        let german = logs_view(&Fake { connected: true, requesting: false, entries: json!([{ "id": 1, "size": 4096, "status": "Heruntergeladen", "statusId": "downloaded", "received": true, "selected": false, "time": "2026-09-08T14:42:51.000" }]) }, &[]);
        assert_eq!(german["anyDownloaded"], true, "LogDownloadController wraps every status in tr(), so comparing against the English word made this permanently false outside English and whatever it gates never appeared");
        assert_eq!(german["entries"][0]["statusId"], "downloaded", "the id travels beside the text so a head can tell Available from Error without reading either word");
        assert_eq!(idle["entries"][0]["sizeText"], "4.0 KB");
        assert_eq!(idle["entries"][0]["timeState"], "known");
        assert!(idle["entries"][0].get("timeText").is_none(), "the head renders the time in its own locale");
        let asking = logs_view(&Fake { connected: true, requesting: true, entries: json!([]) }, &[]);
        assert_eq!(asking["canRefresh"], false);
        assert_eq!(asking["canDownload"], false);
        assert_eq!(asking["canCancel"], true);
        assert_eq!(asking["emptyText"], "Asking the vehicle for its logs\u{2026}");
        let none = logs_view(&Fake { connected: false, requesting: false, entries: json!([]) }, &[]);
        assert_eq!(none["emptyText"], "Connect a vehicle to list its logs.");
        assert_eq!(none["canErase"], false);

        // This is the case the old assertion above passed for the wrong reason: it read false
        // because the list was empty, never because there was no vehicle to erase from.
        let dropped = logs_view(&Fake { connected: false, requesting: false, entries: json!([{ "id": 1, "size": 4096, "status": "Available", "statusId": "available", "received": true, "selected": true, "time": "2026-09-08T14:42:51.000" }]) }, &[]);
        assert_eq!(dropped["canErase"], false, "eraseAll returns on a null vehicle after a log line and nothing else, so offering it means the operator confirms a destructive action and is told nothing happened");
        assert_eq!(dropped["canDownload"], false, "and the same window offers a download whose every byte would have to come from the vehicle that is gone");
        assert_eq!(dropped["canCancel"], false);
    }
    #[test]
    fn every_log_action_says_why_it_would_have_done_nothing() {
        let ready = Controller { connected: true, busy: false, entries: 3, selected: 1, save_path: true };
        let token = |action, state| refusal(action, state).map(|(t, _)| t);
        [Action::Refresh, Action::Download, Action::EraseAll].iter().for_each(|a| assert_eq!(token(*a, ready), None, "{a:?}"));
        assert_eq!(token(Action::Refresh, Controller { connected: false, ..ready }), Some("noVehicle"));
        assert_eq!(token(Action::EraseAll, Controller { connected: false, ..ready }), Some("noVehicle"), "eraseAll returns on a null vehicle after a log line, so the operator confirmed a destructive action and heard nothing");
        assert_eq!(token(Action::Refresh, Controller { busy: true, ..ready }), Some("busy"), "refresh() returns without a word while a transfer holds pointers into the model");
        assert_eq!(token(Action::Download, Controller { selected: 0, ..ready }), Some("nothingSelected"), "download() with nothing selected is a no-op");
        assert_eq!(token(Action::Download, Controller { save_path: false, ..ready }), Some("noSavePath"), "_downloadToDirectory returns on an empty path after clearing the download state");
        assert_eq!(token(Action::EraseAll, Controller { entries: 0, ..ready }), Some("noLogs"));
        assert_eq!(token(Action::Cancel, ready), Some("idle"));
        assert_eq!(token(Action::Cancel, Controller { connected: false, busy: true, ..ready }), None, "a cancel has to reach a transfer whose vehicle has already gone");
    }

    #[test]
    fn a_refused_log_action_never_reaches_qt_and_an_accepted_one_carries_its_folder() {
        use std::cell::RefCell;
        struct Recording { selected: u64, save: &'static str, calls: RefCell<Vec<(String, String)>> }
        impl Backend for Recording {
            fn get(&self, _p: &str) -> String { json!({ "kind": "object", "elements": [{ "id": 1 }] }).to_string() }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": true }),
                    "settings.appSettings" => json!({ "kind": "object", "logSavePath": self.save }),
                    _ => json!({ "kind": "object", "requestingList": false, "downloadingLogs": false, "selectedCount": self.selected }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, p: &str, a: &str) -> String {
                self.calls.borrow_mut().push((p.to_string(), a.to_string()));
                json!({ "ok": true }).to_string()
            }
            fn watch(&self, _p: &[String]) {}
        }
        let empty = Recording { selected: 0, save: "/Users/p/Logs", calls: RefCell::new(Vec::new()) };
        let refused = act(&empty, Action::Download, "logDownload.download", "[]");
        assert_eq!((&refused["ok"], &refused["refusal"]), (&json!(false), &json!("nothingSelected")));
        assert!(empty.calls.borrow().is_empty());

        let unsaved = Recording { selected: 2, save: "", calls: RefCell::new(Vec::new()) };
        assert_eq!(act(&unsaved, Action::Download, "logDownload.download", "[]")["refusal"], "noSavePath");
        let taken = act(&unsaved, Action::Download, "logDownload.download", r#"["/Volumes/Card"]"#);
        assert_eq!((&taken["ok"], &taken["reason"]), (&json!(true), &Value::Null), "a folder passed with the call stands in for an unset setting, which is what download(path) does");
        assert_eq!(unsaved.calls.borrow().as_slice(), &[("logDownload.download".to_string(), r#"["/Volumes/Card"]"#.to_string())]);

        let erase = Recording { selected: 0, save: "", calls: RefCell::new(Vec::new()) };
        assert_eq!(act(&erase, Action::EraseAll, "logDownload.eraseAll", "[]")["ok"], true);
        assert_eq!(erase.calls.borrow()[0], ("logDownload.eraseAll".to_string(), "[]".to_string()));
    }

    #[test]
    fn an_unusable_save_path_says_which_kind_of_unusable_it_is() {
        struct Saving(&'static str, &'static str);
        impl Backend for Saving {
            fn get(&self, _p: &str) -> String { json!({ "kind": "object", "elements": [] }).to_string() }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": true }),
                    "settings.appSettings" => json!({ "kind": "object", "logSavePath": self.0, "savePath": { "kind": "fact", "valueString": self.1 } }),
                    _ => json!({ "kind": "object", "requestingList": false, "downloadingLogs": false }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        let usable = logs_view(&Saving("/Users/p/Docs/Logs", "/Users/p/Docs"), &[]);
        assert_eq!(usable["savePath"], "/Users/p/Docs/Logs", "LogDownload.swift read this off settings.appSettings while view.logs already served every other thing that page draws");
        assert_eq!(usable["savePathReason"], Value::Null);

        assert_eq!(logs_view(&Saving("", ""), &[])["savePathReason"], "notChosen", "AppSettings::logSavePath returns an empty QString both when nothing was ever chosen and when the chosen directory has gone, and an operator acts on those differently: pick a folder, versus the folder you picked is missing");
        assert_eq!(logs_view(&Saving("", "/Volumes/Card/QGC"), &[])["savePathReason"], "missing", "the setting still names a directory, so it was chosen and has since disappeared");
    }


    #[test]
    fn a_log_is_selected_only_where_there_is_one_and_not_mid_transfer() {
        use std::cell::RefCell;
        struct Model(RefCell<Vec<bool>>, bool);
        impl Backend for Model {
            fn get(&self, _p: &str) -> String { json!({ "kind": "object", "elements": self.0.borrow().iter().map(|s| json!({ "selected": s })).collect::<Vec<_>>() }).to_string() }
            fn get_fields(&self, _p: &str, _f: &str) -> String { json!({ "kind": "object", "downloadingLogs": self.1, "requestingList": false }).to_string() }
            fn set(&self, p: &str, v: &str) -> String {
                let index = selection_index(p).unwrap();
                self.0.borrow_mut()[index] = object(v)["value"].as_bool().unwrap();
                json!({ "ok": true }).to_string()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let idle = Model(RefCell::new(vec![false, false]), false);
        assert_eq!(write_selected(&idle, "logDownload.model.1.selected", r#"{"value":true}"#)["result"], true);
        assert_eq!(write_selected(&idle, "logDownload.model.4.selected", r#"{"value":true}"#)["refusal"], "noSuchLog", "a stale row index reaches a model entry that is not there");
        assert_eq!(write_selected(&Model(RefCell::new(vec![false]), true), "logDownload.model.0.selected", r#"{"value":true}"#)["refusal"], "busy", "the selection is what download() works through, so changing it mid-transfer changes the transfer");
        assert_eq!(selection_index("logDownload.model.x.selected"), None);
        assert_eq!(selection_index("logDownload.model.3.status"), None);
    }
}
