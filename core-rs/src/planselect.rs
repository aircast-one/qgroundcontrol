use serde_json::{Value, json};

use crate::read::{flag, integer, object};
use crate::router::Backend;

fn sequences(backend: &dyn Backend) -> Option<Vec<i64>> {
    let listed = object(&backend.get_fields("plan.missionController.visualItems", "sequenceNumber"));
    listed.get("elements")?.as_array().map(|items| items.iter().filter_map(|item| item.get("sequenceNumber")?.as_i64()).collect())
}

fn selected(backend: &dyn Backend) -> Option<i64> {
    integer(&object(&backend.get_fields("plan.missionController", "currentPlanViewSeqNum")), "currentPlanViewSeqNum")
}

pub fn select(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let Some(sequence) = given.get(0).and_then(Value::as_i64) else {
        return json!({ "ok": false, "refusal": "malformed", "reason": "An item is selected by its sequence number." });
    };
    let Some(listed) = sequences(backend) else {
        return json!({ "ok": false, "refusal": "unavailable", "reason": "The plan did not list its items." });
    };
    if !listed.contains(&sequence) {
        return json!({ "ok": false, "refusal": "noSuchItem", "reason": format!("The plan has no item {sequence}.") });
    }
    let force = given.get(1).and_then(Value::as_bool).unwrap_or(false);
    let dispatched = flag(&object(&backend.invoke(path, &json!([sequence, force]).to_string())), "ok");
    let now = selected(backend);
    let took = dispatched && now == Some(sequence);
    json!({ "ok": took, "refusal": Value::Null, "selected": now, "reason": match took { true => Value::Null, false => json!("The plan did not select that item.") } })
}

pub fn write_undo_tracking(backend: &dyn Backend, path: &str, value: &str) -> Value {
    let Some(on) = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value")?.as_bool()) else {
        return json!({ "ok": false, "result": false, "refusal": "malformed", "reason": "Undo tracking is turned on with true and off with false." });
    };
    let answered = flag(&object(&backend.set(path, &json!({ "value": on }).to_string())), "ok");
    let held = object(&backend.get_fields("plan", "undoTracking")).get("undoTracking").and_then(Value::as_bool);
    let took = answered && held == Some(on);
    json!({ "ok": took, "result": took, "refusal": Value::Null, "reason": match took { true => Value::Null, false => json!("The plan did not keep that undo setting.") } })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    struct Plan {
        selected: Cell<i64>,
        tracking: Cell<bool>,
        calls: RefCell<Vec<String>>,
    }

    impl Backend for Plan {
        fn get(&self, _p: &str) -> String { String::new() }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "plan.missionController.visualItems" => json!({ "kind": "object", "elements": [{ "sequenceNumber": 0 }, { "sequenceNumber": 1 }, { "sequenceNumber": 4 }] }),
                "plan" => json!({ "kind": "object", "undoTracking": self.tracking.get() }),
                _ => json!({ "kind": "object", "currentPlanViewSeqNum": self.selected.get() }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, v: &str) -> String {
            self.tracking.set(object(v)["value"].as_bool().unwrap());
            json!({ "ok": true }).to_string()
        }
        fn invoke(&self, _p: &str, a: &str) -> String {
            self.calls.borrow_mut().push(a.to_string());
            self.selected.set(object(a)[0].as_i64().unwrap());
            json!({ "ok": true }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn an_item_is_selected_only_by_a_sequence_number_the_plan_holds() {
        let plan = Plan { selected: Cell::new(0), tracking: Cell::new(false), calls: RefCell::new(Vec::new()) };
        let path = "plan.missionController.setCurrentPlanViewSeqNum";
        assert_eq!(select(&plan, path, "[2, true]")["refusal"], "noSuchItem", "setCurrentPlanViewSeqNum with a sequence nothing has clears the selection and every insert rule that hangs off it, in silence");
        assert!(plan.calls.borrow().is_empty());
        let chosen = select(&plan, path, "[4, true]");
        assert_eq!((&chosen["ok"], &chosen["selected"]), (&json!(true), &json!(4)));
        assert_eq!(plan.calls.borrow().as_slice(), &["[4,true]".to_string()]);
        assert_eq!(select(&plan, path, "[]")["refusal"], "malformed");
        assert_eq!(write_undo_tracking(&plan, "plan.undoTracking", r#"{"value":true}"#)["result"], true);
        assert_eq!(write_undo_tracking(&plan, "plan.undoTracking", r#"{"value":"yes"}"#)["refusal"], "malformed");
    }
}
