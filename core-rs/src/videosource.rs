use serde_json::{Value, json};

use crate::read::{flag, integer, object};
use crate::router::Backend;
use crate::video::slot_flag;

struct Sources {
    count: usize,
    active: Option<i64>,
    configured: Vec<bool>,
}

fn sources(backend: &dyn Backend) -> Sources {
    let video = object(&backend.get_fields("video", "cameraStatuses,activeVideoSource"));
    let count = video.get("cameraStatuses").and_then(Value::as_array).map_or(0, Vec::len);
    Sources {
        count,
        active: integer(&video, "activeVideoSource"),
        configured: (0..count).map(|slot| slot_flag(backend, "settings.videoSettings.sourceEnabled", slot) && slot_flag(backend, "settings.videoSettings.sourceConfigured", slot)).collect(),
    }
}

fn set_refusal(slot: Option<i64>, state: &Sources) -> Option<(&'static str, String)> {
    let Some(slot) = slot else {
        return Some(("malformed", "A video source is chosen by its slot number.".to_string()));
    };
    match () {
        _ if state.count == 0 => Some(("noSources", "There are no video sources.".to_string())),
        _ if slot < 0 || slot as usize >= state.count => Some(("noSuchSource", format!("Video sources run from 0 to {}.", state.count - 1))),
        _ if slot > 0 && !state.configured[slot as usize] => Some(("notConfigured", format!("Camera {} is not set up.", slot + 1))),
        _ => None,
    }
}

pub fn set(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let slot = serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0)?.as_f64()).filter(|v| v.fract() == 0.0).map(|v| v as i64);
    let before = sources(backend);
    if let Some((token, reason)) = set_refusal(slot, &before) {
        return json!({ "ok": false, "refusal": token, "reason": reason });
    }
    if before.active == slot {
        return json!({ "ok": true, "refusal": Value::Null, "activeSource": slot, "unchanged": true, "reason": Value::Null });
    }
    let dispatched = flag(&object(&backend.invoke(path, &json!([slot]).to_string())), "ok");
    let now = sources(backend).active;
    let took = dispatched && now == slot;
    json!({
        "ok": took,
        "refusal": Value::Null,
        "activeSource": now,
        "unchanged": false,
        "reason": match took { true => Value::Null, false => json!("The video did not switch to that source.") },
    })
}

pub fn switch(backend: &dyn Backend, path: &str) -> Value {
    let before = sources(backend);
    if 1 + before.configured.iter().skip(1).filter(|c| **c).count() <= 1 {
        return json!({ "ok": false, "refusal": "nothingToSwitch", "reason": "There is only one video source to show." });
    }
    let dispatched = flag(&object(&backend.invoke(path, "[]")), "ok");
    let now = sources(backend).active;
    let moved = dispatched && now != before.active;
    json!({
        "ok": moved,
        "refusal": Value::Null,
        "activeSource": now,
        "reason": match moved { true => Value::Null, false => json!("The video stayed on the same source.") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Video {
        active: RefCell<i64>,
        configured: Vec<bool>,
        obeys: bool,
    }

    impl Backend for Video {
        fn get(&self, _p: &str) -> String { String::new() }
        fn get_fields(&self, _p: &str, _f: &str) -> String {
            json!({ "kind": "object", "cameraStatuses": vec!["ok"; self.configured.len()], "activeVideoSource": *self.active.borrow() }).to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, p: &str, a: &str) -> String {
            if p.starts_with("settings.videoSettings.") {
                let slot = object(a)[0].as_u64().unwrap_or(99) as usize;
                return json!({ "ok": true, "result": self.configured.get(slot).copied().unwrap_or(false) }).to_string();
            }
            if self.obeys {
                let next = match p.ends_with("switchActiveVideoSource") {
                    true => (*self.active.borrow() + 1) % self.configured.len() as i64,
                    false => object(a)[0].as_i64().unwrap(),
                };
                *self.active.borrow_mut() = next;
            }
            json!({ "ok": true }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn a_video_source_is_chosen_only_from_the_cameras_that_are_set_up() {
        let video = Video { active: RefCell::new(0), configured: vec![true, true, false], obeys: true };
        let state = sources(&video);
        assert_eq!(set_refusal(Some(1), &state), None);
        assert_eq!(set_refusal(Some(5), &state).map(|r| r.0), Some("noSuchSource"), "setActiveVideoSource clamps 5 to the last slot without a word, so a head asking for a camera that does not exist is shown a different one");
        assert_eq!(set_refusal(Some(2), &state).map(|r| r.0), Some("notConfigured"));
        assert_eq!(set(&video, "video.setActiveVideoSource", "[0]")["unchanged"], true);
        let moved = set(&video, "video.setActiveVideoSource", "[1]");
        assert_eq!((&moved["ok"], &moved["activeSource"]), (&json!(true), &json!(1)));
        assert_eq!(switch(&video, "video.switchActiveVideoSource")["ok"], true);

        let primary_unset = Video { active: RefCell::new(0), configured: vec![false, true], obeys: true };
        assert_eq!(switch(&primary_unset, "video.switchActiveVideoSource")["ok"], true, "switchableIndices always holds slot 0 and adds each configured extra, so the primary's own setup is not what decides whether there is something to switch to");
        assert_eq!(set_refusal(Some(0), &sources(&primary_unset)), None, "and the primary slot is always selectable");
        let single = Video { active: RefCell::new(0), configured: vec![true, false], obeys: true };
        assert_eq!(switch(&single, "video.switchActiveVideoSource")["refusal"], "nothingToSwitch", "switchActiveVideoSource returns in silence with one switchable source");
        let deaf = Video { active: RefCell::new(0), configured: vec![true, true], obeys: false };
        assert_eq!(set(&deaf, "video.setActiveVideoSource", "[1]")["ok"], false, "the active source is read back rather than assumed");
    }
}
