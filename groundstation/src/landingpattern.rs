use serde_json::{Value, json};

use crate::surveyitems::{CMD_DO_SET_CAM_TRIGG_DIST, CMD_NAV_WAYPOINT, FRAME_GLOBAL, FRAME_GLOBAL_RELATIVE_ALT, FRAME_MISSION, Item};

pub const FIXED_WING_PATTERN: &str = "fwLandingPattern";
pub const VTOL_PATTERN: &str = "vtolLandingPattern";

const CMD_DO_LAND_START: u16 = 189;
const CMD_DO_CHANGE_SPEED: u16 = 178;
const CMD_IMAGE_STOP_CAPTURE: u16 = 2001;
const CMD_VIDEO_STOP_CAPTURE: u16 = 2501;
const CMD_NAV_LOITER_TO_ALT: u16 = 31;
const CMD_NAV_LAND: u16 = 21;
const CMD_NAV_VTOL_LAND: u16 = 85;
const SPEED_TYPE_AIRSPEED: f64 = 0.0;

pub fn land_start_has_coordinate(firmware_type: i64, vehicle_type: i64) -> bool {
    let firmware = crate::plandoc::firmware(firmware_type);
    firmware != crate::cmdinfo::Firmware::ArduPilot && crate::cmdinfo::tree(firmware, crate::plandoc::vehicle_class(vehicle_type)).get(&i64::from(CMD_DO_LAND_START)).is_some_and(|c| c.specifies_coordinate)
}

pub fn notes(vtol: bool) -> Vec<&'static str> {
    match vtol {
        true => vec!["* Actual flight path will vary.", "* Avoid tailwind on approach to land."],
        false => vec!["* Approximate glide slope altitudes.", "* Actual flight path will vary.", "* Avoid tailwind on landing."],
    }
}

pub fn is_landing(kind: &str) -> bool {
    kind == FIXED_WING_PATTERN || kind == VTOL_PATTERN
}

pub struct Point3 {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
}

pub fn coordinate(pattern: &Value, key: &str) -> Option<Point3> {
    let at = pattern.get(key)?.as_array()?;
    Some(Point3 { latitude: at.first()?.as_f64()?, longitude: at.get(1)?.as_f64()?, altitude: at.get(2).and_then(Value::as_f64).unwrap_or(0.0) })
}

pub fn approach(pattern: &Value) -> Option<Point3> {
    coordinate(pattern, "landingApproachCoordinate").or_else(|| coordinate(pattern, "loiterCoordinate"))
}

fn flag(pattern: &Value, key: &str) -> bool {
    pattern.get(key).and_then(Value::as_bool).unwrap_or(false)
}

const FIXED_WING_META: &str = include_str!("../../src/MissionManager/FWLandingPattern.FactMetaData.json");
const VTOL_META: &str = include_str!("../../src/MissionManager/VTOLLandingPattern.FactMetaData.json");
pub const WIZARD: &str = "wizardMode";

pub struct Fresh {
    pub vtol: bool,
    pub land: (f64, f64),
    pub ardupilot: bool,
    pub relative: bool,
    pub transition_distance: Option<f64>,
}

fn fact(fresh: &Fresh, name: &str) -> Value {
    let file = if fresh.vtol { VTOL_META } else { FIXED_WING_META };
    let Some(meta) = crate::factmeta::from_file(file).ok().and_then(|mut all| all.remove(name)) else { return Value::Null };
    meta.default.as_ref().map(|d| crate::settingsstore::typed(&meta.value_type, d).unwrap_or_else(|| d.clone())).unwrap_or(Value::Null)
}

pub fn fresh(fresh: &Fresh) -> Value {
    let number = |name: &str| fact(fresh, name).as_f64().unwrap_or(0.0);
    let on = |name: &str| fact(fresh, name).as_bool().unwrap_or(false);
    let (approach_altitude, land_altitude) = (number("FinalApproachAltitude"), number("LandingAltitude"));
    let distance = match fresh.vtol || on("ValueSetIsDistance") {
        true if fresh.vtol => fresh.transition_distance.unwrap_or_else(|| number("LandingDistance")),
        true => number("LandingDistance"),
        false => (approach_altitude - land_altitude) / number("GlideSlope").to_radians().tan(),
    };
    let heading = number("LandingHeading");
    let clockwise = on("LoiterClockwise");
    let slope = crate::surveygrid::at_distance_and_azimuth(fresh.land, distance, heading + 180.0);
    let approach = match on("UseLoiterToAlt") {
        true => crate::surveygrid::at_distance_and_azimuth(slope, number("LoiterRadius"), heading - 180.0 + if clockwise { -90.0 } else { 90.0 }),
        false => slope,
    };
    let cameras = !fresh.ardupilot;
    let mut pattern = json!({
        "altitudesAreRelative": fresh.relative,
        "complexItemType": if fresh.vtol { VTOL_PATTERN } else { FIXED_WING_PATTERN },
        "finalApproachSpeed": fact(fresh, "FinalApproachSpeed"),
        "landCoordinate": [fresh.land.0, fresh.land.1, land_altitude],
        "landingApproachCoordinate": [approach.0, approach.1, approach_altitude],
        "loiterClockwise": clockwise,
        "loiterRadius": fact(fresh, "LoiterRadius"),
        "stopTakingPhotos": cameras && on("StopTakingPhotos"),
        "stopVideoPhotos": cameras && on("StopTakingVideo"),
        "type": "ComplexItem",
        "useDoChangeSpeed": on("UseDoChangeSpeed"),
        "useLoiterToAlt": on("UseLoiterToAlt"),
        "version": if fresh.vtol { 1 } else { 2 },
        WIZARD: !fresh.vtol,
    });
    if !fresh.vtol {
        pattern["valueSetIsDistance"] = fact(fresh, "ValueSetIsDistance");
    }
    pattern
}

pub fn moved(pattern: &Value, member: &str, value: &Value) -> Option<Value> {
    let key = match member {
        "landingCoordinate" => "landCoordinate",
        "finalApproachCoordinate" => "landingApproachCoordinate",
        WIZARD => {
            let mut changed = pattern.clone();
            changed[WIZARD] = json!(value.as_bool()?);
            return Some(changed);
        }
        _ => return None,
    };
    let (latitude, longitude) = crate::fenceedit::point(Some(value))?;
    let altitude = pattern.get(key).and_then(|at| at.get(2)).cloned().unwrap_or(json!(0.0));
    let mut changed = pattern.clone();
    changed[key] = json!([latitude, longitude, altitude]);
    if key == "landingApproachCoordinate" {
        changed.as_object_mut()?.remove("loiterCoordinate");
    }
    Some(changed)
}

struct Geometry {
    land: (f64, f64),
    approach: (f64, f64),
    heading: f64,
    distance: f64,
}

fn geometry(pattern: &Value) -> Option<Geometry> {
    let land = coordinate(pattern, "landCoordinate")?;
    let approach = approach(pattern)?;
    let slope = slope_start(pattern)?;
    let land = (land.latitude, land.longitude);
    Some(Geometry { land, approach: (approach.latitude, approach.longitude), heading: crate::surveygrid::azimuth_to(slope, land), distance: crate::surveygrid::distance_between(land, slope) })
}

fn is_vtol(pattern: &Value) -> bool {
    pattern.get("complexItemType").and_then(Value::as_str) == Some(VTOL_PATTERN)
}

fn altitudes(pattern: &Value) -> (f64, f64) {
    (approach(pattern).map_or(0.0, |a| a.altitude), coordinate(pattern, "landCoordinate").map_or(0.0, |l| l.altitude))
}

fn glide_slope(pattern: &Value, distance: f64) -> f64 {
    let (high, low) = altitudes(pattern);
    ((high - low) / distance).atan().to_degrees()
}

const FIELDS: [(&str, &str); 13] = [
    ("FinalApproachAltitude", "finalApproachAltitude"),
    ("UseDoChangeSpeed", "useDoChangeSpeed"),
    ("FinalApproachSpeed", "finalApproachSpeed"),
    ("LoiterRadius", "loiterRadius"),
    ("LandingAltitude", "landingAltitude"),
    ("LandingHeading", "landingHeading"),
    ("LandingDistance", "landingDistance"),
    ("LoiterClockwise", "loiterClockwise"),
    ("UseLoiterToAlt", "useLoiterToAlt"),
    ("StopTakingPhotos", "stopTakingPhotos"),
    ("StopTakingVideo", "stopTakingVideo"),
    ("ValueSetIsDistance", "valueSetIsDistance"),
    ("GlideSlope", "glideSlope"),
];

fn field_values(pattern: &Value) -> Vec<(&'static str, &'static str, Value)> {
    let Some(g) = geometry(pattern) else { return Vec::new() };
    let (high, low) = altitudes(pattern);
    let value = |name: &str| match name {
        "FinalApproachAltitude" => Some(json!(high)),
        "UseDoChangeSpeed" => pattern.get("useDoChangeSpeed").cloned(),
        "FinalApproachSpeed" => pattern.get("finalApproachSpeed").cloned(),
        "LoiterRadius" => pattern.get("loiterRadius").cloned(),
        "LandingAltitude" => Some(json!(low)),
        "LandingHeading" => Some(json!(g.heading)),
        "LandingDistance" => Some(json!(g.distance)),
        "LoiterClockwise" => pattern.get("loiterClockwise").cloned(),
        "UseLoiterToAlt" => pattern.get("useLoiterToAlt").cloned(),
        "StopTakingPhotos" => pattern.get("stopTakingPhotos").cloned(),
        "StopTakingVideo" => pattern.get("stopVideoPhotos").cloned(),
        "ValueSetIsDistance" if !is_vtol(pattern) => pattern.get("valueSetIsDistance").cloned(),
        "GlideSlope" if !is_vtol(pattern) => Some(json!(glide_slope(pattern, g.distance))),
        _ => None,
    };
    FIELDS.iter().filter_map(|(name, suffix)| value(name).map(|v| (*name, *suffix, v))).collect()
}

pub fn fields(pattern: &Value, item: &str, units: &crate::surveydoc::Units) -> Vec<Value> {
    let file = if is_vtol(pattern) { VTOL_META } else { FIXED_WING_META };
    let by_distance = pattern.get("valueSetIsDistance").and_then(Value::as_bool).unwrap_or(true);
    field_values(pattern)
        .into_iter()
        .filter_map(|(name, suffix, value)| crate::surveydoc::fact_control(file, name, value, item, suffix, units).map(|control| (suffix, control)))
        .map(|(suffix, control)| match (suffix, control) {
            ("landingDistance", Value::Object(map)) if !is_vtol(pattern) => Value::Object(map.into_iter().chain([("enabled".to_string(), json!(by_distance))]).collect()),
            ("glideSlope", Value::Object(map)) => Value::Object(map.into_iter().chain([("enabled".to_string(), json!(!by_distance))]).collect()),
            (_, control) => control,
        })
        .collect()
}

pub fn raw(pattern: &Value, suffix: &str, value: &Value, units: &crate::surveydoc::Units) -> Value {
    let file = if is_vtol(pattern) { VTOL_META } else { FIXED_WING_META };
    let unit = FIELDS.iter().find(|(_, s)| *s == suffix).and_then(|(name, _)| crate::factmeta::from_file(file).ok()?.remove(*name)).and_then(|m| crate::surveydoc::cooked_unit(m.units.as_deref().unwrap_or(""), units));
    match (unit, value.as_f64()) {
        (Some(u), Some(v)) => json!(u.meters(v)),
        _ => value.clone(),
    }
}

fn laid_out(pattern: &Value, heading: f64, distance: f64) -> Option<Value> {
    let g = geometry(pattern)?;
    let slope = crate::surveygrid::at_distance_and_azimuth(g.land, distance, heading + 180.0);
    let radius = pattern.get("loiterRadius").and_then(Value::as_f64).unwrap_or(0.0);
    let approach = match flag(pattern, "useLoiterToAlt") {
        true => crate::surveygrid::at_distance_and_azimuth(slope, radius, heading - 180.0 + if flag(pattern, "loiterClockwise") { -90.0 } else { 90.0 }),
        false => slope,
    };
    let (high, _) = altitudes(pattern);
    let mut changed = pattern.clone();
    changed["landingApproachCoordinate"] = json!([approach.0, approach.1, high]);
    Some(changed)
}

fn circled(before: &Value, after: &Value) -> Option<Value> {
    let g = geometry(before)?;
    let radius = after.get("loiterRadius").and_then(Value::as_f64).unwrap_or(0.0);
    if crate::surveygrid::distance_between(g.land, g.approach) < radius {
        return Some(after.clone());
    }
    let reach = (radius.powi(2) + g.distance.powi(2)).sqrt();
    let turn = (radius / reach).asin().to_degrees() * if flag(after, "loiterClockwise") { -1.0 } else { 1.0 };
    let approach = crate::surveygrid::at_distance_and_azimuth(g.land, reach, g.heading + 180.0 + turn);
    let (high, _) = altitudes(after);
    let mut changed = after.clone();
    changed["landingApproachCoordinate"] = json!([approach.0, approach.1, high]);
    Some(changed)
}

pub fn edit(pattern: &Value, suffix: &str, value: &Value) -> Option<Value> {
    let g = geometry(pattern)?;
    let with = |key: &str, v: Value| {
        let mut changed = pattern.clone();
        changed[key] = v;
        changed
    };
    let number = value.as_f64();
    let on = value.as_bool().or_else(|| number.map(|n| n != 0.0));
    match suffix {
        "finalApproachAltitude" => {
            let mut changed = pattern.clone();
            changed["landingApproachCoordinate"][2] = json!(number?);
            changed.as_object_mut()?.remove("loiterCoordinate");
            Some(changed)
        }
        "landingAltitude" => {
            let mut changed = pattern.clone();
            changed["landCoordinate"][2] = json!(number?);
            Some(changed)
        }
        "finalApproachSpeed" => Some(with("finalApproachSpeed", json!(number?))),
        "useDoChangeSpeed" => Some(with("useDoChangeSpeed", json!(on?))),
        "altitudesAreRelative" => Some(with("altitudesAreRelative", json!(on?))),
        "stopTakingPhotos" => Some(with("stopTakingPhotos", json!(on?))),
        "stopTakingVideo" => Some(with("stopVideoPhotos", json!(on?))),
        "valueSetIsDistance" if !is_vtol(pattern) => Some(with("valueSetIsDistance", json!(on?))),
        "landingHeading" => laid_out(pattern, number?, g.distance),
        "landingDistance" => laid_out(pattern, g.heading, number?),
        "glideSlope" if !is_vtol(pattern) => {
            let (high, low) = altitudes(pattern);
            laid_out(pattern, g.heading, (high - low) / number?.to_radians().tan())
        }
        "loiterRadius" => circled(pattern, &with("loiterRadius", json!(number?))),
        "loiterClockwise" => circled(pattern, &with("loiterClockwise", json!(on?))),
        "useLoiterToAlt" => {
            let switched = with("useLoiterToAlt", json!(on?));
            laid_out(&switched, g.heading, g.distance)
        }
        _ => None,
    }
}

fn p(item: &crate::plandoc::Simple, n: usize) -> f64 {
    item.params[n - 1].unwrap_or(f64::NAN)
}

fn zeros(item: &crate::plandoc::Simple, range: std::ops::RangeInclusive<usize>) -> bool {
    range.into_iter().all(|n| p(item, n) == 0.0)
}

fn valid_land(item: &crate::plandoc::Simple, vtol: bool) -> bool {
    let placed = item.frame == i64::from(FRAME_GLOBAL_RELATIVE_ALT) || item.frame == i64::from(FRAME_GLOBAL);
    match vtol {
        true => item.command == i64::from(CMD_NAV_VTOL_LAND) && placed && zeros(item, 1..=3) && p(item, 4).is_nan(),
        false => item.command == i64::from(CMD_NAV_LAND) && placed && zeros(item, 1..=3) && (p(item, 4) == 0.0 || p(item, 4) == 1.0),
    }
}

fn scan_at(items: &[crate::plandoc::Item], start: usize, vtol: bool, ardupilot: bool) -> Option<(usize, Value)> {
    let simple = |at: usize| match items.get(at) {
        Some(crate::plandoc::Item::Simple(s)) => Some(s),
        _ => None,
    };
    let land = simple(start.checked_sub(1)?).filter(|s| valid_land(s, vtol))?;
    let approach_at = start.checked_sub(2)?;
    let approach = simple(approach_at)?;
    let loiter = match approach.command {
        c if c == i64::from(CMD_NAV_LOITER_TO_ALT) => {
            let first = p(approach, 1);
            let heading_ok = if ardupilot { first == 0.0 || first == 1.0 } else { first == 1.0 };
            (approach.frame == land.frame && heading_ok && p(approach, 3) == 0.0 && p(approach, 4) == 1.0).then_some(true)?
        }
        c if c == i64::from(CMD_NAV_WAYPOINT) => {
            let ok = approach.frame == land.frame && zeros(approach, 1..=3) && (ardupilot || p(approach, 4).is_nan()) && !p(approach, 5).is_nan() && !p(approach, 6).is_nan();
            ok.then_some(false)?
        }
        _ => return None,
    };
    let video = approach_at.checked_sub(1).and_then(simple).is_some_and(|s| s.command == i64::from(CMD_VIDEO_STOP_CAPTURE) && p(s, 1) == 0.0);
    let after_video = approach_at - usize::from(video);
    let photos = after_video.checked_sub(2).and_then(|at| Some((simple(at)?, simple(at + 1)?))).is_some_and(|(trigger, stop)| {
        trigger.command == i64::from(CMD_DO_SET_CAM_TRIGG_DIST) && zeros(trigger, 1..=7) && stop.command == i64::from(CMD_IMAGE_STOP_CAPTURE) && p(stop, 1) == 0.0
    });
    let after_photos = after_video - if photos { 2 } else { 0 };
    let speed = after_photos.checked_sub(1).and_then(simple).filter(|s| s.command == i64::from(CMD_DO_CHANGE_SPEED) && p(s, 1) == SPEED_TYPE_AIRSPEED && p(s, 2) >= -2.0 && p(s, 3) == -1.0 && p(s, 4) == 0.0);
    let after_speed = after_photos - usize::from(speed.is_some());
    let first = after_speed.checked_sub(1)?;
    simple(first).filter(|s| s.command == i64::from(CMD_DO_LAND_START) && zeros(s, 1..=7))?;
    let defaults = |name: &str| fact(&Fresh { vtol, land: (0.0, 0.0), ardupilot, relative: true, transition_distance: None }, name);
    let radius = if loiter { json!(p(approach, 2).abs()) } else { defaults("LoiterRadius") };
    let mut pattern = json!({
        "altitudesAreRelative": land.frame == i64::from(FRAME_GLOBAL_RELATIVE_ALT),
        "complexItemType": if vtol { VTOL_PATTERN } else { FIXED_WING_PATTERN },
        "finalApproachSpeed": speed.map_or_else(|| defaults("FinalApproachSpeed"), |s| json!(p(s, 2))),
        "landCoordinate": [p(land, 5), p(land, 6), p(land, 7)],
        "landingApproachCoordinate": [p(approach, 5), p(approach, 6), p(approach, 7)],
        "loiterClockwise": if loiter { json!(p(approach, 2) > 0.0) } else { defaults("LoiterClockwise") },
        "loiterRadius": radius,
        "stopTakingPhotos": photos,
        "stopVideoPhotos": video,
        "type": "ComplexItem",
        "useDoChangeSpeed": speed.is_some(),
        "useLoiterToAlt": loiter,
        "version": if vtol { 1 } else { 2 },
    });
    if !vtol {
        pattern["valueSetIsDistance"] = defaults("ValueSetIsDistance");
    }
    Some((first, pattern))
}

fn scan_all(items: Vec<crate::plandoc::Item>, vtol: bool, ardupilot: bool) -> (Vec<crate::plandoc::Item>, bool) {
    std::iter::successors(Some((items.len() as i64, items, false)), |(start, items, found)| {
        let at = usize::try_from(*start).ok()?;
        match scan_at(items, at, vtol, ardupilot) {
            Some((first, pattern)) => {
                let count = at - first;
                let folded = crate::plandoc::Item::Complex { kind: pattern["complexItemType"].as_str().unwrap_or("").to_string(), json: pattern, item_count: count };
                let rebuilt: Vec<crate::plandoc::Item> = items[..first].iter().cloned().chain(std::iter::once(folded)).chain(items[at..].iter().cloned()).collect();
                Some((first as i64, rebuilt, true))
            }
            None => Some((start - 1, items.clone(), *found)),
        }
    })
    .last()
    .map(|(_, items, found)| (items, found))
    .unwrap_or_default()
}

pub fn fold(items: Vec<crate::plandoc::Item>, ardupilot: bool) -> Vec<crate::plandoc::Item> {
    match scan_all(items, false, ardupilot) {
        (folded, true) => folded,
        (unchanged, false) => scan_all(unchanged, true, ardupilot).0,
    }
}

pub fn slope_start(pattern: &Value) -> Option<(f64, f64)> {
    let approach = approach(pattern)?;
    let land = coordinate(pattern, "landCoordinate")?;
    let (from, to) = ((land.latitude, land.longitude), (approach.latitude, approach.longitude));
    let radius = pattern.get("loiterRadius").and_then(Value::as_f64).unwrap_or(0.0);
    let apart = crate::surveygrid::distance_between(from, to);
    match flag(pattern, "useLoiterToAlt") && apart >= radius {
        true => {
            let turn = (radius / apart).asin().to_degrees() * if flag(pattern, "loiterClockwise") { 1.0 } else { -1.0 };
            let along = (apart.powi(2) - radius.powi(2)).sqrt();
            Some(crate::surveygrid::at_distance_and_azimuth(from, along, crate::surveygrid::azimuth_to(from, to) + turn))
        }
        false => Some(to),
    }
}

pub fn view_inputs(pattern: &Value) -> Option<(Value, Value)> {
    let place = |p: &Point3| json!({ "valid": true, "latitude": p.latitude, "longitude": p.longitude, "altitude": p.altitude });
    let land = coordinate(pattern, "landCoordinate")?;
    let approach = approach(pattern)?;
    let loiters_down = flag(pattern, "useLoiterToAlt") && slope_start(pattern).is_some_and(|s| s != (approach.latitude, approach.longitude));
    let (slope_latitude, slope_longitude) = slope_start(pattern)?;
    let slope = Point3 { latitude: slope_latitude, longitude: slope_longitude, altitude: if loiters_down { land.altitude } else { approach.altitude } };
    let item = json!({ "kind": "object", "isSimpleItem": false, "landingCoordinate": place(&land), "slopeStartCoordinate": place(&slope), "finalApproachCoordinate": place(&approach) });
    let facts: Vec<Value> = field_values(pattern).into_iter().map(|(_, property, value)| json!({ "property": property, "value": value })).collect();
    Some((item, json!({ "kind": "object", "facts": facts })))
}

pub struct Row {
    pub approach: (f64, f64),
    pub land: (f64, f64),
    pub approach_altitude: f64,
    pub land_altitude: f64,
    pub relative: bool,
    pub distance: f64,
}

pub fn row(pattern: &Value) -> Option<Row> {
    let approach = approach(pattern)?;
    let land = coordinate(pattern, "landCoordinate")?;
    let slope = slope_start(pattern)?;
    let (a, l) = ((approach.latitude, approach.longitude), (land.latitude, land.longitude));
    Some(Row {
        approach: a,
        land: l,
        approach_altitude: approach.altitude,
        land_altitude: land.altitude,
        relative: pattern.get("altitudesAreRelative").and_then(Value::as_bool).unwrap_or(true),
        distance: crate::surveygrid::distance_between(a, slope) + crate::surveygrid::distance_between(slope, l),
    })
}

pub fn items(pattern: &Value, land_start_has_coordinate: bool) -> Result<Vec<Item>, String> {
    let kind = pattern.get("complexItemType").and_then(Value::as_str).unwrap_or("");
    let approach = approach(pattern).ok_or("A landing pattern has no approach coordinate.")?;
    let land = coordinate(pattern, "landCoordinate").ok_or("A landing pattern has no landing coordinate.")?;
    let frame = if pattern.get("altitudesAreRelative").and_then(Value::as_bool).unwrap_or(true) { FRAME_GLOBAL_RELATIVE_ALT } else { FRAME_GLOBAL };
    let at = |p: &Point3| [Some(p.latitude), Some(p.longitude), Some(p.altitude)];
    let [lat, lon, alt] = at(&approach);
    let land_start = match land_start_has_coordinate {
        true => Item { command: CMD_DO_LAND_START, frame, params: [Some(0.0), Some(0.0), Some(0.0), Some(0.0), lat, lon, alt] },
        false => Item { command: CMD_DO_LAND_START, frame: FRAME_MISSION, params: [Some(0.0); 7] },
    };
    let speed = flag(pattern, "useDoChangeSpeed").then(|| {
        let airspeed = pattern.get("finalApproachSpeed").and_then(Value::as_f64).unwrap_or(0.0).trunc();
        Item { command: CMD_DO_CHANGE_SPEED, frame: FRAME_MISSION, params: [Some(SPEED_TYPE_AIRSPEED), Some(airspeed), Some(-1.0), Some(0.0), Some(0.0), Some(0.0), Some(0.0)] }
    });
    let photos = flag(pattern, "stopTakingPhotos").then(|| {
        [
            Item { command: CMD_DO_SET_CAM_TRIGG_DIST, frame: FRAME_MISSION, params: [Some(0.0); 7] },
            Item { command: CMD_IMAGE_STOP_CAPTURE, frame: FRAME_MISSION, params: [Some(0.0), None, None, None, None, None, None] },
        ]
    });
    let video = flag(pattern, "stopVideoPhotos").then(|| Item { command: CMD_VIDEO_STOP_CAPTURE, frame: FRAME_MISSION, params: [Some(0.0), None, None, None, None, None, None] });
    let radius = pattern.get("loiterRadius").and_then(Value::as_f64).unwrap_or(0.0);
    let final_approach = match flag(pattern, "useLoiterToAlt") {
        true => Item { command: CMD_NAV_LOITER_TO_ALT, frame, params: [Some(1.0), Some(if flag(pattern, "loiterClockwise") { radius } else { -radius }), Some(0.0), Some(1.0), lat, lon, alt] },
        false => Item { command: CMD_NAV_WAYPOINT, frame, params: [Some(0.0), Some(0.0), Some(0.0), None, lat, lon, alt] },
    };
    let [land_lat, land_lon, land_alt] = at(&land);
    let touchdown = match kind {
        VTOL_PATTERN => Item { command: CMD_NAV_VTOL_LAND, frame, params: [Some(0.0), Some(0.0), Some(0.0), None, land_lat, land_lon, land_alt] },
        _ => Item { command: CMD_NAV_LAND, frame, params: [Some(0.0), Some(0.0), Some(0.0), Some(0.0), land_lat, land_lon, land_alt] },
    };
    Ok(std::iter::once(land_start)
        .chain(speed)
        .chain(photos.into_iter().flatten())
        .chain(video)
        .chain([final_approach, touchdown])
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn landing_editors_carry_their_own_notes() {
        assert_eq!(notes(false), ["* Approximate glide slope altitudes.", "* Actual flight path will vary.", "* Avoid tailwind on landing."], "FWLandingPatternEditor");
        assert_eq!(notes(true), ["* Actual flight path will vary.", "* Avoid tailwind on approach to land."], "VTOLLandingPatternEditor");
    }

    #[test]
    fn distance_and_glide_slope_are_exclusive_like_the_radio_buttons() {
        let built = fresh(&Fresh { vtol: false, land: (-35.37, 149.172), ardupilot: true, relative: true, transition_distance: None });
        let units = crate::surveydoc::Units { vertical: &crate::read::Unit { name: "m".into(), factor: 1.0 }, horizontal: &crate::read::Unit { name: "m".into(), factor: 1.0 } };
        let enabled = |pattern: &Value, suffix: &str| fields(pattern, "item", &units).into_iter().find(|f| f["pathSuffix"] == suffix).map(|f| f["enabled"].clone());
        let by_slope = edit(&built, "valueSetIsDistance", &json!(false)).unwrap();
        assert_eq!(enabled(&by_slope, "glideSlope"), Some(json!(true)));
        assert_eq!(enabled(&by_slope, "landingDistance"), Some(json!(false)), "FWLandingPatternEditor disables the value the radio did not choose");
        let by_distance = edit(&built, "valueSetIsDistance", &json!(true)).unwrap();
        assert_eq!(enabled(&by_distance, "glideSlope"), Some(json!(false)));
    }

    #[test]
    fn altitudes_relative_to_launch_is_a_checkbox_the_core_edits() {
        let built = fresh(&Fresh { vtol: false, land: (-35.37, 149.172), ardupilot: true, relative: true, transition_distance: None });
        let absolute = edit(&built, "altitudesAreRelative", &json!(false)).unwrap();
        assert_eq!(absolute["altitudesAreRelative"], false, "FWLandingPatternEditor's Altitudes relative to launch");
        assert_eq!(edit(&absolute, "altitudesAreRelative", &json!(true)).unwrap()["altitudesAreRelative"], true);
    }

    #[test]
    fn a_downloaded_landing_sequence_folds_back_into_the_pattern_it_came_from() {
        let sent: Vec<Value> = serde_json::from_str(include_str!("../tests/fixtures/fwland-sent-by-qt.json")).unwrap();
        let items: Vec<crate::plandoc::Item> = sent
            .iter()
            .skip(1)
            .map(|i| {
                let params: [Option<f64>; 7] = std::array::from_fn(|k| i["params"][k].as_f64().map(|v| if k == 4 || k == 5 { if i["frame"] == 2 { v } else { v / 1e7 } } else { v }));
                crate::plandoc::Item::Simple(crate::plandoc::Simple { command: i["command"].as_i64().unwrap(), frame: i["frame"].as_i64().unwrap(), params, auto_continue: true, altitude: None, sections: Vec::new() })
            })
            .collect();
        let folded = fold(items, true);
        assert_eq!(folded.len(), 3, "takeoff, waypoint and the pattern");
        let crate::plandoc::Item::Complex { kind, json, item_count } = &folded[2] else { panic!("the landing sequence did not fold") };
        assert_eq!((kind.as_str(), *item_count), (FIXED_WING_PATTERN, 7));
        let original: Value = serde_json::from_str(include_str!("../tests/fixtures/fwland-pattern.json")).unwrap();
        ["useDoChangeSpeed", "stopTakingPhotos", "stopVideoPhotos", "useLoiterToAlt", "loiterClockwise", "altitudesAreRelative"].iter().for_each(|key| assert_eq!(json[*key], original[*key], "{key}"));
        assert_eq!((json["finalApproachSpeed"].as_f64(), json["loiterRadius"].as_f64()), (Some(14.0), Some(75.0)));
    }

    #[test]
    fn each_landing_field_edit_moves_the_pattern_as_qt_moves_it() {
        let recorded: Value = serde_json::from_str(include_str!("../tests/fixtures/fwland-edits-by-qt.json")).unwrap();
        let near = |a: &Value, b: &Value| a.as_array().unwrap().iter().zip(b.as_array().unwrap()).all(|(x, y)| (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() < 1e-6);
        recorded["steps"].as_array().unwrap().iter().fold(recorded["start"].clone(), |pattern, step| {
            let field = step["field"].as_str().unwrap();
            let edited = edit(&pattern, field, &step["value"]).unwrap();
            ["landingApproachCoordinate", "landCoordinate"].iter().for_each(|key| assert!(near(&edited[*key], &step["after"][*key]), "{field}: {key} {} vs Qt {}", edited[*key], step["after"][*key]));
            ["useDoChangeSpeed", "loiterClockwise", "loiterRadius", "useLoiterToAlt"].iter().for_each(|key| assert_eq!(edited[*key].as_f64().or(edited[*key].as_bool().map(f64::from)), step["after"][*key].as_f64().or(step["after"][*key].as_bool().map(f64::from)), "{field}: {key}"));
            step["after"].clone()
        });
    }

    #[test]
    fn a_new_landing_pattern_is_laid_out_behind_the_touchdown_as_qt_lays_it() {
        let qt: Value = serde_json::from_str(include_str!("../tests/fixtures/landing-inserted-by-qt.json")).unwrap();
        [(false, "fixedWing"), (true, "vtol")].iter().for_each(|(vtol, name)| {
            let built = fresh(&Fresh { vtol: *vtol, land: (-35.37, 149.172), ardupilot: true, relative: true, transition_distance: None });
            assert_eq!(built[WIZARD], json!(!vtol), "Qt opens a fixed wing landing in its wizard and a VTOL one finished");
            let mut saved = built.clone();
            saved.as_object_mut().unwrap().remove(WIZARD);
            let near = |a: &Value, b: &Value| a.as_array().unwrap().iter().zip(b.as_array().unwrap()).all(|(x, y)| (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() < 1e-9);
            assert!(near(&saved["landingApproachCoordinate"], &qt[name]["landingApproachCoordinate"]), "{name}: {} vs {}", saved["landingApproachCoordinate"], qt[name]["landingApproachCoordinate"]);
            saved["landingApproachCoordinate"] = qt[name]["landingApproachCoordinate"].clone();
            fn numbers(v: &Value) -> Value {
                match v {
                    Value::Number(n) => json!(n.as_f64()),
                    Value::Array(a) => Value::Array(a.iter().map(numbers).collect()),
                    Value::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), numbers(v))).collect()),
                    other => other.clone(),
                }
            }
            assert_eq!(numbers(&saved), numbers(&qt[*name]));
        });
    }

    #[test]
    fn a_loiter_approach_starts_its_glide_where_the_circle_meets_the_line_to_land() {
        let pattern: Value = serde_json::from_str(include_str!("../tests/fixtures/fwland-pattern.json")).unwrap();
        let row = row(&pattern).unwrap();
        assert!((row.distance - 944.2174039346808).abs() < 1e-6, "Qt measured this pattern at 944.22 m, core {}", row.distance);
        let straight = serde_json::json!({ "landingApproachCoordinate": pattern["landingApproachCoordinate"], "landCoordinate": pattern["landCoordinate"], "useLoiterToAlt": false });
        let direct = crate::surveygrid::distance_between((row.approach.0, row.approach.1), (row.land.0, row.land.1));
        assert!((self::row(&straight).unwrap().distance - direct).abs() < 1e-9, "without a loiter the glide starts at the approach point");
    }

    #[test]
    fn a_fixed_wing_landing_uploads_the_items_qt_sent() {
        let pattern: Value = serde_json::from_str(include_str!("../tests/fixtures/fwland-pattern.json")).unwrap();
        let sent: Vec<Value> = serde_json::from_str(include_str!("../tests/fixtures/fwland-sent-by-qt.json")).unwrap();
        let ours = items(&pattern, false).unwrap();
        let qt: Vec<&Value> = sent.iter().skip(3).collect();
        assert_eq!(ours.len(), qt.len());
        ours.iter().zip(qt).for_each(|(mine, theirs)| {
            assert_eq!(i64::from(mine.command), theirs["command"].as_i64().unwrap());
            assert_eq!(i64::from(mine.frame), theirs["frame"].as_i64().unwrap(), "command {}", mine.command);
            (0..4).for_each(|k| {
                let want = theirs["params"][k].as_f64();
                let got = mine.params[k];
                assert!(match (got, want) { (Some(a), Some(b)) => (a - b).abs() < 1e-4, (None, None) => true, _ => false }, "command {} param {} {:?} vs {:?}", mine.command, k + 1, got, want);
            });
        });
    }
}
