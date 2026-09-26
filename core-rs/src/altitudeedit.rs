use serde_json::{Value, json};

use crate::altitudemodes::{Inputs, modes, omitted, read_inputs};
use crate::read::{flag, integer, object};
use crate::router::Backend;

const FRAME: &str = "plan.missionController.globalAltitudeFrame";

fn frame_refusal(raw: Option<i64>, inputs: &Inputs) -> Option<(&'static str, String)> {
    let Some(raw) = raw else {
        return Some(("malformed", "An altitude mode is one of the raw values view.altitudeModes lists.".to_string()));
    };
    if let Some(gone) = omitted(inputs).into_iter().find(|m| m["raw"] == raw) {
        return Some(("notOffered", gone["reason"].as_str().unwrap_or("That altitude mode is not offered here.").to_string()));
    }
    match modes(inputs).into_iter().find(|m| m["raw"] == raw) {
        None => Some(("malformed", format!("{raw} is not an altitude mode."))),
        Some(mode) if mode["enabled"] != true => Some(("notYet", mode["reason"].as_str().unwrap_or("").to_string())),
        Some(_) => None,
    }
}

pub fn write_global(backend: &dyn Backend, value: &str) -> Value {
    let raw = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value")?.as_f64()).filter(|v| v.fract() == 0.0).map(|v| v as i64);
    let current = integer(&object(&backend.get_fields("plan.missionController", "globalAltitudeFrame")), "globalAltitudeFrame").unwrap_or(-1);
    if let Some((token, reason)) = frame_refusal(raw, &read_inputs(backend, true, current)) {
        return json!({ "ok": false, "result": false, "refusal": token, "reason": reason });
    }
    let raw = raw.unwrap_or_default();
    let answered = flag(&object(&backend.set(FRAME, &json!({ "value": raw }).to_string())), "ok");
    let held = integer(&object(&backend.get_fields("plan.missionController", "globalAltitudeFrame")), "globalAltitudeFrame");
    let took = answered && held == Some(raw);
    json!({
        "ok": took,
        "result": took,
        "refusal": Value::Null,
        "reason": match took { true => Value::Null, false => json!("The plan did not keep that altitude mode.") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::altitudemodes::{ABSOLUTE, CALC_ABOVE_TERRAIN, MIXED, RELATIVE, TERRAIN_FRAME};
    use std::cell::RefCell;

    #[test]
    fn a_plan_altitude_mode_is_written_only_when_the_picker_would_offer_it() {
        let inputs = Inputs { mission: true, current: RELATIVE, holds_altitude_above_terrain: false, has_items: true, show_absolute: true };
        let token = |raw, i: &Inputs| frame_refusal(Some(raw), i).map(|r| r.0);
        assert_eq!(token(ABSOLUTE, &inputs), None);
        assert_eq!(token(MIXED, &inputs), None);
        assert_eq!(token(CALC_ABOVE_TERRAIN, &inputs), None);
        assert_eq!(token(TERRAIN_FRAME, &inputs), Some("notOffered"), "a vehicle that cannot hold an altitude above terrain is never given a plan measured in that frame");
        assert_eq!(token(ABSOLUTE, &Inputs { show_absolute: false, ..inputs }), Some("notOffered"));
        assert_eq!(token(ABSOLUTE, &Inputs { has_items: false, ..inputs }), Some("notYet"));
        assert_eq!(token(9, &inputs), Some("malformed"));
        assert_eq!(frame_refusal(None, &inputs).map(|r| r.0), Some("malformed"));
    }

    #[test]
    fn the_write_lands_on_the_property_upstream_renamed_it_to() {
        struct Plan(RefCell<i64>, RefCell<Vec<String>>);
        impl Backend for Plan {
            fn get(&self, p: &str) -> String { self.get_fields(p, "") }
            fn get_fields(&self, p: &str, _f: &str) -> String {
                match p {
                    "plan.missionController" => json!({ "kind": "object", "containsItems": true, "globalAltitudeFrame": *self.0.borrow() }),
                    "corePlugin.options" => json!({ "kind": "object", "showMissionAbsoluteAltitude": true }),
                    _ => json!({ "kind": "object" }),
                }
                .to_string()
            }
            fn set(&self, p: &str, v: &str) -> String {
                self.1.borrow_mut().push(p.to_string());
                *self.0.borrow_mut() = object(v)["value"].as_i64().unwrap();
                json!({ "ok": true }).to_string()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let plan = Plan(RefCell::new(RELATIVE), RefCell::new(Vec::new()));
        let written = write_global(&plan, r#"{"value":2}"#);
        assert_eq!((&written["ok"], &written["result"]), (&json!(true), &json!(true)));
        assert_eq!(
            plan.1.borrow().as_slice(),
            &[FRAME.to_string()],
            "MissionController has no globalAltitudeMode since the upstream merge; the property is globalAltitudeFrame with the same five values, so the head's write reached nothing"
        );
        assert_eq!(*plan.0.borrow(), ABSOLUTE);
    }
}
