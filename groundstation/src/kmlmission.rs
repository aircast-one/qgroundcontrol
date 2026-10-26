use std::collections::BTreeMap;

use crate::cmdinfo::{Command, Firmware};
use crate::kml::{Route, RouteAltitude};
use crate::planfile::{Mission, SimpleItem};

const CMD_NAV_WAYPOINT: i64 = 16;
const CMD_NAV_RETURN_TO_LAUNCH: i64 = 20;
const CMD_DO_SET_ROI_LOCATION: i64 = 195;
const CMD_DO_SET_ROI_NONE: i64 = 197;
const CMD_DO_SET_ROI: i64 = 201;
const CMD_DO_MOUNT_CONTROL: i64 = 205;
const MAV_ROI_NONE: f64 = 0.0;
const MAV_ROI_LOCATION: f64 = 3.0;
const MOUNT_MODE_MAVLINK_TARGETING: f64 = 2.0;
const FRAME_GLOBAL: i64 = 0;
const FRAME_MISSION: i64 = 2;
const FRAME_GLOBAL_RELATIVE_ALT: i64 = 3;
const GIMBAL_CHANGE_DEGREES: f64 = 1.0;

type Position = (f64, f64, f64);

pub struct Imported {
    pub mission: Mission,
    pub look_ats: usize,
    pub camera_from_recording: bool,
}

struct Builder {
    commands: BTreeMap<i64, Command>,
    px4: bool,
    mode: i64,
}

impl Builder {
    fn item(
        &self,
        command: i64,
        frame: i64,
        mode: Option<i64>,
        set: &[(usize, f64)],
    ) -> SimpleItem {
        let defaults = crate::plandoc::param_defaults(self.commands.get(&command));
        SimpleItem {
            frame,
            command,
            params: std::array::from_fn(|i| {
                set.iter()
                    .find(|(at, _)| *at == i)
                    .map_or(defaults[i].unwrap_or(f64::NAN), |(_, value)| *value)
            }),
            auto_continue: true,
            altitude_mode: mode,
        }
    }

    fn at(
        &self,
        command: i64,
        (lat, lon, alt): Position,
        mode: i64,
        set: &[(usize, f64)],
    ) -> SimpleItem {
        let placed: Vec<(usize, f64)> = set
            .iter()
            .copied()
            .chain([(4, lat), (5, lon), (6, alt)])
            .collect();
        self.item(command, FRAME_GLOBAL, Some(mode), &placed)
    }

    fn look_at(&self, target: Position, mode: i64) -> SimpleItem {
        match self.px4 {
            true => self.at(CMD_DO_SET_ROI_LOCATION, target, mode, &[]),
            false => self.at(CMD_DO_SET_ROI, target, mode, &[(0, MAV_ROI_LOCATION)]),
        }
    }

    fn stop_looking(&self) -> SimpleItem {
        match self.px4 {
            true => self.item(CMD_DO_SET_ROI_NONE, FRAME_MISSION, None, &[]),
            false => self.item(
                CMD_DO_SET_ROI,
                FRAME_GLOBAL_RELATIVE_ALT,
                Some(crate::altitudemodes::RELATIVE),
                &[(0, MAV_ROI_NONE), (4, 0.0), (5, 0.0), (6, 0.0)],
            ),
        }
    }

    fn gimbal(&self, (pitch, yaw): (f64, f64)) -> SimpleItem {
        self.item(
            CMD_DO_MOUNT_CONTROL,
            FRAME_MISSION,
            None,
            &[
                (0, pitch),
                (1, 0.0),
                (2, yaw),
                (3, 0.0),
                (4, 0.0),
                (5, 0.0),
                (6, MOUNT_MODE_MAVLINK_TARGETING),
            ],
        )
    }
}

#[derive(Default)]
struct Camera {
    looking: Option<Position>,
    gimbal: Option<(f64, f64)>,
}

fn placemarks_before(route: &Route) -> Vec<Option<Position>> {
    let distance =
        |a: &Position, b: &Position| crate::surveygrid::distance_between((a.0, a.1), (b.0, b.1));
    route
        .placemarks
        .iter()
        .fold(vec![None; route.path.len()], |taken, placemark| {
            let nearest = route
                .path
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| distance(placemark, a).total_cmp(&distance(placemark, b)))
                .map_or(0, |(i, _)| i);
            match (nearest..taken.len()).find(|i| taken[*i].is_none()) {
                Some(free) => taken
                    .into_iter()
                    .enumerate()
                    .map(|(i, held)| if i == free { Some(*placemark) } else { held })
                    .collect(),
                None => taken,
            }
        })
}

pub fn mission(
    route: &Route,
    firmware_type: i64,
    vehicle_type: i64,
    default_altitude: f64,
) -> Imported {
    let firmware = crate::plandoc::firmware(firmware_type);
    let mode = match route.altitude {
        RouteAltitude::Absolute => crate::altitudemodes::ABSOLUTE,
        _ => crate::altitudemodes::CALC_ABOVE_TERRAIN,
    };
    let build = Builder {
        commands: crate::cmdinfo::tree(firmware, crate::plandoc::vehicle_class(vehicle_type)),
        px4: firmware == Firmware::Px4,
        mode,
    };
    let before = placemarks_before(route);
    let steps: Vec<(Vec<SimpleItem>, bool, bool)> = route
        .path
        .iter()
        .enumerate()
        .scan(Camera::default(), |camera, (i, coordinate)| {
            let recorded = route.look_at.get(i).copied();
            let (looked, from_look_at) = match recorded {
                Some(Some(target)) if camera.looking != Some(target) => {
                    camera.looking = Some(target);
                    camera.gimbal = None;
                    (
                        Some(build.look_at(target, crate::altitudemodes::ABSOLUTE)),
                        true,
                    )
                }
                Some(None) if camera.looking.is_some() => {
                    camera.looking = None;
                    camera.gimbal = None;
                    (Some(build.stop_looking()), false)
                }
                _ => (None, false),
            };
            let placemark = before[i].map(|tower| {
                camera.looking = Some(tower);
                let altitude = match build.mode {
                    crate::altitudemodes::ABSOLUTE => tower.2,
                    _ => 0.0,
                };
                build.look_at((tower.0, tower.1, altitude), build.mode)
            });
            let pitch = route.gimbal.get(i).map_or(f64::NAN, |(pitch, _)| *pitch);
            let yaw = route
                .gimbal
                .get(i)
                .map(|(_, yaw)| *yaw)
                .filter(|yaw| yaw.is_finite())
                .unwrap_or(0.0);
            let unchanged = camera.gimbal.is_some_and(|(held_pitch, held_yaw)| {
                (pitch - held_pitch).abs() < GIMBAL_CHANGE_DEGREES
                    && (yaw - held_yaw).abs() < GIMBAL_CHANGE_DEGREES
            });
            let gimbal = (camera.looking.is_none() && pitch.is_finite() && !unchanged).then(|| {
                camera.gimbal = Some((pitch, yaw));
                build.gimbal((pitch, yaw))
            });
            let altitude = match route.altitude {
                RouteAltitude::Ground => default_altitude,
                _ => coordinate.2,
            };
            let waypoint = build.at(
                CMD_NAV_WAYPOINT,
                (coordinate.0, coordinate.1, altitude),
                build.mode,
                &[],
            );
            let from_recording = from_look_at || gimbal.is_some();
            Some((
                looked
                    .into_iter()
                    .chain(placemark)
                    .chain(gimbal)
                    .chain(std::iter::once(waypoint))
                    .collect(),
                from_recording,
                camera.looking.is_some(),
            ))
        })
        .collect();
    let looking_at_end = steps.last().is_some_and(|(_, _, looking)| *looking);
    let items = steps
        .iter()
        .flat_map(|(items, _, _)| items.iter().cloned())
        .chain(looking_at_end.then(|| build.stop_looking()))
        .chain(std::iter::once(build.item(
            CMD_NAV_RETURN_TO_LAUNCH,
            FRAME_MISSION,
            None,
            &[],
        )))
        .collect();
    Imported {
        mission: Mission {
            home: (0.0, 0.0, 0.0),
            firmware_type,
            vehicle_type,
            cruise_speed: f64::NAN,
            hover_speed: f64::NAN,
            global_altitude_mode: mode,
            items,
        },
        look_ats: before.iter().flatten().count(),
        camera_from_recording: steps.iter().any(|(_, from_recording, _)| *from_recording),
    }
}

pub struct Added {
    pub takeoff: bool,
    pub takeoff_needs_placing: bool,
}

pub fn summary(route: &Route, imported: &Imported, default_altitude: f64, added: &Added) -> String {
    let count = route.path.len();
    let altitude = match route.altitude {
        RouteAltitude::Ground => format!(
            "The file has no flight altitudes: waypoints are {default_altitude} m above the terrain. Check the terrain profile before uploading."
        ),
        RouteAltitude::AboveGround => {
            "Altitudes are kept above the terrain, as in the file.".to_string()
        }
        RouteAltitude::Absolute => {
            "Altitudes are kept above sea level, as in the file.".to_string()
        }
    };
    let takeoff = match (added.takeoff, added.takeoff_needs_placing) {
        (true, true) => Some("A takeoff was added at the start. Drag its T on the map to the climb-out spot, into the wind, and press Done before uploading.".to_string()),
        (true, false) => Some("A takeoff was added at the start.".to_string()),
        (false, _) => None,
    };
    [
        Some(format!("Imported {count} waypoints.")),
        (route.path_count > 1)
            .then(|| format!("{} paths were joined end to end.", route.path_count)),
        (route.source_vertex_count > count).then(|| {
            format!(
                "The path had {} points and was simplified to {count} waypoints.",
                route.source_vertex_count
            )
        }),
        (imported.look_ats > 0).then(|| {
            format!(
                "{} placemarks became look-at points along the path.",
                imported.look_ats
            )
        }),
        imported
            .camera_from_recording
            .then(|| "The camera angles and look-at points from the recording were added.".to_string()),
        Some(altitude),
        (route.altitude != RouteAltitude::Absolute).then(|| "Terrain-following altitudes need terrain data from the internet. Offline, set the altitude mode to Relative before uploading.".to_string()),
        takeoff,
        Some("A return to launch was added at the end.".to_string()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n")
}
