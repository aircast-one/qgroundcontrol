use crate::modes::{self, AUTOPILOT_ARDUPILOT, AUTOPILOT_PX4, FLAG_CUSTOM, VehicleClass};

pub const CMD_NAV_TAKEOFF: u16 = 22;
pub const CMD_DO_SET_MODE: u16 = 176;
pub const CMD_DO_CHANGE_SPEED: u16 = 178;
pub const CMD_DO_REPOSITION: u16 = 192;
pub const CMD_DO_GO_AROUND: u16 = 191;
pub const CMD_DO_SET_ROI_NONE: u16 = 197;
pub const CMD_DO_GRIPPER: u16 = 211;
pub const CMD_MISSION_START: u16 = 300;
pub const CMD_DO_VTOL_TRANSITION: u16 = 3000;
pub const CMD_DO_DIGICAM_CONTROL: u16 = 203;
pub const CMD_DO_SET_MISSION_CURRENT: u16 = 224;
pub const CMD_DO_SET_HOME: u16 = 179;
pub const CMD_PREFLIGHT_STORAGE: u16 = 245;
pub const CMD_PREFLIGHT_REBOOT_SHUTDOWN: u16 = 246;
pub const CMD_DO_SET_ROI_LOCATION: u16 = 195;
pub const CMD_CONDITION_YAW: u16 = 115;
pub const FRAME_GLOBAL_RELATIVE_ALT: u8 = 3;
pub const SET_HOME_TERRAIN_MIN: f64 = -500.0;
pub const SET_HOME_TERRAIN_MAX: f64 = 10000.0;
pub const APM_ROI_ALTITUDE_LIMIT: f64 = 83000.0;
pub const CMD_DO_ORBIT: u16 = 34;
pub const CMD_DO_SET_GLOBAL_ORIGIN: u16 = 611;
pub const ORBIT_YAW_BEHAVIOUR_UNCHANGED: f64 = 5.0;
pub const VTOL_STATE_MC: u8 = 3;
pub const VTOL_STATE_FW: u8 = 4;
pub const MOTOR_TEST_THROTTLE_PERCENT: f64 = 0.0;
pub const MOTOR_TEST_ORDER_BOARD: f64 = 2.0;
pub const CMD_COMPONENT_ARM_DISARM: u16 = 400;
pub const REPOSITION_CHANGE_MODE: f64 = 1.0;
pub const FRAME_GLOBAL: u8 = 0;
pub const FRAME_LOCAL_OFFSET_NED: u8 = 7;
pub const CAP_COMMAND_INT: u64 = 8;
pub const ARM_MAGIC: f64 = 21196.0;

#[derive(Debug, Clone, PartialEq)]
pub struct VehicleState {
    pub autopilot: u8,
    pub vehicle_type: u8,
    pub base_mode: u8,
    pub flight_mode: String,
    pub armed: bool,
    pub altitude_amsl: Option<f64>,
    pub altitude_relative: Option<f64>,
    pub home_altitude: Option<f64>,
    pub capabilities: u64,
    pub reposition_supported: Option<bool>,
    pub minimum_takeoff_altitude: f64,
    pub current_heading: Option<f64>,
    pub announced_modes: Vec<(String, u32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Command { command: u16, params: [f64; 7], command_int: bool, frame: u8, show_error: bool },
    SetMode { mode: String, base_mode: u8, custom_mode: u32, via_command: bool },
    WaitForMode(String),
    Arm,
    WaitArmed,
    PositionTargetLocalNed { frame: u8, type_mask: u16, x: f64, y: f64, z: f64 },
    GuidedMissionItem { latitude: f64, longitude: f64, altitude_relative: f64 },
    SkipIfNoDelta,
    AwaitAccepted { command: u16, failure: Option<String> },
    FailWith { mode: String, arm: String },
}

fn failing_with(mode: &str, arm: &str, steps: Vec<Step>) -> Vec<Step> {
    std::iter::once(Step::FailWith { mode: mode.to_string(), arm: arm.to_string() }).chain(steps).collect()
}

#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    Steps(Vec<Step>),
    Refused(String),
}

fn nan() -> f64 {
    f64::NAN
}

pub fn set_mode(state: &VehicleState, mode: &str) -> Option<Vec<Step>> {
    let custom = state.announced_modes.iter().find(|(name, _)| name == mode).map(|(_, custom)| *custom).or_else(|| modes::custom_mode_for(state.autopilot, state.vehicle_type, mode))?;
    let base = (state.base_mode & !FLAG_CUSTOM) | FLAG_CUSTOM;
    let via_command = state.autopilot == AUTOPILOT_ARDUPILOT;
    Some(vec![Step::SetMode { mode: mode.to_string(), base_mode: base, custom_mode: custom, via_command }, Step::WaitForMode(mode.to_string())])
}

fn mode_or_refuse(state: &VehicleState, mode: &str) -> Result<Vec<Step>, String> {
    set_mode(state, mode).ok_or_else(|| format!("{mode} is not a mode of this vehicle"))
}

pub fn pause_mode(state: &VehicleState) -> &'static str {
    match (state.autopilot, modes::vehicle_class(state.vehicle_type)) {
        (AUTOPILOT_PX4, _) => "Hold",
        (_, VehicleClass::FixedWing) => "Loiter",
        (_, VehicleClass::Rover) => "Hold",
        _ => "Brake",
    }
}

pub fn pause(state: &VehicleState) -> Plan {
    match state.autopilot {
        AUTOPILOT_PX4 => Plan::Steps(vec![Step::Command { command: CMD_DO_REPOSITION, params: [-1.0, REPOSITION_CHANGE_MODE, 0.0, nan(), nan(), nan(), nan()], command_int: false, frame: FRAME_GLOBAL, show_error: true }]),
        _ => match mode_or_refuse(state, pause_mode(state)) {
            Ok(steps) => Plan::Steps(steps),
            Err(reason) => Plan::Refused(reason),
        },
    }
}

pub fn rtl(state: &VehicleState, smart: bool) -> Plan {
    let mode = match (state.autopilot, smart) {
        (AUTOPILOT_PX4, _) => "Return",
        (_, true) => "Smart RTL",
        (_, false) => "RTL",
    };
    match mode_or_refuse(state, mode) {
        Ok(steps) => Plan::Steps(steps),
        Err(reason) => Plan::Refused(reason),
    }
}

pub fn land(state: &VehicleState) -> Plan {
    match mode_or_refuse(state, "Land") {
        Ok(steps) => Plan::Steps(steps),
        Err(reason) => Plan::Refused(reason),
    }
}

pub fn guided_takeoff_with_altitude(state: &VehicleState) -> bool {
    matches!(state.autopilot, AUTOPILOT_PX4 | AUTOPILOT_ARDUPILOT) && (modes::vehicle_class(state.vehicle_type) == VehicleClass::MultiRotor || matches!(state.vehicle_type, 19..=25))
}

pub fn start_takeoff(state: &VehicleState, flying: bool) -> Plan {
    let mode_then_arm = |refusal: &str, arm: &str| match set_mode(state, "Takeoff") {
        Some(steps) => Plan::Steps(failing_with(refusal, arm, steps.into_iter().chain([Step::Arm]).collect())),
        None => Plan::Refused(refusal.to_string()),
    };
    match (state.autopilot, flying, state.armed) {
        (AUTOPILOT_ARDUPILOT, true, _) => Plan::Refused("Unable to start takeoff: Vehicle is already in the air.".into()),
        (AUTOPILOT_ARDUPILOT, false, true) => Plan::Steps(Vec::new()),
        (AUTOPILOT_ARDUPILOT, false, false) => mode_then_arm("Unable to start takeoff: Vehicle failed to change to Takeoff mode.", "Unable to start takeoff: Vehicle failed to arm."),
        _ => mode_then_arm("Unable to start takeoff: Vehicle not changing to Takeoff flight mode.", "Unable to start takeoff: Vehicle rejected arming."),
    }
}

pub fn takeoff(state: &VehicleState, altitude_relative: f64) -> Plan {
    let Some(amsl) = state.altitude_amsl.filter(|a| a.is_finite()) else { return Plan::Refused("Unable to takeoff, vehicle position not known.".into()) };
    match state.autopilot {
        AUTOPILOT_PX4 => Plan::Steps(vec![
            Step::Command { command: CMD_NAV_TAKEOFF, params: [nan(), nan(), 0.0, nan(), nan(), nan(), altitude_relative + amsl], command_int: false, frame: FRAME_GLOBAL, show_error: true },
            Step::AwaitAccepted { command: CMD_NAV_TAKEOFF, failure: None },
            Step::Arm,
        ]),
        AUTOPILOT_ARDUPILOT => {
            let class = modes::vehicle_class(state.vehicle_type);
            if class != VehicleClass::MultiRotor && !matches!(state.vehicle_type, 19..=25) {
                return Plan::Refused("Vehicle does not support guided takeoff".into());
            }
            let altitude = if altitude_relative.is_finite() && altitude_relative > state.minimum_takeoff_altitude { altitude_relative } else { state.minimum_takeoff_altitude };
            let guided = match mode_or_refuse(state, "Guided") {
                Ok(steps) => steps,
                Err(reason) => return Plan::Refused(reason),
            };
            let steps = guided.into_iter().chain([Step::Arm, Step::WaitArmed, Step::Command { command: CMD_NAV_TAKEOFF, params: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, altitude], command_int: false, frame: FRAME_GLOBAL, show_error: true }]).collect();
            Plan::Steps(failing_with("Unable to takeoff: Vehicle failed to change to Guided mode.", "Unable to takeoff: Vehicle failed to arm.", steps))
        }
        _ => Plan::Refused("Vehicle does not support guided takeoff".into()),
    }
}

pub fn set_current_mission(state: &VehicleState, sequence: f64) -> Plan {
    if !sequence.is_finite() || sequence < 1.0 || sequence.fract() != 0.0 {
        return Plan::Refused("A waypoint is chosen by its sequence number, from 1.".into());
    }
    let sent = if state.autopilot == AUTOPILOT_ARDUPILOT { sequence } else { sequence - 1.0 };
    Plan::Steps(vec![Step::Command { command: CMD_DO_SET_MISSION_CURRENT, params: [sent, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], command_int: false, frame: FRAME_GLOBAL, show_error: true }])
}

pub fn orbit(state: &VehicleState, latitude: f64, longitude: f64, radius: f64, amsl: f64) -> Plan {
    let command_int = state.capabilities & CAP_COMMAND_INT != 0;
    Plan::Steps(vec![Step::Command { command: CMD_DO_ORBIT, params: [radius, nan(), ORBIT_YAW_BEHAVIOUR_UNCHANGED, nan(), latitude, longitude, amsl], command_int, frame: FRAME_GLOBAL, show_error: true }])
}

pub fn estimator_origin(latitude: f64, longitude: f64, altitude: f64) -> Plan {
    Plan::Steps(vec![Step::Command { command: CMD_DO_SET_GLOBAL_ORIGIN, params: [0.0, 0.0, 0.0, 0.0, latitude, longitude, altitude], command_int: true, frame: FRAME_GLOBAL, show_error: false }])
}

pub fn reset_parameters() -> Plan {
    Plan::Steps(vec![Step::Command { command: CMD_PREFLIGHT_STORAGE, params: [2.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0], command_int: false, frame: FRAME_GLOBAL, show_error: true }])
}

pub fn set_home(latitude: f64, longitude: f64, terrain_amsl: Option<f64>) -> Plan {
    match terrain_amsl {
        None => Plan::Refused("Set Home failed, terrain data not available for selected coordinate".into()),
        Some(height) if !(SET_HOME_TERRAIN_MIN..=SET_HOME_TERRAIN_MAX).contains(&height) => Plan::Refused("Set Home failed, the terrain height there is out of range".into()),
        Some(height) => Plan::Steps(vec![Step::Command { command: CMD_DO_SET_HOME, params: [0.0, 0.0, 0.0, nan(), latitude, longitude, height], command_int: false, frame: FRAME_GLOBAL, show_error: true }]),
    }
}

pub fn roi(state: &VehicleState, latitude: f64, longitude: f64, altitude: f64, frame: u8) -> Plan {
    if state.autopilot != AUTOPILOT_PX4 && altitude.abs() >= APM_ROI_ALTITUDE_LIMIT {
        return Plan::Refused("That ROI altitude is beyond what ArduPilot accepts.".into());
    }
    let command_int = state.capabilities & CAP_COMMAND_INT != 0;
    Plan::Steps(vec![Step::Command { command: CMD_DO_SET_ROI_LOCATION, params: [nan(), nan(), nan(), nan(), latitude, longitude, altitude], command_int, frame, show_error: true }])
}

pub fn initial_bearing(from: (f64, f64), to: (f64, f64)) -> f64 {
    let (lat1, lat2) = (from.0.to_radians(), to.0.to_radians());
    let delta = (to.1 - from.1).to_radians();
    let y = delta.sin() * lat2.cos();
    let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * delta.cos();
    y.atan2(x).to_degrees().rem_euclid(360.0)
}

pub fn change_heading(state: &VehicleState, vehicle_at: Option<(f64, f64)>, target: (f64, f64), max_yaw_rate: Option<f64>) -> Plan {
    let Some(from) = vehicle_at else { return Plan::Refused("The vehicle position is not known, so there is no heading to the point.".into()) };
    let bearing = initial_bearing(from, target);
    match state.autopilot {
        AUTOPILOT_PX4 => Plan::Steps(vec![Step::Command { command: CMD_DO_REPOSITION, params: [-1.0, REPOSITION_CHANGE_MODE, 0.0, bearing.to_radians(), nan(), nan(), nan()], command_int: false, frame: FRAME_GLOBAL, show_error: true }]),
        AUTOPILOT_ARDUPILOT => {
            let current = state.current_heading.unwrap_or(0.0);
            let raw = bearing - current;
            let diff = if raw < -180.0 { raw + 360.0 } else if raw > 180.0 { raw - 360.0 } else { raw };
            let direction = if diff > 0.0 { 1.0 } else { -1.0 };
            Plan::Steps(vec![Step::Command { command: CMD_CONDITION_YAW, params: [diff.abs(), max_yaw_rate.unwrap_or(0.0), direction, 1.0, 0.0, 0.0, 0.0], command_int: false, frame: FRAME_GLOBAL, show_error: true }])
        }
        _ => Plan::Refused("Vehicle does not support guided rotate".into()),
    }
}

const DEFAULT_MAX_GOTO_METERS: f64 = 1000.0;

pub fn max_goto_meters() -> f64 {
    crate::settingsstore::raw_setting("settings.flyViewSettings.maxGoToLocationDistance").and_then(|v| v.as_f64()).unwrap_or(DEFAULT_MAX_GOTO_METERS)
}

pub fn too_far_refusal(from: Option<(f64, f64)>, to: (f64, f64), max_meters: f64) -> Option<String> {
    let distance = crate::surveygrid::distance_between(from?, to);
    (distance > max_meters).then(|| {
        let unit = crate::units::cooking("m");
        let shown = unit.map_or(max_meters, |u| (u.shown)(max_meters));
        format!("New location is too far. Must be less than {} {}.", shown.round() as i64, unit.map_or("m", |u| u.name))
    })
}

fn loiter_direction(radius: f64) -> f64 {
    match radius {
        r if r > 0.0 => 0.0,
        r if r < 0.0 => 1.0,
        _ => f64::NAN,
    }
}

pub fn goto(state: &VehicleState, latitude: f64, longitude: f64, loiter_radius: f64) -> Plan {
    match state.autopilot {
        AUTOPILOT_PX4 => {
            let Some(amsl) = state.altitude_amsl.filter(|a| a.is_finite()) else { return Plan::Refused("Unable to go to location, vehicle position not known.".into()) };
            let command_int = state.capabilities & CAP_COMMAND_INT != 0;
            Plan::Steps(vec![Step::Command { command: CMD_DO_REPOSITION, params: [-1.0, REPOSITION_CHANGE_MODE, 0.0, nan(), latitude, longitude, amsl], command_int, frame: FRAME_GLOBAL, show_error: true }])
        }
        AUTOPILOT_ARDUPILOT => {
            let Some(relative) = state.altitude_relative.filter(|a| a.is_finite()) else { return Plan::Refused("Unable to go to location, vehicle position not known.".into()) };
            let amsl = state.altitude_amsl.unwrap_or(f64::NAN);
            let mut steps = Vec::new();
            if state.reposition_supported != Some(false) {
                steps.push(Step::Command { command: CMD_DO_REPOSITION, params: [-1.0, REPOSITION_CHANGE_MODE, loiter_radius.abs(), loiter_direction(loiter_radius), latitude, longitude, amsl], command_int: true, frame: FRAME_GLOBAL, show_error: false });
            }
            if state.reposition_supported != Some(true) {
                match mode_or_refuse(state, "Guided") {
                    Ok(mode_steps) => steps.extend(mode_steps),
                    Err(reason) => return Plan::Refused(reason),
                }
                steps.push(Step::GuidedMissionItem { latitude, longitude, altitude_relative: relative });
            }
            Plan::Steps(steps)
        }
        _ => Plan::Refused("Vehicle does not support guided goto".into()),
    }
}

const PAUSE_FAILED: &str = "Unable to pause vehicle.";

pub fn change_altitude(state: &VehicleState, delta: f64, pause_first: bool) -> Plan {
    match state.autopilot {
        AUTOPILOT_PX4 => {
            let Some(home) = state.home_altitude.filter(|a| a.is_finite()) else { return Plan::Refused("Unable to change altitude, home position altitude unknown.".into()) };
            let Some(relative) = state.altitude_relative.filter(|a| a.is_finite()) else { return Plan::Refused("Unable to change altitude, vehicle altitude not known.".into()) };
            let target = home + relative + delta;
            let mut steps = Vec::new();
            if pause_first {
                steps.push(Step::Command { command: CMD_DO_REPOSITION, params: [-1.0, REPOSITION_CHANGE_MODE, 0.0, nan(), nan(), nan(), nan()], command_int: false, frame: FRAME_GLOBAL, show_error: false });
                steps.push(Step::AwaitAccepted { command: CMD_DO_REPOSITION, failure: Some(PAUSE_FAILED.into()) });
            }
            steps.push(Step::Command { command: CMD_DO_REPOSITION, params: [-1.0, REPOSITION_CHANGE_MODE, 0.0, nan(), nan(), nan(), target], command_int: false, frame: FRAME_GLOBAL, show_error: true });
            Plan::Steps(steps)
        }
        AUTOPILOT_ARDUPILOT if modes::vehicle_class(state.vehicle_type) == VehicleClass::Rover => Plan::Refused("Change altitude not supported.".into()),
        AUTOPILOT_ARDUPILOT => {
            if state.altitude_relative.is_none_or(|a| !a.is_finite()) {
                return Plan::Refused("Unable to change altitude, vehicle altitude not known.".into());
            }
            let mut steps = Vec::new();
            if pause_first {
                match mode_or_refuse(state, pause_mode(state)) {
                    Ok(mode_steps) => steps.extend(failing_with(PAUSE_FAILED, "", mode_steps)),
                    Err(reason) => return Plan::Refused(reason),
                }
                steps.push(Step::FailWith { mode: "Unable to change to Guided mode.".into(), arm: String::new() });
            }
            if delta.abs() < 0.01 {
                steps.push(Step::SkipIfNoDelta);
                return Plan::Steps(steps);
            }
            match mode_or_refuse(state, "Guided") {
                Ok(mode_steps) => steps.extend(mode_steps),
                Err(reason) => return Plan::Refused(reason),
            }
            steps.push(Step::PositionTargetLocalNed { frame: FRAME_LOCAL_OFFSET_NED, type_mask: 0xFFF8, x: 0.0, y: 0.0, z: -delta });
            Plan::Steps(steps)
        }
        _ => Plan::Refused("Vehicle does not support guided altitude change".into()),
    }
}

pub fn change_speed(ground: bool, metres_per_second: f64) -> Plan {
    Plan::Steps(vec![Step::Command { command: CMD_DO_CHANGE_SPEED, params: [if ground { 1.0 } else { 0.0 }, metres_per_second, -1.0, 0.0, nan(), nan(), nan()], command_int: false, frame: FRAME_GLOBAL, show_error: true }])
}

pub fn arm(arm: bool, force: bool) -> Step {
    Step::Command { command: CMD_COMPONENT_ARM_DISARM, params: [if arm { 1.0 } else { 0.0 }, if force { ARM_MAGIC } else { 0.0 }, 0.0, 0.0, 0.0, 0.0, 0.0], command_int: false, frame: FRAME_GLOBAL, show_error: true }
}

pub fn start_mission(state: &VehicleState, flying: bool) -> Plan {
    let arm = |mut steps: Vec<Step>| {
        steps.extend([Step::Arm, Step::WaitArmed]);
        steps
    };
    const APM_ARM: &str = "Unable to start mission: Vehicle failed to arm.";
    const APM_AUTO: &str = "Unable to start mission: Vehicle failed to change to Auto mode.";
    let plan = |mode: &str, arm_text: &str, steps: Result<Vec<Step>, String>| steps.map(|steps| failing_with(mode, arm_text, steps)).map_or_else(Plan::Refused, Plan::Steps);
    match state.autopilot {
        AUTOPILOT_PX4 => plan("Unable to start mission: Vehicle not changing to Mission flight mode.", "Unable to start mission: Vehicle rejected arming.", mode_or_refuse(state, "Mission").map(arm)),
        AUTOPILOT_ARDUPILOT if flying => plan(APM_AUTO, APM_ARM, mode_or_refuse(state, "Auto")),
        AUTOPILOT_ARDUPILOT if modes::vehicle_class(state.vehicle_type) == VehicleClass::FixedWing => plan(APM_AUTO, APM_ARM, mode_or_refuse(state, "Auto").map(arm)),
        AUTOPILOT_ARDUPILOT => {
            let armed = match state.armed {
                true => Ok(vec![]),
                false => mode_or_refuse(state, "Guided").map(arm),
            };
            plan(
                "Unable to start mission: Vehicle failed to change to Guided mode.",
                APM_ARM,
                armed.map(|mut steps| {
                    steps.push(Step::Command { command: CMD_MISSION_START, params: [0.0; 7], command_int: false, frame: FRAME_GLOBAL, show_error: true });
                    steps
                }),
            )
        }
        _ => Plan::Refused("Vehicle does not support starting a mission".into()),
    }
}

pub fn emergency_stop() -> Plan {
    Plan::Steps(vec![arm(false, true)])
}

pub fn abort_landing(climb_out: f64) -> Plan {
    match climb_out.is_finite() {
        true => Plan::Steps(vec![Step::Command { command: CMD_DO_GO_AROUND, params: [climb_out, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], command_int: false, frame: FRAME_GLOBAL, show_error: true }]),
        false => Plan::Refused("Abort Landing needs a climb-out height.".into()),
    }
}

pub fn gripper(action: f64) -> Plan {
    match action {
        0.0..=2.0 if action.fract() == 0.0 => Plan::Steps(vec![Step::Command { command: CMD_DO_GRIPPER, params: [0.0, action, 0.0, 0.0, 0.0, 0.0, 0.0], command_int: false, frame: FRAME_GLOBAL, show_error: true }]),
        _ => Plan::Refused("The gripper is sent 1 to grab, 0 to release or 2 to hold.".into()),
    }
}

pub const CMD_AIRFRAME_CONFIGURATION: u16 = 2520;
pub const CMD_DO_AUX_FUNCTION: u16 = 218;
const APM_AUX_MOTOR_INTERLOCK: f64 = 32.0;
const AUX_SWITCH_HIGH: f64 = 2.0;
const AUX_SWITCH_LOW: f64 = 0.0;
const ALL_GEARS: f64 = -1.0;

pub fn landing_gear(retract: bool) -> Plan {
    Plan::Steps(vec![Step::Command { command: CMD_AIRFRAME_CONFIGURATION, params: [ALL_GEARS, if retract { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0, 0.0, 0.0], command_int: false, frame: FRAME_GLOBAL, show_error: true }])
}

pub fn motor_interlock(state: &VehicleState, enable: bool) -> Plan {
    match state.autopilot == crate::modes::AUTOPILOT_ARDUPILOT {
        true => Plan::Steps(vec![Step::Command { command: CMD_DO_AUX_FUNCTION, params: [APM_AUX_MOTOR_INTERLOCK, if enable { AUX_SWITCH_HIGH } else { AUX_SWITCH_LOW }, 0.0, 0.0, 0.0, 0.0, 0.0], command_int: false, frame: FRAME_GLOBAL, show_error: true }]),
        false => Plan::Steps(Vec::new()),
    }
}

pub fn trigger_camera() -> Plan {
    Plan::Steps(vec![Step::Command { command: CMD_DO_DIGICAM_CONTROL, params: [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0], command_int: false, frame: FRAME_GLOBAL, show_error: true }])
}

pub fn motor_test(motor: f64, percent: f64, seconds: f64) -> Plan {
    Plan::Steps(vec![Step::Command { command: crate::mavcmd::CMD_DO_MOTOR_TEST, params: [motor, MOTOR_TEST_THROTTLE_PERCENT, percent, seconds, 0.0, MOTOR_TEST_ORDER_BOARD, 0.0], command_int: false, frame: FRAME_GLOBAL, show_error: true }])
}

pub fn vtol_transition(forward: bool) -> Plan {
    let state = f64::from(if forward { VTOL_STATE_FW } else { VTOL_STATE_MC });
    Plan::Steps(vec![Step::Command { command: CMD_DO_VTOL_TRANSITION, params: [state, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], command_int: false, frame: FRAME_GLOBAL, show_error: true }])
}

pub fn cancel_roi(state: &VehicleState) -> Plan {
    let command_int = state.capabilities & CAP_COMMAND_INT != 0;
    Plan::Steps(vec![Step::Command { command: CMD_DO_SET_ROI_NONE, params: [nan(); 7], command_int, frame: FRAME_GLOBAL, show_error: true }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_goto_beyond_the_fly_view_limit_is_refused_in_the_app_distance_unit() {
        let from = Some((47.0, 8.0));
        assert_eq!(too_far_refusal(from, (47.005, 8.0), 1000.0), None, "556 m is inside a 1000 m limit");
        assert_eq!(too_far_refusal(from, (47.01, 8.0), 1000.0).as_deref(), Some("New location is too far. Must be less than 1000 m."));
        assert_eq!(too_far_refusal(None, (47.01, 8.0), 1000.0), None, "with no vehicle position the goto refuses on its own");
    }

    fn px4() -> VehicleState {
        VehicleState { autopilot: AUTOPILOT_PX4, vehicle_type: 2, base_mode: 0x81, flight_mode: "Position".into(), armed: true, altitude_amsl: Some(500.0), altitude_relative: Some(20.0), home_altitude: Some(480.0), capabilities: CAP_COMMAND_INT, reposition_supported: None, minimum_takeoff_altitude: 2.5, current_heading: Some(90.0), announced_modes: Vec::new() }
    }

    fn copter() -> VehicleState {
        VehicleState { autopilot: AUTOPILOT_ARDUPILOT, vehicle_type: 2, base_mode: 0x81, flight_mode: "Loiter".into(), armed: false, altitude_amsl: Some(500.0), altitude_relative: Some(20.0), home_altitude: Some(480.0), capabilities: 0, reposition_supported: None, minimum_takeoff_altitude: 2.5, current_heading: None, announced_modes: Vec::new() }
    }

    fn command(step: &Step) -> (u16, [f64; 7], bool) {
        match step {
            Step::Command { command, params, command_int, .. } => (*command, *params, *command_int),
            other => panic!("not a command: {other:?}"),
        }
    }

    fn modes_of(plan: Plan) -> Vec<String> {
        let Plan::Steps(steps) = plan else { panic!("refused") };
        steps.iter().filter(|step| !matches!(step, Step::FailWith { .. })).map(|step| match step {
            Step::SetMode { mode, .. } => format!("mode {mode}"),
            Step::WaitForMode(mode) => format!("wait {mode}"),
            Step::Arm => "arm".into(),
            Step::WaitArmed => "armed".into(),
            Step::Command { command, .. } => format!("cmd {command}"),
            other => format!("{other:?}"),
        }).collect()
    }

    #[test]
    fn a_mode_the_vehicle_announces_is_set_by_its_announced_number() {
        let external = VehicleState { announced_modes: vec![("MyMode".into(), 385_875_968)], ..px4() };
        let steps = set_mode(&external, "MyMode").expect("FirmwarePlugin::updateAvailableFlightModes rebuilds the name table from AVAILABLE_MODES");
        assert!(matches!(steps[0], Step::SetMode { custom_mode: 385_875_968, .. }));
    }

    #[test]
    fn an_ardurover_refuses_an_altitude_change_as_its_plugin_does() {
        let rover = VehicleState { vehicle_type: 10, ..copter() };
        assert_eq!(change_altitude(&rover, 10.0, false), Plan::Refused("Change altitude not supported.".into()), "ArduRoverFirmwarePlugin::guidedModeChangeAltitude");
    }

    #[test]
    fn a_mission_starts_the_way_each_firmware_plugin_starts_it() {
        assert_eq!(modes_of(start_mission(&px4(), false)), ["mode Mission", "wait Mission", "arm", "armed"]);
        assert_eq!(modes_of(start_mission(&copter(), true)), ["mode Auto", "wait Auto"], "in the air ArduPilot only switches to Auto");
        assert_eq!(modes_of(start_mission(&copter(), false)), ["mode Guided", "wait Guided", "arm", "armed", "cmd 300"], "a copter on the ground arms in Guided and is sent MISSION_START");
        assert_eq!(modes_of(start_mission(&VehicleState { armed: true, ..copter() }, false)), ["cmd 300"], "already armed, it is not switched to Guided first");
        let plane = VehicleState { vehicle_type: 1, ..copter() };
        assert_eq!(modes_of(start_mission(&plane, false)), ["mode Auto", "wait Auto", "arm", "armed"], "a plane is put in Auto before arming, never armed in Guided");
        let tilt_rotor = VehicleState { vehicle_type: 21, ..copter() };
        assert_eq!(modes_of(start_mission(&tilt_rotor, false)), ["mode Auto", "wait Auto", "arm", "armed"], "arming a VTOL in Guided would arm its rotors in forward-flight position");
    }

    #[test]
    fn the_single_command_actions_match_what_qt_sends() {
        assert_eq!(modes_of(emergency_stop()), ["cmd 400"]);
        let Plan::Steps(steps) = emergency_stop() else { panic!() };
        assert_eq!(command(&steps[0]).1[..2], [0.0, ARM_MAGIC]);
        let Plan::Steps(steps) = abort_landing(30.0) else { panic!() };
        assert_eq!((command(&steps[0]).0, command(&steps[0]).1[0]), (CMD_DO_GO_AROUND, 30.0));
        let Plan::Steps(steps) = gripper(1.0) else { panic!() };
        assert_eq!((command(&steps[0]).0, command(&steps[0]).1[..2].to_vec()), (CMD_DO_GRIPPER, vec![0.0, 1.0]));
        let Plan::Steps(steps) = motor_test(3.0, 20.0, 5.0) else { panic!("a motor test is always sent") };
        assert_eq!(command(&steps[0]).1, [3.0, 0.0, 20.0, 5.0, 0.0, 2.0, 0.0], "Vehicle::motorTest sends a throttle percent in board order");
        let Plan::Steps(steps) = vtol_transition(true) else { panic!("a transition is always sent") };
        assert_eq!((command(&steps[0]).0, command(&steps[0]).1[0]), (CMD_DO_VTOL_TRANSITION, f64::from(VTOL_STATE_FW)), "setVtolInFwdFlight sends MAV_VTOL_STATE_FW for forward flight");
        assert!(matches!(gripper(2.0), Plan::Steps(_)), "GRIPPER_ACTION_HOLD is 2, which the joystick's Gripper Hold sends");
        assert!(matches!(gripper(3.0), Plan::Refused(_)));
        let Plan::Steps(steps) = landing_gear(true) else { panic!() };
        assert_eq!(command(&steps[0]), (CMD_AIRFRAME_CONFIGURATION, [-1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], false), "Vehicle::landingGearRetract: all gears, up");
        let Plan::Steps(steps) = set_current_mission(&px4(), 3.0) else { panic!() };
        assert_eq!(command(&steps[0]).0, CMD_DO_SET_MISSION_CURRENT);
        assert!(matches!(steps[0], Step::Command { params, .. } if params[0] == 2.0), "PX4 does not count home, so Vehicle::setCurrentMissionSequence steps back one");
        let Plan::Steps(steps) = set_current_mission(&copter(), 3.0) else { panic!() };
        assert!(matches!(steps[0], Step::Command { params, .. } if params[0] == 3.0), "ArduPilot keeps home as item 0");
        assert!(matches!(set_current_mission(&copter(), 0.0), Plan::Refused(_)));
        let Plan::Steps(steps) = orbit(&px4(), 47.4, 8.5, -30.0, 520.0) else { panic!() };
        assert!(matches!(steps[0], Step::Command { command: CMD_DO_ORBIT, params, .. } if params[0] == -30.0 && params[2] == ORBIT_YAW_BEHAVIOUR_UNCHANGED && params[6] == 520.0));
        let Plan::Steps(steps) = estimator_origin(47.4, 8.5, 480.0) else { panic!() };
        assert!(matches!(steps[0], Step::Command { command: CMD_DO_SET_GLOBAL_ORIGIN, command_int: true, .. }));
        assert!(matches!(&reset_parameters(), Plan::Steps(s) if matches!(s[0], Step::Command { command: CMD_PREFLIGHT_STORAGE, params, .. } if params[0] == 2.0 && params[1] == -1.0)), "reset params to default, leave mission storage alone");
        assert!((initial_bearing((47.0, 8.0), (48.0, 8.0)) - 0.0).abs() < 1e-9);
        assert!((initial_bearing((0.0, 0.0), (0.0, 1.0)) - 90.0).abs() < 1e-9);
        let facing = |heading: f64| VehicleState { current_heading: Some(heading), ..copter() };
        let Plan::Steps(steps) = change_heading(&facing(350.0), Some((0.0, 0.0)), (0.0, 1.0), Some(90.0)) else { panic!() };
        assert!(matches!(steps[0], Step::Command { command: CMD_CONDITION_YAW, params, .. } if (params[0] - 100.0).abs() < 1e-9 && params[1] == 90.0 && params[2] == 1.0 && params[3] == 1.0), "ArduPilot turns the short way, relative, at ATC_RATE_Y_MAX");
        let Plan::Steps(steps) = change_heading(&px4(), Some((0.0, 0.0)), (0.0, 1.0), None) else { panic!() };
        assert!(matches!(steps[0], Step::Command { command: CMD_DO_REPOSITION, params, .. } if (params[3] - std::f64::consts::FRAC_PI_2).abs() < 1e-9));
        assert!(matches!(set_home(47.0, 8.0, None), Plan::Refused(_)), "QGC refuses a home with no terrain height under it");
        assert!(matches!(set_home(47.0, 8.0, Some(20000.0)), Plan::Refused(_)));
        let Plan::Steps(steps) = set_home(47.0, 8.0, Some(410.0)) else { panic!() };
        assert!(matches!(steps[0], Step::Command { command: CMD_DO_SET_HOME, params, .. } if params[6] == 410.0));
        let Plan::Steps(steps) = roi(&copter(), 47.0, 8.0, 0.0, FRAME_GLOBAL_RELATIVE_ALT) else { panic!() };
        assert!(matches!(steps[0], Step::Command { command: CMD_DO_SET_ROI_LOCATION, frame: FRAME_GLOBAL_RELATIVE_ALT, .. }));
        assert!(matches!(roi(&copter(), 47.0, 8.0, 90000.0, FRAME_GLOBAL_RELATIVE_ALT), Plan::Refused(_)));
        let Plan::Steps(steps) = cancel_roi(&px4()) else { panic!() };
        assert_eq!((command(&steps[0]).0, command(&steps[0]).2), (CMD_DO_SET_ROI_NONE, true));
        let Plan::Steps(steps) = cancel_roi(&copter()) else { panic!() };
        assert!(!command(&steps[0]).2, "without the COMMAND_INT capability it goes as COMMAND_LONG, as Qt sends it");
    }

    #[test]
    fn px4_plans_use_reposition_and_amsl_altitudes() {
        let state = px4();
        let Plan::Steps(steps) = takeoff(&state, 15.0) else { panic!() };
        let (cmd, params, _) = command(&steps[0]);
        assert_eq!((cmd, params[0].is_nan(), params[2], params[6]), (CMD_NAV_TAKEOFF, true, 0.0, 515.0), "PX4FirmwarePlugin::guidedModeTakeoff sends no pitch and AMSL altitude");
        assert_eq!(&steps[1..], &[Step::AwaitAccepted { command: CMD_NAV_TAKEOFF, failure: None }, Step::Arm], "and arms once the takeoff is accepted");
        let Plan::Steps(steps) = goto(&state, 47.4, 8.5, 0.0) else { panic!() };
        let (cmd, params, int) = command(&steps[0]);
        assert_eq!((cmd, params[1], params[4], params[5], params[6], int), (CMD_DO_REPOSITION, 1.0, 47.4, 8.5, 500.0, true));
        let Plan::Steps(steps) = change_altitude(&state, 5.0, true) else { panic!() };
        assert_eq!(steps.len(), 3);
        assert!(command(&steps[0]).1[6].is_nan());
        assert_eq!(steps[1], Step::AwaitAccepted { command: CMD_DO_REPOSITION, failure: Some("Unable to pause vehicle.".into()) }, "PX4 only changes altitude once the pause is accepted");
        assert_eq!(command(&steps[2]).1[6], 505.0);
        let Plan::Steps(steps) = pause(&state) else { panic!() };
        assert_eq!(command(&steps[0]).0, CMD_DO_REPOSITION);
        assert_eq!(rtl(&state, true), Plan::Steps(vec![Step::SetMode { mode: "Return".into(), base_mode: 0x81, custom_mode: modes::px4(4, 5), via_command: false }, Step::WaitForMode("Return".into())]));
        assert!(matches!(land(&state), Plan::Steps(_)));
        let blind = VehicleState { altitude_amsl: None, ..px4() };
        assert!(matches!(takeoff(&blind, 15.0), Plan::Refused(_)));
        assert!(matches!(change_altitude(&VehicleState { home_altitude: None, ..px4() }, 1.0, false), Plan::Refused(_)));
    }

    #[test]
    fn ardupilot_plans_change_mode_first_and_use_relative_altitudes() {
        let state = copter();
        let Plan::Steps(steps) = takeoff(&state, 1.0) else { panic!() };
        assert!(matches!(&steps[0], Step::FailWith { mode, arm } if mode == "Unable to takeoff: Vehicle failed to change to Guided mode." && arm == "Unable to takeoff: Vehicle failed to arm."));
        assert!(matches!(&steps[1], Step::SetMode { mode, via_command: true, custom_mode: 4, .. } if mode == "Guided"));
        assert_eq!(steps[3], Step::Arm);
        assert_eq!(command(&steps[5]).1[6], 2.5, "the minimum takeoff altitude wins over a lower request");
        let Plan::Steps(steps) = goto(&state, 47.4, 8.5, 30.0) else { panic!() };
        assert_eq!((command(&steps[0]).1[2], command(&steps[0]).1[3]), (30.0, 0.0), "a positive radius loiters clockwise");
        let Plan::Steps(steps) = goto(&state, 47.4, 8.5, -30.0) else { panic!() };
        assert_eq!((command(&steps[0]).1[2], command(&steps[0]).1[3]), (30.0, 1.0), "a negative radius is sent as its size with the counter-clockwise flag");
        let Plan::Steps(steps) = goto(&state, 47.4, 8.5, 0.0) else { panic!() };
        assert!(command(&steps[0]).1[3].is_nan(), "no radius leaves a copter's yaw mode alone");
        assert!(matches!(steps.last(), Some(Step::GuidedMissionItem { altitude_relative, .. }) if *altitude_relative == 20.0));
        let supported = VehicleState { reposition_supported: Some(true), ..copter() };
        assert_eq!(match goto(&supported, 1.0, 2.0, 0.0) { Plan::Steps(s) => s.len(), _ => 0 }, 1);
        let unsupported = VehicleState { reposition_supported: Some(false), ..copter() };
        assert!(matches!(goto(&unsupported, 1.0, 2.0, 0.0), Plan::Steps(s) if matches!(s[0], Step::SetMode { .. })));
        let Plan::Steps(steps) = change_altitude(&state, 3.0, true) else { panic!() };
        assert!(matches!(&steps[0], Step::FailWith { mode, .. } if mode == "Unable to pause vehicle."));
        assert!(matches!(&steps[1], Step::SetMode { mode, .. } if mode == "Brake"));
        assert!(matches!(steps.last(), Some(Step::PositionTargetLocalNed { z, type_mask: 0xFFF8, .. }) if *z == -3.0));
        let Plan::Steps(hold) = change_altitude(&state, 0.0, true) else { panic!() };
        assert_eq!(hold.last(), Some(&Step::SkipIfNoDelta));
        assert!(matches!(pause(&VehicleState { vehicle_type: 1, ..copter() }), Plan::Steps(s) if matches!(&s[0], Step::SetMode { mode, .. } if mode == "Loiter")));
        assert!(matches!(rtl(&state, true), Plan::Steps(s) if matches!(&s[0], Step::SetMode { custom_mode: 21, .. })));
        assert!(matches!(takeoff(&VehicleState { vehicle_type: 10, ..copter() }, 5.0), Plan::Refused(_)));
        assert_eq!(command(&arm(true, true)).1[1], ARM_MAGIC);
        assert_eq!(command(&change_speed(true, 4.5).into_steps()[0]).1[..2], [1.0, 4.5]);
    }

    impl Plan {
        fn into_steps(self) -> Vec<Step> {
            match self {
                Plan::Steps(s) => s,
                Plan::Refused(r) => panic!("{r}"),
            }
        }
    }
    #[test]
    fn every_command_number_is_the_one_the_dialect_calls_it() {
        use mavlink::dialects::ardupilotmega::{MavCmd, MavFrame};
        use num_traits::FromPrimitive;

        [
            (CMD_NAV_TAKEOFF, MavCmd::MAV_CMD_NAV_TAKEOFF),
            (CMD_DO_SET_MODE, MavCmd::MAV_CMD_DO_SET_MODE),
            (CMD_DO_CHANGE_SPEED, MavCmd::MAV_CMD_DO_CHANGE_SPEED),
            (CMD_DO_REPOSITION, MavCmd::MAV_CMD_DO_REPOSITION),
            (CMD_COMPONENT_ARM_DISARM, MavCmd::MAV_CMD_COMPONENT_ARM_DISARM),
        ]
        .iter()
        .for_each(|(number, named)| {
            assert_eq!(
                MavCmd::from_u32(*number as u32),
                Some(*named),
                "{number} is what the core puts on the wire and {named:?} is what it means to send; the other tests here restate these constants rather than check them, so a mistyped digit would be pinned instead of caught and the aircraft would be sent a different command"
            );
        });

        [(FRAME_GLOBAL, MavFrame::MAV_FRAME_GLOBAL), (FRAME_LOCAL_OFFSET_NED, MavFrame::MAV_FRAME_LOCAL_OFFSET_NED)]
            .iter()
            .for_each(|(number, named)| {
                assert_eq!(MavFrame::from_u8(*number), Some(*named), "a wrong frame reinterprets the coordinates rather than rejecting them");
            });
    }


    #[test]
    fn a_plane_takes_off_by_mode_and_arm_as_start_takeoff_does() {
        let plane = VehicleState { autopilot: AUTOPILOT_ARDUPILOT, vehicle_type: 1, armed: false, ..px4() };
        assert!(!guided_takeoff_with_altitude(&plane), "supports.guidedTakeoffWithAltitude is multirotor or VTOL only");
        let Plan::Steps(steps) = start_takeoff(&plane, false) else { panic!("a grounded plane can start a takeoff") };
        assert!(matches!(steps.get(1), Some(Step::SetMode { mode, .. }) if mode == "Takeoff"));
        assert_eq!(steps.first(), Some(&Step::FailWith { mode: "Unable to start takeoff: Vehicle failed to change to Takeoff mode.".into(), arm: "Unable to start takeoff: Vehicle failed to arm.".into() }), "APMFirmwarePlugin::startTakeoff's own failure texts");
        assert_eq!(steps.last(), Some(&Step::Arm));
        assert!(matches!(start_takeoff(&plane, true), Plan::Refused(r) if r.contains("already in the air")));
        let px4_plane = VehicleState { vehicle_type: 1, ..px4() };
        assert!(matches!(start_takeoff(&px4_plane, false), Plan::Steps(steps) if steps.last() == Some(&Step::Arm)));
    }
}
