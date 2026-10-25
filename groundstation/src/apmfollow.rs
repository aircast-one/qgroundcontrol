use std::collections::BTreeSet;

use mavlink::dialects::ardupilotmega::MavMessage;
use serde_json::{Value, json};

use crate::read::{Unit, flag, object};
use crate::router::Backend;
use crate::transport::{Frame, LinkId};

pub const DEPS: &[&str] = &[
    "vehicle.parameterManager.parametersReady",
    "vehicle.rover",
    "vehicles.vehicles.count",
    "settings.unitsSettings.horizontalDistanceUnits",
    "settings.unitsSettings.verticalDistanceUnits",
];
pub const FOLLOW_ENABLE: &str = "apmFollow.enable";
pub const FOLLOW_RESET: &str = "apmFollow.reset";
pub const FOLLOW_LEAD: &str = "apmFollow.lead";
pub const FOLLOW_POSITION: &str = "apmFollow.position";
pub const FOLLOW_POINT: &str = "apmFollow.point";
pub const FOLLOW_OFFSETS: &str = "apmFollow.offsets";
pub const FOLLOW_HEIGHT: &str = "apmFollow.height";

const OFFSET_TYPE_RELATIVE: f64 = 1.0;
const ALTITUDE_TYPE_RELATIVE: f64 = 1.0;
const YAW_NONE: f64 = 0.0;
const YAW_FACE: f64 = 1.0;
const YAW_HEADING: f64 = 2.0;
const YAW_FLIGHT: f64 = 3.0;
const DEFAULT_ANGLE: f64 = 45.0;
const DEFAULT_DISTANCE: f64 = 5.0;
const STATION_POINT_VALUES: [f64; 3] = [YAW_NONE, YAW_FACE, YAW_FLIGHT];
const VEHICLE_POINT_VALUES: [f64; 4] = [YAW_NONE, YAW_FACE, YAW_FLIGHT, YAW_HEADING];
const POSITION_OPTIONS: [&str; 2] = ["Maintain Current Offsets", "Specify Offsets"];
const ROVER_MISSING: [&str; 6] = [
    "FOLL_DIST_MAX",
    "FOLL_SYSID",
    "FOLL_OFS_X",
    "FOLL_OFS_Y",
    "FOLL_OFS_Z",
    "FOLL_OFS_TYPE",
];
const COPTER_MISSING: [&str; 2] = ["FOLL_ALT_TYPE", "FOLL_YAW_BEHAVE"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lead {
    pub id: u8,
    pub vehicle: bool,
}

impl Lead {
    pub fn name(self) -> String {
        match self.vehicle {
            true => format!("Vehicle {}", self.id),
            false => "Ground station".to_string(),
        }
    }

    fn is(self, sysid: f64) -> bool {
        f64::from(self.id) == sysid
    }

    pub fn point_values(self) -> &'static [f64] {
        match self.vehicle {
            true => &VEHICLE_POINT_VALUES,
            false => &STATION_POINT_VALUES,
        }
    }

    pub fn point_options(self) -> Vec<String> {
        let name = self.name();
        let (at, along) = match self.vehicle {
            true => (name.clone(), name.clone()),
            false => (
                "ground station location".to_string(),
                "ground station".to_string(),
            ),
        };
        [
            "Maintain current vehicle orientation".to_string(),
            format!("Point at {at}"),
            format!("Same direction as {along} movement"),
            format!("Same heading as {name}"),
        ]
        .into_iter()
        .take(self.point_values().len())
        .collect()
    }
}

pub fn leads(gcs: u8, vehicles: &[u8], follower: Option<u8>) -> Vec<Lead> {
    std::iter::once(Lead {
        id: gcs,
        vehicle: false,
    })
    .chain(
        vehicles
            .iter()
            .filter(|id| Some(**id) != follower && **id != gcs)
            .map(|id| Lead {
                id: *id,
                vehicle: true,
            }),
    )
    .collect()
}

fn connected_leads() -> Vec<Lead> {
    let (vehicles, follower) = {
        let hub = crate::hub::lock();
        (hub.vehicle_ids(), hub.active_id())
    };
    leads(crate::mavout::gcs_system(), &vehicles, follower)
}

fn heading_to_radians(heading: f64) -> f64 {
    (-(heading - 90.0)).to_radians()
}

fn radians_to_heading(radians: f64) -> f64 {
    let heading = 90.0 - radians.to_degrees();
    match heading {
        h if h < 0.0 => h + 360.0,
        h if h > 360.0 => h - 360.0,
        h => h,
    }
}

pub fn offsets(heading: f64, distance: f64) -> (f64, f64) {
    let radians = heading_to_radians(heading);
    let snap = |v: f64| if v.abs() < 0.0001 { 0.0 } else { v };
    match (distance == 0.0, radians == 0.0) {
        (true, _) => (0.0, 0.0),
        (false, true) => (0.0, distance),
        (false, false) => (
            snap(radians.sin() * distance),
            snap(radians.cos() * distance),
        ),
    }
}

pub fn angle_and_distance(x: f64, y: f64) -> (f64, f64) {
    let radians = x.atan2(y);
    let distance = if radians == 0.0 { y } else { x / radians.sin() };
    (radians_to_heading(radians), distance)
}

pub fn supported(
    sysid: f64,
    leads: &[Lead],
    offset_type: f64,
    altitude_type: f64,
    yaw: f64,
    rover: bool,
) -> bool {
    leads
        .iter()
        .find(|lead| lead.is(sysid))
        .is_some_and(|lead| {
            offset_type == OFFSET_TYPE_RELATIVE
                && (rover
                    || (altitude_type == ALTITUDE_TYPE_RELATIVE
                        && lead.point_values().contains(&yaw)))
        })
}

fn path(name: &str) -> String {
    format!("vehicle.parameterManager.getParameter(-1,{name})")
}

fn raw(backend: &dyn Backend, name: &str) -> Option<f64> {
    let fact = backend.value(&path(name));
    let present = fact.get("kind").and_then(Value::as_str) == Some("fact")
        && fact
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|n| !n.is_empty());
    present
        .then(|| {
            fact.get("rawValue")
                .or(fact.get("value"))
                .and_then(Value::as_f64)
        })
        .flatten()
}

fn write(backend: &dyn Backend, name: &str, value: f64) -> bool {
    flag(
        &object(&backend.set(
            &format!("{}.rawValue", path(name)),
            &json!({ "value": value }).to_string(),
        )),
        "ok",
    )
}

fn rover(backend: &dyn Backend) -> bool {
    flag(&backend.value_fields("vehicle", "rover"), "rover")
}

pub fn follow_view(backend: &dyn Backend, _args: &[String]) -> Value {
    let Some(enable) = raw(backend, "FOLL_ENABLE") else {
        return json!({ "kind": "object", "class": "ApmFollow", "available": false });
    };
    let rover = rover(backend);
    let params_available = raw(backend, "FOLL_SYSID").is_some();
    let value = |name: &str| raw(backend, name).unwrap_or(0.0);
    let leads = connected_leads();
    let sysid = value("FOLL_SYSID");
    let lead = leads.iter().position(|lead| lead.is(sysid));
    let shown = lead.map_or(leads[0], |at| leads[at]);
    let ok = params_available
        && supported(
            sysid,
            &leads,
            value("FOLL_OFS_TYPE"),
            value("FOLL_ALT_TYPE"),
            value("FOLL_YAW_BEHAVE"),
            rover,
        );
    let (x, y, z) = (
        value("FOLL_OFS_X"),
        value("FOLL_OFS_Y"),
        value("FOLL_OFS_Z"),
    );
    let maintain = x == 0.0 && y == 0.0 && z == 0.0;
    let (angle, distance) = if maintain {
        (0.0, 0.0)
    } else {
        angle_and_distance(x, y)
    };
    let enabled = enable == 1.0;
    let (horizontal, vertical) = (Unit::horizontal(backend), Unit::vertical(backend));
    json!({
        "kind": "object",
        "class": "ApmFollow",
        "available": true,
        "enabled": enabled,
        "waiting": enabled && !params_available,
        "supported": !params_available || ok,
        "unsupportedText": match lead {
            None if sysid != 0.0 => format!("This vehicle is set to follow system {sysid}, which is neither this ground station nor a connected vehicle. It holds position in Follow mode until that system's position reaches it."),
            _ => "The vehicle parameters required for follow me are currently set in a way which is not supported. Using follow with this setup may lead to unpredictable/hazardous results.".to_string(),
        },
        "showSettings": enabled && params_available && ok,
        "rover": rover,
        "leads": leads.iter().map(|lead| json!({ "id": lead.id, "name": lead.name() })).collect::<Vec<Value>>(),
        "leadIndex": lead.map_or(-1, |at| at as i64),
        "positionOptions": POSITION_OPTIONS,
        "positionIndex": if maintain { 0 } else { 1 },
        "pointOptions": if rover { Vec::new() } else { shown.point_options() },
        "pointIndex": shown.point_values().iter().position(|v| *v == value("FOLL_YAW_BEHAVE")).map_or(-1, |i| i as i64),
        "angle": angle,
        "distance": distance,
        "height": -z,
        "horizontalUnit": horizontal.name,
        "horizontalMetresPerUnit": horizontal.meters(1.0),
        "verticalUnit": vertical.name,
        "verticalMetresPerUnit": vertical.meters(1.0),
    })
}

fn args(text: &str) -> Vec<Value> {
    serde_json::from_str::<Vec<Value>>(text).unwrap_or_default()
}

fn answer(ok: bool) -> Value {
    match ok {
        true => json!({ "ok": true }),
        false => {
            json!({ "ok": false, "reason": "The vehicle did not take the follow me parameters." })
        }
    }
}

fn write_offsets(backend: &dyn Backend, heading: f64, distance: f64) -> bool {
    let (x, y) = offsets(heading, distance);
    write(backend, "FOLL_OFS_X", x) && write(backend, "FOLL_OFS_Y", y)
}

fn current_lead(backend: &dyn Backend, leads: &[Lead]) -> Option<Lead> {
    let sysid = raw(backend, "FOLL_SYSID")?;
    leads.iter().copied().find(|lead| lead.is(sysid))
}

fn reset(backend: &dyn Backend) -> bool {
    let leads = connected_leads();
    let lead = current_lead(backend, &leads).unwrap_or(leads[0]);
    let base = write(backend, "FOLL_SYSID", f64::from(lead.id))
        && write(backend, "FOLL_OFS_TYPE", OFFSET_TYPE_RELATIVE);
    let copter = rover(backend)
        || (write(backend, "FOLL_ALT_TYPE", ALTITUDE_TYPE_RELATIVE)
            && write(backend, "FOLL_YAW_BEHAVE", YAW_FACE));
    base && copter && write_offsets(backend, DEFAULT_ANGLE, DEFAULT_DISTANCE)
}

fn follow_lead(backend: &dyn Backend, lead: Lead) -> bool {
    let heading_kept = rover(backend)
        || lead.vehicle
        || raw(backend, "FOLL_YAW_BEHAVE") != Some(YAW_HEADING)
        || write(backend, "FOLL_YAW_BEHAVE", YAW_FACE);
    heading_kept && write(backend, "FOLL_SYSID", f64::from(lead.id))
}

fn enable(backend: &dyn Backend, on: bool) -> Value {
    if !write(backend, "FOLL_ENABLE", if on { 1.0 } else { 0.0 }) {
        return answer(false);
    }
    if !on {
        return answer(true);
    }
    let names: Vec<&str> = ROVER_MISSING
        .iter()
        .chain(if rover(backend) {
            [].iter()
        } else {
            COPTER_MISSING.iter()
        })
        .copied()
        .collect();
    crate::guided::dispatch(
        backend,
        Some(json!({ "action": "refreshParameters", "names": names })),
        crate::guided::active_id(backend),
        "vehicle.parameterManager.refreshAllParameters",
        "[]",
    )
}

pub fn run(backend: &dyn Backend, action: &str, text: &str) -> Value {
    let given = args(text);
    let number = |i: usize| given.get(i).and_then(Value::as_f64);
    match action {
        FOLLOW_ENABLE => enable(
            backend,
            given.first().and_then(Value::as_bool).unwrap_or(false),
        ),
        FOLLOW_RESET => answer(reset(backend)),
        FOLLOW_LEAD => {
            match number(0).and_then(|id| connected_leads().into_iter().find(|lead| lead.is(id))) {
                Some(lead) => answer(follow_lead(backend, lead)),
                None => {
                    json!({ "ok": false, "reason": "apmFollow.lead takes the system id of this ground station or of a connected vehicle." })
                }
            }
        }
        FOLLOW_POSITION => match number(0) {
            Some(0.0) => answer(
                ["FOLL_OFS_X", "FOLL_OFS_Y", "FOLL_OFS_Z"]
                    .iter()
                    .all(|n| write(backend, n, 0.0)),
            ),
            Some(_) => answer(reset(backend)),
            None => json!({ "ok": false, "reason": "apmFollow.position takes the option index." }),
        },
        FOLLOW_POINT => match number(0).and_then(|i| {
            current_lead(backend, &connected_leads())?
                .point_values()
                .get(i as usize)
                .copied()
        }) {
            Some(yaw) => answer(write(backend, "FOLL_YAW_BEHAVE", yaw)),
            None => {
                json!({ "ok": false, "reason": "apmFollow.point takes one of the options offered for the vehicle being followed." })
            }
        },
        FOLLOW_OFFSETS => match (number(0), number(1)) {
            (Some(angle), Some(distance)) if (0.0..=360.0).contains(&angle) && distance >= 0.0 => {
                answer(write_offsets(backend, angle, distance))
            }
            _ => {
                json!({ "ok": false, "reason": "apmFollow.offsets takes an angle of 0 to 360 degrees and a distance of at least 0 m." })
            }
        },
        FOLLOW_HEIGHT => match number(0).filter(|h| *h >= 0.0) {
            Some(height) => answer(write(backend, "FOLL_OFS_Z", -height)),
            None => {
                json!({ "ok": false, "reason": "apmFollow.height takes a height of at least 0 m." })
            }
        },
        _ => json!({ "ok": false, "reason": format!("{action} is not a follow me action") }),
    }
}

pub fn owns(path: &str) -> bool {
    [
        FOLLOW_ENABLE,
        FOLLOW_RESET,
        FOLLOW_LEAD,
        FOLLOW_POSITION,
        FOLLOW_POINT,
        FOLLOW_OFFSETS,
        FOLLOW_HEIGHT,
    ]
    .contains(&path)
}

pub fn follows(lead: u8, id: u8, enable: Option<f64>, sysid: Option<f64>) -> bool {
    id != lead && enable == Some(1.0) && sysid == Some(f64::from(lead))
}

pub fn lead_position(frame: &Frame, gcs: u8) -> Option<u8> {
    let lead = frame.header.system_id;
    (!frame.replay && lead != gcs && matches!(frame.message, MavMessage::GLOBAL_POSITION_INT(_)))
        .then_some(lead)
}

fn follower_links(lead: u8) -> BTreeSet<LinkId> {
    let hub = crate::hub::lock();
    hub.in_arrival_order()
        .into_iter()
        .filter(|vehicle| !vehicle.replay)
        .filter(|vehicle| {
            let param = |name| {
                vehicle
                    .parameter(vehicle.component, name)
                    .map(|value| value.as_f64())
            };
            follows(lead, vehicle.id, param("FOLL_ENABLE"), param("FOLL_SYSID"))
        })
        .map(|vehicle| vehicle.link)
        .collect()
}

pub fn relay(frame: &Frame) {
    let Some(lead) = lead_position(frame, crate::mavout::gcs_system()) else {
        return;
    };
    let links = follower_links(lead);
    if links.is_empty() {
        return;
    }
    let Some(bytes) = crate::forwarding::outgoing(frame) else {
        return;
    };
    links.iter().for_each(|link| {
        crate::linkhost::write(&crate::linkhost::TRANSPORTS, *link, &bytes);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angle_and_distance_round_trip_through_the_xy_offsets() {
        assert_eq!(offsets(0.0, 0.0), (0.0, 0.0));
        assert_eq!(
            offsets(90.0, 5.0),
            (0.0, 5.0),
            "a 90 degree heading is geometric zero, straight along y"
        );
        let (x, y) = offsets(45.0, 5.0);
        assert!((x - 3.5355).abs() < 0.001 && (y - 3.5355).abs() < 0.001);
        let (angle, distance) = angle_and_distance(x, y);
        assert!((angle - 45.0).abs() < 1e-9 && (distance - 5.0).abs() < 1e-9);
        assert_eq!(
            offsets(0.0, 5.0).1,
            0.0,
            "a near-zero component snaps to zero"
        );
        assert_eq!(angle_and_distance(0.0, 7.0), (90.0, 7.0));
    }

    #[test]
    fn a_relative_follow_of_this_ground_station_or_a_connected_vehicle_is_supported() {
        let leads = leads(255, &[1, 2], Some(1));
        assert_eq!(
            leads,
            vec![
                Lead {
                    id: 255,
                    vehicle: false
                },
                Lead {
                    id: 2,
                    vehicle: true
                }
            ],
            "the follower is never its own lead"
        );
        assert!(supported(255.0, &leads, 1.0, 1.0, 1.0, false));
        assert!(
            supported(2.0, &leads, 1.0, 1.0, 1.0, false),
            "following vehicle 2"
        );
        assert!(
            !supported(3.0, &leads, 1.0, 1.0, 1.0, false),
            "a system that is not connected"
        );
        assert!(
            !supported(255.0, &leads, 0.0, 1.0, 1.0, false),
            "NED offsets"
        );
        assert!(
            !supported(255.0, &leads, 1.0, 0.0, 1.0, false),
            "absolute altitude"
        );
        assert!(
            !supported(255.0, &leads, 1.0, 1.0, 2.0, false),
            "a ground station has no heading to copy"
        );
        assert!(
            supported(2.0, &leads, 1.0, 1.0, 2.0, false),
            "a lead vehicle's heading can be copied"
        );
        assert!(
            supported(255.0, &leads, 1.0, 0.0, 2.0, true),
            "a rover has no altitude or yaw requirement"
        );
    }

    #[test]
    fn a_lead_vehicle_adds_its_heading_after_the_options_a_ground_station_offers() {
        let station = Lead {
            id: 255,
            vehicle: false,
        };
        let vehicle = Lead {
            id: 2,
            vehicle: true,
        };
        assert_eq!(
            station.point_options(),
            [
                "Maintain current vehicle orientation",
                "Point at ground station location",
                "Same direction as ground station movement"
            ]
        );
        assert_eq!(
            vehicle.point_options(),
            [
                "Maintain current vehicle orientation",
                "Point at Vehicle 2",
                "Same direction as Vehicle 2 movement",
                "Same heading as Vehicle 2"
            ]
        );
        assert_eq!(
            vehicle.point_values()[..3],
            station.point_values()[..],
            "an option keeps its index when the lead changes"
        );
    }

    #[test]
    fn only_a_live_lead_position_reaches_an_enabled_follower_of_that_lead() {
        use mavlink::MavHeader;
        use mavlink::dialects::ardupilotmega::{GLOBAL_POSITION_INT_DATA, HEARTBEAT_DATA};
        let frame = |system_id, replay, message| Frame {
            link: 0,
            replay,
            v2: true,
            header: MavHeader {
                system_id,
                component_id: 1,
                sequence: 0,
            },
            message,
            raw: Vec::new(),
        };
        let position = || MavMessage::GLOBAL_POSITION_INT(GLOBAL_POSITION_INT_DATA::default());
        assert_eq!(lead_position(&frame(2, false, position()), 255), Some(2));
        assert_eq!(
            lead_position(&frame(2, true, position()), 255),
            None,
            "a replayed log never reaches a real follower"
        );
        assert_eq!(
            lead_position(&frame(255, false, position()), 255),
            None,
            "this station's own position goes out through follow me"
        );
        assert_eq!(
            lead_position(
                &frame(2, false, MavMessage::HEARTBEAT(HEARTBEAT_DATA::default())),
                255
            ),
            None,
            "a relayed heartbeat would make the follower's companion take the lead for its own autopilot"
        );

        assert!(follows(2, 1, Some(1.0), Some(2.0)));
        assert!(!follows(2, 1, Some(0.0), Some(2.0)), "follow disabled");
        assert!(
            !follows(2, 1, Some(1.0), Some(255.0)),
            "follows this station"
        );
        assert!(!follows(2, 1, None, None), "parameters not loaded");
        assert!(!follows(2, 2, Some(1.0), Some(2.0)), "the lead itself");
    }
}
