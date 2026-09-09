pub const AUTOPILOT_ARDUPILOT: u8 = 3;
pub const AUTOPILOT_PX4: u8 = 12;
pub const FLAG_CUSTOM: u8 = 1;
pub const FLAG_TEST: u8 = 2;
pub const FLAG_AUTO: u8 = 4;
pub const FLAG_GUIDED: u8 = 8;
pub const FLAG_STABILIZE: u8 = 16;
pub const FLAG_MANUAL: u8 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleClass {
    FixedWing,
    MultiRotor,
    Rover,
    Sub,
    Other,
}

pub fn vehicle_class(mav_type: u8) -> VehicleClass {
    match mav_type {
        1 | 16 | 19 | 20 | 21 | 22 | 23 | 24 | 25 => VehicleClass::FixedWing,
        2 | 3 | 4 | 13 | 14 | 15 => VehicleClass::MultiRotor,
        10 | 11 => VehicleClass::Rover,
        12 => VehicleClass::Sub,
        _ => VehicleClass::Other,
    }
}

pub const fn px4(main: u32, sub: u32) -> u32 {
    (main << 16) | (sub << 24)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mode {
    pub name: &'static str,
    pub custom_mode: u32,
    pub can_be_set: bool,
    pub advanced: bool,
}

pub const PX4_MODES: [Mode; 17] = [
    Mode { name: "Manual", custom_mode: px4(1, 0), can_be_set: true, advanced: false },
    Mode { name: "Stabilized", custom_mode: px4(7, 0), can_be_set: true, advanced: false },
    Mode { name: "Acro", custom_mode: px4(5, 0), can_be_set: true, advanced: true },
    Mode { name: "Rattitude", custom_mode: px4(8, 0), can_be_set: true, advanced: true },
    Mode { name: "Altitude", custom_mode: px4(2, 0), can_be_set: true, advanced: false },
    Mode { name: "Offboard", custom_mode: px4(6, 0), can_be_set: true, advanced: true },
    Mode { name: "Simple", custom_mode: px4(9, 0), can_be_set: false, advanced: true },
    Mode { name: "Position", custom_mode: px4(3, 0), can_be_set: true, advanced: false },
    Mode { name: "Orbit", custom_mode: px4(3, 1), can_be_set: false, advanced: true },
    Mode { name: "Hold", custom_mode: px4(4, 3), can_be_set: true, advanced: false },
    Mode { name: "Mission", custom_mode: px4(4, 4), can_be_set: true, advanced: false },
    Mode { name: "Return", custom_mode: px4(4, 5), can_be_set: true, advanced: false },
    Mode { name: "Land", custom_mode: px4(4, 6), can_be_set: false, advanced: false },
    Mode { name: "Precision Land", custom_mode: px4(4, 9), can_be_set: true, advanced: true },
    Mode { name: "Ready", custom_mode: px4(4, 1), can_be_set: false, advanced: false },
    Mode { name: "Return to Groundstation", custom_mode: px4(4, 7), can_be_set: false, advanced: true },
    Mode { name: "Takeoff", custom_mode: px4(4, 2), can_be_set: false, advanced: false },
];

pub const COPTER_MODES: [Mode; 26] = [
    Mode { name: "Stabilize", custom_mode: 0, can_be_set: true, advanced: false },
    Mode { name: "Acro", custom_mode: 1, can_be_set: true, advanced: true },
    Mode { name: "Altitude Hold", custom_mode: 2, can_be_set: true, advanced: false },
    Mode { name: "Auto", custom_mode: 3, can_be_set: true, advanced: false },
    Mode { name: "Guided", custom_mode: 4, can_be_set: true, advanced: false },
    Mode { name: "Loiter", custom_mode: 5, can_be_set: true, advanced: false },
    Mode { name: "RTL", custom_mode: 6, can_be_set: true, advanced: false },
    Mode { name: "Circle", custom_mode: 7, can_be_set: true, advanced: true },
    Mode { name: "Land", custom_mode: 9, can_be_set: true, advanced: false },
    Mode { name: "Drift", custom_mode: 11, can_be_set: true, advanced: true },
    Mode { name: "Sport", custom_mode: 13, can_be_set: true, advanced: true },
    Mode { name: "Flip", custom_mode: 14, can_be_set: true, advanced: true },
    Mode { name: "Autotune", custom_mode: 15, can_be_set: true, advanced: true },
    Mode { name: "Position Hold", custom_mode: 16, can_be_set: true, advanced: false },
    Mode { name: "Brake", custom_mode: 17, can_be_set: true, advanced: true },
    Mode { name: "Throw", custom_mode: 18, can_be_set: true, advanced: true },
    Mode { name: "Avoid ADSB", custom_mode: 19, can_be_set: true, advanced: true },
    Mode { name: "Guided No GPS", custom_mode: 20, can_be_set: true, advanced: true },
    Mode { name: "Smart RTL", custom_mode: 21, can_be_set: true, advanced: true },
    Mode { name: "Flow Hold", custom_mode: 22, can_be_set: true, advanced: true },
    Mode { name: "Follow", custom_mode: 23, can_be_set: true, advanced: true },
    Mode { name: "ZigZag", custom_mode: 24, can_be_set: true, advanced: true },
    Mode { name: "SystemID", custom_mode: 25, can_be_set: true, advanced: true },
    Mode { name: "AutoRotate", custom_mode: 26, can_be_set: true, advanced: true },
    Mode { name: "AutoRTL", custom_mode: 27, can_be_set: true, advanced: true },
    Mode { name: "Turtle", custom_mode: 28, can_be_set: true, advanced: true },
];

pub const PLANE_MODES: [Mode; 26] = [
    Mode { name: "Manual", custom_mode: 0, can_be_set: true, advanced: false },
    Mode { name: "Circle", custom_mode: 1, can_be_set: true, advanced: true },
    Mode { name: "Stabilize", custom_mode: 2, can_be_set: true, advanced: true },
    Mode { name: "Training", custom_mode: 3, can_be_set: true, advanced: true },
    Mode { name: "Acro", custom_mode: 4, can_be_set: true, advanced: true },
    Mode { name: "FBW A", custom_mode: 5, can_be_set: true, advanced: false },
    Mode { name: "FBW B", custom_mode: 6, can_be_set: true, advanced: true },
    Mode { name: "Cruise", custom_mode: 7, can_be_set: true, advanced: false },
    Mode { name: "Autotune", custom_mode: 8, can_be_set: true, advanced: true },
    Mode { name: "Auto", custom_mode: 10, can_be_set: true, advanced: false },
    Mode { name: "RTL", custom_mode: 11, can_be_set: true, advanced: false },
    Mode { name: "Loiter", custom_mode: 12, can_be_set: true, advanced: false },
    Mode { name: "Takeoff", custom_mode: 13, can_be_set: true, advanced: true },
    Mode { name: "Avoid ADSB", custom_mode: 14, can_be_set: true, advanced: true },
    Mode { name: "Guided", custom_mode: 15, can_be_set: true, advanced: false },
    Mode { name: "Initializing", custom_mode: 16, can_be_set: true, advanced: true },
    Mode { name: "QuadPlane Stabilize", custom_mode: 17, can_be_set: true, advanced: true },
    Mode { name: "QuadPlane Hover", custom_mode: 18, can_be_set: true, advanced: true },
    Mode { name: "QuadPlane Loiter", custom_mode: 19, can_be_set: true, advanced: false },
    Mode { name: "QuadPlane Land", custom_mode: 20, can_be_set: true, advanced: false },
    Mode { name: "QuadPlane RTL", custom_mode: 21, can_be_set: true, advanced: false },
    Mode { name: "QuadPlane AutoTune", custom_mode: 22, can_be_set: true, advanced: true },
    Mode { name: "QuadPlane Acro", custom_mode: 23, can_be_set: true, advanced: true },
    Mode { name: "Thermal", custom_mode: 24, can_be_set: true, advanced: true },
    Mode { name: "Loiter to QLand", custom_mode: 25, can_be_set: true, advanced: true },
    Mode { name: "Autoland", custom_mode: 26, can_be_set: true, advanced: true },
];

pub const ROVER_MODES: [Mode; 15] = [
    Mode { name: "Manual", custom_mode: 0, can_be_set: true, advanced: false },
    Mode { name: "Acro", custom_mode: 1, can_be_set: true, advanced: true },
    Mode { name: "Learning", custom_mode: 2, can_be_set: true, advanced: true },
    Mode { name: "Steering", custom_mode: 3, can_be_set: true, advanced: false },
    Mode { name: "Hold", custom_mode: 4, can_be_set: true, advanced: false },
    Mode { name: "Loiter", custom_mode: 5, can_be_set: true, advanced: false },
    Mode { name: "Follow", custom_mode: 6, can_be_set: true, advanced: true },
    Mode { name: "Simple", custom_mode: 7, can_be_set: true, advanced: true },
    Mode { name: "Dock", custom_mode: 8, can_be_set: true, advanced: true },
    Mode { name: "Circle", custom_mode: 9, can_be_set: true, advanced: true },
    Mode { name: "Auto", custom_mode: 10, can_be_set: true, advanced: false },
    Mode { name: "RTL", custom_mode: 11, can_be_set: true, advanced: false },
    Mode { name: "Smart RTL", custom_mode: 12, can_be_set: true, advanced: true },
    Mode { name: "Guided", custom_mode: 15, can_be_set: true, advanced: false },
    Mode { name: "Initializing", custom_mode: 16, can_be_set: true, advanced: true },
];

pub const SUB_MODES: [Mode; 11] = [
    Mode { name: "Manual", custom_mode: 19, can_be_set: true, advanced: false },
    Mode { name: "Stabilize", custom_mode: 0, can_be_set: true, advanced: false },
    Mode { name: "Acro", custom_mode: 1, can_be_set: true, advanced: true },
    Mode { name: "Depth Hold", custom_mode: 2, can_be_set: true, advanced: false },
    Mode { name: "Auto", custom_mode: 3, can_be_set: true, advanced: false },
    Mode { name: "Guided", custom_mode: 4, can_be_set: true, advanced: false },
    Mode { name: "Circle", custom_mode: 7, can_be_set: true, advanced: true },
    Mode { name: "Surface", custom_mode: 9, can_be_set: true, advanced: false },
    Mode { name: "Position Hold", custom_mode: 16, can_be_set: true, advanced: false },
    Mode { name: "Motor Detection", custom_mode: 20, can_be_set: true, advanced: true },
    Mode { name: "Surftrak", custom_mode: 21, can_be_set: true, advanced: true },
];

pub fn table(autopilot: u8, mav_type: u8) -> &'static [Mode] {
    match (autopilot, vehicle_class(mav_type)) {
        (AUTOPILOT_PX4, _) => &PX4_MODES,
        (AUTOPILOT_ARDUPILOT, VehicleClass::FixedWing) => &PLANE_MODES,
        (AUTOPILOT_ARDUPILOT, VehicleClass::Rover) => &ROVER_MODES,
        (AUTOPILOT_ARDUPILOT, VehicleClass::Sub) => &SUB_MODES,
        (AUTOPILOT_ARDUPILOT, _) => &COPTER_MODES,
        _ => &[],
    }
}

pub fn name(autopilot: u8, mav_type: u8, base_mode: u8, custom_mode: u32) -> String {
    let modes = table(autopilot, mav_type);
    let known = || modes.iter().find(|m| m.custom_mode == custom_mode).map(|m| m.name.to_string());
    match autopilot {
        AUTOPILOT_PX4 | AUTOPILOT_ARDUPILOT => {
            if base_mode & FLAG_CUSTOM != 0 { known().unwrap_or_else(|| format!("Mode {custom_mode}")) } else { String::new() }
        }
        _ if base_mode == 0 => "PreFlight".to_string(),
        _ if base_mode & FLAG_CUSTOM != 0 => known().unwrap_or_else(|| format!("Custom:0x{custom_mode:x}")),
        _ => [(FLAG_MANUAL, "Manual"), (FLAG_STABILIZE, "Stabilize"), (FLAG_GUIDED, "Guided"), (FLAG_AUTO, "Auto"), (FLAG_TEST, "Test")]
            .iter()
            .filter(|(bit, _)| base_mode & bit != 0)
            .map(|(_, n)| *n)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

pub fn custom_mode_for(autopilot: u8, mav_type: u8, mode_name: &str) -> Option<u32> {
    table(autopilot, mav_type).iter().find(|m| m.name == mode_name).map(|m| m.custom_mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn px4_modes_decode_from_the_main_and_sub_fields() {
        assert_eq!(name(AUTOPILOT_PX4, 2, FLAG_CUSTOM, px4(4, 4)), "Mission");
        assert_eq!(name(AUTOPILOT_PX4, 2, FLAG_CUSTOM, px4(3, 0)), "Position");
        assert_eq!(name(AUTOPILOT_PX4, 2, FLAG_CUSTOM, px4(3, 1)), "Orbit");
        assert_eq!(name(AUTOPILOT_PX4, 2, FLAG_CUSTOM, px4(4, 8)), "Mode 134479872");
        assert_eq!(name(AUTOPILOT_PX4, 2, 0, px4(4, 4)), "");
        assert_eq!(custom_mode_for(AUTOPILOT_PX4, 1, "Return"), Some(px4(4, 5)));
        assert_eq!(PX4_MODES.iter().filter(|m| m.can_be_set).count(), 11);
    }

    #[test]
    fn ardupilot_modes_follow_the_vehicle_class_and_the_generic_plugin_reads_the_flags() {
        assert_eq!(name(AUTOPILOT_ARDUPILOT, 2, FLAG_CUSTOM, 17), "Brake");
        assert_eq!(name(AUTOPILOT_ARDUPILOT, 1, FLAG_CUSTOM, 5), "FBW A");
        assert_eq!(name(AUTOPILOT_ARDUPILOT, 10, FLAG_CUSTOM, 4), "Hold");
        assert_eq!(name(AUTOPILOT_ARDUPILOT, 12, FLAG_CUSTOM, 19), "Manual");
        assert_eq!(name(AUTOPILOT_ARDUPILOT, 2, FLAG_CUSTOM, 99), "Mode 99");
        assert_eq!(name(0, 2, 0, 0), "PreFlight");
        assert_eq!(name(0, 2, FLAG_MANUAL | FLAG_STABILIZE, 0), "Manual Stabilize");
        assert_eq!(name(0, 2, FLAG_CUSTOM, 0x1f), "Custom:0x1f");
        assert_eq!(custom_mode_for(AUTOPILOT_ARDUPILOT, 2, "Smart RTL"), Some(21));
    }
}
