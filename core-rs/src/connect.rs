#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    AutopilotVersion,
    ProtocolVersion,
    ComponentInformation,
    StandardModes,
    Parameters,
    Mission,
    GeoFence,
    RallyPoints,
    Complete,
}

pub const STEPS: [Step; 9] = [
    Step::AutopilotVersion,
    Step::ProtocolVersion,
    Step::StandardModes,
    Step::ComponentInformation,
    Step::Parameters,
    Step::Mission,
    Step::GeoFence,
    Step::RallyPoints,
    Step::Complete,
];

pub const WEIGHTS: [u32; 9] = [1, 1, 1, 5, 5, 2, 1, 1, 1];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    pub present: bool,
    pub high_latency: bool,
    pub log_replay: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vehicle {
    pub px4: bool,
    pub apm: bool,
    pub fence_supported: bool,
    pub rally_supported: bool,
    pub max_proto_version: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    RequestMessage { message_id: u32 },
    RequestComponentInformation,
    RequestStandardModes,
    RefreshParameters,
    LoadMission,
    LoadGeoFence,
    LoadRallyPoints,
    FirstMissionLoadComplete,
    FirstGeoFenceLoadComplete,
    FirstRallyPointLoadComplete,
    SetCapabilities(u64),
    SetMaxProtoVersion(u32),
    Progress(f64),
    InitialConnectComplete,
}

pub const MSG_AUTOPILOT_VERSION: u32 = 148;
pub const MSG_PROTOCOL_VERSION: u32 = 300;
pub const CAP_MISSION_INT: u64 = 4;
pub const CAP_COMMAND_INT: u64 = 8;
pub const CAP_MAVLINK2: u64 = 4096;
pub const CAP_MISSION_FENCE: u64 = 8192;
pub const CAP_MISSION_RALLY: u64 = 16384;

#[derive(Debug, Clone, PartialEq)]
pub struct AutopilotVersion {
    pub capabilities: u64,
    pub flight_sw_version: u32,
    pub flight_custom_version: [u8; 8],
    pub uid: u64,
    pub vendor_id: u16,
    pub product_id: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Firmware {
    pub version: Option<(u8, u8, u8, u8)>,
    pub custom: Option<(u8, u8, u8)>,
    pub git_hash: String,
}

pub fn firmware_from(version: &AutopilotVersion, px4: bool) -> Firmware {
    let sw = version.flight_sw_version;
    let custom = version.flight_custom_version;
    let git_hash = if px4 { custom.iter().rev().map(|b| format!("{b:02x}")).collect() } else { String::from_utf8_lossy(&custom).trim_end_matches('\0').to_string() };
    Firmware {
        version: (sw != 0).then(|| (((sw >> 24) & 0xFF) as u8, ((sw >> 16) & 0xFF) as u8, ((sw >> 8) & 0xFF) as u8, (sw & 0xFF) as u8)),
        custom: px4.then_some((custom[2], custom[1], custom[0])),
        git_hash,
    }
}

pub fn assumed_capabilities(vehicle: &Vehicle) -> u64 {
    let mavlink2 = if vehicle.max_proto_version >= 200 { CAP_MAVLINK2 } else { 0 };
    let firmware = if vehicle.px4 || vehicle.apm { CAP_MISSION_INT | CAP_COMMAND_INT | CAP_MISSION_FENCE | CAP_MISSION_RALLY } else { 0 };
    mavlink2 | firmware
}

#[derive(Debug, Clone)]
pub struct Connect {
    index: usize,
    active: bool,
    total_weight: u32,
}

impl Default for Connect {
    fn default() -> Self {
        Connect { index: usize::MAX, active: false, total_weight: WEIGHTS.iter().sum() }
    }
}

impl Connect {
    pub fn current(&self) -> Option<Step> {
        self.active.then(|| STEPS.get(self.index).copied()).flatten()
    }

    pub fn progress(&self, within_step: f64) -> f64 {
        let done: u32 = WEIGHTS.iter().take(self.index.min(WEIGHTS.len())).sum();
        let current = WEIGHTS.get(self.index).copied().unwrap_or(0) as f64 * within_step.clamp(0.0, 1.0);
        (done as f64 + current) / self.total_weight as f64
    }

    pub fn start(&mut self, link: &Link, vehicle: &Vehicle) -> Vec<Action> {
        self.active = true;
        self.index = usize::MAX;
        self.advance(link, vehicle)
    }

    pub fn advance(&mut self, link: &Link, vehicle: &Vehicle) -> Vec<Action> {
        if !self.active {
            return Vec::new();
        }
        self.index = self.index.wrapping_add(1);
        let Some(step) = STEPS.get(self.index).copied() else {
            self.active = false;
            return Vec::new();
        };
        let quiet = !link.present || link.high_latency || link.log_replay;
        let mut actions = vec![Action::Progress(self.progress(0.0))];
        let chain = |mut more: Vec<Action>, actions: &mut Vec<Action>| actions.append(&mut more);
        match step {
            Step::AutopilotVersion if quiet => chain(self.advance(link, vehicle), &mut actions),
            Step::AutopilotVersion => actions.push(Action::RequestMessage { message_id: MSG_AUTOPILOT_VERSION }),
            Step::ProtocolVersion if quiet || vehicle.apm => chain(self.advance(link, vehicle), &mut actions),
            Step::ProtocolVersion => actions.push(Action::RequestMessage { message_id: MSG_PROTOCOL_VERSION }),
            Step::ComponentInformation => actions.push(Action::RequestComponentInformation),
            Step::StandardModes => actions.push(Action::RequestStandardModes),
            Step::Parameters => actions.push(Action::RefreshParameters),
            Step::Mission if !link.present => chain(self.advance(link, vehicle), &mut actions),
            Step::Mission if quiet => actions.push(Action::FirstMissionLoadComplete),
            Step::Mission => actions.push(Action::LoadMission),
            Step::GeoFence if !link.present => chain(self.advance(link, vehicle), &mut actions),
            Step::GeoFence if quiet || !vehicle.fence_supported => actions.push(Action::FirstGeoFenceLoadComplete),
            Step::GeoFence => actions.push(Action::LoadGeoFence),
            Step::RallyPoints if !link.present => chain(self.advance(link, vehicle), &mut actions),
            Step::RallyPoints if quiet || !vehicle.rally_supported => actions.push(Action::FirstRallyPointLoadComplete),
            Step::RallyPoints => actions.push(Action::LoadRallyPoints),
            Step::Complete => {
                chain(self.advance(link, vehicle), &mut actions);
                actions.push(Action::InitialConnectComplete);
            }
        }
        actions
    }

    pub fn on_autopilot_version(&mut self, link: &Link, vehicle: &Vehicle, received: Option<&AutopilotVersion>) -> Vec<Action> {
        let capabilities = match received {
            Some(version) => version.capabilities,
            None => assumed_capabilities(vehicle),
        };
        let mut actions = vec![Action::SetCapabilities(capabilities)];
        actions.append(&mut self.advance(link, vehicle));
        actions
    }

    pub fn on_protocol_version(&mut self, link: &Link, vehicle: &Vehicle, max_version: Option<u32>) -> Vec<Action> {
        let mut actions = vec![Action::SetMaxProtoVersion(max_version.unwrap_or(100))];
        actions.append(&mut self.advance(link, vehicle));
        actions
    }

    pub fn on_step_done(&mut self, link: &Link, vehicle: &Vehicle) -> Vec<Action> {
        self.advance(link, vehicle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn live() -> Link {
        Link { present: true, high_latency: false, log_replay: false }
    }

    fn px4() -> Vehicle {
        Vehicle { px4: true, apm: false, fence_supported: true, rally_supported: true, max_proto_version: 200 }
    }

    fn requests(actions: &[Action]) -> Vec<&Action> {
        actions.iter().filter(|a| !matches!(a, Action::Progress(_))).collect()
    }

    #[test]
    fn a_px4_vehicle_walks_every_step_in_order() {
        let mut connect = Connect::default();
        let (link, vehicle) = (live(), px4());
        assert_eq!(requests(&connect.start(&link, &vehicle)), vec![&Action::RequestMessage { message_id: MSG_AUTOPILOT_VERSION }]);
        let version = AutopilotVersion { capabilities: 0x1010, flight_sw_version: 0x01_0F_02_FF, flight_custom_version: [0xaa, 0xbb, 0xcc, 1, 2, 3, 4, 5], uid: 9, vendor_id: 1, product_id: 2 };
        let after_version = connect.on_autopilot_version(&link, &vehicle, Some(&version));
        assert_eq!(requests(&after_version), vec![&Action::SetCapabilities(0x1010), &Action::RequestMessage { message_id: MSG_PROTOCOL_VERSION }]);
        let after_proto = connect.on_protocol_version(&link, &vehicle, Some(200));
        assert_eq!(requests(&after_proto), vec![&Action::SetMaxProtoVersion(200), &Action::RequestStandardModes]);
        assert_eq!(requests(&connect.on_step_done(&link, &vehicle)), vec![&Action::RequestComponentInformation]);
        assert_eq!(requests(&connect.on_step_done(&link, &vehicle)), vec![&Action::RefreshParameters]);
        assert_eq!(requests(&connect.on_step_done(&link, &vehicle)), vec![&Action::LoadMission]);
        assert_eq!(requests(&connect.on_step_done(&link, &vehicle)), vec![&Action::LoadGeoFence]);
        assert_eq!(requests(&connect.on_step_done(&link, &vehicle)), vec![&Action::LoadRallyPoints]);
        let done = connect.on_step_done(&link, &vehicle);
        assert_eq!(requests(&done), vec![&Action::InitialConnectComplete]);
        assert_eq!(connect.current(), None);
        assert!(connect.on_step_done(&link, &vehicle).is_empty());
        let firmware = firmware_from(&version, true);
        assert_eq!(firmware.version, Some((1, 15, 2, 255)));
        assert_eq!(firmware.custom, Some((0xcc, 0xbb, 0xaa)));
        assert_eq!(firmware.git_hash, "0504030201ccbbaa");
    }

    #[test]
    fn ardupilot_skips_the_protocol_version_and_a_missing_answer_assumes_capabilities() {
        let mut connect = Connect::default();
        let link = live();
        let vehicle = Vehicle { px4: false, apm: true, fence_supported: false, rally_supported: true, max_proto_version: 100 };
        connect.start(&link, &vehicle);
        let after = connect.on_autopilot_version(&link, &vehicle, None);
        assert_eq!(requests(&after), vec![&Action::SetCapabilities(CAP_MISSION_INT | CAP_COMMAND_INT | CAP_MISSION_FENCE | CAP_MISSION_RALLY), &Action::RequestStandardModes]);
        connect.on_step_done(&link, &vehicle);
        connect.on_step_done(&link, &vehicle);
        connect.on_step_done(&link, &vehicle);
        assert_eq!(requests(&connect.on_step_done(&link, &vehicle)), vec![&Action::FirstGeoFenceLoadComplete]);
        assert_eq!(requests(&connect.on_step_done(&link, &vehicle)), vec![&Action::LoadRallyPoints]);
        let apm = AutopilotVersion { capabilities: 0, flight_sw_version: 0, flight_custom_version: *b"abc12345", uid: 0, vendor_id: 0, product_id: 0 };
        let unstamped = firmware_from(&apm, false);
        assert_eq!((unstamped.version, unstamped.git_hash.as_str()), (None, "abc12345"));
        let stamped = AutopilotVersion { flight_sw_version: 0x04_05_06_00, ..apm };
        assert_eq!(firmware_from(&stamped, false).version, Some((4, 5, 6, 0)));
    }

    #[test]
    fn a_replay_or_high_latency_link_answers_the_loads_itself_and_progress_is_weighted() {
        let mut connect = Connect::default();
        let link = Link { present: true, high_latency: true, log_replay: false };
        let vehicle = px4();
        let started = connect.start(&link, &vehicle);
        assert_eq!(requests(&started), vec![&Action::RequestStandardModes]);
        assert_eq!(connect.current(), Some(Step::StandardModes));
        connect.on_step_done(&link, &vehicle);
        connect.on_step_done(&link, &vehicle);
        assert_eq!(requests(&connect.on_step_done(&link, &vehicle)), vec![&Action::FirstMissionLoadComplete]);
        assert!((connect.progress(0.5) - (13.0 + 1.0) / 18.0).abs() < 1e-9);
        let mut absent = Connect::default();
        let none = Link { present: false, high_latency: false, log_replay: false };
        absent.start(&none, &vehicle);
        absent.on_step_done(&none, &vehicle);
        absent.on_step_done(&none, &vehicle);
        let end = absent.on_step_done(&none, &vehicle);
        assert_eq!(requests(&end), vec![&Action::InitialConnectComplete]);
        assert!(absent.on_step_done(&none, &vehicle).is_empty());
    }
}
