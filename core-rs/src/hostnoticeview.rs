use serde_json::{Value, json};

use crate::read::{integer, object};
use crate::router::Backend;

// MainActivity.kt read the host notice list raw and applied four rules of its own: notices after the
// id it had acknowledged, the destination the last navigation notice names, which notices become a
// banner (every kind but navigation), and one banner per distinct line. The rules are served here;
// the thirty-second repeat window stays with the head, because it is about what the head last drew.
pub const DEPS: &[&str] = &["host.notices", "host.dropped"];
const NAVIGATION_NOTICE: &str = "navigation";

fn banner(title: &str, text: &str) -> String {
    [title, text].iter().filter(|part| !part.trim().is_empty()).copied().collect::<Vec<_>>().join(" \u{b7} ")
}

pub fn host_notices_view(backend: &dyn Backend, args: &[String]) -> Value {
    let host = object(&backend.get_fields("host", "notices,dropped"));
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
                        "banner": banner(&text("title"), &text("text")),
                    }))
                })
                .collect()
        })
        .unwrap_or_default();
    let after: Vec<&Value> = notices.iter().filter(|n| n["id"].as_i64().unwrap_or(-1) > through).collect();
    let destination = after.iter().rev().find(|n| n["kind"] == NAVIGATION_NOTICE).and_then(|n| n["title"].as_str()).filter(|t| !t.trim().is_empty());
    let mut banners: Vec<String> = Vec::new();
    after.iter().filter(|n| n["kind"] != NAVIGATION_NOTICE).filter_map(|n| n["banner"].as_str()).for_each(|line| {
        if !line.is_empty() && !banners.iter().any(|kept| kept == line) {
            banners.push(line.to_string());
        }
    });
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
        assert_eq!((&view["latestId"], &view["dropped"]), (&json!(7), &json!(2)));
        assert_eq!(view["notices"][4]["known"], false);
        let fresh = host_notices_view(&host, &[]);
        assert_eq!(fresh["unseen"].as_array().unwrap().len(), 5, "no argument means nothing has been acknowledged yet");
        assert_eq!(host_notices_view(&Host(json!({ "kind": "null" })), &[])["available"], false);
    }
}
