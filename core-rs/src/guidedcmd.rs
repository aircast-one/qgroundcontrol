use crate::modes::{self, AUTOPILOT_ARDUPILOT, AUTOPILOT_PX4, FLAG_CUSTOM, VehicleClass};

pub const CMD_NAV_TAKEOFF: u16 = 22;
pub const CMD_DO_SET_MODE: u16 = 176;
pub const CMD_DO_CHANGE_SPEED: u16 = 178;
pub const CMD_DO_REPOSITION: u16 = 192;
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
    let custom = modes::custom_mode_for(state.autopilot, state.vehicle_type, mode)?;
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

pub fn takeoff(state: &VehicleState, altitude_relative: f64) -> Plan {
    let Some(amsl) = state.altitude_amsl.filter(|a| a.is_finite()) else { return Plan::Refused("Unable to takeoff, vehicle position not known.".into()) };
    match state.autopilot {
        AUTOPILOT_PX4 => Plan::Steps(vec![Step::Command { command: CMD_NAV_TAKEOFF, params: [-1.0, 0.0, 0.0, nan(), nan(), nan(), altitude_relative + amsl], command_int: false, frame: FRAME_GLOBAL, show_error: true }]),
        AUTOPILOT_ARDUPILOT => {
            let class = modes::vehicle_class(state.vehicle_type);
            if class != VehicleClass::MultiRotor && !matches!(state.vehicle_type, 19..=25) {
                return Plan::Refused("Vehicle does not support guided takeoff".into());
            }
            let altitude = if altitude_relative.is_finite() && altitude_relative > state.minimum_takeoff_altitude { altitude_relative } else { state.minimum_takeoff_altitude };
            let mut steps = match mode_or_refuse(state, "Guided") {
                Ok(steps) => steps,
                Err(reason) => return Plan::Refused(reason),
            };
            steps.push(Step::Arm);
            steps.push(Step::WaitArmed);
            steps.push(Step::Command { command: CMD_NAV_TAKEOFF, params: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, altitude], command_int: false, frame: FRAME_GLOBAL, show_error: true });
            Plan::Steps(steps)
        }
        _ => Plan::Refused("Vehicle does not support guided takeoff".into()),
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
                steps.push(Step::Command { command: CMD_DO_REPOSITION, params: [-1.0, REPOSITION_CHANGE_MODE, loiter_radius, nan(), latitude, longitude, amsl], command_int: true, frame: FRAME_GLOBAL, show_error: false });
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

pub fn change_altitude(state: &VehicleState, delta: f64, pause_first: bool) -> Plan {
    match state.autopilot {
        AUTOPILOT_PX4 => {
            let Some(home) = state.home_altitude.filter(|a| a.is_finite()) else { return Plan::Refused("Unable to change altitude, home position altitude unknown.".into()) };
            let Some(relative) = state.altitude_relative.filter(|a| a.is_finite()) else { return Plan::Refused("Unable to change altitude, vehicle altitude not known.".into()) };
            let target = home + relative + delta;
            let mut steps = Vec::new();
            if pause_first {
                steps.push(Step::Command { command: CMD_DO_REPOSITION, params: [-1.0, REPOSITION_CHANGE_MODE, 0.0, nan(), nan(), nan(), nan()], command_int: false, frame: FRAME_GLOBAL, show_error: false });
            }
            steps.push(Step::Command { command: CMD_DO_REPOSITION, params: [-1.0, REPOSITION_CHANGE_MODE, 0.0, nan(), nan(), nan(), target], command_int: false, frame: FRAME_GLOBAL, show_error: true });
            Plan::Steps(steps)
        }
        AUTOPILOT_ARDUPILOT => {
            if state.altitude_relative.is_none_or(|a| !a.is_finite()) {
                return Plan::Refused("Unable to change altitude, vehicle altitude not known.".into());
            }
            let mut steps = Vec::new();
            if pause_first {
                match mode_or_refuse(state, pause_mode(state)) {
                    Ok(mode_steps) => steps.extend(mode_steps),
                    Err(reason) => return Plan::Refused(reason),
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn px4() -> VehicleState {
        VehicleState { autopilot: AUTOPILOT_PX4, vehicle_type: 2, base_mode: 0x81, flight_mode: "Position".into(), armed: true, altitude_amsl: Some(500.0), altitude_relative: Some(20.0), home_altitude: Some(480.0), capabilities: CAP_COMMAND_INT, reposition_supported: None, minimum_takeoff_altitude: 2.5, current_heading: Some(90.0) }
    }

    fn copter() -> VehicleState {
        VehicleState { autopilot: AUTOPILOT_ARDUPILOT, vehicle_type: 2, base_mode: 0x81, flight_mode: "Loiter".into(), armed: false, altitude_amsl: Some(500.0), altitude_relative: Some(20.0), home_altitude: Some(480.0), capabilities: 0, reposition_supported: None, minimum_takeoff_altitude: 2.5, current_heading: None }
    }

    fn command(step: &Step) -> (u16, [f64; 7], bool) {
        match step {
            Step::Command { command, params, command_int, .. } => (*command, *params, *command_int),
            other => panic!("not a command: {other:?}"),
        }
    }

    #[test]
    fn px4_plans_use_reposition_and_amsl_altitudes() {
        let state = px4();
        let Plan::Steps(steps) = takeoff(&state, 15.0) else { panic!() };
        let (cmd, params, _) = command(&steps[0]);
        assert_eq!((cmd, params[0], params[6]), (CMD_NAV_TAKEOFF, -1.0, 515.0));
        let Plan::Steps(steps) = goto(&state, 47.4, 8.5, 0.0) else { panic!() };
        let (cmd, params, int) = command(&steps[0]);
        assert_eq!((cmd, params[1], params[4], params[5], params[6], int), (CMD_DO_REPOSITION, 1.0, 47.4, 8.5, 500.0, true));
        let Plan::Steps(steps) = change_altitude(&state, 5.0, true) else { panic!() };
        assert_eq!(steps.len(), 2);
        assert!(command(&steps[0]).1[6].is_nan());
        assert_eq!(command(&steps[1]).1[6], 505.0);
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
        assert!(matches!(&steps[0], Step::SetMode { mode, via_command: true, custom_mode: 4, .. } if mode == "Guided"));
        assert_eq!(steps[2], Step::Arm);
        assert_eq!(command(&steps[4]).1[6], 2.5, "the minimum takeoff altitude wins over a lower request");
        let Plan::Steps(steps) = goto(&state, 47.4, 8.5, 30.0) else { panic!() };
        assert_eq!(command(&steps[0]).1[2], 30.0);
        assert!(matches!(steps.last(), Some(Step::GuidedMissionItem { altitude_relative, .. }) if *altitude_relative == 20.0));
        let supported = VehicleState { reposition_supported: Some(true), ..copter() };
        assert_eq!(match goto(&supported, 1.0, 2.0, 0.0) { Plan::Steps(s) => s.len(), _ => 0 }, 1);
        let unsupported = VehicleState { reposition_supported: Some(false), ..copter() };
        assert!(matches!(goto(&unsupported, 1.0, 2.0, 0.0), Plan::Steps(s) if matches!(s[0], Step::SetMode { .. })));
        let Plan::Steps(steps) = change_altitude(&state, 3.0, true) else { panic!() };
        assert!(matches!(&steps[0], Step::SetMode { mode, .. } if mode == "Brake"));
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
}
