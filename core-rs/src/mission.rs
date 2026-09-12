use serde_json::{Value, json};

use crate::planfile::{self, Mission, SimpleItem, altitude_mode_for_frame};
use crate::router::Backend;

pub const DEPS: &[&str] = &[];

fn number(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

fn coordinate(v: &Value) -> Option<(f64, f64, f64)> {
    let c = v.as_array().filter(|c| c.len() >= 3)?;
    Some((number(&c[0]), number(&c[1]), number(&c[2])))
}

fn seven_params(item: &Value) -> Result<[f64; 7], String> {
    let first_four: Vec<f64> = match item.get("params").and_then(Value::as_array) {
        Some(list) => list.iter().map(number).collect(),
        None => (1..=4).map(|i| item.get(format!("param{i}")).map(number).ok_or(format!("param{i} missing"))).collect::<Result<_, _>>()?,
    };
    let coordinate = item.get("coordinate").and_then(coordinate).map(|(lat, lon, alt)| vec![lat, lon, alt]).unwrap_or_default();
    let all: Vec<f64> = first_four.into_iter().chain(coordinate).collect();
    all.try_into().map_err(|v: Vec<f64>| format!("expected 7 parameters, found {}", v.len()))
}

fn simple_item(item: &Value) -> Result<SimpleItem, String> {
    let kind = item.get("type").and_then(Value::as_str).unwrap_or("");
    if kind != "SimpleItem" && kind != "missionItem" {
        return Err(format!("item type {kind:?} is not a simple item"));
    }
    let frame = item.get("frame").and_then(Value::as_i64).ok_or("frame missing")?;
    Ok(SimpleItem {
        frame,
        command: item.get("command").and_then(Value::as_i64).ok_or("command missing")?,
        params: seven_params(item)?,
        auto_continue: item.get("autoContinue").and_then(Value::as_bool).unwrap_or(true),
        altitude_mode: altitude_mode_for_frame(frame),
    })
}

pub fn parse(text: &str) -> Result<Mission, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let version = root.get("version").and_then(Value::as_i64).unwrap_or(1);
    if !(1..=2).contains(&version) {
        return Err(format!("mission file version {version} is not 1 or 2"));
    }
    if root.get("complexItems").and_then(Value::as_array).is_some_and(|c| !c.is_empty()) {
        return Err("complex items in a legacy mission file are not supported".to_string());
    }
    let items = root.get("items").and_then(Value::as_array).ok_or("no items array")?;
    let items: Vec<SimpleItem> = items.iter().enumerate().map(|(i, item)| simple_item(item).map_err(|e| format!("item {}: {e}", i + 1))).collect::<Result<_, _>>()?;
    let home_object = root.get("plannedHomePosition").and_then(|h| h.get("coordinate")).and_then(coordinate);
    let home_array = root.get("plannedHomePosition").and_then(coordinate);
    let home = home_object.or(home_array).or_else(|| items.first().map(|i| (i.params[4], i.params[5], i.params[6]))).unwrap_or_default();
    Ok(Mission {
        home,
        firmware_type: root.get("firmwareType").or(root.get("MAV_AUTOPILOT")).and_then(Value::as_i64).unwrap_or(0),
        vehicle_type: root.get("vehicleType").and_then(Value::as_i64).unwrap_or(2),
        cruise_speed: root.get("cruiseSpeed").map(number).unwrap_or(15.0),
        hover_speed: root.get("hoverSpeed").map(number).unwrap_or(5.0),
        global_altitude_mode: planfile::ALTITUDE_MODE_MIXED,
        items,
    })
}

pub fn mission_file_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return crate::read::refused("this needs the path of a mission file to read, and none was given") };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return json!({ "kind": "missionFile", "readable": false, "valid": false, "error": e.to_string() }),
    };
    match parse(&text) {
        Ok(mission) => json!({
            "kind": "missionFile",
            "readable": true,
            "valid": true,
            "firmwareType": mission.firmware_type,
            "home": { "latitude": mission.home.0, "longitude": mission.home.1, "altitude": mission.home.2 },
            "itemCount": mission.items.len(),
            "plan": planfile::write(&mission),
        }),
        Err(e) => json!({ "kind": "missionFile", "readable": true, "valid": false, "error": e }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{}/../test/MissionManager/{name}.mission", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    #[test]
    fn the_version_two_fixture_reads_its_six_items_and_home() {
        let mission = parse(&fixture("OldFileFormat")).unwrap();
        assert_eq!((mission.items.len(), mission.firmware_type), (6, 3));
        assert!((mission.home.0 - 47.66013776).abs() < 1e-6);
        assert_eq!((mission.items[0].command, mission.items[0].frame, mission.items[5].command), (22, 2, 21));
        assert_eq!(mission.items[5].params[6], 3.0);
        assert_eq!(planfile::parse(&planfile::write(&mission)).unwrap().items.len(), 6);
        assert_eq!(mission.global_altitude_mode, planfile::ALTITUDE_MODE_MIXED);
    }

    #[test]
    fn the_version_one_fixtures_read_their_flat_parameters() {
        let mission = parse(&fixture("100Waypoints")).unwrap();
        assert_eq!((mission.items.len(), mission.firmware_type), (99, 3));
        assert_eq!(mission.items[0].params[0], 20.0);
        assert!((mission.items[1].params[4] - 34.469587).abs() < 1e-6);
        assert_eq!(parse(&fixture("800Waypoints")).unwrap().items.len(), 828);
    }

    #[test]
    fn a_broken_file_is_refused_with_a_reason() {
        assert!(parse("[]").unwrap_err().contains("no items"));
        assert_eq!(parse(r#"{"version":3,"items":[]}"#).unwrap_err(), "mission file version 3 is not 1 or 2");
        assert!(parse(r#"{"items":[{"type":"SimpleItem","frame":3}]}"#).unwrap_err().starts_with("item 1: command missing"));
        assert!(parse(r#"{"items":[{"type":"ComplexItem","frame":3,"command":16}]}"#).unwrap_err().contains("not a simple item"));
        assert!(parse(r#"{"items":[],"complexItems":[{"id":1}]}"#).unwrap_err().contains("complex items"));
        assert_eq!(parse(r#"{"version":2,"vehicleType":1,"items":[]}"#).unwrap().vehicle_type, 1);
    }
}
