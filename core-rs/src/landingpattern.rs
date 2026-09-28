use serde_json::Value;

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
