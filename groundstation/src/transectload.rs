use serde_json::{Map, Value, json};

use crate::altitudemodes::{ABSOLUTE, CALC_ABOVE_TERRAIN, RELATIVE};
use crate::qtjson::{to_int, validate_keys};

const CORRIDOR: &str = "CorridorScan";
const SURVEY: &str = "survey";
const STRUCTURE: &str = "StructureScan";
const TRANSECT_STYLE_KEY: &str = "TransectStyleComplexItem";
const CAMERA_CALC_KEY: &str = "CameraCalc";
const CAMERA_SPEC_NONE: i32 = 0;
const CAMERA_SPEC_CUSTOM: i32 = 1;
const MANUAL_CAMERA: &str = "Manual (no camera specs)";
const CUSTOM_CAMERA: &str = "Custom Camera";

pub fn complex(kind: &str, item: &Value, class: crate::cmdinfo::VehicleClass) -> Result<Value, String> {
    match kind {
        SURVEY | "Survey" => survey(item, class),
        CORRIDOR | "Corridor Scan" => corridor(item),
        STRUCTURE | "Structure Scan" => structure(item),
        crate::landingpattern::FIXED_WING_PATTERN | "Fixed Wing Landing" => crate::landingpattern::loaded(crate::landingpattern::FIXED_WING_PATTERN, item),
        crate::landingpattern::VTOL_PATTERN | "VTOL Landing" => crate::landingpattern::loaded(crate::landingpattern::VTOL_PATTERN, item),
        other => Err(format!("Unsupported complex item type: {other}")),
    }
}

pub fn of_type(item: &Value, complex_type: &str) -> Result<(), String> {
    let text = |key: &str| item.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let (item_type, saved_type) = (text("type"), text("complexItemType"));
    match item_type == "ComplexItem" && saved_type == complex_type {
        true => Ok(()),
        false => Err(format!("{} does not support loading this complex mission item type: {item_type}:{saved_type}", crate::noticeboard::application_name())),
    }
}

fn survey(item: &Value, class: crate::cmdinfo::VehicleClass) -> Result<Value, String> {
    validate_keys(item, &[("version", "Double", true)])?;
    match to_int(&item["version"], 0) {
        version @ (4 | 5) => {
            survey_keys(item, version)?;
            of_type(item, SURVEY)?;
            shape(item, "polygon")?;
            let loaded = transect_loaded(item)?;
            Ok(match to_int(&loaded[TRANSECT_STYLE_KEY]["CameraShots"], 0) == 0 {
                true => {
                    let inner = with(loaded[TRANSECT_STYLE_KEY].clone(), "CameraShots", json!(survey_shots(&loaded[TRANSECT_STYLE_KEY])));
                    with(loaded, TRANSECT_STYLE_KEY, inner)
                }
                false => loaded,
            })
        }
        2 | 3 => {
            survey_v3(item)?;
            Ok(survey_from_v3(item, class))
        }
        version => Err(format!("Survey items do not support version {version}")),
    }
}

fn survey_v3(item: &Value) -> Result<(), String> {
    validate_keys(item, &[
        ("type", "String", true),
        ("complexItemType", "String", true),
        ("polygon", "Array", true),
        ("grid", "Object", true),
        ("camera", "Object", false),
        ("cameraTriggerDistance", "Double", true),
        ("manualGrid", "Bool", true),
        ("fixedValueIsAltitude", "Bool", true),
        ("hoverAndCapture", "Bool", false),
        ("refly90Degrees", "Bool", false),
        ("cameraTriggerInTurnaround", "Bool", false),
    ])?;
    of_type(item, SURVEY)?;
    validate_keys(&item["grid"], &[("altitude", "Double", true), ("relativeAltitude", "Bool", true), ("angle", "Double", true), ("spacing", "Double", true), ("entryLocation", "Double", false), ("turnAroundDistance", "Double", true)])?;
    if !item["manualGrid"].as_bool().unwrap_or(true) {
        let camera = item.get("camera").ok_or("manualGrid = false but camera object is missing")?;
        let camera: Map<String, Value> = camera.as_object().cloned().unwrap_or_default().into_iter().map(|(key, value)| (if key == "imageSizeOverlap" { "imageSideOverlap".to_string() } else { key }, value)).collect();
        validate_keys(&Value::Object(camera), &[
            ("groundResolution", "Double", true),
            ("imageFrontalOverlap", "Double", true),
            ("imageSideOverlap", "Double", true),
            ("sensorWidth", "Double", true),
            ("sensorHeight", "Double", true),
            ("resolutionWidth", "Double", true),
            ("resolutionHeight", "Double", true),
            ("focalLength", "Double", true),
            ("name", "String", true),
            ("orientationLandscape", "Bool", true),
            ("minTriggerInterval", "Double", false),
        ])?;
    }
    shape(item, "polygon")
}

const ENTRY_LOCATION_TOP_RIGHT: i64 = 1;

fn survey_from_v3(item: &Value, class: crate::cmdinfo::VehicleClass) -> Value {
    use crate::cmdinfo::VehicleClass;
    let fresh = crate::surveydoc::fresh(&crate::surveydoc::Fresh {
        center: (0.0, 0.0),
        remembered: &crate::settingsstore::stored_text,
        multirotor: class == VehicleClass::MultiRotor,
        alternates: matches!(class, VehicleClass::FixedWing | VehicleClass::Vtol),
        default_altitude: 0.0,
        distance_mode: RELATIVE,
        previous_mode: None,
    });
    let grid = &item["grid"];
    let flag_or = |key: &str, default: bool| item[key].as_bool().unwrap_or(default);
    let mode = if grid["relativeAltitude"].as_bool().unwrap_or(true) { RELATIVE } else { ABSOLUTE };
    let shared = json!({
        "version": 2,
        "CameraName": MANUAL_CAMERA,
        "AdjustedFootprintSide": grid["spacing"],
        "AdjustedFootprintFrontal": item["cameraTriggerDistance"],
        "DistanceToSurface": grid["altitude"],
        "DistanceMode": mode,
    });
    let calc = match item["manualGrid"].as_bool().unwrap_or(true) {
        true => shared,
        false => {
            let camera = item["camera"].as_object().cloned().unwrap_or_default();
            let side = camera.get("imageSizeOverlap").or_else(|| camera.get("imageSideOverlap")).cloned().unwrap_or(Value::Null);
            let whole = |value: Option<&Value>| json!(value.map_or(0, |v| to_int(v, 0)));
            let number = |key: &str| camera.get(key).cloned().unwrap_or(json!(0));
            let named: Map<String, Value> = shared.as_object().cloned().unwrap_or_default().into_iter().chain([
                ("CameraName".to_string(), camera.get("name").cloned().unwrap_or(json!(""))),
                ("ValueSetIsDistance".to_string(), json!(flag_or("fixedValueIsAltitude", true))),
                ("ImageDensity".to_string(), number("groundResolution")),
                ("FrontalOverlap".to_string(), whole(camera.get("imageFrontalOverlap"))),
                ("SideOverlap".to_string(), whole(Some(&side))),
                ("SensorWidth".to_string(), number("sensorWidth")),
                ("SensorHeight".to_string(), number("sensorHeight")),
                ("ImageWidth".to_string(), whole(camera.get("resolutionWidth"))),
                ("ImageHeight".to_string(), whole(camera.get("resolutionHeight"))),
                ("FocalLength".to_string(), number("focalLength")),
                ("Landscape".to_string(), json!(camera.get("orientationLandscape").and_then(Value::as_bool).unwrap_or(true))),
                ("FixedOrientation".to_string(), json!(false)),
                ("MinTriggerInterval".to_string(), camera.get("minTriggerInterval").cloned().unwrap_or(json!(0))),
            ]).collect();
            Value::Object(named)
        }
    };
    let transect = json!({
        "version": 2,
        "TurnAroundDistance": grid["turnAroundDistance"],
        "CameraTriggerInTurnAround": flag_or("cameraTriggerInTurnaround", true),
        "HoverAndCapture": flag_or("hoverAndCapture", false),
        "Refly90Degrees": flag_or("refly90Degrees", false),
        "CameraCalc": calc,
    });
    let entry = grid.get("entryLocation").map_or(ENTRY_LOCATION_TOP_RIGHT, |e| i64::from(to_int(e, 0)));
    let converted = [("polygon", item["polygon"].clone()), ("angle", grid["angle"].clone()), ("entryLocation", json!(entry)), (TRANSECT_STYLE_KEY, transect)]
        .into_iter()
        .fold(fresh, |survey, (key, value)| with(survey, key, value));
    crate::surveydoc::regenerate(&converted)
}

fn corridor(item: &Value) -> Result<Value, String> {
    validate_keys(item, &[("version", "Double", true), ("type", "String", true), ("complexItemType", "String", true), ("CorridorWidth", "Double", true), ("EntryPoint", "Double", true), ("polyline", "Array", true)])?;
    of_type(item, CORRIDOR)?;
    match to_int(&item["version"], 0) {
        2 => {}
        version => return Err(format!("{CORRIDOR} complex item version {version} not supported")),
    }
    shape(item, "polyline")?;
    let loaded = transect_loaded(item)?;
    Ok(match to_int(&loaded[TRANSECT_STYLE_KEY]["CameraShots"], 0) == 0 {
        true => {
            let shots = crate::surveydoc::corridor_shots(&loaded);
            let inner = with(loaded[TRANSECT_STYLE_KEY].clone(), "CameraShots", json!(shots));
            with(loaded, TRANSECT_STYLE_KEY, inner)
        }
        false => loaded,
    })
}

fn structure(item: &Value) -> Result<Value, String> {
    validate_keys(item, &[
        ("version", "Double", true),
        ("type", "String", true),
        ("complexItemType", "String", true),
        ("polygon", "Array", true),
        ("ScanBottomAlt", "Double", true),
        ("StructureHeight", "Double", true),
        ("Layers", "Double", true),
        (CAMERA_CALC_KEY, "Object", true),
        ("EntranceAltitude", "Double", true),
        ("GimbalPitch", "Double", true),
        ("StartFromTop", "Bool", true),
    ])?;
    of_type(item, STRUCTURE)?;
    match to_int(&item["version"], 0) {
        3 => {}
        version => return Err(format!("{STRUCTURE} version {version} not supported")),
    }
    let calc = camera_calc(&item[CAMERA_CALC_KEY], false)?;
    shape(item, "polygon")?;
    Ok(with(item.clone(), CAMERA_CALC_KEY, calc))
}

fn shape(item: &Value, key: &str) -> Result<(), String> {
    let path = item.get(key).ok_or_else(|| format!("The following required keys are missing: {key}"))?;
    crate::plandoc::coordinates(path, false)
}

fn transect_loaded(item: &Value) -> Result<Value, String> {
    validate_keys(item, &[(TRANSECT_STYLE_KEY, "Object", true)])?;
    let inner = transect_style(&item[TRANSECT_STYLE_KEY])?;
    Ok(with(item.clone(), TRANSECT_STYLE_KEY, inner))
}

pub fn mission_item(saved: &Value) -> Result<Value, String> {
    let converted = coordinate_into_params(params_into_array(saved)?)?;
    validate_keys(&converted, &[("type", "String", true), ("frame", "Double", true), ("command", "Double", true), ("params", "Array", true), ("autoContinue", "Bool", true), ("doJumpId", "Double", false)])?;
    if converted["type"] != "SimpleItem" {
        return Err(format!("Type found: {} must be: SimpleItem", converted["type"].as_str().unwrap_or("")));
    }
    let params = converted["params"].as_array().map(Vec::as_slice).unwrap_or_default();
    if params.len() != 7 {
        return Err("params key must contains 7 values".to_string());
    }
    params[..4]
        .iter()
        .enumerate()
        .find(|(_, param)| !(param.is_number() || param.is_null()))
        .map_or(Ok(()), |(index, param)| Err(format!("Param {} incorrect type {}, must be double or null", index + 1, crate::plandoc::qt_json_type(param))))?;
    let doubles: Vec<Value> = params.iter().map(|param| if param.is_number() || param.is_null() { param.clone() } else { json!(param.as_f64().unwrap_or(0.0)) }).collect();
    Ok(with(converted, "params", Value::Array(doubles)))
}

fn params_into_array(saved: &Value) -> Result<Value, String> {
    if saved.get("params").is_some() {
        return Ok(saved.clone());
    }
    const SEPARATE: [&str; 4] = ["param1", "param2", "param3", "param4"];
    validate_keys(saved, &[("type", "String", true), ("param1", "Double", true), ("param2", "Double", true), ("param3", "Double", true), ("param4", "Double", true)])?;
    let params: Vec<Value> = SEPARATE.iter().map(|key| json!(saved[*key].as_f64().unwrap_or(0.0))).collect();
    let kept = saved.as_object().cloned().unwrap_or_default().into_iter().filter(|(key, _)| !SEPARATE.contains(&key.as_str())).map(|(key, value)| match (key.as_str(), value.as_str()) {
        ("type", Some("missionItem")) => (key, json!("SimpleItem")),
        _ => (key, value),
    });
    Ok(Value::Object(kept.chain([("params".to_string(), Value::Array(params))]).collect()))
}

fn coordinate_into_params(json: Value) -> Result<Value, String> {
    if json.get("coordinate").is_none() {
        return Ok(json);
    }
    validate_keys(&json, &[("coordinate", "Array", true)])?;
    crate::plandoc::coordinate(&json["coordinate"], true)?;
    let coordinate = json["coordinate"].as_array().cloned().unwrap_or_default();
    let params: Vec<Value> = json["params"].as_array().cloned().unwrap_or_default().into_iter().chain(coordinate).collect();
    let kept = json.as_object().cloned().unwrap_or_default().into_iter().filter(|(key, _)| key != "coordinate" && key != "params");
    Ok(Value::Object(kept.chain([("params".to_string(), Value::Array(params))]).collect()))
}

const CMD_NAV_WAYPOINT: i64 = 16;
const CMD_DO_SET_CAM_TRIGG_DIST: i64 = 206;
const CMD_IMAGE_START_CAPTURE: i64 = 2000;

fn survey_shots(transect: &Value) -> i64 {
    let trigger_distance = transect[CAMERA_CALC_KEY]["AdjustedFootprintFrontal"].as_f64().unwrap_or(0.0);
    let listed = |key: &str| transect[key].as_array().cloned().unwrap_or_default();
    let param = |item: &Value, index: usize| item["params"][index].as_f64().unwrap_or(f64::NAN);
    let command = |item: &Value| item["command"].as_f64().map_or(0, |c| c as i64);
    match (trigger_distance == 0.0, transect["CameraTriggerInTurnAround"].as_bool().unwrap_or(false), transect["HoverAndCapture"].as_bool().unwrap_or(false)) {
        (true, _, _) => 0,
        (false, true, _) => {
            let points: Vec<Option<(f64, f64)>> = listed("VisualTransectPoints").iter().map(|p| Some((p[0].as_f64()?, p[1].as_f64()?))).collect();
            let distance: f64 = points.windows(2).map(|pair| geo_distance(pair[0], pair[1])).sum();
            (distance / trigger_distance).ceil() as i64
        }
        (false, false, true) => listed("Items").iter().filter(|item| command(item) == CMD_IMAGE_START_CAPTURE).count() as i64,
        (false, false, false) => {
            let walked = listed("Items").iter().fold((false, None, None, 0i64), |(waiting, start, end, shots), item| match command(item) {
                CMD_NAV_WAYPOINT if waiting => (waiting, start, Some((param(item, 4), param(item, 5))), shots),
                CMD_NAV_WAYPOINT => (waiting, Some((param(item, 4), param(item, 5))), end, shots),
                CMD_DO_SET_CAM_TRIGG_DIST if param(item, 0) > 0.0 => (true, start, end, shots),
                CMD_DO_SET_CAM_TRIGG_DIST => (false, None, None, shots + (geo_distance(end, start) / trigger_distance).ceil() as i64),
                _ => (waiting, start, end, shots),
            });
            walked.3
        }
    }
}

fn geo_distance(from: Option<(f64, f64)>, to: Option<(f64, f64)>) -> f64 {
    let valid = |at: Option<(f64, f64)>| at.filter(|(lat, lon)| lat.abs() <= 90.0 && lon.abs() <= 180.0);
    valid(from).zip(valid(to)).map_or(0.0, |(a, b)| crate::surveygrid::distance_between(a, b))
}

fn with(mut object: Value, key: &str, value: Value) -> Value {
    object[key] = value;
    object
}

fn survey_keys(item: &Value, version: i32) -> Result<i32, String> {
    let common = [("type", "String", true), ("complexItemType", "String", true), ("entryLocation", "Double", true), ("angle", "Double", true), ("flyAlternateTransects", "Bool", false)];
    let keys: Vec<(&str, &str, bool)> = common.into_iter().chain((version == 5).then_some(("splitConcavePolygons", "Bool", true))).collect();
    validate_keys(item, &keys)?;
    Ok(version)
}

fn transect_style(saved: &Value) -> Result<Value, String> {
    let stamped = saved.get("version").map_or(0, |v| to_int(v, 0));
    let version = if stamped == 0 { 1 } else { stamped };
    let (inner, follow_terrain) = match version {
        1 => {
            let mut upgraded: Map<String, Value> = saved.as_object().cloned().unwrap_or_default();
            upgraded.entry("CameraShots").or_insert(json!(0));
            let follow = upgraded.remove("FollowTerrain").and_then(|v| v.as_bool()).unwrap_or(false);
            (Value::Object(upgraded), follow)
        }
        2 => (saved.clone(), false),
        other => return Err(format!("TransectStyleComplexItem version {other} not supported")),
    };
    validate_keys(&inner, &[
        ("version", "Double", true),
        ("TurnAroundDistance", "Double", true),
        ("CameraTriggerInTurnAround", "Bool", true),
        ("HoverAndCapture", "Bool", true),
        ("Refly90Degrees", "Bool", true),
        (CAMERA_CALC_KEY, "Object", true),
        ("VisualTransectPoints", "Array", true),
        ("Items", "Array", true),
        ("CameraShots", "Double", true),
    ])?;
    crate::plandoc::coordinates(&inner["VisualTransectPoints"], false)?;
    let items = inner["Items"].as_array().map(Vec::as_slice).unwrap_or_default().iter().map(mission_item).collect::<Result<Vec<_>, _>>()?;
    let inner = with(inner, "Items", Value::Array(items));
    let calc = camera_calc(&inner[CAMERA_CALC_KEY], follow_terrain)?;
    if to_int(&calc["DistanceMode"], 0) == CALC_ABOVE_TERRAIN as i32 {
        validate_keys(&inner, &[("TerrainAdjustTolerance", "Double", true), ("TerrainAdjustMaxClimbRate", "Double", true), ("TerrainAdjustMaxDescentRate", "Double", true), ("TerrainFlightSpeed", "Double", false)])?;
    }
    Ok(with(with(inner, CAMERA_CALC_KEY, calc), "version", json!(2)))
}

fn camera_calc(saved: &Value, follow_terrain: bool) -> Result<Value, String> {
    let stamped = saved.get("version").map_or(0, |v| to_int(v, 0));
    let mut json: Map<String, Value> = saved.as_object().cloned().unwrap_or_default();
    if stamped == 0 {
        match json.remove("CameraSpecType").map_or(CAMERA_SPEC_NONE, |v| to_int(&v, CAMERA_SPEC_NONE)) {
            CAMERA_SPEC_CUSTOM => json.insert("CameraName".into(), json!(CUSTOM_CAMERA)),
            CAMERA_SPEC_NONE => json.insert("CameraName".into(), json!(MANUAL_CAMERA)),
            _ => None,
        };
    }
    if matches!(stamped, 0 | 1) {
        let relative = json.remove("DistanceToSurfaceRelative").and_then(|v| v.as_bool()).unwrap_or(false);
        let mode = match (follow_terrain, relative) {
            (true, _) => CALC_ABOVE_TERRAIN,
            (false, true) => RELATIVE,
            (false, false) => ABSOLUTE,
        };
        json.insert("DistanceMode".into(), json!(mode));
    }
    let version = if matches!(stamped, 0 | 1) { 2 } else { stamped };
    if version != 2 {
        return Err(format!("CameraCalc section version {version} not supported"));
    }
    json.insert("version".into(), json!(2));
    let calc = Value::Object(json);
    validate_keys(&calc, &[("CameraName", "String", true), ("AdjustedFootprintSide", "Double", true), ("AdjustedFootprintFrontal", "Double", true), ("DistanceToSurface", "Double", true), ("DistanceMode", "Double", true)])?;
    if calc["CameraName"] != MANUAL_CAMERA {
        validate_keys(&calc, &[("ValueSetIsDistance", "Bool", true), ("ImageDensity", "Double", true), ("FrontalOverlap", "Double", true), ("SideOverlap", "Double", true)])?;
        validate_keys(&calc, &[("SensorWidth", "Double", true), ("SensorHeight", "Double", true), ("ImageWidth", "Double", true), ("ImageHeight", "Double", true), ("FocalLength", "Double", true), ("Landscape", "Bool", true), ("FixedOrientation", "Bool", true), ("MinTriggerInterval", "Double", true)])?;
    }
    Ok(calc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complex_any(kind: &str, item: &Value) -> Result<Value, String> {
        complex(kind, item, crate::cmdinfo::VehicleClass::MultiRotor)
    }

    fn manual_calc() -> Value {
        json!({ "version": 2, "CameraName": MANUAL_CAMERA, "AdjustedFootprintSide": 25, "AdjustedFootprintFrontal": 25, "DistanceToSurface": 50, "DistanceMode": 1 })
    }

    fn corridor(inner: Value) -> Value {
        json!({ "version": 2, "type": "ComplexItem", "complexItemType": CORRIDOR, "CorridorWidth": 50, "EntryPoint": 0, "polyline": [[47.0, 8.0], [47.001, 8.0]], TRANSECT_STYLE_KEY: inner })
    }

    fn transect(calc: Value) -> Value {
        json!({ "version": 2, "TurnAroundDistance": 10, "CameraTriggerInTurnAround": true, "HoverAndCapture": false, "Refly90Degrees": false, "CameraCalc": calc, "VisualTransectPoints": [], "Items": [], "CameraShots": 4 })
    }

    #[test]
    fn a_corridor_is_validated_like_corridor_scan_complex_item() {
        assert!(complex_any(CORRIDOR, &corridor(transect(manual_calc()))).is_ok());
        assert_eq!(complex_any(CORRIDOR, &with(corridor(transect(manual_calc())), "version", json!(3))), Err("CorridorScan complex item version 3 not supported".to_string()));
        let mut bare = corridor(transect(manual_calc()));
        bare.as_object_mut().unwrap().retain(|k, _| k != "polyline" && k != "CorridorWidth");
        assert_eq!(complex_any(CORRIDOR, &bare), Err("The following required keys are missing: CorridorWidth, polyline".to_string()));
        let terrain = transect(with(manual_calc(), "DistanceMode", json!(CALC_ABOVE_TERRAIN)));
        assert_eq!(complex_any(CORRIDOR, &corridor(terrain)), Err("The following required keys are missing: TerrainAdjustTolerance, TerrainAdjustMaxClimbRate, TerrainAdjustMaxDescentRate".to_string()));
    }

    #[test]
    fn version_one_keys_are_upgraded_like_the_qgc_loaders() {
        let mut calc = manual_calc().as_object().unwrap().clone();
        calc.remove("version");
        calc.remove("DistanceMode");
        calc.remove("CameraName");
        calc.insert("DistanceToSurfaceRelative".into(), json!(true));
        let mut inner = transect(Value::Object(calc)).as_object().unwrap().clone();
        inner.insert("version".into(), json!(1));
        inner.remove("CameraShots");
        let upgraded = complex_any(CORRIDOR, &corridor(Value::Object(inner.clone()))).unwrap();
        let style = &upgraded[TRANSECT_STYLE_KEY];
        assert_eq!((style["version"].clone(), style["CameraShots"].clone()), (json!(2), json!(0)));
        assert_eq!((style["CameraCalc"]["DistanceMode"].clone(), style["CameraCalc"]["CameraName"].clone()), (json!(RELATIVE), json!(MANUAL_CAMERA)));
        assert!(style["CameraCalc"].get("DistanceToSurfaceRelative").is_none());
        inner.insert("FollowTerrain".into(), json!(true));
        inner.insert("TerrainAdjustTolerance".into(), json!(10));
        inner.insert("TerrainAdjustMaxClimbRate".into(), json!(0));
        inner.insert("TerrainAdjustMaxDescentRate".into(), json!(0));
        let followed = complex_any(CORRIDOR, &corridor(Value::Object(inner.clone()))).unwrap();
        assert_eq!(followed[TRANSECT_STYLE_KEY]["CameraCalc"]["DistanceMode"], json!(CALC_ABOVE_TERRAIN));
        assert!(followed[TRANSECT_STYLE_KEY].get("FollowTerrain").is_none());
        inner.remove("version");
        assert_eq!(complex_any(CORRIDOR, &corridor(Value::Object(inner))), Err("The following required keys are missing: version".to_string()), "an unstamped section upgrades in a local, so TransectStyleComplexItem::_load still finds no version key");
    }

    #[test]
    fn a_corridor_saved_with_no_shots_recounts_them() {
        let unshot = with(transect(manual_calc()), "CameraShots", json!(0));
        let counted = complex_any(CORRIDOR, &corridor(with(unshot, "CameraTriggerInTurnAround", json!(false)))).unwrap();
        assert_eq!(counted[TRANSECT_STYLE_KEY]["CameraShots"], json!(10), "CorridorScanComplexItem::_loadWorker recounts a zero: 111 m of polyline at 25 m per photo is 5, on each of 2 transects");
        assert_eq!(complex_any(CORRIDOR, &corridor(transect(manual_calc()))).unwrap()[TRANSECT_STYLE_KEY]["CameraShots"], json!(4), "a saved count is kept");
    }

    #[test]
    fn survey_versions_follow_survey_complex_item() {
        let survey = |version: i64| json!({ "version": version, "type": "ComplexItem", "complexItemType": "survey", "entryLocation": 0, "angle": 0, "splitConcavePolygons": true, "polygon": [[47.0, 8.0], [47.001, 8.0], [47.001, 8.001]], TRANSECT_STYLE_KEY: transect(manual_calc()) });
        assert!(complex_any(SURVEY, &survey(5)).is_ok());
        let mut old = survey(5);
        old.as_object_mut().unwrap().remove("splitConcavePolygons");
        assert_eq!(complex_any(SURVEY, &old), Err("The following required keys are missing: splitConcavePolygons".to_string()), "version 5 added the key and requires it");
        assert!(complex_any(SURVEY, &with(old, "version", json!(4))).is_ok());
        assert_eq!(complex_any(SURVEY, &survey(6)), Err("Survey items do not support version 6".to_string()));
    }

    fn structure() -> Value {
        json!({ "version": 3, "type": "ComplexItem", "complexItemType": STRUCTURE, "polygon": [[47.0, 8.0], [47.001, 8.0], [47.001, 8.001]], "ScanBottomAlt": 0, "StructureHeight": 25, "Layers": 1, CAMERA_CALC_KEY: manual_calc(), "EntranceAltitude": 50, "GimbalPitch": 0, "StartFromTop": true })
    }

    #[test]
    fn a_structure_scan_is_validated_like_structure_scan_complex_item_load() {
        assert!(complex_any(STRUCTURE, &structure()).is_ok());
        let mut bare = structure();
        bare.as_object_mut().unwrap().retain(|k, _| k != "Layers" && k != "StartFromTop");
        assert_eq!(complex_any(STRUCTURE, &bare), Err("The following required keys are missing: Layers, StartFromTop".to_string()));
        assert_eq!(complex_any(STRUCTURE, &with(structure(), "GimbalPitch", json!("0"))), Err("Incorrect value type - key:type:expected GimbalPitch:String:Double".to_string()));
        assert_eq!(complex_any(STRUCTURE, &with(structure(), "version", json!(2))), Err("StructureScan version 2 not supported".to_string()));
        assert_eq!(complex_any(STRUCTURE, &with(structure(), "polygon", json!([[47.0, 8.0, 10.0]]))), Err("Coordinate array must contain 2 values".to_string()), "QGCMapPolygon::loadFromJson reads two-value vertices");
        let calc = with(manual_calc(), "version", json!(3));
        assert_eq!(complex_any(STRUCTURE, &with(structure(), CAMERA_CALC_KEY, calc)), Err("CameraCalc section version 3 not supported".to_string()));
        let named = with(manual_calc(), "CameraName", json!("Sony ILCE-QX1"));
        assert_eq!(complex_any(STRUCTURE, &with(structure(), CAMERA_CALC_KEY, named)), Err("The following required keys are missing: ValueSetIsDistance, ImageDensity, FrontalOverlap, SideOverlap".to_string()));
        let mut old_calc = manual_calc().as_object().unwrap().clone();
        old_calc.remove("version");
        old_calc.remove("DistanceMode");
        old_calc.insert("DistanceToSurfaceRelative".into(), json!(true));
        let upgraded = complex_any(STRUCTURE, &with(structure(), CAMERA_CALC_KEY, Value::Object(old_calc))).unwrap();
        assert_eq!((upgraded[CAMERA_CALC_KEY]["version"].clone(), upgraded[CAMERA_CALC_KEY]["DistanceMode"].clone()), (json!(2), json!(RELATIVE)), "the camera section is upgraded like CameraCalc::load and saved upgraded");
    }

    #[test]
    fn a_complex_type_is_resolved_like_qgc_core_plugin_create_complex_mission_item() {
        assert_eq!(complex_any("Orbit", &json!({})), Err("Unsupported complex item type: Orbit".to_string()));
        let app = crate::noticeboard::application_name();
        let canonical = with(structure(), "complexItemType", json!("Structure Scan"));
        assert_eq!(complex_any("Structure Scan", &canonical), Err(format!("{app} does not support loading this complex mission item type: ComplexItem:Structure Scan")), "the canonical name creates the item, whose load then refuses the type");
        let corridor = with(corridor(transect(manual_calc())), "complexItemType", json!("Corridor Scan"));
        assert_eq!(complex_any("Corridor Scan", &corridor), Err(format!("{app} does not support loading this complex mission item type: ComplexItem:Corridor Scan")));
    }

    #[test]
    fn the_shape_is_read_before_the_transects_like_the_qgc_loaders() {
        let mut no_line = corridor(transect(manual_calc()));
        no_line["polyline"] = json!([[47.0]]);
        assert_eq!(complex_any(CORRIDOR, &no_line), Err("Coordinate array must contain 2 values".to_string()));
        let survey = json!({ "version": 5, "type": "ComplexItem", "complexItemType": SURVEY, "entryLocation": 0, "angle": 0, "splitConcavePolygons": true, TRANSECT_STYLE_KEY: transect(manual_calc()) });
        assert_eq!(complex_any(SURVEY, &survey), Err("The following required keys are missing: polygon".to_string()));
        assert_eq!(complex_any(SURVEY, &with(survey, "polygon", json!({}))), Err("value for coordinate array is not array".to_string()));
    }

    #[test]
    fn transect_points_and_items_are_loaded_like_transect_style_complex_item() {
        let visual = with(transect(manual_calc()), "VisualTransectPoints", json!([[47.0, 8.0, 3.0]]));
        assert_eq!(complex_any(CORRIDOR, &corridor(visual)), Err("Coordinate array must contain 2 values".to_string()));
        let item = |item: Value| complex_any(CORRIDOR, &corridor(with(transect(manual_calc()), "Items", json!([item]))));
        assert_eq!(item(json!({ "type": "SimpleItem", "command": 16, "frame": 3, "params": [0, 0, 0, 0, 47.0, 8.0, 50] })), Err("The following required keys are missing: autoContinue".to_string()));
        assert_eq!(item(json!({ "type": "ComplexItem", "autoContinue": true, "command": 16, "frame": 3, "params": [0, 0, 0, 0, 47.0, 8.0, 50] })), Err("Type found: ComplexItem must be: SimpleItem".to_string()));
        let v2 = item(json!({ "type": "SimpleItem", "autoContinue": true, "command": 16, "frame": 3, "params": [0, 0, 0, 0], "coordinate": [47.0, 8.0, 50] })).unwrap();
        assert_eq!(v2[TRANSECT_STYLE_KEY]["Items"][0]["params"], json!([0, 0, 0, 0, 47.0, 8.0, 50]), "MissionItem::_convertJsonV2ToV3 moves the coordinate into params 5-7");
        assert!(v2[TRANSECT_STYLE_KEY]["Items"][0].get("coordinate").is_none());
        assert_eq!(item(json!({ "type": "SimpleItem", "autoContinue": true, "command": 16, "frame": 3, "params": [0, 0, 0, 0], "coordinate": [47.0, 8.0] })), Err("Coordinate array must contain 3 values".to_string()));
        assert_eq!(item(json!({ "type": "SimpleItem", "autoContinue": true, "command": 16, "frame": 3, "params": [0, 0, 0] })), Err("params key must contains 7 values".to_string()));
        assert_eq!(item(json!({ "type": "SimpleItem", "autoContinue": true, "command": 16, "frame": 3, "params": [0, "x", 0, 0, 47.0, 8.0, 50] })), Err("Param 2 incorrect type 3, must be double or null".to_string()));
        let v1 = item(json!({ "type": "missionItem", "autoContinue": true, "command": 16, "frame": 3, "param1": 1, "param2": 2, "param3": 3, "param4": 4, "coordinate": [47.0, 8.0, 50] })).unwrap();
        assert_eq!((v1[TRANSECT_STYLE_KEY]["Items"][0]["type"].clone(), v1[TRANSECT_STYLE_KEY]["Items"][0]["params"].clone()), (json!("SimpleItem"), json!([1.0, 2.0, 3.0, 4.0, 47.0, 8.0, 50])));
        assert_eq!(item(json!({ "type": "missionItem", "param1": 1 })), Err("The following required keys are missing: param2, param3, param4".to_string()));
    }

    #[test]
    fn a_survey_saved_with_no_shots_recounts_them_from_its_items() {
        let survey = |transect: Value| json!({ "version": 5, "type": "ComplexItem", "complexItemType": SURVEY, "entryLocation": 0, "angle": 0, "splitConcavePolygons": true, "polygon": [[47.0, 8.0], [47.001, 8.0], [47.001, 8.001]], TRANSECT_STYLE_KEY: transect });
        let waypoint = |lat: f64| json!({ "type": "SimpleItem", "autoContinue": true, "command": 16, "frame": 3, "params": [0, 0, 0, 0, lat, 8.0, 50] });
        let trigger = |distance: f64| json!({ "type": "SimpleItem", "autoContinue": true, "command": 206, "frame": 2, "params": [distance, 0, 1, 0, 0, 0, 0] });
        let items = json!([waypoint(47.0), trigger(25.0), waypoint(47.001), trigger(0.0)]);
        let unshot = with(with(with(transect(manual_calc()), "CameraShots", json!(0)), "CameraTriggerInTurnAround", json!(false)), "Items", items);
        let counted = complex_any(SURVEY, &survey(unshot.clone())).unwrap();
        assert_eq!(counted[TRANSECT_STYLE_KEY]["CameraShots"], json!(5), "SurveyComplexItem::_recalcCameraShots walks trigger start to stop: 111 m at 25 m is 5");
        let hovered = with(with(unshot.clone(), "HoverAndCapture", json!(true)), "Items", json!([waypoint(47.0), { "type": "SimpleItem", "autoContinue": true, "command": 2000, "frame": 2, "params": [0, 0, 1, 0, 0, 0, 0] }]));
        assert_eq!(complex_any(SURVEY, &survey(hovered)).unwrap()[TRANSECT_STYLE_KEY]["CameraShots"], json!(1), "hover and capture counts the IMAGE_START_CAPTURE commands");
        let turning = with(with(unshot, "CameraTriggerInTurnAround", json!(true)), "VisualTransectPoints", json!([[47.0, 8.0], [47.001, 8.0]]));
        assert_eq!(complex_any(SURVEY, &survey(turning)).unwrap()[TRANSECT_STYLE_KEY]["CameraShots"], json!(5), "with images in turnarounds it is the visual path over the trigger distance");
    }

    #[test]
    fn an_old_survey_is_validated_like_load_v3() {
        let grid = json!({ "altitude": 50, "relativeAltitude": true, "angle": 0, "spacing": 25, "turnAroundDistance": 10 });
        let v3 = json!({ "version": 3, "type": "ComplexItem", "complexItemType": SURVEY, "polygon": [[47.0, 8.0], [47.001, 8.0], [47.001, 8.001]], "grid": grid, "cameraTriggerDistance": 25, "manualGrid": false, "fixedValueIsAltitude": true });
        assert_eq!(complex_any(SURVEY, &v3), Err("manualGrid = false but camera object is missing".to_string()));
        let camera = json!({ "groundResolution": 3, "imageFrontalOverlap": 70, "imageSizeOverlap": 70, "sensorWidth": 6, "sensorHeight": 4, "resolutionWidth": 4000, "resolutionHeight": 3000, "focalLength": 4, "name": "x" });
        assert_eq!(complex_any(SURVEY, &with(v3.clone(), "camera", camera)), Err("The following required keys are missing: orientationLandscape".to_string()), "the old imageSizeOverlap typo stands in for imageSideOverlap");
        assert_eq!(complex_any(SURVEY, &with(v3, "grid", json!({}))), Err("The following required keys are missing: altitude, relativeAltitude, angle, spacing, turnAroundDistance".to_string()));
    }

    #[test]
    fn an_old_survey_is_rebuilt_into_the_current_format_like_load_v3_and_rebuild_transects() {
        let grid = json!({ "altitude": 60, "relativeAltitude": false, "angle": 30, "spacing": 20, "turnAroundDistance": 15 });
        let v3 = json!({ "version": 3, "type": "ComplexItem", "complexItemType": SURVEY, "polygon": [[47.0, 8.0], [47.002, 8.0], [47.002, 8.003], [47.0, 8.003]], "grid": grid, "cameraTriggerDistance": 25, "manualGrid": true, "fixedValueIsAltitude": true });
        let loaded = complex_any(SURVEY, &v3).unwrap();
        assert_eq!((loaded["version"].clone(), loaded["angle"].clone(), loaded["entryLocation"].clone()), (json!(5), json!(30), json!(ENTRY_LOCATION_TOP_RIGHT)), "an absent entry location is EntryLocationTopRight, and the item saves as version 5");
        let transect = &loaded[TRANSECT_STYLE_KEY];
        assert_eq!((transect["TurnAroundDistance"].clone(), transect["CameraTriggerInTurnAround"].clone(), transect["HoverAndCapture"].clone()), (json!(15), json!(true), json!(false)));
        assert_eq!(transect["CameraCalc"], json!({ "version": 2, "CameraName": MANUAL_CAMERA, "AdjustedFootprintSide": 20, "AdjustedFootprintFrontal": 25, "DistanceToSurface": 60, "DistanceMode": ABSOLUTE }));
        assert!(transect["Items"].as_array().is_some_and(|items| !items.is_empty()), "V2/3 has no saved items, so the transects are rebuilt");
        assert!(transect["Items"].as_array().unwrap().iter().filter(|i| i["command"] == 16).all(|i| i["frame"] == 0), "an absolute grid altitude flies in the global frame");
        assert!(crate::plandoc::complex_count(SURVEY, &loaded).unwrap() > 0);
        let camera = json!({ "groundResolution": 3.5, "imageFrontalOverlap": 70, "imageSizeOverlap": 60, "sensorWidth": 6.17, "sensorHeight": 4.55, "resolutionWidth": 4000, "resolutionHeight": 3000, "focalLength": 4.5, "name": "Sony ILCE-QX1", "orientationLandscape": false });
        let named = complex_any(SURVEY, &with(with(v3, "manualGrid", json!(false)), "camera", camera)).unwrap();
        let calc = &named[TRANSECT_STYLE_KEY]["CameraCalc"];
        assert_eq!((calc["CameraName"].clone(), calc["SideOverlap"].clone(), calc["Landscape"].clone(), calc["FixedOrientation"].clone(), calc["MinTriggerInterval"].clone()), (json!("Sony ILCE-QX1"), json!(60), json!(false), json!(false), json!(0)));
        assert_eq!((calc["AdjustedFootprintSide"].clone(), calc["DistanceToSurface"].clone()), (json!(20), json!(60)), "the saved grid values are kept, not recalculated from the camera");
    }
}
