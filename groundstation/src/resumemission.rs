use crate::plantransfer::Item;

pub const JUMP_REFUSAL: &str = "Unable to generate resume mission due to MAV_CMD_DO_JUMP command.";

const CMD_DO_JUMP: u16 = 177;
const CMD_DO_CHANGE_SPEED: u16 = 178;
const CMD_DO_CONTROL_VIDEO: u16 = 200;
const CMD_DO_SET_ROI: u16 = 201;
const CMD_DO_DIGICAM_CONFIGURE: u16 = 202;
const CMD_DO_DIGICAM_CONTROL: u16 = 203;
const CMD_DO_MOUNT_CONFIGURE: u16 = 204;
const CMD_DO_MOUNT_CONTROL: u16 = 205;
const CMD_DO_SET_CAM_TRIGG_DIST: u16 = 206;
const CMD_DO_FENCE_ENABLE: u16 = 207;
const CMD_SET_CAMERA_MODE: u16 = 530;
const CMD_IMAGE_START_CAPTURE: u16 = 2000;
const CMD_IMAGE_STOP_CAPTURE: u16 = 2001;
const CMD_VIDEO_START_CAPTURE: u16 = 2500;
const CMD_VIDEO_STOP_CAPTURE: u16 = 2501;

const CARRIED_FORWARD: &[u16] = &[
    CMD_DO_CONTROL_VIDEO,
    CMD_DO_SET_ROI,
    CMD_DO_DIGICAM_CONFIGURE,
    CMD_DO_DIGICAM_CONTROL,
    CMD_DO_MOUNT_CONFIGURE,
    CMD_DO_MOUNT_CONTROL,
    CMD_DO_SET_CAM_TRIGG_DIST,
    CMD_DO_FENCE_ENABLE,
    CMD_IMAGE_START_CAPTURE,
    CMD_IMAGE_STOP_CAPTURE,
    CMD_VIDEO_START_CAPTURE,
    CMD_VIDEO_STOP_CAPTURE,
    CMD_DO_CHANGE_SPEED,
    CMD_SET_CAMERA_MODE,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shape {
    pub flies_through: bool,
    pub takeoff: bool,
}

pub fn shape(commands: &std::collections::BTreeMap<i64, crate::cmdinfo::Command>, command: u16) -> Shape {
    commands.get(&i64::from(command)).map_or(Shape::default(), |c| Shape { flies_through: c.specifies_coordinate && !c.standalone_coordinate, takeoff: c.is_takeoff })
}

fn resume_at(items: &[Item], asked: usize, shape_of: &impl Fn(u16) -> Shape) -> usize {
    let clamped = asked.min(items.len().saturating_sub(1));
    match shape_of(items[clamped].command).flies_through {
        true => clamped,
        false => (1..clamped).rev().find(|i| shape_of(items[*i].command).flies_through).unwrap_or(0),
    }
}

#[derive(Default)]
struct Seen {
    roi: bool,
    camera_mode: bool,
    camera_start_stop: bool,
}

fn kept_from_back(item: &Item, seen: &mut Seen) -> bool {
    let first = |flag: &mut bool| !std::mem::replace(flag, true);
    match item.command {
        CMD_SET_CAMERA_MODE => first(&mut seen.camera_mode),
        CMD_DO_SET_ROI => first(&mut seen.roi),
        CMD_DO_SET_CAM_TRIGG_DIST | CMD_IMAGE_STOP_CAPTURE | CMD_VIDEO_START_CAPTURE | CMD_VIDEO_STOP_CAPTURE => first(&mut seen.camera_start_stop),
        CMD_IMAGE_START_CAPTURE if item.params[2] != 0.0 => false,
        CMD_IMAGE_START_CAPTURE => first(&mut seen.camera_start_stop),
        _ => true,
    }
}

pub fn resume_items(items: &[Item], asked: usize, sends_home: bool, shape_of: impl Fn(u16) -> Shape) -> Result<Vec<Item>, &'static str> {
    if items.iter().any(|item| item.command == CMD_DO_JUMP) {
        return Err(JUMP_REFUSAL);
    }
    if items.is_empty() {
        return Err("The vehicle holds no mission to resume.");
    }
    let resume = resume_at(items, asked, &shape_of);
    let picked: Vec<(usize, &Item)> = items
        .iter()
        .enumerate()
        .filter(|(i, item)| (*i == 0 && sends_home) || *i >= resume || CARRIED_FORWARD.contains(&item.command) || shape_of(item.command).takeoff)
        .collect();
    let prefix = picked.iter().filter(|(i, _)| *i < resume).count();
    let mut seen = Seen::default();
    let kept_prefix: Vec<bool> = picked[..prefix].iter().rev().map(|(_, item)| kept_from_back(item, &mut seen)).collect::<Vec<_>>().into_iter().rev().collect();
    let current = usize::from(sends_home);
    Ok(picked
        .iter()
        .enumerate()
        .filter(|(at, _)| kept_prefix.get(*at).copied().unwrap_or(true))
        .map(|(_, (_, item))| *item)
        .enumerate()
        .map(|(seq, item)| Item { seq: seq as u16, current: seq == current, ..item.clone() })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAV_WAYPOINT: u16 = 16;
    const NAV_TAKEOFF: u16 = 22;

    fn item(seq: u16, command: u16) -> Item {
        Item { seq, frame: 3, command, current: false, auto_continue: true, params: [0.0, 0.0, 0.0, 0.0, f64::from(seq), 0.0, 10.0] }
    }

    fn shape_of(command: u16) -> Shape {
        Shape { flies_through: matches!(command, NAV_WAYPOINT | NAV_TAKEOFF), takeoff: command == NAV_TAKEOFF }
    }

    #[test]
    fn a_resumed_mission_keeps_home_takeoff_and_the_settings_before_the_resume_point() {
        let mission = [item(0, NAV_WAYPOINT), item(1, NAV_TAKEOFF), item(2, CMD_DO_CHANGE_SPEED), item(3, NAV_WAYPOINT), item(4, CMD_DO_SET_ROI), item(5, CMD_DO_SET_ROI), item(6, NAV_WAYPOINT), item(7, NAV_WAYPOINT)];
        let resumed = resume_items(&mission, 6, true, shape_of).unwrap();
        let commands: Vec<u16> = resumed.iter().map(|i| i.command).collect();
        assert_eq!(commands, [NAV_WAYPOINT, NAV_TAKEOFF, CMD_DO_CHANGE_SPEED, CMD_DO_SET_ROI, NAV_WAYPOINT, NAV_WAYPOINT], "the waypoint at 3 is skipped and only the last ROI is kept");
        assert_eq!(resumed[3].params[4], 5.0, "the kept ROI is the later one");
        assert_eq!(resumed.iter().map(|i| i.seq).collect::<Vec<_>>(), (0..6).collect::<Vec<u16>>());
        assert_eq!(resumed.iter().position(|i| i.current), Some(1), "the item after home is current when home is sent");
    }

    #[test]
    fn a_resume_point_on_a_command_backs_up_to_the_waypoint_before_it() {
        let mission = [item(0, NAV_WAYPOINT), item(1, NAV_WAYPOINT), item(2, CMD_DO_CHANGE_SPEED), item(3, NAV_WAYPOINT)];
        let resumed = resume_items(&mission, 2, false, shape_of).unwrap();
        assert_eq!(resumed.iter().map(|i| i.params[4]).collect::<Vec<_>>(), [1.0, 2.0, 3.0]);
        assert_eq!(resumed.iter().position(|i| i.current), Some(0));
    }

    #[test]
    fn camera_triggers_by_distance_are_not_replayed_twice_and_jumps_refuse() {
        let mut timed = item(2, CMD_IMAGE_START_CAPTURE);
        timed.params[2] = 5.0;
        let mission = [item(0, NAV_WAYPOINT), item(1, CMD_IMAGE_STOP_CAPTURE), timed, item(3, CMD_DO_SET_CAM_TRIGG_DIST), item(4, NAV_WAYPOINT)];
        let commands: Vec<u16> = resume_items(&mission, 4, true, shape_of).unwrap().iter().map(|i| i.command).collect();
        assert_eq!(commands, [NAV_WAYPOINT, CMD_DO_SET_CAM_TRIGG_DIST, NAV_WAYPOINT], "a counted capture is dropped and only the last start/stop survives");
        assert_eq!(resume_items(&[item(0, NAV_WAYPOINT), item(1, CMD_DO_JUMP)], 0, true, shape_of), Err(JUMP_REFUSAL));
    }
}
