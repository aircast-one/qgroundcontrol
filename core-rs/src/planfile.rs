use serde_json::{Value, json};

use crate::router::Backend;
use crate::waypoints::Waypoints;

pub const DEPS: &[&str] = &[];

#[derive(Debug, PartialEq)]
pub struct PlanFile {
    pub version: i64,
    pub ground_station: String,
    pub firmware_type: i64,
    pub vehicle_type: i64,
    pub cruise_speed: f64,
    pub hover_speed: f64,
    pub home: Option<(f64, f64, f64)>,
    pub items: Vec<Value>,
    pub fence_polygons: usize,
    pub fence_circles: usize,
    pub rally_points: usize,
}

pub fn parse(text: &str) -> Result<PlanFile, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    if root.get("fileType").and_then(Value::as_str) != Some("Plan") {
        return Err("fileType is not \"Plan\"".to_string());
    }
    let mission = root.get("mission").ok_or("no mission object")?;
    let items: Vec<Value> = mission.get("items").and_then(Value::as_array).cloned().unwrap_or_default();
    let number = |v: &Value, key: &str| v.get(key).and_then(Value::as_f64).unwrap_or(0.0);
    let home = mission.get("plannedHomePosition").and_then(Value::as_array).filter(|a| a.len() >= 3).map(|a| (a[0].as_f64().unwrap_or(0.0), a[1].as_f64().unwrap_or(0.0), a[2].as_f64().unwrap_or(0.0)));
    let fence = root.get("geoFence").cloned().unwrap_or(Value::Null);
    let rally = root.get("rallyPoints").cloned().unwrap_or(Value::Null);
    let count = |v: &Value, key: &str| v.get(key).and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);
    Ok(PlanFile {
        version: root.get("version").and_then(Value::as_i64).unwrap_or(0),
        ground_station: root.get("groundStation").and_then(Value::as_str).unwrap_or("").to_string(),
        firmware_type: mission.get("firmwareType").and_then(Value::as_i64).unwrap_or(0),
        vehicle_type: mission.get("vehicleType").and_then(Value::as_i64).unwrap_or(0),
        cruise_speed: number(mission, "cruiseSpeed"),
        hover_speed: number(mission, "hoverSpeed"),
        home,
        items: items.iter().map(item_json).collect(),
        fence_polygons: count(&fence, "polygons"),
        fence_circles: count(&fence, "circles"),
        rally_points: count(&rally, "points"),
    })
}

fn item_json(item: &Value) -> Value {
    let kind = item.get("type").and_then(Value::as_str).unwrap_or("");
    let params: Vec<Value> = item.get("params").and_then(Value::as_array).cloned().unwrap_or_default();
    let listed = item.get("coordinate").and_then(Value::as_array).filter(|c| c.len() >= 3).map(|c| json!({ "latitude": c[0], "longitude": c[1], "altitude": c[2] }));
    let coordinate = listed.or_else(|| (params.len() >= 7 && kind == "SimpleItem").then(|| json!({ "latitude": params[4], "longitude": params[5], "altitude": params[6] })));
    json!({
        "type": kind,
        "command": item.get("command").cloned().unwrap_or(Value::Null),
        "frame": item.get("frame").cloned().unwrap_or(Value::Null),
        "doJumpId": item.get("doJumpId").cloned().unwrap_or(Value::Null),
        "autoContinue": item.get("autoContinue").and_then(Value::as_bool).unwrap_or(true),
        "complexItemType": item.get("complexItemType").cloned().unwrap_or(Value::Null),
        "coordinate": coordinate,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleItem {
    pub frame: i64,
    pub command: i64,
    pub params: [f64; 7],
    pub auto_continue: bool,
    pub altitude_mode: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mission {
    pub home: (f64, f64, f64),
    pub firmware_type: i64,
    pub vehicle_type: i64,
    pub cruise_speed: f64,
    pub hover_speed: f64,
    pub global_altitude_mode: i64,
    pub items: Vec<SimpleItem>,
}

pub const ALTITUDE_MODE_RELATIVE: i64 = 1;
pub const ALTITUDE_MODE_ABSOLUTE: i64 = 2;
pub const ALTITUDE_MODE_TERRAIN_FRAME: i64 = 4;

fn item_object(sequence: usize, item: &SimpleItem) -> Value {
    let base = [
        ("type", json!("SimpleItem")),
        ("frame", json!(item.frame)),
        ("command", json!(item.command)),
        ("autoContinue", json!(item.auto_continue)),
        ("doJumpId", json!(sequence)),
        ("params", json!(item.params)),
    ];
    let altitude = item
        .altitude_mode
        .map(|mode| [("AltitudeMode", json!(mode)), ("Altitude", json!(item.params[6])), ("AMSLAltAboveTerrain", Value::Null)])
        .into_iter()
        .flatten();
    Value::Object(base.into_iter().chain(altitude).map(|(key, value)| (key.to_string(), value)).collect())
}

pub fn write(mission: &Mission) -> String {
    let items: Vec<Value> = mission.items.iter().enumerate().map(|(i, item)| item_object(i + 1, item)).collect();
    let root = json!({
        "fileType": "Plan",
        "groundStation": "QGroundControl",
        "version": 1,
        "mission": {
            "version": 2,
            "plannedHomePosition": [mission.home.0, mission.home.1, mission.home.2],
            "firmwareType": mission.firmware_type,
            "vehicleType": mission.vehicle_type,
            "cruiseSpeed": mission.cruise_speed,
            "hoverSpeed": mission.hover_speed,
            "globalPlanAltitudeMode": mission.global_altitude_mode,
            "items": items,
        },
        "geoFence": { "version": 2, "circles": [], "polygons": [] },
        "rallyPoints": { "version": 2, "points": [] },
    });
    serde_json::to_string_pretty(&root).unwrap()
}

pub fn altitude_mode_for_frame(frame: i64) -> Option<i64> {
    match frame {
        3 => Some(ALTITUDE_MODE_RELATIVE),
        0 => Some(ALTITUDE_MODE_ABSOLUTE),
        10 => Some(ALTITUDE_MODE_TERRAIN_FRAME),
        _ => None,
    }
}

pub fn from_waypoints(file: &Waypoints, firmware_type: i64, vehicle_type: i64) -> Mission {
    let home = file.home.as_ref().or(file.items.first()).map(|r| (r.latitude, r.longitude, r.altitude)).unwrap_or_default();
    Mission {
        home,
        firmware_type,
        vehicle_type,
        cruise_speed: 15.0,
        hover_speed: 5.0,
        global_altitude_mode: ALTITUDE_MODE_RELATIVE,
        items: file
            .items
            .iter()
            .map(|r| SimpleItem {
                frame: r.frame,
                command: r.command,
                params: [r.params[0], r.params[1], r.params[2], r.params[3], r.latitude, r.longitude, r.altitude],
                auto_continue: r.auto_continue,
                altitude_mode: altitude_mode_for_frame(r.frame),
            })
            .collect(),
    }
}

pub fn plan_from_waypoints_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return json!({ "kind": "null" }) };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return json!({ "kind": "planFromWaypoints", "readable": false, "valid": false, "error": e.to_string() }),
    };
    let (firmware_type, vehicle_type) = (
        args.get(1).and_then(|a| a.parse().ok()).unwrap_or(12),
        args.get(2).and_then(|a| a.parse().ok()).unwrap_or(2),
    );
    match crate::waypoints::parse(&text) {
        Ok(file) => json!({ "kind": "planFromWaypoints", "readable": true, "valid": true, "itemCount": file.items.len(), "plan": write(&from_waypoints(&file, firmware_type, vehicle_type)) }),
        Err(e) => json!({ "kind": "planFromWaypoints", "readable": true, "valid": false, "error": e }),
    }
}

pub fn plan_file_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let Some(path) = args.first().filter(|p| !p.is_empty()) else { return json!({ "kind": "null" }) };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return json!({ "kind": "object", "class": "PlanFile", "path": path, "readable": false, "error": e.to_string() }),
    };
    match parse(&text) {
        Err(error) => json!({ "kind": "object", "class": "PlanFile", "path": path, "readable": true, "valid": false, "error": error }),
        Ok(plan) => json!({
            "kind": "object",
            "class": "PlanFile",
            "path": path,
            "readable": true,
            "valid": true,
            "error": "",
            "version": plan.version,
            "groundStation": plan.ground_station,
            "firmwareType": plan.firmware_type,
            "vehicleType": plan.vehicle_type,
            "cruiseSpeed": plan.cruise_speed,
            "hoverSpeed": plan.hover_speed,
            "home": plan.home.map(|(lat, lon, alt)| json!({ "latitude": lat, "longitude": lon, "altitude": alt })),
            "itemCount": plan.items.len(),
            "simpleCount": plan.items.iter().filter(|i| i["type"] == "SimpleItem").count(),
            "complexCount": plan.items.iter().filter(|i| i["type"] == "ComplexItem").count(),
            "items": plan.items,
            "fencePolygons": plan.fence_polygons,
            "fenceCircles": plan.fence_circles,
            "rallyPoints": plan.rally_points,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../test/MissionManager/SectionTest.plan")).expect("the SectionTest fixture")
    }

    #[test]
    fn the_section_test_plan_reads_back_its_five_items_and_home() {
        let plan = parse(&fixture()).unwrap();
        assert_eq!(plan.version, 1);
        assert_eq!(plan.items.len(), 5);
        assert!(plan.items.iter().all(|i| i["type"] == "SimpleItem"));
        let (lat, lon, alt) = plan.home.unwrap();
        assert!((lat - 47.6334).abs() < 1e-3 && (lon + 122.0908).abs() < 1e-3 && alt == 20.0);
        assert!(plan.items[0]["coordinate"].is_object());
        assert_eq!(plan.rally_points, 0);
    }

    fn sample() -> Mission {
        Mission {
            home: (47.6, -122.1, 20.0),
            firmware_type: 12,
            vehicle_type: 2,
            cruise_speed: 15.0,
            hover_speed: 5.0,
            global_altitude_mode: ALTITUDE_MODE_RELATIVE,
            items: vec![
                SimpleItem { frame: 3, command: 22, params: [0.0, 0.0, 0.0, f64::NAN, 47.6, -122.1, 30.0], auto_continue: true, altitude_mode: Some(ALTITUDE_MODE_RELATIVE) },
                SimpleItem { frame: 3, command: 16, params: [0.0, 0.0, 0.0, f64::NAN, 47.61, -122.11, 30.0], auto_continue: true, altitude_mode: Some(ALTITUDE_MODE_RELATIVE) },
                SimpleItem { frame: 2, command: 177, params: [1.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0], auto_continue: true, altitude_mode: None },
            ],
        }
    }

    #[test]
    fn a_written_plan_reads_back_through_the_reader() {
        let text = write(&sample());
        let plan = parse(&text).unwrap();
        assert_eq!((plan.version, plan.ground_station.as_str(), plan.firmware_type, plan.vehicle_type), (1, "QGroundControl", 12, 2));
        assert_eq!(plan.home, Some((47.6, -122.1, 20.0)));
        assert_eq!(plan.items.len(), 3);
        assert_eq!(plan.items[1]["coordinate"]["latitude"], json!(47.61));
        assert_eq!(plan.items[2]["doJumpId"], json!(3));
        let root: Value = serde_json::from_str(&text).unwrap();
        let items = root["mission"]["items"].as_array().unwrap();
        assert_eq!((items[0]["AltitudeMode"].clone(), items[0]["Altitude"].clone(), items[0]["AMSLAltAboveTerrain"].clone()), (json!(1), json!(30.0), Value::Null));
        assert!(items[2].get("AltitudeMode").is_none());
        assert_eq!(items[0]["params"][3], Value::Null);
        assert_eq!((root["geoFence"]["version"].clone(), root["rallyPoints"]["version"].clone()), (json!(2), json!(2)));
    }

    #[test]
    fn a_waypoints_file_becomes_a_plan_with_the_home_row_as_home() {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../test/MissionManager/MissionPlanner.waypoints")).unwrap();
        let file = crate::waypoints::parse(&text).unwrap();
        let mission = from_waypoints(&file, 12, 2);
        let home = file.home.as_ref().unwrap();
        assert_eq!(mission.home, (home.latitude, home.longitude, home.altitude));
        assert_eq!(mission.items.len(), file.items.len());
        assert!(mission.items.iter().zip(&file.items).all(|(m, r)| m.params[4] == r.latitude && m.altitude_mode == altitude_mode_for_frame(r.frame)));
        assert_eq!(parse(&write(&mission)).unwrap().items.len(), file.items.len());
    }

    #[test]
    fn a_non_plan_file_is_refused_with_a_reason() {
        assert!(parse("not json").unwrap_err().starts_with("not JSON"));
        assert_eq!(parse(r#"{"fileType":"Mission"}"#).unwrap_err(), "fileType is not \"Plan\"");
        assert_eq!(parse(r#"{"fileType":"Plan"}"#).unwrap_err(), "no mission object");
    }
}
