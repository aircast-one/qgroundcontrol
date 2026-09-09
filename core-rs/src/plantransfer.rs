pub const PLAN_MISSION: u8 = 0;
pub const PLAN_FENCE: u8 = 1;
pub const PLAN_RALLY: u8 = 2;
pub const CMD_FENCE_RETURN_POINT: u16 = 5000;
pub const CMD_FENCE_POLYGON_INCLUSION: u16 = 5001;
pub const CMD_FENCE_POLYGON_EXCLUSION: u16 = 5002;
pub const CMD_FENCE_CIRCLE_INCLUSION: u16 = 5003;
pub const CMD_FENCE_CIRCLE_EXCLUSION: u16 = 5004;
pub const CMD_RALLY_POINT: u16 = 5100;
pub const ACK_TIMEOUT_MS: u64 = 1500;
pub const ITEM_TIMEOUT_MS: u64 = 250;
pub const MAX_RETRY: u32 = 5;
pub const RESULT_ACCEPTED: u8 = 0;
pub const RESULT_INVALID_SEQUENCE: u8 = 13;
pub const CMD_DO_JUMP: u16 = 177;
pub const FRAME_MISSION: u8 = 2;
pub const FRAME_GLOBAL_INT: u8 = 5;
pub const FRAME_GLOBAL_RELATIVE_ALT_INT: u8 = 6;
pub const FRAME_GLOBAL: u8 = 0;
pub const FRAME_GLOBAL_RELATIVE_ALT: u8 = 3;

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub seq: u16,
    pub frame: u8,
    pub command: u16,
    pub current: bool,
    pub auto_continue: bool,
    pub params: [f64; 7],
}

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    RequestList,
    RequestItem(u16),
    SendCount(u16),
    SendItem(Item),
    SendAck,
    StartTimer(u64),
    StopTimer,
    Progress(f64),
    HomePosition(f64, f64, f64),
    Done { success: bool, error: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    Count,
    Item,
    Request,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transaction {
    Read,
    Write,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    pub inclusion: bool,
    pub vertices: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Circle {
    pub inclusion: bool,
    pub center: (f64, f64),
    pub radius: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fence {
    pub polygons: Vec<Polygon>,
    pub circles: Vec<Circle>,
    pub breach_return: Option<(f64, f64, f64)>,
}

fn plain(command: u16, frame: u8, params: [f64; 7]) -> Item {
    Item { seq: 0, frame, command, current: false, auto_continue: false, params }
}

pub fn fence_items(fence: &Fence) -> Vec<Item> {
    let polygons = fence.polygons.iter().flat_map(|polygon| {
        let command = if polygon.inclusion { CMD_FENCE_POLYGON_INCLUSION } else { CMD_FENCE_POLYGON_EXCLUSION };
        let count = polygon.vertices.len() as f64;
        polygon.vertices.iter().map(move |(lat, lon)| plain(command, FRAME_GLOBAL, [count, 0.0, 0.0, 0.0, *lat, *lon, 0.0]))
    });
    let circles = fence.circles.iter().map(|circle| plain(if circle.inclusion { CMD_FENCE_CIRCLE_INCLUSION } else { CMD_FENCE_CIRCLE_EXCLUSION }, FRAME_GLOBAL, [circle.radius, 0.0, 0.0, 0.0, circle.center.0, circle.center.1, 0.0]));
    let breach = fence.breach_return.map(|(lat, lon, alt)| plain(CMD_FENCE_RETURN_POINT, FRAME_GLOBAL_RELATIVE_ALT, [0.0, 0.0, 0.0, 0.0, lat, lon, alt]));
    polygons.chain(circles).chain(breach).collect()
}

pub fn fence_from_items(items: &[Item]) -> Result<Fence, String> {
    let mut fence = Fence::default();
    let mut open: Vec<(f64, f64)> = Vec::new();
    let mut expected: Option<(u16, usize)> = None;
    for item in items {
        match item.command {
            CMD_FENCE_POLYGON_INCLUSION | CMD_FENCE_POLYGON_EXCLUSION => {
                let count = item.params[0] as usize;
                match expected {
                    None => expected = Some((item.command, count)),
                    Some((_, wanted)) if wanted != count => return Err(format!("GeoFence load: Vertex count change mid-polygon - actual:expected {count}:{wanted}")),
                    Some((command, _)) if command != item.command => return Err(format!("GeoFence load: Polygon type changed before last load complete - actual:expected {}:{command}", item.command)),
                    _ => {}
                }
                open.push((item.params[4], item.params[5]));
                if open.len() == count {
                    fence.polygons.push(Polygon { inclusion: item.command == CMD_FENCE_POLYGON_INCLUSION, vertices: std::mem::take(&mut open) });
                    expected = None;
                }
            }
            CMD_FENCE_CIRCLE_INCLUSION | CMD_FENCE_CIRCLE_EXCLUSION => {
                if !open.is_empty() {
                    return Err("GeoFence load: Incomplete polygon loaded".to_string());
                }
                fence.circles.push(Circle { inclusion: item.command == CMD_FENCE_CIRCLE_INCLUSION, center: (item.params[4], item.params[5]), radius: item.params[0] });
            }
            CMD_FENCE_RETURN_POINT => fence.breach_return = Some((item.params[4], item.params[5], item.params[6])),
            other => return Err(format!("GeoFence load: Unsupported command {other}")),
        }
    }
    Ok(fence)
}

pub fn rally_items(points: &[(f64, f64, f64)]) -> Vec<Item> {
    points.iter().map(|(lat, lon, alt)| plain(CMD_RALLY_POINT, FRAME_GLOBAL_RELATIVE_ALT, [0.0, 0.0, 0.0, 0.0, *lat, *lon, *alt])).collect()
}

pub fn rally_from_items(items: &[Item]) -> Vec<(f64, f64, f64)> {
    items.iter().take_while(|i| i.command == CMD_RALLY_POINT).map(|i| (i.params[4], i.params[5], i.params[6])).collect()
}

#[derive(Debug, Default)]
pub struct Transfer {
    plan_type: u8,
    apm: bool,
    skip_first: bool,
    transaction: Option<Transaction>,
    expect: Option<Expect>,
    retries: u32,
    to_read: Vec<u16>,
    count_to_read: u16,
    to_write: Vec<u16>,
    writing: Vec<Item>,
    pub items: Vec<Item>,
}

fn result_text(result: u8) -> String {
    match result {
        0 => "Mission accepted".to_string(),
        1 => "Unspecified error".to_string(),
        2 => "Coordinate frame is not supported".to_string(),
        3 => "Command is not supported".to_string(),
        4 => "Mission item exceeds storage space".to_string(),
        5 => "One of the parameters has an invalid value".to_string(),
        6..=12 => format!("Param {} has an invalid value", result - 5),
        13 => "Received mission item out of sequence".to_string(),
        14 => "Not accepting any mission commands".to_string(),
        15 => "Mission operation cancelled".to_string(),
        other => format!("QGC Internal Error: unknown mission result {other}"),
    }
}

impl Transfer {
    pub fn new(apm: bool, plan_type: u8) -> Transfer {
        Transfer { plan_type, apm, skip_first: apm && plan_type == PLAN_MISSION, ..Transfer::default() }
    }

    pub fn in_progress(&self) -> bool {
        self.transaction.is_some()
    }

    pub fn transaction(&self) -> Option<Transaction> {
        self.transaction
    }

    fn expecting(&mut self, expect: Expect) -> Vec<Out> {
        self.expect = Some(expect);
        vec![Out::StartTimer(if expect == Expect::Item { ITEM_TIMEOUT_MS } else { ACK_TIMEOUT_MS })]
    }

    fn take_expected(&mut self, expect: Expect) -> bool {
        if self.expect != Some(expect) {
            return false;
        }
        self.expect = None;
        true
    }

    fn finish(&mut self, success: bool, error: &str) -> Vec<Out> {
        self.to_read.clear();
        self.to_write.clear();
        self.expect = None;
        let transaction = self.transaction.take();
        match (transaction, success) {
            (Some(Transaction::Read), false) => self.items.clear(),
            (Some(Transaction::Write), true) => self.items = std::mem::take(&mut self.writing),
            _ => {}
        }
        self.writing.clear();
        vec![Out::StopTimer, Out::Progress(1.0), Out::Done { success, error: error.to_string() }]
    }

    pub fn load(&mut self) -> Vec<Out> {
        if self.in_progress() {
            return Vec::new();
        }
        self.retries = 0;
        self.transaction = Some(Transaction::Read);
        self.request_list()
    }

    fn request_list(&mut self) -> Vec<Out> {
        self.to_read.clear();
        self.items.clear();
        let mut out = vec![Out::RequestList];
        out.extend(self.expecting(Expect::Count));
        out
    }

    fn request_next(&mut self) -> Vec<Out> {
        let Some(next) = self.to_read.first().copied() else { return Vec::new() };
        let mut out = vec![Out::RequestItem(next)];
        out.extend(self.expecting(Expect::Item));
        out
    }

    pub fn write(&mut self, items: Vec<Item>) -> Vec<Out> {
        if self.in_progress() {
            return Vec::new();
        }
        let skip_first = self.skip_first;
        let first = usize::from(skip_first && !items.is_empty());
        self.writing = items
            .into_iter()
            .skip(first)
            .enumerate()
            .map(|(i, item)| {
                let seq = i as u16;
                let params = match (skip_first, item.command) {
                    (true, CMD_DO_JUMP) => [item.params[0].trunc() - 1.0, item.params[1], item.params[2], item.params[3], item.params[4], item.params[5], item.params[6]],
                    _ => item.params,
                };
                Item { seq, current: i == 0, params, ..item }
            })
            .collect();
        self.to_write = (0..self.writing.len() as u16).collect();
        self.retries = 0;
        self.transaction = Some(Transaction::Write);
        let mut out = vec![Out::Progress(0.0)];
        out.extend(self.write_count());
        out
    }

    fn write_count(&mut self) -> Vec<Out> {
        let mut out = vec![Out::SendCount(self.writing.len() as u16)];
        out.extend(self.expecting(Expect::Request));
        out
    }

    pub fn on_timeout(&mut self) -> Vec<Out> {
        match self.expect {
            None => Vec::new(),
            Some(Expect::Count) if self.retries > MAX_RETRY => self.finish(false, "Mission request list failed, maximum retries exceeded."),
            Some(Expect::Count) => {
                self.retries += 1;
                self.request_list()
            }
            Some(Expect::Item) if self.retries > MAX_RETRY => self.finish(false, "Mission read failed, maximum retries exceeded."),
            Some(Expect::Item) => {
                self.retries += 1;
                self.request_next()
            }
            Some(Expect::Request) if self.to_write.is_empty() => self.finish(false, "Mission write failed, vehicle failed to send final ack."),
            Some(Expect::Request) if self.to_write.first() == Some(&0) && self.retries > MAX_RETRY => self.finish(false, "Mission write mission count failed, maximum retries exceeded."),
            Some(Expect::Request) if self.to_write.first() == Some(&0) => {
                self.retries += 1;
                self.write_count()
            }
            Some(Expect::Request) => self.finish(false, "Vehicle did not request all items from ground station: MISSION_REQUEST"),
        }
    }

    pub fn on_count(&mut self, count: u16) -> Vec<Out> {
        if !self.take_expected(Expect::Count) {
            return Vec::new();
        }
        self.retries = 0;
        if count == 0 {
            let mut out = vec![Out::StopTimer, Out::SendAck];
            out.extend(self.finish(true, ""));
            return out;
        }
        self.to_read = (0..count).collect();
        self.count_to_read = count;
        let mut out = vec![Out::StopTimer];
        out.extend(self.request_next());
        out
    }

    pub fn on_item(&mut self, item: Item) -> Vec<Out> {
        let frame = match item.frame {
            FRAME_GLOBAL_INT => FRAME_GLOBAL,
            FRAME_GLOBAL_RELATIVE_ALT_INT => FRAME_GLOBAL_RELATIVE_ALT,
            other => other,
        };
        let item = Item { frame, ..item };
        if !self.take_expected(Expect::Item) {
            return match (self.apm, self.plan_type, item.seq) {
                (true, PLAN_MISSION, 0) => vec![Out::HomePosition(item.params[4], item.params[5], item.params[6])],
                _ => Vec::new(),
            };
        }
        if !self.to_read.contains(&item.seq) {
            return self.expecting(Expect::Item);
        }
        self.to_read.retain(|s| *s != item.seq);
        let params = match (item.command, self.skip_first) {
            (CMD_DO_JUMP, true) => [item.params[0].trunc() + 1.0, item.params[1], item.params[2], item.params[3], item.params[4], item.params[5], item.params[6]],
            _ => item.params,
        };
        let seq = item.seq;
        self.items.push(Item { params, ..item });
        self.retries = 0;
        let mut out = vec![Out::StopTimer, Out::Progress(seq as f64 / self.count_to_read.max(1) as f64)];
        if self.to_read.is_empty() {
            out.push(Out::SendAck);
            out.extend(self.finish(true, ""));
        } else {
            out.extend(self.request_next());
        }
        out
    }

    pub fn on_request(&mut self, seq: u16) -> Vec<Out> {
        if !self.take_expected(Expect::Request) {
            return Vec::new();
        }
        if seq as usize >= self.writing.len() {
            return self.finish(false, &format!("Vehicle requested item outside range, count:request {}:{seq}. Send to Vehicle failed.", self.writing.len()));
        }
        self.to_write.retain(|s| *s != seq);
        let item = self.writing[seq as usize].clone();
        let mut out = vec![Out::StopTimer, Out::Progress(seq as f64 / self.writing.len() as f64), Out::SendItem(Item { current: seq == 0, ..item })];
        out.extend(self.expecting(Expect::Request));
        out
    }

    pub fn on_ack(&mut self, result: u8) -> Vec<Out> {
        if self.apm && result == RESULT_INVALID_SEQUENCE {
            return Vec::new();
        }
        let Some(expected) = self.expect else { return Vec::new() };
        self.expect = None;
        match (expected, result, self.to_write.is_empty()) {
            (Expect::Request, RESULT_ACCEPTED, true) => self.finish(true, ""),
            (Expect::Request, RESULT_ACCEPTED, false) => self.finish(false, "Vehicle acknowledged the mission before requesting every item."),
            _ => self.finish(false, &result_text(result)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn waypoint(seq: u16, lat: f64) -> Item {
        Item { seq, frame: FRAME_GLOBAL_RELATIVE_ALT_INT, command: 16, current: false, auto_continue: true, params: [0.0, 0.0, 0.0, 0.0, lat, 8.5, 50.0] }
    }

    fn timer(out: &[Out]) -> Option<u64> {
        out.iter().find_map(|o| match o { Out::StartTimer(ms) => Some(*ms), _ => None })
    }

    #[test]
    fn a_read_walks_count_and_items_and_acks_the_vehicle() {
        let mut transfer = Transfer::new(false, PLAN_MISSION);
        let started = transfer.load();
        assert_eq!((started[0].clone(), timer(&started)), (Out::RequestList, Some(ACK_TIMEOUT_MS)));
        assert!(transfer.load().is_empty(), "one transaction at a time");
        let counted = transfer.on_count(2);
        assert!(counted.contains(&Out::RequestItem(0)));
        assert_eq!(timer(&counted), Some(ITEM_TIMEOUT_MS));
        assert!(transfer.on_item(waypoint(5, 1.0)).contains(&Out::StartTimer(ITEM_TIMEOUT_MS)), "an unexpected sequence keeps waiting");
        let first = transfer.on_item(waypoint(0, 47.0));
        assert!(first.contains(&Out::RequestItem(1)));
        let last = transfer.on_item(waypoint(1, 47.1));
        assert!(last.contains(&Out::SendAck));
        assert!(matches!(last.last(), Some(Out::Done { success: true, .. })));
        assert_eq!(transfer.items.len(), 2);
        assert_eq!(transfer.items[1].frame, FRAME_GLOBAL_RELATIVE_ALT, "int frames come back as their float twins, as the Qt manager stores them");
        assert!(!transfer.in_progress());
    }

    #[test]
    fn retries_follow_the_qt_manager_and_give_up_after_five() {
        let mut transfer = Transfer::new(false, PLAN_MISSION);
        transfer.load();
        let retried: Vec<Vec<Out>> = (0..6).map(|_| transfer.on_timeout()).collect();
        assert!(retried.iter().all(|r| r.contains(&Out::RequestList)));
        let gave_up = transfer.on_timeout();
        assert!(matches!(gave_up.last(), Some(Out::Done { success: false, error }) if error.contains("maximum retries")));
        assert!(!transfer.in_progress());
    }

    #[test]
    fn a_write_skips_the_home_item_on_ardupilot_and_serves_requests_until_the_ack() {
        let mut transfer = Transfer::new(true, PLAN_MISSION);
        let jump = Item { command: CMD_DO_JUMP, params: [3.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], ..waypoint(2, 0.0) };
        let started = transfer.write(vec![waypoint(0, 0.0), waypoint(1, 47.0), jump]);
        assert!(started.contains(&Out::SendCount(2)), "the home item is not sent to ArduPilot");
        let first = transfer.on_request(0);
        assert!(matches!(first.iter().find(|o| matches!(o, Out::SendItem(_))), Some(Out::SendItem(item)) if item.seq == 0 && item.current && item.params[4] == 47.0));
        let second = transfer.on_request(1);
        assert!(matches!(second.iter().find(|o| matches!(o, Out::SendItem(_))), Some(Out::SendItem(item)) if item.command == CMD_DO_JUMP && item.params[0] == 2.0), "a jump target moves down with the sequence");
        let done = transfer.on_ack(RESULT_ACCEPTED);
        assert!(matches!(done.last(), Some(Out::Done { success: true, .. })));
        assert_eq!(transfer.items.len(), 2, "a successful write becomes the known mission");
        let mut refused = Transfer::new(false, PLAN_MISSION);
        refused.write(vec![waypoint(0, 0.0)]);
        assert!(matches!(refused.on_ack(4).last(), Some(Out::Done { success: false, error }) if error.contains("storage")));
        let mut outside = Transfer::new(false, PLAN_MISSION);
        outside.write(vec![waypoint(0, 0.0)]);
        assert!(matches!(outside.on_request(7).last(), Some(Out::Done { success: false, error }) if error.contains("outside range")));
    }

    #[test]
    fn item_retries_use_the_short_timer_and_an_early_ack_fails_a_write() {
        let mut transfer = Transfer::new(true, PLAN_MISSION);
        transfer.load();
        transfer.on_count(1);
        let retried: Vec<Vec<Out>> = (0..6).map(|_| transfer.on_timeout()).collect();
        assert!(retried.iter().all(|r| r.contains(&Out::RequestItem(0)) && timer(r) == Some(ITEM_TIMEOUT_MS)));
        assert!(matches!(transfer.on_timeout().last(), Some(Out::Done { success: false, error }) if error.contains("Mission read failed")));
        let mut reading = Transfer::new(true, PLAN_MISSION);
        reading.load();
        reading.on_count(1);
        let jump = Item { command: CMD_DO_JUMP, params: [3.7, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], ..waypoint(0, 0.0) };
        reading.on_item(jump);
        assert_eq!(reading.items[0].params[0], 4.0, "a jump read from ArduPilot moves up past the home item the head shows");
        let mut early = Transfer::new(false, PLAN_MISSION);
        early.write(vec![waypoint(0, 0.0), waypoint(1, 1.0)]);
        early.on_request(0);
        assert!(matches!(early.on_ack(RESULT_ACCEPTED).last(), Some(Out::Done { success: false, error }) if error.contains("before requesting every item")));
        let mut stalled = Transfer::new(false, PLAN_MISSION);
        stalled.write(vec![waypoint(0, 0.0), waypoint(1, 1.0)]);
        stalled.on_request(0);
        assert!(matches!(stalled.on_timeout().last(), Some(Out::Done { success: false, error }) if error.contains("did not request all items")));
    }

    #[test]
    fn an_unrequested_first_item_on_ardupilot_is_the_home_position() {
        let mut transfer = Transfer::new(true, PLAN_MISSION);
        assert_eq!(transfer.on_item(waypoint(0, 47.5)), vec![Out::HomePosition(47.5, 8.5, 50.0)]);
        let mut px4 = Transfer::new(false, PLAN_MISSION);
        assert!(px4.on_item(waypoint(0, 47.5)).is_empty());
        assert!(px4.on_ack(RESULT_ACCEPTED).is_empty(), "an ack with nothing expected is ignored");
        let mut apm = Transfer::new(true, PLAN_MISSION);
        apm.load();
        assert!(apm.on_ack(RESULT_INVALID_SEQUENCE).is_empty(), "ArduPilot's invalid-sequence ack is ignored");
    }

    #[test]
    fn fence_items_round_trip_and_bad_sequences_are_named_as_the_geofence_manager_names_them() {
        let fence = Fence {
            polygons: vec![Polygon { inclusion: true, vertices: vec![(47.0, 8.0), (47.1, 8.0), (47.1, 8.1)] }, Polygon { inclusion: false, vertices: vec![(46.0, 7.0), (46.1, 7.0), (46.1, 7.1)] }],
            circles: vec![Circle { inclusion: true, center: (47.05, 8.05), radius: 120.0 }],
            breach_return: Some((47.02, 8.02, 30.0)),
        };
        let items = fence_items(&fence);
        assert_eq!(items.len(), 8);
        assert_eq!((items[0].command, items[0].params[0], items[0].frame), (CMD_FENCE_POLYGON_INCLUSION, 3.0, FRAME_GLOBAL));
        assert_eq!((items[7].command, items[7].frame, items[7].params[6]), (CMD_FENCE_RETURN_POINT, FRAME_GLOBAL_RELATIVE_ALT, 30.0));
        assert_eq!(fence_from_items(&items).unwrap(), fence);
        let mut broken = items.clone();
        broken[1].params[0] = 4.0;
        assert!(fence_from_items(&broken).unwrap_err().contains("Vertex count change"));
        let mut mixed = items.clone();
        mixed[1].command = CMD_FENCE_POLYGON_EXCLUSION;
        assert!(fence_from_items(&mixed).unwrap_err().contains("Polygon type changed"));
        assert!(fence_from_items(&[items[0].clone(), items[6].clone()]).unwrap_err().contains("Incomplete polygon"));
        assert!(fence_from_items(&[plain(16, 0, [0.0; 7])]).unwrap_err().contains("Unsupported command 16"));
        let points = vec![(47.0, 8.0, 50.0), (47.1, 8.1, 60.0)];
        assert_eq!(rally_from_items(&rally_items(&points)), points);
        let mut fence_transfer = Transfer::new(true, PLAN_FENCE);
        assert!(fence_transfer.write(vec![items[0].clone(), items[1].clone(), items[2].clone()]).contains(&Out::SendCount(3)), "a fence keeps its first item on ArduPilot");
        let mut rally = Transfer::new(true, PLAN_RALLY);
        assert!(rally.on_item(waypoint(0, 47.0)).is_empty(), "an unrequested rally item is not a home position");
    }
}
