use serde_json::{Value, json};

use crate::read::{flag, object, refused};
use crate::router::Backend;

// The macOS mission item editor read four raw groups per selected item - the item itself, its
// textFieldFacts and comboboxFacts lists, and cameraCalc - and chose among them in the head: the
// lists when the item has any, else a complex item's own facts, plus the camera block for a
// complex item. Each fact is served here in view.control's shape, with the path the head writes.
pub const DEPS: &[&str] = &[
    "plan.missionController.visualItems.count",
    "plan.controllerVehicle.vtol",
    "plan.controllerVehicle.apmFirmware",
    "plan.missionController@visualItemsReset",
    "plan.dirty",
    "settings.unitsSettings.horizontalDistanceUnits",
    "settings.unitsSettings.verticalDistanceUnits",
    "vehicle.homePosition",
    crate::coreplan::CHANGED,
];

const ITEM_ROOT: &str = "plan.missionController.visualItems";
const LISTS: [&str; 3] = ["textFieldFacts", "comboboxFacts", "nanFacts"];
// Mission Settings edits the launch altitude through its own control, so it is not a field here.
const LAUNCH_ALTITUDE: &str = "plannedHomePositionAltitude";
const SURVEY_PROPERTIES: [&str; 4] = ["distanceToSurface", "imageDensity", "frontalOverlap", "sideOverlap"];
const OPTICS_PROPERTIES: [&str; 7] = ["sensorWidth", "sensorHeight", "imageWidth", "imageHeight", "focalLength", "landscape", "minTriggerInterval"];

fn field(fact: &Value, item: &str, suffix: &str, group: &str) -> Value {
    let mut control = crate::control::decode(fact, &format!("{item}.{suffix}"));
    if let Value::Object(map) = &mut control {
        map.insert("pathSuffix".to_string(), json!(suffix));
        map.insert("group".to_string(), json!(group));
    }
    control
}

fn named(fact: &Value) -> bool {
    fact.get("name").and_then(Value::as_str).is_some_and(|name| !name.is_empty())
}

fn listed(backend: &dyn Backend, item: &str) -> Vec<Value> {
    LISTS
        .iter()
        .flat_map(|list| {
            let read = object(&backend.get(&format!("{item}.{list}")));
            let elements = read.get("elements").and_then(Value::as_array).cloned().unwrap_or_default();
            elements
                .into_iter()
                .enumerate()
                .filter(|(_, fact)| named(fact))
                .map(move |(at, fact)| match (*list == "nanFacts", field(&fact, item, &format!("{list}.{at}"), "Settings")) {
                    (true, Value::Object(map)) => Value::Object(map.into_iter().chain([("optional".to_string(), json!(true))]).collect()),
                    (_, control) => control,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

pub fn area_help(kind: &str, complete: bool) -> Option<&'static str> {
    (!complete).then_some(match kind {
        "CorridorScan" => "Use the Polyline Tools to create the polyline which defines the corridor.",
        "StructureScan" => "Draw the structure outline with the Polygon Tools, at the top of the map.",
        _ => "Use the Polygon Tools to create the polygon which outlines your survey area.",
    })
}

fn qt_area_help(backend: &dyn Backend, item: &str, read: &Value) -> Option<&'static str> {
    let (kind, shape) = [("SurveyComplexItem", "survey", "surveyAreaPolygon"), ("CorridorScanComplexItem", "CorridorScan", "corridorPolyline"), ("StructureScanComplexItem", "StructureScan", "structurePolygon")]
        .into_iter()
        .find(|(class, _, _)| read.get("class").and_then(Value::as_str) == Some(*class))
        .map(|(_, kind, shape)| (kind, shape))?;
    area_help(kind, flag(&object(&backend.get(&format!("{item}.{shape}"))), "isValid"))
}

fn qt_entry_point(item: &str, read: &Value) -> Option<Value> {
    let kind = match read.get("class").and_then(Value::as_str)? {
        "SurveyComplexItem" => "survey",
        "CorridorScanComplexItem" => "CorridorScan",
        "StructureScanComplexItem" => return Some(json!({ "label": "Entry vertex", "value": (read.get("entryVertex")?.as_i64()? + 1).to_string(), "path": format!("{item}.rotateEntryPoint") })),
        _ => return None,
    };
    let name = crate::surveydoc::entry_point_name(kind, read.get("entryPoint")?.as_i64()?)?;
    Some(json!({ "label": "Start from", "value": name, "path": format!("{item}.rotateEntryPoint") }))
}

fn owned(read: &Value, item: &str) -> Vec<Value> {
    read.get("facts")
        .and_then(Value::as_array)
        .map(|facts| {
            facts
                .iter()
                .filter(|fact| named(fact))
                .filter_map(|fact| {
                    let property = fact.get("property").and_then(Value::as_str).filter(|p| !p.is_empty() && *p != LAUNCH_ALTITUDE)?;
                    Some(field(fact, item, property, "Settings"))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn strings(read: &Value, key: &str) -> Vec<String> {
    read.get(key).and_then(Value::as_array).map(|list| list.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default()
}

fn camera(backend: &dyn Backend, item: &str) -> Value {
    let read = object(&backend.get(&format!("{item}.cameraCalc")));
    if read.get("kind").and_then(Value::as_str) != Some("object") {
        return Value::Null;
    }
    let text = |key: &str| read.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let custom = flag(&read, "isCustomCamera");
    let wanted: Vec<&str> = match custom {
        true => OPTICS_PROPERTIES.iter().chain(SURVEY_PROPERTIES.iter()).copied().collect(),
        false => SURVEY_PROPERTIES.to_vec(),
    };
    let facts = read.get("facts").and_then(Value::as_array).cloned().unwrap_or_default();
    let shown: Vec<Value> = wanted
        .iter()
        .filter_map(|property| facts.iter().find(|fact| fact.get("property").and_then(Value::as_str) == Some(property)))
        .filter(|fact| named(fact))
        .map(|fact| {
            let property = fact.get("property").and_then(Value::as_str).unwrap_or_default();
            field(fact, item, &format!("cameraCalc.{property}"), "Camera")
        })
        .collect();
    json!({
        "brand": text("cameraBrand"),
        "model": text("cameraModel"),
        "brands": strings(&read, "cameraBrandList"),
        "models": strings(&read, "cameraModelList"),
        "manualName": text("xlatManualCameraName"),
        "customName": text("xlatCustomCameraName"),
        "custom": custom,
        "distanceMode": read.get("distanceMode").cloned().unwrap_or(Value::Null),
        "valueSetIsDistance": facts.iter().find(|fact| fact.get("property").and_then(Value::as_str) == Some("valueSetIsDistance")).and_then(|fact| fact.get("value")).map_or(true, |value| value.as_bool().unwrap_or_else(|| value.as_f64().is_some_and(|v| v != 0.0))),
        "valueSetIsDistancePath": format!("{item}.cameraCalc.valueSetIsDistance"),
        "brandPath": format!("{item}.cameraCalc.cameraBrand"),
        "modelPath": format!("{item}.cameraCalc.cameraModel"),
        "facts": shown,
    })
}

pub const LAND_ALTITUDE_HINT: &str = "Altitude is the approximate ground altitude. Normally 0 when landing back at the launch location.";

pub fn altitude_hint(land: bool, mode: Option<i64>, amsl_sent: Option<String>) -> Option<String> {
    match (land, mode) {
        (true, _) => Some(LAND_ALTITUDE_HINT.to_string()),
        (false, Some(crate::altitudemodes::CALC_ABOVE_TERRAIN)) => amsl_sent.map(|sent| format!("Actual AMSL alt sent: {sent}")),
        _ => None,
    }
}

pub fn without_hidden_mission_speed(facts: Value, index: usize, vtol: bool, apm: bool) -> Value {
    match (facts, index == 0 && (vtol || apm)) {
        (Value::Object(mut map), true) => {
            map.insert("speedSection".to_string(), Value::Null);
            Value::Object(map)
        }
        (other, _) => other,
    }
}

pub fn command_info(read: &Value) -> [(String, Value); 3] {
    let text = |key: &str| read.get(key).and_then(Value::as_str).filter(|t| !t.is_empty()).map_or(Value::Null, |t| json!(t));
    [
        ("takeoff".to_string(), json!(flag(read, "isTakeoffItem"))),
        ("category".to_string(), text("category")),
        ("commandDescription".to_string(), text("commandDescription")),
    ]
}

pub fn item_facts_view(backend: &dyn Backend, args: &[String]) -> Value {
    let Some(index) = args.first().and_then(|a| a.parse::<usize>().ok()) else {
        return refused("view.itemFacts needs the index of the item in the plan, as view.itemFacts(3)");
    };
    if crate::coreplan::enabled() {
        return crate::coreplan::item_facts(backend, index);
    }
    let item = format!("{ITEM_ROOT}.{index}");
    let read = object(&backend.get(&item));
    let available = read.get("kind").and_then(Value::as_str) == Some("object");
    let simple = flag(&read, "isSimpleItem");
    let lists = match available {
        true => listed(backend, &item),
        false => Vec::new(),
    };
    let fields = match (lists.is_empty(), available && !simple) {
        (true, true) => owned(&read, &item),
        _ => lists,
    };
    let info = command_info(&read);
    let built = json!({
        "kind": "object",
        "class": "ItemFacts",
        "available": available,
        "index": index,
        "simple": simple,
        "fields": fields,
        "areaHelp": (available && !simple).then(|| qt_area_help(backend, &item, &read)).flatten(),
        "entryPoint": qt_entry_point(&item, &read),
        "landing": matches!(read.get("class").and_then(Value::as_str), Some("FixedWingLandingComplexItem" | "VTOLLandingComplexItem")),
        "altitudesAreRelative": matches!(read.get("class").and_then(Value::as_str), Some("FixedWingLandingComplexItem" | "VTOLLandingComplexItem")).then(|| read.get("altitudesAreRelative").and_then(Value::as_bool)).flatten(),
        "camera": match available && !simple {
            true => camera(backend, &item),
            false => Value::Null,
        },
        // Served by index here rather than only for the plan's current item in view.missionItems,
        // so an editor showing item N reads item N's speed and altitude mode.
        "speedSection": match available {
            true => crate::missionitems::speed_section(backend, index as i64),
            false => Value::Null,
        },
        "altitudeMode": crate::missionitems::frame(&read).map(|mode| mode as i64),
        "altitudeHint": match simple && flag(&read, "specifiesAltitude") {
            true => altitude_hint(
                flag(&read, "isLandCommand"),
                read.get("altitudeFrame").and_then(Value::as_i64),
                crate::read::fact_property(&read, "amslAltAboveTerrain").and_then(|fact| Some(format!("{} {}", fact.get("valueString")?.as_str()?, fact.get("units").and_then(Value::as_str).unwrap_or_default()).trim().to_string())),
            ),
            false => None,
        },
        "launchAltitude": match index == 0 && !crate::read::flag(&object(&backend.get("vehicle.homePosition")), "valid") {
            true => crate::read::fact_property(&read, LAUNCH_ALTITUDE).map(|fact| field(fact, &item, LAUNCH_ALTITUDE, "Settings")),
            false => None,
        },
        "rawEdit": simple && flag(&read, "rawEdit"),
        "friendlyEditAllowed": simple && flag(&read, "friendlyEditAllowed"),
        "previousCoordinate": match available {
            true => qt_previous_coordinate(backend, index).map_or(Value::Null, |(latitude, longitude)| json!({ "latitude": latitude, "longitude": longitude })),
            false => Value::Null,
        },
    });
    let controller = object(&backend.get_fields("plan.controllerVehicle", "vtol,apmFirmware"));
    match built {
        Value::Object(map) => without_hidden_mission_speed(Value::Object(map.into_iter().chain(info).collect()), index, flag(&controller, "vtol"), flag(&controller, "apmFirmware")),
        other => other,
    }
}

fn qt_previous_coordinate(backend: &dyn Backend, index: usize) -> Option<(f64, f64)> {
    (1..index).rev().find_map(|before| {
        let read = object(&backend.get_fields(&format!("{ITEM_ROOT}.{before}"), "isSimpleItem,specifiesCoordinate,isStandaloneCoordinate,coordinate"));
        let placed = flag(&read, "isSimpleItem") && flag(&read, "specifiesCoordinate") && !flag(&read, "isStandaloneCoordinate");
        let at = read.get("coordinate")?;
        placed.then(|| at.get("latitude")?.as_f64().zip(at.get("longitude")?.as_f64())).flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mission_settings_flight_speed_is_dropped_where_mission_settings_editor_hides_it() {
        let facts = || json!({ "speedSection": { "available": true } });
        assert_eq!(without_hidden_mission_speed(facts(), 0, false, false)["speedSection"]["available"], true);
        assert_eq!(without_hidden_mission_speed(facts(), 0, true, false)["speedSection"], Value::Null, "_showFlightSpeed is false for a VTOL");
        assert_eq!(without_hidden_mission_speed(facts(), 0, false, true)["speedSection"], Value::Null, "and for ArduPilot");
        assert_eq!(without_hidden_mission_speed(facts(), 3, true, true)["speedSection"]["available"], true, "a waypoint's own speed section is not the mission settings one");
    }

    #[test]
    fn command_info_carries_what_the_command_picker_and_editor_read() {
        let info = command_info(&json!({ "isTakeoffItem": true, "category": "Basic", "commandDescription": "Take off from the ground" }));
        assert_eq!(info.map(|(k, v)| (k, v)), [("takeoff".to_string(), json!(true)), ("category".to_string(), json!("Basic")), ("commandDescription".to_string(), json!("Take off from the ground"))]);
        assert_eq!(command_info(&json!({}))[1].1, Value::Null);
    }

    #[test]
    fn the_altitude_card_explains_land_and_calculated_terrain_altitudes() {
        assert_eq!(altitude_hint(true, Some(crate::altitudemodes::RELATIVE), None).as_deref(), Some(LAND_ALTITUDE_HINT));
        assert_eq!(altitude_hint(false, Some(crate::altitudemodes::CALC_ABOVE_TERRAIN), Some("512.3 m".into())).as_deref(), Some("Actual AMSL alt sent: 512.3 m"));
        assert_eq!(altitude_hint(false, Some(crate::altitudemodes::RELATIVE), Some("512.3 m".into())), None);
    }

    #[test]
    fn an_unfinished_shape_names_the_tool_that_finishes_it() {
        assert!(area_help("survey", false).unwrap().contains("Polygon Tools"));
        assert!(area_help("CorridorScan", false).unwrap().contains("Polyline Tools"));
        assert!(area_help("StructureScan", false).unwrap().starts_with("Draw the structure outline"));
        assert_eq!(area_help("survey", true), None);
    }

    fn fact(name: &str, property: &str, value: f64) -> Value {
        json!({ "kind": "fact", "name": name, "property": property, "value": value, "valueString": format!("{value}"), "units": "m", "readOnly": false, "shortDescription": "" })
    }

    struct Plan {
        simple: bool,
        lists: bool,
        custom: bool,
    }

    impl Backend for Plan {
        fn get(&self, path: &str) -> String {
            match path {
                "plan.missionController.visualItems.2" => json!({ "kind": "object", "isSimpleItem": self.simple, "altitudeFrame": 1, "facts": [fact("Altitude", "altitude", 50.0), fact("Launch", LAUNCH_ALTITUDE, 0.0)] }),
                "plan.missionController.visualItems.2.textFieldFacts" if self.lists => json!({ "kind": "list", "elements": [fact("Hold", "", 5.0), json!({ "kind": "fact", "name": "" })] }),
                "plan.missionController.visualItems.2.comboboxFacts" if self.lists => json!({ "kind": "list", "elements": [fact("Mode", "", 1.0)] }),
                "plan.missionController.visualItems.2.cameraCalc" => json!({
                    "kind": "object", "cameraBrand": "Sony", "cameraModel": "RX100", "cameraBrandList": ["Manual", "Sony"], "cameraModelList": ["RX100"],
                    "xlatManualCameraName": "Manual (no camera specs)", "xlatCustomCameraName": "Custom Camera", "isCustomCamera": self.custom, "distanceMode": 1,
                    "facts": [fact("SensorWidth", "sensorWidth", 13.2), fact("FrontalOverlap", "frontalOverlap", 70.0), fact("DistanceToSurface", "distanceToSurface", 50.0), fact("ValueSetIsDistance", "valueSetIsDistance", 0.0)],
                }),
                _ => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, path: &str, _f: &str) -> String { self.get(path) }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn suffixes(view: &Value, key: &str) -> Vec<String> {
        view[key].as_array().unwrap().iter().map(|f| f["pathSuffix"].as_str().unwrap().to_string()).collect()
    }

    #[test]
    fn a_simple_item_edits_its_lists_and_carries_no_camera() {
        let view = item_facts_view(&Plan { simple: true, lists: true, custom: false }, &["2".to_string()]);
        assert_eq!(suffixes(&view, "fields"), ["textFieldFacts.0", "comboboxFacts.0"], "an unnamed element is not a field anyone can edit, and the position stays the list's own");
        assert_eq!(view["fields"][0]["path"], "plan.missionController.visualItems.2.textFieldFacts.0", "the path is the one the head writes");
        assert!(crate::factwrite::owns(view["fields"][0]["path"].as_str().unwrap()), "and that write is one the core validates");
        assert_eq!(view["fields"][0]["class"], "Control", "each field is view.control's shape, which the head already decodes");
        assert_eq!(view["camera"], Value::Null);
        assert_eq!(view["altitudeMode"], 1, "the item's own altitude mode, by the index asked about");
        assert_eq!(view["speedSection"], Value::Null, "an item with no speedSection object carries none");
    }

    #[test]
    fn a_complex_item_without_lists_edits_its_own_facts_and_its_camera() {
        let view = item_facts_view(&Plan { simple: false, lists: false, custom: false }, &["2".to_string()]);
        assert_eq!(suffixes(&view, "fields"), ["altitude"], "Mission Settings edits the launch altitude through its own control");
        let camera = &view["camera"];
        assert_eq!((&camera["brand"], &camera["model"], &camera["custom"], &camera["distanceMode"]), (&json!("Sony"), &json!("RX100"), &json!(false), &json!(1)));
        assert_eq!(camera["brands"], json!(["Manual", "Sony"]));
        assert_eq!(camera["brandPath"], "plan.missionController.visualItems.2.cameraCalc.cameraBrand");
        let shown: Vec<&str> = camera["facts"].as_array().unwrap().iter().map(|f| f["pathSuffix"].as_str().unwrap()).collect();
        assert_eq!(shown, ["cameraCalc.distanceToSurface", "cameraCalc.frontalOverlap"], "a catalogue camera shows only the survey figures, in the head's order");
        assert_eq!(camera["facts"][0]["group"], "Camera");
        assert_eq!((&camera["valueSetIsDistance"], &camera["valueSetIsDistancePath"]), (&json!(false), &json!("plan.missionController.visualItems.2.cameraCalc.valueSetIsDistance")), "Set by reads the bool fact, which Qt serves as a number");

        let custom = item_facts_view(&Plan { simple: false, lists: false, custom: true }, &["2".to_string()]);
        assert_eq!(custom["camera"]["facts"][0]["pathSuffix"], "cameraCalc.sensorWidth", "a custom camera's optics come first, then the survey figures");

        let listed = item_facts_view(&Plan { simple: false, lists: true, custom: false }, &["2".to_string()]);
        assert_eq!(suffixes(&listed, "fields"), ["textFieldFacts.0", "comboboxFacts.0"], "a complex item with lists edits the lists, as the head chose");
    }

    #[test]
    fn an_item_that_is_not_there_says_so() {
        let view = item_facts_view(&Plan { simple: true, lists: false, custom: false }, &["9".to_string()]);
        assert_eq!((&view["available"], &view["fields"], &view["camera"]), (&json!(false), &json!([]), &Value::Null));
        assert_eq!(item_facts_view(&Plan { simple: true, lists: false, custom: false }, &[])["kind"], "null");
    }
}
