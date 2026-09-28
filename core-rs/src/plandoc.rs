use serde_json::{Map, Value, json};

use crate::cmdinfo::{self, Firmware, VehicleClass};

const FRAME_GLOBAL: i64 = 0;
const FRAME_GLOBAL_RELATIVE_ALT: i64 = 3;
const FRAME_GLOBAL_TERRAIN_ALT: i64 = 10;
const TRANSECT_STYLE: &[&str] = &["survey", "CorridorScan"];
const FENCE_VERSION: i64 = 2;
const RALLY_VERSION: i64 = 2;
const MISSION_VERSION: i64 = 2;

#[derive(Debug, Clone, PartialEq)]
pub struct Altitude {
    pub mode: i64,
    pub altitude: f64,
    pub amsl_above_terrain: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Simple {
    pub command: i64,
    pub frame: i64,
    pub params: [Option<f64>; 7],
    pub auto_continue: bool,
    pub altitude: Option<Altitude>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Simple(Simple),
    Complex { kind: String, json: Value, item_count: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub firmware_type: i64,
    pub vehicle_type: i64,
    pub cruise_speed: f64,
    pub hover_speed: f64,
    pub global_altitude_mode: i64,
    pub home: Option<[f64; 3]>,
    pub items: Vec<Item>,
    pub fence: Value,
    pub rally: Value,
}

pub fn firmware(firmware_type: i64) -> Firmware {
    match firmware_type {
        12 => Firmware::Px4,
        3 => Firmware::ArduPilot,
        _ => Firmware::Generic,
    }
}

pub fn vehicle_class(vehicle_type: i64) -> VehicleClass {
    match vehicle_type {
        1 => VehicleClass::FixedWing,
        2 | 3 | 4 | 13 | 14 | 15 => VehicleClass::MultiRotor,
        19..=25 => VehicleClass::Vtol,
        12 => VehicleClass::Sub,
        10 | 11 => VehicleClass::Rover,
        _ => VehicleClass::Generic,
    }
}

pub fn load(text: &str) -> Result<Document, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| format!("The plan is not JSON: {e}"))?;
    if root.get("fileType").and_then(Value::as_str) != Some("Plan") {
        return Err("The file is not a plan.".to_string());
    }
    let mission = root.get("mission").ok_or("The plan has no mission.")?;
    let number = |key: &str, default: f64| mission.get(key).and_then(Value::as_f64).unwrap_or(default);
    let integer = |key: &str| mission.get(key).and_then(Value::as_i64).unwrap_or(0);
    let home = mission
        .get("plannedHomePosition")
        .and_then(Value::as_array)
        .filter(|h| h.len() >= 3)
        .map(|h| [h[0].as_f64().unwrap_or(0.0), h[1].as_f64().unwrap_or(0.0), h[2].as_f64().unwrap_or(0.0)])
        .ok_or("The plan has no planned home position.")?;
    let (firmware_type, vehicle_type) = (integer("firmwareType"), integer("vehicleType"));
    let commands = cmdinfo::tree(firmware(firmware_type), vehicle_class(vehicle_type));
    let items = mission
        .get("items")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(|item| load_item(item, &commands)).collect::<Result<Vec<_>, _>>())
        .transpose()?
        .unwrap_or_default();
    Ok(Document {
        firmware_type,
        vehicle_type,
        cruise_speed: number("cruiseSpeed", 0.0),
        hover_speed: number("hoverSpeed", 0.0),
        global_altitude_mode: integer("globalPlanAltitudeMode"),
        home: Some(home),
        items,
        fence: current_or_empty(root.get("geoFence"), FENCE_VERSION, json!({ "circles": [], "polygons": [], "version": FENCE_VERSION })),
        rally: current_or_empty(root.get("rallyPoints"), RALLY_VERSION, json!({ "points": [], "version": RALLY_VERSION })),
    })
}

fn current_or_empty(section: Option<&Value>, version: i64, empty: Value) -> Value {
    section.filter(|s| s.get("version").and_then(Value::as_i64) == Some(version)).cloned().unwrap_or(empty)
}

fn load_item(item: &Value, commands: &std::collections::BTreeMap<i64, cmdinfo::Command>) -> Result<Item, String> {
    match item.get("type").and_then(Value::as_str) {
        Some("SimpleItem") => load_simple(item, commands).map(Item::Simple),
        Some("ComplexItem") => {
            let kind = item.get("complexItemType").and_then(Value::as_str).unwrap_or("").to_string();
            let item_count = match TRANSECT_STYLE.contains(&kind.as_str()) {
                true => item.get("TransectStyleComplexItem").and_then(|t| t.get("Items")).and_then(Value::as_array).map(Vec::len).ok_or_else(|| format!("The {kind} item has no saved mission items."))?,
                false => return Err(format!("The core cannot hold a {kind} item yet.")),
            };
            Ok(Item::Complex { kind, json: item.clone(), item_count })
        }
        other => Err(format!("Unknown item type: {}", other.unwrap_or("none"))),
    }
}

fn load_simple(item: &Value, commands: &std::collections::BTreeMap<i64, cmdinfo::Command>) -> Result<Simple, String> {
    let saved = item.get("params").and_then(Value::as_array).ok_or("A mission item has no params.")?;
    let coordinate = item.get("coordinate").and_then(Value::as_array);
    let params: Vec<Option<f64>> = match (saved.len(), coordinate) {
        (7, _) => saved.iter().map(Value::as_f64).collect(),
        (4, Some(c)) if c.len() >= 3 => saved.iter().chain(c.iter().take(3)).map(Value::as_f64).collect(),
        _ => return Err("A mission item needs seven params, or four and a coordinate.".to_string()),
    };
    let field = |key: &str| item.get(key).and_then(Value::as_i64).ok_or_else(|| format!("A mission item has no {key}."));
    let (command, frame) = (field("command")?, field("frame")?);
    let specifies_altitude = commands.get(&command).is_some_and(|c| c.specifies_coordinate || c.specifies_altitude_only);
    let saved_altitude = ["AltitudeMode", "Altitude", "AMSLAltAboveTerrain"].iter().any(|key| item.get(*key).is_some());
    let altitude = match (saved_altitude, specifies_altitude) {
        (true, _) => Some(Altitude {
            mode: item.get("AltitudeMode").and_then(Value::as_i64).ok_or("A mission item's altitude has no mode.")?,
            altitude: item.get("Altitude").and_then(Value::as_f64).ok_or("A mission item's altitude has no value.")?,
            amsl_above_terrain: item.get("AMSLAltAboveTerrain").and_then(Value::as_f64),
        }),
        (false, true) => Some(Altitude {
            mode: match frame {
                FRAME_GLOBAL_RELATIVE_ALT => crate::altitudemodes::RELATIVE,
                _ => crate::altitudemodes::ABSOLUTE,
            },
            altitude: params[6].unwrap_or(f64::NAN),
            amsl_above_terrain: None,
        }),
        (false, false) => None,
    };
    Ok(Simple {
        command,
        frame,
        params: [params[0], params[1], params[2], params[3], params[4], params[5], params[6]],
        auto_continue: item.get("autoContinue").and_then(Value::as_bool).unwrap_or(true),
        altitude,
    })
}

pub struct Downloaded {
    pub frame: i64,
    pub command: i64,
    pub params: [f64; 7],
    pub auto_continue: bool,
}

pub fn from_vehicle(items: &[Downloaded], sends_home: bool, template: &Document) -> Document {
    let commands = cmdinfo::tree(firmware(template.firmware_type), vehicle_class(template.vehicle_type));
    let fake_home = items.first().filter(|_| sends_home);
    let home = fake_home
        .filter(|h| h.params[4] != 0.0 || h.params[5] != 0.0)
        .map(|h| [h.params[4], h.params[5], h.params[6]])
        .or(template.home);
    let listed = &items[usize::from(fake_home.is_some())..];
    let simple = |item: &Downloaded| {
        let specifies_altitude = commands.get(&item.command).is_some_and(|c| c.specifies_coordinate || c.specifies_altitude_only);
        Item::Simple(Simple {
            command: item.command,
            frame: item.frame,
            params: item.params.map(|p| Some(p).filter(|p| !p.is_nan())),
            auto_continue: item.auto_continue,
            altitude: specifies_altitude.then(|| Altitude {
                mode: match item.frame {
                    FRAME_GLOBAL_TERRAIN_ALT => crate::altitudemodes::TERRAIN_FRAME,
                    FRAME_GLOBAL => crate::altitudemodes::ABSOLUTE,
                    _ => crate::altitudemodes::RELATIVE,
                },
                altitude: item.params[6],
                amsl_above_terrain: None,
            }),
        })
    };
    Document {
        home,
        items: listed.iter().map(simple).collect(),
        global_altitude_mode: match listed.is_empty() {
            true => crate::altitudemodes::RELATIVE,
            false => crate::altitudemodes::MIXED,
        },
        ..template.clone()
    }
}

pub struct EditDefaults {
    pub mission_item_altitude: f64,
}

const CMD_NAV_WAYPOINT: i64 = 16;
const PLANNED_HOME_OFFSET_M: f64 = 30.0;

fn frame_for(mode: i64) -> i64 {
    match mode {
        crate::altitudemodes::RELATIVE => FRAME_GLOBAL_RELATIVE_ALT,
        crate::altitudemodes::TERRAIN_FRAME => FRAME_GLOBAL_TERRAIN_ALT,
        _ => FRAME_GLOBAL,
    }
}

fn default_mode(doc: &Document) -> i64 {
    match doc.global_altitude_mode {
        crate::altitudemodes::MIXED => crate::altitudemodes::RELATIVE,
        mode => mode,
    }
}

fn previous_altitude(doc: &Document, commands: &std::collections::BTreeMap<i64, cmdinfo::Command>, visual_index: i64) -> Option<(f64, i64)> {
    let before = usize::try_from(visual_index - 1).ok()?.min(doc.items.len());
    doc.items[..before].iter().rev().find_map(|item| match item {
        Item::Simple(s) if commands.get(&s.command).is_some_and(|c| c.specifies_coordinate && !c.standalone_coordinate) => s.altitude.as_ref().map(|a| (a.altitude, a.mode)),
        _ => None,
    })
}

fn param_defaults(command: Option<&cmdinfo::Command>) -> [Option<f64>; 7] {
    std::array::from_fn(|i| {
        let listed = command.and_then(|c| c.params.get(&(i as u8 + 1)));
        match listed {
            Some(param) => param.get("default").and_then(Value::as_f64),
            None => Some(0.0),
        }
    })
}

pub fn insert_waypoint(doc: &Document, latitude: f64, longitude: f64, visual_index: i64, defaults: &EditDefaults) -> Document {
    let commands = cmdinfo::tree(firmware(doc.firmware_type), vehicle_class(doc.vehicle_type));
    let listed = param_defaults(commands.get(&CMD_NAV_WAYPOINT));
    let (altitude, mode) = previous_altitude(doc, &commands, visual_index)
        .map(|(altitude, mode)| (altitude, if doc.global_altitude_mode == crate::altitudemodes::MIXED { mode } else { default_mode(doc) }))
        .unwrap_or((defaults.mission_item_altitude, default_mode(doc)));
    let waypoint = Item::Simple(Simple {
        command: CMD_NAV_WAYPOINT,
        frame: frame_for(mode),
        params: [listed[0], Some(0.0), listed[2], listed[3], Some(latitude), Some(longitude), Some(altitude)],
        auto_continue: true,
        altitude: Some(Altitude { mode, altitude, amsl_above_terrain: None }),
    });
    let at = usize::try_from(visual_index - 1).ok().filter(|i| *i <= doc.items.len()).unwrap_or(doc.items.len());
    let items: Vec<Item> = doc.items[..at].iter().cloned().chain(std::iter::once(waypoint)).chain(doc.items[at..].iter().cloned()).collect();
    let first_coordinate = items.iter().find_map(|item| match item {
        Item::Simple(s) if commands.get(&s.command).is_some_and(|c| c.specifies_coordinate) => Some((s.params[4].unwrap_or(0.0), s.params[5].unwrap_or(0.0))),
        _ => None,
    });
    let home = doc.home.or_else(|| {
        let (lat, lon) = crate::surveygrid::at_distance_and_azimuth(first_coordinate.unwrap_or((latitude, longitude)), PLANNED_HOME_OFFSET_M, 0.0);
        Some([lat, lon, 0.0])
    });
    Document { items, home, ..doc.clone() }
}

pub fn remove(doc: &Document, visual_index: usize) -> Option<Document> {
    let at = visual_index.checked_sub(1).filter(|i| *i < doc.items.len())?;
    Some(Document { items: doc.items.iter().enumerate().filter(|(i, _)| *i != at).map(|(_, item)| item.clone()).collect(), ..doc.clone() })
}

pub fn save(doc: &Document) -> Value {
    let starts = doc.items.iter().scan(1usize, |next, item| {
        let start = *next;
        *next += match item {
            Item::Simple(_) => 1,
            Item::Complex { item_count, .. } => *item_count,
        };
        Some(start)
    });
    let items: Vec<Value> = doc.items.iter().zip(starts).map(|(item, seq)| save_item(item, seq)).collect();
    json!({
        "fileType": "Plan",
        "groundStation": "QGroundControl",
        "version": 1,
        "geoFence": doc.fence,
        "rallyPoints": doc.rally,
        "mission": {
            "cruiseSpeed": doc.cruise_speed,
            "firmwareType": doc.firmware_type,
            "globalPlanAltitudeMode": doc.global_altitude_mode,
            "hoverSpeed": doc.hover_speed,
            "items": items,
            "plannedHomePosition": doc.home.unwrap_or([0.0, 0.0, 0.0]),
            "vehicleType": doc.vehicle_type,
            "version": MISSION_VERSION,
        },
    })
}

fn save_item(item: &Item, seq: usize) -> Value {
    match item {
        Item::Complex { json, .. } => json.clone(),
        Item::Simple(simple) => {
            let base = json!({
                "type": "SimpleItem",
                "autoContinue": simple.auto_continue,
                "command": simple.command,
                "doJumpId": seq,
                "frame": simple.frame,
                "params": simple.params,
            });
            let altitude: Map<String, Value> = simple
                .altitude
                .iter()
                .flat_map(|a| [("AltitudeMode", json!(a.mode)), ("Altitude", json!(a.altitude)), ("AMSLAltAboveTerrain", json!(a.amsl_above_terrain))])
                .map(|(k, v)| (k.to_string(), v))
                .collect();
            match base {
                Value::Object(fields) => Value::Object(fields.into_iter().chain(altitude).collect()),
                other => other,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by_value(value: Value) -> Value {
        match value {
            Value::Number(n) => json!(n.as_f64()),
            Value::Array(items) => Value::Array(items.into_iter().map(by_value).collect()),
            Value::Object(fields) => Value::Object(fields.into_iter().map(|(k, v)| (k, by_value(v))).collect()),
            other => other,
        }
    }

    fn without_home_altitude(mut plan: Value) -> Value {
        plan["mission"]["plannedHomePosition"][2] = Value::Null;
        by_value(plan)
    }

    fn resaves_as_qt(original: &str, resaved_by_qt: &str) {
        let doc = load(original).unwrap();
        let qt: Value = serde_json::from_str(resaved_by_qt).unwrap();
        assert_eq!(without_home_altitude(save(&doc)), without_home_altitude(qt));
    }

    #[test]
    fn an_old_format_plan_is_written_back_the_way_qt_upgrades_it() {
        resaves_as_qt(include_str!("../../test/MissionManager/SectionTest.plan"), include_str!("../tests/fixtures/sectiontest-resaved-by-qt.plan"));
    }

    #[test]
    fn a_survey_plan_is_written_back_the_way_qt_writes_it() {
        resaves_as_qt(include_str!("../tests/fixtures/survey-upload.plan"), include_str!("../tests/fixtures/survey-resaved-by-qt.plan"));
    }

    #[test]
    fn a_loaded_document_flattens_to_the_same_upload_as_the_file() {
        let text = include_str!("../tests/fixtures/survey-upload.plan");
        let from_doc = crate::planitems::flatten(&save(&load(text).unwrap())).unwrap();
        let from_file = crate::planitems::flatten(&serde_json::from_str(text).unwrap()).unwrap();
        assert_eq!(format!("{from_doc:?}"), format!("{from_file:?}"));
    }

    fn section() -> Document {
        load(include_str!("../../test/MissionManager/SectionTest.plan")).unwrap()
    }

    const QT_DEFAULTS: EditDefaults = EditDefaults { mission_item_altitude: 75.0 };

    fn matches_qt(doc: &Document, qt: &str) {
        let qt: Value = serde_json::from_str(qt).unwrap();
        assert_eq!(without_home_altitude(save(doc)), without_home_altitude(qt));
    }

    #[test]
    fn an_appended_waypoint_takes_the_default_altitude_because_qt_never_looks_back_on_append() {
        matches_qt(&insert_waypoint(&section(), 47.634, -122.089, -1, &QT_DEFAULTS), include_str!("../tests/fixtures/edit-A-by-qt.plan"));
    }

    #[test]
    fn an_inserted_waypoint_copies_the_altitude_before_it() {
        matches_qt(&insert_waypoint(&section(), 47.6335, -122.0885, 3, &QT_DEFAULTS), include_str!("../tests/fixtures/edit-B-by-qt.plan"));
    }

    #[test]
    fn a_removed_item_renumbers_those_after_it() {
        matches_qt(&remove(&section(), 2).unwrap(), include_str!("../tests/fixtures/edit-C-by-qt.plan"));
        assert!(remove(&section(), 0).is_none(), "visual index zero is the mission settings item, which is never removed");
        assert!(remove(&section(), 6).is_none());
    }

    #[test]
    fn the_first_waypoint_of_an_empty_plan_puts_home_thirty_metres_north_of_it() {
        let empty = Document { home: None, items: Vec::new(), ..section() };
        let placed = insert_waypoint(&empty, 47.64, -122.1, -1, &QT_DEFAULTS);
        matches_qt(&placed, include_str!("../tests/fixtures/edit-D-by-qt.plan"));
        let home = placed.home.unwrap();
        assert!((home[0] - 47.64026979617687).abs() < 1e-12 && home[1] == -122.1 && home[2] == 0.0);
    }

    #[test]
    fn version_one_fences_and_rally_points_are_dropped_as_qt_drops_them() {
        let doc = load(include_str!("../../test/MissionManager/SectionTest.plan")).unwrap();
        assert_eq!(doc.fence, json!({ "circles": [], "polygons": [], "version": 2 }));
        assert_eq!(doc.rally, json!({ "points": [], "version": 2 }));
    }

    #[test]
    fn an_item_the_core_cannot_hold_refuses_the_whole_plan() {
        let plan = json!({ "fileType": "Plan", "mission": { "plannedHomePosition": [0, 0, 0], "items": [{ "type": "ComplexItem", "complexItemType": "StructureScan" }] } });
        assert!(load(&plan.to_string()).unwrap_err().contains("StructureScan"));
    }
}
