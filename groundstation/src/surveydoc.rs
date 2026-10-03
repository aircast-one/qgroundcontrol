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

fn camera_shots(transects: &[Vec<Coord>], trigger_distance: f64, in_turnaround: bool, hover: bool, complex_distance: f64) -> i64 {
    match (trigger_distance == 0.0, in_turnaround) {
        (true, _) => 0,
        (false, true) => (complex_distance / trigger_distance).ceil() as i64,
        (false, false) => transects
            .iter()
            .filter_map(|transect| {
                let cameras: Vec<&Coord> = transect.iter().filter(|c| hover || c.kind != Kind::Turnaround).collect();
                Some((cameras.first()?.at, cameras.last()?.at))
            })
            .map(|(first, last)| (surveygrid::distance_between(first, last) / trigger_distance).ceil() as i64)
            .sum(),
    }
}

fn with_hover_points(transects: Vec<Vec<Coord>>, trigger_distance: f64, hover: bool) -> Vec<Vec<Coord>> {
    if !hover || trigger_distance <= 0.0 {
        return transects;
    }
    transects
        .into_iter()
        .map(|transect| {
            let entry = transect.iter().position(|c| c.kind == surveygrid::Kind::SurveyEntry);
            let exit = transect.iter().position(|c| c.kind == surveygrid::Kind::SurveyExit);
            let Some((entry, exit)) = entry.zip(exit) else { return transect };
            let (from, to) = (transect[entry].at, transect[exit].at);
            let length = surveygrid::distance_between(from, to);
            let azimuth = surveygrid::azimuth_to(from, to);
            let count = if trigger_distance < length { (length / trigger_distance).floor() as usize } else { 0 };
            let hovers = (1..=count).map(|i| Coord { at: surveygrid::at_distance_and_azimuth(from, trigger_distance * i as f64, azimuth), kind: surveygrid::Kind::InteriorHoverTrigger });
            transect[..=entry].iter().copied().chain(hovers).chain(transect[entry + 1..].iter().copied()).collect()
        })
        .collect()
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
    let trigger_distance = f64::from(number(&calc, "AdjustedFootprintFrontal").unwrap_or(0.0) as f32);
    let hover = flag(&transect, "HoverAndCapture");
    let transects = with_hover_points(surveygrid::typed_transects(&polygon(survey), &params), trigger_distance, hover);
    let in_turnaround = flag(&transect, "CameraTriggerInTurnAround");
    let mut changed = survey.clone();
    changed["TransectStyleComplexItem"] = rebuilt(&transect, &calc, &transects, trigger_distance, in_turnaround, |complex_distance| camera_shots(&transects, trigger_distance, in_turnaround, hover, complex_distance));
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

const BUILT_IN_UNITS: [(&str, &str, f64); 6] = [
    ("centi-degrees", "deg", 0.01),
    ("radians", "deg", 180.0 / std::f64::consts::PI),
    ("rad", "deg", 180.0 / std::f64::consts::PI),
    ("gimbal-degrees", "deg", -1.0),
    ("norm", "%", 100.0),
    ("centi-celsius", "C", 0.01),
];

pub fn cooked_unit(raw: &str, units: &Units) -> Option<crate::read::Unit> {
    cooked(raw, units)
}

fn cooked(raw: &str, units: &Units) -> Option<crate::read::Unit> {
    match raw {
        "vertical m" => Some(units.vertical.clone()),
        "m" | "meter" | "meters" | "horizontal m" => Some(units.horizontal.clone()),
        other => BUILT_IN_UNITS.iter().find(|(name, _, _)| *name == other).map(|(_, shown, factor)| crate::read::Unit { name: shown.to_string(), factor: *factor }),
    }
}

fn fact(meta: &crate::factmeta::MetaData, value: Value, units: &Units) -> Value {
    let decimals = meta.decimal_places.unwrap_or(DEFAULT_DECIMAL_PLACES);
    let whole = integer_typed(&meta.value_type);
    let shown = |v: &Value, d: i64| shown(v, if whole { 0 } else { d });
    let bool_typed = meta.value_type == crate::factmeta::ValueType::Bool;
    let raw_units = meta.units.clone().unwrap_or_default();
    let unit = cooked(&raw_units, units);
    let unit = unit.as_ref();
    let cook = |v: f64| unit.map_or(v, |u| u.show(v)) + 0.0;
    let number = |v: &Option<Value>| v.as_ref().and_then(Value::as_f64).map(cook);
    let (first, second) = (number(&meta.min), number(&meta.max));
    let (min, max) = match (first, second) {
        (Some(a), Some(b)) if a > b => (Some(b), Some(a)),
        bounds => bounds,
    };
    let raw = value.clone();
    let value = match (unit, value.as_f64()) {
        (Some(_), Some(v)) => json!(cook(v)),
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

pub fn fact_control(file: &str, name: &str, value: Value, item: &str, suffix: &str, units: &Units) -> Option<Value> {
    let meta = meta(file, name)?;
    let typed = crate::settingsstore::typed(&meta.value_type, &value).unwrap_or(value);
    Some(control(&meta, typed, item, suffix, "Settings", units))
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

const TERRAIN_ADJUST: [&str; 3] = ["TerrainAdjustTolerance", "TerrainAdjustMaxClimbRate", "TerrainAdjustMaxDescentRate"];
const NOT_FOLLOWING_TERRAIN: &str = "Only applies when the altitude is calculated above terrain.";

fn terrain_gated(control: Value, name: &str, follows_terrain: bool) -> Value {
    match (control, TERRAIN_ADJUST.contains(&name) && !follows_terrain) {
        (Value::Object(mut fields), true) => {
            fields.insert("enabled".to_string(), json!(false));
            fields.insert("disabledReason".to_string(), json!(NOT_FOLLOWING_TERRAIN));
            Value::Object(fields)
        }
        (other, _) => other,
    }
}

pub fn grid_label(structure: bool, property: &str) -> Option<&'static str> {
    match (structure, property) {
        (true, "distanceToSurface") => Some("Scan distance"),
        (false, "distanceToSurface") => Some("Altitude"),
        (_, "imageDensity") => Some("Ground resolution"),
        (_, "frontalOverlap") => Some("Front overlap"),
        (_, "sideOverlap") => Some("Side overlap"),
        (true, "adjustedFootprintFrontal") => Some("Layer height"),
        (true, "adjustedFootprintSide") => Some("Trigger distance"),
        (false, "adjustedFootprintFrontal") => Some("Trigger distance"),
        (false, "adjustedFootprintSide") => Some("Spacing"),
        _ => None,
    }
}

pub fn row_label(vtol_landing: bool, property: &str) -> Option<&'static str> {
    match (vtol_landing, property) {
        (_, "gridAngle") => Some("Angle"),
        (_, "corridorWidth") => Some("Width"),
        (_, "turnAroundDistance") => Some("Turnaround distance"),
        (_, "hoverAndCapture") => Some("Hover to capture each image"),
        (_, "refly90Degrees") => Some("Refly at 90° for a cross grid"),
        (_, "cameraTriggerInTurnAround") => Some("Images in turnarounds"),
        (_, "flyAlternateTransects") => Some("Fly alternate transects"),
        (_, "terrainAdjustTolerance") => Some("Tolerance"),
        (_, "terrainAdjustMaxClimbRate") => Some("Max Climb Rate"),
        (_, "terrainAdjustMaxDescentRate") => Some("Max Descent Rate"),
        (_, "structureHeight") => Some("Structure height"),
        (_, "scanBottomAlt") => Some("Scan bottom altitude"),
        (_, "entranceAlt") => Some("Entrance and exit altitude"),
        (_, "gimbalPitch") => Some("Gimbal pitch"),
        (_, "startFromTop") => Some("Start from"),
        (_, "useLoiterToAlt") => Some("Use loiter to altitude"),
        (_, "loiterRadius") => Some("Radius"),
        (_, "loiterClockwise") => Some("Loiter clockwise"),
        (_, "landingHeading") => Some("Heading"),
        (_, "finalApproachAltitude" | "landingAltitude") => Some("Altitude"),
        (false, "useDoChangeSpeed") => Some("Flight Speed"),
        (false, "landingDistance") => Some("Distance"),
        (true, "landingDistance") => Some("Landing Dist"),
        (false, "glideSlope") => Some("Glide Slope"),
        _ => None,
    }
}

pub fn row_labelled(control: Value, vtol_landing: bool, property: &str) -> Value {
    let shown = match row_label(vtol_landing, property) {
        Some(label) => labelled(control, label),
        None => control,
    };
    match (property, shown) {
        ("startFromTop", Value::Object(mut fields)) => {
            fields.insert("options".to_string(), json!([{ "label": "Bottom", "raw": "false" }, { "label": "Top", "raw": "true" }]));
            Value::Object(fields)
        }
        (_, other) => other,
    }
}

const STRUCTURE_ROWS: [(&str, &str); 5] = [("StartFromTop", "startFromTop"), ("StructureHeight", "structureHeight"), ("ScanBottomAlt", "scanBottomAlt"), ("EntranceAltitude", "entranceAlt"), ("GimbalPitch", "gimbalPitch")];

pub fn editor_rank(property: &str) -> usize {
    STRUCTURE_ROWS.iter().position(|(_, suffix)| *suffix == property).unwrap_or(STRUCTURE_ROWS.len())
}

pub fn labelled(control: Value, label: &str) -> Value {
    match control {
        Value::Object(mut fields) => {
            fields.insert("label".to_string(), json!(label));
            fields.insert("shortLabel".to_string(), json!(label));
            Value::Object(fields)
        }
        other => other,
    }
}

fn read_only(control: Value) -> Value {
    match control {
        Value::Object(mut fields) => {
            fields.insert("readOnly".to_string(), json!(true));
            Value::Object(fields)
        }
        other => other,
    }
}

fn disabled(control: Value, reason: &str) -> Value {
    match control {
        Value::Object(mut fields) => {
            fields.insert("enabled".to_string(), json!(false));
            fields.insert("disabledReason".to_string(), json!(reason));
            Value::Object(fields)
        }
        other => other,
    }
}

const HOVER_NEEDS_FIXED_ALTITUDE: &str = "Only with a relative or absolute altitude.";
const REFLY_NOT_WITH_TERRAIN: &str = "Not while the altitude is calculated above terrain.";
const TURNAROUND_NOT_WITH_HOVER: &str = "Not while hovering to capture each image.";

pub fn editor_shows(name: &str, class: crate::cmdinfo::VehicleClass) -> bool {
    use crate::cmdinfo::VehicleClass::{FixedWing, Vtol};
    match name {
        "SplitConcavePolygons" | "Layers" => false,
        "FlyAlternateTransects" => matches!(class, FixedWing | Vtol),
        _ => true,
    }
}

pub fn fields(survey: &Value, item: &str, class: crate::cmdinfo::VehicleClass, units: &Units) -> Vec<Value> {
    use crate::cmdinfo::VehicleClass::{MultiRotor, Vtol};
    let multirotor = class == MultiRotor;
    let hover_allowed = matches!(class, MultiRotor | Vtol);
    let transect = survey.get("TransectStyleComplexItem").cloned().unwrap_or(Value::Null);
    let turnaround = if multirotor { "TurnAroundDistanceMultiRotor" } else { "TurnAroundDistance" };
    let corridor = survey.get("complexItemType").and_then(Value::as_str) == Some("CorridorScan");
    let listed: Vec<(&str, &str, &str, &Value, &str)> = vec![
        (TRANSECT_META, turnaround, "turnAroundDistance", &transect, "TurnAroundDistance"),
        (TRANSECT_META, "CameraTriggerInTurnAround", "cameraTriggerInTurnAround", &transect, "CameraTriggerInTurnAround"),
        (TRANSECT_META, "HoverAndCapture", "hoverAndCapture", &transect, "HoverAndCapture"),
        (TRANSECT_META, "Refly90Degrees", "refly90Degrees", &transect, "Refly90Degrees"),
        (TRANSECT_META, "TerrainAdjustTolerance", "terrainAdjustTolerance", &transect, "TerrainAdjustTolerance"),
        (TRANSECT_META, "TerrainAdjustMaxDescentRate", "terrainAdjustMaxDescentRate", &transect, "TerrainAdjustMaxDescentRate"),
        (TRANSECT_META, "TerrainAdjustMaxClimbRate", "terrainAdjustMaxClimbRate", &transect, "TerrainAdjustMaxClimbRate"),
    ];
    let own: Vec<(&str, &str, &str, &Value, &str)> = match corridor {
        true => vec![(CORRIDOR_META, "CorridorWidth", "corridorWidth", survey, "CorridorWidth")],
        false => vec![
            (SURVEY_META, "GridAngle", "gridAngle", survey, "angle"),
            (SURVEY_META, "FlyAlternateTransects", "flyAlternateTransects", survey, "flyAlternateTransects"),
        ],
    };
    let manual_camera = survey.pointer("/CameraCalc/CameraName").and_then(Value::as_str).is_none_or(|name| name == MANUAL_CAMERA);
    let structure: Vec<(&str, &str, &str, &Value, &str)> = STRUCTURE_ROWS
        .iter()
        .filter(|(name, _)| *name != "GimbalPitch" || manual_camera)
        .map(|(name, suffix)| (STRUCTURE_META, *name, *suffix, survey, *name))
        .collect();
    let chosen = match is_structure(survey) {
        true => structure,
        false => listed.into_iter().chain(own).collect(),
    };
    let distance_mode = Some(calc_of(survey).get("DistanceMode").and_then(Value::as_i64).unwrap_or(crate::altitudemodes::FRAME_RELATIVE));
    let follows_terrain = distance_mode == Some(crate::altitudemodes::FRAME_CALC_ABOVE_TERRAIN);
    let fixed_altitude = matches!(distance_mode, Some(crate::altitudemodes::FRAME_RELATIVE | crate::altitudemodes::FRAME_ABSOLUTE));
    let hovering = hover_allowed && transect.get("HoverAndCapture").and_then(Value::as_bool) == Some(true);
    chosen
        .into_iter()
        .filter(|(_, name, ..)| *name != "HoverAndCapture" || hover_allowed)
        .filter(|(_, name, ..)| editor_shows(name, class))
        .filter(|(_, name, ..)| !corridor || !matches!(*name, "HoverAndCapture" | "Refly90Degrees"))
        .filter_map(|(file, name, suffix, owner, key)| {
            let meta = meta(file, name)?;
            let value = with_default(owner.get(key), &meta);
            let shown = row_labelled(terrain_gated(control(&meta, value, item, suffix, "Settings", units), name, follows_terrain), false, suffix);
            Some(match name {
                "HoverAndCapture" if !fixed_altitude => disabled(shown, HOVER_NEEDS_FIXED_ALTITUDE),
                "Refly90Degrees" if follows_terrain => disabled(shown, REFLY_NOT_WITH_TERRAIN),
                "CameraTriggerInTurnAround" if hovering => disabled(shown, TURNAROUND_NOT_WITH_HOVER),
                _ => shown,
            })
        })
        .collect()
}

const OPTICS: [(&str, &str); 7] = [("SensorWidth", "sensorWidth"), ("SensorHeight", "sensorHeight"), ("ImageWidth", "imageWidth"), ("ImageHeight", "imageHeight"), ("FocalLength", "focalLength"), ("Landscape", "landscape"), ("MinTriggerInterval", "minTriggerInterval")];
const SENSOR_ROWS: usize = 5;
const FLIGHT: [(&str, &str); 4] = [("DistanceToSurface", "distanceToSurface"), ("ImageDensity", "imageDensity"), ("FrontalOverlap", "frontalOverlap"), ("SideOverlap", "sideOverlap")];
const MANUAL_SPACING: [(&str, &str); 2] = [("AdjustedFootprintFrontal", "adjustedFootprintFrontal"), ("AdjustedFootprintSide", "adjustedFootprintSide")];

fn cameras() -> Vec<Value> {
    serde_json::from_str::<Value>(CAMERA_LIST).ok().and_then(|v| v.get("cameraMetaData").and_then(Value::as_array).cloned()).unwrap_or_default()
}

fn known_camera<'a>(known: &'a [Value], name: &str) -> Option<&'a Value> {
    known.iter().find(|c| c.get("canonicalName").and_then(Value::as_str) == Some(name))
}

fn is_structure(item: &Value) -> bool {
    item.get("complexItemType").and_then(Value::as_str) == Some("StructureScan")
}

pub fn calc_of(item: &Value) -> Value {
    match is_structure(item) {
        true => item.get("CameraCalc").cloned(),
        false => item.get("TransectStyleComplexItem").and_then(|t| t.get("CameraCalc")).cloned(),
    }
    .unwrap_or(Value::Null)
}

fn with_calc(item: &Value, calc: Value) -> Value {
    let mut changed = item.clone();
    match is_structure(item) {
        true => {
            if calc.get("CameraName").and_then(Value::as_str).is_some_and(|name| name != MANUAL_CAMERA) {
                changed["GimbalPitch"] = json!(0);
            }
            changed["CameraCalc"] = calc;
        }
        false => changed["TransectStyleComplexItem"]["CameraCalc"] = calc,
    }
    changed
}

pub fn camera(survey: &Value, item: &str, units: &Units, terrain_frame: bool) -> Value {
    let calc = calc_of(survey);
    let name = calc.get("CameraName").and_then(Value::as_str).unwrap_or(MANUAL_CAMERA).to_string();
    let known = cameras();
    let listed = known_camera(&known, &name);
    let brand = listed.and_then(|c| c.get("brand").and_then(Value::as_str)).map_or_else(|| name.clone(), str::to_string);
    let model = listed.and_then(|c| c.get("model").and_then(Value::as_str)).unwrap_or("").to_string();
    let custom = name == CUSTOM_CAMERA;
    let name_is_manual = name == MANUAL_CAMERA;
    let brands: Vec<String> = [MANUAL_CAMERA.to_string(), CUSTOM_CAMERA.to_string()]
        .into_iter()
        .chain(known.iter().filter_map(|c| c.get("brand").and_then(Value::as_str).map(str::to_string)))
        .fold(Vec::new(), |seen, b| if seen.contains(&b) { seen } else { seen.into_iter().chain(std::iter::once(b)).collect() });
    let models: Vec<String> = known.iter().filter(|c| c.get("brand").and_then(Value::as_str) == Some(brand.as_str())).filter_map(|c| c.get("model").and_then(Value::as_str).map(str::to_string)).collect();
    let fixed_orientation = calc.get("FixedOrientation").and_then(Value::as_bool).unwrap_or(false);
    let optics: Vec<(&str, &str)> = match (custom, name_is_manual) {
        (true, _) => OPTICS.to_vec(),
        (false, true) => Vec::new(),
        (false, false) => OPTICS[..SENSOR_ROWS].iter().chain(OPTICS[SENSOR_ROWS..=SENSOR_ROWS].iter().filter(|_| !fixed_orientation)).copied().collect(),
    };
    let wanted: Vec<(&str, &str)> = optics.iter().chain(FLIGHT.iter()).chain(MANUAL_SPACING.iter()).copied().collect();
    let sensor_read_only = |name: &str| !custom && OPTICS[..SENSOR_ROWS].iter().any(|(sensor, _)| *sensor == name);
    let facts: Vec<Value> = wanted
        .iter()
        .filter_map(|(name, suffix)| {
            let meta = meta(CAMERA_META, name).or_else(|| meta(CAMERA_SPEC_META, name))?;
            let built = control(&meta, with_default(calc.get(*name), &meta), item, &format!("cameraCalc.{suffix}"), "Camera", units);
            let shown = match grid_label(is_structure(survey), suffix) {
                Some(label) => labelled(built, label),
                None => built,
            };
            let spacing = MANUAL_SPACING.iter().any(|(spacing, _)| spacing == name);
            Some(match (spacing && !name_is_manual) || sensor_read_only(name) {
                true => read_only(shown),
                false => shown,
            })
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
        "distanceModes": crate::altitudemodes::transect_distance_modes(name == MANUAL_CAMERA, terrain_frame),
        "distanceModePath": format!("{item}.cameraCalc.distanceMode"),
        "valueSetIsDistance": calc.get("ValueSetIsDistance").and_then(Value::as_bool).unwrap_or(true),
        "valueSetIsDistancePath": format!("{item}.cameraCalc.valueSetIsDistance"),
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
        "corridorWidth" => Some(("survey", "CorridorWidth".to_string())),
        "entranceAlt" => Some(("survey", "EntranceAltitude".to_string())),
        "structureHeight" | "scanBottomAlt" | "gimbalPitch" | "startFromTop" => Some(("survey", suffix.chars().next().map(|c| c.to_ascii_uppercase().to_string() + &suffix[1..]).unwrap_or_default())),
        "flyAlternateTransects" | "splitConcavePolygons" => Some(("survey", suffix.to_string())),
        _ => match suffix.strip_prefix("cameraCalc.") {
            Some(calc) => {
                let key = capital(calc);
                OPTICS.iter().chain(FLIGHT.iter()).chain(MANUAL_SPACING.iter()).any(|(name, _)| *name == key).then_some(("calc", key)).or((calc == "valueSetIsDistance").then(|| ("calc", "ValueSetIsDistance".to_string()))).or((calc == "distanceMode").then(|| ("calc", "DistanceMode".to_string())))
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

const CALC_FACTS: [&str; 4] = ["ValueSetIsDistance", "ImageDensity", "FrontalOverlap", "SideOverlap"];

fn meta_default(file: &str, name: &str) -> Value {
    meta(file, name).and_then(|m| m.default.as_ref().map(|d| crate::settingsstore::typed(&m.value_type, d).unwrap_or_else(|| d.clone()))).unwrap_or(Value::Null)
}

fn named_camera(calc: &Value, name: &str) -> Value {
    let known = cameras();
    let mut changed = calc.clone();
    changed["CameraName"] = json!(name);
    CALC_FACTS
        .iter()
        .map(|key| (*key, CAMERA_META))
        .chain(SPECS.iter().map(|(key, _)| (*key, CAMERA_SPEC_META)))
        .filter(|(key, _)| calc.get(*key).is_none_or(Value::is_null))
        .for_each(|(key, file)| changed[key] = meta_default(file, key));
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

pub fn regenerate_item(item: &Value) -> Value {
    match item.get("complexItemType").and_then(Value::as_str) {
        Some("CorridorScan") => regenerate_corridor(item),
        Some("StructureScan") => relayered(item),
        _ => regenerate(item),
    }
}

pub fn set(survey: &Value, suffix: &str, value: &Value, units: &Units) -> Option<Value> {
    let calc = calc_of(survey);
    let raw = |key: &str| {
        let unit = [CAMERA_META, CAMERA_SPEC_META, TRANSECT_META, SURVEY_META, CORRIDOR_META, STRUCTURE_META].iter().find_map(|file| meta(file, key)).and_then(|m| cooked(m.units.as_deref().unwrap_or(""), units));
        match (unit, value.as_f64()) {
            (Some(u), Some(v)) => json!(u.meters(v)),
            _ => value.clone(),
        }
    };
    let changed = match suffix {
        "cameraCalc.cameraBrand" | "cameraCalc.cameraModel" => with_calc(survey, named_camera(&calc, &chosen_camera(&calc, suffix, value)?)),
        _ => {
            let (owner, key) = target(suffix)?;
            let manual = calc.get("CameraName").and_then(Value::as_str).is_none_or(|name| name == MANUAL_CAMERA);
            let custom = calc.get("CameraName").and_then(Value::as_str) == Some(CUSTOM_CAMERA);
            if (key.starts_with("AdjustedFootprint") && !manual) || (!custom && OPTICS[..SENSOR_ROWS].iter().any(|(sensor, _)| *sensor == key)) {
                return None;
            }
            match owner {
                "survey" => {
                    let mut changed = survey.clone();
                    changed[key.as_str()] = raw(&key);
                    changed
                }
                "transect" => {
                    let mut changed = survey.clone();
                    changed["TransectStyleComplexItem"][key.as_str()] = raw(&key);
                    if key == "HoverAndCapture" && raw(&key) == json!(true) {
                        changed["TransectStyleComplexItem"]["CameraTriggerInTurnAround"] = json!(false);
                    }
                    changed
                }
                _ => {
                    let mut edited = calc.clone();
                    edited[key.as_str()] = raw(&key);
                    let mut changed = with_calc(survey, recalculated(&edited));
                    if key == "DistanceMode" && raw(&key).as_i64() == Some(crate::altitudemodes::FRAME_CALC_ABOVE_TERRAIN) && changed.get("TransectStyleComplexItem").is_some() {
                        changed["TransectStyleComplexItem"]["Refly90Degrees"] = json!(false);
                        changed["TransectStyleComplexItem"]["HoverAndCapture"] = json!(false);
                    }
                    changed
                }
            }
        }
    };
    Some(regenerate_item(&changed))
}

fn relayered(scan: &Value) -> Value {
    let mut changed = scan.clone();
    changed["Layers"] = json!(crate::structurescan::saved_plan(scan).layers);
    changed
}

pub fn changed_remembered(item: &Value, multirotor: bool, stored: &dyn Fn(&str) -> Option<String>) -> Vec<(String, Value)> {
    if item.get("complexItemType").and_then(Value::as_str).is_some_and(crate::landingpattern::is_landing) {
        return Vec::new();
    }
    let kind = item.get("complexItemType").and_then(Value::as_str).unwrap_or("survey");
    let corridor = kind == "CorridorScan";
    let structure = kind == "StructureScan";
    let group = match kind {
        "CorridorScan" | "StructureScan" => kind,
        _ => "Survey",
    };
    let transect = item.get("TransectStyleComplexItem").cloned().unwrap_or(Value::Null);
    let calc = if structure { item.get("CameraCalc").cloned().unwrap_or(Value::Null) } else { transect.get("CameraCalc").cloned().unwrap_or(Value::Null) };
    let calc_names = ["CameraName", "ValueSetIsDistance", "DistanceToSurface", "ImageDensity", "FrontalOverlap", "SideOverlap", "AdjustedFootprintSide", "AdjustedFootprintFrontal", "SensorWidth", "SensorHeight", "ImageWidth", "ImageHeight", "FocalLength", "Landscape", "FixedOrientation", "MinTriggerInterval"];
    let turnaround = if multirotor { "TurnAroundDistanceMultiRotor" } else { "TurnAroundDistance" };
    let transect_names = [("CameraTriggerInTurnAround", "CameraTriggerInTurnAround"), ("HoverAndCapture", "HoverAndCapture"), ("Refly90Degrees", "Refly90Degrees"), (turnaround, "TurnAroundDistance"), ("TerrainAdjustTolerance", "TerrainAdjustTolerance"), ("TerrainAdjustMaxClimbRate", "TerrainAdjustMaxClimbRate"), ("TerrainAdjustMaxDescentRate", "TerrainAdjustMaxDescentRate")];
    let own: Vec<(&str, &str)> = match (corridor, structure) {
        (true, _) => vec![("CorridorWidth", "CorridorWidth")],
        (_, true) => ["EntranceAltitude", "ScanBottomAlt", "StructureHeight", "Layers", "GimbalPitch", "StartFromTop"].iter().map(|n| (*n, *n)).collect(),
        _ => vec![("GridAngle", "angle"), ("FlyAlternateTransects", "flyAlternateTransects"), ("SplitConcavePolygons", "splitConcavePolygons")],
    };
    let manual_implies: Vec<(String, Value)> = match calc.get("CameraName").and_then(Value::as_str) == Some(MANUAL_CAMERA) {
        true => vec![("FixedOrientation".to_string(), json!(false)), ("MinTriggerInterval".to_string(), json!(0))],
        false => Vec::new(),
    };
    calc_names
        .iter()
        .filter_map(|name| calc.get(*name).map(|v| (name.to_string(), v.clone())))
        .chain(manual_implies)
        .chain(transect_names.iter().filter_map(|(name, key)| transect.get(*key).map(|v| (name.to_string(), v.clone()))))
        .chain(own.iter().filter_map(|(name, key)| item.get(*key).map(|v| (name.to_string(), v.clone()))))
        .filter(|(name, value)| {
            let Some(meta) = [CAMERA_META, CAMERA_SPEC_META, TRANSECT_META, SURVEY_META, CORRIDOR_META, STRUCTURE_META].iter().find_map(|file| meta(file, name)) else { return true };
            let held = stored(&format!("{group}/{name}")).map(Value::String).or_else(|| meta.default.clone());
            let typed = |v: &Value| crate::settingsstore::typed(&meta.value_type, v);
            held.as_ref().and_then(typed) != typed(value)
        })
        .map(|(name, value)| (format!("{group}/{name}"), value))
        .collect()
}

pub struct Fresh<'a> {
    pub center: (f64, f64),
    pub remembered: &'a dyn Fn(&str) -> Option<String>,
    pub multirotor: bool,
    pub alternates: bool,
    pub default_altitude: f64,
    pub distance_mode: i64,
    pub previous_mode: Option<i64>,
}

const SAVED_BY_EVERY_CAMERA: [&str; 6] = ["version", "AdjustedFootprintSide", "AdjustedFootprintFrontal", "DistanceToSurface", "DistanceMode", "CameraName"];

fn remembered(fresh: &Fresh, group: &str, file: &str, name: &str) -> Value {
    let Some(meta) = meta(file, name) else { return Value::Null };
    let stored = (fresh.remembered)(&format!("{group}/{name}")).and_then(|text| crate::settingsstore::typed(&meta.value_type, &Value::String(text)));
    stored.or_else(|| meta.default.as_ref().map(|d| crate::settingsstore::typed(&meta.value_type, d).unwrap_or_else(|| d.clone()))).unwrap_or(Value::Null)
}

fn fresh_calc(fresh: &Fresh, group: &str, lowered_when_manual: bool) -> serde_json::Map<String, Value> {
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
    let settled = match lowered_when_manual && (manual || !by_distance) {
        true => {
            let mut lowered = named.clone();
            lowered["DistanceToSurface"] = json!(fresh.default_altitude);
            recalculated(&lowered)
        }
        false => named,
    };
    let calc: serde_json::Map<String, Value> = settled.as_object().map(|o| o.iter().filter(|(k, _)| !manual || SAVED_BY_EVERY_CAMERA.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
    calc.into_iter().map(|(k, v)| match (k.as_str(), fresh.previous_mode) {
        ("DistanceMode", Some(mode)) => (k, json!(mode)),
        _ => (k, v),
    }).collect()
}

fn fresh_transect(fresh: &Fresh, group: &str) -> Value {
    let calc = fresh_calc(fresh, group, true);
    let follows_terrain = calc.get("DistanceMode").and_then(Value::as_i64) == Some(crate::altitudemodes::FRAME_CALC_ABOVE_TERRAIN);
    let turnaround = if fresh.multirotor { "TurnAroundDistanceMultiRotor" } else { "TurnAroundDistance" };
    json!({
        "CameraCalc": calc,
        "CameraTriggerInTurnAround": remembered(fresh, group, TRANSECT_META, "CameraTriggerInTurnAround"),
        "HoverAndCapture": if follows_terrain { json!(false) } else { remembered(fresh, group, TRANSECT_META, "HoverAndCapture") },
        "Refly90Degrees": if follows_terrain { json!(false) } else { remembered(fresh, group, TRANSECT_META, "Refly90Degrees") },
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

const STRUCTURE_META: &str = include_str!("../../src/MissionManager/StructureScan.SettingsGroup.json");

pub fn fresh_structure(fresh: &Fresh) -> Value {
    let calc = fresh_calc(fresh, "StructureScan", false);
    let own = |name: &str| remembered(fresh, "StructureScan", STRUCTURE_META, name);
    let (height, bottom) = (own("StructureHeight").as_f64().unwrap_or(0.0), own("ScanBottomAlt").as_f64().unwrap_or(0.0));
    let frontal = calc.get("AdjustedFootprintFrontal").and_then(Value::as_f64).unwrap_or(0.0);
    let layers = ((height - bottom).max(0.0) / frontal).ceil().max(1.0);
    let polygon: Vec<Value> = crate::missionkinds::default_area(fresh.center.0, fresh.center.1).iter().map(|(lat, lon)| json!([lat, lon])).collect();
    json!({
        "CameraCalc": calc,
        "EntranceAltitude": fresh.default_altitude,
        "GimbalPitch": own("GimbalPitch"),
        "Layers": layers,
        "ScanBottomAlt": bottom,
        "StartFromTop": own("StartFromTop"),
        "StructureHeight": height,
        "complexItemType": "StructureScan",
        "polygon": polygon,
        "type": "ComplexItem",
        "version": 3,
    })
}

const SURVEY_ENTRY_NAMES: [&str; 4] = ["top left", "top right", "bottom left", "bottom right"];
const CORRIDOR_ENTRY_NAMES: [&str; 4] = ["start", "start, other side", "end", "end, other side"];

pub fn entry_point_name(kind: &str, entry: i64) -> Option<&'static str> {
    let names = match kind {
        "CorridorScan" => CORRIDOR_ENTRY_NAMES,
        "survey" => SURVEY_ENTRY_NAMES,
        _ => return None,
    };
    names.get(usize::try_from(entry).ok()?).copied()
}

pub fn entry_point(kind: &str, item: &Value) -> i64 {
    let key = if kind == "CorridorScan" { "EntryPoint" } else { "entryLocation" };
    item.get(key).and_then(Value::as_i64).unwrap_or(0)
}

pub fn rotated_entry(kind: &str, item: &Value) -> Option<Value> {
    if kind == "StructureScan" {
        return crate::structurescan::rotated_entry(item);
    }
    let entry = entry_point(kind, item);
    let (key, next) = match kind {
        "survey" => ("entryLocation", if entry >= 3 { 0 } else { entry + 1 }),
        "CorridorScan" => {
            let spacing = item.get("TransectStyleComplexItem").and_then(|t| t.get("CameraCalc")).and_then(|c| number(c, "AdjustedFootprintSide")).unwrap_or(0.0);
            let step = if crate::corridorscan::transect_count(number(item, "CorridorWidth").unwrap_or(0.0), spacing) < 2 { 2 } else { 1 };
            ("EntryPoint", if entry + step > 3 { 0 } else { entry + step })
        }
        _ => return None,
    };
    let mut moved = item.clone();
    moved[key] = json!(next);
    Some(regenerate_item(&moved))
}

pub fn corridor_shots(corridor: &Value) -> i64 {
    let transect = corridor.get("TransectStyleComplexItem").cloned().unwrap_or(Value::Null);
    let calc = transect.get("CameraCalc").cloned().unwrap_or(Value::Null);
    let trigger_distance = number(&calc, "AdjustedFootprintFrontal").unwrap_or(0.0);
    let points = |key: &str, from: &Value| -> Vec<Point> { from.get(key).and_then(Value::as_array).map(|p| p.iter().filter_map(|v| Some((v.get(0)?.as_f64()?, v.get(1)?.as_f64()?))).collect()).unwrap_or_default() };
    let length = |line: &[Point]| -> f64 { line.windows(2).map(|pair| surveygrid::distance_between(pair[0], pair[1])).sum() };
    let transects = crate::corridorscan::transect_count(number(corridor, "CorridorWidth").unwrap_or(0.0), number(&calc, "AdjustedFootprintSide").unwrap_or(0.0));
    match (trigger_distance == 0.0, flag(&transect, "CameraTriggerInTurnAround")) {
        (true, _) => 0,
        (false, true) => (length(&points("VisualTransectPoints", &transect)) / trigger_distance).ceil() as i64,
        (false, false) => (length(&points("polyline", corridor)) / trigger_distance).ceil() as i64 * transects as i64,
    }
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

const TERRAIN_TOLERANCE_DEFAULT: f64 = 10.0;
const TERRAIN_FLIGHT_SPEED_DEFAULT: f64 = 5.0;
pub const TERRAIN_FLIGHT_SPEED: &str = "TerrainFlightSpeed";

fn terrain_adjust(transect: &Value, distance_to_surface: f64) -> crate::terrainfollow::Adjust {
    crate::terrainfollow::Adjust {
        distance_to_surface,
        tolerance: number(transect, "TerrainAdjustTolerance").unwrap_or(TERRAIN_TOLERANCE_DEFAULT),
        max_climb_rate: number(transect, "TerrainAdjustMaxClimbRate").unwrap_or(0.0),
        max_descent_rate: number(transect, "TerrainAdjustMaxDescentRate").unwrap_or(0.0),
        flight_speed: number(transect, TERRAIN_FLIGHT_SPEED).unwrap_or(TERRAIN_FLIGHT_SPEED_DEFAULT),
    }
}

fn rebuilt(transect: &Value, calc: &Value, transects: &[Vec<Coord>], trigger_distance: f64, in_turnaround: bool, shots: impl Fn(f64) -> i64) -> Value {
    let plan = Plan {
        altitude: number(calc, "DistanceToSurface").unwrap_or(0.0),
        trigger_distance,
        altitude_mode: calc.get("DistanceMode").and_then(Value::as_i64).unwrap_or(crate::altitudemodes::RELATIVE),
        images_in_turnaround: in_turnaround,
        hover_and_capture: flag(transect, "HoverAndCapture"),
        condition_gate_supported: surveyitems::CONDITION_GATE_SUPPORTED.load(std::sync::atomic::Ordering::Relaxed),
    };
    let follows_terrain = plan.altitude_mode == crate::altitudemodes::CALC_ABOVE_TERRAIN;
    let adjust = terrain_adjust(transect, plan.altitude);
    let followed = follows_terrain.then(|| crate::terrainfollow::follow(transects, &adjust, &crate::terrainservice::height)).flatten();
    let built = match (follows_terrain, &followed) {
        (false, _) => surveyitems::items(transects, &plan),
        (true, Some(path)) => surveyitems::flown(&path.iter().map(|w| (w.coord, w.altitude)).collect::<Vec<_>>(), &plan),
        (true, None) => Vec::new(),
    };
    let visual: Vec<Point> = transects.iter().flatten().map(|c| c.at).collect();
    let complex_distance: f64 = visual.windows(2).map(|pair| surveygrid::distance_between(pair[0], pair[1])).sum();
    let items: Vec<Value> = built
        .iter()
        .enumerate()
        .map(|(i, item)| json!({ "autoContinue": true, "command": item.command, "doJumpId": i + 1, "frame": item.frame, "params": item.params, "type": "SimpleItem" }))
        .collect();
    let mut rebuilt = transect.clone();
    rebuilt["Items"] = Value::Array(items);
    rebuilt["VisualTransectPoints"] = json!(visual.iter().map(|(lat, lon)| json!([lat, lon])).collect::<Vec<_>>());
    rebuilt["CameraShots"] = json!(shots(complex_distance));
    let terrain_keys = [("TerrainAdjustTolerance", adjust.tolerance), ("TerrainAdjustMaxClimbRate", adjust.max_climb_rate), ("TerrainAdjustMaxDescentRate", adjust.max_descent_rate), (TERRAIN_FLIGHT_SPEED, adjust.flight_speed)];
    let object = rebuilt.as_object_mut().expect("a transect is an object");
    match follows_terrain {
        true => terrain_keys.iter().for_each(|(key, value)| {
            object.insert((*key).to_string(), json!(value));
        }),
        false => terrain_keys.iter().for_each(|(key, _)| {
            object.remove(*key);
        }),
    }
    rebuilt
}

pub fn waiting_for_terrain(item: &Value) -> bool {
    let transect = item.get("TransectStyleComplexItem").unwrap_or(&Value::Null);
    let empty = |key: &str| transect.get(key).and_then(Value::as_array).is_none_or(Vec::is_empty);
    calc_of(item).get("DistanceMode").and_then(Value::as_i64) == Some(crate::altitudemodes::CALC_ABOVE_TERRAIN) && empty("Items") && !empty("VisualTransectPoints")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_refly_and_turnaround_switches_follow_surveyitemeditor() {
        let metres = crate::read::Unit { name: "m".to_string(), factor: 1.0 };
        let units = Units { vertical: &metres, horizontal: &metres };
        let survey = |mode: i64, hover: bool| json!({ "complexItemType": "survey", "TransectStyleComplexItem": { "HoverAndCapture": hover, "CameraCalc": { "DistanceMode": mode, "CameraName": MANUAL_CAMERA } } });
        let enabled = |s: &Value, hover_allowed: bool, suffix: &str| fields(s, "p", if hover_allowed { crate::cmdinfo::VehicleClass::MultiRotor } else { crate::cmdinfo::VehicleClass::FixedWing }, &units).into_iter().find(|c| c["pathSuffix"] == suffix).map(|c| c["enabled"] != false);
        assert_eq!(enabled(&survey(1, false), false, "hoverAndCapture"), None, "a fixed wing never sees hover-and-capture");
        assert_eq!(enabled(&survey(1, false), true, "hoverAndCapture"), Some(true));
        assert_eq!(enabled(&survey(3, false), true, "hoverAndCapture"), Some(false), "only with a relative or absolute altitude");
        assert_eq!(enabled(&survey(3, false), true, "refly90Degrees"), Some(false), "no refly while following terrain");
        assert_eq!(enabled(&survey(1, true), true, "cameraTriggerInTurnAround"), Some(false), "no turnaround images while hovering");
        assert_eq!(enabled(&survey(1, true), false, "cameraTriggerInTurnAround"), Some(true), "the turnaround switch is not held back by a hover switch the vehicle never shows, as QGC gates it on hoverAndCaptureAllowed");
    }

    #[test]
    fn the_terrain_frame_is_written_and_gates_the_terrain_adjust_rows() {
        let metres = crate::read::Unit { name: "m".to_string(), factor: 1.0 };
        let units = Units { vertical: &metres, horizontal: &metres };
        let survey = json!({ "complexItemType": "survey", "CameraCalc": { "DistanceMode": 1, "CameraName": MANUAL_CAMERA }, "TransectStyleComplexItem": {} });
        let rows = |s: &Value| fields(s, "p", crate::cmdinfo::VehicleClass::MultiRotor, &units).into_iter().filter(|c| c["name"].as_str().is_some_and(|n| n.starts_with("TerrainAdjust"))).map(|c| c["enabled"].clone()).collect::<Vec<_>>();
        assert_eq!(rows(&survey).len(), 3, "the three terrain-adjust rows have to be found at all");
        assert!(rows(&survey).iter().all(|enabled| *enabled == json!(false)), "{:?}", rows(&survey));
        let following = set(&survey, "cameraCalc.distanceMode", &json!(3), &units).expect("the frame is a camera calc key the core writes");
        assert_eq!(calc_of(&following)["DistanceMode"], 3);
        assert!(rows(&following).iter().all(|enabled| *enabled != json!(false)));
        let both = json!({ "complexItemType": "survey", "CameraCalc": { "DistanceMode": 1, "CameraName": MANUAL_CAMERA }, "TransectStyleComplexItem": { "Refly90Degrees": true, "HoverAndCapture": true, "CameraCalc": { "DistanceMode": 1, "CameraName": MANUAL_CAMERA } } });
        let terrain = set(&both, "cameraCalc.distanceMode", &json!(3), &units).unwrap();
        assert_eq!((terrain["TransectStyleComplexItem"]["Refly90Degrees"].clone(), terrain["TransectStyleComplexItem"]["HoverAndCapture"].clone()), (json!(false), json!(false)), "TransectStyleComplexItem::_distanceModeChanged clears both, so greyed-out switches cannot still shape the flight");
    }

    #[test]
    fn a_structure_scan_derives_its_layers_and_a_manual_camera_sets_its_own_spacing() {
        let fixture: Value = serde_json::from_str(include_str!("../tests/fixtures/structure-inserted-by-qt.json")).unwrap();
        let rows = fields(&fixture["structure"], "p", crate::cmdinfo::VehicleClass::MultiRotor, &metric());
        let labels: Vec<Value> = rows.iter().map(|f| f["label"].clone()).collect();
        assert_eq!(labels, [json!("Start from"), json!("Structure height"), json!("Scan bottom altitude"), json!("Entrance and exit altitude"), json!("Gimbal pitch")], "StructureScanEditor SCAN card; Layers is only a STATISTICS row");
        assert_eq!(rows[0]["options"], json!([{ "label": "Bottom", "raw": "false" }, { "label": "Top", "raw": "true" }]), "Start from is a Bottom/Top segmented control over startFromTop");
        assert!(set(&fixture["structure"], "layers", &json!(5), &metric()).is_none());
        let survey = json!({ "complexItemType": "survey", "TransectStyleComplexItem": { "CameraCalc": { "CameraName": MANUAL_CAMERA, "AdjustedFootprintSide": 25.0, "AdjustedFootprintFrontal": 25.0 } } });
        let suffixes: Vec<String> = camera(&survey, "p", &metric(), true)["facts"].as_array().unwrap().iter().filter_map(|f| f["pathSuffix"].as_str().map(str::to_string)).collect();
        assert!(suffixes.contains(&"cameraCalc.adjustedFootprintFrontal".to_string()) && suffixes.contains(&"cameraCalc.adjustedFootprintSide".to_string()), "CameraCalcGrid edits trigger distance and spacing for a manual camera: {suffixes:?}");
        let wider = set(&survey, "cameraCalc.adjustedFootprintSide", &json!(40.0), &metric()).expect("a manual camera's spacing is writable");
        assert_eq!(calc_of(&wider)["AdjustedFootprintSide"], 40.0);
        let labels: Vec<Value> = camera(&survey, "p", &metric(), true)["facts"].as_array().unwrap().iter().filter(|f| f["name"].as_str().is_some_and(|n| n.starts_with("AdjustedFootprint"))).map(|f| f["label"].clone()).collect();
        assert_eq!(labels, [json!("Trigger distance"), json!("Spacing")], "TransectStyleComplexItemEditor names them for CameraCalcGrid");
    }

    static METRES: std::sync::LazyLock<crate::read::Unit> = std::sync::LazyLock::new(|| crate::read::Unit { name: "m".to_string(), factor: 1.0 });

    fn metric() -> Units<'static> {
        Units { vertical: &METRES, horizontal: &METRES }
    }

    #[test]
    fn a_cooked_write_is_stored_raw_and_a_gimbal_angle_reads_upside_down_as_qt_shows_it() {
        let fixture: Value = serde_json::from_str(include_str!("../tests/fixtures/structure-inserted-by-qt.json")).unwrap();
        let feet = crate::read::Unit { name: "ft".to_string(), factor: 3.28084 };
        let imperial = Units { vertical: &feet, horizontal: &feet };
        let higher = set(&fixture["structure"], "entranceAlt", &json!(328.084), &imperial).unwrap();
        assert!((higher["EntranceAltitude"].as_f64().unwrap() - 100.0).abs() < 1e-9, "328.084 ft is stored as 100 m");
        let pitched = set(&fixture["structure"], "gimbalPitch", &json!(45.0), &metric()).unwrap();
        assert_eq!(pitched["GimbalPitch"], -45.0);
        let mut real = pitched["CameraCalc"].clone();
        real["CameraName"] = json!("Sony ILCE-QX1");
        let camera = with_calc(&pitched, real);
        assert_eq!(camera["GimbalPitch"], 0, "StructureScanComplexItem::_updateGimbalPitch zeroes the pitch for a real camera");
        assert!(fields(&camera, "i", crate::cmdinfo::VehicleClass::MultiRotor, &metric()).iter().all(|f| f["pathSuffix"] != "gimbalPitch"), "and StructureScanEditor shows it only for a manual camera");
        let shown = fields(&pitched, "i", crate::cmdinfo::VehicleClass::MultiRotor, &metric()).into_iter().find(|f| f["pathSuffix"] == "gimbalPitch").unwrap();
        assert_eq!((shown["value"].as_f64(), shown["minimum"].as_f64(), shown["maximum"].as_f64(), shown["units"].as_str()), (Some(45.0), Some(0.0), Some(90.0), Some("deg")));
    }

    #[test]
    fn rotating_the_entry_walks_the_corners_and_a_single_pass_corridor_jumps_ends() {
        let corridor: Value = serde_json::from_str(include_str!("../tests/fixtures/corridor-inserted-by-qt.json")).unwrap();
        let survey: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-inserted-by-qt.json")).unwrap();
        let item = survey.get("survey").unwrap_or(&survey).clone();
        let mut last = item.clone();
        last["entryLocation"] = json!(3);
        assert_eq!(entry_point("survey", &rotated_entry("survey", &last).unwrap()), 0);
        assert_eq!(entry_point("survey", &rotated_entry("survey", &item).unwrap()), entry_point("survey", &item) + 1);
        let mut narrow = corridor["corridor"].clone();
        narrow["CorridorWidth"] = json!(0.0);
        narrow["EntryPoint"] = json!(0);
        assert_eq!(entry_point("CorridorScan", &rotated_entry("CorridorScan", &narrow).unwrap()), 2, "one transect has no other side");
        assert_eq!(entry_point_name("CorridorScan", 3), Some("end, other side"));
        assert_eq!(rotated_entry("StructureScan", &item), None);
    }

    #[test]
    fn a_cleared_shape_regenerates_without_flight_lines_until_it_is_refilled() {
        let corridor: Value = serde_json::from_str(include_str!("../tests/fixtures/corridor-inserted-by-qt.json")).unwrap();
        let cleared = |item: &Value, key: &str| {
            let mut emptied = item.clone();
            emptied[key] = json!([]);
            regenerate_item(&emptied)
        };
        assert_eq!(cleared(&corridor["corridor"], "polyline")["polyline"], json!([]));
        let survey: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-inserted-by-qt.json")).unwrap();
        let item = survey.get("survey").unwrap_or(&survey);
        assert_eq!(cleared(item, "polygon")["polygon"], json!([]));
    }

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
            previous_mode: None,
        });
        built["TransectStyleComplexItem"]["Items"].as_array_mut().unwrap().iter_mut().for_each(|item| item["doJumpId"] = json!(item["doJumpId"].as_i64().unwrap() + 1));
        assert_eq!(by_value(&built), by_value(&fixture["corridor"]));
    }

    #[test]
    fn a_wider_corridor_flies_more_passes_and_lists_its_width_among_its_fields() {
        let fixture: Value = serde_json::from_str(include_str!("../tests/fixtures/corridor-inserted-by-qt.json")).unwrap();
        let corridor = &fixture["corridor"];
        let passes = |c: &Value| c["TransectStyleComplexItem"]["Items"].as_array().unwrap().iter().filter(|i| i["command"] == 16).count();
        let wider = set(corridor, "corridorWidth", &json!(80.0), &metric()).unwrap();
        assert_eq!(wider["CorridorWidth"], 80.0);
        assert!(passes(&wider) > passes(corridor), "{} passes at 80 m against {} at 50 m", passes(&wider), passes(corridor));
        let metres = crate::read::Unit { name: "m".to_string(), factor: 1.0 };
        let listed: Vec<String> = fields(corridor, "i", crate::cmdinfo::VehicleClass::MultiRotor, &Units { vertical: &metres, horizontal: &metres }).iter().filter_map(|f| f["pathSuffix"].as_str().map(str::to_string)).collect();
        assert!(listed.contains(&"corridorWidth".to_string()) && !listed.contains(&"gridAngle".to_string()), "{listed:?}");
        assert!(!listed.iter().any(|s| s == "hoverAndCapture" || s == "refly90Degrees"), "CorridorScanEditor shows width, turnaround and images in turnarounds only: {listed:?}");
    }

    #[test]
    fn a_terrain_following_scan_waits_for_its_heights_before_save_or_upload() {
        let scan = |mode: i64, items: Value| json!({ "complexItemType": "survey", "TransectStyleComplexItem": { "CameraCalc": { "DistanceMode": mode }, "Items": items, "VisualTransectPoints": [[47.0, 8.0], [47.0, 8.01]] } });
        assert!(waiting_for_terrain(&scan(crate::altitudemodes::CALC_ABOVE_TERRAIN, json!([]))));
        assert!(!waiting_for_terrain(&scan(crate::altitudemodes::CALC_ABOVE_TERRAIN, json!([{ "command": 16 }]))));
        assert!(!waiting_for_terrain(&scan(crate::altitudemodes::RELATIVE, json!([]))), "only calc-above-terrain needs heights to build");
        let adjust = terrain_adjust(&json!({ "TerrainAdjustMaxClimbRate": 3.0 }), 50.0);
        assert_eq!((adjust.tolerance, adjust.max_climb_rate, adjust.flight_speed), (10.0, 3.0, 5.0), "QGC's defaults: 10 m tolerance, 5 m/s until the flight status arrives");
    }

    #[test]
    fn a_manual_camera_remembers_the_trigger_interval_it_resets_though_it_saves_none() {
        let stored = |key: &str| match key {
            "Survey/MinTriggerInterval" => Some("1".to_string()),
            "Survey/DistanceToSurface" => Some("50".to_string()),
            _ => None,
        };
        let manual = json!({ "complexItemType": "survey", "TransectStyleComplexItem": { "CameraCalc": { "CameraName": MANUAL_CAMERA, "DistanceToSurface": 75 } } });
        let changed: Vec<String> = changed_remembered(&manual, true, &stored).into_iter().map(|(key, value)| format!("{key}={value}")).collect();
        assert!(changed.contains(&"Survey/MinTriggerInterval=0".to_string()) && changed.contains(&"Survey/DistanceToSurface=75".to_string()), "{changed:?}");
        assert!(!changed.iter().any(|c| c.starts_with("Survey/FixedOrientation")), "false is already the default");
    }

    #[test]
    fn a_new_structure_scan_starts_from_the_remembered_settings_as_qt_builds_it() {
        let fixture: Value = serde_json::from_str(include_str!("../tests/fixtures/structure-inserted-by-qt.json")).unwrap();
        let remembered = |key: &str| key.strip_prefix("StructureScan/").and_then(|name| fixture["remembered"][name].as_str()).map(str::to_string);
        let built = fresh_structure(&Fresh {
            center: (fixture["center"][0].as_f64().unwrap(), fixture["center"][1].as_f64().unwrap()),
            remembered: &remembered,
            multirotor: true,
            alternates: false,
            default_altitude: fixture["defaultAltitude"].as_f64().unwrap(),
            distance_mode: crate::altitudemodes::RELATIVE,
            previous_mode: None,
        });
        assert_eq!(by_value(&built), by_value(&fixture["structure"]));
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
            previous_mode: None,
        });
        built["TransectStyleComplexItem"]["Items"].as_array_mut().unwrap().iter_mut().for_each(|item| item["doJumpId"] = json!(item["doJumpId"].as_i64().unwrap() + 1));
        assert_eq!(by_value(&built), by_value(&fixture["survey"]));
    }

    #[test]
    fn a_new_survey_after_a_terrain_item_drops_the_remembered_refly_and_hover() {
        let remembered = |key: &str| matches!(key, "Survey/Refly90Degrees" | "Survey/HoverAndCapture").then(|| "true".to_string());
        let built = fresh(&Fresh { center: (47.0, 8.0), remembered: &remembered, multirotor: true, alternates: false, default_altitude: 50.0, distance_mode: crate::altitudemodes::RELATIVE, previous_mode: Some(crate::altitudemodes::FRAME_CALC_ABOVE_TERRAIN) });
        let transect = &built["TransectStyleComplexItem"];
        assert_eq!((transect["Refly90Degrees"].clone(), transect["HoverAndCapture"].clone()), (json!(false), json!(false)), "applyPreviousAltitudeFrame sets the mode, and _distanceModeChanged clears both");
    }

    #[test]
    fn a_named_camera_shows_its_spacing_without_letting_it_be_typed() {
        let plan: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-upload.plan")).unwrap();
        let sony = set(&plan["mission"]["items"][0], "cameraCalc.cameraBrand", &json!("Sony"), &metric()).unwrap();
        let spacing: Vec<(Value, Value)> = camera(&sony, "p", &metric(), true)["facts"].as_array().unwrap().iter().filter(|f| f["name"].as_str().is_some_and(|n| n.starts_with("AdjustedFootprint"))).map(|f| (f["shortLabel"].clone(), f["readOnly"].clone())).collect();
        assert_eq!(spacing, [(json!("Trigger distance"), json!(true)), (json!("Spacing"), json!(true))], "CameraCalcGrid shows them as plain rows for any camera but the manual one");
        assert!(set(&sony, "cameraCalc.adjustedFootprintSide", &json!(40.0), &metric()).is_none(), "a write the camera calc would overwrite is refused rather than reported as taken");
    }

    #[test]
    fn a_named_camera_shows_its_sensor_read_only_and_an_orientation_choice_unless_fixed() {
        let plan: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-upload.plan")).unwrap();
        let sony = set(&plan["mission"]["items"][0], "cameraCalc.cameraBrand", &json!("Sony"), &metric()).unwrap();
        let rows = |survey: &Value| -> Vec<(String, bool)> {
            camera(survey, "p", &metric(), true)["facts"].as_array().unwrap().iter().filter(|f| f["pathSuffix"].as_str().is_some_and(|s| OPTICS.iter().any(|(_, o)| s == format!("cameraCalc.{o}")))).map(|f| (f["pathSuffix"].as_str().unwrap().to_string(), f["readOnly"].as_bool().unwrap())).collect()
        };
        let sensor = ["sensorWidth", "sensorHeight", "imageWidth", "imageHeight", "focalLength"].map(|s| (format!("cameraCalc.{s}"), true));
        assert_eq!(rows(&sony), sensor.iter().cloned().chain([("cameraCalc.landscape".to_string(), false)]).collect::<Vec<_>>(), "CameraCalcCamera: SENSOR card disabled unless custom, Orientation shown unless fixedOrientation");
        let portrait = set(&sony, "cameraCalc.landscape", &json!(false), &metric()).unwrap();
        assert_eq!(calc_of(&portrait)["Landscape"], json!(false));
        assert!(set(&sony, "cameraCalc.sensorWidth", &json!(20.0), &metric()).is_none(), "a catalogue camera's sensor is not typed in");
        let fixed = set(&sony, "cameraCalc.cameraModel", &json!("a7R II Zeiss 21mm f/2.8"), &metric()).unwrap();
        assert_eq!(rows(&fixed), sensor.to_vec());
    }

    #[test]
    fn a_structure_scan_relayers_when_its_layer_height_changes_and_on_load() {
        let fixture: Value = serde_json::from_str(include_str!("../tests/fixtures/structure-inserted-by-qt.json")).unwrap();
        let mut manual = fixture["structure"].clone();
        manual["CameraCalc"]["CameraName"] = json!(MANUAL_CAMERA);
        manual["StructureHeight"] = json!(100.0);
        manual["ScanBottomAlt"] = json!(50.0);
        let thinner = set(&manual, "cameraCalc.adjustedFootprintFrontal", &json!(10.0), &metric()).unwrap();
        assert_eq!(thinner["Layers"], 5, "StructureScanComplexItem::_recalcLayerInfo: ceil((100 - 50) / 10)");
        let labels: Vec<Value> = camera(&manual, "p", &metric(), true)["facts"].as_array().unwrap().iter().filter(|f| f["name"].as_str().is_some_and(|n| n.starts_with("AdjustedFootprint"))).map(|f| f["shortLabel"].clone()).collect();
        assert_eq!(labels, [json!("Layer height"), json!("Trigger distance")], "StructureScanEditor names them for CameraCalcGrid");
        let distance = camera(&manual, "p", &metric(), true)["facts"].as_array().unwrap().iter().find(|f| f["name"] == "DistanceToSurface").map(|f| f["shortLabel"].clone());
        assert_eq!(distance, Some(json!("Scan distance")), "StructureScanEditor calls the distance Scan distance");
        let mut stale = thinner.clone();
        stale["Layers"] = json!(9);
        assert_eq!(crate::structurescan::saved_plan(&stale).layers, 5, "a hand-set count saved before layers were derived is not flown");
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
        let higher = set(&survey, "cameraCalc.distanceToSurface", &json!(100.0), &metric()).unwrap();
        let calc = &higher["TransectStyleComplexItem"]["CameraCalc"];
        assert!((calc["AdjustedFootprintSide"].as_f64().unwrap() - 2.0 * survey["TransectStyleComplexItem"]["CameraCalc"]["AdjustedFootprintSide"].as_f64().unwrap()).abs() < 1e-9, "doubling the height doubles the footprint");
        assert!(higher["TransectStyleComplexItem"]["Items"].as_array().unwrap().len() < survey["TransectStyleComplexItem"]["Items"].as_array().unwrap().len(), "wider spacing means fewer transects");
        let hovering = set(&survey, "hoverAndCapture", &json!(true), &metric()).unwrap();
        assert_eq!(hovering["TransectStyleComplexItem"]["CameraTriggerInTurnAround"], false, "TransectStyleComplexItem turns images-in-turnaround off when hover and capture comes on");
        let items = hovering["TransectStyleComplexItem"]["Items"].as_array().unwrap();
        assert!(items.iter().any(|i| i["command"] == 2000) && items.iter().all(|i| i["command"] != 206), "a hovering survey photographs at each stop and never distance-triggers");
        let turned = set(&survey, "gridAngle", &json!(0.0), &metric()).unwrap();
        assert_eq!(turned["angle"], 0.0);
        assert!(set(&survey, "noSuchField", &json!(1), &metric()).is_none());
    }

    #[test]
    fn a_camera_chosen_after_manual_writes_every_key_camera_calc_load_requires() {
        let manual = json!({ "version": 2, "CameraName": MANUAL_CAMERA, "AdjustedFootprintSide": 25.0, "AdjustedFootprintFrontal": 25.0, "DistanceToSurface": 50.0, "DistanceMode": 1 });
        let custom = named_camera(&manual, CUSTOM_CAMERA);
        let missing: Vec<&str> = CALC_FACTS.iter().copied().chain(SPECS.iter().map(|(key, _)| *key)).filter(|key| custom.get(*key).is_none_or(Value::is_null)).collect();
        assert!(missing.is_empty(), "QGC writes every fact, so a plan saved after picking a camera must reload: missing {missing:?}");
    }

    #[test]
    fn choosing_a_known_camera_takes_its_optics_from_the_database() {
        let plan: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-upload.plan")).unwrap();
        let survey = plan["mission"]["items"][0].clone();
        let sony = set(&survey, "cameraCalc.cameraBrand", &json!("Sony"), &metric()).unwrap();
        let calc = &sony["TransectStyleComplexItem"]["CameraCalc"];
        let first = cameras().into_iter().find(|c| c["brand"] == "Sony").unwrap();
        assert_eq!(calc["CameraName"], first["canonicalName"]);
        assert_eq!(calc["SensorWidth"], first["sensorWidth"]);
        let metres = crate::read::Unit { name: "m".to_string(), factor: 1.0 };
        let described = camera(&sony, "p", &Units { vertical: &metres, horizontal: &metres }, true);
        assert_eq!((described["brand"].as_str(), described["model"].as_str()), (Some("Sony"), first["model"].as_str()));
        let manual = set(&sony, "cameraCalc.cameraBrand", &json!(MANUAL_CAMERA), &metric()).unwrap();
        assert_eq!(manual["TransectStyleComplexItem"]["CameraCalc"]["ValueSetIsDistance"], true);
        assert!(set(&survey, "cameraCalc.cameraModel", &json!("anything"), &metric()).is_none(), "a custom camera has no models to pick from");
    }

    #[test]
    fn survey_editor_rows_follow_survey_item_editor_visibility() {
        let plan: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-upload.plan")).unwrap();
        let survey = plan["mission"]["items"][0].clone();
        let names = |class| fields(&survey, "i", class, &metric()).iter().filter_map(|f| f["name"].as_str().map(str::to_string)).collect::<Vec<_>>();
        use crate::cmdinfo::VehicleClass::{FixedWing, MultiRotor, Vtol};
        assert!([FixedWing, MultiRotor, Vtol].iter().all(|class| !names(*class).contains(&"SplitConcavePolygons".to_string())), "SurveyItemEditor has no split concave polygons row");
        assert!(names(FixedWing).contains(&"FlyAlternateTransects".to_string()) && names(Vtol).contains(&"FlyAlternateTransects".to_string()));
        assert!(!names(MultiRotor).contains(&"FlyAlternateTransects".to_string()), "alternate transects only for fixedWing || vtol");
    }

    #[test]
    fn a_survey_editor_offers_the_fields_and_camera_qt_offers() {
        let plan: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-upload.plan")).unwrap();
        let survey = plan["mission"]["items"][0].clone();
        let qt: Value = serde_json::from_str(include_str!("../tests/fixtures/itemfacts-survey-by-qt.json")).unwrap();
        let item = "plan.missionController.visualItems.1";
        let metres = crate::read::Unit { name: "m".to_string(), factor: 1.0 };
        let units = Units { vertical: &metres, horizontal: &metres };
        let mine = json!({ "fields": fields(&survey, item, crate::cmdinfo::VehicleClass::MultiRotor, &units), "camera": camera(&survey, item, &units, true) });
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
        assert_eq!(mine["camera"]["valueSetIsDistance"], json!(survey["TransectStyleComplexItem"]["CameraCalc"]["ValueSetIsDistance"]), "Set by follows the plan's ValueSetIsDistance");
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
