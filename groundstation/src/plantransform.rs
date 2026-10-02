use serde_json::{Value, json};

use crate::plandoc::{Document, Item, Simple};
use crate::read::object;
use crate::router::Backend;
use crate::surveygrid::{at_distance_and_azimuth, azimuth_to, distance_between};

pub const DEPS: &[&str] = &["plan.missionController.plannedHomePosition", "vehicle.coordinate", "vehicles.activeVehicleAvailable", crate::coreplan::CHANGED];

pub const OFFSET: &str = "plan.missionController.offsetMission";
pub const REPOSITION: &str = "plan.missionController.repositionMission";
pub const ROTATE: &str = "plan.missionController.rotateMission";
const LATITUDE: usize = 4;
const LONGITUDE: usize = 5;
const ALTITUDE: usize = 6;
const FUZZY: f64 = 1e-12;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Transform {
    Offset { east: f64, north: f64, up: f64 },
    Reposition { latitude: f64, longitude: f64 },
    Rotate { degrees_cw: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scope {
    pub takeoff: bool,
    pub landing: bool,
}

fn fuzzy_zero(value: f64) -> bool {
    value.abs() <= FUZZY
}

fn skipped(command: &crate::cmdinfo::Command, scope: Scope) -> bool {
    (!scope.takeoff && command.is_takeoff) || (!scope.landing && command.is_land)
}

fn moved_point(at: (f64, f64), transform: Transform, home: Option<(f64, f64)>) -> Option<(f64, f64)> {
    match transform {
        Transform::Offset { east, north, .. } => {
            let distance = east.hypot(north);
            let azimuth = if fuzzy_zero(distance) { 0.0 } else { east.atan2(north).to_degrees() };
            Some(at_distance_and_azimuth(at, distance, azimuth))
        }
        Transform::Reposition { latitude, longitude } => {
            let old_home = home?;
            Some(at_distance_and_azimuth((latitude, longitude), distance_between(old_home, at), azimuth_to(old_home, at)))
        }
        Transform::Rotate { degrees_cw } => {
            let centre = home?;
            let distance = distance_between(centre, at);
            (!fuzzy_zero(distance)).then(|| at_distance_and_azimuth(centre, distance, azimuth_to(centre, at) + degrees_cw))
        }
    }
}

fn moves_horizontally(transform: Transform) -> bool {
    match transform {
        Transform::Offset { east, north, .. } => !fuzzy_zero(east) || !fuzzy_zero(north),
        Transform::Reposition { .. } => true,
        Transform::Rotate { degrees_cw } => !fuzzy_zero(degrees_cw),
    }
}

fn transformed_simple(simple: &Simple, command: &crate::cmdinfo::Command, transform: Transform, home: Option<(f64, f64)>, scope: Scope) -> Simple {
    if skipped(command, scope) {
        return simple.clone();
    }
    let coordinate = simple.params[LATITUDE].zip(simple.params[LONGITUDE]).filter(|(lat, lon)| lat.is_finite() && lon.is_finite());
    let placed = coordinate
        .filter(|_| command.specifies_coordinate && !command.standalone_coordinate && moves_horizontally(transform))
        .and_then(|at| moved_point(at, transform, home));
    let up = match transform {
        Transform::Offset { up, .. } if !fuzzy_zero(up) && (command.specifies_coordinate || command.specifies_altitude_only) => Some(up),
        _ => None,
    };
    let altitude = simple.altitude.as_ref().map(|held| crate::plandoc::Altitude { altitude: held.altitude + up.unwrap_or(0.0), ..held.clone() });
    let params: [Option<f64>; 7] = std::array::from_fn(|i| match (i, placed, up) {
        (LATITUDE, Some((lat, _)), _) => Some(lat),
        (LONGITUDE, Some((_, lon)), _) => Some(lon),
        (ALTITUDE, _, Some(up)) => simple.params[ALTITUDE].map(|a| a + up),
        _ => simple.params[i],
    });
    Simple { params, altitude: if up.is_some() { altitude } else { simple.altitude.clone() }, ..simple.clone() }
}

fn translated(at: &Value, distance: f64, azimuth: f64) -> Value {
    match at.as_array().and_then(|a| Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))) {
        Some(point) => {
            let (lat, lon) = at_distance_and_azimuth(point, distance, azimuth);
            json!([lat, lon].into_iter().chain(at.as_array().into_iter().flatten().skip(2).filter_map(Value::as_f64)).collect::<Vec<f64>>())
        }
        None => at.clone(),
    }
}

fn with_keys(json: &Value, keys: &[&str], change: impl Fn(&Value) -> Value) -> Value {
    match json {
        Value::Object(fields) => Value::Object(fields.iter().map(|(key, value)| (key.clone(), if keys.contains(&key.as_str()) { change(value) } else { value.clone() })).collect()),
        other => other.clone(),
    }
}

fn moved_complex(kind: &str, json: &Value, transform: Transform, home: Option<(f64, f64)>) -> Option<Value> {
    let landing = crate::landingpattern::is_landing(kind);
    let old = match landing {
        true => crate::landingpattern::coordinate(json, "landCoordinate").map(|at| (at.latitude, at.longitude)),
        false => crate::missionitems::complex_entry(json),
    }?;
    let new = moved_point(old, transform, home)?;
    let (distance, azimuth) = (distance_between(old, new), azimuth_to(old, new));
    let moved = match landing {
        true => with_keys(json, &["landCoordinate", "loiterCoordinate", "landingApproachCoordinate"], |at| translated(at, distance, azimuth)),
        false => with_keys(json, &["polygon", "polyline"], |shape| Value::Array(shape.as_array().into_iter().flatten().map(|at| translated(at, distance, azimuth)).collect())),
    };
    Some(if landing { moved } else { crate::surveydoc::regenerate_item(&moved) })
}

pub fn move_complex_to(item: &Item, latitude: f64, longitude: f64) -> Option<Item> {
    let Item::Complex { kind, json, item_count } = item else { return None };
    let old = match crate::landingpattern::is_landing(kind) {
        true => crate::landingpattern::coordinate(json, "landCoordinate").map(|at| (at.latitude, at.longitude)),
        false => crate::missionitems::complex_entry(json),
    }?;
    let placed = moved_complex(kind, json, Transform::Reposition { latitude, longitude }, Some(old))?;
    Some(Item::Complex { kind: kind.clone(), item_count: crate::plandoc::complex_count(kind, &placed).unwrap_or(*item_count), json: placed })
}

fn complex_altitude(kind: &str, json: &Value) -> Option<f64> {
    match kind {
        k if crate::landingpattern::is_landing(k) => crate::landingpattern::approach(json).map(|at| at.altitude),
        "StructureScan" => json.get("EntranceAltitude")?.as_f64(),
        _ => json.get("TransectStyleComplexItem")?.get("CameraCalc")?.get("DistanceToSurface")?.as_f64(),
    }
}

fn transformed_complex(item: &Item, transform: Transform, home: Option<(f64, f64)>, scope: Scope) -> Item {
    let Item::Complex { kind, json, item_count } = item else { return item.clone() };
    if !scope.landing && crate::landingpattern::is_landing(kind) {
        return item.clone();
    }
    let placed = moves_horizontally(transform).then(|| moved_complex(kind, json, transform, home)).flatten().unwrap_or_else(|| json.clone());
    let moved = Item::Complex { kind: kind.clone(), item_count: crate::plandoc::complex_count(kind, &placed).unwrap_or(*item_count), json: placed };
    match transform {
        Transform::Offset { up, .. } if !fuzzy_zero(up) => match moved {
            Item::Complex { ref kind, ref json, .. } => complex_altitude(kind, json).map_or(moved.clone(), |altitude| crate::coreplan::with_new_altitude(&moved, altitude + up)),
            other => other,
        },
        _ => moved,
    }
}

pub fn transform(doc: &Document, transform: Transform, scope: Scope) -> Result<Document, String> {
    let home = doc.home.map(|[lat, lon, _]| (lat, lon));
    match transform {
        Transform::Reposition { latitude, longitude } if !(latitude.abs() <= 90.0 && longitude.abs() <= 180.0) => return Err("Cannot reposition mission to an invalid coordinate".to_string()),
        Transform::Reposition { .. } if home.is_none() => return Err("Home position must be set to reposition the mission.".to_string()),
        Transform::Rotate { .. } if home.is_none() => return Err("Home position must be set to rotate the mission.".to_string()),
        _ => {}
    }
    let commands = crate::cmdinfo::tree(crate::plandoc::firmware(doc.firmware_type), crate::plandoc::vehicle_class(doc.vehicle_type));
    let items = doc
        .items
        .iter()
        .map(|item| match item {
            Item::Simple(simple) => commands.get(&simple.command).map_or_else(|| item.clone(), |command| Item::Simple(transformed_simple(simple, command, transform, home, scope))),
            Item::Complex { .. } => transformed_complex(item, transform, home, scope),
        })
        .collect();
    let moved_home = doc.home.map(|[lat, lon, alt]| {
        let at = (moves_horizontally(transform).then(|| moved_point((lat, lon), transform, home)).flatten()).unwrap_or((lat, lon));
        [at.0, at.1, alt]
    });
    Ok(Document { items, home: moved_home, ..doc.clone() })
}

fn args_of(args: &str) -> Value {
    serde_json::from_str(args).unwrap_or(Value::Null)
}

fn number(args: &Value, index: usize, default: f64) -> Option<f64> {
    match args.get(index) {
        None | Some(Value::Null) => Some(default),
        Some(value) => value.as_f64().filter(|v| v.is_finite()),
    }
}

fn boolean(args: &Value, index: usize, default: bool) -> bool {
    args.get(index).and_then(Value::as_bool).unwrap_or(default)
}

pub fn parse(path: &str, args: &str) -> Result<(Transform, Scope), String> {
    let given = args_of(args);
    match path {
        OFFSET => {
            let (Some(east), Some(north), Some(up)) = (number(&given, 0, f64::NAN).filter(|v| !v.is_nan()), number(&given, 1, f64::NAN).filter(|v| !v.is_nan()), number(&given, 2, 0.0)) else {
                return Err("offsetMission takes east and north metres, then optional up metres.".to_string());
            };
            Ok((Transform::Offset { east, north, up }, Scope { takeoff: boolean(&given, 3, false), landing: boolean(&given, 4, false) }))
        }
        REPOSITION => {
            let at = given.get(0).and_then(|at| Some((at.get("latitude")?.as_f64()?, at.get("longitude")?.as_f64()?)));
            let Some((latitude, longitude)) = at else { return Err("repositionMission takes the new home's latitude and longitude.".to_string()) };
            Ok((Transform::Reposition { latitude, longitude }, Scope { takeoff: boolean(&given, 1, true), landing: boolean(&given, 2, true) }))
        }
        _ => {
            let Some(degrees_cw) = number(&given, 0, f64::NAN).filter(|v| !v.is_nan()) else {
                return Err("rotateMission takes the clockwise degrees.".to_string());
            };
            Ok((Transform::Rotate { degrees_cw }, Scope { takeoff: boolean(&given, 1, false), landing: boolean(&given, 2, false) }))
        }
    }
}

pub fn owns(path: &str) -> bool {
    [OFFSET, REPOSITION, ROTATE].contains(&path)
}

pub fn run(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let (operation, scope) = match parse(path, args) {
        Ok(parsed) => parsed,
        Err(reason) => return json!({ "ok": false, "reason": reason }),
    };
    if crate::coreplan::enabled() {
        return crate::coreplan::apply(|doc| transform(doc, operation, scope));
    }
    let home = home_of(backend);
    let refusal = match operation {
        Transform::Reposition { .. } if home.is_none() => Some("Home position must be set to reposition the mission."),
        Transform::Rotate { .. } if home.is_none() => Some("Home position must be set to rotate the mission."),
        _ => None,
    };
    match refusal {
        Some(reason) => json!({ "ok": false, "reason": reason }),
        None => object(&backend.invoke(path, args)),
    }
}

fn point(value: Option<&Value>) -> Option<(f64, f64)> {
    let value = value?;
    let (lat, lon) = (value.get("latitude")?.as_f64()?, value.get("longitude")?.as_f64()?);
    (value.get("isValid").and_then(Value::as_bool) != Some(false) && lat.is_finite() && lon.is_finite()).then_some((lat, lon))
}

fn home_of(backend: &dyn Backend) -> Option<(f64, f64)> {
    match crate::coreplan::enabled() {
        true => crate::coreplan::current_document().home.map(|[lat, lon, _]| (lat, lon)),
        false => point(object(&backend.get_fields("plan.missionController", "plannedHomePosition")).get("plannedHomePosition")),
    }
}

pub fn transform_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let connected = crate::read::flag(&object(&backend.get_fields("vehicles", "activeVehicleAvailable")), "activeVehicleAvailable");
    let vehicle = connected.then(|| point(object(&backend.get_fields("vehicle", "coordinate")).get("coordinate"))).flatten();
    let home = home_of(backend);
    json!({
        "kind": "object",
        "class": "PlanTransform",
        "hasHome": home.is_some(),
        "home": home.map(|(latitude, longitude)| json!({ "latitude": latitude, "longitude": longitude })),
        "vehicle": vehicle.map(|(latitude, longitude)| json!({ "latitude": latitude, "longitude": longitude })),
        "coordinateSystems": if connected { vec!["Geographic", "Universal Transverse Mercator", "Military Grid Reference", "Vehicle Position"] } else { vec!["Geographic", "Universal Transverse Mercator", "Military Grid Reference"] },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plandoc::Altitude;

    fn simple(command: i64, at: (f64, f64), altitude: f64) -> Item {
        Item::Simple(Simple { command, frame: 3, params: [Some(0.0), Some(0.0), Some(0.0), None, Some(at.0), Some(at.1), Some(altitude)], auto_continue: true, altitude: Some(Altitude { mode: 1, altitude, amsl_above_terrain: None }), sections: vec![] })
    }

    fn doc() -> Document {
        let mut doc = crate::plandoc::load(r#"{"fileType":"Plan","version":1,"groundStation":"QGroundControl","mission":{"version":2,"firmwareType":3,"vehicleType":2,"cruiseSpeed":15,"hoverSpeed":5,"plannedHomePosition":[47.0,8.0,500],"items":[]},"geoFence":{"version":2,"polygons":[],"circles":[]},"rallyPoints":{"version":2,"points":[]}}"#, 2).unwrap();
        doc.items = vec![simple(22, (47.0, 8.0), 30.0), simple(16, (47.001, 8.0), 50.0), simple(21, (47.0, 8.001), 0.0)];
        doc
    }

    fn at(doc: &Document, index: usize) -> (f64, f64, f64) {
        match &doc.items[index] {
            Item::Simple(s) => (s.params[LATITUDE].unwrap(), s.params[LONGITUDE].unwrap(), s.altitude.as_ref().unwrap().altitude),
            Item::Complex { .. } => panic!(),
        }
    }

    #[test]
    fn a_survey_moved_to_a_point_puts_its_entry_there_as_set_coordinate_does() {
        let plan: Value = serde_json::from_str(include_str!("../tests/fixtures/survey-upload.plan")).unwrap();
        let survey = Item::Complex { kind: "survey".into(), json: plan["mission"]["items"][0].clone(), item_count: 1 };
        let Some(moved @ Item::Complex { .. }) = move_complex_to(&survey, -35.36, 149.17) else { panic!("a placed survey moves") };
        let Item::Complex { json, .. } = moved else { unreachable!() };
        let entry = crate::missionitems::complex_entry(&json).unwrap();
        assert!(distance_between(entry, (-35.36, 149.17)) < 1.0, "TransectStyleComplexItem::setCoordinate translates the polygon so the entry lands on the point: {entry:?}");
    }

    #[test]
    fn an_offset_moves_waypoints_and_home_but_not_takeoff_or_landing_unless_asked() {
        let moved = transform(&doc(), Transform::Offset { east: 0.0, north: 100.0, up: 10.0 }, Scope { takeoff: false, landing: false }).unwrap();
        assert_eq!(at(&moved, 0), at(&doc(), 0), "the takeoff stays");
        assert_eq!(at(&moved, 2), at(&doc(), 2), "the landing stays");
        let (lat, lon, alt) = at(&moved, 1);
        assert!((distance_between((47.001, 8.0), (lat, lon)) - 100.0).abs() < 0.01 && lat > 47.001 && (lon - 8.0).abs() < 1e-9);
        assert_eq!(alt, 60.0);
        assert!(moved.home.unwrap()[0] > 47.0, "the settings item specifies a coordinate, so offsetMission moves home too");
        assert_eq!(moved.home.unwrap()[2], 500.0, "Home altitude is not modified");
        let all = transform(&doc(), Transform::Offset { east: 0.0, north: 100.0, up: 0.0 }, Scope { takeoff: true, landing: true }).unwrap();
        assert_ne!(at(&all, 2), at(&doc(), 2), "asked to, the landing moves");
        assert_eq!(at(&all, 0), at(&doc(), 0), "ArduCopter's takeoff specifies no coordinate in its command info, so even asked to it has none to move");
    }

    #[test]
    fn a_rotation_turns_items_about_home_and_a_reposition_carries_them_with_it() {
        let turned = transform(&doc(), Transform::Rotate { degrees_cw: 90.0 }, Scope { takeoff: false, landing: false }).unwrap();
        let (lat, lon, _) = at(&turned, 1);
        assert!((azimuth_to((47.0, 8.0), (lat, lon)) - 90.0).abs() < 1e-6, "a point due north of home ends due east");
        assert_eq!(turned.home, doc().home);
        let carried = transform(&doc(), Transform::Reposition { latitude: 48.0, longitude: 9.0 }, Scope { takeoff: true, landing: true }).unwrap();
        let home = carried.home.unwrap();
        assert!((home[0] - 48.0).abs() < 1e-9 && (home[1] - 9.0).abs() < 1e-9);
        let (lat, lon, _) = at(&carried, 1);
        assert!((distance_between((48.0, 9.0), (lat, lon)) - distance_between((47.0, 8.0), (47.001, 8.0))).abs() < 0.01);
        let homeless = Document { home: None, ..doc() };
        assert!(transform(&homeless, Transform::Rotate { degrees_cw: 10.0 }, Scope { takeoff: false, landing: false }).is_err());
    }

    #[test]
    fn complex_items_move_with_their_entry_and_landings_only_when_asked() {
        let corridor: Value = serde_json::from_str(include_str!("../tests/fixtures/corridor-inserted-by-qt.json")).unwrap();
        let landing: Value = serde_json::from_str(include_str!("../tests/fixtures/fwland-pattern.json")).unwrap();
        let complex = |json: &Value| Item::Complex { kind: json["complexItemType"].as_str().unwrap().to_string(), json: json.clone(), item_count: crate::plandoc::complex_count(json["complexItemType"].as_str().unwrap(), json).unwrap_or(1) };
        let base = Document { items: vec![complex(&corridor["corridor"]), complex(&landing)], ..doc() };
        let north = Transform::Offset { east: 0.0, north: 100.0, up: 10.0 };
        let moved = transform(&base, north, Scope { takeoff: false, landing: false }).unwrap();
        let json_of = |item: &Item| match item { Item::Complex { json, .. } => json.clone(), Item::Simple(_) => panic!() };
        let vertex = |json: &Value| (json["polyline"][0][0].as_f64().unwrap(), json["polyline"][0][1].as_f64().unwrap());
        let (before, after) = (vertex(&corridor["corridor"]), vertex(&json_of(&moved.items[0])));
        assert!((distance_between(before, after) - 100.0).abs() < 0.5 && azimuth_to(before, after).abs() < 1.0, "the corridor slid 100 m north");
        assert_eq!(json_of(&moved.items[0])["TransectStyleComplexItem"]["CameraCalc"]["DistanceToSurface"].as_f64(), corridor["corridor"]["TransectStyleComplexItem"]["CameraCalc"]["DistanceToSurface"].as_f64().map(|a| a + 10.0));
        assert_eq!(json_of(&moved.items[1]), landing, "a landing stays unless landing items are included");
        let with_landing = transform(&base, north, Scope { takeoff: false, landing: true }).unwrap();
        let land = |json: &Value| (json["landCoordinate"][0].as_f64().unwrap(), json["landCoordinate"][1].as_f64().unwrap());
        assert!((distance_between(land(&landing), land(&json_of(&with_landing.items[1]))) - 100.0).abs() < 0.5, "a landing moves by its touchdown point");
    }

    #[test]
    fn arguments_take_qgcs_defaults() {
        assert_eq!(parse(OFFSET, "[1, 2]").unwrap(), (Transform::Offset { east: 1.0, north: 2.0, up: 0.0 }, Scope { takeoff: false, landing: false }));
        assert_eq!(parse(REPOSITION, r#"[{"latitude":1,"longitude":2}]"#).unwrap().1, Scope { takeoff: true, landing: true });
        assert!(parse(ROTATE, "[]").is_err());
    }
}
