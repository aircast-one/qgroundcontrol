use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "logDownload.requestingList", "logDownload.downloadingLogs", "logDownload.model"];

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

pub fn empty_text(connected: bool, requesting: bool) -> &'static str {
    match (requesting, connected) {
        (true, _) => "Asking the vehicle for its logs\u{2026}",
        (false, true) => "No logs listed yet. Refresh to ask the vehicle.",
        (false, false) => "Connect a vehicle to list its logs.",
    }
}

pub fn logs_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let connected = flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
    let root = object(&backend.get_fields("logDownload", "requestingList,downloadingLogs"));
    let requesting = flag(&root, "requestingList");
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
        "requestingList": requesting,
        "downloading": downloading,
        "busy": busy,
        "canRefresh": connected && !busy,
        "canDownload": !busy,
        "canCancel": busy,
        "canErase": !entries.is_empty() && !busy,
        "anyDownloaded": entries.iter().any(|e| e["statusId"] == "downloaded"),
        "emptyText": empty_text(connected, requesting),
        "eraseWarning": erase_warning(entries.len()),
        "entries": entries,
    })
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
    fn the_buttons_follow_the_controller_state() {
        struct Fake { connected: bool, requesting: bool, entries: Value }
        impl Backend for Fake {
            fn get(&self, _p: &str) -> String { json!({ "kind": "object", "elements": self.entries }).to_string() }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": self.connected }),
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
        assert_eq!(idle["canDownload"], true, "nothing selected and not busy still enables download, as the QGC page does");
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
        assert_eq!(asking["canDownload"], false, "download follows busy-ness alone, as the QGC page does");
        assert_eq!(asking["canCancel"], true);
        assert_eq!(asking["emptyText"], "Asking the vehicle for its logs\u{2026}");
        let none = logs_view(&Fake { connected: false, requesting: false, entries: json!([]) }, &[]);
        assert_eq!(none["emptyText"], "Connect a vehicle to list its logs.");
        assert_eq!(none["canErase"], false);
    }
}
