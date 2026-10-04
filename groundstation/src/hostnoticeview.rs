use serde_json::{Value, json};

use crate::read::{integer, object};
use crate::router::Backend;

pub const DEPS: &[&str] = &[crate::noticeboard::NOTICES_CHANGED];
const NAVIGATION_NOTICE: &str = "navigation";

fn banner(title: &str, text: &str, app: &str) -> String {
    [title, text].iter().filter(|part| !part.trim().is_empty() && part.trim() != app).copied().collect::<Vec<_>>().join(" \u{b7} ")
}

pub fn host_notices_view(backend: &dyn Backend, args: &[String]) -> Value {
    let host = object(&backend.get_fields("host", "notices,dropped"));
    let app = crate::noticeboard::application_name();
    let through = args.first().and_then(|a| a.trim().parse::<i64>().ok()).unwrap_or(-1);
    let notices: Vec<Value> = host
        .get("notices")
        .and_then(Value::as_array)
        .map(|listed| {
            listed
                .iter()
                .filter_map(|n| {
                    let id = n.get("id")?.as_i64().filter(|id| *id >= 0)?;
                    let text = |key: &str| n.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
                    let kind = text("kind");
                    Some(json!({
                        "id": id,
                        "kind": kind,
                        "known": crate::contract::NOTICE_KINDS.contains(&kind.as_str()),
                        "title": text("title"),
                        "text": text("text"),
                        "banner": banner(&text("title"), &text("text"), &app),
                    }))
                })
                .collect()
        })
        .unwrap_or_default();
    let after: Vec<&Value> = notices.iter().filter(|n| n["id"].as_i64().unwrap_or(-1) > through).collect();
    let destination = after.iter().rev().find(|n| n["kind"] == NAVIGATION_NOTICE).and_then(|n| n["title"].as_str()).filter(|t| !t.trim().is_empty());
    let banners: Vec<String> = after
        .iter()
        .filter(|n| n["kind"] != NAVIGATION_NOTICE && n["kind"] != crate::noticeboard::MESSAGE)
        .filter_map(|n| n["banner"].as_str())
        .filter(|line| !line.is_empty())
        .fold(Vec::new(), |kept, line| if kept.iter().any(|k: &String| k == line) { kept } else { kept.into_iter().chain(std::iter::once(line.to_string())).collect() });
    let dialogs: Vec<Value> = after
        .iter()
        .filter(|n| n["kind"] == crate::noticeboard::MESSAGE)
        .map(|n| {
            let title = n["title"].as_str().filter(|t| !t.trim().is_empty()).map(str::to_string).unwrap_or_else(|| app.clone());
            let text = n["text"].as_str().unwrap_or_default();
            match text {
                crate::noticeboard::REBOOT_VEHICLE_TEXT => json!({ "title": title, "text": format!("{text} Click Ok to reboot the vehicle now."), "action": "rebootVehicle" }),
                crate::connectnotices::SETUP_INCOMPLETE => json!({ "title": title, "text": text, "action": "openSetup" }),
                _ => json!({ "title": title, "text": text, "action": "" }),
            }
        })
        .filter(|d| !d["text"].as_str().unwrap_or_default().is_empty())
        .fold(Vec::new(), |kept, dialog| if kept.contains(&dialog) { kept } else { kept.into_iter().chain(std::iter::once(dialog)).collect() });
    json!({
        "kind": "object",
        "class": "HostNotices",
        "available": host.get("kind").and_then(Value::as_str) == Some("object"),
        "acknowledgedThrough": through,
        "latestId": notices.iter().filter_map(|n| n["id"].as_i64()).max(),
        "dropped": integer(&host, "dropped").unwrap_or(0),
        "notices": notices,
        "unseen": after,
        "destination": destination,
        "banners": banners,
        "dialogs": dialogs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Host(Value);
    impl Backend for Host {
        fn get(&self, p: &str) -> String { self.get_fields(p, "") }
        fn get_fields(&self, _p: &str, _f: &str) -> String { self.0.to_string() }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_notice_rules_are_the_ones_main_activity_applied() {
        let host = Host(json!({ "kind": "object", "dropped": 2, "notices": [
            { "id": 3, "kind": "message", "title": "Old", "text": "" },
            { "id": 4, "kind": "vehicleError", "title": "Battery", "text": "Low voltage" },
            { "id": 5, "kind": "navigation", "title": "plan", "text": "" },
            { "id": 6, "kind": "vehicleError", "title": "Battery", "text": "Low voltage" },
            { "id": 7, "kind": "banner", "title": "", "text": "Unrecognised kinds are shown, not guessed at" },
            { "id": -1, "kind": "message", "title": "no id", "text": "" },
        ] }));
        let view = host_notices_view(&host, &["3".to_string()]);
        let unseen: Vec<i64> = view["unseen"].as_array().unwrap().iter().map(|n| n["id"].as_i64().unwrap()).collect();
        assert_eq!(unseen, vec![4, 5, 6, 7], "only notices after the acknowledged id, and a notice without an id is dropped");
        assert_eq!(view["destination"], "plan");
        assert_eq!(view["banners"], json!(["Battery \u{b7} Low voltage", "Unrecognised kinds are shown, not guessed at"]), "one banner per distinct line, and navigation is a destination rather than a banner");
        assert_eq!(banner("Aircast QGC", "EKF variance", "Aircast QGC"), "EKF variance", "the application's own name says nothing about which vehicle or part spoke");
        assert_eq!((&view["latestId"], &view["dropped"]), (&json!(7), &json!(2)));
        assert_eq!(view["notices"][4]["known"], false);
        let fresh = host_notices_view(&host, &[]);
        assert_eq!(fresh["unseen"].as_array().unwrap().len(), 5, "no argument means nothing has been acknowledged yet");
        assert_eq!(fresh["banners"].as_array().unwrap().iter().any(|b| b == "Old"), false, "an app message is a dialog, not a banner");
        assert_eq!(host_notices_view(&Host(json!({ "kind": "null" })), &[])["available"], false);
    }

    #[test]
    fn app_messages_become_ok_dialogs_titled_with_the_app_name_like_show_app_message() {
        let host = Host(json!({ "kind": "object", "dropped": 0, "notices": [
            { "id": 1, "kind": "message", "title": "", "text": "Parameters could not be loaded" },
            { "id": 2, "kind": "message", "title": "", "text": "Parameters could not be loaded" },
            { "id": 3, "kind": "message", "title": "Links", "text": "Connect not allowed" },
            { "id": 4, "kind": "vehicleError", "title": "", "text": "EKF failure" },
        ] }));
        let view = host_notices_view(&host, &[]);
        let app = crate::noticeboard::application_name();
        assert_eq!(view["dialogs"], json!([{ "title": app, "text": "Parameters could not be loaded", "action": "" }, { "title": "Links", "text": "Connect not allowed", "action": "" }]));
        assert_eq!(view["banners"], json!(["EKF failure"]));
    }

    #[test]
    fn a_reboot_notice_asks_to_reboot_the_vehicle_like_show_reboot_vehicle_dialog() {
        let host = Host(json!({ "kind": "object", "dropped": 0, "notices": [
            { "id": 1, "kind": "message", "title": "", "text": crate::noticeboard::REBOOT_VEHICLE_TEXT },
        ] }));
        let dialog = &host_notices_view(&host, &[])["dialogs"][0];
        assert_eq!((dialog["text"].as_str(), dialog["action"].as_str()), (Some("Reboot vehicle for changes to take effect. Click Ok to reboot the vehicle now."), Some("rebootVehicle")));
    }

    #[test]
    fn the_setup_incomplete_notice_offers_to_open_setup() {
        let host = Host(json!({ "kind": "object", "dropped": 0, "notices": [
            { "id": 1, "kind": "message", "title": "", "text": crate::connectnotices::SETUP_INCOMPLETE },
        ] }));
        assert_eq!(host_notices_view(&host, &[])["dialogs"][0]["action"], "openSetup");
    }
}
