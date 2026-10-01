use serde_json::{Value, json};

use crate::read::object;
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "vehicle.distanceSensors.rotationNone",
    "vehicle.distanceSensors.rotationYaw45",
    "vehicle.distanceSensors.rotationYaw90",
    "vehicle.distanceSensors.rotationYaw135",
    "vehicle.distanceSensors.rotationYaw180",
    "vehicle.distanceSensors.rotationYaw225",
    "vehicle.distanceSensors.rotationYaw270",
    "vehicle.distanceSensors.rotationYaw315",
];

const SECTORS: [&str; 8] = ["rotationNone", "rotationYaw45", "rotationYaw90", "rotationYaw135", "rotationYaw180", "rotationYaw225", "rotationYaw270", "rotationYaw315"];
const NO_VALUE: &str = "–.––";
const RANGE_METERS: f64 = 6.0;

pub fn sectors(group: &Value) -> Vec<Value> {
    let facts = group.get("facts").and_then(Value::as_array).cloned().unwrap_or_default();
    SECTORS
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let fact = facts.iter().find(|fact| fact.get("property").and_then(Value::as_str) == Some(name) || fact.get("name").and_then(Value::as_str) == Some(name));
            let meters = fact.and_then(|f| f.get("rawValue").or(f.get("value"))).and_then(Value::as_f64).filter(|m| m.is_finite());
            json!({
                "bearing": index * 45,
                "meters": meters,
                "text": fact.and_then(|f| f.get("valueString")).and_then(Value::as_str).filter(|_| meters.is_some()).unwrap_or(NO_VALUE),
            })
        })
        .collect()
}

pub fn proximity_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let group = object(&backend.get("vehicle.distanceSensors"));
    json!({
        "kind": "object",
        "class": "ProximityRadar",
        "shown": group.get("telemetryAvailable").and_then(Value::as_bool).unwrap_or(false),
        "rangeMeters": RANGE_METERS,
        "sectors": sectors(&group),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_of_the_eight_yaw_sectors_reads_its_distance_or_dashes() {
        let group = json!({ "telemetryAvailable": true, "facts": [
            { "property": "rotationNone", "rawValue": 2.5, "valueString": "2.50" },
            { "property": "rotationYaw90", "rawValue": null, "valueString": "–.––" },
            { "property": "rotationPitch270", "rawValue": 1.0, "valueString": "1.00" },
        ] });
        let read = sectors(&group);
        assert_eq!(read.len(), 8, "downward and upward sensors are not part of the radar");
        assert_eq!((read[0]["bearing"].clone(), read[0]["meters"].clone(), read[0]["text"].clone()), (json!(0), json!(2.5), json!("2.50")));
        assert_eq!((read[2]["meters"].clone(), read[2]["text"].clone()), (Value::Null, json!(NO_VALUE)));
        assert_eq!(read[7]["bearing"], 315);
    }
}
