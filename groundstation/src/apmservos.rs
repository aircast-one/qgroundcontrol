use serde_json::{Value, json};

use crate::control::decode;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicle.parameterManager.parametersReady"];
pub const MAX_SERVOS: usize = 16;
const NO_OUTPUT: u16 = u16::MAX;

pub fn outputs(raw: &[u16; MAX_SERVOS]) -> Vec<i32> {
    raw.iter().map(|v| if *v == NO_OUTPUT { -1 } else { i32::from(*v) }).collect()
}

pub fn position(pwm: i32, min: Option<f64>, max: Option<f64>) -> Option<f64> {
    let (min, max) = (min?, max?);
    (pwm >= 0 && max - min > 0.0).then(|| ((f64::from(pwm) - min) / (max - min)).clamp(0.0, 1.0))
}

fn parameter(backend: &dyn Backend, name: &str) -> Option<Value> {
    let path = format!("vehicle.parameterManager.getParameter(-1,{name})");
    let fact = backend.value(&path);
    let present = fact.get("kind").and_then(Value::as_str) == Some("fact") && fact.get("name").and_then(Value::as_str).is_some_and(|n| !n.is_empty());
    present.then(|| decode(&fact, &path))
}

fn number(fact: &Option<Value>) -> Option<f64> {
    fact.as_ref().and_then(|f| f["value"].as_f64().or_else(|| f["valueString"].as_str().and_then(|s| s.parse().ok())))
}

pub fn servos_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let pwm = crate::hub::lock().active().map(|v| v.servo_outputs.clone()).unwrap_or_default();
    let servos: Vec<Value> = (1..=MAX_SERVOS)
        .filter_map(|n| {
            let function = parameter(backend, &format!("SERVO{n}_FUNCTION"))?;
            let (min, max) = (parameter(backend, &format!("SERVO{n}_MIN")), parameter(backend, &format!("SERVO{n}_MAX")));
            let value = pwm.get(n - 1).copied().unwrap_or(-1);
            Some(json!({
                "index": n,
                "pwm": (value >= 0).then_some(value),
                "position": position(value, number(&min), number(&max)),
                "function": function,
                "min": min,
                "trim": parameter(backend, &format!("SERVO{n}_TRIM")),
                "max": max,
                "reversed": parameter(backend, &format!("SERVO{n}_REVERSED")),
            }))
        })
        .collect();
    json!({ "kind": "object", "class": "ApmServos", "available": !servos.is_empty(), "servos": servos })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unused_output_reads_as_no_value_and_the_bar_stays_within_the_range() {
        let raw = [1500, NO_OUTPUT, 900, 2100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, NO_OUTPUT];
        assert_eq!(outputs(&raw)[..4], [1500, -1, 900, 2100]);
        assert_eq!(position(1500, Some(1000.0), Some(2000.0)), Some(0.5));
        assert_eq!(position(900, Some(1000.0), Some(2000.0)), Some(0.0));
        assert_eq!(position(2100, Some(1000.0), Some(2000.0)), Some(1.0));
        assert_eq!(position(-1, Some(1000.0), Some(2000.0)), None);
        assert_eq!(position(1500, Some(2000.0), Some(2000.0)), None, "an empty range has no position");
    }
}
