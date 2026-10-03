use serde_json::{Map, Value, json};

use crate::altitudemodes::{ABSOLUTE, CALC_ABOVE_TERRAIN, RELATIVE};
use crate::qtjson::{to_int, validate_keys};

const CORRIDOR: &str = "CorridorScan";
const SURVEY: &str = "survey";
const TRANSECT_STYLE_KEY: &str = "TransectStyleComplexItem";
const CAMERA_CALC_KEY: &str = "CameraCalc";
const CAMERA_SPEC_NONE: i32 = 0;
const CAMERA_SPEC_CUSTOM: i32 = 1;
const MANUAL_CAMERA: &str = "Manual (no camera specs)";
const CUSTOM_CAMERA: &str = "Custom Camera";

pub fn applies(kind: &str) -> bool {
    kind == CORRIDOR || kind == SURVEY
}

pub fn loaded(kind: &str, item: &Value) -> Result<Value, String> {
    match kind {
        CORRIDOR => corridor_header(item)?,
        _ => match survey_header(item)? {
            version if version < 4 => return Ok(item.clone()),
            version => survey_keys(item, version)?,
        },
    };
    validate_keys(item, &[(TRANSECT_STYLE_KEY, "Object", true)])?;
    let inner = transect_style(&item[TRANSECT_STYLE_KEY])?;
    let loaded = with(item.clone(), TRANSECT_STYLE_KEY, inner);
    Ok(match kind == CORRIDOR && to_int(&loaded[TRANSECT_STYLE_KEY]["CameraShots"], 0) == 0 {
        true => {
            let shots = crate::surveydoc::corridor_shots(&loaded);
            let inner = with(loaded[TRANSECT_STYLE_KEY].clone(), "CameraShots", json!(shots));
            with(loaded, TRANSECT_STYLE_KEY, inner)
        }
        false => loaded,
    })
}

fn with(mut object: Value, key: &str, value: Value) -> Value {
    object[key] = value;
    object
}

fn corridor_header(item: &Value) -> Result<i32, String> {
    validate_keys(item, &[("version", "Double", true), ("type", "String", true), ("complexItemType", "String", true), ("CorridorWidth", "Double", true), ("EntryPoint", "Double", true), ("polyline", "Array", true)])?;
    match to_int(&item["version"], 0) {
        2 => Ok(2),
        version => Err(format!("{CORRIDOR} complex item version {version} not supported")),
    }
}

fn survey_header(item: &Value) -> Result<i32, String> {
    validate_keys(item, &[("version", "Double", true)])?;
    match to_int(&item["version"], 0) {
        version if (2..=5).contains(&version) => Ok(version),
        version => Err(format!("Survey items do not support version {version}")),
    }
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
        assert!(loaded(CORRIDOR, &corridor(transect(manual_calc()))).is_ok());
        assert_eq!(loaded(CORRIDOR, &with(corridor(transect(manual_calc())), "version", json!(3))), Err("CorridorScan complex item version 3 not supported".to_string()));
        let mut bare = corridor(transect(manual_calc()));
        bare.as_object_mut().unwrap().retain(|k, _| k != "polyline" && k != "CorridorWidth");
        assert_eq!(loaded(CORRIDOR, &bare), Err("The following required keys are missing: CorridorWidth, polyline".to_string()));
        let terrain = transect(with(manual_calc(), "DistanceMode", json!(CALC_ABOVE_TERRAIN)));
        assert_eq!(loaded(CORRIDOR, &corridor(terrain)), Err("The following required keys are missing: TerrainAdjustTolerance, TerrainAdjustMaxClimbRate, TerrainAdjustMaxDescentRate".to_string()));
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
        let upgraded = loaded(CORRIDOR, &corridor(Value::Object(inner.clone()))).unwrap();
        let style = &upgraded[TRANSECT_STYLE_KEY];
        assert_eq!((style["version"].clone(), style["CameraShots"].clone()), (json!(2), json!(0)));
        assert_eq!((style["CameraCalc"]["DistanceMode"].clone(), style["CameraCalc"]["CameraName"].clone()), (json!(RELATIVE), json!(MANUAL_CAMERA)));
        assert!(style["CameraCalc"].get("DistanceToSurfaceRelative").is_none());
        inner.insert("FollowTerrain".into(), json!(true));
        inner.insert("TerrainAdjustTolerance".into(), json!(10));
        inner.insert("TerrainAdjustMaxClimbRate".into(), json!(0));
        inner.insert("TerrainAdjustMaxDescentRate".into(), json!(0));
        let followed = loaded(CORRIDOR, &corridor(Value::Object(inner.clone()))).unwrap();
        assert_eq!(followed[TRANSECT_STYLE_KEY]["CameraCalc"]["DistanceMode"], json!(CALC_ABOVE_TERRAIN));
        assert!(followed[TRANSECT_STYLE_KEY].get("FollowTerrain").is_none());
        inner.remove("version");
        assert_eq!(loaded(CORRIDOR, &corridor(Value::Object(inner))), Err("The following required keys are missing: version".to_string()), "an unstamped section upgrades in a local, so TransectStyleComplexItem::_load still finds no version key");
    }

    #[test]
    fn a_corridor_saved_with_no_shots_recounts_them() {
        let unshot = with(transect(manual_calc()), "CameraShots", json!(0));
        let counted = loaded(CORRIDOR, &corridor(with(unshot, "CameraTriggerInTurnAround", json!(false)))).unwrap();
        assert_eq!(counted[TRANSECT_STYLE_KEY]["CameraShots"], json!(10), "CorridorScanComplexItem::_loadWorker recounts a zero: 111 m of polyline at 25 m per photo is 5, on each of 2 transects");
        assert_eq!(loaded(CORRIDOR, &corridor(transect(manual_calc()))).unwrap()[TRANSECT_STYLE_KEY]["CameraShots"], json!(4), "a saved count is kept");
    }

    #[test]
    fn survey_versions_follow_survey_complex_item() {
        let survey = |version: i64| json!({ "version": version, "type": "ComplexItem", "complexItemType": "survey", "entryLocation": 0, "angle": 0, "splitConcavePolygons": true, TRANSECT_STYLE_KEY: transect(manual_calc()) });
        assert!(loaded(SURVEY, &survey(5)).is_ok());
        let mut old = survey(5);
        old.as_object_mut().unwrap().remove("splitConcavePolygons");
        assert_eq!(loaded(SURVEY, &old), Err("The following required keys are missing: splitConcavePolygons".to_string()), "version 5 added the key and requires it");
        assert!(loaded(SURVEY, &with(old, "version", json!(4))).is_ok());
        assert_eq!(loaded(SURVEY, &survey(6)), Err("Survey items do not support version 6".to_string()));
    }
}
