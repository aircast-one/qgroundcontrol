use crate::mocklink::Vehicle;

const VOLTS_PER_LOW_CELL: f64 = 3.5;
const VOLTS_PER_FULL_CELL: f64 = 4.2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    pub cruise: f64,
    pub accel: f64,
    pub climb: f64,
    pub descent: f64,
    pub land: f64,
    pub land_low: f64,
    pub pilot_speed: f64,
    pub pilot_accel: f64,
    pub pilot_climb: f64,
    pub pilot_descent: f64,
    pub pilot_yaw: f64,
    pub rtl_alt: f64,
    pub arrive: f64,
    pub loiter_radius: f64,
    pub circle_radius: f64,
    pub bank: f64,
    pub capacity_mah: f64,
    pub cells: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reaction {
    Warn,
    Land,
    Return,
    Hold,
    Disarm,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    pub name: &'static str,
    pub mah_left: f64,
    pub volts: f64,
    pub fraction: f64,
    pub reaction: Reaction,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkLoss {
    pub after_s: f64,
    pub reaction: Reaction,
    pub keeps_auto: bool,
    pub keeps_pilot: bool,
    pub keeps_landing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fence {
    pub max_alt: Option<f64>,
    pub radius: Option<f64>,
    pub zones: bool,
    pub reaction: Reaction,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Failsafes {
    pub battery: Vec<Level>,
    pub link: Option<LinkLoss>,
    pub fence: Option<Fence>,
    pub disarm_landed_s: Option<f64>,
    pub disarm_idle_s: Option<f64>,
}

fn pick(read: &dyn Fn(&str) -> Option<f64>, names: &[(&str, f64)]) -> Option<f64> {
    names
        .iter()
        .find_map(|(name, scale)| read(name).map(|value| value * scale))
        .filter(|value| *value > 0.0)
}

pub fn tuning(read: &dyn Fn(&str) -> Option<f64>, vehicle: Vehicle) -> Tuning {
    let plane = vehicle == Vehicle::Plane;
    let get = |names: &[(&str, f64)], fallback: f64| pick(read, names).unwrap_or(fallback);
    let descent = get(
        &[
            ("WP_SPD_DN", 1.0),
            ("WPNAV_SPEED_DN", 0.01),
            ("MPC_Z_V_AUTO_DN", 1.0),
            ("TECS_SINK_MAX", 1.0),
        ],
        1.5,
    );
    let pilot_climb = get(
        &[
            ("PILOT_SPD_UP", 1.0),
            ("PILOT_SPEED_UP", 0.01),
            ("MPC_Z_VEL_MAX_UP", 1.0),
        ],
        2.5,
    );
    let cells = pick(read, &[("BAT1_N_CELLS", 1.0)])
        .or_else(|| pick(read, &[("BATT_LOW_VOLT", 1.0 / VOLTS_PER_LOW_CELL)]))
        .or_else(|| pick(read, &[("MOT_BAT_VOLT_MAX", 1.0 / VOLTS_PER_FULL_CELL)]))
        .map_or(4.0, f64::round);
    Tuning {
        cruise: get(
            &[
                ("WP_SPD", 1.0),
                ("WPNAV_SPEED", 0.01),
                ("MPC_XY_CRUISE", 1.0),
                ("AIRSPEED_CRUISE", 1.0),
                ("TRIM_ARSPD_CM", 0.01),
                ("WP_SPEED", 1.0),
                ("CRUISE_SPEED", 1.0),
            ],
            if plane { 18.0 } else { 5.0 },
        ),
        accel: get(
            &[
                ("WP_ACC", 1.0),
                ("WPNAV_ACCEL", 0.01),
                ("MPC_ACC_HOR", 1.0),
                ("WP_ACCEL", 1.0),
            ],
            2.5,
        ),
        climb: get(
            &[
                ("WP_SPD_UP", 1.0),
                ("WPNAV_SPEED_UP", 0.01),
                ("MPC_Z_V_AUTO_UP", 1.0),
                ("TECS_CLMB_MAX", 1.0),
            ],
            2.5,
        ),
        descent,
        land: get(
            &[
                ("LAND_SPD_MS", 1.0),
                ("LAND_SPEED", 0.01),
                ("MPC_LAND_SPEED", 1.0),
            ],
            descent,
        ),
        land_low: get(
            &[
                ("LAND_ALT_LOW_M", 1.0),
                ("LAND_ALT_LOW", 0.01),
                ("MPC_LAND_ALT1", 1.0),
            ],
            0.0,
        ),
        pilot_speed: get(
            &[
                ("LOIT_SPEED_MS", 1.0),
                ("LOIT_SPEED", 0.01),
                ("MPC_VEL_MANUAL", 1.0),
            ],
            10.0,
        ),
        pilot_accel: get(
            &[
                ("LOIT_ACC_MAX_M", 1.0),
                ("LOIT_ACC_MAX", 0.01),
                ("MPC_ACC_HOR_MAX", 1.0),
            ],
            2.5,
        ),
        pilot_climb,
        pilot_descent: get(
            &[
                ("PILOT_SPD_DN", 1.0),
                ("PILOT_SPEED_DN", 0.01),
                ("MPC_Z_VEL_MAX_DN", 1.0),
            ],
            pilot_climb,
        ),
        pilot_yaw: get(&[("PILOT_Y_RATE", 1.0), ("MPC_MAN_Y_MAX", 1.0)], 90.0),
        rtl_alt: get(
            &[
                ("RTL_ALT_M", 1.0),
                ("RTL_ALT", 0.01),
                ("RTL_RETURN_ALT", 1.0),
                ("RTL_ALTITUDE", 1.0),
                ("ALT_HOLD_RTL", 0.01),
            ],
            0.0,
        ),
        arrive: get(
            &[
                ("WP_RADIUS_M", 1.0),
                ("WPNAV_RADIUS", 0.01),
                ("NAV_ACC_RAD", 1.0),
                ("WP_RADIUS", 1.0),
            ],
            if plane { 90.0 } else { 1.0 },
        ),
        loiter_radius: read("WP_LOITER_RAD")
            .map(f64::abs)
            .filter(|radius| *radius > 0.0)
            .unwrap_or(60.0),
        circle_radius: get(&[("CIRCLE_RADIUS_M", 1.0), ("CIRCLE_RADIUS", 0.01)], 5.0),
        bank: get(&[("ROLL_LIMIT_DEG", 1.0), ("LIM_ROLL_CD", 0.01)], 45.0),
        capacity_mah: get(&[("BATT_CAPACITY", 1.0), ("BAT1_CAPACITY", 1.0)], 5000.0),
        cells,
    }
}

fn apm_battery_reaction(vehicle: Vehicle, action: i64) -> Reaction {
    match (vehicle, action) {
        (Vehicle::Copter, 1) => Reaction::Land,
        (Vehicle::Copter, 2..=4 | 6) => Reaction::Return,
        (Vehicle::Copter, 5) => Reaction::Disarm,
        (Vehicle::Plane, 1 | 2) => Reaction::Return,
        (Vehicle::Plane, 3 | 5) => Reaction::Disarm,
        (Vehicle::Plane, 4 | 6) => Reaction::Land,
        (Vehicle::Rover, 1 | 3 | 4) => Reaction::Return,
        (Vehicle::Rover, 2) => Reaction::Hold,
        (Vehicle::Rover, 5) => Reaction::Disarm,
        _ => Reaction::Warn,
    }
}

fn apm_fence_reaction(vehicle: Vehicle, action: i64) -> Reaction {
    match (vehicle, action) {
        (Vehicle::Copter, 1 | 3 | 5) => Reaction::Return,
        (Vehicle::Copter, 2) => Reaction::Land,
        (Vehicle::Copter, 4) => Reaction::Hold,
        (Vehicle::Plane, 1 | 3 | 4) => Reaction::Return,
        (Vehicle::Rover, 1 | 3 | 4) => Reaction::Return,
        (Vehicle::Rover, 2) => Reaction::Hold,
        _ => Reaction::Warn,
    }
}

fn px4_reaction(action: i64) -> Option<Reaction> {
    match action {
        1 => Some(Reaction::Hold),
        2 => Some(Reaction::Return),
        3 => Some(Reaction::Land),
        5 | 6 => Some(Reaction::Disarm),
        _ => None,
    }
}

pub fn failsafes(read: &dyn Fn(&str) -> Option<f64>, vehicle: Vehicle) -> Failsafes {
    let whole = |name: &str| read(name).map_or(0, |value| value as i64);
    let value =
        |name: &str, fallback: f64| read(name).filter(|value| *value > 0.0).unwrap_or(fallback);
    let options = whole("FS_OPTIONS");
    match vehicle {
        Vehicle::Px4 => Failsafes {
            battery: vec![
                Level {
                    name: "low",
                    mah_left: 0.0,
                    volts: 0.0,
                    fraction: value("BAT_LOW_THR", 0.0),
                    reaction: Reaction::Warn,
                },
                Level {
                    name: "critical",
                    mah_left: 0.0,
                    volts: 0.0,
                    fraction: value("BAT_CRIT_THR", 0.0),
                    reaction: match whole("COM_LOW_BAT_ACT") {
                        2 => Reaction::Land,
                        3 => Reaction::Return,
                        _ => Reaction::Warn,
                    },
                },
                Level {
                    name: "emergency",
                    mah_left: 0.0,
                    volts: 0.0,
                    fraction: value("BAT_EMERGEN_THR", 0.0),
                    reaction: match whole("COM_LOW_BAT_ACT") {
                        2 | 3 => Reaction::Land,
                        _ => Reaction::Warn,
                    },
                },
            ],
            link: px4_reaction(whole("NAV_DLL_ACT")).map(|reaction| LinkLoss {
                after_s: value("COM_DL_LOSS_T", 10.0),
                reaction,
                keeps_auto: false,
                keeps_pilot: false,
                keeps_landing: false,
            }),
            fence: match whole("GF_ACTION") {
                0 => None,
                action => Some(Fence {
                    max_alt: read("GF_MAX_VER_DIST").filter(|v| *v > 0.0),
                    radius: read("GF_MAX_HOR_DIST").filter(|v| *v > 0.0),
                    zones: true,
                    reaction: match action {
                        2 => Reaction::Hold,
                        3 => Reaction::Return,
                        4 => Reaction::Disarm,
                        5 => Reaction::Land,
                        _ => Reaction::Warn,
                    },
                }),
            },
            disarm_landed_s: read("COM_DISARM_LAND").filter(|v| *v > 0.0),
            disarm_idle_s: read("COM_DISARM_PRFLT").filter(|v| *v > 0.0),
        },
        Vehicle::Copter | Vehicle::Plane | Vehicle::Rover => {
            let level = |name, act: &str, mah: &str, volts: &str| Level {
                name,
                mah_left: value(mah, 0.0),
                volts: value(volts, 0.0),
                fraction: 0.0,
                reaction: apm_battery_reaction(vehicle, whole(act)),
            };
            let link = match vehicle {
                Vehicle::Plane => (whole("FS_GCS_ENABL") > 0).then(|| {
                    let action = whole("FS_LONG_ACTN");
                    let reaction = match action {
                        2 => Reaction::Warn,
                        3 => Reaction::Disarm,
                        _ => Reaction::Return,
                    };
                    LinkLoss {
                        after_s: value("FS_LONG_TIMEOUT", 5.0),
                        reaction,
                        keeps_auto: action == 0,
                        keeps_pilot: false,
                        keeps_landing: true,
                    }
                }),
                Vehicle::Rover => (whole("FS_GCS_ENABLE") > 0).then(|| LinkLoss {
                    after_s: value("FS_GCS_TIMEOUT", 5.0),
                    reaction: apm_battery_reaction(Vehicle::Rover, whole("FS_ACTION")),
                    keeps_auto: false,
                    keeps_pilot: false,
                    keeps_landing: false,
                }),
                _ => match whole("FS_GCS_ENABLE") {
                    0 => None,
                    enable => Some(LinkLoss {
                        after_s: value("FS_GCS_TIMEOUT", 5.0),
                        reaction: if enable == 5 {
                            Reaction::Land
                        } else {
                            Reaction::Return
                        },
                        keeps_auto: enable == 2 || options & 2 != 0,
                        keeps_pilot: options & 16 != 0,
                        keeps_landing: options & 8 != 0,
                    }),
                },
            };
            let types = whole("FENCE_TYPE");
            let delay = read("DISARM_DELAY")
                .filter(|v| *v > 0.0)
                .filter(|_| vehicle != Vehicle::Plane);
            Failsafes {
                battery: vec![
                    level("low", "BATT_FS_LOW_ACT", "BATT_LOW_MAH", "BATT_LOW_VOLT"),
                    level(
                        "critical",
                        "BATT_FS_CRT_ACT",
                        "BATT_CRT_MAH",
                        "BATT_CRT_VOLT",
                    ),
                ],
                link,
                fence: (whole("FENCE_ENABLE") > 0).then(|| Fence {
                    max_alt: read("FENCE_ALT_MAX").filter(|_| types & 1 != 0),
                    radius: read("FENCE_RADIUS").filter(|_| types & 2 != 0),
                    zones: types & 4 != 0,
                    reaction: apm_fence_reaction(vehicle, whole("FENCE_ACTION")),
                }),
                disarm_landed_s: delay,
                disarm_idle_s: delay,
            }
        }
        _ => Failsafes {
            battery: Vec::new(),
            link: None,
            fence: None,
            disarm_landed_s: None,
            disarm_idle_s: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(pairs: &'static [(&'static str, f64)]) -> impl Fn(&str) -> Option<f64> {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| *value)
        }
    }

    #[test]
    fn speeds_come_from_whichever_firmware_generation_named_them() {
        let new = tuning(
            &reading(&[
                ("WP_SPD", 10.0),
                ("LAND_SPD_MS", 0.5),
                ("RTL_ALT_M", 15.0),
                ("BATT_LOW_VOLT", 10.5),
            ]),
            Vehicle::Copter,
        );
        let old = tuning(
            &reading(&[
                ("WPNAV_SPEED", 1000.0),
                ("LAND_SPEED", 50.0),
                ("RTL_ALT", 1500.0),
                ("MOT_BAT_VOLT_MAX", 25.2),
            ]),
            Vehicle::Copter,
        );
        assert_eq!(
            (
                new.cruise,
                new.land,
                new.rtl_alt,
                new.cells,
                new.circle_radius
            ),
            (10.0, 0.5, 15.0, 3.0, 5.0),
            "ArduCopter 4.7 names, in metres"
        );
        assert_eq!(
            (old.cruise, old.land, old.rtl_alt, old.cells),
            (10.0, 0.5, 15.0, 6.0),
            "ArduCopter 4.5 names, in centimetres"
        );
        let plane = tuning(
            &reading(&[("AIRSPEED_CRUISE", 22.0), ("WP_LOITER_RAD", -80.0)]),
            Vehicle::Plane,
        );
        assert_eq!(
            (plane.cruise, plane.loiter_radius, plane.arrive),
            (22.0, 80.0, 90.0),
            "a counter-clockwise loiter radius is still a radius"
        );
        assert_eq!(
            tuning(
                &reading(&[("PILOT_SPD_UP", 3.0), ("PILOT_SPD_DN", 0.0)]),
                Vehicle::Copter
            )
            .pilot_descent,
            3.0,
            "a zero descent speed means the climb speed"
        );
    }

    #[test]
    fn each_firmware_maps_its_own_failsafe_numbers() {
        let copter = failsafes(
            &reading(&[
                ("BATT_FS_LOW_ACT", 2.0),
                ("BATT_LOW_MAH", 500.0),
                ("FS_GCS_ENABLE", 5.0),
                ("FS_OPTIONS", 16.0),
                ("FENCE_ENABLE", 1.0),
                ("FENCE_TYPE", 3.0),
                ("FENCE_ALT_MAX", 100.0),
                ("FENCE_RADIUS", 150.0),
                ("FENCE_ACTION", 1.0),
                ("DISARM_DELAY", 10.0),
            ]),
            Vehicle::Copter,
        );
        assert_eq!(
            (copter.battery[0].reaction, copter.battery[0].mah_left),
            (Reaction::Return, 500.0)
        );
        assert_eq!(
            copter
                .link
                .map(|l| (l.reaction, l.keeps_pilot, l.keeps_auto)),
            Some((Reaction::Land, true, false))
        );
        assert_eq!(
            copter.fence,
            Some(Fence {
                max_alt: Some(100.0),
                radius: Some(150.0),
                zones: false,
                reaction: Reaction::Return
            })
        );
        assert_eq!(
            failsafes(&reading(&[("BATT_FS_LOW_ACT", 2.0)]), Vehicle::Plane).battery[0].reaction,
            Reaction::Return,
            "plane action 2 is Land, which flies to a landing, not a vertical descent"
        );
        let px4 = failsafes(
            &reading(&[
                ("COM_LOW_BAT_ACT", 3.0),
                ("BAT_CRIT_THR", 0.07),
                ("NAV_DLL_ACT", 2.0),
                ("GF_ACTION", 3.0),
            ]),
            Vehicle::Px4,
        );
        assert_eq!(
            (px4.battery[1].reaction, px4.battery[2].reaction),
            (Reaction::Return, Reaction::Land)
        );
        assert_eq!(
            (px4.link.map(|l| l.reaction), px4.fence.map(|f| f.reaction)),
            (Some(Reaction::Return), Some(Reaction::Return))
        );
        assert_eq!(
            failsafes(&reading(&[]), Vehicle::Copter).link,
            None,
            "no failsafe unless the parameters turn it on"
        );
    }
}
