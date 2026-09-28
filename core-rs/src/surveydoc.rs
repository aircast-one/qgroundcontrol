use serde_json::{Value, json};

use crate::surveygrid::{self, Coord, Kind, Params};
use crate::surveyitems::{self, Plan};

type Point = (f64, f64);

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub fn polygon(survey: &Value) -> Vec<Point> {
    survey.get("polygon").and_then(Value::as_array).map(|p| p.iter().filter_map(|v| Some((v.get(0)?.as_f64()?, v.get(1)?.as_f64()?))).collect()).unwrap_or_default()
}

fn camera_shots(transects: &[Vec<Coord>], trigger_distance: f64, in_turnaround: bool, complex_distance: f64) -> i64 {
    match (trigger_distance == 0.0, in_turnaround) {
        (true, _) => 0,
        (false, true) => (complex_distance / trigger_distance).ceil() as i64,
        (false, false) => transects
            .iter()
            .filter_map(|transect| {
                let cameras: Vec<&Coord> = transect.iter().filter(|c| c.kind != Kind::Turnaround).collect();
                Some((cameras.first()?.at, cameras.last()?.at))
            })
            .map(|(first, last)| (surveygrid::distance_between(first, last) / trigger_distance).ceil() as i64)
            .sum(),
    }
}

pub fn regenerate(survey: &Value) -> Value {
    let transect = survey.get("TransectStyleComplexItem").cloned().unwrap_or(Value::Null);
    let calc = transect.get("CameraCalc").cloned().unwrap_or(Value::Null);
    let params = Params {
        grid_angle: number(survey, "angle").unwrap_or(0.0),
        grid_spacing: number(&calc, "AdjustedFootprintSide").unwrap_or(0.0),
        turnaround: number(&transect, "TurnAroundDistance").unwrap_or(0.0),
        refly: flag(&transect, "Refly90Degrees"),
        alternate: flag(survey, "flyAlternateTransects"),
        entry: survey.get("entryLocation").and_then(Value::as_i64).unwrap_or(0),
    };
    let transects = surveygrid::typed_transects(&polygon(survey), &params);
    let trigger_distance = f64::from(number(&calc, "AdjustedFootprintFrontal").unwrap_or(0.0) as f32);
    let in_turnaround = flag(&transect, "CameraTriggerInTurnAround");
    let mut changed = survey.clone();
    changed["TransectStyleComplexItem"] = rebuilt(&transect, &calc, &transects, trigger_distance, in_turnaround, |complex_distance| camera_shots(&transects, trigger_distance, in_turnaround, complex_distance));
    changed
}

const SURVEY_META: &str = include_str!("../../src/MissionManager/Survey.SettingsGroup.json");
const TRANSECT_META: &str = include_str!("../../src/MissionManager/TransectStyle.SettingsGroup.json");
const CAMERA_META: &str = include_str!("../../src/MissionManager/CameraCalc.FactMetaData.json");
const CAMERA_SPEC_META: &str = include_str!("../../src/MissionManager/CameraSpec.FactMetaData.json");
const CAMERA_LIST: &str = include_str!("../../src/Camera/CameraMetaData.json");
const MANUAL_CAMERA: &str = "Manual (no camera specs)";
const CUSTOM_CAMERA: &str = "Custom Camera";
const DEFAULT_DECIMAL_PLACES: i64 = 3;

fn meta(file: &str, name: &str) -> Option<crate::factmeta::MetaData> {
    crate::factmeta::from_file(file).ok()?.remove(name)
}

fn integer_typed(value_type: &crate::factmeta::ValueType) -> bool {
    use crate::factmeta::ValueType::*;
    matches!(value_type, Uint8 | Int8 | Uint16 | Int16 | Uint32 | Int32 | Uint64 | Int64)
}

fn shown(value: &Value, decimals: i64) -> String {
    match value {
        Value::Bool(b) => b.to_string(),
        other => other.as_f64().map_or_else(String::new, |n| format!("{n:.prec$}", prec = usize::try_from(decimals).unwrap_or(0))),
    }
}

pub struct Units<'a> {
    pub vertical: &'a crate::read::Unit,
    pub horizontal: &'a crate::read::Unit,
}

fn cooked<'a>(raw: &str, units: &'a Units) -> Option<&'a crate::read::Unit> {
    match raw {
        "vertical m" => Some(units.vertical),
        "m" | "meter" | "meters" | "horizontal m" => Some(units.horizontal),
        _ => None,
    }
}

fn fact(meta: &crate::factmeta::MetaData, value: Value, units: &Units) -> Value {
    let decimals = meta.decimal_places.unwrap_or(DEFAULT_DECIMAL_PLACES);
    let whole = integer_typed(&meta.value_type);
    let shown = |v: &Value, d: i64| shown(v, if whole { 0 } else { d });
    let bool_typed = meta.value_type == crate::factmeta::ValueType::Bool;
    let raw_units = meta.units.clone().unwrap_or_default();
    let unit = cooked(&raw_units, units);
    let cook = |v: f64| unit.map_or(v, |u| u.show(v));
    let number = |v: &Option<Value>| v.as_ref().and_then(Value::as_f64).map(cook);
    let (min, max) = (number(&meta.min), number(&meta.max));
    let raw = value.clone();
    let value = match (unit, value.as_f64()) {
        (Some(u), Some(v)) => json!(u.show(v)),
        _ => value,
    };
    let default = meta.default.as_ref().map(|d| d.as_f64().map_or_else(|| d.clone(), |n| json!(cook(n))));
    json!({
        "kind": "fact",
        "name": meta.name,
        "shortDescription": meta.short_description,
        "value": value,
        "rawValue": raw,
        "valueString": shown(&value, decimals),
        "units": unit.map_or(raw_units.clone(), |u| u.name.clone()),
        "rawUnits": raw_units,
        "decimalPlaces": decimals,
        "typeIsBool": bool_typed,
        "typeIsInteger": whole,
        "min": min,
        "max": max,
        "minString": min.map(|m| shown(&json!(m), decimals)),
        "maxString": max.map(|m| shown(&json!(m), decimals)),
        "minIsDefaultForType": min.is_none(),
        "maxIsDefaultForType": max.is_none(),
        "defaultValueAvailable": default.is_some(),
        "defaultValue": default,
        "defaultValueString": default.as_ref().map(|d| shown(d, decimals)),
        "valueEqualsDefault": default.as_ref().is_some_and(|d| d == &value || d.as_f64().zip(value.as_f64()).is_some_and(|(a, b)| a == b)),
        "readOnly": false,
    })
}

fn control(meta: &crate::factmeta::MetaData, value: Value, item: &str, suffix: &str, group: &str, units: &Units) -> Value {
    let mut built = crate::control::decode(&fact(meta, value, units), &format!("{item}.{suffix}"));
    if let Value::Object(map) = &mut built {
        map.insert("pathSuffix".to_string(), json!(suffix));
        map.insert("group".to_string(), json!(group));
    }
    built
}

fn with_default(value: Option<&Value>, meta: &crate::factmeta::MetaData) -> Value {
    value.cloned().or_else(|| meta.default.clone()).unwrap_or(Value::Null)
}

pub fn fields(survey: &Value, item: &str, multirotor: bool, units: &Units) -> Vec<Value> {
    let transect = survey.get("TransectStyleComplexItem").cloned().unwrap_or(Value::Null);
    let turnaround = if multirotor { "TurnAroundDistanceMultiRotor" } else { "TurnAroundDistance" };
    let listed: [(&str, &str, &str, &Value, &str); 10] = [
        (TRANSECT_META, turnaround, "turnAroundDistance", &transect, "TurnAroundDistance"),
        (TRANSECT_META, "CameraTriggerInTurnAround", "cameraTriggerInTurnAround", &transect, "CameraTriggerInTurnAround"),
        (TRANSECT_META, "HoverAndCapture", "hoverAndCapture", &transect, "HoverAndCapture"),
        (TRANSECT_META, "Refly90Degrees", "refly90Degrees", &transect, "Refly90Degrees"),
        (TRANSECT_META, "TerrainAdjustTolerance", "terrainAdjustTolerance", &transect, "TerrainAdjustTolerance"),
        (TRANSECT_META, "TerrainAdjustMaxDescentRate", "terrainAdjustMaxDescentRate", &transect, "TerrainAdjustMaxDescentRate"),
        (TRANSECT_META, "TerrainAdjustMaxClimbRate", "terrainAdjustMaxClimbRate", &transect, "TerrainAdjustMaxClimbRate"),
        (SURVEY_META, "GridAngle", "gridAngle", survey, "angle"),
        (SURVEY_META, "FlyAlternateTransects", "flyAlternateTransects", survey, "flyAlternateTransects"),
        (SURVEY_META, "SplitConcavePolygons", "splitConcavePolygons", survey, "splitConcavePolygons"),
    ];
    listed
        .iter()
        .filter_map(|(file, name, suffix, owner, key)| {
            let meta = meta(file, name)?;
            let value = with_default(owner.get(*key), &meta);
            Some(control(&meta, value, item, suffix, "Settings", units))
        })
        .collect()
}

const OPTICS: [(&str, &str); 7] = [("SensorWidth", "sensorWidth"), ("SensorHeight", "sensorHeight"), ("ImageWidth", "imageWidth"), ("ImageHeight", "imageHeight"), ("FocalLength", "focalLength"), ("Landscape", "landscape"), ("MinTriggerInterval", "minTriggerInterval")];
const FLIGHT: [(&str, &str); 4] = [("DistanceToSurface", "distanceToSurface"), ("ImageDensity", "imageDensity"), ("FrontalOverlap", "frontalOverlap"), ("SideOverlap", "sideOverlap")];

fn cameras() -> Vec<Value> {
    serde_json::from_str::<Value>(CAMERA_LIST).ok().and_then(|v| v.get("cameraMetaData").and_then(Value::as_array).cloned()).unwrap_or_default()
}

fn known_camera<'a>(known: &'a [Value], name: &str) -> Option<&'a Value> {
    known.iter().find(|c| c.get("canonicalName").and_then(Value::as_str) == Some(name))
}

pub fn camera(survey: &Value, item: &str, units: &Units) -> Value {
    let calc = survey.get("TransectStyleComplexItem").and_then(|t| t.get("CameraCalc")).cloned().unwrap_or(Value::Null);
    let name = calc.get("CameraName").and_then(Value::as_str).unwrap_or(MANUAL_CAMERA).to_string();
    let known = cameras();
    let listed = known_camera(&known, &name);
    let brand = listed.and_then(|c| c.get("brand").and_then(Value::as_str)).map_or_else(|| name.clone(), str::to_string);
    let model = listed.and_then(|c| c.get("model").and_then(Value::as_str)).unwrap_or("").to_string();
    let custom = name == CUSTOM_CAMERA;
    let brands: Vec<String> = [MANUAL_CAMERA.to_string(), CUSTOM_CAMERA.to_string()]
        .into_iter()
        .chain(known.iter().filter_map(|c| c.get("brand").and_then(Value::as_str).map(str::to_string)))
        .fold(Vec::new(), |seen, b| if seen.contains(&b) { seen } else { seen.into_iter().chain(std::iter::once(b)).collect() });
    let models: Vec<String> = known.iter().filter(|c| c.get("brand").and_then(Value::as_str) == Some(brand.as_str())).filter_map(|c| c.get("model").and_then(Value::as_str).map(str::to_string)).collect();
    let wanted: Vec<(&str, &str)> = match custom {
        true => OPTICS.iter().chain(FLIGHT.iter()).copied().collect(),
        false => FLIGHT.to_vec(),
    };
    let facts: Vec<Value> = wanted
        .iter()
        .filter_map(|(name, suffix)| {
            let meta = meta(CAMERA_META, name).or_else(|| meta(CAMERA_SPEC_META, name))?;
            Some(control(&meta, with_default(calc.get(*name), &meta), item, &format!("cameraCalc.{suffix}"), "Camera", units))
        })
        .collect();
    json!({
        "brand": brand,
        "model": model,
        "brands": brands,
        "models": models,
        "manualName": MANUAL_CAMERA,
        "customName": CUSTOM_CAMERA,
        "custom": custom,
        "distanceMode": calc.get("DistanceMode").cloned().unwrap_or(Value::Null),
        "brandPath": format!("{item}.cameraCalc.cameraBrand"),
        "modelPath": format!("{item}.cameraCalc.cameraModel"),
        "facts": facts,
    })
}

fn target(suffix: &str) -> Option<(&'static str, String)> {
    let transect = ["TurnAroundDistance", "CameraTriggerInTurnAround", "HoverAndCapture", "Refly90Degrees", "TerrainAdjustTolerance", "TerrainAdjustMaxDescentRate", "TerrainAdjustMaxClimbRate"];
    let capital = |s: &str| s.chars().next().map(|c| c.to_ascii_uppercase().to_string() + &s[c.len_utf8()..]).unwrap_or_default();
    match suffix {
        "gridAngle" => Some(("survey", "angle".to_string())),
        "flyAlternateTransects" | "splitConcavePolygons" => Some(("survey", suffix.to_string())),
        _ => match suffix.strip_prefix("cameraCalc.") {
            Some(calc) => {
                let key = capital(calc);
                OPTICS.iter().chain(FLIGHT.iter()).any(|(name, _)| *name == key).then_some(("calc", key)).or((calc == "valueSetIsDistance").then(|| ("calc", "ValueSetIsDistance".to_string())))
            }
            None => transect.iter().find(|name| capital(suffix) == **name).map(|name| ("transect", name.to_string())),
        },
    }
}

fn recalculated(calc: &Value) -> Value {
    let number = |key: &str| calc.get(key).and_then(Value::as_f64).unwrap_or(0.0);
    if calc.get("CameraName").and_then(Value::as_str) == Some(MANUAL_CAMERA) {
        return calc.clone();
    }
    let camera = crate::cameracalc::Camera {
        focal_length: number("FocalLength"),
        sensor_width: number("SensorWidth"),
        sensor_height: number("SensorHeight"),
        image_width: number("ImageWidth"),
        image_height: number("ImageHeight"),
        landscape: calc.get("Landscape").and_then(Value::as_bool).unwrap_or(true),
        frontal_overlap: number("FrontalOverlap"),
        side_overlap: number("SideOverlap"),
    };
    let by_distance = calc.get("ValueSetIsDistance").and_then(Value::as_bool).unwrap_or(true);
    let footprint = match by_distance {
        true => crate::cameracalc::from_distance(&camera, number("DistanceToSurface")),
        false => crate::cameracalc::from_density(&camera, number("ImageDensity")),
    };
    footprint.map_or_else(
        || calc.clone(),
        |f| {
            let mut changed = calc.clone();
            changed["ImageDensity"] = json!(f.image_density);
            changed["DistanceToSurface"] = json!(f.distance_to_surface);
            changed["AdjustedFootprintSide"] = json!(f.adjusted_side);
            changed["AdjustedFootprintFrontal"] = json!(f.adjusted_frontal);
            changed
        },
    )
}

const SPECS: [(&str, &str); 8] = [("SensorWidth", "sensorWidth"), ("SensorHeight", "sensorHeight"), ("ImageWidth", "imageWidth"), ("ImageHeight", "imageHeight"), ("FocalLength", "focalLength"), ("Landscape", "landscape"), ("FixedOrientation", "fixedOrientation"), ("MinTriggerInterval", "minTriggerInterval")];

fn named_camera(calc: &Value, name: &str) -> Value {
    let known = cameras();
    let mut changed = calc.clone();
    changed["CameraName"] = json!(name);
    match known_camera(&known, name) {
        Some(camera) => SPECS.iter().for_each(|(key, spec)| changed[*key] = camera.get(*spec).cloned().unwrap_or(Value::Null)),
        None => {
            changed["FixedOrientation"] = json!(false);
            changed["MinTriggerInterval"] = json!(0);
            if name == MANUAL_CAMERA {
                changed["ValueSetIsDistance"] = json!(true);
            }
        }
    }
    let mut settled = recalculated(&changed);
    if name != MANUAL_CAMERA && settled.get("DistanceMode").and_then(Value::as_i64) == Some(crate::altitudemodes::ABSOLUTE) {
        settled["DistanceMode"] = json!(crate::altitudemodes::RELATIVE);
    }
    settled
}

fn chosen_camera(calc: &Value, suffix: &str, value: &Value) -> Option<String> {
    let known = cameras();
    let wanted = value.as_str()?;
    match suffix {
        "cameraCalc.cameraBrand" if wanted == MANUAL_CAMERA || wanted == CUSTOM_CAMERA => Some(wanted.to_string()),
        "cameraCalc.cameraBrand" => known.iter().find(|c| c.get("brand").and_then(Value::as_str) == Some(wanted)).and_then(|c| c.get("canonicalName")?.as_str().map(str::to_string)),
        "cameraCalc.cameraModel" => {
            let current = calc.get("CameraName").and_then(Value::as_str).unwrap_or("");
            let brand = known_camera(&known, current)?.get("brand")?.as_str()?.to_string();
            known.iter().find(|c| c.get("brand").and_then(Value::as_str) == Some(brand.as_str()) && c.get("model").and_then(Value::as_str) == Some(wanted)).and_then(|c| c.get("canonicalName")?.as_str().map(str::to_string))
        }
        _ => None,
    }
}

pub fn set(survey: &Value, suffix: &str, value: &Value) -> Option<Value> {
    if suffix == "cameraCalc.cameraBrand" || suffix == "cameraCalc.cameraModel" {
        let calc = &survey["TransectStyleComplexItem"]["CameraCalc"];
        let name = chosen_camera(calc, suffix, value)?;
        let mut changed = survey.clone();
        changed["TransectStyleComplexItem"]["CameraCalc"] = named_camera(calc, &name);
        return Some(regenerate(&changed));
    }
    let (owner, key) = target(suffix)?;
    let mut changed = survey.clone();
    match owner {
        "survey" => changed[key.as_str()] = value.clone(),
        "transect" => changed["TransectStyleComplexItem"][key.as_str()] = value.clone(),
        _ => {
            let mut calc = changed["TransectStyleComplexItem"]["CameraCalc"].clone();
            calc[key.as_str()] = value.clone();
            changed["TransectStyleComplexItem"]["CameraCalc"] = recalculated(&calc);
        }
    }
    Some(regenerate(&changed))
}

pub struct Fresh<'a> {
    pub center: (f64, f64),
    pub remembered: &'a dyn Fn(&str) -> Option<String>,
    pub multirotor: bool,
    pub alternates: bool,
    pub default_altitude: f64,
    pub distance_mode: i64,
}

const SAVED_BY_EVERY_CAMERA: [&str; 6] = ["version", "AdjustedFootprintSide", "AdjustedFootprintFrontal", "DistanceToSurface", "DistanceMode", "CameraName"];

fn remembered(fresh: &Fresh, group: &str, file: &str, name: &str) -> Value {
    let Some(meta) = meta(file, name) else { return Value::Null };
    let stored = (fresh.remembered)(&format!("{group}/{name}")).and_then(|text| crate::settingsstore::typed(&meta.value_type, &Value::String(text)));
    stored.or_else(|| meta.default.as_ref().map(|d| crate::settingsstore::typed(&meta.value_type, d).unwrap_or_else(|| d.clone()))).unwrap_or(Value::Null)
}

fn fresh_transect(fresh: &Fresh, group: &str) -> Value {
    let calc_keys = ["CameraName", "ValueSetIsDistance", "DistanceToSurface", "ImageDensity", "FrontalOverlap", "SideOverlap", "AdjustedFootprintSide", "AdjustedFootprintFrontal"];
    let spec_keys = ["SensorWidth", "SensorHeight", "ImageWidth", "ImageHeight", "FocalLength", "Landscape", "FixedOrientation", "MinTriggerInterval"];
    let stored_calc: serde_json::Map<String, Value> = calc_keys
        .iter()
        .map(|k| (k.to_string(), remembered(fresh, group, CAMERA_META, k)))
        .chain(spec_keys.iter().map(|k| (k.to_string(), remembered(fresh, group, CAMERA_SPEC_META, k))))
        .chain([("DistanceMode".to_string(), json!(fresh.distance_mode)), ("version".to_string(), json!(2))])
        .collect();
    let stored_name = stored_calc.get("CameraName").and_then(Value::as_str).unwrap_or(MANUAL_CAMERA).to_string();
    let name = match stored_name.as_str() {
        MANUAL_CAMERA | CUSTOM_CAMERA => stored_name,
        known if known_camera(&cameras(), known).is_some() => stored_name,
        _ => CUSTOM_CAMERA.to_string(),
    };
    let named = named_camera(&Value::Object(stored_calc), &name);
    let manual = name == MANUAL_CAMERA;
    let by_distance = named.get("ValueSetIsDistance").and_then(Value::as_bool).unwrap_or(true);
    let settled = match manual || !by_distance {
        true => {
            let mut lowered = named.clone();
            lowered["DistanceToSurface"] = json!(fresh.default_altitude);
            recalculated(&lowered)
        }
        false => named,
    };
    let calc: serde_json::Map<String, Value> = settled.as_object().map(|o| o.iter().filter(|(k, _)| !manual || SAVED_BY_EVERY_CAMERA.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
    let turnaround = if fresh.multirotor { "TurnAroundDistanceMultiRotor" } else { "TurnAroundDistance" };
    json!({
        "CameraCalc": calc,
        "CameraTriggerInTurnAround": remembered(fresh, group, TRANSECT_META, "CameraTriggerInTurnAround"),
        "HoverAndCapture": remembered(fresh, group, TRANSECT_META, "HoverAndCapture"),
        "Refly90Degrees": remembered(fresh, group, TRANSECT_META, "Refly90Degrees"),
        "TurnAroundDistance": remembered(fresh, group, TRANSECT_META, turnaround),
        "version": 2,
    })
}

pub fn fresh(fresh: &Fresh) -> Value {
    let alternates = match fresh.alternates {
        true => remembered(fresh, "Survey", SURVEY_META, "FlyAlternateTransects"),
        false => json!(false),
    };
    let polygon: Vec<Value> = crate::missionkinds::default_area(fresh.center.0, fresh.center.1).iter().map(|(lat, lon)| json!([lat, lon])).collect();
    regenerate(&json!({
        "TransectStyleComplexItem": fresh_transect(fresh, "Survey"),
        "angle": remembered(fresh, "Survey", SURVEY_META, "GridAngle"),
        "complexItemType": "survey",
        "entryLocation": 0,
        "flyAlternateTransects": alternates,
        "polygon": polygon,
        "splitConcavePolygons": remembered(fresh, "Survey", SURVEY_META, "SplitConcavePolygons"),
        "type": "ComplexItem",
        "version": 5,
    }))
}

const CORRIDOR_META: &str = include_str!("../../src/MissionManager/CorridorScan.SettingsGroup.json");

pub fn fresh_corridor(fresh: &Fresh) -> Value {
    let polyline: Vec<Value> = crate::missionkinds::default_line(fresh.center.0, fresh.center.1).iter().map(|(lat, lon)| json!([lat, lon])).collect();
    regenerate_corridor(&json!({
        "CorridorWidth": remembered(fresh, "CorridorScan", CORRIDOR_META, "CorridorWidth"),
        "EntryPoint": 0,
        "TransectStyleComplexItem": fresh_transect(fresh, "CorridorScan"),
        "complexItemType": "CorridorScan",
        "polyline": polyline,
        "type": "ComplexItem",
        "version": 2,
    }))
}

pub fn regenerate_corridor(corridor: &Value) -> Value {
    let transect = corridor.get("TransectStyleComplexItem").cloned().unwrap_or(Value::Null);
    let calc = transect.get("CameraCalc").cloned().unwrap_or(Value::Null);
    let polyline: Vec<Point> = corridor.get("polyline").and_then(Value::as_array).map(|p| p.iter().filter_map(|v| Some((v.get(0)?.as_f64()?, v.get(1)?.as_f64()?))).collect()).unwrap_or_default();
    let params = crate::corridorscan::Params {
        width: number(corridor, "CorridorWidth").unwrap_or(0.0),
        spacing: number(&calc, "AdjustedFootprintSide").unwrap_or(0.0),
        turnaround: number(&transect, "TurnAroundDistance").unwrap_or(0.0),
        entry: corridor.get("EntryPoint").and_then(Value::as_i64).unwrap_or(0),
    };
    let transects = crate::corridorscan::typed_transects(&polyline, &params);
    let trigger_distance = f64::from(number(&calc, "AdjustedFootprintFrontal").unwrap_or(0.0) as f32);
    let in_turnaround = flag(&transect, "CameraTriggerInTurnAround");
    let length: f64 = polyline.windows(2).map(|pair| surveygrid::distance_between(pair[0], pair[1])).sum();
    let shots = |complex_distance: f64| match (trigger_distance == 0.0, in_turnaround) {
        (true, _) => 0,
        (false, true) => (complex_distance / trigger_distance).ceil() as i64,
        (false, false) => (length / trigger_distance).ceil() as i64 * transects.len() as i64,
    };
    let mut changed = corridor.clone();
    changed["TransectStyleComplexItem"] = rebuilt(&transect, &calc, &transects, trigger_distance, in_turnaround, shots);
    changed
}

fn rebuilt(transect: &Value, calc: &Value, transects: &[Vec<Coord>], trigger_distance: f64, in_turnaround: bool, shots: impl Fn(f64) -> i64) -> Value {
    let plan = Plan {
        altitude: number(calc, "DistanceToSurface").unwrap_or(0.0),
        trigger_distance,
        altitude_mode: calc.get("DistanceMode").and_then(Value::as_i64).unwrap_or(crate::altitudemodes::RELATIVE),
        images_in_turnaround: in_turnaround,
    };
    let visual: Vec<Point> = transects.iter().flatten().map(|c| c.at).collect();
    let complex_distance: f64 = visual.windows(2).map(|pair| surveygrid::distance_between(pair[0], pair[1])).sum();
    let items: Vec<Value> = surveyitems::items(transects, &plan)
        .iter()
        .enumerate()
        .map(|(i, item)| json!({ "autoContinue": true, "command": item.command, "doJumpId": i + 1, "frame": item.frame, "params": item.params, "type": "SimpleItem" }))
        .collect();
    let mut rebuilt = transect.clone();
    rebuilt["Items"] = Value::Array(items);
    rebuilt["VisualTransectPoints"] = json!(visual.iter().map(|(lat, lon)| json!([lat, lon])).collect::<Vec<_>>());
    rebuilt["CameraShots"] = json!(shots(complex_distance));
    rebuilt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_corridor_starts_from_the_remembered_corridor_settings_as_qt_builds_it() {
        let fixture: Value = serde_json::from_str(include_str!("../tests/fixtures/corridor-inserted-by-qt.json")).unwrap();
        let remembered = |key: &str| key.strip_prefix("CorridorScan/").and_then(|name| fixture["remembered"][name].as_str()).map(str::to_string);
        let mut built = fresh_corridor(&Fresh {
            center: (fixture["center"][0].as_f64().unwrap(), fixture["center"][1].as_f64().unwrap()),
            remembered: &remembered,
            multirotor: true,
            alternates: false,
            default_altitude: fixture["defaultAltitude"].as_f64().unwrap(),
            distance_mode: crate::altitudemodes::RELATIVE,
        });
        built["TransectStyleComplexItem"]["Items"].as_array_mut().unwrap().iter_mut().for_each(|item| item["doJumpId"] = json!(item["doJumpId"].as_i64().unwrap() + 1));
        assert_eq!(by_value(&built), by_value(&fixture["corridor"]));
    }

    #[test]
    fn a_new_survey_starts_from_the_remembered_survey_settings_as_qt_builds_it() {
        let fixture: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-inserted-by-qt.json")).unwrap();
        let remembered = |key: &str| key.strip_prefix("Survey/").and_then(|name| fixture["remembered"][name].as_str()).map(str::to_string);
        let mut built = fresh(&Fresh {
            center: (fixture["center"][0].as_f64().unwrap(), fixture["center"][1].as_f64().unwrap()),
            remembered: &remembered,
            multirotor: true,
            alternates: false,
            default_altitude: fixture["defaultAltitude"].as_f64().unwrap(),
            distance_mode: crate::altitudemodes::RELATIVE,
        });
        built["TransectStyleComplexItem"]["Items"].as_array_mut().unwrap().iter_mut().for_each(|item| item["doJumpId"] = json!(item["doJumpId"].as_i64().unwrap() + 1));
        assert_eq!(by_value(&built), by_value(&fixture["survey"]));
    }

    fn by_value(value: &Value) -> Value {
        match value {
            Value::Number(n) => json!((n.as_f64().unwrap() * 1e9).round() / 1e9),
            Value::Array(items) => Value::Array(items.iter().map(by_value).collect()),
            Value::Object(fields) => Value::Object(fields.iter().map(|(k, v)| (k.clone(), by_value(v))).collect()),
            other => other.clone(),
        }
    }

    #[test]
    fn a_camera_write_recomputes_the_footprint_and_the_survey_regenerates() {
        let plan: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-upload.plan")).unwrap();
        let survey = plan["mission"]["items"][0].clone();
        let higher = set(&survey, "cameraCalc.distanceToSurface", &json!(100.0)).unwrap();
        let calc = &higher["TransectStyleComplexItem"]["CameraCalc"];
        assert!((calc["AdjustedFootprintSide"].as_f64().unwrap() - 2.0 * survey["TransectStyleComplexItem"]["CameraCalc"]["AdjustedFootprintSide"].as_f64().unwrap()).abs() < 1e-9, "doubling the height doubles the footprint");
        assert!(higher["TransectStyleComplexItem"]["Items"].as_array().unwrap().len() < survey["TransectStyleComplexItem"]["Items"].as_array().unwrap().len(), "wider spacing means fewer transects");
        let turned = set(&survey, "gridAngle", &json!(0.0)).unwrap();
        assert_eq!(turned["angle"], 0.0);
        assert!(set(&survey, "noSuchField", &json!(1)).is_none());
    }

    #[test]
    fn choosing_a_known_camera_takes_its_optics_from_the_database() {
        let plan: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-upload.plan")).unwrap();
        let survey = plan["mission"]["items"][0].clone();
        let sony = set(&survey, "cameraCalc.cameraBrand", &json!("Sony")).unwrap();
        let calc = &sony["TransectStyleComplexItem"]["CameraCalc"];
        let first = cameras().into_iter().find(|c| c["brand"] == "Sony").unwrap();
        assert_eq!(calc["CameraName"], first["canonicalName"]);
        assert_eq!(calc["SensorWidth"], first["sensorWidth"]);
        let metres = crate::read::Unit { name: "m".to_string(), factor: 1.0 };
        let described = camera(&sony, "p", &Units { vertical: &metres, horizontal: &metres });
        assert_eq!((described["brand"].as_str(), described["model"].as_str()), (Some("Sony"), first["model"].as_str()));
        let manual = set(&sony, "cameraCalc.cameraBrand", &json!(MANUAL_CAMERA)).unwrap();
        assert_eq!(manual["TransectStyleComplexItem"]["CameraCalc"]["ValueSetIsDistance"], true);
        assert!(set(&survey, "cameraCalc.cameraModel", &json!("anything")).is_none(), "a custom camera has no models to pick from");
    }

    #[test]
    fn a_survey_editor_offers_the_fields_and_camera_qt_offers() {
        let plan: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-upload.plan")).unwrap();
        let survey = plan["mission"]["items"][0].clone();
        let qt: Value = serde_json::from_str(include_str!("../tests/fixtures/itemfacts-survey-by-qt.json")).unwrap();
        let item = "plan.missionController.visualItems.1";
        let metres = crate::read::Unit { name: "m".to_string(), factor: 1.0 };
        let units = Units { vertical: &metres, horizontal: &metres };
        let mine = json!({ "fields": fields(&survey, item, true, &units), "camera": camera(&survey, item, &units) });
        let rows = |v: &Value, key: &str| v[key].as_array().cloned().unwrap_or_default();
        assert_eq!(rows(&mine, "fields").len(), rows(&qt, "fields").len());
        rows(&mine, "fields").iter().zip(rows(&qt, "fields")).for_each(|(core, qt)| {
            let (core, qt) = (by_value(core), by_value(&qt));
            let differing: Vec<String> = qt.as_object().unwrap().iter().filter(|(k, v)| core.get(k.as_str()) != Some(v)).map(|(k, v)| format!("{k}: core {} qt {v}", core.get(k.as_str()).unwrap_or(&Value::Null))).collect();
            assert!(differing.is_empty(), "{}: {}", qt["name"], differing.join("; "));
        });
        let facts = |v: &Value| v["camera"]["facts"].as_array().cloned().unwrap_or_default();
        facts(&mine).iter().zip(facts(&qt)).for_each(|(core, qt)| {
            let (core, qt) = (by_value(core), by_value(&qt));
            let differing: Vec<String> = qt.as_object().unwrap().iter().filter(|(k, v)| core.get(k.as_str()) != Some(v)).map(|(k, v)| format!("{k}: core {} qt {v}", core.get(k.as_str()).unwrap_or(&Value::Null))).collect();
            assert!(differing.is_empty(), "{}: {}", qt["name"], differing.join("; "));
        });
        ["brand", "model", "brands", "models", "custom", "distanceMode", "brandPath", "modelPath"].iter().for_each(|key| assert_eq!(mine["camera"][key], qt["camera"][key], "{key}"));
    }

    #[test]
    fn a_survey_regenerates_the_items_points_and_shots_qt_saved_for_it() {
        let plan: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-upload.plan")).unwrap();
        let saved = plan["mission"]["items"][0].clone();
        let rebuilt = regenerate(&saved);
        ["Items", "VisualTransectPoints", "CameraShots"].iter().for_each(|key| {
            assert_eq!(by_value(&rebuilt["TransectStyleComplexItem"][key]), by_value(&saved["TransectStyleComplexItem"][key]), "{key}");
        });
    }
}
