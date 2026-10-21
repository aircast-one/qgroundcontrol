use mavlink::dialects::ardupilotmega::*;

use crate::mocklink::{AUTOPILOT, Out};

pub const GIMBAL: u8 = 154;
const STATUS_DEFAULT_INTERVAL_US: i64 = 1_000_000;
const ATTITUDE_DEFAULT_INTERVAL_US: i64 = 100_000;
const MANAGER_INFORMATION: u32 = 280;
const MANAGER_STATUS: u32 = 281;
const DEVICE_ATTITUDE_STATUS: u32 = 285;
const PITCH_RANGE: (f32, f32) = (-90.0, 30.0);
const ROLL_LIMIT: f32 = 45.0;
const YAW_LIMIT: f32 = 180.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Gimbal {
    pitch: f32,
    yaw: f32,
    earth_yaw: Option<f32>,
    status_interval_us: i64,
    attitude_interval_us: i64,
    status_sent_ms: Option<u64>,
    attitude_sent_ms: Option<u64>,
    primary: (u8, u8),
    secondary: (u8, u8),
}

impl Default for Gimbal {
    fn default() -> Gimbal {
        Gimbal {
            pitch: 0.0,
            yaw: 0.0,
            earth_yaw: None,
            status_interval_us: 0,
            attitude_interval_us: 0,
            status_sent_ms: None,
            attitude_sent_ms: None,
            primary: (0, 0),
            secondary: (0, 0),
        }
    }
}

fn body_yaw(yaw: f32) -> f32 {
    ((yaw + 540.0).rem_euclid(360.0) - 180.0).clamp(-YAW_LIMIT, YAW_LIMIT)
}

fn due(interval_us: i64, last: Option<u64>, now_ms: u64) -> bool {
    interval_us > 0
        && last.is_none_or(|sent| now_ms.saturating_sub(sent) >= (interval_us / 1000) as u64)
}

fn ack(component: u8, from: (u8, u8), command: MavCmd, result: MavResult) -> Out {
    (
        component,
        MavMessage::COMMAND_ACK(COMMAND_ACK_DATA {
            command,
            result,
            progress: 0,
            result_param2: 0,
            target_system: from.0,
            target_component: from.1,
        }),
    )
}

impl Gimbal {
    pub fn aim(&self) -> (f64, f64) {
        (f64::from(self.yaw), f64::from(self.pitch))
    }

    pub fn point(&mut self, pitch: f32, yaw: f32) {
        self.earth_yaw = None;
        self.pitch = pitch.clamp(PITCH_RANGE.0, PITCH_RANGE.1);
        self.yaw = body_yaw(yaw);
    }

    pub fn hold(&mut self, vehicle_heading: f64) {
        if let Some(earth) = self.earth_yaw {
            self.yaw = body_yaw(earth - vehicle_heading as f32);
        }
    }

    pub fn tick(&mut self, now_ms: u64) -> Vec<Out> {
        let status = due(self.status_interval_us, self.status_sent_ms, now_ms).then(|| {
            self.status_sent_ms = Some(now_ms);
            self.status()
        });
        let attitude = due(self.attitude_interval_us, self.attitude_sent_ms, now_ms).then(|| {
            self.attitude_sent_ms = Some(now_ms);
            self.attitude()
        });
        status.into_iter().chain(attitude).collect()
    }

    pub fn command(
        &mut self,
        from: (u8, u8),
        component: u8,
        command: MavCmd,
        params: [f32; 7],
    ) -> Option<Vec<Out>> {
        let reply = |result| Some(vec![ack(component, from, command, result)]);
        match command {
            MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL => {
                let default = match params[0] as u32 {
                    DEVICE_ATTITUDE_STATUS => ATTITUDE_DEFAULT_INTERVAL_US,
                    _ => STATUS_DEFAULT_INTERVAL_US,
                };
                let interval = match params[1] as i64 {
                    0 => default,
                    asked if asked < -1 => return None,
                    asked => asked,
                };
                match params[0] as u32 {
                    MANAGER_STATUS => self.status_interval_us = interval,
                    DEVICE_ATTITUDE_STATUS => self.attitude_interval_us = interval,
                    _ => return None,
                }
                reply(MavResult::MAV_RESULT_ACCEPTED)
            }
            MavCmd::MAV_CMD_REQUEST_MESSAGE if params[0] as u32 == MANAGER_INFORMATION => {
                Some(vec![
                    information(),
                    ack(component, from, command, MavResult::MAV_RESULT_ACCEPTED),
                ])
            }
            MavCmd::MAV_CMD_DO_GIMBAL_MANAGER_PITCHYAW => {
                let (pitch, yaw) = (params[0], params[1]);
                match pitch.is_nan() && yaw.is_nan() {
                    true => reply(MavResult::MAV_RESULT_DENIED),
                    false => {
                        if !pitch.is_nan() {
                            self.pitch = pitch.clamp(PITCH_RANGE.0, PITCH_RANGE.1);
                        }
                        if !yaw.is_nan() {
                            let earth =
                                params[4] as u32 & crate::gimbal::FLAG_YAW_IN_EARTH_FRAME != 0;
                            self.earth_yaw = earth.then_some(yaw);
                            if !earth {
                                self.yaw = yaw.clamp(-YAW_LIMIT, YAW_LIMIT);
                            }
                        }
                        reply(MavResult::MAV_RESULT_ACCEPTED)
                    }
                }
            }
            MavCmd::MAV_CMD_DO_GIMBAL_MANAGER_CONFIGURE => {
                let pick = |asked: f32, held: u8| if asked as u8 > 0 { asked as u8 } else { held };
                self.primary = (
                    pick(params[0], self.primary.0),
                    pick(params[1], self.primary.1),
                );
                self.secondary = (
                    pick(params[2], self.secondary.0),
                    pick(params[3], self.secondary.1),
                );
                reply(MavResult::MAV_RESULT_ACCEPTED)
            }
            _ => None,
        }
    }

    fn status(&self) -> Out {
        (
            AUTOPILOT,
            MavMessage::GIMBAL_MANAGER_STATUS(GIMBAL_MANAGER_STATUS_DATA {
                gimbal_device_id: GIMBAL,
                primary_control_sysid: self.primary.0,
                primary_control_compid: self.primary.1,
                secondary_control_sysid: self.secondary.0,
                secondary_control_compid: self.secondary.1,
                ..Default::default()
            }),
        )
    }

    fn attitude(&self) -> Out {
        let (pitch, yaw) = (self.pitch.to_radians() * 0.5, self.yaw.to_radians() * 0.5);
        let q = [
            pitch.cos() * yaw.cos(),
            -pitch.sin() * yaw.sin(),
            pitch.sin() * yaw.cos(),
            pitch.cos() * yaw.sin(),
        ];
        (
            GIMBAL,
            MavMessage::GIMBAL_DEVICE_ATTITUDE_STATUS(GIMBAL_DEVICE_ATTITUDE_STATUS_DATA {
                q,
                flags: GimbalDeviceFlags::GIMBAL_DEVICE_FLAGS_NEUTRAL
                    | GimbalDeviceFlags::GIMBAL_DEVICE_FLAGS_ROLL_LOCK
                    | GimbalDeviceFlags::GIMBAL_DEVICE_FLAGS_PITCH_LOCK,
                delta_yaw: self.yaw,
                ..Default::default()
            }),
        )
    }
}

fn information() -> Out {
    (
        AUTOPILOT,
        MavMessage::GIMBAL_MANAGER_INFORMATION(GIMBAL_MANAGER_INFORMATION_DATA {
            cap_flags: GimbalManagerCapFlags::GIMBAL_MANAGER_CAP_FLAGS_HAS_ROLL_AXIS
                | GimbalManagerCapFlags::GIMBAL_MANAGER_CAP_FLAGS_HAS_PITCH_AXIS
                | GimbalManagerCapFlags::GIMBAL_MANAGER_CAP_FLAGS_HAS_YAW_AXIS
                | GimbalManagerCapFlags::GIMBAL_MANAGER_CAP_FLAGS_HAS_YAW_FOLLOW
                | GimbalManagerCapFlags::GIMBAL_MANAGER_CAP_FLAGS_HAS_YAW_LOCK
                | GimbalManagerCapFlags::GIMBAL_MANAGER_CAP_FLAGS_HAS_RETRACT
                | GimbalManagerCapFlags::GIMBAL_MANAGER_CAP_FLAGS_HAS_NEUTRAL,
            gimbal_device_id: GIMBAL,
            roll_min: (-ROLL_LIMIT).to_radians(),
            roll_max: ROLL_LIMIT.to_radians(),
            pitch_min: PITCH_RANGE.0.to_radians(),
            pitch_max: PITCH_RANGE.1.to_radians(),
            yaw_min: (-YAW_LIMIT).to_radians(),
            yaw_max: YAW_LIMIT.to_radians(),
            ..Default::default()
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const GCS: (u8, u8) = (255, 190);

    #[test]
    fn status_flows_only_once_an_interval_is_asked_and_a_command_moves_the_gimbal() {
        let mut gimbal = Gimbal::default();
        assert!(gimbal.tick(0).is_empty());
        gimbal
            .command(
                GCS,
                1,
                MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL,
                [285.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            )
            .unwrap();
        gimbal
            .command(
                GCS,
                1,
                MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL,
                [281.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            )
            .unwrap();
        assert_eq!(gimbal.tick(1000).len(), 2);
        assert_eq!(
            gimbal.tick(1100).len(),
            1,
            "the attitude reports ten times a second so a steered camera follows promptly"
        );
        let moved = gimbal
            .command(
                GCS,
                1,
                MavCmd::MAV_CMD_DO_GIMBAL_MANAGER_PITCHYAW,
                [-90.0, 30.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            )
            .unwrap();
        assert!(
            matches!(&moved[0], (1, MavMessage::COMMAND_ACK(a)) if a.result == MavResult::MAV_RESULT_ACCEPTED)
        );
        gimbal.tick(5000);
        assert_eq!(
            (gimbal.pitch, gimbal.yaw),
            (-90.0, 30.0),
            "the commanded angles hold"
        );
        assert!(
            gimbal
                .command(
                    GCS,
                    1,
                    MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL,
                    [33.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
                )
                .is_none(),
            "other intervals are the autopilot's"
        );
    }

    #[test]
    fn a_gimbal_nobody_commands_holds_still() {
        let mut gimbal = Gimbal::default();
        gimbal
            .command(
                GCS,
                1,
                MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL,
                [285.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            )
            .unwrap();
        let reported: Vec<(f32, f32)> = (0..120)
            .flat_map(|second| gimbal.tick(second * 1000))
            .filter_map(|(_, message)| match message {
                MavMessage::GIMBAL_DEVICE_ATTITUDE_STATUS(a) => {
                    let (_, pitch, _) = crate::gimbal::euler_degrees(a.q);
                    Some((pitch, a.delta_yaw))
                }
                _ => None,
            })
            .collect();
        assert_eq!(reported.len(), 120);
        assert!(
            reported
                .iter()
                .all(|(pitch, yaw)| pitch.abs() < 0.01 && yaw.abs() < 0.01),
            "it stays level and forward over two minutes: {reported:?}"
        );
    }

    #[test]
    fn the_gimbal_looks_straight_down_and_tells_the_app_it_can() {
        let mut gimbal = Gimbal::default();
        let aim = |gimbal: &mut Gimbal, pitch: f32| {
            gimbal.command(
                GCS,
                1,
                MavCmd::MAV_CMD_DO_GIMBAL_MANAGER_PITCHYAW,
                [pitch, 20.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            )
        };
        gimbal
            .command(
                GCS,
                1,
                MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL,
                [285.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            )
            .unwrap();
        aim(&mut gimbal, -90.0).unwrap();
        let reported = gimbal.tick(60_000);
        assert_eq!(gimbal.pitch, -90.0, "straight down holds");
        assert!(
            matches!(&reported[..], [(GIMBAL, MavMessage::GIMBAL_DEVICE_ATTITUDE_STATUS(a))]
                if (crate::gimbal::euler_degrees(a.q).1 + 90.0).abs() < 0.1),
            "the attitude it reports is straight down"
        );
        aim(&mut gimbal, 60.0).unwrap();
        assert_eq!(gimbal.pitch, 30.0, "it tilts up no further than 30");
        assert!(
            matches!(information(), (AUTOPILOT, MavMessage::GIMBAL_MANAGER_INFORMATION(i))
                if (i.pitch_min.to_degrees() + 90.0).abs() < 0.01
                    && (i.pitch_max.to_degrees() - 30.0).abs() < 0.01),
            "the app is told the same range"
        );
    }
}
#[cfg(test)]
mod earth_frame_tests {
    use super::*;

    #[test]
    fn a_heading_held_against_north_stays_put_while_the_vehicle_turns() {
        let mut gimbal = Gimbal::default();
        let earth = crate::gimbal::FLAG_YAW_LOCK | crate::gimbal::FLAG_YAW_IN_EARTH_FRAME;
        gimbal.command(
            (255, 190),
            GIMBAL,
            MavCmd::MAV_CMD_DO_GIMBAL_MANAGER_PITCHYAW,
            [-30.0, 170.0, f32::NAN, f32::NAN, earth as f32, 0.0, 0.0],
        );
        gimbal.hold(320.0);
        assert_eq!(
            gimbal.aim(),
            (-150.0, -30.0),
            "170 north-relative from a drone facing 320"
        );
        gimbal.hold(185.0);
        assert_eq!(
            gimbal.aim(),
            (-15.0, -30.0),
            "the drone turned, the camera did not"
        );
        gimbal.command(
            (255, 190),
            GIMBAL,
            MavCmd::MAV_CMD_DO_GIMBAL_MANAGER_PITCHYAW,
            [-30.0, 20.0, f32::NAN, f32::NAN, 0.0, 0.0, 0.0],
        );
        gimbal.hold(90.0);
        assert_eq!(
            gimbal.aim(),
            (20.0, -30.0),
            "a vehicle-frame command turns with the drone again"
        );
    }
}
