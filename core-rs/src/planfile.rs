use serde_json::{Value, json};

use crate::router::Backend;

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

    #[test]
    fn a_non_plan_file_is_refused_with_a_reason() {
        assert!(parse("not json").unwrap_err().starts_with("not JSON"));
        assert_eq!(parse(r#"{"fileType":"Mission"}"#).unwrap_err(), "fileType is not \"Plan\"");
        assert_eq!(parse(r#"{"fileType":"Plan"}"#).unwrap_err(), "no mission object");
    }
}
