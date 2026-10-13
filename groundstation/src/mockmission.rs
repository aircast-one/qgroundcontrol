use std::collections::BTreeMap;

use mavlink::dialects::ardupilotmega::*;

const JUMP_BUDGET_PER_ITEM: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Guidance {
    pub speed: Option<f64>,
    pub facing: Option<f64>,
    pub roi: Option<(f64, f64)>,
    pub orbit: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Photos {
    every_m: Option<f64>,
    every_s: Option<f64>,
    left: Option<u32>,
    since_m: f64,
    since_s: f64,
    pub taken: u16,
}

impl Photos {
    pub fn every_metres(self, metres: f64) -> Photos {
        Photos { every_m: (metres > 0.0).then_some(metres), since_m: 0.0, ..self }
    }

    pub fn every_seconds(self, seconds: f64, count: u32) -> Photos {
        Photos { every_s: Some(seconds), left: (count > 0).then_some(count), since_s: 0.0, ..self }
    }

    pub fn stopped(self) -> Photos {
        Photos { every_s: None, left: None, ..self }
    }

    pub fn snapped(self) -> Photos {
        Photos { taken: self.taken.saturating_add(1), ..self }
    }

    pub fn advanced(self, travelled_m: f64, dt: f64) -> (Photos, bool) {
        let since_m = self.since_m + travelled_m;
        let since_s = self.since_s + dt;
        let by_distance = self.every_m.is_some_and(|every| since_m >= every);
        let by_time = self.every_s.is_some_and(|every| since_s >= every) && self.left != Some(0);
        let next = Photos {
            since_m: if by_distance { 0.0 } else { since_m },
            since_s: if by_time { 0.0 } else { since_s },
            left: self.left.map(|left| if by_time { left.saturating_sub(1) } else { left }),
            ..self
        };
        (next, by_distance || by_time)
    }
}

#[derive(Debug, PartialEq)]
pub enum Act<'a> {
    Perform(&'a MISSION_ITEM_INT_DATA),
    Fly(&'a MISSION_ITEM_INT_DATA),
    Complete,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Runner {
    pub current: u16,
    pub state: MissionState,
    arrived_at: Option<u64>,
    jumps: BTreeMap<u16, i32>,
}

impl Default for Runner {
    fn default() -> Runner {
        Runner { current: 0, state: MissionState::MISSION_STATE_NOT_STARTED, arrived_at: None, jumps: BTreeMap::new() }
    }
}

impl Runner {
    pub fn active(&self) -> bool {
        self.state == MissionState::MISSION_STATE_ACTIVE
    }

    pub fn pause(&mut self) {
        self.state = if self.active() { MissionState::MISSION_STATE_PAUSED } else { self.state };
    }

    pub fn complete(&mut self) {
        self.state = MissionState::MISSION_STATE_COMPLETE;
    }

    pub fn resume_at(&mut self) -> u16 {
        match self.state {
            MissionState::MISSION_STATE_COMPLETE => {
                self.jumps = BTreeMap::new();
                0
            }
            _ => self.current,
        }
    }

    pub fn walk<'a>(&mut self, items: &'a [MISSION_ITEM_INT_DATA], from: usize) -> Vec<Act<'a>> {
        self.arrived_at = None;
        self.step(items, from, items.len() * JUMP_BUDGET_PER_ITEM + 1, Vec::new())
    }

    fn step<'a>(&mut self, items: &'a [MISSION_ITEM_INT_DATA], at: usize, budget: usize, acts: Vec<Act<'a>>) -> Vec<Act<'a>> {
        match items.get(at) {
            Some(item) if budget > 0 && flies(item.command) => {
                self.current = at as u16;
                self.state = MissionState::MISSION_STATE_ACTIVE;
                acts.into_iter().chain([Act::Fly(item)]).collect()
            }
            Some(item) if budget > 0 && item.command == MavCmd::MAV_CMD_DO_JUMP => {
                let next = self.jump(item, at);
                self.step(items, next, budget - 1, acts)
            }
            Some(item) if budget > 0 => self.step(items, at + 1, budget - 1, acts.into_iter().chain([Act::Perform(item)]).collect()),
            _ => {
                self.complete();
                acts.into_iter().chain([Act::Complete]).collect()
            }
        }
    }

    fn jump(&mut self, item: &MISSION_ITEM_INT_DATA, at: usize) -> usize {
        let left = self.jumps.get(&(at as u16)).copied().unwrap_or(item.param2 as i32);
        match left {
            0 => at + 1,
            n => {
                self.jumps.insert(at as u16, if n < 0 { n } else { n - 1 });
                item.param1 as usize
            }
        }
    }

    pub fn arrive(&mut self, now: u64) -> (bool, u64) {
        let first = self.arrived_at.is_none();
        let since = *self.arrived_at.get_or_insert(now);
        (first, now.saturating_sub(since))
    }

    pub fn report(&self, total: u16) -> MISSION_CURRENT_DATA {
        let state = if total == 0 { MissionState::MISSION_STATE_NO_MISSION } else { self.state };
        MISSION_CURRENT_DATA { seq: self.current, total, mission_state: state, ..Default::default() }
    }
}

pub fn loiters(command: MavCmd) -> bool {
    matches!(command, MavCmd::MAV_CMD_NAV_LOITER_UNLIM | MavCmd::MAV_CMD_NAV_LOITER_TURNS | MavCmd::MAV_CMD_NAV_LOITER_TIME | MavCmd::MAV_CMD_NAV_LOITER_TO_ALT)
}

pub fn flies(command: MavCmd) -> bool {
    matches!(
        command,
        MavCmd::MAV_CMD_NAV_WAYPOINT
            | MavCmd::MAV_CMD_NAV_SPLINE_WAYPOINT
            | MavCmd::MAV_CMD_NAV_LOITER_UNLIM
            | MavCmd::MAV_CMD_NAV_LOITER_TURNS
            | MavCmd::MAV_CMD_NAV_LOITER_TIME
            | MavCmd::MAV_CMD_NAV_LOITER_TO_ALT
            | MavCmd::MAV_CMD_NAV_RETURN_TO_LAUNCH
            | MavCmd::MAV_CMD_NAV_LAND
            | MavCmd::MAV_CMD_NAV_TAKEOFF
            | MavCmd::MAV_CMD_NAV_VTOL_TAKEOFF
            | MavCmd::MAV_CMD_NAV_VTOL_LAND
            | MavCmd::MAV_CMD_NAV_DELAY
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(command: MavCmd, param1: f32, param2: f32) -> MISSION_ITEM_INT_DATA {
        MISSION_ITEM_INT_DATA { command, param1, param2, ..Default::default() }
    }

    #[test]
    fn a_walk_performs_do_items_on_the_way_to_the_next_nav_item_and_jumps_count_down() {
        let items = [item(MavCmd::MAV_CMD_NAV_WAYPOINT, 0.0, 0.0), item(MavCmd::MAV_CMD_DO_CHANGE_SPEED, 1.0, 3.0), item(MavCmd::MAV_CMD_DO_JUMP, 0.0, 1.0), item(MavCmd::MAV_CMD_NAV_LAND, 0.0, 0.0)];
        let mut runner = Runner::default();
        assert_eq!(runner.walk(&items, 1), [Act::Perform(&items[1]), Act::Fly(&items[0])], "the jump sends it back to item 0 once");
        assert_eq!(runner.walk(&items, 1), [Act::Perform(&items[1]), Act::Fly(&items[3])], "and is spent the second time round");
        assert_eq!((runner.current, runner.active()), (3, true));
        assert_eq!(runner.walk(&items, 4), [Act::Complete]);
        let forever = [item(MavCmd::MAV_CMD_DO_JUMP, 0.0, -1.0)];
        assert_eq!(Runner::default().walk(&forever, 0), [Act::Complete], "a loop with nothing to fly runs out of budget instead of spinning");
    }

    #[test]
    fn photos_come_by_distance_or_by_interval_until_the_count_runs_out() {
        let by_distance = Photos::default().every_metres(10.0);
        let (after_nine, due) = by_distance.advanced(9.0, 0.1);
        assert!(!due);
        assert!(after_nine.advanced(1.0, 0.1).1, "the tenth metre takes a photo");
        let twice = Photos::default().every_seconds(1.0, 2);
        let shots = (0..50).scan(twice, |photos, _| {
            let (next, due) = photos.advanced(0.0, 0.1);
            *photos = next;
            Some(due)
        });
        assert_eq!(shots.filter(|due| *due).count(), 2, "a count of two stops after two");
        assert!(!Photos::default().every_metres(0.0).advanced(100.0, 0.1).1, "a zero distance turns the trigger off");
    }
}
