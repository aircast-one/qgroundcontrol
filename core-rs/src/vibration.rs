use serde_json::{Value, json};

use crate::read::{flag, object};
use crate::router::Backend;

// "vehicle.vibration" is a fact group object, so the watcher had nothing to bind to and every
// reading arrived through the re-read that only runs while the event loop is idle. Each fact
// underneath it binds through Fact::rawValueChanged, which is the whole content of this view.
pub const DEPS: &[&str] = &[
    "vehicles.activeVehicleAvailable",
    "vehicle.vibration.xAxis",
    "vehicle.vibration.yAxis",
    "vehicle.vibration.zAxis",
    "vehicle.vibration.clipCount1",
    "vehicle.vibration.clipCount2",
    "vehicle.vibration.clipCount3",
];

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
        "label": axis.to_uppercase(),
        "value": value,
        "fraction": value.map(|v| (v / SCALE_MAXIMUM).clamp(0.0, 1.0)),
        "severity": value.map(severity),
    })
}

pub fn vibration_view(backend: &dyn Backend, _args: &[String]) -> Value {
    // DEPS has always listed activeVehicleAvailable first, so this view already recomputes when it
    // changes and then dropped it. A head needing it had to fetch it in a second call, and a
    // vehicle disappearing between the two reads gave a reading that was connected with no axes,
    // or disconnected with axes.
    let connected = flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
    let group = object(&backend.get("vehicle.vibration"));
    let fact = |name: &str| group.get("facts").and_then(Value::as_array).and_then(|facts| facts.iter().find(|f| f.get("name").and_then(Value::as_str) == Some(name)));
    let number = |name: &str| fact(name).and_then(|f| f.get("value")).and_then(Value::as_f64).filter(|v| v.is_finite());
    let axes: Vec<(&str, Option<f64>)> = [("x", "xAxis"), ("y", "yAxis"), ("z", "zAxis")].iter().map(|(axis, name)| (*axis, number(name))).collect();
    let worst = axes.iter().filter_map(|(_, v)| *v).fold(None, |acc: Option<f64>, v| Some(acc.map_or(v, |a| a.max(v))));
    let clip_counts: Vec<Value> = ["clipCount1", "clipCount2", "clipCount3"].iter().map(|name| json!(number(name).map(|v| v as i64).unwrap_or(0))).collect();
    let units = fact("xAxis").and_then(|f| f.get("units")).and_then(Value::as_str).unwrap_or("");
    let silence = match (axes.iter().any(|(_, v)| v.is_some()), connected) {
        (true, _) => None,
        (false, false) => Some(("noVehicle", "No vehicle is connected.")),
        (false, true) => Some(("notReported", "This vehicle reports no vibration measurements.")),
    };
    json!({
        "kind": "object",
        "class": "Vibration",
        "connected": connected,
        "available": axes.iter().all(|(_, v)| v.is_some()),
        "silentReason": silence.map(|(token, _)| json!(token)).unwrap_or(Value::Null),
        "silentText": silence.map(|(_, sentence)| json!(sentence)).unwrap_or(Value::Null),
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

    struct Quiet(bool);
    impl Backend for Quiet {
        fn get(&self, path: &str) -> String {
            match path {
                "vehicles" => json!({ "kind": "object", "activeVehicleAvailable": self.0 }).to_string(),
                _ => json!({ "kind": "object", "facts": [ { "name": "xAxis", "value": null }, { "name": "yAxis" }, { "name": "zAxis" } ] }).to_string(),
            }
        }
        fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    #[test]
    fn no_axes_says_whether_there_is_a_vehicle_to_have_them() {
        let gone = vibration_view(&Quiet(false), &[]);
        assert_eq!(gone["connected"], false);
        assert_eq!(gone["silentReason"], "noVehicle", "a head fetching this flag itself races the axes it qualifies, so it travels in the same read");
        assert!(gone["silentText"].as_str().unwrap().contains("No vehicle"), "the sentence still travels beside the token; a head that wants to spell it its own way now can, and one that does not has the wording");

        let mute = vibration_view(&Quiet(true), &[]);
        assert_eq!(mute["connected"], true);
        assert_eq!(mute["silentReason"], "notReported", "connected and not reporting is a different thing from absent, and available:false spells them the same");
        assert!(mute["silentText"].as_str().unwrap().contains("no vibration measurements"));
        assert_ne!(mute["silentReason"], gone["silentReason"]);
    }

    struct Fake(Option<(f64, f64, f64)>);
    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            if path == "vehicles" {
                return json!({ "kind": "object", "activeVehicleAvailable": self.0.is_some() }).to_string();
            }
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
        assert_eq!(view["axes"][1]["label"], "Y");
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
