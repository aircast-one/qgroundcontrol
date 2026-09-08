use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &["vehicles.activeVehicleAvailable", "vehicle.vibration"];

pub const SCALE_MAXIMUM: f64 = 90.0;
pub const WARNING_LEVEL: f64 = 30.0;
pub const DANGER_LEVEL: f64 = 60.0;

pub fn severity(value: f64) -> &'static str {
    match value {
        v if v >= DANGER_LEVEL => "danger",
        v if v >= WARNING_LEVEL => "warning",
        _ => "normal",
    }
}

fn axis_json(axis: &str, value: Option<f64>) -> Value {
    json!({
        "axis": axis,
        "value": value,
        "fraction": value.map(|v| (v / SCALE_MAXIMUM).clamp(0.0, 1.0)),
        "severity": value.map(severity),
    })
}

pub fn vibration_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let group = object(&backend.get("vehicle.vibration"));
    let fact = |name: &str| group.get("facts").and_then(Value::as_array).and_then(|facts| facts.iter().find(|f| f.get("name").and_then(Value::as_str) == Some(name)));
    let number = |name: &str| fact(name).and_then(|f| f.get("value")).and_then(Value::as_f64).filter(|v| v.is_finite());
    let axes: Vec<(&str, Option<f64>)> = [("x", "xAxis"), ("y", "yAxis"), ("z", "zAxis")].iter().map(|(axis, name)| (*axis, number(name))).collect();
    let worst = axes.iter().filter_map(|(_, v)| *v).fold(None, |acc: Option<f64>, v| Some(acc.map_or(v, |a| a.max(v))));
    let clip_counts: Vec<Value> = ["clipCount1", "clipCount2", "clipCount3"].iter().map(|name| json!(number(name).map(|v| v as i64).unwrap_or(0))).collect();
    let units = fact("xAxis").and_then(|f| f.get("units")).and_then(Value::as_str).unwrap_or("");
    json!({
        "kind": "object",
        "class": "Vibration",
        "available": axes.iter().all(|(_, v)| v.is_some()),
        "units": units,
        "scaleMaximum": SCALE_MAXIMUM,
        "warningLevel": WARNING_LEVEL,
        "dangerLevel": DANGER_LEVEL,
        "axes": axes.iter().map(|(axis, v)| axis_json(axis, *v)).collect::<Vec<_>>(),
        "worst": worst.map(severity),
        "clipCounts": clip_counts,
        "clipping": clip_counts.iter().any(|c| c.as_i64().unwrap_or(0) > 0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(Option<(f64, f64, f64)>);
    impl Backend for Fake {
        fn get(&self, _path: &str) -> String {
            match self.0 {
                Some((x, y, z)) => json!({ "kind": "object", "facts": [
                    { "name": "xAxis", "value": x, "units": "m/s²" }, { "name": "yAxis", "value": y }, { "name": "zAxis", "value": z },
                    { "name": "clipCount1", "value": 0 }, { "name": "clipCount2", "value": 3 }, { "name": "clipCount3", "value": 0 },
                ] }),
                None => json!({ "kind": "object", "facts": [ { "name": "xAxis", "value": null }, { "name": "yAxis" }, { "name": "zAxis" } ] }),
            }
            .to_string()
        }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn the_bands_are_thirty_and_sixty() {
        assert_eq!(severity(15.0), "normal");
        assert_eq!(severity(30.0), "warning");
        assert_eq!(severity(59.9), "warning");
        assert_eq!(severity(60.0), "danger");
    }

    #[test]
    fn the_view_reports_each_axis_and_the_worst() {
        let view = vibration_view(&Fake(Some((15.0, 45.0, 75.0))), &[]);
        assert_eq!(view["available"], true);
        assert_eq!(view["axes"][0]["severity"], "normal");
        assert_eq!(view["axes"][1]["severity"], "warning");
        assert_eq!(view["axes"][2]["severity"], "danger");
        assert_eq!(view["axes"][1]["fraction"], 0.5);
        assert_eq!(view["worst"], "danger");
        assert_eq!(view["clipCounts"][1], 3);
        assert_eq!(view["clipping"], true);
        assert_eq!(view["units"], "m/s²");
    }

    #[test]
    fn without_a_reading_nothing_is_available() {
        let view = vibration_view(&Fake(None), &[]);
        assert_eq!(view["available"], false);
        assert_eq!(view["worst"], Value::Null);
        assert_eq!(view["axes"][0]["fraction"], Value::Null);
    }
}
