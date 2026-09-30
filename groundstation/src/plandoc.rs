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
    pub sections: Vec<Simple>,
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
    let items = crate::landingpattern::fold(items, firmware(firmware_type) == Firmware::ArduPilot);
    let (settings_sections, items) = fold(items, vehicle_class(vehicle_type));
    Ok(Document {
        firmware_type,
        vehicle_type,
        cruise_speed: number("cruiseSpeed", 0.0),
        hover_speed: number("hoverSpeed", 0.0),
        global_altitude_mode: integer("globalPlanAltitudeMode"),
        home: Some(home),
        settings_sections,
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
            let item_count = complex_count(&kind, item)?;
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
        sections: Vec::new(),
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
        let info = commands.get(&item.command);
        let specifies_altitude = info.is_some_and(|c| c.specifies_coordinate || c.specifies_altitude_only);
        let launched_here = info.is_some_and(|c| c.is_takeoff && !c.specifies_coordinate);
        let params = match (launched_here, home) {
            (true, Some(h)) => [item.params[0], item.params[1], item.params[2], item.params[3], h[0], h[1], item.params[6]],
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
    let landings = crate::landingpattern::fold(listed.iter().map(simple).collect(), firmware(template.firmware_type) == Firmware::ArduPilot);
    let (settings_sections, items) = fold(landings, vehicle_class(template.vehicle_type));
    Document {
        home,
        settings_sections,
        items,
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

pub fn previous_altitude_mode(doc: &Document, visual_index: i64) -> Option<i64> {
    let commands = cmdinfo::tree(firmware(doc.firmware_type), vehicle_class(doc.vehicle_type));
    previous_altitude(doc, &commands, visual_index).map(|(_, mode)| mode)
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

const CMD_NAV_RETURN_TO_LAUNCH: i64 = 20;
const CMD_DO_SET_ROI_LOCATION: i64 = 195;
const FRAME_MISSION: i64 = 2;

fn with_command_defaults(doc: &Document, commands: &std::collections::BTreeMap<i64, cmdinfo::Command>, command: i64, kept: [Option<f64>; 7], defaults: &EditDefaults) -> Simple {
    let info = commands.get(&command);
    let coordinate = info.is_some_and(|c| c.specifies_coordinate || c.standalone_coordinate);
    let specifies_altitude = info.is_some_and(|c| c.specifies_coordinate || c.specifies_altitude_only);
    let listed = param_defaults(info);
    let (latitude, longitude) = match coordinate {
        true => (kept[4], kept[5]),
        false => (Some(0.0), Some(0.0)),
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
    let param = |i: usize| info.and_then(|c| c.params.get(&(i as u8 + 1))).map_or(Some(0.0), |_| listed[i]);
    let seventh = match altitude {
        Some(alt) => Some(alt),
        None => info.and_then(|c| c.params.get(&7)).map_or(Some(0.0), |_| listed[6]),
    };
    let fifth = info.and_then(|c| c.params.get(&5)).map_or(latitude, |_| listed[4]);
    let sixth = info.and_then(|c| c.params.get(&6)).map_or(longitude, |_| listed[5]);
    Simple {
        command,
        frame: match altitude {
            Some(_) => frame_for(mode),
            None => FRAME_MISSION,
        },
        params: [param(0), if command == CMD_NAV_WAYPOINT { Some(0.0) } else { param(1) }, param(2), param(3), fifth, sixth, seventh],
        auto_continue: true,
        altitude: altitude.map(|altitude| Altitude { mode, altitude, amsl_above_terrain: None }),
        sections: Vec::new(),
    }
}

pub fn insert_simple(doc: &Document, command: i64, latitude: f64, longitude: f64, visual_index: i64, defaults: &EditDefaults) -> Document {
    let commands = cmdinfo::tree(firmware(doc.firmware_type), vehicle_class(doc.vehicle_type));
    let fresh = with_command_defaults(doc, &commands, command, [None, None, None, None, Some(latitude), Some(longitude), None], defaults);
    let land = commands.get(&command).is_some_and(|c| c.is_land);
    let inherited = previous_altitude(doc, &commands, visual_index).filter(|_| fresh.altitude.is_some() && !land);
    let placed = match inherited {
        Some((altitude, previous_mode)) => {
            let mode = match doc.global_altitude_mode {
                crate::altitudemodes::MIXED => previous_mode,
                _ => fresh.altitude.as_ref().map_or(previous_mode, |a| a.mode),
            };
            Simple {
                frame: frame_for(mode),
                params: [fresh.params[0], fresh.params[1], fresh.params[2], fresh.params[3], fresh.params[4], fresh.params[5], Some(altitude)],
                altitude: Some(Altitude { mode, altitude, amsl_above_terrain: None }),
                ..fresh
            }
        }
        None => fresh,
    };
    let at = usize::try_from(visual_index - 1).ok().filter(|i| *i <= doc.items.len()).unwrap_or(doc.items.len());
    let items: Vec<Item> = doc.items[..at].iter().cloned().chain(std::iter::once(Item::Simple(placed))).chain(doc.items[at..].iter().cloned()).collect();
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

pub fn complex_count(kind: &str, item: &Value) -> Result<usize, String> {
    match TRANSECT_STYLE.contains(&kind) {
        true => item.get("TransectStyleComplexItem").and_then(|t| t.get("Items")).and_then(Value::as_array).map(Vec::len).ok_or_else(|| format!("The {kind} item has no saved mission items.")),
        false if kind == "StructureScan" => crate::structurescan::saved_items(item).map(|items| items.len()),
        false if crate::landingpattern::is_landing(kind) => crate::landingpattern::items(item, false).map(|items| items.len()),
        false => Err(format!("The core cannot hold a {kind} item yet.")),
    }
}

pub fn insert_complex(doc: &Document, kind: &str, json: Value, center: (f64, f64), visual_index: i64) -> Document {
    let item_count = complex_count(kind, &json).unwrap_or(0);
    let at = usize::try_from(visual_index - 1).ok().filter(|i| *i <= doc.items.len()).unwrap_or(doc.items.len());
    let item = Item::Complex { kind: kind.to_string(), json, item_count };
    let items: Vec<Item> = doc.items[..at].iter().cloned().chain(std::iter::once(item)).chain(doc.items[at..].iter().cloned()).collect();
    let home = doc.home.or_else(|| {
        let (lat, lon) = crate::surveygrid::at_distance_and_azimuth(center, PLANNED_HOME_OFFSET_M, 0.0);
        Some([lat, lon, 0.0])
    });
    Document { items, home, ..doc.clone() }
}

const CMD_DO_SET_ROI: i64 = 201;
const MAV_ROI_LOCATION: f64 = 3.0;

pub fn insert_roi(doc: &Document, latitude: f64, longitude: f64, visual_index: i64, defaults: &EditDefaults) -> Document {
    let placed = insert_simple(doc, CMD_DO_SET_ROI_LOCATION, latitude, longitude, visual_index, defaults);
    if firmware(doc.firmware_type) == Firmware::Px4 {
        return placed;
    }
    let at = usize::try_from(visual_index - 1).ok().filter(|i| *i <= doc.items.len()).unwrap_or(doc.items.len()) + 1;
    set_command(&placed, at, CMD_DO_SET_ROI, defaults).and_then(|changed| set_param(&changed, at, 1, MAV_ROI_LOCATION)).unwrap_or(placed)
}

pub fn insert_waypoint(doc: &Document, latitude: f64, longitude: f64, visual_index: i64, defaults: &EditDefaults) -> Document {
    insert_simple(doc, CMD_NAV_WAYPOINT, latitude, longitude, visual_index, defaults)
}

pub fn insert_land(doc: &Document, latitude: f64, longitude: f64, visual_index: i64, defaults: &EditDefaults) -> Result<Document, String> {
    match vehicle_class(doc.vehicle_type) {
        VehicleClass::FixedWing | VehicleClass::Vtol => Err("The core cannot build a landing pattern yet.".to_string()),
        _ => Ok(insert_simple(doc, CMD_NAV_RETURN_TO_LAUNCH, latitude, longitude, visual_index, defaults)),
    }
}

const CMD_NAV_TAKEOFF: i64 = 22;

pub fn insert_takeoff(doc: &Document, visual_index: i64, defaults: &EditDefaults) -> Result<Document, String> {
    let home = doc.home.ok_or("A takeoff is placed at the launch position, and this plan has none yet.")?;
    match vehicle_class(doc.vehicle_type) {
        VehicleClass::FixedWing => Err("A fixed-wing takeoff needs its climb-out placed on the map.".to_string()),
        _ => {
            let inserted = insert_simple(doc, CMD_NAV_TAKEOFF, home[0], home[1], visual_index, defaults);
            let at = usize::try_from(visual_index - 1).ok().filter(|i| *i <= doc.items.len()).unwrap_or(doc.items.len());
            let launched = |item: &Item| match item {
                Item::Simple(s) => Item::Simple(Simple { params: [s.params[0], s.params[1], s.params[2], s.params[3], Some(home[0]), Some(home[1]), s.params[6]], ..s.clone() }),
                other => other.clone(),
            };
            Ok(Document { items: inserted.items.iter().enumerate().map(|(i, item)| if i == at { launched(item) } else { item.clone() }).collect(), ..inserted })
        }
    }
}

pub fn set_global_altitude_mode(doc: &Document, mode: i64) -> Document {
    Document { global_altitude_mode: mode, ..doc.clone() }
}

fn simple_at(doc: &Document, visual_index: usize) -> Option<(usize, &Simple)> {
    let at = visual_index.checked_sub(1)?;
    match doc.items.get(at)? {
        Item::Simple(simple) => Some((at, simple)),
        Item::Complex { .. } => None,
    }
}

fn replaced(doc: &Document, at: usize, item: Simple) -> Document {
    Document { items: doc.items.iter().enumerate().map(|(i, existing)| if i == at { Item::Simple(item.clone()) } else { existing.clone() }).collect(), ..doc.clone() }
}

pub fn set_command(doc: &Document, visual_index: usize, command: i64, defaults: &EditDefaults) -> Option<Document> {
    let (at, current) = simple_at(doc, visual_index)?;
    let commands = cmdinfo::tree(firmware(doc.firmware_type), vehicle_class(doc.vehicle_type));
    match current.command == command {
        true => Some(doc.clone()),
        false => Some(replaced(doc, at, with_command_defaults(doc, &commands, command, current.params, defaults))),
    }
}

pub fn set_param(doc: &Document, visual_index: usize, param: usize, value: f64) -> Option<Document> {
    let (at, current) = simple_at(doc, visual_index)?;
    let slot = param.checked_sub(1).filter(|p| *p < 7)?;
    let params: [Option<f64>; 7] = std::array::from_fn(|i| if i == slot { Some(value) } else { current.params[i] });
    let altitude = match slot {
        6 => current.altitude.as_ref().map(|a| Altitude { altitude: value, ..a.clone() }),
        _ => current.altitude.clone(),
    };
    Some(replaced(doc, at, Simple { params, altitude, ..current.clone() }))
}

fn speed_change(class: VehicleClass, speed: f64) -> Simple {
    let ground = match class {
        VehicleClass::MultiRotor => 1.0,
        _ => 0.0,
    };
    Simple {
        command: CMD_DO_CHANGE_SPEED,
        frame: FRAME_MISSION,
        params: [Some(ground), Some(speed), Some(-1.0), Some(0.0), Some(0.0), Some(0.0), Some(0.0)],
        auto_continue: true,
        altitude: None,
        sections: Vec::new(),
    }
}

fn with_speed(sections: &[Simple], class: VehicleClass, speed: Option<f64>) -> Vec<Simple> {
    let others = sections.iter().filter(|s| s.command != CMD_DO_CHANGE_SPEED).cloned();
    others.chain(speed.map(|v| speed_change(class, v))).collect()
}

pub fn set_speed(doc: &Document, visual_index: usize, speed: Option<f64>) -> Option<Document> {
    let class = vehicle_class(doc.vehicle_type);
    match visual_index {
        0 => Some(Document { settings_sections: with_speed(&doc.settings_sections, class, speed), ..doc.clone() }),
        _ => {
            let (at, current) = simple_at(doc, visual_index)?;
            (current.command == CMD_NAV_WAYPOINT).then(|| replaced(doc, at, Simple { sections: with_speed(&current.sections, class, speed), ..current.clone() }))
        }
    }
}

pub fn specified_speed(doc: &Document, visual_index: usize) -> Option<f64> {
    let sections = match visual_index {
        0 => &doc.settings_sections,
        _ => &simple_at(doc, visual_index)?.1.sections,
    };
    sections.iter().find(|s| s.command == CMD_DO_CHANGE_SPEED).and_then(|s| s.params[1])
}

pub fn set_altitude(doc: &Document, visual_index: usize, altitude: f64) -> Option<Document> {
    let (at, current) = simple_at(doc, visual_index)?;
    let held = current.altitude.as_ref()?;
    let params = [current.params[0], current.params[1], current.params[2], current.params[3], current.params[4], current.params[5], Some(altitude)];
    Some(replaced(doc, at, Simple { params, altitude: Some(Altitude { altitude, ..held.clone() }), ..current.clone() }))
}

pub fn remove(doc: &Document, visual_index: usize) -> Option<Document> {
    let at = visual_index.checked_sub(1).filter(|i| *i < doc.items.len())?;
    Some(Document { items: doc.items.iter().enumerate().filter(|(i, _)| *i != at).map(|(_, item)| item.clone()).collect(), ..doc.clone() })
}

pub fn save(doc: &Document) -> Value {
    let section = |s: &Simple| Item::Simple(Simple { frame: FRAME_MISSION, ..s.clone() });
    let settings = doc.settings_sections.iter().map(section);
    let spans: Vec<(Item, usize)> = settings
        .chain(doc.items.iter().flat_map(|item| match item {
            Item::Simple(simple) => std::iter::once(Item::Simple(Simple { sections: Vec::new(), ..simple.clone() })).chain(simple.sections.iter().map(section)).collect::<Vec<_>>(),
            complex => vec![complex.clone()],
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
    let items: Vec<Value> = spans.iter().zip(starts).map(|((item, _), seq)| save_item(item, seq)).collect();
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
    let gimbal = item.command == CMD_DO_MOUNT_CONTROL && p(item, 1) == 0.0 && p(item, 3) == 0.0 && p(item, 4) == 0.0 && p(item, 5) == 0.0 && p(item, 6) == MOUNT_MODE_MAVLINK_TARGETING;
    let photo = item.command == CMD_IMAGE_START_CAPTURE && p(item, 0) == 0.0 && p(item, 1) == 0.0 && p(item, 2) == 1.0;
    let interval = item.command == CMD_IMAGE_START_CAPTURE && p(item, 0) == 0.0 && p(item, 1) >= 1.0 && p(item, 2) == 0.0;
    let trigger_zero = item.command == CMD_DO_SET_CAM_TRIGG_DIST && zero(item, 0);
    let stop_photos = trigger_zero && next.is_some_and(|n| n.command == CMD_IMAGE_STOP_CAPTURE && p(n, 0) == 0.0);
    let trigger_start = item.command == CMD_DO_SET_CAM_TRIGG_DIST && p(item, 0) > 0.0 && p(item, 1) == 0.0 && p(item, 2) == 1.0 && zero(item, 3);
    let video = item.command == CMD_VIDEO_START_CAPTURE && p(item, 0) == 0.0 && p(item, 1) == VIDEO_CAPTURE_STATUS_INTERVAL;
    let stop_video = item.command == CMD_VIDEO_STOP_CAPTURE && p(item, 0) == 0.0;
    let mode = item.command == CMD_SET_CAMERA_MODE && p(item, 0) == 0.0 && [0.0, 1.0, 2.0].contains(&p(item, 1)) && p(item, 2).is_nan();
    match () {
        _ if not(Found::Gimbal) && gimbal => Some((Found::Gimbal, 1)),
        _ if not(Found::Action) && (photo || interval) => Some((Found::Action, 1)),
        _ if not(Found::Action) && stop_photos => Some((Found::Action, 2)),
        _ if not(Found::Action) && (trigger_start || trigger_zero || video || stop_video) => Some((Found::Action, 1)),
        _ if not(Found::Mode) && mode => Some((Found::Mode, 1)),
        _ => None,
    }
}

const CAMERA_ACTIONS: [(&str, i64); 7] = [("No change", 0), ("Take photo", 6), ("Take photos (time)", 1), ("Take photos (distance)", 2), ("Stop taking photos", 3), ("Start recording video", 4), ("Stop recording video", 5)];

pub fn camera_section(sections: &[Simple]) -> Value {
    let gimbal = sections.iter().find(|s| s.command == CMD_DO_MOUNT_CONTROL);
    let action = sections.iter().enumerate().find_map(|(i, item)| {
        let next = sections.get(i + 1);
        match item.command {
            CMD_IMAGE_START_CAPTURE if p(item, 2) == 1.0 => Some(6),
            CMD_IMAGE_START_CAPTURE => Some(1),
            CMD_DO_SET_CAM_TRIGG_DIST if zero(item, 0) && next.is_some_and(|n| n.command == CMD_IMAGE_STOP_CAPTURE) => Some(3),
            CMD_DO_SET_CAM_TRIGG_DIST => Some(2),
            CMD_VIDEO_START_CAPTURE => Some(4),
            CMD_VIDEO_STOP_CAPTURE => Some(5),
            _ => None,
        }
    });
    let action = action.unwrap_or(0);
    let degrees = |property: &str, value: f64| json!({ "property": property, "value": value, "valueString": format!("{value:.0}"), "enumOrValueString": format!("{value:.0}"), "units": "deg" });
    let chosen = CAMERA_ACTIONS.iter().position(|(_, v)| *v == action).unwrap_or(0);
    json!({
        "kind": "object",
        "class": "CameraSection",
        "specifyGimbal": gimbal.is_some(),
        "facts": [
            degrees("gimbalPitch", -gimbal.map_or(0.0, |g| p(g, 0))),
            degrees("gimbalYaw", gimbal.map_or(0.0, |g| p(g, 2))),
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

fn camera_span(rest: &[Simple], found: Vec<Found>) -> usize {
    match camera_match(rest, &found) {
        Some((kind, taken)) => taken + camera_span(&rest[taken..], found.into_iter().chain(std::iter::once(kind)).collect()),
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
            let folded = Simple { sections: simples(&rest[..span]), ..owner.clone() };
            std::iter::once(Item::Simple(folded)).chain(fold_rest(&rest[span..], class)).collect()
        }
        Some((first, rest)) => std::iter::once(first.clone()).chain(fold_rest(rest, class)).collect(),
    }
}

fn fold(items: Vec<Item>, class: VehicleClass) -> (Vec<Simple>, Vec<Item>) {
    let settings = section_span(&items, class);
    (simples(&items[..settings]), fold_rest(&items[settings..], class))
}

fn save_item(item: &Item, seq: usize) -> Value {
    match item {
        Item::Complex { json, .. } => {
            let mut numbered = json.clone();
            if let Some(fields) = numbered.as_object_mut() {
                fields.remove(crate::landingpattern::WIZARD);
            }
            if let Some(items) = numbered.pointer_mut("/TransectStyleComplexItem/Items").and_then(Value::as_array_mut) {
                items.iter_mut().enumerate().for_each(|(i, item)| item["doJumpId"] = json!(seq + i));
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
    fn landing_a_multirotor_appends_a_return_to_launch_as_qt_does() {
        matches_qt(&insert_land(&section(), 47.6325, -122.0870, -1, &QT_DEFAULTS).unwrap(), include_str!("../tests/fixtures/edit-F-by-qt.plan"));
        let plane = Document { vehicle_type: 1, ..section() };
        assert!(insert_land(&plane, 47.6325, -122.0870, -1, &QT_DEFAULTS).is_err(), "a fixed-wing landing is a pattern, not a simple item");
    }

    #[test]
    fn an_altitude_edit_moves_param_seven_with_it() {
        matches_qt(&set_altitude(&section(), 2, 33.0).unwrap(), include_str!("../tests/fixtures/edit-G-by-qt.plan"));
        assert!(set_altitude(&section(), 9, 33.0).is_none(), "there is no ninth item");
    }

    #[test]
    fn changing_a_command_resets_its_parameters_and_altitude_but_keeps_the_position() {
        matches_qt(&set_command(&section(), 2, 19, &QT_DEFAULTS).unwrap(), include_str!("../tests/fixtures/edit-H-by-qt.plan"));
    }

    #[test]
    fn a_takeoff_sits_on_the_launch_position_at_the_default_altitude() {
        let without_takeoff = remove(&section(), 1).unwrap();
        matches_qt(&insert_takeoff(&without_takeoff, 1, &QT_DEFAULTS).unwrap(), include_str!("../tests/fixtures/edit-takeoff-by-qt.plan"));
        assert!(insert_takeoff(&Document { home: None, ..without_takeoff.clone() }, 1, &QT_DEFAULTS).is_err());
        assert!(insert_takeoff(&Document { vehicle_type: 1, ..without_takeoff }, 1, &QT_DEFAULTS).is_err(), "Qt opens a plane's takeoff in the wizard for its climb-out");
    }

    #[test]
    fn a_region_of_interest_is_the_location_command_only_where_the_firmware_takes_it() {
        let px4 = Document { firmware_type: 12, ..section() };
        let placed = insert_roi(&px4, 47.63, -122.09, -1, &QT_DEFAULTS);
        assert!(matches!(placed.items.last(), Some(Item::Simple(s)) if s.command == CMD_DO_SET_ROI_LOCATION));
        let apm = Document { firmware_type: 3, ..section() };
        let placed = insert_roi(&apm, 47.63, -122.09, 2, &QT_DEFAULTS);
        let Some(Item::Simple(roi)) = placed.items.get(1) else { panic!("the region of interest goes where it was asked") };
        assert_eq!((roi.command, roi.params[0], roi.params[4], roi.params[5]), (CMD_DO_SET_ROI, Some(MAV_ROI_LOCATION), Some(47.63), Some(-122.09)));
    }

    #[test]
    fn a_new_item_takes_the_altitude_mode_of_the_positioned_item_before_it() {
        let absolute = set_global_altitude_mode(&section(), crate::altitudemodes::ABSOLUTE);
        let with_absolute = insert_waypoint(&absolute, 47.634, -122.089, -1, &QT_DEFAULTS);
        let mixed = set_global_altitude_mode(&with_absolute, crate::altitudemodes::MIXED);
        let last = mixed.items.len() as i64;
        assert_eq!(previous_altitude_mode(&mixed, last + 1), Some(crate::altitudemodes::ABSOLUTE));
        assert_eq!(previous_altitude_mode(&mixed, -1), None, "appending finds nothing, as _findPreviousAltitude walks down from index -2");
    }

    #[test]
    fn a_survey_numbers_its_items_from_the_sequence_it_lands_on() {
        let fixture: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-inserted-by-qt.json")).unwrap();
        let placed = insert_complex(&section(), "survey", fixture["survey"].clone(), (47.63, -122.09), 2);
        let saved = save(&placed);
        let survey = &saved["mission"]["items"][1]["TransectStyleComplexItem"]["Items"];
        let before = &saved["mission"]["items"][0]["doJumpId"];
        assert_eq!(survey[0]["doJumpId"].as_i64(), before.as_i64().map(|n| n + 1));
        let after = saved["mission"]["items"][2]["doJumpId"].as_i64().unwrap();
        assert_eq!(after, survey[0]["doJumpId"].as_i64().unwrap() + survey.as_array().unwrap().len() as i64);
    }

    #[test]
    fn the_plan_altitude_mode_changes_new_items_and_leaves_existing_ones() {
        let absolute = set_global_altitude_mode(&section(), crate::altitudemodes::ABSOLUTE);
        matches_qt(&absolute, include_str!("../tests/fixtures/edit-absolute-by-qt.plan"));
        matches_qt(&insert_waypoint(&absolute, 47.634, -122.089, -1, &QT_DEFAULTS), include_str!("../tests/fixtures/edit-absolute-then-waypoint-by-qt.plan"));
    }

    fn qt_view(text: &str) -> Vec<Value> {
        serde_json::from_str::<Value>(text).unwrap()["items"].as_array().unwrap().clone()
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
        let qt = qt_view(include_str!("../tests/fixtures/missionitems-sectiontest-by-qt.json"));
        assert_eq!(folded_counts(&section()), qt.iter().map(|i| i["foldedCommands"].as_i64().unwrap()).collect::<Vec<_>>(), "the mount control after the second waypoint is that waypoint's camera section, so Qt shows five rows where the file holds six items");
        let survey = load(include_str!("../tests/fixtures/survey-upload.plan")).unwrap();
        let qt = qt_view(include_str!("../tests/fixtures/missionitems-survey-by-qt.json"));
        assert_eq!(folded_counts(&survey), qt.iter().map(|i| i["foldedCommands"].as_i64().unwrap()).collect::<Vec<_>>());
    }

    #[test]
    fn a_speed_change_folds_only_when_it_is_the_kind_the_airframe_flies_by() {
        let speed = |ground: f64| json!({ "type": "SimpleItem", "command": 178, "frame": 2, "doJumpId": 3, "params": [ground, 12, -1, 0, 0, 0, 0] });
        let plan = |ground: f64, vehicle: i64| json!({ "fileType": "Plan", "mission": { "firmwareType": 3, "vehicleType": vehicle, "plannedHomePosition": [1, 2, 0], "items": [
            { "type": "SimpleItem", "command": 16, "frame": 3, "doJumpId": 1, "params": [0, 0, 0, 0, 1.0, 2.0, 30] },
            speed(ground),
        ] } }).to_string();
        assert_eq!(load(&plan(1.0, 2)).unwrap().items.len(), 1, "a multirotor flies by ground speed");
        assert_eq!(load(&plan(0.0, 2)).unwrap().items.len(), 2, "an airspeed change on a multirotor stays its own row");
        assert_eq!(load(&plan(0.0, 1)).unwrap().items.len(), 1, "a plane flies by airspeed");
        let resaved = save(&load(&plan(1.0, 2)).unwrap());
        assert_eq!(resaved["mission"]["items"].as_array().unwrap().len(), 2, "a folded section is still written out after its owner");
        assert_eq!(resaved["mission"]["items"][1]["doJumpId"], 2);
    }

    #[test]
    fn a_speed_section_is_the_change_speed_command_shaped_for_the_airframe() {
        let with = set_speed(&section(), 2, Some(8.0)).unwrap();
        assert_eq!(specified_speed(&with, 2), Some(8.0));
        let written = save(&with);
        let items = written["mission"]["items"].as_array().unwrap();
        assert_eq!((items[2]["command"].as_i64(), items[2]["params"][0].as_f64(), items[2]["params"][2].as_f64()), (Some(178), Some(1.0), Some(-1.0)), "a multirotor's speed section is a ground speed with no throttle change, the shape SpeedSection writes");
        assert_eq!(load(&written.to_string()).unwrap().items.len(), section().items.len(), "and it folds back into its waypoint on load");
        assert_eq!(specified_speed(&set_speed(&with, 2, None).unwrap(), 2), None);
        assert!(set_speed(&section(), 1, Some(8.0)).is_none(), "a takeoff has no speed section");
        assert_eq!(specified_speed(&set_speed(&section(), 0, Some(6.0)).unwrap(), 0), Some(6.0), "the settings item carries the plan's opening speed");
    }

    #[test]
    fn a_field_edit_moves_the_param_it_shows() {
        let edited = set_param(&section(), 2, 1, 4.0).unwrap();
        let Item::Simple(s) = &edited.items[1] else { panic!() };
        assert_eq!(s.params[0], Some(4.0));
        assert!(set_param(&section(), 2, 8, 1.0).is_none());
    }

    #[test]
    fn version_one_fences_and_rally_points_are_dropped_as_qt_drops_them() {
        let doc = load(include_str!("../../test/MissionManager/SectionTest.plan")).unwrap();
        assert_eq!(doc.fence, json!({ "circles": [], "polygons": [], "version": 2 }));
        assert_eq!(doc.rally, json!({ "points": [], "version": 2 }));
    }

    #[test]
    fn an_item_the_core_cannot_hold_refuses_the_whole_plan() {
        let plan = json!({ "fileType": "Plan", "mission": { "plannedHomePosition": [0, 0, 0], "items": [{ "type": "ComplexItem", "complexItemType": "FWLandingPattern" }] } });
        assert!(load(&plan.to_string()).unwrap_err().contains("FWLandingPattern"));
    }
}
