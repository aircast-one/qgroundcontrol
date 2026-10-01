use serde_json::{Value, json};

use crate::read::{Unit, flag, format_measure, integer, object, text};
use crate::router::Backend;

pub const DEPS: &[&str] = &[
    "plan.missionController.visualItems.count",
    "plan.missionController.currentPlanViewVIIndex",
    "plan.missionController.containsItems",
    "plan.missionController@visualItemsReset",
    "plan.missionController@newItemsFromVehicle",
    "settings.unitsSettings.horizontalDistanceUnits",
    "settings.unitsSettings.verticalDistanceUnits",
    "settings.unitsSettings.speedUnits",
    "plan.controllerVehicle.rover",
    crate::coreplan::CHANGED,
];

const FIELDS: &str = "lastSequenceNumber,specifiedFlightSpeed,additionalTimeDelay,minAMSLAltitude,maxAMSLAltitude,sequenceNumber,abbreviation,commandName,commandDescription,isCurrentItem,specifiesCoordinate,isStandaloneCoordinate,specifiesAltitudeOnly,isSimpleItem,isTakeoffItem,isLandCommand,isSurveyItem,homePosition,coordinate,amslEntryAlt,altDifference,azimuth,distance,distanceFromStart,readyForSaveState,readyForSaveMessage,dirty,altitude,altitudeFrame,altitudeMode,isIncomplete,exitCoordinate,exitCoordinateSameAsEntry,commandName,command,category,specifiesAltitude,cameraShots,complexDistance,plannedHomePositionAltitude";

const READY_TO_SAVE: i64 = 0;
const AWAITING_TERRAIN: i64 = 1;
const NOT_READY_FOR_SAVE: i64 = 2;
const RETURN_TO_LAUNCH: i64 = 20;

pub fn items_view(backend: &dyn Backend, args: &[String]) -> Value {
    if crate::coreplan::enabled() {
        return crate::coreplan::view(backend);
    }
    let everything = args.iter().any(|arg| arg == "fields");
    let vertical = Unit::vertical(backend);
    let speed = Unit::speed(backend);
    let imperial = crate::missionsummary::imperial(backend);
    let shapes = args.iter().any(|arg| arg == "geometry");
    let count = integer(&object(&backend.get("plan.missionController.visualItems.count")), "value").unwrap_or(0);
    let has_items = flag(&object(&backend.get_fields("plan.missionController", "containsItems")), "containsItems");
    if count <= 0 {
        return json!({ "kind": "object", "class": "MissionItems", "available": false, "linksStartToHome": false, "items": [], "selected": -1, "reason": "This plan has no items yet." });
    }
    let current = integer(&object(&backend.get("plan.missionController.currentPlanViewVIIndex")), "value").unwrap_or(-1);
    let listed = object(&backend.get_fields("plan.missionController.visualItems", FIELDS));
    let items: Vec<Value> = match listed.get("elements").and_then(Value::as_array) {
        Some(elements) => elements.iter().enumerate().map(|(index, element)| item(element, index as i64, &vertical, &speed, imperial)).collect(),
        None => (0..count)
            .map(|index| item(&object(&backend.get_fields(&format!("plan.missionController.visualItems.{index}"), FIELDS)), index, &vertical, &speed, imperial))
            .collect(),
    };
    let items: Vec<Value> = match walked(&items) {
        true => items,
        false => items.into_iter().map(unwalked).collect(),
    };
    let items: Vec<Value> = match shapes {
        false => items,
        true => items
            .into_iter()
            .enumerate()
            .map(|(index, mut listed)| {
                listed["geometry"] = geometry_of(backend, index as i64, listed["kind"].as_str().unwrap_or_default(), &vertical);
                listed
            })
            .collect(),
    };
    let items: Vec<Value> = match everything {
        false => items,
        true => items
            .into_iter()
            .enumerate()
            .map(|(index, mut listed)| {
                listed["fields"] = fields_of(backend, index as i64);
                listed
            })
            .collect(),
    };
    let rover = flag(&object(&backend.get_fields("plan.controllerVehicle", "rover")), "rover");
    json!({
        "kind": "object",
        "class": "MissionItems",
        "available": has_items,
        "linksStartToHome": rover || starts_from_the_ground(&items),
        "editing": editable(backend, current),
        "selected": current,
        "items": items,
        "reason": match has_items {
            true => "",
            false => "This plan has no items yet.",
        },
    })
}

const SETTINGS_NAME: &str = "Initial Camera Settings";

fn abbreviation(command: i64) -> &'static str {
    match command {
        22 => "Takeoff",
        21 => "Land",
        84 => "Transition Direction",
        85 => "VTOL Land",
        201 | 195 => "ROI",
        19 | 18 | 17 | 31 => "Loiter",
        _ => "",
    }
}

struct Leg {
    azimuth: f64,
    distance: f64,
    alt_difference: f64,
    from_start: f64,
}

fn amsl_entry(simple: &crate::plandoc::Simple, home_altitude: f64) -> f64 {
    let seventh = simple.params[6].unwrap_or(f64::NAN);
    let mode = simple.altitude.as_ref().map_or(match simple.frame { 10 => crate::altitudemodes::TERRAIN_FRAME, 0 => crate::altitudemodes::ABSOLUTE, _ => crate::altitudemodes::RELATIVE }, |a| a.mode);
    match mode {
        crate::altitudemodes::RELATIVE => seventh + home_altitude,
        _ => seventh,
    }
}

struct Survey {
    unfinished: bool,
    landing: bool,
    touchdown_altitude: Option<f64>,
    entry: (f64, f64),
    exit: (f64, f64),
    amsl: f64,
    exit_amsl: f64,
    lowest: f64,
    highest: f64,
    shots: i64,
    distance: f64,
}

fn structure(json: &Value, home_altitude: f64) -> Result<Survey, String> {
    let flight = crate::structurescan::saved_flight(json)?;
    let entry = *flight.first().ok_or("A structure scan without an outline has no rows to describe.")?;
    let plan = crate::structurescan::saved_plan(json);
    let (top, bottom) = crate::structurescan::top_and_bottom(&plan);
    Ok(Survey {
        unfinished: false,
        landing: false,
        touchdown_altitude: None,
        entry,
        exit: entry,
        amsl: plan.entrance_alt + home_altitude,
        exit_amsl: plan.entrance_alt + home_altitude,
        lowest: bottom.min(plan.entrance_alt) + home_altitude,
        highest: top.max(plan.entrance_alt) + home_altitude,
        shots: crate::structurescan::camera_shots(&flight, plan.adjusted_side, plan.layers),
        distance: crate::structurescan::scan_distance(&flight, &plan),
    })
}

fn landing(json: &Value, home_altitude: f64) -> Result<Survey, String> {
    let row = crate::landingpattern::row(json).ok_or("A landing pattern needs an approach and a landing coordinate.")?;
    let base = if row.relative { home_altitude } else { 0.0 };
    Ok(Survey {
        unfinished: json.get(crate::landingpattern::WIZARD).and_then(Value::as_bool) == Some(true),
        landing: true,
        touchdown_altitude: Some(row.land_altitude),
        entry: row.approach,
        exit: row.land,
        amsl: row.approach_altitude + base,
        exit_amsl: row.land_altitude + base,
        lowest: row.land_altitude + base,
        highest: row.approach_altitude + base,
        shots: 0,
        distance: row.distance,
    })
}

fn survey(json: &Value, home_altitude: f64) -> Result<Survey, String> {
    if json.get("complexItemType").and_then(Value::as_str).is_some_and(crate::landingpattern::is_landing) {
        return landing(json, home_altitude);
    }
    if json.get("complexItemType").and_then(Value::as_str) == Some("StructureScan") {
        return structure(json, home_altitude);
    }
    let transect = json.get("TransectStyleComplexItem").ok_or("A survey has no transect data.")?;
    let points: Vec<(f64, f64)> = transect
        .get("VisualTransectPoints")
        .and_then(Value::as_array)
        .map(|points| points.iter().filter_map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?))).collect())
        .unwrap_or_default();
    let (Some(entry), Some(exit)) = (points.first().copied(), points.last().copied()) else {
        return Err("A survey without transects has no rows to describe.".to_string());
    };
    let calc = transect.get("CameraCalc").ok_or("A survey has no camera settings.")?;
    let surface = calc.get("DistanceToSurface").and_then(Value::as_f64).ok_or("A survey has no distance to the surface.")?;
    let generic = crate::cmdinfo::tree(crate::cmdinfo::Firmware::Generic, crate::cmdinfo::VehicleClass::Generic);
    let saved: Vec<(f64, f64, f64)> = transect
        .get("Items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|item| item.get("command").and_then(Value::as_i64).and_then(|c| generic.get(&c)).is_some_and(|info| info.specifies_coordinate && !info.standalone_coordinate))
                .filter_map(|item| {
                    let params = item.get("params")?.as_array()?;
                    Some((params.get(4)?.as_f64()?, params.get(5)?.as_f64()?, params.get(6)?.as_f64()?))
                })
                .collect()
        })
        .unwrap_or_default();
    let terrain = |at: Option<&(f64, f64, f64)>| at.and_then(|(latitude, longitude, altitude)| crate::terrainservice::height(*latitude, *longitude).map(|ground| altitude + ground)).unwrap_or(f64::NAN);
    let saved_band = saved.iter().fold((f64::NAN, f64::NAN), |(low, high), (_, _, altitude)| (low.min(*altitude), high.max(*altitude)));
    let (amsl, exit_amsl, lowest, highest) = match calc.get("DistanceMode").and_then(Value::as_i64) {
        Some(crate::altitudemodes::RELATIVE) => (surface + home_altitude, surface + home_altitude, surface + home_altitude, surface + home_altitude),
        Some(crate::altitudemodes::ABSOLUTE) => (surface, surface, surface, surface),
        Some(crate::altitudemodes::CALC_ABOVE_TERRAIN) => (saved.first().map_or(f64::NAN, |p| p.2), saved.last().map_or(f64::NAN, |p| p.2), saved_band.0, saved_band.1),
        Some(crate::altitudemodes::TERRAIN_FRAME) => (terrain(saved.first()), terrain(saved.last()), surface, surface),
        _ => return Err("A survey has an unknown altitude mode.".to_string()),
    };
    Ok(Survey {
        unfinished: false,
        landing: false,
        touchdown_altitude: None,
        entry,
        exit,
        amsl,
        exit_amsl,
        lowest,
        highest,
        shots: transect.get("CameraShots").and_then(Value::as_i64).unwrap_or(0),
        distance: points.windows(2).map(|pair| crate::surveygrid::distance_between(pair[0], pair[1])).sum(),
    })
}

struct Flight {
    entry: (f64, f64),
    exit: (f64, f64),
    amsl: f64,
    exit_amsl: f64,
    band: (f64, f64),
    is_land: bool,
    within: f64,
}

fn flight(item: &crate::plandoc::Item, commands: &std::collections::BTreeMap<i64, crate::cmdinfo::Command>, home_altitude: f64) -> Option<Flight> {
    match item {
        crate::plandoc::Item::Simple(s) => {
            let info = commands.get(&s.command)?;
            (info.specifies_coordinate && !info.standalone_coordinate).then(|| {
                let at = (s.params[4].unwrap_or(0.0), s.params[5].unwrap_or(0.0));
                let amsl = amsl_entry(s, home_altitude);
                Flight { entry: at, exit: at, amsl, exit_amsl: amsl, band: (amsl, amsl), is_land: info.is_land, within: 0.0 }
            })
        }
        crate::plandoc::Item::Complex { json, .. } => survey(json, home_altitude).ok().map(|v| Flight { entry: v.entry, exit: v.exit, amsl: v.amsl, exit_amsl: v.exit_amsl, band: (v.lowest, v.highest), is_land: v.landing, within: v.distance }),
    }
}

fn legs(doc: &crate::plandoc::Document, commands: &std::collections::BTreeMap<i64, crate::cmdinfo::Command>) -> Vec<Leg> {
    let home = doc.home.unwrap_or([0.0, 0.0, 0.0]);
    let flights: Vec<Option<Flight>> = doc.items.iter().map(|item| flight(item, commands, home[2])).collect();
    let first_flight = flights.iter().position(Option::is_some).unwrap_or(flights.len());
    let takes_off_first = doc.items.iter().take(first_flight + 1).any(|item| matches!(item, crate::plandoc::Item::Simple(s) if s.command == 22 || s.command == 84));
    let link_start_to_home = doc.home.is_some() && takes_off_first;
    struct Walk {
        last: Option<((f64, f64), f64, bool)>,
        total: f64,
        rtl: bool,
    }
    let unflown = || Leg { azimuth: 0.0, distance: 0.0, alt_difference: 0.0, from_start: 0.0 };
    doc.items
        .iter()
        .zip(flights)
        .scan(Walk { last: None, total: 0.0, rtl: false }, |walk, (item, flown)| {
            walk.rtl = walk.rtl || matches!(item, crate::plandoc::Item::Simple(s) if s.command == 20);
            let Some(f) = flown.filter(|_| !walk.rtl) else {
                return Some(unflown());
            };
            let previous = walk.last.or(link_start_to_home.then_some(((home[0], home[1]), home[2], false)));
            let leg = match previous {
                Some((from, from_amsl, was_land)) => {
                    let distance = if was_land { 0.0 } else { crate::surveygrid::distance_between(from, f.entry) };
                    walk.total += distance;
                    Leg { azimuth: crate::surveygrid::azimuth_to(from, f.entry), distance, alt_difference: f.amsl - from_amsl, from_start: walk.total }
                }
                None => unflown(),
            };
            walk.total += f.within;
            walk.last = Some((f.exit, f.exit_amsl, f.is_land));
            Some(leg)
        })
        .collect()
}

pub struct Speeds {
    pub hover: f64,
    pub cruise: f64,
    pub ascent: f64,
    pub descent: f64,
}

#[derive(Debug, Default)]
pub struct FlightStatus {
    pub total_distance: f64,
    pub planned_distance: f64,
    pub hover_distance: f64,
    pub cruise_distance: f64,
    pub total_time: f64,
    pub max_telemetry: f64,
    pub min_amsl: f64,
    pub max_amsl: f64,
}

fn specified_speed(sections: &[crate::plandoc::Simple]) -> Option<f64> {
    sections.iter().find(|s| s.command == 178).and_then(|s| s.params[1])
}

fn additional_delay(item: &crate::plandoc::Item) -> f64 {
    match item {
        crate::plandoc::Item::Simple(s) => match s.command {
            16 | 112 | 93 => s.params[0].unwrap_or(0.0),
            _ => 0.0,
        },
        crate::plandoc::Item::Complex { .. } => 0.0,
    }
}

fn greatest_distance_to(json: &Value, to: (f64, f64)) -> f64 {
    if let Some(row) = json.get("complexItemType").and_then(Value::as_str).filter(|k| crate::landingpattern::is_landing(k)).and_then(|_| crate::landingpattern::row(json)) {
        return crate::surveygrid::distance_between(row.approach, to).max(crate::surveygrid::distance_between(row.land, to));
    }
    if json.get("complexItemType").and_then(Value::as_str) == Some("StructureScan") {
        return crate::structurescan::saved_flight(json).unwrap_or_default().iter().map(|at| crate::surveygrid::distance_between(*at, to)).fold(0.0, f64::max);
    }
    json.get("TransectStyleComplexItem")
        .and_then(|t| t.get("VisualTransectPoints"))
        .and_then(Value::as_array)
        .map(|points| points.iter().filter_map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?))).map(|at| crate::surveygrid::distance_between(at, to)).fold(0.0, f64::max))
        .unwrap_or(0.0)
}

const MAV_VTOL_STATE_MC: i64 = 3;
const MAV_VTOL_STATE_FW: i64 = 4;

pub fn flight_status(doc: &crate::plandoc::Document, speeds: &Speeds) -> Option<FlightStatus> {
    let class = crate::plandoc::vehicle_class(doc.vehicle_type);
    let vtol = class == crate::cmdinfo::VehicleClass::Vtol;
    let multirotor = class == crate::cmdinfo::VehicleClass::MultiRotor;
    let simples = || doc.items.iter().filter_map(|item| match item {
        crate::plandoc::Item::Simple(s) => Some(s),
        crate::plandoc::Item::Complex { .. } => None,
    });
    let before_rtl: Vec<&crate::plandoc::Simple> = simples().scan(false, |past, s| {
        let seen = !*past;
        *past = *past || s.command == 20;
        seen.then_some(s)
    }).collect();
    let starts_hovering = match vtol {
        true => before_rtl.iter().rev().find(|s| s.command == 22 || s.command == 84).is_some_and(|s| s.command == 84),
        false => multirotor,
    };
    let commands = crate::cmdinfo::tree(crate::plandoc::firmware(doc.firmware_type), class);
    let home = doc.home;
    let home_alt = home.map_or(0.0, |h| h[2]);
    struct Walk {
        status: FlightStatus,
        hovering: bool,
        hover: f64,
        cruise: f64,
        last: Option<((f64, f64), f64, bool)>,
        first_coordinate: bool,
        link: bool,
        rtl: bool,
        past_land: bool,
    }
    let add = |w: &mut Walk, distance: f64, extra: f64| {
        let time = distance / if w.hovering { w.hover } else { w.cruise } + extra;
        w.status.total_time += time;
        w.status.planned_distance += distance;
        match w.hovering {
            true => w.status.hover_distance += distance,
            false => w.status.cruise_distance += distance,
        }
    };
    let start = Walk {
        status: FlightStatus { min_amsl: f64::NAN, max_amsl: f64::NAN, ..FlightStatus::default() },
        hovering: starts_hovering,
        hover: speeds.hover,
        cruise: speeds.cruise,
        last: None,
        first_coordinate: true,
        link: false,
        rtl: false,
        past_land: false,
    };
    let settings_speed = specified_speed(&doc.settings_sections);
    let seeded = Walk {
        hover: if starts_hovering { settings_speed.unwrap_or(speeds.hover) } else { speeds.hover },
        cruise: if starts_hovering { speeds.cruise } else { settings_speed.unwrap_or(speeds.cruise) },
        ..start
    };
    let walked = doc.items.iter().fold(seeded, |mut w, item| {
        let simple = match item {
            crate::plandoc::Item::Simple(s) => Some(s),
            crate::plandoc::Item::Complex { .. } => None,
        };
        w.rtl = w.rtl || simple.is_some_and(|s| s.command == 20);
        if !w.rtl {
            if let Some(s) = simple.filter(|s| w.first_coordinate && (s.command == 22 || s.command == 84)) {
                if home.is_some() {
                    w.link = true;
                    if multirotor || vtol {
                        let climb = (home_alt - amsl_entry(s, home_alt)).abs() / speeds.ascent;
                        w.status.total_time += climb;
                    }
                }
            }
            if !w.past_land {
                add(&mut w, 0.0, additional_delay(item));
            }
            if let Some(f) = flight(item, &commands, home_alt) {
                let (low, high) = f.band;
                w.status.min_amsl = w.status.min_amsl.min(low);
                w.status.max_amsl = w.status.max_amsl.max(high);
                w.first_coordinate = false;
                let previous = w.last.or(w.link.then_some((home.map_or((0.0, 0.0), |h| (h[0], h[1])), home_alt, false)));
                if let (Some((from, _, was_land)), true) = (previous, w.last.is_some() || w.link) {
                    let distance = crate::surveygrid::distance_between(from, f.entry);
                    if !was_land {
                        w.status.total_distance += distance;
                        if !w.past_land {
                            add(&mut w, distance, 0.0);
                        }
                    }
                    let to_home = home.map_or(0.0, |h| crate::surveygrid::distance_between((h[0], h[1]), f.entry));
                    w.status.max_telemetry = w.status.max_telemetry.max(to_home);
                }
                if let crate::plandoc::Item::Complex { json, .. } = item {
                    w.status.max_telemetry = w.status.max_telemetry.max(greatest_distance_to(json, f.exit));
                    if !w.past_land {
                        add(&mut w, f.within, 0.0);
                    }
                    w.status.total_distance += f.within;
                }
                w.last = Some((f.exit, f.exit_amsl, f.is_land));
            }
        }
        let own_speed = |s: &crate::plandoc::Simple| (s.command == 178).then_some(s.params[1]).flatten().filter(|speed| *speed > 0.0);
        if let Some(changed) = simple.and_then(|s| specified_speed(&s.sections).or_else(|| own_speed(s))) {
            match w.hovering {
                true => w.hover = changed,
                false => w.cruise = changed,
            }
        }
        if let Some(s) = simple.filter(|_| vtol) {
            w.hovering = match (s.command, s.params[0].map(|p| p as i64)) {
                (22 | 84 | 21, _) => false,
                (85, _) => true,
                (3000, Some(MAV_VTOL_STATE_MC)) => true,
                (3000, Some(MAV_VTOL_STATE_FW)) => false,
                _ => w.hovering,
            };
        }
        w.past_land = w.past_land || simple.is_some_and(|s| commands.get(&s.command).is_some_and(|c| c.is_land)) || matches!(item, crate::plandoc::Item::Complex { kind, .. } if crate::landingpattern::is_landing(kind));
        w
    });
    let mut w = walked;
    if let (true, Some((exit, exit_amsl, _)), Some(h)) = (w.rtl, w.last, home) {
        let distance = crate::surveygrid::distance_between(exit, (h[0], h[1]));
        if !w.past_land {
            let land = (h[2] - exit_amsl).abs() / speeds.descent;
            add(&mut w, distance, land);
        }
    }
    if w.link {
        w.status.min_amsl = w.status.min_amsl.min(home_alt);
        w.status.max_amsl = w.status.max_amsl.max(home_alt);
    }
    Some(w.status)
}

fn altitude_fact(property: &str, metres: f64) -> Value {
    json!([{ "property": property, "value": metres, "rawValue": metres, "units": "m" }])
}

pub fn document_view(doc: &crate::plandoc::Document, selected: i64, vertical: &Unit, speed: &Unit, imperial: bool, rover: bool) -> Result<Value, String> {
    let items: Vec<Value> = document_reads(doc, selected)?.into_iter().enumerate().map(|(index, read)| item(&read, index as i64, vertical, speed, imperial)).collect();
    let items: Vec<Value> = match walked(&items) {
        true => items,
        false => items.into_iter().map(unwalked).collect(),
    };
    let has_items = !doc.items.is_empty();
    Ok(json!({
        "kind": "object",
        "class": "MissionItems",
        "available": has_items,
        "linksStartToHome": rover || starts_from_the_ground(&items),
        "editing": Value::Null,
        "selected": selected,
        "items": items,
        "reason": if has_items { "" } else { "This plan has no items yet." },
    }))
}

pub fn document_reads(doc: &crate::plandoc::Document, selected: i64) -> Result<Vec<Value>, String> {
    let surveys: Vec<Option<Survey>> = doc
        .items
        .iter()
        .map(|item| match item {
            crate::plandoc::Item::Complex { kind, json, .. } if kind == "survey" || kind == "CorridorScan" || kind == "StructureScan" || crate::landingpattern::is_landing(kind) => survey(json, doc.home.map_or(0.0, |h| h[2])).map(Some),
            crate::plandoc::Item::Complex { kind, .. } => Err(format!("The core cannot describe a {kind} item's rows yet.")),
            crate::plandoc::Item::Simple(_) => Ok(None),
        })
        .collect::<Result<_, _>>()?;
    let commands = crate::cmdinfo::tree(crate::plandoc::firmware(doc.firmware_type), crate::plandoc::vehicle_class(doc.vehicle_type));
    let home = doc.home.unwrap_or([0.0, 0.0, 0.0]);
    let settings = json!({
        "homePosition": true,
        "isSimpleItem": false,
        "specifiesCoordinate": true,
        "isStandaloneCoordinate": false,
        "sequenceNumber": 0,
        "lastSequenceNumber": doc.settings_sections.len(),
        "abbreviation": "Home",
        "commandName": SETTINGS_NAME,
        "commandDescription": SETTINGS_NAME,
        "isCurrentItem": selected == 0,
        "coordinate": { "latitude": home[0], "longitude": home[1], "altitude": home[2], "valid": doc.home.is_some() },
        "exitCoordinateSameAsEntry": true,
        "amslEntryAlt": home[2],
        "minAMSLAltitude": home[2],
        "maxAMSLAltitude": home[2],
        "altDifference": 0.0, "azimuth": 0.0, "distance": 0.0, "distanceFromStart": 0.0,
        "additionalTimeDelay": 0.0,
        "readyForSaveState": READY_TO_SAVE,
        "facts": altitude_fact("plannedHomePositionAltitude", home[2]),
    });
    let starts = doc.items.iter().scan(doc.settings_sections.len() + 1, |next, item| {
        let start = *next;
        *next += match item {
            crate::plandoc::Item::Simple(s) => 1 + s.sections.len(),
            crate::plandoc::Item::Complex { item_count, .. } => *item_count,
        };
        Some(start)
    });
    let reads: Vec<Value> = doc
        .items
        .iter()
        .zip(starts)
        .zip(legs(doc, &commands))
        .zip(surveys)
        .enumerate()
        .map(|(i, (((item, seq), leg), pattern))| {
            let crate::plandoc::Item::Simple(s) = item else {
                let (Some(v), crate::plandoc::Item::Complex { item_count, kind, .. }) = (pattern, item) else { return Value::Null };
                let (class, name, abbreviation) = match kind.as_str() {
                    "CorridorScan" => ("CorridorScanComplexItem", "Corridor Scan", "C"),
                    "StructureScan" => ("StructureScanComplexItem", "Structure Scan", "S"),
                    crate::landingpattern::VTOL_PATTERN => ("VTOLLandingComplexItem", "Landing Pattern", "L"),
                    crate::landingpattern::FIXED_WING_PATTERN => ("FixedWingLandingComplexItem", "Landing Pattern", "L"),
                    _ => ("SurveyComplexItem", "Survey", "S"),
                };
                return json!({
                    "class": class,
                    "isSimpleItem": false,
                    "isSurveyItem": class == "SurveyComplexItem",
                    "homePosition": false,
                    "specifiesCoordinate": true,
                    "isStandaloneCoordinate": false,
                    "sequenceNumber": seq,
                    "lastSequenceNumber": seq + item_count - 1,
                    "abbreviation": abbreviation,
                    "commandName": name,
                    "commandDescription": name,
                    "isCurrentItem": selected == i as i64 + 1,
                    "coordinate": { "latitude": if v.landing { v.exit.0 } else { v.entry.0 }, "longitude": if v.landing { v.exit.1 } else { v.entry.1 }, "altitude": v.touchdown_altitude, "valid": true },
                    "exitCoordinate": { "latitude": v.exit.0, "longitude": v.exit.1, "altitude": v.touchdown_altitude, "valid": true },
                    "exitCoordinateSameAsEntry": !v.landing && v.entry == v.exit,
                    "isLandCommand": v.landing,
                    "amslEntryAlt": v.amsl,
                    "minAMSLAltitude": v.lowest,
                    "maxAMSLAltitude": v.highest,
                    "cameraShots": (!v.landing).then_some(v.shots),
                    "complexDistance": v.distance,
                    "additionalTimeDelay": 0.0,
                    "altDifference": leg.alt_difference,
                    "azimuth": leg.azimuth,
                    "distance": leg.distance,
                    "distanceFromStart": leg.from_start,
                    "readyForSaveState": if v.unfinished { NOT_READY_FOR_SAVE } else { READY_TO_SAVE },
                    "readyForSaveMessage": if v.unfinished { "Finish the landing setup" } else { "" },
                });
            };
            let info = commands.get(&s.command);
            let coordinate = info.is_some_and(|c| c.specifies_coordinate);
            let altitude = s.altitude.as_ref();
            json!({
                "isSimpleItem": true,
                "homePosition": false,
                "specifiesCoordinate": coordinate,
                "isStandaloneCoordinate": info.is_some_and(|c| c.standalone_coordinate),
                "specifiesAltitudeOnly": info.is_some_and(|c| c.specifies_altitude_only),
                "specifiesAltitude": altitude.is_some(),
                "isTakeoffItem": info.is_some_and(|c| c.is_takeoff),
                "isLandCommand": info.is_some_and(|c| c.is_land),
                "sequenceNumber": seq,
                "lastSequenceNumber": seq + s.sections.len(),
                "abbreviation": abbreviation(s.command),
                "commandName": info.map(|c| c.friendly_name.clone()).unwrap_or_default(),
                "commandDescription": info.map(|c| c.description.clone()).unwrap_or_default(),
                "category": info.map(|c| c.category.clone()).unwrap_or_default(),
                "command": s.command,
                "isCurrentItem": selected == i as i64 + 1,
                "coordinate": coordinate.then(|| json!({ "latitude": s.params[4], "longitude": s.params[5], "altitude": Value::Null, "valid": true })),
                "exitCoordinateSameAsEntry": true,
                "amslEntryAlt": amsl_entry(s, home[2]),
                "altitudeFrame": altitude.map_or(match s.frame { 10 => crate::altitudemodes::TERRAIN_FRAME, 0 => crate::altitudemodes::ABSOLUTE, _ => crate::altitudemodes::RELATIVE }, |a| a.mode),
                "specifiedFlightSpeed": specified_speed(&s.sections).or_else(|| (s.command == 178).then_some(s.params[1]).flatten().filter(|speed| *speed > 0.0)),
                "facts": altitude_fact("altitude", altitude.map_or(0.0, |a| a.altitude)),
                "additionalTimeDelay": match s.command { 16 | 112 | 93 => s.params[0].unwrap_or(0.0), _ => 0.0 },
                "altDifference": leg.alt_difference,
                "azimuth": leg.azimuth,
                "distance": leg.distance,
                "distanceFromStart": leg.from_start,
                "readyForSaveState": READY_TO_SAVE,
            })
        })
        .collect();
    Ok(std::iter::once(settings).chain(reads).collect())
}

// MissionController::_recalcFlightPathSegments walks from i = 1: item 0 is the MissionSettingsItem
// and MissionSettingsItem::specifiesCoordinate() returns true unconditionally, so a walk that
// starts at 0 sees a flown leg on the settings row and stops before any takeoff.
fn starts_from_the_ground(items: &[Value]) -> bool {
    items
        .iter()
        .skip(1)
        .find_map(|item| match (item["command"].as_i64() == Some(RETURN_TO_LAUNCH), item["kind"] == "takeoff", item["flownLeg"] == true) {
            (true, ..) => Some(false),
            (_, true, _) => Some(true),
            (_, _, true) => Some(false),
            _ => None,
        })
        .unwrap_or(false)
}

fn placed(read: &Value) -> Option<Value> {
    at_key(read, "coordinate")
}

fn at_key(read: &Value, key: &str) -> Option<Value> {
    if !flag(read, "specifiesCoordinate") {
        return None;
    }
    let at = read.get(key)?;
    let number = |key: &str| at.get(key).and_then(Value::as_f64).filter(|value| value.is_finite());
    let (Some(latitude), Some(longitude)) = (number("latitude"), number("longitude")) else {
        return None;
    };
    let unset = latitude == 0.0 && longitude == 0.0;
    (at.get("valid").and_then(Value::as_bool) == Some(true) && !unset).then(|| at.clone())
}

fn item(read: &Value, index: i64, vertical: &Unit, speed: &Unit, imperial: bool) -> Value {
    let coordinate = placed(read);
    let exit = match flag(read, "exitCoordinateSameAsEntry") {
        true => None,
        false => at_key(read, "exitCoordinate").filter(|exit| Some(exit) != coordinate.as_ref()),
    };
    let ready = integer(read, "readyForSaveState");
    json!({
        "index": index,
        "sequence": integer(read, "sequenceNumber"),
        "foldedCommands": integer(read, "lastSequenceNumber")
            .zip(integer(read, "sequenceNumber"))
            .map(|(last, first)| (last - first).max(0)),
        "abbreviation": text(read, "abbreviation"),
        "name": text(read, "commandName"),
        "description": text(read, "commandDescription"),
        "kind": kind(read),
        "selected": flag(read, "isCurrentItem"),
        "coordinate": coordinate,
        "exitCoordinate": exit,
        "altitude": height(read),
        "altitudeText": height_metres(read).map(|metres| format_measure(vertical.show(metres), &vertical.name)),
        "altitudeUnits": height(read).map(|_| vertical.name.clone()),
        "altitudeEditUnits": fact_units(read, "altitude").or_else(|| fact_units(read, "plannedHomePositionAltitude")),
        "altitudeMetres": height_metres(read),
        "specifiesAltitude": flag(read, "isSimpleItem").then(|| flag(read, "specifiesAltitude")),
        "altitudeOnly": flag(read, "specifiesAltitudeOnly"),
        "category": Some(text(read, "category")).filter(|category| !category.is_empty()),
        "cameraShots": number(read, "cameraShots").map(|shots| shots as i64).filter(|shots| *shots > 0),
        "patternDistance": number(read, "complexDistance").filter(|metres| *metres > 0.0),
        "simple": read.get("isSimpleItem").and_then(Value::as_bool),
        "altitudeMode": frame(read).map(|mode| mode as i64),
        "altitudeAmsl": number(read, "amslEntryAlt"),
        "extraSeconds": number(read, "additionalTimeDelay"),
        "speedChange": number(read, "specifiedFlightSpeed"),
        "speedChangeText": number(read, "specifiedFlightSpeed").filter(|mps| mps.is_finite() && *mps > 0.0).map(|mps| format_measure(speed.show(mps), &speed.name)),
        "altitudeAmslLowest": number(read, "minAMSLAltitude"),
        "altitudeAmslHighest": number(read, "maxAMSLAltitude"),
        "altitudeBandText": band(read, vertical),
        "altitudeSource": altitude_source(read, vertical),
        "altitudeFrame": altitude_frame(read, vertical),
        "altitudeFrameText": altitude_frame(read, vertical).and_then(frame_word),
        "specifiesCoordinate": flag(read, "specifiesCoordinate"),
        "altitudeChange": number(read, "altDifference"),
        "altitudeChangeText": number(read, "altDifference").map(|change| crate::read::altitude_text(change, vertical, true)),
        "azimuth": number(read, "azimuth"),
        "azimuthText": number(read, "azimuth").map(|bearing| format!("{}\u{b0}", (bearing.round() as i64).rem_euclid(360))),
        "distance": number(read, "distance"),
        "distanceText": number(read, "distance").map(|metres| crate::missionsummary::distance_text(metres, imperial)),
        "gradientText": gradient_text(number(read, "altDifference"), number(read, "distance")),
        "distanceFromStart": number(read, "distanceFromStart"),
        "edited": flag(read, "dirty"),
        "incomplete": flag(read, "isIncomplete"),
        "endsRoute": flag(read, "isLandCommand") || integer(read, "command") == Some(RETURN_TO_LAUNCH),
        "command": integer(read, "command"),
        "flownLeg": flag(read, "specifiesCoordinate") && !flag(read, "isStandaloneCoordinate") && !flag(read, "isIncomplete"),
        "movable": coordinate.is_some(),
        "blocked": ready.is_some_and(|state| state != READY_TO_SAVE && state != AWAITING_TERRAIN),
        "awaitingTerrain": ready == Some(AWAITING_TERRAIN),
        "blockedReason": match ready {
            Some(state) if state != READY_TO_SAVE => Some(text(read, "readyForSaveMessage")).filter(|message| !message.is_empty()),
            _ => None,
        },
    })
}

fn editable(backend: &dyn Backend, current: i64) -> Value {
    if current <= 0 {
        return Value::Null;
    }
    match (fields_of(backend, current), speed_section(backend, current)) {
        (Value::Null, Value::Null) => Value::Null,
        (fields, speed) => json!({ "index": current, "fields": fields, "speedSection": speed }),
    }
}

// The macOS head read an item's speedSection raw for three things: whether the item can carry a
// speed change at all, whether it does, and the speed with its units. The value is read off the
// flightSpeed Fact the head writes, so what it shows and what it writes are one path.
pub(crate) fn speed_section(backend: &dyn Backend, index: i64) -> Value {
    let path = format!("plan.missionController.visualItems.{index}.speedSection");
    let section = object(&backend.get_fields(&path, "available,specifyFlightSpeed"));
    if section.get("kind").and_then(Value::as_str) != Some("object") {
        return Value::Null;
    }
    let value_path = format!("{path}.flightSpeed");
    let speed = object(&backend.get(&value_path));
    let is_fact = speed.get("kind").and_then(Value::as_str) == Some("fact");
    json!({
        "available": flag(&section, "available"),
        "specified": flag(&section, "specifyFlightSpeed"),
        "value": is_fact.then(|| speed.get("value").and_then(Value::as_f64).filter(|v| v.is_finite())).flatten(),
        "units": is_fact.then(|| speed.get("units").and_then(Value::as_str).filter(|u| !u.is_empty()).map(str::to_string)).flatten(),
        "path": value_path,
        "specifyPath": format!("{path}.specifyFlightSpeed"),
    })
}

pub fn gradient_text(alt_difference: Option<f64>, distance: Option<f64>) -> Option<String> {
    let (rise, run) = (alt_difference?, distance.filter(|d| *d > 0.0)?);
    Some(format!("{:.0} deg", (rise / run).atan().to_degrees()))
}

const LEG_FIGURES: [&str; 6] = ["distance", "distanceText", "distanceFromStart", "azimuth", "azimuthText", "altitudeChangeText"];

fn walked(items: &[Value]) -> bool {
    let legs = items.iter().filter(|item| flag(item, "flownLeg")).count();
    let reached = |item: &Value| item.get("distanceFromStart").and_then(Value::as_f64).is_some_and(|metres| metres > 0.0);
    legs < 2 || items.iter().any(reached)
}

fn unwalked(item: Value) -> Value {
    match item {
        Value::Object(mut fields) => {
            LEG_FIGURES.iter().for_each(|key| {
                fields.insert((*key).to_string(), Value::Null);
            });
            Value::Object(fields)
        }
        other => other,
    }
}

const MODE_RELATIVE: f64 = 1.0;
const MODE_ABSOLUTE: f64 = 2.0;
const MODE_CALC_ABOVE_TERRAIN: f64 = 3.0;
const MODE_TERRAIN_FRAME: f64 = 4.0;

pub fn frame_word(frame: &str) -> Option<&'static str> {
    match frame {
        "terrain" => Some("AGL"),
        "amsl" => Some("AMSL"),
        "launch" => Some(""),
        _ => None,
    }
}

fn altitude_frame(read: &Value, vertical: &Unit) -> Option<&'static str> {
    if flag(read, "homePosition") {
        return Some("amsl");
    }
    if band(read, vertical).is_some() {
        return Some("amsl");
    }
    height(read)?;
    match frame(read)? {
        MODE_RELATIVE => Some("launch"),
        MODE_ABSOLUTE => Some("amsl"),
        MODE_CALC_ABOVE_TERRAIN | MODE_TERRAIN_FRAME => Some("terrain"),
        _ => None,
    }
}

pub(crate) fn frame(read: &Value) -> Option<f64> {
    number(read, "altitudeFrame").or_else(|| number(read, "altitudeMode"))
}

fn altitude_source(read: &Value, vertical: &Unit) -> Option<&'static str> {
    match (band(read, vertical).is_some(), height(read).is_some()) {
        (true, _) => Some("band"),
        (false, true) => Some("text"),
        (false, false) => None,
    }
}

fn band(read: &Value, vertical: &Unit) -> Option<String> {
    if flag(read, "homePosition") {
        return None;
    }
    let edge = |key: &str| number(read, key).filter(|metres| metres.is_finite());
    match (edge("minAMSLAltitude"), edge("maxAMSLAltitude")) {
        (Some(low), Some(high)) if high > low => Some(crate::read::range_text(low, high, vertical)),
        (Some(low), Some(high)) if high == low => Some(crate::read::altitude_text(low, vertical, false)),
        _ => None,
    }
}

fn height_metres(read: &Value) -> Option<f64> {
    if flag(read, "isSimpleItem") && !flag(read, "specifiesAltitude") {
        return None;
    }
    match flag(read, "homePosition") {
        true => fact_raw_number(read, "altitude").or_else(|| fact_raw_number(read, "plannedHomePositionAltitude")),
        false => fact_raw_number(read, "altitude"),
    }
}

fn height(read: &Value) -> Option<f64> {
    if flag(read, "isSimpleItem") && !flag(read, "specifiesAltitude") {
        return None;
    }
    match flag(read, "homePosition") {
        true => fact_number(read, "altitude").or_else(|| fact_number(read, "plannedHomePositionAltitude")),
        false => fact_number(read, "altitude"),
    }
}

fn layer_altitudes(item: &Value, layers: i64) -> Option<Vec<f64>> {
    let bottom = number(item, "bottomFlightAlt")?;
    let top = number(item, "topFlightAlt")?;
    let steps = (layers - 1).max(0) as f64;
    let increment = match steps > 0.0 {
        true => (top - bottom) / steps,
        false => 0.0,
    };
    Some((0..layers.max(1)).map(|layer| bottom + increment * layer as f64).collect())
}

fn path_at(backend: &dyn Backend, path: &str) -> Vec<Value> {
    object(&backend.get(path))
        .get("value")
        .and_then(Value::as_array)
        .map(|points| {
            points
                .iter()
                .filter_map(|at| Some(json!({ "latitude": at.get("latitude")?.as_f64()?, "longitude": at.get("longitude")?.as_f64()? })))
                .collect()
        })
        .unwrap_or_default()
}

fn geometry_of(backend: &dyn Backend, index: i64, kind: &str, vertical: &Unit) -> Value {
    let Some(shape) = crate::missionkinds::lookup(kind).and_then(|kind| kind.geometry) else {
        return Value::Null;
    };
    let vertices = object(&backend.get(&format!("plan.missionController.visualItems.{index}.{}.path", shape.1)));
    let listed: Vec<Value> = vertices
        .get("value")
        .and_then(Value::as_array)
        .map(|points| {
            points
                .iter()
                .filter_map(|at| Some(json!({ "latitude": at.get("latitude")?.as_f64()?, "longitude": at.get("longitude")?.as_f64()? })))
                .collect()
        })
        .unwrap_or_default();
    let transects: Vec<Value> = object(&backend.get(&format!("plan.missionController.visualItems.{index}.visualTransectPoints")))
        .get("value")
        .and_then(Value::as_array)
        .map(|points| {
            points
                .iter()
                .filter_map(|at| Some(json!({ "latitude": at.get("latitude")?.as_f64()?, "longitude": at.get("longitude")?.as_f64()? })))
                .collect()
        })
        .unwrap_or_default();
    let base = format!("plan.missionController.visualItems.{index}");
    let flown_loop = path_at(backend, &format!("{base}.flightPolygon.path"));
    let item = object(&backend.get(&base));
    let layers = fact_number(&item, "layers").map(|count| count as i64);
    let stack = layers.and_then(|count| layer_altitudes(&item, count));
    match listed.is_empty() {
        true => Value::Null,
        false => json!({
            "shape": shape.0,
            "property": shape.1,
            "vertices": listed,
            "transects": transects,
            "flightLoop": (!flown_loop.is_empty()).then_some(flown_loop),
            "layers": layers,
            "layerAltitudesMetres": stack.clone(),
            "layerSpanText": stack.as_ref().and_then(|heights| {
                let (low, high) = (heights.first()?, heights.last()?);
                Some(crate::read::range_text(*low, *high, vertical))
            }),
        }),
    }
}

fn editable_facts(facts: &Value, path: &str) -> Vec<Value> {
    facts
        .get("facts")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|fact| {
                    let property = fact.get("property").and_then(Value::as_str).filter(|property| !property.is_empty())?;
                    let described = fact.get("shortDescription").and_then(Value::as_str).filter(|described| !described.is_empty());
                    let named = fact.get("name").and_then(Value::as_str).filter(|name| !name.is_empty());
                    Some(json!({
                        "name": property,
                        "label": described.or(named).unwrap_or(property),
                        "units": fact.get("units").and_then(Value::as_str).filter(|units| !units.is_empty()),
                        "value": fact.get("value"),
                        "text": fact.get("enumOrValueString").and_then(Value::as_str),
                        "choices": fact.get("enumStrings"),
                        "path": format!("{path}.{property}"),
                    }))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn fields_of(backend: &dyn Backend, index: i64) -> Value {
    if index <= 0 {
        return Value::Null;
    }
    let path = format!("plan.missionController.visualItems.{index}");
    let own = editable_facts(&object(&backend.get(&path)), &path);
    let camera_path = format!("{path}.cameraCalc");
    let camera = editable_facts(&object(&backend.get(&camera_path)), &camera_path);
    let fields: Vec<Value> = own.into_iter().chain(camera).collect();
    match fields.is_empty() {
        true => Value::Null,
        false => Value::Array(fields),
    }
}

fn fact_units(read: &Value, name: &str) -> Option<String> {
    read.get("facts")
        .and_then(Value::as_array)
        .and_then(|facts| facts.iter().find(|fact| fact.get("property").and_then(Value::as_str) == Some(name)))
        .and_then(|fact| fact.get("units"))
        .and_then(Value::as_str)
        .filter(|units| !units.is_empty())
        .map(str::to_string)
}

fn fact_raw_number(read: &Value, name: &str) -> Option<f64> {
    read.get("facts")
        .and_then(Value::as_array)
        .and_then(|facts| facts.iter().find(|fact| fact.get("property").and_then(Value::as_str) == Some(name)))
        .and_then(|fact| fact.get("rawValue").or_else(|| fact.get("value")))
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}

fn fact_number(read: &Value, name: &str) -> Option<f64> {
    read.get("facts")
        .and_then(Value::as_array)
        .and_then(|facts| facts.iter().find(|fact| fact.get("property").and_then(Value::as_str) == Some(name)))
        .and_then(|fact| fact.get("value"))
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}

fn number(read: &Value, key: &str) -> Option<f64> {
    read.get(key).and_then(Value::as_f64).filter(|value| value.is_finite())
}

fn kind(read: &Value) -> &'static str {
    if flag(read, "homePosition") {
        return "settings";
    }
    if flag(read, "isSurveyItem") {
        return "survey";
    }
    if flag(read, "isTakeoffItem") {
        return "takeoff";
    }
    if flag(read, "isLandCommand") {
        return "land";
    }
    if read.get("isSimpleItem").and_then(Value::as_bool) == Some(false) {
        return crate::missionkinds::by_class(&text(read, "class")).map(|kind| kind.id).unwrap_or("complex");
    }
    if !flag(read, "isSimpleItem") {
        return "unreadable";
    }
    if flag(read, "specifiesAltitudeOnly") {
        return "altitude";
    }
    match flag(read, "specifiesCoordinate") && !flag(read, "isStandaloneCoordinate") {
        true => "waypoint",
        false => "command",
    }
}

#[cfg(test)]
mod from_the_document {
    use super::*;

    fn metres() -> (Unit, Unit) {
        (Unit { name: "m".to_string(), factor: 1.0 }, Unit { name: "m/s".to_string(), factor: 1.0 })
    }

    fn by_value(value: Value) -> Value {
        match value {
            Value::Number(n) => json!((n.as_f64().unwrap() * 1e6).round() / 1e6),
            Value::Array(items) => Value::Array(items.into_iter().map(by_value).collect()),
            Value::Object(fields) => Value::Object(fields.into_iter().map(|(k, v)| (k, by_value(v))).collect()),
            other => other,
        }
    }

    fn agrees(plan: &str, terrain_under_home: f64, qt: &str) {
        let loaded = crate::plandoc::load(plan).unwrap();
        let doc = crate::plandoc::Document { home: loaded.home.map(|h| [h[0], h[1], terrain_under_home]), ..loaded };
        let (vertical, speed) = metres();
        let mine = by_value(document_view(&doc, 0, &vertical, &speed, false, false).unwrap());
        let qt = by_value(serde_json::from_str(qt).unwrap());
        let rows = |v: &Value| v["items"].as_array().unwrap().clone();
        assert_eq!(rows(&mine).len(), rows(&qt).len());
        rows(&mine).iter().zip(rows(&qt)).for_each(|(core, qt)| {
            let differing: Vec<String> = qt.as_object().unwrap().iter().filter(|(k, v)| core.get(k.as_str()) != Some(v)).map(|(k, v)| format!("{k}: core {} qt {v}", core.get(k.as_str()).unwrap_or(&Value::Null))).collect();
            assert!(differing.is_empty(), "row {}: {}", qt["index"], differing.join("; "));
        });
    }

    #[test]
    fn an_ardupilot_takeoff_has_no_position_yet_still_links_the_first_leg_to_home() {
        let qt: Value = serde_json::from_str(include_str!("../tests/fixtures/missionitems-ardupilot-takeoff-by-qt.json")).unwrap();
        let home_altitude = qt["items"][0]["altitudeMetres"].as_f64().unwrap();
        agrees(include_str!("../tests/fixtures/ardupilot-takeoff-without-coordinate.plan"), home_altitude, include_str!("../tests/fixtures/missionitems-ardupilot-takeoff-by-qt.json"));
    }

    fn status_matches(plan: &str, home_altitude: f64, qt: &str) {
        let loaded = crate::plandoc::load(plan).unwrap();
        let doc = crate::plandoc::Document { home: loaded.home.map(|h| [h[0], h[1], home_altitude]), ..loaded };
        let status = flight_status(&doc, &Speeds { hover: 5.0, cruise: 15.0, ascent: 3.0, descent: 1.0 }).unwrap();
        let qt: Value = serde_json::from_str(qt).unwrap();
        let close = |mine: f64, key: &str| (mine - qt[key].as_f64().unwrap()).abs() < 1e-6;
        assert!(close(status.total_distance, "distanceMetres"), "distance {} qt {}", status.total_distance, qt["distanceMetres"]);
        assert!(close(status.total_time, "timeSeconds"), "time {} qt {}", status.total_time, qt["timeSeconds"]);
        assert!(close(status.max_telemetry, "maxTelemetryMetres"), "telemetry {} qt {}", status.max_telemetry, qt["maxTelemetryMetres"]);
        assert_eq!(json!([status.min_amsl, status.max_amsl]), qt["altitudeBandMetres"]);
    }

    #[test]
    fn the_flight_status_of_the_core_plan_is_qts() {
        status_matches(include_str!("../../test/MissionManager/SectionTest.plan"), 35.0, include_str!("../tests/fixtures/summary-SectionTest-by-qt.json"));
        status_matches(include_str!("../tests/fixtures/survey-upload.plan"), 585.0, include_str!("../tests/fixtures/summary-survey-upload-by-qt.json"));
        status_matches(include_str!("../tests/fixtures/ardupilot-takeoff-without-coordinate.plan"), 585.0, include_str!("../tests/fixtures/summary-sitl-base-by-qt.json"));
    }

    #[test]
    fn a_survey_row_carries_its_pattern_as_qt_describes_it() {
        agrees(include_str!("../tests/fixtures/survey-upload.plan"), 585.0, include_str!("../tests/fixtures/missionitems-survey-by-qt.json"));
    }

    #[test]
    fn the_rows_of_a_plan_the_core_holds_are_the_rows_qt_shows_for_it() {
        let loaded = crate::plandoc::load(include_str!("../../test/MissionManager/SectionTest.plan")).unwrap();
        let terrain_under_home = 35.0;
        let doc = crate::plandoc::Document { home: loaded.home.map(|h| [h[0], h[1], terrain_under_home]), ..loaded };
        let (vertical, speed) = metres();
        let mine = by_value(document_view(&doc, 0, &vertical, &speed, false, false).unwrap());
        let qt = by_value(serde_json::from_str(include_str!("../tests/fixtures/missionitems-sectiontest-by-qt.json")).unwrap());
        let rows = |v: &Value| v["items"].as_array().unwrap().clone();
        assert_eq!(rows(&mine).len(), rows(&qt).len());
        rows(&mine).iter().zip(rows(&qt)).for_each(|(core, qt)| {
            let differing: Vec<String> = qt.as_object().unwrap().iter().filter(|(k, v)| core.get(k.as_str()) != Some(v)).map(|(k, v)| format!("{k}: core {} qt {v}", core.get(k.as_str()).unwrap_or(&Value::Null))).collect();
            assert!(differing.is_empty(), "row {}: {}", qt["index"], differing.join("; "));
        });
        let top = |v: &Value| v.as_object().unwrap().iter().filter(|(k, _)| *k != "items").map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>();
        assert_eq!(top(&mine), top(&qt));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leg_gradient_is_the_climb_angle_qgc_shows_and_needs_a_leg() {
        assert_eq!(gradient_text(Some(100.0), Some(100.0)).as_deref(), Some("45 deg"));
        assert_eq!(gradient_text(Some(-10.0), Some(1000.0)).as_deref(), Some("-1 deg"));
        assert_eq!(gradient_text(Some(10.0), Some(0.0)), None);
        assert_eq!(gradient_text(None, Some(10.0)), None);
    }

    #[test]
    fn a_vtol_flies_hover_and_cruise_legs_by_its_transitions_as_qt_times_them() {
        let doc = crate::plandoc::load(include_str!("../tests/fixtures/vtol-transitions.plan")).unwrap();
        let status = flight_status(&doc, &Speeds { hover: 6.0, cruise: 18.0, ascent: 3.0, descent: 1.0 }).unwrap();
        assert!((status.total_time - 561.2644666123344).abs() < 1e-6, "Qt timed this plan at 561.26 s, core {}", status.total_time);
        assert!((status.total_distance - 3546.2558210867846).abs() < 1e-6);
        assert_eq!(doc.items.len(), 9, "a VTOL has no speed section, so its speed changes stay rows of their own");
    }

    struct Plan(Vec<Value>, i64);

    impl Backend for Plan {
        fn get(&self, path: &str) -> String {
            match path {
                "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": self.0.len() }).to_string(),
                "plan.missionController.currentPlanViewVIIndex" => json!({ "kind": "value", "value": self.1 }).to_string(),
                _ => String::new(),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            if path == "plan.missionController" {
                return json!({ "kind": "object", "containsItems": self.0.len() > 1 }).to_string();
            }
            match path.rsplit_once('.').and_then(|(_, index)| index.parse::<usize>().ok()).and_then(|index| self.0.get(index)) {
                Some(item) => item.to_string(),
                None => String::new(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    struct Ground(Vec<Value>, bool);
    impl Backend for Ground {
        fn get(&self, path: &str) -> String { Plan(self.0.clone(), -1).get(path) }
        fn get_fields(&self, path: &str, fields: &str) -> String {
            match path {
                "plan.controllerVehicle" => json!({ "kind": "object", "rover": self.1 }).to_string(),
                _ => Plan(self.0.clone(), -1).get_fields(path, fields),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn command(number: i64) -> Value {
        json!({ "kind": "object", "sequenceNumber": number, "abbreviation": "DO", "commandName": "Set Camera Mode",
                "isSimpleItem": true, "specifiesCoordinate": false, "command": 530, "readyForSaveState": 0 })
    }

    fn returning() -> Value {
        json!({ "kind": "object", "sequenceNumber": 9, "abbreviation": "RTL", "commandName": "Return To Launch",
                "isSimpleItem": true, "specifiesCoordinate": false, "command": RETURN_TO_LAUNCH, "readyForSaveState": 0 })
    }

    #[test]
    fn the_route_is_drawn_back_to_home_when_the_mission_starts_from_the_ground() {
        let links = |items: Vec<Value>, rover: bool| items_view(&Ground(items, rover), &[])["linksStartToHome"].clone();

        assert_eq!(links(vec![settings(), takeoff(), waypoint(2, 100.0)], false), json!(true), "MissionController links the first item back to home when a takeoff is reached before any coordinate item, because that is a mission starting from the ground");
        assert_eq!(links(vec![settings(), waypoint(1, 100.0), waypoint(2, 100.0)], false), json!(false), "a mission that begins at a waypoint is flown to, not launched from home");

        assert_eq!(
            links(vec![settings(), command(1), takeoff(), waypoint(3, 100.0)], false),
            json!(true),
            "a DO_ command between the settings row and the takeoff does not specify a coordinate, so the takeoff is still the FIRST coordinate item - both heads were reading index 1 and would have missed this"
        );
        assert_eq!(links(vec![settings(), waypoint(1, 100.0), takeoff()], false), json!(false), "a takeoff AFTER the first coordinate item is not a mission starting from the ground, and firstCoordinateNotFound is what QGC tests");
        assert_eq!(links(vec![settings(), returning(), takeoff()], false), json!(false), "the takeoff clause is guarded by !linkEndToHome, so an RTL earlier in the list suppresses it");

        assert_eq!(links(vec![settings(), waypoint(1, 100.0)], true), json!(true), "a rover always links start to home whatever its items say - _controllerVehicle->rover() seeds the flag before the walk begins");
        assert_eq!(items_view(&Ground(vec![], false), &[])["linksStartToHome"], json!(false), "an empty plan answers the question rather than omitting the key - the contract records this field on every missionItems read, and a head finding it absent has to invent what absence means");
        assert_eq!(links(vec![settings(), waypoint(1, 100.0)], false), json!(false), "and the same plan on anything else does not, which is the half a head keying on the translated vehicle type string got wrong outside English");
    }

    fn at(latitude: f64, longitude: f64) -> Value {
        json!({ "kind": "coordinate", "valid": true, "latitude": latitude, "longitude": longitude, "altitude": 50.0 })
    }

    fn settings() -> Value {
        json!({ "kind": "object", "homePosition": true, "sequenceNumber": 0, "abbreviation": "Launch", "commandName": "Mission Settings",
                "isSimpleItem": false, "specifiesCoordinate": true, "coordinate": at(47.0, 8.0) })
    }

    fn takeoff() -> Value {
        json!({
            "kind": "object", "sequenceNumber": 1, "abbreviation": "Takeoff", "commandName": "Takeoff",
            "isSimpleItem": true, "isTakeoffItem": true, "specifiesCoordinate": true, "coordinate": at(47.0, 8.0),
            "amslEntryAlt": 520.0, "distance": 0.0, "distanceFromStart": 0.0, "readyForSaveState": 0,
        })
    }

    fn pattern_item() -> Value {
        json!({ "kind": "object", "sequenceNumber": 5, "abbreviation": "FWL", "commandName": "Fixed Wing Landing", "isSimpleItem": false })
    }

    fn waypoint(sequence: i64, distance: f64) -> Value {
        json!({
            "kind": "object", "sequenceNumber": sequence, "abbreviation": sequence.to_string(), "commandName": "Waypoint",
            "isSimpleItem": true, "specifiesCoordinate": true, "coordinate": at(47.1, 8.1),
            "amslEntryAlt": 540.0, "altDifference": 20.0, "azimuth": 45.0, "distance": distance,
            "distanceFromStart": distance, "readyForSaveState": 0, "dirty": true,
        })
    }

    fn survey() -> Value {
        json!({
            "kind": "object", "sequenceNumber": 3, "abbreviation": "S", "commandName": "Survey",
            "isSimpleItem": false, "isSurveyItem": true, "specifiesCoordinate": true, "coordinate": at(47.2, 8.2),
            "readyForSaveState": 2, "readyForSaveMessage": "The survey needs an area before it can be saved.",
        })
    }

    #[test]
    fn the_index_is_the_position_in_qts_list_and_never_the_sequence_number() {
        struct Listed(Vec<Value>);
        impl Backend for Listed {
            fn get(&self, path: &str) -> String {
                match path {
                    "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": self.0.len() }).to_string(),
                    "plan.missionController.currentPlanViewVIIndex" => json!({ "kind": "value", "value": 0 }).to_string(),
                    _ => String::new(),
                }
            }
            fn get_fields(&self, path: &str, _fields: &str) -> String {
                match path {
                    "plan.missionController" => json!({ "kind": "object", "containsItems": true }).to_string(),
                    "plan.missionController.visualItems" => json!({ "kind": "object", "elements": self.0 }).to_string(),
                    _ => String::new(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        let plan = vec![settings(), takeoff(), pattern_item(), waypoint(9, 120.0)];
        let served = |view: Value| view["items"].as_array().unwrap().clone();

        [served(items_view(&Listed(plan.clone()), &[])), served(items_view(&Plan(plan.clone(), 0), &[]))]
            .iter()
            .for_each(|listed| {
                assert_eq!(listed.len(), 4, "both the elements branch and the per-index fallback are live and feed the same head, and neither drops anything, so a position in this list is a position in Qt's");
                listed.iter().enumerate().for_each(|(at, item)| {
                    assert_eq!(item["index"], json!(at as i64), "Mission.swift builds raw paths as plan.missionController.visualItems.<index>.<property> and WRITES through them, so an index that is a position in a filtered list moves a waypoint the operator did not touch");
                });
                assert_eq!(listed[3]["sequence"], json!(9), "sequence is the item's own sequenceNumber and diverges from the position as soon as a complex item is in the plan; serving one where the other is meant is invisible until it is");
                assert_ne!(listed[3]["index"], listed[3]["sequence"], "and this plan is one where they diverge, so the assertions above are not agreeing by accident");
            });
    }

    #[test]
    fn a_plan_lists_its_items_once_rather_than_a_path_per_property() {
        let view = items_view(&Plan(vec![settings(), takeoff(), waypoint(2, 400.0)], 1), &[]);
        assert_eq!(view["available"], true);
        let items = view["items"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["kind"], "settings");
        assert_eq!(items[1]["kind"], "takeoff");
        assert_eq!(items[2]["kind"], "waypoint");
        assert_eq!(items[2]["distance"], 400.0);
        assert_eq!(items[2]["azimuth"], 45.0);
        assert_eq!(items[1]["coordinate"]["latitude"], 47.0);
        assert_eq!(view["selected"], 1);
    }

    #[test]
    fn an_item_that_cannot_be_saved_says_so_and_says_why() {
        let view = items_view(&Plan(vec![settings(), survey()], 1), &[]);
        let survey = &view["items"].as_array().unwrap()[1];
        assert_eq!(survey["kind"], "survey");
        assert_eq!(survey["blocked"], true);
        assert_eq!(survey["blockedReason"], "The survey needs an area before it can be saved.");
        let fine = items_view(&Plan(vec![settings(), takeoff()], 1), &[]);
        assert_eq!(fine["items"][1]["blocked"], false);
        assert_eq!(fine["items"][1]["blockedReason"], Value::Null, "an item with nothing wrong carries no reason, so a head drawing one is drawing a real problem");
    }

    #[test]
    fn an_item_with_no_place_on_the_map_carries_no_coordinate_rather_than_a_zero_one() {
        let command = json!({ "kind": "object", "sequenceNumber": 4, "abbreviation": "ROI", "commandName": "Region of interest", "isSimpleItem": true, "specifiesCoordinate": false, "coordinate": { "kind": "coordinate", "valid": false, "latitude": 0.0, "longitude": 0.0 } });
        let view = items_view(&Plan(vec![settings(), command], 0), &[]);
        let listed = &view["items"].as_array().unwrap()[1];
        assert_eq!(listed["coordinate"], Value::Null, "an invalid coordinate drawn on a map is a pin in the Atlantic");
        assert_eq!(listed["kind"], "command");
    }

    #[test]
    fn a_plan_holding_only_its_settings_entry_is_an_empty_plan() {
        let bare = items_view(&Plan(vec![settings()], 0), &[]);
        assert_eq!(bare["available"], false, "removing everything leaves the settings entry, so a count of one is not a plan with something in it");
        assert_eq!(bare["items"].as_array().unwrap().len(), 1, "the entry is still listed, because a head may want to draw the planned home position");
        assert!(!bare["reason"].as_str().unwrap().is_empty());

        let none = items_view(&Plan(Vec::new(), -1), &[]);
        assert_eq!(none["available"], false);
        assert_eq!(none["items"].as_array().unwrap().len(), 0);
        assert_eq!(none["selected"], -1);
    }

    const ALWAYS_PRESENT: [&str; 4] = ["kind", "class", "facts", "children"];

    #[test]
    fn every_field_this_view_reads_is_one_it_asked_for() {
        let body = include_str!("missionitems.rs").split("#[cfg(test)]").next().unwrap().to_string();
        let requested: Vec<&str> = FIELDS.split(',').map(str::trim).collect();
        let read: Vec<String> = ["flag(read, \"", "number(read, \"", "integer(read, \"", "text(read, \"", "read.get(\"", "at_key(read, \""]
            .iter()
            .flat_map(|opener| {
                body.match_indices(opener).filter_map(|(at, _)| {
                    let tail = &body[at + opener.len()..];
                    tail.find('"').map(|end| tail[..end].to_string())
                })
            })
            .collect();
        assert!(read.len() > 20, "the scan found {} reads, too few to be this view", read.len());

        let unasked: Vec<&String> = read
            .iter()
            .filter(|key| !requested.contains(&key.as_str()) && !ALWAYS_PRESENT.contains(&key.as_str()))
            .collect();
        assert!(unasked.is_empty(), "these are read from an item and never requested, so they always come back missing: {unasked:?}");
    }

    #[test]
    fn an_item_with_no_position_is_not_one_a_head_can_drag() {
        let unplaced = items_view(&Plan(vec![settings(), json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true, "coordinate": at(0.0, 0.0) })], 1), &[]);
        assert_eq!(unplaced["items"][1]["coordinate"], Value::Null, "nought by nought is the unset coordinate, not a place off Africa");
        assert_eq!(unplaced["items"][1]["movable"], false);

        let placed = items_view(&Plan(vec![settings(), json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true, "coordinate": at(47.397, 8.546) })], 1), &[]);
        assert_eq!(placed["items"][1]["movable"], true);

        assert_eq!(unplaced["items"][0]["kind"], "settings");
        assert_eq!(unplaced["items"][0]["movable"], true, "MissionSettingsItem::specifiesCoordinate() is true unconditionally and the coordinate is the planned home, which QGC lets an operator drag. This assertion used to read false and passed only because the fixture withheld the coordinate that every real settings row carries");

        let nowhere = items_view(&Plan(vec![settings(), json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "commandName": "Set Camera Mode", "specifiesCoordinate": false })], 1), &[]);
        assert_eq!(nowhere["items"][1]["movable"], false, "an item with no place cannot be moved to another one, and a command item is the one that actually has none");
    }

    #[test]
    fn a_complex_item_the_catalogue_does_not_name_is_still_listed_as_complex() {
        let pattern = json!({ "kind": "object", "sequenceNumber": 5, "abbreviation": "FWL", "commandName": "Fixed Wing Landing", "isSimpleItem": false });
        let view = items_view(&Plan(vec![settings(), pattern], 1), &[]);
        assert_eq!(view["items"][1]["kind"], "complex", "an item type the core has no entry for still has to draw as something rather than as a waypoint");
        let corridor = items_view(&Plan(vec![settings(), json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": false, "class": "CorridorScanComplexItem", "commandName": "\u{56de}\u{5eca}\u{30b9}\u{30ad}\u{30e3}\u{30f3}" })], 1), &[]);
        assert_eq!(corridor["items"][1]["kind"], "corridor", "a complex item is identified by what it is, not by what the interface happens to call it here");

        let structure = items_view(&Plan(vec![settings(), json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": false, "class": "StructureScanComplexItem", "commandName": "\u{69cb}\u{9020}\u{30b9}\u{30ad}\u{30e3}\u{30f3}" })], 1), &[]);
        assert_eq!(structure["items"][1]["kind"], "structure");
        assert_eq!(view["items"][1]["name"], "Fixed Wing Landing");
    }
}

#[cfg(test)]
mod reported {
    use super::*;

    struct One(Value);

    impl Backend for One {
        fn get(&self, path: &str) -> String {
            match path {
                "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": 2 }).to_string(),
                "plan.missionController.currentPlanViewVIIndex" => json!({ "kind": "value", "value": 1 }).to_string(),
                _ => String::new(),
            }
        }
        fn get_fields(&self, path: &str, _fields: &str) -> String {
            match path {
                "plan.missionController" => json!({ "kind": "object", "containsItems": true }).to_string(),
                "plan.missionController.visualItems.0" => json!({ "kind": "object", "homePosition": true, "sequenceNumber": 0, "isSimpleItem": false }).to_string(),
                "plan.missionController.visualItems.1" => self.0.to_string(),
                _ => String::new(),
            }
        }
        fn set(&self, _p: &str, _v: &str) -> String { String::new() }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn listed(item: Value) -> Value {
        items_view(&One(item), &[])["items"][1].clone()
    }

    #[test]
    fn an_altitude_is_drawn_in_the_unit_the_operator_chose_by_the_core_and_not_by_a_head() {
        struct Imperial(Value);
        impl Backend for Imperial {
            fn get(&self, path: &str) -> String { One(self.0.clone()).get(path) }
            fn get_fields(&self, path: &str, fields: &str) -> String {
                match path {
                    "units" => json!({ "kind": "object", "appSettingsVerticalDistanceUnitsString": "ft" }).to_string(),
                    _ => One(self.0.clone()).get_fields(path, fields),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, path: &str, args: &str) -> String {
                match path {
                    "units.metersToAppSettingsVerticalDistanceUnits" => json!({ "ok": true, "result": serde_json::from_str::<Vec<f64>>(args).unwrap()[0] * 3.2808399 }).to_string(),
                    _ => String::new(),
                }
            }
            fn watch(&self, _p: &[String]) {}
        }
        let item = json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesAltitude": true, "category": "Basic", "facts": [ { "name": "Altitude", "property": "altitude", "value": 75.0 } ] });
        let metric = listed(item.clone());
        assert_eq!(metric["altitudeText"], "75.0 m");
        assert_eq!(metric["altitudeUnits"], "m", "an editor needs the number and the unit apart, and parsing them back out of the text is the shape this replaces");
        assert_eq!(metric["altitude"], 75.0, "the raw metres travel too, so a head drawing a bar is not parsing its own string back");
        assert_eq!(metric["specifiesAltitude"], true, "an item that specifies an altitude of zero would be indistinguishable from one that specifies none, if this were inferred from the value");
        assert_eq!(metric["category"], "Basic");
        assert_eq!(metric["simple"], true, "whether a command can be changed follows from this, and inferring it from the presence of a command number is inferring a fact from the absence of another");

        let imperial = items_view(&Imperial(item), &[])["items"][1].clone();
        assert_eq!(imperial["altitudeText"], "246 ft", "the feet come from the app's own conversion, not a factor the core keeps its own copy of");

        let cooked = json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesAltitude": true,
            "facts": [ { "name": "Altitude", "property": "altitude", "value": 246.06, "rawValue": 75.0, "units": "ft" } ] });
        let twice = items_view(&Imperial(cooked), &[])["items"][1].clone();
        assert_eq!(twice["altitudeText"], "246 ft", "Fact::value is the COOKED value and altitude declares setRawUnits(\"m\"), which FactMetaData maps to UnitHorizontalDistance with a metres-to-feet translator - so with feet chosen the value is ALREADY feet and converting it again drew 75 m as 807 ft");
        assert_eq!(twice["altitude"], 246.06, "the editable number stays the cooked one an editor bound to that fact would show, so this fixes the text without moving the field a head writes back");
        assert_eq!(twice["altitudeMetres"], 75.0, "the raw quantity, for a head that would rather convert and write metres through the fact's rawValue setter than trust a cooked unit that binds once");
        assert_eq!(twice["altitudeEditUnits"], "ft", "AltitudeFactTextField takes its unitsLabel from fact.units, so the editor's number and unit come from one fact and agree whatever the settings say - altitude follows the HORIZONTAL preference because QGC declares rawUnits m, while altitudeText follows VERTICAL, and pairing the cooked number with the vertical name is what made them disagree");
        assert_eq!(metric["altitudeEditUnits"], Value::Null, "the fake without a units key gets none, which is what compactFactJson emitted before rawValue and units were added to it");
        assert_eq!(imperial["altitudeUnits"], "ft");
        assert_eq!(imperial["altitude"], 75.0);
    }

    #[test]
    fn a_flag_only_one_kind_of_item_carries_is_absent_on_the_others() {
        let survey = listed(json!({ "kind": "object", "sequenceNumber": 3, "isSimpleItem": false, "specifiesCoordinate": true, "minAMSLAltitude": 585.0, "maxAMSLAltitude": 660.0 }));
        assert_eq!(survey["specifiesAltitude"], Value::Null, "specifiesAltitude is a Q_PROPERTY on SimpleMissionItem alone, so on a survey it reads false because it is not there - and a survey does state altitudes, which is what makes false the wrong answer rather than a harmless one");

        let start = listed(json!({ "kind": "object", "sequenceNumber": 0, "homePosition": true, "isSimpleItem": false, "facts": [ { "property": "plannedHomePositionAltitude", "value": 12.0 } ] }));
        assert_eq!(start["specifiesAltitude"], Value::Null);
        assert_eq!(start["altitudeText"], "12.0 m", "the launch row states one, and its altitude is how a head should ask rather than the flag");

        let waypoint = listed(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesAltitude": true, "specifiesCoordinate": true, "facts": [ { "property": "altitude", "value": 50.0 } ] }));
        assert_eq!(waypoint["specifiesAltitude"], true, "where the property exists the answer still travels");

        let command = listed(json!({ "kind": "object", "sequenceNumber": 2, "isSimpleItem": true, "specifiesAltitude": false, "altitudeMode": 1 }));
        assert_eq!(command["specifiesAltitude"], false, "including the false that means no");
        assert_eq!(command["incomplete"], false, "isIncomplete is ComplexMissionItem-only and reads absent here too, but false is the right answer for a simple item, so it is left alone");
    }

    #[test]
    fn an_editable_field_is_labelled_the_way_qgc_labels_it_and_the_camera_is_included() {
        struct Editable;
        impl Backend for Editable {
            fn get(&self, path: &str) -> String {
                match path {
                    "plan.missionController.visualItems.1" => json!({ "kind": "object", "facts": [
                        { "property": "turnAroundDistance", "name": "TurnAroundDistanceMultiRotor", "shortDescription": "Turn around distance", "value": 10.0, "units": "m" },
                    ] }),
                    "plan.missionController.visualItems.1.cameraCalc" => json!({ "kind": "object", "facts": [
                        { "property": "distanceToSurface", "name": "DistanceToSurface", "shortDescription": "Altitude above the surface", "value": 50.0, "units": "m" },
                        { "property": "frontalOverlap", "name": "FrontalOverlap", "value": 70.0 },
                    ] }),
                    "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": 2 }),
                    "plan.missionController.currentPlanViewVIIndex" => json!({ "kind": "value", "value": 1 }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "plan.missionController" => json!({ "kind": "object", "containsItems": true }).to_string(),
                    _ => json!({ "kind": "null" }).to_string(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let fields = items_view(&Editable, &["fields".to_string()])["items"][1]["fields"].clone();
        let names: Vec<&str> = fields.as_array().unwrap().iter().map(|f| f["name"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["turnAroundDistance", "distanceToSurface", "frontalOverlap"], "the camera calc is a child object, so its facts arrive under children rather than facts and an item's own list alone is short by the whole camera group");

        assert_eq!(fields[0]["label"], "Turn around distance", "the metadata carries a sentence for an operator; the Fact's own name is a Q_PROPERTY spelling and is not one");
        assert_eq!(fields[1]["path"], "plan.missionController.visualItems.1.cameraCalc.distanceToSurface", "a camera field is written through the camera, so the path a head writes to has to say so");
        assert_eq!(fields[2]["label"], "FrontalOverlap", "a fact with no short description falls back to its name rather than to nothing");

        let unique: std::collections::BTreeSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), names.len(), "the list is two fact sources chained, so a name appearing on both an item and its camera would give a head two entries it cannot tell apart - no property is shared between TransectStyleComplexItem and CameraCalc today, and this is what notices if one ever is");
    }

    #[test]
    fn the_launch_point_moves_because_qgc_lets_an_operator_move_it() {
        let start = listed(json!({ "kind": "object", "sequenceNumber": 0, "homePosition": true, "isSimpleItem": false,
            "specifiesCoordinate": true, "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.4, "longitude": 8.5 } }));
        assert_eq!(start["movable"], true, "MissionSettingsItem::setCoordinate exists and is commented \"Should only be called if the end user is moving\" - dragging the launch point moves it and the plan recomputes, measured on a handset");
        assert_eq!(start["kind"], "settings", "the kind still travels, so a head that wants to refuse the drag can decide that for itself");
    }

    #[test]
    fn every_altitude_says_which_frame_it_is_measured_from() {
        let simple = |mode: i64, metres: f64| json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true,
            "specifiesAltitude": true, "specifiesCoordinate": true, "altitudeMode": mode,
            "facts": [ { "property": "altitude", "value": metres } ] });

        assert_eq!(listed(simple(1, 75.0))["altitudeFrame"], "launch", "a relative altitude is measured from the launch point");
        let current = json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesAltitude": true, "specifiesCoordinate": true, "altitudeFrame": 2,
            "facts": [ { "property": "altitude", "value": 75.0 } ] });
        assert_eq!(
            (&listed(current.clone())["altitudeFrame"], &listed(current)["altitudeMode"]),
            (&json!("amsl"), &json!(2)),
            "SimpleMissionItem serves altitudeFrame since the upstream merge, and a view asking only for altitudeMode answered null for both fields on every item in a running app while every fixture here still carried the old name"
        );
        assert_eq!(listed(simple(1, 75.0))["altitudeFrameText"], "", "launch-relative is the default and carries no suffix, so a head draws the number alone rather than inventing a word for the ordinary case");
        assert_eq!(frame_word("launch"), Some(""), "an empty word is an answer - this frame is spelled with nothing after the number");
        assert_eq!(frame_word("seabed"), None, "and a token the core has not named is no answer at all: mapping it to the empty word would spell an unrecognised frame as the default one, which is the order these two fields arrived in tonight - the token was served for weeks before anything named it");
        assert_eq!(listed(simple(3, 40.0))["altitudeFrameText"], "AGL", "each head was inventing this word, so the same plan could read AGL on one and above terrain on the other with nothing to notice");
        assert_eq!(listed(simple(2, 541.0))["altitudeFrameText"], "AMSL");
        assert_eq!(listed(simple(2, 541.0))["altitudeFrame"], "amsl");
        assert_eq!(listed(simple(3, 40.0))["altitudeFrame"], "terrain", "calculated-above-terrain is what the operator typed above the ground, whatever it is converted to on upload");
        assert_eq!(listed(simple(4, 40.0))["altitudeFrame"], "terrain");

        let start = listed(json!({ "kind": "object", "sequenceNumber": 0, "homePosition": true, "isSimpleItem": false,
            "facts": [ { "property": "plannedHomePositionAltitude", "value": 491.0 } ] }));
        assert_eq!(start["altitudeFrame"], "amsl", "the launch row carries no altitudeMode at all, and its height is a sea-level one");

        let survey = listed(json!({ "kind": "object", "sequenceNumber": 3, "isSimpleItem": false, "specifiesCoordinate": true,
            "minAMSLAltitude": 520.0, "maxAMSLAltitude": 560.0 }));
        assert_eq!(survey["altitudeFrame"], "amsl", "a pattern's band is sea-level, and it carries no altitudeMode either - so the two rows a head cannot infer are exactly the two that differ from the plan's default");
        assert_eq!(survey["altitudeText"], Value::Null, "and the frame describes whichever height the row does have, since a band and a text never both arrive");

        assert_eq!(survey["altitudeSource"], "band", "a head holds two altitude strings and nothing told it which one this row means, so the one place that knows has to say");
        assert_eq!(listed(simple(1, 75.0))["altitudeSource"], "text");
        assert_eq!(listed(json!({ "kind": "object", "sequenceNumber": 4, "isSimpleItem": true, "specifiesAltitude": false }))["altitudeSource"], Value::Null, "a row that states no height at all names no string, rather than naming an empty one");
        let both = listed(json!({ "kind": "object", "sequenceNumber": 5, "isSimpleItem": false, "specifiesCoordinate": true,
            "minAMSLAltitude": 520.0, "maxAMSLAltitude": 560.0, "facts": [ { "property": "altitude", "value": 75.0 } ] }));
        assert!(!both["altitudeBandText"].is_null());
        assert!(!both["altitudeText"].is_null());
        assert_eq!(both["altitudeSource"], "band", "the two are mutually exclusive in every plan measured so far, which is an observation and not a guarantee - if a row ever carries both, the band is the one that describes a pattern and this says so rather than leaving each head to pick");

        let command = listed(json!({ "kind": "object", "sequenceNumber": 2, "isSimpleItem": true, "specifiesAltitude": false, "altitudeMode": 1 }));
        assert_eq!(command["altitudeFrame"], Value::Null, "a DO_ command still carries an altitudeMode, because that is a SimpleMissionItem property - so the mode alone is not evidence there is a height to frame");
    }

    #[test]
    fn a_pattern_that_spans_heights_says_the_span_rather_than_nothing() {
        let survey = listed(json!({
            "kind": "object", "sequenceNumber": 3, "isSimpleItem": false, "specifiesCoordinate": true,
            "minAMSLAltitude": 585.0, "maxAMSLAltitude": 660.0,
        }));
        assert_eq!(survey["altitudeText"], Value::Null, "a survey has no single altitude fact, so the row it draws has always been blank");
        assert_eq!(survey["altitudeBandText"], "585 m to 660 m", "it has a band instead, which minAMSLAltitude and maxAMSLAltitude have been carrying all along");

        let flat = listed(json!({
            "kind": "object", "sequenceNumber": 3, "isSimpleItem": false, "specifiesCoordinate": true,
            "minAMSLAltitude": 585.0, "maxAMSLAltitude": 585.0,
        }));
        assert_eq!(flat["altitudeBandText"], "585 m", "a pattern over level ground spans nothing, and \"585 m to 585 m\" reads as a mistake");

        let waypoint = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesAltitude": true,
            "specifiesCoordinate": true, "facts": [ { "property": "altitude", "value": 50.0 } ],
        }));
        assert_eq!(waypoint["altitudeBandText"], Value::Null, "a simple item carries no band - the property is declared on ComplexMissionItem - and inventing one from its single altitude would be a second spelling of the same number");

        let start = listed(json!({
            "kind": "object", "sequenceNumber": 0, "homePosition": true, "isSimpleItem": false,
            "minAMSLAltitude": 0.0, "maxAMSLAltitude": 0.0,
            "facts": [ { "property": "plannedHomePositionAltitude", "value": 487.0 } ],
        }));
        assert_eq!(start["altitudeBandText"], Value::Null, "MissionSettingsItem is a ComplexMissionItem, so it carries the band properties and answers its entry altitude for both - a launch point is one height, and a head reaching for the band before altitudeText would draw that pair instead of the real 487 m");
        assert_eq!(start["altitudeText"], "487 m");
    }

    #[test]
    fn only_a_command_that_states_an_altitude_shows_one() {
        let altitude = |value: f64| json!({ "property": "altitude", "value": value });
        let speed = listed(json!({
            "kind": "object", "sequenceNumber": 2, "isSimpleItem": true, "specifiesAltitude": false,
            "specifiesCoordinate": false, "facts": [ altitude(0.0) ],
        }));
        assert_eq!(speed["altitudeText"], Value::Null, "a plan loaded from a file sets the altitude fact from param7 without asking whether the command specifies one, so a DO_ item carries a real zero rather than a NaN and no fallback is involved");
        assert_eq!(speed["altitude"], Value::Null);

        let waypoint = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesAltitude": true,
            "specifiesCoordinate": true, "facts": [ altitude(0.0) ],
        }));
        assert_eq!(waypoint["altitudeText"], "0.0 m", "a waypoint at zero states an altitude of zero, which is a different thing from stating none");

        let home = listed(json!({
            "kind": "object", "sequenceNumber": 0, "homePosition": true, "isSimpleItem": false,
            "facts": [ json!({ "property": "plannedHomePositionAltitude", "value": 12.0 }) ],
        }));
        assert_eq!(home["altitudeText"], "12.0 m", "the launch row is not a SimpleMissionItem, so it carries no specifiesAltitude property at all - gating on the bare flag would have blanked it");

        let survey = listed(json!({
            "kind": "object", "sequenceNumber": 3, "isSimpleItem": false, "specifiesCoordinate": true,
            "facts": [ altitude(80.0) ],
        }));
        assert_eq!(survey["altitudeText"], "80.0 m", "and a complex item carries none either, for the same reason");
    }

    #[test]
    fn leg_figures_are_absent_until_the_controller_has_walked_the_plan() {
        struct Plan(Vec<Value>);
        impl Backend for Plan {
            fn get(&self, path: &str) -> String {
                match path {
                    "plan.missionController.visualItems.count" => json!({ "kind": "value", "value": self.0.len() }).to_string(),
                    "plan.missionController.currentPlanViewVIIndex" => json!({ "kind": "value", "value": 1 }).to_string(),
                    _ => String::new(),
                }
            }
            fn get_fields(&self, path: &str, _f: &str) -> String {
                match path {
                    "plan.missionController" => json!({ "kind": "object", "containsItems": true }).to_string(),
                    "plan.missionController.visualItems" => json!({ "kind": "object", "elements": self.0 }).to_string(),
                    _ => String::new(),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let place = |sequence: i64| json!({ "kind": "object", "sequenceNumber": sequence, "isSimpleItem": true, "specifiesCoordinate": true, "distance": 0.0, "distanceFromStart": 0.0, "azimuth": 0.0, "altDifference": 0.0 });
        let fresh = items_view(&Plan(vec![place(0), place(1), place(2)]), &[]);
        let row = &fresh["items"][1];
        assert_eq!(row["distance"], Value::Null, "the controller fills these in a walk that runs after an insert returns, and until it does every one of them reads zero - which is a real possible distance, so a head cannot tell the difference and caches it");
        assert_eq!(row["distanceText"], Value::Null);
        assert_eq!(row["distanceFromStart"], Value::Null);
        assert_eq!(row["azimuthText"], Value::Null, "a bearing of zero is due north, which is the same trap one field over");
        assert_eq!(row["altitudeChangeText"], Value::Null);
        assert_eq!(row["kind"], "waypoint", "everything the walk does not produce still travels; this withholds four figures, not the item");

        let walked_plan = vec![place(0), json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true, "distance": 120.0, "distanceFromStart": 120.0 }), place(2)];
        let done = items_view(&Plan(walked_plan), &[]);
        assert_eq!(done["items"][1]["distanceText"], "120 m", "one item reporting a distance from the start is the tell that the walk has run, and then every figure it produced is trusted - including the zeros, which are real once something moved");
        assert_eq!(done["items"][2]["distance"], 0.0);
    }

    #[test]
    fn an_item_says_how_many_commands_it_folds_in_behind_itself() {
        let plain = listed(json!({ "kind": "object", "sequenceNumber": 2, "lastSequenceNumber": 2, "isSimpleItem": true, "specifiesAltitude": true, "specifiesCoordinate": true, "facts": [] }));
        assert_eq!(plain["foldedCommands"], 0, "a waypoint that emits one command folds nothing");

        let with_speed = listed(json!({ "kind": "object", "sequenceNumber": 2, "lastSequenceNumber": 3, "isSimpleItem": true, "specifiesAltitude": true, "specifiesCoordinate": true, "specifiedFlightSpeed": 12.0, "facts": [] }));
        assert_eq!(with_speed["foldedCommands"], 1, "a per-item speed emits its own DO_CHANGE_SPEED, so the next row's sequence jumps and both heads showed an unexplained hole in the numbering");

        let survey = listed(json!({ "kind": "object", "sequenceNumber": 4, "lastSequenceNumber": 216, "isSimpleItem": false, "isSurveyItem": true, "facts": [] }));
        assert_eq!(survey["foldedCommands"], 212, "and a pattern occupies the whole span it generates, which is the same phenomenon at a different scale");

        let unknown = listed(json!({ "kind": "object", "sequenceNumber": 2, "isSimpleItem": true, "specifiesAltitude": true, "specifiesCoordinate": true, "facts": [] }));
        assert_eq!(unknown["foldedCommands"], Value::Null, "an item that did not report its last sequence says nothing rather than claiming it folds none");
    }

    #[test]
    fn a_speed_the_plan_sets_is_spelled_in_the_operators_speed_unit() {
        struct Knots(Value);
        impl Backend for Knots {
            fn get(&self, path: &str) -> String { One(self.0.clone()).get(path) }
            fn get_fields(&self, path: &str, fields: &str) -> String {
                match path {
                    "units" => json!({ "kind": "object", "appSettingsSpeedUnitsString": "kn" }).to_string(),
                    _ => One(self.0.clone()).get_fields(path, fields),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, path: &str, args: &str) -> String {
                match path {
                    "units.metersSecondToAppSettingsSpeedUnits" => json!({ "ok": true, "result": serde_json::from_str::<Vec<f64>>(args).unwrap()[0] * 1.94384 }).to_string(),
                    _ => String::new(),
                }
            }
            fn watch(&self, _p: &[String]) {}
        }
        let change = json!({ "kind": "object", "sequenceNumber": 2, "isSimpleItem": true, "specifiesAltitude": false, "specifiedFlightSpeed": 12.0 });
        assert_eq!(listed(change.clone())["speedChangeText"], "12.0 m/s", "seconds have no unit preference but speed does, and a head spelling this itself would rebuild the defect the obstacle label was");
        assert_eq!(listed(change.clone())["speedChange"], 12.0, "the raw metres per second still travel");

        let knots = items_view(&Knots(change), &[])["items"][1].clone();
        assert_eq!(knots["speedChangeText"], "23.3 kn", "and it follows the speed setting, which is separate from both distance settings");

        let ordinary = listed(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesAltitude": true, "specifiesCoordinate": true, "facts": [ { "property": "altitude", "value": 50.0 } ] }));
        assert_eq!(ordinary["speedChangeText"], Value::Null, "an item that sets no speed says nothing");

        let zeroed = listed(json!({ "kind": "object", "sequenceNumber": 2, "isSimpleItem": true, "specifiesAltitude": false, "specifiedFlightSpeed": 0.0 }));
        assert_eq!(zeroed["speedChangeText"], Value::Null, "a commanded speed of zero is not a speed, and spelling it \"0.0 m/s\" reads as an instruction to stop");
        assert_eq!(zeroed["speedChange"], 0.0, "the raw value still travels for anyone who wants to know it is there and zero");
    }

    #[test]
    fn a_command_with_no_altitude_does_not_inherit_the_launch_altitude() {
        let home_altitude = json!({ "property": "plannedHomePositionAltitude", "value": 0.0 });
        let speed_change = listed(json!({
            "kind": "object", "sequenceNumber": 2, "isSimpleItem": true, "specifiesCoordinate": false,
            "facts": [ { "property": "altitude", "value": null }, home_altitude.clone() ],
        }));
        assert_eq!(speed_change["altitudeText"], Value::Null, "QGC sets the altitude fact to NaN when an item specifies no altitude, so falling back to the launch altitude gave every DO_ command the home height as its own");
        assert_eq!(speed_change["altitude"], Value::Null);

        let start = items_view(&One(json!({ "kind": "object", "sequenceNumber": 0, "homePosition": true, "facts": [ home_altitude ] })), &[])["items"][1].clone();
        assert_eq!(start["altitudeText"], "0.0 m", "the fallback is right for the row whose altitude is the launch altitude, which is the row it was written for");
    }

    #[test]
    fn a_leg_is_spelled_by_the_same_code_that_spells_the_strip_above_it() {
        struct Units(Value, f64);
        impl Backend for Units {
            fn get(&self, path: &str) -> String {
                match path {
                    "settings.unitsSettings.horizontalDistanceUnits.rawValue" => json!({ "kind": "value", "value": self.1 }).to_string(),
                    _ => One(self.0.clone()).get(path),
                }
            }
            fn get_fields(&self, path: &str, fields: &str) -> String {
                match (path, self.1) {
                    ("units", 0.0) => json!({ "kind": "object", "appSettingsVerticalDistanceUnitsString": "ft" }).to_string(),
                    _ => One(self.0.clone()).get_fields(path, fields),
                }
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, path: &str, args: &str) -> String {
                match (path, self.1) {
                    ("units.metersToAppSettingsVerticalDistanceUnits", 0.0) => json!({ "ok": true, "result": serde_json::from_str::<Vec<f64>>(args).unwrap()[0] * 3.2808399 }).to_string(),
                    _ => String::new(),
                }
            }
            fn watch(&self, _p: &[String]) {}
        }
        let leg = |metres: f64| json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true, "distance": metres, "distanceFromStart": metres, "azimuth": 47.4, "altDifference": -12.0 });
        let row = |item: Value, raw: f64| items_view(&Units(item, raw), &[])["items"][1].clone();

        let short = row(leg(449.36), 1.0);
        assert_eq!(short["distanceText"], "449 m");
        assert_eq!(short["distanceText"], crate::missionsummary::distance_text(449.36, false), "the strip and the row are one function, so they cannot disagree about a number they both draw");
        assert_eq!(short["azimuthText"], "47\u{b0}");
        assert_eq!(row(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "azimuth": 359.7 }), 1.0)["azimuthText"], "0\u{b0}", "QGC rounds before it wraps, so a bearing a third of a degree short of north reads as north and never as 360");
        assert_eq!(short["altitudeChangeText"], "-12.0 m", "a descent reads as a descent; the bare magnitude leaves the operator to work out the direction from the two altitudes either side");
        assert_eq!(short["altitudeChange"], -12.0, "the signed number still travels, so a head drawing an arrow is not parsing its own string back");

        let far = row(leg(1500.0), 1.0);
        assert_eq!(far["distanceText"], "1.50 km", "the kilometre threshold is the part a head reimplementing this would get wrong");

        let feet = row(leg(449.36), 0.0);
        assert_eq!(feet["distanceText"], "1474 ft");
        assert_eq!(feet["altitudeChangeText"], "-39.4 ft", "an altitude is drawn in the vertical unit, which QGC keeps apart from the horizontal one, and by the app's own conversion rather than a factor the core copies");

        let climb = row(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "altDifference": 12.0 }), 1.0);
        assert_eq!(climb["altitudeChangeText"], "+12.0 m");
        let unknown = row(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true }), 1.0);
        assert_eq!(unknown["azimuthText"], Value::Null, "a bearing the controller has not worked out is absent, not a plausible one");
        assert_eq!(unknown["altitudeChangeText"], Value::Null);

        let command = row(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": false }), 1.0);
        assert_eq!(command["specifiesCoordinate"], false, "a change-speed or camera-trigger command has no place by design, and a list that says \"no position\" about it reports the nature of the command as a defect in the plan");
        assert_eq!(command["altitudeOnly"], false, "which is not the same as a takeoff, whose place is withheld by the display and not by the command");
        assert_eq!(short["specifiesCoordinate"], true);
        assert_eq!(climb["distanceText"], Value::Null, "a leg with no distance has no text, rather than a plausible zero");
    }

    #[test]
    fn waiting_for_terrain_is_not_the_operators_task() {
        let waiting = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true,
            "readyForSaveState": AWAITING_TERRAIN, "readyForSaveMessage": "Waiting for terrain data",
        }));
        assert_eq!(waiting["blocked"], false, "there is nothing for an operator to fix while a terrain server is being waited on, and a banner naming this item invites a click that cannot help");
        assert_eq!(waiting["awaitingTerrain"], true);
        assert_eq!(waiting["blockedReason"], "Waiting for terrain data", "it still says what it is waiting for");

        let missing = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true,
            "readyForSaveState": 2, "readyForSaveMessage": "The survey needs an area",
        }));
        assert_eq!(missing["blocked"], true, "an item missing something the operator has to supply is a task");
        assert_eq!(missing["awaitingTerrain"], false);
    }

    #[test]
    fn a_takeoff_that_has_not_been_placed_carries_no_place() {
        let unplaced = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "isTakeoffItem": true, "specifiesCoordinate": true,
            "coordinate": { "kind": "coordinate", "valid": true, "latitude": 0.0, "longitude": 0.0, "altitude": 0.0 },
        }));
        assert_eq!(unplaced["coordinate"], Value::Null, "zero, zero is a valid coordinate by Qt's rules and is where an unplaced takeoff sits, so a head trusting valid draws it in the Atlantic");

        let placed = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true,
            "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0, "altitude": 50.0 },
        }));
        assert_eq!(placed["coordinate"]["latitude"], 47.0);

        let never = listed(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": false, "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0 } }));
        assert_eq!(never["coordinate"], Value::Null, "an item that does not place itself on the map has no place, whatever its coordinate happens to hold");
    }

    #[test]
    fn the_height_the_operator_set_travels_beside_the_height_above_the_sea() {
        let item = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true, "specifiesAltitude": true,
            "amslEntryAlt": 660.0,
            "altitudeMode": 1,
            "facts": [ { "name": "Altitude", "property": "altitude", "value": 75.0 } ],
        }));
        assert_eq!(item["altitude"], 75.0, "a fact is found by its property, because name is the label an operator reads and has a capital in it");
        assert_eq!(item["altitudeAmsl"], 660.0, "and the one it flies at, which differ by the launch elevation");
        assert_eq!(item["altitudeMode"], 1, "altitudeMode is a plain property on the item and not a fact, so searching the facts for it finds nothing");
    }

    #[test]
    fn the_plans_own_entry_reports_the_height_it_keeps_under_its_own_name() {
        let settings = items_view(&One(json!({ "kind": "object", "sequenceNumber": 1 })), &[])["items"][0].clone();
        assert_eq!(settings["kind"], "settings");

        let launch = listed(json!({
            "kind": "object", "sequenceNumber": 0, "homePosition": true, "isSimpleItem": false,
            "facts": [ { "name": "Altitude", "property": "plannedHomePositionAltitude", "value": 585.0 } ],
        }));
        assert_eq!(launch["altitude"], 585.0, "the launch elevation is a height and the row that shows it goes blank if only the waypoint name is looked for");
        assert_eq!(launch["altitudeText"], "585 m", "every measure the core serves rounds the same way, and a row that kept a tenth here read 585.0 m beside a summary saying 585 m to 660 m");

        assert_eq!(listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesAltitude": true,
            "facts": [ { "name": "Altitude", "property": "altitude", "value": 50.0, "units": "ft" } ],
        }))["altitudeUnits"], "m", "the unit a head draws is the operator's display preference, never the unit the fact declares - Fact::units is CONSTANT and binds once at setRawUnits, so a row spelled from it keeps saying metres after the operator chooses feet");
    }

    #[test]
    fn a_structure_scan_carries_the_loop_it_flies_rather_than_transects_it_has_none_of() {
        struct Shapes;
        impl Backend for Shapes {
            fn get(&self, path: &str) -> String {
                let at = |lat: f64, lon: f64| json!({ "latitude": lat, "longitude": lon });
                match path {
                    "plan.missionController.visualItems.4.surveyAreaPolygon.path" => json!({ "kind": "value", "value": [at(47.0, 8.0), at(47.0, 8.1), at(47.1, 8.1)] }),
                    "plan.missionController.visualItems.4.visualTransectPoints" => json!({ "kind": "value", "value": [at(47.01, 8.01), at(47.01, 8.09)] }),
                    "plan.missionController.visualItems.5.structurePolygon.path" => json!({ "kind": "value", "value": [at(47.2, 8.2), at(47.2, 8.3), at(47.3, 8.3)] }),
                    "plan.missionController.visualItems.5.flightPolygon.path" => json!({ "kind": "value", "value": [at(47.19, 8.19), at(47.19, 8.31), at(47.31, 8.31)] }),
                    "plan.missionController.visualItems.5" => json!({ "kind": "object", "bottomFlightAlt": 12.0, "topFlightAlt": 42.0, "facts": [ { "property": "layers", "value": 3.0 } ] }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }

        let metric = Unit { factor: 1.0, name: "m".to_string() };
        let survey = geometry_of(&Shapes, 4, "survey", &metric);
        assert_eq!(survey["transects"].as_array().unwrap().len(), 2);
        assert_eq!(survey["flightLoop"], Value::Null, "a survey mows and its route is the transect list, so there is no loop to draw and an empty one would be a shape rather than an absence");
        assert_eq!(survey["layers"], Value::Null);

        let structure = geometry_of(&Shapes, 5, "structure", &metric);
        assert_eq!(structure["vertices"].as_array().unwrap().len(), 3, "the structure outline is what the operator drew");
        assert_eq!(structure["transects"].as_array().unwrap().len(), 0, "StructureScanComplexItem is a plain ComplexMissionItem with no visualTransectPoints, so the route was read from a property it does not have and both heads drew a boundary with nothing through it");
        assert_eq!(structure["flightLoop"].as_array().unwrap().len(), 3, "the flown path is flightPolygon, offset outward from the structure by the camera distance");
        assert_eq!(structure["layers"], 3, "and it is flown once per layer, stacked in altitude - a head drawing one loop draws a third of the mission");
        assert_eq!(structure["layerAltitudesMetres"], json!([12.0, 27.0, 42.0]), "the layer count alone still under-draws the mission, and the spacing is not a head's to invent: QGC puts top at bottom plus (layers - 1) camera footprints in both the start-from-top and start-from-bottom branches, so the layers are evenly spaced between the two ends");
        assert_eq!(survey["layerAltitudesMetres"], Value::Null);
        assert_eq!(structure["layerSpanText"], "12.0 m to 42.0 m", "the array alone had no consumer: raw metres force a head to hardcode a unit and a precision, in a plan tab where every other measurement arrives already spelled, so a feet rig read metres beside a camera line reading feet");
        assert_eq!(survey["layerSpanText"], Value::Null);
        let feet = Unit { factor: 3.2808399, name: "ft".to_string() };
        assert_eq!(geometry_of(&Shapes, 5, "structure", &feet)["layerSpanText"], "39 ft to 138 ft", "the heights are held in metres and spelled in the operator's unit, and a metric fake alone cannot tell a conversion from an identity");

        struct OneLayer;
        impl Backend for OneLayer {
            fn get(&self, path: &str) -> String {
                match path {
                    "plan.missionController.visualItems.5.structurePolygon.path" => json!({ "kind": "value", "value": [ { "latitude": 47.2, "longitude": 8.2 } ] }),
                    "plan.missionController.visualItems.5" => json!({ "kind": "object", "bottomFlightAlt": 20.0, "topFlightAlt": 20.0, "facts": [ { "property": "layers", "value": 1.0 } ] }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String { self.get(p) }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        assert_eq!(geometry_of(&OneLayer, 5, "structure", &metric)["layerAltitudesMetres"], json!([20.0]), "a single layer divides by no steps at all, and the default Layers value is 1");
        assert_eq!(survey["flightLoop"], Value::Null, "the absent cases stay absent rather than becoming an empty shape a head would draw");
    }

    #[test]
    fn a_route_stops_at_a_command_number_rather_than_at_a_word() {
        let rtl = listed(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "command": RETURN_TO_LAUNCH, "commandName": "Zur\u{00fc}ck zum Start" }));
        assert_eq!(rtl["endsRoute"], true, "a translated command name is a different string in every locale, and a route drawn past a return to launch is a line across the map");
        assert_eq!(rtl["command"], RETURN_TO_LAUNCH);

        let land = listed(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "isLandCommand": true, "command": 21 }));
        assert_eq!(land["endsRoute"], true);

        let waypoint = listed(json!({ "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "command": 16, "commandName": "Waypoint" }));
        assert_eq!(waypoint["endsRoute"], false);
    }

    #[test]
    fn a_pattern_the_vehicle_leaves_by_another_corner_says_where_it_leaves() {
        let survey = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": false, "isSurveyItem": true,
            "specifiesCoordinate": true, "isIncomplete": false, "exitCoordinateSameAsEntry": false,
            "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0 },
            "exitCoordinate": { "kind": "coordinate", "valid": true, "latitude": 47.1, "longitude": 8.1 },
        }));
        assert_eq!(survey["exitCoordinate"]["latitude"], 47.1, "a route drawn from where the vehicle went in doubles back across the pattern");
        assert_eq!(survey["coordinate"]["latitude"], 47.0);

        let waypoint = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true, "exitCoordinateSameAsEntry": true,
            "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0 },
            "exitCoordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0 },
        }));
        assert_eq!(waypoint["exitCoordinate"], Value::Null, "a waypoint is left where it was entered, and drawing a second point there is a point on top of a point");
    }

    #[test]
    fn a_complex_item_with_no_shape_yet_is_not_a_leg_the_vehicle_flies() {
        let drawn = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": false, "isSurveyItem": true,
            "specifiesCoordinate": true, "isIncomplete": true,
            "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0 },
        }));
        assert_eq!(drawn["incomplete"], true);
        assert_eq!(drawn["flownLeg"], false, "a survey still being drawn has a centre but no route through it, and drawing a leg to it puts a line across the map");

        let finished = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": false, "isSurveyItem": true,
            "specifiesCoordinate": true, "isIncomplete": false,
            "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0 },
        }));
        assert_eq!(finished["flownLeg"], true);

        let standalone = listed(json!({
            "kind": "object", "sequenceNumber": 1, "isSimpleItem": true, "specifiesCoordinate": true, "isStandaloneCoordinate": true,
            "coordinate": { "kind": "coordinate", "valid": true, "latitude": 47.0, "longitude": 8.0 },
        }));
        assert_eq!(standalone["flownLeg"], false, "a region of interest has a place and the vehicle does not fly to it");
    }

    #[test]
    fn an_item_that_did_not_read_is_not_quietly_a_complex_one() {
        let unread = listed(json!({ "kind": "null" }));
        assert_eq!(unread["kind"], "unreadable", "a read that found nothing used to classify as complex, because complex was the only negative test");
    }
    #[test]
    fn the_altitude_mode_numbers_here_are_the_same_enum_as_everywhere_else() {
        assert_eq!(
            [MODE_RELATIVE as i64, MODE_ABSOLUTE as i64, MODE_CALC_ABOVE_TERRAIN as i64],
            [crate::altitudemodes::RELATIVE, crate::altitudemodes::ABSOLUTE, crate::altitudemodes::CALC_ABOVE_TERRAIN],
            "this module keeps a third copy of QGroundControlQmlGlobal::AltMode as floats for writing into facts, and altitudemodes is the copy pinned against the header"
        );
    }


    #[test]
    fn the_speed_section_says_whether_an_item_can_change_speed_and_to_what() {
        struct Section(Option<(bool, bool)>, Value);
        impl Backend for Section {
            fn get(&self, p: &str) -> String {
                match p {
                    "plan.missionController.visualItems.2.speedSection.flightSpeed" => self.1.clone(),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn get_fields(&self, p: &str, _f: &str) -> String {
                match (p, self.0) {
                    ("plan.missionController.visualItems.2.speedSection", Some((available, specified))) => json!({ "kind": "object", "available": available, "specifyFlightSpeed": specified }),
                    _ => json!({ "kind": "null" }),
                }
                .to_string()
            }
            fn set(&self, _p: &str, _v: &str) -> String { String::new() }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let speed = json!({ "kind": "fact", "name": "FlightSpeed", "value": 12.5, "units": "m/s" });
        let section = speed_section(&Section(Some((true, true)), speed.clone()), 2);
        assert_eq!((&section["available"], &section["specified"], &section["value"], &section["units"]), (&json!(true), &json!(true), &json!(12.5), &json!("m/s")));
        assert_eq!(section["path"], "plan.missionController.visualItems.2.speedSection.flightSpeed", "the head writes the value it reads, through the same fact path");
        assert_eq!(section["specifyPath"], "plan.missionController.visualItems.2.speedSection.specifyFlightSpeed");
        assert!(crate::factwrite::owns(section["path"].as_str().unwrap()), "and the core validates that write");
        assert_eq!(speed_section(&Section(Some((false, false)), speed), 2)["available"], false, "a DO_ command carries no speed section it can set");
        assert_eq!(speed_section(&Section(Some((true, false)), json!({ "kind": "null" })), 2)["value"], Value::Null);
        assert_eq!(speed_section(&Section(None, json!({ "kind": "null" })), 2), Value::Null, "an item with no speedSection object has nothing to show");
    }
}
