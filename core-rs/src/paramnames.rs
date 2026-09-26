use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

// ParameterManager::parameterNames maps the component through _actualComponentId and returns the
// keys of whatever map it finds, so a list asked for before the load finishes is a partial one that
// reads exactly like a vehicle with few parameters, and an unknown component is an empty list that
// reads like a component with none. Both are told apart here.
const DEFAULT_COMPONENT: i64 = -1;
const HIGHEST_COMPONENT: i64 = 255;

fn refused(token: &str, reason: String) -> Value {
    json!({ "ok": false, "result": Value::Null, "refusal": token, "reason": reason })
}

pub fn names(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
    let component = match given.get(0) {
        None | Some(Value::Null) => Some(DEFAULT_COMPONENT),
        Some(c) => c.as_i64().filter(|c| *c == DEFAULT_COMPONENT || (1..=HIGHEST_COMPONENT).contains(c)),
    };
    let Some(component) = component else {
        return refused("badComponent", format!("A component id runs from 1 to {HIGHEST_COMPONENT}, or -1 for the autopilot's own."));
    };
    if !flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable") {
        return refused("noVehicle", "Connect a vehicle to list its parameters.".to_string());
    }
    if !flag(&object(&backend.get_fields("vehicle.parameterManager", "parametersReady")), "parametersReady") {
        return refused("notReady", "The vehicle's parameters are still loading.".to_string());
    }
    let answer = object(&backend.invoke(path, &json!([component]).to_string()));
    let result = answer.get("result").filter(|r| r.is_array()).cloned();
    let listed = result.as_ref().and_then(Value::as_array).map_or(0, Vec::len);
    match (flag(&answer, "ok"), result) {
        (true, Some(_)) if listed == 0 => refused("noSuchComponent", format!("Component {component} holds no parameters on this vehicle.")),
        (true, Some(result)) => json!({ "ok": true, "result": result, "refusal": Value::Null, "reason": Value::Null }),
        _ => refused("unanswered", "The parameter manager did not list its parameters.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Manager {
        connected: bool,
        ready: bool,
        names: Value,
    }

    impl Backend for Manager {
        fn get(&self, p: &str) -> String { self.get_fields(p, "") }
        fn get_fields(&self, p: &str, _f: &str) -> String {
            match p {
                "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": self.connected }),
                _ => json!({ "kind": "object", "parametersReady": self.ready }),
            }
            .to_string()
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { json!({ "ok": true, "result": self.names }).to_string() }
        fn watch(&self, _p: &[String]) {}
    }

    const PATH: &str = "vehicle.parameterManager.parameterNames";

    #[test]
    fn a_parameter_list_is_served_only_once_it_is_whole_and_an_empty_one_says_why() {
        let loaded = Manager { connected: true, ready: true, names: json!(["BAT1_N_CELLS", "RTL_RETURN_ALT"]) };
        let listed = names(&loaded, PATH, "[1]");
        assert_eq!((&listed["ok"], &listed["result"]), (&json!(true), &json!(["BAT1_N_CELLS", "RTL_RETURN_ALT"])), "the head reads result, so the claim keeps Qt's key");
        assert_eq!(names(&loaded, PATH, "[-1]")["ok"], true);
        assert_eq!(names(&loaded, PATH, "[0]")["refusal"], "badComponent");
        assert_eq!(names(&loaded, PATH, "[\"1\"]")["refusal"], "badComponent");
        assert_eq!(names(&Manager { ready: false, ..loaded }, PATH, "[1]")["refusal"], "notReady", "a list asked for mid-load is partial and reads like a small vehicle");
        assert_eq!(names(&Manager { connected: false, ready: false, names: json!([]) }, PATH, "[1]")["refusal"], "noVehicle");
        assert_eq!(names(&Manager { connected: true, ready: true, names: json!([]) }, PATH, "[42]")["refusal"], "noSuchComponent");
    }
}
