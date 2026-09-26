use serde_json::{Value, json};

use crate::read::{flag, integer, object};
use crate::router::Backend;

// MultiVehicleManager::deselectAllVehicles clears the list and returns void, so the bridge answered
// ok for an empty selection it did nothing to and for a clear it could not confirm alike. The
// selection count is read before and after; ok means the selection is empty now.
const SELECTED_COUNT: &str = "vehicles.selectedVehicles.count";

fn selected(backend: &dyn Backend) -> i64 {
    integer(&object(&backend.get(SELECTED_COUNT)), "value").unwrap_or(0).max(0)
}

pub fn deselect_all(backend: &dyn Backend, path: &str) -> Value {
    let before = selected(backend);
    if before == 0 {
        return json!({ "ok": false, "refusal": "nothingSelected", "reason": "No vehicle is selected.", "cleared": 0 });
    }
    let dispatched = flag(&object(&backend.invoke(path, "[]")), "ok");
    let cleared = dispatched && selected(backend) == 0;
    json!({
        "ok": cleared,
        "refusal": Value::Null,
        "cleared": if cleared { before } else { 0 },
        "reason": match cleared { true => Value::Null, false => json!("The selection is still there.") },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Fleet {
        chosen: RefCell<i64>,
        obeys: bool,
        calls: RefCell<usize>,
    }

    impl Backend for Fleet {
        fn get(&self, _p: &str) -> String { json!({ "kind": "value", "value": *self.chosen.borrow() }).to_string() }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String {
            *self.calls.borrow_mut() += 1;
            if self.obeys {
                *self.chosen.borrow_mut() = 0;
            }
            json!({ "ok": true }).to_string()
        }
        fn watch(&self, _p: &[String]) {}
    }

    const PATH: &str = "vehicles.deselectAllVehicles";

    #[test]
    fn clearing_the_selection_says_what_it_cleared_and_refuses_an_empty_one() {
        let fleet = Fleet { chosen: RefCell::new(3), obeys: true, calls: RefCell::new(0) };
        let cleared = deselect_all(&fleet, PATH);
        assert_eq!((&cleared["ok"], &cleared["cleared"]), (&json!(true), &json!(3)));
        assert_eq!(deselect_all(&fleet, PATH)["refusal"], "nothingSelected", "the bridge answered ok for a clear of nothing");
        assert_eq!(*fleet.calls.borrow(), 1, "an empty selection is not dispatched");

        let deaf = Fleet { chosen: RefCell::new(2), obeys: false, calls: RefCell::new(0) };
        assert_eq!(deselect_all(&deaf, PATH)["ok"], false, "the count is read back, since a dispatched void call is not a cleared selection");
    }
}
