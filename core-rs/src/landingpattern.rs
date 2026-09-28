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

pub struct Fresh<'a> {
    pub vtol: bool,
    pub land: (f64, f64),
    pub remembered: &'a dyn Fn(&str) -> Option<String>,
    pub ardupilot: bool,
    pub relative: bool,
}

fn fact(fresh: &Fresh, name: &str) -> Value {
    let (file, group) = if fresh.vtol { (VTOL_META, "VTOLLanding") } else { (FIXED_WING_META, "FixedWingLanding") };
    let Some(meta) = crate::factmeta::from_file(file).ok().and_then(|mut all| all.remove(name)) else { return Value::Null };
    (fresh.remembered)(&format!("{group}/{name}"))
        .and_then(|text| crate::settingsstore::typed(&meta.value_type, &Value::String(text)))
        .or_else(|| meta.default.as_ref().map(|d| crate::settingsstore::typed(&meta.value_type, d).unwrap_or_else(|| d.clone())))
        .unwrap_or(Value::Null)
}

pub fn fresh(fresh: &Fresh) -> Value {
    let number = |name: &str| fact(fresh, name).as_f64().unwrap_or(0.0);
    let on = |name: &str| fact(fresh, name).as_bool().unwrap_or(false);
    let (approach_altitude, land_altitude) = (number("FinalApproachAltitude"), number("LandingAltitude"));
    let distance = match fresh.vtol || on("ValueSetIsDistance") {
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
    fn a_new_landing_pattern_is_laid_out_behind_the_touchdown_as_qt_lays_it() {
        let qt: Value = serde_json::from_str(include_str!("../tests/fixtures/landing-inserted-by-qt.json")).unwrap();
        let nothing = |_: &str| None;
        [(false, "fixedWing"), (true, "vtol")].iter().for_each(|(vtol, name)| {
            let built = fresh(&Fresh { vtol: *vtol, land: (-35.37, 149.172), remembered: &nothing, ardupilot: true, relative: true });
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
