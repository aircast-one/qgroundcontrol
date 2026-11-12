use crate::guidedcmd::{self, Plan};
use crate::payloadcheck::{DROP_ALL_NEEDS_BOARD, DROP_BOARD_BAYS, DROP_BOARD_ONLY_DROPS, Servo};

pub const RELEASE: f64 = 0.0;

pub fn board_empties(loaded: f32) -> Option<std::collections::BTreeSet<i64>> {
    let mask = loaded.round() as i64;
    (1..1 << DROP_BOARD_BAYS).contains(&mask).then(|| {
        (1..=DROP_BOARD_BAYS)
            .filter(|bay| mask & (1 << (DROP_BOARD_BAYS - bay)) == 0)
            .collect()
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum Payload {
    None,
    Gripper,
    ServoBays(Vec<Servo>),
    DropBoard { component: u8 },
    DeviceDropper { bays: i64 },
}

impl Payload {
    pub fn detect(
        board: Option<u8>,
        servos: Vec<Servo>,
        device_bays: i64,
        gripper: bool,
    ) -> Payload {
        match (board, servos.is_empty(), device_bays > 0, gripper) {
            (Some(component), ..) => Payload::DropBoard { component },
            (None, false, ..) => Payload::ServoBays(servos),
            (None, true, true, _) => Payload::DeviceDropper { bays: device_bays },
            (None, true, false, true) => Payload::Gripper,
            (None, true, false, false) => Payload::None,
        }
    }

    pub fn bays(&self) -> i64 {
        match self {
            Payload::DropBoard { .. } => DROP_BOARD_BAYS,
            Payload::ServoBays(servos) => servos.len() as i64,
            Payload::DeviceDropper { bays } => *bays,
            Payload::Gripper | Payload::None => 0,
        }
    }

    pub fn drops_all(&self) -> bool {
        matches!(self, Payload::DropBoard { .. })
    }

    pub fn drop_all(&self) -> Plan {
        match self {
            Payload::DropBoard { component } => guidedcmd::drop_all_on(*component, self.bays()),
            _ => Plan::Refused(DROP_ALL_NEEDS_BOARD.into()),
        }
    }

    pub fn grip(&self, action: f64, bay: f64, refusal: impl FnOnce() -> Option<String>) -> Plan {
        match self {
            Payload::DropBoard { component } if action == RELEASE => {
                guidedcmd::drop_on(*component, bay)
            }
            Payload::DropBoard { .. } => Plan::Refused(DROP_BOARD_ONLY_DROPS.into()),
            Payload::ServoBays(servos) => {
                match crate::payloadcheck::servo_command(servos, action, bay as i64) {
                    Ok((output, pwm)) => guidedcmd::set_servo(output, pwm),
                    Err(reason) => Plan::Refused(reason),
                }
            }
            _ if action == RELEASE => {
                refusal().map_or_else(|| guidedcmd::gripper(action, bay), Plan::Refused)
            }
            _ => guidedcmd::gripper(action, bay),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVOS: [Servo; 2] = [
        Servo {
            output: 9,
            release: 1900.0,
            hold: 1100.0,
        },
        Servo {
            output: 10,
            release: 1900.0,
            hold: 1100.0,
        },
    ];

    #[test]
    fn one_payload_owns_the_drone_and_its_bays_are_where_its_drops_go() {
        let detect = |board, servos: &[Servo], device, gripper| {
            let payload = Payload::detect(board, servos.to_vec(), device, gripper);
            (payload.bays(), payload)
        };
        assert_eq!(
            detect(Some(169), &SERVOS, 6, true),
            (4, Payload::DropBoard { component: 169 }),
            "a drop board on the bus takes the drops, whatever else is set up"
        );
        assert_eq!(
            detect(None, &SERVOS, 6, true),
            (2, Payload::ServoBays(SERVOS.to_vec())),
            "servos set up per bay come next"
        );
        assert_eq!(
            detect(None, &[], 6, true),
            (6, Payload::DeviceDropper { bays: 6 })
        );
        assert_eq!(detect(None, &[], 0, true), (0, Payload::Gripper));
        assert_eq!(detect(None, &[], 0, false), (0, Payload::None));
    }

    #[test]
    fn only_a_drop_board_drops_all_and_it_only_drops() {
        let board = Payload::DropBoard { component: 169 };
        assert!(board.drops_all());
        assert!(!Payload::Gripper.drops_all());
        assert!(matches!(
            board.grip(1.0, 2.0, || None),
            Plan::Refused(reason) if reason == DROP_BOARD_ONLY_DROPS
        ));
        assert!(matches!(
            Payload::Gripper.drop_all(),
            Plan::Refused(reason) if reason == DROP_ALL_NEEDS_BOARD
        ));
        assert!(matches!(
            Payload::Gripper.grip(RELEASE, 0.0, || Some("Arm first.".into())),
            Plan::Refused(reason) if reason == "Arm first."
        ));
    }
}
