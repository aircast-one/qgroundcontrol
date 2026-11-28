use serde_json::{Map, Value, json};

use crate::cmdinfo::{self, Firmware, VehicleClass};

const FRAME_GLOBAL: i64 = 0;
const FRAME_GLOBAL_RELATIVE_ALT: i64 = 3;
const FRAME_GLOBAL_TERRAIN_ALT: i64 = 10;
const FRAME_GLOBAL_INT: i64 = 5;
const FRAME_GLOBAL_RELATIVE_ALT_INT: i64 = 6;
const FRAME_GLOBAL_TERRAIN_ALT_INT: i64 = 11;
pub const TRANSECT_STYLE: &[&str] = &["survey", "CorridorScan"];
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
    pub sections: Vec<Simple>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Simple(Simple),
    Complex {
        kind: String,
        json: Value,
        item_count: usize,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub firmware_type: i64,
    pub vehicle_type: i64,
    pub cruise_speed: f64,
    pub hover_speed: f64,
    pub global_altitude_mode: i64,
    pub home: Option<[f64; 3]>,
    pub settings_sections: Vec<Simple>,
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
        7 => VehicleClass::Airship,
        _ => VehicleClass::Generic,
    }
}

fn validate_plan(root: &Value) -> Result<(), String> {
    crate::qtjson::validate_keys(
        root,
        &[
            ("groundStation", "String", true),
            ("fileType", "String", true),
            ("version", "Double", true),
        ],
    )?;
    let file_type = root["fileType"].as_str().unwrap_or("");
    if file_type != "Plan" {
        return Err(format!(
            "Incorrect file type key expected:Plan actual:{file_type}"
        ));
    }
    let version = crate::qtjson::to_int(&root["version"], 0);
    match version {
        v if v < 1 => return Err(format!("File version {v} is no longer supported")),
        v if v > 1 => {
            return Err(format!(
                "File version {v} is newer than current supported version 1"
            ));
        }
        _ => {}
    }
    crate::qtjson::validate_keys(
        root,
        &[
            ("mission", "Object", true),
            ("geoFence", "Object", true),
            ("rallyPoints", "Object", true),
        ],
    )?;
    crate::qtjson::validate_keys(
        &root["mission"],
        &[
            ("plannedHomePosition", "Array", true),
            ("items", "Array", true),
            ("firmwareType", "Double", true),
            ("vehicleType", "Double", false),
            ("cruiseSpeed", "Double", false),
            ("hoverSpeed", "Double", false),
            ("globalPlanAltitudeMode", "Double", false),
        ],
    )
    .map_err(|e| format!("Mission: {e}"))
}

fn validate_fence_and_rally(root: &Value) -> Result<(), String> {
    validate_section(
        &root["geoFence"],
        FENCE_VERSION,
        &[
            ("circles", "Array", true),
            ("polygons", "Array", true),
            ("breachReturn", "Array", false),
        ],
        "GeoFence supports version",
        true,
    )?;
    if is_current(&root["geoFence"], FENCE_VERSION) {
        validate_fence(&root["geoFence"])?;
    }
    validate_section(
        &root["rallyPoints"],
        RALLY_VERSION,
        &[("points", "Array", true)],
        "Rally Points supports version",
        false,
    )?;
    if is_current(&root["rallyPoints"], RALLY_VERSION) {
        coordinates(&root["rallyPoints"]["points"], true).map_err(|e| format!("Rally: {e}"))?;
    }
    Ok(())
}

fn is_current(section: &Value, version: i64) -> bool {
    section
        .get("version")
        .map(|v| i64::from(crate::qtjson::to_int(v, 0)))
        == Some(version)
}

fn validate_fence(fence: &Value) -> Result<(), String> {
    let listed = |key: &str| fence[key].as_array().map(Vec::as_slice).unwrap_or_default();
    listed("polygons").iter().try_for_each(|polygon| {
        fence_shape(polygon, "Polygon")?;
        polygon
            .get("polygon")
            .ok_or_else(|| "The following required keys are missing: polygon".to_string())
            .and_then(|path| coordinates(path, false))
    })?;
    listed("circles").iter().try_for_each(|circle| {
        fence_shape(circle, "Circle")?;
        crate::qtjson::validate_keys(circle, &[("circle", "Object", true)])?;
        crate::qtjson::validate_keys(
            &circle["circle"],
            &[("center", "Array", true), ("radius", "Double", true)],
        )?;
        coordinate(&circle["circle"]["center"], false)
    })?;
    fence
        .get("breachReturn")
        .map_or(Ok(()), |point| coordinate(point, true))
}

fn fence_shape(shape: &Value, kind: &str) -> Result<(), String> {
    if !shape.is_object() {
        return Err(format!(
            "GeoFence {} not stored as object",
            kind.to_lowercase()
        ));
    }
    crate::qtjson::validate_keys(
        shape,
        &[("version", "Double", true), ("inclusion", "Bool", true)],
    )?;
    is_current(shape, 1)
        .then_some(())
        .ok_or_else(|| format!("GeoFence {kind} only supports version 1"))
}

pub(crate) fn coordinates(value: &Value, altitude: bool) -> Result<(), String> {
    value
        .as_array()
        .ok_or_else(|| "value for coordinate array is not array".to_string())?
        .iter()
        .try_for_each(|point| coordinate(point, altitude))
}

pub(crate) fn coordinate(value: &Value, altitude: bool) -> Result<(), String> {
    let values = value
        .as_array()
        .ok_or_else(|| "value for coordinate is not array".to_string())?;
    let count = if altitude { 3 } else { 2 };
    if values.len() != count {
        return Err(format!("Coordinate array must contain {count} values"));
    }
    values
        .iter()
        .find(|v| !(v.is_number() || v.is_null()))
        .map_or(Ok(()), |v| {
            Err(format!(
                "Coordinate array may only contain double values, found: {}",
                qt_json_type(v)
            ))
        })
}

pub(crate) fn qt_json_type(value: &Value) -> u8 {
    match value {
        Value::Null => 0,
        Value::Bool(_) => 1,
        Value::Number(_) => 2,
        Value::String(_) => 3,
        Value::Array(_) => 4,
        Value::Object(_) => 5,
    }
}

fn validate_section(
    section: &Value,
    version: i64,
    keys: &[(&str, &str, bool)],
    refusal: &str,
    unversioned_is_old: bool,
) -> Result<(), String> {
    let held = section
        .get("version")
        .map(|v| i64::from(crate::qtjson::to_int(v, 0)));
    if held == Some(1) || (held.is_none() && unversioned_is_old) {
        return Ok(());
    }
    crate::qtjson::validate_keys(section, &[&[("version", "Double", true)], keys].concat())?;
    (held == Some(version))
        .then_some(())
        .ok_or_else(|| format!("{refusal} {version}"))
}

pub fn load(text: &str, offline_vehicle_type: i64) -> Result<Document, String> {
    load_with(text, offline_vehicle_type, resolve_jumps)
}

pub fn load_text_mission(text: &str, offline_vehicle_type: i64) -> Result<Document, String> {
    load_with(text, offline_vehicle_type, |items, _| Ok(items))
}

type Jumps = fn(Vec<Item>, &[Value]) -> Result<Vec<Item>, String>;

fn load_with(text: &str, offline_vehicle_type: i64, jumps: Jumps) -> Result<Document, String> {
    let root: Value =
        serde_json::from_str(text).map_err(|e| format!("The plan is not JSON: {e}"))?;
    validate_plan(&root)?;
    let mission = &root["mission"];
    let number =
        |key: &str, default: f64| mission.get(key).and_then(Value::as_f64).unwrap_or(default);
    let integer = |key: &str| mission.get(key).and_then(whole).unwrap_or(0);
    coordinate(&mission["plannedHomePosition"], true).map_err(|e| format!("Mission: {e}"))?;
    let saved_home = |index: usize| {
        mission["plannedHomePosition"][index]
            .as_f64()
            .unwrap_or(0.0)
    };
    let home = [saved_home(0), saved_home(1), saved_home(2)];
    let (firmware_type, vehicle_type) = (
        integer("firmwareType"),
        mission
            .get("vehicleType")
            .and_then(Value::as_f64)
            .map_or(offline_vehicle_type, |v| v as i64),
    );
    let commands = cmdinfo::tree(firmware(firmware_type), vehicle_class(vehicle_type));
    let saved = mission["items"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let items = saved
        .iter()
        .enumerate()
        .map(|(index, item)| load_item(index, item, &commands, vehicle_class(vehicle_type)))
        .collect::<Result<Vec<_>, _>>()
        .and_then(|items| jumps(items, saved))
        .map_err(|e| format!("Mission: {e}"))?;
    validate_fence_and_rally(&root)?;
    let items = crate::landingpattern::fold(items, firmware(firmware_type) == Firmware::ArduPilot);
    let (settings_sections, items) = fold(items, vehicle_class(vehicle_type));
    Ok(Document {
        firmware_type,
        vehicle_type,
        cruise_speed: number("cruiseSpeed", f64::NAN),
        hover_speed: number("hoverSpeed", f64::NAN),
        global_altitude_mode: integer("globalPlanAltitudeMode"),
        home: Some(home),
        settings_sections,
        items,
        fence: current_or_empty(
            root.get("geoFence"),
            FENCE_VERSION,
            json!({ "circles": [], "polygons": [], "version": FENCE_VERSION }),
        ),
        rally: current_or_empty(
            root.get("rallyPoints"),
            RALLY_VERSION,
            json!({ "points": [], "version": RALLY_VERSION }),
        ),
    })
}

const CMD_DO_JUMP: i64 = 177;

pub fn item_spans(doc: &Document) -> Vec<(usize, usize)> {
    doc.items
        .iter()
        .scan(doc.settings_sections.len() + 1, |next, item| {
            let start = *next;
            let span = match item {
                Item::Simple(simple) => 1 + simple.sections.len(),
                Item::Complex { item_count, .. } => *item_count,
            };
            *next += span;
            Some((start, span))
        })
        .collect()
}

pub const JUMP_COMMAND: i64 = CMD_DO_JUMP;
pub const JUMP_TARGET_PARAM: usize = 1;

pub fn row_of_sequence(doc: &Document, sequence: f64) -> Option<usize> {
    jump_target(&item_spans(doc), Some(sequence)).map(|(at, _)| at + 1)
}

pub fn first_sequence(doc: &Document, visual_index: usize) -> Option<usize> {
    item_spans(doc)
        .get(visual_index.checked_sub(1)?)
        .map(|(start, _)| *start)
}

fn jump_seq(item: &Item) -> Option<f64> {
    match item {
        Item::Simple(simple) if simple.command == CMD_DO_JUMP => simple.params[0],
        _ => None,
    }
}

fn jump_target(spans: &[(usize, usize)], seq: Option<f64>) -> Option<(usize, usize)> {
    let seq = seq.filter(|seq| seq.is_finite() && *seq >= 1.0)? as usize;
    spans
        .iter()
        .position(|(start, span)| (*start..start + span).contains(&seq))
        .map(|at| (at, seq - spans[at].0))
}

fn kept_jumps(
    before: &Document,
    after: Document,
    moved_to: &dyn Fn(usize) -> Option<usize>,
) -> Document {
    let (old, new) = (item_spans(before), item_spans(&after));
    let came_from = |to: usize| (0..before.items.len()).find(|from| moved_to(*from) == Some(to));
    let items = after
        .items
        .iter()
        .enumerate()
        .map(|(to, item)| {
            let retargeted = came_from(to)
                .and(jump_target(&old, jump_seq(item)))
                .and_then(|(target, offset)| Some(new[moved_to(target)?].0 + offset));
            match (item, retargeted) {
                (Item::Simple(jump), Some(seq)) => Item::Simple(Simple {
                    params: std::array::from_fn(|i| match i {
                        0 => Some(seq as f64),
                        _ => jump.params[i],
                    }),
                    ..jump.clone()
                }),
                _ => item.clone(),
            }
        })
        .collect();
    Document { items, ..after }
}

fn inserted_at(doc: &Document, at: usize, item: Item) -> Document {
    let items = doc.items[..at]
        .iter()
        .cloned()
        .chain(std::iter::once(item))
        .chain(doc.items[at..].iter().cloned())
        .collect();
    kept_jumps(
        doc,
        Document {
            items,
            ..doc.clone()
        },
        &|from| Some(if from < at { from } else { from + 1 }),
    )
}

fn resolve_jumps(items: Vec<Item>, saved: &[Value]) -> Result<Vec<Item>, String> {
    let starts = items.iter().scan(1i64, |next, item| {
        let start = *next;
        *next += match item {
            Item::Simple(_) => 1,
            Item::Complex { item_count, .. } => *item_count as i64,
        };
        Some(start)
    });
    let targets: Vec<(i64, i64)> = saved
        .iter()
        .zip(starts)
        .filter(|(item, _)| item["type"] == "SimpleItem")
        .filter_map(|(item, seq)| whole(&item["doJumpId"]).map(|id| (id, seq)))
        .collect();
    items
        .into_iter()
        .map(|item| match item {
            Item::Simple(jump) if jump.command == CMD_DO_JUMP => {
                let id = jump.params[0].unwrap_or(f64::NAN) as i64;
                let seq = targets
                    .iter()
                    .find(|(target, _)| *target == id)
                    .map(|(_, seq)| *seq)
                    .ok_or_else(|| format!("Could not find doJumpId: {id}"))?;
                let params = std::array::from_fn(|i| {
                    if i == 0 {
                        Some(seq as f64)
                    } else {
                        jump.params[i]
                    }
                });
                Ok(Item::Simple(Simple { params, ..jump }))
            }
            other => Ok(other),
        })
        .collect()
}

fn current_or_empty(section: Option<&Value>, version: i64, empty: Value) -> Value {
    section
        .filter(|s| s.get("version").and_then(whole) == Some(version))
        .cloned()
        .unwrap_or(empty)
}

fn load_item(
    index: usize,
    item: &Value,
    commands: &std::collections::BTreeMap<i64, cmdinfo::Command>,
    class: VehicleClass,
) -> Result<Item, String> {
    if !item.is_object() {
        return Err(format!("Mission item {index} is not an object"));
    }
    crate::qtjson::validate_keys(item, &[("type", "String", true)])?;
    match item["type"].as_str().unwrap_or("") {
        "SimpleItem" => load_simple(item, commands).map(Item::Simple),
        "ComplexItem" => {
            crate::qtjson::validate_keys(item, &[("complexItemType", "String", true)])?;
            let kind = item["complexItemType"].as_str().unwrap_or("").to_string();
            let json = crate::transectload::complex(&kind, item, class)?;
            let item_count = complex_count(&kind, &json)?;
            Ok(Item::Complex {
                kind,
                json,
                item_count,
            })
        }
        other => Err(format!("Unknown item type: {other}")),
    }
}

fn load_simple(
    saved: &Value,
    commands: &std::collections::BTreeMap<i64, cmdinfo::Command>,
) -> Result<Simple, String> {
    let item = crate::transectload::mission_item(saved)?;
    let param = |index: usize| item["params"][index].as_f64();
    let params: [Option<f64>; 7] = std::array::from_fn(param);
    let (command, frame) = (
        whole(&item["command"]).unwrap_or(0),
        whole(&item["frame"]).unwrap_or(0),
    );
    let specifies_altitude = commands
        .get(&command)
        .is_some_and(|c| c.specifies_coordinate || c.specifies_altitude_only);
    let saved_altitude = ["AltitudeMode", "Altitude", "AMSLAltAboveTerrain"]
        .iter()
        .any(|key| item.get(*key).is_some());
    let altitude = match (specifies_altitude, saved_altitude) {
        (true, true) => {
            crate::qtjson::validate_keys(
                &item,
                &[
                    ("AltitudeMode", "Double", true),
                    ("Altitude", "Double", true),
                    ("AMSLAltAboveTerrain", "NULL", true),
                ],
            )?;
            Some(Altitude {
                mode: item["AltitudeMode"].as_f64().map_or(0, |v| v as i64),
                altitude: item["Altitude"].as_f64().unwrap_or(f64::NAN),
                amsl_above_terrain: item["AMSLAltAboveTerrain"].as_f64(),
            })
        }
        (true, false) => Some(Altitude {
            mode: match frame {
                FRAME_GLOBAL_RELATIVE_ALT => crate::altitudemodes::RELATIVE,
                _ => crate::altitudemodes::ABSOLUTE,
            },
            altitude: params[6].unwrap_or(f64::NAN),
            amsl_above_terrain: None,
        }),
        (false, _) => None,
    };
    Ok(Simple {
        command,
        frame,
        params,
        auto_continue: item["autoContinue"].as_bool().unwrap_or(true),
        altitude,
        sections: Vec::new(),
    })
}

#[derive(Debug, Clone)]
pub struct Downloaded {
    pub frame: i64,
    pub command: i64,
    pub params: [f64; 7],
    pub auto_continue: bool,
}

pub fn from_vehicle(items: &[Downloaded], sends_home: bool, template: &Document) -> Document {
    let commands = cmdinfo::tree(
        firmware(template.firmware_type),
        vehicle_class(template.vehicle_type),
    );
    let fake_home = items.first().filter(|_| sends_home);
    let home = fake_home
        .filter(|h| h.params[4] != 0.0 || h.params[5] != 0.0)
        .map(|h| [h.params[4], h.params[5], h.params[6]]);
    let listed = &items[usize::from(fake_home.is_some())..];
    let simple = |item: &Downloaded| {
        let info = commands.get(&item.command);
        let specifies_altitude =
            info.is_some_and(|c| c.specifies_coordinate || c.specifies_altitude_only);
        let launched_here = info.is_some_and(|c| c.is_takeoff && !c.specifies_coordinate);
        let params = match (launched_here, home) {
            (true, Some(h)) => [
                item.params[0],
                item.params[1],
                item.params[2],
                item.params[3],
                h[0],
                h[1],
                item.params[6],
            ],
            _ => item.params,
        };
        Item::Simple(Simple {
            command: item.command,
            frame: item.frame,
            params: params.map(|p| Some(p).filter(|p| !p.is_nan())),
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
            sections: Vec::new(),
        })
    };
    let landings = crate::landingpattern::fold(
        listed.iter().map(simple).collect(),
        firmware(template.firmware_type) == Firmware::ArduPilot,
    );
    let (settings_sections, items) = fold(landings, vehicle_class(template.vehicle_type));
    let climbs_from_launch = |s: &Simple| {
        commands.get(&s.command).is_some_and(|c| c.is_takeoff)
            && s.altitude.as_ref().map(|held| held.mode) == Some(crate::altitudemodes::RELATIVE)
    };
    let modes: Vec<i64> = items
        .iter()
        .filter_map(|item| match item {
            Item::Simple(s) if climbs_from_launch(s) => None,
            Item::Simple(s) => s.altitude.as_ref().map(|held| held.mode),
            Item::Complex { .. } => Some(crate::altitudemodes::MIXED),
        })
        .collect();
    Document {
        home,
        settings_sections,
        global_altitude_mode: match modes.first() {
            None => crate::altitudemodes::RELATIVE,
            Some(first) if modes.iter().all(|mode| mode == first) => *first,
            Some(_) => crate::altitudemodes::MIXED,
        },
        items,
        ..template.clone()
    }
}

pub struct EditDefaults {
    pub mission_item_altitude: f64,
    pub map_center: Option<(f64, f64)>,
    pub vtol_transition_distance: f64,
}

pub const VTOL_TRANSITION_DISTANCE_DEFAULT: f64 = 300.0;

pub const CMD_NAV_WAYPOINT: i64 = 16;
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

fn previous_altitude(
    doc: &Document,
    commands: &std::collections::BTreeMap<i64, cmdinfo::Command>,
    visual_index: i64,
) -> Option<(f64, i64)> {
    let before = usize::try_from(visual_index - 1).ok()?.min(doc.items.len());
    doc.items[..before]
        .iter()
        .rev()
        .find_map(|item| match item {
            Item::Simple(s)
                if commands
                    .get(&s.command)
                    .is_some_and(|c| c.specifies_coordinate && !c.standalone_coordinate) =>
            {
                s.altitude.as_ref().map(|a| (a.altitude, a.mode))
            }
            _ => None,
        })
}

pub fn planned_home(doc: &Document, vehicle_home: Option<[f64; 3]>) -> Option<[f64; 3]> {
    let commands = cmdinfo::tree(firmware(doc.firmware_type), vehicle_class(doc.vehicle_type));
    let takes_off = doc.items.iter().any(|item| matches!(item, Item::Simple(s) if commands.get(&s.command).is_some_and(|c| c.is_takeoff)));
    doc.home
        .or(vehicle_home.filter(|_| takes_off))
        .or_else(|| home_from_first_coordinate(doc, None))
}

pub fn home_from_first_coordinate(doc: &Document, clicked: Option<(f64, f64)>) -> Option<[f64; 3]> {
    let commands = cmdinfo::tree(firmware(doc.firmware_type), vehicle_class(doc.vehicle_type));
    let first = doc.items.iter().find_map(|item| match item {
        Item::Simple(s)
            if commands
                .get(&s.command)
                .is_some_and(|c| c.specifies_coordinate) =>
        {
            s.params[4].zip(s.params[5])
        }
        _ => None,
    });
    first.or(clicked).map(|from| {
        let (latitude, longitude) =
            crate::surveygrid::at_distance_and_azimuth(from, PLANNED_HOME_OFFSET_M, 0.0);
        [latitude, longitude, 0.0]
    })
}

fn whole(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value.as_f64().map(|f| {
            if f.fract() == 0.0 && (f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&f) {
                f as i64
            } else {
                0
            }
        })
    })
}

pub fn previous_coordinate(doc: &Document, visual_index: i64) -> Option<(f64, f64)> {
    let commands = cmdinfo::tree(firmware(doc.firmware_type), vehicle_class(doc.vehicle_type));
    let before = usize::try_from(visual_index - 1).ok()?.min(doc.items.len());
    doc.items[..before]
        .iter()
        .rev()
        .find_map(|item| match item {
            Item::Simple(s)
                if commands
                    .get(&s.command)
                    .is_some_and(|c| c.specifies_coordinate && !c.standalone_coordinate) =>
            {
                s.params[4].zip(s.params[5])
            }
            _ => None,
        })
}

pub fn previous_altitude_mode(doc: &Document, visual_index: i64) -> Option<i64> {
    let commands = cmdinfo::tree(firmware(doc.firmware_type), vehicle_class(doc.vehicle_type));
    previous_altitude(doc, &commands, visual_index).map(|(_, mode)| mode)
}

pub fn param_defaults(command: Option<&cmdinfo::Command>) -> [Option<f64>; 7] {
    std::array::from_fn(|i| {
        let listed = command.and_then(|c| c.params.get(&(i as u8 + 1)));
        match listed {
            Some(param) => param.get("default").and_then(Value::as_f64),
            None => Some(0.0),
        }
    })
}

const CMD_NAV_RETURN_TO_LAUNCH: i64 = 20;
const CMD_DO_SET_ROI_LOCATION: i64 = 195;
const FRAME_MISSION: i64 = 2;

fn with_command_defaults(
    doc: &Document,
    commands: &std::collections::BTreeMap<i64, cmdinfo::Command>,
    command: i64,
    kept: [Option<f64>; 7],
    defaults: &EditDefaults,
) -> Simple {
    let info = commands.get(&command);
    let coordinate = info.is_some_and(|c| c.specifies_coordinate || c.standalone_coordinate);
    let specifies_altitude =
        info.is_some_and(|c| c.specifies_coordinate || c.specifies_altitude_only);
    let listed = param_defaults(info);
    let unplaced = kept[4].unwrap_or(0.0) == 0.0 && kept[5].unwrap_or(0.0) == 0.0;
    let (latitude, longitude) = match (
        coordinate,
        unplaced.then_some(defaults.map_center).flatten(),
    ) {
        (true, Some((latitude, longitude))) => (Some(latitude), Some(longitude)),
        (true, None) => (kept[4], kept[5]),
        (false, _) => (Some(0.0), Some(0.0)),
    };
    let takeoff = info.is_some_and(|c| c.is_takeoff);
    let mode = match takeoff {
        true => crate::altitudemodes::RELATIVE,
        false => default_mode(doc),
    };
    let grounded = info.is_some_and(|c| c.is_land) || command == CMD_DO_SET_ROI_LOCATION;
    let altitude = match (specifies_altitude, grounded) {
        (true, true) => Some(0.0),
        (true, false) => Some(defaults.mission_item_altitude),
        (false, _) => None,
    };
    let param = |i: usize| {
        info.and_then(|c| c.params.get(&(i as u8 + 1)))
            .map_or(Some(0.0), |_| listed[i])
    };
    let seventh = match altitude {
        Some(alt) => Some(alt),
        None => info
            .and_then(|c| c.params.get(&7))
            .map_or(Some(0.0), |_| listed[6]),
    };
    let fifth = info
        .and_then(|c| c.params.get(&5))
        .map_or(latitude, |_| listed[4]);
    let sixth = info
        .and_then(|c| c.params.get(&6))
        .map_or(longitude, |_| listed[5]);
    Simple {
        command,
        frame: match altitude {
            Some(_) => frame_for(mode),
            None => FRAME_MISSION,
        },
        params: [
            param(0),
            if command == CMD_NAV_WAYPOINT {
                Some(0.0)
            } else {
                param(1)
            },
            param(2),
            param(3),
            fifth,
            sixth,
            seventh,
        ],
        auto_continue: true,
        altitude: altitude.map(|altitude| Altitude {
            mode,
            altitude,
            amsl_above_terrain: None,
        }),
        sections: Vec::new(),
    }
}

pub fn insert_simple(
    doc: &Document,
    command: i64,
    latitude: f64,
    longitude: f64,
    visual_index: i64,
    defaults: &EditDefaults,
) -> Document {
    let commands = commands_at(
        doc,
        usize::try_from(visual_index - 1)
            .ok()
            .filter(|at| *at <= doc.items.len())
            .unwrap_or(doc.items.len()),
    );
    let fresh = with_command_defaults(
        doc,
        &commands,
        command,
        [
            None,
            None,
            None,
            None,
            Some(latitude),
            Some(longitude),
            None,
        ],
        defaults,
    );
    let land = commands.get(&command).is_some_and(|c| c.is_land);
    let takeoff = commands.get(&command).is_some_and(|c| c.is_takeoff);
    let inherited = previous_altitude(doc, &commands, visual_index)
        .filter(|_| fresh.altitude.is_some() && !land);
    let placed = match inherited {
        Some((previous, previous_mode)) => {
            let mode = match (takeoff, doc.global_altitude_mode) {
                (true, _) | (false, crate::altitudemodes::MIXED) => previous_mode,
                _ => fresh.altitude.as_ref().map_or(previous_mode, |a| a.mode),
            };
            let altitude = fresh
                .altitude
                .as_ref()
                .filter(|route| route.mode == mode)
                .map_or(previous, |route| route.altitude);
            Simple {
                frame: frame_for(mode),
                params: [
                    fresh.params[0],
                    fresh.params[1],
                    fresh.params[2],
                    fresh.params[3],
                    fresh.params[4],
                    fresh.params[5],
                    Some(altitude),
                ],
                altitude: Some(Altitude {
                    mode,
                    altitude,
                    amsl_above_terrain: None,
                }),
                ..fresh
            }
        }
        None => fresh,
    };
    let at = usize::try_from(visual_index - 1)
        .ok()
        .filter(|i| *i <= doc.items.len())
        .unwrap_or(doc.items.len());
    let placed = inserted_at(doc, at, Item::Simple(placed));
    Document {
        home: doc
            .home
            .or_else(|| home_from_first_coordinate(&placed, Some((latitude, longitude)))),
        ..placed
    }
}

pub fn commands_at(doc: &Document, at: usize) -> std::collections::BTreeMap<i64, cmdinfo::Command> {
    cmdinfo::tree(firmware(doc.firmware_type), command_class_at(doc, at))
}

pub fn past_takeoff(doc: &Document, kind: &str, visual_index: i64) -> i64 {
    let takeoff = doc
        .items
        .iter()
        .position(|item| !rides_along(item))
        .filter(|first_place| {
            matches!(&doc.items[*first_place], Item::Simple(simple) if commands_at(doc, *first_place).get(&simple.command).is_some_and(|c| c.is_takeoff))
        });
    match takeoff.map(|at| at as i64 + 1) {
        Some(takeoff) if kind != "takeoff" && (1..=takeoff).contains(&visual_index) => takeoff + 1,
        _ => visual_index,
    }
}

pub fn complex_count(kind: &str, item: &Value) -> Result<usize, String> {
    match TRANSECT_STYLE.contains(&kind) {
        true => item
            .get("TransectStyleComplexItem")
            .and_then(|t| t.get("Items"))
            .and_then(Value::as_array)
            .map(Vec::len)
            .ok_or_else(|| format!("The {kind} item has no saved mission items.")),
        false if kind == "StructureScan" => {
            crate::structurescan::saved_items(item).map(|items| items.len())
        }
        false if crate::landingpattern::is_landing(kind) => {
            crate::landingpattern::items(item, false).map(|items| items.len())
        }
        false => Err(format!("The core cannot hold a {kind} item yet.")),
    }
}

pub fn insert_complex(
    doc: &Document,
    kind: &str,
    json: Value,
    center: (f64, f64),
    visual_index: i64,
) -> Document {
    let item_count = complex_count(kind, &json).unwrap_or(0);
    let at = usize::try_from(visual_index - 1)
        .ok()
        .filter(|i| *i <= doc.items.len())
        .unwrap_or(doc.items.len());
    let placed = inserted_at(
        doc,
        at,
        Item::Complex {
            kind: kind.to_string(),
            json,
            item_count,
        },
    );
    let home = doc.home.or_else(|| {
        let (lat, lon) =
            crate::surveygrid::at_distance_and_azimuth(center, PLANNED_HOME_OFFSET_M, 0.0);
        Some([lat, lon, 0.0])
    });
    Document { home, ..placed }
}

const CMD_DO_SET_ROI: i64 = 201;
const MAV_ROI_LOCATION: f64 = 3.0;

pub fn insert_roi(
    doc: &Document,
    latitude: f64,
    longitude: f64,
    visual_index: i64,
    defaults: &EditDefaults,
) -> Document {
    let placed = insert_simple(
        doc,
        CMD_DO_SET_ROI_LOCATION,
        latitude,
        longitude,
        visual_index,
        defaults,
    );
    if firmware(doc.firmware_type) == Firmware::Px4 {
        return placed;
    }
    let at = usize::try_from(visual_index - 1)
        .ok()
        .filter(|i| *i <= doc.items.len())
        .unwrap_or(doc.items.len())
        + 1;
    set_command(&placed, at, CMD_DO_SET_ROI, defaults)
        .and_then(|changed| set_param(&changed, at, 1, MAV_ROI_LOCATION))
        .unwrap_or(placed)
}

pub fn insert_waypoint(
    doc: &Document,
    latitude: f64,
    longitude: f64,
    visual_index: i64,
    defaults: &EditDefaults,
) -> Document {
    insert_simple(
        doc,
        CMD_NAV_WAYPOINT,
        latitude,
        longitude,
        visual_index,
        defaults,
    )
}

pub fn insert_land(
    doc: &Document,
    latitude: f64,
    longitude: f64,
    visual_index: i64,
    defaults: &EditDefaults,
) -> Result<Document, String> {
    match vehicle_class(doc.vehicle_type) {
        VehicleClass::FixedWing | VehicleClass::Vtol => {
            Err("The core cannot build a landing pattern yet.".to_string())
        }
        _ => Ok(insert_simple(
            doc,
            CMD_NAV_RETURN_TO_LAUNCH,
            latitude,
            longitude,
            visual_index,
            defaults,
        )),
    }
}

const CMD_NAV_TAKEOFF: i64 = 22;
const CMD_NAV_VTOL_TAKEOFF: i64 = 84;

const CLIMB_OUT_DEGREES: f64 = 30.0;
const ABSOLUTE_CLIMB_OUT_FACTOR: f64 = 1.5;

pub fn climb_out_distance(altitude: f64, relative: bool, fallback: f64) -> f64 {
    match (relative, altitude) {
        (true, a) if a == 0.0 => fallback,
        (true, a) => a / CLIMB_OUT_DEGREES.to_radians().tan(),
        (false, a) => a * ABSOLUTE_CLIMB_OUT_FACTOR,
    }
}

pub fn insert_takeoff(
    doc: &Document,
    visual_index: i64,
    defaults: &EditDefaults,
) -> Result<Document, String> {
    let home = doc
        .home
        .ok_or("A takeoff is placed at the launch position, and this plan has none yet.")?;
    let class = vehicle_class(doc.vehicle_type);
    let relative = doc.global_altitude_mode != crate::altitudemodes::ABSOLUTE;
    let at_point = match class {
        VehicleClass::Vtol => crate::surveygrid::at_distance_and_azimuth(
            (home[0], home[1]),
            defaults.vtol_transition_distance,
            0.0,
        ),
        VehicleClass::FixedWing => crate::surveygrid::at_distance_and_azimuth(
            (home[0], home[1]),
            climb_out_distance(
                defaults.mission_item_altitude,
                relative,
                defaults.vtol_transition_distance,
            ),
            0.0,
        ),
        _ => (home[0], home[1]),
    };
    let command = if class == VehicleClass::Vtol {
        CMD_NAV_VTOL_TAKEOFF
    } else {
        CMD_NAV_TAKEOFF
    };
    let inserted = insert_simple(doc, command, at_point.0, at_point.1, visual_index, defaults);
    let at = usize::try_from(visual_index - 1)
        .ok()
        .filter(|i| *i <= doc.items.len())
        .unwrap_or(doc.items.len());
    let launched = |item: &Item| match item {
        Item::Simple(s) => Item::Simple(Simple {
            params: [
                s.params[0],
                s.params[1],
                s.params[2],
                s.params[3],
                Some(at_point.0),
                Some(at_point.1),
                s.params[6],
            ],
            ..s.clone()
        }),
        other => other.clone(),
    };
    Ok(Document {
        items: inserted
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if i == at {
                    launched(item)
                } else {
                    item.clone()
                }
            })
            .collect(),
        ..inserted
    })
}

pub fn set_launch(
    doc: &Document,
    visual_index: usize,
    latitude: f64,
    longitude: f64,
    same_location: bool,
    transition_distance: f64,
) -> Option<Document> {
    let (at, takeoff) = simple_at(doc, visual_index)?;
    let moved = Document {
        home: Some([latitude, longitude, doc.home.map_or(0.0, |h| h[2])]),
        ..doc.clone()
    };
    if takeoff.params[4].zip(takeoff.params[5]).is_some() {
        return Some(moved);
    }
    let distance = match vehicle_class(doc.vehicle_type) {
        VehicleClass::FixedWing => climb_out_distance(
            takeoff.params[6].unwrap_or(0.0),
            takeoff.frame == FRAME_GLOBAL_RELATIVE_ALT,
            transition_distance,
        ),
        _ => transition_distance,
    };
    let point = match same_location {
        true => (latitude, longitude),
        false => crate::surveygrid::at_distance_and_azimuth((latitude, longitude), distance, 0.0),
    };
    let params = [
        takeoff.params[0],
        takeoff.params[1],
        takeoff.params[2],
        takeoff.params[3],
        Some(point.0),
        Some(point.1),
        takeoff.params[6],
    ];
    Some(replaced(
        &moved,
        at,
        Simple {
            params,
            ..takeoff.clone()
        },
    ))
}

pub fn set_global_altitude_mode(doc: &Document, mode: i64) -> Document {
    Document {
        global_altitude_mode: mode,
        ..doc.clone()
    }
}

fn simple_at(doc: &Document, visual_index: usize) -> Option<(usize, &Simple)> {
    let at = visual_index.checked_sub(1)?;
    match doc.items.get(at)? {
        Item::Simple(simple) => Some((at, simple)),
        Item::Complex { .. } => None,
    }
}

fn replaced(doc: &Document, at: usize, item: Simple) -> Document {
    Document {
        items: doc
            .items
            .iter()
            .enumerate()
            .map(|(i, existing)| {
                if i == at {
                    Item::Simple(item.clone())
                } else {
                    existing.clone()
                }
            })
            .collect(),
        ..doc.clone()
    }
}

const VTOL_STATE_MC: i64 = 3;
const VTOL_STATE_FW: i64 = 4;

pub fn command_class_at(doc: &Document, at: usize) -> VehicleClass {
    let class = vehicle_class(doc.vehicle_type);
    if class != VehicleClass::Vtol {
        return class;
    }
    let is_rtl = |item: &Item| matches!(item, Item::Simple(s) if s.command == 20);
    if doc.items.iter().take(at + 1).any(is_rtl) {
        return class;
    }
    let before_rtl = doc.items.iter().position(is_rtl).unwrap_or(doc.items.len());
    let last_takeoff = doc.items[..before_rtl]
        .iter()
        .rev()
        .find_map(|item| match item {
            Item::Simple(s) if s.command == 22 || s.command == 84 => Some(s.command),
            _ => None,
        });
    let start = if last_takeoff == Some(84) {
        VehicleClass::MultiRotor
    } else {
        VehicleClass::FixedWing
    };
    doc.items
        .iter()
        .take(at)
        .fold(start, |mode, item| match item {
            Item::Simple(s) => match (s.command, s.params[0].map(|p| p as i64)) {
                (22 | 84 | 21, _) => VehicleClass::FixedWing,
                (85, _) => VehicleClass::MultiRotor,
                (3000, Some(VTOL_STATE_MC)) => VehicleClass::MultiRotor,
                (3000, Some(VTOL_STATE_FW)) => VehicleClass::FixedWing,
                _ => mode,
            },
            Item::Complex { .. } => mode,
        })
}

pub fn set_command(
    doc: &Document,
    visual_index: usize,
    command: i64,
    defaults: &EditDefaults,
) -> Option<Document> {
    let (at, current) = simple_at(doc, visual_index)?;
    let commands = commands_at(doc, at);
    match current.command == command {
        true => Some(doc.clone()),
        false => Some(replaced(
            doc,
            at,
            with_command_defaults(doc, &commands, command, current.params, defaults),
        )),
    }
}

pub fn command_at(
    doc: &Document,
    visual_index: usize,
    command: i64,
    defaults: &EditDefaults,
) -> Option<(Document, Option<usize>)> {
    let (at, current) = simple_at(doc, visual_index)?;
    let commands = commands_at(doc, at);
    let positioned = |id: i64| commands.get(&id).is_some_and(|c| c.specifies_coordinate);
    let lands = commands.get(&current.command).is_some_and(|c| c.is_land);
    let takes_off = commands.get(&current.command).is_some_and(|c| c.is_takeoff);
    match current.command != command
        && (positioned(current.command) || takes_off)
        && !positioned(command)
        && !lands
    {
        false => set_command(doc, visual_index, command, defaults).map(|changed| (changed, None)),
        true => {
            let added = visual_index
                + 1
                + doc.items[at + 1..]
                    .iter()
                    .take_while(|item| rides_along(item))
                    .count();
            Some((
                insert_simple(doc, command, 0.0, 0.0, added as i64, defaults),
                Some(added),
            ))
        }
    }
}

pub const FRAME_NEEDS_A_DATUM: &str =
    "This item has a height, so its frame is above sea level, relative to launch or above terrain.";

fn mode_of_frame(frame: i64) -> Option<i64> {
    match frame {
        FRAME_GLOBAL | FRAME_GLOBAL_INT => Some(crate::altitudemodes::ABSOLUTE),
        FRAME_GLOBAL_RELATIVE_ALT | FRAME_GLOBAL_RELATIVE_ALT_INT => {
            Some(crate::altitudemodes::RELATIVE)
        }
        FRAME_GLOBAL_TERRAIN_ALT | FRAME_GLOBAL_TERRAIN_ALT_INT => {
            Some(crate::altitudemodes::TERRAIN_FRAME)
        }
        _ => None,
    }
}

pub fn set_frame(doc: &Document, visual_index: usize, frame: i64) -> Result<Document, String> {
    let (at, current) =
        simple_at(doc, visual_index).ok_or_else(|| format!("Item {visual_index} has no frame."))?;
    let framed = |doc: &Document, item: &Simple| {
        replaced(
            doc,
            at,
            Simple {
                frame,
                ..item.clone()
            },
        )
    };
    match (current.altitude.is_some(), mode_of_frame(frame)) {
        (false, _) => Ok(framed(doc, current)),
        (true, None) => Err(FRAME_NEEDS_A_DATUM.to_string()),
        (true, Some(mode)) => set_altitude_mode(doc, visual_index, mode)
            .and_then(|moded| simple_at(&moded, visual_index).map(|(_, item)| framed(&moded, item)))
            .ok_or_else(|| format!("Item {visual_index} has no frame.")),
    }
}

pub fn set_altitude_mode(doc: &Document, visual_index: usize, mode: i64) -> Option<Document> {
    let (at, current) = simple_at(doc, visual_index)?;
    let held = current.altitude.as_ref()?;
    let frame = match mode {
        crate::altitudemodes::RELATIVE => FRAME_GLOBAL_RELATIVE_ALT,
        crate::altitudemodes::ABSOLUTE | crate::altitudemodes::CALC_ABOVE_TERRAIN => FRAME_GLOBAL,
        crate::altitudemodes::TERRAIN_FRAME => FRAME_GLOBAL_TERRAIN_ALT,
        _ => return None,
    };
    let calculated = mode == crate::altitudemodes::CALC_ABOVE_TERRAIN;
    let params: [Option<f64>; 7] = std::array::from_fn(|i| {
        if i == 6 {
            (!calculated).then_some(held.altitude)
        } else {
            current.params[i]
        }
    });
    let amsl_above_terrain = if calculated {
        None
    } else {
        held.amsl_above_terrain
    };
    Some(replaced(
        doc,
        at,
        Simple {
            frame,
            params,
            altitude: Some(Altitude {
                mode,
                amsl_above_terrain,
                ..held.clone()
            }),
            ..current.clone()
        },
    ))
}

fn flown_mode(mode: i64, firmware_type: i64) -> i64 {
    match firmware(firmware_type) == Firmware::Px4 && mode == crate::altitudemodes::TERRAIN_FRAME {
        true => crate::altitudemodes::CALC_ABOVE_TERRAIN,
        false => mode,
    }
}

pub fn retyped(doc: Document, firmware_type: i64, vehicle_type: i64) -> Document {
    Document {
        firmware_type,
        vehicle_type,
        global_altitude_mode: flown_mode(doc.global_altitude_mode, firmware_type),
        ..doc
    }
}

fn unflown_pattern(kind: &str, class: VehicleClass) -> Option<&'static str> {
    match kind {
        "StructureScan" if !matches!(class, VehicleClass::MultiRotor | VehicleClass::Vtol) => {
            Some("Structure Scan")
        }
        crate::landingpattern::FIXED_WING_PATTERN if class != VehicleClass::FixedWing => {
            Some("Landing Pattern")
        }
        crate::landingpattern::VTOL_PATTERN if class != VehicleClass::Vtol => {
            Some("Landing Pattern")
        }
        _ => None,
    }
}

fn unflown_items(doc: &Document, firmware_type: i64, vehicle_type: i64) -> Vec<String> {
    let class = vehicle_class(vehicle_type);
    let flown: Vec<i64> = crate::commandtree::offered(firmware(firmware_type), class)
        .iter()
        .map(|command| command.id)
        .collect();
    let names = cmdinfo::tree(firmware(doc.firmware_type), vehicle_class(doc.vehicle_type));
    let unflown = |seq: usize, simple: &Simple| {
        (!flown.contains(&simple.command)).then(|| {
            let name = names.get(&simple.command).map_or_else(
                || format!("command {}", simple.command),
                |c| c.friendly_name.clone(),
            );
            format!("item {seq} ({name})")
        })
    };
    let settings = doc
        .settings_sections
        .iter()
        .enumerate()
        .filter_map(|(i, simple)| unflown(i + 1, simple));
    let items = doc
        .items
        .iter()
        .zip(item_spans(doc))
        .flat_map(|(item, (seq, _))| match item {
            Item::Simple(simple) => std::iter::once(simple)
                .chain(&simple.sections)
                .enumerate()
                .filter_map(|(i, simple)| unflown(seq + i, simple))
                .collect::<Vec<_>>(),
            Item::Complex { kind, .. } => unflown_pattern(kind, class)
                .map(|name| format!("item {seq} ({name})"))
                .into_iter()
                .collect(),
        });
    settings.chain(items).collect()
}

fn without_unflown_camera_mode(doc: &Document, firmware_type: i64, vehicle_type: i64) -> Document {
    let flies_camera_mode =
        crate::commandtree::offered(firmware(firmware_type), vehicle_class(vehicle_type))
            .iter()
            .any(|command| command.id == CMD_SET_CAMERA_MODE);
    match camera_state(&doc.settings_sections).mode.is_some() && !flies_camera_mode {
        true => set_camera(doc, 0, "specifyCameraMode", &json!(false))
            .map(|unmoded| kept_jumps(doc, unmoded, &Some))
            .unwrap_or_else(|| doc.clone()),
        false => doc.clone(),
    }
}

pub fn for_vehicle(
    doc: &Document,
    firmware_type: i64,
    vehicle_type: i64,
) -> Result<Document, String> {
    let doc = &without_unflown_camera_mode(doc, firmware_type, vehicle_type);
    let unflown = unflown_items(doc, firmware_type, vehicle_type);
    if !unflown.is_empty() {
        return Err(format!(
            "The connected vehicle cannot fly {}. Remove {}, then convert the plan.",
            unflown.join(", "),
            if unflown.len() == 1 { "it" } else { "them" }
        ));
    }
    (1..=doc.items.len()).try_fold(
        retyped(doc.clone(), firmware_type, vehicle_type),
        |current, at| {
            let held_mode = simple_at(&current, at)
                .and_then(|(_, simple)| simple.altitude.as_ref())
                .map(|held| held.mode);
            match held_mode.filter(|mode| flown_mode(*mode, firmware_type) != *mode) {
                Some(mode) => set_altitude_mode(&current, at, flown_mode(mode, firmware_type))
                    .ok_or_else(|| format!("Item {at} could not leave the terrain frame.")),
                None => Ok(current),
            }
        },
    )
}

fn above_ground(mode: i64) -> bool {
    mode == crate::altitudemodes::CALC_ABOVE_TERRAIN || mode == crate::altitudemodes::TERRAIN_FRAME
}

fn altitude_datum(mode: i64, launch: f64, ground: &dyn Fn() -> Option<f64>) -> Option<f64> {
    match mode {
        crate::altitudemodes::RELATIVE => Some(launch),
        crate::altitudemodes::ABSOLUTE => Some(0.0),
        _ if above_ground(mode) => ground(),
        _ => None,
    }
}

pub const GROUND_UNKNOWN: &str = "The ground height under this item is not known yet, so its altitude cannot be kept in that mode. Try again when the terrain has loaded.";
pub const LAUNCH_GROUND_UNKNOWN: &str = "The ground height at the launch position is not known yet, so the altitude cannot be kept in that mode. Try again when the terrain has loaded.";

pub fn altitude_keeping_height(
    doc: &Document,
    visual_index: usize,
    mode: i64,
    launch: Option<f64>,
    terrain: &dyn Fn(f64, f64) -> Option<f64>,
) -> Result<f64, String> {
    let refused = || format!("Item {visual_index} cannot take that altitude mode.");
    let (_, current) = simple_at(doc, visual_index).ok_or_else(refused)?;
    let held = current.altitude.as_ref().ok_or_else(refused)?;
    if !(crate::altitudemodes::RELATIVE..=crate::altitudemodes::TERRAIN_FRAME).contains(&mode) {
        return Err(refused());
    }
    if held.mode == mode || (above_ground(held.mode) && above_ground(mode)) {
        return Ok(held.altitude);
    }
    let launch = match (doc.home, launch.filter(|it| it.is_finite())) {
        (None, _) => Err("Set the launch position first, so the altitude can be kept.".to_string()),
        (Some(_), None) => Err(LAUNCH_GROUND_UNKNOWN.to_string()),
        (Some(_), Some(launch)) => Ok(launch),
    }?;
    let ground = || {
        current.params[4]
            .zip(current.params[5])
            .and_then(|(latitude, longitude)| terrain(latitude, longitude))
            .filter(|it| it.is_finite())
    };
    match (
        altitude_datum(held.mode, launch, &ground),
        altitude_datum(mode, launch, &ground),
    ) {
        (Some(from), Some(to)) => Ok(held.altitude + from - to),
        _ => Err(GROUND_UNKNOWN.to_string()),
    }
}

pub fn set_altitude_mode_keeping_height(
    doc: &Document,
    visual_index: usize,
    mode: i64,
    launch: Option<f64>,
    terrain: &dyn Fn(f64, f64) -> Option<f64>,
) -> Result<Document, String> {
    let altitude = altitude_keeping_height(doc, visual_index, mode, launch, terrain)?;
    set_altitude(doc, visual_index, altitude)
        .and_then(|kept| set_altitude_mode(&kept, visual_index, mode))
        .ok_or_else(|| format!("Item {visual_index} cannot take that altitude mode."))
}

pub fn above_terrain(doc: &Document, terrain: &dyn Fn(f64, f64) -> Option<f64>) -> Document {
    let items = doc
        .items
        .iter()
        .map(|item| match item {
            Item::Simple(simple) => Item::Simple(simple_above_terrain(simple, terrain)),
            other => other.clone(),
        })
        .collect();
    Document {
        items,
        ..doc.clone()
    }
}

fn simple_above_terrain(simple: &Simple, terrain: &dyn Fn(f64, f64) -> Option<f64>) -> Simple {
    match (&simple.altitude, simple.params[4], simple.params[5]) {
        (Some(held), Some(latitude), Some(longitude))
            if held.mode == crate::altitudemodes::CALC_ABOVE_TERRAIN =>
        {
            let amsl = terrain(latitude, longitude)
                .filter(|ground| ground.is_finite())
                .map(|ground| ground + held.altitude);
            let params: [Option<f64>; 7] =
                std::array::from_fn(|i| if i == 6 { amsl } else { simple.params[i] });
            Simple {
                params,
                altitude: Some(Altitude {
                    amsl_above_terrain: amsl,
                    ..held.clone()
                }),
                ..simple.clone()
            }
        }
        _ => simple.clone(),
    }
}

pub fn awaiting_terrain(doc: &Document) -> bool {
    doc.items.iter().any(|item| matches!(item, Item::Simple(Simple { altitude: Some(held), .. }) if held.mode == crate::altitudemodes::CALC_ABOVE_TERRAIN && !held.amsl_above_terrain.is_some_and(f64::is_finite)))
}

pub fn servo_drop_bay(item: &Item, servos: &[crate::payloadcheck::Servo]) -> Option<i64> {
    match item {
        Item::Simple(s) => crate::payloadcheck::servo_drop_bay(
            s.command,
            s.params[0].unwrap_or(f64::NAN),
            s.params[1].unwrap_or(f64::NAN),
            servos,
        ),
        _ => None,
    }
}

pub fn is_drop_at(doc: &Document, at: usize, servos: &[crate::payloadcheck::Servo]) -> bool {
    doc.items
        .get(at)
        .is_some_and(|item| match servo_drop_bay(item, servos) {
            Some(bay) => !doc.items[..at]
                .iter()
                .any(|before| servo_drop_bay(before, servos) == Some(bay)),
            None => matches!(item, Item::Simple(s) if crate::payloadcheck::drops(s.command, s.params[0].unwrap_or(f64::NAN), s.params[1].unwrap_or(f64::NAN), servos)),
        })
}

pub fn free_bay(doc: &Document, servos: &[crate::payloadcheck::Servo]) -> Option<i64> {
    let used: Vec<i64> = doc
        .items
        .iter()
        .filter_map(|item| servo_drop_bay(item, servos))
        .collect();
    (1..=servos.len() as i64).find(|bay| !used.contains(bay))
}

const CMD_DO_LAND_START: i64 = 189;
const CMD_DO_VTOL_TRANSITION: i64 = 3000;
const CMD_JUMP_TAG: i64 = 600;
const CMD_DO_JUMP_TAG: i64 = 601;
const KEEPS_ITS_OWN_PLACE: [i64; 6] = [
    CMD_NAV_RETURN_TO_LAUNCH,
    CMD_DO_JUMP,
    CMD_DO_LAND_START,
    CMD_DO_VTOL_TRANSITION,
    CMD_JUMP_TAG,
    CMD_DO_JUMP_TAG,
];

pub fn rides_along(item: &Item) -> bool {
    matches!(item, Item::Simple(s) if s.altitude.is_none() && !KEEPS_ITS_OWN_PLACE.contains(&s.command))
}

pub fn riders_after(doc: &Document, at: usize) -> usize {
    doc.items
        .get(at + 1..)
        .unwrap_or_default()
        .iter()
        .take_while(|item| rides_along(item))
        .count()
}

pub fn drop_after(
    doc: &Document,
    visual_index: usize,
    servos: &[crate::payloadcheck::Servo],
) -> Option<Option<usize>> {
    let at = visual_index.checked_sub(1)?;
    matches!(doc.items.get(at)?, Item::Simple(s) if s.command == CMD_NAV_WAYPOINT).then_some(())?;
    Some(
        (at + 1..at + 1 + riders_after(doc, at))
            .find(|rider| is_drop_at(doc, *rider, servos))
            .map(|rider| rider + 1),
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropRelease {
    pub output: f64,
    pub pwm: f64,
    pub bay: Option<i64>,
}

pub fn drop_release(
    doc: &Document,
    drop_visual: usize,
    servos: &[crate::payloadcheck::Servo],
) -> Option<DropRelease> {
    let (_, drop) = simple_at(doc, drop_visual)?;
    (drop.command == i64::from(crate::payloadcheck::CMD_DO_SET_SERVO)).then_some(())?;
    let (output, pwm) = drop.params[0].zip(drop.params[1])?;
    Some(DropRelease {
        output,
        pwm,
        bay: crate::payloadcheck::servo_drop_bay(drop.command, output, pwm, servos),
    })
}

const DROP_NEEDS_A_WAYPOINT: &str = "Add a waypoint before the drop. A drop falls where the drone is when it reaches the item before it, so after takeoff it would fall over the launch point.";
const EVERY_BAY_DROPS: &str = "Every bay already drops in this mission. Add a bay in Settings, Fly View, Payload, or remove a drop.";
pub const NOT_A_WAYPOINT: &str = "A drop belongs to a waypoint. Pick the waypoint it falls at.";
pub const NO_DROP_HERE: &str = "This waypoint has no drop.";
pub const WAIT_NEEDS_A_HOVER: &str =
    "Only a multirotor can wait over the point before it drops. A plane flies through its drop.";
pub const WAIT_RANGE_SECONDS: std::ops::RangeInclusive<f64> = 0.0..=600.0;
pub const WAIT_OUT_OF_RANGE: &str = "The wait before a drop is from 0 to 600 seconds.";
pub const WAIT_TOO_SHORT: &str = "A wait before a drop is at least 1 second, or none. The drone needs that long to finish the camera actions before it drops.";
pub const NOT_A_SERVO_DROP: &str =
    "This drop opens the gripper, which has no servo output or PWM of its own.";

pub fn insert_drop(
    doc: &Document,
    visual_index: i64,
    defaults: &EditDefaults,
    servos: &[crate::payloadcheck::Servo],
) -> Result<Document, String> {
    let at = usize::try_from(visual_index - 1)
        .ok()
        .filter(|i| *i <= doc.items.len())
        .unwrap_or(doc.items.len());
    if !drop_has_a_place(doc, at) {
        return Err(DROP_NEEDS_A_WAYPOINT.to_string());
    }
    let dropped = match servos.is_empty() {
        true => insert_simple(
            doc,
            i64::from(crate::payloadcheck::CMD_DO_GRIPPER),
            0.0,
            0.0,
            visual_index,
            defaults,
        ),
        false => {
            let servo = free_bay(doc, servos)
                .and_then(|bay| servos.get(usize::try_from(bay - 1).ok()?))
                .ok_or_else(|| EVERY_BAY_DROPS.to_string())?;
            let placed = insert_simple(
                doc,
                i64::from(crate::payloadcheck::CMD_DO_SET_SERVO),
                0.0,
                0.0,
                visual_index,
                defaults,
            );
            set_param(&placed, at + 1, 1, servo.output as f64)
                .and_then(|output| set_param(&output, at + 1, 2, servo.release))
                .ok_or_else(|| "The drop could not be given its servo.".to_string())?
        }
    };
    Ok(hold_before_drop(
        &dropped,
        at,
        crate::payloadcheck::DROP_HOLD_SECONDS,
    ))
}

const CMD_NAV_DELAY: i64 = 93;

fn is_delay(doc: &Document, visual_index: usize) -> bool {
    matches!(doc.items.get(visual_index.wrapping_sub(1)), Some(Item::Simple(s)) if s.command == CMD_NAV_DELAY)
}

fn waits_before(doc: &Document, waypoint_visual: usize, drop_visual: usize) -> bool {
    drop_visual > waypoint_visual + 1 && is_delay(doc, drop_visual - 1)
}

pub fn drop_wait(doc: &Document, waypoint_visual: usize, drop_visual: usize) -> f64 {
    match waits_before(doc, waypoint_visual, drop_visual) {
        true => simple_at(doc, drop_visual - 1)
            .and_then(|(_, delay)| delay.params[0])
            .filter(|seconds| *seconds > 0.0)
            .unwrap_or(0.0),
        false => 0.0,
    }
}

const CMD_NAV_LAST: i64 = 95;
const NAV_BEYOND_LAST: [i64; 3] = [213, 42702, 42703];
pub const SHORTEST_DELAY_SECONDS: f64 = 1.0;
const TIME_OF_DAY_UNUSED: f64 = -1.0;

fn is_nav(command: i64) -> bool {
    command <= CMD_NAV_LAST || NAV_BEYOND_LAST.contains(&command)
}

fn flown_commands_of(item: &Item) -> Vec<i64> {
    match item {
        Item::Simple(simple) => std::iter::once(simple.command)
            .chain(simple.sections.iter().map(|section| section.command))
            .collect(),
        Item::Complex { .. } => vec![CMD_NAV_WAYPOINT],
    }
}

fn queued_before(doc: &Document, at: usize) -> usize {
    doc.items[..at]
        .iter()
        .rev()
        .flat_map(|item| flown_commands_of(item).into_iter().rev())
        .take_while(|command| !is_nav(*command))
        .count()
}

fn runs_out_at_once(delay: &Simple) -> bool {
    delay.command == CMD_NAV_DELAY
        && delay.params[0].is_some_and(|seconds| seconds > 0.0 && seconds < SHORTEST_DELAY_SECONDS)
}

fn closing_delay() -> Item {
    Item::Simple(mission_command(
        CMD_NAV_DELAY,
        [
            SHORTEST_DELAY_SECONDS,
            TIME_OF_DAY_UNUSED,
            TIME_OF_DAY_UNUSED,
            TIME_OF_DAY_UNUSED,
            0.0,
            0.0,
            0.0,
        ],
    ))
}

pub fn as_flown(doc: &Document) -> Document {
    if firmware(doc.firmware_type) != Firmware::ArduPilot {
        return doc.clone();
    }
    let items: Vec<Item> = doc
        .items
        .iter()
        .enumerate()
        .map(|(at, item)| match item {
            Item::Simple(delay) if runs_out_at_once(delay) && queued_before(doc, at) > 1 => {
                Item::Simple(Simple {
                    params: std::array::from_fn(|i| match i {
                        0 => Some(SHORTEST_DELAY_SECONDS),
                        _ => delay.params[i],
                    }),
                    ..delay.clone()
                })
            }
            _ => item.clone(),
        })
        .collect();
    let left_at_the_end = queued_before(doc, doc.items.len());
    let flies = doc.items.iter().flat_map(flown_commands_of).any(is_nav);
    Document {
        items: items
            .into_iter()
            .chain((flies && left_at_the_end > 1).then(closing_delay))
            .collect(),
        ..doc.clone()
    }
}

pub fn wait_offered(doc: &Document) -> bool {
    vehicle_class(doc.vehicle_type) == VehicleClass::MultiRotor
}

fn found_drop(
    doc: &Document,
    waypoint_visual: usize,
    servos: &[crate::payloadcheck::Servo],
) -> Result<Option<usize>, String> {
    drop_after(doc, waypoint_visual, servos).ok_or_else(|| NOT_A_WAYPOINT.to_string())
}

pub fn remove_drop(
    doc: &Document,
    waypoint_visual: usize,
    servos: &[crate::payloadcheck::Servo],
) -> Result<Document, String> {
    let drop_visual =
        found_drop(doc, waypoint_visual, servos)?.ok_or_else(|| NO_DROP_HERE.to_string())?;
    let without = remove(doc, drop_visual)?;
    match waits_before(doc, waypoint_visual, drop_visual) {
        true => remove(&without, drop_visual - 1),
        false => Ok(without),
    }
}

fn with_wait(
    doc: &Document,
    waypoint_visual: usize,
    drop_visual: usize,
    wait: f64,
    defaults: &EditDefaults,
) -> Result<Document, String> {
    if !WAIT_RANGE_SECONDS.contains(&wait) {
        return Err(WAIT_OUT_OF_RANGE.to_string());
    }
    if wait > 0.0 && !wait_offered(doc) {
        return Err(WAIT_NEEDS_A_HOVER.to_string());
    }
    if wait > 0.0 && wait < SHORTEST_DELAY_SECONDS {
        return Err(WAIT_TOO_SHORT.to_string());
    }
    let timed = |doc: &Document, delay_visual: usize| {
        set_param(doc, delay_visual, 1, wait)
            .ok_or_else(|| "The wait before the drop could not be set.".to_string())
    };
    match (waits_before(doc, waypoint_visual, drop_visual), wait > 0.0) {
        (true, true) => timed(doc, drop_visual - 1),
        (true, false) => remove(doc, drop_visual - 1),
        (false, true) => timed(
            &insert_simple(doc, CMD_NAV_DELAY, 0.0, 0.0, drop_visual as i64, defaults),
            drop_visual,
        ),
        (false, false) => Ok(doc.clone()),
    }
}

pub fn place_drop(
    doc: &Document,
    waypoint_visual: usize,
    wait: Option<f64>,
    defaults: &EditDefaults,
    servos: &[crate::payloadcheck::Servo],
) -> Result<Document, String> {
    let placed = match found_drop(doc, waypoint_visual, servos)? {
        Some(_) => doc.clone(),
        None => insert_drop(doc, waypoint_visual as i64 + 1, defaults, servos)?,
    };
    let drop_visual = found_drop(&placed, waypoint_visual, servos)?
        .ok_or_else(|| "The drop was not added at this waypoint.".to_string())?;
    match wait {
        Some(wait) => with_wait(&placed, waypoint_visual, drop_visual, wait, defaults),
        None => Ok(placed),
    }
}

pub fn set_drop_servo(
    doc: &Document,
    waypoint_visual: usize,
    output: Option<f64>,
    pwm: Option<f64>,
    servos: &[crate::payloadcheck::Servo],
) -> Result<Document, String> {
    let drop_visual =
        found_drop(doc, waypoint_visual, servos)?.ok_or_else(|| NO_DROP_HERE.to_string())?;
    let release =
        drop_release(doc, drop_visual, servos).ok_or_else(|| NOT_A_SERVO_DROP.to_string())?;
    crate::payloadcheck::servo_release(output.unwrap_or(release.output), pwm.unwrap_or(release.pwm))
        .and_then(|(output, pwm)| {
            set_param(doc, drop_visual, 1, output)
                .and_then(|moved| set_param(&moved, drop_visual, 2, pwm))
                .ok_or_else(|| NO_DROP_HERE.to_string())
        })
}

pub fn replace_drop(
    doc: &Document,
    waypoint_visual: usize,
    defaults: &EditDefaults,
    was: &[crate::payloadcheck::Servo],
    servos: &[crate::payloadcheck::Servo],
) -> Result<Document, String> {
    let drop_visual =
        found_drop(doc, waypoint_visual, was)?.ok_or_else(|| NO_DROP_HERE.to_string())?;
    let wait = drop_wait(doc, waypoint_visual, drop_visual);
    place_drop(
        &remove_drop(doc, waypoint_visual, was)?,
        waypoint_visual,
        Some(wait),
        defaults,
        servos,
    )
}

pub fn drop_has_a_place(doc: &Document, drop_at: usize) -> bool {
    let last_place = doc
        .items
        .get(..drop_at)
        .unwrap_or(&doc.items)
        .iter()
        .rposition(|item| !matches!(item, Item::Simple(s) if s.altitude.is_none()));
    last_place.is_some_and(|at| !matches!(&doc.items[at], Item::Simple(s) if s.command == CMD_NAV_TAKEOFF || s.command == CMD_NAV_VTOL_TAKEOFF))
}

pub fn hold_before_drop(doc: &Document, drop_at: usize, seconds: f64) -> Document {
    let last_place = doc
        .items
        .get(..drop_at)
        .unwrap_or(&doc.items)
        .iter()
        .rposition(|item| !matches!(item, Item::Simple(s) if s.altitude.is_none()));
    let waypoint = last_place.filter(|at| matches!(&doc.items[*at], Item::Simple(s) if s.command == CMD_NAV_WAYPOINT && s.params[0].unwrap_or(0.0) == 0.0));
    match (vehicle_class(doc.vehicle_type), waypoint) {
        (VehicleClass::MultiRotor, Some(at)) => {
            set_param(doc, at + 1, 1, seconds).unwrap_or_else(|| doc.clone())
        }
        _ => doc.clone(),
    }
}

pub fn set_param(
    doc: &Document,
    visual_index: usize,
    param: usize,
    value: f64,
) -> Option<Document> {
    let (at, current) = simple_at(doc, visual_index)?;
    let slot = param.checked_sub(1).filter(|p| *p < 7)?;
    let params: [Option<f64>; 7] = std::array::from_fn(|i| {
        if i == slot {
            Some(value)
        } else {
            current.params[i]
        }
    });
    let altitude = match slot {
        6 => current.altitude.as_ref().map(|a| Altitude {
            altitude: value,
            ..a.clone()
        }),
        _ => current.altitude.clone(),
    };
    Some(replaced(
        doc,
        at,
        Simple {
            params,
            altitude,
            ..current.clone()
        },
    ))
}

const LOITER_COMMANDS: [i64; 4] = [17, 18, 19, 31];
const CMD_NAV_LOITER_TO_ALT: i64 = 31;

pub fn clear_param(doc: &Document, visual_index: usize, param: usize) -> Option<Document> {
    let (at, current) = simple_at(doc, visual_index)?;
    let slot = param.checked_sub(1).filter(|p| *p < 4)?;
    Some(replaced(
        doc,
        at,
        Simple {
            params: std::array::from_fn(|i| match i == slot {
                true => None,
                false => current.params[i],
            }),
            ..current.clone()
        },
    ))
}

pub fn set_loiter_radius(doc: &Document, visual_index: usize, radius: f64) -> Option<Document> {
    let (_, current) = simple_at(doc, visual_index)?;
    LOITER_COMMANDS.contains(&current.command).then_some(())?;
    set_param(
        doc,
        visual_index,
        if current.command == CMD_NAV_LOITER_TO_ALT {
            2
        } else {
            3
        },
        radius,
    )
}

fn speed_change(class: VehicleClass, speed: f64) -> Simple {
    let ground = match class {
        VehicleClass::MultiRotor => 1.0,
        _ => 0.0,
    };
    Simple {
        command: CMD_DO_CHANGE_SPEED,
        frame: FRAME_MISSION,
        params: [
            Some(ground),
            Some(speed),
            Some(-1.0),
            Some(0.0),
            Some(0.0),
            Some(0.0),
            Some(0.0),
        ],
        auto_continue: true,
        altitude: None,
        sections: Vec::new(),
    }
}

fn with_speed(sections: &[Simple], class: VehicleClass, speed: Option<f64>) -> Vec<Simple> {
    let others = sections
        .iter()
        .filter(|s| s.command != CMD_DO_CHANGE_SPEED)
        .cloned();
    others
        .chain(speed.map(|v| speed_change(class, v)))
        .collect()
}

pub fn set_speed(doc: &Document, visual_index: usize, speed: Option<f64>) -> Option<Document> {
    let class = vehicle_class(doc.vehicle_type);
    match visual_index {
        0 => Some(Document {
            settings_sections: with_speed(&doc.settings_sections, class, speed),
            ..doc.clone()
        }),
        _ => {
            let (at, current) = simple_at(doc, visual_index)?;
            (current.command == CMD_NAV_WAYPOINT).then(|| {
                replaced(
                    doc,
                    at,
                    Simple {
                        sections: with_speed(&current.sections, class, speed),
                        ..current.clone()
                    },
                )
            })
        }
    }
}

pub fn specified_speed(doc: &Document, visual_index: usize) -> Option<f64> {
    let sections = match visual_index {
        0 => &doc.settings_sections,
        _ => &simple_at(doc, visual_index)?.1.sections,
    };
    sections
        .iter()
        .find(|s| s.command == CMD_DO_CHANGE_SPEED)
        .and_then(|s| s.params[1])
}

pub fn set_altitude(doc: &Document, visual_index: usize, altitude: f64) -> Option<Document> {
    let (at, current) = simple_at(doc, visual_index)?;
    let held = current.altitude.as_ref()?;
    let params = [
        current.params[0],
        current.params[1],
        current.params[2],
        current.params[3],
        current.params[4],
        current.params[5],
        Some(altitude),
    ];
    Some(replaced(
        doc,
        at,
        Simple {
            params,
            altitude: Some(Altitude {
                altitude,
                ..held.clone()
            }),
            ..current.clone()
        },
    ))
}

pub const ORDER_PINNED: &str = "The takeoff, landing and return keep their places in the mission.";

fn pins_order(doc: &Document, item: &Item) -> bool {
    match item {
        Item::Simple(simple) => {
            simple.command == CMD_NAV_RETURN_TO_LAUNCH
                || cmdinfo::tree(firmware(doc.firmware_type), vehicle_class(doc.vehicle_type))
                    .get(&simple.command)
                    .is_some_and(|command| command.is_takeoff || command.is_land)
        }
        Item::Complex { kind, .. } => crate::landingpattern::is_landing(kind),
    }
}

fn group_at(doc: &Document, at: usize) -> std::ops::Range<usize> {
    match rides_along(&doc.items[at]) {
        true => at..at + 1,
        false => at..at + 1 + riders_after(doc, at),
    }
}

fn group_before(doc: &Document, at: usize) -> Option<std::ops::Range<usize>> {
    let before = at.checked_sub(1)?;
    let head = match rides_along(&doc.items[at]) {
        true => before,
        false => (0..=before)
            .rev()
            .find(|i| !rides_along(&doc.items[*i]))
            .unwrap_or(before),
    };
    Some(head..at)
}

pub fn move_item(
    doc: &Document,
    visual_index: usize,
    later: bool,
) -> Result<(Document, usize), String> {
    let at = visual_index
        .checked_sub(1)
        .filter(|at| *at < doc.items.len())
        .ok_or_else(|| format!("This plan has no item {visual_index} to move."))?;
    let moving = group_at(doc, at);
    let passed =
        match later {
            true => Some(moving.end)
                .filter(|next| *next < doc.items.len())
                .map(|next| match rides_along(&doc.items[at]) {
                    true => next..next + 1,
                    false => group_at(doc, next),
                }),
            false => group_before(doc, at),
        }
        .ok_or_else(|| "The item is already at that end of the mission.".to_string())?;
    if pins_order(doc, &doc.items[at]) || pins_order(doc, &doc.items[passed.start]) {
        return Err(ORDER_PINNED.to_string());
    }
    let (first, second) = match later {
        true => (passed.clone(), moving.clone()),
        false => (moving.clone(), passed.clone()),
    };
    let start = moving.start.min(passed.start);
    let end = moving.end.max(passed.end);
    let order: Vec<usize> = (0..start)
        .chain(first)
        .chain(second)
        .chain(end..doc.items.len())
        .collect();
    let landed = order.iter().position(|from| *from == at).unwrap_or(at);
    Ok((
        kept_jumps(
            doc,
            Document {
                items: order.iter().map(|from| doc.items[*from].clone()).collect(),
                ..doc.clone()
            },
            &|from| order.iter().position(|it| *it == from),
        ),
        landed + 1,
    ))
}

pub fn remove_with_riders(doc: &Document, visual_index: usize) -> Result<Document, String> {
    let at = visual_index
        .checked_sub(1)
        .filter(|i| *i < doc.items.len())
        .ok_or_else(|| format!("This plan has no item {visual_index} to remove."))?;
    group_at(doc, at)
        .rev()
        .try_fold(doc.clone(), |left, gone| remove(&left, gone + 1))
}

pub fn remove(doc: &Document, visual_index: usize) -> Result<Document, String> {
    let at = visual_index
        .checked_sub(1)
        .filter(|i| *i < doc.items.len())
        .ok_or_else(|| format!("This plan has no item {visual_index} to remove."))?;
    let spans = item_spans(doc);
    let jumped_from = doc.items.iter().enumerate().find(|(from, item)| {
        *from != at && jump_target(&spans, jump_seq(item)).is_some_and(|(target, _)| target == at)
    });
    if let Some((from, _)) = jumped_from {
        return Err(format!(
            "The jump at item {} goes to item {}. Change or remove that jump first.",
            spans[from].0, spans[at].0
        ));
    }
    Ok(kept_jumps(
        doc,
        Document {
            items: doc
                .items
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != at)
                .map(|(_, item)| item.clone())
                .collect(),
            ..doc.clone()
        },
        &|from| match from {
            from if from < at => Some(from),
            from if from == at => None,
            from => Some(from - 1),
        },
    ))
}

pub fn save(doc: &Document) -> Value {
    encode(
        doc,
        &[
            crate::landingpattern::WIZARD,
            crate::structurescan::ENTRY_VERTEX,
        ],
    )
}

pub fn mission(doc: &Document) -> Value {
    encode(doc, &[crate::landingpattern::WIZARD])
}

fn encode(doc: &Document, unsaved: &[&str]) -> Value {
    let section = |s: &Simple| {
        Item::Simple(Simple {
            frame: FRAME_MISSION,
            ..s.clone()
        })
    };
    let settings = doc.settings_sections.iter().map(section);
    let spans: Vec<(Item, usize)> = settings
        .chain(doc.items.iter().flat_map(|item| {
            match item {
                Item::Simple(simple) => std::iter::once(Item::Simple(Simple {
                    sections: Vec::new(),
                    ..simple.clone()
                }))
                .chain(simple.sections.iter().map(section))
                .collect::<Vec<_>>(),
                complex => vec![complex.clone()],
            }
        }))
        .map(|item| {
            let span = match &item {
                Item::Simple(_) => 1,
                Item::Complex { item_count, .. } => *item_count,
            };
            (item, span)
        })
        .collect();
    let starts = spans.iter().scan(1usize, |next, (_, span)| {
        let start = *next;
        *next += span;
        Some(start)
    });
    let items: Vec<Value> = spans
        .iter()
        .zip(starts)
        .map(|((item, _), seq)| save_item(item, seq, unsaved))
        .collect();
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

const CMD_DO_CHANGE_SPEED: i64 = 178;
const CMD_DO_MOUNT_CONTROL: i64 = 205;
const CMD_DO_SET_CAM_TRIGG_DIST: i64 = 206;
const CMD_SET_CAMERA_MODE: i64 = 530;
const CMD_IMAGE_START_CAPTURE: i64 = 2000;
const CMD_IMAGE_STOP_CAPTURE: i64 = 2001;
const CMD_VIDEO_START_CAPTURE: i64 = 2500;
const CMD_VIDEO_STOP_CAPTURE: i64 = 2501;
const MOUNT_MODE_MAVLINK_TARGETING: f64 = 2.0;
const VIDEO_CAPTURE_STATUS_INTERVAL: f64 = 0.2;

fn p(item: &Simple, i: usize) -> f64 {
    item.params[i].unwrap_or(f64::NAN)
}

fn zero(item: &Simple, from: usize) -> bool {
    (from..7).all(|i| p(item, i) == 0.0)
}

#[derive(Clone, Copy, PartialEq)]
enum Found {
    Gimbal,
    Action,
    Mode,
}

fn camera_match(rest: &[Simple], found: &[Found]) -> Option<(Found, usize)> {
    let item = rest.first()?;
    let next = rest.get(1);
    let not = |kind: Found| !found.contains(&kind);
    let gimbal = item.command == CMD_DO_MOUNT_CONTROL
        && p(item, 1) == 0.0
        && p(item, 3) == 0.0
        && p(item, 4) == 0.0
        && p(item, 5) == 0.0
        && p(item, 6) == MOUNT_MODE_MAVLINK_TARGETING;
    let photo = item.command == CMD_IMAGE_START_CAPTURE
        && p(item, 0) == 0.0
        && p(item, 1) == 0.0
        && p(item, 2) == 1.0;
    let interval = item.command == CMD_IMAGE_START_CAPTURE
        && p(item, 0) == 0.0
        && p(item, 1) >= 1.0
        && p(item, 2) == 0.0;
    let trigger_zero = item.command == CMD_DO_SET_CAM_TRIGG_DIST && zero(item, 0);
    let stop_photos =
        trigger_zero && next.is_some_and(|n| n.command == CMD_IMAGE_STOP_CAPTURE && p(n, 0) == 0.0);
    let trigger_start = item.command == CMD_DO_SET_CAM_TRIGG_DIST
        && p(item, 0) > 0.0
        && p(item, 1) == 0.0
        && p(item, 2) == 1.0
        && zero(item, 3);
    let video = item.command == CMD_VIDEO_START_CAPTURE
        && p(item, 0) == 0.0
        && p(item, 1) == VIDEO_CAPTURE_STATUS_INTERVAL;
    let stop_video = item.command == CMD_VIDEO_STOP_CAPTURE && p(item, 0) == 0.0;
    let mode = item.command == CMD_SET_CAMERA_MODE
        && p(item, 0) == 0.0
        && [0.0, 1.0, 2.0].contains(&p(item, 1))
        && p(item, 2).is_nan();
    match () {
        _ if not(Found::Gimbal) && gimbal => Some((Found::Gimbal, 1)),
        _ if not(Found::Action) && (photo || interval) => Some((Found::Action, 1)),
        _ if not(Found::Action) && stop_photos => Some((Found::Action, 2)),
        _ if not(Found::Action) && (trigger_start || trigger_zero || video || stop_video) => {
            Some((Found::Action, 1))
        }
        _ if not(Found::Mode) && mode => Some((Found::Mode, 1)),
        _ => None,
    }
}

const CAMERA_ACTIONS: [(&str, i64); 7] = [
    ("No change", 0),
    ("Take photo", 6),
    ("Take photos (time)", 1),
    ("Take photos (distance)", 2),
    ("Stop taking photos", 3),
    ("Start recording video", 4),
    ("Stop recording video", 5),
];

pub fn camera_action_value(index: usize) -> Option<i64> {
    CAMERA_ACTIONS.get(index).map(|(_, value)| *value)
}

pub fn camera_section(sections: &[Simple]) -> Value {
    let gimbal = sections.iter().find(|s| s.command == CMD_DO_MOUNT_CONTROL);
    let action = sections.iter().enumerate().find_map(|(i, item)| {
        let next = sections.get(i + 1);
        match item.command {
            CMD_IMAGE_START_CAPTURE if p(item, 2) == 1.0 => Some(6),
            CMD_IMAGE_START_CAPTURE => Some(1),
            CMD_DO_SET_CAM_TRIGG_DIST
                if zero(item, 0) && next.is_some_and(|n| n.command == CMD_IMAGE_STOP_CAPTURE) =>
            {
                Some(3)
            }
            CMD_DO_SET_CAM_TRIGG_DIST => Some(2),
            CMD_VIDEO_START_CAPTURE => Some(4),
            CMD_VIDEO_STOP_CAPTURE => Some(5),
            _ => None,
        }
    });
    let action = action.unwrap_or(0);
    let degrees = |property: &str, value: f64, (user_min, user_max): (f64, f64)| json!({ "property": property, "value": value, "valueString": format!("{value:.0}"), "enumOrValueString": format!("{value:.0}"), "units": "deg", "userMin": user_min, "userMax": user_max, "decimalPlaces": 0 });
    let chosen = CAMERA_ACTIONS
        .iter()
        .position(|(_, v)| *v == action)
        .unwrap_or(0);
    let state_mode = sections
        .iter()
        .find(|s| s.command == CMD_SET_CAMERA_MODE)
        .map(|m| p(m, 1));
    let start = sections
        .iter()
        .find(|s| s.command == CMD_IMAGE_START_CAPTURE)
        .map(|s| p(s, 1))
        .filter(|t| *t >= 1.0)
        .unwrap_or(DEFAULT_INTERVAL_TIME);
    let distance = trigger_distance(sections, action);
    let number = |property: &str, value: f64, units: &str| json!({ "property": property, "value": value, "valueString": format!("{value}"), "enumOrValueString": format!("{value}"), "units": units });
    let modes = ["Photo", "Video", "Survey"];
    let mode_value = state_mode.unwrap_or(0.0);
    json!({
        "kind": "object",
        "class": "CameraSection",
        "specifyGimbal": gimbal.is_some(),
        "specifyCameraMode": state_mode.is_some(),
        "facts": [
            number("cameraPhotoIntervalTime", start, "secs"),
            number("cameraPhotoIntervalDistance", distance, "m"),
            {
                "property": "cameraMode",
                "value": mode_value,
                "valueString": format!("{mode_value}"),
                "enumOrValueString": modes.get(mode_value as usize).copied().unwrap_or(""),
                "enumStrings": modes,
                "enumValues": [0, 1, 2],
                "enumIndex": mode_value as i64,
                "units": "",
            },
            degrees("gimbalPitch", gimbal.map_or(0.0, |g| p(g, 0)), GIMBAL_PITCH_USER_RANGE),
            degrees("gimbalYaw", gimbal.map_or(0.0, |g| p(g, 2)), GIMBAL_YAW_USER_RANGE),
            {
                "property": "cameraAction",
                "value": action,
                "valueString": action.to_string(),
                "enumOrValueString": CAMERA_ACTIONS[chosen].0,
                "enumStrings": CAMERA_ACTIONS.iter().map(|(label, _)| *label).collect::<Vec<_>>(),
                "enumValues": CAMERA_ACTIONS.iter().map(|(_, v)| *v).collect::<Vec<_>>(),
                "enumIndex": chosen,
                "units": "",
            },
        ],
    })
}

const GIMBAL_PITCH_USER_RANGE: (f64, f64) = (-90.0, 90.0);
const GIMBAL_YAW_USER_RANGE: (f64, f64) = (-180.0, 180.0);
const DEFAULT_INTERVAL_TIME: f64 = 10.0;
const DEFAULT_INTERVAL_DISTANCE: f64 = 1.0;
const CAMERA_COMMANDS: [i64; 7] = [
    CMD_SET_CAMERA_MODE,
    CMD_DO_MOUNT_CONTROL,
    CMD_IMAGE_START_CAPTURE,
    CMD_IMAGE_STOP_CAPTURE,
    CMD_DO_SET_CAM_TRIGG_DIST,
    CMD_VIDEO_START_CAPTURE,
    CMD_VIDEO_STOP_CAPTURE,
];

#[derive(Clone, Debug, PartialEq)]
pub struct CameraState {
    pub mode: Option<f64>,
    pub gimbal: Option<(f64, f64)>,
    pub action: i64,
    pub interval_time: f64,
    pub interval_distance: f64,
}

fn trigger_distance(sections: &[Simple], action: i64) -> f64 {
    sections
        .iter()
        .find(|s| s.command == CMD_DO_SET_CAM_TRIGG_DIST)
        .filter(|_| action == 2)
        .map_or(DEFAULT_INTERVAL_DISTANCE, |t| p(t, 0))
}

pub fn camera_state(sections: &[Simple]) -> CameraState {
    let found = |command: i64| sections.iter().find(|s| s.command == command);
    let start = found(CMD_IMAGE_START_CAPTURE);
    let action = camera_section(sections)["facts"]
        .as_array()
        .and_then(|facts| facts.iter().find(|f| f["property"] == "cameraAction"))
        .and_then(|f| f["value"].as_i64())
        .unwrap_or(0);
    CameraState {
        mode: found(CMD_SET_CAMERA_MODE).map(|m| p(m, 1)),
        gimbal: found(CMD_DO_MOUNT_CONTROL).map(|g| (p(g, 0), p(g, 2))),
        action,
        interval_time: start
            .map(|s| p(s, 1))
            .filter(|t| *t >= 1.0)
            .unwrap_or(DEFAULT_INTERVAL_TIME),
        interval_distance: trigger_distance(sections, action),
    }
}

fn mission_command(command: i64, params: [f64; 7]) -> Simple {
    Simple {
        command,
        frame: FRAME_MISSION,
        params: params.map(Some),
        auto_continue: true,
        altitude: None,
        sections: Vec::new(),
    }
}

pub fn camera_items(state: &CameraState) -> Vec<Simple> {
    let nan = f64::NAN;
    let mode = state
        .mode
        .map(|m| mission_command(CMD_SET_CAMERA_MODE, [0.0, m, nan, nan, nan, nan, nan]));
    let gimbal = state.gimbal.map(|(pitch, yaw)| {
        mission_command(
            CMD_DO_MOUNT_CONTROL,
            [pitch, 0.0, yaw, 0.0, 0.0, 0.0, MOUNT_MODE_MAVLINK_TARGETING],
        )
    });
    let action: Vec<Simple> = match state.action {
        1 => vec![mission_command(
            CMD_IMAGE_START_CAPTURE,
            [0.0, state.interval_time.trunc(), 0.0, nan, nan, nan, nan],
        )],
        2 => vec![mission_command(
            CMD_DO_SET_CAM_TRIGG_DIST,
            [state.interval_distance, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
        )],
        3 => vec![
            mission_command(CMD_DO_SET_CAM_TRIGG_DIST, [0.0; 7]),
            mission_command(CMD_IMAGE_STOP_CAPTURE, [0.0, nan, nan, nan, nan, nan, nan]),
        ],
        4 => vec![mission_command(
            CMD_VIDEO_START_CAPTURE,
            [0.0, VIDEO_CAPTURE_STATUS_INTERVAL, nan, nan, nan, nan, nan],
        )],
        5 => vec![mission_command(
            CMD_VIDEO_STOP_CAPTURE,
            [0.0, nan, nan, nan, nan, nan, nan],
        )],
        6 => vec![mission_command(
            CMD_IMAGE_START_CAPTURE,
            [0.0, 0.0, 1.0, 0.0, nan, nan, nan],
        )],
        _ => Vec::new(),
    };
    mode.into_iter().chain(gimbal).chain(action).collect()
}

pub fn edited_camera(state: &CameraState, property: &str, value: &Value) -> Option<CameraState> {
    let number = value.as_f64().or_else(|| value.as_bool().map(f64::from));
    let on = value.as_bool().or_else(|| number.map(|n| n != 0.0));
    Some(match property {
        "cameraAction" => CameraState {
            action: number? as i64,
            ..state.clone()
        },
        "cameraPhotoIntervalTime" => CameraState {
            interval_time: number?.max(1.0),
            ..state.clone()
        },
        "cameraPhotoIntervalDistance" => CameraState {
            interval_distance: number?.max(0.1),
            ..state.clone()
        },
        "specifyCameraMode" => CameraState {
            mode: on?.then(|| state.mode.unwrap_or(0.0)),
            ..state.clone()
        },
        "cameraMode" => CameraState {
            mode: Some(number?),
            ..state.clone()
        },
        "specifyGimbal" => CameraState {
            gimbal: on?.then(|| state.gimbal.unwrap_or((0.0, 0.0))),
            ..state.clone()
        },
        "gimbalPitch" => CameraState {
            gimbal: Some((
                number?.clamp(GIMBAL_PITCH_USER_RANGE.0, GIMBAL_PITCH_USER_RANGE.1),
                state.gimbal.map_or(0.0, |g| g.1),
            )),
            ..state.clone()
        },
        "gimbalYaw" => CameraState {
            gimbal: Some((
                state.gimbal.map_or(0.0, |g| g.0),
                number?.clamp(-180.0, 180.0),
            )),
            ..state.clone()
        },
        _ => return None,
    })
}

fn with_camera(sections: &[Simple], property: &str, value: &Value) -> Option<Vec<Simple>> {
    let state = edited_camera(&camera_state(sections), property, value)?;
    let others = sections
        .iter()
        .filter(|s| !CAMERA_COMMANDS.contains(&s.command))
        .cloned();
    Some(camera_items(&state).into_iter().chain(others).collect())
}

const CAMERA_MODE_IMAGE_SURVEY: f64 = 2.0;
const CAMERA_MODE_IMAGE: f64 = 0.0;

fn camera_mode_supported(doc: &Document) -> bool {
    camera_state(&doc.settings_sections).mode.is_some()
        || cmdinfo::tree(firmware(doc.firmware_type), VehicleClass::Generic)
            .contains_key(&CMD_SET_CAMERA_MODE)
}

pub fn with_survey_camera(doc: Document) -> Document {
    match camera_mode_supported(&doc) && camera_state(&doc.settings_sections).mode.is_none() {
        true => set_camera(&doc, 0, "specifyCameraMode", &json!(true))
            .and_then(|moded| set_camera(&moded, 0, "cameraMode", &json!(CAMERA_MODE_IMAGE_SURVEY)))
            .unwrap_or(doc),
        false => doc,
    }
}

fn has_area_scan(doc: &Document) -> bool {
    doc.items.iter().any(|item| matches!(item, Item::Complex { kind, .. } if TRANSECT_STYLE.contains(&kind.as_str())))
}

pub fn after_scan_removed(before: &Document, after: Document) -> Document {
    let undo = has_area_scan(before)
        && !has_area_scan(&after)
        && camera_state(&after.settings_sections).mode == Some(CAMERA_MODE_IMAGE);
    match undo {
        true => set_camera(&after, 0, "specifyCameraMode", &json!(false))
            .map(|unmoded| kept_jumps(&after, unmoded, &Some))
            .unwrap_or(after),
        false => after,
    }
}

fn camera_kind(command: i64) -> Option<StepKind> {
    match command {
        CMD_SET_CAMERA_MODE => Some(StepKind::Mode),
        CMD_DO_MOUNT_CONTROL => Some(StepKind::Gimbal),
        _ if CAMERA_COMMANDS.contains(&command) => Some(StepKind::Camera),
        _ => None,
    }
}

fn only(state: &CameraState, kinds: &[StepKind]) -> CameraState {
    let kept = |kind: StepKind| kinds.contains(&kind);
    CameraState {
        mode: state.mode.filter(|_| kept(StepKind::Mode)),
        gimbal: state.gimbal.filter(|_| kept(StepKind::Gimbal)),
        action: match kept(StepKind::Camera) {
            true => state.action,
            false => 0,
        },
        ..state.clone()
    }
}

pub fn run_commands(doc: &Document, at: usize) -> Vec<Simple> {
    match doc.items.get(at) {
        Some(Item::Simple(owner)) if owner.command == CMD_NAV_WAYPOINT => owner
            .sections
            .iter()
            .cloned()
            .chain(simples(&doc.items[at + 1..at + 1 + riders_after(doc, at)]))
            .collect(),
        Some(Item::Simple(other)) => other.sections.clone(),
        _ => Vec::new(),
    }
}

const CAMERA_KINDS: [StepKind; 3] = [StepKind::Mode, StepKind::Gimbal, StepKind::Camera];

fn set_run_camera(doc: &Document, at: usize, property: &str, value: &Value) -> Option<Document> {
    let Item::Simple(owner) = doc.items.get(at)? else {
        return None;
    };
    let riders = at + 1..at + 1 + riders_after(doc, at);
    let ridden = |kind: StepKind| -> Vec<usize> {
        riders
            .clone()
            .filter(|i| matches!(&doc.items[*i], Item::Simple(s) if camera_kind(s.command) == Some(kind)))
            .collect()
    };
    let state = edited_camera(&camera_state(&run_commands(doc, at)), property, value)?;
    let folded_kinds = owner
        .sections
        .iter()
        .filter_map(|section| camera_kind(section.command))
        .fold(Vec::<StepKind>::new(), |kinds, kind| {
            match kinds.contains(&kind) {
                true => kinds,
                false => kinds.into_iter().chain([kind]).collect(),
            }
        });
    let arriving = CAMERA_KINDS
        .into_iter()
        .filter(|kind| ridden(*kind).is_empty() && !folded_kinds.contains(kind));
    let (set_first, set_last): (Vec<StepKind>, Vec<StepKind>) =
        arriving.partition(|kind| *kind == StepKind::Mode);
    let sections = set_first
        .into_iter()
        .chain(folded_kinds.iter().copied())
        .chain(set_last)
        .flat_map(|kind| camera_items(&only(&state, &[kind])))
        .chain(
            owner
                .sections
                .iter()
                .filter(|s| !CAMERA_COMMANDS.contains(&s.command))
                .cloned(),
        )
        .collect();
    let folded = replaced(
        doc,
        at,
        Simple {
            sections,
            ..owner.clone()
        },
    );
    CAMERA_KINDS
        .into_iter()
        .rev()
        .filter(|kind| !ridden(*kind).is_empty())
        .try_fold(folded, |edited, kind| {
            let places = ridden(kind);
            let cleared = places
                .iter()
                .rev()
                .try_fold(edited, |left, place| remove(&left, place + 1).ok())?;
            Some(
                camera_items(&only(&state, &[kind]))
                    .into_iter()
                    .rev()
                    .fold(cleared, |placed, item| {
                        inserted_at(&placed, places[0], Item::Simple(item))
                    }),
            )
        })
}

pub fn set_camera(
    doc: &Document,
    visual_index: usize,
    property: &str,
    value: &Value,
) -> Option<Document> {
    match visual_index {
        0 => Some(Document {
            settings_sections: with_camera(&doc.settings_sections, property, value)?,
            ..doc.clone()
        }),
        _ => {
            let (at, current) = simple_at(doc, visual_index)?;
            (current.command == CMD_NAV_WAYPOINT)
                .then(|| set_run_camera(doc, at, property, value))
                .flatten()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StepKind {
    Hover,
    Wait,
    Gimbal,
    Camera,
    Mode,
    Drop,
    Command,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub kind: StepKind,
    pub commands: Vec<Simple>,
    pub item: Option<usize>,
    pub seconds: Option<f64>,
}

pub fn steps(
    doc: &Document,
    waypoint_visual: usize,
    servos: &[crate::payloadcheck::Servo],
) -> Option<Vec<Step>> {
    let (at, owner) = simple_at(doc, waypoint_visual)?;
    (owner.command == CMD_NAV_WAYPOINT).then_some(())?;
    let hover = owner.params[0]
        .filter(|seconds| *seconds > 0.0)
        .map(|seconds| Step {
            kind: StepKind::Hover,
            commands: Vec::new(),
            item: None,
            seconds: Some(seconds),
        });
    let folded = owner
        .sections
        .iter()
        .filter(|section| section.command != CMD_DO_CHANGE_SPEED)
        .map(|section| (section.clone(), None));
    let riding = (at + 1..at + 1 + riders_after(doc, at)).filter_map(|i| match &doc.items[i] {
        Item::Simple(rider) => Some((rider.clone(), Some(i + 1))),
        Item::Complex { .. } => None,
    });
    let run = folded
        .chain(riding)
        .fold(Vec::<Step>::new(), |listed, (command, item)| {
            let closes_photos = command.command == CMD_IMAGE_STOP_CAPTURE
                && listed.last().is_some_and(|last| {
                    last.kind == StepKind::Camera
                        && last.commands.last().map(|c| c.command)
                            == Some(CMD_DO_SET_CAM_TRIGG_DIST)
                });
            let kind = camera_kind(command.command).unwrap_or(match command.command {
                CMD_NAV_DELAY => StepKind::Wait,
                _ if item.is_some_and(|visual| is_drop_at(doc, visual - 1, servos)) => {
                    StepKind::Drop
                }
                _ => StepKind::Command,
            });
            match closes_photos {
                true => listed
                    .iter()
                    .enumerate()
                    .map(|(i, step)| match i + 1 == listed.len() {
                        true => Step {
                            commands: step
                                .commands
                                .iter()
                                .cloned()
                                .chain([command.clone()])
                                .collect(),
                            ..step.clone()
                        },
                        false => step.clone(),
                    })
                    .collect(),
                false => listed
                    .into_iter()
                    .chain([Step {
                        kind,
                        seconds: (kind == StepKind::Wait)
                            .then(|| command.params[0])
                            .flatten(),
                        commands: vec![command],
                        item,
                    }])
                    .collect(),
            }
        });
    Some(hover.into_iter().chain(run).collect())
}

pub const NO_SUCH_ACTION: &str = "This waypoint has no such action to move.";
pub const ACTION_AT_THE_END: &str = "That action is already in that place.";
pub const WAIT_SECONDS_RANGE: &str =
    "A wait is from 1 to 600 seconds. Remove it to fly on at once.";
pub const NOT_A_WAIT: &str = "That action is not a wait.";

pub fn set_wait(
    doc: &Document,
    waypoint_visual: usize,
    place: usize,
    seconds: f64,
    servos: &[crate::payloadcheck::Servo],
) -> Result<Document, String> {
    let listed = steps(doc, waypoint_visual, servos).ok_or_else(|| NOT_A_WAYPOINT.to_string())?;
    let item = listed
        .get(place)
        .filter(|step| step.kind == StepKind::Wait)
        .and_then(|step| step.item)
        .ok_or_else(|| NOT_A_WAIT.to_string())?;
    (WAIT_RANGE_SECONDS.contains(&seconds) && seconds >= SHORTEST_DELAY_SECONDS)
        .then_some(())
        .ok_or_else(|| WAIT_SECONDS_RANGE.to_string())?;
    set_param(doc, item, 1, seconds).ok_or_else(|| NOT_A_WAIT.to_string())
}

fn delay_of(seconds: f64) -> Simple {
    mission_command(
        CMD_NAV_DELAY,
        [
            seconds,
            TIME_OF_DAY_UNUSED,
            TIME_OF_DAY_UNUSED,
            TIME_OF_DAY_UNUSED,
            0.0,
            0.0,
            0.0,
        ],
    )
}

pub const HOLD_STAYS_FIRST: &str =
    "Hover stays first. To wait later in the list, make it 1 s or longer.";
pub const PLANE_HOLD_STAYS_FIRST: &str =
    "Hover stays first. A plane can't stop in the air, so it can't wait between other actions.";
pub const VEHICLE_HOLD_STAYS_FIRST: &str =
    "Hover stays first. This vehicle can't wait between other actions.";
pub const MODE_BEFORE_CAMERA: &str =
    "Set the camera mode before the photo or video. Otherwise the camera shoots in the old mode.";

fn hold_stays_first(doc: &Document) -> &'static str {
    match vehicle_class(doc.vehicle_type) {
        VehicleClass::MultiRotor => HOLD_STAYS_FIRST,
        VehicleClass::FixedWing | VehicleClass::Vtol => PLANE_HOLD_STAYS_FIRST,
        _ => VEHICLE_HOLD_STAYS_FIRST,
    }
}

fn refused(doc: &Document, order: &[&Step]) -> Option<(&'static str, &'static [StepKind])> {
    let hover_moved = order.iter().enumerate().any(|(at, step)| {
        step.kind == StepKind::Hover
            && at > 0
            && !(wait_offered(doc)
                && step
                    .seconds
                    .is_some_and(|seconds| seconds >= SHORTEST_DELAY_SECONDS))
    });
    let place = |kind: StepKind| order.iter().position(|step| step.kind == kind);
    let shoots_first = match (place(StepKind::Mode), place(StepKind::Camera)) {
        (Some(mode), Some(camera)) => camera < mode,
        _ => false,
    };
    match (hover_moved, shoots_first) {
        (true, _) => Some((hold_stays_first(doc), &[StepKind::Hover])),
        (_, true) => Some((MODE_BEFORE_CAMERA, &[StepKind::Mode, StepKind::Camera])),
        _ => None,
    }
}

fn flyable(doc: &Document, order: &[&Step]) -> Result<(), String> {
    refused(doc, order).map_or(Ok(()), |(reason, _)| Err(reason.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusedPlace {
    pub place: usize,
    pub reason: &'static str,
    pub about: usize,
}

fn concerned(listed: &[Step], from: usize, about: &[StepKind]) -> usize {
    about
        .iter()
        .filter_map(|kind| listed.iter().position(|step| step.kind == *kind))
        .find(|at| *at != from)
        .unwrap_or(from)
}

fn moved<'a>(listed: &'a [Step], from: usize, to: usize) -> Vec<&'a Step> {
    let without: Vec<&Step> = listed
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != from)
        .map(|(_, step)| step)
        .collect();
    without[..to]
        .iter()
        .copied()
        .chain([&listed[from]])
        .chain(without[to..].iter().copied())
        .collect()
}

pub fn action_places(
    doc: &Document,
    waypoint_visual: usize,
    servos: &[crate::payloadcheck::Servo],
) -> Option<Vec<(Vec<usize>, Vec<RefusedPlace>)>> {
    let listed = steps(doc, waypoint_visual, servos)?;
    Some(
        (0..listed.len())
            .map(|from| {
                let answers: Vec<(usize, Option<(&'static str, &'static [StepKind])>)> = (0
                    ..listed.len())
                    .map(|to| (to, refused(doc, &moved(&listed, from, to))))
                    .collect();
                (
                    answers
                        .iter()
                        .filter(|(_, answer)| answer.is_none())
                        .map(|(to, _)| *to)
                        .collect(),
                    answers
                        .iter()
                        .filter_map(|(to, answer)| {
                            answer.map(|(reason, about)| RefusedPlace {
                                place: *to,
                                reason,
                                about: concerned(&listed, from, about),
                            })
                        })
                        .collect(),
                )
            })
            .collect(),
    )
}

pub fn move_action(
    doc: &Document,
    waypoint_visual: usize,
    from: usize,
    to: usize,
    servos: &[crate::payloadcheck::Servo],
) -> Result<Document, String> {
    let listed = steps(doc, waypoint_visual, servos).ok_or_else(|| NOT_A_WAYPOINT.to_string())?;
    (from < listed.len() && to < listed.len())
        .then_some(())
        .ok_or_else(|| NO_SUCH_ACTION.to_string())?;
    (from != to)
        .then_some(())
        .ok_or_else(|| ACTION_AT_THE_END.to_string())?;
    let order = moved(&listed, from, to);
    flyable(doc, &order)?;
    let (at, owner) = simple_at(doc, waypoint_visual).ok_or_else(|| NOT_A_WAYPOINT.to_string())?;
    let arrives_holding = order
        .first()
        .filter(|first| first.kind == StepKind::Hover)
        .and_then(|first| first.seconds);
    let flown: Vec<&Step> = order
        .iter()
        .skip(usize::from(arrives_holding.is_some()))
        .copied()
        .collect();
    let leading_camera = flown
        .iter()
        .enumerate()
        .take_while(|(i, step)| {
            CAMERA_KINDS.contains(&step.kind)
                && !flown[..*i].iter().any(|before| before.kind == step.kind)
        })
        .count();
    let commands_of = |step: &Step| match step.kind {
        StepKind::Hover => vec![delay_of(step.seconds.unwrap_or(0.0))],
        _ => step.commands.clone(),
    };
    let speed = owner
        .sections
        .iter()
        .filter(|section| section.command == CMD_DO_CHANGE_SPEED)
        .cloned();
    let run: Vec<Item> = flown[..leading_camera]
        .iter()
        .flat_map(|step| commands_of(step))
        .chain(speed)
        .chain(
            flown[leading_camera..]
                .iter()
                .flat_map(|step| commands_of(step)),
        )
        .map(Item::Simple)
        .collect();
    let arriving = Simple {
        params: std::array::from_fn(|i| match i {
            0 => Some(arrives_holding.unwrap_or(0.0)),
            _ => owner.params[i],
        }),
        sections: Vec::new(),
        ..owner.clone()
    };
    let folded = fold_rest(
        &std::iter::once(Item::Simple(arriving))
            .chain(run)
            .collect::<Vec<_>>(),
        vehicle_class(doc.vehicle_type),
    );
    let riders = riders_after(doc, at);
    let grown = folded.len();
    Ok(kept_jumps(
        doc,
        Document {
            items: doc.items[..at]
                .iter()
                .cloned()
                .chain(folded)
                .chain(doc.items[at + 1 + riders..].iter().cloned())
                .collect(),
            ..doc.clone()
        },
        &|was| match was {
            was if was <= at => Some(was),
            was if was > at + riders => Some(was + grown - 1 - riders),
            _ => None,
        },
    ))
}

fn camera_span(rest: &[Simple], found: Vec<Found>) -> usize {
    match camera_match(rest, &found) {
        Some((kind, taken)) => {
            taken
                + camera_span(
                    &rest[taken..],
                    found.into_iter().chain(std::iter::once(kind)).collect(),
                )
        }
        None => 0,
    }
}

fn speed_span(rest: &[Simple], class: VehicleClass) -> usize {
    let Some(item) = rest.first() else { return 0 };
    let shaped = item.command == CMD_DO_CHANGE_SPEED && p(item, 2) == -1.0 && zero(item, 3);
    let kind_matches = match class {
        VehicleClass::MultiRotor => p(item, 0) == 1.0,
        VehicleClass::FixedWing => p(item, 0) == 0.0,
        _ => false,
    };
    usize::from(shaped && kind_matches)
}

fn section_span(rest: &[Item], class: VehicleClass) -> usize {
    let simple: Vec<Simple> = rest
        .iter()
        .map_while(|item| match item {
            Item::Simple(s) => Some(s.clone()),
            Item::Complex { .. } => None,
        })
        .collect();
    let camera = camera_span(&simple, Vec::new());
    camera + speed_span(&simple[camera..], class)
}

fn simples(items: &[Item]) -> Vec<Simple> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Simple(s) => Some(s.clone()),
            Item::Complex { .. } => None,
        })
        .collect()
}

fn fold_rest(items: &[Item], class: VehicleClass) -> Vec<Item> {
    match items.split_first() {
        None => Vec::new(),
        Some((Item::Simple(owner), rest)) if owner.command == CMD_NAV_WAYPOINT => {
            let span = section_span(rest, class);
            let folded = Simple {
                sections: simples(&rest[..span]),
                ..owner.clone()
            };
            std::iter::once(Item::Simple(folded))
                .chain(fold_rest(&rest[span..], class))
                .collect()
        }
        Some((first, rest)) => std::iter::once(first.clone())
            .chain(fold_rest(rest, class))
            .collect(),
    }
}

fn fold(items: Vec<Item>, class: VehicleClass) -> (Vec<Simple>, Vec<Item>) {
    let settings = section_span(&items, class);
    (
        simples(&items[..settings]),
        fold_rest(&items[settings..], class),
    )
}

fn save_item(item: &Item, seq: usize, unsaved: &[&str]) -> Value {
    match item {
        Item::Complex { json, .. } => {
            let mut numbered = json.clone();
            if let Some(fields) = numbered.as_object_mut() {
                fields.retain(|key, _| !unsaved.contains(&key.as_str()));
            }
            if let Some(items) = numbered
                .pointer_mut("/TransectStyleComplexItem/Items")
                .and_then(Value::as_array_mut)
            {
                items
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, item)| item["doJumpId"] = json!(seq + i));
            }
            numbered
        }
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
                .flat_map(|a| {
                    [
                        ("AltitudeMode", json!(a.mode)),
                        ("Altitude", json!(a.altitude)),
                        ("AMSLAltAboveTerrain", json!(a.amsl_above_terrain)),
                    ]
                })
                .map(|(k, v)| (k.to_string(), v))
                .collect();
            match base {
                Value::Object(fields) => {
                    Value::Object(fields.into_iter().chain(altitude).collect())
                }
                other => other,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_camera_section_writes_the_commands_camera_section_appends() {
        let doc = Document {
            settings_sections: Vec::new(),
            ..section()
        };
        let timed = set_camera(&doc, 0, "cameraAction", &json!(1)).unwrap();
        assert_eq!(
            timed
                .settings_sections
                .iter()
                .map(|s| (s.command, s.params[1]))
                .collect::<Vec<_>>(),
            [(CMD_IMAGE_START_CAPTURE, Some(10.0))],
            "Take photos (time) starts with the default 10 s interval"
        );
        let quicker = set_camera(&timed, 0, "cameraPhotoIntervalTime", &json!(5)).unwrap();
        assert_eq!(camera_state(&quicker.settings_sections).interval_time, 5.0);
        let aimed = set_camera(
            &set_camera(&quicker, 0, "specifyGimbal", &json!(true)).unwrap(),
            0,
            "gimbalPitch",
            &json!(-45.0),
        )
        .unwrap();
        let commands: Vec<i64> = aimed.settings_sections.iter().map(|s| s.command).collect();
        assert_eq!(
            commands,
            [CMD_DO_MOUNT_CONTROL, CMD_IMAGE_START_CAPTURE],
            "gimbal before the action, as appendSectionItems orders them"
        );
        assert_eq!(
            camera_state(&aimed.settings_sections).gimbal,
            Some((-45.0, 0.0))
        );
        assert_eq!(
            aimed.settings_sections[0].params[0],
            Some(-45.0),
            "MAV_CMD_DO_MOUNT_CONTROL param1 is the pitch itself, as CameraSection writes and scans it"
        );
        let moded = set_camera(&aimed, 0, "specifyCameraMode", &json!(true)).unwrap();
        assert_eq!(moded.settings_sections[0].command, CMD_SET_CAMERA_MODE);
        let stopped = set_camera(&moded, 0, "cameraAction", &json!(3)).unwrap();
        assert_eq!(
            stopped
                .settings_sections
                .iter()
                .map(|s| s.command)
                .collect::<Vec<_>>(),
            [
                CMD_SET_CAMERA_MODE,
                CMD_DO_MOUNT_CONTROL,
                CMD_DO_SET_CAM_TRIGG_DIST,
                CMD_IMAGE_STOP_CAPTURE
            ]
        );
        assert_eq!(
            camera_state(&stopped.settings_sections).action,
            3,
            "the written commands read back as the same action"
        );
    }

    #[test]
    fn a_lone_zero_trigger_distance_reads_as_take_photos_distance_at_zero_like_scan_trigger_stop_distance()
     {
        let zero_trigger = mission_command(CMD_DO_SET_CAM_TRIGG_DIST, [0.0; 7]);
        let state = camera_state(std::slice::from_ref(&zero_trigger));
        assert_eq!(
            (state.action, state.interval_distance),
            (2, 0.0),
            "CameraSection::_scanTriggerStopDistance sets TakePhotoIntervalDistance with the raw 0 distance"
        );
        let stop = mission_command(
            CMD_IMAGE_STOP_CAPTURE,
            [
                0.0,
                f64::NAN,
                f64::NAN,
                f64::NAN,
                f64::NAN,
                f64::NAN,
                f64::NAN,
            ],
        );
        let stopping = camera_state(&[zero_trigger, stop]);
        assert_eq!(
            (stopping.action, stopping.interval_distance),
            (3, DEFAULT_INTERVAL_DISTANCE),
            "scanStopTakingPhotos leaves the distance fact at its default"
        );
        let distance = camera_section(&[mission_command(CMD_DO_SET_CAM_TRIGG_DIST, [0.0; 7])])["facts"][1]["value"].as_f64();
        assert_eq!(distance, Some(0.0));
    }

    fn by_value(value: Value) -> Value {
        match value {
            Value::Number(n) => json!(n.as_f64()),
            Value::Array(items) => Value::Array(items.into_iter().map(by_value).collect()),
            Value::Object(fields) => {
                Value::Object(fields.into_iter().map(|(k, v)| (k, by_value(v))).collect())
            }
            other => other,
        }
    }

    fn without_home_altitude(mut plan: Value) -> Value {
        plan["mission"]["plannedHomePosition"][2] = Value::Null;
        by_value(plan)
    }

    fn resaves_as_qt(original: &str, resaved_by_qt: &str) {
        let doc = load(original, 2).unwrap();
        let qt: Value = serde_json::from_str(resaved_by_qt).unwrap();
        assert_eq!(without_home_altitude(save(&doc)), without_home_altitude(qt));
    }

    #[test]
    fn an_old_format_plan_is_written_back_the_way_qt_upgrades_it() {
        resaves_as_qt(
            include_str!("../../test/MissionManager/SectionTest.plan"),
            include_str!("../tests/fixtures/sectiontest-resaved-by-qt.plan"),
        );
    }

    #[test]
    fn a_survey_plan_is_written_back_the_way_qt_writes_it() {
        resaves_as_qt(
            include_str!("../tests/fixtures/survey-upload.plan"),
            include_str!("../tests/fixtures/survey-resaved-by-qt.plan"),
        );
    }

    #[test]
    fn a_loaded_document_flattens_to_the_same_upload_as_the_file() {
        let text = include_str!("../tests/fixtures/survey-upload.plan");
        let from_doc = crate::planitems::flatten(&save(&load(text, 2).unwrap())).unwrap();
        let from_file = crate::planitems::flatten(&serde_json::from_str(text).unwrap()).unwrap();
        assert_eq!(format!("{from_doc:?}"), format!("{from_file:?}"));
    }

    #[test]
    fn an_upload_flies_the_rotated_entry_vertex_the_file_forgets() {
        let scan = json!({ "version": 3, "type": "ComplexItem", "complexItemType": "StructureScan", "polygon": [[47.0, 8.0], [47.001, 8.0], [47.001, 8.001]], "ScanBottomAlt": 0, "StructureHeight": 25, "Layers": 1, "CameraCalc": { "version": 2, "CameraName": "Manual (no camera specs)", "AdjustedFootprintSide": 25, "AdjustedFootprintFrontal": 25, "DistanceToSurface": 50, "DistanceMode": 1 }, "EntranceAltitude": 50, "GimbalPitch": 0, "StartFromTop": true });
        let plan = json!({ "fileType": "Plan", "version": 1, "groundStation": "QGroundControl", "geoFence": { "circles": [], "polygons": [], "version": 2 }, "rallyPoints": { "points": [], "version": 2 }, "mission": { "version": 2, "firmwareType": 3, "vehicleType": 2, "cruiseSpeed": 15, "hoverSpeed": 5, "globalPlanAltitudeMode": 1, "plannedHomePosition": [47.0, 8.0, 0], "items": [scan] } });
        let doc = load(&plan.to_string(), 2).unwrap();
        let Item::Complex {
            kind,
            json,
            item_count,
        } = &doc.items[0]
        else {
            panic!()
        };
        let rotated = Document {
            items: vec![Item::Complex {
                kind: kind.clone(),
                json: crate::structurescan::rotated_entry(json).unwrap(),
                item_count: *item_count,
            }],
            ..doc.clone()
        };
        let first = |plan: &Value| {
            crate::planitems::flatten(plan)
                .unwrap()
                .iter()
                .skip(1)
                .find(|i| i.command == 16)
                .map(|i| (i.params[4], i.params[5]))
                .unwrap()
        };
        assert_ne!(
            first(&mission(&rotated)),
            first(&mission(&doc)),
            "QGC appendMissionItems flies from the live _entryVertex"
        );
        assert_eq!(
            save(&rotated),
            save(&doc),
            "StructureScanComplexItem::save writes no entry vertex"
        );
    }

    fn section() -> Document {
        load(
            include_str!("../../test/MissionManager/SectionTest.plan"),
            2,
        )
        .unwrap()
    }

    const QT_DEFAULTS: EditDefaults = EditDefaults {
        mission_item_altitude: 75.0,
        map_center: None,
        vtol_transition_distance: VTOL_TRANSITION_DISTANCE_DEFAULT,
    };

    #[test]
    fn a_command_with_no_position_picked_on_a_waypoint_is_added_at_it_instead_of_replacing_it() {
        let text = r#"{"fileType":"Plan","version":1,"groundStation":"QGroundControl","mission":{"version":2,"firmwareType":3,"vehicleType":2,"cruiseSpeed":15,"hoverSpeed":5,"plannedHomePosition":[47.0,8.0,500],"items":[
            {"type":"SimpleItem","autoContinue":true,"command":22,"doJumpId":1,"frame":3,"params":[0,0,0,null,0,0,5]},
            {"type":"SimpleItem","autoContinue":true,"command":16,"doJumpId":2,"frame":3,"params":[0,0,0,null,47.001,8.0,26]},
            {"type":"SimpleItem","autoContinue":true,"command":93,"doJumpId":3,"frame":2,"params":[10,-1,-1,-1,0,0,0]},
            {"type":"SimpleItem","autoContinue":true,"command":16,"doJumpId":4,"frame":3,"params":[0,0,0,null,47.002,8.0,26]},
            {"type":"SimpleItem","autoContinue":true,"command":20,"doJumpId":5,"frame":2,"params":[0,0,0,0,0,0,0]}]},"geoFence":{"version":2,"polygons":[],"circles":[]},"rallyPoints":{"version":2,"points":[]}}"#;
        let doc = load(text, 2).unwrap();
        let commands = |doc: &Document| {
            doc.items
                .iter()
                .map(|item| match item {
                    Item::Simple(s) => s.command,
                    Item::Complex { .. } => 0,
                })
                .collect::<Vec<_>>()
        };
        let (servo, added) = command_at(&doc, 2, 183, &QT_DEFAULTS).unwrap();
        assert_eq!(
            (commands(&servo), added),
            (vec![22, 16, 93, 183, 16, 20], Some(4)),
            "Set servo picked on waypoint 2 keeps the waypoint and lands after the hold already attached to it, before the next waypoint"
        );
        let (last, added) = command_at(&doc, 4, 183, &QT_DEFAULTS).unwrap();
        assert_eq!(
            (commands(&last), added),
            (vec![22, 16, 93, 16, 183, 20], Some(5)),
            "on the last waypoint it lands before the return, which is not one of the waypoint's actions"
        );
        let (loiter, added) = command_at(&doc, 2, 19, &QT_DEFAULTS).unwrap();
        assert_eq!(
            (commands(&loiter), added),
            (vec![22, 19, 93, 16, 20], None),
            "another command with a position still replaces the waypoint"
        );
        let (delay, added) = command_at(&doc, 3, 183, &QT_DEFAULTS).unwrap();
        assert_eq!(
            (commands(&delay), added),
            (vec![22, 16, 183, 16, 20], None),
            "an item that had no position is changed in place"
        );
    }

    #[test]
    fn whole_numbers_written_as_floats_load_as_qt_reads_them_with_to_int() {
        let text = r#"{"fileType":"Plan","version":1,"groundStation":"QGroundControl","mission":{"version":2,"firmwareType":12.0,"vehicleType":2,"cruiseSpeed":15,"hoverSpeed":5,"plannedHomePosition":[47.0,8.0,500],"items":[{"type":"SimpleItem","autoContinue":true,"command":16.0,"doJumpId":1.0,"frame":3.0,"params":[0,0,0,null,47.001,8.0,50]}]},"geoFence":{"version":2,"polygons":[],"circles":[]},"rallyPoints":{"version":2,"points":[]}}"#;
        let doc = load(text, 2).expect("16.0 is a command");
        assert_eq!(
            doc.firmware_type, 12,
            "12.0 is PX4, not the generic firmware a failed integer read falls back to"
        );
        assert_eq!(
            whole(&json!(16.5)),
            Some(0),
            "QJsonValue::toInt gives the default for a fractional double"
        );
    }

    #[test]
    fn a_downloaded_px4_mission_puts_home_thirty_metres_north_of_its_first_waypoint() {
        let waypoint = Downloaded {
            frame: 3,
            command: 16,
            params: [0.0, 0.0, 0.0, 0.0, 47.0, 8.0, 50.0],
            auto_continue: true,
        };
        let template = Document {
            home: Some([-35.36, 149.17, 0.0]),
            ..section()
        };
        let downloaded = from_vehicle(&[waypoint.clone()], false, &template);
        assert_eq!(
            downloaded.home, None,
            "the fly view keeps no derived home; only the plan view places one"
        );
        let home = planned_home(&downloaded, Some([1.0, 2.0, 3.0]))
            .expect("MissionController::_setPlannedHomePositionFromFirstCoordinate places one");
        assert!(
            (home[0] - 47.0 - 30.0 / 111_195.0).abs() < 1e-6
                && (home[1] - 8.0).abs() < 1e-9
                && home[2] == 0.0,
            "not the previous plan's home, and not the vehicle's without a takeoff: {home:?}"
        );
        let takeoff = Downloaded {
            command: 22,
            ..waypoint
        };
        let launched = from_vehicle(&[takeoff], false, &template);
        assert_eq!(
            planned_home(&launched, Some([1.0, 2.0, 3.0])),
            Some([1.0, 2.0, 3.0]),
            "TakeoffMissionItem copies the active vehicle's home into an unset planned home"
        );
    }

    #[test]
    fn a_command_that_gains_a_coordinate_lands_on_the_map_centre_hint() {
        let rtl = Simple {
            command: 20,
            frame: FRAME_MISSION,
            params: [Some(0.0); 7],
            auto_continue: true,
            altitude: None,
            sections: Vec::new(),
        };
        let doc = Document {
            items: vec![Item::Simple(rtl.clone())],
            ..section()
        };
        let hinted = EditDefaults {
            map_center: Some((47.39, 8.54)),
            ..QT_DEFAULTS
        };
        let moved =
            |defaults: &EditDefaults| match &set_command(&doc, 1, CMD_NAV_WAYPOINT, defaults)
                .unwrap()
                .items[0]
            {
                Item::Simple(s) => (s.params[4], s.params[5]),
                Item::Complex { .. } => panic!("a waypoint is simple"),
            };
        assert_eq!(
            moved(&hinted),
            (Some(47.39), Some(8.54)),
            "SimpleMissionItem::_setDefaultsForCommand uses _mapCenterHint"
        );
        assert_eq!(
            moved(&QT_DEFAULTS),
            (Some(0.0), Some(0.0)),
            "without a hint it keeps the zeros it had"
        );
        let placed = Document {
            items: vec![Item::Simple(Simple {
                params: [
                    Some(0.0),
                    Some(0.0),
                    Some(0.0),
                    Some(0.0),
                    Some(1.0),
                    Some(2.0),
                    Some(0.0),
                ],
                ..rtl
            })],
            ..section()
        };
        assert!(
            matches!(&set_command(&placed, 1, CMD_NAV_WAYPOINT, &hinted).unwrap().items[0], Item::Simple(s) if (s.params[4], s.params[5]) == (Some(1.0), Some(2.0))),
            "an item that already has a place keeps it"
        );
    }

    fn matches_qt(doc: &Document, qt: &str) {
        let qt: Value = serde_json::from_str(qt).unwrap();
        assert_eq!(without_home_altitude(save(doc)), without_home_altitude(qt));
    }

    #[test]
    fn an_appended_waypoint_takes_the_default_altitude_because_qt_never_looks_back_on_append() {
        matches_qt(
            &insert_waypoint(&section(), 47.634, -122.089, -1, &QT_DEFAULTS),
            include_str!("../tests/fixtures/edit-A-by-qt.plan"),
        );
    }

    #[test]
    fn an_inserted_waypoint_takes_the_route_altitude_the_pilot_set_where_qt_copies_the_one_before_it()
     {
        let placed = insert_waypoint(&section(), 47.6335, -122.0885, 3, &QT_DEFAULTS);
        let Some(Item::Simple(inserted)) = placed.items.get(2) else {
            panic!("the waypoint goes in at visual index 3")
        };
        assert_eq!(
            inserted.altitude.as_ref().map(|a| a.altitude),
            Some(QT_DEFAULTS.mission_item_altitude),
            "the route settings promise new waypoints the route altitude, so a new waypoint no longer copies the 20 m before it as Qt does"
        );
        matches_qt(
            &set_altitude(&placed, 3, 20.0).unwrap(),
            include_str!("../tests/fixtures/edit-B-by-qt.plan"),
        );
    }

    #[test]
    fn an_inserted_waypoint_in_a_frame_other_than_the_route_keeps_the_altitude_before_it() {
        let amsl = Document {
            global_altitude_mode: crate::altitudemodes::MIXED,
            items: section()
                .items
                .iter()
                .map(|item| match item {
                    Item::Simple(s) if s.altitude.is_some() => Item::Simple(Simple {
                        frame: FRAME_GLOBAL,
                        altitude: s.altitude.clone().map(|a| Altitude {
                            mode: crate::altitudemodes::ABSOLUTE,
                            altitude: 520.0,
                            ..a
                        }),
                        ..s.clone()
                    }),
                    other => other.clone(),
                })
                .collect(),
            ..section()
        };
        let placed = insert_waypoint(&amsl, 47.6335, -122.0885, 3, &QT_DEFAULTS);
        let Some(Item::Simple(inserted)) = placed.items.get(2) else {
            panic!("the waypoint goes in at visual index 3")
        };
        assert_eq!(
            inserted.altitude.as_ref().map(|a| (a.mode, a.altitude)),
            Some((crate::altitudemodes::ABSOLUTE, 520.0)),
            "the route altitude is above launch, so it would put an AMSL waypoint underground"
        );
    }

    #[test]
    fn a_removed_item_renumbers_those_after_it() {
        matches_qt(
            &remove(&section(), 2).unwrap(),
            include_str!("../tests/fixtures/edit-C-by-qt.plan"),
        );
        assert!(
            remove(&section(), 0).is_err(),
            "visual index zero is the mission settings item, which is never removed"
        );
        assert!(remove(&section(), 6).is_err());
    }

    #[test]
    fn the_first_waypoint_of_an_empty_plan_puts_home_thirty_metres_north_of_it() {
        let empty = Document {
            home: None,
            items: Vec::new(),
            ..section()
        };
        let placed = insert_waypoint(&empty, 47.64, -122.1, -1, &QT_DEFAULTS);
        matches_qt(&placed, include_str!("../tests/fixtures/edit-D-by-qt.plan"));
        let home = placed.home.unwrap();
        assert!((home[0] - 47.64026979617687).abs() < 1e-12 && home[1] == -122.1 && home[2] == 0.0);
    }

    #[test]
    fn a_takeoff_copies_the_previous_altitude_frame_whatever_the_plan_mode() {
        let doc = section();
        let amsl = Document {
            global_altitude_mode: crate::altitudemodes::ABSOLUTE,
            items: doc
                .items
                .iter()
                .map(|item| match item {
                    Item::Simple(s) if s.altitude.is_some() => Item::Simple(Simple {
                        altitude: s.altitude.clone().map(|a| Altitude {
                            mode: crate::altitudemodes::ABSOLUTE,
                            ..a
                        }),
                        ..s.clone()
                    }),
                    other => other.clone(),
                })
                .collect(),
            ..doc
        };
        let end = amsl.items.len() as i64 + 1;
        let placed = insert_simple(&amsl, CMD_NAV_TAKEOFF, 47.63, -122.08, end, &QT_DEFAULTS);
        let Some(Item::Simple(takeoff)) = placed.items.last() else {
            panic!("a takeoff is a simple item")
        };
        assert_eq!(
            takeoff.altitude.as_ref().map(|a| a.mode),
            Some(crate::altitudemodes::ABSOLUTE),
            "insertTakeoffItem copies _findPreviousAltitude's frame, not the relative default"
        );
    }

    #[test]
    fn landing_a_multirotor_appends_a_return_to_launch_as_qt_does() {
        matches_qt(
            &insert_land(&section(), 47.6325, -122.0870, -1, &QT_DEFAULTS).unwrap(),
            include_str!("../tests/fixtures/edit-F-by-qt.plan"),
        );
        let plane = Document {
            vehicle_type: 1,
            ..section()
        };
        assert!(
            insert_land(&plane, 47.6325, -122.0870, -1, &QT_DEFAULTS).is_err(),
            "a fixed-wing landing is a pattern, not a simple item"
        );
    }

    #[test]
    fn an_altitude_edit_moves_param_seven_with_it() {
        matches_qt(
            &set_altitude(&section(), 2, 33.0).unwrap(),
            include_str!("../tests/fixtures/edit-G-by-qt.plan"),
        );
        assert!(
            set_altitude(&section(), 9, 33.0).is_none(),
            "there is no ninth item"
        );
    }

    #[test]
    fn changing_a_command_resets_its_parameters_and_altitude_but_keeps_the_position() {
        matches_qt(
            &set_command(&section(), 2, 19, &QT_DEFAULTS).unwrap(),
            include_str!("../tests/fixtures/edit-H-by-qt.plan"),
        );
    }

    #[test]
    fn nothing_but_the_launch_goes_before_the_takeoff_so_an_insert_ahead_of_it_lands_just_after() {
        let doc = section();
        assert_eq!(
            past_takeoff(&doc, "waypoint", 1),
            2,
            "after the launch is still before the takeoff, so the waypoint moves past it"
        );
        assert_eq!(past_takeoff(&doc, "survey", 1), 2);
        assert_eq!(
            past_takeoff(&doc, "waypoint", 3),
            3,
            "after the takeoff an item stays where it was asked for"
        );
        assert_eq!(
            past_takeoff(&doc, "waypoint", -1),
            -1,
            "the end is past the takeoff already"
        );
        assert_eq!(
            past_takeoff(&remove(&doc, 1).unwrap(), "waypoint", 1),
            1,
            "with no takeoff there is nothing to keep clear of"
        );
        assert_eq!(past_takeoff(&doc, "takeoff", 1), 1);
    }

    #[test]
    fn a_takeoff_sits_on_the_launch_position_at_the_default_altitude() {
        let without_takeoff = remove(&section(), 1).unwrap();
        matches_qt(
            &insert_takeoff(&without_takeoff, 1, &QT_DEFAULTS).unwrap(),
            include_str!("../tests/fixtures/edit-takeoff-by-qt.plan"),
        );
        assert!(
            insert_takeoff(
                &Document {
                    home: None,
                    ..without_takeoff.clone()
                },
                1,
                &QT_DEFAULTS
            )
            .is_err()
        );
        let plane = insert_takeoff(
            &Document {
                vehicle_type: 1,
                ..without_takeoff.clone()
            },
            1,
            &QT_DEFAULTS,
        )
        .unwrap();
        let Item::Simple(climb) = &plane.items[0] else {
            panic!("takeoff")
        };
        let launch = without_takeoff.home.unwrap();
        let out = crate::terrain::qt_distance(
            (launch[0], launch[1]),
            (climb.params[4].unwrap(), climb.params[5].unwrap()),
        );
        assert!(
            (out - climb_out_distance(QT_DEFAULTS.mission_item_altitude, true, 0.0)).abs() < 0.5,
            "a plane's takeoff sits on a 30 degree climb-out due north of launch, as TakeoffMissionItem::_setLaunchCoordinate puts it: {out}"
        );
        assert!(climb.params[4].unwrap() > launch[0]);
        assert!(
            (climb_out_distance(50.0, true, 300.0) - 50.0 / 30f64.to_radians().tan()).abs() < 1e-9
        );
        assert_eq!(
            climb_out_distance(0.0, true, 300.0),
            300.0,
            "a zero relative altitude falls back to the transition distance"
        );
        assert_eq!(climb_out_distance(50.0, false, 300.0), 75.0);
        let vtol = insert_takeoff(
            &Document {
                vehicle_type: 20,
                ..without_takeoff.clone()
            },
            1,
            &QT_DEFAULTS,
        )
        .unwrap();
        let Item::Simple(takeoff) = &vtol.items[0] else {
            panic!("takeoff")
        };
        let home = without_takeoff.home.unwrap();
        let at = (takeoff.params[4].unwrap(), takeoff.params[5].unwrap());
        assert!(
            (crate::surveygrid::distance_between((home[0], home[1]), at)
                - VTOL_TRANSITION_DISTANCE_DEFAULT)
                .abs()
                < 0.5,
            "a VTOL takes off the transition distance north of launch"
        );
        assert_eq!(
            takeoff.command, CMD_NAV_VTOL_TAKEOFF,
            "insertTakeoffItem sends MAV_CMD_NAV_VTOL_TAKEOFF for a VTOL; command 22 is a fixed-wing takeoff on a QuadPlane"
        );
    }

    #[test]
    fn moving_the_launch_moves_home_and_places_only_an_unplaced_takeoff() {
        let placed = insert_takeoff(&remove(&section(), 1).unwrap(), 1, &QT_DEFAULTS).unwrap();
        let Item::Simple(takeoff) = &placed.items[0] else {
            panic!("takeoff")
        };
        let moved = set_launch(&placed, 1, 47.5, 8.5, true, 300.0).unwrap();
        assert_eq!(
            (moved.home.map(|h| (h[0], h[1])), &moved.items[0]),
            (Some((47.5, 8.5)), &placed.items[0]),
            "TakeoffMissionItem::_setLaunchCoordinate leaves a valid takeoff where it is"
        );
        let unplaced = replaced(
            &placed,
            0,
            Simple {
                params: [
                    takeoff.params[0],
                    takeoff.params[1],
                    takeoff.params[2],
                    takeoff.params[3],
                    None,
                    None,
                    takeoff.params[6],
                ],
                ..takeoff.clone()
            },
        );
        let Item::Simple(together) = &set_launch(&unplaced, 1, 47.5, 8.5, true, 300.0)
            .unwrap()
            .items[0]
        else {
            panic!("takeoff")
        };
        assert_eq!(
            (together.params[4], together.params[5]),
            (Some(47.5), Some(8.5))
        );
        let Item::Simple(apart) = &set_launch(&unplaced, 1, 47.5, 8.5, false, 300.0)
            .unwrap()
            .items[0]
        else {
            panic!("takeoff")
        };
        assert!(
            (crate::surveygrid::distance_between(
                (47.5, 8.5),
                (apart.params[4].unwrap(), apart.params[5].unwrap())
            ) - 300.0)
                .abs()
                < 0.5
        );
        let plane = Document {
            vehicle_type: 1,
            ..unplaced.clone()
        };
        let in_terrain = replaced(
            &plane,
            0,
            Simple {
                frame: FRAME_GLOBAL_TERRAIN_ALT,
                params: [None, None, None, None, None, None, Some(100.0)],
                ..takeoff.clone()
            },
        );
        let Item::Simple(climb) = &set_launch(&in_terrain, 1, 47.5, 8.5, false, 300.0)
            .unwrap()
            .items[0]
        else {
            panic!("takeoff")
        };
        assert!(
            (crate::surveygrid::distance_between(
                (47.5, 8.5),
                (climb.params[4].unwrap(), climb.params[5].unwrap())
            ) - 150.0)
                .abs()
                < 0.5,
            "only a relative-frame takeoff climbs out at 30 degrees"
        );
    }

    #[test]
    fn a_region_of_interest_is_the_location_command_only_where_the_firmware_takes_it() {
        let px4 = Document {
            firmware_type: 12,
            ..section()
        };
        let placed = insert_roi(&px4, 47.63, -122.09, -1, &QT_DEFAULTS);
        assert!(
            matches!(placed.items.last(), Some(Item::Simple(s)) if s.command == CMD_DO_SET_ROI_LOCATION)
        );
        let apm = Document {
            firmware_type: 3,
            ..section()
        };
        let placed = insert_roi(&apm, 47.63, -122.09, 2, &QT_DEFAULTS);
        let Some(Item::Simple(roi)) = placed.items.get(1) else {
            panic!("the region of interest goes where it was asked")
        };
        assert_eq!(
            (roi.command, roi.params[0], roi.params[4], roi.params[5]),
            (
                CMD_DO_SET_ROI,
                Some(MAV_ROI_LOCATION),
                Some(47.63),
                Some(-122.09)
            )
        );
    }

    #[test]
    fn a_payload_drop_is_a_gripper_release_with_no_place_of_its_own() {
        let apm = Document {
            firmware_type: 3,
            ..section()
        };
        let placed = insert_simple(&apm, 211, 0.0, 0.0, 2, &QT_DEFAULTS);
        let Some(Item::Simple(drop)) = placed.items.get(1) else {
            panic!("the drop goes where it was asked")
        };
        assert_eq!(
            (
                drop.command,
                drop.params[0],
                drop.params[1],
                drop.params[4],
                drop.params[5],
                drop.altitude.is_none()
            ),
            (211, Some(1.0), Some(0.0), Some(0.0), Some(0.0), true),
            "gripper 1, action 0 = release, no coordinate: it fires at the waypoint before it"
        );
        assert_eq!(placed.items.len(), apm.items.len() + 1);
    }

    #[test]
    fn with_a_servo_per_bay_each_drop_moves_the_next_bays_output_and_survives_a_save() {
        let servos: Vec<crate::payloadcheck::Servo> = (9..=12)
            .map(|output| crate::payloadcheck::Servo {
                output,
                release: 1100.0,
                hold: 1900.0,
            })
            .collect();
        let copter = Document {
            firmware_type: 3,
            vehicle_type: 2,
            items: Vec::new(),
            ..section()
        };
        let takeoff = insert_simple(&copter, CMD_NAV_TAKEOFF, 47.63, -122.08, 1, &QT_DEFAULTS);
        let waypoints = (0..4).fold(takeoff, |doc, n| {
            insert_simple(
                &doc,
                CMD_NAV_WAYPOINT,
                47.631 + f64::from(n) * 0.001,
                -122.08,
                -1,
                &QT_DEFAULTS,
            )
        });
        let dropped = [6, 5, 4, 3].iter().fold(waypoints, |doc, at| {
            insert_drop(&doc, *at, &QT_DEFAULTS, &servos)
                .expect("a waypoint stands before each drop")
        });
        let set_servos: Vec<(f64, f64)> = dropped
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Simple(s)
                    if s.command == i64::from(crate::payloadcheck::CMD_DO_SET_SERVO) =>
                {
                    Some((
                        s.params[0].unwrap_or_default(),
                        s.params[1].unwrap_or_default(),
                    ))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            set_servos,
            [
                (12.0, 1100.0),
                (11.0, 1100.0),
                (10.0, 1100.0),
                (9.0, 1100.0)
            ],
            "each drop takes the lowest bay no drop uses yet, at its release PWM"
        );
        assert_eq!(
            drop_after(&dropped, 2, &servos),
            Some(Some(3)),
            "the map's Drop label finds a servo drop"
        );
        assert_eq!(
            drop_after(&dropped, 2, &[]),
            Some(None),
            "with one gripper a DO_SET_SERVO is no drop"
        );
        assert_eq!(
            insert_drop(&dropped, -1, &QT_DEFAULTS, &servos).unwrap_err(),
            EVERY_BAY_DROPS
        );
        let saved = load(&save(&dropped).to_string(), 2).expect("a saved plan loads");
        assert_eq!(
            saved
                .items
                .iter()
                .filter_map(|item| servo_drop_bay(item, &servos))
                .collect::<Vec<_>>(),
            [4, 3, 2, 1],
            "a .plan keeps output and PWM, so every drop is still its bay's after a reload"
        );
        assert!(
            matches!(&saved.items[1], Item::Simple(s) if s.command == CMD_NAV_WAYPOINT && s.params[0] == Some(crate::payloadcheck::DROP_HOLD_SECONDS)),
            "a multirotor holds before a servo drop as before a gripper drop"
        );
    }

    #[test]
    fn a_waypoint_knows_the_drop_that_follows_it() {
        let copter = Document {
            firmware_type: 3,
            vehicle_type: 2,
            items: Vec::new(),
            ..section()
        };
        let takeoff = insert_simple(&copter, CMD_NAV_TAKEOFF, 47.63, -122.08, 1, &QT_DEFAULTS);
        let waypoints = insert_simple(
            &insert_simple(&takeoff, CMD_NAV_WAYPOINT, 47.631, -122.08, 2, &QT_DEFAULTS),
            CMD_NAV_WAYPOINT,
            47.632,
            -122.08,
            3,
            &QT_DEFAULTS,
        );
        assert_eq!(
            drop_after(&waypoints, 2, &[]),
            Some(None),
            "a waypoint with nothing after it offers a drop"
        );
        assert_eq!(
            drop_after(&waypoints, 1, &[]),
            None,
            "the takeoff offers none"
        );
        let dropped = insert_simple(&waypoints, 211, 0.0, 0.0, 3, &QT_DEFAULTS);
        assert_eq!(
            drop_after(&dropped, 2, &[]),
            Some(Some(3)),
            "the drop at item 3 belongs to waypoint 2"
        );
        assert_eq!(
            drop_after(&dropped, 4, &[]),
            Some(None),
            "the next waypoint has its own, not waypoint 2's"
        );
        assert_eq!(drop_after(&dropped, 9, &[]), None);
    }

    #[test]
    fn a_drop_needs_a_waypoint_before_it_not_the_takeoff() {
        let copter = Document {
            firmware_type: 3,
            vehicle_type: 2,
            ..section()
        };
        let waypoint = copter
            .items
            .iter()
            .rposition(|item| matches!(item, Item::Simple(s) if s.command == CMD_NAV_WAYPOINT))
            .unwrap();
        assert!(
            drop_has_a_place(&copter, waypoint + 1),
            "after a waypoint the drop falls over it"
        );
        assert!(
            !drop_has_a_place(
                &Document {
                    items: Vec::new(),
                    ..copter.clone()
                },
                0
            ),
            "an empty plan has nowhere to drop"
        );
        let takeoff_only = insert_simple(
            &Document {
                items: Vec::new(),
                ..copter.clone()
            },
            CMD_NAV_TAKEOFF,
            47.63,
            -122.08,
            1,
            &QT_DEFAULTS,
        );
        assert!(
            !drop_has_a_place(&takeoff_only, 1),
            "straight after takeoff the drop would fall over the launch point"
        );
        let dropped = insert_simple(&takeoff_only, 211, 0.0, 0.0, 2, &QT_DEFAULTS);
        assert!(
            !drop_has_a_place(&dropped, 2),
            "a second drop looks past the first to the takeoff"
        );
    }

    #[test]
    fn a_waypoint_aims_photographs_waits_and_then_drops_from_its_servo_in_that_order() {
        let servos = [crate::payloadcheck::Servo {
            output: 4,
            release: 2000.0,
            hold: 1500.0,
        }];
        let copter = Document {
            firmware_type: 3,
            vehicle_type: 2,
            items: Vec::new(),
            ..section()
        };
        let takeoff = insert_simple(&copter, CMD_NAV_TAKEOFF, 47.63, -122.08, 1, &QT_DEFAULTS);
        let waypoint = insert_simple(
            &takeoff,
            CMD_NAV_WAYPOINT,
            47.631,
            -122.08,
            -1,
            &QT_DEFAULTS,
        );
        let aimed = [
            ("specifyGimbal", json!(true)),
            ("gimbalPitch", json!(-60.0)),
            ("cameraAction", json!(6)),
        ]
        .iter()
        .try_fold(waypoint, |doc, (property, value)| {
            set_camera(&doc, 2, property, value)
        })
        .unwrap();
        let flown = |doc: &Document| -> Vec<(i64, Option<f64>, Option<f64>)> {
            doc.items
                .iter()
                .flat_map(|item| match item {
                    Item::Simple(s) => std::iter::once(s)
                        .chain(s.sections.iter())
                        .map(|s| (s.command, s.params[0], s.params[1]))
                        .collect(),
                    Item::Complex { .. } => Vec::new(),
                })
                .collect()
        };
        let planned = place_drop(&aimed, 2, Some(10.0), &QT_DEFAULTS, &servos).unwrap();
        assert_eq!(
            flown(&planned)
                .iter()
                .map(|(command, ..)| *command)
                .collect::<Vec<_>>(),
            [
                CMD_NAV_TAKEOFF,
                CMD_NAV_WAYPOINT,
                CMD_DO_MOUNT_CONTROL,
                CMD_IMAGE_START_CAPTURE,
                CMD_NAV_DELAY,
                i64::from(crate::payloadcheck::CMD_DO_SET_SERVO)
            ],
            "Oleg's drop: the camera aims and shoots on arrival, the drone waits, then servo 4 opens"
        );
        let steps = flown(&planned);
        assert_eq!(
            steps[1].1,
            Some(2.0),
            "it stands still over the point before anything happens"
        );
        assert_eq!(steps[4].1, Some(10.0), "the wait asked for");
        assert_eq!(
            (steps[5].1, steps[5].2),
            (Some(4.0), Some(2000.0)),
            "servo 4 to its release PWM"
        );
        assert_eq!(drop_wait(&planned, 2, 4), 10.0);
        let quicker = place_drop(&planned, 2, Some(0.0), &QT_DEFAULTS, &servos).unwrap();
        assert_eq!(
            flown(&quicker)
                .iter()
                .map(|(command, ..)| *command)
                .collect::<Vec<_>>(),
            [
                CMD_NAV_TAKEOFF,
                CMD_NAV_WAYPOINT,
                CMD_DO_MOUNT_CONTROL,
                CMD_IMAGE_START_CAPTURE,
                i64::from(crate::payloadcheck::CMD_DO_SET_SERVO)
            ],
            "placed again with no wait, the delay goes with the old drop"
        );
        assert_eq!(drop_wait(&quicker, 2, 3), 0.0);
        assert_eq!(
            remove_drop(&planned, 2, &servos).unwrap().items.len(),
            2,
            "removing the drop takes its wait with it"
        );
    }

    fn route(vehicle_type: i64, tail: &str) -> Document {
        let text = format!(
            r#"{{"fileType":"Plan","version":1,"groundStation":"QGroundControl","mission":{{"version":2,"firmwareType":3,"vehicleType":{vehicle_type},"cruiseSpeed":15,"hoverSpeed":5,"plannedHomePosition":[47.0,8.0,500],"items":[
            {{"type":"SimpleItem","autoContinue":true,"command":22,"doJumpId":1,"frame":3,"params":[0,0,0,null,0,0,5]}},
            {{"type":"SimpleItem","autoContinue":true,"command":16,"doJumpId":2,"frame":3,"params":[0,0,0,null,47.001,8.0,21]}},
            {{"type":"SimpleItem","autoContinue":true,"command":16,"doJumpId":3,"frame":3,"params":[0,0,0,null,47.002,8.0,22]}},
            {{"type":"SimpleItem","autoContinue":true,"command":16,"doJumpId":4,"frame":3,"params":[0,0,0,null,47.003,8.0,23]}}{tail}]}},"geoFence":{{"version":2,"polygons":[],"circles":[]}},"rallyPoints":{{"version":2,"points":[]}}}}"#
        );
        load(&text, 2).unwrap()
    }

    fn flown_commands(doc: &Document) -> Vec<(i64, Option<f64>, Option<f64>)> {
        doc.items
            .iter()
            .filter_map(|item| match item {
                Item::Simple(s) => Some((s.command, s.params[0], s.params[1])),
                Item::Complex { .. } => None,
            })
            .collect()
    }

    const SERVO: i64 = crate::payloadcheck::CMD_DO_SET_SERVO as i64;

    fn bay(output: i64, release: f64) -> crate::payloadcheck::Servo {
        crate::payloadcheck::Servo {
            output,
            release,
            hold: 1100.0,
        }
    }

    #[test]
    fn a_servo_is_the_drop_where_it_first_opens_a_payload_bay_and_any_other_servo_is_a_plain_set_servo()
     {
        let nine = [bay(9, 1900.0)];
        let first = place_drop(&route(2, ""), 2, Some(5.0), &QT_DEFAULTS, &nine).unwrap();
        let four = [bay(4, 1500.0)];
        let both = place_drop(&first, 5, None, &QT_DEFAULTS, &four).unwrap();
        assert_eq!(
            flown_commands(&both)
                .iter()
                .filter(|(command, ..)| *command == SERVO)
                .map(|(_, output, pwm)| (*output, *pwm))
                .collect::<Vec<_>>(),
            [(Some(9.0), Some(1900.0)), (Some(4.0), Some(1500.0))]
        );
        assert_eq!(
            (drop_after(&both, 2, &four), drop_after(&both, 5, &four)),
            (Some(None), Some(Some(6))),
            "with the bay on servo 4, servo 9 opens no bay: it is a plain servo, and the drop is the one on servo 4"
        );
        assert_eq!(
            crate::missionitems::document_reads(&both, -1).unwrap()[4]["commandName"],
            "Set servo 9 to 1900",
            "a servo that drops nothing reads by its output and PWM"
        );
        let again = set_param(&both, 4, 1, 4.0).unwrap();
        assert_eq!(
            (drop_after(&again, 2, &four), drop_after(&again, 5, &four)),
            (Some(Some(4)), Some(None)),
            "moved onto servo 4 at the first waypoint, it opens the bay first and is the drop; the later one opens an empty bay, so it is a plain servo"
        );
        assert_eq!(
            crate::missionitems::document_reads(&again, -1).unwrap()[6]["commandName"],
            "Set servo 4 to 1500"
        );
        assert_eq!(drop_wait(&both, 2, 4), 5.0);
        let moved = set_drop_servo(&both, 5, None, Some(1200.0), &four).unwrap();
        assert_eq!(
            (
                drop_release(&moved, 6, &four).map(|it| (it.output, it.pwm)),
                drop_release(&moved, 4, &four).map(|it| (it.output, it.pwm))
            ),
            (Some((4.0, 1200.0)), Some((9.0, 1900.0))),
            "editing one waypoint's drop writes that item and no other"
        );
        assert_eq!(
            set_drop_servo(&both, 5, Some(40.0), None, &four).unwrap_err(),
            "A servo output is a whole number from 1 to 32."
        );
        assert_eq!(
            set_drop_servo(&both, 5, None, Some(3000.0), &four).unwrap_err(),
            "A servo PWM is from 500 to 2500."
        );
        assert_eq!(
            set_drop_servo(&both, 7, Some(4.0), None, &four).unwrap_err(),
            NO_DROP_HERE
        );
        let closing = set_param(&both, 6, 2, 1100.0).unwrap();
        assert_eq!(
            drop_after(&closing, 5, &four),
            Some(None),
            "a servo sent back to the bay's hold PWM closes it, which is no drop"
        );
        let two_bays = [bay(9, 1900.0), bay(4, 1500.0)];
        assert_eq!(
            (4..=6)
                .filter_map(|at| drop_release(&both, at, &two_bays)?.bay)
                .collect::<Vec<_>>(),
            [1, 2],
            "with a servo per bay each drop names the bay its own output belongs to"
        );
        assert_eq!(
            (
                drop_after(&both, 2, &two_bays),
                drop_after(&both, 5, &two_bays)
            ),
            (Some(Some(4)), Some(Some(6))),
            "and each is its waypoint's drop"
        );
        let gripper = place_drop(&route(2, ""), 2, None, &QT_DEFAULTS, &[]).unwrap();
        assert_eq!(drop_release(&gripper, 3, &[]), None);
        assert_eq!(
            set_drop_servo(&gripper, 2, Some(4.0), None, &[]).unwrap_err(),
            NOT_A_SERVO_DROP
        );
    }

    #[test]
    fn the_core_finds_a_waypoints_drop_itself_and_says_why_it_cannot_place_or_remove_one() {
        let servos = [bay(4, 1500.0)];
        let plan = route(2, "");
        assert_eq!(
            place_drop(&plan, 1, None, &QT_DEFAULTS, &servos).unwrap_err(),
            NOT_A_WAYPOINT,
            "the takeoff takes no drop"
        );
        assert_eq!(
            place_drop(&plan, 9, None, &QT_DEFAULTS, &servos).unwrap_err(),
            NOT_A_WAYPOINT
        );
        assert_eq!(remove_drop(&plan, 2, &servos).unwrap_err(), NO_DROP_HERE);
        let dropped = place_drop(&plan, 2, None, &QT_DEFAULTS, &servos).unwrap();
        assert_eq!(
            place_drop(&dropped, 4, None, &QT_DEFAULTS, &servos).unwrap_err(),
            EVERY_BAY_DROPS,
            "one bay drops once, and the second waypoint is told so"
        );
        assert_eq!(
            place_drop(&dropped, 2, Some(601.0), &QT_DEFAULTS, &servos).unwrap_err(),
            WAIT_OUT_OF_RANGE
        );
        assert_eq!(
            place_drop(&dropped, 2, Some(-1.0), &QT_DEFAULTS, &servos).unwrap_err(),
            WAIT_OUT_OF_RANGE
        );
        assert_eq!(
            remove_drop(&dropped, 2, &servos).unwrap(),
            Document {
                items: plan
                    .items
                    .iter()
                    .enumerate()
                    .map(|(at, item)| match (at, item) {
                        (1, Item::Simple(held)) => Item::Simple(Simple {
                            params: std::array::from_fn(|i| match i {
                                0 => Some(crate::payloadcheck::DROP_HOLD_SECONDS),
                                _ => held.params[i],
                            }),
                            ..held.clone()
                        }),
                        _ => item.clone(),
                    })
                    .collect(),
                ..plan.clone()
            },
            "a drop with no wait leaves only the hold the waypoint took for it"
        );
        let lit = insert_simple(&dropped, CMD_DO_CHANGE_SPEED, 0.0, 0.0, 3, &QT_DEFAULTS);
        assert_eq!(
            drop_after(&lit, 2, &servos),
            Some(Some(4)),
            "another command between the waypoint and its drop does not hide the drop"
        );
        let waited = place_drop(&lit, 2, Some(8.0), &QT_DEFAULTS, &servos).unwrap();
        assert_eq!(
            flown_commands(&waited)[2..5]
                .iter()
                .map(|(command, ..)| *command)
                .collect::<Vec<_>>(),
            [CMD_DO_CHANGE_SPEED, CMD_NAV_DELAY, SERVO],
            "the wait goes straight before the drop"
        );
        assert_eq!(
            flown_commands(&remove_drop(&waited, 2, &servos).unwrap())[2].0,
            CMD_DO_CHANGE_SPEED,
            "removing the drop leaves the other command and takes the wait"
        );
        let by_hand = set_param(
            &insert_simple(&dropped, CMD_NAV_DELAY, 0.0, 0.0, 3, &QT_DEFAULTS),
            3,
            1,
            12.0,
        )
        .unwrap();
        assert_eq!(
            drop_wait(&by_hand, 2, 4),
            12.0,
            "a delay the pilot put before the drop by hand is that drop's wait"
        );
        assert_eq!(
            remove_drop(&by_hand, 2, &servos).unwrap().items.len(),
            plan.items.len()
        );
        let plane = route(1, "");
        let flown_through = place_drop(&plane, 2, None, &QT_DEFAULTS, &servos).unwrap();
        assert_eq!(
            place_drop(&flown_through, 2, Some(5.0), &QT_DEFAULTS, &servos).unwrap_err(),
            WAIT_NEEDS_A_HOVER,
            "a plane cannot wait over the point, and is told so instead of losing the wait"
        );
        assert!(!wait_offered(&plane) && wait_offered(&plan));
    }

    #[test]
    fn a_waypoint_takes_its_wait_and_drop_along_when_it_moves_and_when_it_is_deleted() {
        let servos = [bay(4, 1500.0)];
        let plan = place_drop(&route(2, ""), 3, Some(10.0), &QT_DEFAULTS, &servos).unwrap();
        let heights = |doc: &Document| -> Vec<(i64, Option<f64>)> {
            doc.items
                .iter()
                .filter_map(|item| match item {
                    Item::Simple(s) => Some((s.command, s.altitude.as_ref().map(|a| a.altitude))),
                    Item::Complex { .. } => None,
                })
                .collect()
        };
        let waypoint = |height: f64| (CMD_NAV_WAYPOINT, Some(height));
        let attached = [(CMD_NAV_DELAY, None), (SERVO, None)];
        assert_eq!(
            heights(&plan)[1..],
            [
                waypoint(21.0),
                waypoint(22.0),
                attached[0],
                attached[1],
                waypoint(23.0)
            ]
        );
        let (earlier, landed) = move_item(&plan, 3, false).unwrap();
        assert_eq!(
            (heights(&earlier)[1..].to_vec(), landed),
            (
                vec![
                    waypoint(22.0),
                    attached[0],
                    attached[1],
                    waypoint(21.0),
                    waypoint(23.0)
                ],
                2
            ),
            "moved up, the waypoint still waits and drops over itself, not over the one it passed"
        );
        let (later, landed) = move_item(&plan, 3, true).unwrap();
        assert_eq!(
            (heights(&later)[1..].to_vec(), landed),
            (
                vec![
                    waypoint(21.0),
                    waypoint(23.0),
                    waypoint(22.0),
                    attached[0],
                    attached[1]
                ],
                4
            )
        );
        let (passed, landed) = move_item(&plan, 2, true).unwrap();
        assert_eq!(
            (heights(&passed)[1..].to_vec(), landed),
            (heights(&earlier)[1..].to_vec(), 5),
            "a waypoint moved down goes past the next waypoint and everything attached to it"
        );
        assert_eq!(
            heights(&remove_with_riders(&plan, 3).unwrap())[1..],
            [waypoint(21.0), waypoint(23.0)],
            "a deleted waypoint leaves no wait and no drop behind for the waypoint before it"
        );
        assert_eq!(
            heights(&remove_with_riders(&plan, 5).unwrap())[1..],
            [waypoint(21.0), waypoint(22.0), attached[0], waypoint(23.0)],
            "an attached command is deleted alone"
        );
        let (lifted, landed) = move_item(&plan, 5, false).unwrap();
        assert_eq!(
            (heights(&lifted)[3..5].to_vec(), landed),
            (vec![attached[1], attached[0]], 4),
            "an attached command moves one place"
        );
    }

    fn plan_with(vehicle_type: i64, commands: &[i64]) -> Document {
        let items: Vec<String> = commands
            .iter()
            .enumerate()
            .map(|(at, command)| match [16, 21, 22, 84, 19].contains(command) {
                true => format!(
                    r#"{{"type":"SimpleItem","autoContinue":true,"command":{command},"doJumpId":{},"frame":3,"params":[0,0,0,null,{},8.0,30]}}"#,
                    at + 1,
                    47.0 + at as f64 * 0.001
                ),
                false => format!(
                    r#"{{"type":"SimpleItem","autoContinue":true,"command":{command},"doJumpId":{},"frame":2,"params":[1,5,-1,0,0,0,0]}}"#,
                    at + 1
                ),
            })
            .collect();
        load(
            &format!(
                r#"{{"fileType":"Plan","version":1,"groundStation":"QGroundControl","mission":{{"version":2,"firmwareType":3,"vehicleType":{vehicle_type},"cruiseSpeed":15,"hoverSpeed":5,"plannedHomePosition":[47.0,8.0,500],"items":[{}]}},"geoFence":{{"version":2,"polygons":[],"circles":[]}},"rallyPoints":{{"version":2,"points":[]}}}}"#,
                items.join(",")
            ),
            2,
        )
        .unwrap()
    }

    #[test]
    fn only_a_takeoff_that_starts_the_route_keeps_new_items_behind_it() {
        let rover_leg_first = plan_with(2, &[16, 16, 16, 22, 16]);
        assert_eq!(
            (1..=5)
                .map(|at| past_takeoff(&rover_leg_first, "waypoint", at))
                .collect::<Vec<_>>(),
            [1, 2, 3, 4, 5],
            "a takeoff in the middle of the route moves nothing: a waypoint goes after the row that was selected"
        );
        let two_takeoffs = plan_with(2, &[22, 16, 21, 22, 16]);
        assert_eq!(
            [1, 2, 3, 4].map(|at| past_takeoff(&two_takeoffs, "waypoint", at)),
            [2, 2, 3, 4],
            "the second takeoff is an ordinary row; only the launch is sent past the first"
        );
        let servo_first = plan_with(2, &[183, 22, 16]);
        assert_eq!(
            [1, 2, 3].map(|at| past_takeoff(&servo_first, "waypoint", at)),
            [3, 3, 3],
            "a servo command ahead of the takeoff is no place, so the takeoff still starts the route"
        );
        assert_eq!(past_takeoff(&plan_with(20, &[84, 16]), "waypoint", 1), 2);
        assert_eq!(past_takeoff(&plan_with(2, &[16, 16]), "waypoint", 1), 1);
    }

    #[test]
    fn a_command_picked_on_a_waypoint_goes_before_a_jump_and_replaces_a_landing() {
        let looped = plan_with(2, &[22, 16, 16, 177, 16, 21]);
        let (with_servo, added) = command_at(&looped, 3, 183, &QT_DEFAULTS).unwrap();
        assert_eq!(
            (
                added,
                flown_commands(&with_servo)[2..5]
                    .iter()
                    .map(|it| it.0)
                    .collect::<Vec<_>>()
            ),
            (Some(4), vec![16, 183, 177]),
            "the servo runs on every lap, so it sits before the jump back, not after the last lap"
        );
        let (again, added) = command_at(&with_servo, 3, 178, &QT_DEFAULTS).unwrap();
        assert_eq!(
            (
                added,
                flown_commands(&again)[2..6]
                    .iter()
                    .map(|it| it.0)
                    .collect::<Vec<_>>()
            ),
            (Some(5), vec![16, 183, 178, 177]),
            "a second command follows the first, still before the jump"
        );
        let (landed, added) = command_at(&looped, 6, 183, &QT_DEFAULTS).unwrap();
        assert_eq!(
            (
                added,
                flown_commands(&landed)
                    .iter()
                    .map(|it| it.0)
                    .collect::<Vec<_>>()
            ),
            (None, vec![22, 16, 16, 177, 16, 183]),
            "nothing runs after a landing, so a command picked on it replaces it as Change command says"
        );
    }

    #[test]
    fn a_command_picked_on_a_takeoff_a_waypoint_before_a_pattern_or_a_vtol_waypoint_lands_where_it_runs()
     {
        let copter = plan_with(2, &[22, 16, 16]);
        let (on_takeoff, added) = command_at(&copter, 1, 183, &QT_DEFAULTS).unwrap();
        assert_eq!(
            (
                added,
                flown_commands(&on_takeoff)
                    .iter()
                    .map(|it| it.0)
                    .collect::<Vec<_>>()
            ),
            (Some(2), vec![22, 183, 16, 16]),
            "picked on the takeoff, the command runs once the climb is done and the takeoff stays"
        );
        let pattern = load(include_str!("../tests/fixtures/survey-upload.plan"), 2)
            .unwrap()
            .items
            .into_iter()
            .find(|item| matches!(item, Item::Complex { .. }))
            .unwrap();
        let surveyed = Document {
            items: copter.items[..2]
                .iter()
                .cloned()
                .chain([pattern, copter.items[2].clone()])
                .collect(),
            ..copter.clone()
        };
        let (with_servo, added) = command_at(&surveyed, 2, 183, &QT_DEFAULTS).unwrap();
        assert_eq!(added, Some(3));
        assert!(
            matches!(&with_servo.items[2], Item::Simple(servo) if servo.command == 183)
                && matches!(&with_servo.items[3], Item::Complex { .. }),
            "the command sits after its waypoint and ahead of the pattern, which stays whole"
        );
        let vtol = plan_with(20, &[84, 16, 16, 20]);
        let (hovering, added) = command_at(&vtol, 2, 3000, &QT_DEFAULTS).unwrap();
        assert_eq!(
            (
                added,
                flown_commands(&hovering)
                    .iter()
                    .map(|it| it.0)
                    .collect::<Vec<_>>()
            ),
            (Some(3), vec![84, 16, 3000, 16, 20]),
            "a VTOL plan takes the command at the waypoint under the tree that waypoint flies with"
        );
        assert_eq!(
            command_at(&vtol, 2, 16, &QT_DEFAULTS).map(|(_, added)| added),
            Some(None),
            "the command already there changes nothing"
        );
    }

    #[test]
    fn people_count_waypoints_and_letter_the_rest() {
        let reads =
            crate::missionitems::document_reads(&plan_with(2, &[22, 16, 183, 19, 16, 20]), -1)
                .unwrap();
        assert_eq!(
            crate::missionitems::stops(&reads),
            [None, None, Some(1), None, None, Some(2), None],
            "home, the takeoff, a servo command, a loiter and the return take no number"
        );
        assert_eq!(
            reads
                .iter()
                .map(crate::missionitems::letter)
                .collect::<Vec<_>>(),
            [Some('H'), Some('T'), None, None, Some('L'), None, Some('R')]
        );
        let lettered = |abbreviation: &str| {
            crate::missionitems::letter(
                &json!({ "abbreviation": abbreviation, "isSimpleItem": true }),
            )
        };
        assert_eq!(
            ["A", "z", "[", "_", "7", ""].map(lettered),
            [Some('A'), Some('z'), None, None, None, None]
        );
        assert_eq!(
            crate::missionitems::letter(&json!({ "abbreviation": "S", "isSimpleItem": false })),
            None,
            "a pattern is a numbered stop"
        );
    }

    #[test]
    fn ardupilot_starts_only_the_first_command_left_after_the_last_place_so_a_mission_ending_in_several_is_closed_with_a_delay()
     {
        let servos = [bay(4, 1500.0)];
        let aimed = [("specifyGimbal", json!(true)), ("cameraAction", json!(6))]
            .iter()
            .try_fold(route(2, ""), |doc, (property, value)| {
                set_camera(&doc, 4, property, value)
            })
            .unwrap();
        let dropping = place_drop(&aimed, 4, None, &QT_DEFAULTS, &servos).unwrap();
        let sent = |doc: &Document| -> Vec<(i64, Option<f64>)> {
            as_flown(doc)
                .items
                .iter()
                .flat_map(|item| match item {
                    Item::Simple(s) => std::iter::once(s)
                        .chain(s.sections.iter())
                        .map(|s| (s.command, s.params[0]))
                        .collect(),
                    Item::Complex { .. } => Vec::new(),
                })
                .skip(3)
                .collect()
        };
        assert_eq!(
            sent(&dropping),
            [
                (
                    CMD_NAV_WAYPOINT,
                    Some(crate::payloadcheck::DROP_HOLD_SECONDS)
                ),
                (CMD_DO_MOUNT_CONTROL, Some(0.0)),
                (CMD_IMAGE_START_CAPTURE, Some(0.0)),
                (SERVO, Some(4.0)),
                (CMD_NAV_DELAY, Some(SHORTEST_DELAY_SECONDS))
            ],
            "tilt, photo and drop on the last waypoint: the photo and the drop are fetched one by one while a navigation command runs, so one is added for them to run under"
        );
        assert_eq!(
            sent(&aimed).last(),
            Some(&(CMD_NAV_DELAY, Some(SHORTEST_DELAY_SECONDS))),
            "a tilt and a photo alone need it too"
        );
        let waited = place_drop(&aimed, 4, Some(10.0), &QT_DEFAULTS, &servos).unwrap();
        assert_eq!(
            sent(&waited)[3..],
            [(CMD_NAV_DELAY, Some(10.0)), (SERVO, Some(4.0))],
            "after a wait the drop is the one command left, and that one is started before the mission ends"
        );
        assert_eq!(
            sent(&place_drop(&route(2, ""), 4, None, &QT_DEFAULTS, &servos).unwrap()).last(),
            Some(&(SERVO, Some(4.0))),
            "a drop alone on the last waypoint runs as it is"
        );
        let returning = insert_simple(
            &dropping,
            CMD_NAV_RETURN_TO_LAUNCH,
            0.0,
            0.0,
            -1,
            &QT_DEFAULTS,
        );
        let unchanged = |doc: &Document| save(&as_flown(doc)) == save(doc);
        assert!(
            unchanged(&returning),
            "with a return after them the commands run on the way home"
        );
        let px4 = Document {
            firmware_type: 12,
            ..dropping.clone()
        };
        assert!(unchanged(&px4), "this is ArduPilot's mission runner");
        let no_route = Document {
            items: Vec::new(),
            ..dropping.clone()
        };
        assert!(unchanged(&no_route));
    }

    #[test]
    fn a_delay_that_would_end_at_once_is_held_a_second_so_the_commands_queued_before_it_all_run() {
        let servos = [bay(4, 1500.0)];
        let dropped = place_drop(&route(2, ""), 2, None, &QT_DEFAULTS, &servos).unwrap();
        assert_eq!(
            place_drop(&dropped, 2, Some(0.5), &QT_DEFAULTS, &servos).unwrap_err(),
            WAIT_TOO_SHORT,
            "ArduPilot drops the commands still queued when the next navigation command completes, so a drop's wait is a second or none"
        );
        let aimed = [("specifyGimbal", json!(true)), ("cameraAction", json!(6))]
            .iter()
            .try_fold(dropped, |doc, (property, value)| {
                set_camera(&doc, 2, property, value)
            })
            .unwrap();
        let by_hand = |seconds: f64| {
            set_param(
                &insert_simple(&aimed, CMD_NAV_DELAY, 0.0, 0.0, 3, &QT_DEFAULTS),
                3,
                1,
                seconds,
            )
            .unwrap()
        };
        let delay_sent = |doc: &Document| match &as_flown(doc).items[2] {
            Item::Simple(delay) => (delay.command, delay.params[0]),
            Item::Complex { .. } => (0, None),
        };
        assert_eq!(
            delay_sent(&by_hand(0.2)),
            (CMD_NAV_DELAY, Some(SHORTEST_DELAY_SECONDS)),
            "a tilt and a photo wait before this delay, and at 0.2 s the photo would be skipped"
        );
        assert_eq!(delay_sent(&by_hand(5.0)), (CMD_NAV_DELAY, Some(5.0)));
        let bare = set_param(
            &insert_simple(&route(2, ""), CMD_NAV_DELAY, 0.0, 0.0, 3, &QT_DEFAULTS),
            3,
            1,
            0.2,
        )
        .unwrap();
        assert!(
            save(&as_flown(&bare)) == save(&bare),
            "with nothing queued before it a short delay is left as the pilot wrote it"
        );
    }

    fn kinds(
        doc: &Document,
        waypoint: usize,
        servos: &[crate::payloadcheck::Servo],
    ) -> Vec<StepKind> {
        steps(doc, waypoint, servos)
            .unwrap()
            .iter()
            .map(|step| step.kind)
            .collect()
    }

    fn sent_after(doc: &Document, from: usize) -> Vec<(i64, Option<f64>, Option<f64>)> {
        doc.items[from..]
            .iter()
            .flat_map(|item| match item {
                Item::Simple(s) => std::iter::once(s)
                    .chain(s.sections.iter())
                    .map(|s| (s.command, s.params[0], s.params[1]))
                    .collect(),
                Item::Complex { .. } => Vec::new(),
            })
            .collect()
    }

    fn olegs_first_waypoint(servos: &[crate::payloadcheck::Servo]) -> Document {
        let aimed = [
            ("specifyGimbal", json!(true)),
            ("gimbalPitch", json!(0.0)),
            ("gimbalYaw", json!(60.0)),
            ("cameraAction", json!(6)),
        ]
        .iter()
        .try_fold(route(2, ""), |doc, (property, value)| {
            set_camera(&doc, 2, property, value)
        })
        .unwrap();
        let dropping = place_drop(&aimed, 2, None, &QT_DEFAULTS, servos).unwrap();
        set_param(&dropping, 2, 1, 10.0).unwrap()
    }

    #[test]
    fn the_pilot_orders_a_waypoints_actions_and_the_mission_items_follow_that_order() {
        let servos = [bay(4, 1500.0)];
        let first = olegs_first_waypoint(&servos);
        assert_eq!(
            kinds(&first, 2, &servos),
            [
                StepKind::Hover,
                StepKind::Gimbal,
                StepKind::Camera,
                StepKind::Drop
            ],
            "as added: hold on arrival, aim, shoot, drop"
        );
        let aim_first = move_action(&first, 2, 0, 1, &servos).unwrap();
        let ordered = move_action(&aim_first, 2, 1, 2, &servos).unwrap();
        assert_eq!(
            sent_after(&ordered, 1)[..5],
            [
                (CMD_NAV_WAYPOINT, Some(0.0), Some(0.0)),
                (CMD_DO_MOUNT_CONTROL, Some(0.0), Some(0.0)),
                (CMD_IMAGE_START_CAPTURE, Some(0.0), Some(0.0)),
                (CMD_NAV_DELAY, Some(10.0), Some(-1.0)),
                (SERVO, Some(4.0), Some(1500.0))
            ],
            "Oleg's waypoint: camera yaw 60 pitch 0, photo, hold 10 s, then servo 4 to 1500; a hold that is not first is a delay the aircraft flies"
        );
        assert_eq!(
            kinds(&ordered, 2, &servos),
            [
                StepKind::Gimbal,
                StepKind::Camera,
                StepKind::Wait,
                StepKind::Drop
            ]
        );
        let closing = set_param(
            &insert_simple(&ordered, SERVO, 0.0, 0.0, 7, &QT_DEFAULTS),
            7,
            1,
            4.0,
        )
        .and_then(|doc| set_param(&doc, 7, 2, 2000.0))
        .unwrap();
        let second = [
            ("specifyGimbal", json!(true)),
            ("gimbalPitch", json!(-20.0)),
            ("gimbalYaw", json!(90.0)),
        ]
        .iter()
        .try_fold(closing, |doc, (property, value)| {
            set_camera(&doc, 6, property, value)
        })
        .unwrap();
        let servo_first = move_action(&second, 6, 0, 1, &servos).unwrap();
        assert_eq!(
            sent_after(&servo_first, 5)[..3],
            [
                (CMD_NAV_WAYPOINT, Some(0.0), Some(0.0)),
                (SERVO, Some(4.0), Some(2000.0)),
                (CMD_DO_MOUNT_CONTROL, Some(-20.0), Some(0.0))
            ],
            "the next waypoint: servo to 2000, then camera yaw 90, 20 down"
        );
        let reloaded = load(&save(&servo_first).to_string(), 2).unwrap();
        assert!(
            save(&reloaded) == save(&servo_first)
                && [2, 6]
                    .iter()
                    .all(|waypoint| kinds(&reloaded, *waypoint, &servos)
                        == kinds(&servo_first, *waypoint, &servos)),
            "a saved plan reads back in the order it was written"
        );
        let tilted = set_camera(&servo_first, 6, "gimbalPitch", &json!(-35.0)).unwrap();
        assert_eq!(
            sent_after(&tilted, 5)[1..3],
            [
                (SERVO, Some(4.0), Some(2000.0)),
                (CMD_DO_MOUNT_CONTROL, Some(-35.0), Some(0.0))
            ],
            "the camera is edited where it stands in the order"
        );
        assert_eq!(
            camera_state(&run_commands(&tilted, 5)).gimbal,
            Some((-35.0, 90.0)),
            "and every reader finds it there"
        );
        let reads = crate::missionitems::document_reads(&tilted, -1).unwrap();
        assert_eq!(
            reads[6]["specifiedGimbalYaw"], 90.0,
            "the map wedge reads the yaw from a camera command that follows the servo"
        );
        let shot = set_camera(&tilted, 6, "cameraAction", &json!(6)).unwrap();
        assert_eq!(
            kinds(&shot, 6, &servos),
            [StepKind::Camera, StepKind::Command, StepKind::Gimbal],
            "a new camera action starts on arrival, and can then be moved; the servo at the second waypoint opens the bay the first one already emptied, so it is a plain servo, not a drop"
        );
        let untilted = set_camera(&shot, 6, "specifyGimbal", &json!(false)).unwrap();
        assert_eq!(
            kinds(&untilted, 6, &servos),
            [StepKind::Camera, StepKind::Command]
        );
        assert_eq!(
            move_action(&servo_first, 6, 1, 1, &servos).unwrap_err(),
            ACTION_AT_THE_END
        );
        assert_eq!(
            move_action(&servo_first, 6, 5, 6, &servos).unwrap_err(),
            NO_SUCH_ACTION
        );
        assert_eq!(
            move_action(&servo_first, 1, 0, 1, &servos).unwrap_err(),
            NOT_A_WAYPOINT
        );
    }

    #[test]
    fn every_action_can_stand_in_every_place_and_nothing_is_lost_or_added_on_the_way() {
        let servos = [bay(4, 1500.0)];
        let start = olegs_first_waypoint(&servos);
        let weight = |doc: &Document| {
            let sent = sent_after(doc, 1);
            let waited: f64 = sent
                .iter()
                .take_while(|(command, ..)| *command != CMD_NAV_WAYPOINT || sent[0].0 == *command)
                .take(6)
                .filter(|(command, ..)| [CMD_NAV_WAYPOINT, CMD_NAV_DELAY].contains(command))
                .take(2)
                .filter_map(|(_, seconds, _)| *seconds)
                .sum();
            let others: Vec<i64> = {
                let commands: Vec<i64> = sent
                    .iter()
                    .skip(1)
                    .map(|(command, ..)| *command)
                    .take_while(|command| *command != CMD_NAV_WAYPOINT)
                    .filter(|command| *command != CMD_NAV_DELAY)
                    .collect();
                [CMD_DO_MOUNT_CONTROL, CMD_IMAGE_START_CAPTURE, SERVO]
                    .into_iter()
                    .filter(|wanted| commands.contains(wanted))
                    .collect()
            };
            (waited, others)
        };
        let moves = [
            (3, false),
            (2, false),
            (1, false),
            (0, true),
            (1, true),
            (2, true),
            (0, true),
            (1, true),
            (3, false),
            (2, false),
            (1, false),
            (0, true),
            (2, true),
        ];
        let visited = moves
            .iter()
            .fold(vec![start.clone()], |seen, (from, later)| {
                let next = move_action(
                    seen.last().unwrap(),
                    2,
                    *from,
                    match *later {
                        true => *from + 1,
                        false => *from - 1,
                    },
                    &servos,
                )
                .unwrap();
                seen.into_iter().chain([next]).collect()
            });
        visited.iter().for_each(|doc| {
            assert_eq!(
                weight(doc),
                (
                    10.0,
                    vec![CMD_DO_MOUNT_CONTROL, CMD_IMAGE_START_CAPTURE, SERVO]
                ),
                "ten seconds of waiting, one aim, one photo and one drop in every order"
            );
            assert_eq!(kinds(doc, 2, &servos).len(), 4);
            let reloaded = load(&save(doc).to_string(), 2).unwrap();
            assert_eq!(
                (kinds(&reloaded, 2, &servos), save(&reloaded)),
                (kinds(doc, 2, &servos), save(doc)),
                "each order reads back as written"
            );
            assert_eq!(
                doc.items.len() - riders_after(doc, 1),
                start.items.len() - 1
            );
        });
        let orders: std::collections::BTreeSet<String> = visited
            .iter()
            .map(|doc| format!("{:?}", kinds(doc, 2, &servos)))
            .collect();
        assert!(orders.len() >= 8, "the moves reach many orders: {orders:?}");
        assert!(
            visited
                .iter()
                .any(|doc| kinds(doc, 2, &servos).first() == Some(&StepKind::Drop)),
            "the drop can come first"
        );
    }

    #[test]
    fn a_reordered_waypoint_moves_and_dies_whole_keeps_jumps_true_and_is_still_sent_so_it_all_runs()
    {
        let servos = [bay(4, 1500.0)];
        let jumping = insert_simple(
            &olegs_first_waypoint(&servos),
            CMD_DO_JUMP,
            0.0,
            0.0,
            -1,
            &QT_DEFAULTS,
        );
        let last = jumping.items.len();
        let to_third = set_param(&jumping, last, 1, 6.0).unwrap();
        let target = |doc: &Document| match doc.items.last() {
            Some(Item::Simple(jump)) => jump.params[0].and_then(|seq| row_of_sequence(doc, seq)),
            _ => None,
        };
        assert_eq!(
            target(&to_third),
            Some(4),
            "the jump goes to the second waypoint"
        );
        let reordered = move_action(&to_third, 2, 0, 1, &servos).unwrap();
        assert_eq!(
            target(&reordered).map(|row| match &reordered.items[row - 1] {
                Item::Simple(s) => (s.command, s.params[4]),
                Item::Complex { .. } => (0, None),
            }),
            Some((CMD_NAV_WAYPOINT, Some(47.002))),
            "a hold turned into a delay adds a mission item, and the jump still lands on the same waypoint"
        );
        let (moved, landed) = move_item(&reordered, 2, true).unwrap();
        assert_eq!(
            (kinds(&moved, landed, &servos), kinds(&moved, 2, &servos)),
            (kinds(&reordered, 2, &servos), Vec::new()),
            "moved down the list, the waypoint keeps its actions in their order"
        );
        let gone = remove_with_riders(&reordered, 2).unwrap();
        assert_eq!(
            flown_commands(&gone)
                .iter()
                .map(|it| it.0)
                .collect::<Vec<_>>(),
            [
                CMD_NAV_TAKEOFF,
                CMD_NAV_WAYPOINT,
                CMD_NAV_WAYPOINT,
                CMD_DO_JUMP
            ]
        );
        let ending = move_action(
            &move_action(&olegs_first_waypoint(&servos), 2, 0, 1, &servos).unwrap(),
            2,
            2,
            3,
            &servos,
        )
        .unwrap();
        let at_the_end = Document {
            items: ending.items[..1 + 1 + riders_after(&ending, 1)].to_vec(),
            ..ending.clone()
        };
        assert_eq!(
            sent_after(&as_flown(&at_the_end), 1)
                .iter()
                .map(|(command, seconds, _)| (*command, *seconds))
                .collect::<Vec<_>>(),
            [
                (CMD_NAV_WAYPOINT, Some(0.0)),
                (CMD_DO_MOUNT_CONTROL, Some(0.0)),
                (CMD_NAV_DELAY, Some(10.0)),
                (SERVO, Some(4.0)),
                (CMD_IMAGE_START_CAPTURE, Some(0.0)),
                (CMD_NAV_DELAY, Some(SHORTEST_DELAY_SECONDS))
            ],
            "aim, hold, then drop and photo on the last waypoint: two commands are left after the hold, so the mission is closed with a delay for both to run"
        );
    }

    #[test]
    fn every_plan_in_the_repository_loads_and_is_sent_as_it_was_written_when_nothing_is_reordered()
    {
        let plans: Vec<std::path::PathBuf> = ["../test/MissionManager", "tests/fixtures"]
            .iter()
            .flat_map(|folder| {
                std::fs::read_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(folder))
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|entry| entry.path())
            })
            .filter(|path| path.extension().is_some_and(|suffix| suffix == "plan"))
            .collect();
        let sent = |plan: &Value| {
            crate::planitems::flatten(plan).ok().map(|items| {
                items
                    .iter()
                    .map(|item| {
                        (
                            item.command,
                            item.frame,
                            item.params.map(|value| format!("{value:.7}")),
                        )
                    })
                    .collect::<Vec<_>>()
            })
        };
        let checked: Vec<String> = plans
            .iter()
            .filter_map(|path| {
                let text = std::fs::read_to_string(path).ok()?;
                let written = sent(&serde_json::from_str(&text).ok()?)?;
                let loaded = load(&text, 2).ok()?;
                let name = path.file_name()?.to_string_lossy().to_string();
                assert_eq!(
                    sent(&save(&loaded)),
                    Some(written),
                    "{name} is uploaded item for item as the file holds it"
                );
                let again = load(&save(&loaded).to_string(), 2).ok()?;
                assert!(
                    save(&again) == save(&loaded),
                    "{name} reads back the same after a save"
                );
                Some(name)
            })
            .collect();
        assert!(
            checked.len() >= 5,
            "the fixtures were found and checked: {checked:?}"
        );
    }

    #[test]
    fn a_camera_aimed_above_the_horizon_keeps_its_angle_from_the_editor_to_the_map_and_back_from_a_file()
     {
        let servos = [bay(4, 1500.0)];
        let raised = set_camera(
            &olegs_first_waypoint(&servos),
            2,
            "gimbalPitch",
            &json!(20.0),
        )
        .unwrap();
        let pitch_fact = |doc: &Document| {
            camera_section(&run_commands(doc, 1))["facts"]
                .as_array()
                .and_then(|facts| {
                    facts
                        .iter()
                        .find(|fact| fact["property"] == "gimbalPitch")
                        .cloned()
                })
                .unwrap_or(Value::Null)
        };
        assert_eq!(
            (
                pitch_fact(&raised)["value"].clone(),
                pitch_fact(&raised)["userMax"].clone()
            ),
            (json!(20.0), json!(90.0)),
            "the editor keeps 20 degrees up and offers up to straight up"
        );
        assert!(
            sent_after(&raised, 1).contains(&(CMD_DO_MOUNT_CONTROL, Some(20.0), Some(0.0))),
            "the mission item sent to the aircraft aims 20 degrees up"
        );
        assert_eq!(
            crate::missionitems::document_reads(&raised, -1).unwrap()[2]["specifiedGimbalPitch"],
            20.0,
            "the map reads the same angle"
        );
        let reloaded = load(&save(&raised).to_string(), 2).unwrap();
        assert_eq!(
            camera_state(&run_commands(&reloaded, 1)).gimbal,
            Some((20.0, 60.0)),
            "read back from a file, the camera still looks 20 degrees up"
        );
        assert_eq!(
            camera_state(&run_commands(
                &set_camera(&raised, 2, "gimbalPitch", &json!(120.0)).unwrap(),
                1
            ))
            .gimbal,
            Some((90.0, 60.0)),
            "past straight up, it stops at straight up"
        );
    }

    #[test]
    fn a_dragged_action_lands_in_the_place_it_is_dropped_in_one_move_and_unflyable_places_are_refused()
     {
        let servos = [bay(4, 1500.0)];
        let first = olegs_first_waypoint(&servos);
        let dropped_first = move_action(&first, 2, 3, 0, &servos).unwrap();
        assert_eq!(
            kinds(&dropped_first, 2, &servos),
            [
                StepKind::Drop,
                StepKind::Wait,
                StepKind::Gimbal,
                StepKind::Camera
            ],
            "the drop dragged from last to first passes three actions in one move, and the hold it pushes off first is now a wait"
        );
        assert_eq!(
            sent_after(&dropped_first, 1)[..5],
            [
                (CMD_NAV_WAYPOINT, Some(0.0), Some(0.0)),
                (SERVO, Some(4.0), Some(1500.0)),
                (CMD_NAV_DELAY, Some(10.0), Some(-1.0)),
                (CMD_DO_MOUNT_CONTROL, Some(0.0), Some(0.0)),
                (CMD_IMAGE_START_CAPTURE, Some(0.0), Some(0.0))
            ]
        );
        let places = |doc: &Document| -> Vec<Vec<usize>> {
            action_places(doc, 2, &servos)
                .unwrap()
                .into_iter()
                .map(|(to, _)| to)
                .collect()
        };
        assert_eq!(
            places(&first),
            vec![vec![0, 1, 2, 3]; 4],
            "on a multirotor every action may go anywhere: a ten second hold becomes a wait"
        );
        let brief = set_param(&first, 2, 1, 0.5).unwrap();
        let hold_refused = |place| RefusedPlace {
            place,
            reason: HOLD_STAYS_FIRST,
            about: 0,
        };
        assert_eq!(
            action_places(&brief, 2, &servos).unwrap()[0],
            (
                vec![0],
                vec![hold_refused(1), hold_refused(2), hold_refused(3)]
            ),
            "a hold shorter than a second cannot become a wait, so it may not leave first place"
        );
        assert_eq!(
            action_places(&brief, 2, &servos).unwrap()[2].1,
            vec![hold_refused(0)],
            "the photo held over first place is refused for the hold's sake, and the hold is the row the reason is about"
        );
        assert_eq!(
            places(&brief)[1..],
            [vec![1, 2, 3], vec![1, 2, 3], vec![1, 2, 3]],
            "and nothing may be put above it"
        );
        assert_eq!(
            move_action(&brief, 2, 2, 0, &servos).unwrap_err(),
            HOLD_STAYS_FIRST
        );
        let moded = set_camera(&first, 2, "specifyCameraMode", &json!(true)).unwrap();
        let camera_at = kinds(&moded, 2, &servos)
            .iter()
            .position(|kind| *kind == StepKind::Camera)
            .unwrap();
        let mode_at = kinds(&moded, 2, &servos)
            .iter()
            .position(|kind| *kind == StepKind::Mode)
            .unwrap();
        assert_eq!(
            move_action(&moded, 2, mode_at, camera_at, &servos).unwrap_err(),
            MODE_BEFORE_CAMERA,
            "a camera mode dropped after the photo it is for would leave the photo in the old mode"
        );
        let refusal_at = |from: usize, to: usize| {
            action_places(&moded, 2, &servos).unwrap()[from]
                .1
                .iter()
                .find(|refused| refused.place == to)
                .map(|refused| refused.about)
        };
        assert_eq!(
            (
                refusal_at(mode_at, camera_at),
                refusal_at(camera_at, mode_at)
            ),
            (Some(camera_at), Some(mode_at)),
            "a refusal points at the other row of the pair: the photo when the mode is dragged past it, the mode when the photo is dragged above it"
        );
        assert!(
            mode_at < camera_at && !places(&moded)[mode_at].contains(&camera_at),
            "a mode added later is set before the photo, and is not offered a place after it"
        );
        assert_eq!(
            move_action(&first, 2, 1, 1, &servos).unwrap_err(),
            ACTION_AT_THE_END
        );
        assert_eq!(
            move_action(&first, 2, 1, 4, &servos).unwrap_err(),
            NO_SUCH_ACTION
        );
    }

    #[test]
    fn a_wait_is_retimed_from_its_own_row_within_a_second_and_ten_minutes() {
        let servos = [bay(4, 1500.0)];
        let ordered = move_action(&olegs_first_waypoint(&servos), 2, 0, 2, &servos).unwrap();
        let wait_at = kinds(&ordered, 2, &servos)
            .iter()
            .position(|kind| *kind == StepKind::Wait)
            .unwrap();
        let longer = set_wait(&ordered, 2, wait_at, 25.0, &servos).unwrap();
        assert_eq!(
            steps(&longer, 2, &servos).unwrap()[wait_at].seconds,
            Some(25.0)
        );
        assert_eq!(
            kinds(&longer, 2, &servos),
            kinds(&ordered, 2, &servos),
            "the wait keeps its place"
        );
        assert_eq!(
            [0.5, 601.0]
                .map(|seconds| set_wait(&ordered, 2, wait_at, seconds, &servos).unwrap_err()),
            [
                WAIT_SECONDS_RANGE.to_string(),
                WAIT_SECONDS_RANGE.to_string()
            ]
        );
        assert_eq!(
            set_wait(&ordered, 2, 0, 5.0, &servos).unwrap_err(),
            NOT_A_WAIT
        );
    }

    #[test]
    fn editing_a_camera_action_keeps_the_order_the_pilot_gave_the_camera_commands() {
        let servos = [bay(4, 1500.0)];
        let shot_then_aimed = [
            ("cameraAction", json!(6)),
            ("specifyGimbal", json!(true)),
            ("gimbalPitch", json!(-30.0)),
        ]
        .iter()
        .try_fold(route(2, ""), |doc, (property, value)| {
            set_camera(&doc, 2, property, value)
        })
        .unwrap();
        assert_eq!(
            kinds(&shot_then_aimed, 2, &servos),
            [StepKind::Camera, StepKind::Gimbal],
            "added photo first, then the tilt: they run in the order added"
        );
        let retilted = set_camera(&shot_then_aimed, 2, "gimbalPitch", &json!(-60.0)).unwrap();
        assert_eq!(
            sent_after(&retilted, 1)[1..3],
            [
                (CMD_IMAGE_START_CAPTURE, Some(0.0), Some(0.0)),
                (CMD_DO_MOUNT_CONTROL, Some(-60.0), Some(0.0))
            ],
            "changing the tilt does not put the tilt back in front of the photo"
        );
        assert_eq!(
            kinds(&load(&save(&retilted).to_string(), 2).unwrap(), 2, &servos),
            [StepKind::Camera, StepKind::Gimbal]
        );
    }

    #[test]
    fn a_multirotor_stops_over_the_drop_and_a_plane_flies_through() {
        let copter = Document {
            firmware_type: 3,
            vehicle_type: 2,
            ..section()
        };
        let at = copter
            .items
            .iter()
            .rposition(|item| matches!(item, Item::Simple(s) if s.command == CMD_NAV_WAYPOINT))
            .unwrap();
        let hold = |doc: &Document| match &doc.items[at] {
            Item::Simple(s) => s.params[0],
            Item::Complex { .. } => None,
        };
        let dropped = hold_before_drop(
            &insert_simple(&copter, 211, 0.0, 0.0, at as i64 + 2, &QT_DEFAULTS),
            at + 1,
            2.0,
        );
        assert_eq!(
            hold(&dropped),
            Some(2.0),
            "the waypoint before the drop holds, so the payload leaves a drone that is standing still"
        );
        let held = set_param(&copter, at + 1, 1, 5.0).unwrap();
        assert_eq!(
            hold(&hold_before_drop(
                &insert_simple(&held, 211, 0.0, 0.0, at as i64 + 2, &QT_DEFAULTS),
                at + 1,
                2.0
            )),
            Some(5.0),
            "a hold the pilot set is kept"
        );
        let plane = Document {
            vehicle_type: 1,
            ..copter
        };
        assert_eq!(
            hold(&hold_before_drop(
                &insert_simple(&plane, 211, 0.0, 0.0, at as i64 + 2, &QT_DEFAULTS),
                at + 1,
                2.0
            )),
            hold(&plane),
            "a plane cannot stop over its target"
        );
    }

    #[test]
    fn an_items_altitude_mode_sets_its_mavlink_frame_as_simple_mission_item_does() {
        let doc = section();
        let at = doc
            .items
            .iter()
            .position(|item| matches!(item, Item::Simple(s) if s.altitude.is_some()))
            .unwrap()
            + 1;
        let simple = |d: &Document| match &d.items[at - 1] {
            Item::Simple(s) => s.clone(),
            _ => unreachable!(),
        };
        let shown = simple(&doc).altitude.unwrap().altitude;
        let terrain = set_altitude_mode(&doc, at, crate::altitudemodes::TERRAIN_FRAME).unwrap();
        assert_eq!(
            (
                simple(&terrain).frame,
                simple(&terrain).altitude.unwrap().mode,
                simple(&terrain).params[6]
            ),
            (
                FRAME_GLOBAL_TERRAIN_ALT,
                crate::altitudemodes::TERRAIN_FRAME,
                Some(shown)
            )
        );
        let calc =
            set_altitude_mode(&terrain, at, crate::altitudemodes::CALC_ABOVE_TERRAIN).unwrap();
        assert_eq!(simple(&calc).frame, FRAME_GLOBAL);
        assert_eq!(
            simple(&calc).params[6],
            None,
            "_altitudeFrameChanged clears param7 until terrain answers"
        );
        let relative = set_altitude_mode(&calc, at, crate::altitudemodes::RELATIVE).unwrap();
        assert_eq!(
            (simple(&relative).frame, simple(&relative).params[6]),
            (FRAME_GLOBAL_RELATIVE_ALT, Some(shown))
        );
        assert_eq!(
            set_altitude_mode(&doc, at, crate::altitudemodes::MIXED),
            None
        );
    }

    #[test]
    fn an_item_moves_one_place_but_never_past_the_takeoff_or_off_the_ends() {
        let doc = set_altitude(&section(), 3, 77.0).unwrap();
        let heights = |d: &Document| {
            d.items
                .iter()
                .map(|item| match item {
                    Item::Simple(s) => (s.command, s.altitude.as_ref().map(|a| a.altitude)),
                    Item::Complex { .. } => (-1, None),
                })
                .collect::<Vec<_>>()
        };
        let later = move_item(&doc, 3, true).unwrap().0;
        assert_eq!(heights(&later)[3], heights(&doc)[2]);
        assert_eq!(heights(&later)[2], heights(&doc)[3]);
        assert_eq!(
            heights(&move_item(&later, 4, false).unwrap().0),
            heights(&doc)
        );
        assert_eq!(
            move_item(&doc, 2, false),
            Err(ORDER_PINNED.to_string()),
            "nothing moves above the takeoff"
        );
        assert_eq!(move_item(&doc, 1, true), Err(ORDER_PINNED.to_string()));
        assert!(move_item(&doc, doc.items.len(), true).is_err());
        assert!(
            move_item(&doc, 0, true).is_err(),
            "home is not an item to move"
        );
    }

    #[test]
    fn a_mission_read_back_keeps_the_one_altitude_frame_all_its_items_share() {
        let row = |command: i64, frame: i64, latitude: f64| Downloaded {
            frame,
            command,
            params: [0.0, 0.0, 0.0, 0.0, latitude, 8.0, 50.0],
            auto_continue: true,
        };
        let template = Document {
            firmware_type: 3,
            vehicle_type: 2,
            ..section()
        };
        let relative = [
            row(16, 0, 47.0),
            row(22, 3, 0.0),
            row(16, 3, 47.01),
            row(178, 2, 0.0),
            row(16, 3, 47.02),
            row(20, 2, 0.0),
        ];
        assert_eq!(
            from_vehicle(&relative, true, &template).global_altitude_mode,
            crate::altitudemodes::RELATIVE,
            "a route uploaded relative to launch reads back relative to launch, not as Mixed modes"
        );
        let mixed = [row(16, 0, 47.0), row(16, 3, 47.01), row(16, 0, 47.02)];
        assert_eq!(
            from_vehicle(&mixed, true, &template).global_altitude_mode,
            crate::altitudemodes::MIXED
        );
    }

    #[test]
    fn a_terrain_or_amsl_mission_read_back_keeps_its_frame_past_the_takeoff_it_climbs_from_launch()
    {
        let row = |command: i64, frame: i64, latitude: f64| Downloaded {
            frame,
            command,
            params: [0.0, 0.0, 0.0, 0.0, latitude, 8.0, 50.0],
            auto_continue: true,
        };
        let template = Document {
            firmware_type: 3,
            vehicle_type: 2,
            ..section()
        };
        let read_back = |frame: i64| {
            from_vehicle(
                &[
                    row(16, 0, 47.0),
                    row(22, 3, 0.0),
                    row(16, frame, 47.01),
                    row(16, frame, 47.02),
                ],
                true,
                &template,
            )
            .global_altitude_mode
        };
        assert_eq!(
            read_back(FRAME_GLOBAL_TERRAIN_ALT),
            crate::altitudemodes::TERRAIN_FRAME,
            "a terrain plan always climbs out relative to launch, so its takeoff does not make it Mixed modes"
        );
        assert_eq!(read_back(FRAME_GLOBAL), crate::altitudemodes::ABSOLUTE);
        let takeoff_alone = [row(16, 0, 47.0), row(22, 3, 0.0)];
        assert_eq!(
            from_vehicle(&takeoff_alone, true, &template).global_altitude_mode,
            crate::altitudemodes::RELATIVE
        );
    }

    #[test]
    fn a_plan_converted_for_a_vehicle_takes_its_types_and_drops_frames_it_cannot_fly() {
        let doc = section();
        let at = doc
            .items
            .iter()
            .position(|item| matches!(item, Item::Simple(s) if s.altitude.is_some()))
            .unwrap()
            + 1;
        let terrain = Document {
            global_altitude_mode: crate::altitudemodes::TERRAIN_FRAME,
            ..set_altitude_mode(&doc, at, crate::altitudemodes::TERRAIN_FRAME).unwrap()
        };
        let mode = |d: &Document| match &d.items[at - 1] {
            Item::Simple(s) => s.altitude.as_ref().unwrap().mode,
            _ => unreachable!(),
        };
        let apm = for_vehicle(&terrain, 3, 2).unwrap();
        assert_eq!(
            (apm.firmware_type, apm.vehicle_type, mode(&apm)),
            (3, 2, crate::altitudemodes::TERRAIN_FRAME)
        );
        assert_eq!(apm.items.len(), terrain.items.len());
        let px4 = for_vehicle(&terrain, 12, 2).unwrap();
        assert_eq!(
            (px4.firmware_type, mode(&px4), px4.global_altitude_mode),
            (
                12,
                crate::altitudemodes::CALC_ABOVE_TERRAIN,
                crate::altitudemodes::CALC_ABOVE_TERRAIN
            ),
            "PX4 flies no terrain frame, so those items keep their height above the ground another way"
        );
    }

    #[test]
    fn a_plan_is_not_converted_for_a_vehicle_that_cannot_fly_some_of_its_items() {
        let item = |command: i64, latitude: f64| json!({ "type": "SimpleItem", "command": command, "frame": 3, "doJumpId": 1, "autoContinue": true, "params": [0, 0, 0, 0, latitude, 8.0, 50] });
        let vtol = load(
            &json!({ "fileType": "Plan", "version": 1, "groundStation": "QGroundControl", "geoFence": { "version": 2, "circles": [], "polygons": [] }, "rallyPoints": { "version": 2, "points": [] }, "mission": { "firmwareType": 12, "vehicleType": 20, "plannedHomePosition": [47.0, 8.0, 0], "items": [item(84, 47.0), item(16, 47.001)] } })
            .to_string(),
            2,
        )
        .unwrap();
        assert_eq!(
            for_vehicle(&vtol, 12, 2),
            Err("The connected vehicle cannot fly item 1 (VTOL takeoff). Remove it, then convert the plan.".to_string()),
            "a copter has no VTOL takeoff, so the plan keeps its VTOL type and the mismatch warning"
        );
        assert_eq!(for_vehicle(&vtol, 12, 20).map(|d| d.vehicle_type), Ok(20));
        let landing = |kind: &str| Document {
            items: [
                vtol.items[1..].to_vec(),
                vec![Item::Complex {
                    kind: kind.to_string(),
                    json: json!({}),
                    item_count: 3,
                }],
            ]
            .concat(),
            ..vtol.clone()
        };
        assert_eq!(
            for_vehicle(&landing(crate::landingpattern::FIXED_WING_PATTERN), 3, 2),
            Err("The connected vehicle cannot fly item 2 (Landing Pattern). Remove it, then convert the plan.".to_string())
        );
        assert!(for_vehicle(&landing(crate::landingpattern::FIXED_WING_PATTERN), 3, 1).is_ok());
        assert_eq!(
            for_vehicle(
                &Document {
                    items: [
                        vtol.items.clone(),
                        landing(crate::landingpattern::VTOL_PATTERN).items[1..].to_vec()
                    ]
                    .concat(),
                    ..vtol.clone()
                },
                12,
                2
            ),
            Err("The connected vehicle cannot fly item 1 (VTOL takeoff), item 3 (Landing Pattern). Remove them, then convert the plan.".to_string())
        );
        assert!(for_vehicle(&landing(crate::landingpattern::VTOL_PATTERN), 3, 1).is_err());
        assert!(
            for_vehicle(&landing("StructureScan"), 12, 1).is_err(),
            "a plane cannot hover around a structure"
        );
        let camera_mode = Document {
            firmware_type: 12,
            vehicle_type: 2,
            ..set_camera(&section(), 0, "specifyCameraMode", &json!(true)).unwrap()
        };
        let converted = for_vehicle(&camera_mode, 3, 2).unwrap();
        assert_eq!(
            (
                converted.firmware_type,
                camera_state(&converted.settings_sections).mode
            ),
            (3, None),
            "ArduPilot takes no camera mode in a mission; the start-of-mission camera mode a survey set is the plan's own setting, folded into Launch where no one can reach it, so converting drops it instead of asking for it to be removed"
        );
    }

    #[test]
    fn changing_an_items_altitude_mode_keeps_it_at_the_same_height() {
        let doc = Document {
            home: Some([47.0, 8.0, 0.0]),
            ..section()
        };
        let at = doc
            .items
            .iter()
            .position(|item| matches!(item, Item::Simple(s) if s.altitude.is_some()))
            .unwrap()
            + 1;
        let height = |d: &Document| match &d.items[at - 1] {
            Item::Simple(s) => (s.altitude.as_ref().unwrap().altitude, s.params[6]),
            _ => unreachable!(),
        };
        let no_ground = |_: f64, _: f64| None;
        let launch = Some(481.0);
        let relative = set_altitude(
            &set_altitude_mode(&doc, at, crate::altitudemodes::RELATIVE).unwrap(),
            at,
            50.0,
        )
        .unwrap();
        let amsl = set_altitude_mode_keeping_height(
            &relative,
            at,
            crate::altitudemodes::ABSOLUTE,
            launch,
            &no_ground,
        )
        .unwrap();
        assert_eq!(
            height(&amsl),
            (531.0, Some(531.0)),
            "50 m above a 481 m launch, whatever altitude the planned home still carries"
        );
        assert_eq!(
            height(
                &set_altitude_mode_keeping_height(
                    &amsl,
                    at,
                    crate::altitudemodes::RELATIVE,
                    launch,
                    &no_ground
                )
                .unwrap()
            ),
            (50.0, Some(50.0))
        );
        let terrain = set_altitude_mode_keeping_height(
            &amsl,
            at,
            crate::altitudemodes::TERRAIN_FRAME,
            launch,
            &|_, _| Some(500.0),
        )
        .unwrap();
        assert_eq!(height(&terrain), (31.0, Some(31.0)));
        assert_eq!(
            height(
                &set_altitude_mode_keeping_height(
                    &terrain,
                    at,
                    crate::altitudemodes::CALC_ABOVE_TERRAIN,
                    launch,
                    &no_ground
                )
                .unwrap()
            )
            .0,
            31.0,
            "both terrain modes measure from the ground, so no lookup is needed"
        );
        assert_eq!(
            set_altitude_mode_keeping_height(
                &amsl,
                at,
                crate::altitudemodes::TERRAIN_FRAME,
                launch,
                &no_ground
            ),
            Err(GROUND_UNKNOWN.to_string())
        );
    }

    #[test]
    fn a_height_is_not_kept_against_a_launch_whose_ground_is_not_known() {
        let doc = Document {
            home: Some([47.0, 8.0, 0.0]),
            ..section()
        };
        let at = doc
            .items
            .iter()
            .position(|item| matches!(item, Item::Simple(s) if s.altitude.is_some()))
            .unwrap()
            + 1;
        let relative = set_altitude(
            &set_altitude_mode(&doc, at, crate::altitudemodes::RELATIVE).unwrap(),
            at,
            50.0,
        )
        .unwrap();
        assert_eq!(
            set_altitude_mode_keeping_height(
                &relative,
                at,
                crate::altitudemodes::ABSOLUTE,
                None,
                &|_, _| Some(481.0)
            ),
            Err(LAUNCH_GROUND_UNKNOWN.to_string()),
            "a new home sits at 0 until its terrain arrives, and 50 m above it would be underground at a 481 m site"
        );
        assert_eq!(
            set_altitude_mode_keeping_height(
                &Document {
                    home: None,
                    ..relative.clone()
                },
                at,
                crate::altitudemodes::ABSOLUTE,
                Some(481.0),
                &|_, _| None
            ),
            Err("Set the launch position first, so the altitude can be kept.".to_string())
        );
        assert_eq!(
            set_altitude_mode_keeping_height(
                &relative,
                at,
                crate::altitudemodes::ABSOLUTE,
                Some(f64::NAN),
                &|_, _| None
            ),
            Err(LAUNCH_GROUND_UNKNOWN.to_string())
        );
    }

    #[test]
    fn a_calc_above_terrain_waypoint_flies_terrain_plus_its_height_and_waits_without_terrain() {
        let doc = section();
        let at = doc
            .items
            .iter()
            .position(|item| matches!(item, Item::Simple(s) if s.altitude.is_some()))
            .unwrap()
            + 1;
        let simple = |d: &Document| match &d.items[at - 1] {
            Item::Simple(s) => s.clone(),
            _ => unreachable!(),
        };
        let calc = set_altitude(
            &set_altitude_mode(&doc, at, crate::altitudemodes::CALC_ABOVE_TERRAIN).unwrap(),
            at,
            50.0,
        )
        .unwrap();
        let unknown = above_terrain(&calc, &|_, _| None);
        assert_eq!(
            simple(&unknown).params[6],
            None,
            "50 m above ground is never sent as 50 m AMSL"
        );
        assert_eq!(
            unknown,
            above_terrain(&calc, &|_, _| None),
            "a plan waiting on terrain still equals itself, so undo and change checks see no change"
        );
        assert!(awaiting_terrain(&unknown), "upload waits for terrain");
        let known = above_terrain(&calc, &|_, _| Some(500.0));
        assert_eq!(
            (simple(&known).params[6], simple(&known).frame),
            (Some(550.0), FRAME_GLOBAL)
        );
        assert_eq!(
            simple(&known).altitude.unwrap().amsl_above_terrain,
            Some(550.0)
        );
        assert!(!awaiting_terrain(&known));
        assert!(
            !awaiting_terrain(&above_terrain(&doc, &|_, _| None)),
            "other altitude modes never wait"
        );
    }

    #[test]
    fn a_new_item_takes_the_altitude_mode_of_the_positioned_item_before_it() {
        let absolute = set_global_altitude_mode(&section(), crate::altitudemodes::ABSOLUTE);
        let with_absolute = insert_waypoint(&absolute, 47.634, -122.089, -1, &QT_DEFAULTS);
        let mixed = set_global_altitude_mode(&with_absolute, crate::altitudemodes::MIXED);
        let last = mixed.items.len() as i64;
        assert_eq!(
            previous_altitude_mode(&mixed, last + 1),
            Some(crate::altitudemodes::ABSOLUTE)
        );
        assert_eq!(
            previous_altitude_mode(&mixed, -1),
            None,
            "appending finds nothing, as _findPreviousAltitude walks down from index -2"
        );
    }

    #[test]
    fn move_to_previous_item_takes_the_last_positioned_item_before_this_one() {
        let apm = Document {
            firmware_type: 3,
            ..section()
        };
        let first = insert_waypoint(&apm, 47.1, 8.1, -1, &QT_DEFAULTS);
        let roi = insert_roi(&first, 47.9, 8.9, -1, &QT_DEFAULTS);
        let second = insert_waypoint(&roi, 47.2, 8.2, -1, &QT_DEFAULTS);
        let last = second.items.len() as i64;
        assert_eq!(
            previous_coordinate(&second, last),
            Some((47.1, 8.1)),
            "an ROI is a standalone coordinate and is skipped, as _setPlanViewState skips it"
        );
        assert_eq!(previous_coordinate(&second, 1), None);
    }

    #[test]
    fn a_scan_sets_the_mission_camera_to_survey_mode_unless_one_is_chosen() {
        let px4 = Document {
            firmware_type: 12,
            ..section()
        };
        let surveyed = with_survey_camera(px4.clone());
        assert_eq!(
            camera_state(&surveyed.settings_sections).mode,
            Some(CAMERA_MODE_IMAGE_SURVEY),
            "_insertComplexMissionItemWorker sets CAMERA_MODE_IMAGE_SURVEY on the mission's camera section"
        );
        let chosen = set_camera(&px4, 0, "specifyCameraMode", &json!(true)).unwrap();
        assert_eq!(
            camera_state(&with_survey_camera(chosen).settings_sections).mode,
            Some(0.0),
            "a mode the operator already set is left alone"
        );
        let photo = set_camera(&surveyed, 0, "cameraMode", &json!(CAMERA_MODE_IMAGE)).unwrap();
        let with_scan = Document {
            items: vec![Item::Complex {
                kind: "survey".into(),
                json: json!({}),
                item_count: 1,
            }],
            ..photo.clone()
        };
        assert_eq!(
            camera_state(&after_scan_removed(&with_scan, photo.clone()).settings_sections).mode,
            None,
            "removing the last area scan unspecifies an Image mode, as removeVisualItem does"
        );
        assert_eq!(
            camera_state(&after_scan_removed(&with_scan, surveyed.clone()).settings_sections).mode,
            Some(CAMERA_MODE_IMAGE_SURVEY),
            "QGC only clears a mode left at 0"
        );
    }

    #[test]
    fn a_survey_numbers_its_items_from_the_sequence_it_lands_on() {
        let fixture: Value =
            serde_json::from_str(include_str!("../tests/fixtures/survey-inserted-by-qt.json"))
                .unwrap();
        let placed = insert_complex(
            &section(),
            "survey",
            fixture["survey"].clone(),
            (47.63, -122.09),
            2,
        );
        let saved = save(&placed);
        let survey = &saved["mission"]["items"][1]["TransectStyleComplexItem"]["Items"];
        let before = &saved["mission"]["items"][0]["doJumpId"];
        assert_eq!(
            survey[0]["doJumpId"].as_i64(),
            before.as_i64().map(|n| n + 1)
        );
        let after = saved["mission"]["items"][2]["doJumpId"].as_i64().unwrap();
        assert_eq!(
            after,
            survey[0]["doJumpId"].as_i64().unwrap() + survey.as_array().unwrap().len() as i64
        );
    }

    #[test]
    fn the_plan_altitude_mode_changes_new_items_and_leaves_existing_ones() {
        let absolute = set_global_altitude_mode(&section(), crate::altitudemodes::ABSOLUTE);
        matches_qt(
            &absolute,
            include_str!("../tests/fixtures/edit-absolute-by-qt.plan"),
        );
        matches_qt(
            &insert_waypoint(&absolute, 47.634, -122.089, -1, &QT_DEFAULTS),
            include_str!("../tests/fixtures/edit-absolute-then-waypoint-by-qt.plan"),
        );
    }

    fn qt_view(text: &str) -> Vec<Value> {
        serde_json::from_str::<Value>(text).unwrap()["items"]
            .as_array()
            .unwrap()
            .clone()
    }

    fn folded_counts(doc: &Document) -> Vec<i64> {
        std::iter::once(doc.settings_sections.len() as i64)
            .chain(doc.items.iter().map(|item| match item {
                Item::Simple(s) => s.sections.len() as i64,
                Item::Complex { item_count, .. } => *item_count as i64 - 1,
            }))
            .collect()
    }

    #[test]
    fn section_commands_fold_into_the_item_before_them_as_qt_shows_them() {
        let qt = qt_view(include_str!(
            "../tests/fixtures/missionitems-sectiontest-by-qt.json"
        ));
        assert_eq!(
            folded_counts(&section()),
            qt.iter()
                .map(|i| i["foldedCommands"].as_i64().unwrap())
                .collect::<Vec<_>>(),
            "the mount control after the second waypoint is that waypoint's camera section, so Qt shows five rows where the file holds six items"
        );
        let survey = load(include_str!("../tests/fixtures/survey-upload.plan"), 2).unwrap();
        let qt = qt_view(include_str!(
            "../tests/fixtures/missionitems-survey-by-qt.json"
        ));
        assert_eq!(
            folded_counts(&survey),
            qt.iter()
                .map(|i| i["foldedCommands"].as_i64().unwrap())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_speed_change_folds_only_when_it_is_the_kind_the_airframe_flies_by() {
        let speed = |ground: f64| json!({ "type": "SimpleItem", "autoContinue": true, "command": 178, "frame": 2, "doJumpId": 3, "params": [ground, 12, -1, 0, 0, 0, 0] });
        let plan = |ground: f64, vehicle: i64| {
            json!({ "fileType": "Plan", "version": 1, "groundStation": "QGroundControl", "geoFence": { "version": 2, "circles": [], "polygons": [] }, "rallyPoints": { "version": 2, "points": [] }, "mission": { "firmwareType": 3, "vehicleType": vehicle, "plannedHomePosition": [1, 2, 0], "items": [
            { "type": "SimpleItem", "autoContinue": true, "command": 16, "frame": 3, "doJumpId": 1, "params": [0, 0, 0, 0, 1.0, 2.0, 30] },
            speed(ground),
        ] } }).to_string()
        };
        assert_eq!(
            load(&plan(1.0, 2), 2).unwrap().items.len(),
            1,
            "a multirotor flies by ground speed"
        );
        assert_eq!(
            load(&plan(0.0, 2), 2).unwrap().items.len(),
            2,
            "an airspeed change on a multirotor stays its own row"
        );
        assert_eq!(
            load(&plan(0.0, 1), 2).unwrap().items.len(),
            1,
            "a plane flies by airspeed"
        );
        let resaved = save(&load(&plan(1.0, 2), 2).unwrap());
        assert_eq!(
            resaved["mission"]["items"].as_array().unwrap().len(),
            2,
            "a folded section is still written out after its owner"
        );
        assert_eq!(resaved["mission"]["items"][1]["doJumpId"], 2);
    }

    #[test]
    fn a_speed_section_is_the_change_speed_command_shaped_for_the_airframe() {
        let with = set_speed(&section(), 2, Some(8.0)).unwrap();
        assert_eq!(specified_speed(&with, 2), Some(8.0));
        let written = save(&with);
        let items = written["mission"]["items"].as_array().unwrap();
        assert_eq!(
            (
                items[2]["command"].as_i64(),
                items[2]["params"][0].as_f64(),
                items[2]["params"][2].as_f64()
            ),
            (Some(178), Some(1.0), Some(-1.0)),
            "a multirotor's speed section is a ground speed with no throttle change, the shape SpeedSection writes"
        );
        assert_eq!(
            load(&written.to_string(), 2).unwrap().items.len(),
            section().items.len(),
            "and it folds back into its waypoint on load"
        );
        assert_eq!(
            specified_speed(&set_speed(&with, 2, None).unwrap(), 2),
            None
        );
        assert!(
            set_speed(&section(), 1, Some(8.0)).is_none(),
            "a takeoff has no speed section"
        );
        assert_eq!(
            specified_speed(&set_speed(&section(), 0, Some(6.0)).unwrap(), 0),
            Some(6.0),
            "the settings item carries the plan's opening speed"
        );
    }

    #[test]
    fn a_field_edit_moves_the_param_it_shows() {
        let edited = set_param(&section(), 2, 1, 4.0).unwrap();
        let Item::Simple(s) = &edited.items[1] else {
            panic!()
        };
        assert_eq!(s.params[0], Some(4.0));
        assert!(set_param(&section(), 2, 8, 1.0).is_none());
    }

    #[test]
    fn a_loiter_radius_lands_in_the_param_setradius_writes() {
        let loiter = |command: i64| Document {
            items: vec![Item::Simple(Simple {
                command,
                frame: FRAME_MISSION,
                params: [Some(0.0); 7],
                auto_continue: true,
                altitude: None,
                sections: Vec::new(),
            })],
            ..section()
        };
        let radius = |doc: Document| match &doc.items[0] {
            Item::Simple(s) => (s.params[1], s.params[2]),
            _ => panic!(),
        };
        assert_eq!(
            radius(set_loiter_radius(&loiter(17), 1, -80.0).unwrap()),
            (Some(0.0), Some(-80.0)),
            "a negative radius turns counter-clockwise"
        );
        assert_eq!(
            radius(set_loiter_radius(&loiter(31), 1, 60.0).unwrap()),
            (Some(60.0), Some(0.0)),
            "loiter-to-altitude keeps its radius in param 2"
        );
        assert!(
            set_loiter_radius(&loiter(CMD_NAV_WAYPOINT), 1, 60.0).is_none(),
            "a waypoint has no loiter radius"
        );
    }

    #[test]
    fn version_one_fences_and_rally_points_are_dropped_as_qt_drops_them() {
        let doc = load(
            include_str!("../../test/MissionManager/SectionTest.plan"),
            2,
        )
        .unwrap();
        assert_eq!(
            doc.fence,
            json!({ "circles": [], "polygons": [], "version": 2 })
        );
        assert_eq!(doc.rally, json!({ "points": [], "version": 2 }));
    }

    #[test]
    fn an_item_the_core_cannot_hold_refuses_the_whole_plan() {
        let plan = json!({ "fileType": "Plan", "version": 1, "groundStation": "QGroundControl", "geoFence": { "version": 2, "circles": [], "polygons": [] }, "rallyPoints": { "version": 2, "points": [] }, "mission": { "firmwareType": 12, "plannedHomePosition": [0, 0, 0], "items": [{ "type": "ComplexItem", "complexItemType": "FWLandingPattern" }] } });
        assert!(
            load(&plan.to_string(), 2)
                .unwrap_err()
                .contains("FWLandingPattern")
        );
    }

    #[test]
    fn a_jump_targets_the_sequence_of_the_item_carrying_its_do_jump_id() {
        let item = |command: i64, id: i64, p1: f64| json!({ "type": "SimpleItem", "command": command, "frame": 2, "doJumpId": id, "autoContinue": true, "params": [p1, 0, 0, 0, 0, 0, 0] });
        let plan = |items: Value| {
            json!({ "fileType": "Plan", "version": 1, "groundStation": "QGroundControl", "geoFence": { "version": 2, "circles": [], "polygons": [] }, "rallyPoints": { "version": 2, "points": [] }, "mission": { "firmwareType": 12, "vehicleType": 2, "plannedHomePosition": [0, 0, 0], "items": items } }).to_string()
        };
        let doc = load(
            &plan(json!([
                item(203, 40, 0.0),
                item(203, 50, 0.0),
                item(177, 60, 50.0)
            ])),
            2,
        )
        .unwrap();
        let Item::Simple(jump) = &doc.items[2] else {
            panic!()
        };
        assert_eq!(
            jump.params[0],
            Some(2.0),
            "doJumpId 50 is the second item, sequence 2 after home"
        );
        assert_eq!(
            load(&plan(json!([item(177, 1, 9.0)])), 2).unwrap_err(),
            "Mission: Could not find doJumpId: 9"
        );
    }

    fn jumping() -> Document {
        let item = |command: i64, id: i64, p1: f64, latitude: f64| json!({ "type": "SimpleItem", "command": command, "frame": 3, "doJumpId": id, "autoContinue": true, "params": [p1, 0, 0, 0, latitude, 8.0, 50] });
        let loaded = load(
            &json!({ "fileType": "Plan", "version": 1, "groundStation": "QGroundControl", "geoFence": { "version": 2, "circles": [], "polygons": [] }, "rallyPoints": { "version": 2, "points": [] }, "mission": { "firmwareType": 12, "vehicleType": 2, "plannedHomePosition": [47.0, 8.0, 0], "items": [
                item(22, 1, 0.0, 47.0),
                item(16, 2, 0.0, 47.001),
                item(16, 3, 0.0, 47.002),
                item(16, 4, 0.0, 47.003),
                item(177, 5, 3.0, 0.0)
            ] } })
            .to_string(),
            2,
        )
        .unwrap();
        let survey = Item::Complex {
            kind: "survey".to_string(),
            json: json!({}),
            item_count: 4,
        };
        jump_to(
            &Document {
                items: [
                    &loaded.items[..2],
                    std::slice::from_ref(&survey),
                    &loaded.items[2..],
                ]
                .concat(),
                ..loaded
            },
            7.0,
        )
    }

    fn jump_to(doc: &Document, seq: f64) -> Document {
        Document {
            items: doc
                .items
                .iter()
                .map(|item| match item {
                    Item::Simple(jump) if jump.command == CMD_DO_JUMP => Item::Simple(Simple {
                        params: std::array::from_fn(|i| match i {
                            0 => Some(seq),
                            _ => jump.params[i],
                        }),
                        ..jump.clone()
                    }),
                    other => other.clone(),
                })
                .collect(),
            ..doc.clone()
        }
    }

    fn jump_lands_on(doc: &Document) -> (Option<f64>, Option<f64>, usize) {
        let spans = item_spans(doc);
        let seq = doc.items.iter().find_map(jump_seq);
        let (at, offset) = jump_target(&spans, seq).unwrap();
        let latitude = match &doc.items[at] {
            Item::Simple(simple) => simple.params[4],
            Item::Complex { .. } => None,
        };
        (seq, latitude, offset)
    }

    #[test]
    fn a_jump_keeps_landing_on_the_same_item_when_items_move_around_it() {
        let doc = jumping();
        assert_eq!(
            jump_lands_on(&doc),
            (Some(7.0), Some(47.002), 0),
            "takeoff 1, waypoint 2, the survey's four 3 to 6, then the target at 7"
        );
        assert_eq!(
            jump_lands_on(&move_item(&doc, 3, false).unwrap().0),
            (Some(7.0), Some(47.002), 0),
            "swapping two items before the target leaves its sequence alone"
        );
        let target_up = move_item(&doc, 4, false).unwrap().0;
        assert_eq!(
            jump_lands_on(&target_up),
            (Some(3.0), Some(47.002), 0),
            "the target moved above the survey, so it is four sequence numbers earlier"
        );
        assert_eq!(
            jump_lands_on(&move_item(&target_up, 3, true).unwrap().0),
            (Some(7.0), Some(47.002), 0)
        );
        assert_eq!(
            jump_lands_on(&move_item(&doc, 4, true).unwrap().0),
            (Some(8.0), Some(47.002), 0),
            "the target moved one place down"
        );
        assert_eq!(
            jump_lands_on(&move_item(&doc, 6, false).unwrap().0),
            (Some(7.0), Some(47.002), 0),
            "the jump itself moved, its target did not"
        );
        assert_eq!(
            jump_lands_on(&move_item(&jump_to(&doc, 5.0), 3, false).unwrap().0),
            (Some(4.0), None, 2),
            "a jump into the survey's third item follows the survey"
        );
    }

    #[test]
    fn a_jump_keeps_its_target_when_items_are_inserted_or_removed_before_it() {
        let doc = jumping();
        assert_eq!(
            jump_lands_on(&insert_waypoint(&doc, 47.0005, 8.0, 2, &QT_DEFAULTS)),
            (Some(8.0), Some(47.002), 0)
        );
        assert_eq!(
            jump_lands_on(&insert_complex(
                &doc,
                "survey",
                json!({ "TransectStyleComplexItem": { "Items": [{}, {}] } }),
                (47.0, 8.0),
                2
            )),
            (Some(9.0), Some(47.002), 0),
            "a survey of two items pushes the target two places on"
        );
        assert_eq!(
            jump_lands_on(&remove(&doc, 3).unwrap()),
            (Some(3.0), Some(47.002), 0),
            "removing the survey before the target takes its four sequence numbers away"
        );
        assert_eq!(
            remove(&doc, 4),
            Err("The jump at item 9 goes to item 7. Change or remove that jump first.".to_string()),
            "the target of a jump is not removed from under it"
        );
        assert_eq!(
            remove(&doc, 6).map(|d| d.items.len()),
            Ok(5),
            "the jump itself can go"
        );
    }

    #[test]
    fn a_plan_is_held_to_the_keys_and_versions_plan_master_controller_requires() {
        let valid = || json!({ "fileType": "Plan", "version": 1, "groundStation": "QGroundControl", "geoFence": { "version": 2, "circles": [], "polygons": [] }, "rallyPoints": { "version": 2, "points": [] }, "mission": { "firmwareType": 12, "plannedHomePosition": [0, 0, 0], "items": [] } });
        let without = |path: &[&str]| {
            let mut plan = valid();
            let (last, parents) = path.split_last().unwrap();
            parents
                .iter()
                .fold(&mut plan, |at, key| &mut at[*key])
                .as_object_mut()
                .unwrap()
                .remove(*last);
            load(&plan.to_string(), 2).unwrap_err()
        };
        assert_eq!(
            without(&["groundStation"]),
            "The following required keys are missing: groundStation"
        );
        assert_eq!(
            without(&["rallyPoints"]),
            "The following required keys are missing: rallyPoints"
        );
        assert_eq!(
            without(&["mission", "firmwareType"]),
            "Mission: The following required keys are missing: firmwareType"
        );
        assert_eq!(
            without(&["geoFence", "polygons"]),
            "The following required keys are missing: polygons"
        );
        let with = |path: &[&str], value: Value| {
            let mut plan = valid();
            *path.iter().fold(&mut plan, |at, key| &mut at[*key]) = value;
            load(&plan.to_string(), 2)
        };
        assert_eq!(
            with(&["version"], json!(2)).unwrap_err(),
            "File version 2 is newer than current supported version 1"
        );
        assert_eq!(
            with(&["fileType"], json!("Mission")).unwrap_err(),
            "Incorrect file type key expected:Plan actual:Mission"
        );
        assert_eq!(
            with(&["mission", "items"], json!({})).unwrap_err(),
            "Mission: Incorrect value type - key:type:expected items:Object:Array"
        );
        assert_eq!(
            with(&["rallyPoints", "version"], json!(3)).unwrap_err(),
            "Rally Points supports version 2"
        );
        assert!(
            with(&["geoFence"], json!({})).is_ok(),
            "an unversioned fence is old data QGC ignores"
        );
        assert_eq!(
            with(&["rallyPoints"], json!({})).unwrap_err(),
            "The following required keys are missing: version, points",
            "while an unversioned rally section is not"
        );
        assert_eq!(
            with(&["mission", "firmwareType"], json!(3))
                .unwrap()
                .vehicle_type,
            2,
            "no vehicleType takes the offline vehicle class"
        );
    }

    #[test]
    fn fence_shapes_and_rally_points_are_checked_like_geofence_and_rally_controllers() {
        let plan = |fence: Value, rally: Value| {
            json!({"fileType": "Plan", "version": 1, "groundStation": "QGroundControl", "mission": {"firmwareType": 12, "plannedHomePosition": [47.0, 8.0, 500], "items": []}, "geoFence": fence, "rallyPoints": rally}).to_string()
        };
        let fence = |polygons: Value, circles: Value| {
            plan(
                json!({"version": 2, "polygons": polygons, "circles": circles}),
                json!({"version": 2, "points": []}),
            )
        };
        let error = |text: String| load(&text, 2).unwrap_err();
        let polygon = json!({"version": 1, "inclusion": true, "polygon": [[47.0, 8.0], [47.1, 8.0], [47.1, 8.1]]});
        let circle = json!({"version": 1, "inclusion": false, "circle": {"center": [47.0, 8.0], "radius": 30}});
        assert!(load(&fence(json!([polygon]), json!([circle])), 2).is_ok());
        assert_eq!(
            error(fence(json!([1]), json!([]))),
            "GeoFence polygon not stored as object"
        );
        assert_eq!(
            error(fence(json!([]), json!(["x"]))),
            "GeoFence circle not stored as object"
        );
        assert_eq!(
            error(fence(json!([{"version": 1}]), json!([]))),
            "The following required keys are missing: inclusion"
        );
        assert_eq!(
            error(fence(
                json!([{"version": 2, "inclusion": true, "polygon": []}]),
                json!([])
            )),
            "GeoFence Polygon only supports version 1"
        );
        assert_eq!(
            error(fence(json!([{"version": 1, "inclusion": true}]), json!([]))),
            "The following required keys are missing: polygon"
        );
        assert_eq!(
            error(fence(
                json!([{"version": 1, "inclusion": true, "polygon": [[47.0, 8.0, 5.0]]}]),
                json!([])
            )),
            "Coordinate array must contain 2 values"
        );
        assert_eq!(
            error(fence(
                json!([{"version": 1, "inclusion": true, "polygon": {}}]),
                json!([])
            )),
            "value for coordinate array is not array"
        );
        assert_eq!(
            error(fence(json!([]), json!([{"version": 3, "inclusion": true}]))),
            "GeoFence Circle only supports version 1"
        );
        assert_eq!(
            error(fence(
                json!([]),
                json!([{"version": 1, "inclusion": true, "circle": {"center": [47.0, 8.0]}}])
            )),
            "The following required keys are missing: radius"
        );
        assert_eq!(
            error(fence(
                json!([]),
                json!([{"version": 1, "inclusion": true, "circle": {"center": ["47", 8.0], "radius": 1}}])
            )),
            "Coordinate array may only contain double values, found: 3"
        );
        assert_eq!(
            error(plan(
                json!({"version": 2, "polygons": [], "circles": [], "breachReturn": [47.0, 8.0]}),
                json!({"version": 2, "points": []})
            )),
            "Coordinate array must contain 3 values"
        );
        assert_eq!(
            error(plan(
                json!({"version": 2, "polygons": [], "circles": []}),
                json!({"version": 2, "points": [[47.0, 8.0]]})
            )),
            "Rally: Coordinate array must contain 3 values"
        );
        assert!(
            load(
                &plan(
                    json!({"version": 1, "polygons": [1]}),
                    json!({"version": 1, "points": [1]})
                ),
                2
            )
            .is_ok(),
            "old version 1 sections are ignored, not checked"
        );
        assert!(load(&plan(json!({"version": 2, "polygons": [{"version": 1, "inclusion": true, "polygon": [[null, 8.0]]}], "circles": []}), json!({"version": 2, "points": [[47.0, 8.0, null]]})), 2).is_ok(), "null is a NaN coordinate");
    }

    #[test]
    fn mission_items_load_like_mission_controller_load_json_mission_file_v2() {
        let plan = |home: Value, items: Value| {
            json!({"fileType": "Plan", "version": 1, "groundStation": "QGroundControl", "geoFence": {"version": 2, "polygons": [], "circles": []}, "rallyPoints": {"version": 2, "points": []}, "mission": {"firmwareType": 12, "vehicleType": 2, "plannedHomePosition": home, "items": items}}).to_string()
        };
        let error = |home: Value, items: Value| load(&plan(home, items), 2).unwrap_err();
        assert_eq!(
            error(json!([47.0, 8.0]), json!([])),
            "Mission: Coordinate array must contain 3 values",
            "the planned home is read with GeoJsonHelper::loadGeoCoordinate"
        );
        assert_eq!(
            error(json!([47.0, 8.0, 0]), json!([7])),
            "Mission: Mission item 0 is not an object"
        );
        assert_eq!(
            error(json!([47.0, 8.0, 0]), json!([{"command": 16}])),
            "Mission: The following required keys are missing: type"
        );
        assert_eq!(
            error(json!([47.0, 8.0, 0]), json!([{"type": "ComplexItem"}])),
            "Mission: The following required keys are missing: complexItemType"
        );
        assert_eq!(
            error(
                json!([47.0, 8.0, 0]),
                json!([{"type": "ComplexItem", "complexItemType": "Orbit"}])
            ),
            "Mission: Unsupported complex item type: Orbit"
        );
        let waypoint = json!({"type": "SimpleItem", "autoContinue": true, "command": 16, "frame": 3, "params": [0, 0, 0, 0, 47.0, 8.0, 50], "AltitudeMode": 1, "Altitude": 50});
        assert_eq!(
            error(json!([47.0, 8.0, 0]), json!([waypoint])),
            "Mission: The following required keys are missing: AMSLAltAboveTerrain",
            "SimpleMissionItem::load wants all three altitude keys once one is saved"
        );
        let delay = json!({"type": "SimpleItem", "autoContinue": true, "command": 93, "frame": 2, "params": [5, 0, 0, 0, 0, 0, 0], "AltitudeMode": 1});
        assert!(
            load(&plan(json!([47.0, 8.0, 0]), json!([delay])), 2).is_ok(),
            "a command without altitude never reads the altitude keys"
        );
        let nan = json!({"type": "SimpleItem", "autoContinue": true, "command": 16, "frame": 3, "params": [0, 0, 0, 0, 47.0, 8.0, 50], "AltitudeMode": 1, "Altitude": 50, "AMSLAltAboveTerrain": null});
        assert!(
            load(&plan(json!([47.0, 8.0, 0]), json!([nan])), 2).is_ok(),
            "a null AMSL altitude is a NaN double"
        );
    }

    #[test]
    fn mission_items_are_checked_before_fence_and_rally_like_plan_master_controller() {
        let plan = json!({"fileType": "Plan", "version": 1, "groundStation": "QGroundControl",
            "mission": {"firmwareType": 12, "plannedHomePosition": [47.0, 8.0, 500], "items": [{"type": "Bogus"}]},
            "geoFence": {"version": 2, "polygons": [1], "circles": []}, "rallyPoints": {"version": 3, "points": []}});
        assert_eq!(
            load(&plan.to_string(), 2).unwrap_err(),
            "Mission: Unknown item type: Bogus",
            "MissionController::load prefixes every item error with Mission:"
        );
        let fixed = json!({"fileType": "Plan", "version": 1, "groundStation": "QGroundControl",
            "mission": {"firmwareType": 12, "plannedHomePosition": [47.0, 8.0, 500], "items": []},
            "geoFence": {"version": 2, "polygons": [1], "circles": []}, "rallyPoints": {"version": 3, "points": []}});
        assert_eq!(
            load(&fixed.to_string(), 2).unwrap_err(),
            "GeoFence polygon not stored as object",
            "fence before rally"
        );
    }
}
